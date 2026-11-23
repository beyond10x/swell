//! CompilePattern (`#sec-compilepattern`) and the matcher of 22.2.2 (`#sec-pattern-semantics`)
//! with and without `u`, read through the crate's public interface: a table of rows that reaches
//! every instruction and every point of the semantics, the forms of B.1.2
//! (`#sec-regular-expressions-patterns`), the positions in code units, the two stored forms of a
//! subject, the search of RegExpBuiltinExec (`#sec-regexpbuiltinexec`) with AdvanceStringIndex
//! (`#sec-advancestringindex`), and the refusal of the flag `i` until its story.
//!
//! An expected answer is written as the captures in order, group 0 first, each `start..end` in
//! code units or `-` when unset, or `no` for no match.

use swell_regexp::{
    Budget, CompileError, Flags, MAX_GRANT, ParseError, Program, Stop, Subject, Unlimited,
    compile_pattern, parse_pattern, search,
};

#[cfg(test)]
fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[cfg(test)]
fn program(pattern: &str, flags: &str) -> Program {
    let parsed_flags = Flags::parse(&units(flags)).expect("the flags");
    let tree = parse_pattern(&units(pattern), parsed_flags, 1000, &mut Unlimited)
        .unwrap_or_else(|e| panic!("/{pattern}/{flags}: {e:?}"));
    compile_pattern(&tree, &mut Unlimited).unwrap_or_else(|e| panic!("/{pattern}/{flags}: {e:?}"))
}

/// The answer of a search of `program` in `subject` from `start`, written as the module says.
#[cfg(test)]
fn answer(program: &Program, subject: Subject<'_>, start: usize) -> String {
    match search(program, subject, start, &mut Unlimited) {
        Ok(None) => "no".to_string(),
        Ok(Some(found)) => found
            .captures
            .iter()
            .map(|capture| match capture {
                Some(range) => format!("{}..{}", range.start, range.end),
                None => "-".to_string(),
            })
            .collect::<Vec<_>>()
            .join(" "),
        Err(stop) => format!("stop {stop:?}"),
    }
}

/// Rows: pattern, flags, subject, start index, expected answer.
const ROWS: &[(&str, &str, &str, usize, &str)] = &[
    // A code unit, the end of the match, and the search over the start indices.
    ("a", "", "ba", 0, "1..2"),
    ("abc", "", "xabcx", 0, "1..4"),
    ("x", "", "abc", 0, "no"),
    ("", "", "abc", 0, "0..0"),
    ("a", "", "aba", 1, "2..3"),
    ("a", "", "ab", 2, "no"),
    // Alternatives in order (a choice and a jump).
    ("a|ab", "", "abc", 0, "0..1"),
    ("ab|a", "", "abc", 0, "0..2"),
    ("a|", "", "b", 0, "0..0"),
    ("x|y|z", "", "--z", 0, "2..3"),
    (
        "((a)|(ab))((c)|(bc))",
        "",
        "abc",
        0,
        "0..3 0..1 0..1 - 1..3 - 1..3",
    ),
    // Greedy and lazy quantifiers, the counter of {n,m}.
    ("a*", "", "aaab", 0, "0..3"),
    ("a*?", "", "aaab", 0, "0..0"),
    ("a+?", "", "aaab", 0, "0..1"),
    ("a??b", "", "ab", 0, "0..2"),
    ("a{2,3}", "", "aaaa", 0, "0..3"),
    ("a{2,3}?", "", "aaaa", 0, "0..2"),
    ("a{2}", "", "a", 0, "no"),
    ("a{2,}", "", "aaaaa", 0, "0..5"),
    ("a{0}b", "", "ab", 0, "1..2"),
    ("(a){0}b\\1", "", "ab", 0, "1..2 -"),
    ("a[a-z]{2,4}", "", "abcdefghi", 0, "0..5"),
    ("a[a-z]{2,4}?", "", "abcdefghi", 0, "0..3"),
    ("(aa|aabaac|ba|b|c)*", "", "aabaac", 0, "0..4 2..4"),
    // The empty check (RepeatMatcher, step 2.b) and the counter past its minimum.
    ("(a*)*", "", "b", 0, "0..0 -"),
    ("(a*)b\\1+", "", "baaaac", 0, "0..1 0..0"),
    ("(a*){2}", "", "aa", 0, "0..2 2..2"),
    ("(?:a|)*b", "", "aab", 0, "0..3"),
    ("(a|)+?b", "", "ab", 0, "0..2 0..1"),
    // Captures reset at each iteration (RepeatMatcher, step 4).
    (
        "(z)((a+)?(b+)?(c))*",
        "",
        "zaacbbbcac",
        0,
        "0..10 0..1 8..10 8..9 - 9..10",
    ),
    ("(?:(a)|b)*", "", "ab", 0, "0..2 -"),
    ("(?:(a)|(b))+", "", "ab", 0, "0..2 - 1..2"),
    // Lookahead: atomic, keeps the captures of a positive one, resets those of a negative one.
    ("(?=(a+))", "", "baaabac", 0, "1..1 1..4"),
    ("(?=(a+))a*b\\1", "", "baaabac", 0, "3..6 3..4"),
    (
        "(.*?)a(?!(a+)b\\2c)\\2(.*)",
        "",
        "baaabaac",
        0,
        "0..8 0..2 - 3..8",
    ),
    ("(?!(a)b)a", "", "ac", 0, "0..1 -"),
    ("(?=(a|ab))ab", "", "ab", 0, "0..2 0..1"),
    ("(?=a|ab)abc|ab", "", "ab", 0, "0..2"),
    ("a(?=b)", "", "acab", 0, "2..3"),
    ("a(?!b)", "", "abac", 0, "2..3"),
    // Lookbehind: read right to left, captures filled so.
    (
        "(?<=\\$)\\d+(\\.\\d*)?",
        "",
        "cost $10.53",
        0,
        "6..11 8..11",
    ),
    ("(?<=(\\d+)(\\d+))$", "", "1053", 0, "4..4 0..1 1..4"),
    ("(?<!a)b", "", "abcb", 0, "3..4"),
    ("(?<=\\1(a))b", "", "aab", 0, "2..3 1..2"),
    ("(?<=(a)\\1)b", "", "ab", 0, "1..2 0..1"),
    ("(?<=(?=(a))a)b", "", "ab", 0, "1..2 0..1"),
    ("(?<=a(?<!ba))c", "", "bacac", 0, "4..5"),
    ("(?<=(a|ab))c", "", "abc", 0, "2..3 0..2"),
    ("(?<=a{2,3}?)b", "", "aaab", 0, "3..4"),
    ("(?<=(a{2,3}?))b", "", "aaab", 0, "3..4 1..3"),
    // Backreferences by number and by name; to an unset group the empty String.
    ("(a)\\1", "", "aa", 0, "0..2 0..1"),
    ("(a)\\1", "", "ab", 0, "no"),
    ("\\1(a)", "", "aa", 0, "0..1 0..1"),
    ("(a)|\\1b", "", "b", 0, "0..1 -"),
    ("(?<x>a)\\k<x>", "", "aa", 0, "0..2 0..1"),
    ("(?:(?<n>x)|(?<n>y))\\k<n>", "", "yy", 0, "0..2 - 0..1"),
    ("(?:(?<n>x)|(?<n>y))\\k<n>", "", "xx", 0, "0..2 0..1 -"),
    ("(?:(?<n>x)|(?<n>y))\\k<n>", "", "xy", 0, "no"),
    ("(?:(?<n>x)|(?<n>y)|z)\\k<n>", "", "z", 0, "0..1 - -"),
    // Assertions with and without m.
    ("^a", "", "ba", 0, "no"),
    ("^a", "m", "b\na", 0, "2..3"),
    ("^a", "m", "b\u{2028}a", 0, "2..3"),
    ("a$", "", "a\nb", 0, "no"),
    ("a$", "m", "a\rb", 0, "0..1"),
    ("a$", "", "ba", 0, "1..2"),
    ("^$", "m", "a\n\nb", 0, "2..2"),
    ("\\bfoo\\b", "", "a foo b", 0, "2..5"),
    ("\\bfoo\\b", "", "afoo", 0, "no"),
    ("\\Boo", "", "foo", 0, "1..3"),
    ("\\B", "", "", 0, "0..0"),
    ("\\b", "", "", 0, "no"),
    // `.` with and without s; the modifiers m and s.
    (".", "", "\n", 0, "no"),
    (".", "s", "\n", 0, "0..1"),
    (".", "", "\r\u{2028}\u{2029}a", 0, "3..4"),
    ("(?s:.)", "", "\n", 0, "0..1"),
    ("(?-s:.)", "s", "\nx", 0, "1..2"),
    ("(?s:(?-s:.))", "", "\nx", 0, "1..2"),
    ("(?m:^b)", "", "a\nb", 0, "2..3"),
    ("(?-m:^b)", "m", "a\nb", 0, "no"),
    ("(?m:a$)|b$", "", "a\nb", 0, "0..1"),
    // Classes: sorted disjoint ranges, negation, class escapes, \s with Zs.
    ("[a-c]+", "", "xxbcay", 0, "2..5"),
    ("[^a-c]+", "", "abxyc", 0, "2..4"),
    ("[ca-b]+", "", "-cab-", 0, "1..4"),
    ("[a-dc-f]+", "", "-afbe-", 0, "1..5"),
    ("\\d+", "", "ab123", 0, "2..5"),
    ("\\D", "", "12a", 0, "2..3"),
    ("\\w+", "", "  a_1Z!", 0, "2..6"),
    ("\\W", "", "ab-", 0, "2..3"),
    ("\\s+", "", "a \t\u{a0}\u{3000}\u{feff}\nb", 0, "1..7"),
    ("\\s", "", "a\u{2007}", 0, "1..2"),
    ("\\s", "", "a\u{200b}", 0, "no"),
    ("\\S", "", " \u{2029}x", 0, "2..3"),
    ("[\\d\\s]+", "", "a1 2b", 0, "1..4"),
    ("[^\\W\\d]+", "", "12ab_3", 0, "2..5"),
    ("[\\S\\s]", "", "\n", 0, "0..1"),
    ("[^\\s]", "", " \u{a0}b", 0, "2..3"),
    ("[]", "", "a", 0, "no"),
    ("[^]", "", "\n", 0, "0..1"),
    // Under u: code points, positions in code units.
    (".", "u", "\u{1F600}", 0, "0..2"),
    (".", "", "\u{1F600}", 0, "0..1"),
    ("^.$", "u", "\u{1F600}", 0, "0..2"),
    ("^.$", "", "\u{1F600}", 0, "no"),
    ("\u{1F600}", "u", "x\u{1F600}", 0, "1..3"),
    ("\u{1F600}", "", "x\u{1F600}", 0, "1..3"),
    ("\\u{1F601}", "u", "\u{1F600}\u{1F601}", 0, "2..4"),
    ("[\u{1F600}-\u{1F602}]", "u", "a\u{1F601}", 0, "1..3"),
    ("[^a]", "u", "\u{1F600}", 0, "0..2"),
    ("[^a]", "", "\u{1F600}", 0, "0..1"),
    ("\\uD83D", "u", "\u{1F600}", 0, "no"),
    ("\\uD83D", "", "\u{1F600}", 0, "0..1"),
    ("\\uDE00", "u", "\u{1F600}", 0, "no"),
    ("\\S", "u", "\u{1F600}", 0, "0..2"),
    ("\\W", "u", "\u{1F600}", 0, "0..2"),
    ("(.)\\1", "u", "\u{1F600}\u{1F600}", 0, "0..4 0..2"),
    ("(?<=\u{1F600})a", "u", "\u{1F600}a", 0, "2..3"),
    ("(?<=^.)a", "u", "\u{1F600}a", 0, "2..3"),
    ("(?<=^.)a", "", "\u{1F600}a", 0, "no"),
    ("(?<=(.))a", "u", "\u{1F600}a", 0, "2..3 0..2"),
    ("\u{1F600}+", "u", "\u{1F600}\u{1F600}x", 0, "0..4"),
    ("\u{1F600}{2}", "u", "\u{1F600}\u{1F600}", 0, "0..4"),
    ("\\bx", "u", "\u{1F600}x", 0, "2..3"),
    // The forms of B.1.2.
    ("\\8", "", "8", 0, "0..1"),
    ("a{", "", "a{", 0, "0..2"),
    ("a{1", "", "a{1", 0, "0..3"),
    ("x{1,a}", "", "x{1,a}", 0, "0..6"),
    ("}", "", "}", 0, "0..1"),
    ("]", "", "]", 0, "0..1"),
    ("\\c", "", "\\c", 0, "0..2"),
    ("\\cA", "", "\u{1}", 0, "0..1"),
    ("[\\c_]", "", "\u{1f}", 0, "0..1"),
    ("[\\c]", "", "c", 0, "0..1"),
    ("\\101", "", "A", 0, "0..1"),
    ("\\08", "", "\u{0}8", 0, "0..2"),
    ("\\1", "", "\u{1}", 0, "0..1"),
    ("\\k", "", "k", 0, "0..1"),
    ("\\x4", "", "x4", 0, "0..2"),
    ("\\u00", "", "u00", 0, "0..3"),
    ("\\z", "", "z", 0, "0..1"),
    ("[\\b]", "", "\u{8}", 0, "0..1"),
    ("[\\d-z]+", "", "1-z", 0, "0..3"),
    ("[a-\\d]+", "", "a-5", 0, "0..3"),
    ("(?=a)*a", "", "a", 0, "0..1"),
    ("(?=(a))?a", "", "a", 0, "0..1 -"),
    ("(?=(a))+a", "", "a", 0, "0..1 0..1"),
    ("(?!a){2}b", "", "b", 0, "0..1"),
];

#[test]
fn the_rows_of_the_table_match() {
    let mut failures = Vec::new();
    for &(pattern, flags, subject, start, expected) in ROWS {
        let program = program(pattern, flags);
        let got = answer(&program, Subject::Utf16(&units(subject)), start);
        if got != expected {
            failures.push(format!(
                "/{pattern}/{flags} on {subject:?} from {start}: {got}, expected {expected}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Subjects with lone surrogates, which a `&str` cannot hold.
#[test]
fn lone_surrogates_match_as_code_points_under_u_and_as_code_units_without() {
    let rows: &[(&str, &str, &[u16], &str)] = &[
        ("\\uD83D", "u", &[0x61, 0xD83D, 0x62], "1..2"),
        ("\\uD83D", "u", &[0xD83D, 0xDE00, 0xD83D], "2..3"),
        ("\\uDE00", "u", &[0xD83D, 0xDE00, 0xDE00], "2..3"),
        ("^.", "u", &[0xDE00, 0xD83D], "0..1"),
        (".$", "u", &[0x61, 0xD83D], "1..2"),
        ("[\\uD800-\\uDBFF]", "u", &[0xD83D, 0xDE00, 0xD800], "2..3"),
        ("[\\uD800-\\uDBFF]", "", &[0xD83D, 0xDE00, 0xD800], "0..1"),
        // A backreference to a lone lead does not match the lead of a pair under u.
        ("(\\uD83D)\\1", "u", &[0xD83D, 0xD83D, 0xDE00], "no"),
        ("(\\uD83D)\\1", "", &[0xD83D, 0xD83D, 0xDE00], "0..2 0..1"),
        // Read right to left, the unit before a lone trail that a pair holds is no match.
        ("(?<=\\1(.))x", "u", &[0xD83D, 0xDE00, 0xDE00, 0x78], "no"),
        (
            "(?<=\\1(.))x",
            "",
            &[0xD83D, 0xDE00, 0xDE00, 0x78],
            "3..4 2..3",
        ),
        (
            "(?<=(\\uDE00)\\1)x",
            "",
            &[0xDE00, 0xDE00, 0x78],
            "2..3 1..2",
        ),
        // Without u a quantifier after a pair written in the pattern repeats its trail.
        ("\u{1F600}{2}", "", &[0xD83D, 0xDE00, 0xDE00], "0..3"),
    ];
    for &(pattern, flags, subject, expected) in rows {
        let program = program(pattern, flags);
        let got = answer(&program, Subject::Utf16(subject), 0);
        assert_eq!(got, expected, "/{pattern}/{flags} on {subject:x?}");
    }
}

/// One pattern per row of `docs/plan/slice-05/consumer-regexps.tsv`, written for this test with
/// the constructs of that row.
#[test]
fn the_constructs_of_the_consumer_table_match() {
    let rows: &[(&str, &str, &[u16], &str)] = &[
        // anchors, identity escapes, \s, a non-capturing group, { and } as pattern
        // characters (Annex B), captures, alternation
        (
            "^\\s*(?:\\{(\\w+)}|x{y|(\\w+)\\.)\\s*$",
            "",
            &units(" {abc} "),
            "0..7 2..5 -",
        ),
        (
            "^\\s*(?:\\{(\\w+)}|x{y|(\\w+)\\.)\\s*$",
            "",
            &units("x{y"),
            "0..3 - -",
        ),
        // a class of \u escapes
        ("[\\u0000-\\u001F\\u007F]", "", &units("ab\u{7f}"), "2..3"),
        // anchors, captures, a negated class, a quantified group
        (
            "^([^:]+)(?::(\\d+))*$",
            "",
            &units("host:80:81"),
            "0..10 0..4 8..10",
        ),
        // \n
        ("\\n", "g", &units("a\nb"), "1..2"),
        // captures, classes of surrogate ranges, a negative lookahead, alternation, ^ in a group
        (
            "([\\uD800-\\uDBFF])(?![\\uDC00-\\uDFFF])|(^|[^\\uD800-\\uDBFF])([\\uDC00-\\uDFFF])",
            "g",
            &[0x61, 0xD800, 0x62],
            "1..2 1..2 - -",
        ),
        (
            "([\\uD800-\\uDBFF])(?![\\uDC00-\\uDFFF])|(^|[^\\uD800-\\uDBFF])([\\uDC00-\\uDFFF])",
            "g",
            &[0xDC00, 0x62],
            "0..1 - 0..0 0..1",
        ),
        (
            "([\\uD800-\\uDBFF])(?![\\uDC00-\\uDFFF])|(^|[^\\uD800-\\uDBFF])([\\uDC00-\\uDFFF])",
            "g",
            &[0xD83D, 0xDE00],
            "no",
        ),
        // \s
        ("\\s+", "g", &units("a  b"), "1..3"),
        // \w, \d, identity escapes, a negated class, an optional non-capturing group
        ("\\w+\\.\\d+(?:[^\\s]+)?", "g", &units("v1.23x y"), "0..6"),
        // identity escapes of $ { }, a capture, a negated class
        ("\\$\\{([^}]*)\\}", "g", &units("a ${x} b"), "2..6 4..5"),
        // a class of escaped syntax characters
        ("[\\^$.*+?()[\\]{}|\\\\]", "g", &units("ab*"), "2..3"),
        // anchors, a capture, an empty alternative, identity escapes
        ("^(\\-|)\\d+$", "", &units("-12"), "0..3 0..1"),
        ("^(\\-|)\\d+$", "", &units("12"), "0..2 0..0"),
        // { and } as pattern characters (Annex B), a capture, a negated class
        ("{([^}]*)}", "g", &units("a{b}c"), "1..4 2..3"),
        // a class of \x escapes
        ("[\\x00-\\x1f\\x7f]", "g", &units("a\u{1}"), "1..2"),
    ];
    for &(pattern, flags, subject, expected) in rows {
        let program = program(pattern, flags);
        let got = answer(&program, Subject::Utf16(subject), 0);
        assert_eq!(got, expected, "/{pattern}/{flags} on {subject:x?}");
    }
}

#[test]
fn positions_are_code_unit_indices_in_every_mode() {
    let pair = units("\u{1F600}b");
    // A match after a surrogate pair answers the code unit index past it, with and without u.
    for flags in ["", "u"] {
        let found = search(
            &program("b", flags),
            Subject::Utf16(&pair),
            0,
            &mut Unlimited,
        )
        .expect("no stop")
        .expect("a match");
        assert_eq!(found.index, 2, "/b/{flags}");
        assert_eq!(found.captures.get(0), Some(2..3), "/b/{flags}");
    }
    // A start index on the second unit of a pair starts the try at its first under u; the
    // index the search answers is the caller's.
    let one = units("\u{1F600}");
    let found = search(&program(".", "u"), Subject::Utf16(&one), 1, &mut Unlimited)
        .expect("no stop")
        .expect("a match");
    assert_eq!((found.index, found.captures.get(0)), (1, Some(0..2)));
    let found = search(
        &program("\u{1F600}", "u"),
        Subject::Utf16(&one),
        1,
        &mut Unlimited,
    )
    .expect("no stop")
    .expect("a match");
    assert_eq!((found.index, found.captures.get(0)), (1, Some(0..2)));
    // Without u the second unit is a character of its own.
    let found = search(
        &program("\\uDE00", ""),
        Subject::Utf16(&one),
        1,
        &mut Unlimited,
    )
    .expect("no stop")
    .expect("a match");
    assert_eq!((found.index, found.captures.get(0)), (1, Some(1..2)));
    // Captures past pairs count code units.
    let text = units("\u{1F600}\u{1F601}x\u{1F602}");
    let found = search(
        &program("(\u{1F601})(x)(.)", "u"),
        Subject::Utf16(&text),
        0,
        &mut Unlimited,
    )
    .expect("no stop")
    .expect("a match");
    let all: Vec<_> = found.captures.iter().collect();
    assert_eq!(all, vec![Some(2..7), Some(2..4), Some(4..5), Some(5..7)]);
    assert_eq!(found.captures.len(), 4);
    assert_eq!(found.captures.get(4), None);
    // A start index past the length tries nothing.
    assert!(
        search(&program("", "u"), Subject::Utf16(&one), 3, &mut Unlimited)
            .expect("no stop")
            .is_none()
    );
    let found = search(&program("", "u"), Subject::Utf16(&one), 2, &mut Unlimited)
        .expect("no stop")
        .expect("a match");
    assert_eq!(found.captures.get(0), Some(2..2));
}

#[test]
fn a_latin1_and_a_16_bit_subject_match_alike() {
    let mut compared = 0;
    for &(pattern, flags, subject, start, expected) in ROWS {
        if subject.chars().any(|c| u32::from(c) > 0xFF) {
            continue;
        }
        let program = program(pattern, flags);
        let wide = units(subject);
        let narrow: Vec<u8> = subject.chars().map(|c| u32::from(c) as u8).collect();
        let a = answer(&program, Subject::Utf16(&wide), start);
        let b = answer(&program, Subject::Latin1(&narrow), start);
        assert_eq!(a, expected, "/{pattern}/{flags} on {subject:?} (16-bit)");
        assert_eq!(b, expected, "/{pattern}/{flags} on {subject:?} (Latin-1)");
        compared += 1;
    }
    assert!(compared > 100, "{compared} rows over both forms");
    // A Latin-1 subject with code units past ASCII.
    let narrow = [0x61u8, 0xA0, 0xE9, 0xFF];
    let program = program("\\s(\u{e9})[\u{f0}-\u{ff}]$", "u");
    assert_eq!(
        answer(&program, Subject::Latin1(&narrow), 0),
        "1..4 2..3".to_string()
    );
    assert_eq!(Subject::Latin1(&narrow).len(), 4);
}

/// A budget that counts the steps it grants net of the returns.
#[derive(Default)]
struct Steps {
    granted: u64,
    refunded: u64,
}

impl Budget for Steps {
    fn charge(&mut self, _bytes: usize) -> Result<(), Stop> {
        Ok(())
    }

    fn credit(&mut self, _bytes: usize) {}

    fn grant(&mut self, asked: u32) -> Result<u32, Stop> {
        let given = asked.clamp(1, MAX_GRANT);
        self.granted += u64::from(given);
        Ok(given)
    }

    fn refund(&mut self, steps: u32) {
        self.refunded += u64::from(steps);
    }

    fn poll(&mut self) -> Result<(), Stop> {
        Ok(())
    }
}

/// The steps a search of `pattern` with `flags` in `subject` from `start` spends, and whether
/// it matched.
#[cfg(test)]
fn steps_of(pattern: &str, flags: &str, subject: &[u16], start: usize) -> (u64, Option<usize>) {
    let program = program(pattern, flags);
    let mut budget = Steps::default();
    let found = search(&program, Subject::Utf16(subject), start, &mut budget).expect("no stop");
    (budget.granted - budget.refunded, found.map(|f| f.index))
}

#[test]
fn sticky_search_tries_one_index_and_unicode_search_steps_over_a_pair() {
    let ba = units("ba");
    // /a/ in "ba": index 0 tried (1), the code unit fails (1), index 1 tried (1), the code
    // unit (1), the end of the match (1).
    assert_eq!(steps_of("a", "", &ba, 0), (5, Some(1)));
    // Sticky: one try, the code unit fails, no second index.
    assert_eq!(steps_of("a", "y", &ba, 0), (2, None));
    assert_eq!(steps_of("a", "y", &ba, 1), (3, Some(1)));
    assert_eq!(steps_of("a", "gy", &ba, 0), (2, None));
    // A start index past the length tries nothing.
    assert_eq!(steps_of("a", "", &ba, 3), (0, None));
    // Under u a search moves past a pair in one step of AdvanceStringIndex: a lone trail is
    // never found inside a pair. Tries at 0, 2 and 3 (the end), each one try and one unit.
    let pair_x = units("\u{1F600}x");
    assert_eq!(steps_of("\\uDE00", "u", &pair_x, 0), (6, None));
    // Without u the trail is found at index 1: a try that fails (2), then one that matches (3).
    assert_eq!(steps_of("\\uDE00", "", &pair_x, 0), (5, Some(1)));
    // Sticky under u from the second unit of a pair: the try starts at the pair's first unit.
    let program = program("\u{1F600}", "uy");
    let found = search(&program, Subject::Utf16(&pair_x), 1, &mut Unlimited)
        .expect("no stop")
        .expect("a match");
    assert_eq!((found.index, found.captures.get(0)), (1, Some(0..2)));
    let program = self::program("x", "uy");
    assert!(
        search(&program, Subject::Utf16(&pair_x), 1, &mut Unlimited)
            .expect("no stop")
            .is_none()
    );
}

#[test]
fn case_insensitive_matching_is_a_refusal_of_slice_5() {
    let compile = |pattern: &str, flags: &str| {
        let flags = Flags::parse(&units(flags)).expect("the flags");
        let tree = parse_pattern(&units(pattern), flags, 1000, &mut Unlimited)?;
        Ok::<_, ParseError>(compile_pattern(&tree, &mut Unlimited))
    };
    for (pattern, flags) in [
        ("a", "i"),
        ("(?i:a)", ""),
        ("x(?i:a)", "u"),
        ("(?-i:a)", ""),
    ] {
        match compile(pattern, flags) {
            Ok(Err(error @ CompileError::Refused(_))) => {
                assert_eq!(
                    error.message(),
                    "case-insensitive matching is not supported before slice 5",
                    "/{pattern}/{flags}"
                );
                assert!(error.is_refusal());
            }
            other => panic!("/{pattern}/{flags}: {other:?}"),
        }
    }
    // An early error comes first: the parse answers it before the compile step runs.
    match compile("(?i-i:a)", "") {
        Err(ParseError::Pattern(e)) => {
            assert_eq!(e.message(), "a modifier both added and removed");
        }
        other => panic!("(?i-i:a): {other:?}"),
    }
    assert!(matches!(compile("a(", "i"), Err(ParseError::Pattern(_))));
    // The modifiers m and s alone are compiled.
    assert!(matches!(compile("(?ms-:a)", ""), Ok(Ok(_))));
    assert!(matches!(compile("(?-ms:a)", "ms"), Ok(Ok(_))));
}

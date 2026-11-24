//! The flag `v` (`#sec-patterns` with [+UnicodeSetsMode]) read through the crate's public
//! interface: the class set expressions (ClassUnion, ClassIntersection `&&`, ClassSubtraction
//! `--`, nested classes), the strings of `\q{...}` and the properties of strings, which match
//! longest first (`#sec-compileatom`, the alternatives of a CharSet with strings); the early
//! errors and the grammar errors of `#sec-patterns-static-semantics-early-errors`; the case
//! folding of `#sec-maybesimplecasefolding`, applied to the set before its complement; the
//! nesting limit and no Rust recursion for nested classes; the steps of a class and the memory
//! budget of the compile step.
#![cfg(test)]

use swell_regexp::{
    Budget, CompileError, Flags, MAX_GRANT, ParseError, Pattern, Program, Stop, Subject, Unlimited,
    compile_pattern, parse_pattern, search,
};

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn flags(text: &str) -> Flags {
    Flags::parse(&units(text)).expect("the flags")
}

fn parse(pattern: &str, f: &str) -> Result<Pattern, ParseError> {
    parse_pattern(&units(pattern), flags(f), 1000, &mut Unlimited)
}

fn program(pattern: &str, f: &str) -> Program {
    let tree = parse(pattern, f).unwrap_or_else(|e| panic!("/{pattern}/{f}: {e:?}"));
    compile_pattern(&tree, &mut Unlimited).unwrap_or_else(|e| panic!("/{pattern}/{f}: {e:?}"))
}

/// The match of `program` in `subject` from 0: `start..end` of group 0, or `no`.
fn answer(program: &Program, subject: &str) -> String {
    match search(program, Subject::Utf16(&units(subject)), 0, &mut Unlimited) {
        Ok(Some(found)) => match found.captures.get(0) {
            Some(range) => format!("{}..{}", range.start, range.end),
            None => "unset".to_string(),
        },
        Ok(None) => "no".to_string(),
        Err(stop) => format!("stop {stop:?}"),
    }
}

/// Each `(pattern, subject, answer)` under the flags `f`.
fn holds(f: &str, cases: &[(&str, &str, &str)]) {
    for &(pattern, subject, expected) in cases {
        let found = answer(&program(pattern, f), subject);
        assert_eq!(found, expected, "/{pattern}/{f} on {subject:?}");
    }
}

#[test]
fn union_intersection_and_subtraction_of_characters_and_classes() {
    holds(
        "v",
        &[
            ("^[abc]$", "b", "0..1"),
            ("^[abc]$", "d", "no"),
            ("^[[a-z]&&[aeiou]]$", "e", "0..1"),
            ("^[[a-z]&&[aeiou]]$", "b", "no"),
            ("^[\\w--\\d]$", "a", "0..1"),
            ("^[\\w--\\d]$", "_", "0..1"),
            ("^[\\w--\\d]$", "1", "no"),
            ("^[[a-z]--[aeiou]--x]$", "b", "0..1"),
            ("^[[a-z]--[aeiou]--x]$", "a", "no"),
            ("^[[a-z]--[aeiou]--x]$", "x", "no"),
            ("^[\\p{L}&&\\p{ASCII}]$", "a", "0..1"),
            ("^[\\p{L}&&\\p{ASCII}]$", "\u{E9}", "no"),
            ("^[\\p{L}--[a-z]]$", "\u{E9}", "0..1"),
            ("^[\\p{L}--[a-z]]$", "A", "0..1"),
            ("^[\\p{L}--[a-z]]$", "a", "no"),
            ("^[^a-c]$", "d", "0..1"),
            ("^[^a-c]$", "b", "no"),
            ("^[^a-c]$", "\u{1F600}", "0..2"),
            ("^[[^a-c]&&[a-e]]$", "d", "0..1"),
            ("^[[^a-c]&&[a-e]]$", "a", "no"),
            ("^[]$", "a", "no"),
            ("^[^]$", "a", "0..1"),
            ("^[a[b[c]]]$", "c", "0..1"),
            ("^[a[^b-z]]$", "a", "0..1"),
            ("^[a[^b-z]]$", "b", "no"),
            ("^[a[^b-z]]$", "1", "0..1"),
            ("^[\\D&&[0-9a]]$", "a", "0..1"),
            ("^[\\D&&[0-9a]]$", "1", "no"),
            ("^[\\S--\\s]$", "x", "0..1"),
            ("^[\\S--\\s]$", " ", "no"),
            ("^[\\W&&[!a]]$", "!", "0..1"),
            ("^[\\W&&[!a]]$", "a", "no"),
            ("^[\\P{L}&&[a1]]$", "1", "0..1"),
            ("^[\\P{L}&&[a1]]$", "a", "no"),
            (
                "^[[\\u{1F600}-\\u{1F64F}]--\\u{1F601}]$",
                "\u{1F600}",
                "0..2",
            ),
            ("^[[\\u{1F600}-\\u{1F64F}]--\\u{1F601}]$", "\u{1F601}", "no"),
            (
                "^[\\&\\-\\!\\#\\%\\,\\:\\;\\<\\=\\>\\@\\`\\~]+$",
                "&-!#%,:;<=>@`~",
                "0..14",
            ),
            ("^[a&b]+$", "a&b", "0..3"),
            ("^[\\b\\x41\\u0042\\u{43}]+$", "\u{8}ABC", "0..4"),
            ("\\p{Lu}", "aBc", "1..2"),
            ("^\\P{L}$", "1", "0..1"),
            ("^\\P{L}$", "a", "no"),
            ("(?<=[[a-z]&&[^aeiou]])x", "bx", "1..2"),
            ("(?<=[[a-z]&&[^aeiou]])x", "ax", "no"),
            ("^[\\d--[13579]]+", "2468x", "0..4"),
            ("[[[a-c][x-z]]&&[b-y]]", "axb", "1..2"),
        ],
    );
    // An intersection or a subtraction of many operands evaluates left to right.
    holds(
        "v",
        &[
            ("^[\\w&&[a-z]&&[^x]&&[a-y]]$", "x", "no"),
            ("^[\\w&&[a-z]&&[^x]&&[a-y]]$", "b", "0..1"),
            ("^[\\w--[a-c]--[x-z]--\\d]+$", "dw_", "0..3"),
            ("^[\\w--[a-c]--[x-z]--\\d]+$", "dz", "no"),
        ],
    );
}

#[test]
fn strings_of_q_match_longest_first() {
    holds(
        "v",
        &[
            ("^[\\q{abc|ab|a}]", "abcd", "0..3"),
            // The longer strings are tried first and given up when the rest fails.
            ("^[\\q{abc|ab|a}]c", "abc", "0..3"),
            ("^[\\q{abc|ab|a}]bc$", "abc", "0..3"),
            ("^[\\q{a|ab|abc}]$", "ab", "0..2"),
            ("^[\\q{abc|ab}a-z]$", "ab", "0..2"),
            ("^[\\q{abc|ab}a-z]$", "x", "0..1"),
            ("^[\\q{abc|ab}a-z]$", "abd", "no"),
            ("^[\\q{}]$", "", "0..0"),
            ("^[\\q{|a}]b", "b", "0..1"),
            ("^[\\q{|a}]b", "ab", "0..2"),
            ("^[\\q{ab}--\\q{ab}]", "ab", "no"),
            ("^[[\\q{ab|cd}a]--\\q{ab}]$", "ab", "no"),
            ("^[[\\q{ab|cd}a]--\\q{ab}]$", "cd", "0..2"),
            ("^[[\\q{ab|cd}a]--\\q{ab}]$", "a", "0..1"),
            ("^[[\\q{ab|cd}]&&[\\q{cd|ef}]]$", "cd", "0..2"),
            ("^[[\\q{ab|cd}]&&[\\q{cd|ef}]]$", "ab", "no"),
            (
                "^[\\q{\\u{1F600}\\u{1F601}|x}]$",
                "\u{1F600}\u{1F601}",
                "0..4",
            ),
            ("^[\\q{a\\-b|\\&\\&}]+$", "a-b&&a-b", "0..8"),
            ("^[\\q{a}&&a]$", "a", "0..1"),
            ("(?<=[\\q{ab|b}])c", "abc", "2..3"),
            ("(?<=^[\\q{ab|b}])c", "abc", "2..3"),
            ("(?<=^[\\q{a|b}])c", "abc", "no"),
            ("^[\\q{abc|a}]+$", "abcabca", "0..7"),
            ("^(?:[\\q{ab|a}])(b)?$", "ab", "0..2"),
            ("^[\\q{xyz}\\q{xy}]z$", "xyz", "0..3"),
        ],
    );
    let found = search(
        &program("^([\\q{ab|a}])(b?)$", "v"),
        Subject::Utf16(&units("ab")),
        0,
        &mut Unlimited,
    )
    .expect("no stop")
    .expect("a match");
    assert_eq!(
        found.captures.get(1),
        Some(0..2),
        "the longest string first"
    );
    assert_eq!(found.captures.get(2), Some(2..2));
}

#[test]
fn a_property_of_strings_matches_its_sequences() {
    let germany = "\u{1F1E9}\u{1F1EA}";
    let france = "\u{1F1EB}\u{1F1F7}";
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
    let england = "\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}";
    holds(
        "v",
        &[
            ("^\\p{RGI_Emoji_Flag_Sequence}$", germany, "0..4"),
            ("^\\p{RGI_Emoji_Flag_Sequence}$", "\u{1F1E9}", "no"),
            ("^\\p{Emoji_Keycap_Sequence}$", "#\u{FE0F}\u{20E3}", "0..3"),
            ("^\\p{Emoji_Keycap_Sequence}$", "#", "no"),
            ("^\\p{RGI_Emoji}$", "\u{231A}", "0..1"),
            ("^\\p{RGI_Emoji}$", "\u{A9}\u{FE0F}", "0..2"),
            ("^\\p{RGI_Emoji}$", "\u{A9}", "no"),
            ("^\\p{Basic_Emoji}$", "\u{231A}", "0..1"),
            ("^\\p{RGI_Emoji}", family, "0..11"),
            ("^\\p{RGI_Emoji_ZWJ_Sequence}$", family, "0..11"),
            (
                "^\\p{RGI_Emoji_Modifier_Sequence}$",
                "\u{1F44D}\u{1F3FD}",
                "0..4",
            ),
            ("^\\p{RGI_Emoji_Tag_Sequence}$", england, "0..14"),
            ("^[\\p{RGI_Emoji}--\\q{\u{1F1E9}\u{1F1EA}}]$", germany, "no"),
            (
                "^[\\p{RGI_Emoji}--\\q{\u{1F1E9}\u{1F1EA}}]$",
                france,
                "0..4",
            ),
            (
                "^[\\p{RGI_Emoji_Flag_Sequence}&&\\q{\u{1F1E9}\u{1F1EA}|x}]$",
                germany,
                "0..4",
            ),
            (
                "^[\\p{RGI_Emoji_Flag_Sequence}&&\\q{\u{1F1E9}\u{1F1EA}|x}]$",
                "x",
                "no",
            ),
            (
                "(?<=\\p{RGI_Emoji_Flag_Sequence})x",
                "\u{1F1E9}\u{1F1EA}x",
                "4..5",
            ),
            ("^[\\p{RGI_Emoji}a]+$", "a\u{231A}a", "0..3"),
            ("^\\p{RGI_Emoji}{2}$", "\u{1F44D}\u{1F3FD}", "0..4"),
        ],
    );
    let all = format!("{germany}{france}{family}{england}\u{231A}#\u{FE0F}\u{20E3}");
    let whole = format!("0..{}", all.encode_utf16().count());
    holds("v", &[("^\\p{RGI_Emoji}+$", &all, &whole)]);
}

/// The error of `/pattern/f`: its message and offset.
fn error(pattern: &str, f: &str) -> (String, usize) {
    match parse(pattern, f) {
        Err(ParseError::Pattern(e)) => {
            assert!(!e.is_refusal(), "/{pattern}/{f}: {}", e.message());
            (e.message(), e.offset)
        }
        other => panic!("/{pattern}/{f}: {other:?}"),
    }
}

#[test]
fn the_early_errors_of_v() {
    let reserved = "a reserved double punctuator in a class";
    let mixed = "class set operations mixed in one class";
    let syntax = "a class set syntax character that is not escaped";
    let missing = "a class set operation without its operand";
    let strings = "a negated class or \\P of a set that may contain strings";
    for (pattern, message, offset) in [
        ("[a!!b]", reserved, 2),
        ("[!!]", reserved, 1),
        ("[&&a]", reserved, 1),
        ("[a&&&b]", reserved, 4),
        ("[\\q{a&&b}]", reserved, 5),
        ("[a&&]", missing, 4),
        ("[a--]", missing, 4),
        ("[ab&&c]", mixed, 3),
        ("[a&&b--c]", mixed, 5),
        ("[a--b&&c]", mixed, 5),
        ("[a-z&&b]", mixed, 4),
        ("[a--b-c]", mixed, 5),
        ("[a&&b-c]", mixed, 5),
        ("[a&&&&b]", reserved, 4),
        ("[a&&bc]", mixed, 5),
        ("[[a][b]&&[c]]", mixed, 7),
        ("[a-]", syntax, 2),
        ("[-a]", syntax, 1),
        ("[(]", syntax, 1),
        ("[)]", syntax, 1),
        ("[{]", syntax, 1),
        ("[}]", syntax, 1),
        ("[/]", syntax, 1),
        ("[|]", syntax, 1),
        ("[\\q{a|(}]", syntax, 6),
        ("[^\\p{RGI_Emoji}]", strings, 0),
        ("\\P{RGI_Emoji}", strings, 0),
        ("[\\P{RGI_Emoji}]", strings, 1),
        ("[^\\q{ab}]", strings, 0),
        ("[^\\q{}]", strings, 0),
        ("[a[^\\q{ab}]]", strings, 2),
        ("[^[\\q{ab}--a]]", strings, 0),
        ("[^[\\p{RGI_Emoji}]]", strings, 0),
        ("[z-a]", "range out of order in character class", 1),
        ("[\\q{a]", syntax, 5),
        ("[\\q{a", "unterminated character class", 0),
        ("[\\q{\\d}]", "invalid escape", 4),
        ("[a-\\d]", syntax, 2),
        ("[\\q]", "invalid escape", 1),
        ("\\q{a}", "invalid escape", 0),
        ("[a", "unterminated character class", 0),
        ("[[a]", "unterminated character class", 0),
        ("\\p{Basic_Emoji=Yes}", "invalid property name", 0),
    ] {
        assert_eq!(
            error(pattern, "v"),
            (message.to_string(), offset),
            "/{pattern}/v"
        );
    }
    // A property of strings is the early error of an unknown name without `v`.
    for pattern in ["\\p{RGI_Emoji}", "[\\p{Basic_Emoji}]"] {
        assert_eq!(
            error(pattern, "u").0,
            "invalid property name",
            "/{pattern}/u"
        );
    }
    for pattern in [
        "[a&&b]",
        "[a--b]",
        "[[a]&&[b]]",
        "[\\&\\&]",
        "[&]",
        "[a&b]",
        "[!]",
        "[^\\q{a}]",
        "[^\\q{a|b}]",
        "[^[\\q{ab}&&a]]",
        "[^\\p{RGI_Emoji}&&a]",
        "[\\q{}]",
        "[\\q{a|b|}]",
        "[\\p{RGI_Emoji}--\\q{x}]",
        "[^\\p{L}]",
        "\\p{RGI_Emoji}",
        "[[a-z]--b]",
        "[\\d--3]",
        "[^]",
        "[]",
        "[^^]",
        "[\\q{\\}|\\|}]",
        "(?<a>x)\\k<a>[a]",
    ] {
        if let Err(e) = parse(pattern, "v") {
            panic!("/{pattern}/v: {e:?}");
        }
    }
}

#[test]
fn case_folding_under_v() {
    holds(
        "vi",
        &[
            ("^[\\q{AbC}]$", "abc", "0..3"),
            ("^[\\q{AbC}]$", "ABC", "0..3"),
            ("^[a-z]$", "K", "0..1"),
            ("^[a-z]$", "\u{212A}", "0..1"),
            ("^[^a]$", "A", "no"),
            ("^\\p{Lowercase}$", "A", "0..1"),
            ("^[\\w--\\q{k}]$", "K", "no"),
            ("^[\\w--\\q{k}]$", "\u{212A}", "no"),
            ("^[\\w--\\q{k}]$", "a", "0..1"),
            ("^[[a-z]&&\\q{A}]$", "a", "0..1"),
        ],
    );
    // `\P{Lu}` under `v` with `i`: the folding is applied to the set of `Lu` and the complement
    // taken after it (MaybeSimpleCaseFolding, then CharacterComplement against the characters
    // that are their own simple case folding), so it holds a character when no character of its
    // canonical value is in `Lu`, and `[^\P{Lu}]` holds both cases of a letter. Under `u` with
    // `i` the complement is taken first and the folding at the match, so `\P{Lu}` holds a
    // character when some character of its canonical value is not in `Lu`, and `[^\P{Lu}]`
    // holds no cased letter (`#sec-runtime-semantics-charactersetmatcher-abstract-operation`).
    for (pattern, flags, a, upper_a) in [
        ("^\\P{Lu}$", "vi", "no", "no"),
        ("^\\P{Lu}$", "ui", "0..1", "0..1"),
        ("^[^\\P{Lu}]$", "vi", "0..1", "0..1"),
        ("^[^\\P{Lu}]$", "ui", "no", "no"),
        ("^\\P{Lu}$", "v", "0..1", "no"),
        ("^[^\\P{Lu}]$", "v", "no", "0..1"),
        ("^[\\p{Lu}&&[a-z]]$", "vi", "0..1", "0..1"),
        ("^[\\p{Lu}&&[a-z]]$", "v", "no", "no"),
    ] {
        let program = program(pattern, flags);
        assert_eq!(answer(&program, "a"), a, "/{pattern}/{flags} on \"a\"");
        assert_eq!(
            answer(&program, "A"),
            upper_a,
            "/{pattern}/{flags} on \"A\""
        );
    }
    // A character outside a class and a backreference fold as under `u`.
    holds(
        "vi",
        &[("^a(b)\\1$", "ABb", "0..3"), ("\\u212A", "k", "0..1")],
    );
}

#[test]
fn nested_classes_past_the_nesting_limit_are_the_nesting_error() {
    let nested = |n: usize| units(&format!("{}a{}", "[".repeat(n), "]".repeat(n)));
    assert!(parse_pattern(&nested(1000), flags("v"), 1000, &mut Unlimited).is_ok());
    match parse_pattern(&nested(1001), flags("v"), 1000, &mut Unlimited) {
        Err(ParseError::Pattern(e)) => {
            assert_eq!(e.message(), "nesting deeper than parser_nesting (1000)");
            assert_eq!(e.offset, 1000, "the offset of the 1,001st class");
            assert!(!e.is_refusal());
        }
        other => panic!("1,001 classes at a limit of 1,000: {other:?}"),
    }
    // Groups and classes count together.
    let mixed = |groups: usize, classes: usize| {
        units(&format!(
            "{}{}a{}{}",
            "(".repeat(groups),
            "[".repeat(classes),
            "]".repeat(classes),
            ")".repeat(groups)
        ))
    };
    assert!(parse_pattern(&mixed(998, 2), flags("v"), 1000, &mut Unlimited).is_ok());
    assert!(matches!(
        parse_pattern(&mixed(999, 2), flags("v"), 1000, &mut Unlimited),
        Err(ParseError::Pattern(e)) if e.offset == 1000
    ));
    // Siblings are no nesting.
    let siblings = units(&"[a[b][c]]".repeat(500));
    assert!(parse_pattern(&siblings, flags("v"), 2, &mut Unlimited).is_ok());
}

/// Runs `body` on a thread of 64 KiB.
fn on_a_small_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(body)
        .expect("a thread")
        .join()
        .expect("the thread ends");
}

#[test]
fn nested_classes_use_no_rust_recursion() {
    on_a_small_stack(|| {
        // 10,000 nested classes, each level a union with a string, an intersection or a
        // subtraction, at a limit of 10,000: parsed, compiled, matched and dropped. Each level
        // holds what the one inside it holds, `xy` and `a`.
        let kinds = [("[\\q{xy}", "]"), ("[[\\q{xy}\\w]&&", "]"), ("[", "--b]")];
        let mut text = String::new();
        for level in 0..10_000 {
            text.push_str(kinds[level % 3].0);
        }
        text.push('a');
        for level in (0..10_000).rev() {
            text.push_str(kinds[level % 3].1);
        }
        let tree = parse_pattern(&units(&text), flags("v"), 10_000, &mut Unlimited)
            .expect("10,000 nested classes at a limit of 10,000");
        let program = compile_pattern(&tree, &mut Unlimited).expect("the program");
        drop(tree);
        for (subject, expected) in [("a", "0..1"), ("b", "no"), ("xy", "0..2"), ("x", "no")] {
            assert_eq!(answer(&program, subject), expected, "{subject}");
        }
        let plain = units(&format!("{}a{}", "[".repeat(10_000), "]".repeat(10_000)));
        let tree = parse_pattern(&plain, flags("vi"), 10_000, &mut Unlimited).expect("a tree");
        let program = compile_pattern(&tree, &mut Unlimited).expect("the program");
        assert_eq!(answer(&program, "A"), "0..1");
    });
}

/// A budget that records what it is asked: it refuses a charge that would hold more than
/// `limit`, and counts the steps it grants and not returned.
#[derive(Debug, Default)]
struct Recording {
    charged: usize,
    credited: usize,
    high: usize,
    limit: Option<usize>,
    steps: u64,
}

impl Recording {
    fn held(&self) -> usize {
        self.charged - self.credited
    }
}

impl Budget for Recording {
    fn charge(&mut self, bytes: usize) -> Result<(), Stop> {
        assert!(bytes > 0, "a charge of nothing");
        let held = self.held() + bytes;
        if self.limit.is_some_and(|limit| held > limit) {
            return Err(Stop::MemoryBudget);
        }
        self.charged += bytes;
        self.high = self.high.max(held);
        Ok(())
    }

    fn credit(&mut self, bytes: usize) {
        self.credited += bytes;
        assert!(self.credited <= self.charged, "credited past the charges");
    }

    fn grant(&mut self, asked: u32) -> Result<u32, Stop> {
        let granted = asked.clamp(1, MAX_GRANT);
        self.steps += u64::from(granted);
        Ok(granted)
    }

    fn refund(&mut self, steps: u32) {
        self.steps -= u64::from(steps);
    }

    fn poll(&mut self) -> Result<(), Stop> {
        Ok(())
    }
}

#[test]
fn a_class_set_past_the_memory_budget_stops_the_compile() {
    // A class whose own content is large: 2,000 strings of `\q{...}`. Its program is linear in
    // them; the high mark of its compile is measured here, and half of it is refused.
    let strings: Vec<String> = (0..2000).map(|i| format!("s{i:04}")).collect();
    let pattern = format!("[\\q{{{}}}--\\q{{s0001}}]", strings.join("|"));
    let tree = parse(&pattern, "v").expect("the tree");
    let mut probe = Recording::default();
    let program = compile_pattern(&tree, &mut probe).expect("the program");
    let high = probe.high;
    assert!(high > 2000 * 5 * 4, "{high} bytes at the high mark");
    assert_eq!(probe.held(), program.heap_bytes());
    assert_eq!(answer(&program, "s1999"), "0..5");
    assert_eq!(answer(&program, "s0001"), "no");
    program.release(&mut probe);
    assert_eq!(probe.held(), 0);
    let mut budget = Recording {
        limit: Some(high / 2),
        ..Recording::default()
    };
    assert_eq!(
        compile_pattern(&tree, &mut budget).err(),
        Some(CompileError::Stopped(Stop::MemoryBudget))
    );
    assert_eq!(budget.held(), 0, "nothing held after the stop");
    // The same pattern with the budget raised to the high mark compiles.
    let mut budget = Recording {
        limit: Some(high),
        ..Recording::default()
    };
    let program = compile_pattern(&tree, &mut budget).expect("the program at the high mark");
    program.release(&mut budget);
    assert_eq!(budget.held(), 0);
}

/// The steps of the search of `/pattern/f` in `subject`, which must match it whole.
fn steps_of(pattern: &str, f: &str, subject: &str) -> u64 {
    let program = program(pattern, f);
    let mut budget = Recording::default();
    let subject = units(subject);
    let found = search(&program, Subject::Utf16(&subject), 0, &mut budget)
        .expect("no stop")
        .expect("a match");
    assert_eq!(
        found.captures.get(0),
        Some(0..subject.len()),
        "/{pattern}/{f}"
    );
    drop(found);
    assert_eq!(budget.held(), 0, "/{pattern}/{f}: every byte credited");
    budget.steps
}

#[test]
fn a_class_under_v_charges_one_step_per_operand() {
    // `^X*$` on 100 ASCII letters runs the class X 101 times (the last at the end, which fails).
    // A class under `v` charges one step per operand each time, its instruction's step the
    // first: an operand is its own ranges, `\s`, a property escape or a string, each a bounded
    // amount of work, and a union, an intersection, a subtraction or a complement takes none.
    let subject = &"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(2)[..100];
    let runs = 101u64;
    let one = steps_of("^\\w*$", "v", subject);
    assert_eq!(steps_of("^\\w*$", "u", subject), one);
    assert_eq!(steps_of("^\\p{L}*$", "v", subject), one, "\\p{{L}}");
    assert_eq!(steps_of("^[\\p{L}]*$", "v", subject), one, "[\\p{{L}}]");
    assert_eq!(steps_of("^[a-zA-Z]*$", "v", subject), one, "[a-zA-Z]");
    assert_eq!(steps_of("^[^\\P{L}]*$", "v", subject), one, "[^\\P{{L}}]");
    let operands = [
        "\\p{N}",
        "[\\p{P}]",
        "\\p{S}",
        "[^\\p{Z}]",
        "\\p{M}",
        "\\P{Any}",
        "\\p{sc=Greek}",
        "\\p{scx=Cyrl}",
        "\\p{Lt}",
        "[\\p{Lm}]",
        "\\p{Lo}",
        "\\p{Emoji_Presentation}",
        "\\p{Cn}",
        "\\p{Co}",
        "\\p{Cs}",
    ];
    for n in [4usize, 16] {
        let mut union: String = operands.iter().take(n - 1).copied().collect();
        union.push_str("\\p{L}");
        let steps = steps_of(&format!("^[{union}]*$"), "v", subject);
        assert_eq!(
            steps,
            one + (n as u64 - 1) * runs,
            "a union of {n} operands: {steps} steps, one operand {one}"
        );
        let intersection = vec!["\\p{Any}"; n - 1].join("&&");
        let steps = steps_of(&format!("^[{intersection}&&\\p{{L}}]*$"), "v", subject);
        assert_eq!(
            steps,
            one + (n as u64 - 1) * runs,
            "an intersection of {n} operands: {steps} steps"
        );
    }
}

#[test]
fn a_class_with_strings_is_charged_per_try_and_per_candidate() {
    // Against "a": `[a-z]` is its one operand. `[\q{xy|xz}a-z]` has three operands; its two
    // strings are tried, each charged one step per code point (2 + 2), and the one candidate
    // found, the character, is evaluated over its three operands (the instruction's step the
    // first): 1 + 4 + 3 - 1 = 7 steps for the class, 6 more than `[a-z]`.
    let base = steps_of("^[a-z]$", "v", "a");
    assert_eq!(steps_of("^[\\q{xy|xz}a-z]$", "v", "a") - base, 6);
    // `\p{RGI_Emoji_Flag_Sequence}` (its longest sequence 2 code points) against a flag: one
    // try of a sequence of two code points (1 step), two candidates found (the sequence and the
    // character), the first evaluated over its one operand: 1 + 1 + 1 - 1 = 2 steps, the steps
    // of two classes of one character each.
    let flag = "\u{1F1E9}\u{1F1EA}";
    let base = steps_of("^[\\u{1F1E9}][\\u{1F1EA}]$", "v", flag);
    let class = steps_of("^\\p{RGI_Emoji_Flag_Sequence}$", "v", flag);
    assert_eq!(class, base, "{class} and {base} steps");
    // `\p{RGI_Emoji}` joins six tables, the longest of its sequences 10 code points: 9 tries.
    let one = steps_of("^[\\u{231A}]$", "v", "\u{231A}");
    assert_eq!(steps_of("^\\p{RGI_Emoji}$", "v", "\u{231A}"), one + 9);
}

//! The property escapes `\p{...}` and `\P{...}` under `u` (`#sec-patterns`, CharacterClassEscape
//! with [+UnicodeMode]; `#sec-compiletocharset`; `#sec-runtime-semantics-unicodematchproperty-p`,
//! `#sec-runtime-semantics-unicodematchpropertyvalue-p-v`) read through the crate's public
//! interface: each escape matches the code points `swell-unicode` gives its property, its
//! complement the rest, in a class and outside one; the early errors of
//! `#sec-patterns-static-semantics-early-errors`; `\p` without `u` as the identity escape of
//! B.1.2; and a long subject of one property against the budget.
#![cfg(test)]

use swell_regexp::{
    Budget, Flags, MAX_GRANT, ParseError, Program, Stop, Subject, Unlimited, compile_pattern,
    match_at, parse_pattern, search,
};
use swell_unicode::{Property, lone_property, property_value};

#[cfg(test)]
fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[cfg(test)]
fn parse(pattern: &str, flags: &str) -> Result<swell_regexp::Pattern, ParseError> {
    let flags = Flags::parse(&units(flags)).expect("the flags");
    parse_pattern(&units(pattern), flags, 1000, &mut Unlimited)
}

#[cfg(test)]
fn program(pattern: &str, flags: &str) -> Program {
    let tree = parse(pattern, flags).unwrap_or_else(|e| panic!("/{pattern}/{flags}: {e:?}"));
    compile_pattern(&tree, &mut Unlimited).unwrap_or_else(|e| panic!("/{pattern}/{flags}: {e:?}"))
}

/// The match of `program` in `subject` from 0: `start..end` of group 0, or `no`.
#[cfg(test)]
fn answer(program: &Program, subject: &[u16]) -> String {
    match search(program, Subject::Utf16(subject), 0, &mut Unlimited) {
        Ok(Some(found)) => match found.captures.get(0) {
            Some(range) => format!("{}..{}", range.start, range.end),
            None => "unset".to_string(),
        },
        Ok(None) => "no".to_string(),
        Err(stop) => format!("stop {stop:?}"),
    }
}

/// The UTF-16 code units of the code point `c`; a surrogate is its one code unit.
#[cfg(test)]
fn encode(c: u32) -> Vec<u16> {
    match char::from_u32(c) {
        Some(ch) => {
            let mut buffer = [0u16; 2];
            ch.encode_utf16(&mut buffer).to_vec()
        }
        None => vec![u16::try_from(c).expect("a surrogate")],
    }
}

/// The escapes the samples below take, each with the property `swell-unicode` gives it.
#[cfg(test)]
fn escapes() -> Vec<(&'static str, Property)> {
    let lone = |name: &str| lone_property(&units(name)).unwrap_or_else(|| panic!("{name}"));
    let valued = |name: &str, value: &str| {
        property_value(&units(name), &units(value)).unwrap_or_else(|| panic!("{name}={value}"))
    };
    vec![
        ("L", lone("L")),
        ("Lu", lone("Lu")),
        ("gc=Nd", valued("gc", "Nd")),
        ("General_Category=Cn", valued("General_Category", "Cn")),
        ("Script=Greek", valued("Script", "Greek")),
        ("scx=Grek", valued("scx", "Grek")),
        ("sc=Zzzz", valued("sc", "Zzzz")),
        ("Alphabetic", lone("Alphabetic")),
        ("ID_Start", lone("ID_Start")),
        ("Cased", lone("Cased")),
        ("Emoji_Presentation", lone("Emoji_Presentation")),
        ("Assigned", lone("Assigned")),
        ("ASCII", lone("ASCII")),
        ("Any", lone("Any")),
    ]
}

/// The code points a sample of `property` tests: the first and the last of every 40th range and
/// the code points beside them.
#[cfg(test)]
fn samples(property: Property) -> Vec<u32> {
    let mut points = vec![0, 0x41, 0xD800, 0xDFFF, 0x10FFFF];
    for (first, last) in property.ranges().step_by(40) {
        points.extend([first, last, last.saturating_add(1)]);
        if let Some(before) = first.checked_sub(1) {
            points.push(before);
        }
    }
    points.retain(|&c| c <= 0x10FFFF);
    points
}

#[test]
fn property_escapes_match_their_ranges_with_u() {
    for (escape, property) in escapes() {
        let alone = program(&format!("^\\p{{{escape}}}$"), "u");
        let in_class = program(&format!("^[\\p{{{escape}}}]$"), "u");
        for c in samples(property) {
            let subject = encode(c);
            let expected = if property.contains(c) {
                format!("0..{}", subject.len())
            } else {
                "no".to_string()
            };
            assert_eq!(
                answer(&alone, &subject),
                expected,
                "\\p{{{escape}}} U+{c:04X}"
            );
            assert_eq!(
                answer(&in_class, &subject),
                expected,
                "[\\p{{{escape}}}] U+{c:04X}"
            );
        }
    }
    for (pattern, subject, expected) in [
        ("\\p{Lu}", units("aBc"), "1..2"),
        ("\\p{L}+", units("ab1"), "0..2"),
        ("\\p{L}", units("1\u{1D49C}"), "1..3"),
        ("\\p{Cs}", vec![0x61, 0xD800], "1..2"),
        ("\\p{Cs}", vec![0xD835, 0xDC9C], "no"),
        ("(?<=\\p{Lu})b", units("Ab"), "1..2"),
        ("(?<!\\p{Lu})b", units("Ab"), "no"),
        ("[\\p{Nd}x]+", units("-x12x-"), "1..5"),
        ("\\p{Script_Extensions=Latin}", units("\u{B7}"), "0..1"),
        ("\\p{Script=Latin}", units("\u{B7}"), "no"),
        ("\\p{ID_Continue}", units("\u{200C}"), "0..1"),
    ] {
        let found = answer(&program(pattern, "u"), &subject);
        assert_eq!(found, expected, "/{pattern}/u on {subject:X?}");
    }
}

#[test]
fn a_complement_escape_matches_the_rest() {
    for (escape, property) in escapes() {
        let complement = program(&format!("^\\P{{{escape}}}$"), "u");
        let negated = program(&format!("^[^\\p{{{escape}}}]$"), "u");
        let twice = program(&format!("^[^\\P{{{escape}}}]$"), "u");
        for c in samples(property) {
            let subject = encode(c);
            let whole = format!("0..{}", subject.len());
            let (inside, outside) = if property.contains(c) {
                (whole.as_str(), "no")
            } else {
                ("no", whole.as_str())
            };
            assert_eq!(
                answer(&complement, &subject),
                outside,
                "\\P{{{escape}}} U+{c:04X}"
            );
            assert_eq!(
                answer(&negated, &subject),
                outside,
                "[^\\p{{{escape}}}] U+{c:04X}"
            );
            assert_eq!(
                answer(&twice, &subject),
                inside,
                "[^\\P{{{escape}}}] U+{c:04X}"
            );
        }
    }
    for (pattern, subject, expected) in [
        ("\\P{Lu}", units("Ab"), "1..2"),
        ("[\\P{L}\\p{L}]", units("a"), "0..1"),
        ("[\\P{L}\\p{L}]", units("!"), "0..1"),
        ("[\\P{L}\\p{L}]", units("\u{1D49C}"), "0..2"),
        ("\\P{Any}", units("abc"), "no"),
        ("[^\\P{Any}]", units("abc"), "0..1"),
        ("[\\P{ASCII}a-c]", units("x\u{E9}"), "1..2"),
        ("[\\P{ASCII}a-c]", units("xb"), "1..2"),
        ("[^\\P{ASCII}a-c]", units("ax"), "1..2"),
        ("\\P{L}+", units("ab12c"), "2..4"),
    ] {
        let found = answer(&program(pattern, "u"), &subject);
        assert_eq!(found, expected, "/{pattern}/u on {subject:X?}");
    }
}

#[test]
fn property_escapes_are_early_errors_where_ecma_262_says() {
    // Each pattern is an early error under `u`, named at the offset of the escape's `\` (of the
    // range's first atom for a range), and is not a refusal.
    for (pattern, offset) in [
        ("\\p{Foo}", 0),
        ("a\\P{Foo}", 1),
        ("\\p{ascii}", 0),
        ("\\p{lu}", 0),
        ("\\p{IsLu}", 0),
        ("\\p{Lowercase_letter}", 0),
        ("\\p{Script}", 0),
        ("\\P{General_Category}", 0),
        ("\\p{Script_Extensions}", 0),
        ("\\p{Greek}", 0),
        ("\\p{ASCII=Yes}", 0),
        ("\\p{Alphabetic=T}", 0),
        ("\\p{Script=Foo}", 0),
        ("\\p{Script=Lu}", 0),
        ("\\p{gc=Greek}", 0),
        ("\\p{Block=Basic_Latin}", 0),
        ("\\p{Line_Break=Alphabetic}", 0),
        ("\\p{WSpace}", 0),
        ("\\p{IDS_Unary_Operator}", 0),
        ("\\p{RGI_Emoji}", 0),
        ("\\p{Basic_Emoji}", 0),
        ("\\p{}", 0),
        ("\\p{L", 0),
        ("\\p{gc=L", 0),
        ("\\p", 0),
        ("\\pL", 0),
        ("\\P", 0),
        ("\\p{=L}", 0),
        ("\\p{gc=}", 0),
        ("\\p{gc=Lu=Lu}", 0),
        ("\\p{ Lu}", 0),
        ("\\p{Lu }", 0),
        ("\\p{^Lu}", 0),
        ("\\p{L-u}", 0),
        ("\\p{gc:Lu}", 0),
        ("[\\p{Foo}]", 1),
        ("[\\p{L}-z]", 1),
        ("[a-\\p{L}]", 1),
        ("[\\p{L}-\\p{N}]", 1),
    ] {
        match parse(pattern, "u") {
            Err(ParseError::Pattern(e)) => {
                assert!(!e.is_refusal(), "/{pattern}/u: {}", e.message());
                assert_eq!(e.offset, offset, "/{pattern}/u: {}", e.message());
                assert!(
                    !e.message().contains("not supported"),
                    "/{pattern}/u: {}",
                    e.message()
                );
            }
            other => panic!("/{pattern}/u: {other:?}"),
        }
    }
    for pattern in [
        "\\p{L}",
        "\\P{L}",
        "\\p{gc=L}",
        "\\p{General_Category=Letter}",
        "\\p{sc=Grek}",
        "\\P{Script_Extensions=Inherited}",
        "\\p{scx=Qaai}",
        "\\p{Any}",
        "\\p{ASCII}",
        "\\p{Assigned}",
        "\\p{Lowercase}",
        "\\p{Lowercase_Letter}",
        "\\p{space}",
        "\\p{Script=Katakana_Or_Hiragana}",
        "\\p{Script=Unknown}",
        "[\\p{L}\\P{N}a-z\\d]",
        "[^\\p{Lu}]",
        "\\p{Lu}{2,3}",
    ] {
        assert!(parse(pattern, "u").is_ok(), "/{pattern}/u");
    }
}

#[test]
fn without_u_p_is_an_identity_escape() {
    for (pattern, subject, expected) in [
        ("\\p{L}", "p{L}", "0..4"),
        ("\\p{L}", "a", "no"),
        ("^\\P$", "P", "0..1"),
        ("[\\p]", "p", "0..1"),
        ("[\\P{]+", "{P}", "0..2"),
        ("\\p{2}", "xpp", "1..3"),
        ("\\pL", "pL", "0..2"),
    ] {
        let found = answer(&program(pattern, ""), &units(subject));
        assert_eq!(found, expected, "/{pattern}/ on {subject:?}");
    }
}

/// A budget that counts the steps it grants and not returned, and the bytes held.
#[cfg(test)]
#[derive(Debug, Default)]
struct Counting {
    steps: u64,
    held: usize,
}

#[cfg(test)]
impl Budget for Counting {
    fn charge(&mut self, bytes: usize) -> Result<(), Stop> {
        self.held += bytes;
        Ok(())
    }

    fn credit(&mut self, bytes: usize) {
        self.held -= bytes;
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

/// The steps of the search of `/pattern/u` in `subject`, which must match it whole.
#[cfg(test)]
fn steps_of(pattern: &str, subject: &[u16]) -> u64 {
    let program = program(pattern, "u");
    let mut budget = Counting::default();
    let found = search(&program, Subject::Utf16(subject), 0, &mut budget)
        .expect("no stop")
        .expect("a match");
    assert_eq!(
        found.captures.get(0),
        Some(0..subject.len()),
        "/{pattern}/u"
    );
    drop(found);
    assert_eq!(budget.held, 0, "/{pattern}/u: every byte credited");
    budget.steps
}

#[test]
fn a_class_charges_one_step_per_property_escape() {
    // `^X*$` on 100 ASCII letters runs the class X 101 times (the last at the end, which fails).
    // A class of n property escapes charges n steps each time (decision 7, "What one step of the
    // matcher is": a property is a bounded number of binary searches, so no number of the
    // pattern multiplies the work of one step); one escape, in a class or outside one, charges
    // the one step of its instruction, as `\w` does.
    let subject = units(&"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(2)[..100]);
    let runs = 101u64;
    let one = steps_of("^\\w*$", &subject);
    assert_eq!(steps_of("^\\p{L}*$", &subject), one, "\\p{{L}}");
    assert_eq!(steps_of("^\\P{N}*$", &subject), one, "\\P{{N}}");
    assert_eq!(steps_of("^[\\p{L}]*$", &subject), one, "[\\p{{L}}]");
    assert_eq!(
        steps_of("^[a-z\\p{L}\\d]*$", &subject),
        one,
        "[a-z\\p{{L}}\\d]"
    );
    let escapes = [
        "\\p{N}",
        "\\p{P}",
        "\\p{S}",
        "\\p{Z}",
        "\\p{M}",
        "\\P{Any}",
        "\\p{sc=Greek}",
        "\\p{scx=Cyrl}",
        "\\p{Lt}",
        "\\p{Lm}",
        "\\p{Lo}",
        "\\p{Emoji_Presentation}",
        "\\p{Cn}",
        "\\p{Co}",
        "\\p{Cs}",
    ];
    for n in [4usize, 16] {
        let mut class: String = escapes.iter().take(n - 1).copied().collect();
        class.push_str("\\p{L}");
        let steps = steps_of(&format!("^[{class}]*$"), &subject);
        assert_eq!(
            steps,
            one + (n as u64 - 1) * runs,
            "a class of {n} property escapes: {steps} steps, one escape {one}"
        );
    }
}

/// The runs of the code points `points` that `holds` answers the same for, in order: each its
/// start and end in code units of their UTF-16 form, and the answer.
#[cfg(test)]
fn runs_of(points: &[u32], holds: impl Fn(u32) -> bool) -> Vec<(usize, usize, bool)> {
    let mut runs: Vec<(usize, usize, bool)> = Vec::new();
    let mut at = 0usize;
    for &c in points {
        let end = at + encode(c).len();
        let held = holds(c);
        match runs.last_mut() {
            Some(run) if run.2 == held => run.1 = end,
            _ => runs.push((at, end, held)),
        }
        at = end;
    }
    runs
}

/// The runs `program`, `(X+)|(Y+)`, finds in `subject` from 0 to its end, one search each, each
/// with `true` when group 1 took it.
#[cfg(test)]
fn runs_found(program: &Program, subject: &[u16]) -> Vec<(usize, usize, bool)> {
    let mut runs = Vec::new();
    let mut from = 0usize;
    while from < subject.len() {
        let found = search(program, Subject::Utf16(subject), from, &mut Unlimited)
            .expect("no stop")
            .expect("a run");
        let whole = found.captures.get(0).expect("group 0");
        assert_eq!(whole.start, from, "a run starts where the last one ended");
        runs.push((whole.start, whole.end, found.captures.get(1).is_some()));
        from = whole.end;
    }
    runs
}

#[test]
fn a_class_answers_each_character_of_a_long_subject_as_its_property_does() {
    // A subject read up, then down, then in jumps: each run `(\p{X}+)|(\P{X}+)` finds ends where
    // the property's answer changes, each of the pattern's two classes answering for itself,
    // whatever the class answered for the character before.
    let mut points: Vec<u32> = (0..0x3000).collect();
    points.extend(0x1F000..0x1F700);
    points.extend((0x2F00..0x3100).rev());
    points.extend([
        0xD800, 0x41, 0xDC00, 0x10FFFF, 0, 0x1F600, 0x0378, 0x10FFFE, 0x61, 0xE0001,
    ]);
    let subject: Vec<u16> = points.iter().flat_map(|&c| encode(c)).collect();
    for (escape, property) in escapes() {
        let program = program(&format!("(\\p{{{escape}}}+)|(\\P{{{escape}}}+)"), "u");
        assert_eq!(
            runs_found(&program, &subject),
            runs_of(&points, |c| property.contains(c)),
            "\\p{{{escape}}}"
        );
    }
    let digit = lone_property(&units("Nd")).expect("Nd");
    let mark = lone_property(&units("M")).expect("M");
    let class = |c: u32| {
        (0x61..=0x6B).contains(&c)
            || digit.contains(c)
            || (0x1F600..=0x1F64F).contains(&c)
            || !mark.contains(c)
    };
    let items = "a-k\\p{Nd}\\u{1F600}-\\u{1F64F}\\P{M}";
    let program = program(&format!("([{items}]+)|([^{items}]+)"), "u");
    assert_eq!(
        runs_found(&program, &subject),
        runs_of(&points, class),
        "a class of ranges and escapes"
    );
}

#[test]
fn a_long_subject_of_one_property_matches_within_the_budget() {
    // Every code point of `L`, 141,000 or so, against `^\p{L}+$`: one match, a bounded number of
    // steps per code point (the loop's head, the class, the loop's tail), and every byte
    // credited.
    let letters = lone_property(&units("L")).expect("L");
    let mut subject: Vec<u16> = Vec::new();
    let mut points = 0u64;
    for (first, last) in letters.ranges() {
        for c in first..=last {
            subject.extend(encode(c));
            points += 1;
        }
    }
    assert!(points > 100_000, "{points} code points of L");
    let program = program("^\\p{L}+$", "u");
    let mut budget = Counting::default();
    let found = search(&program, Subject::Utf16(&subject), 0, &mut budget)
        .expect("no stop")
        .expect("a match");
    assert_eq!(found.captures.get(0), Some(0..subject.len()));
    drop(found);
    assert_eq!(budget.held, 0, "every byte credited");
    assert!(
        budget.steps <= 4 * points + 16,
        "{} steps for {points} code points",
        budget.steps
    );
}

// The class memo of the matcher (`ClassSet::contains_around`) against an oracle without it: a
// search of `^(?:C)$` on the one character alone (a machine of its own, so no earlier answer),
// over subjects read up, down, in a linear congruential order and back and forth, forwards,
// inside a lookbehind and with backtracking.

/// A linear congruential sequence (the constants of Numerical Recipes), no dependency.
#[cfg(test)]
fn lcg(seed: u32) -> impl FnMut() -> u32 {
    let mut x = seed;
    move || {
        x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        x
    }
}

/// U+0000, U+10FFFF, the ends of ASCII and of the surrogates, and for up to 50 ranges spread
/// over the list of each of `properties` (its first and last range among them) the code points
/// one before, at and one after each end of the range; sorted, each once.
#[cfg(test)]
fn edges(properties: &[Property]) -> Vec<u32> {
    let mut points = vec![
        0, 1, 0x7F, 0x80, 0xD7FF, 0xD800, 0xDBFF, 0xDC00, 0xDFFF, 0xE000, 0xFFFF, 0x10000,
        0x10FFFE, 0x10FFFF,
    ];
    for property in properties {
        let ranges: Vec<(u32, u32)> = property.ranges().collect();
        let step = (ranges.len() / 50).max(1);
        for &(first, last) in ranges.iter().step_by(step).chain(ranges.last()) {
            for c in [
                first.wrapping_sub(1),
                first,
                first + 1,
                last.wrapping_sub(1),
                last,
                last + 1,
            ] {
                if c <= 0x10FFFF {
                    points.push(c);
                }
            }
        }
    }
    points.sort_unstable();
    points.dedup();
    points
}

/// `points` with a `-` put between a high and a low surrogate that would meet, so that each
/// code point stays one character of the subject under `u`.
#[cfg(test)]
fn apart(points: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::with_capacity(points.len());
    for &c in points {
        if (0xDC00..=0xDFFF).contains(&c)
            && out.last().is_some_and(|p| (0xD800..=0xDBFF).contains(p))
        {
            out.push(0x2D);
        }
        out.push(c);
    }
    out
}

/// The orders a subject is read in: up, down, a linear congruential shuffle, and back and forth
/// (of each three points in a row the middle one, the last, then the first).
#[cfg(test)]
fn orders(points: &[u32]) -> Vec<(&'static str, Vec<u32>)> {
    let up = points.to_vec();
    let down: Vec<u32> = points.iter().rev().copied().collect();
    let mut shuffled = points.to_vec();
    let mut next = lcg(0x5EED);
    for i in (1..shuffled.len()).rev() {
        let j = (next() as usize) % (i + 1);
        shuffled.swap(i, j);
    }
    let mut zigzag = Vec::new();
    for window in points.windows(3) {
        zigzag.extend([window[1], window[2], window[0]]);
    }
    [
        ("up", up),
        ("down", down),
        ("shuffled", shuffled),
        ("zigzag", zigzag),
    ]
    .into_iter()
    .map(|(name, order)| (name, apart(&order)))
    .collect()
}

/// The classes the cases take, each with its complement and its flags: a negated class and its
/// twin, `\P` inside a negated class, `i` with `u` (the answer for the folded character), own
/// ranges beside property escapes (the gap between two ranges of the class), `\s` and `\w`,
/// `\P{Any}`, and an escape outside a class under `v`.
#[cfg(test)]
const CLASSES: &[(&str, &str, &str)] = &[
    ("[\\p{L}]", "[^\\p{L}]", "u"),
    ("[^\\P{L}]", "[\\P{L}]", "u"),
    ("\\P{Script=Greek}", "\\p{Script=Greek}", "u"),
    ("[\\p{Lu}]", "[^\\p{Lu}]", "iu"),
    ("[k-q\\p{Lu}]", "[^k-q\\p{Lu}]", "iu"),
    (
        "[a-zA-Z\\u{1F600}-\\u{1F64F}\\p{Nd}]",
        "[^a-zA-Z\\u{1F600}-\\u{1F64F}\\p{Nd}]",
        "u",
    ),
    (
        "[\\u0370-\\u03FF\\P{Assigned}\\p{ASCII}]",
        "[^\\u0370-\\u03FF\\P{Assigned}\\p{ASCII}]",
        "u",
    ),
    ("[\\s\\p{N}]", "[^\\s\\p{N}]", "u"),
    ("[\\w\\p{Cs}]", "[^\\w\\p{Cs}]", "iu"),
    ("[\\p{scx=Grek}\\p{Lu}]", "[^\\p{scx=Grek}\\p{Lu}]", "u"),
    ("[\\P{Any}x]", "[^\\P{Any}x]", "u"),
    ("[0-9\\p{Any}]", "[^0-9\\p{Any}]", "u"),
    ("\\p{L}", "\\P{L}", "v"),
];

/// The answer of `class` under `flags` for each of `points`, each asked alone; it checks that
/// `complement` answers the other way.
#[cfg(test)]
fn oracle(
    class: &str,
    complement: &str,
    flags: &str,
    points: &[u32],
) -> std::collections::BTreeMap<u32, bool> {
    let alone = program(&format!("^(?:{class})$"), flags);
    let other = program(&format!("^(?:{complement})$"), flags);
    let mut oracle = std::collections::BTreeMap::new();
    for &c in points {
        if oracle.contains_key(&c) {
            continue;
        }
        let subject = encode(c);
        let held = answer(&alone, &subject) != "no";
        assert_ne!(
            held,
            answer(&other, &subject) != "no",
            "/{class}/{flags} and /{complement}/{flags} at U+{c:04X}"
        );
        oracle.insert(c, held);
    }
    oracle
}

/// The edges of the cases: of `L`, `Script=Greek`, `Any`, `ASCII`, `Assigned`, `Nd` and `Lu`.
#[cfg(test)]
fn case_points() -> Vec<u32> {
    let lone = |name: &str| lone_property(&units(name)).unwrap_or_else(|| panic!("{name}"));
    let greek = property_value(&units("Script"), &units("Greek")).expect("Greek");
    let mut points = edges(&[
        lone("L"),
        greek,
        lone("Any"),
        lone("ASCII"),
        lone("Assigned"),
        lone("Nd"),
        lone("Lu"),
    ]);
    points.push(0x2D);
    points
}

#[test]
fn a_class_answers_as_a_search_of_the_character_alone_in_every_order() {
    // Each class of CLASSES as `(C+)|(D+)` over the edges read in four orders: the runs it
    // finds end where the oracle's answer changes.
    let points = case_points();
    let orders = orders(&points);
    for &(class, complement, flags) in CLASSES {
        let oracle = oracle(class, complement, flags, &points);
        let runs = program(&format!("({class}+)|({complement}+)"), flags);
        for (name, order) in &orders {
            let subject: Vec<u16> = order.iter().flat_map(|&c| encode(c)).collect();
            assert_eq!(
                runs_found(&runs, &subject),
                runs_of(order, |c| oracle[&c]),
                "/({class}+)|({complement}+)/{flags} read {name}"
            );
        }
    }
}

/// The code unit index of each point of `points` in its UTF-16 form, and the end.
#[cfg(test)]
fn offsets(points: &[u32]) -> Vec<usize> {
    let mut offsets = vec![0usize];
    for &c in points {
        let last = *offsets.last().expect("one");
        offsets.push(last + encode(c).len());
    }
    offsets
}

#[test]
fn a_class_in_a_lookbehind_answers_as_a_search_of_the_character_alone() {
    // `(?<=(C+))` tried at each character boundary of a subject: the lookbehind reads the class
    // backwards across the ranges' ends, and group 1 starts where the run of C that ends there
    // starts.
    let mut points: Vec<u32> = (0x0360..0x0400).collect();
    let mut shuffled = case_points();
    let mut next = lcg(0xB0B);
    for i in (1..shuffled.len()).rev() {
        let j = (next() as usize) % (i + 1);
        shuffled.swap(i, j);
    }
    points.extend(shuffled.iter().take(600));
    points.extend((0x1F5F0..0x1F660).rev());
    let points = apart(&points);
    let offsets = offsets(&points);
    let subject: Vec<u16> = points.iter().flat_map(|&c| encode(c)).collect();
    for &(class, complement, flags) in CLASSES {
        let oracle = oracle(class, complement, flags, &points);
        let behind = program(&format!("(?<=({class}+))"), flags);
        for k in 0..=points.len() {
            let mut j = k;
            while j > 0 && oracle[&points[j - 1]] {
                j -= 1;
            }
            let expected = (j < k).then(|| offsets[j]..offsets[k]);
            let found = match_at(
                &behind,
                Subject::Utf16(&subject),
                offsets[k],
                &mut Unlimited,
            )
            .expect("no stop")
            .map(|captures| captures.get(1).expect("group 1"));
            assert_eq!(
                found, expected,
                "/(?<=({class}+))/{flags} at {}",
                offsets[k]
            );
        }
    }
}

#[test]
fn a_class_asked_again_by_backtracking_answers_as_a_search_of_the_character_alone() {
    // `(C*)(C)(D)` and `(C+?)(D)` from the start of each run of C: the greedy loop reads the run
    // and the character after it, then backtracks one character and asks C again there; the lazy
    // one asks D and C in turn at every character.
    let points = apart(&orders(&case_points())[2].1);
    let offsets = offsets(&points);
    let subject: Vec<u16> = points.iter().flat_map(|&c| encode(c)).collect();
    for &(class, complement, flags) in CLASSES {
        let oracle = oracle(class, complement, flags, &points);
        let greedy = program(&format!("({class}*)({class})({complement})"), flags);
        let lazy = program(&format!("({class}+?)({complement})"), flags);
        let mut k = 0;
        while k < points.len() {
            if !oracle[&points[k]] {
                k += 1;
                continue;
            }
            let mut e = k;
            while e < points.len() && oracle[&points[e]] {
                e += 1;
            }
            let found = match_at(
                &greedy,
                Subject::Utf16(&subject),
                offsets[k],
                &mut Unlimited,
            )
            .expect("no stop")
            .map(|c| (c.get(1), c.get(2), c.get(3)));
            let lazy_found = match_at(&lazy, Subject::Utf16(&subject), offsets[k], &mut Unlimited)
                .expect("no stop")
                .map(|c| (c.get(1), c.get(2)));
            if e < points.len() {
                assert_eq!(
                    found,
                    Some((
                        Some(offsets[k]..offsets[e - 1]),
                        Some(offsets[e - 1]..offsets[e]),
                        Some(offsets[e]..offsets[e + 1]),
                    )),
                    "/({class}*)({class})({complement})/{flags} at {}",
                    offsets[k]
                );
                assert_eq!(
                    lazy_found,
                    Some((
                        Some(offsets[k]..offsets[e]),
                        Some(offsets[e]..offsets[e + 1])
                    )),
                    "/({class}+?)({complement})/{flags} at {}",
                    offsets[k]
                );
            } else {
                assert_eq!(
                    found, None,
                    "/({class}*)({class})({complement})/{flags} at the end"
                );
                assert_eq!(
                    lazy_found, None,
                    "/({class}+?)({complement})/{flags} at the end"
                );
            }
            k = e;
        }
    }
}

#[test]
fn two_classes_asked_in_turn_on_neighbouring_code_points_answer_for_themselves() {
    // `^(?:(C)(D))+$` and `^(?:(D)(C))+$` over pairs of neighbouring code points, one held by C
    // and the next not (or the other way): the two classes are asked in turn, each about a code
    // point inside the range the other just answered for.
    let points = case_points();
    for &(class, complement, flags) in CLASSES {
        let oracle = oracle(class, complement, flags, &points);
        let mut held_first: Vec<u32> = Vec::new();
        let mut other_first: Vec<u32> = Vec::new();
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            // No pair starts with a low surrogate or ends with a high one, so no two code units
            // of the subject join into one character.
            if b != a + 1 || (0xDC00..=0xDFFF).contains(&a) || (0xD800..=0xDBFF).contains(&b) {
                continue;
            }
            match (oracle[&a], oracle[&b]) {
                (true, false) => held_first.extend([a, b]),
                (false, true) => other_first.extend([a, b]),
                _ => {}
            }
        }
        for (pattern, list) in [
            (format!("^(?:({class})({complement}))+$"), &held_first),
            (format!("^(?:({complement})({class}))+$"), &other_first),
        ] {
            if list.is_empty() {
                continue;
            }
            let subject: Vec<u16> = list.iter().flat_map(|&c| encode(c)).collect();
            assert_eq!(
                answer(&program(&pattern, flags), &subject),
                format!("0..{}", subject.len()),
                "/{pattern}/{flags} over {} pairs",
                list.len() / 2
            );
        }
    }
}

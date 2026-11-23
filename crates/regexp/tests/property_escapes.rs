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
    parse_pattern, search,
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

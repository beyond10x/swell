//! Case-insensitive matching (`docs/design/engine.md`, decision 7, "Case folding"): the flag `i`
//! and the modifier `i` (`#sec-compilepattern`, `#sec-updatemodifiers`), with Canonicalize
//! (`#sec-runtime-semantics-canonicalize-ch`) by the simple case folding of `CaseFolding.txt`
//! under `u` and by the uppercase mapping without it; a character, a class with its ranges and
//! property escapes (`#sec-runtime-semantics-charactersetmatcher-abstract-operation`), a
//! backreference (`#sec-backreference-matcher`) and WordCharacters (`#sec-wordcharacters`)
//! under it.
//!
//! An expected answer is written as the captures in order, group 0 first, each `start..end` in
//! code units or `-` when unset, or `no` for no match.

use swell_regexp::{
    Budget, Flags, MAX_GRANT, ParseError, Program, Stop, Subject, Unlimited, canonicalize,
    compile_pattern, parse_pattern, search,
};
use swell_unicode::simple_case_fold;

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

/// The answer of a search of `/pattern/flags` in `subject` from 0, written as the module says.
#[cfg(test)]
fn answer(pattern: &str, flags: &str, subject: &str) -> String {
    let program = program(pattern, flags);
    match search(&program, Subject::Utf16(&units(subject)), 0, &mut Unlimited) {
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

/// Rows: pattern, flags, subject, expected answer.
#[cfg(test)]
fn check(rows: &[(&str, &str, &str, &str)]) {
    for &(pattern, flags, subject, expected) in rows {
        assert_eq!(
            answer(pattern, flags, subject),
            expected,
            "/{pattern}/{flags} in {subject:?}"
        );
    }
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

/// The steps a search of `/pattern/flags` in `subject` from 0 spends, and the range of its
/// match.
#[cfg(test)]
fn steps_of(pattern: &str, flags: &str, subject: &str) -> (u64, Option<std::ops::Range<usize>>) {
    let program = program(pattern, flags);
    let mut budget = Steps::default();
    let found = search(&program, Subject::Utf16(&units(subject)), 0, &mut budget)
        .unwrap_or_else(|stop| panic!("/{pattern}/{flags}: {stop:?}"));
    (
        budget.granted - budget.refunded,
        found.and_then(|f| f.captures.get(0)),
    )
}

#[test]
fn canonicalize_without_u_uses_the_uppercase_mapping_and_keeps_ascii_apart() {
    check(&[
        ("a", "i", "A", "0..1"),
        ("A", "i", "xa", "1..2"),
        ("abc", "i", "-AbC", "1..4"),
        ("s", "i", "S", "0..1"),
        // U+017F maps to S and U+212A to K: a code unit of 128 or above that maps below 128
        // is its own canonical value, so they match nothing of ASCII.
        ("\u{17f}", "i", "s", "no"),
        ("\u{17f}", "i", "S", "no"),
        ("s", "i", "\u{17f}", "no"),
        ("\u{212a}", "i", "k", "no"),
        ("\u{212a}", "i", "K", "no"),
        ("k", "i", "\u{212a}", "no"),
        ("\u{17f}", "i", "\u{17f}", "0..1"),
        ("\u{131}", "i", "I", "no"),
        ("i", "i", "\u{130}", "no"),
        // A mapping of several code units: ß maps to SS, U+1F80 to U+1F08 U+0399.
        ("\u{df}", "i", "\u{1e9e}", "no"),
        ("\u{1f80}", "i", "\u{1f88}", "no"),
        ("\u{df}", "i", "SS", "no"),
        // Beyond ASCII: sigma in its three forms, micro sign and mu, Latin-1.
        ("\u{3c3}", "i", "\u{3a3}", "0..1"),
        ("\u{3c3}", "i", "\u{3c2}", "0..1"),
        ("\u{3c2}", "i", "\u{3c3}", "0..1"),
        ("\u{b5}", "i", "\u{39c}", "0..1"),
        ("\u{b5}", "i", "\u{3bc}", "0..1"),
        ("\u{e9}", "i", "\u{c9}", "0..1"),
        ("\u{e5}", "i", "\u{212b}", "no"),
        // U+2126 OHM SIGN is its own uppercase: not omega without u.
        ("\u{3c9}", "i", "\u{2126}", "no"),
        // Code units: the two halves of a pair are compared one by one.
        ("\u{10400}", "i", "\u{10428}", "no"),
        ("\u{10400}", "i", "\u{10400}", "0..2"),
        // A character with no case, and a lookbehind.
        ("1", "i", "1", "0..1"),
        ("(?<=a)b", "i", "AB", "1..2"),
        ("(?<!a)b", "i", "AB", "no"),
    ]);
    // Without i nothing of this changes.
    check(&[("a", "", "A", "no"), ("\u{3c3}", "", "\u{3a3}", "no")]);
}

#[test]
fn canonicalize_with_u_uses_simple_case_folding() {
    check(&[
        ("a", "ui", "A", "0..1"),
        ("\u{17f}", "ui", "s", "0..1"),
        ("\u{17f}", "ui", "S", "0..1"),
        ("s", "ui", "\u{17f}", "0..1"),
        ("\u{212a}", "ui", "k", "0..1"),
        ("\u{212a}", "ui", "K", "0..1"),
        ("k", "ui", "\u{212a}", "0..1"),
        ("\u{df}", "ui", "\u{1e9e}", "0..1"),
        ("\u{1f80}", "ui", "\u{1f88}", "0..1"),
        ("\u{3c9}", "ui", "\u{2126}", "0..1"),
        ("\u{3a9}", "ui", "\u{2126}", "0..1"),
        ("\u{e5}", "ui", "\u{212b}", "0..1"),
        // A code point of a supplementary plane, read whole.
        ("\u{10400}", "ui", "\u{10428}", "0..2"),
        ("\u{10428}", "ui", "x\u{10400}", "1..3"),
        ("\\u{10400}", "ui", "\u{10428}", "0..2"),
        // No simple folding: the dotless i, the capital I with a dot, and ß against SS.
        ("\u{131}", "ui", "i", "no"),
        ("\u{131}", "ui", "I", "no"),
        ("\u{130}", "ui", "i", "no"),
        ("\u{df}", "ui", "SS", "no"),
        ("(?<=\u{17f})x", "ui", "SX", "1..2"),
    ]);
    // Without i nothing of this changes.
    check(&[
        ("\u{17f}", "u", "s", "no"),
        ("\u{10400}", "u", "\u{10428}", "no"),
    ]);
}

#[test]
fn a_class_matches_by_its_closure() {
    check(&[
        ("[a-z]", "i", "Q", "0..1"),
        ("[A-Z]", "i", "q", "0..1"),
        ("[a-z]+", "i", "-HeLLo-", "1..6"),
        ("[\u{e0}-\u{fe}]", "i", "\u{c0}", "0..1"),
        ("[\u{3c3}]", "i", "\u{3a3}", "0..1"),
        ("[\u{3c3}]", "i", "\u{3c2}", "0..1"),
        ("[\\u0430-\\u044f]", "i", "\u{410}", "0..1"),
        // Without u the members of 128 and above that map below 128 stay apart.
        ("[a-z]", "i", "\u{17f}", "no"),
        ("[a-z]", "i", "\u{212a}", "no"),
        ("[\\x80-\\uffff]", "i", "k", "no"),
        ("[\\x80-\\uffff]", "i", "s", "no"),
        // Under u a member of the class with the folding of the character is enough.
        ("[a-z]", "ui", "\u{17f}", "0..1"),
        ("[a-z]", "ui", "\u{212a}", "0..1"),
        ("[\\x80-\\uffff]", "ui", "k", "0..1"),
        ("[\\x80-\\uffff]", "ui", "S", "0..1"),
        ("[\\x80-\\uffff]", "ui", "a", "no"),
        ("[\\u{10400}-\\u{10410}]", "ui", "\u{10430}", "0..2"),
        ("[\\u{10400}-\\u{10410}]", "ui", "\u{10439}", "no"),
        ("[\\u{2126}]", "ui", "\u{3c9}", "0..1"),
        // An escape and \s in a class.
        ("[\\dA-F]+", "i", "0x1fA", "0..1"),
        ("[\\da-f]+", "i", "x1FAg", "1..4"),
        ("[\\s\\d]", "i", "a 1", "1..2"),
        ("[\\S]", "i", " a", "1..2"),
    ]);
    // A class under i charges one step, as without i: the try, the class, the end.
    assert_eq!(steps_of("[a-z]", "i", "Q"), (3, Some(0..1)));
    assert_eq!(steps_of("[a-z]", "", "q"), (3, Some(0..1)));
    assert_eq!(steps_of("[\\x80-\\uffff]", "ui", "k"), (3, Some(0..1)));
}

#[test]
fn a_negated_class_under_i() {
    check(&[
        ("[^a-z]", "i", "Q", "no"),
        ("[^a-z]", "i", "Q1", "1..2"),
        ("[^A-Z]", "i", "q", "no"),
        ("[^\u{3c3}]", "i", "\u{3a3}\u{3c2}x", "2..3"),
        // The inversion comes after the members: a character one of whose case equivalents is
        // a member is not in the negated class.
        ("[^\u{17f}]", "ui", "s", "no"),
        ("[^\u{17f}]", "ui", "S", "no"),
        ("[^\u{17f}]", "i", "s", "0..1"),
        ("[^\u{212a}]", "ui", "k", "no"),
        ("[^\u{212a}]", "i", "k", "0..1"),
        ("[^\\x80-\\uffff]", "ui", "k", "no"),
        ("[^\\x80-\\uffff]", "ui", "a", "0..1"),
        ("[^\\d]", "i", "1a", "1..2"),
        ("[^\\s]", "i", " A", "1..2"),
    ]);
}

#[test]
fn a_property_escape_under_i() {
    check(&[
        // Outside a class and in one: one case equivalent of the character that has the
        // property is enough.
        ("\\p{Lu}", "ui", "a", "0..1"),
        ("\\p{Lu}", "ui", "1", "no"),
        ("\\p{Ll}", "ui", "A", "0..1"),
        ("[\\p{Lu}]", "ui", "z", "0..1"),
        ("(?i:\\p{Lu})", "u", "a", "0..1"),
        ("\\p{Lu}", "u", "a", "no"),
        ("\\p{Lowercase}", "ui", "Q", "0..1"),
        // U+01C5, a titlecase letter, shares its folding with U+01C4, an uppercase one.
        ("\\p{Lu}", "ui", "\u{1c5}", "0..1"),
        // The Kelvin sign and the long s fold into ASCII.
        ("\\p{ASCII}", "ui", "\u{212a}", "0..1"),
        ("\\p{ASCII}", "ui", "\u{17f}", "0..1"),
        ("\\p{ASCII}", "u", "\u{212a}", "no"),
        // U+00B5 MICRO SIGN is of the script Common and folds to U+03BC, a Greek letter.
        ("\\p{Script=Greek}", "ui", "\u{b5}", "0..1"),
        ("\\p{Script=Greek}", "u", "\u{b5}", "no"),
        // \P is the complement before the case equivalents are read: A has the equivalent a,
        // which is not Lu, so \P{Lu} holds for A.
        ("\\P{Lu}", "ui", "A", "0..1"),
        ("\\P{Lu}", "ui", "a", "0..1"),
        ("\\P{Ll}", "ui", "a", "0..1"),
        ("[\\P{Lu}]", "ui", "A", "0..1"),
        // A negated class: the inversion after every item.
        ("[^\\p{Lu}]", "ui", "a", "no"),
        ("[^\\p{Lu}]", "ui", "aA1", "2..3"),
        ("[^\\P{Lu}]", "ui", "A", "no"),
        ("[^\\P{Lu}]", "ui", "a1", "no"),
        ("[^\\p{Lu}\\p{Nd}]", "ui", "a1-", "2..3"),
        ("[\\p{Nd}\\p{Lu}x]", "ui", "-b", "1..2"),
    ]);
    // One step per property escape of a class under i, as without it.
    let subject = "--------a";
    let (without, found) = steps_of("[\\p{Nd}\\p{Ll}\\p{Lo}]", "u", subject);
    assert_eq!(found, Some(8..9));
    let (with, found) = steps_of("[\\p{Nd}\\p{Lu}\\p{Lo}]", "ui", subject);
    assert_eq!(found, Some(8..9));
    assert_eq!(with, without);
    // Nine tries of one class each: the try, three steps of the class, and the end at the last.
    assert_eq!(with, 9 * (1 + 3) + 1);
}

#[test]
fn a_backreference_under_i_charges_a_step_per_code_unit() {
    check(&[
        ("^(abc)\\1$", "i", "abcABC", "0..6 0..3"),
        ("(a)\\1", "i", "aA", "0..2 0..1"),
        ("(\u{3c3})\\1\\1", "i", "\u{3c3}\u{3a3}\u{3c2}", "0..3 0..1"),
        ("(\u{17f})\\1", "i", "\u{17f}s", "no"),
        ("(\u{17f})\\1", "ui", "\u{17f}s", "0..2 0..1"),
        ("(\u{17f})\\1", "ui", "\u{17f}\u{212a}", "no"),
        ("(\u{10400})\\1", "ui", "\u{10400}\u{10428}", "0..4 0..2"),
        ("(\u{10400})\\1", "i", "\u{10400}\u{10428}", "no"),
        ("(?<n>x)\\k<n>", "i", "xX", "0..2 0..1"),
        // In a lookbehind the group is matched first, then the backreference before it.
        ("(?<=\\1(a))b", "i", "Aab", "2..3 1..2"),
        ("(?<=\\1(a))b", "", "Aab", "no"),
        // Without i the case is kept.
        ("(a)\\1", "", "aA", "no"),
    ]);
    // A backreference under i charges one step per code unit it compares, before the
    // comparison, beside the step of its instruction; the characters under i one step each,
    // as without it.
    let lower = "a".repeat(1000);
    let mixed = format!("{lower}{}", "A".repeat(1000));
    let (without_ref, _) = steps_of("(a{1000})", "i", &mixed);
    let (with_ref, whole) = steps_of("(a{1000})\\1", "i", &mixed);
    assert_eq!(whole, Some(0..2000));
    assert_eq!(with_ref - without_ref, 1 + 1000);
    let (case_kept, whole) = steps_of("(a{1000})\\1", "", &lower.repeat(2));
    assert_eq!(whole, Some(0..2000));
    assert_eq!(with_ref, case_kept);
    // Under u a pair is two code units and two steps.
    let (one, _) = steps_of("(\u{10400})", "ui", "\u{10400}\u{10428}");
    let (two, whole) = steps_of("(\u{10400})\\1", "ui", "\u{10400}\u{10428}");
    assert_eq!(whole, Some(0..4));
    assert_eq!(two - one, 1 + 2);
}

#[test]
fn the_modifier_i_sets_its_atoms_only() {
    check(&[
        ("(?i:a)b", "", "AB", "no"),
        ("(?i:a)b", "", "Ab", "0..2"),
        ("a(?-i:b)", "i", "AB", "no"),
        ("a(?-i:b)", "i", "Ab", "0..2"),
        ("(?i-:a)", "", "A", "0..1"),
        ("(?-i:a)", "i", "A", "no"),
        ("(?i:a(?-i:b)c)", "", "AbC", "0..3"),
        ("(?i:a(?-i:b)c)", "", "ABC", "no"),
        ("(?i:[a-z])", "", "Q", "0..1"),
        ("(?i:[a-z])[a-z]", "", "QQ", "no"),
        ("(a)(?i:\\1)", "", "aA", "0..2 0..1"),
        ("(a)\\1", "", "aA", "no"),
        // The group matched under i; the backreference outside it compares the case.
        ("(?i:(a))\\1", "", "Aa", "no"),
        ("(?i:(a))\\1", "", "AA", "0..2 0..1"),
        ("(?i:^a$)", "", "x\nA", "no"),
        ("(?im:^a$)", "m", "x\nA", "2..3"),
        ("(?is:a.)", "", "A\n", "0..2"),
        ("(?i:\u{17f})", "u", "S", "0..1"),
        ("(?i:\u{17f})", "", "S", "no"),
        ("(?i:\\p{Ll})", "u", "Q", "0..1"),
        ("(?<=(?i:a))b", "", "Ab", "1..2"),
    ]);
    // An early error comes first: the parse answers it before the compile step runs.
    let parse = |pattern: &str, flags: &str| {
        let flags = Flags::parse(&units(flags)).expect("the flags");
        parse_pattern(&units(pattern), flags, 1000, &mut Unlimited).map(|_| ())
    };
    match parse("(?i-i:a)", "") {
        Err(ParseError::Pattern(e)) => {
            assert_eq!(e.message(), "a modifier both added and removed");
        }
        other => panic!("(?i-i:a): {other:?}"),
    }
    assert!(matches!(parse("a(", "i"), Err(ParseError::Pattern(_))));
    // The flags of the program are those of the pattern; a modifier changes none of them.
    assert!(!program("(?i:a)", "").flags().ignore_case());
    assert!(program("a", "i").flags().ignore_case());
}

#[test]
fn word_characters_under_u_and_i() {
    check(&[
        // Under u and i WordCharacters has U+017F and U+212A, which fold into it.
        ("\\w", "ui", "\u{17f}", "0..1"),
        ("\\w", "ui", "\u{212a}", "0..1"),
        ("[\\w]", "ui", "\u{212a}", "0..1"),
        ("(?i:\\w)", "u", "\u{17f}", "0..1"),
        ("\\w", "u", "\u{17f}", "no"),
        ("\\w", "i", "\u{17f}", "no"),
        ("\\w", "i", "\u{212a}", "no"),
        // And \W is its complement: neither they nor what folds to them are in it.
        ("\\W", "ui", "\u{17f}", "no"),
        ("\\W", "ui", "\u{212a}", "no"),
        ("\\W", "ui", "S", "no"),
        ("\\W", "ui", "k", "no"),
        ("[\\W]", "ui", "s", "no"),
        ("[^\\w]", "ui", "\u{17f}", "no"),
        ("\\W", "ui", "!", "0..1"),
        ("\\W", "i", "\u{17f}", "0..1"),
        ("\\W", "u", "\u{17f}", "0..1"),
        // \b and \B read the same WordCharacters.
        ("\\b", "ui", "\u{17f}!", "0..0"),
        ("\\b", "u", "\u{17f}!", "no"),
        ("\\b", "i", "\u{17f}!", "no"),
        ("(?i:\\b)", "u", "\u{212a}!", "0..0"),
        ("\\B", "ui", "\u{212a}", "no"),
        ("\\B", "u", "\u{212a}", "0..0"),
        ("x\\b", "ui", "x\u{17f}x!", "2..3"),
    ]);
}

/// The matcher finds the characters of one canonical value without `u` among the code units
/// of one simple case folding (`canonicalize.rs`): that holds when every code unit has the
/// simple case folding of its canonical value, checked here for all 65,536.
#[test]
fn canonicalize_without_u_stays_inside_a_folding_group() {
    for unit in 0..=0xFFFF_u32 {
        let canonical = canonicalize(unit, false);
        assert!(canonical <= 0xFFFF, "{unit:04X}");
        assert_eq!(
            simple_case_fold(canonical),
            simple_case_fold(unit),
            "{unit:04X} and its canonical value {canonical:04X}"
        );
    }
    // Canonicalize under u is the simple case folding; without u the rows of the clause.
    for (c, unicode, canonical) in [
        (0x61, false, 0x41),
        (0x41, false, 0x41),
        (0x17F, false, 0x17F),
        (0x212A, false, 0x212A),
        (0xDF, false, 0xDF),
        (0x3C2, false, 0x3A3),
        (0xB5, false, 0x39C),
        (0x131, false, 0x131),
        (0x1F80, false, 0x1F80),
        (0xD801, false, 0xD801),
        (0x41, true, 0x61),
        (0x17F, true, 0x73),
        (0x212A, true, 0x6B),
        (0x1E9E, true, 0xDF),
        (0x10400, true, 0x10428),
    ] {
        assert_eq!(canonicalize(c, unicode), canonical, "{c:04X} {unicode}");
    }
}

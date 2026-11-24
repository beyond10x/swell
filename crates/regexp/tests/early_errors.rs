//! The early errors of a pattern (`#sec-patterns-static-semantics-early-errors` and
//! `#sec-patterns-static-semantics-early-errors-annexb`), each with a pattern that hits it and
//! one beside it that does not, under `v` too where it reads as under `u`; the flags of
//! `#sec-regexpinitialize`; and that the parser refuses no pattern of slice 5.
//!
//! Not held here: CountLeftCapturingParensWithin ≥ 2^32 - 1 (a pattern of more than 2^32
//! parentheses), the early errors of property escapes, which `property_escapes.rs` holds, and
//! those of the class set expressions of the flag `v`, which `unicode_sets.rs` holds.

use swell_regexp::{ErrorKind, Flags, ParseError, Pattern, PatternError, Unlimited, parse_pattern};

#[cfg(test)]
fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[cfg(test)]
fn parse(pattern: &str, flags: &str) -> Result<Pattern, ParseError> {
    let flags = Flags::parse(&units(flags)).expect("the flags");
    parse_pattern(&units(pattern), flags, 1000, &mut Unlimited)
}

#[cfg(test)]
fn error(pattern: &str, flags: &str) -> PatternError {
    match parse(pattern, flags) {
        Err(ParseError::Pattern(error)) => error,
        other => panic!("/{pattern}/{flags}: {other:?}"),
    }
}

/// Each pattern answers `kind` at `offset` in each of the flag sets, and each pattern beside
/// it parses there.
#[cfg(test)]
fn hits(kind: ErrorKind, flag_sets: &[&str], wrong: &[(&str, usize)], right: &[&str]) {
    for flags in flag_sets {
        for &(pattern, offset) in wrong {
            let e = error(pattern, flags);
            assert_eq!(e.kind, kind, "/{pattern}/{flags}");
            assert_eq!(e.offset, offset, "/{pattern}/{flags}");
            assert!(!e.is_refusal(), "/{pattern}/{flags}");
        }
        for pattern in right {
            if let Err(e) = parse(pattern, flags) {
                panic!("/{pattern}/{flags}: {e:?}");
            }
        }
    }
}

#[test]
fn a_quantifier_whose_minimum_passes_its_maximum() {
    hits(
        ErrorKind::QuantifierOutOfOrder,
        &["", "u", "v"],
        &[
            ("a{2,1}", 1),
            ("x(a){10,9}", 4),
            ("a{010,9}", 1),
            ("a{4294967296,4294967295}", 1),
            ("a{99999999999999999999,99999999999999999998}", 1),
        ],
        &[
            "a{1,2}",
            "a{2,2}",
            "a{9,010}",
            "a{4294967295,4294967296}",
            "a{0,99999999999999999999}",
        ],
    );
}

#[test]
fn a_backreference_past_the_count_of_groups() {
    hits(
        ErrorKind::BackreferencePastGroups,
        &["u", "v"],
        &[("(a)\\2", 3), ("\\1", 0), ("(a)(b)\\3(c)\\4", 11)],
        &["(a)\\1", "\\1(a)", "(a)(b)\\2", "(a)(b)\\3(c)"],
    );
    // Without `u` the escape past the count is a legacy octal or an identity escape (B.1.2).
    hits(
        ErrorKind::BackreferencePastGroups,
        &[""],
        &[],
        &["(a)\\2", "\\1", "\\9"],
    );
}

#[test]
fn a_k_escape_naming_no_group() {
    hits(
        ErrorKind::UnknownGroupName,
        &["", "u", "v"],
        &[
            ("(?<a>x)\\k<b>", 7),
            ("\\k<ab>(?<a>x)", 0),
            ("(?<a>x)|\\k<A>", 8),
        ],
        &["(?<a>x)\\k<a>", "\\k<a>(?<a>x)", "(?<a>x)|\\k<a>"],
    );
    hits(
        ErrorKind::UnknownGroupName,
        &["u", "v"],
        &[("\\k<a>", 0)],
        &[],
    );
    hits(ErrorKind::UnknownGroupName, &[""], &[], &["\\k<a>"]);
}

#[test]
fn duplicate_group_names_only_in_different_alternatives() {
    hits(
        ErrorKind::DuplicateGroupName,
        &["", "u", "v"],
        &[
            ("(?<a>x)(?<a>y)", 7),
            ("(?<a>x(?<a>y))", 6),
            ("(?<a>x)|(?<a>y)(?<a>z)", 15),
            ("(?:(?<a>x)|(?<a>y))(?<a>z)", 19),
            ("((?<a>x)|b)(?<a>y)", 11),
            ("(?:(?:(?<a>x)))(?<a>y)", 15),
            ("(?:(?:(?<a>x))|y)(?<a>z)", 17),
            ("(?<b>x)|((?<b>y)|z)(?<b>w)", 19),
            ("(?<a>x|(?<a>y))", 7),
            // The earlier declaration two closed levels deep, the inner one opened in the
            // second alternative of the outer: the alternative that counts is the outer's.
            ("(?:x|(?:(?<a>y)))(?<a>z)", 17),
            ("(?:x|(?:(?<a>y)(?<b>y)))(?<b>z)", 24),
        ],
        &[
            "(?<a>x)|(?<a>y)",
            "(?<a>x)|(?<a>y)|(?<a>z)",
            "(?:(?<a>x)|(?<a>y))\\k<a>",
            "((?<a>x)|(?<a>y))|(?<a>z)",
            "(?:(?:(?<a>x)|(?<a>y))|(?<a>z))",
            "(?:(?:(?<a>x))|(?<a>z))",
            "(?<a>x)|(?<b>y)|(?<a>z)(?<b>w)",
            "(?<a>x)(?<b>y)",
            "(?:x|(?:(?<a>y)))|(?<a>z)",
            "(?:x|(?:(?<a>y)(?<b>y)))|(?<a>z)(?<b>w)",
        ],
    );
    let tree = parse("(?<a>x)|(?<b>y)|(?<a>z)", "").expect("a tree");
    assert_eq!(
        tree.group_names().collect::<Vec<_>>(),
        vec![(1, "a"), (2, "b"), (3, "a")]
    );
}

#[test]
fn a_class_range_out_of_order() {
    hits(
        ErrorKind::RangeOutOfOrder,
        &["", "u"],
        &[
            ("[b-a]", 1),
            ("x[ab-a]", 3),
            ("[\\x62-\\x61]", 1),
            ("[a-\\-]", 1),
        ],
        &["[a-b]", "[a-a]", "[\\x61-\\x62]", "[--a]"],
    );
    // Under `v` a range is a ClassSetRange of a ClassUnion, and `\-` is a reserved punctuator.
    hits(
        ErrorKind::RangeOutOfOrder,
        &["v"],
        &[
            ("[b-a]", 1),
            ("x[ab-a]", 3),
            ("[\\x62-\\x61]", 1),
            ("[a-\\-]", 1),
            ("[[a-b]z-y]", 6),
        ],
        &["[a-b]", "[a-a]", "[\\x61-\\x62]", "[\\--a]", "[[a-b]y-z]"],
    );
}

#[test]
fn a_class_range_with_a_class_escape_under_u() {
    hits(
        ErrorKind::ClassEscapeInRange,
        &["u"],
        &[("[\\d-a]", 1), ("[a-\\w]", 1), ("x[\\s-\\S]", 2)],
        &["[\\d-]", "[-\\d]", "[\\d\\-a]"],
    );
    // B.1.2.1: without `u` such a range is the union of its ends and `-`.
    hits(
        ErrorKind::ClassEscapeInRange,
        &[""],
        &[],
        &["[\\d-a]", "[a-\\w]", "[\\s-\\S]"],
    );
}

#[test]
fn a_group_name_escape_that_is_no_identifier_start_or_part() {
    hits(
        ErrorKind::InvalidGroupName,
        &["", "u", "v"],
        &[
            ("(?<\\u0030>x)", 3),
            ("(?<a\\u002D>x)", 4),
            ("(?<1>x)", 3),
            ("(?<>x)", 0),
        ],
        &[
            "(?<\\u0061>x)",
            "(?<a\\u0030>x)",
            "(?<\\u{24}_>x)",
            "(?<a1>x)",
        ],
    );
}

#[test]
fn an_invalid_braced_quantifier_without_u() {
    hits(
        ErrorKind::InvalidBracedQuantifier,
        &[""],
        &[
            ("{1}", 0),
            ("x|{2,}", 2),
            ("({1,2})", 1),
            ("a{1}{2}", 4),
            ("^{3}", 1),
            ("(?<=a){1}", 6),
        ],
        &["a{1}", "{", "{1", "{1,", "{,1}", "a{1}b{2}"],
    );
}

#[test]
fn modifiers_early_errors() {
    hits(
        ErrorKind::ModifierAddedAndRemoved,
        &["", "u", "i", "v"],
        &[("(?i-i:a)", 0), ("(?ims-m:a)", 0), ("x(?s-is:a)", 1)],
        &["(?i-m:a)", "(?ms-i:a)"],
    );
    hits(
        ErrorKind::RepeatedModifier,
        &["", "u", "v"],
        &[
            ("(?ii:a)", 0),
            ("(?imi:a)", 0),
            ("(?-ss:a)", 0),
            ("(?m-ii:a)", 0),
        ],
        &["(?ims:a)", "(?-ims:a)", "(?i-ms:a)"],
    );
    hits(
        ErrorKind::EmptyModifiers,
        &["", "u", "v"],
        &[("(?-:a)", 0), ("a(?-:)", 1)],
        &["(?i-:a)", "(?-i:a)", "(?:a)"],
    );
    hits(
        ErrorKind::InvalidGroup,
        &["", "u", "v"],
        &[
            ("(?x:a)", 0),
            ("(?i)", 0),
            ("(?i-m)", 0),
            ("(?", 0),
            ("(?ix:a)", 0),
        ],
        &[],
    );
}

#[test]
fn a_pattern_with_i_and_an_early_error_answers_the_early_error() {
    let e = error("a{2,1}", "i");
    assert_eq!(e.kind, ErrorKind::QuantifierOutOfOrder);
    assert!(!e.is_refusal());
    assert!(parse("(?i:a)", "i").is_ok());
    assert!(parse("a", "dgimsy").is_ok());
}

#[test]
fn flags_are_d_g_i_m_s_u_v_y_each_once_and_not_u_with_v() {
    let flags = Flags::parse(&units("dgimsuy")).expect("the flags");
    assert!(flags.has_indices() && flags.global() && flags.ignore_case() && flags.multiline());
    assert!(flags.dot_all() && flags.unicode() && flags.sticky() && !flags.unicode_sets());
    let v = Flags::parse(&units("v")).expect("the flag v");
    assert!(v.unicode_sets() && v.either_unicode() && !v.unicode());
    let none = Flags::parse(&[]).expect("no flag");
    assert_eq!(none, Flags::default());
    assert!(!none.either_unicode());
    for (flags, kind, offset) in [
        ("x", ErrorKind::UnknownFlag, 0),
        ("gG", ErrorKind::UnknownFlag, 1),
        ("gg", ErrorKind::RepeatedFlag, 1),
        ("gimg", ErrorKind::RepeatedFlag, 3),
        ("ggx", ErrorKind::UnknownFlag, 2),
        ("uv", ErrorKind::UnicodeWithSets, 1),
        ("vgu", ErrorKind::UnicodeWithSets, 2),
        (" ", ErrorKind::UnknownFlag, 0),
    ] {
        let e = Flags::parse(&units(flags)).expect_err(flags);
        assert_eq!((e.kind, e.offset), (kind, offset), "flags {flags:?}");
        assert!(!e.is_refusal());
    }
}

#[test]
fn the_patterns_of_slice_5_are_no_refusals() {
    // The flag `v` parses, and its early errors are early errors.
    for (pattern, flags) in [("a", "v"), ("[a&&b]", "gv"), ("[\\p{L}--\\q{a|bc}]", "vi")] {
        assert!(parse(pattern, flags).is_ok(), "/{pattern}/{flags}");
    }
    let e = error("a{2,1}", "v");
    assert_eq!((e.kind, e.offset), (ErrorKind::QuantifierOutOfOrder, 1));
    assert!(!e.is_refusal());
    // A group name outside ASCII is no refusal (`group_names.rs`): it parses, and a reference
    // to a name no group has is the early error.
    for (pattern, flags) in [
        ("(?<é>a)", ""),
        ("(?<aé>a)", "u"),
        ("(?<\\u00E9>a)", ""),
        ("(?<a\\u{10000}>a)", "u"),
        ("(?<\\uD835\\uDC9C>a)", ""),
        ("(?<\u{1D49C}>a)", ""),
        ("(?<\u{1D49C}>a)", "u"),
    ] {
        assert!(parse(pattern, flags).is_ok(), "/{pattern}/{flags}");
    }
    let e = error("(?<a>x)\\k<é>", "");
    assert_eq!((e.kind, e.offset), (ErrorKind::UnknownGroupName, 7));
    assert!(!e.is_refusal());
    // A property escape is no refusal: it parses under `u`, and without `u` `\p` is an identity
    // escape.
    for pattern in ["\\p{L}", "a\\P{Lu}", "[a\\p{L}]"] {
        assert!(parse(pattern, "u").is_ok(), "/{pattern}/u");
        assert!(parse(pattern, "").is_ok(), "/{pattern}/");
    }
    // The grammar errors and the nesting limit are no refusals.
    assert!(!error("(", "").is_refusal());
    assert_eq!(error("(", "").message(), "unterminated group");
}

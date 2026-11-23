//! Group names outside ASCII (`#prod-RegExpIdentifierName`, the early errors of
//! RegExpIdentifierStart and RegExpIdentifierPart in `#sec-patterns-static-semantics-early-errors`):
//! a name starts with an IdentifierStartChar (`ID_Start`, `$`, `_`) and goes on with
//! IdentifierPartChars (`ID_Continue`, `$`, U+200C, U+200D), written or as a `\u` escape of
//! RegExpUnicodeEscapeSequence[+UnicodeMode] (with and without braces, a pair of escaped
//! surrogates one code point) under `u` and without it; without `u` a written surrogate pair is
//! one code point (UnicodeLeadSurrogate UnicodeTrailSurrogate). The name is its code points
//! (`#sec-static-semantics-capturinggroupname`), so an escaped name and the written one are the
//! same name for the duplicate check and for `\k`.

use swell_regexp::{ErrorKind, Flags, ParseError, Pattern, PatternError, Unlimited, parse_pattern};

#[cfg(test)]
fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[cfg(test)]
fn parse_units(pattern: &[u16], flags: &str) -> Result<Pattern, ParseError> {
    let flags = Flags::parse(&units(flags)).expect("the flags");
    parse_pattern(pattern, flags, 1000, &mut Unlimited)
}

#[cfg(test)]
fn parse(pattern: &str, flags: &str) -> Result<Pattern, ParseError> {
    parse_units(&units(pattern), flags)
}

#[cfg(test)]
fn names(pattern: &str, flags: &str) -> Vec<(u32, String)> {
    match parse(pattern, flags) {
        Ok(tree) => tree
            .group_names()
            .map(|(number, name)| (number, name.to_string()))
            .collect(),
        Err(e) => panic!("/{pattern}/{flags}: {e:?}"),
    }
}

#[cfg(test)]
fn error_of(result: Result<Pattern, ParseError>, what: &str) -> PatternError {
    match result {
        Err(ParseError::Pattern(error)) => error,
        other => panic!("{what}: {other:?}"),
    }
}

#[test]
fn group_names_outside_ascii_with_and_without_u() {
    for flags in ["", "u", "gu", "d"] {
        for (pattern, name) in [
            ("(?<é>a)", "é"),
            ("(?<café>a)", "café"),
            ("(?<π_1>a)\\k<π_1>", "π_1"),
            // U+2118 and U+309B: Other_ID_Start, folded into ID_Start.
            ("(?<℘>a)", "℘"),
            ("(?<゛>a)", "゛"),
            // U+1D49C, written: a surrogate pair, one code point in both modes.
            ("(?<𝒜>a)", "𝒜"),
            ("(?<x𝒜y>a)", "x𝒜y"),
            // U+00B7 (Other_ID_Continue), U+0300 and U+0660 (ID_Continue) after the start.
            ("(?<a·̀٠>a)", "a·̀٠"),
            // U+200C and U+200D as parts.
            ("(?<a\u{200C}b\u{200D}>a)", "a\u{200C}b\u{200D}"),
            ("(?<$é_>a)", "$é_"),
            ("(?<_ૺ>a)", "_ૺ"),
        ] {
            assert_eq!(
                names(pattern, flags),
                vec![(1, name.to_string())],
                "/{pattern}/{flags}"
            );
        }
        assert_eq!(
            names("(?<α>a)(?<β>b)|(?<α>c)", flags),
            vec![
                (1, "α".to_string()),
                (2, "β".to_string()),
                (3, "α".to_string())
            ],
            "/(?<α>a)(?<β>b)|(?<α>c)/{flags}"
        );
    }
    // Without `u` the name is read by code point, but a character outside the name stays a
    // code unit: a quantifier after a written surrogate pair repeats its trail alone.
    assert!(parse("(?<𝒜>a)𝒜+", "").is_ok());
}

#[test]
fn an_escaped_group_name_equals_the_written_one() {
    for flags in ["", "u"] {
        for (pattern, name) in [
            ("(?<\\u00e9>a)", "é"),
            ("(?<\\u{e9}>a)", "é"),
            ("(?<\\u{0000E9}>a)", "é"),
            ("(?<caf\\u00E9>a)", "café"),
            ("(?<\\u{1D49C}>a)", "𝒜"),
            ("(?<\\uD835\\uDC9C>a)", "𝒜"),
            ("(?<a\\u200C>a)", "a\u{200C}"),
            ("(?<a\\u{200d}>a)", "a\u{200D}"),
            ("(?<\\u2118>a)", "℘"),
        ] {
            assert_eq!(
                names(pattern, flags),
                vec![(1, name.to_string())],
                "/{pattern}/{flags}"
            );
        }
        // A reference by the other spelling finds the group.
        for pattern in [
            "(?<é>a)\\k<\\u00e9>",
            "(?<\\u00e9>a)\\k<é>",
            "\\k<\\u{e9}>(?<é>a)",
            "(?<𝒜>a)\\k<\\uD835\\uDC9C>",
            "(?<\\u{1D49C}>a)\\k<𝒜>",
        ] {
            assert!(parse(pattern, flags).is_ok(), "/{pattern}/{flags}");
        }
        // Declared twice by two spellings where both might participate: the duplicate name.
        for (pattern, offset) in [
            ("(?<é>a)(?<\\u00e9>b)", 7),
            ("(?<\\u{e9}>a)(?<é>b)", 12),
            ("(?<𝒜>a)(?<\\uD835\\uDC9C>b)", 8),
        ] {
            let e = error_of(parse(pattern, flags), pattern);
            assert_eq!(
                (e.kind, e.offset),
                (ErrorKind::DuplicateGroupName, offset),
                "/{pattern}/{flags}"
            );
            assert!(!e.is_refusal(), "/{pattern}/{flags}");
        }
        assert!(parse("(?<é>a)|(?<\\u00e9>b)", flags).is_ok());
        // Names that differ by a code point stay apart: é and e followed by U+0301 are
        // canonically equivalent and two names (`#sec-identifier-names`).
        assert_eq!(
            names("(?<é>a)(?<e\u{301}>b)", flags),
            vec![(1, "é".to_string()), (2, "e\u{301}".to_string())]
        );
    }
    // Under `u` a reference to a name no group has is the early error, by code point.
    let e = error_of(parse("(?<é>a)\\k<\\u00e8>", "u"), "\\k<è>");
    assert_eq!(e.kind, ErrorKind::UnknownGroupName);
}

#[test]
fn a_name_that_is_not_id_start_is_an_early_error() {
    for flags in ["", "u"] {
        for (pattern, offset) in [
            // U+0300 and U+0660: ID_Continue, not ID_Start.
            ("(?<\u{300}a>x)", 3),
            ("(?<\\u0300a>x)", 3),
            ("(?<٠>x)", 3),
            ("(?<\\u{660}>x)", 3),
            // U+200C and U+200D are parts only.
            ("(?<\u{200C}a>x)", 3),
            ("(?<\\u200Da>x)", 3),
            // U+2E2F, VERTICAL TILDE: Lm and Pattern_Syntax, neither ID_Start nor ID_Continue.
            ("(?<\u{2E2F}>x)", 3),
            ("(?<a\u{2E2F}>x)", 4),
            ("(?<a\\u2E2F>x)", 4),
            // U+00D7, U+20AC and U+3000: no identifier code point.
            ("(?<a×>x)", 4),
            ("(?<€>x)", 3),
            ("(?<a\u{3000}>x)", 4),
            // An escaped lone surrogate is no identifier code point, nor two escapes of a lead.
            ("(?<\\uD835>x)", 3),
            ("(?<a\\uDC9C>x)", 4),
            ("(?<\\uD835\\uD835>x)", 3),
            // The reference is a name too.
            ("(?<a>x)\\k<\\u0300>", 10),
        ] {
            let e = error_of(parse(pattern, flags), pattern);
            assert_eq!(
                (e.kind, e.offset),
                (ErrorKind::InvalidGroupName, offset),
                "/{pattern}/{flags}"
            );
            assert!(!e.is_refusal(), "/{pattern}/{flags}");
            assert_eq!(
                e.message(),
                "invalid capture group name",
                "/{pattern}/{flags}"
            );
        }
        // The same code points after the start, where they are parts.
        for pattern in [
            "(?<a\u{300}>x)",
            "(?<a\\u0300>x)",
            "(?<a٠>x)",
            "(?<a\u{200C}>x)",
        ] {
            assert!(parse(pattern, flags).is_ok(), "/{pattern}/{flags}");
        }
    }
    // A written lone surrogate, which a UTF-8 text cannot hold, in both modes and at both
    // places.
    for flags in ["", "u"] {
        for (name, offset) in [(vec![0xD835_u16], 3), (vec![0x61, 0xDC9C], 4)] {
            let mut pattern = units("(?<");
            pattern.extend(&name);
            pattern.extend(units(">x)"));
            let e = error_of(parse_units(&pattern, flags), "a lone surrogate");
            assert_eq!(
                (e.kind, e.offset),
                (ErrorKind::InvalidGroupName, offset),
                "{name:X?} /{flags}"
            );
        }
    }
}

//! Identifiers outside ASCII (`#sec-names-and-keywords`): IdentifierStartChar is `ID_Start`,
//! `$` or `_`, IdentifierPartChar is `ID_Continue`, `$`, U+200C or U+200D (UAX #31, the tables
//! of `swell-unicode`), written (a surrogate pair is one code point, `#sec-source-text`) or as a
//! `\u` escape whose code point is checked alone (`#sec-identifier-names-static-semantics-early-errors`).
//! The flags after a regular expression literal are IdentifierPartChars, so a flag outside ASCII
//! is the early error of an unknown flag
//! (`#sec-primary-expression-regular-expression-literals-static-semantics-early-errors`).

use swell_syntax::{MessageKind, ParseOptions, Script, SyntaxError, message_kind, parse_script};

#[cfg(test)]
fn parse_units(source: &[u16]) -> Result<Script, Vec<SyntaxError>> {
    parse_script(source, ParseOptions { max_nesting: 1_000 })
}

#[cfg(test)]
fn parse(text: &str) -> Result<Script, Vec<SyntaxError>> {
    parse_units(&text.encode_utf16().collect::<Vec<_>>())
}

#[cfg(test)]
fn var_names(text: &str) -> Vec<String> {
    match parse(text) {
        Ok(script) => script.scope.var_names.clone(),
        Err(errors) => panic!("{text:?}: {errors:?}"),
    }
}

/// The first message of the errors of `source`, which must be a grammar or early error.
#[cfg(test)]
fn early_error(source: &str, errors: Result<Script, Vec<SyntaxError>>) -> String {
    let errors = match errors {
        Ok(_) => panic!("{source:?} parsed"),
        Err(errors) => errors,
    };
    for e in &errors {
        assert_eq!(
            message_kind(&e.message),
            MessageKind::Grammar,
            "{source:?}: {e:?}"
        );
    }
    errors[0].message.clone()
}

#[test]
fn identifiers_outside_ascii_lex_by_id_start_and_id_continue() {
    for (source, name) in [
        ("var café = 1;", "café"),
        ("var π = 3;", "π"),
        ("var Ωmega;", "Ωmega"),
        // U+2118 and U+309B: Other_ID_Start, folded into ID_Start.
        ("var ℘;", "℘"),
        ("var ゛;", "゛"),
        // U+1D49C, written: a surrogate pair, one code point.
        ("var 𝒜 = 1;", "𝒜"),
        ("var a𝒜b;", "a𝒜b"),
        // U+00B7 (Other_ID_Continue), U+0300, U+0660 and U+0AFA (ID_Continue) as parts.
        ("var a·̀٠;", "a·̀٠"),
        ("var _ૺ;", "_ૺ"),
        // U+200C and U+200D as parts.
        ("var a\u{200C}b\u{200D};", "a\u{200C}b\u{200D}"),
        ("var $é_1;", "$é_1"),
        // Not white space and not a part: the name ends before U+00A0 and U+3000.
        ("var é\u{A0}= 1;", "é"),
        ("var é\u{3000}= 1;", "é"),
    ] {
        assert_eq!(var_names(source), vec![name], "{source:?}");
    }
    // A name outside ASCII is a name everywhere one stands: a label, a parameter, a property
    // name, a function name, a member.
    let script =
        parse("ä: for (;;) { break ä; } function ƒ(ö, ü) { return ö.ß + { ŋ: 1 }.ŋ; } ƒ.名前;")
            .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(script.scope.var_names, vec!["ƒ"]);
    // The name ends at a code point that is no part, which is then a token of its own.
    assert_eq!(
        early_error("var a€;", parse("var a€;")),
        "unexpected character U+20AC"
    );
}

#[test]
fn an_escape_outside_ascii_names_its_code_point() {
    for (source, name) in [
        ("var \\u00e9 = 1;", "é"),
        ("var \\u{e9};", "é"),
        ("var caf\\u00E9;", "café"),
        ("var \\u03C0\\u{3c0};", "ππ"),
        ("var \\u2118;", "℘"),
        ("var \\u{1D49C};", "𝒜"),
        ("var a\\u0300;", "a\u{300}"),
        ("var a\\u200C\\u{200d};", "a\u{200C}\u{200D}"),
        ("var _\\u0AFA;", "_ૺ"),
    ] {
        assert_eq!(var_names(source), vec![name], "{source:?}");
    }
    // The escaped name and the written one are one binding.
    let script = parse("var \\u00e9; var é; function f(\\u{e9}) { return é; }")
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(script.scope.var_names, vec!["é", "f"]);
    // Duplicate lexical names by two spellings are the early error of a duplicate.
    let message = early_error("let \\u00e9; let é;", parse("let \\u00e9; let é;"));
    assert!(message.contains("é"), "{message}");
}

#[test]
fn a_code_point_that_is_not_id_start_is_an_early_error() {
    for (source, message, column) in [
        // U+0300 and U+0660: ID_Continue, not ID_Start.
        (
            "var \\u0300a = 1;",
            "the escape \\u{300} is no identifier code point here",
            5,
        ),
        (
            "var \\u{660};",
            "the escape \\u{660} is no identifier code point here",
            5,
        ),
        ("var \u{300}a = 1;", "unexpected character U+0300", 5),
        ("var ٠;", "unexpected character U+0660", 5),
        // U+200C: a part only.
        (
            "var \\u200Ca;",
            "the escape \\u{200C} is no identifier code point here",
            5,
        ),
        ("var \u{200C}a;", "unexpected character U+200C", 5),
        // U+2E2F, VERTICAL TILDE: Lm and Pattern_Syntax, neither ID_Start nor ID_Continue.
        (
            "var a\\u2E2F;",
            "the escape \\u{2E2F} is no identifier code point here",
            6,
        ),
        ("var a\u{2E2F};", "unexpected character U+2E2F", 6),
        ("var \u{2E2F};", "unexpected character U+2E2F", 5),
        // A surrogate pair of two escapes is two code points, each checked alone.
        (
            "var \\uD835\\uDC9C;",
            "the escape \\u{D835} is no identifier code point here",
            5,
        ),
        (
            "var a\\uDC9C;",
            "the escape \\u{DC9C} is no identifier code point here",
            6,
        ),
        // A digit outside ASCII may not follow a numeric literal either.
        (
            "x = 3π;",
            "identifier start immediately after a numeric literal",
            6,
        ),
        // U+1D7CE, outside the Basic Multilingual Plane: Nd, ID_Continue and not ID_Start.
        (
            "var \\u{1D7CE};",
            "the escape \\u{1D7CE} is no identifier code point here",
            5,
        ),
    ] {
        let errors = parse(source);
        let first = match &errors {
            Err(errors) => errors[0].clone(),
            Ok(_) => panic!("{source:?} parsed"),
        };
        assert_eq!(early_error(source, errors), message, "{source:?}");
        assert_eq!((first.line, first.column), (1, column), "{source:?}");
    }
    // A written lone surrogate, which a UTF-8 text cannot hold, at the start and as a part.
    for (units, at) in [
        (vec![0x76_u16, 0x61, 0x72, 0x20, 0xD835, 0x3B], "U+D835"),
        (vec![0x76, 0x61, 0x72, 0x20, 0x61, 0xDC9C, 0x3B], "U+DC9C"),
    ] {
        let message = early_error("a lone surrogate", parse_units(&units));
        assert_eq!(message, format!("unexpected character {at}"));
    }
}

#[test]
fn a_flag_outside_ascii_is_an_unknown_flag() {
    for source in ["x = /a/é;", "x = /a/gé;", "x = /a/g\u{200C};", "x = /a/𝒜;"] {
        assert_eq!(
            early_error(source, parse(source)),
            "invalid regular expression flags",
            "{source:?}"
        );
    }
    // A code point that is no IdentifierPartChar ends the flags and is a token of its own.
    assert_eq!(
        early_error("x = /a/g€;", parse("x = /a/g€;")),
        "unexpected character U+20AC"
    );
    // White space outside ASCII ends them too.
    assert!(parse("x = /a/g\u{A0};").is_ok());
}

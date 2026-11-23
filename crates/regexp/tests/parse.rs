//! ParsePattern (`#sec-parsepattern`): every production of 22.2.1 (`#sec-patterns`) with and
//! without `u`, the grammar of B.1.2 (`#sec-regular-expressions-patterns`) without `u` or `v`,
//! the second parse with [NamedCaptureGroups] (B.1.2.9), and a surrogate pair under `u`.
//!
//! Each tree is written out by [`show`] in a short form: a character as itself when it is an
//! ASCII letter or digit and as `U+XXXX` otherwise; the terms of an alternative separated by a
//! space, `ε` for none; a Disjunction as `{a | b}`; a group as `(<number> body)` with
//! `<name>` after its number when it has one.

use swell_regexp::{
    Assertion, ClassEscape, ClassItem, Flags, Look, Node, NodeId, ParseError, Pattern, Unlimited,
    parse_pattern,
};

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
fn character(c: u32) -> String {
    match char::from_u32(c) {
        Some(ch) if ch.is_ascii_alphanumeric() => ch.to_string(),
        _ => format!("U+{c:04X}"),
    }
}

#[cfg(test)]
fn escape(e: ClassEscape) -> &'static str {
    match e {
        ClassEscape::Digit => "\\d",
        ClassEscape::NotDigit => "\\D",
        ClassEscape::Space => "\\s",
        ClassEscape::NotSpace => "\\S",
        ClassEscape::Word => "\\w",
        ClassEscape::NotWord => "\\W",
    }
}

#[cfg(test)]
fn modifiers(m: swell_regexp::Modifiers) -> String {
    let mut text = String::new();
    for (on, letter) in [
        (m.ignore_case(), 'i'),
        (m.multiline(), 'm'),
        (m.dot_all(), 's'),
    ] {
        if on {
            text.push(letter);
        }
    }
    text
}

#[cfg(test)]
fn node(p: &Pattern, id: NodeId) -> String {
    match *p.node(id).expect("a node of the tree") {
        Node::Disjunction(list) => {
            let alternatives: Vec<String> = p.children(list).iter().map(|&a| node(p, a)).collect();
            format!("{{{}}}", alternatives.join(" | "))
        }
        Node::Alternative(list) => {
            if list.is_empty() {
                return "ε".to_string();
            }
            let terms: Vec<String> = p.children(list).iter().map(|&t| node(p, t)).collect();
            terms.join(" ")
        }
        Node::Assertion(a) => match a {
            Assertion::Start => "^",
            Assertion::End => "$",
            Assertion::WordBoundary => "\\b",
            Assertion::NotWordBoundary => "\\B",
        }
        .to_string(),
        Node::Look { look, body } => {
            let open = match look {
                Look::Ahead => "(?=",
                Look::NotAhead => "(?!",
                Look::Behind => "(?<=",
                Look::NotBehind => "(?<!",
            };
            format!("{open} {})", node(p, body))
        }
        Node::Quantified { body, quantifier } => {
            let q = match (quantifier.min, quantifier.max) {
                (0, None) => "*".to_string(),
                (1, None) => "+".to_string(),
                (0, Some(1)) => "?".to_string(),
                (n, Some(m)) if n == m => format!("{{{n}}}"),
                (n, None) => format!("{{{n},}}"),
                (n, Some(m)) => format!("{{{n},{m}}}"),
            };
            let lazy = if quantifier.greedy { "" } else { "?" };
            format!("{}{q}{lazy}", node(p, body))
        }
        Node::Character(c) => character(c),
        Node::Dot => ".".to_string(),
        Node::Escape(e) => escape(e).to_string(),
        Node::Class { negated, items } => {
            let items: Vec<String> = p
                .class_items(items)
                .iter()
                .map(|item| match *item {
                    ClassItem::Character(c) => character(c),
                    ClassItem::Range(a, b) => format!("{}-{}", character(a), character(b)),
                    ClassItem::Escape(e) => escape(e).to_string(),
                })
                .collect();
            let caret = if negated { "^" } else { "" };
            if items.is_empty() {
                format!("[{caret}]")
            } else if negated {
                format!("[^ {}]", items.join(" "))
            } else {
                format!("[{}]", items.join(" "))
            }
        }
        Node::Group { number, name, body } => match name {
            Some(name) => format!("({number}<{}> {})", p.name(name), node(p, body)),
            None => format!("({number} {})", node(p, body)),
        },
        Node::NonCapturing { body } => format!("(?: {})", node(p, body)),
        Node::Modified { add, remove, body } => {
            let remove = if remove.is_empty() {
                String::new()
            } else {
                format!("-{}", modifiers(remove))
            };
            format!("(?{}{remove}: {})", modifiers(add), node(p, body))
        }
        Node::Backreference(n) => format!("\\{n}"),
        Node::NamedBackreference(name) => format!("\\k<{}>", p.name(name)),
    }
}

#[cfg(test)]
fn show(p: &Pattern) -> String {
    node(p, p.root())
}

/// Each pattern parses with `flags` and is written out as given.
#[cfg(test)]
fn check(flags: &str, cases: &[(&str, &str)]) {
    for &(pattern, expected) in cases {
        match parse(pattern, flags) {
            Ok(tree) => assert_eq!(show(&tree), expected, "/{pattern}/{flags}"),
            Err(error) => panic!("/{pattern}/{flags}: {error:?}"),
        }
    }
}

/// The productions both modes share, as each mode reads them.
const SHARED: &[(&str, &str)] = &[
    ("", "ε"),
    ("a", "a"),
    ("ab", "a b"),
    ("a|b|", "{a | b | ε}"),
    ("|", "{ε | ε}"),
    ("^a$", "^ a $"),
    ("\\b\\B", "\\b \\B"),
    ("(?=a)(?!b)(?<=c)(?<!d)", "(?= a) (?! b) (?<= c) (?<! d)"),
    ("a*b+c?d{2}e{2,}f{2,3}", "a* b+ c? d{2} e{2,} f{2,3}"),
    (
        "a*?b+?c??d{2}?e{2,}?f{2,3}?",
        "a*? b+? c?? d{2}? e{2,}? f{2,3}?",
    ),
    ("a{0010,0020}", "a{10,20}"),
    ("a{99999999999}", "a{4294967295}"),
    (".", "."),
    ("(a)(?:b)(c|d)", "(1 a) (?: b) (2 {c | d})"),
    ("(?<n>c)(?<$_x9>d)", "(1<n> c) (2<$_x9> d)"),
    ("(?<\\u0061\\u{62}>c)", "(1<ab> c)"),
    (
        "(?i:a)(?m-s:b)(?-i:c)(?ims:d)",
        "(?i: a) (?m-s: b) (?-i: c) (?ims: d)",
    ),
    ("\\d\\D\\s\\S\\w\\W", "\\d \\D \\s \\S \\w \\W"),
    ("(a)\\1", "(1 a) \\1"),
    ("\\1(a)", "\\1 (1 a)"),
    ("(?<a>x)\\k<a>", "(1<a> x) \\k<a>"),
    ("\\f\\n\\r\\t\\v", "U+000C U+000A U+000D U+0009 U+000B"),
    ("\\cA\\cz\\cJ", "U+0001 U+001A U+000A"),
    ("\\0", "U+0000"),
    ("\\x41\\x7a", "A z"),
    ("\\u0042\\u00e9", "B U+00E9"),
    (
        "\\^\\$\\\\\\.\\*\\+\\?\\(\\)\\[\\]\\{\\}\\|\\/",
        "U+005E U+0024 U+005C U+002E U+002A U+002B U+003F U+0028 U+0029 U+005B U+005D U+007B U+007D U+007C U+002F",
    ),
    ("[abc]", "[a b c]"),
    ("[^a-z\\d]", "[^ a-z \\d]"),
    ("[]", "[]"),
    ("[^]", "[^]"),
    ("[-a]", "[U+002D a]"),
    ("[a-]", "[a U+002D]"),
    ("[a-c-e]", "[a-c U+002D e]"),
    ("[--a]", "[U+002D-a]"),
    ("[\\b\\-]", "[U+0008 U+002D]"),
    ("[\\d\\D\\s\\S\\w\\W]", "[\\d \\D \\s \\S \\w \\W]"),
    (
        "[\\f\\cA\\0\\x41\\u0042\\]]",
        "[U+000C U+0001 U+0000 A B U+005D]",
    ),
    ("[\\x00-\\x7F]", "[U+0000-U+007F]"),
    (
        "[(){}|.*+?^$]",
        "[U+0028 U+0029 U+007B U+007D U+007C U+002E U+002A U+002B U+003F U+005E U+0024]",
    ),
    ("(?:(a)|(b))+\\2", "(?: {(1 a) | (2 b)})+ \\2"),
    ("((((a))))", "(1 (2 (3 (4 a))))"),
];

#[test]
fn every_production_parses_without_u() {
    check("", SHARED);
    check(
        "",
        &[
            ("(?=a)*", "(?= a)*"),
            ("\\-", "U+002D"),
            ("\\z\\é", "z U+00E9"),
            ("[\\z]", "[z]"),
        ],
    );
    let tree = parse("(a)(?<n>b)((c))", "").expect("a tree");
    assert_eq!(tree.group_count(), 4);
    assert_eq!(tree.group_names().collect::<Vec<_>>(), vec![(2, "n")]);
    assert!(!tree.flags().unicode());
}

#[test]
fn every_production_parses_with_u() {
    check("u", SHARED);
    check(
        "u",
        &[
            ("\\u{1F600}", "U+1F600"),
            ("\\u{0000000041}", "A"),
            ("\\u{10FFFF}", "U+10FFFF"),
            ("[\\u{61}-\\u{7A}]", "[a-z]"),
            ("[\\-]", "[U+002D]"),
            ("\\k<a>(?<a>b)", "\\k<a> (1<a> b)"),
        ],
    );
    let tree = parse("(a)(?<n>b)", "u").expect("a tree");
    assert_eq!(tree.group_count(), 2);
    assert!(tree.flags().unicode());
}

#[test]
fn the_annex_b_grammar_holds_without_u_or_v() {
    check(
        "",
        &[
            // ExtendedPatternCharacter: `{`, `}` and `]` alone.
            ("{", "U+007B"),
            ("}", "U+007D"),
            ("]", "U+005D"),
            ("{}", "U+007B U+007D"),
            ("a{", "a U+007B"),
            ("a{1", "a U+007B 1"),
            ("a{1,", "a U+007B 1 U+002C"),
            ("a{,1}", "a U+007B U+002C 1 U+007D"),
            ("x{1}}", "x{1} U+007D"),
            ("^{", "^ U+007B"),
            // `\u` and `\x` with no digits are identity escapes; `{41}` is then a quantifier.
            ("\\u{41}", "u{41}"),
            ("\\u12", "u 1 2"),
            ("\\x4", "x 4"),
            // `\` [lookahead = c] in a pattern and in a class, and ClassControlLetter.
            ("\\c", "U+005C c"),
            ("\\c1", "U+005C c 1"),
            ("[\\c]", "[U+005C c]"),
            ("[\\c1]", "[U+0011]"),
            ("[\\c_]", "[U+001F]"),
            ("[\\cA]", "[U+0001]"),
            // Legacy octal escapes, and DecimalEscape only up to the count of groups.
            ("\\1", "U+0001"),
            ("\\7", "U+0007"),
            ("\\08", "U+0000 8"),
            ("\\01", "U+0001"),
            ("\\377", "U+00FF"),
            ("\\400", "U+0020 0"),
            ("\\8\\9", "8 9"),
            ("(a)\\2", "(1 a) U+0002"),
            ("(a)\\1", "(1 a) \\1"),
            ("(a)\\12", "(1 a) U+000A"),
            ("[\\1\\8]", "[U+0001 8]"),
            // QuantifiableAssertion.
            ("(?=a)*", "(?= a)*"),
            ("(?!a){2}", "(?! a){2}"),
            ("(?=a)+?", "(?= a)+?"),
            // A class escape at an end of a class range (B.1.2.1).
            ("[\\d-a]", "[\\d U+002D a]"),
            ("[a-\\d]", "[a U+002D \\d]"),
            ("[\\s-\\w]", "[\\s U+002D \\w]"),
            // The IdentityEscape of B.1.2, `\k` among them in a pattern with no group name.
            ("\\k", "k"),
            ("\\k<a>", "k U+003C a U+003E"),
            ("[\\k]", "[k]"),
            ("\\p{L}", "p U+007B L U+007D"),
            ("[\\B]", "[B]"),
            ("\\_", "U+005F"),
        ],
    );
}

#[test]
fn the_annex_b_forms_are_errors_with_u() {
    for pattern in [
        "{",
        "}",
        "]",
        "a{",
        "a{1",
        "a{1,",
        "x{1}}",
        "\\u{41",
        "\\u12",
        "\\x4",
        "\\c",
        "\\c1",
        "[\\c]",
        "[\\c1]",
        "[\\c_]",
        "\\1",
        "\\08",
        "\\01",
        "\\8",
        "(a)\\2",
        "[\\1]",
        "(?=a)*",
        "(?!a){2}",
        "[\\d-a]",
        "[a-\\d]",
        "\\k",
        "\\k<a>",
        "[\\k]",
        "[\\B]",
        "\\_",
        "\\z",
        "\\-",
        "\\u{110000}",
        "\\p",
        "[\\p]",
    ] {
        match parse(pattern, "u") {
            Err(ParseError::Pattern(error)) => {
                assert!(!error.is_refusal(), "/{pattern}/u: {error:?}");
            }
            other => panic!("/{pattern}/u: {other:?}"),
        }
        assert!(parse(pattern, "").is_ok(), "/{pattern}/ without u");
    }
}

#[test]
fn a_pattern_with_a_group_name_is_parsed_again_with_named_capture_groups() {
    // Without a group name `\k` is an identity escape, in a pattern and in a class.
    check(
        "",
        &[
            ("\\k", "k"),
            ("\\k<a>", "k U+003C a U+003E"),
            ("a\\k<b", "a k U+003C b"),
            ("[\\k]", "[k]"),
        ],
    );
    // With one it is a reference, also before the group it names.
    check(
        "",
        &[
            ("(?<a>x)\\k<a>", "(1<a> x) \\k<a>"),
            ("\\k<a>(?<a>x)", "\\k<a> (1<a> x)"),
            ("(?<a>x)|\\k<a>", "{(1<a> x) | \\k<a>}"),
        ],
    );
    // Parsed again, the pattern meets the grammar with [+NamedCaptureGroups].
    for pattern in [
        "\\k(?<a>x)",
        "(?<a>x)\\k",
        "\\k<b>(?<a>x)",
        "(?<a>x)[\\k]",
        "\\k<a(?<a>x)",
    ] {
        match parse(pattern, "") {
            Err(ParseError::Pattern(error)) => assert!(!error.is_refusal(), "/{pattern}/"),
            other => panic!("/{pattern}/: {other:?}"),
        }
    }
}

#[test]
fn a_surrogate_pair_is_one_code_point_with_u() {
    let grin = "\u{1F600}";
    check(
        "u",
        &[
            (grin, "U+1F600"),
            ("\\uD83D\\uDE00", "U+1F600"),
            ("\\uD83D\\uDE00+", "U+1F600+"),
            ("\u{1F600}+", "U+1F600+"),
            ("[\u{1F600}]", "[U+1F600]"),
            ("[\u{1F600}-\u{1F602}]", "[U+1F600-U+1F602]"),
            ("[\\uD83D\\uDE00-\\uD83D\\uDE02]", "[U+1F600-U+1F602]"),
            ("\\uD83D", "U+D83D"),
            ("\\uD83D\\u0041", "U+D83D A"),
            ("\\uDE00\\uD83D", "U+DE00 U+D83D"),
        ],
    );
    check(
        "",
        &[
            (grin, "U+D83D U+DE00"),
            ("\\uD83D\\uDE00", "U+D83D U+DE00"),
            ("\u{1F600}+", "U+D83D U+DE00+"),
            ("[\u{1F600}]", "[U+D83D U+DE00]"),
        ],
    );
    // Without `u` the class range runs from the second unit of one pair to the first of the
    // next: out of order.
    assert!(matches!(
        parse("[\u{1F600}-\u{1F602}]", ""),
        Err(ParseError::Pattern(_))
    ));
    // A lone surrogate is one character in either mode.
    let lone = [0x61, 0xD83D, 0x62];
    for flags in ["", "u"] {
        let tree = parse_units(&lone, flags).expect("a tree");
        assert_eq!(show(&tree), "a U+D83D b", "flags {flags:?}");
    }
}

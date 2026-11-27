//! The properties of the property escapes `\p{...}` and `\P{...}`
//! (`#sec-runtime-semantics-unicodematchproperty-p`,
//! `#sec-runtime-semantics-unicodematchpropertyvalue-p-v`): their names as ECMA-262's tables and
//! `PropertyValueAliases.txt` give them, the unions of the groups of General_Category, the rule of
//! Script_Extensions, and `ID_Start` and `ID_Continue`.
//!
//! The tables of names below are ECMA-262's (`#table-nonbinary-unicode-properties`,
//! `#table-binary-unicode-properties`), written here from the pinned draft; the values of
//! General_Category and Script are read from the vendored `PropertyValueAliases.txt`, and the
//! extensions from `ScriptExtensions.txt`.
#![cfg(test)]

use std::fs;
use std::path::PathBuf;

use swell_unicode::{
    LONGEST_STRING, Property, is_id_continue, is_id_start, lone_property, property_value,
    simple_case_fold_equivalents, string_property,
};

/// ECMA-262's table of non-binary properties: each name and alias.
const NON_BINARY: &[&str] = &[
    "General_Category",
    "gc",
    "Script",
    "sc",
    "Script_Extensions",
    "scx",
];

/// ECMA-262's table of binary properties: each name with its aliases.
const BINARY: &[&[&str]] = &[
    &["ASCII"],
    &["ASCII_Hex_Digit", "AHex"],
    &["Alphabetic", "Alpha"],
    &["Any"],
    &["Assigned"],
    &["Bidi_Control", "Bidi_C"],
    &["Bidi_Mirrored", "Bidi_M"],
    &["Case_Ignorable", "CI"],
    &["Cased"],
    &["Changes_When_Casefolded", "CWCF"],
    &["Changes_When_Casemapped", "CWCM"],
    &["Changes_When_Lowercased", "CWL"],
    &["Changes_When_NFKC_Casefolded", "CWKCF"],
    &["Changes_When_Titlecased", "CWT"],
    &["Changes_When_Uppercased", "CWU"],
    &["Dash"],
    &["Default_Ignorable_Code_Point", "DI"],
    &["Deprecated", "Dep"],
    &["Diacritic", "Dia"],
    &["Emoji"],
    &["Emoji_Component", "EComp"],
    &["Emoji_Modifier", "EMod"],
    &["Emoji_Modifier_Base", "EBase"],
    &["Emoji_Presentation", "EPres"],
    &["Extended_Pictographic", "ExtPict"],
    &["Extender", "Ext"],
    &["Grapheme_Base", "Gr_Base"],
    &["Grapheme_Extend", "Gr_Ext"],
    &["Hex_Digit", "Hex"],
    &["IDS_Binary_Operator", "IDSB"],
    &["IDS_Trinary_Operator", "IDST"],
    &["ID_Continue", "IDC"],
    &["ID_Start", "IDS"],
    &["Ideographic", "Ideo"],
    &["Join_Control", "Join_C"],
    &["Logical_Order_Exception", "LOE"],
    &["Lowercase", "Lower"],
    &["Math"],
    &["Noncharacter_Code_Point", "NChar"],
    &["Pattern_Syntax", "Pat_Syn"],
    &["Pattern_White_Space", "Pat_WS"],
    &["Quotation_Mark", "QMark"],
    &["Radical"],
    &["Regional_Indicator", "RI"],
    &["Sentence_Terminal", "STerm"],
    &["Soft_Dotted", "SD"],
    &["Terminal_Punctuation", "Term"],
    &["Unified_Ideograph", "UIdeo"],
    &["Uppercase", "Upper"],
    &["Variation_Selector", "VS"],
    &["White_Space", "space"],
    &["XID_Continue", "XIDC"],
    &["XID_Start", "XIDS"],
];

/// The groups of General_Category and their members (UAX #44, table 12).
const GROUPS: &[(&str, &[&str])] = &[
    ("C", &["Cc", "Cf", "Cs", "Co", "Cn"]),
    ("L", &["Lu", "Ll", "Lt", "Lm", "Lo"]),
    ("LC", &["Lu", "Ll", "Lt"]),
    ("M", &["Mn", "Mc", "Me"]),
    ("N", &["Nd", "Nl", "No"]),
    ("P", &["Pc", "Pd", "Ps", "Pe", "Pi", "Pf", "Po"]),
    ("S", &["Sm", "Sc", "Sk", "So"]),
    ("Z", &["Zs", "Zl", "Zp"]),
];

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn lone(name: &str) -> Option<Property> {
    lone_property(&units(name))
}

fn valued(name: &str, value: &str) -> Option<Property> {
    property_value(&units(name), &units(value))
}

fn data(path: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/unicode/17.0.0/ucd")
        .join(path);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The fields of the data lines of `text` whose first field is `first`, comments cut.
fn lines_of(text: &str, first: &str) -> Vec<Vec<String>> {
    text.lines()
        .map(|line| line.split('#').next().unwrap_or_default())
        .map(|line| {
            line.split(';')
                .map(|f| f.trim().to_owned())
                .collect::<Vec<_>>()
        })
        .filter(|fields| fields.first().map(String::as_str) == Some(first))
        .collect()
}

/// `ranges` sorted with overlapping and adjacent ranges joined.
fn merged(mut ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    ranges.sort_unstable();
    let mut out: Vec<(u32, u32)> = Vec::new();
    for (first, last) in ranges {
        match out.last_mut() {
            Some(previous) if first <= previous.1.saturating_add(1) => {
                previous.1 = previous.1.max(last);
            }
            _ => out.push((first, last)),
        }
    }
    out
}

fn ranges(property: Property) -> Vec<(u32, u32)> {
    property.ranges().collect()
}

#[test]
fn every_name_and_alias_of_the_three_tables_resolves() {
    for name in NON_BINARY {
        assert!(lone(name).is_none(), "{name} alone is no lone value");
    }
    for (name, value) in [("General_Category", "Lu"), ("gc", "Lu")] {
        assert_eq!(valued(name, value), lone("Lu"), "{name}={value}");
    }
    let greek = valued("Script", "Greek");
    assert!(greek.is_some());
    assert_eq!(valued("sc", "Grek"), greek);
    let extensions = valued("Script_Extensions", "Greek");
    assert!(extensions.is_some());
    assert_ne!(
        extensions, greek,
        "Script_Extensions is a property of its own"
    );
    assert_eq!(valued("scx", "Grek"), extensions);

    for names in BINARY {
        let first = lone(names[0]);
        assert!(first.is_some(), "{}", names[0]);
        for alias in &names[1..] {
            assert_eq!(lone(alias), first, "{alias}");
        }
    }

    let aliases = data("PropertyValueAliases.txt");
    let gc = lines_of(&aliases, "gc");
    assert_eq!(gc.len(), 38, "the values of General_Category");
    for fields in &gc {
        let short = lone(&fields[1]);
        assert!(short.is_some(), "{fields:?}");
        for name in &fields[1..] {
            assert_eq!(lone(name), short, "{name}");
            assert_eq!(valued("General_Category", name), short, "{name}");
            assert_eq!(valued("gc", name), short, "{name}");
        }
    }
    let sc = lines_of(&aliases, "sc");
    assert_eq!(sc.len(), 176, "the values of Script");
    for fields in &sc {
        let script = valued("Script", &fields[1]);
        let extensions = valued("Script_Extensions", &fields[1]);
        assert!(script.is_some() && extensions.is_some(), "{fields:?}");
        for name in &fields[1..] {
            assert_eq!(valued("sc", name), script, "{name}");
            assert_eq!(valued("scx", name), extensions, "{name}");
        }
    }
}

#[test]
fn a_loose_or_unknown_name_resolves_to_nothing() {
    for name in [
        "",
        "lu",
        "LU",
        "Lowercase Letter",
        "lowercase_letter",
        "Lowercase-Letter",
        "IsLu",
        "isLowercase",
        "ascii",
        "any",
        "Alphabetic ",
        "WSpace",
        "IDS_Unary_Operator",
        "Full_Composition_Exclusion",
        "Other_Alphabetic",
        "Hyphen",
        "Greek",
        "Grek",
        "General_Category",
        "Script_Extensions",
        "InGreek",
        "Block",
    ] {
        assert!(lone(name).is_none(), "\\p{{{name}}}");
    }
    for (name, value) in [
        ("General_Category", ""),
        ("General_Category", "Greek"),
        ("general_category", "Lu"),
        ("GC", "Lu"),
        ("gc", "lu"),
        ("Script", "greek"),
        ("Script", "Lu"),
        ("Script", "Old Persian"),
        ("scx", "xpeo"),
        ("Script_Extensions", "Is_Greek"),
        ("Scx", "Greek"),
        ("ASCII", "Y"),
        ("ASCII", "Yes"),
        ("Alphabetic", "T"),
        ("Block", "Basic_Latin"),
        ("Line_Break", "Alphabetic"),
        ("", "Lu"),
    ] {
        assert!(valued(name, value).is_none(), "\\p{{{name}={value}}}");
    }
    assert!(valued("Script", "Xpeo").is_some() && valued("Script", "Old_Persian").is_some());
}

#[test]
fn general_category_groups_are_unions() {
    for (group, members) in GROUPS {
        let property = lone(group).unwrap_or_else(|| panic!("{group}"));
        let union = merged(
            members
                .iter()
                .flat_map(|member| ranges(lone(member).unwrap_or_else(|| panic!("{member}"))))
                .collect(),
        );
        assert_eq!(ranges(property), union, "{group}");
        for &(first, last) in &union {
            assert!(
                property.contains(first) && property.contains(last),
                "{group}"
            );
            if let Some(before) = first.checked_sub(1) {
                assert!(!property.contains(before), "{group} {before:04X}");
            }
            assert!(!property.contains(last + 1), "{group} {:04X}", last + 1);
        }
    }
    assert_eq!(lone("Letter"), lone("L"));
    assert_eq!(lone("Cased_Letter"), lone("LC"));
    assert_eq!(lone("Combining_Mark"), lone("M"));
    assert_eq!(lone("punct"), lone("P"));
    // Every code point has one value: the values together are every code point.
    let all: Vec<(u32, u32)> = merged(
        GROUPS
            .iter()
            .filter(|(group, _)| *group != "LC")
            .flat_map(|(group, _)| ranges(lone(group).unwrap_or_else(|| panic!("{group}"))))
            .collect(),
    );
    assert_eq!(all, [(0, 0x10FFFF)]);
    let assigned = lone("Assigned").unwrap_or_else(|| panic!("Assigned"));
    let unassigned = lone("Cn").unwrap_or_else(|| panic!("Cn"));
    assert_eq!(
        merged(
            ranges(assigned)
                .into_iter()
                .chain(ranges(unassigned))
                .collect()
        ),
        [(0, 0x10FFFF)]
    );
    assert!(assigned.contains(0x41) && !assigned.contains(0x0378) && unassigned.contains(0x0378));
}

#[test]
fn script_extensions_follow_their_rule() {
    let aliases = data("PropertyValueAliases.txt");
    let extensions = data("ScriptExtensions.txt");
    // The code points the file lists, each with its scripts by short name.
    let mut listed: Vec<(u32, u32, Vec<String>)> = Vec::new();
    for line in extensions.lines() {
        let data = line.split('#').next().unwrap_or_default();
        let Some((points, scripts)) = data.split_once(';') else {
            continue;
        };
        let points = points.trim();
        let (first, last) = points.split_once("..").unwrap_or((points, points));
        let parse =
            |hex: &str| u32::from_str_radix(hex, 16).unwrap_or_else(|e| panic!("{hex}: {e}"));
        listed.push((
            parse(first),
            parse(last),
            scripts.split_whitespace().map(str::to_owned).collect(),
        ));
    }
    assert_eq!(listed.len(), 206, "the data lines of ScriptExtensions.txt");
    let listed_ranges = merged(listed.iter().map(|(a, b, _)| (*a, *b)).collect());
    for fields in lines_of(&aliases, "sc") {
        let short = &fields[1];
        let script = valued("sc", short).unwrap_or_else(|| panic!("{short}"));
        let extension = valued("scx", short).unwrap_or_else(|| panic!("{short}"));
        // The rule: the code points whose extensions name the script, and those of the script
        // the file does not list.
        let mut expected: Vec<(u32, u32)> = listed
            .iter()
            .filter(|(_, _, scripts)| scripts.contains(short))
            .map(|(a, b, _)| (*a, *b))
            .collect();
        for (first, last) in ranges(script) {
            let mut at = first;
            for &(a, b) in &listed_ranges {
                if b < at || a > last {
                    continue;
                }
                if a > at {
                    expected.push((at, a - 1));
                }
                at = b.saturating_add(1);
            }
            if at <= last {
                expected.push((at, last));
            }
        }
        let expected = merged(expected);
        assert_eq!(ranges(extension), expected, "scx={short}");
        for &(first, last) in &expected {
            assert!(
                extension.contains(first) && extension.contains(last),
                "scx={short}"
            );
        }
    }
    // U+00B7 MIDDLE DOT has the script Common and extensions that name Latin and Greek, not
    // Common.
    let middle_dot = 0xB7;
    assert!(valued("sc", "Zyyy").is_some_and(|p| p.contains(middle_dot)));
    assert!(!valued("scx", "Zyyy").is_some_and(|p| p.contains(middle_dot)));
    assert!(valued("scx", "Latn").is_some_and(|p| p.contains(middle_dot)));
    assert!(valued("scx", "Grek").is_some_and(|p| p.contains(middle_dot)));
    assert!(!valued("sc", "Latn").is_some_and(|p| p.contains(middle_dot)));
    // `Hrkt` has no code point; `Zzzz` is every code point with no script.
    assert_eq!(
        ranges(valued("sc", "Hrkt").unwrap_or_else(|| panic!("Hrkt"))),
        []
    );
    let unknown = valued("sc", "Unknown").unwrap_or_else(|| panic!("Unknown"));
    assert!(unknown.contains(0x0378) && unknown.contains(0xD800) && !unknown.contains(0x41));
}

#[test]
fn the_tables_are_sorted_and_disjoint() {
    let aliases = data("PropertyValueAliases.txt");
    let mut properties: Vec<(String, Property)> = Vec::new();
    for fields in lines_of(&aliases, "gc") {
        properties.push((
            fields[1].clone(),
            lone(&fields[1]).unwrap_or_else(|| panic!("{fields:?}")),
        ));
    }
    for fields in lines_of(&aliases, "sc") {
        for name in ["sc", "scx"] {
            let property = valued(name, &fields[1]).unwrap_or_else(|| panic!("{fields:?}"));
            properties.push((format!("{name}={}", fields[1]), property));
        }
    }
    for names in BINARY {
        properties.push((
            names[0].to_owned(),
            lone(names[0]).unwrap_or_else(|| panic!("{}", names[0])),
        ));
    }
    let mut total = 0usize;
    for (name, property) in &properties {
        let ranges = ranges(*property);
        total += ranges.len();
        let mut previous: Option<u32> = None;
        for &(first, last) in &ranges {
            assert!(
                first <= last && last <= 0x10FFFF,
                "{name}: {first:04X}..{last:04X}"
            );
            if let Some(previous) = previous {
                assert!(
                    first > previous.saturating_add(1),
                    "{name}: {first:04X} not apart"
                );
            }
            previous = Some(last);
            assert!(
                property.contains(first) && property.contains(last),
                "{name}"
            );
            if let Some(before) = first.checked_sub(1) {
                assert!(!property.contains(before), "{name}: {before:04X}");
            }
            assert!(!property.contains(last + 1), "{name}: {:04X}", last + 1);
        }
    }
    assert!(total > 17_000, "{total} ranges");
    for (name, property) in &properties {
        assert!(
            !property.contains(0x110000) && !property.contains(u32::MAX),
            "{name}"
        );
    }
    let any = lone("Any").unwrap_or_else(|| panic!("Any"));
    assert_eq!(ranges(any), [(0, 0x10FFFF)]);
    assert_eq!(
        ranges(lone("ASCII").unwrap_or_else(|| panic!("ASCII"))),
        [(0, 0x7F)]
    );
}

#[test]
fn contains_around_answers_as_contains_over_its_whole_range() {
    // Every property of the three tables, walked from U+0000 range by range: the range
    // `contains_around` gives holds the code point asked, every code point of it answers as the
    // code point does (it lies inside one range of the property, or in a gap between two), and
    // the walk takes a bounded number of ranges, so that a subject read in order is answered with
    // one search per range (`swell-regexp`, the matcher's class).
    let aliases = data("PropertyValueAliases.txt");
    let mut properties: Vec<(String, Property)> = Vec::new();
    for fields in lines_of(&aliases, "gc") {
        properties.push((fields[1].clone(), lone(&fields[1]).expect("gc")));
    }
    for fields in lines_of(&aliases, "sc") {
        for name in ["sc", "scx"] {
            let property = valued(name, &fields[1]).expect("sc");
            properties.push((format!("{name}={}", fields[1]), property));
        }
    }
    for names in BINARY {
        properties.push((names[0].to_owned(), lone(names[0]).expect("binary")));
    }
    assert_eq!(properties.len(), 38 + 2 * 176 + BINARY.len());
    for (name, property) in &properties {
        let union = ranges(*property);
        let mut c = 0u32;
        let mut walked = 0usize;
        loop {
            let (held, first, last) = property.contains_around(c);
            assert!(
                first <= c && c <= last,
                "{name}: U+{c:04X} outside {first:04X}..{last:04X}"
            );
            assert_eq!(held, property.contains(c), "{name}: U+{c:04X}");
            let at = union.partition_point(|&(_, end)| end < first);
            match union.get(at) {
                Some(&(start, end)) if start <= first => assert!(
                    held && last <= end,
                    "{name}: {first:04X}..{last:04X} against {start:04X}..{end:04X}"
                ),
                next => assert!(
                    !held && next.is_none_or(|&(start, _)| last < start),
                    "{name}: {first:04X}..{last:04X} {held} before {next:04X?}"
                ),
            }
            walked += 1;
            if last >= 0x10FFFF {
                break;
            }
            c = last + 1;
        }
        // At most each range of a table the property reads and each gap between them, and the
        // rows of ScriptExtensions.txt for a value of Script_Extensions.
        assert!(
            walked <= 8 * union.len() + 2 * 206 + 2,
            "{name}: {walked} ranges for {}",
            union.len()
        );
        for c in [0x110000, u32::MAX] {
            let (held, first, last) = property.contains_around(c);
            assert!(!held && first <= c && c <= last, "{name}: {c:X}");
        }
    }
}

#[test]
fn id_start_and_id_continue_answer_as_the_data_file() {
    // Each code point with the answers of `DerivedCoreProperties.txt` for `ID_Start` and
    // `ID_Continue`, printed by an awk lookup of its lines; the last twelve are the first and
    // the last code point of the data lines `00F8..01BA`, `1F5F..1F7D` and `10CC0..10CF2` of
    // `ID_Start` and `00D8..00F6`, `0D85..0D96` and `1FF2..1FF4` of `ID_Continue`.
    const ROWS: &[(u32, bool, bool)] = &[
        (0x0024, false, false),
        (0x005F, false, true),
        (0x0030, false, true),
        (0x0061, true, true),
        (0x00AA, true, true),
        (0x00B7, false, true),
        (0x0300, false, true),
        (0x0345, false, true),
        (0x1885, true, true),
        (0x2118, true, true),
        (0x212E, true, true),
        (0x309B, true, true),
        (0x200C, false, true),
        (0x200D, false, true),
        (0x30FB, false, true),
        (0xE0100, false, true),
        (0x10FFFF, false, false),
        (0xD800, false, false),
        (0x0E00, false, false),
        (0x0E3A, false, true),
        (0x1E2AE, false, true),
        (0x00F8, true, true),
        (0x01BA, true, true),
        (0x1F5F, true, true),
        (0x1F7D, true, true),
        (0x10CC0, true, true),
        (0x10CF2, true, true),
        (0x00D8, true, true),
        (0x00F6, true, true),
        (0x0D85, true, true),
        (0x0D96, true, true),
        (0x1FF2, true, true),
        (0x1FF4, true, true),
    ];
    for &(c, start, continues) in ROWS {
        assert_eq!(
            (is_id_start(c), is_id_continue(c)),
            (start, continues),
            "U+{c:04X}"
        );
    }
    let start = lone("ID_Start").unwrap_or_else(|| panic!("ID_Start"));
    let continues = lone("ID_Continue").unwrap_or_else(|| panic!("ID_Continue"));
    for &(c, _, _) in ROWS {
        assert_eq!(start.contains(c), is_id_start(c), "U+{c:04X}");
        assert_eq!(continues.contains(c), is_id_continue(c), "U+{c:04X}");
    }
    assert!(!is_id_start(0x110000) && !is_id_continue(u32::MAX));
}

/// ECMA-262's table of the binary properties of strings
/// (`#table-binary-unicode-properties-of-strings`), which gives no alias.
const OF_STRINGS: &[&str] = &[
    "Basic_Emoji",
    "Emoji_Keycap_Sequence",
    "RGI_Emoji_Modifier_Sequence",
    "RGI_Emoji_Flag_Sequence",
    "RGI_Emoji_Tag_Sequence",
    "RGI_Emoji_ZWJ_Sequence",
    "RGI_Emoji",
];

/// A line of an emoji file: its sequence (empty for a range), its range of single code points,
/// and its property.
type EmojiLine = (Vec<u32>, Option<(u32, u32)>, String);

#[test]
fn the_properties_of_strings_resolve() {
    // Each line of the two emoji files: its code points, a range of single code points or one
    // sequence, and its property.
    let mut lines: Vec<EmojiLine> = Vec::new();
    for file in ["emoji-sequences.txt", "emoji-zwj-sequences.txt"] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/unicode/17.0.0/emoji")
            .join(file);
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        for line in text.lines() {
            let fields: Vec<&str> = line
                .split('#')
                .next()
                .unwrap_or_default()
                .split(';')
                .collect();
            let [points, property, ..] = fields.as_slice() else {
                continue;
            };
            let hex = |text: &str| u32::from_str_radix(text, 16).expect("hex");
            let points = points.trim();
            match points.split_once("..") {
                Some((first, last)) => {
                    lines.push((
                        Vec::new(),
                        Some((hex(first), hex(last))),
                        property.trim().into(),
                    ));
                }
                None => {
                    let sequence: Vec<u32> = points.split(' ').map(hex).collect();
                    lines.push((sequence, None, property.trim().into()));
                }
            }
        }
    }
    assert!(lines.len() > 3000, "{} lines", lines.len());
    let union = string_property(&units("RGI_Emoji")).expect("RGI_Emoji");
    let mut longest_of_all = 0;
    for &name in OF_STRINGS {
        let property = string_property(&units(name)).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(property.name(), name);
        assert_eq!(lone(name), None, "{name} is no property of code points");
        let own: Vec<_> = lines
            .iter()
            .filter(|(_, _, of)| name == "RGI_Emoji" || of == name)
            .collect();
        assert!(!own.is_empty(), "{name}");
        let mut sequences = 0;
        let mut longest = 1;
        for (sequence, range, _) in own {
            if let Some((first, last)) = range {
                for c in [*first, *last, (first + last) / 2] {
                    assert!(property.contains(c), "{name} U+{c:04X}");
                    assert!(union.contains(c), "RGI_Emoji U+{c:04X}");
                }
            } else if let [c] = sequence.as_slice() {
                assert!(property.contains(*c), "{name} U+{c:04X}");
            } else {
                sequences += 1;
                longest = longest.max(sequence.len());
                assert!(
                    property.contains_sequence(sequence),
                    "{name} {sequence:04X?}"
                );
                assert!(
                    union.contains_sequence(sequence),
                    "RGI_Emoji {sequence:04X?}"
                );
                // The matcher folds a sequence under `v` with `i` by trying the case equivalents
                // of the one code point of the window that has some; a sequence with two would
                // need their product.
                let cased = sequence
                    .iter()
                    .filter(|&&c| simple_case_fold_equivalents(c).count() > 1)
                    .count();
                assert!(
                    cased <= 1,
                    "{name} {sequence:04X?}: {cased} code points with a case"
                );
            }
        }
        assert_eq!(property.sequences().count(), sequences, "{name}");
        assert_eq!(property.longest(), longest, "{name}");
        assert!(property.longest() <= LONGEST_STRING, "{name}");
        longest_of_all = longest_of_all.max(longest);
        assert!(!property.contains(u32::from('a')), "{name} a");
        assert!(!property.contains_sequence(&[0x61, 0x62]), "{name} ab");
        assert!(!property.contains_sequence(&[]), "{name} the empty string");
    }
    assert_eq!(LONGEST_STRING, longest_of_all);
    assert_eq!(LONGEST_STRING, 10, "the ZWJ sequences of Unicode 17.0.0");
    assert_eq!(union.longest(), LONGEST_STRING);
    // The folded lookup: Ⓜ️ (U+24C2 U+FE0F) and its folding ⓜ️ (U+24DC U+FE0F).
    let basic = string_property(&units("Basic_Emoji")).expect("Basic_Emoji");
    assert!(basic.contains_sequence(&[0x24C2, 0xFE0F]));
    assert!(!basic.contains_sequence(&[0x24DC, 0xFE0F]));
    assert!(basic.contains_sequence_folded(&[0x24DC, 0xFE0F]));
    assert!(basic.contains_sequence_folded(&[0x24C2, 0xFE0F]));
    assert!(!basic.contains_sequence_folded(&[0x24DC, 0x24DC]));
    assert!(union.contains_sequence_folded(&[0x1F1E9, 0x1F1EA]));
    // No alias, no loose matching, no value.
    for name in [
        "rgi_emoji",
        "RGI Emoji",
        "RGIEmoji",
        "Basic_Emoji=Yes",
        "Emoji",
        "",
    ] {
        assert!(string_property(&units(name)).is_none(), "{name:?}");
    }
}

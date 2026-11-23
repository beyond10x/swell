//! The normalization tables of `swell-unicode` and the four forms of UAX #15 (NFC, NFD, NFKC,
//! NFKD) that `String.prototype.normalize` asks for (`#sec-string.prototype.normalize`): the full
//! decomposition of each code point (canonical, or compatibility for NFKC and NFKD), the
//! canonical ordering of each run of non-starters, and the canonical composition of NFC and
//! NFKC by the primary composites.
//!
//! The rows a test reads are the committed table files, taken into the test as text: what the
//! generator wrote is what the lookups answer.
//!
//! The counts were printed on the vendored data of `data/unicode/17.0.0/ucd/`:
//! `awk -F';' '$6 != "" {n++; if ($6 !~ /^</) {c++; if (split($6, a, " ") == 2) t++}} $4 != "0"
//! {cc++} END {print n, c, t, cc}' UnicodeData.txt` gives 5914 decompositions, 2081 of them
//! canonical, 1046 of those of two code points, and 968 code points whose canonical combining
//! class is not 0; `grep -v '^#' CompositionExclusions.txt | grep -c '[0-9A-F]'` gives its 81
//! code points.

#![cfg(test)]

use std::convert::Infallible;

use swell_unicode::{
    Form, Stop, canonical_combining_class, decomposition, normalize, primary_composite,
};

const DECOMPOSITIONS: &str = include_str!("../src/tables/decompositions.rs");
const COMBINING_CLASS: &str = include_str!("../src/tables/combining_class.rs");
const COMPOSITIONS: &str = include_str!("../src/tables/compositions.rs");
const COMPOSITION_EXCLUSIONS: &str =
    include_str!("../../../data/unicode/17.0.0/ucd/CompositionExclusions.txt");

/// A number of the text of a table: `0x` and hexadecimal digits, or decimal digits.
fn number(text: &str) -> u32 {
    match text.strip_prefix("0x") {
        Some(digits) => u32::from_str_radix(digits, 16).unwrap_or_else(|_| panic!("{text:?}")),
        None => text.parse().unwrap_or_else(|_| panic!("{text:?}")),
    }
}

/// The rows of the constant `name` in `table`: each `(...)` of the array after
/// `pub(crate) const <name>: ... = &[` up to its `];`, on one line or on several, as the
/// numbers it holds in order (those of a nested `&[...]` among them).
fn rows(table: &str, name: &str) -> Vec<Vec<u32>> {
    let head = format!("pub(crate) const {name}:");
    let at = table
        .find(&head)
        .unwrap_or_else(|| panic!("the table has no constant {name}"));
    let constant = &table[at..];
    let start = constant.find("= &[").unwrap() + "= &[".len();
    let end = constant.find("\n];").unwrap();
    let array = &constant[start..end];
    let mut rows = Vec::new();
    let mut depth = 0;
    let mut row_start = 0;
    for (index, c) in array.char_indices() {
        match c {
            '(' => {
                if depth == 0 {
                    row_start = index + 1;
                }
                depth += 1;
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let row = &array[row_start..index];
                    rows.push(
                        row.split(|c: char| !c.is_ascii_alphanumeric())
                            .filter(|token| !token.is_empty())
                            .map(number)
                            .collect(),
                    );
                }
            }
            _ => {}
        }
    }
    assert_eq!(depth, 0, "{name}");
    rows
}

/// The rows of a decomposition table: a code point and its one-level decomposition.
fn mappings(name: &str) -> Vec<(u32, Vec<u32>)> {
    rows(DECOMPOSITIONS, name)
        .into_iter()
        .map(|row| (row[0], row[1..].to_vec()))
        .collect()
}

/// `input` in the form `form`, with buffers that take every code point.
fn nf(form: Form, input: &[u32]) -> Vec<u32> {
    let mut decomposed: Vec<u32> = Vec::new();
    let mut out: Vec<u32> = Vec::new();
    let done: Result<(), Stop<Infallible>> =
        normalize(form, input.iter().copied(), &mut decomposed, &mut out);
    assert_eq!(done, Ok(()), "{input:04X?}");
    out
}

/// The code points of a column of a row: hexadecimal code points separated by spaces.
fn column(text: &str) -> Vec<u32> {
    text.split_whitespace()
        .map(|point| u32::from_str_radix(point, 16).unwrap())
        .collect()
}

/// Rows of the form of `NormalizationTest.txt` (that file is not vendored), written for this
/// test from the decompositions, combining classes and exclusions of the vendored
/// `UnicodeData.txt`, `CompositionExclusions.txt` and `DerivedNormalizationProps.txt`: the
/// source, its NFC, NFD, NFKC and NFKD. The first rows are those of Part 0 of that file and
/// of the figures of UAX #15.
const ROWS: &[&str] = &[
    // Latin with combining marks, in and out of canonical order.
    "1E0A;1E0A;0044 0307;1E0A;0044 0307",
    "1E0C;1E0C;0044 0323;1E0C;0044 0323",
    "1E0A 0323;1E0C 0307;0044 0323 0307;1E0C 0307;0044 0323 0307",
    "1E0C 0307;1E0C 0307;0044 0323 0307;1E0C 0307;0044 0323 0307",
    "0044 0307 0323;1E0C 0307;0044 0323 0307;1E0C 0307;0044 0323 0307",
    "1E0E 0307;1E0E 0307;0044 0331 0307;1E0E 0307;0044 0331 0307",
    "0061 0323 0302;1EAD;0061 0323 0302;1EAD;0061 0323 0302",
    "0061 0302 0323;1EAD;0061 0323 0302;1EAD;0061 0323 0302",
    "0041 0301 0300;00C1 0300;0041 0301 0300;00C1 0300;0041 0301 0300",
    "0071 0307 0323;0071 0323 0307;0071 0323 0307;0071 0323 0307;0071 0323 0307",
    "1E14;1E14;0045 0304 0300;1E14;0045 0304 0300",
    "0045 0304 0300;1E14;0045 0304 0300;1E14;0045 0304 0300",
    "1E08;1E08;0043 0327 0301;1E08;0043 0327 0301",
    "0043 0301 0327;1E08;0043 0327 0301;1E08;0043 0327 0301",
    "00C7 0301;1E08;0043 0327 0301;1E08;0043 0327 0301",
    "0106 0327;1E08;0043 0327 0301;1E08;0043 0327 0301",
    "00C5;00C5;0041 030A;00C5;0041 030A",
    "0301 0041 0301 0041 0301;0301 00C1 00C1;0301 0041 0301 0041 0301;0301 00C1 00C1;0301 0041 0301 0041 0301",
    "0061 0062 0063;0061 0062 0063;0061 0062 0063;0061 0062 0063;0061 0062 0063",
    // Greek, decomposed through three levels.
    "1F83;1F83;03B1 0314 0300 0345;1F83;03B1 0314 0300 0345",
    "03B1 0345 0314 0300;1F83;03B1 0314 0300 0345;1F83;03B1 0314 0300 0345",
    "0390;0390;03B9 0308 0301;0390;03B9 0308 0301",
    // Hangul, by the arithmetic of chapter 3.
    "AC00;AC00;1100 1161;AC00;1100 1161",
    "AC01;AC01;1100 1161 11A8;AC01;1100 1161 11A8",
    "1100 1161 11A8;AC01;1100 1161 11A8;AC01;1100 1161 11A8",
    "AC00 11A8;AC01;1100 1161 11A8;AC01;1100 1161 11A8",
    "D4DB;D4DB;1111 1171 11B6;D4DB;1111 1171 11B6",
    "1100 0300 1161;1100 0300 1161;1100 0300 1161;1100 0300 1161;1100 0300 1161",
    // Two starters that compose.
    "0B47 0B3E;0B4B;0B47 0B3E;0B4B;0B47 0B3E",
    // Singletons.
    "212B;00C5;0041 030A;00C5;0041 030A",
    "2126;03A9;03A9;03A9;03A9",
    "2FA1D;2A600;2A600;2A600;2A600",
    // Composition exclusions and non-starter decompositions.
    "0958;0915 093C;0915 093C;0915 093C;0915 093C",
    "0915 093C;0915 093C;0915 093C;0915 093C;0915 093C",
    "0344;0308 0301;0308 0301;0308 0301;0308 0301",
    "0F73;0F71 0F72;0F71 0F72;0F71 0F72;0F71 0F72",
    "1D15E;1D157 1D165;1D157 1D165;1D157 1D165;1D157 1D165",
    "FB2C;05E9 05BC 05C1;05E9 05BC 05C1;05E9 05BC 05C1;05E9 05BC 05C1",
    // Compatibility ligatures and other compatibility decompositions.
    "FB01;FB01;FB01;0066 0069;0066 0069",
    "1E9B 0323;1E9B 0323;017F 0323 0307;1E69;0073 0323 0307",
    "1E9B;1E9B;017F 0307;1E61;0073 0307",
    "01C4;01C4;01C4;0044 017D;0044 005A 030C",
    "2460;2460;2460;0031;0031",
    "00BD;00BD;00BD;0031 2044 0032;0031 2044 0032",
    "3392;3392;3392;004D 0048 007A;004D 0048 007A",
    "FF21;FF21;FF21;0041;0041",
    "FDFA;FDFA;FDFA;0635 0644 0649 0020 0627 0644 0644 0647 0020 0639 0644 064A 0647 0020 0648 0633 0644 0645;0635 0644 0649 0020 0627 0644 0644 0647 0020 0639 0644 064A 0647 0020 0648 0633 0644 0645",
    // A lone surrogate is a code point with no decomposition and class 0.
    "D800;D800;D800;D800;D800",
    "D800 0301 DC00;D800 0301 DC00;D800 0301 DC00;D800 0301 DC00;D800 0301 DC00",
];

/// Each row holds as `NormalizationTest.txt` says its rows hold: c2 = NFC(c1) = NFC(c2) =
/// NFC(c3) and c4 = NFC(c4) = NFC(c5); c3 = NFD(c1) = NFD(c2) = NFD(c3) and c5 = NFD(c4) =
/// NFD(c5); c4 = NFKC(c1) = ... = NFKC(c5); c5 = NFKD(c1) = ... = NFKD(c5). The empty
/// sequence is each of its forms.
#[test]
fn the_four_forms_of_rows_written_for_the_test() {
    assert!(ROWS.len() >= 40, "a few dozen rows");
    for row in ROWS {
        let c: Vec<Vec<u32>> = row.split(';').map(column).collect();
        assert_eq!(c.len(), 5, "{row}");
        for source in &c[0..3] {
            assert_eq!(nf(Form::Nfc, source), c[1], "NFC {row}");
            assert_eq!(nf(Form::Nfd, source), c[2], "NFD {row}");
        }
        for source in &c[3..5] {
            assert_eq!(nf(Form::Nfc, source), c[3], "NFC {row}");
            assert_eq!(nf(Form::Nfd, source), c[4], "NFD {row}");
        }
        for source in &c {
            assert_eq!(nf(Form::Nfkc, source), c[3], "NFKC {row}");
            assert_eq!(nf(Form::Nfkd, source), c[4], "NFKD {row}");
        }
    }
    for form in [Form::Nfc, Form::Nfd, Form::Nfkc, Form::Nfkd] {
        assert_eq!(nf(form, &[]), Vec::<u32>::new());
    }
}

/// The canonical ordering algorithm (UAX #15, section 3, D108 and D109 of chapter 3): each run
/// of non-starters is sorted by canonical combining class, and code points of one class keep
/// their order; a starter ends a run. Also a long run whose classes alternate.
#[test]
fn canonical_ordering_sorts_each_run_of_non_starters() {
    assert_eq!(canonical_combining_class(0x0041), 0);
    assert_eq!(canonical_combining_class(0x0301), 230);
    assert_eq!(canonical_combining_class(0x0316), 220);
    assert_eq!(canonical_combining_class(0x0315), 232);
    assert_eq!(canonical_combining_class(0x0345), 240);
    assert_eq!(canonical_combining_class(0x05B0), 10);
    assert_eq!(canonical_combining_class(0x093C), 7);
    assert_eq!(canonical_combining_class(0x1D165), 216);
    for point in [0x0000, 0xAC00, 0xD800, 0xDC00, 0x10FFFF, 0x110000, u32::MAX] {
        assert_eq!(canonical_combining_class(point), 0, "{point:04X}");
    }
    // 230, 220, 230, 232: 220 first, the two of 230 in their order, then 232.
    assert_eq!(
        nf(Form::Nfd, &[0x0061, 0x0301, 0x0316, 0x0300, 0x0315]),
        [0x0061, 0x0316, 0x0301, 0x0300, 0x0315]
    );
    // Two runs, one before any starter: each sorted on its own.
    assert_eq!(
        nf(Form::Nfd, &[0x0301, 0x0316, 0x0062, 0x0345, 0x0323, 0x05B0]),
        [0x0316, 0x0301, 0x0062, 0x05B0, 0x0323, 0x0345]
    );
    // A run already in order, and one of a single class, are kept.
    assert_eq!(
        nf(Form::Nfd, &[0x0062, 0x05B0, 0x0323, 0x0301, 0x0345]),
        [0x0062, 0x05B0, 0x0323, 0x0301, 0x0345]
    );
    assert_eq!(
        nf(Form::Nfd, &[0x0062, 0x0301, 0x0300, 0x0308]),
        [0x0062, 0x0301, 0x0300, 0x0308]
    );
    // A decomposition's marks join the run around it: U+00E9 (e and U+0301) before U+0316.
    assert_eq!(nf(Form::Nfd, &[0x00E9, 0x0316]), [0x0065, 0x0316, 0x0301]);
    // 2,000 marks of 230 and 220 alternating, each class in its order of the input.
    let mut long = vec![0x0062];
    for index in 0..1_000_u32 {
        long.push(if index % 2 == 0 { 0x0301 } else { 0x0300 });
        long.push(if index % 3 == 0 { 0x0316 } else { 0x0323 });
    }
    let below: Vec<u32> = long[1..]
        .iter()
        .copied()
        .filter(|&point| canonical_combining_class(point) == 220)
        .collect();
    let above: Vec<u32> = long[1..]
        .iter()
        .copied()
        .filter(|&point| canonical_combining_class(point) == 230)
        .collect();
    let mut expected = vec![0x0062];
    expected.extend(&below);
    expected.extend(&above);
    assert_eq!(nf(Form::Nfd, &long), expected);
    // Composed after the ordering: b has no composite, so NFC keeps the order of NFD.
    assert_eq!(nf(Form::Nfc, &long), expected);
}

/// The two decomposition tables as rows.
struct Tables {
    canonical: Vec<(u32, Vec<u32>)>,
    compat: Vec<(u32, Vec<u32>)>,
}

impl Tables {
    /// The one-level decomposition of `point`: by the compatibility rows first for
    /// `compatibility`, else by the canonical rows.
    fn one(&self, point: u32, compatibility: bool) -> Option<&[u32]> {
        let find =
            |table: &'_ [(u32, Vec<u32>)]| table.binary_search_by_key(&point, |(key, _)| *key).ok();
        if compatibility && let Some(at) = find(&self.compat) {
            return Some(&self.compat[at].1);
        }
        find(&self.canonical).map(|at| self.canonical[at].1.as_slice())
    }

    /// The full decomposition of `point` by the rows, expanded by recursion: the reference the
    /// library's loop is held against. `deepest` is raised to the levels it took.
    fn reference(
        &self,
        point: u32,
        compatibility: bool,
        level: usize,
        deepest: &mut usize,
    ) -> Vec<u32> {
        match self.one(point, compatibility) {
            Some(points) => {
                *deepest = (*deepest).max(level + 1);
                points
                    .iter()
                    .copied()
                    .flat_map(|p| self.reference(p, compatibility, level + 1, deepest))
                    .collect()
            }
            None => vec![point],
        }
    }
}

/// Each code point of the decomposition tables decomposes fully by the library's explicit
/// stack, with no call per level, to what the recursion over the rows gives, in the same order
/// (the ordering comes after); the deepest takes three levels (U+1F83 to U+1F03 U+0345, U+1F01
/// U+0300 U+0345, U+03B1 U+0314 U+0300 U+0345), and no code point of a full decomposition has
/// one. Run on a thread with a stack of 256 KiB: the loop and 20,000 code points of the deepest
/// decomposition need no more.
#[test]
fn a_deep_decomposition_uses_no_recursion() {
    let tables = Tables {
        canonical: mappings("CANONICAL"),
        compat: mappings("COMPATIBILITY"),
    };
    let handle = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let mut deepest = 0;
            for (form, compatibility) in [(Form::Nfd, false), (Form::Nfkd, true)] {
                let keys: Vec<u32> = tables
                    .canonical
                    .iter()
                    .chain(tables.compat.iter().filter(|_| compatibility))
                    .map(|(key, _)| *key)
                    .chain([0x0041, 0xAC00, 0xD7A3, 0xD800])
                    .collect();
                for point in keys {
                    let mut full: Vec<u32> = Vec::new();
                    let done: Result<(), Stop<Infallible>> =
                        swell_unicode::decompose(form, [point], &mut full);
                    assert_eq!(done, Ok(()), "{point:04X}");
                    if !(0xAC00..=0xD7A3).contains(&point) {
                        let expected = tables.reference(point, compatibility, 0, &mut deepest);
                        assert_eq!(full, expected, "{point:04X}");
                    }
                    for &p in &full {
                        assert_eq!(
                            decomposition(p, compatibility),
                            None,
                            "{point:04X}: {p:04X}"
                        );
                        assert!(
                            tables.one(p, compatibility).is_none(),
                            "{point:04X}: {p:04X}"
                        );
                    }
                }
            }
            assert_eq!(deepest, 3);
            let long = vec![0x1F83; 5_000];
            assert_eq!(nf(Form::Nfd, &long).len(), 20_000);
            assert_eq!(nf(Form::Nfc, &long), long);
        })
        .unwrap();
    handle.join().unwrap();
}

/// The tables as the generator wrote them: the decompositions sorted by code point with no
/// code point in both tables, a canonical decomposition of one or two code points, a
/// compatibility one of one to 18; the combining classes sorted, disjoint and merged ranges of
/// classes 1 to 254 that cover the 968 code points; the primary composites sorted by their pair,
/// each the canonical decomposition of its composite, none of the 81 code points of
/// `CompositionExclusions.txt`, and each of as many UTF-16 code units as the first of its pair.
#[test]
fn the_tables_are_sorted_and_disjoint() {
    let canonical = mappings("CANONICAL");
    let compat = mappings("COMPATIBILITY");
    assert_eq!(canonical.len(), 2081);
    assert_eq!(canonical.len() + compat.len(), 5914);
    assert_eq!(
        canonical
            .iter()
            .filter(|(_, points)| points.len() == 2)
            .count(),
        1046
    );
    for (name, table, most) in [("CANONICAL", &canonical, 2), ("COMPATIBILITY", &compat, 18)] {
        assert!(table.windows(2).all(|w| w[0].0 < w[1].0), "{name}");
        assert!(
            table
                .iter()
                .all(|(_, points)| (1..=most).contains(&points.len())),
            "{name}"
        );
        assert_eq!(
            table.iter().map(|(_, points)| points.len()).max(),
            Some(most),
            "{name}"
        );
        for (key, points) in table.iter() {
            assert_eq!(
                decomposition(*key, name == "COMPATIBILITY"),
                Some(points.as_slice()),
                "{name} {key:04X}"
            );
        }
    }
    for (key, _) in &compat {
        assert!(
            canonical.binary_search_by_key(key, |(k, _)| *k).is_err(),
            "{key:04X} is in both tables"
        );
        assert_eq!(decomposition(*key, false), None, "{key:04X}");
    }

    let classes = rows(COMBINING_CLASS, "COMBINING_CLASS");
    let mut covered = 0;
    for range in &classes {
        let [first, last, class] = range.as_slice() else {
            panic!("{range:04X?}");
        };
        assert!(first <= last && (1..=254).contains(class), "{range:04X?}");
        covered += last - first + 1;
        for point in [*first, *last] {
            assert_eq!(u32::from(canonical_combining_class(point)), *class);
        }
        assert_ne!(u32::from(canonical_combining_class(last + 1)), *class);
    }
    assert_eq!(covered, 968);
    for w in classes.windows(2) {
        // Disjoint, and merged: two ranges that touch have different classes.
        assert!(w[0][1] < w[1][0], "{:04X?}", w);
        assert!(w[0][1] + 1 < w[1][0] || w[0][2] != w[1][2], "{:04X?}", w);
    }

    let composites = rows(COMPOSITIONS, "COMPOSITIONS");
    let excluded: Vec<u32> = COMPOSITION_EXCLUSIONS
        .lines()
        .map(|line| line.split('#').next().unwrap().trim())
        .filter(|data| !data.is_empty())
        .map(|data| u32::from_str_radix(data, 16).unwrap())
        .collect();
    assert_eq!(excluded.len(), 81);
    assert!(
        composites
            .windows(2)
            .all(|w| (w[0][0], w[0][1]) < (w[1][0], w[1][1]))
    );
    let units = |point: u32| if point > 0xFFFF { 2 } else { 1 };
    for row in &composites {
        let [first, second, composite] = row.as_slice() else {
            panic!("{row:04X?}");
        };
        assert_eq!(
            decomposition(*composite, false),
            Some([*first, *second].as_slice()),
            "{composite:04X}"
        );
        assert!(!excluded.contains(composite), "{composite:04X}");
        assert_eq!(units(*first), units(*composite), "{composite:04X}");
        assert_eq!(primary_composite(*first, *second), Some(*composite));
    }
    // A pair with no composite, an excluded one, and the Hangul syllables by arithmetic.
    assert_eq!(primary_composite(0x0071, 0x0307), None);
    assert_eq!(primary_composite(0x0915, 0x093C), None);
    assert_eq!(primary_composite(0x1100, 0x1161), Some(0xAC00));
    assert_eq!(primary_composite(0xAC00, 0x11A8), Some(0xAC01));
    assert_eq!(primary_composite(0xAC01, 0x11A8), None);
    assert_eq!(primary_composite(0xAC00, 0x11A7), None);
}

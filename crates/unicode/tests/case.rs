//! The case tables of `swell-unicode` and their lookups: the full mappings `to_lowercase` and
//! `to_uppercase` (`SpecialCasing.txt` first, the simple mapping of `UnicodeData.txt` else),
//! the lowercase mapping in the context `Final_Sigma`, and the properties `Cased` and
//! `Case_Ignorable` that context reads (Unicode chapter 3, section 3.13, table 3-17); and the
//! simple case folding of `CaseFolding.txt` with the code points of one folding, which
//! Canonicalize of a regular expression reads under `u` and `i`.
//!
//! The rows a test reads are the committed table files, taken into the test as text: what the
//! generator wrote is what the lookups answer.
//!
//! The counts were printed on the vendored data of `data/unicode/17.0.0/ucd/`:
//! `awk -F';' '$13 != "" {u++} $14 != "" {l++} END {print u, l}' UnicodeData.txt` gives 1505
//! simple uppercase and 1488 simple lowercase mappings; of the 119 data lines of
//! `SpecialCasing.txt`, `grep -v '^#' SpecialCasing.txt | grep . | awk -F'#' '{split($1, a,
//! ";"); if (a[5] ~ /[A-Za-z]/) c++; else u++} END {print c, u}'` gives 16 with a condition
//! (`Final_Sigma` and 15 of a language) and 103 without.

#![cfg(test)]

use swell_unicode::{
    MOST_CASE_FOLD_EQUIVALENTS, final_sigma_lowercase, is_case_ignorable, is_cased,
    simple_case_fold, simple_case_fold_equivalents, to_lowercase, to_uppercase,
};

const CASE_FOLDING: &str = include_str!("../src/tables/case_folding.rs");
const LOWERCASE: &str = include_str!("../src/tables/lowercase.rs");
const UPPERCASE: &str = include_str!("../src/tables/uppercase.rs");
const SPECIAL_CASING: &str = include_str!("../src/tables/special_casing.rs");
const CASED: &str = include_str!("../src/tables/cased.rs");
const CASE_IGNORABLE: &str = include_str!("../src/tables/case_ignorable.rs");

/// A number of the text of a table, `0x` and hexadecimal digits.
fn hex(text: &str) -> u32 {
    let digits = text
        .trim()
        .strip_prefix("0x")
        .unwrap_or_else(|| panic!("{text:?}"));
    u32::from_str_radix(digits, 16).unwrap_or_else(|_| panic!("{text:?}"))
}

/// The rows of the constant `name` in `table`, written on one line or one row per line: each
/// `(<key>, <rest>)` of the array after `pub(crate) const <name>: ... = &[` up to its `];`, as
/// the key and the numbers of the rest (one number, or the numbers of `&[...]`).
fn rows(table: &str, name: &str) -> Vec<(u32, Vec<u32>)> {
    let head = format!("pub(crate) const {name}:");
    let at = table
        .find(&head)
        .unwrap_or_else(|| panic!("the table has no constant {name}"));
    let constant = &table[at..];
    let start = constant.find("= &[").unwrap() + "= &[".len();
    let end = constant.find("];").unwrap();
    let array = &constant[start..end];
    // Each row is the text between a `(` and its `)` at the array's own level.
    let mut rows = Vec::new();
    let mut depth = 0;
    let mut row_start = 0;
    for (index, c) in array.char_indices() {
        match c {
            '(' | '[' => {
                if depth == 0 {
                    row_start = index + 1;
                }
                depth += 1;
            }
            ')' | ']' => {
                depth -= 1;
                if depth == 0 {
                    let row = &array[row_start..index];
                    let (key, rest) = row.split_once(", ").unwrap_or_else(|| panic!("{row:?}"));
                    let rest = rest.trim_start_matches("&[").trim_end_matches(']');
                    rows.push((hex(key), rest.split(", ").map(hex).collect()));
                }
            }
            _ => {}
        }
    }
    assert_eq!(depth, 0, "{name}: {array:?}");
    rows
}

/// The pairs of a constant of pairs.
fn pairs(table: &str, name: &str) -> Vec<(u32, u32)> {
    rows(table, name)
        .into_iter()
        .map(|(key, rest)| match rest.as_slice() {
            [value] => (key, *value),
            _ => panic!("{name}: {key:04X} is not a pair"),
        })
        .collect()
}

/// The keys of the rows of `SPECIAL_CASING`'s constant `name`.
fn special_keys(name: &str) -> Vec<u32> {
    rows(SPECIAL_CASING, name)
        .into_iter()
        .map(|(key, _)| key)
        .collect()
}

#[test]
fn simple_mappings_of_every_table_row_round_trip_where_the_data_says_so() {
    let lowercase = pairs(LOWERCASE, "LOWERCASE");
    let uppercase = pairs(UPPERCASE, "UPPERCASE");
    assert_eq!((lowercase.len(), uppercase.len()), (1488, 1505));
    let special_lower = special_keys("LOWERCASE");
    let special_upper = special_keys("UPPERCASE");

    // Every row is what the lookup answers, where SpecialCasing.txt has no line for its key.
    for &(point, lower) in &lowercase {
        if !special_lower.contains(&point) {
            assert_eq!(to_lowercase(point).as_slice(), [lower], "{point:04X}");
        }
    }
    for &(point, upper) in &uppercase {
        if !special_upper.contains(&point) {
            assert_eq!(to_uppercase(point).as_slice(), [upper], "{point:04X}");
        }
    }

    // Where the data maps a code point to its lowercase and that lowercase back to it, the
    // lookups round trip, unless SpecialCasing.txt maps one of the two (U+1F88 to U+1F80, whose
    // full uppercase is U+1F08 U+0399). The data says so for 1,478 of the 1,488 rows (`awk -F';'
    // '{ if ($14 != "") low[$1] = $14; if ($13 != "") up[$1] = $13 } END { for (p in low) if
    // (up[low[p]] == p) r++; print r }' UnicodeData.txt`); not, for instance, for U+0130 and
    // U+212A, whose lowercase U+0069 and U+006B map to U+0049 and U+004B.
    let mut round_trips = 0;
    for &(point, lower) in &lowercase {
        if uppercase.binary_search(&(lower, point)).is_err() {
            continue;
        }
        round_trips += 1;
        if special_lower.contains(&point) || special_upper.contains(&lower) {
            continue;
        }
        assert_eq!(to_lowercase(point).as_slice(), [lower], "{point:04X}");
        assert_eq!(to_uppercase(lower).as_slice(), [point], "{lower:04X}");
    }
    assert_eq!(round_trips, 1478);
    assert_eq!(
        to_uppercase(to_lowercase(0x0130).as_slice()[0]).as_slice(),
        [0x0049]
    );
    assert_eq!(
        to_uppercase(to_lowercase(0x212A).as_slice()[0]).as_slice(),
        [0x004B]
    );
}

#[test]
fn special_casing_maps_sharp_s_and_the_ligatures_to_several_code_points() {
    assert_eq!(
        (
            special_keys("LOWERCASE").len(),
            special_keys("UPPERCASE").len()
        ),
        (103, 103)
    );
    assert_eq!(to_uppercase(0x00DF).as_slice(), [0x0053, 0x0053]);
    assert_eq!(to_lowercase(0x00DF).as_slice(), [0x00DF]);
    assert_eq!(to_uppercase(0xFB00).as_slice(), [0x0046, 0x0046]);
    assert_eq!(to_uppercase(0xFB03).as_slice(), [0x0046, 0x0046, 0x0049]);
    assert_eq!(to_uppercase(0xFB06).as_slice(), [0x0053, 0x0054]);
    assert_eq!(to_uppercase(0x0587).as_slice(), [0x0535, 0x0552]);
    assert_eq!(to_uppercase(0x0390).as_slice(), [0x0399, 0x0308, 0x0301]);
    assert_eq!(to_uppercase(0x1F80).as_slice(), [0x1F08, 0x0399]);
    // A titlecase letter: its full uppercase is two code points, its lowercase the simple one.
    assert_eq!(to_uppercase(0x1F88).as_slice(), [0x1F08, 0x0399]);
    assert_eq!(to_lowercase(0x1F88).as_slice(), [0x1F80]);
    // U+0130: its full lowercase is two code points, not the simple U+0069.
    assert_eq!(to_lowercase(0x0130).as_slice(), [0x0069, 0x0307]);
    assert_eq!(to_uppercase(0x0130).as_slice(), [0x0130]);
    // A row of every table: an ASCII letter, a Latin-1 letter, a supplementary-plane letter.
    assert_eq!(to_lowercase(0x0041).as_slice(), [0x0061]);
    assert_eq!(to_uppercase(0x00E9).as_slice(), [0x00C9]);
    assert_eq!(to_uppercase(0x10428).as_slice(), [0x10400]);
    assert_eq!(to_lowercase(0x10400).as_slice(), [0x10428]);
}

#[test]
fn final_sigma_reads_its_context() {
    let final_sigma = rows(SPECIAL_CASING, "FINAL_SIGMA");
    assert_eq!(final_sigma, [(0x03A3, vec![0x03C2])]);
    assert_eq!(
        final_sigma_lowercase(0x03A3).map(|m| m.as_slice().to_vec()),
        Some(vec![0x03C2])
    );
    assert_eq!(to_lowercase(0x03A3).as_slice(), [0x03C3]);
    assert_eq!(to_uppercase(0x03C2).as_slice(), [0x03A3]);
    for point in [0x0041, 0x03C3, 0x03C2, 0x0000, 0x110000] {
        assert_eq!(final_sigma_lowercase(point), None, "{point:04X}");
    }
    // What the context reads before and after U+03A3: a cased letter, after any case-ignorable
    // code points.
    for point in [
        0x0041, 0x0061, 0x03A3, 0x0391, 0x00DF, 0x1D4A2, 0x02B0, 0x0345,
    ] {
        assert!(is_cased(point), "{point:04X} is cased");
    }
    for point in [
        0x002E, 0x0027, 0x00AD, 0x0345, 0x02B0, 0x180E, 0x1D242, 0x0300,
    ] {
        assert!(is_case_ignorable(point), "{point:04X} is case-ignorable");
    }
    for point in [0x0020, 0x0030, 0x4E00, 0xD800, 0x10FFFF, 0x110000, u32::MAX] {
        assert!(!is_cased(point) && !is_case_ignorable(point), "{point:04X}");
    }
    for point in [0x002E, 0x180E, 0x1D242] {
        assert!(!is_cased(point), "{point:04X} is not cased");
    }
}

#[test]
fn a_code_point_with_no_mapping_maps_to_itself() {
    for point in [
        0x0000,
        0x0030,
        0x0020,
        0x00AA,
        0x0307,
        0x4E00,
        0xAC00,
        0xD800,
        0xDC00,
        0xFFFF,
        0x1F600,
        0x10FFFF,
        0x110000,
        u32::MAX,
    ] {
        assert_eq!(to_lowercase(point).as_slice(), [point], "{point:04X}");
        assert_eq!(to_uppercase(point).as_slice(), [point], "{point:04X}");
    }
    // The lines of a language are not written: I and i map as the simple mappings do.
    assert_eq!(to_lowercase(0x0049).as_slice(), [0x0069]);
    assert_eq!(to_uppercase(0x0069).as_slice(), [0x0049]);
    assert_eq!(to_lowercase(0x00CC).as_slice(), [0x00EC]);
    // A letter that is already of the case asked for.
    assert_eq!(to_lowercase(0x0061).as_slice(), [0x0061]);
    assert_eq!(to_uppercase(0x0041).as_slice(), [0x0041]);
}

#[test]
fn the_tables_are_sorted_and_disjoint() {
    for (table, name) in [(LOWERCASE, "LOWERCASE"), (UPPERCASE, "UPPERCASE")] {
        let keys: Vec<u32> = pairs(table, name).into_iter().map(|(key, _)| key).collect();
        assert!(keys.windows(2).all(|w| w[0] < w[1]), "{name}");
    }
    for name in ["LOWERCASE", "UPPERCASE", "FINAL_SIGMA"] {
        let rows = rows(SPECIAL_CASING, name);
        assert!(
            rows.windows(2).all(|w| w[0].0 < w[1].0),
            "special casing {name}"
        );
        assert!(
            rows.iter()
                .all(|(_, points)| (1..=3).contains(&points.len())),
            "{name}"
        );
    }
    for (table, name, count) in [
        (CASED, "CASED", 158),
        (CASE_IGNORABLE, "CASE_IGNORABLE", 464),
    ] {
        let ranges = pairs(table, name);
        assert_eq!(ranges.len(), count, "{name}");
        assert!(ranges.iter().all(|(first, last)| first <= last), "{name}");
        // Disjoint and merged: a range starts past the code point after the one before.
        assert!(ranges.windows(2).all(|w| w[0].1 + 1 < w[1].0), "{name}");
        for &(first, last) in &ranges {
            let member = if name == "CASED" {
                is_cased
            } else {
                is_case_ignorable
            };
            assert!(
                member(first) && member(last),
                "{name} {first:04X}..{last:04X}"
            );
            if first > 0 {
                assert!(!member(first - 1), "{name} {:04X}", first - 1);
            }
            assert!(!member(last + 1), "{name} {:04X}", last + 1);
        }
    }
}

/// The simple case folding of `CaseFolding.txt`: its lines of status `C` and `S`, 1,512 on the
/// vendored data (`awk -F'; ' '$2 == "C" || $2 == "S"' CaseFolding.txt | wc -l`). Every row is
/// what `simple_case_fold` answers and every other code point folds to itself; the code points
/// of one folding are what `simple_case_fold_equivalents` answers, the folding first, at most
/// `MOST_CASE_FOLD_EQUIVALENTS` of them, the number the table states.
#[test]
fn simple_case_folding_follows_its_table() {
    let folding = pairs(CASE_FOLDING, "FOLDING");
    assert_eq!(folding.len(), 1512);
    assert!(folding.windows(2).all(|w| w[0].0 < w[1].0));
    let mut grouped: Vec<(u32, u32)> = folding.iter().map(|&(point, f)| (f, point)).collect();
    grouped.sort_unstable();
    assert_eq!(pairs(CASE_FOLDING, "BY_FOLDING"), grouped);
    // Every key lies below U+20000: each code point up to there, and values past the last.
    assert!(folding.iter().all(|&(point, _)| point < 0x20000));
    for point in (0..0x20000).chain([0x10FFFF, 0x110000, u32::MAX]) {
        let row = folding.binary_search_by_key(&point, |&(key, _)| key);
        let expected = row.map_or(point, |at| folding[at].1);
        assert_eq!(simple_case_fold(point), expected, "{point:04X}");
    }
    // A folding is its own folding, and a code point and its folding have one length in UTF-16.
    let mut largest = 1;
    for &(point, f) in &folding {
        assert_eq!(simple_case_fold(f), f, "{point:04X}");
        assert_eq!(point > 0xFFFF, f > 0xFFFF, "{point:04X}");
        let group: Vec<u32> = simple_case_fold_equivalents(point).collect();
        let others: Vec<u32> = grouped
            .iter()
            .filter(|&&(key, _)| key == f)
            .map(|&(_, other)| other)
            .collect();
        assert_eq!(group.first(), Some(&f), "{point:04X}");
        assert_eq!(group[1..], others[..], "{point:04X}");
        assert_eq!(simple_case_fold_equivalents(f).collect::<Vec<_>>(), group);
        largest = largest.max(group.len());
    }
    assert_eq!(largest, MOST_CASE_FOLD_EQUIVALENTS);
    assert_eq!(MOST_CASE_FOLD_EQUIVALENTS, 4);
    assert!(CASE_FOLDING.contains("pub(crate) const LARGEST_GROUP: usize = 4;\n"));
    // Rows the regular expressions read: ASCII, the long s and the Kelvin sign into ASCII, the
    // capital sharp s, the final sigma, a supplementary plane; no simple folding for U+0130
    // (its lines are of status F and T) nor for ß.
    for (point, f) in [
        (0x0041, 0x0061),
        (0x0061, 0x0061),
        (0x017F, 0x0073),
        (0x212A, 0x006B),
        (0x1E9E, 0x00DF),
        (0x00DF, 0x00DF),
        (0x0130, 0x0130),
        (0x0131, 0x0131),
        (0x03C2, 0x03C3),
        (0x10400, 0x10428),
        (0x0030, 0x0030),
    ] {
        assert_eq!(simple_case_fold(point), f, "{point:04X}");
    }
    let group = |point: u32| simple_case_fold_equivalents(point).collect::<Vec<_>>();
    assert_eq!(group(0x212A), [0x006B, 0x004B, 0x212A]);
    assert_eq!(group(0x0073), [0x0073, 0x0053, 0x017F]);
    assert_eq!(group(0x03F4), [0x03B8, 0x0398, 0x03D1, 0x03F4]);
    assert_eq!(group(0x0030), [0x0030]);
    assert_eq!(group(0x110000), [0x110000]);
}

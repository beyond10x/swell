//! The normalization data of the tables of `swell-unicode` (design, decision 8; UAX #15,
//! Unicode Normalization Forms): the decompositions of field 5 of `UnicodeData.txt`, canonical
//! and compatibility, one level each (the full decomposition is a loop at run time); the
//! canonical combining classes of field 3 as ranges of one class; and the primary composites,
//! the canonical decompositions of two code points of every code point that
//! `Full_Composition_Exclusion` of `DerivedNormalizationProps.txt` does not name, checked
//! against `CompositionExclusions.txt`. The Hangul syllables are decomposed and composed by the
//! arithmetic of chapter 3, section 3.12, so no table holds them.
//!
//! The renderers of these tables are in `tables.rs`; the text of a table of rows of several
//! code points is written here, as rustfmt writes it.

use crate::ucd::{Ucd, code_point};

/// The first Hangul syllable, and the number of them (Unicode chapter 3, section 3.12).
const S_BASE: u32 = 0xAC00;
const S_COUNT: u32 = 11_172;

/// The most code points a compatibility decomposition of one level has in the vendored data,
/// and a canonical one (Unicode chapter 3, D69: one or two).
const MOST_COMPATIBILITY: usize = 18;
const MOST_CANONICAL: usize = 2;

/// The decompositions of one level of field 5 of `UnicodeData.txt`, each list sorted by code
/// point.
pub(crate) struct Decompositions {
    /// The decompositions without a tag: one or two code points.
    pub(crate) canonical: Vec<(u32, Vec<u32>)>,
    /// The decompositions with a tag (`<compat>`, `<font>` and the others), whose tag is not
    /// written: one to 18 code points.
    pub(crate) compatibility: Vec<(u32, Vec<u32>)>,
}

impl Decompositions {
    /// The one-level decomposition of `point`: for `compatibility` the compatibility one if
    /// there is one, else the canonical one.
    fn of(&self, point: u32, compatibility: bool) -> Option<&[u32]> {
        if compatibility && let Some(points) = find(&self.compatibility, point) {
            return Some(points);
        }
        find(&self.canonical, point)
    }

    /// The most levels a full decomposition takes, compatibility or canonical: the depth of the
    /// explicit stack the lookups expand a decomposition with. A level is counted for each
    /// decomposition entered; the list of the levels open is kept, not a call per level.
    pub(crate) fn depth(&self) -> Result<usize, String> {
        let mut deepest = 0;
        for (key, _) in self.canonical.iter().chain(&self.compatibility) {
            // Each open level: its code points and the next one to read.
            let mut open: Vec<(&[u32], usize)> = Vec::new();
            if let Some(points) = self.of(*key, true) {
                open.push((points, 0));
            }
            loop {
                let levels = open.len();
                let Some((points, next)) = open.last_mut() else {
                    break;
                };
                deepest = deepest.max(levels);
                let Some(&point) = points.get(*next) else {
                    open.pop();
                    continue;
                };
                *next += 1;
                if (S_BASE..S_BASE + S_COUNT).contains(&point) {
                    return Err(format!(
                        "the decomposition of {key:04X} holds the Hangul syllable {point:04X}"
                    ));
                }
                if let Some(points) = self.of(point, true) {
                    if levels > self.canonical.len() + self.compatibility.len() {
                        return Err(format!("the decomposition of {key:04X} does not end"));
                    }
                    open.push((points, 0));
                }
            }
        }
        Ok(deepest)
    }
}

/// The code points of `point` in `list`, sorted by code point.
fn find(list: &[(u32, Vec<u32>)], point: u32) -> Option<&[u32]> {
    list.binary_search_by_key(&point, |(key, _)| *key)
        .ok()
        .and_then(|at| list.get(at))
        .map(|(_, points)| points.as_slice())
}

/// The decompositions of `source`, a file of the form of `UnicodeData.txt`: field 5, a list of
/// code points after an optional tag `<...>`. A field on a record of a range, a canonical
/// decomposition of more than two code points, a compatibility one of more than 18, a field
/// with no code point or one that is not a list of code points is an error.
pub(crate) fn decompositions(ucd: &mut Ucd, source: &str) -> Result<Decompositions, String> {
    let name = ucd.source_name(source);
    let mut decompositions = Decompositions {
        canonical: Vec::new(),
        compatibility: Vec::new(),
    };
    for record in ucd.unicode_data(source)? {
        let field = record.field(5);
        if field.is_empty() {
            continue;
        }
        let bad = |what: &str| format!("{name}:{}: {what}: {field:?}", record.line);
        if record.first != record.last {
            return Err(bad("a decomposition on a range of code points"));
        }
        let (tagged, list) = match field.strip_prefix('<') {
            Some(rest) => {
                let (_tag, list) = rest
                    .split_once('>')
                    .ok_or_else(|| bad("a tag with no `>`"))?;
                (true, list)
            }
            None => (false, field),
        };
        let points = list
            .split_whitespace()
            .map(|point| code_point(point).ok_or_else(|| bad("not a list of code points")))
            .collect::<Result<Vec<u32>, String>>()?;
        let most = if tagged {
            MOST_COMPATIBILITY
        } else {
            MOST_CANONICAL
        };
        if points.is_empty() || points.len() > most {
            return Err(bad(&format!(
                "a decomposition of {} code points, not one to {most}",
                points.len()
            )));
        }
        let list = if tagged {
            &mut decompositions.compatibility
        } else {
            &mut decompositions.canonical
        };
        list.push((record.first, points));
    }
    if decompositions.canonical.is_empty() || decompositions.compatibility.is_empty() {
        return Err(format!(
            "{name}: no canonical or no compatibility decomposition"
        ));
    }
    Ok(decompositions)
}

/// The canonical combining classes of `source`, a file of the form of `UnicodeData.txt`, that
/// are not 0: field 3, as sorted, disjoint ranges `(first, last, class)` of one class each,
/// two ranges that touch merged when their class is the same. A field that is not a number of
/// 0 to 254 is an error.
pub(crate) fn combining_classes(
    ucd: &mut Ucd,
    source: &str,
) -> Result<Vec<(u32, u32, u8)>, String> {
    let name = ucd.source_name(source);
    let mut ranges: Vec<(u32, u32, u8)> = Vec::new();
    for record in ucd.unicode_data(source)? {
        let field = record.field(3);
        let class = field
            .parse::<u8>()
            .ok()
            .filter(|class| *class < 255 && field.bytes().all(|b| b.is_ascii_digit()))
            .ok_or_else(|| {
                format!(
                    "{name}:{}: not a canonical combining class: {field:?}",
                    record.line
                )
            })?;
        if class == 0 {
            continue;
        }
        match ranges.last_mut() {
            Some(last) if last.2 == class && last.1.saturating_add(1) == record.first => {
                last.1 = record.last;
            }
            _ => ranges.push((record.first, record.last, class)),
        }
    }
    if ranges.is_empty() {
        return Err(format!("{name}: no canonical combining class but 0"));
    }
    Ok(ranges)
}

/// Whether `point` lies in one of `ranges`, sorted and disjoint.
fn in_ranges(ranges: &[(u32, u32)], point: u32) -> bool {
    ranges
        .binary_search_by(|&(first, last)| {
            if last < point {
                std::cmp::Ordering::Less
            } else if first > point {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// The canonical combining class of `point` in `ranges`, as [`combining_classes`] answers them.
fn class_of(ranges: &[(u32, u32, u8)], point: u32) -> u8 {
    let at = ranges.partition_point(|(_, last, _)| *last < point);
    ranges
        .get(at)
        .filter(|(first, _, _)| *first <= point)
        .map_or(0, |(_, _, class)| *class)
}

/// The number of UTF-16 code units of `point`.
fn units(point: u32) -> usize {
    if point > 0xFFFF { 2 } else { 1 }
}

/// The primary composites (UAX #15, D114): `(first, second, composite)` for every canonical
/// decomposition of two code points whose code point `Full_Composition_Exclusion` of
/// `derived` does not name, sorted by the pair. `Full_Composition_Exclusion` must be what
/// UAX #15 derives it from: the code points of `exclusions` (`CompositionExclusions.txt`), the
/// singletons (a canonical decomposition of one code point) and the non-starter decompositions
/// (one of two code points whose code point or first code point has a class other than 0); a
/// difference is an error. So is a composite whose UTF-16 code units are not as many as those of
/// the first of its pair: the lookups replace the first in place.
pub(crate) fn compositions(
    ucd: &mut Ucd,
    unicode_data: &str,
    exclusions: &str,
    derived: &str,
) -> Result<Vec<(u32, u32, u32)>, String> {
    let decompositions = decompositions(ucd, unicode_data)?;
    let classes = combining_classes(ucd, unicode_data)?;
    let exclusions_name = ucd.source_name(exclusions);
    let mut excluded: Vec<u32> = Vec::new();
    for (number, data) in ucd.data_lines(exclusions)? {
        let point = code_point(data)
            .ok_or_else(|| format!("{exclusions_name}:{number}: not a code point: {data:?}"))?;
        excluded.push(point);
    }
    excluded.sort_unstable();
    let full = ucd
        .ranges(derived, &["Full_Composition_Exclusion"])?
        .into_iter()
        .next()
        .unwrap_or_default();

    let mut derived_points: Vec<u32> = excluded.clone();
    for (point, points) in &decompositions.canonical {
        let non_starter = class_of(&classes, *point) != 0
            || points
                .first()
                .is_some_and(|first| class_of(&classes, *first) != 0);
        if points.len() == 1 || non_starter {
            derived_points.push(*point);
        }
    }
    derived_points.sort_unstable();
    derived_points.dedup();
    let derived_name = ucd.source_name(derived);
    for point in &derived_points {
        if !in_ranges(&full, *point) {
            return Err(format!(
                "{derived_name}: Full_Composition_Exclusion does not name {point:04X}, \
                 which is excluded from composition by {exclusions_name} or by its decomposition"
            ));
        }
    }
    let named: u64 = full
        .iter()
        .map(|(first, last)| u64::from(last - first) + 1)
        .sum();
    if named != u64::try_from(derived_points.len()).unwrap_or(u64::MAX) {
        return Err(format!(
            "{derived_name}: Full_Composition_Exclusion names {named} code points, \
             {} by its derivation",
            derived_points.len()
        ));
    }

    let mut composites = Vec::new();
    for (point, points) in &decompositions.canonical {
        let [first, second] = points.as_slice() else {
            continue;
        };
        if in_ranges(&full, *point) {
            continue;
        }
        if units(*first) != units(*point) {
            return Err(format!(
                "the primary composite {point:04X} has not as many UTF-16 code units as {first:04X}"
            ));
        }
        composites.push((*first, *second, *point));
    }
    composites.sort_unstable();
    if composites
        .windows(2)
        .any(|w| matches!(w, [a, b] if (a.0, a.1) == (b.0, b.1)))
    {
        return Err("two primary composites of one pair".to_string());
    }
    Ok(composites)
}

/// The widest list of arguments of a tuple rustfmt keeps on one line (its `fn_call_width`), and
/// the widest array literal (its `array_width`), both of a line of 100.
const CALL_WIDTH: usize = 60;
const ARRAY_WIDTH: usize = 60;
/// The widest line rustfmt writes (its `max_width`).
const MAX_WIDTH: usize = 100;

/// `point` as a table writes it: `0x` and at least four hexadecimal digits.
pub(crate) fn hex(point: u32) -> String {
    format!("{point:#06X}")
}

/// The row `(key, &[points])` of a constant of type `&[(u32, &[u32])]`, as rustfmt writes it at
/// the indent of a row: on one line where its arguments fit `fn_call_width`; else one argument
/// per line, the array on one line where it fits `array_width`, else its items as many to a
/// line as fit `max_width`.
pub(crate) fn mapping_row(key: u32, points: &[u32]) -> String {
    let items: Vec<String> = points.iter().map(|point| hex(*point)).collect();
    let array = format!("[{}]", items.join(", "));
    let arguments = format!("{}, &{array}", hex(key));
    if arguments.len() <= CALL_WIDTH {
        return format!("    ({arguments}),\n");
    }
    let mut text = format!("    (\n        {},\n", hex(key));
    if array.len() <= ARRAY_WIDTH {
        text.push_str(&format!("        &{array},\n"));
    } else {
        text.push_str("        &[\n");
        let indent = "            ";
        let mut line = String::new();
        for item in &items {
            let next = if line.is_empty() {
                format!("{indent}{item},")
            } else {
                format!("{line} {item},")
            };
            if next.len() > MAX_WIDTH && !line.is_empty() {
                text.push_str(&line);
                text.push('\n');
                line = format!("{indent}{item},");
            } else {
                line = next;
            }
        }
        text.push_str(&line);
        text.push_str("\n        ],\n");
    }
    text.push_str("    ),\n");
    text
}

/// The constant `name` of type `&[<row>]` whose rows are `rows`, each written whole with its
/// indent and its line end; a table of these sizes is never one line.
pub(crate) fn constant(name: &str, row: &str, rows: &[String]) -> String {
    let mut text = format!("pub(crate) const {name}: &[{row}] = &[\n");
    for row in rows {
        text.push_str(row);
    }
    text.push_str("];\n");
    text
}

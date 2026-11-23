//! The simple case folding of `CaseFolding.txt` (design, decision 8), which Canonicalize reads
//! under the flags `u` and `v` with `i` (`#sec-runtime-semantics-canonicalize-ch`): the lines of
//! status `C` (common) and `S` (simple). The lines of status `F` (full) and `T` (Turkic) are
//! not read: ECMA-262 names the simple folding.

use crate::ucd::{Ucd, code_point};

/// The simple case folding of `source`, a file of the form of `CaseFolding.txt`
/// (`<code>; <status>; <mapping>; # <name>`): the pairs `(code point, folding)` of the lines of
/// status `C` and `S`, sorted by code point. A line of another form, a status other than `C`,
/// `F`, `S` and `T`, a code point with two simple foldings, a folding that is not a code point
/// or that maps to itself, a folding that is itself folded to another code point, and a pair
/// whose two code points are not both below U+10000 or both above are errors: the matcher
/// reads a folding as a fixed point of the folding, and compares code units in place.
pub(crate) fn simple_case_folding(ucd: &mut Ucd, source: &str) -> Result<Vec<(u32, u32)>, String> {
    let name = ucd.source_name(source);
    let mut pairs = Vec::new();
    for (number, data) in ucd.data_lines(source)? {
        let bad = |what: &str| format!("{name}:{number}: {what}: {data:?}");
        let fields: Vec<&str> = data.split(';').map(str::trim).collect();
        let [code, status, mapping, ""] = fields.as_slice() else {
            return Err(bad("not <code>; <status>; <mapping>;"));
        };
        match *status {
            "C" | "S" => {}
            "F" | "T" => continue,
            _ => return Err(bad("a status other than C, F, S and T")),
        }
        let code = code_point(code).ok_or_else(|| bad("not a code point"))?;
        let folding =
            code_point(mapping).ok_or_else(|| bad("a folding that is not a code point"))?;
        if folding == code {
            return Err(bad("a folding to the code point itself"));
        }
        if (code > 0xFFFF) != (folding > 0xFFFF) {
            return Err(bad(
                "a code point and its folding of different lengths in UTF-16",
            ));
        }
        pairs.push((code, folding));
    }
    pairs.sort_unstable();
    if let Some(pair) = pairs
        .windows(2)
        .find(|pair| matches!(pair, [a, b] if a.0 == b.0))
    {
        let code = pair.first().map(|(code, _)| *code).unwrap_or_default();
        return Err(format!("{name}: {code:04X} has two simple foldings"));
    }
    for &(code, folding) in &pairs {
        if pairs
            .binary_search_by_key(&folding, |&(key, _)| key)
            .is_ok()
        {
            return Err(format!(
                "{name}: {code:04X} folds to {folding:04X}, which folds again"
            ));
        }
    }
    if pairs.is_empty() {
        return Err(format!("{name}: no line of status C or S"));
    }
    Ok(pairs)
}

/// The code points grouped by their folding: the pairs `(folding, code point)` of `pairs`,
/// sorted by folding and then by code point, and the most code points of one folding, the
/// folding itself counted.
pub(crate) fn by_folding(pairs: &[(u32, u32)]) -> (Vec<(u32, u32)>, usize) {
    let mut grouped: Vec<(u32, u32)> = pairs
        .iter()
        .map(|&(code, folding)| (folding, code))
        .collect();
    grouped.sort_unstable();
    let mut largest = 1;
    let mut run = 0;
    let mut previous = None;
    for &(folding, _) in &grouped {
        run = if previous == Some(folding) {
            run + 1
        } else {
            1
        };
        previous = Some(folding);
        largest = largest.max(run + 1);
    }
    (grouped, largest)
}

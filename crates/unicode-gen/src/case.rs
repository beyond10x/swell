//! The case data of the tables of `swell-unicode` (design, decision 8): the simple case
//! mappings of `UnicodeData.txt`, the full mappings of `SpecialCasing.txt` that need no
//! language, and the properties `Cased` and `Case_Ignorable` of `DerivedCoreProperties.txt`
//! that the context `Final_Sigma` reads (Unicode chapter 3, section 3.13, table 3-17).

use crate::ucd::{Ucd, code_point};

/// A simple case mapping of `UnicodeData.txt`, by its field.
#[derive(Clone, Copy)]
pub(crate) enum Simple {
    /// Field 12, `Simple_Uppercase_Mapping`.
    Uppercase,
    /// Field 13, `Simple_Lowercase_Mapping`.
    Lowercase,
}

impl Simple {
    /// The number of its field, from 0.
    fn field(self) -> usize {
        match self {
            Simple::Uppercase => 12,
            Simple::Lowercase => 13,
        }
    }
}

/// The pairs `(code point, mapping)` of the simple mapping `simple` in `source`, a file of the
/// form of `UnicodeData.txt`, sorted by code point: every record whose field is not empty. A
/// field that is not a code point, or a mapping on a record of a range, is an error.
pub(crate) fn simple_mappings(
    ucd: &mut Ucd,
    source: &str,
    simple: Simple,
) -> Result<Vec<(u32, u32)>, String> {
    let name = ucd.source_name(source);
    let mut pairs = Vec::new();
    for record in ucd.unicode_data(source)? {
        let field = record.field(simple.field());
        if field.is_empty() {
            continue;
        }
        let bad = |what: &str| format!("{name}:{}: {what}: {field:?}", record.line);
        let mapping = code_point(field).ok_or_else(|| bad("a mapping that is not a code point"))?;
        if record.first != record.last {
            return Err(bad("a mapping on a range of code points"));
        }
        pairs.push((record.first, mapping));
    }
    if pairs.is_empty() {
        return Err(format!(
            "{name}: no simple mapping in field {}",
            simple.field()
        ));
    }
    Ok(pairs)
}

/// The mappings of `SpecialCasing.txt` the tables hold, each list sorted by code point.
pub(crate) struct SpecialCasing {
    /// The unconditional full lowercase mappings.
    pub(crate) lowercase: Vec<(u32, Vec<u32>)>,
    /// The unconditional full uppercase mappings.
    pub(crate) uppercase: Vec<(u32, Vec<u32>)>,
    /// The lowercase mappings where the context `Final_Sigma` holds.
    pub(crate) final_sigma: Vec<(u32, Vec<u32>)>,
}

/// The lines of `source`, a file of the form of `SpecialCasing.txt`:
/// `<code>; <lower>; <title>; <upper>; (<condition_list>;)?`. A line with no condition goes to
/// the unconditional mappings; a line whose condition list names a language (a lowercase
/// language ID such as `lt`, `tr` or `az`) is not read, since the engine has no host locale
/// (ECMA-402 is out); the condition `Final_Sigma` alone gives the lowercase mapping in that
/// context, and its uppercase mapping must be the code point itself. Any other condition, a
/// field that is not a list of code points, or a mapping of no code point is an error.
pub(crate) fn special_casing(ucd: &mut Ucd, source: &str) -> Result<SpecialCasing, String> {
    let name = ucd.source_name(source);
    let mut special = SpecialCasing {
        lowercase: Vec::new(),
        uppercase: Vec::new(),
        final_sigma: Vec::new(),
    };
    for (number, data) in ucd.data_lines(source)? {
        let bad = |what: &str| format!("{name}:{number}: {what}: {data:?}");
        let fields: Vec<&str> = data.split(';').map(str::trim).collect();
        let (code, lower, upper, condition) = match fields.as_slice() {
            [code, lower, _title, upper, ""] => (*code, *lower, *upper, None),
            [code, lower, _title, upper, condition, ""] => {
                (*code, *lower, *upper, Some(*condition))
            }
            _ => {
                return Err(bad(
                    "not <code>; <lower>; <title>; <upper>; (<condition_list>;)?",
                ));
            }
        };
        let code = code_point(code).ok_or_else(|| bad("not a code point"))?;
        let points = |field: &str| -> Result<Vec<u32>, String> {
            field
                .split_whitespace()
                .map(|point| code_point(point).ok_or_else(|| bad("not a list of code points")))
                .collect()
        };
        match condition {
            None => {
                let (lower, upper) = (points(lower)?, points(upper)?);
                if lower.is_empty() || upper.is_empty() {
                    return Err(bad("a mapping of no code point"));
                }
                special.lowercase.push((code, lower));
                special.uppercase.push((code, upper));
            }
            Some(condition) => {
                let words: Vec<&str> = condition.split_whitespace().collect();
                let language = words
                    .iter()
                    .any(|word| !word.is_empty() && word.chars().all(|c| c.is_ascii_lowercase()));
                if language {
                    continue;
                }
                if words != ["Final_Sigma"] {
                    return Err(bad("a condition the tables do not hold"));
                }
                if points(upper)? != [code] {
                    return Err(bad(
                        "a Final_Sigma line whose uppercase is not its code point",
                    ));
                }
                let lower = points(lower)?;
                if lower.is_empty() {
                    return Err(bad("a mapping of no code point"));
                }
                special.final_sigma.push((code, lower));
            }
        }
    }
    for list in [
        &mut special.lowercase,
        &mut special.uppercase,
        &mut special.final_sigma,
    ] {
        list.sort_by_key(|(code, _)| *code);
    }
    if special.lowercase.is_empty() || special.final_sigma.is_empty() {
        return Err(format!(
            "{name}: no unconditional mapping or no Final_Sigma line"
        ));
    }
    Ok(special)
}

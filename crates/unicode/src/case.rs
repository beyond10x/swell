//! The full case mappings of one code point, as the Unicode Default Case Conversion reads them
//! (Unicode chapter 3, section 3.13) and `String.prototype.toLowerCase` and `toUpperCase` ask
//! for them (`#sec-string.prototype.tolowercase`, `#sec-string.prototype.touppercase`): the
//! locale-insensitive mappings of `SpecialCasing.txt` first, the simple mapping of
//! `UnicodeData.txt` else, and the code point itself where neither has one. The lines of
//! `SpecialCasing.txt` that need a language are not held: the engine has no host locale.
//!
//! The context `Final_Sigma` (table 3-17) is read by the caller, which holds the text: it asks
//! [`is_cased`] and [`is_case_ignorable`] of the code points around U+03A3 and takes
//! [`final_sigma_lowercase`] where the context holds.

use crate::in_ranges;
use crate::tables::{case_ignorable, cased, lowercase, special_casing, uppercase};

/// The full case mapping of one code point: one to three code points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaseMapping {
    points: [u32; 3],
    len: usize,
}

impl CaseMapping {
    /// The mapping of a code point to itself or to one other.
    fn one(point: u32) -> CaseMapping {
        CaseMapping {
            points: [point, 0, 0],
            len: 1,
        }
    }

    /// The mapping to `points`, of which the tables hold one to three; more are cut to three.
    fn of(points: &[u32]) -> CaseMapping {
        let mut mapping = CaseMapping {
            points: [0; 3],
            len: 0,
        };
        for (slot, &point) in mapping.points.iter_mut().zip(points) {
            *slot = point;
            mapping.len += 1;
        }
        mapping
    }

    /// The code points of the mapping, in order: one to three.
    pub fn as_slice(&self) -> &[u32] {
        self.points.get(..self.len).unwrap_or(&[])
    }
}

/// The value of `key` in `pairs`, sorted by key.
fn pair_value(pairs: &[(u32, u32)], key: u32) -> Option<u32> {
    pairs
        .binary_search_by_key(&key, |&(k, _)| k)
        .ok()
        .and_then(|at| pairs.get(at))
        .map(|&(_, value)| value)
}

/// The code points of `key` in `mappings`, sorted by key.
fn mapping_value(mappings: &[(u32, &'static [u32])], key: u32) -> Option<CaseMapping> {
    mappings
        .binary_search_by_key(&key, |&(k, _)| k)
        .ok()
        .and_then(|at| mappings.get(at))
        .map(|&(_, points)| CaseMapping::of(points))
}

/// The full lowercase mapping of `c` (`Lowercase_Mapping`) outside any context: by
/// `SpecialCasing.txt`, else by `Simple_Lowercase_Mapping`, else `c` itself. Any `u32` may be
/// asked; a lone surrogate and a value above U+10FFFF map to themselves.
pub fn to_lowercase(c: u32) -> CaseMapping {
    mapping_value(special_casing::LOWERCASE, c)
        .or_else(|| pair_value(lowercase::LOWERCASE, c).map(CaseMapping::one))
        .unwrap_or_else(|| CaseMapping::one(c))
}

/// The full uppercase mapping of `c` (`Uppercase_Mapping`): by `SpecialCasing.txt`, else by
/// `Simple_Uppercase_Mapping`, else `c` itself. Any `u32` may be asked; a lone surrogate and a
/// value above U+10FFFF map to themselves.
pub fn to_uppercase(c: u32) -> CaseMapping {
    mapping_value(special_casing::UPPERCASE, c)
        .or_else(|| pair_value(uppercase::UPPERCASE, c).map(CaseMapping::one))
        .unwrap_or_else(|| CaseMapping::one(c))
}

/// The lowercase mapping of `c` where the context `Final_Sigma` holds (Unicode chapter 3,
/// table 3-17), if `SpecialCasing.txt` gives `c` one there: U+03A3 to U+03C2. `None` for every
/// other code point, whose lowercase is [`to_lowercase`] in every context.
pub fn final_sigma_lowercase(c: u32) -> Option<CaseMapping> {
    mapping_value(special_casing::FINAL_SIGMA, c)
}

/// Whether `c` has the property `Cased` (`DerivedCoreProperties.txt`). Any `u32` may be asked;
/// a value above U+10FFFF has none.
pub fn is_cased(c: u32) -> bool {
    in_ranges(cased::CASED, c)
}

/// Whether `c` has the property `Case_Ignorable` (`DerivedCoreProperties.txt`). Any `u32` may
/// be asked; a value above U+10FFFF has none.
pub fn is_case_ignorable(c: u32) -> bool {
    in_ranges(case_ignorable::CASE_IGNORABLE, c)
}

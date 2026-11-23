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
//!
//! Beside the mappings, the simple case folding of `CaseFolding.txt` (the lines of status `C`
//! and `S`), which Canonicalize of a regular expression reads under the flags `u` and `v`
//! (`#sec-runtime-semantics-canonicalize-ch`): [`simple_case_fold`], and the code points of one
//! folding, [`simple_case_fold_equivalents`], at most [`MOST_CASE_FOLD_EQUIVALENTS`].

use crate::in_ranges;
use crate::tables::{case_folding, case_ignorable, cased, lowercase, special_casing, uppercase};

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

/// The simple case folding of `c` (`CaseFolding.txt`, status `C` or `S`): the folding the file
/// gives, else `c` itself. A folding is its own folding and has as many UTF-16 code units as
/// `c` (the generator refuses data where not). Any `u32` may be asked; a lone surrogate and a
/// value above U+10FFFF fold to themselves.
pub fn simple_case_fold(c: u32) -> u32 {
    pair_value(case_folding::FOLDING, c).unwrap_or(c)
}

/// The most code points of one simple case folding, the folding itself counted: the most a
/// [`CaseFoldEquivalents`] gives. The generator states it in the table.
pub const MOST_CASE_FOLD_EQUIVALENTS: usize = case_folding::LARGEST_GROUP;

/// The code points whose simple case folding is that of `c`, `c` among them: the folding
/// first, then the code points the file folds to it, in order of code point. Found by three
/// binary searches; at most [`MOST_CASE_FOLD_EQUIVALENTS`] code points.
pub fn simple_case_fold_equivalents(c: u32) -> CaseFoldEquivalents {
    let folding = simple_case_fold(c);
    let grouped = case_folding::BY_FOLDING;
    let start = grouped.partition_point(|&(key, _)| key < folding);
    let end = grouped.partition_point(|&(key, _)| key <= folding);
    CaseFoldEquivalents {
        folding: Some(folding),
        others: grouped.get(start..end).unwrap_or(&[]).iter(),
    }
}

/// The code points of one simple case folding ([`simple_case_fold_equivalents`]).
#[derive(Clone, Debug)]
pub struct CaseFoldEquivalents {
    folding: Option<u32>,
    others: std::slice::Iter<'static, (u32, u32)>,
}

impl Iterator for CaseFoldEquivalents {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if let Some(folding) = self.folding.take() {
            return Some(folding);
        }
        self.others.next().map(|&(_, point)| point)
    }
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

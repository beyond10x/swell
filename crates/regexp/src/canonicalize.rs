//! Canonicalize (`#sec-runtime-semantics-canonicalize-ch`) where [[IgnoreCase]] is true, and the
//! characters of one canonical value, which the test of a class reads under `i`.
//!
//! Under `u` or `v` the canonical value of a character is its simple case folding
//! (`CaseFolding.txt`, status `C` and `S`; [`swell_unicode::simple_case_fold`]). Under neither
//! it is the uppercase mapping of the code unit ([`swell_unicode::to_uppercase`], the Unicode
//! Default Case Conversion with `SpecialCasing.txt`) where that is one code unit, and the code
//! unit itself where the mapping is several code units or maps a code unit of 128 or above to
//! one below 128.
//!
//! A class under `i` holds a character `c` when one of its members `a` has the canonical value
//! of `c` (`#sec-runtime-semantics-charactersetmatcher-abstract-operation`): the matcher tests
//! the characters of that value, [`equivalents`], against the class's ranges and property
//! escapes when the class is run, and the compile step builds no closure of a class. The
//! characters of one canonical value are found in the groups of the simple case folding
//! ([`swell_unicode::simple_case_fold_equivalents`], at most
//! [`swell_unicode::MOST_CASE_FOLD_EQUIVALENTS`] code points, three binary searches): under `u`
//! or `v` they are the group, and under neither the code units of the group with the canonical
//! value of `c`, since two code units with one uppercase mapping have one simple case folding
//! (`case_insensitive::canonicalize_without_u_stays_inside_a_folding_group` holds it for every
//! code unit).

use swell_unicode::{simple_case_fold, simple_case_fold_equivalents, to_uppercase};

/// Canonicalize of the character `c` where [[IgnoreCase]] is true: with `unicode` (the flag `u`
/// or `v`) the simple case folding of the code point `c`; else the uppercase mapping of the
/// code unit `c` where it is one code unit and does not map a code unit of 128 or above to one
/// below 128, else `c`. A value above U+FFFF without `unicode` is no code unit and is its own.
pub fn canonicalize(c: u32, unicode: bool) -> u32 {
    if unicode {
        return simple_case_fold(c);
    }
    if c > 0xFFFF {
        return c;
    }
    let mapped = to_uppercase(c);
    let &[point] = mapped.as_slice() else {
        return c;
    };
    if point > 0xFFFF || (c >= 128 && point < 128) {
        return c;
    }
    point
}

/// The characters whose canonical value is that of `c`, `c` among them: at most
/// [`swell_unicode::MOST_CASE_FOLD_EQUIVALENTS`].
pub(crate) fn equivalents(c: u32, unicode: bool) -> impl Iterator<Item = u32> {
    let canonical = canonicalize(c, unicode);
    simple_case_fold_equivalents(c)
        .filter(move |&e| unicode || (e <= 0xFFFF && canonicalize(e, false) == canonical))
}

/// Whether `c` is the one character of its canonical value, so that a test of its canonical
/// value is a test of `c` itself.
pub(crate) fn has_no_other_case(c: u32, unicode: bool) -> bool {
    equivalents(c, unicode).all(|e| e == c)
}

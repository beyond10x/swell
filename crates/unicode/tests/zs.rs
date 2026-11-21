//! The `Zs` code points, which `WhiteSpace` admits through `USP` (`#sec-white-space`).
//!
//! The 17 code points were printed by
//! `grep -E "; Zs" data/unicode/17.0.0/ucd/extracted/DerivedGeneralCategory.txt`:
//! `0020 00A0 1680 2000..200A 202F 205F 3000`.

use swell_unicode::is_space_separator;

const ZS: [u32; 17] = [
    0x0020, 0x00A0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008,
    0x2009, 0x200A, 0x202F, 0x205F, 0x3000,
];

#[test]
fn space_separators_are_the_17_code_points() {
    let mut found = Vec::new();
    for c in 0..=0x10FFFF {
        if is_space_separator(c) {
            found.push(c);
        }
    }
    assert_eq!(found, ZS);
    assert!(!is_space_separator(0x110000));
    assert!(!is_space_separator(u32::MAX));
}

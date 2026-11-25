//! `parse_float` and `parse_int` of `swell_num::parse`: the steps of `parseFloat`
//! (`#sec-parsefloat-string`, from step 2) and `parseInt` (`#sec-parseint-string-radix`, from
//! step 3) over the code units of a String, and the set of `StrWhiteSpaceChar`.
//!
//! The expected values come from the clauses, from `str::parse::<f64>` of the same decimal
//! digits (correctly rounded, `dec2flt`), and from exact arithmetic ([`exact`]): the integer
//! that a run of digits writes, held as a big integer and rounded to nearest, ties to even, by
//! `Rat::to_f64`. Radix 10 and the radices 2, 4, 8, 16 and 32 must give that value exactly; the
//! other radices give the value of `acc * radix + digit` accumulated in `f64`
//! (`docs/design/engine.md`, decision 6, "parseInt for a radix other than 10").

#![cfg(test)]

// The exact arithmetic is included as `math_dd.rs` includes it; there is no `tests/support/mod.rs`
// to share it through, so the file is a module of this one too.
#[allow(clippy::duplicate_mod)]
#[path = "support/exact.rs"]
mod exact;

use std::time::Instant;

use exact::{Big, Rat};
use swell_num::parse::{is_str_white_space, parse_float, parse_int};

/// Whether two Numbers are the same value: equal bits, or both NaN.
fn same(actual: f64, expected: f64) -> bool {
    actual.to_bits() == expected.to_bits() || (actual.is_nan() && expected.is_nan())
}

/// The UTF-16 code units of `text`.
fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// `parse_float` of `text`, read as UTF-16 code units and, when every unit is below 0x100, as
/// Latin-1 bytes too: both forms of a String must answer the same.
fn float(text: &str) -> f64 {
    let wide = parse_float(&units(text));
    if let Ok(bytes) = units(text)
        .iter()
        .map(|&unit| u8::try_from(unit))
        .collect::<Result<Vec<u8>, _>>()
    {
        assert!(same(parse_float(&bytes), wide), "{text:?} as Latin-1");
    }
    wide
}

/// `parse_int` of `text` in `radix`, in both forms as [`float`] reads them.
fn int(text: &str, radix: i32) -> f64 {
    let wide = parse_int(&units(text), radix);
    if let Ok(bytes) = units(text)
        .iter()
        .map(|&unit| u8::try_from(unit))
        .collect::<Result<Vec<u8>, _>>()
    {
        assert!(same(parse_int(&bytes, radix), wide), "{text:?} as Latin-1");
    }
    wide
}

/// The digit of value `value` (0 to 35), lowercase.
fn digit(value: u32) -> char {
    char::from_digit(value, 36).unwrap()
}

/// A run of `count` digits in `radix`, made by a fixed linear congruential sequence from
/// `seed`, whose first digit is not 0.
fn digits(radix: u32, count: usize, seed: u64) -> String {
    let mut state = seed;
    (0..count)
        .map(|i| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let value = u32::try_from((state >> 33) % u64::from(radix)).unwrap();
            if i == 0 && value == 0 {
                digit(1)
            } else {
                digit(value)
            }
        })
        .collect()
}

/// The integer `text` writes in `radix`, rounded once to nearest, ties to even: exact
/// arithmetic, independent of the crate.
fn exactly(text: &str, radix: u32) -> f64 {
    let mut value = Big::zero();
    let base = Big::from_u64(u64::from(radix));
    for c in text.chars() {
        let d = c.to_digit(radix).unwrap();
        value = value.mul(&base).add(&Big::from_u64(u64::from(d)));
    }
    Rat::new(false, value, Big::from_u64(1)).to_f64()
}

/// The method the design states for a radix other than 2, 4, 8, 10, 16 and 32: `acc * radix +
/// digit` in `f64`, from the first digit.
fn accumulated(text: &str, radix: u32) -> f64 {
    text.chars().fold(0.0, |acc, c| {
        acc * f64::from(radix) + f64::from(c.to_digit(radix).unwrap())
    })
}

/// `parse_float` over a table of 40 entries: the longest prefix that is a `StrDecimalLiteral`
/// after the leading `StrWhiteSpace`, its sign kept for -0, `Infinity` read by the scanner and
/// never Rust's `inf` or `nan`, no hex, no separator, an exponent with no digit left out, NaN
/// for no prefix, and the roundings of the subnormal edge and of the overflow.
#[test]
fn parse_float_reads_the_longest_decimal_prefix() {
    let started = Instant::now();
    let cases: [(&str, f64); 40] = [
        // 3.14, written as a quotient: the division rounds once, to the same Number.
        ("3.14abc", 314.0 / 100.0),
        (" \n 1e3x", 1000.0),
        ("-0", -0.0),
        ("+0", 0.0),
        ("-0.0e7", -0.0),
        (".5", 0.5),
        ("5.", 5.0),
        ("-.5e-1x", -0.05),
        ("1.e5", 100_000.0),
        ("1e", 1.0),
        ("1e+", 1.0),
        ("1e+x", 1.0),
        ("1.5E-2", 0.015),
        ("+Infinityx", f64::INFINITY),
        ("-Infinity", f64::NEG_INFINITY),
        ("Infinity1", f64::INFINITY),
        ("infinity", f64::NAN),
        ("Infinit", f64::NAN),
        ("inf", f64::NAN),
        ("nan", f64::NAN),
        ("NaN", f64::NAN),
        ("0x10", 0.0),
        ("1_0", 1.0),
        ("", f64::NAN),
        ("   ", f64::NAN),
        (".", f64::NAN),
        ("+", f64::NAN),
        ("-.", f64::NAN),
        ("e5", f64::NAN),
        ("+-1", f64::NAN),
        ("1e400", f64::INFINITY),
        ("-1e400", f64::NEG_INFINITY),
        ("1e-400", 0.0),
        ("-1e-400", -0.0),
        ("0e99999999999999999999", 0.0),
        ("1e99999999999999999999", f64::INFINITY),
        (
            "2.2250738585072011e-308",
            f64::from_bits(0x000F_FFFF_FFFF_FFFF),
        ),
        (
            "2.2250738585072009e-308",
            f64::from_bits(0x000F_FFFF_FFFF_FFFF),
        ),
        ("4.9406564584124654e-324", f64::from_bits(1)),
        ("\u{a0}\u{2028}\u{feff}\u{3000}7.25", 7.25),
    ];
    for (text, expected) in cases {
        assert!(same(float(text), expected), "{text:?}: {}", float(text));
    }
    // The two edge values against `str::parse` of the same digits as well.
    for text in ["2.2250738585072011e-308", "2.2250738585072009e-308"] {
        assert!(same(float(text), text.parse::<f64>().unwrap()), "{text}");
    }
    println!(
        "parse_float table: {} entries, {:?}",
        cases.len(),
        started.elapsed()
    );
}

/// A code unit outside the white space of `StringToNumber` ends the prefix before it starts:
/// U+180E (no longer `Zs` since Unicode 6.3), U+200B, a fullwidth digit, a lone surrogate.
#[test]
fn parse_float_reads_no_other_white_space_or_digit() {
    for text in ["\u{180e}1", "\u{200b}1", "\u{ff11}", "\u{85}1"] {
        assert!(float(text).is_nan(), "{text:?}");
    }
    assert!(parse_float(&[0xD800_u16, 0x31]).is_nan());
    assert!(same(parse_float(&[0x31_u16, 0xD800, 0x32]), 1.0));
}

/// 400 decimal digits and a point: the value of `parse_float` is `str::parse` of the same
/// text, which rounds correctly; 20 such texts, and the 400 digits with a tie far past the
/// 17th digit.
#[test]
fn parse_float_rounds_four_hundred_digits_correctly() {
    let started = Instant::now();
    let mut count = 0;
    for seed in 0..20 {
        let all = digits(10, 400, seed);
        let text = format!("{}.{}e-{}", &all[..17], &all[17..], seed * 15);
        assert!(same(float(&text), text.parse::<f64>().unwrap()), "{text}");
        count += 1;
    }
    // 2^53 + 1 written out, then 380 more digits, every one 0 but the last: just above the tie
    // between 2^53 and 2^53 + 2, so 2^53 + 2.
    let tie = format!("9007199254740993.{}1", "0".repeat(380));
    assert!(same(float(&tie), 9_007_199_254_740_994.0), "{tie}");
    assert!(same(float(&tie), tie.parse::<f64>().unwrap()));
    count += 1;
    println!(
        "parse_float of 400 digits: {count} entries, {:?}",
        started.elapsed()
    );
}

/// `parse_int` over a table of 46 entries: the radix rule of steps 8 to 13 (0 is 10, or 16
/// after `0x` or `0X`; a radix outside 2 to 36 is NaN; `0x` stripped for 16 only), the sign,
/// the longest run of digits of the radix, NaN for none, -0 for a negative sign and a zero
/// value, and the rounding of radix 10.
#[test]
fn parse_int_reads_the_radix_rule_and_the_digits() {
    let started = Instant::now();
    let cases: [(&str, i32, f64); 46] = [
        ("123", 0, 123.0),
        ("123", 10, 123.0),
        ("ff", 16, 255.0),
        ("FF", 16, 255.0),
        ("101", 2, 5.0),
        ("1012", 2, 5.0),
        ("z", 36, 35.0),
        ("Z", 36, 35.0),
        ("10", 37, f64::NAN),
        ("10", -1, f64::NAN),
        ("10", 1, f64::NAN),
        ("10", i32::MIN, f64::NAN),
        ("10", i32::MAX, f64::NAN),
        ("0x1f", 16, 31.0),
        ("0x1f", 0, 31.0),
        ("0X1F", 0, 31.0),
        ("0x1f", 10, 0.0),
        ("0x1f", 36, 42_819.0),
        ("0x", 0, f64::NAN),
        ("0x", 16, f64::NAN),
        ("0xg", 0, f64::NAN),
        ("  -12", 0, -12.0),
        ("\t+12", 0, 12.0),
        ("-0", 0, -0.0),
        ("-0", 10, -0.0),
        ("-0x0", 0, -0.0),
        ("-000", 2, -0.0),
        ("+0", 0, 0.0),
        ("-0x10", 0, -16.0),
        ("0b11", 0, 0.0),
        ("08", 0, 8.0),
        ("010", 0, 10.0),
        ("12abc", 0, 12.0),
        ("1e3", 0, 1.0),
        ("1.9", 0, 1.0),
        ("", 0, f64::NAN),
        ("   ", 0, f64::NAN),
        ("-", 0, f64::NAN),
        ("+-1", 0, f64::NAN),
        ("Infinity", 0, f64::NAN),
        ("\u{ff11}", 10, f64::NAN),
        ("\u{feff}\u{2029}5", 0, 5.0),
        ("9007199254740993", 10, 9_007_199_254_740_992.0),
        ("9007199254740995", 10, 9_007_199_254_740_996.0),
        ("-9007199254740993", 0, -9_007_199_254_740_992.0),
        ("1000000000000000000000000", 10, 1e24),
    ];
    for (text, radix, expected) in cases {
        let ours = int(text, radix);
        assert!(same(ours, expected), "{text:?} radix {radix}: {ours}");
    }
    println!(
        "parse_int table: {} entries, {:?}",
        cases.len(),
        started.elapsed()
    );
}

/// Radix 2, 4, 8, 16 and 32 convert exactly and round once, ties to even: 2^53 + 1 rounds to
/// 2^53, 2^53 + 3 to 2^53 + 4, and a 1 bit far below the tie makes it round up; a value at
/// 2^1024 or past it, after rounding, is +∞, and the largest Number is reached exactly.
#[test]
fn parse_int_rounds_a_power_of_two_radix_once() {
    let ones = |count: usize| "1".repeat(count);
    let zeros = |count: usize| "0".repeat(count);
    let cases: [(String, i32, f64); 9] = [
        (format!("1{}1", zeros(52)), 2, 9_007_199_254_740_992.0),
        (format!("1{}11", zeros(51)), 2, 9_007_199_254_740_996.0),
        (
            format!("1{}1{}1", zeros(52), zeros(200)),
            2,
            9_007_199_254_740_994.0 * 2f64.powi(201),
        ),
        ("20000000000001".to_string(), 16, 9_007_199_254_740_992.0),
        (ones(53), 2, 9_007_199_254_740_991.0),
        (ones(1024), 2, f64::INFINITY),
        (format!("{}{}", ones(53), zeros(971)), 2, f64::MAX),
        (format!("{}1{}", ones(53), zeros(970)), 2, f64::INFINITY),
        (format!("-{}", "f".repeat(300)), 16, f64::NEG_INFINITY),
    ];
    for (text, radix, expected) in &cases {
        let ours = int(text, *radix);
        assert!(same(ours, *expected), "{text:?} radix {radix}: {ours}");
    }
}

/// A run of 100 digits at radix 2, 4, 8, 10, 16 and 32 against exact arithmetic, and at
/// radix 3 and 36 against the stated method; 10 runs per radix, each also with 200 and 1,000
/// digits at radix 10 and 16.
#[test]
fn parse_int_of_a_hundred_digits_follows_the_stated_method() {
    let started = Instant::now();
    let mut count = 0;
    for seed in 0..10 {
        for radix in [2, 4, 8, 10, 16, 32] {
            let text = digits(radix, 100, seed);
            let ours = int(&text, i32::try_from(radix).unwrap());
            assert!(same(ours, exactly(&text, radix)), "{text} radix {radix}");
            count += 1;
        }
        for radix in [3, 36] {
            let text = digits(radix, 100, seed);
            let ours = int(&text, i32::try_from(radix).unwrap());
            assert!(
                same(ours, accumulated(&text, radix)),
                "{text} radix {radix}"
            );
            count += 1;
        }
        for (radix, length) in [(10, 200), (16, 200), (10, 300), (16, 250)] {
            let text = digits(radix, length, seed);
            let ours = int(&text, i32::try_from(radix).unwrap());
            assert!(same(ours, exactly(&text, radix)), "{text} radix {radix}");
            count += 1;
        }
    }
    println!(
        "parse_int of long runs: {count} entries, {:?}",
        started.elapsed()
    );
}

/// `StrWhiteSpaceChar` (`#sec-tonumber-applied-to-the-string-type`): the white space of 12.2
/// (TAB, VT, FF, ZWNBSP and the 17 code points of `Zs` in Unicode 17.0.0) and the line
/// terminators of 12.3 (LF, CR, LS, PS), 25 code units in all, every one of them in the Basic
/// Multilingual Plane; the engine's set from `swell-unicode` is held equal to it over every code
/// unit by a test of `swell`.
#[test]
fn str_white_space_is_twenty_five_code_units() {
    let set: Vec<u32> = (0..=0x10FFFF)
        .filter(|&unit| is_str_white_space(unit))
        .collect();
    let mut expected = vec![
        0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x20, 0xA0, 0x1680, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000,
        0xFEFF,
    ];
    expected.extend(0x2000..=0x200A);
    expected.sort_unstable();
    assert_eq!(set, expected);
}

//! `Number::toString ( x, 10 )` (`#sec-numeric-types-number-tostring`): the shortest digits
//! that round-trip, laid out as steps 6 to 12 say.
//!
//! The round trip draws its bit patterns from the crate's own SipHash with a fixed key, reads
//! each result back with the crate's own StringToNumber, and checks the digits and the exponent
//! against the Rust standard library's `{:e}`, which prints the shortest round-trip digits in
//! radix 10 (design, decision 6: "its `{:e}` output is used in tests as a check of the radix-10
//! digits").

use swell_num::literal::string_to_number_trimmed;
use swell_num::siphash::siphash24;
use swell_num::to_string::number_to_string;

#[test]
fn number_to_string_layouts() {
    let cases: [(f64, &str); 32] = [
        (f64::NAN, "NaN"),
        (0.0, "0"),
        (-0.0, "0"),
        (f64::INFINITY, "Infinity"),
        (f64::NEG_INFINITY, "-Infinity"),
        (1.0, "1"),
        (-1.0, "-1"),
        (10.0, "10"),
        (123.0, "123"),
        (1.5, "1.5"),
        (-1.25, "-1.25"),
        (0.1, "0.1"),
        (0.5, "0.5"),
        (100.0, "100"),
        (123.456, "123.456"),
        (1e20, "100000000000000000000"),
        (123e18, "123000000000000000000"),
        (1e21, "1e+21"),
        (1.5e21, "1.5e+21"),
        (123e-20, "1.23e-18"),
        (0.000001, "0.000001"),
        (0.0000015, "0.0000015"),
        (1e-7, "1e-7"),
        (1.5e-7, "1.5e-7"),
        (5e-324, "5e-324"),
        (-5e-324, "-5e-324"),
        (f64::MAX, "1.7976931348623157e+308"),
        (f64::MIN_POSITIVE, "2.2250738585072014e-308"),
        (9007199254740992.0, "9007199254740992"),
        (4.35, "4.35"),
        (0.3, "0.3"),
        (1e100, "1e+100"),
    ];
    for (x, expected) in cases {
        assert_eq!(number_to_string(x), expected, "bits {:#018x}", x.to_bits());
    }
}

/// The digits and the decimal exponent of `x`'s `{:e}` form, `1.2345e-7` as ("12345", -7).
fn rust_digits(x: f64) -> (String, i32) {
    let text = format!("{:e}", x.abs());
    let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    (digits, exponent.parse().unwrap_or(i32::MIN))
}

/// The digits and the decimal exponent of a result of `number_to_string`, in the same form.
fn our_digits(text: &str) -> (String, i32) {
    let text = text.trim_start_matches('-');
    if let Some((mantissa, exponent)) = text.split_once('e') {
        let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
        return (digits, exponent.parse().unwrap_or(i32::MIN));
    }
    let point = text.find('.').unwrap_or(text.len());
    let all: String = text.chars().filter(|c| *c != '.').collect();
    let leading = all.len() - all.trim_start_matches('0').len();
    let digits = all
        .trim_start_matches('0')
        .trim_end_matches('0')
        .to_string();
    let exponent = point as i32 - leading as i32 - 1;
    (digits, exponent)
}

/// Whether `x` lies exactly half-way between the two neighbouring digit strings `ours` and
/// `rust` (same length, same exponent, one apart in the last digit), checked in integers:
/// 2x = (2 × lower + 1) × 10^p, with x = M × 2^E and p the power of ten of the last digit.
/// `false` also when the check does not fit in `u128`.
fn exact_tie(x: f64, ours: &(String, i32), rust: &(String, i32)) -> bool {
    let (Ok(a), Ok(b)) = (ours.0.parse::<u128>(), rust.0.parse::<u128>()) else {
        return false;
    };
    if ours.1 != rust.1 || ours.0.len() != rust.0.len() || a.abs_diff(b) != 1 {
        return false;
    }
    let bits = x.abs().to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i32;
    let fraction = u128::from(bits & ((1u64 << 52) - 1));
    let (m, e) = if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1u128 << 52), biased - 1075)
    };
    let p = ours.1 - ours.0.len() as i32 + 1;
    let twice_middle = 2 * a.min(b) + 1;
    let scale = |mut value: u128, twos: i32, tens: i32| -> Option<u128> {
        for _ in 0..twos.max(0) {
            value = value.checked_mul(2)?;
        }
        for _ in 0..tens.max(0) {
            value = value.checked_mul(10)?;
        }
        Some(value)
    };
    let left = scale(m, e + 1, -p);
    let right = scale(twice_middle, -(e + 1), p);
    matches!((left, right), (Some(l), Some(r)) if l == r)
}

#[test]
fn to_string_then_parse_round_trips() {
    let key = *b"swell to_string\0";
    let mut finite = 0u32;
    let mut drawn = 0u64;
    let mut ties = 0u32;
    while finite < 200_000 {
        let bits = siphash24(&key, &drawn.to_le_bytes());
        drawn += 1;
        let x = f64::from_bits(bits);
        if !x.is_finite() {
            continue;
        }
        finite += 1;
        let text = number_to_string(x);
        let back = string_to_number_trimmed(text.as_bytes());
        let expected = if x == 0.0 { 0.0 } else { x };
        assert_eq!(
            back.to_bits(),
            expected.to_bits(),
            "{text} read back as {back:e}, bits {bits:#018x}"
        );
        if x == 0.0 {
            continue;
        }
        // Rust's `{:e}` gives the same shortest digits, except where x lies exactly half-way
        // between two of them: Rust then takes the upper, Note 2 of the clause the even one.
        let (ours, rust) = (our_digits(&text), rust_digits(x));
        if ours != rust {
            let even = ours.0.ends_with(['0', '2', '4', '6', '8']);
            assert!(
                even && exact_tie(x, &ours, &rust),
                "{text}: digits {ours:?}, Rust {rust:?}, bits {bits:#018x}"
            );
            ties += 1;
        }
    }
    assert_eq!(finite, 200_000);
    println!(
        "to_string_then_parse_round_trips: {finite} finite, {drawn} drawn, {ties} exact tie(s) resolved to the even digit"
    );
}

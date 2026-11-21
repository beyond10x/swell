//! The value of a numeric literal (`#sec-numericvalue`, grammar `#sec-literals-numeric-literals`)
//! and of a `StringNumericLiteral` (`#sec-stringtonumber`, `#sec-runtime-semantics-stringnumericvalue`).
//!
//! The expected roundings were printed by `awk 'BEGIN{printf "%.17g ...", ...}'` (glibc `strtod`,
//! round to nearest, ties to even): `9007199254740993` gives `9007199254740992`,
//! `2.4703282292062328e-324` gives `4.9406564584124654e-324`, `2.4703282292062327e-324` gives
//! `0`, `1.7976931348623158e308` gives `1.7976931348623157e+308`, `1.7976931348623159e308` and
//! `1e400` give `+inf`.

use swell_num::literal::{LiteralForm, numeric_literal, string_to_number_trimmed};

fn literal(text: &str) -> Option<(LiteralForm, u64)> {
    numeric_literal(text.as_bytes()).map(|l| (l.form, l.value.to_bits()))
}

fn value(text: &str) -> f64 {
    match numeric_literal(text.as_bytes()) {
        Some(l) => l.value,
        None => f64::NAN,
    }
}

fn string(text: &str) -> f64 {
    string_to_number_trimmed(text.as_bytes())
}

fn same(actual: f64, expected: f64) -> bool {
    actual.to_bits() == expected.to_bits() || (actual.is_nan() && expected.is_nan())
}

#[test]
fn decimal_literals_round_to_nearest_even() {
    let two_53 = (1u64 << 53) as f64;
    let cases: [(&str, f64); 22] = [
        ("0", 0.0),
        ("7", 7.0),
        ("10", 10.0),
        ("1_000_000", 1_000_000.0),
        ("0.5", 0.5),
        ("5.", 5.0),
        (".25", 0.25),
        ("1e3", 1000.0),
        ("1E+3", 1000.0),
        ("25e-1", 2.5),
        ("1_0.2_5e1_0", 102.5e9),
        ("9007199254740992", two_53),
        ("9007199254740993", two_53),
        ("9007199254740995", two_53 + 4.0),
        ("2.4703282292062328e-324", f64::from_bits(1)),
        ("2.4703282292062327e-324", 0.0),
        ("1.7976931348623158e308", f64::MAX),
        ("1.7976931348623159e308", f64::INFINITY),
        ("1e400", f64::INFINITY),
        ("1e-400", 0.0),
        ("0e999999999999999999999", 0.0),
        ("08.5", 8.5),
    ];
    for (text, expected) in cases {
        assert!(
            same(value(text), expected),
            "{text}: {} for {expected}",
            value(text)
        );
    }
    assert_eq!(literal("1.5").map(|l| l.0), Some(LiteralForm::Decimal));
    assert_eq!(
        literal("089").map(|l| l.0),
        Some(LiteralForm::NonOctalDecimal)
    );
    assert_eq!(
        literal("089"),
        Some((LiteralForm::NonOctalDecimal, 89f64.to_bits()))
    );
    assert_eq!(
        literal("09e1"),
        Some((LiteralForm::NonOctalDecimal, 90f64.to_bits()))
    );
    assert!(LiteralForm::NonOctalDecimal.is_legacy());
    assert!(!LiteralForm::Decimal.is_legacy());

    for text in [
        "",
        ".",
        "e1",
        "1e",
        "1e+",
        "1_",
        "_1",
        "1__0",
        "1_.5",
        "1._5",
        "1e_1",
        "0_1",
        "08_1",
        "1n",
        "1a",
        "1.5.5",
        "-1",
        "+1",
        " 1",
        "Infinity",
        "1\u{0660}",
    ] {
        assert_eq!(literal(text), None, "{text:?} is not a NumericLiteral here");
    }
}

#[test]
fn hex_and_octal_literals() {
    let two_53 = (1u64 << 53) as f64;
    let cases: [(&str, LiteralForm, f64); 16] = [
        ("0x10", LiteralForm::Hex, 16.0),
        ("0XfF", LiteralForm::Hex, 255.0),
        ("0x_f", LiteralForm::Hex, f64::NAN),
        ("0xf_f", LiteralForm::Hex, 255.0),
        ("0x1FFFFFFFFFFFFF", LiteralForm::Hex, two_53 - 1.0),
        ("0x20000000000001", LiteralForm::Hex, two_53),
        ("0x20000000000003", LiteralForm::Hex, two_53 + 4.0),
        // (2^53 + 1) * 2^36 + 1: the tie is broken upwards by the last bit.
        (
            "0x20000000000001000000001",
            LiteralForm::Hex,
            ((1u128 << 89) + (1u128 << 37)) as f64,
        ),
        ("0o17", LiteralForm::Octal, 15.0),
        ("0O7_7", LiteralForm::Octal, 63.0),
        ("0b101", LiteralForm::Binary, 5.0),
        ("0B1_0", LiteralForm::Binary, 2.0),
        ("017", LiteralForm::LegacyOctal, 15.0),
        ("00", LiteralForm::LegacyOctal, 0.0),
        ("0777", LiteralForm::LegacyOctal, 511.0),
        ("019", LiteralForm::NonOctalDecimal, 19.0),
    ];
    for (text, form, expected) in cases {
        if expected.is_nan() {
            assert_eq!(literal(text), None, "{text}");
            continue;
        }
        assert_eq!(literal(text), Some((form, expected.to_bits())), "{text}");
    }
    let huge = "0x".to_string() + &"f".repeat(300);
    assert_eq!(literal(&huge).map(|l| l.1), Some(f64::INFINITY.to_bits()));
    // f64::MAX is (2^53 - 1) * 2^971, in hexadecimal 13 digits f, then 8, then 242 digits 0.
    let largest = "0x".to_string() + &"f".repeat(13) + "8" + &"0".repeat(242);
    assert_eq!(literal(&largest).map(|l| l.1), Some(f64::MAX.to_bits()));
    let below_half = "0x".to_string() + &"f".repeat(13) + "b" + &"f".repeat(242);
    assert_eq!(literal(&below_half).map(|l| l.1), Some(f64::MAX.to_bits()));
    // Half-way between f64::MAX and 2^1024: the tie goes to the even 2^1024, which is infinity.
    let half = "0x".to_string() + &"f".repeat(13) + "c" + &"0".repeat(242);
    assert_eq!(literal(&half).map(|l| l.1), Some(f64::INFINITY.to_bits()));
    assert!(LiteralForm::LegacyOctal.is_legacy());
    assert!(!LiteralForm::Octal.is_legacy());
    for text in [
        "0x", "0xg", "0o8", "0b2", "0x1n", "0x1_", "0x1__1", "07_7", "0_7", "0x1.5", "07.5", "07e1",
    ] {
        assert_eq!(literal(text), None, "{text:?}");
    }
}

#[test]
fn string_numeric_literal_accepts_infinity_signs_and_empty() {
    let cases: [(&str, f64); 24] = [
        ("", 0.0),
        ("0", 0.0),
        ("-0", -0.0),
        ("+0", 0.0),
        ("Infinity", f64::INFINITY),
        ("+Infinity", f64::INFINITY),
        ("-Infinity", f64::NEG_INFINITY),
        ("+.5", 0.5),
        ("-.5e1", -5.0),
        ("5.", 5.0),
        ("010", 10.0),
        ("0x10", 16.0),
        ("0X1f", 31.0),
        ("0o17", 15.0),
        ("0b11", 3.0),
        ("1e1000", f64::INFINITY),
        ("-1e1000", f64::NEG_INFINITY),
        ("9007199254740993", (1u64 << 53) as f64),
        ("1e", f64::NAN),
        ("-0x1", f64::NAN),
        ("+0x1", f64::NAN),
        ("infinity", f64::NAN),
        ("1_000", f64::NAN),
        ("1n", f64::NAN),
    ];
    for (text, expected) in cases {
        assert!(
            same(string(text), expected),
            "{text:?}: {} for {expected}",
            string(text)
        );
    }
    for text in [
        ".",
        "+",
        "-",
        "e5",
        "0x",
        "1 2",
        " 1",
        "1 ",
        "--1",
        "+-1",
        "Infinityx",
        "0x1g",
        "\u{00a0}1",
    ] {
        assert!(string(text).is_nan(), "{text:?}");
    }
    let units: Vec<u16> = "-12.5e-1".encode_utf16().collect();
    assert!(same(string_to_number_trimmed(&units), -1.25));
    let wide: Vec<u16> = vec![0x0031, 0x0130];
    assert!(
        string_to_number_trimmed(&wide).is_nan(),
        "a unit above 0xFF is not narrowed"
    );
}

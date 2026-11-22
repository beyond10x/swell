//! `Number::toString ( x, radix )` (`#sec-numeric-types-number-tostring`): the shortest digits
//! that round-trip, laid out as steps 6 to 12 say.
//!
//! The round trip draws its bit patterns from the crate's own SipHash with a fixed key, reads
//! each result back with the crate's own StringToNumber, and checks the digits and the exponent
//! against the Rust standard library's `{:e}`, which prints the shortest round-trip digits in
//! radix 10 (design, decision 6: "its `{:e}` output is used in tests as a check of the radix-10
//! digits").
//!
//! The other radices are checked against values this file derives itself, not against another
//! engine: in a radix that is a power of two the digits are the bits of x written out
//! (`power_of_two_radices_are_exact`); in every radix the digits are read back exactly with the
//! big integers of [`Nat`], and checked to lie in the interval of the reals that round to x, to
//! be the closer of two neighbours that do (the even one on a tie, Note 2), and to have no
//! shorter digit string in that interval (step 5, "k is as small as possible").

#![cfg(test)]

use std::cell::RefCell;
use std::cmp::Ordering;

use swell_num::literal::string_to_number_trimmed;
use swell_num::siphash::siphash24;
use swell_num::to_string::{number_to_string, number_to_string_radix};

/// Whether the tests of drawn bit patterns run their full counts. The lane `slow` sets
/// `SWELL_FULL_COUNTS=1`; the gate runs a twentieth of each count, the first patterns of the same
/// draw, and every radix and every edge as the full count does.
fn full_counts() -> bool {
    std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

/// `full` under [`full_counts`], else a twentieth of it. With a tenth, the tests of this file
/// took 0.39 to 1.09 s of processor time each, and in two runs of the package alone at a load of
/// 97 to 146 on 17 cores, where its 22 tests share one job of the build pool and a test's time
/// follows the processor time of all of them, four took 17.2 to 21.7 s.
fn count(full: usize) -> usize {
    if full_counts() { full } else { full / 20 }
}

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

/// The first 200,000 finite bit patterns drawn with the key, written and read back; checked by
/// four tests of 50,000 each, this one and the three `to_string_then_parse_round_trips_from_<n>`
/// after it, so that each runs in a process of its own under cargo-nextest and none comes near
/// the 60 s kill at its full count (two tests of 100,000 took 4.1 and 5.1 s of processor time
/// each, and one was 57.2 s in a run of the package at a load of 100 to 112 on 17 cores). The
/// gate checks the first twentieth of each part ([`count`]); the lane `slow` all of it.
#[test]
fn to_string_then_parse_round_trips() {
    round_trips(0..50_000);
}

/// The finite patterns 50,000 to 99,999 of [`to_string_then_parse_round_trips`].
#[test]
fn to_string_then_parse_round_trips_from_50000() {
    round_trips(50_000..100_000);
}

/// The finite patterns 100,000 to 149,999 of [`to_string_then_parse_round_trips`].
#[test]
fn to_string_then_parse_round_trips_from_100000() {
    round_trips(100_000..150_000);
}

/// The finite patterns 150,000 to 199,999 of [`to_string_then_parse_round_trips`].
#[test]
fn to_string_then_parse_round_trips_from_150000() {
    round_trips(150_000..200_000);
}

/// [`to_string_then_parse_round_trips`] for the finite patterns at the indices `part`, the first
/// [`count`] of them.
fn round_trips(part: std::ops::Range<u32>) {
    let len = u32::try_from(count((part.end - part.start) as usize)).unwrap();
    let part = part.start..part.start + len;
    let key = *b"swell to_string\0";
    let mut finite = 0u32;
    let mut checked = 0u32;
    let mut drawn = 0u64;
    let mut ties = 0u32;
    while finite < part.end {
        let bits = siphash24(&key, &drawn.to_le_bytes());
        drawn += 1;
        let x = f64::from_bits(bits);
        if !x.is_finite() {
            continue;
        }
        finite += 1;
        if !part.contains(&(finite - 1)) {
            continue;
        }
        checked += 1;
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
    assert_eq!(finite, part.end);
    assert_eq!(checked, part.end - part.start);
    println!(
        "to_string_then_parse_round_trips, finite patterns {} to {}: {checked} checked, {drawn} drawn, {ties} exact tie(s) resolved to the even digit",
        part.start,
        part.end - 1
    );
}

/// The first `count` finite bit patterns drawn with `key`, as `to_string_then_parse_round_trips`
/// draws them.
fn drawn_finite(key: &[u8; 16], count: usize) -> Vec<f64> {
    let mut values = Vec::with_capacity(count);
    let mut drawn = 0u64;
    while values.len() < count {
        let x = f64::from_bits(siphash24(key, &drawn.to_le_bytes()));
        drawn += 1;
        if x.is_finite() {
            values.push(x);
        }
    }
    values
}

/// Values at the edges of the Number type, added to the drawn ones.
const EDGES: [f64; 12] = [
    5e-324,
    -5e-324,
    f64::MIN_POSITIVE,
    f64::MAX,
    -f64::MAX,
    1.0,
    0.5,
    0.1,
    1e21,
    9007199254740993.0,
    123.456,
    1.0 / 3.0,
];

/// `number_to_string_radix(x, 10)` is `Some(number_to_string(x))` on the 200,000 bit patterns
/// `to_string_then_parse_round_trips` draws, and on the edges; a radix outside 2 to 36 has no
/// result.
///
/// The 200,000 patterns are checked by eight tests of 25,000 each, this one and the seven
/// `radix_ten_entry_is_unchanged_from_<n>` after it, so that each runs in a process of its own
/// under cargo-nextest and none comes near the 60 s kill at its full count (four tests of 50,000
/// took 3.7 to 4.0 s of processor time each, and one was 59.1 s in a run of the package at a
/// load of 100 to 112 on 17 cores); this one checks the edges and the radices outside 2 to 36
/// too. The gate checks the first twentieth of each part ([`count`]); the lane `slow` all of it.
#[test]
fn radix_ten_entry_is_unchanged() {
    radix_ten_entry_on(0..25_000);
    for &x in &EDGES {
        assert_eq!(
            number_to_string_radix(x, 10),
            Some(number_to_string(x)),
            "bits {:#018x}",
            x.to_bits()
        );
    }
    for x in [f64::NAN, 0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(number_to_string_radix(x, 10), Some(number_to_string(x)));
    }
    for radix in [0, 1, 37, 100, u32::MAX] {
        assert_eq!(number_to_string_radix(1.0, radix), None, "radix {radix}");
    }
}

/// `number_to_string_radix(x, 10)` is `Some(number_to_string(x))` for the drawn patterns of
/// `radix_ten_entry_is_unchanged` at the indices `part` of the 200,000, the first [`count`] of
/// them.
fn radix_ten_entry_on(part: std::ops::Range<usize>) {
    let part = part.start..part.start + count(part.end - part.start);
    let key = *b"swell to_string\0";
    let values = drawn_finite(&key, part.end);
    for &x in &values[part] {
        assert_eq!(
            number_to_string_radix(x, 10),
            Some(number_to_string(x)),
            "bits {:#018x}",
            x.to_bits()
        );
    }
}

/// The tests of [`radix_ten_entry_is_unchanged`] after the first, each on the 25,000 patterns
/// from the index in its name.
macro_rules! radix_ten_parts {
    ($($name:ident = $from:literal),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                radix_ten_entry_on($from..$from + 25_000);
            }
        )*
    };
}

radix_ten_parts!(
    radix_ten_entry_is_unchanged_from_25000 = 25_000,
    radix_ten_entry_is_unchanged_from_50000 = 50_000,
    radix_ten_entry_is_unchanged_from_75000 = 75_000,
    radix_ten_entry_is_unchanged_from_100000 = 100_000,
    radix_ten_entry_is_unchanged_from_125000 = 125_000,
    radix_ten_entry_is_unchanged_from_150000 = 150_000,
    radix_ten_entry_is_unchanged_from_175000 = 175_000,
);

/// x as f × 2^e with f a positive integer below 2^53, for a finite x other than ±0, and
/// whether the gap below |x| is half the gap above (a power of two above the smallest normal).
fn decompose(x: f64) -> (u64, i32, bool) {
    let bits = x.abs().to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    if biased == 0 {
        (fraction, -1074, false)
    } else {
        (
            fraction | (1u64 << 52),
            biased - 1075,
            biased > 1 && fraction == 0,
        )
    }
}

/// The digits of x in radix 2^w, written out from its bits: x = f × 2^e is exact in that radix,
/// and no shorter digit string lies within half a unit in the last place of x, so these are the
/// digits of step 5, laid out by step 6.
fn bits_written_out(x: f64, w: u32) -> String {
    let (f, e, _) = decompose(x);
    let w = w as i32;
    let lowest = e + f.trailing_zeros() as i32;
    let top = e + 63 - f.leading_zeros() as i32;
    let bit = |position: i32| -> u32 {
        let i = position - e;
        if (0..64).contains(&i) {
            ((f >> i) & 1) as u32
        } else {
            0
        }
    };
    let digit = |j: i32| -> char {
        let value: u32 = (0..w).map(|b| bit(j * w + b) << b).sum();
        char::from_digit(value, 1 << w).unwrap()
    };
    let (high, low) = (top.div_euclid(w), lowest.div_euclid(w));
    let mut out = String::new();
    if x < 0.0 {
        out.push('-');
    }
    if high >= 0 {
        for j in (0..=high).rev() {
            out.push(digit(j));
        }
        if low < 0 {
            out.push('.');
            for j in (low..0).rev() {
                out.push(digit(j));
            }
        }
    } else {
        out.push_str("0.");
        for j in (low..0).rev() {
            out.push(digit(j));
        }
    }
    out
}

/// In the radices 2, 4, 8, 16 and 32 the result is the bits of x written out
/// ([`bits_written_out`]), on 10,000 drawn bit patterns and the edges. Two tests check them,
/// this one patterns 0 to 4,999 and `power_of_two_radices_are_exact_from_5000` the others, each
/// with the edges, so that neither comes near the 60 s kill at its full count (one test of
/// 10,000 took 4.7 s of processor time and 51.3 s in a run of the package at a load of 100 to
/// 112 on 17 cores); the gate checks the first twentieth of each part and the edges
/// ([`count`]), the lane `slow` all of them.
#[test]
fn power_of_two_radices_are_exact() {
    power_of_two_on(0..5_000);
    // Written out by hand from the bits. 0.1 is 0x3fb999999999999a: 1.999999999999a (hex) ×
    // 2^-4. -6.5 is 1 × 4 + 2 + 2 / 4. 5e-324 is 2^-1074 = 2 × 32^-215. f64::MAX is
    // (2^53 - 1) × 2^971 = 0x1fffffffffffff × 2^3 × 16^242.
    let tenth = format!("0.0001{}101", "1001".repeat(12));
    assert_eq!(number_to_string_radix(0.1, 2), Some(tenth));
    assert_eq!(number_to_string_radix(-6.5, 4).as_deref(), Some("-12.2"));
    let smallest = format!("0.{}2", "0".repeat(214));
    assert_eq!(number_to_string_radix(5e-324, 32), Some(smallest));
    let largest = format!("{}8{}", "f".repeat(13), "0".repeat(242));
    assert_eq!(number_to_string_radix(f64::MAX, 16), Some(largest));
}

/// Patterns 5,000 to 9,999 of [`power_of_two_radices_are_exact`].
#[test]
fn power_of_two_radices_are_exact_from_5000() {
    power_of_two_on(5_000..10_000);
}

/// [`power_of_two_radices_are_exact`] for the drawn patterns at the indices `part` of the
/// 10,000, the first [`count`] of them, and the edges.
fn power_of_two_on(part: std::ops::Range<usize>) {
    let part = part.start..part.start + count(part.end - part.start);
    let key = *b"swell radix bits";
    let values = drawn_finite(&key, part.end);
    for w in 1..=5u32 {
        let radix = 1u32 << w;
        for &x in values[part.clone()].iter().chain(EDGES.iter()) {
            if x == 0.0 {
                continue;
            }
            assert_eq!(
                number_to_string_radix(x, radix).as_deref(),
                Some(bits_written_out(x, w).as_str()),
                "radix {radix}, bits {:#018x}",
                x.to_bits()
            );
        }
    }
}

/// An unsigned integer of any size, 32-bit limbs, least significant first, no zero limb at the
/// top: the exact reading of the test.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Nat(Vec<u32>);

impl Nat {
    fn from_u64(value: u64) -> Nat {
        let mut nat = Nat(vec![value as u32, (value >> 32) as u32]);
        nat.trim();
        nat
    }

    fn trim(&mut self) {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    fn mul_add_small(&mut self, factor: u32, addend: u32) {
        let mut carry = u64::from(addend);
        for limb in &mut self.0 {
            let product = u64::from(*limb) * u64::from(factor) + carry;
            *limb = product as u32;
            carry = product >> 32;
        }
        if carry != 0 {
            self.0.push(carry as u32);
        }
        self.trim();
    }

    fn add_small(&self, addend: u32) -> Nat {
        let mut sum = self.clone();
        sum.mul_add_small(1, addend);
        sum
    }

    /// self - 1, for self ≥ 1.
    fn sub_one(&self) -> Nat {
        let mut difference = self.clone();
        for limb in &mut difference.0 {
            let (value, borrow) = limb.overflowing_sub(1);
            *limb = value;
            if !borrow {
                break;
            }
        }
        difference.trim();
        difference
    }

    /// floor(self / divisor).
    fn div_small(&self, divisor: u32) -> Nat {
        let mut quotient = vec![0u32; self.0.len()];
        let mut remainder = 0u64;
        for (i, &limb) in self.0.iter().enumerate().rev() {
            let current = (remainder << 32) | u64::from(limb);
            quotient[i] = (current / u64::from(divisor)) as u32;
            remainder = current % u64::from(divisor);
        }
        let mut quotient = Nat(quotient);
        quotient.trim();
        quotient
    }

    fn mul(&self, other: &Nat) -> Nat {
        let mut limbs = vec![0u32; self.0.len() + other.0.len()];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &b) in other.0.iter().enumerate() {
                let current = u64::from(limbs[i + j]) + u64::from(a) * u64::from(b) + carry;
                limbs[i + j] = current as u32;
                carry = current >> 32;
            }
            let mut k = i + other.0.len();
            while carry != 0 {
                let current = u64::from(limbs[k]) + carry;
                limbs[k] = current as u32;
                carry = current >> 32;
                k += 1;
            }
        }
        let mut product = Nat(limbs);
        product.trim();
        product
    }

    fn pow(base: u32, exponent: u32) -> Nat {
        let mut power = Nat::from_u64(1);
        for _ in 0..exponent {
            power.mul_add_small(base, 0);
        }
        power
    }

    fn shl(&self, bits: u32) -> Nat {
        let mut limbs = vec![0u32; (bits / 32) as usize];
        let shift = bits % 32;
        let mut carry = 0u32;
        for &limb in &self.0 {
            if shift == 0 {
                limbs.push(limb);
            } else {
                limbs.push((limb << shift) | carry);
                carry = limb >> (32 - shift);
            }
        }
        if carry != 0 {
            limbs.push(carry);
        }
        let mut shifted = Nat(limbs);
        shifted.trim();
        shifted
    }

    fn compare(&self, other: &Nat) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

/// The exact reading of a result in `radix`: the value of the digits as D × radix^p, D without
/// a trailing zero digit, and k, the number of digits of D. Checks the form steps 6 to 12 give:
/// an optional "-", digits of the radix in lower case, a "." only between digits, no leading
/// zero before a digit, no trailing zero after "."; an exponent ("e", a sign, decimal digits)
/// only in radix 10, after a single digit or a digit, "." and more digits.
fn read_exactly(text: &str, radix: u32) -> (Nat, i32, usize) {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (unsigned, exponent) = match unsigned.split_once('e') {
        Some((mantissa, exponent)) if radix == 10 => {
            assert!(
                exponent.starts_with('+') || exponent.starts_with('-'),
                "{text}: an exponent without its sign"
            );
            let (head, _) = mantissa.split_once('.').unwrap_or((mantissa, ""));
            assert!(
                head.len() == 1 && head != "0",
                "{text}: not one non-zero digit before the point"
            );
            (mantissa, exponent.parse::<i32>().unwrap())
        }
        _ => (unsigned, 0),
    };
    let (int, frac) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    assert!(!int.is_empty(), "{text}: no integer digits");
    assert!(
        !unsigned.contains('.') || !frac.is_empty(),
        "{text}: nothing after the point"
    );
    assert!(
        int == "0" || !int.starts_with('0'),
        "{text}: a leading zero"
    );
    assert!(
        !frac.ends_with('0'),
        "{text}: a trailing zero after the point"
    );
    let mut digits: Vec<u32> = Vec::new();
    for c in int.chars().chain(frac.chars()) {
        assert!(!c.is_ascii_uppercase(), "{text}: an upper-case digit");
        let Some(digit) = c.to_digit(radix) else {
            panic!("{text}: {c:?} is not a digit of radix {radix}");
        };
        digits.push(digit);
    }
    let mut p = exponent - frac.len() as i32;
    while digits.len() > 1 && digits.last() == Some(&0) {
        digits.pop();
        p += 1;
    }
    let first = digits.iter().position(|&d| d != 0).unwrap_or(digits.len());
    let significant = &digits[first..];
    let mut d = Nat(Vec::new());
    for &digit in significant {
        d.mul_add_small(radix, digit);
    }
    (d, p, significant.len())
}

/// The interval of the reals that round to the positive finite x (round to nearest, ties to
/// even), and comparisons of D × radix^p with its bounds and with x, exactly.
struct Interval {
    f: u64,
    e: i32,
    unequal: bool,
    radix: u32,
    /// radix^|p| for the exponents p asked so far.
    powers: RefCell<Vec<(u32, Nat)>>,
}

impl Interval {
    fn new(x: f64, radix: u32) -> Interval {
        let (f, e, unequal) = decompose(x);
        Interval {
            f,
            e,
            unequal,
            radix,
            powers: RefCell::new(Vec::new()),
        }
    }

    /// radix^exponent.
    fn power(&self, exponent: u32) -> Nat {
        let mut powers = self.powers.borrow_mut();
        if let Some((_, power)) = powers.iter().find(|(e, _)| *e == exponent) {
            return power.clone();
        }
        let power = Nat::pow(self.radix, exponent);
        powers.push((exponent, power.clone()));
        power
    }

    /// D × radix^p compared with A × 2^q.
    fn compare(&self, d: &Nat, p: i32, a: &Nat, q: i32) -> Ordering {
        let power = self.power(p.unsigned_abs());
        let (mut left, mut right) = if p >= 0 {
            (d.mul(&power), a.clone())
        } else {
            (d.clone(), a.mul(&power))
        };
        if q >= 0 {
            right = right.shl(q as u32);
        } else {
            left = left.shl(q.unsigned_abs());
        }
        left.compare(&right)
    }

    /// Whether D × radix^p reads back as x: inside the interval, its bounds included when f is
    /// even.
    fn reads_back(&self, d: &Nat, p: i32) -> bool {
        let even = self.f.is_multiple_of(2);
        let (low, low_q) = if self.unequal {
            (Nat::from_u64(4 * self.f - 1), self.e - 2)
        } else {
            (Nat::from_u64(2 * self.f - 1), self.e - 1)
        };
        let high = Nat::from_u64(2 * self.f + 1);
        let above = match self.compare(d, p, &low, low_q) {
            Ordering::Greater => true,
            Ordering::Equal => even,
            Ordering::Less => false,
        };
        let below = match self.compare(d, p, &high, self.e - 1) {
            Ordering::Less => true,
            Ordering::Equal => even,
            Ordering::Greater => false,
        };
        above && below
    }

    /// The midpoint of D × radix^p and (D + 1) × radix^p compared with x:
    /// (2D + 1) × radix^p against f × 2^(e+1).
    fn midpoint_above(&self, d: &Nat, p: i32) -> Ordering {
        let twice = d.shl(1).add_small(1);
        self.compare(&twice, p, &Nat::from_u64(self.f), self.e + 1)
    }
}

/// Checks `text`, the result of `number_to_string_radix(x, radix)` for a finite x other than
/// ±0, against step 5 and Note 2: its digits read back as x, no digit string of the same length
/// next to it reads back as x and is closer (of two equally close, s is even), and no shorter
/// digit string reads back as x.
fn check_shortest(x: f64, radix: u32, text: &str) {
    assert_eq!(text.starts_with('-'), x < 0.0, "{text}: sign of {x:e}");
    let (d, p, k) = read_exactly(text, radix);
    let interval = Interval::new(x, radix);
    let context = format!("radix {radix}, {text}, bits {:#018x}", x.to_bits());
    assert!(interval.reads_back(&d, p), "does not read back: {context}");
    let even = d.0.first().copied().unwrap_or(0).is_multiple_of(2);
    // The neighbour above: x at or below the midpoint, on it only when s is even.
    let up = d.add_small(1);
    if interval.reads_back(&up, p) {
        match interval.midpoint_above(&d, p) {
            Ordering::Greater => {}
            Ordering::Equal => assert!(even, "an odd s on a tie: {context}"),
            Ordering::Less => panic!("the digits above are closer: {context}"),
        }
    }
    // The neighbour below: x at or above the midpoint.
    let down = d.sub_one();
    if !down.is_zero() && interval.reads_back(&down, p) {
        match interval.midpoint_above(&down, p) {
            Ordering::Less => {}
            Ordering::Equal => assert!(even, "an odd s on a tie: {context}"),
            Ordering::Greater => panic!("the digits below are closer: {context}"),
        }
    }
    // A shorter digit string near x is a multiple of radix^(p+1): the two around x.
    if k > 1 {
        let shorter = d.div_small(radix);
        for candidate in [shorter.clone(), shorter.add_small(1)] {
            assert!(
                candidate.is_zero() || !interval.reads_back(&candidate, p + 1),
                "a shorter digit string reads back: {candidate:?} × {radix}^{}: {context}",
                p + 1
            );
        }
    }
}

/// For each radix from 2 to 36, on 10,000 drawn bit patterns and the edges, the digits read
/// back as x exactly and no shorter digit string does ([`check_shortest`]); NaN, the zeros and
/// the infinities are written as in every radix.
///
/// The 35 radices are checked in seven groups of five, each group by tests of a range of the
/// patterns, each with the edges: radices 2 to 6 by five tests of 2,000 patterns (this one and
/// `every_radix_reads_back_from_<n>`), every other group by four tests of 2,500
/// (`every_radix_reads_back_<from>_to_<to>` and `..._from_<n>`). So each runs in a process of
/// its own under cargo-nextest and none comes near the 60 s kill at its full count: one test per
/// group of all 10,000 took 5.0 to 10.6 s of processor time (radices 2 to 6 the most), and two
/// were killed at 60 s in a run of the package at a load of 100 to 112 on 17 cores.
#[test]
fn every_radix_reads_back() {
    reads_back_in(2..=6, 0..2_000);
}

/// The tests of [`every_radix_reads_back`] after the first, each on its radices and its range of
/// the 10,000 patterns.
macro_rules! reads_back_parts {
    ($($name:ident = $radices:expr, $patterns:expr);* $(;)?) => {
        $(
            #[test]
            fn $name() {
                reads_back_in($radices, $patterns);
            }
        )*
    };
}

reads_back_parts!(
    every_radix_reads_back_from_2000 = 2..=6, 2_000..4_000;
    every_radix_reads_back_from_4000 = 2..=6, 4_000..6_000;
    every_radix_reads_back_from_6000 = 2..=6, 6_000..8_000;
    every_radix_reads_back_from_8000 = 2..=6, 8_000..10_000;
    every_radix_reads_back_7_to_11 = 7..=11, 0..2_500;
    every_radix_reads_back_7_to_11_from_2500 = 7..=11, 2_500..5_000;
    every_radix_reads_back_7_to_11_from_5000 = 7..=11, 5_000..7_500;
    every_radix_reads_back_7_to_11_from_7500 = 7..=11, 7_500..10_000;
    every_radix_reads_back_12_to_16 = 12..=16, 0..2_500;
    every_radix_reads_back_12_to_16_from_2500 = 12..=16, 2_500..5_000;
    every_radix_reads_back_12_to_16_from_5000 = 12..=16, 5_000..7_500;
    every_radix_reads_back_12_to_16_from_7500 = 12..=16, 7_500..10_000;
    every_radix_reads_back_17_to_21 = 17..=21, 0..2_500;
    every_radix_reads_back_17_to_21_from_2500 = 17..=21, 2_500..5_000;
    every_radix_reads_back_17_to_21_from_5000 = 17..=21, 5_000..7_500;
    every_radix_reads_back_17_to_21_from_7500 = 17..=21, 7_500..10_000;
    every_radix_reads_back_22_to_26 = 22..=26, 0..2_500;
    every_radix_reads_back_22_to_26_from_2500 = 22..=26, 2_500..5_000;
    every_radix_reads_back_22_to_26_from_5000 = 22..=26, 5_000..7_500;
    every_radix_reads_back_22_to_26_from_7500 = 22..=26, 7_500..10_000;
    every_radix_reads_back_27_to_31 = 27..=31, 0..2_500;
    every_radix_reads_back_27_to_31_from_2500 = 27..=31, 2_500..5_000;
    every_radix_reads_back_27_to_31_from_5000 = 27..=31, 5_000..7_500;
    every_radix_reads_back_27_to_31_from_7500 = 27..=31, 7_500..10_000;
    every_radix_reads_back_32_to_36 = 32..=36, 0..2_500;
    every_radix_reads_back_32_to_36_from_2500 = 32..=36, 2_500..5_000;
    every_radix_reads_back_32_to_36_from_5000 = 32..=36, 5_000..7_500;
    every_radix_reads_back_32_to_36_from_7500 = 32..=36, 7_500..10_000;
);

/// [`every_radix_reads_back`] for the radices of `radices` on the drawn patterns at the indices
/// `part` of the 10,000, the first [`count`] of them (the gate a twentieth, the lane `slow`
/// all), and on every edge.
fn reads_back_in(radices: std::ops::RangeInclusive<u32>, part: std::ops::Range<usize>) {
    let part = part.start..part.start + count(part.end - part.start);
    let key = *b"swell radix test";
    let values = drawn_finite(&key, part.end);
    let mut checked = 0u32;
    let (first, last) = (*radices.start(), *radices.end());
    for radix in radices {
        for &x in values[part.clone()].iter().chain(EDGES.iter()) {
            if x == 0.0 {
                continue;
            }
            let text = number_to_string_radix(x, radix).unwrap();
            check_shortest(x, radix, &text);
            checked += 1;
        }
        assert_eq!(
            number_to_string_radix(f64::NAN, radix).as_deref(),
            Some("NaN")
        );
        assert_eq!(number_to_string_radix(0.0, radix).as_deref(), Some("0"));
        assert_eq!(number_to_string_radix(-0.0, radix).as_deref(), Some("0"));
        assert_eq!(
            number_to_string_radix(f64::INFINITY, radix).as_deref(),
            Some("Infinity")
        );
        assert_eq!(
            number_to_string_radix(f64::NEG_INFINITY, radix).as_deref(),
            Some("-Infinity")
        );
    }
    println!(
        "every_radix_reads_back, radices {first} to {last}, patterns {} to {}: {checked} result(s) checked",
        part.start,
        part.end - 1
    );
}

/// The layout of steps 6 to 12 in a radix other than 10: the clause's examples worked out by
/// hand. 255 is 15 × 16 + 15 and 7 × 36 + 3; 0.5 is 1 × 2^-1; 1e21 lies between 36^13 and 36^14,
/// so it has 14 digits before the point and, the radix not being 10, no exponent.
#[test]
fn radix_layout_examples() {
    assert_eq!(number_to_string_radix(255.0, 16).as_deref(), Some("ff"));
    assert_eq!(number_to_string_radix(0.5, 2).as_deref(), Some("0.1"));
    assert_eq!(number_to_string_radix(-255.0, 36).as_deref(), Some("-73"));
    assert_eq!(number_to_string_radix(35.0, 36).as_deref(), Some("z"));
    assert_eq!(number_to_string_radix(36.0, 36).as_deref(), Some("10"));
    assert_eq!(number_to_string_radix(-0.25, 2).as_deref(), Some("-0.01"));
    // 1.5 is 1.111… in radix 3, and 1.5 ± 3^-33 / 2 both lie within 2^-53 of it, as no point
    // with 32 digits after it does (3^-32 / 2 > 2^-53 > 3^-33 / 2): of 1.(33 ones) and
    // 1.(32 ones)2, equally close, s is even for the first (its digit sum, 34, is even; the
    // radix is odd).
    let ones = format!("1.{}", "1".repeat(33));
    assert_eq!(number_to_string_radix(1.5, 3), Some(ones));
    let big = number_to_string_radix(1e21, 36).unwrap();
    assert_eq!(big.len(), 14, "{big}");
    assert!(
        big.chars()
            .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase()),
        "{big}"
    );
    assert_eq!(number_to_string(1e21), "1e+21");
}

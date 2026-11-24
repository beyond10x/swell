//! `TimeClip` (`#sec-timeclip`, 21.4.1.31) and the bound of a time value, 8.64 × 10^15
//! milliseconds (`#sec-time-values-and-time-range`).
//!
//! Where the numbers come from: `8.64e15 + 1` and `8.64e15 + 2` are the Numbers just above the
//! bound, below 2^53 where every Number is an integer, so each is exact; `2^53 + 2` is the
//! first even Number above 2^53; `5e-324` is the smallest positive Number (`Number.MIN_VALUE`).

use swell_num::time_value::{MAX_TIME_VALUE, time_clip};

/// Whether `value` is +0 (and not -0), read by its bits.
fn is_positive_zero(value: f64) -> bool {
    value.to_bits() == 0.0_f64.to_bits()
}

/// 21.4.1.31 step 1: a Number that is not finite is NaN.
#[test]
fn time_clip_of_a_number_that_is_not_finite_is_nan() {
    assert!(time_clip(f64::NAN).is_nan());
    assert!(time_clip(f64::INFINITY).is_nan());
    assert!(time_clip(f64::NEG_INFINITY).is_nan());
}

/// 21.4.1.31 step 2: the bound is kept, the Numbers above it are NaN, on both sides.
#[test]
fn time_clip_keeps_the_bound_and_refuses_what_is_past_it() {
    assert_eq!(MAX_TIME_VALUE, 8.64e15);
    assert_eq!(time_clip(8.64e15), 8.64e15);
    assert_eq!(time_clip(-8.64e15), -8.64e15);
    assert_eq!(8.64e15 + 1.0, 8_640_000_000_000_001.0);
    assert!(time_clip(8.64e15 + 1.0).is_nan());
    assert!(time_clip(8.64e15 + 2.0).is_nan());
    assert!(time_clip(-8.64e15 - 1.0).is_nan());
    assert!(time_clip(9_007_199_254_740_994.0).is_nan());
}

/// 21.4.1.31 step 3: 𝔽(! ToIntegerOrInfinity(time)), which truncates toward zero and makes -0
/// and every fraction between -1 and 1 +0.
#[test]
fn time_clip_truncates_toward_zero_and_answers_positive_zero() {
    assert_eq!(time_clip(1.9), 1.0);
    assert_eq!(time_clip(-1.9), -1.0);
    assert!(is_positive_zero(time_clip(-0.0)));
    assert!(is_positive_zero(time_clip(0.0)));
    assert!(is_positive_zero(time_clip(0.9)));
    assert!(is_positive_zero(time_clip(-0.9)));
    assert!(is_positive_zero(time_clip(5e-324)));
}

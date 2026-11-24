//! Time values (ECMA-262 21.4.1.1, `#sec-time-values-and-time-range`) and `TimeClip`
//! (21.4.1.31, `#sec-timeclip`), written by `story:date-object-clock-and-time-value`.
//!
//! Every operation here is exact on every platform: `abs` and `trunc` (IEEE 754-2019, 5.5.1
//! and 5.9), the comparisons, and the addition of +0, whose sum is +0 for a zero of either sign
//! in round-to-nearest (5.3.1). No cast.

/// The largest magnitude of a time value: 8.64 × 10^15 milliseconds, 100,000,000 days either
/// side of the epoch (`#sec-time-values-and-time-range`).
pub const MAX_TIME_VALUE: f64 = 8.64e15;

/// TimeClip ( time ) (`#sec-timeclip`).
pub fn time_clip(time: f64) -> f64 {
    // 1. If time is not finite, return NaN.
    if !time.is_finite() {
        return f64::NAN;
    }
    // 2. If abs(ℝ(time)) > 8.64 × 10^15, return NaN.
    if time.abs() > MAX_TIME_VALUE {
        return f64::NAN;
    }
    // 3. Return 𝔽(! ToIntegerOrInfinity(time)): the integer toward zero, and +0 for -0 (adding
    // +0 turns -0 into +0 and leaves every other value as it is).
    time.trunc() + 0.0
}

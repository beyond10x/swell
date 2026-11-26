// Temporary stub: replaced by the real fmath of step 2 at merge.
//! The float functions of `swell` as free functions; this stub forwards each to the method or
//! operator of `std`.

/// The square root of `x`.
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

/// The largest integer not above `x`.
pub fn floor(x: f64) -> f64 {
    x.floor()
}

/// The smallest integer not below `x`.
pub fn ceil(x: f64) -> f64 {
    x.ceil()
}

/// `x` rounded toward zero.
pub fn trunc(x: f64) -> f64 {
    x.trunc()
}

/// The least non-negative remainder of `x` divided by `y`.
pub fn rem_euclid(x: f64, y: f64) -> f64 {
    x.rem_euclid(y)
}

/// The remainder of `x` divided by `y`, truncated toward zero, as C's `fmod`.
pub fn fmod(x: f64, y: f64) -> f64 {
    x % y
}

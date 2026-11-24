#![forbid(unsafe_code)]
//! `swell-num`, the number code of swell (design, decisions 2 and 6). It depends on nothing and
//! holds no engine value: it works on `f64`, integers and code units.
//!
//! Slice 1 takes these parts early (design, section 12, "A part before its slice"):
//!
//! - [`siphash`]: SipHash-2-4, the keyed hash of every hash map of the engine (decision 4).
//! - [`literal`]: the value of a numeric literal (`#sec-numericvalue`) and `StringToNumber` of a
//!   trimmed string (`#sec-stringtonumber`).
//! - [`to_string`]: `Number::toString` (`#sec-numeric-types-number-tostring`) for radix 10.
//! - [`date`]: `MakeDay`, `MakeTime`, `MakeDate` and `GetUTCEpochNanoseconds`
//!   (`#sec-getutcepochnanoseconds`).
//!
//! Slice 2 takes the other radices of [`to_string`] early, every radix from 2 to 36, for
//! `Number.prototype.toString` (`story:number-boolean-and-global-functions-early`).
//!
//! Slice 4 adds `radix.rs` (reading a string in a radix, for `parseInt` and `parseFloat`),
//! `math.rs` and `random.rs`; slice 12 adds `bigint.rs`.

pub mod date;
pub mod literal;
pub mod math;
pub mod siphash;
pub mod to_string;

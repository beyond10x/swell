#![no_std]
#![forbid(unsafe_code)]
//! `swell-num`, the number code of swell (design, decisions 2 and 6). It depends on nothing and
//! holds no engine value: it works on `f64`, integers and code units.
//!
//! It is `no_std` with `alloc` (`story:core-builds-without-std`): it takes from the platform
//! only the allocator of the program that links it. No `f64` method of `std` is called; the
//! ones `core` lacks are the functions of [`fmath`].
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
//! Slice 4 adds `date_text.rs` (the text of a date: the parser and the writers), `format.rs`
//! (the digits of `toFixed`, `toExponential` and `toPrecision`), `math.rs`, `nat.rs` (the exact
//! unsigned integer type), `parse.rs` (reading a string in a radix, for `parseInt` and
//! `parseFloat`), `random.rs`, `sum.rs` (the exact accumulator of `Math.sumPrecise`) and
//! `time_value.rs` (`time_clip` and the time-value helpers);
//! slice 12 adds `bigint.rs`.

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod date;
pub mod date_text;
pub mod fmath;
pub mod format;
pub mod literal;
pub mod math;
mod nat;
pub mod parse;
pub mod siphash;
pub mod sum;
pub mod time_value;
pub mod to_string;

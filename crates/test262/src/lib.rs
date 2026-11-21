#![forbid(unsafe_code)]
//! `swell-test262`, the conformance runner of swell (design, decision 11): it reads every case of
//! a pinned test262 checkout, assigns its directory and its delivery slice, counts it as excluded,
//! not applicable or run, runs every run case on the engine, and prints the counts. It depends on
//! `swell`, and on `swell-syntax` for the kind of a parse error's message; only [`run`] and
//! `host` call the engine: a fresh agent and realm per run, with the runner's host.
//!
//! - [`front_matter`]: the reader of a case's `/*---` ... `---*/` block.
//! - [`slices`]: the `swell-slices` block of the design and the slice of a case.
//! - [`features`]: `conformance/test262/features.txt` and its checks.
//! - [`count`]: the cases, their directories, the totals, the slices and the exclusions.
//! - [`run`]: one run of a case on the engine, on worker threads, under the runner's limits.
//! - `host`: the runner's `Host`, with `print` and `$262`.
//! - [`sections`]: `conformance/test262/sections.txt` and the judgement of each section.
//! - [`expected`]: the comparison of the printed lines with `conformance/test262/expected.txt`.

pub mod count;
pub mod expected;
pub mod features;
pub mod front_matter;
mod host;
pub mod run;
pub mod sections;
pub mod slices;

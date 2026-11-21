#![forbid(unsafe_code)]
//! `swell-test262`, the conformance runner of swell (design, decision 11): it reads every case of
//! a pinned test262 checkout, assigns its directory and its delivery slice, counts it as excluded,
//! not applicable or run, and prints the counts. It has no dependency and does not call the
//! engine: [`count::run_outcome`] is the one place that gives the outcome of a run, and slice 1
//! changes it.
//!
//! - [`front_matter`]: the reader of a case's `/*---` ... `---*/` block.
//! - [`slices`]: the `swell-slices` block of the design and the slice of a case.
//! - [`features`]: `conformance/test262/features.txt` and its checks.
//! - [`count`]: the cases, their directories, the totals, the slices and the exclusions.
//! - [`sections`]: `conformance/test262/sections.txt` and the judgement of each section.
//! - [`expected`]: the comparison of the printed lines with `conformance/test262/expected.txt`.

pub mod count;
pub mod expected;
pub mod features;
pub mod front_matter;
pub mod sections;
pub mod slices;

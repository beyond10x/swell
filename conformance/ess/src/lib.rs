#![forbid(unsafe_code)]
//! The scenarios of the ESS model under `spec/` (the suite `task suite` synthesizes), run against
//! the hand-written host interface of `swell`, in process (engine design, decision 10).
//!
//! This is a workspace of its own, tooling that nothing shipped depends on: it takes the runner,
//! `ess-conformance`, by its pinned tag, and has its own `Cargo.lock`. [`SwellTarget`] is the
//! adapter between the runner and the interface (`target.rs`); [`run`] admits a suite's bytes,
//! runs it against a target on a thread of [`STACK_BYTES`] and returns the runner's result with
//! its `ess-conformance-report/2`. [`shipped`] is the target over the interface as it is; the
//! planted-defect test builds one whose interrupt does nothing.
//!
//! A scenario the target cannot run is reported `unsupported` with the target's reason, never
//! passed: the commands without an entry point in slice 1, the answers of the interface the
//! model has no outcome for, and the forced outcomes the scenario's own input cannot give.

#![deny(missing_docs)]

mod target;
mod values;

pub use target::{ConformanceHost, Interrupter, STACK_BYTES, SwellTarget, sets_the_flag};

use ess_conformance::target::ImplementationIdentity;
use ess_conformance::{AdmittedSuite, CountReport, ExecutedRun, Runner};

/// The host interface as it is: the interrupt flag set by `InterruptHandle::interrupt`.
pub fn shipped() -> SwellTarget {
    SwellTarget::new(
        ImplementationIdentity::new(
            "swell",
            concat!(env!("CARGO_PKG_VERSION"), ", rust, in process"),
        ),
        sets_the_flag,
    )
}

/// A run of one suite: what the runner executed, and its `ess-conformance-report/2`.
pub struct Conformance {
    /// The runner's result, scenario by scenario.
    pub run: ExecutedRun,
    /// The count report over the exact suite bytes that ran.
    pub report: CountReport,
}

/// Admits `suite`, the bytes `task suite` writes, and runs it against the target `target`
/// makes, on a thread of [`STACK_BYTES`] (an agent is not `Send`, so the target is made there).
pub fn run(suite: &str, target: fn() -> SwellTarget) -> Result<Conformance, String> {
    let suite = suite.to_owned();
    std::thread::Builder::new()
        .name("swell-conformance".to_owned())
        .stack_size(STACK_BYTES)
        .spawn(move || {
            let admitted = AdmittedSuite::from_json(&suite).map_err(|error| error.to_string())?;
            let target = target();
            let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &target);
            let report =
                CountReport::from_run(&run, &admitted).map_err(|error| error.to_string())?;
            Ok(Conformance { run, report })
        })
        .map_err(|error| format!("starting the run's thread: {error}"))?
        .join()
        .map_err(|_| "the run's thread panicked".to_owned())?
}

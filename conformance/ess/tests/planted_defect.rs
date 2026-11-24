#![forbid(unsafe_code)]
//! The planted defects: the suite over a target with one defect planted must fail at least one
//! scenario, and each scenario it fails must pass over the shipped target, so that it fails for
//! the defect and nothing else.
//!
//! - A target whose `swell.host.Interrupt` leaves the agent's interrupt flag unset. A suite that
//!   passes over it would pass over an engine whose interrupt handle does nothing (HOST-92). A
//!   forced `interrupted` sets no flag in either target: the run reaches it only through the
//!   scenario's `Interrupt`.
//! - A target whose `swell.host.PerformMicrotaskCheckpoint` answers `Normal` without running a
//!   job. A suite that passes over it would pass over an engine whose checkpoint leaves its
//!   promise jobs queued (HOST-79).
//!
//! The suite is the file `task suite` writes: `ESS_SUITE` when set (the task `conform-engine`
//! sets it), `.out/suite.json` at the root of the repository otherwise. A missing suite stops
//! the test with a panic; `cfg(test)` lets `clippy.toml` treat the helpers as test code.
#![cfg(test)]

use ess_conformance::target::ImplementationIdentity;
use swell::{Agent, CheckpointReport, HostError, InterruptHandle, Outcome};
use swell_conformance::{Conformance, ConformanceHost, SwellTarget, run, shipped};

/// The suite `task suite` writes, unless `ESS_SUITE` names another.
fn suite() -> String {
    let path = std::env::var("ESS_SUITE").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../.out/suite.json").to_owned()
    });
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{path}: {error}; run task suite first"))
}

/// The defect: setting the interrupt flag stores nothing.
fn leaves_the_flag_unset(_handle: &InterruptHandle) {}

/// The shipped target with the defect planted.
fn planted() -> SwellTarget {
    SwellTarget::new(
        ImplementationIdentity::new("swell", "planted: Interrupt leaves the flag unset"),
        leaves_the_flag_unset,
    )
}

/// The defect: the checkpoint answers `Normal` and runs nothing, the queue left as it was. A
/// faulted agent still answers `Err(Faulted)`, so the defect is the queue alone.
fn runs_no_job(agent: &mut Agent<ConformanceHost>) -> Result<Outcome<CheckpointReport>, HostError> {
    agent.pending_microtasks()?;
    Ok(Outcome::Normal(CheckpointReport {
        promise_jobs: 0,
        host_microtasks: 0,
    }))
}

/// The shipped target with a checkpoint that runs no job.
fn planted_checkpoint() -> SwellTarget {
    SwellTarget::new(
        ImplementationIdentity::new("swell", "planted: the checkpoint runs no job"),
        swell_conformance::sets_the_flag,
    )
    .with_checkpoint(runs_no_job)
}

/// The scenarios a run reports with `status`.
fn with_status(conformance: &Conformance, status: &str) -> Vec<String> {
    conformance
        .report
        .statuses()
        .into_iter()
        .filter(|(_, reported)| *reported == status)
        .map(|(scenario, _)| scenario)
        .collect()
}

/// The suite over the target `planted` makes fails a scenario, and each scenario it fails passes
/// over the shipped target; `defect` names the defect in the message.
fn fails_a_scenario_for_the_defect(planted: fn() -> SwellTarget, defect: &str) {
    let suite = suite();
    let defective = run(&suite, planted).expect("the suite runs over the planted target");
    let counts = defective.report.counts();
    eprintln!(
        "planted defect: {} scenarios: {} passed, {} failed, {} error, {} unsupported",
        counts.total, counts.passed, counts.failed, counts.error, counts.unsupported
    );
    let failed = with_status(&defective, "failed");
    for scenario in &failed {
        eprintln!("  failed {scenario}");
    }
    assert!(
        !failed.is_empty(),
        "no scenario failed over a target whose {defect}"
    );
    let correct = run(&suite, shipped).expect("the suite runs over the shipped target");
    let passed = with_status(&correct, "passed");
    for scenario in &failed {
        assert!(
            passed.contains(scenario),
            "{scenario} fails over the planted target and does not pass over the shipped one, so \
             it fails for another reason than the defect"
        );
    }
}

#[test]
fn a_target_whose_interrupt_does_nothing_fails_a_scenario() {
    fails_a_scenario_for_the_defect(planted, "interrupt leaves the flag unset");
}

#[test]
fn a_target_whose_checkpoint_runs_no_job_fails_a_scenario() {
    fails_a_scenario_for_the_defect(planted_checkpoint, "checkpoint runs no job");
}

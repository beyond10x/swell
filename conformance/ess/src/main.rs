#![forbid(unsafe_code)]
//! `swell-conformance <suite.json> <report.json>`: runs the suite against the host interface of
//! swell, prints the diagnostic of every check that did not pass, then one line per scenario and
//! the runner's summary line last, and writes the `ess-conformance-report/2` to `<report.json>`.
//!
//! Exit status 0 when the suite has scenarios and none failed or errored (an `unsupported`
//! scenario is counted, and the task `conform-engine` holds the count under its ceiling), 1 when
//! one failed or errored or the suite has none, and 2 when the run could not be made.

use std::process::ExitCode;

use ess_conformance::Status;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [suite, report] = arguments.as_slice() else {
        eprintln!("usage: swell-conformance <suite.json> <report.json>");
        return ExitCode::from(2);
    };
    let text = match std::fs::read_to_string(suite) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("{suite}: {error}");
            return ExitCode::from(2);
        }
    };
    let conformance = match swell_conformance::run(&text, swell_conformance::shipped) {
        Ok(conformance) => conformance,
        Err(error) => {
            eprintln!("{suite}: {error}");
            return ExitCode::from(2);
        }
    };
    let written = conformance
        .report
        .to_canonical_json()
        .map_err(|error| error.to_string())
        .and_then(|json| std::fs::write(report, json).map_err(|error| error.to_string()));
    if let Err(error) = written {
        eprintln!("{report}: {error}");
        return ExitCode::from(2);
    }
    for scenario in &conformance.run.scenarios {
        for check in &scenario.checks {
            if check.status == Status::Passed {
                continue;
            }
            if let Some(diagnostic) = &check.diagnostic {
                println!("{check}\n{diagnostic}\n");
            }
        }
    }
    print!("{}", conformance.run.report());
    let counts = conformance.report.counts();
    if counts.total > 0 && counts.failed == 0 && counts.error == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

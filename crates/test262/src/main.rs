#![forbid(unsafe_code)]
//! `swell-test262`, the conformance runner of swell (design, decision 11).
//!
//! - `swell-test262 meta --test262 <checkout>`: one line per case, as `meta.awk` of the design
//!   prints it.
//! - `swell-test262 count --test262 <checkout> --design <file> --features <file>
//!   [--sections <file>] [--expected <file>] [--workers <n>] [--run all|none]`: runs every run
//!   case on the engine on `<n>` worker threads (default 1), then prints the directory lines,
//!   the total line, the slice lines, `test262-errors <n>`, then with `--sections` one
//!   `test262-section` line per section. The lines on standard output do not depend on `<n>`.
//!   After them, on standard error: `test262-budget <path> <strict|non-strict> <reason>` for
//!   each run a budget ended, `test262-panic <path> <mode> <message>` for each run that
//!   panicked, `test262-refused <runs>` (the runs a refusal of the parser ended), the twenty
//!   runs with the most steps as `test262-steps <steps> <path> <mode>`, and one
//!   `test262-time <slice> <runs> <milliseconds>` per slice. With `--run none` no case runs:
//!   every run case is counted failed and none of the standard error lines of runs is printed,
//!   so that a check of the counting alone (the planted scripts of `tests/gate/`) does not pay
//!   for the runs.
//!
//! Both name a case whose front matter cannot be read on standard error as
//! `test262-error <path>: <reason>`. `count` then prints its problems and exits 1 when there is
//! one: a feature `features.txt` does not list, a difference between `features.txt` and the
//! features the checkout declares, an excluded feature listed as `run`, a line of the
//! `swell-slices` block it cannot read, a line of `sections.txt` it cannot take or a path of it
//! that matches no case. With `--expected` it compares every line it printed with the file in
//! order: each position that differs goes to standard error as `-<expected line>` and
//! `+<printed line>`, then `test262: <n> line(s) differ from <file>`, and it exits 1; when all
//! are equal it prints `test262: <n> lines equal <file>` to standard error. A command it cannot
//! carry out (a wrong argument, a worker count that is not a positive integer, a `--run` other
//! than `all` or `none`, a file or directory it cannot read, a checkout with no case, a worker
//! thread that cannot start) exits 2.

use std::io::{self, BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use swell_test262::count::{self, Loaded};
use swell_test262::{expected, features, run, sections, slices};

const USAGE: &str = "usage: swell-test262 meta --test262 <checkout>\n       \
swell-test262 count --test262 <checkout> --design <file> --features <file> \
[--sections <file>] [--expected <file>] [--workers <n>] [--run all|none]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(message) => {
            eprintln_lossy(&format!("swell-test262: {message}"));
            ExitCode::from(2)
        }
    }
}

/// Writes a line to standard error; a write that fails is dropped, since there is nowhere left
/// to report it.
fn eprintln_lossy(line: &str) {
    let _ = writeln!(io::stderr().lock(), "{line}");
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let Some((command, rest)) = args.split_first() else {
        return Err(USAGE.to_string());
    };
    let options = options(rest)?;
    let get = |name: &str| -> Result<PathBuf, String> {
        options
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| PathBuf::from(v))
            .ok_or_else(|| format!("{command} needs {name}\n{USAGE}"))
    };
    let allowed: &[&str] = match command.as_str() {
        "meta" => &["--test262"],
        "count" => &[
            "--test262",
            "--design",
            "--features",
            "--sections",
            "--expected",
            "--workers",
            "--run",
        ],
        _ => return Err(format!("unknown command {command}\n{USAGE}")),
    };
    if let Some((name, _)) = options.iter().find(|(n, _)| !allowed.contains(&n.as_str())) {
        return Err(format!("{command} takes no {name}\n{USAGE}"));
    }
    let workers = match options.iter().find(|(n, _)| n == "--workers") {
        Some((_, value)) => value
            .parse::<NonZeroUsize>()
            .map_err(|_| format!("--workers needs a positive integer, not {value}\n{USAGE}"))?,
        None => NonZeroUsize::MIN,
    };
    let run_cases = match options.iter().find(|(n, _)| n == "--run") {
        None => true,
        Some((_, value)) => match value.as_str() {
            "all" => true,
            "none" => false,
            _ => return Err(format!("--run needs all or none, not {value}\n{USAGE}")),
        },
    };
    let checkout = get("--test262")?;
    let mut loaded = load(&checkout)?;
    if command == "meta" {
        return meta(&loaded);
    }
    let inputs = Inputs {
        design: get("--design")?,
        features: get("--features")?,
        sections: get("--sections").ok(),
        expected: get("--expected").ok(),
        workers,
        run_cases,
    };
    count(&checkout, &mut loaded, &inputs)
}

/// The files `count` reads besides the checkout, and its worker count.
struct Inputs {
    design: PathBuf,
    features: PathBuf,
    sections: Option<PathBuf>,
    expected: Option<PathBuf>,
    workers: NonZeroUsize,
    /// Whether the run cases are run (`--run all`, the default) or counted failed unrun
    /// (`--run none`).
    run_cases: bool,
}

/// The `--name value` pairs of the arguments.
fn options(args: &[String]) -> Result<Vec<(String, String)>, String> {
    let mut pairs = Vec::new();
    let mut rest = args.iter();
    while let Some(name) = rest.next() {
        if !name.starts_with("--") {
            return Err(format!("unexpected argument {name}\n{USAGE}"));
        }
        let Some(value) = rest.next() else {
            return Err(format!("{name} needs a value\n{USAGE}"));
        };
        if pairs.iter().any(|(n, _): &(String, String)| n == name) {
            return Err(format!("{name} given twice\n{USAGE}"));
        }
        pairs.push((name.clone(), value.clone()));
    }
    Ok(pairs)
}

fn load(checkout: &Path) -> Result<Vec<Loaded>, String> {
    let loaded = count::load(checkout)
        .map_err(|e| format!("{}: cannot read its cases: {e}", checkout.display()))?;
    if loaded.is_empty() {
        return Err(format!("{}: no case under test/", checkout.display()));
    }
    for item in &loaded {
        if let Some(error) = &item.error {
            eprintln_lossy(&format!("test262-error {}: {error}", item.case.path));
        }
    }
    Ok(loaded)
}

fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: cannot be read: {e}", path.display()))
}

fn print(lines: impl IntoIterator<Item = String>) -> Result<(), String> {
    let mut out = BufWriter::new(io::stdout().lock());
    for line in lines {
        writeln!(out, "{line}").map_err(|e| format!("standard output: {e}"))?;
    }
    out.flush().map_err(|e| format!("standard output: {e}"))
}

fn meta(loaded: &[Loaded]) -> Result<ExitCode, String> {
    print(loaded.iter().map(|l| l.case.meta.tsv_line(&l.case.path)))?;
    Ok(ExitCode::SUCCESS)
}

fn count(checkout: &Path, loaded: &mut [Loaded], inputs: &Inputs) -> Result<ExitCode, String> {
    let (rules, mut problems) = slices::read(&read_text(&inputs.design)?);
    let (listed, listed_problems) = features::read_listed(&read_text(&inputs.features)?);
    problems.extend(listed_problems);
    let expected_text = match &inputs.expected {
        Some(file) => Some((file, read_text(file)?)),
        None => None,
    };
    let declared = features::declared(&read_text(&checkout.join("features.txt"))?);
    if declared.is_empty() {
        problems.push(format!(
            "{}: declares no feature",
            checkout.join("features.txt").display()
        ));
    }
    problems.extend(features::check(&listed, &declared, &rules));
    let run_lines = if inputs.run_cases {
        count::run_cases(
            checkout,
            loaded,
            &rules,
            &listed,
            inputs.workers,
            &run::LIMITS,
        )?
    } else {
        Vec::new()
    };
    let report = count::tally(loaded, &rules, &listed);
    for (name, cases) in &report.unlisted {
        problems.push(format!("test262-unlisted-feature {name} {cases}"));
    }
    let mut lines = report.lines();
    if let Some(file) = &inputs.sections {
        let (listed_sections, read_problems) = sections::read(&read_text(file)?);
        problems.extend(read_problems);
        let (counts, count_problems) = listed_sections.count(loaded, &rules, &listed);
        problems.extend(count_problems);
        lines.extend(counts.iter().map(|c| c.line()));
    }
    let mut failed = !problems.is_empty();
    lines.extend(problems);
    print(lines.iter().cloned())?;
    for line in &run_lines {
        eprintln_lossy(line);
    }
    if let Some((file, text)) = expected_text {
        let comparison = expected::compare(&text, &lines);
        for line in &comparison.report {
            eprintln_lossy(line);
        }
        if comparison.equal() {
            eprintln_lossy(&format!(
                "test262: {} lines equal {}",
                comparison.printed,
                file.display()
            ));
        } else {
            eprintln_lossy(&format!(
                "test262: {} line(s) differ from {}",
                comparison.differing,
                file.display()
            ));
            failed = true;
        }
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

//! The cases of a test262 checkout and their counts: per directory, in total and per slice
//! (design, decision 11), with the exclusions of decision 1.
//!
//! A case is a file `test/**/*.js` that is not `*_FIXTURE.js` (INTERPRETING.md, "Modules"). It is
//! excluded when it names a feature `features.txt` excludes, belongs to a slice an
//! `exclude-slice` line names or includes a file an `exclude-include` line names; otherwise it is
//! not applicable when it has the flag `CanBlockIsTrue` (the [[CanBlock]] of the agent is false);
//! otherwise it is run. A case whose front matter cannot be read is run (and fails) whatever it
//! names. A run case has one run with `onlyStrict`, `noStrict`, `module` or `raw`, else two
//! (INTERPRETING.md, "Strict Mode"), and passes when every run passes, and is counted as ended by
//! a budget when a run is. [`run_cases`] runs every run case on the engine through [`run`] and
//! keeps each case's outcome in its [`Loaded`]; [`tally`] counts what was kept, and counts a run
//! case that was not run as failed. Nothing else here knows the engine.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::features::Listed;
use crate::front_matter::{self, FrontMatter};
use crate::run;
use crate::slices::{Rules, Slice};

/// One run of a case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Run {
    /// A script in non-strict code (the first of two runs, or the one run of `noStrict`).
    NonStrict,
    /// A script with `"use strict";` in front (the second of two runs, or the one of
    /// `onlyStrict`).
    Strict,
    /// A module (`module`).
    Module,
    /// The source as it is, with no harness file (`raw`).
    Raw,
}

/// The outcome of one run, and of a case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// It passed.
    Passed,
    /// It failed.
    Failed,
    /// It ran out of its step or memory budget (decision 11).
    Budget,
}

/// A case: its path from the checkout (`test/...`) and its front matter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    /// The path from the checkout, `/`-separated, starting `test/`.
    pub path: String,
    /// The front matter, as far as it was read.
    pub meta: FrontMatter,
}

/// The outcome of one run of a case of the checkout at `checkout`, run on the engine as decision
/// 11 states ([`run`]): a fresh agent and realm, `$262` and `print`, `assert.js` and `sta.js`
/// (not for `raw`), the `includes:` in order, then the case, judged by its `negative`.
/// `doneprintHandle.js` and the `async` judgement come with slice 10.
pub fn run_outcome(checkout: &Path, case: &Case, run: Run) -> Outcome {
    match run::run_all(checkout, &[(case, run)], NonZeroUsize::MIN, &run::LIMITS) {
        Ok(ran) => ran.first().map_or(Outcome::Failed, |ran| ran.outcome),
        Err(_) => Outcome::Failed,
    }
}

/// The runs a case has.
pub fn runs(meta: &FrontMatter) -> Vec<Run> {
    if meta.has_flag("module") {
        vec![Run::Module]
    } else if meta.has_flag("raw") {
        vec![Run::Raw]
    } else if meta.has_flag("onlyStrict") {
        vec![Run::Strict]
    } else if meta.has_flag("noStrict") {
        vec![Run::NonStrict]
    } else {
        vec![Run::NonStrict, Run::Strict]
    }
}

/// The directory a case is counted in (decision 11): its path below `test/` cut to two parts,
/// three under `language/expressions` and `language/statements`; a case whose directory is
/// shallower keeps its own directory.
pub fn directory(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    let below: &[&str] = parts
        .get(1..parts.len().saturating_sub(1))
        .unwrap_or_default();
    let deep = matches!(below, ["language", "expressions" | "statements", ..]);
    let keep = if deep { 3 } else { 2 };
    below
        .get(..keep.min(below.len()))
        .unwrap_or_default()
        .join("/")
}

/// A case as read from the checkout, with the reason its front matter cannot be read.
#[derive(Clone, Debug)]
pub struct Loaded {
    /// The case.
    pub case: Case,
    /// Why its front matter cannot be read, if it cannot.
    pub error: Option<String>,
    /// Its outcome, once [`run_cases`] ran it.
    pub outcome: Option<Outcome>,
}

/// The paths of every case under `<checkout>/test`, in byte order, each with its file. Only
/// regular files count, and a symbolic link is not followed, as `find -type f` does.
pub fn case_paths(checkout: &Path) -> io::Result<Vec<(String, PathBuf)>> {
    let mut found = Vec::new();
    let mut pending = vec![(String::from("test"), checkout.join("test"))];
    while let Some((path, dir)) = pending.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let kind = entry.file_type()?;
            let child = format!("{path}/{name}");
            if kind.is_dir() {
                pending.push((child, entry.path()));
            } else if kind.is_file() && name.ends_with(".js") && !name.ends_with("_FIXTURE.js") {
                found.push((child, entry.path()));
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Reads the front matter of every case under `<checkout>/test`. A file that cannot be read is
/// a case whose front matter cannot be read; a directory that cannot be read fails the whole.
pub fn load(checkout: &Path) -> io::Result<Vec<Loaded>> {
    let paths = case_paths(checkout)?;
    let mut loaded = Vec::with_capacity(paths.len());
    for (path, file) in paths {
        let read = File::open(&file).and_then(|f| front_matter::read(&mut BufReader::new(f)));
        let (meta, error) = match read {
            Ok(read) => (read.meta, read.problem.map(|p| p.to_string())),
            Err(e) => (FrontMatter::default(), Some(format!("cannot be read: {e}"))),
        };
        loaded.push(Loaded {
            case: Case { path, meta },
            error,
            outcome: None,
        });
    }
    Ok(loaded)
}

/// The counts of one directory, of the total or of one slice. Every field counts cases, except
/// `runs`, the runs the cases would have, excluded or not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// Cases that exist.
    pub cases: u64,
    /// Runs the cases would have.
    pub runs: u64,
    /// Cases excluded.
    pub excluded: u64,
    /// Cases not applicable.
    pub not_applicable: u64,
    /// Cases run.
    pub run: u64,
    /// Run cases that passed.
    pub passed: u64,
    /// Run cases that failed.
    pub failed: u64,
    /// Run cases that ran out of a budget.
    pub budget: u64,
}

impl Tally {
    fn columns(&self) -> String {
        format!(
            "{} {} {} {} {} {} {}",
            self.cases,
            self.excluded,
            self.not_applicable,
            self.run,
            self.passed,
            self.failed,
            self.budget
        )
    }
}

/// How a case is counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Excluded,
    NotApplicable,
    Run(Outcome),
}

/// The counts of a whole checkout.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Per directory.
    pub directories: BTreeMap<String, Tally>,
    /// Per slice, by the slice as printed.
    pub slices: BTreeMap<String, Tally>,
    /// Over every case.
    pub total: Tally,
    /// Cases whose front matter cannot be read.
    pub errors: u64,
    /// Features `features.txt` does not list, with the cases naming each.
    pub unlisted: BTreeMap<String, u64>,
}

/// What becomes of a case before it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Plan {
    Excluded,
    NotApplicable,
    /// Its front matter cannot be read: run, and failed without running.
    Unreadable,
    Run,
}

/// The slice of a case and what becomes of it.
fn plan(loaded: &Loaded, rules: &Rules, listed: &Listed) -> (Slice, Plan) {
    let meta = &loaded.case.meta;
    let slice = rules.slice_of(&loaded.case.path, meta.feature_list(), meta.flag_list());
    let excluded = meta.feature_list().any(|name| listed.excludes(name))
        || rules.excludes_slice(&slice).is_some()
        || rules.excludes_include(meta.include_list());
    let plan = if loaded.error.is_some() {
        Plan::Unreadable
    } else if excluded {
        Plan::Excluded
    } else if meta.has_flag("CanBlockIsTrue") {
        Plan::NotApplicable
    } else {
        Plan::Run
    };
    (slice, plan)
}

/// The lines [`run_cases`] answers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunLines {
    /// The lines for standard error.
    pub log: Vec<String>,
    /// One line per run, in the order of the jobs: `test262-run`, the slice, the path, the mode
    /// (`strict`, `non-strict` or `module`), the outcome (`Passed`, `Failed` or `Budget`) and
    /// the detail of the run ([`run::Ran::detail`]), separated by tabs; a tab or a line end in a
    /// field is written as a space, so that a line is one line of six fields.
    pub runs: Vec<String>,
}

/// `text` as one field of a run line: each tab and line end (LF, CR, U+2028, U+2029) a space.
pub fn one_field(text: &str) -> String {
    text.replace(['\t', '\n', '\r', '\u{2028}', '\u{2029}'], " ")
}

/// The name of an outcome as a run line has it.
fn outcome_name(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Passed => "Passed",
        Outcome::Failed => "Failed",
        Outcome::Budget => "Budget",
    }
}

/// One run the count makes: the position of its case in the list, the case, the run, and the
/// slice of the case as printed.
#[derive(Clone, Debug)]
pub struct Job<'c> {
    /// The position of the case in the list, from 0.
    pub index: usize,
    /// The case.
    pub case: &'c Case,
    /// The run.
    pub run: Run,
    /// The slice of the case, as printed.
    pub slice: String,
}

/// The runs the count makes, in its order (the cases in the order of `loaded`, the runs of a
/// case in the order of [`runs`]): every run of every case that is run. Beside them, every slice
/// of the cases of `loaded`, run or not, as printed.
pub fn jobs<'c>(
    loaded: &'c [Loaded],
    rules: &Rules,
    listed: &Listed,
) -> (Vec<Job<'c>>, BTreeSet<String>) {
    let mut slices = BTreeSet::new();
    let mut jobs = Vec::new();
    for (index, item) in loaded.iter().enumerate() {
        let (slice, plan) = plan(item, rules, listed);
        let slice = slice.to_string();
        if plan == Plan::Run {
            for run in runs(&item.case.meta) {
                jobs.push(Job {
                    index,
                    case: &item.case,
                    run,
                    slice: slice.clone(),
                });
            }
        }
        slices.insert(slice);
    }
    (jobs, slices)
}

/// What one run gave, as the count reads it: from a run on the engine ([`Record::of`]) or from
/// the file of a part (`parts`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// Passed, failed, or ended by a budget.
    pub outcome: Outcome,
    /// What ended it ([`run::Ran::detail`]), as the run gave it.
    pub detail: String,
    /// The steps every script of the run spent.
    pub steps: u64,
    /// The time it took.
    pub elapsed: Duration,
    /// It counts in `test262-refused` ([`run::Ran::refused`]).
    pub refused: bool,
    /// The budget that ended it, as `test262-budget` prints it, when its outcome is `Budget`.
    pub budget: Option<String>,
    /// It panicked.
    pub panicked: bool,
}

impl Record {
    /// The record of a run on the engine.
    pub fn of(ran: &run::Ran) -> Record {
        let budget = match (ran.outcome, ran.budget) {
            (Outcome::Budget, Some(reason)) => Some(format!("{reason:?}")),
            _ => None,
        };
        Record {
            outcome: ran.outcome,
            detail: ran.detail.clone(),
            steps: ran.steps,
            elapsed: ran.elapsed,
            refused: ran.refused(),
            budget,
            panicked: ran.panicked,
        }
    }
}

/// The run line of `job` with `record` ([`RunLines::runs`]).
pub fn run_line(job: &Job<'_>, record: &Record) -> String {
    format!(
        "test262-run\t{}\t{}\t{}\t{}\t{}",
        one_field(&job.slice),
        one_field(&job.case.path),
        run::mode(job.run),
        outcome_name(record.outcome),
        one_field(&record.detail)
    )
}

/// Runs every run of every case that is run, on `workers` threads with `limits` (the runner gives
/// [`run::LIMITS`]), and keeps each case's outcome in its [`Loaded`]. Answers the lines for
/// standard error: one `test262-budget <path> <strict|non-strict> <reason>` per run a budget
/// ended and one `test262-panic <path> <mode> <message>` per run that panicked, in case order;
/// then `test262-refused <runs>`, the runs a refusal of the parser ended, all failed ([`run`]);
/// then the twenty runs with the most steps, `test262-steps <steps> <path> <mode>`, the most
/// first; then one `test262-time <slice> <runs> <milliseconds>` per slice, the time its runs
/// took summed. Beside them, one run line per run ([`RunLines::runs`]).
pub fn run_cases(
    checkout: &Path,
    loaded: &mut [Loaded],
    rules: &Rules,
    listed: &Listed,
    workers: NonZeroUsize,
    limits: &swell::Limits,
) -> Result<RunLines, String> {
    let (jobs, slices) = jobs(loaded, rules, listed);
    let pairs: Vec<(&Case, Run)> = jobs.iter().map(|job| (job.case, job.run)).collect();
    let ran = run::run_all(checkout, &pairs, workers, limits)?;
    let records: Vec<Record> = ran.iter().map(Record::of).collect();
    let (lines, outcomes) = lines_of(&jobs, &records, &slices);
    keep_outcomes(loaded, &outcomes);
    Ok(lines)
}

/// The lines [`run_cases`] answers for `jobs`, each with its record in `records` (in the same
/// order), and `slices`, every slice of the cases; beside them the outcome of each case that
/// was run, by its position in the list.
pub fn lines_of(
    jobs: &[Job<'_>],
    records: &[Record],
    slices: &BTreeSet<String>,
) -> (RunLines, BTreeMap<usize, Outcome>) {
    let mut times: BTreeMap<&str, (u64, Duration)> = slices
        .iter()
        .map(|slice| (slice.as_str(), (0, Duration::ZERO)))
        .collect();
    let mut lines = Vec::new();
    let mut run_lines = Vec::with_capacity(records.len());
    let mut outcomes: BTreeMap<usize, Vec<Outcome>> = BTreeMap::new();
    let mut steps = Vec::with_capacity(records.len());
    let mut refused = 0u64;
    for (job, record) in jobs.iter().zip(records) {
        let mode = run::mode(job.run);
        let path = job.case.path.as_str();
        run_lines.push(run_line(job, record));
        if let (Outcome::Budget, Some(reason)) = (record.outcome, &record.budget) {
            lines.push(format!("test262-budget {path} {mode} {reason}"));
        }
        if record.panicked {
            lines.push(format!("test262-panic {path} {mode} {}", record.detail));
        }
        if record.refused {
            refused += 1;
        }
        outcomes.entry(job.index).or_default().push(record.outcome);
        steps.push((Reverse(record.steps), path, mode));
        if let Some((runs, time)) = times.get_mut(job.slice.as_str()) {
            *runs += 1;
            *time = time.saturating_add(record.elapsed);
        }
    }
    lines.push(format!("test262-refused {refused}"));
    steps.sort();
    for (Reverse(spent), path, mode) in steps.into_iter().take(20) {
        lines.push(format!("test262-steps {spent} {path} {mode}"));
    }
    for (slice, (runs, time)) in &times {
        lines.push(format!("test262-time {slice} {runs} {}", time.as_millis()));
    }
    let outcomes = outcomes
        .into_iter()
        .map(|(index, outcomes)| (index, case_outcome(&outcomes)))
        .collect();
    (
        RunLines {
            log: lines,
            runs: run_lines,
        },
        outcomes,
    )
}

/// Keeps each outcome of `outcomes` in the [`Loaded`] at its position.
pub fn keep_outcomes(loaded: &mut [Loaded], outcomes: &BTreeMap<usize, Outcome>) {
    for (&index, &outcome) in outcomes {
        if let Some(item) = loaded.get_mut(index) {
            item.outcome = Some(outcome);
        }
    }
}

/// Counts the cases, with the outcomes [`run_cases`] kept.
pub fn tally(cases: &[Loaded], rules: &Rules, listed: &Listed) -> Report {
    let mut report = Report::default();
    for loaded in cases {
        let Loaded { case, error, .. } = loaded;
        let meta = &case.meta;
        let names: BTreeSet<&str> = meta.feature_list().collect();
        for name in &names {
            if !listed.lists(name) {
                *report.unlisted.entry((*name).to_string()).or_default() += 1;
            }
        }
        let runs = runs(meta);
        let (slice, plan) = plan(loaded, rules, listed);
        let status = match plan {
            Plan::Excluded => Status::Excluded,
            Plan::NotApplicable => Status::NotApplicable,
            Plan::Unreadable => Status::Run(Outcome::Failed),
            Plan::Run => Status::Run(loaded.outcome.unwrap_or(Outcome::Failed)),
        };
        if error.is_some() {
            report.errors += 1;
        }
        let run_count = u64::try_from(runs.len()).unwrap_or(u64::MAX);
        for tally in [
            report.directories.entry(directory(&case.path)).or_default(),
            report.slices.entry(slice.to_string()).or_default(),
        ] {
            add(tally, status, run_count);
        }
        add(&mut report.total, status, run_count);
    }
    report
}

/// The outcome of a case from those of its runs: ended by a budget when one run was, passed when
/// every run passed, else failed.
fn case_outcome(outcomes: &[Outcome]) -> Outcome {
    if outcomes.contains(&Outcome::Budget) {
        Outcome::Budget
    } else if outcomes.iter().all(|&o| o == Outcome::Passed) {
        Outcome::Passed
    } else {
        Outcome::Failed
    }
}

fn add(tally: &mut Tally, status: Status, runs: u64) {
    tally.cases += 1;
    tally.runs += runs;
    match status {
        Status::Excluded => tally.excluded += 1,
        Status::NotApplicable => tally.not_applicable += 1,
        Status::Run(outcome) => {
            tally.run += 1;
            match outcome {
                Outcome::Passed => tally.passed += 1,
                Outcome::Failed => tally.failed += 1,
                Outcome::Budget => tally.budget += 1,
            }
        }
    }
}

impl Report {
    /// The lines the runner prints: directory lines in byte order, the total line, slice lines
    /// in byte order, then `test262-errors <n>`.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for (directory, tally) in &self.directories {
            lines.push(format!("test262 {directory} {}", tally.columns()));
        }
        lines.push(format!("test262 total {}", self.total.columns()));
        for (slice, tally) in &self.slices {
            lines.push(format!(
                "test262-slice {slice} {} {} {} {} {} {} {} {}",
                tally.cases,
                tally.runs,
                tally.excluded,
                tally.not_applicable,
                tally.run,
                tally.passed,
                tally.failed,
                tally.budget
            ));
        }
        lines.push(format!("test262-errors {}", self.errors));
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::one_field;

    #[test]
    fn a_tab_or_line_end_in_a_field_is_a_space() {
        assert_eq!(one_field("a\tb\nc\r\nd\u{2028}e\u{2029}f"), "a b c  d e f");
        assert_eq!(one_field("threw Error: x"), "threw Error: x");
    }
}

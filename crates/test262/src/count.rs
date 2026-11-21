//! The cases of a test262 checkout and their counts: per directory, in total and per slice
//! (design, decision 11), with the exclusions of decision 1.
//!
//! A case is a file `test/**/*.js` that is not `*_FIXTURE.js` (INTERPRETING.md, "Modules"). It is
//! excluded when it names a feature `features.txt` excludes, belongs to a slice an
//! `exclude-slice` line names or includes a file an `exclude-include` line names; otherwise it is
//! not applicable when it has the flag `CanBlockIsTrue` (the [[CanBlock]] of the agent is false);
//! otherwise it is run. A case whose front matter cannot be read is run (and fails) whatever it
//! names. A run case has one run with `onlyStrict`, `noStrict`, `module` or `raw`, else two
//! (INTERPRETING.md, "Strict Mode"), and passes when every run passes. [`run_outcome`] gives the
//! outcome of a run; nothing else here knows the engine.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

use crate::features::Listed;
use crate::front_matter::{self, FrontMatter};
use crate::slices::Rules;

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

/// The outcome of one run of a case.
///
/// **This is the one place slice 1 changes.** No engine exists in the foundation, so every run
/// fails. Slice 1 replaces this body with the call decision 11 states: a fresh agent and realm
/// per run, `$262` and `print`, `assert.js` and `sta.js` (not for `raw`), `doneprintHandle.js`
/// for `async`, the `includes:` in order, then the case, judged by its `negative` and `async`.
/// Nothing else of the runner calls or knows the engine.
pub fn run_outcome(_case: &Case, _run: Run) -> Outcome {
    Outcome::Failed
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

/// Counts the cases.
pub fn tally(cases: &[Loaded], rules: &Rules, listed: &Listed) -> Report {
    let mut report = Report::default();
    for Loaded { case, error } in cases {
        let meta = &case.meta;
        let slice = rules.slice_of(&case.path, meta.feature_list(), meta.flag_list());
        let names: BTreeSet<&str> = meta.feature_list().collect();
        for name in &names {
            if !listed.lists(name) {
                *report.unlisted.entry((*name).to_string()).or_default() += 1;
            }
        }
        let runs = runs(meta);
        let excluded = names.iter().any(|name| listed.excludes(name))
            || rules.excludes_slice(&slice).is_some()
            || rules.excludes_include(meta.include_list());
        let status = if error.is_none() && excluded {
            Status::Excluded
        } else if error.is_none() && meta.has_flag("CanBlockIsTrue") {
            Status::NotApplicable
        } else {
            Status::Run(case_outcome(case, &runs))
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

fn case_outcome(case: &Case, runs: &[Run]) -> Outcome {
    let outcomes: Vec<Outcome> = runs.iter().map(|&run| run_outcome(case, run)).collect();
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

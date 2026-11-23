//! The run in parts: `count --part <k>/<n>` runs the cases of part `k` of `n` and writes what
//! the count needs of their runs to a file; `count --from-parts` reads the files of every part
//! and counts from them, running no case, so that it prints what the whole run prints.
//!
//! A case belongs to part `k` of `n` when its position in the list of every case of the checkout
//! (byte order, [`count::case_paths`], from 0) modulo `n` is `k - 1` ([`part_of`]): the choice
//! depends on nothing but the list and `n`, every case is in exactly one part, a directory of
//! expensive cases is spread over the parts, and part `1/1` is the whole run.
//!
//! The file of a part has three kinds of lines, fields separated by tabs:
//!
//! - first, `test262-part <k>/<n> <all|none> <cases>`: the part, the `--run` it was made with
//!   and the number of cases of the whole list;
//! - `test262-case <path>` for each case of the part, excluded and not applicable ones too, in
//!   the order of the list, each followed by
//! - one line per run of the case, in the order of the whole run: the six fields of the run line
//!   the whole run writes ([`count::run_line`]), then `<steps>`, `<nanoseconds>` (the time the
//!   run took), `<refused>` (`1` when the run counts in `test262-refused`, else `0`), `<budget>`
//!   (`StepBudget` or `MemoryBudget` for a run a budget ended, else `-`) and `<panic>` (for a run
//!   that panicked its detail as the run gave it, with `\` written `\\`, a tab `\t`, a line feed
//!   `\n` and a carriage return `\r`; empty for any other run). Cut to its first six fields such
//!   a line is the run line of the whole run.
//!
//! [`from_parts`] takes every run's outcome from those files and refuses, naming the first case,
//! when a file is not such a file, when two files state different `n`, when a part is missing or
//! given twice, when a file was made with another `--run` or on a list of another length, when a
//! case or a run that the whole run would make is in no part's file, in two, or in a part it does
//! not belong to, when a run has another slice than the count gives it, and when a file holds a
//! case or a run the count does not make: a count from parts never guesses an outcome.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::num::NonZeroUsize;
use std::path::Path;
use std::time::Duration;

use crate::count::{self, Job, Loaded, Outcome, Record, RunLines};
use crate::features::Listed;
use crate::run;
use crate::slices::Rules;

/// Part `k` of `n`, `1 <= k <= n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Part {
    k: usize,
    n: NonZeroUsize,
}

impl Part {
    /// Part `k` of `n`, when `1 <= k <= n`.
    pub fn new(k: usize, n: NonZeroUsize) -> Option<Part> {
        (1..=n.get()).contains(&k).then_some(Part { k, n })
    }

    /// Reads `<k>/<n>`, two positive integers without a sign or a leading zero, `k` at most `n`.
    pub fn parse(text: &str) -> Result<Part, String> {
        let bad = || format!("a part is <k>/<n> with 1 <= k <= n, not {text}");
        let (k, n) = text.split_once('/').ok_or_else(bad)?;
        let number = |s: &str| -> Option<usize> {
            let value: usize = s.parse().ok()?;
            (s.bytes().all(|b| b.is_ascii_digit()) && value.to_string() == s).then_some(value)
        };
        let (k, n) = (number(k).ok_or_else(bad)?, number(n).ok_or_else(bad)?);
        NonZeroUsize::new(n)
            .and_then(|n| Part::new(k, n))
            .ok_or_else(bad)
    }

    /// `k`.
    pub fn k(self) -> usize {
        self.k
    }

    /// `n`.
    pub fn n(self) -> NonZeroUsize {
        self.n
    }

    /// Whether the case at `position` of the list is of this part.
    pub fn holds(self, position: usize) -> bool {
        part_of(position, self.n) == self.k
    }
}

impl fmt::Display for Part {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.k, self.n)
    }
}

/// The part (from 1) of `n` that the case at `position` (from 0) of the list belongs to.
pub fn part_of(position: usize, n: NonZeroUsize) -> usize {
    position % n + 1
}

/// The lines of the file of `part`: the cases of the part of `loaded`, each with its runs, run
/// on the engine with `runner`, the worker count and the limits (the runner gives
/// [`run::LIMITS`]), as `--run all` runs them; with no `runner` (`--run none`) no case runs and
/// the file holds no run line.
pub fn run_part(
    checkout: &Path,
    loaded: &[Loaded],
    rules: &Rules,
    listed: &Listed,
    part: Part,
    runner: Option<(NonZeroUsize, &swell::Limits)>,
) -> Result<Vec<String>, String> {
    let (jobs, _) = count::jobs(loaded, rules, listed);
    let mine: Vec<&Job<'_>> = match runner {
        Some(_) => jobs.iter().filter(|job| part.holds(job.index)).collect(),
        None => Vec::new(),
    };
    let ran = match runner {
        Some((workers, limits)) => {
            let pairs: Vec<_> = mine.iter().map(|job| (job.case, job.run)).collect();
            run::run_all(checkout, &pairs, workers, limits)?
        }
        None => Vec::new(),
    };
    let mut lines = vec![format!(
        "test262-part\t{part}\t{}\t{}",
        mode_name(runner.is_some()),
        loaded.len()
    )];
    let mut runs = mine.iter().zip(&ran).peekable();
    for (index, item) in loaded.iter().enumerate() {
        if !part.holds(index) {
            continue;
        }
        lines.push(format!(
            "test262-case\t{}",
            count::one_field(&item.case.path)
        ));
        while let Some((job, ran)) = runs.next_if(|(job, _)| job.index == index) {
            lines.push(part_run_line(job, &Record::of(ran)));
        }
    }
    Ok(lines)
}

/// The `--run` that runs the cases or not.
fn mode_name(run_cases: bool) -> &'static str {
    if run_cases { "all" } else { "none" }
}

/// The line of a run in the file of a part: the run line, then the steps, the nanoseconds, the
/// refusal, the budget and the panic.
fn part_run_line(job: &Job<'_>, record: &Record) -> String {
    let nanos = u64::try_from(record.elapsed.as_nanos()).unwrap_or(u64::MAX);
    let panic = if record.panicked {
        escape(&record.detail)
    } else {
        String::new()
    };
    format!(
        "{}\t{}\t{nanos}\t{}\t{}\t{panic}",
        count::run_line(job, record),
        record.steps,
        u8::from(record.refused),
        record.budget.as_deref().unwrap_or("-"),
    )
}

/// `text` with `\`, tab, line feed and carriage return escaped, so that it is one field.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out
}

/// The text [`escape`] wrote; `None` for an escape it does not write.
pub fn unescape(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        out.push(match chars.next()? {
            '\\' => '\\',
            't' => '\t',
            'n' => '\n',
            'r' => '\r',
            _ => return None,
        });
    }
    Some(out)
}

/// The file of a part: its name, as messages print it, and its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartFile {
    /// The name of the file.
    pub name: String,
    /// Its text.
    pub text: String,
}

/// The files of the parts at `path`: every regular file whose name ends in `.tsv` when it is a
/// directory, in byte order of the names; the file itself when it is a file.
pub fn read(path: &Path) -> Result<Vec<PartFile>, String> {
    let cannot = |p: &Path, e: std::io::Error| format!("{}: cannot be read: {e}", p.display());
    let meta = fs::metadata(path).map_err(|e| cannot(path, e))?;
    let mut paths = Vec::new();
    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| cannot(path, e))? {
            let entry = entry.map_err(|e| cannot(path, e))?;
            let is_file = entry.file_type().map_err(|e| cannot(path, e))?.is_file();
            if is_file && entry.file_name().to_string_lossy().ends_with(".tsv") {
                paths.push(entry.path());
            }
        }
        paths.sort();
    } else {
        paths.push(path.to_path_buf());
    }
    if paths.is_empty() {
        return Err(format!(
            "--from-parts: no part file (*.tsv) in {}",
            path.display()
        ));
    }
    paths
        .into_iter()
        .map(|p| {
            let text = fs::read_to_string(&p).map_err(|e| cannot(&p, e))?;
            Ok(PartFile {
                name: p.display().to_string(),
                text,
            })
        })
        .collect()
}

/// One file of a part, read.
struct Read<'f> {
    name: &'f str,
    part: Part,
    run_cases: bool,
    cases: usize,
    /// The cases it holds, with their line numbers.
    case_lines: Vec<(usize, &'f str)>,
    /// The runs it holds, with their line numbers.
    run_lines: Vec<(usize, RunRead<'f>)>,
}

/// One run line of a part's file, read.
struct RunRead<'f> {
    slice: &'f str,
    path: &'f str,
    mode: &'f str,
    record: Record,
}

fn read_file(file: &PartFile) -> Result<Read<'_>, String> {
    let name = file.name.as_str();
    let mut lines = file.text.lines().enumerate().map(|(i, l)| (i + 1, l));
    let header_error = || {
        format!(
            "--from-parts: {name}:1: not the first line of a part's file, \
             test262-part<TAB><k>/<n><TAB><all|none><TAB><cases>"
        )
    };
    let (_, header) = lines.next().ok_or_else(header_error)?;
    let fields: Vec<&str> = header.split('\t').collect();
    let [kind, part, mode, cases] = fields.as_slice() else {
        return Err(header_error());
    };
    if *kind != "test262-part" {
        return Err(header_error());
    }
    let part = Part::parse(part).map_err(|e| format!("--from-parts: {name}:1: {e}"))?;
    let run_cases = match *mode {
        "all" => true,
        "none" => false,
        _ => return Err(header_error()),
    };
    let cases: usize = cases.parse().map_err(|_| header_error())?;
    let mut read = Read {
        name,
        part,
        run_cases,
        cases,
        case_lines: Vec::new(),
        run_lines: Vec::new(),
    };
    for (number, line) in lines {
        let at = |what: &str| format!("--from-parts: {name}:{number}: {what}");
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.as_slice() {
            ["test262-case", path] => read.case_lines.push((number, *path)),
            [
                "test262-run",
                slice,
                path,
                mode,
                outcome,
                detail,
                steps,
                nanos,
                refused,
                budget,
                panic,
            ] => {
                let outcome = match *outcome {
                    "Passed" => Outcome::Passed,
                    "Failed" => Outcome::Failed,
                    "Budget" => Outcome::Budget,
                    other => {
                        return Err(at(&format!(
                            "outcome {other} is not Passed, Failed or Budget"
                        )));
                    }
                };
                let steps: u64 = steps
                    .parse()
                    .map_err(|_| at(&format!("steps {steps} is not a number")))?;
                let nanos: u64 = nanos
                    .parse()
                    .map_err(|_| at(&format!("nanoseconds {nanos} is not a number")))?;
                let refused = match *refused {
                    "0" => false,
                    "1" => true,
                    other => return Err(at(&format!("refused {other} is not 0 or 1"))),
                };
                let budget = match (*budget, outcome) {
                    ("-", _) => None,
                    ("StepBudget" | "MemoryBudget", Outcome::Budget) => Some((*budget).to_string()),
                    (other, _) => {
                        return Err(at(&format!(
                            "budget {other} is not - or, for a run ended by a budget, \
                             StepBudget or MemoryBudget"
                        )));
                    }
                };
                let (panicked, detail) = if panic.is_empty() {
                    (false, (*detail).to_string())
                } else {
                    let raw = unescape(panic)
                        .ok_or_else(|| at(&format!("panic {panic} has an unknown escape")))?;
                    (true, raw)
                };
                read.run_lines.push((
                    number,
                    RunRead {
                        slice,
                        path,
                        mode,
                        record: Record {
                            outcome,
                            detail,
                            steps,
                            elapsed: Duration::from_nanos(nanos),
                            refused,
                            budget,
                            panicked,
                        },
                    },
                ));
            }
            _ => {
                return Err(at(
                    "neither test262-case<TAB><path> nor a run line of eleven fields",
                ));
            }
        }
    }
    Ok(read)
}

/// The run lines of the count of `loaded`, taken from the files of every part and run for none,
/// with each case's outcome kept in its [`Loaded`] as [`count::run_cases`] keeps it. `run_cases`
/// is the `--run` of the count, which every part must have been made with; with `--run none`
/// the parts hold no run and the answer has no line, as a count with `--run none` has none.
pub fn from_parts(
    loaded: &mut [Loaded],
    rules: &Rules,
    listed: &Listed,
    files: &[PartFile],
    run_cases: bool,
) -> Result<RunLines, String> {
    let read: Vec<Read<'_>> = files.iter().map(read_file).collect::<Result<_, _>>()?;
    let n = check_parts(&read)?;
    for file in &read {
        if file.run_cases != run_cases {
            return Err(format!(
                "--from-parts: {} was made with --run {}, the count has --run {}",
                file.name,
                mode_name(file.run_cases),
                mode_name(run_cases),
            ));
        }
        if file.cases != loaded.len() {
            return Err(format!(
                "--from-parts: {} was made on a list of {} case(s), the checkout has {}",
                file.name,
                file.cases,
                loaded.len()
            ));
        }
    }
    check_cases(loaded, &read, n)?;
    let (jobs, slices) = count::jobs(loaded, rules, listed);
    let records = take_runs(loaded, &jobs, &read, n, run_cases)?;
    if !run_cases {
        return Ok(RunLines::default());
    }
    let (lines, outcomes) = count::lines_of(&jobs, &records, &slices);
    count::keep_outcomes(loaded, &outcomes);
    Ok(lines)
}

/// The `n` every file states, when every part of `n` is given by exactly one file.
fn check_parts(read: &[Read<'_>]) -> Result<NonZeroUsize, String> {
    let Some(first) = read.first() else {
        return Err("--from-parts: no part file".to_string());
    };
    let n = first.part.n();
    let mut given: BTreeMap<usize, &str> = BTreeMap::new();
    for file in read {
        if file.part.n() != n {
            return Err(format!(
                "--from-parts: {} states part {}, {} states part {}: the parts of one count \
                 state one n",
                file.name, file.part, first.name, first.part
            ));
        }
        if let Some(other) = given.insert(file.part.k(), file.name) {
            return Err(format!(
                "--from-parts: part {} is in two files: {other} and {}",
                file.part, file.name
            ));
        }
    }
    if let Some(k) = (1..=n.get()).find(|k| !given.contains_key(k)) {
        let stated: Vec<String> = given.keys().map(|k| format!("{k}/{n}")).collect();
        return Err(format!(
            "--from-parts: part {k}/{n} is missing; the files state {}",
            stated.join(", ")
        ));
    }
    Ok(n)
}

/// Where a part's file holds a case or a run: the file and the line number.
type Held<'r, 'f, T> = Vec<(&'r Read<'f>, usize, T)>;

/// The first of `held`, in the order of the files and their lines.
fn first_of<'h, 'r, 'f, T>(
    held: impl Iterator<Item = &'h Held<'r, 'f, T>>,
) -> Option<&'h (&'r Read<'f>, usize, T)>
where
    'r: 'h,
    'f: 'r,
    T: 'h,
{
    held.flatten()
        .min_by_key(|(file, number, _)| (file.name, *number))
}

/// Every case of the list is in the file of its part and in no other, and no file holds a case
/// the list does not.
fn check_cases(loaded: &[Loaded], read: &[Read<'_>], n: NonZeroUsize) -> Result<(), String> {
    let mut held: BTreeMap<&str, Held<'_, '_, &str>> = BTreeMap::new();
    for file in read {
        for &(number, path) in &file.case_lines {
            held.entry(path).or_default().push((file, number, path));
        }
    }
    for (index, item) in loaded.iter().enumerate() {
        let path = count::one_field(&item.case.path);
        let belongs = part_of(index, n);
        match held.remove(path.as_str()).unwrap_or_default().as_slice() {
            [] => {
                return Err(format!(
                    "--from-parts: no part holds the case {path} (of part {belongs}/{n})"
                ));
            }
            [(file, _, _)] if file.part.k() != belongs => {
                return Err(format!(
                    "--from-parts: the case {path} is in {} (part {}), it belongs to part \
                     {belongs}/{n}",
                    file.name, file.part
                ));
            }
            [_] => {}
            [(a, la, _), (b, lb, _), ..] => {
                return Err(format!(
                    "--from-parts: the case {path} is in two parts: {}:{la} and {}:{lb}",
                    a.name, b.name
                ));
            }
        }
    }
    if let Some((file, number, path)) = first_of(held.values()) {
        return Err(format!(
            "--from-parts: {}:{number}: the case {path} is not a case of the checkout",
            file.name
        ));
    }
    Ok(())
}

/// The record of every job, in the order of `jobs`, from the files: each run the count makes is
/// in the file of its case's part and in no other, with the slice the count gives it, and no
/// file holds a run the count does not make. With `--run none` (`run_cases` false) the count
/// makes no run, and a file may hold none.
fn take_runs(
    loaded: &[Loaded],
    jobs: &[Job<'_>],
    read: &[Read<'_>],
    n: NonZeroUsize,
    run_cases: bool,
) -> Result<Vec<Record>, String> {
    // By path, then by mode.
    let mut held: BTreeMap<&str, BTreeMap<&str, Held<'_, '_, &RunRead<'_>>>> = BTreeMap::new();
    for file in read {
        for (number, run) in &file.run_lines {
            held.entry(run.path)
                .or_default()
                .entry(run.mode)
                .or_default()
                .push((file, *number, run));
        }
    }
    let mut records = Vec::with_capacity(jobs.len());
    for job in jobs.iter().filter(|_| run_cases) {
        let path = count::one_field(&job.case.path);
        let mode = run::mode(job.run);
        let belongs = part_of(job.index, n);
        match held
            .get_mut(path.as_str())
            .and_then(|modes| modes.remove(mode))
            .unwrap_or_default()
            .as_slice()
        {
            [] => {
                return Err(format!(
                    "--from-parts: no part holds the run {path} {mode} (of part {belongs}/{n})"
                ));
            }
            [(file, _, _)] if file.part.k() != belongs => {
                return Err(format!(
                    "--from-parts: the run {path} {mode} is in {} (part {}), its case belongs \
                     to part {belongs}/{n}",
                    file.name, file.part
                ));
            }
            [(file, number, run)] => {
                let slice = count::one_field(&job.slice);
                if run.slice != slice {
                    return Err(format!(
                        "--from-parts: {}:{number}: the run {path} {mode} has slice {}, the \
                         count gives it {slice}",
                        file.name, run.slice
                    ));
                }
                records.push(run.record.clone());
            }
            [(a, la, _), (b, lb, _), ..] => {
                return Err(format!(
                    "--from-parts: the run {path} {mode} is in two parts: {}:{la} and {}:{lb}",
                    a.name, b.name
                ));
            }
        }
    }
    if let Some((file, number, run)) = first_of(held.values().flat_map(BTreeMap::values)) {
        let is_case = loaded
            .iter()
            .any(|item| count::one_field(&item.case.path) == run.path);
        let reason = if !run_cases {
            "the count runs no case (--run none)".to_string()
        } else if !is_case {
            "its case is not a case of the checkout".to_string()
        } else if jobs
            .iter()
            .any(|job| count::one_field(&job.case.path) == run.path)
        {
            format!("its case has no {} run", run.mode)
        } else {
            "its case is not run (excluded, not applicable or unreadable)".to_string()
        };
        return Err(format!(
            "--from-parts: {}:{number}: the run {} {} is no run of the count: {reason}",
            file.name, run.path, run.mode
        ));
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::{Part, escape, part_of, unescape};
    use std::num::NonZeroUsize;

    #[test]
    fn a_part_is_k_of_n() {
        assert_eq!(
            Part::parse("1/1").map(|p| p.to_string()),
            Ok("1/1".to_string())
        );
        assert_eq!(Part::parse("3/5").map(|p| (p.k(), p.n().get())), Ok((3, 5)));
        for bad in [
            "0/2", "3/2", "1/0", "01/2", "+1/2", "1/2/3", "1", "", "a/b", "1 /2",
        ] {
            assert!(Part::parse(bad).is_err(), "{bad}");
        }
        let three = NonZeroUsize::new(3).unwrap_or(NonZeroUsize::MIN);
        let parts: Vec<usize> = (0..7).map(|i| part_of(i, three)).collect();
        assert_eq!(parts, [1, 2, 3, 1, 2, 3, 1]);
    }

    #[test]
    fn an_escaped_detail_is_read_back() {
        let raw = "panicked: a\tb\nc\r\\d\u{2028}e";
        assert_eq!(escape(raw), "panicked: a\\tb\\nc\\r\\\\d\u{2028}e");
        assert_eq!(unescape(&escape(raw)).as_deref(), Some(raw));
        assert_eq!(unescape("a\\x"), None);
        assert_eq!(unescape("a\\"), None);
    }
}

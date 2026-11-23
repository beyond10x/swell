//! The tests of the run in parts (`count --part <k>/<n>` and `count --from-parts <dir>`), on the
//! fixture trees of `tests/fixtures/`, read and not changed. A part writes the file `--runs`
//! names; the count from the parts reads every `*.tsv` of a directory and runs no case. Each test
//! works in a directory of its own under `CARGO_TARGET_TMPDIR`.
//!
//! The count from the parts prints the bytes the whole run prints, on standard output and on
//! standard error, and writes the same run lines, but for the milliseconds of the
//! `test262-time` lines: a time is a measurement, and the merged line is the sum of the times
//! of the parts' runs, as the whole run's line is the sum of the times of its runs.

use std::error::Error;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process::Command;

use swell::Limits;
use swell_test262::parts::{self, Part, PartFile};
use swell_test262::{count, features, run, slices};

type TestResult = Result<(), Box<dyn Error>>;

/// What one run of the binary printed.
struct Printed {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Printed {
    fn all(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }

    /// The lines of standard error that start with `prefix` and a space.
    fn errors(&self, prefix: &str) -> Vec<&str> {
        self.stderr
            .lines()
            .filter(|l| l.split(' ').next() == Some(prefix))
            .collect()
    }

    /// Standard error with the milliseconds of each `test262-time` line taken off.
    fn stderr_but_times(&self) -> String {
        without_times(self.stderr.lines())
    }
}

/// `lines`, each with a line end, with the milliseconds of each `test262-time` line taken off.
fn without_times<'l>(lines: impl Iterator<Item = &'l str>) -> String {
    lines
        .map(|line| match line.strip_prefix("test262-time ") {
            Some(rest) => {
                let kept = rest.rsplit_once(' ').map_or(rest, |(kept, _)| kept);
                format!("test262-time {kept} <ms>\n")
            }
            None => format!("{line}\n"),
        })
        .collect()
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

/// A directory of the test `name` under `CARGO_TARGET_TMPDIR`, empty.
fn work_dir(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = PathBuf::from(format!("{}/parts-{name}", env!("CARGO_TARGET_TMPDIR")));
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn runner(args: &[&str]) -> std::io::Result<Printed> {
    let output = Command::new(env!("CARGO_BIN_EXE_swell-test262"))
        .args(args)
        .output()?;
    Ok(Printed {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// `count` over the fixture tree `tree` with the fixture design and features and `extra`.
fn count(tree: &str, extra: &[&str]) -> std::io::Result<Printed> {
    let (tree, design, features) = (fixture(tree), fixture("design.md"), fixture("features.txt"));
    let mut args = vec![
        "count",
        "--test262",
        &tree,
        "--design",
        &design,
        "--features",
        &features,
    ];
    args.extend_from_slice(extra);
    runner(&args)
}

/// The path of the file of part `k` of `n` in `dir`.
fn part_file(dir: &std::path::Path, k: usize, n: usize) -> String {
    dir.join(format!("part-{k}-of-{n}.tsv"))
        .display()
        .to_string()
}

/// Runs every part of `n` of `tree` into `dir`, with `extra`; each must exit 0 and print no count
/// line.
fn make_parts(tree: &str, dir: &std::path::Path, n: usize, extra: &[&str]) -> TestResult {
    for k in 1..=n {
        let file = part_file(dir, k, n);
        let part = format!("{k}/{n}");
        let mut args = vec!["--part", part.as_str(), "--runs", file.as_str()];
        args.extend_from_slice(extra);
        let printed = count(tree, &args)?;
        assert_eq!(printed.code, Some(0), "part {part}: {}", printed.all());
        assert!(
            printed
                .stdout
                .lines()
                .all(|l| !l.starts_with("test262 ") && !l.starts_with("test262-slice ")),
            "a part prints no count line: {}",
            printed.stdout
        );
        assert!(std::path::Path::new(&file).is_file(), "{file}");
    }
    Ok(())
}

/// The whole run of `tree` with `extra` and its run lines, and the counts from 2 and from 3 parts
/// with the same `extra`: each prints the bytes of the whole run, but the milliseconds of its
/// time lines, and writes the same run lines. Answers the whole run.
fn assert_parts_print_the_whole_run(
    tree: &str,
    name: &str,
    extra: &[&str],
) -> Result<Printed, Box<dyn Error>> {
    let dir = work_dir(name)?;
    let whole_runs = dir.join("whole-runs.tsv").display().to_string();
    let mut args = vec!["--runs", whole_runs.as_str()];
    args.extend_from_slice(extra);
    let whole = count(tree, &args)?;
    let whole_lines = std::fs::read_to_string(&whole_runs)?;
    assert!(!whole_lines.is_empty(), "{tree}: the whole run made runs");
    for n in [2, 3] {
        let parts_dir = dir.join(format!("parts-of-{n}"));
        std::fs::create_dir_all(&parts_dir)?;
        make_parts(tree, &parts_dir, n, &[])?;
        let merged_runs = dir
            .join(format!("merged-runs-{n}.tsv"))
            .display()
            .to_string();
        let parts_path = parts_dir.display().to_string();
        let mut args = vec![
            "--from-parts",
            parts_path.as_str(),
            "--runs",
            merged_runs.as_str(),
        ];
        args.extend_from_slice(extra);
        let merged = count(tree, &args)?;
        assert_eq!(
            merged.code,
            whole.code,
            "{tree} from {n} parts: {}",
            merged.all()
        );
        assert_eq!(merged.stdout, whole.stdout, "{tree} from {n} parts");
        assert_eq!(
            merged.stderr_but_times(),
            whole.stderr_but_times(),
            "{tree} from {n} parts"
        );
        assert_eq!(
            merged.errors("test262-time").len(),
            whole.errors("test262-time").len()
        );
        assert_eq!(
            std::fs::read_to_string(&merged_runs)?,
            whole_lines,
            "{tree} from {n} parts: the run lines, in the order of the whole run"
        );
    }
    Ok(whole)
}

#[test]
fn the_count_from_parts_of_the_run_tree_prints_what_the_whole_run_prints() -> TestResult {
    let whole = assert_parts_print_the_whole_run("run", "run-tree", &[])?;
    assert_eq!(whole.code, Some(0), "{}", whole.all());
    assert_eq!(whole.errors("test262-refused"), ["test262-refused 0"]);
    assert_eq!(whole.errors("test262-steps").len(), 20);
    Ok(())
}

#[test]
fn the_count_from_parts_of_the_refused_tree_prints_what_the_whole_run_prints() -> TestResult {
    let whole = assert_parts_print_the_whole_run("refused", "refused-tree", &[])?;
    assert_eq!(whole.code, Some(0), "{}", whole.all());
    assert_eq!(whole.errors("test262-refused"), ["test262-refused 6"]);
    Ok(())
}

#[test]
fn the_count_from_parts_of_the_counted_tree_prints_what_the_whole_run_prints() -> TestResult {
    // The tree of excluded, not applicable and unreadable cases, with the sections and the
    // expected file: the comparison and its line come from the count from the parts as well.
    let (sections, expected) = (fixture("sections.txt"), fixture("expected.txt"));
    let whole = assert_parts_print_the_whole_run(
        "tree",
        "counted-tree",
        &["--sections", &sections, "--expected", &expected],
    )?;
    assert_eq!(whole.code, Some(0), "{}", whole.all());
    assert_eq!(whole.errors("test262-error").len(), 4, "{}", whole.stderr);
    assert!(
        whole
            .stderr
            .lines()
            .any(|l| l == format!("test262: 27 lines equal {expected}")),
        "{}",
        whole.stderr
    );
    Ok(())
}

/// The fields of the lines of `text` that start with `kind`, each split at its tabs.
fn lines_of<'t>(text: &'t str, kind: &str) -> Vec<Vec<&'t str>> {
    text.lines()
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .filter(|f| f.first() == Some(&kind))
        .collect()
}

#[test]
fn every_case_is_in_exactly_one_part() -> TestResult {
    // The cases in byte order, as `meta` lists them; case i belongs to part i mod n + 1.
    let listed = runner(&["meta", "--test262", &fixture("tree")])?;
    assert_eq!(listed.code, Some(0), "{}", listed.all());
    let cases: Vec<&str> = listed
        .stdout
        .lines()
        .filter_map(|l| l.split('\t').next())
        .collect();
    assert_eq!(cases.len(), 22);
    let dir = work_dir("exactly-one")?;
    for n in 1..=5usize {
        make_parts("tree", &dir, n, &["--run", "none"])?;
        let mut seen: Vec<(String, usize)> = Vec::new();
        for k in 1..=n {
            let text = std::fs::read_to_string(part_file(&dir, k, n))?;
            assert_eq!(
                text.lines().next(),
                Some(format!("test262-part\t{k}/{n}\tnone\t22").as_str()),
                "{k}/{n}"
            );
            assert_eq!(lines_of(&text, "test262-run"), Vec::<Vec<&str>>::new());
            for fields in lines_of(&text, "test262-case") {
                assert_eq!(fields.len(), 2, "{fields:?}");
                seen.push((fields.get(1).copied().unwrap_or("").to_string(), k));
            }
        }
        seen.sort();
        let expected: Vec<(String, usize)> = cases
            .iter()
            .enumerate()
            .map(|(i, path)| ((*path).to_string(), i % n + 1))
            .collect();
        let mut sorted = expected.clone();
        sorted.sort();
        assert_eq!(seen, sorted, "every case once, in part i mod {n} + 1");
    }
    // The runs of a part are the runs of its cases: the dynamic-refusal tree in 2 parts.
    let runs_dir = work_dir("exactly-one-runs")?;
    make_parts("dynamic-refusal", &runs_dir, 2, &[])?;
    let mut runs: Vec<(String, String)> = Vec::new();
    for k in 1..=2 {
        let text = std::fs::read_to_string(part_file(&runs_dir, k, 2))?;
        let held: Vec<&str> = lines_of(&text, "test262-case")
            .iter()
            .filter_map(|f| f.get(1).copied())
            .collect();
        for fields in lines_of(&text, "test262-run") {
            assert_eq!(fields.len(), 11, "{fields:?}");
            let path = fields.get(2).copied().unwrap_or("");
            assert!(held.contains(&path), "{path} is a case of part {k}/2");
            runs.push((
                path.to_string(),
                fields.get(3).copied().unwrap_or("").to_string(),
            ));
        }
    }
    runs.sort();
    let before = runs.len();
    runs.dedup();
    assert_eq!(runs.len(), before, "no run in two parts");
    assert_eq!(runs.len(), 6, "three cases of two runs each: {runs:?}");
    Ok(())
}

#[test]
fn a_budget_and_a_panic_are_read_from_a_part() -> TestResult {
    // The endless case of tests/fixtures/budget/ in one part of one, with a step budget of 10^6
    // as tests/run.rs runs it: the count from the part prints its budget and steps lines.
    let limits = Limits {
        steps_per_entry: 1_000_000,
        ..run::LIMITS
    };
    let (rules, problems) = slices::read(&std::fs::read_to_string(fixture("design.md"))?);
    assert_eq!(problems, Vec::<String>::new());
    let (listed, problems) =
        features::read_listed(&std::fs::read_to_string(fixture("features.txt"))?);
    assert_eq!(problems, Vec::<String>::new());
    let tree = PathBuf::from(fixture("budget"));
    let loaded = count::load(&tree)?;
    let lines = parts::run_part(
        &tree,
        &loaded,
        &rules,
        &listed,
        Part::parse("1/1")?,
        Some((NonZeroUsize::MIN, &limits)),
    )?;
    let file = PartFile {
        name: "part-1-of-1.tsv".to_string(),
        text: lines.iter().map(|l| format!("{l}\n")).collect(),
    };
    let mut merged = count::load(&tree)?;
    let ran = parts::from_parts(&mut merged, &rules, &listed, &[file], true)?;
    let endless = "test/language/budget/endless.js";
    assert_eq!(
        ran.runs,
        [format!(
            "test262-run\t02\t{endless}\tnon-strict\tBudget\tcase: Terminated(StepBudget)"
        )]
    );
    assert_eq!(
        without_times(ran.log.iter().map(String::as_str)),
        format!(
            "test262-budget {endless} non-strict StepBudget\ntest262-refused 0\n\
             test262-steps 1000000 {endless} non-strict\ntest262-time 02 1 <ms>\n"
        )
    );
    assert!(
        count::tally(&merged, &rules, &listed)
            .lines()
            .contains(&"test262 language/budget 1 0 0 1 0 0 1".to_string())
    );

    // A run that panicked: its detail as the run gave it, tab and line end, is printed on the
    // panic line, and as one field on the run line.
    let file = PartFile {
        name: "panicked.tsv".to_string(),
        text: format!(
            "test262-part\t1/1\tall\t1\ntest262-case\t{endless}\n\
             test262-run\t02\t{endless}\tnon-strict\tFailed\tpanicked: a b c\t7\t1000\t0\t-\t\
             panicked: a\\tb\\nc\n"
        ),
    };
    let mut merged = count::load(&tree)?;
    let ran = parts::from_parts(&mut merged, &rules, &listed, &[file], true)?;
    assert_eq!(
        ran.runs,
        [format!(
            "test262-run\t02\t{endless}\tnon-strict\tFailed\tpanicked: a b c"
        )]
    );
    assert_eq!(
        ran.log.first().map(String::as_str),
        Some(format!("test262-panic {endless} non-strict panicked: a\tb\nc").as_str())
    );
    assert!(
        count::tally(&merged, &rules, &listed)
            .lines()
            .contains(&"test262 language/budget 1 0 0 1 0 1 0".to_string())
    );
    Ok(())
}

/// The count from the parts in `dir` of the tree `tree`; it must exit 2, print no count line and
/// name what it refused. Answers its standard error.
fn refused_from_parts(
    tree: &str,
    dir: &std::path::Path,
    extra: &[&str],
) -> Result<String, Box<dyn Error>> {
    let path = dir.display().to_string();
    let mut args = vec!["--from-parts", path.as_str()];
    args.extend_from_slice(extra);
    let printed = count(tree, &args)?;
    assert_eq!(printed.code, Some(2), "{}", printed.all());
    assert_eq!(printed.stdout, "", "a refused count prints no count line");
    Ok(printed.stderr)
}

/// Appends `line` and a line end to the file `path`.
fn append(path: &str, line: &str) -> TestResult {
    let mut text = std::fs::read_to_string(path)?;
    text.push_str(line);
    text.push('\n');
    std::fs::write(path, text)?;
    Ok(())
}

#[test]
fn a_count_from_parts_refuses_a_missing_part() -> TestResult {
    let dir = work_dir("missing-part")?;
    make_parts("tree", &dir, 3, &["--run", "none"])?;
    std::fs::remove_file(part_file(&dir, 2, 3))?;
    let stderr = refused_from_parts("tree", &dir, &["--run", "none"])?;
    assert!(
        stderr.contains("--from-parts: part 2/3 is missing; the files state 1/3, 3/3"),
        "{stderr}"
    );
    // A run the whole run makes that no part holds: the first run line of part 1 of the
    // dynamic-refusal tree taken out.
    let runs = work_dir("missing-run")?;
    make_parts("dynamic-refusal", &runs, 2, &[])?;
    let first = part_file(&runs, 1, 2);
    let text = std::fs::read_to_string(&first)?;
    let kept: String = text
        .lines()
        .filter(|l| {
            !(l.starts_with("test262-run\t") && l.contains("/caught-in-eval.js\tnon-strict\t"))
        })
        .map(|l| format!("{l}\n"))
        .collect();
    assert_ne!(kept, text);
    std::fs::write(&first, kept)?;
    let stderr = refused_from_parts("dynamic-refusal", &runs, &[])?;
    assert!(
        stderr.contains(
            "--from-parts: no part holds the run \
             test/language/dynamic-refusal/caught-in-eval.js non-strict (of part 1/2)"
        ),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_count_from_parts_refuses_a_run_in_two_parts() -> TestResult {
    let dir = work_dir("run-in-two")?;
    make_parts("dynamic-refusal", &dir, 2, &[])?;
    // The first run of part 2 written into part 1 as well.
    let second = std::fs::read_to_string(part_file(&dir, 2, 2))?;
    let copied = second
        .lines()
        .find(|l| l.starts_with("test262-run\t"))
        .ok_or("part 2/2 holds no run")?;
    let first = part_file(&dir, 1, 2);
    append(&first, copied)?;
    let stderr = refused_from_parts("dynamic-refusal", &dir, &[])?;
    let path = "test/language/dynamic-refusal/caught-in-function.js";
    assert!(
        stderr.contains(&format!(
            "--from-parts: the run {path} non-strict is in two parts: {first}:"
        )) && stderr.contains(&format!(" and {}:", part_file(&dir, 2, 2))),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_count_from_parts_refuses_a_run_of_an_unknown_case() -> TestResult {
    let dir = work_dir("unknown-case")?;
    make_parts("dynamic-refusal", &dir, 2, &[])?;
    let first = part_file(&dir, 1, 2);
    let unknown = "test/language/dynamic-refusal/no-such-case.js";
    append(
        &first,
        &format!("test262-run\t02\t{unknown}\tnon-strict\tPassed\t\t1\t1\t0\t-\t"),
    )?;
    let stderr = refused_from_parts("dynamic-refusal", &dir, &[])?;
    assert!(
        stderr.contains(&format!(
            "the run {unknown} non-strict is no run of the count: its case is not a case of \
             the checkout"
        )) && stderr.contains(&format!("--from-parts: {first}:")),
        "{stderr}"
    );
    // A case line the list does not hold is refused as well.
    let cases = work_dir("unknown-case-line")?;
    make_parts("tree", &cases, 2, &["--run", "none"])?;
    append(
        &part_file(&cases, 2, 2),
        &format!("test262-case\t{unknown}"),
    )?;
    let stderr = refused_from_parts("tree", &cases, &["--run", "none"])?;
    assert!(
        stderr.contains(&format!("the case {unknown} is not a case of the checkout")),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_count_from_parts_refuses_parts_of_different_n() -> TestResult {
    let dir = work_dir("different-n")?;
    make_parts("tree", &dir, 2, &["--run", "none"])?;
    make_parts("tree", &dir, 3, &["--run", "none"])?;
    let stderr = refused_from_parts("tree", &dir, &["--run", "none"])?;
    // The files in byte order of their names: part-1-of-2.tsv, part-1-of-3.tsv, ...
    assert!(
        stderr.contains(&format!(
            "--from-parts: {} states part 1/3, {} states part 1/2: the parts of one count \
             state one n",
            part_file(&dir, 1, 3),
            part_file(&dir, 1, 2)
        )),
        "{stderr}"
    );
    // Parts made with --run none do not count a run with --run all.
    let none = work_dir("other-run")?;
    make_parts("tree", &none, 1, &["--run", "none"])?;
    let stderr = refused_from_parts("tree", &none, &[])?;
    assert!(
        stderr.contains("was made with --run none, the count has --run all"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_part_and_a_count_from_parts_refuse_a_wrong_command() -> TestResult {
    let dir = work_dir("wrong-command")?;
    let file = part_file(&dir, 1, 2);
    let (sections, expected) = (fixture("sections.txt"), fixture("expected.txt"));
    let path = dir.display().to_string();
    for args in [
        vec!["--part", "1/2"],
        vec!["--part", "3/2", "--runs", &file],
        vec!["--part", "0/2", "--runs", &file],
        vec!["--part", "1/2", "--runs", &file, "--expected", &expected],
        vec!["--part", "1/2", "--runs", &file, "--sections", &sections],
        vec!["--part", "1/2", "--runs", &file, "--from-parts", &path],
        vec!["--from-parts", &path, "--workers", "2"],
    ] {
        let printed = count("tree", &args)?;
        assert_eq!(printed.code, Some(2), "{args:?}: {}", printed.all());
        assert_eq!(printed.stdout, "", "{args:?}");
    }
    // A directory without a part's file.
    let stderr = refused_from_parts("tree", &dir, &[])?;
    assert!(
        stderr.contains("--from-parts: no part file (*.tsv) in "),
        "{stderr}"
    );
    Ok(())
}

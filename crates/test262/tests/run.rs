//! The tests of the run path of `swell-test262` (design, decision 11): every run case is run on
//! the engine. They count the fixture tree `tests/fixtures/run/`, written for them with a small
//! `assert.js` and `sta.js` of its own (no file of test262 is copied), with `design.md` and
//! `features.txt`. Each directory of the tree holds the cases of one behaviour, so its line shows
//! the outcome: `test262 <dir> <exist> <excluded> <not-applicable> <run> <passed> <failed>
//! <budget>`. `tests/fixtures/budget/` holds one case that never ends, run with a smaller step
//! budget than the runner's. `tests/fixtures/refused/` holds cases whose source or include the
//! parser refuses or nests too deep, and one early error the ES5 grammar has.
//! `tests/fixtures/eval-script/` holds the cases of `$262.evalScript`: three that pass, and two
//! that catch the `SyntaxError` of a refusal or of the nesting limit inside it and fail.
//!
//! With `--runs <file>` the runner writes one line per run to the file, `test262-run`, the slice,
//! the path, the mode, the outcome and the detail, separated by tabs; the tests read it from
//! `CARGO_TARGET_TMPDIR`, one file per test.

use std::error::Error;
use std::num::NonZeroUsize;
use std::process::Command;

use swell::Limits;
use swell_test262::{count, run};
use swell_test262::{features, slices};

type TestResult = Result<(), Box<dyn Error>>;

/// What one run of the binary printed.
struct Printed {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Printed {
    fn has(&self, line: &str) -> bool {
        self.stdout.lines().any(|l| l == line)
    }

    /// The lines of standard error that start with `prefix` and a space.
    fn errors(&self, prefix: &str) -> Vec<&str> {
        self.stderr
            .lines()
            .filter(|l| l.split(' ').next() == Some(prefix))
            .collect()
    }

    fn all(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

/// `count` over the run tree, with `extra` arguments after the usual ones.
fn count_run(extra: &[&str]) -> std::io::Result<Printed> {
    count_tree("run", extra)
}

/// `count` over the fixture tree `name`, with `extra` arguments after the usual ones.
fn count_tree(name: &str, extra: &[&str]) -> std::io::Result<Printed> {
    let (tree, design, features) = (fixture(name), fixture("design.md"), fixture("features.txt"));
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
    let output = Command::new(env!("CARGO_BIN_EXE_swell-test262"))
        .args(&args)
        .output()?;
    Ok(Printed {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// The run tree counted with the default worker count; it must exit 0.
fn counted() -> Result<Printed, Box<dyn Error>> {
    let printed = count_run(&[])?;
    assert_eq!(printed.code, Some(0), "{}", printed.all());
    Ok(printed)
}

fn assert_line(printed: &Printed, line: &str) {
    assert!(printed.has(line), "{line} not in\n{}", printed.all());
}

#[test]
fn a_passing_case_is_counted_passed() -> TestResult {
    let printed = counted()?;
    // pass.js and includes.js (whose two includes run in order) pass in both runs.
    assert_line(&printed, "test262 harness 2 0 0 2 2 0 0");
    assert_line(&printed, "test262-slice 01 3 6 0 0 3 3 0 0");
    Ok(())
}

#[test]
fn a_failing_assert_is_counted_failed() -> TestResult {
    let printed = counted()?;
    assert_line(&printed, "test262 language/fail 1 0 0 1 0 1 0");
    Ok(())
}

#[test]
fn strict_run_prepends_use_strict() -> TestResult {
    let printed = counted()?;
    // both.js fails only in strict code and is run twice: its strict run fails it. no-strict.js
    // (the same body, noStrict) passes, and only-strict.js passes only in strict code.
    assert_line(&printed, "test262 language/strict 3 0 0 3 2 1 0");
    Ok(())
}

#[test]
fn only_strict_and_no_strict_run_once() -> TestResult {
    let printed = counted()?;
    // Slice 02 has 21 runs: two per case but one for each noStrict, onlyStrict and raw case.
    assert_line(&printed, "test262-slice 02 12 21 0 0 12 5 7 0");
    let times = printed.errors("test262-time");
    assert!(
        times.iter().any(|l| l.starts_with("test262-time 02 21 ")),
        "{times:?}"
    );
    assert_line(&printed, "test262 language/strict 3 0 0 3 2 1 0");
    Ok(())
}

#[test]
fn raw_case_gets_no_harness() -> TestResult {
    let printed = counted()?;
    // no-harness.js throws when assert or Test262Error is defined or when it runs strict.
    assert_line(&printed, "test262 language/raw 1 0 0 1 1 0 0");
    Ok(())
}

#[test]
fn negative_parse_case_needs_the_named_error() -> TestResult {
    let printed = counted()?;
    // named.js passes; other-type.js (a SyntaxError where ReferenceError is named) and
    // parses.js (no parse error) fail.
    assert_line(&printed, "test262 language/negative-parse 3 0 0 3 1 2 0");
    Ok(())
}

#[test]
fn negative_runtime_case_needs_the_named_error() -> TestResult {
    let printed = counted()?;
    // named.js passes; other-type.js (a TypeError where Test262Error is named) and no-throw.js
    // fail.
    assert_line(&printed, "test262 language/negative-runtime 3 0 0 3 1 2 0");
    Ok(())
}

#[test]
fn an_endless_case_is_printed_as_budget() -> TestResult {
    // The runner's budgets are the design's first values; a test build spends 2^30 steps in
    // minutes, so the endless case of tests/fixtures/budget/ runs here with a step budget of
    // 10^6 and the other limits of the runner's table, through the functions `count` calls.
    assert_eq!(run::LIMITS.steps_per_entry, 1 << 30);
    assert_eq!(run::LIMITS.memory_bytes, 512 * 1024 * 1024);
    let limits = Limits {
        steps_per_entry: 1_000_000,
        ..run::LIMITS
    };
    let design = std::fs::read_to_string(fixture("design.md"))?;
    let (rules, problems) = slices::read(&design);
    assert_eq!(problems, Vec::<String>::new());
    let (listed, problems) =
        features::read_listed(&std::fs::read_to_string(fixture("features.txt"))?);
    assert_eq!(problems, Vec::<String>::new());
    let tree = std::path::PathBuf::from(fixture("budget"));
    let mut loaded = count::load(&tree)?;
    let ran = count::run_cases(
        &tree,
        &mut loaded,
        &rules,
        &listed,
        NonZeroUsize::MIN,
        &limits,
    )?;
    assert_eq!(
        ran.runs,
        [
            "test262-run\t02\ttest/language/budget/endless.js\tnon-strict\tBudget\tcase: Terminated(StepBudget)"
        ],
    );
    let lines = ran.log;
    let budget: Vec<&String> = lines
        .iter()
        .filter(|l| l.starts_with("test262-budget "))
        .collect();
    assert_eq!(
        budget,
        ["test262-budget test/language/budget/endless.js non-strict StepBudget"],
        "{lines:?}"
    );
    assert!(
        lines.contains(
            &"test262-steps 1000000 test/language/budget/endless.js non-strict".to_string()
        ),
        "{lines:?}"
    );
    let report = count::tally(&loaded, &rules, &listed);
    assert!(
        report
            .lines()
            .contains(&"test262 language/budget 1 0 0 1 0 0 1".to_string()),
        "{:?}",
        report.lines()
    );
    Ok(())
}

#[test]
fn twenty_most_expensive_runs_are_printed() -> TestResult {
    let printed = counted()?;
    let lines = printed.errors("test262-steps");
    // The tree has 27 runs; twenty are printed, the most steps first.
    assert_eq!(lines.len(), 20, "{}", printed.stderr);
    let mut previous = u64::MAX;
    for line in &lines {
        let words: Vec<&str> = line.split(' ').collect();
        assert_eq!(words.len(), 4, "{line}");
        let steps: u64 = words.get(1).and_then(|w| w.parse().ok()).unwrap_or(0);
        assert!(steps > 0 && steps <= previous, "{line} after {previous}");
        previous = steps;
        let path = words.get(2).copied().unwrap_or_default();
        assert!(
            std::path::Path::new(&fixture("run")).join(path).is_file(),
            "{line}"
        );
        assert!(
            matches!(words.get(3), Some(&"strict" | &"non-strict")),
            "{line}"
        );
    }
    // The six runs of negative-parse/ spend no step: nothing runs when the case does not parse, and
    // parses.js fails before anything runs. They are among the seven left out.
    assert!(
        lines.iter().all(|l| !l.contains("/negative-parse/")),
        "{lines:?}"
    );
    Ok(())
}

#[test]
fn run_time_is_printed_per_slice() -> TestResult {
    let printed = counted()?;
    let times = printed.errors("test262-time");
    // One line per slice of the tree, with the runs it ran: 6 in slice 01, 21 in slice 02.
    assert_eq!(times.len(), 2, "{}", printed.stderr);
    for (line, (slice, runs)) in times.iter().zip([("01", "6"), ("02", "21")]) {
        let words: Vec<&str> = line.split(' ').collect();
        assert_eq!(words.len(), 4, "{line}");
        assert_eq!(words.get(1), Some(&slice), "{line}");
        assert_eq!(words.get(2), Some(&runs), "{line}");
        assert!(
            words.get(3).is_some_and(|ms| ms.parse::<u64>().is_ok()),
            "{line}"
        );
    }
    Ok(())
}

#[test]
fn create_realm_gives_a_fresh_global() -> TestResult {
    let printed = counted()?;
    // create-realm.js checks $262.global, and that two calls of createRealm give two realms,
    // each with its own global object, intrinsics and $262.
    assert_line(&printed, "test262 harness/realm 1 0 0 1 1 0 0");
    Ok(())
}

#[test]
fn worker_count_does_not_change_the_lines() -> TestResult {
    let one = count_run(&["--workers", "1"])?;
    let four = count_run(&["--workers", "4"])?;
    assert_eq!(one.code, Some(0), "{}", one.all());
    assert_eq!(four.code, Some(0), "{}", four.all());
    assert_eq!(one.stdout, four.stdout);
    assert_eq!(one.errors("test262-steps"), four.errors("test262-steps"));
    assert_eq!(one.errors("test262-budget"), four.errors("test262-budget"));
    assert_eq!(counted()?.stdout, one.stdout, "the default is one worker");
    for bad in ["0", "x", "-1"] {
        let printed = count_run(&["--workers", bad])?;
        assert_eq!(printed.code, Some(2), "--workers {bad}: {}", printed.all());
    }
    Ok(())
}

#[test]
fn run_none_counts_without_running() -> TestResult {
    // The counting alone, for the planted scripts of tests/gate/: every run case is counted
    // failed, every other column is as with the cases run, and no line of a run is printed.
    let none = count_run(&["--run", "none"])?;
    assert_eq!(none.code, Some(0), "{}", none.all());
    assert_line(&none, "test262 harness 2 0 0 2 0 2 0");
    assert_line(&none, "test262-slice 01 3 6 0 0 3 0 3 0");
    assert_line(&none, "test262-slice 02 12 21 0 0 12 0 12 0");
    for prefix in [
        "test262-steps",
        "test262-time",
        "test262-refused",
        "test262-budget",
    ] {
        assert_eq!(none.errors(prefix), Vec::<&str>::new(), "{prefix}");
    }
    let all = count_run(&["--run", "all"])?;
    assert_eq!(all.stdout, counted()?.stdout, "the default is all");
    for bad in ["", "some", "None"] {
        let printed = count_run(&["--run", bad])?;
        assert_eq!(printed.code, Some(2), "--run {bad:?}: {}", printed.all());
    }
    Ok(())
}

/// The refused tree counted; it must exit 0.
fn counted_refused() -> Result<Printed, Box<dyn Error>> {
    let printed = count_tree("refused", &[])?;
    assert_eq!(printed.code, Some(0), "{}", printed.all());
    Ok(printed)
}

#[test]
fn a_refused_negative_parse_case_is_counted_failed() -> TestResult {
    let printed = counted_refused()?;
    // negative.js expects a SyntaxError at parse and gets one, but it is the refusal of an arrow
    // function: it fails. positive.js (refused, no negative) and include.js (a refused include)
    // fail as well.
    assert_line(&printed, "test262 language/refused 3 0 0 3 0 3 0");
    // A SyntaxError of the nesting limit is not the error a case names either.
    assert_line(&printed, "test262 language/nesting 1 0 0 1 0 1 0");
    Ok(())
}

#[test]
fn an_early_error_of_the_es5_grammar_passes_a_negative_parse_case() -> TestResult {
    let printed = counted_refused()?;
    assert_line(&printed, "test262 language/early-error 1 0 0 1 1 0 0");
    Ok(())
}

#[test]
fn refused_runs_are_printed() -> TestResult {
    // The three cases of refused/ have two runs each, every one ended by a refusal; the nesting
    // limit and the early error are not refusals.
    let printed = counted_refused()?;
    assert_eq!(printed.errors("test262-refused"), ["test262-refused 6"]);
    assert_eq!(counted()?.errors("test262-refused"), ["test262-refused 0"]);
    Ok(())
}

/// One line of the runs file, split at its tabs.
type RunLine = Vec<String>;

/// The path of the runs file of the test `name`, removed if a former run left it.
fn runs_path(name: &str) -> Result<String, Box<dyn Error>> {
    let path = format!("{}/runs-{name}.tsv", env!("CARGO_TARGET_TMPDIR"));
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(path)
}

/// `count` over the fixture tree `tree` with `--runs` and `extra`; it must exit 0. Answers what
/// it printed and the lines of the runs file, each split at its tabs.
fn counted_runs(
    tree: &str,
    name: &str,
    extra: &[&str],
) -> Result<(Printed, Vec<RunLine>), Box<dyn Error>> {
    let path = runs_path(name)?;
    let mut args = vec!["--runs", path.as_str()];
    args.extend_from_slice(extra);
    let printed = count_tree(tree, &args)?;
    assert_eq!(printed.code, Some(0), "{}", printed.all());
    let text = std::fs::read_to_string(&path)?;
    let lines = text
        .lines()
        .map(|line| line.split('\t').map(str::to_string).collect())
        .collect();
    Ok((printed, lines))
}

/// The run lines of the case `path`, in the order of its runs.
fn runs_of<'l>(lines: &'l [RunLine], path: &str) -> Vec<&'l RunLine> {
    lines
        .iter()
        .filter(|line| line.get(2).map(String::as_str) == Some(path))
        .collect()
}

/// Field `n` (from 1, as `awk` counts) of a run line.
fn field(line: &RunLine, n: usize) -> &str {
    line.get(n - 1).map_or("", String::as_str)
}

#[test]
fn runs_file_has_one_line_per_run() -> TestResult {
    let (printed, lines) = counted_runs("run", "one-line-per-run", &[])?;
    // The run tree has 27 runs: 6 of slice 01 (test/harness/), 21 of slice 02 (test/language/),
    // as its slice lines count them.
    assert_line(&printed, "test262-slice 01 3 6 0 0 3 3 0 0");
    assert_line(&printed, "test262-slice 02 12 21 0 0 12 5 7 0");
    assert_eq!(lines.len(), 27, "{lines:?}");
    let mut paths: Vec<&str> = Vec::new();
    for line in &lines {
        assert_eq!(line.len(), 6, "{line:?}");
        assert_eq!(field(line, 1), "test262-run", "{line:?}");
        let path = field(line, 3);
        let slice = if path.starts_with("test/harness/") {
            "01"
        } else {
            "02"
        };
        assert_eq!(field(line, 2), slice, "{line:?}");
        assert!(
            std::path::Path::new(&fixture("run")).join(path).is_file(),
            "{line:?}"
        );
        assert!(
            matches!(field(line, 4), "strict" | "non-strict"),
            "{line:?}"
        );
        assert!(
            matches!(field(line, 5), "Passed" | "Failed" | "Budget"),
            "{line:?}"
        );
        if paths.last() != Some(&path) {
            paths.push(path);
        }
    }
    // In the order of the jobs: the cases in byte order, each with its runs together, the
    // non-strict run first.
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(paths, sorted, "each case once, in byte order");
    assert_eq!(paths.len(), 15, "{paths:?}");
    let both = runs_of(&lines, "test/language/strict/both.js");
    let modes: Vec<&str> = both.iter().map(|l| field(l, 4)).collect();
    assert_eq!(modes, ["non-strict", "strict"]);
    let outcomes: Vec<&str> = both.iter().map(|l| field(l, 5)).collect();
    assert_eq!(
        outcomes,
        ["Passed", "Failed"],
        "both.js fails in strict code only"
    );
    // A case passes when every run passed: the passed cases of the slice lines.
    for (slice, passed) in [("01", 3), ("02", 5)] {
        let cases = paths
            .iter()
            .filter(|path| {
                let runs = runs_of(&lines, path);
                runs.iter()
                    .all(|l| field(l, 2) == slice && field(l, 5) == "Passed")
            })
            .count();
        assert_eq!(cases, passed, "slice {slice}");
    }
    // With --run none the file is written and holds no line.
    let (none, empty) = counted_runs("run", "run-none", &["--run", "none"])?;
    assert_line(&none, "test262-slice 02 12 21 0 0 12 0 12 0");
    assert_eq!(empty, Vec::<RunLine>::new());
    // A file that cannot be written is a command the runner cannot carry out.
    let missing = format!("{}/no-such-directory/runs.tsv", env!("CARGO_TARGET_TMPDIR"));
    let printed = count_tree("run", &["--runs", &missing, "--run", "none"])?;
    assert_eq!(printed.code, Some(2), "{}", printed.all());
    assert_eq!(
        printed.stdout, "",
        "nothing is printed before the file is written"
    );
    Ok(())
}

#[test]
fn runs_file_names_the_refusal_and_the_thrown_error() -> TestResult {
    let (_, refused) = counted_runs("refused", "refusal", &[])?;
    let positive = runs_of(&refused, "test/language/refused/positive.js");
    assert_eq!(positive.len(), 2, "{refused:?}");
    for line in positive {
        assert_eq!(field(line, 5), "Failed", "{line:?}");
        let detail = field(line, 6);
        assert!(
            detail.starts_with("parse: refused: SyntaxError: ")
                && detail.contains(" is not supported before slice "),
            "{line:?}"
        );
    }
    let nesting = runs_of(&refused, "test/language/nesting/negative.js");
    assert_eq!(nesting.len(), 1, "{refused:?}");
    assert!(
        nesting.iter().all(|l| field(l, 6)
            .starts_with("parse: nesting limit: SyntaxError: nesting deeper than parser_nesting")),
        "{nesting:?}"
    );
    let (_, run) = counted_runs("run", "thrown", &[])?;
    let thrown = runs_of(&run, "test/language/throws/error.js");
    let details: Vec<&str> = thrown.iter().map(|l| field(l, 6)).collect();
    assert_eq!(
        details,
        [
            "threw Error: thrown by the case",
            "threw Error: thrown by the case"
        ]
    );
    // A pass that is not negative has an empty detail; a negative case names the error it
    // passed with.
    let pass = runs_of(&run, "test/harness/pass.js");
    assert!(
        pass.iter()
            .all(|l| field(l, 5) == "Passed" && field(l, 6).is_empty()),
        "{pass:?}"
    );
    let named = runs_of(&run, "test/language/negative-runtime/named.js");
    assert!(
        named
            .iter()
            .all(|l| field(l, 5) == "Passed" && field(l, 6).starts_with("threw Test262Error")),
        "{named:?}"
    );
    Ok(())
}

#[test]
fn runs_file_does_not_change_the_counted_lines() -> TestResult {
    for tree in ["run", "refused"] {
        let without = count_tree(tree, &[])?;
        let (with, _) = counted_runs(tree, &format!("unchanged-{tree}"), &[])?;
        assert_eq!(without.code, Some(0), "{}", without.all());
        assert_eq!(with.stdout, without.stdout, "{tree}");
        // Standard error keeps its lines but the times, which differ from run to run.
        let kept = |p: &Printed| -> Vec<String> {
            p.stderr
                .lines()
                .filter(|l| !l.starts_with("test262-time "))
                .map(str::to_string)
                .collect()
        };
        assert_eq!(kept(&with), kept(&without), "{tree}");
    }
    Ok(())
}

/// The eval-script tree counted with `--runs`.
fn counted_eval_script(name: &str) -> Result<(Printed, Vec<RunLine>), Box<dyn Error>> {
    counted_runs("eval-script", name, &[])
}

/// The case `name` of the eval-script tree passes in both its runs, with an empty detail.
fn assert_eval_script_passes(lines: &[RunLine], name: &str) {
    let path = format!("test/language/eval-script/{name}");
    let runs = runs_of(lines, &path);
    let seen: Vec<(&str, &str, &str)> = runs
        .iter()
        .map(|l| (field(l, 4), field(l, 5), field(l, 6)))
        .collect();
    assert_eq!(
        seen,
        [("non-strict", "Passed", ""), ("strict", "Passed", "")],
        "{path}"
    );
}

#[test]
fn eval_script_declares_in_the_global_scope() -> TestResult {
    let (_, lines) = counted_eval_script("declares")?;
    assert_eval_script_passes(&lines, "declares-a-global.js");
    Ok(())
}

#[test]
fn eval_script_throws_the_first_syntax_error() -> TestResult {
    let (_, lines) = counted_eval_script("syntax-error")?;
    assert_eval_script_passes(&lines, "throws-the-syntax-error.js");
    Ok(())
}

#[test]
fn eval_script_answers_the_completion_value() -> TestResult {
    let (_, lines) = counted_eval_script("completion")?;
    assert_eval_script_passes(&lines, "answers-the-completion.js");
    Ok(())
}

#[test]
fn a_refusal_inside_eval_script_fails_the_run() -> TestResult {
    let (printed, lines) = counted_eval_script("refusal-inside")?;
    // Three cases pass; refusal-fails-the-run.js and nesting-fails-the-run.js catch the
    // SyntaxError they name and fail.
    assert_line(&printed, "test262 language/eval-script 5 0 0 5 3 2 0");
    let runs = runs_of(&lines, "test/language/eval-script/refusal-fails-the-run.js");
    assert_eq!(runs.len(), 2, "{lines:?}");
    for line in runs {
        assert_eq!(field(line, 5), "Failed", "{line:?}");
        let detail = field(line, 6);
        assert!(
            detail.starts_with("evalScript: refused: ")
                && detail.contains(" is not supported before slice "),
            "{line:?}"
        );
    }
    // The two runs of refusal-fails-the-run.js are counted as refused, and not the two of
    // nesting-fails-the-run.js: the nesting limit is not a refusal.
    assert_eq!(printed.errors("test262-refused"), ["test262-refused 2"]);
    Ok(())
}

#[test]
fn the_nesting_limit_inside_eval_script_fails_the_run() -> TestResult {
    // The case nests 2000 parentheses, past the runner's limit.
    assert_eq!(run::LIMITS.parser_nesting, 1_000);
    let (_, lines) = counted_eval_script("nesting-inside")?;
    let runs = runs_of(&lines, "test/language/eval-script/nesting-fails-the-run.js");
    assert_eq!(runs.len(), 2, "{lines:?}");
    for line in runs {
        assert_eq!(field(line, 5), "Failed", "{line:?}");
        assert!(
            field(line, 6)
                .starts_with("evalScript: nesting: nesting deeper than parser_nesting (1000)"),
            "{line:?}"
        );
    }
    Ok(())
}

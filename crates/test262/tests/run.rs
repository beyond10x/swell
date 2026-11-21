//! The tests of the run path of `swell-test262` (design, decision 11): every run case is run on
//! the engine. They count the fixture tree `tests/fixtures/run/`, written for them with a small
//! `assert.js` and `sta.js` of its own (no file of test262 is copied), with `design.md` and
//! `features.txt`. Each directory of the tree holds the cases of one behaviour, so its line shows
//! the outcome: `test262 <dir> <exist> <excluded> <not-applicable> <run> <passed> <failed>
//! <budget>`. `tests/fixtures/budget/` holds one case that never ends, run with a smaller step
//! budget than the runner's. `tests/fixtures/refused/` holds cases whose source or include the
//! parser refuses or nests too deep, and one early error the ES5 grammar has.

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
    let lines = count::run_cases(
        &tree,
        &mut loaded,
        &rules,
        &listed,
        NonZeroUsize::MIN,
        &limits,
    )?;
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

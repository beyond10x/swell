//! The tests of `swell`, the shell binary of `swell-cli`
//! (`docs/design/quality-and-comparison.md` § 7, `story:cli-runs-a-script-file`): each runs the
//! built binary on a fixture of `tests/fixtures/` or on a source given with `-e`, and reads its
//! exit status, standard output and standard error. One test per exit status: 0 normal, 1
//! uncaught throw, 2 syntax error, 3 step budget, 4 memory budget, 5 depth limit, 6 interrupted,
//! 64 usage error.

use std::error::Error;
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

/// What one run of the binary gave.
struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn swell(args: &[&str]) -> Result<Ran, Box<dyn Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_swell"))
        .args(args)
        .output()?;
    Ok(Ran {
        code: output.status.code(),
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
    })
}

fn expect(ran: &Ran, code: i32) -> TestResult {
    if ran.code != Some(code) {
        return Err(format!(
            "exit {:?}, expected {code}; stdout {:?}, stderr {:?}",
            ran.code, ran.stdout, ran.stderr
        )
        .into());
    }
    Ok(())
}

#[test]
fn a_script_that_ends_normally_exits_0_and_prints_a_number_a_string_and_an_object() -> TestResult {
    let ran = swell(&[&fixture("print.js")])?;
    expect(&ran, 0)?;
    assert_eq!(
        ran.stdout,
        "42\ntext\n[object Object]\n1 a null undefined\n\n"
    );
    assert_eq!(ran.stderr, "");
    Ok(())
}

#[test]
fn dash_e_runs_a_source_string() -> TestResult {
    let ran = swell(&["-e", "var x = 6 * 7; print('x is', x)"])?;
    expect(&ran, 0)?;
    assert_eq!(ran.stdout, "x is 42\n");
    Ok(())
}

#[test]
fn console_log_writes_as_print_does() -> TestResult {
    let ran = swell(&["-e", "console.log(1, 'b'); print(console.log === print)"])?;
    expect(&ran, 0)?;
    assert_eq!(ran.stdout, "1 b\ntrue\n");
    Ok(())
}

#[test]
fn date_now_answers_from_the_host_clock() -> TestResult {
    // 1.7e12 ms is 2023-11-14; the engine's own value without a clock would be the epoch.
    let ran = swell(&[
        "-e",
        "var t = Date.now(); print(typeof t, t > 1.7e12, t % 1 === 0)",
    ])?;
    expect(&ran, 0)?;
    assert_eq!(ran.stdout, "number true true\n");
    Ok(())
}

#[test]
fn promise_jobs_run_after_the_script() -> TestResult {
    let ran = swell(&[
        "-e",
        "Promise.resolve(2).then(function (v) { print('job', v) }); print('script')",
    ])?;
    expect(&ran, 0)?;
    assert_eq!(ran.stdout, "script\njob 2\n");
    Ok(())
}

#[test]
fn an_uncaught_throw_exits_1_with_its_message_on_stderr() -> TestResult {
    let ran = swell(&[&fixture("throw.js")])?;
    expect(&ran, 1)?;
    assert_eq!(ran.stdout, "before\n");
    assert!(
        ran.stderr.contains("uncaught TypeError: boom"),
        "stderr {:?}",
        ran.stderr
    );
    Ok(())
}

#[test]
fn an_uncaught_throw_of_a_value_that_is_not_an_error_exits_1() -> TestResult {
    let ran = swell(&["-e", "throw 7"])?;
    expect(&ran, 1)?;
    assert!(ran.stderr.contains("uncaught 7"), "stderr {:?}", ran.stderr);
    Ok(())
}

#[test]
fn a_syntax_error_exits_2_and_runs_nothing() -> TestResult {
    let ran = swell(&[&fixture("syntax.js")])?;
    expect(&ran, 2)?;
    assert_eq!(ran.stdout, "");
    assert!(
        ran.stderr.contains("SyntaxError"),
        "stderr {:?}",
        ran.stderr
    );
    Ok(())
}

#[test]
fn a_step_budget_ends_a_loop_with_status_3() -> TestResult {
    let ran = swell(&["--steps", "100000", &fixture("loop.js")])?;
    expect(&ran, 3)?;
    assert_eq!(ran.stdout, "start\n");
    assert!(
        ran.stderr.contains("step budget"),
        "stderr {:?}",
        ran.stderr
    );
    Ok(())
}

#[test]
fn a_memory_budget_exits_4() -> TestResult {
    let ran = swell(&["--memory", "8000000", &fixture("memory.js")])?;
    expect(&ran, 4)?;
    assert!(
        ran.stderr.contains("memory budget"),
        "stderr {:?}",
        ran.stderr
    );
    Ok(())
}

#[test]
fn a_frame_depth_limit_exits_5() -> TestResult {
    let ran = swell(&["--depth", "200", &fixture("depth.js")])?;
    expect(&ran, 5)?;
    assert!(
        ran.stderr.contains("frame_depth"),
        "stderr {:?}",
        ran.stderr
    );
    Ok(())
}

#[test]
fn a_parser_nesting_limit_exits_5() -> TestResult {
    let source = format!("{}1{}", "(".repeat(50), ")".repeat(50));
    let ran = swell(&["--nesting", "20", "-e", &source])?;
    expect(&ran, 5)?;
    assert!(
        ran.stderr.contains("parser_nesting"),
        "stderr {:?}",
        ran.stderr
    );
    Ok(())
}

#[test]
fn an_interrupt_after_the_timeout_exits_6() -> TestResult {
    let ran = swell(&["--steps", "max", "--timeout", "50", &fixture("loop.js")])?;
    expect(&ran, 6)?;
    assert!(
        ran.stderr.contains("interrupted"),
        "stderr {:?}",
        ran.stderr
    );
    Ok(())
}

#[test]
fn usage_errors_exit_64() -> TestResult {
    let loop_js = fixture("loop.js");
    let missing = fixture("no-such-file.js");
    let cases: [&[&str]; 8] = [
        &[],
        &["--steps"],
        &["--steps", "many", &loop_js],
        &["--memory", "-1", &loop_js],
        &["--no-such-flag", &loop_js],
        &[&loop_js, &loop_js],
        &["-e", "1", &loop_js],
        &[&missing],
    ];
    for args in cases {
        let ran = swell(args)?;
        expect(&ran, 64).map_err(|e| format!("{args:?}: {e}"))?;
        assert_eq!(ran.stdout, "", "{args:?}");
        assert!(
            ran.stderr.starts_with("swell: "),
            "{args:?}: stderr {:?}",
            ran.stderr
        );
    }
    Ok(())
}

#[test]
fn version_prints_the_package_version() -> TestResult {
    let ran = swell(&["--version"])?;
    expect(&ran, 0)?;
    assert_eq!(ran.stdout, format!("swell {}\n", env!("CARGO_PKG_VERSION")));
    Ok(())
}

#[test]
fn help_exits_0_and_names_every_flag() -> TestResult {
    let ran = swell(&["--help"])?;
    expect(&ran, 0)?;
    for flag in [
        "--steps",
        "--memory",
        "--depth",
        "--reentry",
        "--nesting",
        "--string-length",
        "--bigint-bits",
        "--timeout",
        "--version",
        "-e",
    ] {
        assert!(
            ran.stdout.contains(flag),
            "{flag} missing from {:?}",
            ran.stdout
        );
    }
    Ok(())
}

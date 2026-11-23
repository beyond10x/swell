//! The tests of `swell-test262 count --expected`: every printed line is compared with the expected
//! file in order. They count the fixture tree `tests/fixtures/tree/` with `sections.txt`;
//! `expected.txt` holds the 27 lines that prints, `expected-changed-count.txt` the same with one
//! count changed. A tree with one case more is a copy made under `CARGO_TARGET_TMPDIR`.

use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

/// What one run of the binary printed.
struct Printed {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Printed {
    fn has_error(&self, line: &str) -> bool {
        self.stderr.lines().any(|l| l == line)
    }
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn count_expected(tree: &str, expected: &str) -> std::io::Result<Printed> {
    let output = Command::new(env!("CARGO_BIN_EXE_swell-test262"))
        .args([
            "count",
            "--test262",
            tree,
            "--design",
            &fixture("design.md"),
            "--features",
            &fixture("features.txt"),
            "--sections",
            &fixture("sections.txt"),
            "--expected",
            expected,
        ])
        .output()?;
    Ok(Printed {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// Copies the directory `from` to `to`, which must not exist.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[test]
fn equal_lines_pass_the_comparison() -> TestResult {
    let expected = fixture("expected.txt");
    let printed = count_expected(&fixture("tree"), &expected)?;
    assert_eq!(
        printed.code,
        Some(0),
        "{}{}",
        printed.stdout,
        printed.stderr
    );
    assert_eq!(printed.stdout, fs::read_to_string(&expected)?);
    assert!(
        printed.has_error(&format!("test262: 27 lines equal {expected}")),
        "{}",
        printed.stderr
    );
    assert!(!printed.stderr.lines().any(|l| l.starts_with(['-', '+'])));
    Ok(())
}

#[test]
fn a_changed_count_in_the_expected_file_fails_the_run() -> TestResult {
    let expected = fixture("expected-changed-count.txt");
    let printed = count_expected(&fixture("tree"), &expected)?;
    assert_eq!(
        printed.code,
        Some(1),
        "{}{}",
        printed.stdout,
        printed.stderr
    );
    let differences: Vec<&str> = printed
        .stderr
        .lines()
        .filter(|l| l.starts_with(['-', '+']))
        .collect();
    assert_eq!(
        differences,
        [
            "-test262 total 23 5 1 16 5 11 0",
            "+test262 total 22 5 1 16 5 11 0"
        ],
        "{}",
        printed.stderr
    );
    assert!(
        printed.has_error(&format!("test262: 1 line(s) differ from {expected}")),
        "{}",
        printed.stderr
    );
    assert!(!printed.stderr.contains("lines equal"));
    Ok(())
}

#[test]
fn a_changed_count_in_the_printed_lines_fails_the_run() -> TestResult {
    let tree = Path::new(env!("CARGO_TARGET_TMPDIR")).join("expected-one-case-added");
    if tree.exists() {
        fs::remove_dir_all(&tree)?;
    }
    copy_tree(Path::new(&fixture("tree")), &tree)?;
    fs::write(
        tree.join("test/built-ins/Number/added.js"),
        "// Written for the tests of swell-test262.\n/*---\ndescription: One case more.\n---*/\n",
    )?;
    let expected = fixture("expected.txt");
    let printed = count_expected(&tree.to_string_lossy(), &expected)?;
    assert_eq!(
        printed.code,
        Some(1),
        "{}{}",
        printed.stdout,
        printed.stderr
    );
    let differences: Vec<&str> = printed
        .stderr
        .lines()
        .filter(|l| l.starts_with(['-', '+']))
        .collect();
    assert_eq!(
        differences,
        [
            "-test262 built-ins/Number 3 0 0 3 0 3 0",
            "+test262 built-ins/Number 4 0 0 4 1 3 0",
            "-test262 total 22 5 1 16 5 11 0",
            "+test262 total 23 5 1 17 6 11 0",
            "-test262-slice 04 1 2 0 0 1 0 1 0",
            "+test262-slice 04 2 4 0 0 2 1 1 0",
            "-test262-section sec-number-objects incomplete 3 0 0",
            "+test262-section sec-number-objects incomplete 4 1 0",
        ],
        "{}",
        printed.stderr
    );
    assert!(
        printed.has_error(&format!("test262: 4 line(s) differ from {expected}")),
        "{}",
        printed.stderr
    );
    Ok(())
}

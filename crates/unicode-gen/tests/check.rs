//! `swell-unicode-gen --check` against the committed table, and against a changed copy of it
//! under `CARGO_TARGET_TMPDIR`.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

const DATA: &str = "data/unicode/17.0.0/ucd/extracted/DerivedGeneralCategory.txt";
const TABLE: &str = "crates/unicode/src/tables/zs.rs";
const EQUAL: &str = "unicode-tables: 1 table(s) equal the generator output";

/// The root of the workspace: this crate is `crates/unicode-gen`.
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Runs the generator on `root` and returns its exit code and standard output.
fn generator(root: &Path, check: bool) -> Result<(Option<i32>, String), Box<dyn Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_swell-unicode-gen"));
    if check {
        command.arg("--check");
    }
    let output = command.arg("--root").arg(root).output()?;
    let stdout = String::from_utf8(output.stdout)?;
    Ok((output.status.code(), stdout))
}

#[test]
fn check_passes_on_the_committed_table() -> TestResult {
    let (code, stdout) = generator(&workspace(), true)?;
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), [EQUAL]);
    Ok(())
}

#[test]
fn check_fails_on_a_changed_table() -> TestResult {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("unicode-gen-changed-table");
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    for path in [DATA, TABLE] {
        let target = root.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(workspace().join(path), &target)?;
    }
    let (code, stdout) = generator(&root, true)?;
    assert_eq!(
        (code, stdout.trim_end()),
        (Some(0), EQUAL),
        "the unchanged copy"
    );

    let table = root.join(TABLE);
    let committed = fs::read_to_string(&table)?;
    let changed = committed.replacen("0x3000", "0x3001", 1);
    assert_ne!(changed, committed, "the table names U+3000");
    fs::write(&table, &changed)?;

    let (code, stdout) = generator(&root, true)?;
    assert_eq!(code, Some(1), "{stdout}");
    assert!(stdout.contains(TABLE), "{stdout}");
    assert_eq!(
        fs::read_to_string(&table)?,
        changed,
        "--check writes nothing"
    );

    let (code, _) = generator(&root, false)?;
    assert_eq!(code, Some(0));
    assert_eq!(
        fs::read_to_string(&table)?,
        committed,
        "a run without --check rewrites it"
    );
    let (code, stdout) = generator(&root, true)?;
    assert_eq!((code, stdout.trim_end()), (Some(0), EQUAL));
    Ok(())
}

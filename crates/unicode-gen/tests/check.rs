//! `swell-unicode-gen` against the committed tables, and against copies under
//! `CARGO_TARGET_TMPDIR` of `SOURCE`, the list `crates/unicode-gen/tables.txt` and every source
//! and table the list names. The count of tables and what a copy holds come from the list, so a
//! table added to it is checked here without a change of this file.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

/// The list of tables, relative to the root.
const LIST: &str = "crates/unicode-gen/tables.txt";
/// The vendored data, relative to the root; the sources of the list are relative to it.
const DATA: &str = "data/unicode/17.0.0";
/// The source of `zs.rs`, relative to `DATA`.
const GENERAL_CATEGORY: &str = "ucd/extracted/DerivedGeneralCategory.txt";
/// The table of `Zs`, relative to the root.
const ZS_TABLE: &str = "crates/unicode/src/tables/zs.rs";

/// The root of the workspace: this crate is `crates/unicode-gen`.
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One line of the list: the table, its renderer and its sources.
struct Entry {
    table: String,
    sources: Vec<String>,
}

/// The lines of the list under `root`.
fn list(root: &Path) -> Result<Vec<Entry>, Box<dyn Error>> {
    let text = fs::read_to_string(root.join(LIST))?;
    let mut entries = Vec::new();
    for line in text.lines() {
        let mut fields = line.split(' ');
        let table = fields.next().ok_or("a line with no table")?.to_owned();
        fields.next().ok_or("a line with no renderer")?;
        let sources: Vec<String> = fields.map(str::to_owned).collect();
        assert!(!sources.is_empty(), "{line}: a line with no source");
        entries.push(Entry { table, sources });
    }
    Ok(entries)
}

/// The SHA-256 of each path of `SOURCE` under `root`.
fn digests(root: &Path) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let text = fs::read_to_string(root.join(DATA).join("SOURCE"))?;
    let mut digests = BTreeMap::new();
    for line in text.lines() {
        let mut fields = line.split(' ');
        let digest = fields.next().ok_or("an empty line of SOURCE")?;
        let path = fields.next().ok_or("a line of SOURCE with no path")?;
        digests.insert(path.to_owned(), digest.to_owned());
    }
    Ok(digests)
}

/// The line `--check` prints when the `tables` tables of a list equal the generator output.
fn equal(tables: usize) -> String {
    format!("unicode-tables: {tables} table(s) equal the generator output")
}

/// A fresh copy under `CARGO_TARGET_TMPDIR` of `SOURCE`, the list and every source and table
/// the list names.
fn copy(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("unicode-gen-{name}"));
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    let mut paths = vec![format!("{DATA}/SOURCE"), LIST.to_owned()];
    for entry in list(&workspace())? {
        paths.extend(
            entry
                .sources
                .iter()
                .map(|source| format!("{DATA}/{source}")),
        );
        paths.push(entry.table);
    }
    for path in paths {
        let target = root.join(&path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(workspace().join(&path), &target)?;
    }
    Ok(root)
}

/// The exit code, standard output and standard error of one run of the generator.
struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Runs the generator on `root`, with `--check` when `check` is set.
fn generator(root: &Path, check: bool) -> Result<Run, Box<dyn Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_swell-unicode-gen"));
    if check {
        command.arg("--check");
    }
    let output = command.arg("--root").arg(root).output()?;
    Ok(Run {
        code: output.status.code(),
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
    })
}

/// Appends `line` to the list under `root`.
fn append_to_list(root: &Path, line: &str) -> TestResult {
    let path = root.join(LIST);
    let mut text = fs::read_to_string(&path)?;
    text.push_str(line);
    text.push('\n');
    fs::write(&path, text)?;
    Ok(())
}

/// Replaces line `number` (from 1) of `text` with `with`.
fn replace_line(text: &str, number: usize, with: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (index, line) in text.lines().enumerate() {
        out.push_str(if index + 1 == number { with } else { line });
        out.push('\n');
    }
    out
}

#[test]
fn check_passes_on_the_committed_table() -> TestResult {
    let tables = list(&workspace())?.len();
    let run = generator(&workspace(), true)?;
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stdout.lines().collect::<Vec<_>>(), [equal(tables)]);
    Ok(())
}

#[test]
fn check_fails_on_a_changed_table() -> TestResult {
    let root = copy("changed-table")?;
    let equal = equal(list(&root)?.len());
    let run = generator(&root, true)?;
    assert_eq!(
        (run.code, run.stdout.trim_end()),
        (Some(0), equal.as_str()),
        "the unchanged copy: {}",
        run.stderr
    );

    let table = root.join(ZS_TABLE);
    let committed = fs::read_to_string(&table)?;
    let changed = committed.replacen("0x3000", "0x3001", 1);
    assert_ne!(changed, committed, "the table names U+3000");
    fs::write(&table, &changed)?;

    let run = generator(&root, true)?;
    assert_eq!(run.code, Some(1), "{}", run.stdout);
    assert!(run.stdout.contains(ZS_TABLE), "{}", run.stdout);
    assert_eq!(
        fs::read_to_string(&table)?,
        changed,
        "--check writes nothing"
    );

    let run = generator(&root, false)?;
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(
        fs::read_to_string(&table)?,
        committed,
        "a run without --check rewrites it"
    );
    let run = generator(&root, true)?;
    assert_eq!((run.code, run.stdout.trim_end()), (Some(0), equal.as_str()));
    Ok(())
}

#[test]
fn a_table_added_to_a_copy_of_the_list_is_written_and_checked() -> TestResult {
    let root = copy("added-table")?;
    let tables = list(&root)?.len();
    let second = "crates/unicode/src/tables/zs_second.rs";
    append_to_list(&root, &format!("{second} zs {GENERAL_CATEGORY}"))?;

    let run = generator(&root, true)?;
    assert_eq!(run.code, Some(1), "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.contains(second), "{}", run.stdout);
    assert!(!root.join(second).exists(), "--check writes nothing");

    let run = generator(&root, false)?;
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(
        fs::read_to_string(root.join(second))?,
        fs::read_to_string(root.join(ZS_TABLE))?,
        "the renderer zs writes the same text for the same source"
    );

    let run = generator(&root, true)?;
    assert_eq!(
        (run.code, run.stdout.trim_end()),
        (Some(0), equal(tables + 1).as_str()),
        "{}",
        run.stderr
    );
    Ok(())
}

#[test]
fn each_table_names_the_sha256_of_each_source_as_source_gives_it() -> TestResult {
    let digests = digests(&workspace())?;
    let entries = list(&workspace())?;
    assert!(!entries.is_empty(), "the list names a table");
    for entry in entries {
        let text = fs::read_to_string(workspace().join(&entry.table))?;
        let named: Vec<String> = entry
            .sources
            .iter()
            .map(|source| format!("{DATA}/{source}"))
            .collect();
        let mut head = vec![format!(
            "// Generated by swell-unicode-gen from {} (Unicode 17.0.0); do not edit.",
            named.join(", ")
        )];
        for source in &entry.sources {
            let digest = digests
                .get(source)
                .ok_or_else(|| format!("SOURCE does not list {source}"))?;
            head.push(format!("// Source: {source} sha256 {digest}"));
        }
        let lines: Vec<&str> = text.lines().take(head.len()).collect();
        assert_eq!(lines, head, "the head of {}", entry.table);
    }
    Ok(())
}

#[test]
fn a_digest_line_that_differs_from_source_fails_the_check_naming_the_table() -> TestResult {
    let root = copy("changed-digest")?;
    let digest = digests(&root)?
        .remove(GENERAL_CATEGORY)
        .ok_or("SOURCE lists the source of zs.rs")?;
    let flipped = if digest.starts_with('0') { '1' } else { '0' };
    let other = format!("{flipped}{}", digest.get(1..).unwrap_or_default());
    assert_ne!(other, digest);

    // A data line the reader cannot parse: the check names the digest before it renders.
    let data = root.join(DATA).join(GENERAL_CATEGORY);
    let original = fs::read_to_string(&data)?;
    fs::write(&data, format!("{original}not a data line\n"))?;

    let table = root.join(ZS_TABLE);
    let committed = fs::read_to_string(&table)?;
    let changed = committed.replacen(&digest, &other, 1);
    assert_ne!(changed, committed, "zs.rs names the digest of its source");
    fs::write(&table, &changed)?;
    let run = generator(&root, true)?;
    assert_eq!(run.code, Some(1), "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout
            .lines()
            .any(|line| line.contains(ZS_TABLE) && line.contains("digest")),
        "{}",
        run.stdout
    );
    assert_eq!(
        fs::read_to_string(&table)?,
        changed,
        "--check writes nothing"
    );

    // The table as committed, and SOURCE giving another digest for its source.
    fs::write(&table, &committed)?;
    let source = root.join(DATA).join("SOURCE");
    let listed = fs::read_to_string(&source)?;
    fs::write(&source, listed.replacen(&digest, &other, 1))?;
    let run = generator(&root, true)?;
    assert_eq!(run.code, Some(1), "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout
            .lines()
            .any(|line| line.contains(ZS_TABLE) && line.contains("digest")),
        "{}",
        run.stdout
    );
    Ok(())
}

#[test]
fn a_source_that_source_does_not_list_is_an_error() -> TestResult {
    let root = copy("unlisted-source")?;
    let line = list(&root)?.len() + 1;
    append_to_list(
        &root,
        "crates/unicode/src/tables/zs_other.rs zs ucd/NotListed.txt",
    )?;
    let table = fs::read_to_string(root.join(ZS_TABLE))?;
    for check in [true, false] {
        let run = generator(&root, check)?;
        assert_eq!(run.code, Some(2), "{}{}", run.stdout, run.stderr);
        assert!(
            run.stderr.contains(&format!("{LIST}:{line}:"))
                && run.stderr.contains("ucd/NotListed.txt"),
            "{}",
            run.stderr
        );
    }
    assert_eq!(fs::read_to_string(root.join(ZS_TABLE))?, table);
    assert!(!root.join("crates/unicode/src/tables/zs_other.rs").exists());
    Ok(())
}

#[test]
fn a_renderer_the_generator_does_not_know_is_an_error() -> TestResult {
    let root = copy("unknown-renderer")?;
    let line = list(&root)?.len() + 1;
    append_to_list(
        &root,
        &format!("crates/unicode/src/tables/zs_other.rs no_such_renderer {GENERAL_CATEGORY}"),
    )?;
    for check in [true, false] {
        let run = generator(&root, check)?;
        assert_eq!(run.code, Some(2), "{}{}", run.stdout, run.stderr);
        assert!(
            run.stderr.contains(&format!("{LIST}:{line}:"))
                && run.stderr.contains("no_such_renderer"),
            "{}",
            run.stderr
        );
    }
    assert!(!root.join("crates/unicode/src/tables/zs_other.rs").exists());
    Ok(())
}

#[test]
fn a_line_it_cannot_parse_names_the_file_and_the_line() -> TestResult {
    let root = copy("unparsable-line")?;
    let data = root.join(DATA).join(GENERAL_CATEGORY);
    let original = fs::read_to_string(&data)?;
    let number = original
        .lines()
        .position(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .ok_or("a data line")?
        + 1;
    // No `;`, and a code point that is not hexadecimal on a line of a value no table asks for.
    for with in ["not a data line", "ZZZZ ; Cc # not a code point"] {
        fs::write(&data, replace_line(&original, number, with))?;
        let run = generator(&root, true)?;
        assert_eq!(run.code, Some(2), "{with}: {}{}", run.stdout, run.stderr);
        assert!(
            run.stderr
                .contains(&format!("{GENERAL_CATEGORY}:{number}:")),
            "{with}: {}",
            run.stderr
        );
    }
    Ok(())
}

#[test]
fn a_file_of_another_version_is_an_error() -> TestResult {
    let root = copy("other-version")?;
    let equal = equal(list(&root)?.len());
    let data = root.join(DATA).join(GENERAL_CATEGORY);
    let original = fs::read_to_string(&data)?;
    assert!(
        original.starts_with("# DerivedGeneralCategory-17.0.0.txt\n"),
        "the first line names the version"
    );
    let unversioned = replace_line(&original, 1, "# DerivedGeneralCategory.txt");
    let cases = [
        (
            "a first line of 16.0.0",
            replace_line(&original, 1, "# DerivedGeneralCategory-16.0.0.txt"),
            Some(2),
        ),
        (
            "a head of version 16.0",
            replace_line(&unversioned, 2, "# Version: 16.0"),
            Some(2),
        ),
        (
            "a head of version 17.0",
            replace_line(&unversioned, 2, "# Version: 17.0"),
            Some(0),
        ),
        ("a head with no version", unversioned.clone(), Some(2)),
    ];
    for (case, text, code) in cases {
        fs::write(&data, text)?;
        let run = generator(&root, true)?;
        assert_eq!(run.code, code, "{case}: {}{}", run.stdout, run.stderr);
        if code == Some(0) {
            assert_eq!(run.stdout.trim_end(), equal, "{case}");
        } else {
            assert!(
                run.stderr.contains(GENERAL_CATEGORY),
                "{case}: {}",
                run.stderr
            );
        }
    }
    Ok(())
}

#![forbid(unsafe_code)]
//! `swell-unicode-gen`, the tool that writes the tables of `swell-unicode` from the vendored
//! Unicode data (design, decision 8). It is tooling: it reads and writes files, and nothing that
//! ships depends on it.
//!
//! - `swell-unicode-gen [--root <dir>]`: writes every table of the list under `<dir>` (default
//!   `.`, the root of the repository).
//! - `swell-unicode-gen --check [--root <dir>]`: writes nothing. It first compares the digest
//!   lines of each committed table with `data/unicode/17.0.0/SOURCE` and names each table whose
//!   digest differs; then it renders every table in memory and names each one that differs from
//!   its committed file, or has none. It exits 1 when it named one, and otherwise prints
//!   `unicode-tables: <n> table(s) equal the generator output`, `<n>` the number of lines of the
//!   list, and exits 0.
//!
//! The list is `crates/unicode-gen/tables.txt`: one line per table, its fields separated by one
//! space: the path of the table under the root, the name of its renderer (`tables.rs`), and
//! each source as a path under `data/unicode/17.0.0/`. Each table starts with the head
//! `render::head` writes: the generator, the sources, and one line
//! `// Source: <path> sha256 <hex>` per source, the digest as `SOURCE` gives it.
//!
//! Either exits 2 when it cannot do its work: a wrong argument, a line of `SOURCE` or of the
//! list it cannot read, a source that `SOURCE` does not list, a renderer it does not know, a
//! data file it cannot read, a line it cannot parse, a data file of another Unicode version, or
//! a table that would be empty.

mod case;
mod normalize;
mod render;
mod tables;
mod ucd;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use render::Source;
use ucd::{DATA_DIR, Ucd};

/// The list of tables, relative to the root.
const LIST: &str = "crates/unicode-gen/tables.txt";

const USAGE: &str = "usage: swell-unicode-gen [--check] [--root <dir>]";

/// The command that writes the tables, named when a check fails.
const WRITE: &str = "run `cargo run -p swell-unicode-gen` to write it";

/// One line of the list.
struct Entry {
    /// The number of its line in the list, from 1.
    line: usize,
    /// The path of the table under the root.
    table: String,
    renderer: tables::Renderer,
    sources: Vec<Source>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("swell-unicode-gen: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let mut check = false;
    let mut root = PathBuf::from(".");
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--root" => root = PathBuf::from(args.next().ok_or(USAGE)?),
            _ => return Err(format!("unknown argument {arg}\n{USAGE}")),
        }
    }

    let digests = read_source(&root)?;
    let entries = read_list(&root, &digests)?;
    if check {
        return check_tables(&root, &entries);
    }
    let texts = render_all(&root, &entries)?;
    for (entry, text) in entries.iter().zip(&texts) {
        let target = root.join(&entry.table);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&target, text).map_err(|e| format!("{}: {e}", target.display()))?;
    }
    println!("unicode-tables: wrote {} table(s)", entries.len());
    Ok(ExitCode::SUCCESS)
}

/// The digests before the texts: a table whose digest lines differ from `SOURCE` is named
/// before any table is rendered.
fn check_tables(root: &Path, entries: &[Entry]) -> Result<ExitCode, String> {
    let mut differ = 0;
    for entry in entries {
        // A table with no committed file is named by the comparison of the texts.
        let Ok(committed) = fs::read_to_string(root.join(&entry.table)) else {
            continue;
        };
        let named: Vec<&str> = committed
            .lines()
            .skip(1)
            .take_while(|line| line.starts_with("// Source: "))
            .collect();
        let expected: Vec<String> = entry.sources.iter().map(render::source_line).collect();
        if named != expected {
            println!(
                "unicode-tables: {} names a digest of its sources that differs from \
                 {DATA_DIR}/SOURCE; {WRITE}",
                entry.table
            );
            differ += 1;
        }
    }
    if differ > 0 {
        println!(
            "unicode-tables: {differ} of {} table(s) name a digest that differs from {DATA_DIR}/SOURCE",
            entries.len()
        );
        return Ok(ExitCode::from(1));
    }

    let texts = render_all(root, entries)?;
    for (entry, text) in entries.iter().zip(&texts) {
        match fs::read_to_string(root.join(&entry.table)) {
            Ok(committed) if committed == *text => {}
            Ok(_) => {
                println!(
                    "unicode-tables: {} differs from the generator output; {WRITE}",
                    entry.table
                );
                differ += 1;
            }
            Err(e) => {
                println!(
                    "unicode-tables: {} has no committed file ({e}); {WRITE}",
                    entry.table
                );
                differ += 1;
            }
        }
    }
    if differ > 0 {
        println!(
            "unicode-tables: {differ} of {} table(s) differ from the generator output",
            entries.len()
        );
        return Ok(ExitCode::from(1));
    }
    println!(
        "unicode-tables: {} table(s) equal the generator output",
        entries.len()
    );
    Ok(ExitCode::SUCCESS)
}

/// The text of every table of the list, in the order of the list; the data is read once.
fn render_all(root: &Path, entries: &[Entry]) -> Result<Vec<String>, String> {
    let mut ucd = Ucd::new(root);
    entries
        .iter()
        .map(|entry| {
            (entry.renderer)(&mut ucd, &entry.sources).map_err(|e| {
                format!(
                    "{}:{}: {}: {e}",
                    root.join(LIST).display(),
                    entry.line,
                    entry.table
                )
            })
        })
        .collect()
}

/// The SHA-256 of each path of `data/unicode/17.0.0/SOURCE`, from its lines
/// `<sha256> <path> <address>`.
fn read_source(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let path = root.join(DATA_DIR).join("SOURCE");
    let name = path.display();
    let text = fs::read_to_string(&path).map_err(|e| format!("{name}: {e}"))?;
    let mut digests = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let fields: Vec<&str> = line.split(' ').collect();
        let [digest, source, _address] = fields.as_slice() else {
            return Err(format!(
                "{name}:{}: not <sha256> <path> <address>",
                index + 1
            ));
        };
        if digest.len() != 64 || !digest.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
            return Err(format!("{name}:{}: not a SHA-256: {digest:?}", index + 1));
        }
        digests.insert((*source).to_owned(), (*digest).to_owned());
    }
    Ok(digests)
}

/// The lines of the list, each source with the digest `SOURCE` gives it.
fn read_list(root: &Path, digests: &BTreeMap<String, String>) -> Result<Vec<Entry>, String> {
    let path = root.join(LIST);
    let name = path.display();
    let text = fs::read_to_string(&path).map_err(|e| format!("{name}: {e}"))?;
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let fields: Vec<&str> = line.split(' ').collect();
        let [table, renderer, sources @ ..] = fields.as_slice() else {
            return Err(format!(
                "{name}:{number}: not <table> <renderer> <source> ..."
            ));
        };
        if sources.is_empty() || fields.iter().any(|field| field.is_empty()) {
            return Err(format!(
                "{name}:{number}: not <table> <renderer> <source> ..., \
                 the fields separated by one space"
            ));
        }
        if Path::new(table)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(format!(
                "{name}:{number}: the table {table:?} is not a path under the root"
            ));
        }
        if !seen.insert(*table) {
            return Err(format!(
                "{name}:{number}: the table {table} is on an earlier line"
            ));
        }
        let renderer = tables::renderer(renderer).ok_or_else(|| {
            format!("{name}:{number}: the generator has no renderer {renderer:?}")
        })?;
        let sources = sources
            .iter()
            .map(|source| {
                let digest = digests.get(*source).ok_or_else(|| {
                    format!("{name}:{number}: {DATA_DIR}/SOURCE does not list the source {source}")
                })?;
                Ok(Source {
                    path: (*source).to_owned(),
                    digest: digest.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        entries.push(Entry {
            line: number,
            table: (*table).to_owned(),
            renderer,
            sources,
        });
    }
    if entries.is_empty() {
        return Err(format!("{name}: the list names no table"));
    }
    Ok(entries)
}

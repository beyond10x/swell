//! The language of slice 1, one small script per statement, expression, function and strict-mode
//! rule, in `tests/language/`. Each starts with the line `// <anchor>: <expected>`: the anchor of
//! the clause of ECMA-262 it exercises and the outcome it must have, `normal <value>` (the value
//! as `support::script::show` writes it) or `throw <name>` (the `name` of the thrown error, or
//! the thrown value as `show` writes it). A line `// strict-only` or `// sloppy-only` among the
//! leading comment lines limits the modes; without one the script runs as written and with a
//! `"use strict";` prologue (`support::script::run_both`). A script may start with a
//! `HashbangComment` line (`#sec-hashbang`) before that first line; its strict run then gets the
//! prologue after the hashbang line, where a hashbang may not stand. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

mod support;

use std::fs;
use std::path::PathBuf;

use support::script::{Outcome, run};

/// Which modes a script runs in.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Modes {
    Both,
    StrictOnly,
    SloppyOnly,
}

/// What a script's first lines say.
struct Expectation {
    anchor: String,
    outcome: Outcome,
    modes: Modes,
}

/// The expectation of `source`, or why it has none; a hashbang line before it is skipped.
fn expectation(source: &str) -> Result<Expectation, String> {
    let mut lines = source.lines().peekable();
    if lines.peek().is_some_and(|line| line.starts_with("#!")) {
        lines.next();
    }
    let first = lines.next().ok_or("the file is empty")?;
    let rest = first
        .strip_prefix("// ")
        .ok_or("the first line is not a `// ` comment")?;
    let (anchor, expected) = rest
        .split_once(": ")
        .ok_or("the first line has no `<anchor>: <expected>`")?;
    if !anchor.starts_with("sec-") {
        return Err(format!("{anchor:?} is not a clause anchor"));
    }
    let outcome = if let Some(value) = expected.strip_prefix("normal ") {
        Outcome::Normal(value.to_string())
    } else if let Some(name) = expected.strip_prefix("throw ") {
        Outcome::Throw(name.to_string())
    } else {
        return Err(format!(
            "{expected:?} is neither `normal <value>` nor `throw <name>`"
        ));
    };
    let mut modes = Modes::Both;
    for line in lines.take_while(|line| line.starts_with("//")) {
        match line.trim() {
            "// strict-only" => modes = Modes::StrictOnly,
            "// sloppy-only" => modes = Modes::SloppyOnly,
            _ => {}
        }
    }
    Ok(Expectation {
        anchor: anchor.to_string(),
        outcome,
        modes,
    })
}

/// `source` with a `"use strict";` prologue: after its hashbang line, if it has one.
fn with_use_strict(source: &str) -> String {
    match source.split_once('\n') {
        Some((hashbang, rest)) if hashbang.starts_with("#!") => {
            format!("{hashbang}\n\"use strict\";\n{rest}")
        }
        _ => format!("\"use strict\";\n{source}"),
    }
}

#[test]
fn language_scripts_answer_their_expected_outcome() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/language");
    let mut paths: Vec<PathBuf> = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "js"))
        .collect();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "{} holds no .js file",
        directory.display()
    );

    let mut failures = Vec::new();
    let mut runs = 0;
    for path in &paths {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let source = fs::read_to_string(path).unwrap();
        let expected = match expectation(&source) {
            Ok(expected) => expected,
            Err(why) => {
                failures.push(format!("{name}: no expectation line: {why}"));
                continue;
            }
        };
        let mut modes = Vec::new();
        if expected.modes != Modes::StrictOnly {
            modes.push(("sloppy", source.clone()));
        }
        if expected.modes != Modes::SloppyOnly {
            modes.push(("strict", with_use_strict(&source)));
        }
        for (mode, text) in modes {
            runs += 1;
            let found = run(support::host::limits(), &text);
            if found != expected.outcome {
                failures.push(format!(
                    "{name} ({mode}, {}): expected {:?}, found {found:?}",
                    expected.anchor, expected.outcome
                ));
            }
        }
    }
    println!("{} script(s), {runs} run(s)", paths.len());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

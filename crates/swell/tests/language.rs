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

/// The scripts of `tests/language/`, in byte order of their paths.
fn language_scripts() -> Vec<PathBuf> {
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
    paths
}

/// The scripts of part `part` of [`LANGUAGE_PARTS`]: every one whose index in
/// [`language_scripts`] leaves `part` when divided by the number of parts, so that a script
/// added to `tests/language/` lands in a part with nothing to edit.
fn language_part(part: usize) -> Vec<PathBuf> {
    language_scripts()
        .into_iter()
        .enumerate()
        .filter(|(i, _)| i % LANGUAGE_PARTS.len() == part)
        .map(|(_, path)| path)
        .collect()
}

/// The runs of a script: each mode it runs in, `sloppy` or `strict`, with the text it runs as
/// there.
type Runs = Vec<(&'static str, String)>;

/// The expectation of the script `source` and its runs, or why it has no expectation line.
fn runs_of(source: &str) -> Result<(Expectation, Runs), String> {
    let expected = expectation(source)?;
    let mut modes = Vec::new();
    if expected.modes != Modes::StrictOnly {
        modes.push(("sloppy", source.to_string()));
    }
    if expected.modes != Modes::SloppyOnly {
        modes.push(("strict", with_use_strict(source)));
    }
    Ok((expected, modes))
}

/// Every script of `paths` answers its expected outcome in each of its modes; a failure names
/// the script, the mode and the anchor.
fn answer_their_expected_outcome(paths: &[PathBuf]) {
    let mut failures = Vec::new();
    for path in paths {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let source = fs::read_to_string(path).unwrap();
        let (expected, modes) = match runs_of(&source) {
            Ok(runs) => runs,
            Err(why) => {
                failures.push(format!("{name}: no expectation line: {why}"));
                continue;
            }
        };
        for (mode, text) in modes {
            let found = run(support::host::limits(), &text);
            if found != expected.outcome {
                failures.push(format!(
                    "{name} ({mode}, {}): expected {:?}, found {found:?}",
                    expected.anchor, expected.outcome
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The tests of the scripts of `tests/language/`, one per part, and the list of their parts,
/// [`LANGUAGE_PARTS`], whose length is the number of parts. Each part runs in a process of its
/// own under cargo-nextest.
macro_rules! language_parts {
    ($($name:ident = $part:literal),* $(,)?) => {
        /// The parts of the scripts of `tests/language/`, one test each ([`language_part`]).
        const LANGUAGE_PARTS: &[usize] = &[$($part),*];
        $(
            #[test]
            fn $name() {
                let paths = language_part($part);
                assert!(!paths.is_empty(), "part {} holds no script", $part);
                answer_their_expected_outcome(&paths);
            }
        )*
    };
}

// The scripts of `tests/language/` are split over `language_scripts_answer_their_expected_outcome`,
// the first part, and `language_scripts_answer_their_expected_outcome_part_<n>`: a script lands in
// the part of its position in `language_scripts` ([`language_part`]), so a new script needs no
// edit here, and `the_parts_cover_every_script` shows each is in exactly one part. Thirteen
// parts: in the collect mode the scripts took 38 s together (166 scripts, at a load of 43 on 20
// cores), 0.23 s a script; 250 such scripts take 57 s, under 5 s a part in 13. A prime, so
// that two scripts share a part only when they stand 13 apart. Three scripts take 10 to 12 s
// each in that mode (both runs: `function-recursion.js`, `try-completion-finally-abrupt.js`,
// `try-completion-try-and-catch-abrupt.js`); the part of each takes that long, which no number
// of parts shortens.
language_parts!(
    language_scripts_answer_their_expected_outcome = 0,
    language_scripts_answer_their_expected_outcome_part_2 = 1,
    language_scripts_answer_their_expected_outcome_part_3 = 2,
    language_scripts_answer_their_expected_outcome_part_4 = 3,
    language_scripts_answer_their_expected_outcome_part_5 = 4,
    language_scripts_answer_their_expected_outcome_part_6 = 5,
    language_scripts_answer_their_expected_outcome_part_7 = 6,
    language_scripts_answer_their_expected_outcome_part_8 = 7,
    language_scripts_answer_their_expected_outcome_part_9 = 8,
    language_scripts_answer_their_expected_outcome_part_10 = 9,
    language_scripts_answer_their_expected_outcome_part_11 = 10,
    language_scripts_answer_their_expected_outcome_part_12 = 11,
    language_scripts_answer_their_expected_outcome_part_13 = 12,
);

/// The parts together hold every script of `tests/language/` exactly once, and are numbered
/// from 0 without a gap. This test alone prints the total, `<n> script(s), <m> run(s)`: every
/// script and every mode it runs in, counted from the expectation lines.
#[test]
fn the_parts_cover_every_script() {
    let mut parts = LANGUAGE_PARTS.to_vec();
    parts.sort_unstable();
    assert_eq!(parts, (0..LANGUAGE_PARTS.len()).collect::<Vec<_>>());
    let scripts = language_scripts();
    let mut held: Vec<PathBuf> = LANGUAGE_PARTS
        .iter()
        .flat_map(|&part| language_part(part))
        .collect();
    held.sort();
    assert_eq!(held, scripts);
    let runs: usize = scripts
        .iter()
        .map(|path| runs_of(&fs::read_to_string(path).unwrap()).map_or(0, |(_, modes)| modes.len()))
        .sum();
    println!("{} script(s), {runs} run(s)", scripts.len());
}

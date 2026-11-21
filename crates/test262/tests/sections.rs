//! The tests of `sections.txt` and the section block of `swell-test262 count`, run against the
//! fixture tree `tests/fixtures/tree/` with `sections.txt` (four sections, one with an omission)
//! and `sections-no-case.txt` (a path that matches no case). Of the cases of those sections only
//! two of `sec-fixture-language` pass (`1 + 1;` and `if (true) {}`), so the judgement `complete`
//! is shown on counts.

use std::error::Error;
use std::process::Command;

use swell_test262::sections::{self, Judgement, Omission, SectionCount};

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

    fn section_lines(&self) -> Vec<&str> {
        self.stdout
            .lines()
            .filter(|l| l.starts_with("test262-section "))
            .collect()
    }
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn count_with_sections(sections: &str) -> std::io::Result<Printed> {
    let output = Command::new(env!("CARGO_BIN_EXE_swell-test262"))
        .args([
            "count",
            "--test262",
            &fixture("tree"),
            "--design",
            &fixture("design.md"),
            "--features",
            &fixture("features.txt"),
            "--sections",
            &fixture(sections),
        ])
        .output()?;
    Ok(Printed {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[test]
fn sections_file_reads_sections_paths_and_omissions() -> TestResult {
    let text = std::fs::read_to_string(fixture("sections.txt"))?;
    let (read, problems) = sections::read(&text);
    assert_eq!(problems, Vec::<String>::new());
    let list = read.list();
    let anchors: Vec<&str> = list.iter().map(|s| s.anchor.as_str()).collect();
    assert_eq!(
        anchors,
        [
            "sec-object-objects",
            "sec-number-objects",
            "sec-atomics-object",
            "sec-fixture-language"
        ]
    );
    let paths = |i: usize| -> Vec<&str> {
        list.get(i)
            .map(|s| s.paths.iter().map(|p| p.path.as_str()).collect())
            .unwrap_or_default()
    };
    assert_eq!(paths(0), ["test/built-ins/Object/"]);
    assert_eq!(
        paths(3),
        ["test/language/expressions/", "test/language/statements/if/"]
    );
    assert_eq!(
        list.get(2).map(|s| s.omissions.clone()),
        Some(vec![Omission {
            reason: "concurrent-agents".to_string(),
            decision: "open-item-B".to_string(),
        }])
    );
    assert!(list.first().is_some_and(|s| s.omissions.is_empty()));

    // Every problem names its line.
    let (_, problems) = sections::read(
        "section a\npath a test/x/\npath b test/y/\nomit c why decision-1\nsection a\n\
         path a\nsection d\n",
    );
    assert_eq!(
        problems,
        [
            "sections.txt: line 3: path of the undeclared section b",
            "sections.txt: line 4: omit of the undeclared section c",
            "sections.txt: line 5: section a declared twice",
            "sections.txt: line 6: neither section <anchor>, path <anchor> <path> nor omit \
             <anchor> <reason> <decision>: path a",
            "sections.txt: line 7: section d has no path",
        ]
    );
    let (_, problems) = sections::read("");
    assert_eq!(problems, ["sections.txt: lists no section"]);

    let printed = count_with_sections("sections.txt")?;
    assert_eq!(
        printed.code,
        Some(0),
        "{}{}",
        printed.stdout,
        printed.stderr
    );
    assert_eq!(
        printed.section_lines(),
        [
            "test262-section sec-object-objects incomplete 3 0 0",
            "test262-section sec-number-objects incomplete 3 0 0",
            "test262-section sec-atomics-object incomplete 3 0 1",
            "test262-section sec-fixture-language incomplete 4 2 0",
        ]
    );
    // The section block comes after the test262-errors line.
    assert_eq!(
        printed
            .stdout
            .lines()
            .skip_while(|l| !l.starts_with("test262-errors "))
            .skip(1)
            .collect::<Vec<_>>(),
        printed.section_lines()
    );
    Ok(())
}

#[test]
fn a_section_is_incomplete_while_a_case_of_its_paths_does_not_pass() -> TestResult {
    assert_eq!(sections::judge(3, 0, 0), Judgement::Incomplete);
    assert_eq!(sections::judge(3, 2, 0), Judgement::Incomplete);
    // A section with no case has nothing that passes.
    assert_eq!(sections::judge(0, 0, 0), Judgement::Incomplete);
    let printed = count_with_sections("sections.txt")?;
    assert_eq!(
        printed.code,
        Some(0),
        "{}{}",
        printed.stdout,
        printed.stderr
    );
    // built-ins/Object has one excluded case and two run cases, which fail: none is a pass.
    assert!(printed.has("test262 built-ins/Object 3 1 0 2 0 2 0"));
    assert!(
        printed.has("test262-section sec-object-objects incomplete 3 0 0"),
        "{}",
        printed.stdout
    );
    // built-ins/Atomics has an excluded, a not-applicable and a failed case: none is a pass.
    assert!(printed.has("test262 built-ins/Atomics 3 1 1 1 0 1 0"));
    assert!(printed.has("test262-section sec-atomics-object incomplete 3 0 1"));
    Ok(())
}

#[test]
fn a_section_with_an_omission_is_incomplete_when_every_case_passes() {
    assert_eq!(sections::judge(3, 3, 1), Judgement::Incomplete);
    assert_eq!(sections::judge(389, 389, 2), Judgement::Incomplete);
    let count = SectionCount {
        anchor: "sec-atomics-object".to_string(),
        cases: 3,
        passed: 3,
        omissions: 1,
    };
    assert_eq!(count.judgement(), Judgement::Incomplete);
    assert_eq!(
        count.line(),
        "test262-section sec-atomics-object incomplete 3 3 1"
    );
}

#[test]
fn a_section_whose_cases_all_pass_without_omissions_is_complete() {
    assert_eq!(sections::judge(3, 3, 0), Judgement::Complete);
    assert_eq!(Judgement::Complete.to_string(), "complete");
    assert_eq!(Judgement::Incomplete.to_string(), "incomplete");
    let count = SectionCount {
        anchor: "sec-proxy-objects".to_string(),
        cases: 311,
        passed: 311,
        omissions: 0,
    };
    assert_eq!(count.judgement(), Judgement::Complete);
    assert_eq!(
        count.line(),
        "test262-section sec-proxy-objects complete 311 311 0"
    );
}

#[test]
fn a_section_path_that_matches_no_case_fails_the_run() -> TestResult {
    let printed = count_with_sections("sections-no-case.txt")?;
    assert_eq!(
        printed.code,
        Some(1),
        "{}{}",
        printed.stdout,
        printed.stderr
    );
    assert!(
        printed.has("sections.txt: line 4: test/built-ins/NoSuch/ matches no case"),
        "{}",
        printed.stdout
    );
    assert!(printed.has("test262-section sec-object-objects incomplete 3 0 0"));
    let matched = count_with_sections("sections.txt")?;
    assert_eq!(
        matched.code,
        Some(0),
        "{}{}",
        matched.stdout,
        matched.stderr
    );
    assert!(!matched.stdout.contains("matches no case"));
    Ok(())
}

//! `conformance/test262/sections.txt` (design, decision 11): sections of ECMA-262, the test262
//! paths that test each and its deliberate omissions, and the judgement `complete` or
//! `incomplete` of each (`AGENTS.md`, "Goals").
//!
//! The file has no comment and no blank line. Each line is one of
//!
//! - `section <anchor>`: declares a section by its anchor in ECMA-262 (`sec-object-objects`);
//! - `path <anchor> <test262 path>`: a case whose path from the checkout starts with
//!   `<test262 path>` tests the section;
//! - `omit <anchor> <reason> <decision>`: a deliberate omission of the section, for this reason,
//!   by this decision.
//!
//! A `path` or `omit` of a section not declared on an earlier line, a section declared twice, a
//! section with no `path`, a line of another shape and a file with no section are problems; so is
//! a path that matches no case. Each names its line.
//!
//! A section is `complete` only when it has a case, every case of its paths passed and it has no
//! omission; an excluded or not-applicable case is not a pass. Its line, after the
//! `test262-errors` line and in file order, is
//! `test262-section <anchor> <complete|incomplete> <cases> <passed> <omissions>`.

use std::collections::BTreeSet;
use std::fmt;

use crate::count::{self, Loaded};
use crate::features::Listed;
use crate::slices::Rules;

/// A deliberate omission of a section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Omission {
    /// Why, as one word (`concurrent-agents`).
    pub reason: String,
    /// Which decision takes it (`open-item-B`).
    pub decision: String,
}

/// A test262 path of a section, with the line of the file that names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionPath {
    /// The line number, from 1.
    pub line: usize,
    /// The path from the checkout (`test/built-ins/Object/`).
    pub path: String,
}

/// A section of ECMA-262 as the file lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// Its anchor (`sec-object-objects`).
    pub anchor: String,
    /// The test262 paths that test it.
    pub paths: Vec<SectionPath>,
    /// Its deliberate omissions.
    pub omissions: Vec<Omission>,
}

/// The sections of the file, in file order.
#[derive(Clone, Debug, Default)]
pub struct Sections {
    list: Vec<Section>,
}

/// `complete` or `incomplete`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Judgement {
    /// Every case of the section's paths passed, and it has no omission.
    Complete,
    /// Anything else.
    Incomplete,
}

impl fmt::Display for Judgement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Judgement::Complete => "complete",
            Judgement::Incomplete => "incomplete",
        })
    }
}

/// The judgement of a section with `cases` cases, `passed` of them passed, and `omissions`
/// omissions. A section with no case has nothing that passes and is `incomplete`.
pub fn judge(cases: u64, passed: u64, omissions: u64) -> Judgement {
    if cases > 0 && passed == cases && omissions == 0 {
        Judgement::Complete
    } else {
        Judgement::Incomplete
    }
}

/// The counts of one section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionCount {
    /// Its anchor.
    pub anchor: String,
    /// The cases its paths match.
    pub cases: u64,
    /// Those that passed.
    pub passed: u64,
    /// Its omissions.
    pub omissions: u64,
}

impl SectionCount {
    /// Its judgement.
    pub fn judgement(&self) -> Judgement {
        judge(self.cases, self.passed, self.omissions)
    }

    /// Its line, `test262-section <anchor> <complete|incomplete> <cases> <passed> <omissions>`.
    pub fn line(&self) -> String {
        format!(
            "test262-section {} {} {} {} {}",
            self.anchor,
            self.judgement(),
            self.cases,
            self.passed,
            self.omissions
        )
    }
}

/// Reads `sections.txt`, and every problem with its lines.
pub fn read(text: &str) -> (Sections, Vec<String>) {
    let mut list: Vec<Section> = Vec::new();
    let mut declared_on: Vec<usize> = Vec::new();
    let mut problems = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.as_slice() {
            ["section", anchor] => {
                if list.iter().any(|s| s.anchor == *anchor) {
                    problems.push(format!(
                        "sections.txt: line {number}: section {anchor} declared twice"
                    ));
                } else {
                    list.push(Section {
                        anchor: (*anchor).to_string(),
                        paths: Vec::new(),
                        omissions: Vec::new(),
                    });
                    declared_on.push(number);
                }
            }
            [kind @ ("path" | "omit"), anchor, ..]
                if (*kind == "path" && words.len() == 3) || words.len() == 4 =>
            {
                let Some(section) = list.iter_mut().find(|s| s.anchor == *anchor) else {
                    problems.push(format!(
                        "sections.txt: line {number}: {kind} of the undeclared section {anchor}"
                    ));
                    continue;
                };
                match words.as_slice() {
                    [_, _, path] => section.paths.push(SectionPath {
                        line: number,
                        path: (*path).to_string(),
                    }),
                    [_, _, reason, decision] => section.omissions.push(Omission {
                        reason: (*reason).to_string(),
                        decision: (*decision).to_string(),
                    }),
                    _ => {}
                }
            }
            _ => problems.push(format!(
                "sections.txt: line {number}: neither section <anchor>, path <anchor> <path> nor \
                 omit <anchor> <reason> <decision>: {line}"
            )),
        }
    }
    for (section, number) in list.iter().zip(&declared_on) {
        if section.paths.is_empty() {
            problems.push(format!(
                "sections.txt: line {number}: section {} has no path",
                section.anchor
            ));
        }
    }
    if list.is_empty() {
        problems.push("sections.txt: lists no section".to_string());
    }
    (Sections { list }, problems)
}

impl Sections {
    /// The sections, in file order.
    pub fn list(&self) -> &[Section] {
        &self.list
    }

    /// The counts of every section, in file order, over the cases of a checkout counted as
    /// [`count::tally`] counts them; and a problem for each path that matches no case.
    pub fn count(
        &self,
        loaded: &[Loaded],
        rules: &Rules,
        listed: &Listed,
    ) -> (Vec<SectionCount>, Vec<String>) {
        let mut counts = Vec::with_capacity(self.list.len());
        let mut problems = Vec::new();
        for section in &self.list {
            let mut matched = BTreeSet::new();
            for path in &section.paths {
                let hits: Vec<usize> = loaded
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| l.case.path.starts_with(path.path.as_str()))
                    .map(|(i, _)| i)
                    .collect();
                if hits.is_empty() {
                    problems.push(format!(
                        "sections.txt: line {}: {} matches no case",
                        path.line, path.path
                    ));
                }
                matched.extend(hits);
            }
            let cases: Vec<Loaded> = matched
                .iter()
                .filter_map(|&i| loaded.get(i).cloned())
                .collect();
            let total = count::tally(&cases, rules, listed).total;
            counts.push(SectionCount {
                anchor: section.anchor.clone(),
                cases: total.cases,
                passed: total.passed,
                omissions: u64::try_from(section.omissions.len()).unwrap_or(u64::MAX),
            });
        }
        (counts, problems)
    }
}

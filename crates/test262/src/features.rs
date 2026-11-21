//! `conformance/test262/features.txt` (design, decision 1): every test262 feature, `run <name>`
//! or `exclude <name> <reason> <decision>`, one per line, and its checks against the features the
//! checkout declares and the `exclude` lines of the `swell-slices` block.
//!
//! The declared features are the names of the checkout's own `features.txt`: the first word of
//! each line that is not blank and does not start with `#`
//! (`grep -vE '^\s*(#|$)' features.txt | awk '{print $1}'`).

use std::collections::{BTreeMap, BTreeSet};

use crate::slices::Rules;

/// What `features.txt` says of one feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// Cases with the feature run.
    Run,
    /// Cases with the feature are excluded, for this reason, by this decision of the design.
    Exclude {
        /// Why, as one word (`stage3`).
        reason: String,
        /// Which decision of the design takes it (`decision-1`).
        decision: String,
    },
}

/// The lines of `features.txt`.
#[derive(Clone, Debug, Default)]
pub struct Listed {
    entries: BTreeMap<String, Entry>,
}

impl Listed {
    /// Whether the file lists `name`.
    pub fn lists(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Whether the file excludes `name`.
    pub fn excludes(&self, name: &str) -> bool {
        matches!(self.entries.get(name), Some(Entry::Exclude { .. }))
    }

    /// The entry of `name`, if listed.
    pub fn entry(&self, name: &str) -> Option<&Entry> {
        self.entries.get(name)
    }
}

/// Reads `features.txt`, and every problem with its lines (a line that is neither
/// `run <name>` nor `exclude <name> <reason> <decision>`, or a name listed twice).
pub fn read_listed(text: &str) -> (Listed, Vec<String>) {
    let mut listed = Listed::default();
    let mut problems = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let entry = match words.as_slice() {
            ["run", name] => Some((*name, Entry::Run)),
            ["exclude", name, reason, decision] => Some((
                *name,
                Entry::Exclude {
                    reason: (*reason).to_string(),
                    decision: (*decision).to_string(),
                },
            )),
            _ => None,
        };
        match entry {
            Some((name, entry)) => {
                if listed.entries.insert(name.to_string(), entry).is_some() {
                    problems.push(format!(
                        "features.txt: line {}: {name} is listed twice",
                        number + 1
                    ));
                }
            }
            None => problems.push(format!(
                "features.txt: line {}: neither run <name> nor exclude <name> <reason> <decision>: {line}",
                number + 1
            )),
        }
    }
    if listed.entries.is_empty() {
        problems.push("features.txt: lists no feature".to_string());
    }
    (listed, problems)
}

/// The features the checkout's `features.txt` declares.
pub fn declared(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            let word = line.split_whitespace().next()?;
            (!word.starts_with('#')).then(|| word.to_string())
        })
        .collect()
}

/// The problems of `listed` against the declared features and the block's `exclude` lines: a
/// declared feature not listed, a listed feature not declared, an excluded feature listed as
/// `run`.
pub fn check(listed: &Listed, declared: &BTreeSet<String>, rules: &Rules) -> Vec<String> {
    let mut problems = Vec::new();
    for name in declared {
        if !listed.lists(name) {
            problems.push(format!(
                "features.txt: {name} declared by test262, not listed"
            ));
        }
    }
    for name in listed.entries.keys() {
        if !declared.contains(name) {
            problems.push(format!(
                "features.txt: {name} listed, not declared by test262"
            ));
        }
    }
    for (name, _) in rules.excluded_features() {
        if listed.entry(name) == Some(&Entry::Run) {
            problems.push(format!(
                "features.txt: {name} is run, but the swell-slices block excludes it"
            ));
        }
    }
    problems
}

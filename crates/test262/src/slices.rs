//! The `swell-slices` block of `docs/design/engine.md` (section 12, "Delivery order"), read at run
//! time, and the delivery slice of a case.
//!
//! The block is every line between a line that is exactly ```` ```swell-slices ```` and the next
//! line that is exactly ```` ``` ````, as the design's own command extracts it
//! (`awk '$0 == "```swell-slices" {f = 1; next} /^```$/ {f = 0} f'`). Each line is three words:
//!
//! - `dir <prefix> <slice>`, `feature <name> <slice>`, `flag <name> <slice>`: the slice of a case
//!   is the highest among its longest matching `dir` prefix, its features and its flags
//!   (`slices.awk` of the design);
//! - `exclude <feature> <reason>`: a case with that feature is in the slice `x-<reason>`;
//! - `exclude-slice <slice> <reason>`, `exclude-include <harness file> <reason>`: every case of
//!   that slice, or whose `includes:` names that file, is excluded (decision 1, "Two exclusions no
//!   feature name carries"). `slices.awk` ignores these two kinds; the slice of a case does not
//!   change with them.
//!
//! A blank line is skipped. A line of another kind, or of another shape, is a problem the run
//! reports and fails on.

use std::collections::BTreeMap;
use std::fmt;

/// The delivery slice of a case.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Slice {
    /// A numbered slice, printed with two digits as `slices.awk` prints it (`01`).
    Numbered(u32),
    /// The slice of a case with a feature an `exclude` line names, printed `x-<reason>`.
    Excluded(String),
}

impl fmt::Display for Slice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Slice::Numbered(n) => write!(f, "{n:02}"),
            Slice::Excluded(reason) => write!(f, "x-{reason}"),
        }
    }
}

/// The rules of a `swell-slices` block.
#[derive(Clone, Debug, Default)]
pub struct Rules {
    dirs: BTreeMap<String, u32>,
    features: BTreeMap<String, u32>,
    flags: BTreeMap<String, u32>,
    excludes: BTreeMap<String, String>,
    exclude_slices: BTreeMap<u32, String>,
    exclude_includes: BTreeMap<String, String>,
}

/// The lines of the `swell-slices` block (or blocks) of `design`.
pub fn block(design: &str) -> Vec<&str> {
    let mut inside = false;
    let mut lines = Vec::new();
    for line in design.lines() {
        if line == "```swell-slices" {
            inside = true;
            continue;
        }
        if line == "```" {
            inside = false;
        }
        if inside {
            lines.push(line);
        }
    }
    lines
}

/// Reads the rules of the `swell-slices` block of `design`, and every problem with its lines. A
/// design with no rule line is a problem too.
pub fn read(design: &str) -> (Rules, Vec<String>) {
    let mut rules = Rules::default();
    let mut problems = Vec::new();
    let mut count = 0usize;
    for line in block(design) {
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(&kind) = words.first() else {
            continue;
        };
        count += 1;
        let known = matches!(
            kind,
            "dir" | "feature" | "flag" | "exclude" | "exclude-slice" | "exclude-include"
        );
        if !known {
            problems.push(format!("swell-slices: unknown kind {kind}: {line}"));
            continue;
        }
        let (Some(&first), Some(&second), None) = (words.get(1), words.get(2), words.get(3)) else {
            problems.push(format!("swell-slices: not three words: {line}"));
            continue;
        };
        let number = |text: &str| text.parse::<u32>().ok();
        match kind {
            "exclude" => {
                rules.excludes.insert(first.to_string(), second.to_string());
            }
            "exclude-include" => {
                rules
                    .exclude_includes
                    .insert(first.to_string(), second.to_string());
            }
            "exclude-slice" => match number(first) {
                Some(slice) => {
                    rules.exclude_slices.insert(slice, second.to_string());
                }
                None => problems.push(format!("swell-slices: {first} is not a slice: {line}")),
            },
            _ => match number(second) {
                Some(slice) => {
                    let map = match kind {
                        "dir" => &mut rules.dirs,
                        "feature" => &mut rules.features,
                        _ => &mut rules.flags,
                    };
                    map.insert(first.to_string(), slice);
                }
                None => problems.push(format!("swell-slices: {second} is not a slice: {line}")),
            },
        }
    }
    if count == 0 {
        problems.push("swell-slices: the design has no rule line".to_string());
    }
    (rules, problems)
}

impl Rules {
    /// The slice of the case at `path` (from `test/`) with these features and flags: the highest
    /// among its longest matching `dir` prefix, its features and its flags, or `x-<reason>` when
    /// a feature has an `exclude` line (the last such feature names the reason, as in
    /// `slices.awk`). A case nothing matches is in slice `00`.
    pub fn slice_of<'a>(
        &self,
        path: &str,
        features: impl Iterator<Item = &'a str>,
        flags: impl Iterator<Item = &'a str>,
    ) -> Slice {
        let mut best: Option<usize> = None;
        let mut slice = 0;
        for (prefix, &n) in &self.dirs {
            if path.starts_with(prefix.as_str()) && best.is_none_or(|b| prefix.len() > b) {
                best = Some(prefix.len());
                slice = n;
            }
        }
        let mut excluded = None;
        for feature in features {
            if let Some(reason) = self.excludes.get(feature) {
                excluded = Some(reason);
            } else if let Some(&n) = self.features.get(feature) {
                slice = slice.max(n);
            }
        }
        for flag in flags {
            if let Some(&n) = self.flags.get(flag) {
                slice = slice.max(n);
            }
        }
        match excluded {
            Some(reason) => Slice::Excluded(reason.clone()),
            None => Slice::Numbered(slice),
        }
    }

    /// The reason of the `exclude-slice` line naming `slice`, if there is one.
    pub fn excludes_slice(&self, slice: &Slice) -> Option<&str> {
        match slice {
            Slice::Numbered(n) => self.exclude_slices.get(n).map(String::as_str),
            Slice::Excluded(_) => None,
        }
    }

    /// Whether one of `includes` is a file an `exclude-include` line names.
    pub fn excludes_include<'a>(&self, mut includes: impl Iterator<Item = &'a str>) -> bool {
        includes.any(|file| self.exclude_includes.contains_key(file))
    }

    /// The features the `exclude` lines name, with their reasons.
    pub fn excluded_features(&self) -> impl Iterator<Item = (&str, &str)> {
        self.excludes
            .iter()
            .map(|(name, reason)| (name.as_str(), reason.as_str()))
    }
}

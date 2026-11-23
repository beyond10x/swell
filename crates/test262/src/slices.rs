//! The `swell-slices` block of `docs/design/engine.md` (section 12, "Delivery order"), read at run
//! time, and the delivery slice of a case.
//!
//! The block is every line between a line that is exactly ```` ```swell-slices ```` and the next
//! line that is exactly ```` ``` ````, as the design's own command extracts it
//! (`awk '$0 == "```swell-slices" {f = 1; next} /^```$/ {f = 0} f'`). Each line is three words:
//!
//! - `order delivery <slice>,<slice>,...`: the numbered slices in the order they are delivered,
//!   which is not the order of their numbers (slice 6 before 5). The block has exactly this one
//!   line, names no slice twice, and names every slice of a `dir`, `feature`, `flag` and
//!   `exclude-slice` line; otherwise the run fails. It never falls back to the numbers;
//! - `dir <prefix> <slice>`, `feature <name> <slice>`, `flag <name> <slice>`: the slice of a case
//!   is the one delivered last among its longest matching `dir` prefix, its features and its
//!   flags (`slices.awk` of the design);
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

/// The delivery slice of a case. It has no order: the numbers are not the order of delivery,
/// which the block's order line gives.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// The place of each slice the order line names: 1 for the slice delivered first.
    places: BTreeMap<u32, usize>,
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
    let mut orders = 0usize;
    // Each slice a rule line names, with the line, to be found in the order after the block.
    let mut named: Vec<(u32, &str)> = Vec::new();
    for line in block(design) {
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(&kind) = words.first() else {
            continue;
        };
        count += 1;
        let known = matches!(
            kind,
            "dir" | "feature" | "flag" | "exclude" | "exclude-slice" | "exclude-include" | "order"
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
            "order" => {
                orders += 1;
                if orders > 1 {
                    problems.push(format!("swell-slices: a second order line: {line}"));
                    continue;
                }
                if first != "delivery" {
                    problems.push(format!(
                        "swell-slices: an order line is \"order delivery <slice>,<slice>,...\": {line}"
                    ));
                }
                for item in second.split(',') {
                    match number(item) {
                        Some(slice) if rules.places.contains_key(&slice) => problems.push(format!(
                            "swell-slices: the order names slice {slice} twice: {line}"
                        )),
                        Some(slice) => {
                            let place = rules.places.len() + 1;
                            rules.places.insert(slice, place);
                        }
                        None => {
                            problems.push(format!("swell-slices: {item} is not a slice: {line}"));
                        }
                    }
                }
            }
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
                    named.push((slice, line));
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
                    named.push((slice, line));
                }
                None => problems.push(format!("swell-slices: {second} is not a slice: {line}")),
            },
        }
    }
    if count == 0 {
        problems.push("swell-slices: the design has no rule line".to_string());
    } else if orders == 0 {
        problems.push(
            "swell-slices: the block has no order line (order delivery <slice>,<slice>,...)"
                .to_string(),
        );
    } else {
        for (slice, line) in named {
            if !rules.places.contains_key(&slice) {
                problems.push(format!(
                    "swell-slices: the order does not name slice {slice}: {line}"
                ));
            }
        }
    }
    (rules, problems)
}

impl Rules {
    /// The slice of the case at `path` (from `test/`) with these features and flags: the slice
    /// delivered last (the latest place in the order line) among its longest matching `dir`
    /// prefix, its features and its flags, or `x-<reason>` when a feature has an `exclude` line
    /// (the last such feature names the reason, as in `slices.awk`). A case nothing matches is
    /// in slice `00`. A slice the order does not name never replaces another; `read` reports it.
    pub fn slice_of<'a>(
        &self,
        path: &str,
        features: impl Iterator<Item = &'a str>,
        flags: impl Iterator<Item = &'a str>,
    ) -> Slice {
        let mut best: Option<usize> = None;
        let mut slice: Option<u32> = None;
        for (prefix, &n) in &self.dirs {
            if path.starts_with(prefix.as_str()) && best.is_none_or(|b| prefix.len() > b) {
                best = Some(prefix.len());
                slice = Some(n);
            }
        }
        let later = |n: u32, current: Option<u32>| match current {
            None => true,
            Some(c) => self.places.get(&n) > self.places.get(&c),
        };
        let mut excluded = None;
        for feature in features {
            if let Some(reason) = self.excludes.get(feature) {
                excluded = Some(reason);
            } else if let Some(&n) = self.features.get(feature)
                && later(n, slice)
            {
                slice = Some(n);
            }
        }
        for flag in flags {
            if let Some(&n) = self.flags.get(flag)
                && later(n, slice)
            {
                slice = Some(n);
            }
        }
        match excluded {
            Some(reason) => Slice::Excluded(reason.clone()),
            None => Slice::Numbered(slice.unwrap_or(0)),
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

#[cfg(test)]
mod tests {
    use super::{Rules, read};

    /// A design holding one `swell-slices` block of `lines`.
    fn design(lines: &[&str]) -> String {
        format!("# A design\n\n```swell-slices\n{}\n```\n", lines.join("\n"))
    }

    fn rules(lines: &[&str]) -> Rules {
        let (rules, problems) = read(&design(lines));
        assert_eq!(problems, Vec::<String>::new());
        rules
    }

    fn problems(lines: &[&str]) -> Vec<String> {
        read(&design(lines)).1
    }

    fn slice(rules: &Rules, path: &str, features: &[&str], flags: &[&str]) -> String {
        rules
            .slice_of(path, features.iter().copied(), flags.iter().copied())
            .to_string()
    }

    const RULES: [&str; 5] = [
        "dir test/language/ 2",
        "dir test/built-ins/ 3",
        "dir test/built-ins/RegExp/ 5",
        "feature Symbol 6",
        "feature exponentiation 4",
    ];

    #[test]
    fn a_case_counts_in_the_slice_delivered_last_among_its_lines() {
        let mut lines = RULES.to_vec();
        lines.push("order delivery 1,2,3,6,5,4");
        let rules = rules(&lines);
        // A directory of slice 5 and a feature of slice 6: 5 is delivered after 6.
        assert_eq!(
            slice(&rules, "test/built-ins/RegExp/x.js", &["Symbol"], &[]),
            "05"
        );
        // A directory of slice 3 and a feature of slice 6: 6 is delivered after 3.
        assert_eq!(
            slice(&rules, "test/built-ins/Object/x.js", &["Symbol"], &[]),
            "06"
        );
        // Two features, 6 and 4: 4 is delivered last.
        assert_eq!(
            slice(
                &rules,
                "test/language/x.js",
                &["exponentiation", "Symbol"],
                &[]
            ),
            "04"
        );
        assert_eq!(slice(&rules, "test/language/x.js", &[], &[]), "02");
        assert_eq!(slice(&rules, "test/annexB/x.js", &[], &[]), "00");
        assert_eq!(slice(&rules, "test/annexB/x.js", &["Symbol"], &[]), "06");
    }

    #[test]
    fn a_slice_of_a_lower_number_delivered_later_wins() {
        let rules = rules(&[
            "dir test/built-ins/ 3",
            "dir test/built-ins/Date/ 4",
            "dir test/built-ins/Map/ 9",
            "feature Promise 10",
            "feature class 8",
            "flag async 10",
            "order delivery 1,2,3,6,5,7,10,4,9,8",
        ]);
        assert_eq!(
            slice(&rules, "test/built-ins/Date/x.js", &["Promise"], &[]),
            "04"
        );
        assert_eq!(
            slice(&rules, "test/built-ins/Date/x.js", &[], &["async"]),
            "04"
        );
        assert_eq!(
            slice(&rules, "test/built-ins/Map/x.js", &["Promise"], &[]),
            "09"
        );
        assert_eq!(
            slice(&rules, "test/built-ins/Map/x.js", &["class"], &["async"]),
            "08"
        );
        assert_eq!(
            slice(&rules, "test/built-ins/Object/x.js", &["Promise"], &[]),
            "10"
        );
    }

    #[test]
    fn a_block_without_an_order_is_a_problem() {
        assert_eq!(
            problems(&RULES),
            ["swell-slices: the block has no order line (order delivery <slice>,<slice>,...)"]
        );
    }

    #[test]
    fn a_slice_named_twice_in_the_order_is_a_problem() {
        let line = "order delivery 1,2,3,6,5,4,6";
        let mut lines = RULES.to_vec();
        lines.push(line);
        assert_eq!(
            problems(&lines),
            [format!(
                "swell-slices: the order names slice 6 twice: {line}"
            )]
        );
    }

    #[test]
    fn a_slice_of_a_rule_line_the_order_does_not_name_is_a_problem() {
        let mut lines = RULES.to_vec();
        lines.push("flag async 10");
        lines.push("exclude-slice 21 intl402");
        lines.push("order delivery 1,2,3,6,4");
        assert_eq!(
            problems(&lines),
            [
                "swell-slices: the order does not name slice 5: dir test/built-ins/RegExp/ 5",
                "swell-slices: the order does not name slice 10: flag async 10",
                "swell-slices: the order does not name slice 21: exclude-slice 21 intl402",
            ]
        );
    }

    #[test]
    fn an_order_line_of_another_shape_is_a_problem() {
        let mut lines = RULES.to_vec();
        lines.push("order delivery 1,2,3,6,5,4");
        lines.push("order delivery 1,2,3,6,5,4");
        assert_eq!(
            problems(&lines),
            ["swell-slices: a second order line: order delivery 1,2,3,6,5,4"]
        );
        let mut lines = RULES.to_vec();
        lines.push("order delivery 1,2,3,six,5,4");
        assert_eq!(
            problems(&lines),
            [
                "swell-slices: six is not a slice: order delivery 1,2,3,six,5,4",
                "swell-slices: the order does not name slice 6: feature Symbol 6",
            ]
        );
        let mut lines = RULES.to_vec();
        lines.push("order numbers 1,2,3,6,5,4");
        assert_eq!(
            problems(&lines),
            [
                "swell-slices: an order line is \"order delivery <slice>,<slice>,...\": order numbers 1,2,3,6,5,4"
            ]
        );
    }
}

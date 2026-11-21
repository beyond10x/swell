//! `conformance/test262/expected.txt` (design, decision 11): every line `swell-test262 count`
//! prints, so that the gate fails on any difference. A lost pass fails, and a new pass is taken in
//! by committing the new expected file.
//!
//! The printed lines are compared with the lines of the file in order, line by line: the line of
//! the file at position `i` with the printed line at position `i`. A position where they differ,
//! or where only one of the two has a line, is a difference.

/// The outcome of a comparison.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Comparison {
    /// The number of printed lines.
    pub printed: usize,
    /// The number of positions that differ.
    pub differing: usize,
    /// For each position that differs, in order, `-<expected line>` (when the file has one) and
    /// `+<printed line>` (when one was printed).
    pub report: Vec<String>,
}

impl Comparison {
    /// Whether every line is equal.
    pub fn equal(&self) -> bool {
        self.differing == 0
    }
}

/// Compares the printed lines with the text of the expected file. CR LF and LF end a line of the
/// file alike.
pub fn compare(expected: &str, printed: &[String]) -> Comparison {
    let mut wanted = expected.lines();
    let mut got = printed.iter();
    let mut comparison = Comparison {
        printed: printed.len(),
        ..Comparison::default()
    };
    loop {
        match (wanted.next(), got.next()) {
            (None, None) => break,
            (Some(w), Some(g)) if w == g => {}
            (w, g) => {
                comparison.differing += 1;
                if let Some(w) = w {
                    comparison.report.push(format!("-{w}"));
                }
                if let Some(g) = g {
                    comparison.report.push(format!("+{g}"));
                }
            }
        }
    }
    comparison
}

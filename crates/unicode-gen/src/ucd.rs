//! The readers of the vendored Unicode data under `data/unicode/17.0.0/`.
//!
//! Each data file is read once per run ([`Ucd::text`]), and its version is held by one rule,
//! [`version`], that every reader calls.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// The Unicode version of the data the tables are written from.
pub(crate) const UNICODE_VERSION: &str = "17.0.0";

/// The version the line `# Version: <major>.<minor>` of a file's head must give.
const UNICODE_VERSION_LINE: &str = "17.0";

/// The directory of the vendored data, relative to the root.
pub(crate) const DATA_DIR: &str = "data/unicode/17.0.0";

/// The head of a data file, as [`version`] reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Head {
    /// A comment head that names the pinned version: a first line `# <name>-17.0.0.txt`, or a
    /// first line `# <name>.txt` and a line `# Version: 17.0` in the comment block at the head.
    Pinned,
    /// No comment line at the head: the first line is a data line (`UnicodeData.txt`). The
    /// reader does not check its version: `SOURCE` gives it (its address under
    /// `Public/17.0.0/`), and `unicode-data-check` holds the file against its digest there.
    Bare,
}

/// The version rule of every vendored data file. `name` names the file in an error.
///
/// - A first line `# <name>-<version>.txt` gives the version, which must be `17.0.0`.
/// - A first line `# <name>.txt` with no version gives it by the line
///   `# Version: <major>.<minor>` of the comment block at the head, which must be `17.0`; a head
///   with neither line is an error.
/// - A first line that is not a comment is [`Head::Bare`].
pub(crate) fn version(name: &str, text: &str) -> Result<Head, String> {
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default();
    let Some(title) = first.strip_prefix('#') else {
        return Ok(Head::Bare);
    };
    let file = title
        .trim()
        .strip_suffix(".txt")
        .ok_or_else(|| format!("{name}:1: the first line names no file: {first:?}"))?;
    if let Some((_, named)) = file.rsplit_once('-')
        && named.starts_with(|c: char| c.is_ascii_digit())
        && named.chars().all(|c| c.is_ascii_digit() || c == '.')
    {
        if named == UNICODE_VERSION {
            return Ok(Head::Pinned);
        }
        return Err(format!(
            "{name}:1: the data of Unicode {named}, not {UNICODE_VERSION}"
        ));
    }
    for (index, line) in text.lines().enumerate().skip(1) {
        if !line.starts_with('#') {
            break;
        }
        if let Some(named) = line.strip_prefix("# Version:") {
            let named = named.trim();
            if named == UNICODE_VERSION_LINE {
                return Ok(Head::Pinned);
            }
            return Err(format!(
                "{name}:{}: the data of Unicode {named}, not {UNICODE_VERSION_LINE}",
                index + 1
            ));
        }
    }
    Err(format!(
        "{name}: the head names no version: neither a first line \
         `# <name>-{UNICODE_VERSION}.txt` nor a line `# Version: {UNICODE_VERSION_LINE}`"
    ))
}

/// The vendored data under a root, each file read once.
pub(crate) struct Ucd {
    dir: PathBuf,
    texts: BTreeMap<String, String>,
}

impl Ucd {
    /// The data under `<root>/data/unicode/17.0.0`.
    pub(crate) fn new(root: &std::path::Path) -> Self {
        Self {
            dir: root.join(DATA_DIR),
            texts: BTreeMap::new(),
        }
    }

    /// The name of `source` in an error: its path under the root.
    fn name(&self, source: &str) -> String {
        self.dir.join(source).display().to_string()
    }

    /// The text of `source` (a path under `data/unicode/17.0.0/`) and its head, read on the
    /// first call.
    fn text(&mut self, source: &str) -> Result<(&str, Head), String> {
        let name = self.name(source);
        if !self.texts.contains_key(source) {
            let text =
                fs::read_to_string(self.dir.join(source)).map_err(|e| format!("{name}: {e}"))?;
            self.texts.insert(source.to_owned(), text);
        }
        let text = self
            .texts
            .get(source)
            .map(String::as_str)
            .unwrap_or_default();
        Ok((text, version(&name, text)?))
    }

    /// The code points of each of `values` in `source`, a file of lines
    /// `<code point or range> ; <value> [; ...] # ...` (the derived files, `PropList.txt`,
    /// `Scripts.txt`, `ScriptExtensions.txt`, `emoji-data.txt`), each merged into sorted,
    /// disjoint inclusive ranges, in the order of `values`. `<value>` is the second field.
    /// Every data line is parsed, whichever value it has; a value with no code point is an
    /// error.
    pub(crate) fn ranges(
        &mut self,
        source: &str,
        values: &[&str],
    ) -> Result<Vec<Vec<(u32, u32)>>, String> {
        let name = self.name(source);
        let (text, head) = self.text(source)?;
        if head == Head::Bare {
            return Err(format!(
                "{name}:1: not a file of ranges: the file has no comment head"
            ));
        }
        let mut found: Vec<Vec<(u32, u32)>> = vec![Vec::new(); values.len()];
        for (index, line) in text.lines().enumerate() {
            let data = line.split('#').next().unwrap_or_default().trim();
            if data.is_empty() {
                continue;
            }
            let bad = || {
                format!(
                    "{name}:{}: not <code point or range> ; <value>: {line:?}",
                    index + 1
                )
            };
            let mut fields = data.split(';');
            let points = fields.next().unwrap_or_default().trim();
            let value = fields.next().ok_or_else(bad)?.trim();
            let (first, last) = points.split_once("..").unwrap_or((points, points));
            let first = code_point(first).ok_or_else(bad)?;
            let last = code_point(last).ok_or_else(bad)?;
            if first > last {
                return Err(bad());
            }
            if let Some(slot) = values
                .iter()
                .position(|wanted| *wanted == value)
                .and_then(|at| found.get_mut(at))
            {
                slot.push((first, last));
            }
        }
        let mut merged = Vec::with_capacity(values.len());
        for (value, ranges) in values.iter().zip(found) {
            if ranges.is_empty() {
                return Err(format!("{name}: no code point of {value}"));
            }
            merged.push(merge(ranges));
        }
        Ok(merged)
    }
}

/// A code point written as 4 to 6 hexadecimal digits, at most `10FFFF`.
fn code_point(text: &str) -> Option<u32> {
    if !(4..=6).contains(&text.len()) || !text.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(text, 16)
        .ok()
        .filter(|point| *point <= 0x10FFFF)
}

/// `ranges` sorted, with overlapping and adjacent ranges joined.
fn merge(mut ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    ranges.sort_unstable();
    let mut merged: Vec<(u32, u32)> = Vec::with_capacity(ranges.len());
    for (first, last) in ranges {
        match merged.last_mut() {
            Some(previous) if first <= previous.1.saturating_add(1) => {
                previous.1 = previous.1.max(last);
            }
            _ => merged.push((first, last)),
        }
    }
    merged
}

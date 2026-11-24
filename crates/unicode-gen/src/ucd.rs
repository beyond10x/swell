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

/// The vendored data under a root, each file read once and parsed once: a file of ranges for
/// every value it has ([`Ucd::ranges`]), a file of the form of `UnicodeData.txt` into the
/// fields of its records ([`Ucd::unicode_data`]), whichever table asks first.
pub(crate) struct Ucd {
    dir: PathBuf,
    texts: BTreeMap<String, String>,
    /// The merged ranges of each value of each file of ranges read.
    values: BTreeMap<String, BTreeMap<String, Vec<(u32, u32)>>>,
    /// The records of each file of the form of `UnicodeData.txt` read.
    records: BTreeMap<String, Vec<Spans>>,
}

impl Ucd {
    /// The data under `<root>/data/unicode/17.0.0`.
    pub(crate) fn new(root: &std::path::Path) -> Self {
        Self {
            dir: root.join(DATA_DIR),
            texts: BTreeMap::new(),
            values: BTreeMap::new(),
            records: BTreeMap::new(),
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
    /// Every data line is parsed, whichever value it has, on the first call for `source`, and
    /// the ranges of every value are kept for the next; a value with no code point is an error.
    pub(crate) fn ranges(
        &mut self,
        source: &str,
        values: &[&str],
    ) -> Result<Vec<Vec<(u32, u32)>>, String> {
        let name = self.name(source);
        if !self.values.contains_key(source) {
            let (text, head) = self.text(source)?;
            if head == Head::Bare {
                return Err(format!(
                    "{name}:1: not a file of ranges: the file has no comment head"
                ));
            }
            let parsed = parse_ranges(&name, text)?;
            self.values.insert(source.to_owned(), parsed);
        }
        let parsed = self.values.get(source);
        values
            .iter()
            .map(|value| {
                parsed
                    .and_then(|parsed| parsed.get(*value))
                    .cloned()
                    .ok_or_else(|| format!("{name}: no code point of {value}"))
            })
            .collect()
    }
}

/// The ranges of every value of `text`, a file of ranges named `name` ([`Ucd::ranges`]), each
/// merged.
fn parse_ranges(name: &str, text: &str) -> Result<BTreeMap<String, Vec<(u32, u32)>>, String> {
    let mut found: BTreeMap<String, Vec<(u32, u32)>> = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let data = line.split('#').next().unwrap_or_default().trim_ascii();
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
        let points = fields.next().unwrap_or_default().trim_ascii();
        let value = fields.next().ok_or_else(bad)?.trim_ascii();
        let (first, last) = match points.split_once('.') {
            Some((first, rest)) => (first, rest.strip_prefix('.').ok_or_else(bad)?),
            None => (points, points),
        };
        let first = code_point(first).ok_or_else(bad)?;
        let last = code_point(last).ok_or_else(bad)?;
        if first > last {
            return Err(bad());
        }
        match found.get_mut(value) {
            Some(ranges) => ranges.push((first, last)),
            None => {
                found.insert(value.to_owned(), vec![(first, last)]);
            }
        }
    }
    Ok(found
        .into_iter()
        .map(|(value, ranges)| (value, merge(ranges)))
        .collect())
}

/// The number of fields of a line of `UnicodeData.txt`, separated by `;`.
pub(crate) const UNICODE_DATA_FIELDS: usize = 15;

/// The byte ranges of the fields of a line in the text of its file.
type FieldSpans = [(usize, usize); UNICODE_DATA_FIELDS];

/// One record of a file of the form of `UnicodeData.txt` as [`Ucd`] keeps it: the fields as
/// byte ranges of the file's text.
struct Spans {
    line: usize,
    first: u32,
    last: u32,
    fields: FieldSpans,
}

/// One record of `UnicodeData.txt`: the code points `first` to `last` (one line, or the pair of
/// lines `<..., First>` and `<..., Last>`) and the fields of its line, the first line of a pair.
pub(crate) struct DataRecord<'a> {
    /// The number of its (first) line in the file, from 1.
    pub(crate) line: usize,
    pub(crate) first: u32,
    pub(crate) last: u32,
    text: &'a str,
    fields: &'a FieldSpans,
}

impl<'a> DataRecord<'a> {
    /// Field `number`, from 0 (the code point); an empty field is `""`.
    pub(crate) fn field(&self, number: usize) -> &'a str {
        self.fields
            .get(number)
            .and_then(|&(start, end)| self.text.get(start..end))
            .unwrap_or_default()
    }
}

impl Ucd {
    /// The records of `source`, a file of the form of `UnicodeData.txt`: no comment head
    /// ([`Head::Bare`]), and on each line 15 fields separated by `;`, the first a code point.
    /// A line whose name (field 1) is `<X, First>` and the next line, `<X, Last>`, are one
    /// record of the range between their code points. The records are in ascending order of
    /// code points with none twice; any other line is an error that names its line. The file
    /// is parsed on the first call for `source`, and its records are kept for the next.
    pub(crate) fn unicode_data(
        &mut self,
        source: &str,
    ) -> Result<impl Iterator<Item = DataRecord<'_>>, String> {
        let name = self.name(source);
        if !self.records.contains_key(source) {
            let (text, head) = self.text(source)?;
            if head != Head::Bare {
                return Err(format!(
                    "{name}:1: not a file of the form of UnicodeData.txt: it has a comment head"
                ));
            }
            let parsed = parse_unicode_data(&name, text)?;
            self.records.insert(source.to_owned(), parsed);
        }
        let text = self
            .texts
            .get(source)
            .map(String::as_str)
            .unwrap_or_default();
        let records = self
            .records
            .get(source)
            .map(Vec::as_slice)
            .unwrap_or_default();
        Ok(records.iter().map(move |spans| DataRecord {
            line: spans.line,
            first: spans.first,
            last: spans.last,
            text,
            fields: &spans.fields,
        }))
    }
}

/// The records of `text`, a file named `name` of the form of `UnicodeData.txt`
/// ([`Ucd::unicode_data`]). Each line is cut at its `;` in one pass over its bytes.
fn parse_unicode_data(name: &str, text: &str) -> Result<Vec<Spans>, String> {
    let mut records: Vec<Spans> = Vec::new();
    // The line number, the code point, the fields and the range name of a `<X, First>` line.
    let mut open: Option<(usize, u32, FieldSpans, &str)> = None;
    let mut start = 0;
    let mut number = 0;
    while start < text.len() {
        number += 1;
        let rest = text.get(start..).unwrap_or_default();
        let length = rest.find('\n').unwrap_or(rest.len());
        let line = rest.get(..length).unwrap_or_default();
        let line_start = start;
        start += length + 1;
        let bad = |what: &str| format!("{name}:{number}: {what}: {line:?}");
        let mut fields: FieldSpans = [(0, 0); UNICODE_DATA_FIELDS];
        let mut count = 0;
        // The end of the line's text searched so far, from the line's start.
        let mut scanned = 0;
        // The code point and the name, the longest field, are found by a search for `;`; the
        // short fields after them byte by byte.
        for slot in fields.iter_mut().take(2) {
            let Some(at) = line.get(scanned..).and_then(|rest| rest.find(';')) else {
                return Err(bad("not 15 fields"));
            };
            *slot = (line_start + scanned, line_start + scanned + at);
            count += 1;
            scanned += at + 1;
        }
        let mut field_start = line_start + scanned;
        let tail = line.as_bytes().get(scanned..).unwrap_or_default();
        for (offset, &byte) in tail.iter().enumerate() {
            if byte == b';' {
                let at = line_start + scanned + offset;
                let Some(slot) = fields.get_mut(count) else {
                    return Err(bad("not 15 fields"));
                };
                *slot = (field_start, at);
                count += 1;
                field_start = at + 1;
            }
        }
        match fields.get_mut(count) {
            Some(slot) if count + 1 == UNICODE_DATA_FIELDS => {
                *slot = (field_start, line_start + line.len());
            }
            _ => return Err(bad("not 15 fields")),
        }
        let field = |number: usize| {
            fields
                .get(number)
                .and_then(|&(from, to)| text.get(from..to))
                .unwrap_or_default()
        };
        let point = code_point(field(0)).ok_or_else(|| bad("not a code point"))?;
        let label = field(1);
        let (first_line, first, record_fields) = match open.take() {
            Some((first_line, first, first_fields, range)) => {
                if label.strip_prefix(range) != Some(", Last>") || point <= first {
                    return Err(bad("not the line <..., Last> of the line before"));
                }
                (first_line, first, first_fields)
            }
            None => {
                if let Some(range) = label.strip_suffix(", First>") {
                    open = Some((number, point, fields, range));
                    continue;
                }
                if label.ends_with(", Last>") {
                    return Err(bad("a line <..., Last> after no line <..., First>"));
                }
                (number, point, fields)
            }
        };
        if records
            .last()
            .is_some_and(|previous| previous.last >= first)
        {
            return Err(bad("a code point not above the one of the line before"));
        }
        records.push(Spans {
            line: first_line,
            first,
            last: point,
            fields: record_fields,
        });
    }
    if let Some((number, ..)) = open {
        return Err(format!(
            "{name}:{number}: a line <..., First> with no line <..., Last> after it"
        ));
    }
    if records.is_empty() {
        return Err(format!("{name}: no record"));
    }
    Ok(records)
}

impl Ucd {
    /// The data lines of `source`, a file with a comment head of the pinned version: each line
    /// with its number from 1, without its comment (from `#`), trimmed; empty lines are left out.
    pub(crate) fn data_lines(&mut self, source: &str) -> Result<Vec<(usize, &str)>, String> {
        let name = self.name(source);
        let (text, head) = self.text(source)?;
        if head == Head::Bare {
            return Err(format!("{name}:1: the file has no comment head"));
        }
        Ok(text
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line.split('#').next().unwrap_or_default().trim()))
            .filter(|(_, data)| !data.is_empty())
            .collect())
    }

    /// The name of `source` in an error: its path under the root.
    pub(crate) fn source_name(&self, source: &str) -> String {
        self.name(source)
    }

    /// The elements of `source`, a file of sequences (`emoji/emoji-sequences.txt`,
    /// `emoji/emoji-zwj-sequences.txt`) of lines `<code points> ; <property> ; <name> # ...`:
    /// each data line with its number from 1, the property of its second field and its
    /// element, a range of single code points (`<first>..<last>`, or one code point) or a
    /// sequence of two code points or more separated by spaces. Any other line is an error that
    /// names it.
    pub(crate) fn sequences(&mut self, source: &str) -> Result<Vec<SequenceLine>, String> {
        let name = self.name(source);
        let lines = self.data_lines(source)?;
        let mut read = Vec::with_capacity(lines.len());
        for (number, data) in lines {
            let bad = |what: &str| format!("{name}:{number}: {what}: {data:?}");
            let mut fields = data.split(';');
            let points = fields.next().unwrap_or_default().trim_ascii();
            let property = fields
                .next()
                .map(str::trim_ascii)
                .filter(|property| !property.is_empty())
                .ok_or_else(|| bad("not <code points> ; <property>"))?;
            let element = if let Some((first, last)) = points.split_once("..") {
                let first = code_point(first).ok_or_else(|| bad("not a code point"))?;
                let last = code_point(last).ok_or_else(|| bad("not a code point"))?;
                if first > last {
                    return Err(bad("a range whose first code point passes its last"));
                }
                Element::Range(first, last)
            } else {
                let sequence = points
                    .split_ascii_whitespace()
                    .map(code_point)
                    .collect::<Option<Vec<u32>>>()
                    .ok_or_else(|| bad("not a code point"))?;
                match sequence.as_slice() {
                    [] => return Err(bad("no code point")),
                    &[point] => Element::Range(point, point),
                    _ => Element::Sequence(sequence),
                }
            };
            read.push(SequenceLine {
                line: number,
                property: property.to_owned(),
                element,
            });
        }
        Ok(read)
    }
}

/// One data line of a file of sequences ([`Ucd::sequences`]).
pub(crate) struct SequenceLine {
    /// The number of its line in the file, from 1.
    pub(crate) line: usize,
    pub(crate) property: String,
    pub(crate) element: Element,
}

/// What a line of a file of sequences gives its property.
pub(crate) enum Element {
    /// The single code points from the first to the last, both included.
    Range(u32, u32),
    /// One sequence of two code points or more.
    Sequence(Vec<u32>),
}

/// A code point written as 4 to 6 hexadecimal digits, at most `10FFFF`.
pub(crate) fn code_point(text: &str) -> Option<u32> {
    if !(4..=6).contains(&text.len()) {
        return None;
    }
    let mut point: u32 = 0;
    for &digit in text.as_bytes() {
        let value = match digit {
            b'0'..=b'9' => digit - b'0',
            b'A'..=b'F' => digit - b'A' + 10,
            b'a'..=b'f' => digit - b'a' + 10,
            _ => return None,
        };
        point = point * 16 + u32::from(value);
    }
    Some(point).filter(|point| *point <= 0x10FFFF)
}

/// `ranges` sorted, with overlapping and adjacent ranges joined.
pub(crate) fn merge(mut ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
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

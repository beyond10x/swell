//! The reader of a test262 case's front matter, the `/*---` ... `---*/` block (INTERPRETING.md,
//! "Metadata").
//!
//! It reads the block exactly as `meta.awk` of `docs/design/engine.md` ("How the counts were made")
//! does when run with `awk -v RS='\r\n|\r|\n'`, so that `swell-test262 meta` prints that
//! program's output byte for byte: `flags`, `features` and `includes` as an inline list `[a, b]`
//! or a block list of `- a` lines, and the `phase` and `type` of `negative`. Beyond what
//! `meta.awk` does it names the four ways a block can be unreadable ([`Problem`]); the value it
//! read is kept either way.
//!
//! A line ends at CR LF, CR or LF, three of the `LineTerminatorSequence`s of ECMA-262
//! (`#sec-line-terminators`; LS and PS are not taken, as the story decides). `meta.awk` with its
//! default record separator splits at LF
//! only; test262's `line-terminator-normalisation-CR.js` ends its lines in CR alone, and that
//! reading lost its `includes:` (story `test262-counting-runner`, "Correction after the unit's
//! first report").

use std::fmt;
use std::io::{self, BufRead};

/// The fields of a front matter block, each as `meta.awk` prints it: lists joined by commas.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrontMatter {
    /// `flags`, joined by commas.
    pub flags: String,
    /// `features`, joined by commas.
    pub features: String,
    /// The `phase` of `negative`.
    pub negative_phase: String,
    /// The `type` of `negative`.
    pub negative_type: String,
    /// `includes`, joined by commas.
    pub includes: String,
}

impl FrontMatter {
    /// The flags, one by one; an empty name is skipped.
    pub fn flag_list(&self) -> impl Iterator<Item = &str> {
        items(&self.flags)
    }

    /// The features, one by one; an empty name is skipped.
    pub fn feature_list(&self) -> impl Iterator<Item = &str> {
        items(&self.features)
    }

    /// The included harness files, one by one; an empty name is skipped.
    pub fn include_list(&self) -> impl Iterator<Item = &str> {
        items(&self.includes)
    }

    /// Whether `name` is one of the flags.
    pub fn has_flag(&self, name: &str) -> bool {
        self.flag_list().any(|f| f == name)
    }

    /// The line `meta.awk` prints for the case at `path`, without its newline.
    pub fn tsv_line(&self, path: &str) -> String {
        format!(
            "{path}\t{}\t{}\t{}\t{}\t{}",
            self.flags, self.features, self.negative_phase, self.negative_type, self.includes
        )
    }
}

fn items(list: &str) -> impl Iterator<Item = &str> {
    list.split(',').filter(|s| !s.is_empty())
}

/// Why a front matter block cannot be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// No line holds `/*---`.
    NoBlock,
    /// The block has no line holding `---*/`.
    Unterminated,
    /// An inline list of the named key has a `[` and no `]`.
    UnclosedList(&'static str),
    /// `negative` lacks its `phase` or its `type`.
    IncompleteNegative,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Problem::NoBlock => f.write_str("no /*--- block"),
            Problem::Unterminated => f.write_str("a /*--- block without ---*/"),
            Problem::UnclosedList(key) => write!(f, "an inline {key} list without ]"),
            Problem::IncompleteNegative => f.write_str("a negative without both phase and type"),
        }
    }
}

/// What reading one case gave: the fields as far as they were read, and the first problem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Read {
    /// The fields.
    pub meta: FrontMatter,
    /// The first reason the block cannot be read, if any.
    pub problem: Option<Problem>,
}

/// Which list a block list's `- item` lines go to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ListKey {
    None,
    Flags,
    Features,
    Includes,
    Negative,
}

/// The state of `meta.awk` over the lines of one file.
struct Reader {
    meta: FrontMatter,
    seen_block: bool,
    in_block: bool,
    done: bool,
    list: ListKey,
    saw_negative: bool,
    problem: Option<Problem>,
}

impl Reader {
    fn new() -> Self {
        Reader {
            meta: FrontMatter::default(),
            seen_block: false,
            in_block: false,
            done: false,
            list: ListKey::None,
            saw_negative: false,
            problem: None,
        }
    }

    fn note(&mut self, problem: Problem) {
        if self.problem.is_none() {
            self.problem = Some(problem);
        }
    }

    /// Reads one line, without its newline. Returns whether the block has ended.
    fn line(&mut self, line: &[u8]) -> bool {
        if self.done {
            return true;
        }
        if contains(line, b"/*---") {
            self.in_block = true;
            self.seen_block = true;
            return false;
        }
        if self.in_block && contains(line, b"---*/") {
            self.in_block = false;
            self.done = true;
            return true;
        }
        if !self.in_block {
            return false;
        }
        if let Some(colon) = key_length(line) {
            let key = line.get(..colon).unwrap_or_default();
            let rest = trim(line.get(colon + 1..).unwrap_or_default());
            self.list = ListKey::None;
            let target = match key {
                b"flags" => Some((ListKey::Flags, "flags")),
                b"features" => Some((ListKey::Features, "features")),
                b"includes" => Some((ListKey::Includes, "includes")),
                _ => None,
            };
            if let Some((list, name)) = target {
                if rest.contains(&b'[') {
                    let (items, closed) = inline(rest);
                    if !closed {
                        self.note(Problem::UnclosedList(name));
                    }
                    *self.field(list) = text(&items);
                } else if !rest.is_empty() && !matches!(rest.first(), Some(b'|' | b'>')) {
                    *self.field(list) = text(rest);
                } else {
                    self.list = list;
                }
            } else if key == b"negative" {
                self.list = ListKey::Negative;
                self.saw_negative = true;
            }
            return false;
        }
        if self.list == ListKey::Negative {
            if let Some(at) = find(line, b"phase:") {
                self.meta.negative_phase = text(trim(line.get(at + 6..).unwrap_or_default()));
            }
            if let Some(at) = find(line, b"type:") {
                self.meta.negative_type = text(trim(line.get(at + 5..).unwrap_or_default()));
            }
            return false;
        }
        if self.list != ListKey::None
            && let Some(start) = item_start(line)
        {
            let item = text(trim(line.get(start..).unwrap_or_default()));
            let list = self.list;
            let field = self.field(list);
            if !field.is_empty() {
                field.push(',');
            }
            field.push_str(&item);
        }
        false
    }

    fn field(&mut self, list: ListKey) -> &mut String {
        match list {
            ListKey::Flags => &mut self.meta.flags,
            ListKey::Features => &mut self.meta.features,
            // `None` and `Negative` never reach here; `includes` is the harmless answer.
            ListKey::Includes | ListKey::None | ListKey::Negative => &mut self.meta.includes,
        }
    }

    fn finish(mut self) -> Read {
        if !self.seen_block {
            self.note(Problem::NoBlock);
        } else if !self.done {
            self.note(Problem::Unterminated);
        }
        if self.saw_negative
            && (self.meta.negative_phase.is_empty() || self.meta.negative_type.is_empty())
        {
            self.note(Problem::IncompleteNegative);
        }
        Read {
            meta: self.meta,
            problem: self.problem,
        }
    }
}

/// Reads the front matter of a whole file's bytes.
pub fn parse(bytes: &[u8]) -> Read {
    let mut input = bytes;
    // Reading from a byte slice cannot fail; the empty reading is the harmless answer if it did.
    read(&mut input).unwrap_or_else(|_| Reader::new().finish())
}

/// Reads the front matter from `input`, line by line, and stops at the end of the block.
pub fn read(input: &mut impl BufRead) -> io::Result<Read> {
    let mut reader = Reader::new();
    let mut line = Vec::new();
    while next_line(input, &mut line)? {
        if reader.line(&line) {
            break;
        }
    }
    Ok(reader.finish())
}

/// Reads the next line of `input` into `line`, without its end: CR LF, CR or LF, as
/// `awk -v RS='\r\n|\r|\n'` splits records. Returns false at the end of the input when no line
/// is left.
fn next_line(input: &mut impl BufRead, line: &mut Vec<u8>) -> io::Result<bool> {
    line.clear();
    let mut any = false;
    loop {
        let buffer = input.fill_buf()?;
        if buffer.is_empty() {
            return Ok(any);
        }
        any = true;
        match buffer.iter().position(|&b| b == b'\n' || b == b'\r') {
            Some(at) => {
                line.extend_from_slice(buffer.get(..at).unwrap_or_default());
                let end = buffer.get(at).copied();
                input.consume(at + 1);
                if end == Some(b'\r') && input.fill_buf()?.first() == Some(&b'\n') {
                    input.consume(1);
                }
                return Ok(true);
            }
            None => {
                let length = buffer.len();
                line.extend_from_slice(buffer);
                input.consume(length);
            }
        }
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    find(haystack, needle).is_some()
}

/// `match($0, /^[a-zA-Z_]+:/)`: the length of the key when the line starts with one.
fn key_length(line: &[u8]) -> Option<usize> {
    let letters = line
        .iter()
        .take_while(|b| b.is_ascii_alphabetic() || **b == b'_')
        .count();
    (letters > 0 && line.get(letters) == Some(&b':')).then_some(letters)
}

/// `match($0, /^[ \t]*-[ \t]*/)`: where the item starts, after the dash and its blanks.
fn item_start(line: &[u8]) -> Option<usize> {
    let blanks = line.iter().take_while(|b| is_blank(**b)).count();
    if line.get(blanks) != Some(&b'-') {
        return None;
    }
    let after = line
        .get(blanks + 1..)
        .unwrap_or_default()
        .iter()
        .take_while(|b| is_blank(**b))
        .count();
    Some(blanks + 1 + after)
}

fn is_blank(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

/// `gsub(/^[ \t]+|[ \t]+$/, "")`.
fn trim(bytes: &[u8]) -> &[u8] {
    let start = bytes.iter().take_while(|b| is_blank(**b)).count();
    let rest = bytes.get(start..).unwrap_or_default();
    let end = rest.len() - rest.iter().rev().take_while(|b| is_blank(**b)).count();
    rest.get(..end).unwrap_or_default()
}

/// `inl` of `meta.awk`: the text after the first `[` and before the first `]` after it, without
/// blanks, and whether that `]` is there. With no `]` it is the rest of the line.
fn inline(rest: &[u8]) -> (Vec<u8>, bool) {
    let after = match rest.iter().position(|&b| b == b'[') {
        Some(at) => rest.get(at + 1..).unwrap_or_default(),
        None => rest,
    };
    let (inside, closed) = match after.iter().position(|&b| b == b']') {
        Some(at) => (after.get(..at).unwrap_or_default(), true),
        None => (after, false),
    };
    (
        inside.iter().copied().filter(|&b| !is_blank(b)).collect(),
        closed,
    )
}

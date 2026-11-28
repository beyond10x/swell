//! Source scans of the engine crates (`docs/design/host-interface.md`, decisions 1 and 3):
//! `HOST-1` (no unsafe code, no outside package, no thread), `HOST-2` (no ambient authority,
//! `#![no_std]` in each crate root, no `std` path in library code) and `HOST-146` (only
//! `Agent::new` takes the seeds).
//!
//! The engine crates are `swell` and every crate it reaches through a normal or build
//! dependency, read from the manifests. A scan reads the code of `src/` with comments and the
//! contents of string literals blanked, so a comment that names a forbidden item is not code.
//! Each scan is checked on planted text first, so that a scan that finds nothing has been shown
//! to find something.

use std::fs;

use scan::*;

/// The scanners. Inside a `cfg(test)` module, so that clippy's test allowances of
/// `clippy.toml` cover them as they cover the tests.
#[cfg(test)]
pub(crate) mod scan {
    use std::fs;
    use std::path::{Path, PathBuf};

    pub(crate) fn swell_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    pub(super) fn repository_root() -> PathBuf {
        let root = swell_dir().join("..").join("..");
        let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(
            manifest.contains("[workspace]"),
            "{} is not the workspace root",
            root.display()
        );
        root
    }

    /// The directories of a manifest's normal and build dependencies, and the names of those that
    /// have no path (which would be packages from outside the repository).
    pub(super) fn dependencies(crate_dir: &Path) -> (Vec<PathBuf>, Vec<String>) {
        let manifest = fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap();
        let mut paths = Vec::new();
        let mut outside = Vec::new();
        let mut in_dependencies = false;
        for line in manifest.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                let header = line.trim_matches(|c| c == '[' || c == ']');
                in_dependencies = (header.ends_with("dependencies")
                    && !header.ends_with("dev-dependencies"))
                    || header.contains("dependencies.") && !header.contains("dev-dependencies.");
                continue;
            }
            if !in_dependencies || line.is_empty() || line.starts_with('#') {
                continue;
            }
            let name = line.split('=').next().unwrap_or("").trim().to_string();
            match line
                .split("path")
                .nth(1)
                .and_then(|rest| rest.split('"').nth(1))
            {
                Some(path) => paths.push(crate_dir.join(path)),
                None => outside.push(name),
            }
        }
        (paths, outside)
    }

    /// `swell` and every crate it reaches, each once, `swell` first.
    pub(super) fn engine_crates() -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = Vec::new();
        let mut queue = vec![swell_dir()];
        while let Some(dir) = queue.pop() {
            let dir = fs::canonicalize(&dir).unwrap();
            if found.contains(&dir) {
                continue;
            }
            let (paths, outside) = dependencies(&dir);
            assert!(
                outside.is_empty(),
                "{}: dependencies without a path: {outside:?}",
                dir.display()
            );
            queue.extend(paths);
            found.push(dir);
        }
        found
    }

    pub(crate) fn rust_files(dir: &Path, into: &mut Vec<PathBuf>) {
        let mut entries: Vec<PathBuf> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                rust_files(&path, into);
            } else if path.extension().is_some_and(|e| e == "rs") {
                into.push(path);
            }
        }
    }

    /// The code of a Rust source: comments and the contents of string and character literals are
    /// replaced by spaces (newlines kept), and whitespace around `::` is removed.
    pub(crate) fn code_of(source: &str) -> String {
        let chars: Vec<char> = source.chars().collect();
        #[expect(
            clippy::disallowed_methods,
            reason = "test code: the length of one source file of the repository"
        )]
        let mut out = String::with_capacity(source.len());
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let next = chars.get(i + 1).copied();
            if c == '/' && next == Some('/') {
                while i < chars.len() && chars[i] != '\n' {
                    out.push(' ');
                    i += 1;
                }
            } else if c == '/' && next == Some('*') {
                let mut depth = 0;
                while i < chars.len() {
                    if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                        depth += 1;
                        out.push_str("  ");
                        i += 2;
                    } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                        depth -= 1;
                        out.push_str("  ");
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        out.push(if chars[i] == '\n' { '\n' } else { ' ' });
                        i += 1;
                    }
                }
            } else if c == 'r' && (next == Some('"') || next == Some('#')) && raw_prefix(&chars, i)
            {
                // A raw string: r"..." or r#"..."#, also as a byte or C string (br"...",
                // cr#"..."#).
                let mut j = i + 1;
                let mut hashes = 0;
                while chars.get(j) == Some(&'#') {
                    hashes += 1;
                    j += 1;
                }
                if chars.get(j) != Some(&'"') {
                    out.push(c);
                    i += 1;
                    continue;
                }
                out.push('"');
                j += 1;
                loop {
                    match chars.get(j) {
                        None => break,
                        Some('"') if (1..=hashes).all(|k| chars.get(j + k) == Some(&'#')) => {
                            j += 1 + hashes;
                            break;
                        }
                        Some('\n') => out.push('\n'),
                        Some(_) => out.push(' '),
                    }
                    j += 1;
                }
                out.push('"');
                i = j;
            } else if c == '"' {
                out.push('"');
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        out.push(' ');
                        i += 1;
                    }
                    if i < chars.len() {
                        out.push(if chars[i] == '\n' { '\n' } else { ' ' });
                        i += 1;
                    }
                }
                out.push('"');
                i += 1;
            } else if c == '\'' && (next == Some('\\') || chars.get(i + 2) == Some(&'\'')) {
                // A character literal ('x' or '\n'); a lifetime has no closing quote.
                out.push('\'');
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    if chars[i] == '\\' {
                        out.push(' ');
                        i += 1;
                    }
                    out.push(' ');
                    i += 1;
                }
                out.push('\'');
                i += 1;
            } else {
                out.push(c);
                i += 1;
            }
        }
        // `std :: fs` is `std::fs`, wherever it stands: the blanks before each `::` go as well
        // as those after it.
        let parts: Vec<&str> = out.split("::").collect();
        let last = parts.len().saturating_sub(1);
        #[expect(
            clippy::disallowed_methods,
            reason = "test code: at most the length of `out`, one source file of the repository"
        )]
        let mut joined = String::with_capacity(out.len());
        for (k, part) in parts.iter().enumerate() {
            let part = if k == 0 { part } else { part.trim_start() };
            let part = if k == last { part } else { part.trim_end() };
            if k > 0 {
                joined.push_str("::");
            }
            joined.push_str(part);
        }
        joined
    }

    /// Whether the `r` at `at` starts a raw string literal: it begins a token, alone or after
    /// the `b` or `c` of a byte or C string that itself begins one.
    fn raw_prefix(chars: &[char], at: usize) -> bool {
        let before = |k: usize| at.checked_sub(k).and_then(|j| chars.get(j)).copied();
        if !ident_char(before(1)) {
            return true;
        }
        matches!(before(1), Some('b' | 'c')) && !ident_char(before(2))
    }

    pub(crate) fn ident_char(c: Option<char>) -> bool {
        c.is_some_and(|c| c.is_alphanumeric() || c == '_')
    }

    /// Whether `word` occurs in `code` as a whole identifier or path (not as part of a longer name).
    pub(crate) fn has_word(code: &str, word: &str) -> bool {
        code.match_indices(word).any(|(at, _)| {
            let before = code[..at].chars().next_back();
            let after = code[at + word.len()..].chars().next();
            !ident_char(before) && !ident_char(after)
        })
    }

    /// The names imported through a group `std::{...}` (nested groups flattened), as paths below
    /// `std`: `use std::{env, time::{Instant}}` gives `env`, `time::Instant` and `time`.
    pub(super) fn std_group_names(code: &str) -> Vec<String> {
        let mut names = Vec::new();
        for (at, _) in code.match_indices("std::{") {
            let before = code[..at].chars().next_back();
            if ident_char(before) {
                continue;
            }
            let rest = &code[at + "std::{".len()..];
            let mut depth = 1;
            let mut end = rest.len();
            for (k, c) in rest.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = k;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let group: String = rest[..end]
                .chars()
                .map(|c| {
                    if c == '{' || c == '}' || c == ',' {
                        ' '
                    } else {
                        c
                    }
                })
                .collect();
            for token in group.split_whitespace() {
                names.push(token.to_string());
                if let Some((first, _)) = token.split_once("::") {
                    names.push(first.to_string());
                }
            }
        }
        names
    }

    /// The faults of HOST-2 in one file's code.
    pub(crate) fn ambient_authority(code: &str) -> Vec<String> {
        let mut faults = Vec::new();
        for path in ["std::env", "std::fs", "std::net", "std::process"] {
            if has_word(code, path) {
                faults.push(path.to_string());
            }
        }
        for name in ["SystemTime", "Instant", "RandomState"] {
            if has_word(code, name) {
                faults.push(name.to_string());
            }
        }
        for name in std_group_names(code) {
            let first = name.split("::").next().unwrap_or("");
            if ["env", "fs", "net", "process"].contains(&first) {
                faults.push(format!("std::{{{name}}}"));
            }
        }
        faults
    }

    /// The code with every item under `#[cfg(test)]` blanked (newlines kept): the attribute and
    /// what follows it up to the first `;` outside brackets, the first `,` outside brackets and
    /// angle brackets, or the `}` that closes the first `{`. That is an item (a module, a
    /// function, `extern crate std;`, a macro call with braces), a statement, a field or a field
    /// of a struct expression, a parameter, a variant or a match arm; the comma of
    /// `Result<u8, E>` in a signature does not end it. A `)`, `]` or `}` that closes an
    /// enclosing bracket ends it too. A `<` is an angle bracket only right after a name or `::`
    /// and not as `<<` or `<=`, a `>` only right after a non-blank that is not `-` or `=`: the
    /// form rustfmt writes for generics and never for a comparison or a shift. A `<` the scan
    /// takes for a comparison lets a comma end the item early, which leaves code scanned.
    pub(super) fn without_test_items(code: &str) -> String {
        let mut chars: Vec<char> = code.chars().collect();
        let marker: Vec<char> = "#[cfg(test)]".chars().collect();
        let mut i = 0;
        while i + marker.len() <= chars.len() {
            if chars[i..i + marker.len()] != marker[..] {
                i += 1;
                continue;
            }
            let mut depth = 0usize;
            let mut angles = 0usize;
            let mut end = chars.len();
            for (j, &c) in chars.iter().enumerate().skip(i + marker.len()) {
                let before = chars.get(j.wrapping_sub(1)).copied();
                let after = chars.get(j + 1).copied();
                match c {
                    '(' | '[' | '{' => depth += 1,
                    ')' | ']' if depth == 0 => {
                        end = j;
                        break;
                    }
                    ')' | ']' => depth -= 1,
                    '<' if (ident_char(before) || before == Some(':'))
                        && !matches!(after, Some('<' | '=')) =>
                    {
                        angles += 1;
                    }
                    '>' if before.is_some_and(|b| !b.is_whitespace() && b != '-' && b != '=') => {
                        angles = angles.saturating_sub(1);
                    }
                    ',' if depth == 0 && angles > 0 => {}
                    '}' if depth == 0 => {
                        end = j;
                        break;
                    }
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = j + 1;
                            break;
                        }
                    }
                    ';' | ',' if depth == 0 => {
                        end = j + 1;
                        break;
                    }
                    _ => {}
                }
            }
            for c in &mut chars[i..end] {
                if *c != '\n' {
                    *c = ' ';
                }
            }
            i = end;
        }
        chars.into_iter().collect()
    }

    /// The library code of a Rust source: its code (`code_of`) without the items under
    /// `#[cfg(test)]`.
    pub(super) fn library_code(source: &str) -> String {
        without_test_items(&code_of(source))
    }

    /// Each `std` of the code as a whole name, with its line and the path it starts
    /// (`line 3: std::vec::Vec`; `line 1: std` for `extern crate std`).
    pub(crate) fn std_paths(code: &str) -> Vec<String> {
        let mut found = Vec::new();
        for (at, _) in code.match_indices("std") {
            let before = code[..at].chars().next_back();
            let after = code[at + 3..].chars().next();
            if ident_char(before) || ident_char(after) {
                continue;
            }
            let path: String = code[at..]
                .chars()
                .take_while(|&c| ident_char(Some(c)) || c == ':')
                .collect();
            let line = code[..at].matches('\n').count() + 1;
            found.push(format!("line {line}: {}", path.trim_end_matches(':')));
        }
        found
    }

    /// Whether a crate root declares `#![no_std]` on a line of its own, in its code: a line
    /// inside a comment declares nothing.
    pub(super) fn declares_no_std(root: &str) -> bool {
        code_of(root)
            .lines()
            .any(|line| line.trim() == "#![no_std]")
    }

    /// The faults of HOST-1's thread rule in one file's code.
    pub(super) fn thread_spawn(code: &str) -> Vec<String> {
        let mut faults = Vec::new();
        for path in ["thread::spawn", "thread::Builder"] {
            if has_word(code, path) {
                faults.push(path.to_string());
            }
        }
        for name in std_group_names(code) {
            if name == "thread::spawn" || name == "thread::Builder" {
                faults.push(format!("std::{{{name}}}"));
            }
        }
        faults
    }

    /// A public function signature: its file-relative name and its text from `fn` to the first `{`
    /// or `;`.
    #[derive(Debug)]
    pub(super) struct PublicFn {
        pub(super) name: String,
        pub(super) signature: String,
    }

    /// The functions of one file's code that are part of the interface: `pub fn` (not
    /// `pub(crate)` or another restricted visibility), and every `fn` inside a `pub trait`.
    pub(super) fn public_fns(code: &str) -> Vec<PublicFn> {
        // The byte ranges of the bodies of public traits.
        let mut trait_bodies = Vec::new();
        for (at, _) in code.match_indices("pub trait ") {
            if let Some(open) = code[at..].find('{') {
                let start = at + open;
                let mut depth = 0;
                for (k, c) in code[start..].char_indices() {
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                trait_bodies.push(start..start + k);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        let mut found = Vec::new();
        for (at, _) in code.match_indices("fn ") {
            if ident_char(code[..at].chars().next_back()) {
                continue;
            }
            let in_trait = trait_bodies.iter().any(|body| body.contains(&at));
            // The words before `fn` on its line, after the last `;`, `{` or `}`.
            let line_start = code[..at].rfind(['\n', ';', '{', '}']).map_or(0, |k| k + 1);
            let prefix = code[line_start..at].trim();
            let words: Vec<&str> = prefix.split_whitespace().collect();
            let public = words.first().is_some_and(|w| *w == "pub");
            if !public && !in_trait {
                continue;
            }
            let rest = &code[at + 3..];
            let end = rest.find(['{', ';']).unwrap_or(rest.len());
            let signature = rest[..end].split_whitespace().collect::<Vec<_>>().join(" ");
            let name = signature
                .split(|c: char| !ident_char(Some(c)))
                .next()
                .unwrap_or("")
                .to_string();
            found.push(PublicFn { name, signature });
        }
        found
    }

    /// Whether a public signature takes or returns seeds or random bytes.
    pub(super) fn names_a_seed(signature: &str) -> bool {
        let lower = signature.to_lowercase();
        has_word(signature, "Seeds") || lower.contains("seed") || lower.contains("random")
    }

    pub(super) fn read_code(path: &Path) -> String {
        code_of(&fs::read_to_string(path).unwrap())
    }

    pub(super) fn source_files(crate_dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        rust_files(&crate_dir.join("src"), &mut files);
        assert!(!files.is_empty(), "{}: no source file", crate_dir.display());
        files
    }

    /// The characters of a Rust source with comments and the contents of string and character
    /// literals replaced by spaces (newlines kept) and nothing else changed: the character at
    /// an index is the source's at the same index, or a space in its place. A raw string keeps
    /// its `r`, its `#`s and its quotes.
    pub(crate) fn blanked(source: &str) -> Vec<char> {
        let mut chars: Vec<char> = source.chars().collect();
        let blank = |c: &mut char| {
            if *c != '\n' {
                *c = ' ';
            }
        };
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let next = chars.get(i + 1).copied();
            if c == '/' && next == Some('/') {
                while i < chars.len() && chars[i] != '\n' {
                    blank(&mut chars[i]);
                    i += 1;
                }
            } else if c == '/' && next == Some('*') {
                let mut depth = 0;
                while i < chars.len() {
                    if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                        depth += 1;
                        chars[i] = ' ';
                        chars[i + 1] = ' ';
                        i += 2;
                    } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                        depth -= 1;
                        chars[i] = ' ';
                        chars[i + 1] = ' ';
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        blank(&mut chars[i]);
                        i += 1;
                    }
                }
            } else if c == 'r' && (next == Some('"') || next == Some('#')) && raw_prefix(&chars, i)
            {
                let mut j = i + 1;
                let mut hashes = 0;
                while chars.get(j) == Some(&'#') {
                    hashes += 1;
                    j += 1;
                }
                if chars.get(j) != Some(&'"') {
                    i += 1;
                    continue;
                }
                j += 1;
                while j < chars.len() {
                    if chars[j] == '"' && (1..=hashes).all(|k| chars.get(j + k) == Some(&'#')) {
                        j += 1 + hashes;
                        break;
                    }
                    blank(&mut chars[j]);
                    j += 1;
                }
                i = j;
            } else if c == '"' {
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        blank(&mut chars[i]);
                        i += 1;
                    }
                    if i < chars.len() {
                        blank(&mut chars[i]);
                        i += 1;
                    }
                }
                i += 1;
            } else if c == '\'' && (next == Some('\\') || chars.get(i + 2) == Some(&'\'')) {
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    if chars[i] == '\\' {
                        blank(&mut chars[i]);
                        i += 1;
                    }
                    if i < chars.len() {
                        blank(&mut chars[i]);
                        i += 1;
                    }
                }
                i += 1;
            } else {
                i += 1;
            }
        }
        chars
    }

    /// The index after the blanks from `at` on.
    fn skip_blanks(chars: &[char], mut at: usize) -> usize {
        while chars.get(at).is_some_and(|c| c.is_whitespace()) {
            at += 1;
        }
        at
    }

    /// The index of the bracket that closes the one at `open`, counting `(`, `[` and `{`
    /// alike, or the end of the text.
    fn closing(chars: &[char], open: usize) -> usize {
        let mut depth = 0usize;
        for (k, &c) in chars.iter().enumerate().skip(open) {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return k;
                    }
                }
                _ => {}
            }
        }
        chars.len()
    }

    /// One `allow(...)` or `expect(...)` of an attribute: the line of its attribute, whether
    /// that attribute is inner (`#![...]`), the level, the lint paths with their blanks
    /// removed, and the text of its `reason` string as the source has it.
    #[derive(Debug, PartialEq)]
    pub(crate) struct LintLevel {
        pub(crate) line: usize,
        pub(crate) inner: bool,
        pub(crate) level: String,
        pub(crate) lints: Vec<String>,
        pub(crate) reason: Option<String>,
    }

    /// The attributes of blanked text (`blanked`), outer and inner: their line, whether they are
    /// inner, and the indices of their `[` and of the `]` that closes it. An attribute is `#`, an
    /// optional `!` and `[`, with any blanks or line ends between them, to the matching `]`; one
    /// in a comment or a string is blanked away, one in a `macro_rules!` body is found.
    fn attribute_spans(chars: &[char]) -> Vec<(usize, bool, usize, usize)> {
        let mut found = Vec::new();
        let mut line = 1;
        for (at, &c) in chars.iter().enumerate() {
            if c == '\n' {
                line += 1;
            }
            if c != '#' {
                continue;
            }
            let mut k = skip_blanks(chars, at + 1);
            let inner = chars.get(k) == Some(&'!');
            if inner {
                k = skip_blanks(chars, k + 1);
            }
            if chars.get(k) == Some(&'[') {
                found.push((line, inner, k, closing(chars, k)));
            }
        }
        found
    }

    /// Every attribute of a source: its line, whether it is inner, and the text between its
    /// brackets with comments and string contents blanked.
    pub(crate) fn attributes(source: &str) -> Vec<(usize, bool, String)> {
        let chars = blanked(source);
        attribute_spans(&chars)
            .into_iter()
            .map(|(line, inner, open, end)| {
                let body: String = chars[open + 1..end.min(chars.len())].iter().collect();
                (line, inner, body)
            })
            .collect()
    }

    /// Every `allow(...)` and `expect(...)` of every attribute of a source, at any depth of
    /// `cfg_attr`, a `macro_rules!` body included.
    pub(crate) fn lint_levels(source: &str) -> Vec<LintLevel> {
        let raw: Vec<char> = source.chars().collect();
        let chars = blanked(source);
        let mut found = Vec::new();
        for (line, inner, open, end) in attribute_spans(&chars) {
            let mut w = open + 1;
            while w < end {
                let length = chars[w..end]
                    .iter()
                    .take_while(|&&c| ident_char(Some(c)))
                    .count();
                if length == 0 {
                    w += 1;
                    continue;
                }
                let word: String = chars[w..w + length].iter().collect();
                let before = w.checked_sub(1).and_then(|b| chars.get(b)).copied();
                let after = skip_blanks(&chars, w + length);
                if (word == "allow" || word == "expect")
                    && !ident_char(before)
                    && before != Some(':')
                    && chars.get(after) == Some(&'(')
                {
                    let close = closing(&chars, after);
                    let (lints, reason) = lint_list(&raw, &chars, after + 1, close);
                    found.push(LintLevel {
                        line,
                        inner,
                        level: word,
                        lints,
                        reason,
                    });
                }
                w += length;
            }
        }
        found
    }

    /// The lint paths and the reason of the list between `from` and `to`: its items split at
    /// the commas outside brackets; `reason = "..."` is read from the source's own text.
    fn lint_list(
        raw: &[char],
        chars: &[char],
        from: usize,
        to: usize,
    ) -> (Vec<String>, Option<String>) {
        let mut items = Vec::new();
        let mut depth = 0usize;
        let mut start = from;
        let to = to.min(chars.len());
        for (k, &c) in chars.iter().enumerate().take(to).skip(from) {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth = depth.saturating_sub(1),
                ',' if depth == 0 => {
                    items.push(start..k);
                    start = k + 1;
                }
                _ => {}
            }
        }
        items.push(start..to);
        let mut lints = Vec::new();
        let mut reason = None;
        for item in items {
            let text: String = chars[item.clone()]
                .iter()
                .filter(|c| !c.is_whitespace())
                .collect();
            if text.is_empty() {
                continue;
            }
            if let Some(value) = text.strip_prefix("reason=") {
                let open = (item.start..item.end).find(|&k| chars[k] == '"');
                let close = open.and_then(|o| (o + 1..item.end).find(|&k| chars[k] == '"'));
                reason = match (open, close) {
                    (Some(o), Some(c)) => Some(raw[o + 1..c].iter().collect()),
                    _ => Some(value.to_string()),
                };
            } else {
                lints.push(text);
            }
        }
        (lints, reason)
    }

    /// The lint groups that hold `clippy::disallowed_methods` or every lint.
    pub(crate) const GROUPS: [&str; 3] = ["clippy::style", "clippy::all", "warnings"];

    /// The lint list of the one `allow` of `clippy::disallowed_methods` the crate has: the
    /// attribute of `crates/swell/src/lib.rs` that lets test code call the listed methods.
    pub(crate) const LIB_ALLOW: [&str; 1] = ["clippy::disallowed_methods"];

    /// The faults of one `allow` or `expect` of a file of `crates/swell` under the rule of
    /// `story:buffer-growth-outside-the-charged-list-is-listed`: a listed call carries
    /// `#[expect(clippy::disallowed_methods, reason = "...")]` with a reason that is not blank,
    /// so an `allow` of the lint, an `allow` or `expect` of a group that holds it, and an
    /// `expect` of it without a reason are refused. `lib_allow` is true for the one `allow`
    /// of `lib.rs` that the rule takes, which the caller hands out once.
    pub(crate) fn growth_attribute_faults(attribute: &LintLevel, lib_allow: bool) -> Vec<String> {
        let mut faults = Vec::new();
        let names_lint = attribute
            .lints
            .iter()
            .any(|lint| lint == "clippy::disallowed_methods");
        let groups: Vec<&str> = attribute
            .lints
            .iter()
            .map(String::as_str)
            .filter(|lint| GROUPS.contains(lint))
            .collect();
        let bang = if attribute.inner { "!" } else { "" };
        let at = format!("line {}: #{bang}[{}(", attribute.line, attribute.level);
        if !groups.is_empty() {
            faults.push(format!(
                "{at}{})]: a group that holds the listed lint",
                groups.join(", ")
            ));
        }
        if attribute.level == "allow" && names_lint && !lib_allow {
            faults.push(format!(
                "{at}clippy::disallowed_methods)]: an allow of the listed lint"
            ));
        }
        if attribute.level == "expect"
            && names_lint
            && attribute
                .reason
                .as_deref()
                .is_none_or(|reason| reason.trim().is_empty())
        {
            faults.push(format!(
                "{at}clippy::disallowed_methods)]: no reason, or a blank one"
            ));
        }
        faults
    }
}

#[test]
fn the_scans_find_planted_text() {
    // Comments and strings are not code.
    let code = code_of(
        "// std::fs\n/* std::env /* nested */ Instant */ let s = \"std::net\"; let c = '\"';\nlet r = r#\"std::process\"#;",
    );
    assert!(ambient_authority(&code).is_empty(), "{code}");
    // Code is.
    assert_eq!(ambient_authority(&code_of("use std :: fs;")), ["std::fs"]);
    assert_eq!(
        ambient_authority(&code_of("let t = std::time::SystemTime::now();")),
        ["SystemTime"]
    );
    assert_eq!(
        ambient_authority(&code_of("use std::time::Instant;")),
        ["Instant"]
    );
    assert_eq!(
        ambient_authority(&code_of("use std::collections::hash_map::RandomState;")),
        ["RandomState"]
    );
    assert_eq!(
        ambient_authority(&code_of("use std::{io, env};")),
        ["std::{env}"]
    );
    assert_eq!(
        ambient_authority(&code_of("use std::{net::TcpStream, io};")),
        ["std::{net::TcpStream}", "std::{net}"]
    );
    assert_eq!(
        ambient_authority(&code_of("let p = std::process::id();")),
        ["std::process"]
    );
    assert!(ambient_authority(&code_of("struct Instantiate; let e = myenv::fs;")).is_empty());
    assert_eq!(
        thread_spawn(&code_of("std::thread::spawn(|| {});")),
        ["thread::spawn"]
    );
    assert_eq!(
        thread_spawn(&code_of("use std::{thread::Builder};")),
        ["thread::Builder", "std::{thread::Builder}"]
    );
    assert!(thread_spawn(&code_of("// thread::spawn\nlet x = 1;")).is_empty());

    // HOST-2's `std` rule: a `std` path of library code is found with its line, one in an item
    // under `#[cfg(test)]` is not, and neither is one in a comment, a string or a longer name.
    assert_eq!(
        std_paths(&library_code("use std::vec::Vec;\nfn f() {}")),
        ["line 1: std::vec::Vec"]
    );
    assert_eq!(
        std_paths(&library_code("extern crate std;")),
        ["line 1: std"]
    );
    assert_eq!(
        std_paths(&library_code(
            "let a = 1;\nlet pi = ::std::f64::consts::PI;"
        )),
        ["line 2: std::f64::consts::PI"]
    );
    let planted = "#[cfg(test)]\nextern crate std;\n\
                   #[cfg(test)]\nmod tests {\n    use std::println;\n    fn g() { println!(); }\n}\n\
                   #[cfg(test)]\nstd::thread_local! { static X: u8 = const { 0 }; }\n\
                   struct S {\n    #[cfg(test)]\n    seen: std::vec::Vec<u8>,\n    kept: u8,\n}\n\
                   fn h() -> std :: string :: String { make() }";
    assert_eq!(
        std_paths(&library_code(planted)),
        ["line 15: std::string::String"]
    );
    assert_eq!(
        std_paths(&library_code(
            "struct S {\n    #[cfg(test)]\n    a: u8,\n    b: std::boxed::Box<u8>,\n}"
        )),
        ["line 4: std::boxed::Box"]
    );
    assert_eq!(
        std_paths(&library_code(
            "#[cfg(test)]\nconst N: [u8; 2] = [1, 2];\nconst M: std::primitive::u8 = 1;"
        )),
        ["line 3: std::primitive::u8"]
    );
    assert!(
        std_paths(&library_code(
            "// std::fs\nlet s = \"std::x\"; let stdout = 1; let x = mystd::a; let c = core::fmt::Error;"
        ))
        .is_empty()
    );
    assert!(declares_no_std(
        "#![forbid(unsafe_code)]\n//! Text.\n#![no_std]\n"
    ));
    assert!(!declares_no_std(
        "#![forbid(unsafe_code)]\n// #![no_std]\n#![cfg_attr(not(test), no_std)]\n"
    ));

    // Public functions, a trait's functions, and not restricted ones.
    let code = code_of(
        "pub fn new(host: H, seeds: Seeds) -> Self {}\n\
         pub(crate) fn internal(seed: u64) {}\n\
         fn private(random: u8) {}\n\
         pub trait Host { fn random_bytes(cx: &mut Cx) -> Vec<u8>; fn other(); }\n\
         pub const fn limit(&self) -> u64 { 1 }",
    );
    let fns = public_fns(&code);
    let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["new", "random_bytes", "other", "limit"]);
    let seeded: Vec<&str> = fns
        .iter()
        .filter(|f| names_a_seed(&f.signature))
        .map(|f| f.name.as_str())
        .collect();
    assert_eq!(seeded, ["new", "random_bytes"]);
}

/// HOST-1: `swell` and every crate it depends on declare `#![forbid(unsafe_code)]`, the root
/// `Cargo.lock` names no package from outside the repository, and no engine crate calls
/// `std::thread::spawn` or `std::thread::Builder`.
#[test]
fn host_1_forbid_unsafe_no_outside_package_no_thread_spawn() {
    let crates = engine_crates();
    assert!(
        crates.len() >= 2,
        "swell and swell-num at least: {crates:?}"
    );
    let mut faults = Vec::new();
    for dir in &crates {
        let lib = dir.join("src").join("lib.rs");
        let text = fs::read_to_string(&lib).unwrap();
        if !text
            .lines()
            .any(|line| line.trim() == "#![forbid(unsafe_code)]")
        {
            faults.push(format!("{}: no #![forbid(unsafe_code)]", lib.display()));
        }
        for file in source_files(dir) {
            for fault in thread_spawn(&read_code(&file)) {
                faults.push(format!("{}: {fault}", file.display()));
            }
        }
    }
    let lock = fs::read_to_string(repository_root().join("Cargo.lock")).unwrap();
    let packages = lock.lines().filter(|line| *line == "[[package]]").count();
    assert!(
        packages >= crates.len(),
        "Cargo.lock lists {packages} package(s)"
    );
    for line in lock.lines().filter(|line| line.starts_with("source = ")) {
        faults.push(format!(
            "Cargo.lock: a package from outside the repository: {line}"
        ));
    }
    assert!(faults.is_empty(), "{faults:#?}");
}

/// HOST-2: the engine crates use none of `SystemTime`, `Instant`, `std::env`, `std::fs`,
/// `std::net`, `std::process` and `RandomState`; each crate root declares `#![no_std]`, and no
/// library code names a `std` path. An item under `#[cfg(test)]` (a test module, the
/// `extern crate std` of the unit tests, a counter of the test build) is not library code.
#[test]
fn host_2_no_ambient_authority_in_engine_crates() {
    let crates = engine_crates();
    assert!(
        crates.len() >= 6,
        "swell and the five crates below it at least: {crates:?}"
    );
    let mut faults = Vec::new();
    let mut files = 0;
    for dir in &crates {
        let lib = dir.join("src").join("lib.rs");
        if !declares_no_std(&fs::read_to_string(&lib).unwrap()) {
            faults.push(format!("{}: no #![no_std]", lib.display()));
        }
        for file in source_files(dir) {
            files += 1;
            let code = read_code(&file);
            for fault in ambient_authority(&code) {
                faults.push(format!("{}: {fault}", file.display()));
            }
            for fault in std_paths(&without_test_items(&code)) {
                faults.push(format!("{}: {fault}", file.display()));
            }
        }
    }
    assert!(files >= 2, "{files} file(s) scanned");
    assert!(faults.is_empty(), "{faults:#?}");
}

/// HOST-146: `Agent::new` is the only function of the interface that takes `Seeds`, and no
/// other public function, read or hook takes or returns seed or random bytes.
#[test]
fn host_146_only_agent_new_takes_seeds() {
    let src = swell_dir().join("src");
    let mut allowed = Vec::new();
    let mut faults = Vec::new();
    let mut scanned = 0;
    for file in source_files(&swell_dir()) {
        let relative = file
            .strip_prefix(&src)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        for function in public_fns(&read_code(&file)) {
            scanned += 1;
            if !names_a_seed(&function.signature) {
                continue;
            }
            if relative == "host/agent.rs"
                && function.name == "new"
                && has_word(&function.signature, "Seeds")
            {
                allowed.push(function.signature);
            } else {
                faults.push(format!("{relative}: fn {}", function.signature));
            }
        }
    }
    assert!(scanned >= 20, "{scanned} public function(s) scanned");
    assert_eq!(
        allowed.len(),
        1,
        "Agent::new taking Seeds, found: {allowed:?}"
    );
    assert!(faults.is_empty(), "{faults:#?}");
}

/// A raw byte or C string with a quote inside (`br#"x"y"#`) is a literal like any other: the
/// code after it is read, and the `std` of the `extern crate` there, which only a hosted build
/// compiles, is found (adversary pass 1 on unit gate; the scan read `br` as a name followed by
/// a plain string before).
#[test]
fn a_raw_byte_string_hides_no_std_path_after_it() {
    let planted = "const A: &[u8] = br#\"x\"y\"#;\n\
                   #[cfg(not(target_os = \"none\"))]\n\
                   extern crate std;\n\
                   const B: u8 = 0;\n";
    assert_eq!(std_paths(&library_code(planted)), ["line 3: std"]);
    let planted = "const C: &core::ffi::CStr = cr#\"a\"b\"#;\nextern crate std;\n";
    assert_eq!(std_paths(&library_code(planted)), ["line 2: std"]);
    let planted = "let name = br\"std::fs\";\nlet s = std::fs::read;\n";
    assert_eq!(std_paths(&library_code(planted)), ["line 2: std::fs::read"]);
}

/// `#![no_std]` inside a block comment declares nothing (adversary pass 1 on unit gate).
#[test]
fn no_std_inside_a_block_comment_declares_nothing() {
    assert!(!declares_no_std(
        "/*\n#![no_std]\n*/\n#![forbid(unsafe_code)]\n"
    ));
}

/// A function under `#[cfg(test)]` whose return type has a comma in its generics is test code
/// as a whole; its `std` path is not library code (adversary pass 1 on unit gate). A `<` of a
/// comparison in a test statement does not carry the blanking past the statement's `;`.
#[test]
fn a_test_item_with_a_comma_in_its_generics_is_test_code() {
    let planted = "#[cfg(test)]\n\
                   fn parse() -> Result<u8, std::num::ParseIntError> {\n    \"1\".parse()\n}\n\
                   fn kept() {}\n";
    assert_eq!(std_paths(&library_code(planted)), Vec::<String>::new());
    let planted = "#[cfg(test)]\nlet small = a < b;\nlet p: std::time::Duration = d;\n";
    assert_eq!(
        std_paths(&library_code(planted)),
        ["line 3: std::time::Duration"]
    );
    let planted =
        "#[cfg(test)]\nfn f() -> Box<dyn Fn(u8) -> u8> { todo() }\nfn g(x: std::any::TypeId) {}\n";
    assert_eq!(
        std_paths(&library_code(planted)),
        ["line 3: std::any::TypeId"]
    );
}

/// A `<<` in the discriminant of a test-only variant is no angle bracket: the variant's `,`
/// ends it, and the next variant is library code (adversary pass 2 on unit gate: it was
/// blanked up to the enum's `}`).
#[test]
fn a_shift_in_a_test_variant_leaves_the_next_variant_scanned() {
    let planted = "enum Flags {\n\
                   \x20   #[cfg(test)]\n\
                   \x20   Probe = 1 << 7,\n\
                   \x20   Hosted(std::string::String),\n\
                   }\n";
    assert_eq!(
        std_paths(&library_code(planted)),
        ["line 4: std::string::String"]
    );
}

/// A `<` in the guard of a test-only match arm is no angle bracket: the arm's `,` ends it, and
/// the next arm's block, with an `extern crate std` behind a feature the bare build does not
/// enable, is library code (adversary pass 2 on unit gate: it was blanked).
#[test]
fn a_comparison_in_a_test_match_arm_leaves_the_next_arm_scanned() {
    let planted = "fn f(n: u8) -> u8 {\n\
                   \x20   match n {\n\
                   \x20       #[cfg(test)]\n\
                   \x20       0 if n < 3 => 1,\n\
                   \x20       _ => {\n\
                   \x20           #[cfg(feature = \"fault-injection\")]\n\
                   \x20           {\n\
                   \x20               extern crate std;\n\
                   \x20               std::eprintln!(\"hosted\");\n\
                   \x20           }\n\
                   \x20           0\n\
                   \x20       }\n\
                   \x20   }\n\
                   }\n";
    assert_eq!(
        std_paths(&library_code(planted)),
        ["line 8: std", "line 9: std::eprintln"]
    );
}

/// `#[cfg(test)]` on a function parameter blanks the parameter up to the `)`; the rest of the
/// signature and the body are library code (adversary pass 2 on unit gate: a `)` at depth 0 did
/// not end the item, and the body was blanked).
#[test]
fn a_test_parameter_leaves_the_function_body_scanned() {
    let planted = "pub fn hosted(#[cfg(test)] probe: u8) -> u8 {\n\
                   \x20   #[cfg(feature = \"fault-injection\")]\n\
                   \x20   {\n\
                   \x20       extern crate std;\n\
                   \x20       std::eprintln!(\"hosted\");\n\
                   \x20   }\n\
                   \x20   0\n\
                   }\n";
    assert_eq!(
        std_paths(&library_code(planted)),
        ["line 4: std", "line 5: std::eprintln"]
    );
}

/// The faults of the rule of `story:buffer-growth-outside-the-charged-list-is-listed` among the
/// `allow` and `expect` attributes `levels` of the file `relative` of `crates/swell`; the one
/// `allow` of `src/lib.rs` is taken once, through `lib_allow_left`.
fn growth_faults_of(
    relative: &str,
    levels: &[LintLevel],
    lib_allow_left: &mut usize,
) -> Vec<String> {
    let mut faults = Vec::new();
    for attribute in levels {
        let lib_allow = relative == "src/lib.rs"
            && attribute.level == "allow"
            && attribute.lints == LIB_ALLOW
            && *lib_allow_left > 0;
        if lib_allow {
            *lib_allow_left -= 1;
        }
        for fault in growth_attribute_faults(attribute, lib_allow) {
            faults.push(format!("{relative}: {fault}"));
        }
    }
    faults
}

/// The faults of the growth rule in planted text, read as a file other than `lib.rs`.
fn planted_growth_faults(source: &str) -> Vec<String> {
    growth_faults_of("src/planted.rs", &lint_levels(source), &mut 0)
}

/// The growth rule's scan on planted text: each form clippy lets through and the rule refuses
/// is found (adversary passes 1 and 2 on unit growth), and the forms it takes are not.
#[test]
fn the_growth_scan_finds_planted_attributes() {
    // An inner allow with a reason: at the top of a module file, of an inline module, a fn
    // body, a block, a loop body, a test fn body (A1 to A5, T4).
    for planted in [
        "#![allow(\n    clippy::disallowed_methods,\n    reason = \"probe A1\"\n)]\nuse alloc::vec::Vec;\n",
        "pub mod a2 {\n    #![allow(clippy::disallowed_methods, reason = \"probe A2\")]\n}\n",
        "pub fn a3() {\n    #![allow(clippy::disallowed_methods, reason = \"probe A3\")]\n    f()\n}\n",
        "fn a4() {\n    {\n        #![allow(clippy::disallowed_methods, reason = \"probe A4\")]\n    }\n}\n",
        "fn a5() {\n    for _ in 0..n {\n        #![allow(clippy::disallowed_methods, reason = \"A5\")]\n    }\n}\n",
        "#[test]\nfn t4() {\n    #![allow(clippy::disallowed_methods, reason = \"probe T4\")]\n}\n",
    ] {
        let faults = planted_growth_faults(planted);
        assert_eq!(faults.len(), 1, "{planted}: {faults:?}");
        assert!(faults[0].contains("#![allow("), "{faults:?}");
    }
    // An outer allow, with or without a reason, in a `cfg_attr` at any depth, and spread over
    // lines with blanks between `#`, `!` and `[`.
    for planted in [
        "#[allow(clippy::disallowed_methods)]\nfn f() {}\n",
        "#[allow(clippy::disallowed_methods, reason = \"x\")]\nfn f() {}\n",
        "#[cfg_attr(all(), cfg_attr(test, allow(clippy :: disallowed_methods, reason = \"x\")))]\n",
        "#\n!\n[\n    allow(\n        clippy::disallowed_methods,\n    )\n]\n",
    ] {
        assert_eq!(planted_growth_faults(planted).len(), 1, "{planted}");
    }
    // The groups that hold the lint, as an expect with a reason or an allow (B1 to B6).
    for planted in [
        "#[expect(clippy::style, reason = \"probe B5\")]\nfn b5() {}\n",
        "#[expect(clippy::all, reason = \"probe B4\")]\nfn b4() {}\n",
        "#[expect(warnings, reason = \"probe B6\")]\nfn b6() {}\n",
        "#[allow(clippy::all, reason = \"probe B1\")]\nfn b1() {}\n",
        "#[allow(clippy::style)]\nfn b2() {}\n",
        "#![allow(warnings)]\n",
    ] {
        let faults = planted_growth_faults(planted);
        assert_eq!(faults.len(), 1, "{planted}: {faults:?}");
        assert!(faults[0].contains("a group"), "{faults:?}");
    }
    // An expect without a reason, with an empty one or a blank one (B7, B8, B9).
    for planted in [
        "#[expect(clippy::disallowed_methods)]\nlet v = Vec::with_capacity(n);\n",
        "#[expect(clippy::disallowed_methods, reason = \"\")]\nfn b7() {}\n",
        "#[expect(clippy::disallowed_methods, reason = \" \")]\nfn b8() {}\n",
        "#[cfg_attr(not(test), expect(clippy::disallowed_methods))]\nfn b9() {}\n",
    ] {
        let faults = planted_growth_faults(planted);
        assert_eq!(faults.len(), 1, "{planted}: {faults:?}");
        assert!(faults[0].contains("no reason"), "{faults:?}");
    }
    // A `macro_rules!` body is text of the file (B10, B11), with its line.
    let planted = "macro_rules! quiet {\n    ($i:item) => {\n        #[allow(clippy::disallowed_methods)]\n        $i\n    };\n}\n";
    assert_eq!(
        planted_growth_faults(planted),
        [
            "src/planted.rs: line 3: #[allow(clippy::disallowed_methods)]: an allow of the listed lint"
        ]
    );
    let planted = "macro_rules! inner {\n    () => {\n        fn f() {\n            #![allow(clippy::disallowed_methods)]\n        }\n    };\n}\n";
    assert_eq!(planted_growth_faults(planted).len(), 1, "{planted}");
    // An attribute in a comment or a string is not one.
    for planted in [
        "// #[allow(clippy::disallowed_methods)]\nfn f() {}\n",
        "/* #![allow(warnings)] */\nfn f() {}\n",
        "/// #[expect(clippy::disallowed_methods)]\nfn f() {}\n",
        "let s = \"#[allow(clippy::disallowed_methods)]\";\n",
        "let s = r#\"#![allow(clippy::all)]\"#;\n",
        "let c = '#'; let d = '['; let e = r#ident;\n",
    ] {
        assert_eq!(
            planted_growth_faults(planted),
            Vec::<String>::new(),
            "{planted}"
        );
    }
    // The forms the rule takes: an expect with a reason, in one line or wrapped; an expect of
    // another lint; and the one allow of `lib.rs`, once.
    for planted in [
        "#[expect(clippy::disallowed_methods, reason = \"charged by `coerce_all`\")]\nlet v = 1;\n",
        "#[expect(\n    clippy::disallowed_methods,\n    reason = \"at most `TEXT_BYTES` bytes\"\n)]\nlet v = 1;\n",
        "#![expect(\n    dead_code,\n    reason = \"each test file uses a part\"\n)]\n",
        "#[expect(clippy::allow_attributes, reason = \"x\")]\n#[cfg_attr(test, expect(clippy::expect_used))]\n",
    ] {
        assert_eq!(
            planted_growth_faults(planted),
            Vec::<String>::new(),
            "{planted}"
        );
    }
    let lib = "#![no_std]\n#![cfg_attr(\n    test,\n    allow(\n        clippy::disallowed_methods,\n        reason = \"test code is not held to the charged list\"\n    )\n)]\n";
    assert_eq!(
        growth_faults_of("src/lib.rs", &lint_levels(lib), &mut 1),
        Vec::<String>::new()
    );
    let twice =
        format!("{lib}#![cfg_attr(test, allow(clippy::disallowed_methods, reason = \"x\"))]\n");
    assert_eq!(
        growth_faults_of("src/lib.rs", &lint_levels(&twice), &mut 1).len(),
        1
    );
    assert_eq!(
        growth_faults_of("src/other.rs", &lint_levels(lib), &mut 1).len(),
        1
    );
    // The reason is read from the source, not from the blanked code.
    let levels = lint_levels("#[expect(clippy::disallowed_methods, reason = \"a, b (c)\")]\n");
    assert_eq!(levels.len(), 1);
    assert_eq!(levels[0].reason.as_deref(), Some("a, b (c)"));
    assert_eq!(levels[0].lints, ["clippy::disallowed_methods"]);
    // The scan of the other crates finds the lint's name in any attribute text.
    let attrs = attributes(
        "#[macro_export]\nmacro_rules! quiet {\n    ($i:item) => {\n        #[allow(clippy::disallowed_methods)]\n        $i\n    };\n}\n// #[allow(clippy::disallowed_methods)]\n",
    );
    let naming: Vec<usize> = attrs
        .iter()
        .filter(|(_, _, body)| names_listed_lint(body))
        .map(|(line, _, _)| *line)
        .collect();
    assert_eq!(naming, [4]);
}

/// Whether the text of an attribute names `clippy::disallowed_methods`.
fn names_listed_lint(body: &str) -> bool {
    let compact: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    compact.contains("clippy::disallowed_methods")
}

/// `story:buffer-growth-outside-the-charged-list-is-listed`: a listed call of `crates/swell`
/// carries `#[expect(clippy::disallowed_methods, reason = "...")]` with a reason that is not
/// blank, or the gate is red. Clippy's own lints refuse an outer `allow` and an attribute
/// without a reason; this scan of every `.rs` file of `src/` and `tests/` refuses what they let
/// through: an inner `allow` of the lint in any position, an `allow` or `expect` of a group that
/// holds it, and an `expect` of it with a blank reason. The one `allow` the crate takes is the
/// attribute of `src/lib.rs` for test code, once.
#[test]
fn growth_attributes_are_expects_with_a_reason() {
    let dir = swell_dir();
    let mut files = Vec::new();
    rust_files(&dir.join("src"), &mut files);
    rust_files(&dir.join("tests"), &mut files);
    let mut faults = Vec::new();
    let mut expects = 0;
    let mut lib_allow_left = 1;
    for file in &files {
        let relative = file
            .strip_prefix(&dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let levels = lint_levels(&fs::read_to_string(file).unwrap());
        expects += levels
            .iter()
            .filter(|attribute| {
                attribute.level == "expect"
                    && attribute
                        .lints
                        .iter()
                        .any(|lint| lint == "clippy::disallowed_methods")
            })
            .count();
        faults.extend(growth_faults_of(&relative, &levels, &mut lib_allow_left));
    }
    assert!(files.len() >= 100, "{} file(s) scanned", files.len());
    assert!(expects >= 55, "{expects} expect(s) of the listed lint");
    assert_eq!(
        lib_allow_left, 0,
        "the allow of src/lib.rs for test code is not there"
    );
    assert!(faults.is_empty(), "{faults:#?}");
}

/// No `.rs` file of another crate of the workspace has an attribute that names
/// `clippy::disallowed_methods`, in a `macro_rules!` body or not: a `#[macro_export]` macro of
/// such a crate could put an `allow` on an item of `swell` that neither clippy lint reports
/// (adversary pass 2 on unit growth). A macro from outside the workspace cannot reach `swell`:
/// the engine takes no crate from outside the repository (HOST-1, above).
#[test]
fn no_other_crate_names_the_listed_lint() {
    let crates = repository_root().join("crates");
    let mut dirs: Vec<_> = fs::read_dir(&crates)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("src").is_dir() && !path.ends_with("swell"))
        .collect();
    dirs.sort();
    assert!(dirs.len() >= 5, "{dirs:?}");
    let mut faults = Vec::new();
    let mut files = 0;
    for dir in &dirs {
        for file in source_files(dir) {
            files += 1;
            let source = fs::read_to_string(&file).unwrap();
            for (line, _, body) in attributes(&source) {
                if names_listed_lint(&body) {
                    faults.push(format!("{}:{line}: #[{}]", file.display(), body.trim()));
                }
            }
        }
    }
    assert!(files >= 20, "{files} file(s) scanned");
    assert!(faults.is_empty(), "{faults:#?}");
}

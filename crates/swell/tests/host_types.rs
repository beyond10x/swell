//! The host types of the test binary (`story:slow-lane-holds-its-bound-on-the-slower-runner`).
//!
//! The engine is generic over its host (`impl<H: Host> Agent<H>`), so each type of
//! `crates/swell/tests/` that implements `Host` compiles the engine once more inside the one
//! test binary `swell::main`: at 19 such types that unit took 311 to 358 s at opt-level 1. The
//! tests hold three: the scripted type of `support/scripted.rs`, whose hooks are values, and
//! `TestHost` and `ProvidedHost` of `support/host.rs`, which keep the trait's provided defaults
//! and so test them.
//!
//! The guard below reads every source of the directory with the scanners of `source_scan.rs`
//! (`code_of` blanks comments and the contents of literals) and refuses, naming file and line:
//!
//! - an `impl` of `Host` (any path to the trait, any generics, on one line or wrapped) without
//!   the marker `// host-type: <reason>` on the line before it, read from the source as
//!   written, with a reason of the closed set `scripted`, `provided-defaults`, `own-constant`;
//! - a second `impl` marked `scripted`;
//! - an `impl` marked `provided-defaults` outside `tests/support/host.rs`, for a type other
//!   than `TestHost` and `ProvidedHost`, or a second time for one of them;
//! - an `impl` marked `own-constant` whose body does not define `SUPPORTED_IMPORT_ATTRIBUTES`,
//!   or defines it as the empty list (the trait's default, the value of the three types) or
//!   with the text of the value another such `impl` has, white space aside: the constant is
//!   per type, so a type of its own is one copy of the engine per value of the constant;
//! - an `impl` of `Host` with a type parameter (one copy of the engine per argument; a
//!   lifetime is none);
//! - `Host` renamed in a `use`;
//! - `Host` named inside the body of a `macro_rules!`, and an `impl` of a trait a macro
//!   variable names (`impl $t for ...`), through which a macro writes an `impl` of `Host`
//!   with the trait named at its call.
//!
//! `Host` as a bound (`impl<H: swell::Host> Trait for X<H>`, `fn f<H: Host>`, `impl Host` as
//! the type of an argument) is no type of its own and passes. The guard holds no count and no list a story edits. Each refusal is proven
//! on planted text below, as `source_scan.rs` proves its own scans.
#![cfg(test)]

use std::fs;

use crate::source_scan::scan::{code_of, ident_char, rust_files, swell_dir};

/// The reasons a marker may give.
const REASONS: [&str; 3] = ["scripted", "provided-defaults", "own-constant"];

/// The two types that keep the trait's provided defaults.
const PROVIDED_DEFAULTS: [&str; 2] = ["TestHost", "ProvidedHost"];

/// The file of those two types, as the guard names the files: relative to `crates/swell`.
const PROVIDED_DEFAULTS_FILE: &str = "tests/support/host.rs";

/// The trait's default of the constant, the empty list, with every blank taken out.
const EMPTY_LIST: &str = "&[]";

/// The text a marker line starts with.
const MARKER: &str = "// host-type: ";

/// The constant an `own-constant` type defines.
const CONSTANT: &str = "SUPPORTED_IMPORT_ATTRIBUTES";

/// The byte offsets of `word` in `code` as a whole identifier.
fn words(code: &str, word: &str) -> Vec<usize> {
    code.match_indices(word)
        .filter(|&(at, _)| {
            !ident_char(code[..at].chars().next_back())
                && !ident_char(code[at + word.len()..].chars().next())
        })
        .map(|(at, _)| at)
        .collect()
}

/// The line, from 1, of the byte offset `at` of `code`.
fn line_of(code: &str, at: usize) -> usize {
    code[..at].matches('\n').count() + 1
}

/// The offset of the `{` that opens the body of the item header starting at `from`, or `None`
/// when a `;` ends it. An `impl Trait` type (`fn g(host: impl Host) {}`) reaches the `{` of the
/// function with no `for` before it, so `split_header` takes it for no `impl` of a trait.
fn body_start(code: &str, from: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut angles = 0usize;
    let mut previous = ' ';
    for (k, c) in code[from..].char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            '<' => angles += 1,
            '>' if previous != '-' => angles = angles.saturating_sub(1),
            '{' if depth == 0 && angles == 0 => return Some(from + k),
            ';' if depth == 0 => return None,
            _ => {}
        }
        previous = c;
    }
    None
}

/// The offset just past the bracket that closes the one at `open`, counting `{` and `}` only.
fn block_end(code: &str, open: usize) -> usize {
    let mut depth = 0usize;
    for (k, c) in code[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return open + k + 1;
                }
            }
            _ => {}
        }
    }
    code.len()
}

/// The parts of `text` separated by the commas outside its brackets.
fn top_level_parts(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut previous = ' ';
    let mut start = 0;
    for (k, c) in text.char_indices() {
        match c {
            '(' | '[' | '{' | '<' => depth += 1,
            '>' if previous == '-' => {}
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(&text[start..k]);
                start = k + 1;
            }
            _ => {}
        }
        previous = c;
    }
    parts.push(&text[start..]);
    parts
}

/// A header `<generics> Trait for Type where ...` split into its generics (without the angle
/// brackets), its trait and its self type; `None` for an inherent `impl`.
fn split_header(header: &str) -> Option<(&str, &str, &str)> {
    let header = header.trim_start();
    let (generics, rest) = if header.starts_with('<') {
        let mut depth = 0usize;
        let mut previous = ' ';
        let mut end = header.len();
        for (k, c) in header.char_indices() {
            match c {
                '<' => depth += 1,
                '>' if previous != '-' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        end = k;
                        break;
                    }
                }
                _ => {}
            }
            previous = c;
        }
        (&header[1..end], header.get(end + 1..).unwrap_or(""))
    } else {
        ("", header)
    };
    let mut angles = 0usize;
    let mut previous = ' ';
    let mut found = None;
    for (k, c) in rest.char_indices() {
        match c {
            '<' => angles += 1,
            '>' if previous != '-' => angles = angles.saturating_sub(1),
            'f' if angles == 0
                && rest[k..].starts_with("for")
                && !ident_char(rest[..k].chars().next_back())
                && !ident_char(rest[k + 3..].chars().next()) =>
            {
                found = Some(k);
                break;
            }
            _ => {}
        }
        previous = c;
    }
    let at = found?;
    let self_type = &rest[at + 3..];
    let self_type = match words(self_type, "where").first() {
        Some(&end) => &self_type[..end],
        None => self_type,
    };
    Some((generics, rest[..at].trim(), self_type.trim()))
}

/// The last segment of a path, without its generic arguments: `Host` of `::swell::Host`.
fn last_segment(path: &str) -> &str {
    let path = path.split('<').next().unwrap_or("");
    path.rsplit("::").next().unwrap_or("").trim()
}

/// One `impl` of `Host` of one file.
struct HostImpl {
    file: String,
    line: usize,
    reason: Option<String>,
    self_type: String,
    type_parameter: bool,
    /// Whether the body defines the constant, and the text of its value without blanks.
    constant: Option<String>,
}

/// The text of the value of the constant defined at `at` of `code`, read from the lines of
/// `source` that hold it, with every blank taken out.
fn constant_value(code: &str, source: &str, at: usize) -> String {
    let first = line_of(code, at);
    let end = code[at..].find(';').map_or(code.len(), |k| at + k);
    let last = line_of(code, end);
    let lines: Vec<&str> = source
        .lines()
        .skip(first - 1)
        .take(last + 1 - first)
        .collect();
    let text = lines.join("\n");
    let after_name = text
        .find(CONSTANT)
        .map_or("", |k| &text[k + CONSTANT.len()..]);
    let value = after_name.split_once('=').map_or("", |(_, value)| value);
    let value = value.rfind(';').map_or(value, |k| &value[..k]);
    value.chars().filter(|c| !c.is_whitespace()).collect()
}

/// The `impl`s of `Host` of one file, and the faults of its `use` items and macro bodies.
fn scan_file(file: &str, source: &str, impls: &mut Vec<HostImpl>, faults: &mut Vec<String>) {
    let code = code_of(source);
    let raw: Vec<&str> = source.lines().collect();
    for at in words(&code, "impl") {
        let Some(open) = body_start(&code, at + "impl".len()) else {
            continue;
        };
        let Some((generics, trait_path, self_type)) = split_header(&code[at + 4..open]) else {
            continue;
        };
        let line = line_of(&code, at);
        if trait_path.starts_with('$') && !trait_path.starts_with("$crate::") {
            faults.push(format!(
                "{file}:{line}: an impl of the trait the macro variable {trait_path} names: a \
                 macro called with swell::Host writes an impl of Host that carries no marker; \
                 name the trait in the macro's body"
            ));
            continue;
        }
        if last_segment(trait_path) != "Host" {
            continue;
        }
        let reason = line
            .checked_sub(2)
            .and_then(|k| raw.get(k))
            .and_then(|before| before.trim().strip_prefix(MARKER))
            .map(|reason| reason.trim().to_string());
        let type_parameter = top_level_parts(generics).iter().any(|part| {
            let part = part.trim();
            !part.is_empty() && !part.starts_with('\'')
        });
        let body = &code[open..block_end(&code, open)];
        let constant = words(body, CONSTANT)
            .into_iter()
            .find(|&k| body[..k].trim_end().ends_with("const"))
            .map(|k| constant_value(&code, source, open + k));
        impls.push(HostImpl {
            file: file.to_string(),
            line,
            reason,
            self_type: last_segment(self_type).to_string(),
            type_parameter,
            constant,
        });
    }
    for at in words(&code, "use") {
        let end = code[at..].find(';').map_or(code.len(), |k| at + k);
        let item = &code[at..end];
        for k in words(item, "Host") {
            let after = item[k + "Host".len()..].trim_start();
            if after.starts_with("as") {
                faults.push(format!(
                    "{file}:{}: Host renamed in a use: name the trait Host, so the guard of \
                     host_types.rs sees each impl of it",
                    line_of(&code, at + k)
                ));
            }
        }
    }
    for at in words(&code, "macro_rules") {
        if !code[at + "macro_rules".len()..].starts_with('!') {
            continue;
        }
        let Some(open) = code[at..].find(['{', '(', '[']).map(|k| at + k) else {
            continue;
        };
        let (opening, closing) = match code[open..].chars().next() {
            Some('(') => ('(', ')'),
            Some('[') => ('[', ']'),
            _ => ('{', '}'),
        };
        let mut depth = 0usize;
        let mut end = code.len();
        for (k, c) in code[open..].char_indices() {
            if c == opening {
                depth += 1;
            } else if c == closing {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    end = open + k;
                    break;
                }
            }
        }
        for k in words(&code[open..end], "Host") {
            faults.push(format!(
                "{file}:{}: Host named inside the body of a macro_rules!: an impl a macro writes \
                 carries no marker the guard of host_types.rs reads",
                line_of(&code, open + k)
            ));
        }
    }
}

/// The faults of the sources `files` (a name for the messages and the text of each), in the
/// order of the files and of their lines.
fn faults(files: &[(String, String)]) -> Vec<String> {
    let mut impls = Vec::new();
    let mut faults = Vec::new();
    for (file, source) in files {
        scan_file(file, source, &mut impls, &mut faults);
    }
    let mut scripted: Option<&HostImpl> = None;
    let mut own_constants: Vec<&HostImpl> = Vec::new();
    let mut provided: Vec<&HostImpl> = Vec::new();
    for found in &impls {
        let place = format!("{}:{}", found.file, found.line);
        if found.type_parameter {
            faults.push(format!(
                "{place}: an impl of Host with a type parameter: each argument is one more copy \
                 of the engine in swell::main"
            ));
        }
        let Some(reason) = &found.reason else {
            faults.push(format!(
                "{place}: an impl of Host for {} without the marker `{MARKER}<reason>` on the \
                 line before it; fold the type into the scripted type of support/scripted.rs",
                found.self_type
            ));
            continue;
        };
        match reason.as_str() {
            "scripted" => match scripted {
                Some(first) => faults.push(format!(
                    "{place}: a second impl of Host marked scripted, for {}; the first is {}:{} \
                     ({})",
                    found.self_type, first.file, first.line, first.self_type
                )),
                None => scripted = Some(found),
            },
            "provided-defaults" => {
                if found.file != PROVIDED_DEFAULTS_FILE {
                    faults.push(format!(
                        "{place}: an impl of Host marked provided-defaults for {} outside \
                         {PROVIDED_DEFAULTS_FILE}; only TestHost and ProvidedHost of that file \
                         keep the provided defaults",
                        found.self_type
                    ));
                } else if !PROVIDED_DEFAULTS.contains(&found.self_type.as_str()) {
                    faults.push(format!(
                        "{place}: an impl of Host marked provided-defaults for {}; only TestHost \
                         and ProvidedHost keep the provided defaults",
                        found.self_type
                    ));
                } else if let Some(first) = provided
                    .iter()
                    .find(|first| first.self_type == found.self_type)
                {
                    faults.push(format!(
                        "{place}: a second impl of Host marked provided-defaults for {}; the \
                         first is {}:{}",
                        found.self_type, first.file, first.line
                    ));
                } else {
                    provided.push(found);
                }
            }
            "own-constant" => match &found.constant {
                None => faults.push(format!(
                    "{place}: an impl of Host marked own-constant for {} that does not define \
                     {CONSTANT}",
                    found.self_type
                )),
                Some(value) if value == EMPTY_LIST => faults.push(format!(
                    "{place}: an impl of Host marked own-constant for {} whose {CONSTANT} is \
                     the empty list, the trait's default, which the scripted type, TestHost \
                     and ProvidedHost have: one type per value",
                    found.self_type
                )),
                Some(value) => {
                    match own_constants
                        .iter()
                        .find(|other| other.constant.as_ref() == Some(value))
                    {
                        Some(other) => faults.push(format!(
                            "{place}: an impl of Host marked own-constant for {} with the value \
                             of {CONSTANT} of {} at {}:{}: one type per value",
                            found.self_type, other.self_type, other.file, other.line
                        )),
                        None => own_constants.push(found),
                    }
                }
            },
            other => faults.push(format!(
                "{place}: the marker of the impl of Host for {} gives the reason {other:?}, not \
                 one of {}",
                found.self_type,
                REASONS.join(", ")
            )),
        }
    }
    faults
}

/// The faults of one planted file, `tests/planted.rs`.
fn planted(source: &str) -> Vec<String> {
    faults(&[("tests/planted.rs".to_string(), source.to_string())])
}

/// The faults of one planted file in the place of `tests/support/host.rs`.
fn in_support(source: &str) -> Vec<String> {
    faults(&[(PROVIDED_DEFAULTS_FILE.to_string(), source.to_string())])
}

/// The sources of `crates/swell/tests/` hold the three host types and no other: each `impl` of
/// `Host` carries its marker, one is scripted, and the two of the provided defaults are
/// `TestHost` and `ProvidedHost`.
#[test]
fn the_tests_hold_the_scripted_host_type_and_the_two_of_the_provided_defaults() {
    let root = swell_dir();
    let mut paths = Vec::new();
    rust_files(&root.join("tests"), &mut paths);
    let files: Vec<(String, String)> = paths
        .iter()
        .map(|path| {
            let name = path.strip_prefix(&root).unwrap().display().to_string();
            (name, fs::read_to_string(path).unwrap())
        })
        .collect();
    assert!(files.len() > 100, "{} files read", files.len());
    let found = faults(&files);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// An `impl` without the marker is refused with its file and line.
#[test]
fn a_plain_impl_without_the_marker_is_refused() {
    let source = "use swell::Host;\n\nstruct Mine;\n\nimpl Host for Mine {\n    fn f() {}\n}\n";
    assert_eq!(
        planted(source),
        [
            "tests/planted.rs:5: an impl of Host for Mine without the marker `// host-type: \
          <reason>` on the line before it; fold the type into the scripted type of \
          support/scripted.rs"
        ]
    );
}

/// A generic `impl` is refused for its type parameter, its marker given.
#[test]
fn a_generic_impl_is_refused() {
    let source = "// host-type: scripted\nimpl<T: Default + 'static> swell::Host for Mine<T> {}\n";
    assert_eq!(
        planted(source),
        [
            "tests/planted.rs:2: an impl of Host with a type parameter: each argument is one more \
          copy of the engine in swell::main"
        ]
    );
    let source = "// host-type: scripted\nimpl<const N: usize> swell::Host for Mine<N> {}\n";
    assert_eq!(planted(source).len(), 1, "{:?}", planted(source));
}

/// The trait named by the path `::swell::Host` is the trait.
#[test]
fn an_impl_of_the_path_from_the_crate_root_is_refused() {
    let source = "struct Mine;\nimpl ::swell::Host for Mine {}\n";
    assert_eq!(
        planted(source),
        [
            "tests/planted.rs:2: an impl of Host for Mine without the marker `// host-type: \
          <reason>` on the line before it; fold the type into the scripted type of \
          support/scripted.rs"
        ]
    );
}

/// An `impl` whose trait and `for` stand on lines of their own is refused at the line of the
/// `impl`, and a marker above a comment line does not count.
#[test]
fn a_wrapped_impl_is_refused_at_its_first_line() {
    let source = "// host-type: scripted\n// a comment between\nimpl\n    swell :: Host\n    for\n    Mine\n{\n}\n";
    assert_eq!(
        planted(source),
        [
            "tests/planted.rs:3: an impl of Host for Mine without the marker `// host-type: \
          <reason>` on the line before it; fold the type into the scripted type of \
          support/scripted.rs"
        ]
    );
}

/// `Host` renamed in a `use`, alone or in a group, is refused.
#[test]
fn a_renamed_use_is_refused() {
    let source = "use swell::Host as Engine;\nuse swell::{\n    Agent,\n    Host as Other,\n};\n";
    let message = |line: usize| {
        format!(
            "tests/planted.rs:{line}: Host renamed in a use: name the trait Host, so the guard of \
             host_types.rs sees each impl of it"
        )
    };
    assert_eq!(planted(source), [message(1), message(4)]);
}

/// `Host` inside a `macro_rules!` body is refused, the impl the macro writes too.
#[test]
fn host_in_a_macro_body_is_refused() {
    let source = "macro_rules! host {\n    ($t:ty) => {\n        // host-type: scripted\n        impl swell::Host for $t {}\n    };\n}\n";
    assert_eq!(
        planted(source),
        [
            "tests/planted.rs:4: Host named inside the body of a macro_rules!: an impl a macro \
          writes carries no marker the guard of host_types.rs reads"
        ]
    );
}

/// A marker with a reason outside the closed set is refused.
#[test]
fn an_unknown_reason_is_refused() {
    let source = "// host-type: faster\nimpl Host for Mine {}\n";
    assert_eq!(
        planted(source),
        [
            "tests/planted.rs:2: the marker of the impl of Host for Mine gives the reason \
          \"faster\", not one of scripted, provided-defaults, own-constant"
        ]
    );
}

/// A second `scripted` is refused, naming the first, across files.
#[test]
fn two_scripted_impls_are_refused() {
    let first = "// host-type: scripted\nimpl Host for Scripted {}\n";
    let second = "\n// host-type: scripted\nimpl Host for Again {}\n";
    let found = faults(&[
        ("tests/a.rs".to_string(), first.to_string()),
        ("tests/b.rs".to_string(), second.to_string()),
    ]);
    assert_eq!(
        found,
        [
            "tests/b.rs:3: a second impl of Host marked scripted, for Again; the first is \
          tests/a.rs:2 (Scripted)"
        ]
    );
}

/// `provided-defaults` on a third type is refused.
#[test]
fn a_third_provided_defaults_is_refused() {
    let source = "// host-type: provided-defaults\nimpl Host for TestHost {}\n\
                  // host-type: provided-defaults\nimpl Host for ProvidedHost {}\n\
                  // host-type: provided-defaults\nimpl Host for Third {}\n";
    assert_eq!(
        in_support(source),
        [
            "tests/support/host.rs:6: an impl of Host marked provided-defaults for Third; only \
             TestHost and ProvidedHost keep the provided defaults"
        ]
    );
}

/// `provided-defaults` outside `tests/support/host.rs` is refused, also for `TestHost`, and a
/// second `TestHost` in that file is refused, naming the first; the exact messages of the case
/// below it.
#[test]
fn provided_defaults_is_taken_once_each_and_only_in_the_support_file() {
    let source = "// host-type: provided-defaults\nimpl Host for TestHost {}\n";
    assert_eq!(
        planted(source),
        [
            "tests/planted.rs:2: an impl of Host marked provided-defaults for TestHost outside \
             tests/support/host.rs; only TestHost and ProvidedHost of that file keep the \
             provided defaults"
        ]
    );
    let source = "// host-type: provided-defaults\nimpl Host for TestHost {}\n\
                  // host-type: provided-defaults\nimpl Host for TestHost {}\n";
    assert_eq!(
        in_support(source),
        [
            "tests/support/host.rs:4: a second impl of Host marked provided-defaults for \
             TestHost; the first is tests/support/host.rs:2"
        ]
    );
}

/// `own-constant` without the constant is refused.
#[test]
fn an_own_constant_without_the_constant_is_refused() {
    let source = "// host-type: own-constant\nimpl Host for Loader {\n    fn f() {}\n}\n";
    assert_eq!(
        planted(source),
        [
            "tests/planted.rs:2: an impl of Host marked own-constant for Loader that does not \
          define SUPPORTED_IMPORT_ATTRIBUTES"
        ]
    );
}

/// Two `own-constant` with one value of the constant are refused, however the value is laid
/// out; two with different values pass.
#[test]
fn two_own_constants_with_one_value_are_refused() {
    let source = "// host-type: own-constant\nimpl Host for Loader {\n    const SUPPORTED_IMPORT_ATTRIBUTES: &'static [&'static str] = &[\"type\"];\n}\n\
                  // host-type: own-constant\nimpl Host for Again {\n    const SUPPORTED_IMPORT_ATTRIBUTES: &'static [&'static str] =\n        &[ \"type\" ];\n}\n";
    assert_eq!(
        planted(source),
        [
            "tests/planted.rs:6: an impl of Host marked own-constant for Again with the value of \
          SUPPORTED_IMPORT_ATTRIBUTES of Loader at tests/planted.rs:2: one type per value"
        ]
    );
    let source = "// host-type: own-constant\nimpl Host for Loader {\n    const SUPPORTED_IMPORT_ATTRIBUTES: &'static [&'static str] = &[\"type\"];\n}\n\
                  // host-type: own-constant\nimpl Host for Other {\n    const SUPPORTED_IMPORT_ATTRIBUTES: &'static [&'static str] = &[\"type\", \"mode\"];\n}\n";
    assert_eq!(planted(source), Vec::<String>::new());
}

/// `Host` as a bound is no type of its own and passes, as do the three marked types, an
/// `impl Trait` argument and an inherent `impl`.
#[test]
fn host_as_a_bound_passes() {
    let source = "use swell::Host;\n\
                  impl<H: swell::Host> Trait for Wrapper<H> {}\n\
                  impl<H> Trait for Other<H> where H: Host {}\n\
                  fn f<H: Host>(agent: &mut swell::Agent<H>) {}\n\
                  fn g(host: impl Host) {}\n\
                  impl Scripted {\n    fn new() {}\n}\n\
                  // host-type: scripted\nimpl Host for Scripted {}\n";
    let support = "// host-type: provided-defaults\nimpl swell::Host for TestHost {}\n\
                   // host-type: provided-defaults\nimpl Host for ProvidedHost {}\n";
    let found = faults(&[
        (PROVIDED_DEFAULTS_FILE.to_string(), support.to_string()),
        ("tests/planted.rs".to_string(), source.to_string()),
    ]);
    assert_eq!(found, Vec::<String>::new());
}

/// A lifetime parameter is no type parameter: the engine is compiled once for the type,
/// whatever the lifetime.
#[test]
fn a_lifetime_parameter_is_no_type_parameter() {
    let source = "// host-type: scripted\nimpl<'a> Host for Scripted {}\n";
    assert_eq!(planted(source), Vec::<String>::new());
}

/// A body that reads `SUPPORTED_IMPORT_ATTRIBUTES` without defining it does not define it.
#[test]
fn an_own_constant_that_only_reads_the_constant_is_refused() {
    let source = "// host-type: own-constant\nimpl Host for Loader {\n    fn n() -> usize {\n        Self::SUPPORTED_IMPORT_ATTRIBUTES.len()\n    }\n}\n";
    assert_eq!(
        planted(source),
        [
            "tests/planted.rs:2: an impl of Host marked own-constant for Loader that does not \
             define SUPPORTED_IMPORT_ATTRIBUTES"
        ]
    );
}

/// A `where` clause after the self type is not part of its name.
#[test]
fn a_where_clause_is_not_part_of_the_type_name() {
    let source = "// host-type: provided-defaults\nimpl Host for TestHost where TestHost: Sized {}\n\
                  // host-type: provided-defaults\nimpl Host for ProvidedHost {}\n";
    assert_eq!(in_support(source), Vec::<String>::new());
}

/// The empty list as the value of an `own-constant`, written with blanks, is refused with its
/// message.
#[test]
fn an_own_constant_with_the_empty_list_written_with_blanks_is_refused() {
    let source = "// host-type: own-constant\nimpl Host for Loader {\n    const SUPPORTED_IMPORT_ATTRIBUTES: &'static [&'static str] = & [\n    ];\n}\n";
    assert_eq!(
        planted(source),
        [
            "tests/planted.rs:2: an impl of Host marked own-constant for Loader whose \
             SUPPORTED_IMPORT_ATTRIBUTES is the empty list, the trait's default, which the \
             scripted type, TestHost and ProvidedHost have: one type per value"
        ]
    );
}

/// The message of an `impl` of a trait a macro variable names; `$crate::` is no variable.
#[test]
fn an_impl_of_a_macro_variable_names_the_variable() {
    let source = "macro_rules! host_type {\n    ($trait:path, $t:ident) => {\n        impl $trait for $t {}\n        impl $crate::Other for $t {}\n    };\n}\n";
    assert_eq!(
        planted(source),
        [
            "tests/planted.rs:3: an impl of the trait the macro variable $trait names: a macro \
             called with swell::Host writes an impl of Host that carries no marker; name the \
             trait in the macro's body"
        ]
    );
}

/// An `impl` of `Host` that a `macro_rules!` writes for a trait it takes as an argument is an
/// `impl` of `Host` without a marker: the trait is named at the call, not in the body.
#[test]
fn an_impl_a_macro_writes_for_the_trait_given_as_its_argument_is_refused() {
    let source = "macro_rules! host_type {\n    ($trait:path, $t:ident) => {\n        impl $trait for $t {}\n    };\n}\n\nstruct Mine;\nhost_type!(swell::Host, Mine);\n";
    let found = planted(source);
    assert!(
        found
            .iter()
            .any(|fault| fault.starts_with("tests/planted.rs:")),
        "the impl of Host the macro writes for Mine passes the guard: {found:?}"
    );
}

/// `provided-defaults` on a second type named `TestHost`, in a module of its own, is a fourth
/// type of `Host` in the test binary.
#[test]
fn a_second_type_named_test_host_marked_provided_defaults_is_refused() {
    let support = "// host-type: provided-defaults\nimpl Host for TestHost {}\n\
                   // host-type: provided-defaults\nimpl Host for ProvidedHost {}\n";
    let other = "mod again {\n    pub struct TestHost;\n    // host-type: provided-defaults\n    impl swell::Host for TestHost {}\n}\n";
    let found = faults(&[
        ("tests/support/host.rs".to_string(), support.to_string()),
        ("tests/planted.rs".to_string(), other.to_string()),
    ]);
    assert!(
        found
            .iter()
            .any(|fault| fault.starts_with("tests/planted.rs:4:")),
        "a fourth type of Host passes the guard: {found:?}"
    );
}

/// `own-constant` is for a type whose `SUPPORTED_IMPORT_ATTRIBUTES` no other type has; the
/// trait's default `&[]` is the value of the scripted type, `TestHost` and `ProvidedHost`.
#[test]
fn an_own_constant_with_the_value_of_the_default_is_refused() {
    let source = "// host-type: own-constant\nimpl Host for Loader {\n    const SUPPORTED_IMPORT_ATTRIBUTES: &'static [&'static str] = &[];\n}\n";
    let found = planted(source);
    assert!(
        found
            .iter()
            .any(|fault| fault.starts_with("tests/planted.rs:2:")),
        "an own-constant type with the default value passes the guard: {found:?}"
    );
}

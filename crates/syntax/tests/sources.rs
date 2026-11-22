//! The tests of `swell-syntax` over source text: every file under `tests/sources/` and the cases
//! that need code units a UTF-8 file cannot hold, a nesting a file would not show, or a look into
//! the tree.
//!
//! Each source file starts with a comment line `// <anchor>: <result>`: the ECMA-262 clause it
//! exercises and what parsing it gives, `parses`, `SyntaxError "<message>"` or
//! `refused "<message>"`. A directory with no `.js` file fails its test (`AGENTS.md`, "A check
//! that finds nothing to read fails").

use std::fs;
use std::path::{Path, PathBuf};

use swell_syntax::ast::{Expr, FunctionId, FunctionKind, Stmt};
use swell_syntax::{
    MessageKind, ParseOptions, Script, SyntaxError, message_kind, parse_dynamic_function,
    parse_eval, parse_script,
};

const LIMIT: u32 = 1_000;

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn parse_units(source: &[u16], max_nesting: u32) -> Result<Script, Vec<SyntaxError>> {
    parse_script(source, ParseOptions { max_nesting })
}

fn parse(text: &str) -> Result<Script, Vec<SyntaxError>> {
    parse_units(&units(text), LIMIT)
}

fn messages(errors: &[SyntaxError]) -> Vec<String> {
    errors.iter().map(|e| e.message.clone()).collect()
}

/// The `.js` files of one directory under `tests/sources/`, sorted; fails when there is none.
fn sources(directory: &str) -> Result<Vec<(PathBuf, String)>, String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/sources")
        .join(directory);
    let mut files = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if path.extension().is_some_and(|x| x == "js") {
            files.push(path);
        }
    }
    files.sort();
    if files.is_empty() {
        return Err(format!("{}: no .js file to read", dir.display()));
    }
    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok((path, text))
        })
        .collect()
}

/// The first line of `text` when it is a `HashbangComment` (`#sec-hashbang`), with its line
/// terminator: such a comment stands only at the very start of a source, so a source that has
/// one carries its header comment on the second line.
fn hashbang_line(text: &str) -> &str {
    if !text.starts_with("#!") {
        return "";
    }
    match text.find('\n') {
        Some(end) => &text[..=end],
        None => text,
    }
}

/// `text` with a `"use strict";` prologue, after its hashbang comment if it has one.
fn with_use_strict(text: &str) -> String {
    let hashbang = hashbang_line(text);
    format!("{hashbang}\"use strict\";\n{}", &text[hashbang.len()..])
}

/// The header of a source file: its anchor and the rest of its first line after `: ` (of its
/// second line after a hashbang comment).
fn header(path: &Path, text: &str) -> Result<(String, String), String> {
    let first = text[hashbang_line(text).len()..]
        .lines()
        .next()
        .unwrap_or("");
    let rest = first
        .strip_prefix("// ")
        .ok_or_else(|| format!("{}: no header comment", path.display()))?;
    let (anchor, result) = rest
        .split_once(": ")
        .ok_or_else(|| format!("{}: header is not `// <anchor>: <result>`", path.display()))?;
    if !anchor.starts_with("sec-") {
        return Err(format!(
            "{}: the header names no clause anchor",
            path.display()
        ));
    }
    Ok((anchor.to_string(), result.to_string()))
}

/// The quoted message of a header result `<word> "<message>"`.
fn quoted(path: &Path, result: &str, word: &str) -> Result<String, String> {
    result
        .strip_prefix(word)
        .and_then(|r| r.strip_prefix(" \""))
        .and_then(|r| r.strip_suffix('"'))
        .map(str::to_string)
        .ok_or_else(|| {
            format!(
                "{}: header result is not {word} \"<message>\"",
                path.display()
            )
        })
}

/// Whether a comment line `// sloppy-only` follows the header: the source is valid only as
/// non-strict code (a `with` statement, `#sec-with-statement-static-semantics-early-errors`).
fn sloppy_only(text: &str) -> bool {
    text[hashbang_line(text).len()..]
        .lines()
        .skip(1)
        .take_while(|line| line.starts_with("//"))
        .any(|line| line.trim() == "// sloppy-only")
}

/// Every source under `valid/` parses as written and, with a `"use strict";` prologue, as
/// strict code; a `// sloppy-only` source is instead an early error (not a refusal) in strict
/// code.
#[test]
fn valid_sources_parse_in_both_modes() {
    let files = sources("valid").unwrap();
    let mut sloppy = 0;
    for (path, text) in &files {
        let (_, result) = header(path, text).unwrap();
        assert_eq!(result, "parses", "{}", path.display());
        match parse(text) {
            Ok(script) => assert!(
                !script.strict || text.contains("\"use strict\""),
                "{}: strict without a directive",
                path.display()
            ),
            Err(errors) => panic!("{} as written: {:?}", path.display(), errors),
        }
        let strict = with_use_strict(text);
        if sloppy_only(text) {
            sloppy += 1;
            let errors = parse(&strict)
                .err()
                .unwrap_or_else(|| panic!("{}: parses as strict code", path.display()));
            assert!(
                errors
                    .iter()
                    .all(|e| message_kind(&e.message) == MessageKind::Grammar),
                "{}: {errors:?}",
                path.display()
            );
            continue;
        }
        match parse(&strict) {
            Ok(script) => assert!(
                script.strict,
                "{}: not strict with the prologue",
                path.display()
            ),
            Err(errors) => panic!("{} with \"use strict\": {:?}", path.display(), errors),
        }
    }
    println!(
        "valid: {} file(s), each parsed in both modes but {sloppy} sloppy-only",
        files.len()
    );
}

#[test]
fn early_error_sources_fail_with_their_message() {
    let files = sources("early-errors").unwrap();
    for (path, text) in &files {
        let (_, result) = header(path, text).unwrap();
        let expected = quoted(path, &result, "SyntaxError").unwrap();
        match parse(text) {
            Ok(_) => panic!(
                "{}: parsed, expected SyntaxError {expected:?}",
                path.display()
            ),
            Err(errors) => {
                assert_eq!(
                    messages(&errors).first(),
                    Some(&expected),
                    "{}: {:?}",
                    path.display(),
                    errors
                );
                assert!(
                    !errors[0].message.contains("is not supported before slice"),
                    "{}: an early error must not be a refusal",
                    path.display()
                );
                assert!(errors[0].line >= 1 && errors[0].column >= 1);
            }
        }
    }
    println!("early-errors: {} file(s)", files.len());
}

#[test]
fn refused_sources_name_their_slice() {
    let files = sources("refused").unwrap();
    for (path, text) in &files {
        let (_, result) = header(path, text).unwrap();
        let expected = quoted(path, &result, "refused").unwrap();
        let (construct, slice) = expected
            .rsplit_once(" is not supported before slice ")
            .unwrap_or_else(|| panic!("{}: not a refusal message: {expected:?}", path.display()));
        assert!(!construct.is_empty(), "{}", path.display());
        let slice: u32 = slice
            .parse()
            .unwrap_or_else(|_| panic!("{}: slice {slice:?} is no number", path.display()));
        assert!(
            (2..=19).contains(&slice),
            "{}: slice {slice}",
            path.display()
        );
        match parse(text) {
            Ok(_) => panic!("{}: parsed, expected {expected:?}", path.display()),
            Err(errors) => assert_eq!(
                messages(&errors),
                vec![expected.clone()],
                "{}",
                path.display()
            ),
        }
    }
    println!("refused: {} file(s)", files.len());
}

/// A hashbang comment (`#sec-hashbang`) is a comment at the very start of a source only: it
/// may run to the end of the input, it holds no line terminator of its own, and after anything
/// else, white space included, `#!` is an error.
#[test]
fn hashbang_comment_only_at_the_start() {
    for text in [
        "#!",
        "#! a b c",
        "#!\nx = 1;",
        "#!/x\r\nx = 1;",
        "#!\u{2028}x",
    ] {
        assert!(parse(text).is_ok(), "{text:?}: {:?}", parse(text).err());
    }
    for text in [" #!\n", "\n#!\n", "x = 1;\n#!\n", "/**/#!\n"] {
        let errors = parse(text)
            .err()
            .unwrap_or_else(|| panic!("{text:?} parsed"));
        assert_eq!(
            messages(&errors),
            vec!["hashbang comment not at the start of the source".to_string()],
            "{text:?}"
        );
    }
}

/// The message a regular expression literal throws before slice 5 is a refusal of slice 5 as
/// [`message_kind`] reads it (`docs/design/engine.md`, decision 11, "Refusals").
#[test]
fn message_kind_reads_the_regular_expression_literal_refusal() {
    assert_eq!(
        swell_syntax::REGEXP_REFUSAL,
        "regular expression literal is not supported before slice 5"
    );
    assert_eq!(
        message_kind(swell_syntax::REGEXP_REFUSAL),
        MessageKind::Refusal { slice: 5 }
    );
}

/// `message_kind` tells the three kinds of message apart from the text alone, as a host reads it
/// from a `SyntaxError` object: every refusal under `refused/` is a refusal of the slice its
/// header names, every message of a source under `early-errors/` is a grammar or early error,
/// and the nesting limit is the nesting limit.
#[test]
fn message_kind_tells_refusals_from_early_errors() {
    let refused = sources("refused").unwrap();
    for (path, text) in &refused {
        let (_, result) = header(path, text).unwrap();
        let expected = quoted(path, &result, "refused").unwrap();
        let slice: u32 = expected
            .rsplit_once(' ')
            .and_then(|(_, n)| n.parse().ok())
            .unwrap_or_else(|| panic!("{}: no slice in {expected:?}", path.display()));
        let errors = parse(text)
            .err()
            .unwrap_or_else(|| panic!("{}: parsed", path.display()));
        for error in &errors {
            assert_eq!(
                message_kind(&error.message),
                MessageKind::Refusal { slice },
                "{}: {:?}",
                path.display(),
                error.message
            );
        }
    }
    let early = sources("early-errors").unwrap();
    let mut messages_seen = 0;
    for (path, text) in &early {
        let errors = parse(text)
            .err()
            .unwrap_or_else(|| panic!("{}: parsed", path.display()));
        for error in &errors {
            messages_seen += 1;
            assert_eq!(
                message_kind(&error.message),
                MessageKind::Grammar,
                "{}: {:?}",
                path.display(),
                error.message
            );
        }
    }
    let errors = parse_units(&units("[[[1]]];"), 2).err().unwrap();
    assert_eq!(
        message_kind(&errors[0].message),
        MessageKind::Nesting { limit: 2 }
    );
    for text in [
        " is not supported before slice 2",
        "x is not supported before slice ",
        "x is not supported before slice 2a",
        "nesting deeper than parser_nesting (2",
        "nesting deeper than parser_nesting ()",
    ] {
        assert_eq!(message_kind(text), MessageKind::Grammar, "{text:?}");
    }
    println!(
        "message kinds: {} refused file(s), {} early-error file(s) with {messages_seen} message(s)",
        refused.len(),
        early.len()
    );
}

/// Set in the environment of a child process a test starts: the nesting limit the child runs its
/// test body with.
const STACK_CHILD: &str = "SWELL_SYNTAX_STACK_CHILD";

/// Runs `body` with nesting limit `limit` on a thread whose stack is
/// `ParseOptions::stack_bytes` for that limit, in a child process that runs only the test
/// `test`. A stack overflow aborts the process, which no test can catch in its own process; in
/// the child it fails this test alone, with the child's output.
fn on_bounded_stack(test: &str, limit: u32, body: fn(u32)) -> Result<(), String> {
    let stack = ParseOptions { max_nesting: limit }.stack_bytes();
    let done = format!("{STACK_CHILD}: {test} at {limit} done");
    if let Some(value) = std::env::var_os(STACK_CHILD) {
        if value.to_str() != Some(limit.to_string().as_str()) {
            return Ok(());
        }
        std::thread::Builder::new()
            .stack_size(stack)
            .spawn(move || body(limit))
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| format!("{test} at {limit}: the body failed"))?;
        println!("{done}");
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let out = std::process::Command::new(exe)
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(STACK_CHILD, limit.to_string())
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    if out.status.success() && stdout.contains(&done) {
        return Ok(());
    }
    Err(format!(
        "{test} with parser_nesting {limit} on a stack of {stack} bytes: {}\n{stdout}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    ))
}

#[test]
fn nesting_of_100000_brackets_is_one_error() {
    on_bounded_stack("nesting_of_100000_brackets_is_one_error", LIMIT, |limit| {
        let mut source = vec![u16::from(b'['); 100_000];
        source.extend(std::iter::repeat_n(u16::from(b']'), 100_000));
        let errors = parse_units(&source, limit).err().unwrap();
        assert_eq!(
            messages(&errors),
            vec!["nesting deeper than parser_nesting (1000)".to_string()]
        );
        let mut parens = vec![u16::from(b'('); 100_000];
        parens.push(u16::from(b'0'));
        parens.extend(std::iter::repeat_n(u16::from(b')'), 100_000));
        let errors = parse_units(&parens, limit).err().unwrap();
        assert_eq!(errors.len(), 1);
        // A long chain built by a loop, not by recursion, is bounded the same way.
        let chain = format!("x = a{};", "+a".repeat(100_000));
        let errors = parse_units(&units(&chain), limit).err().unwrap();
        assert_eq!(
            messages(&errors),
            vec!["nesting deeper than parser_nesting (1000)".to_string()]
        );
        let blocks = format!("{}{}", "{".repeat(100_000), "}".repeat(100_000));
        let errors = parse_units(&units(&blocks), limit).err().unwrap();
        assert_eq!(errors.len(), 1);
    })
    .unwrap();
    // The bound fits the stack of a 256 KiB thread at this limit.
    assert!(ParseOptions { max_nesting: LIMIT }.stack_bytes() <= 256 * 1024);
}

/// A source of `n` nested `kind`s; each kind drives another recursion of the parser, or, for the
/// chains, a tree as deep as the limit that the parser builds by a loop.
fn nested(kind: &str, n: usize) -> Option<String> {
    let r = |s: &str| s.repeat(n);
    Some(match kind {
        "array" => format!("{}{};", r("["), r("]")),
        "paren" => format!("{}0{};", r("("), r(")")),
        "block" => format!("{}{}", r("{"), r("}")),
        "object" => format!("x = {}0{};", r("{a:"), r("}")),
        "arguments" => format!("{}0{};", r("f("), r(")")),
        "index" => format!("{}0{};", r("a["), r("]")),
        "new" => format!("{}X;", r("new ")),
        "not" => format!("x = {}a;", r("!")),
        "typeof" => format!("{}a;", r("typeof ")),
        "assign" => format!("{}0;", r("a = ")),
        "conditional" => format!("{}c;", r("a ? b : ")),
        "conditional_middle" => format!("{}b{};", r("a ? "), r(" : c")),
        "sum_chain" => format!("x = a{};", r("+a")),
        "member_chain" => format!("x = a{};", r(".b")),
        "call_chain" => format!("f{};", r("()")),
        "function_declaration" => format!("{}{}", r("function f(){"), r("}")),
        "function_expression" => format!("x = {}0{};", r("function(){ return "), r("; }")),
        "getter" => format!("x = {}0{};", r("{get a(){ return "), r("; }}")),
        "if" => format!("{};", r("if (a) ")),
        "else" => format!("{};", r("if (a) ; else ")),
        "while" => format!("{};", r("while (a) ")),
        "for" => format!("{};", r("for (;;) ")),
        "for_in" => format!("{};", r("for (x in o) ")),
        "do_while" => format!("{};{}", r("do "), r(" while (a);")),
        "try" => format!("{}{}", r("try {"), r("} finally {}")),
        "catch" => format!("{}{}", r("try {} catch (e) {"), r("}")),
        "switch" => format!("{}{}", r("switch (a) { case 0: "), r("}")),
        "string_inside" => format!("x = {}\"a\\u{{1F600}}\\x41\\101\"{};", r("("), r(")")),
        "regexp_inside" => format!("x = {}/a[/]b\\//gi{};", r("("), r(")")),
        // Each label of a chain has a name of its own: a label inside the same label is an
        // early error.
        "labelled" => format!("{};", (0..n).map(|i| format!("l{i}: ")).collect::<String>()),
        "optional_chain" => format!("x = a{};", r("?.b")),
        "optional_index" => format!("x = {}0{};", r("a?.["), r("]")),
        "optional_call" => format!("x = {}0{};", r("f?.("), r(")")),
        "coalesce" => format!("x = a{};", r(" ?? a")),
        "method" => format!("x = {}0{};", r("{m(){ return "), r("; }}")),
        "with" => format!("{};", r("with (o) ")),
        _ => return None,
    })
}

const NESTING_KINDS: [&str; 36] = [
    "array",
    "paren",
    "block",
    "object",
    "arguments",
    "index",
    "new",
    "not",
    "typeof",
    "assign",
    "conditional",
    "conditional_middle",
    "sum_chain",
    "member_chain",
    "call_chain",
    "function_declaration",
    "function_expression",
    "getter",
    "if",
    "else",
    "while",
    "for",
    "for_in",
    "do_while",
    "try",
    "catch",
    "switch",
    "string_inside",
    "regexp_inside",
    "labelled",
    "optional_chain",
    "optional_index",
    "optional_call",
    "coalesce",
    "method",
    "with",
];

/// The parse entries a source goes through: `parse_script`, `parse_eval` with a non-strict
/// caller, and the body of `parse_dynamic_function` with the parameters `a, b`.
#[derive(Clone, Copy, Debug)]
enum Entry {
    Script,
    Eval,
    FunctionBody,
}

const ENTRIES: [Entry; 3] = [Entry::Script, Entry::Eval, Entry::FunctionBody];

/// `source` parsed through `entry` with nesting limit `limit`.
fn parse_through(entry: Entry, source: &[u16], limit: u32) -> Result<Script, Vec<SyntaxError>> {
    let options = ParseOptions { max_nesting: limit };
    match entry {
        Entry::Script => parse_script(source, options),
        Entry::Eval => parse_eval(source, false, options),
        Entry::FunctionBody => parse_dynamic_function(&units("a, b"), source, options),
    }
}

/// Whether `n` nested `kind`s parse through `entry` at `limit` (`Some(true)`), fail with the
/// nesting error alone (`Some(false)`), or fail otherwise (`None`).
fn admitted(entry: Entry, kind: &str, n: usize, limit: u32) -> Option<bool> {
    let text = nested(kind, n)?;
    match parse_through(entry, &units(&text), limit) {
        Ok(_) => Some(true),
        Err(e) if e.len() == 1 && e.first()?.message.contains("parser_nesting") => Some(false),
        Err(_) => None,
    }
}

/// Every kind of nesting, at the deepest the limit admits and 100,000 deep, parses or fails on
/// the stack `ParseOptions::stack_bytes` gives, in a child process, through each parse entry
/// (`parse_script`, `parse_eval` and the body of `parse_dynamic_function`, whose three parses
/// each stay within the limit): the test fails when a frame of the parser grows past
/// `STACK_BYTES_PER_LEVEL`, or its fixed part past `STACK_BYTES_FIXED`, and the stack
/// overflows. At 10,000 levels the per-level bound decides; at 10 the fixed part does.
#[test]
fn every_nesting_kind_fits_the_stack_bound() {
    for limit in [10, 10_000] {
        on_bounded_stack("every_nesting_kind_fits_the_stack_bound", limit, |limit| {
            for entry in ENTRIES {
                for kind in NESTING_KINDS {
                    // The deepest source the limit admits, found by bisection on this stack.
                    let (mut parses, mut refused) = (0, limit as usize + 2);
                    assert_eq!(
                        admitted(entry, kind, refused, limit),
                        Some(false),
                        "{entry:?} {kind}"
                    );
                    while refused - parses > 1 {
                        let mid = (parses + refused) / 2;
                        match admitted(entry, kind, mid, limit) {
                            Some(true) => parses = mid,
                            Some(false) => refused = mid,
                            None => panic!(
                                "{entry:?} {kind} {mid}: an error other than the nesting error"
                            ),
                        }
                    }
                    assert!(
                        parses > 0 || limit < 20,
                        "{entry:?} {kind}: nothing parses at {limit}"
                    );
                    assert_eq!(
                        admitted(entry, kind, 100_000, limit),
                        Some(false),
                        "{entry:?} {kind}"
                    );
                }
            }
        })
        .unwrap();
    }
}

/// `#sec-performeval` and `#sec-createdynamicfunction` parse through the same parser and the
/// same nesting bound as a script: at `max_nesting` 1,000, `0` inside 2,000 nested parentheses
/// is one error naming `parser_nesting` through `parse_eval`, as the body of
/// `parse_dynamic_function` and as its parameter text; inside 10 it parses.
#[test]
fn eval_and_function_entries_share_the_nesting_bound() {
    let nested = |n: usize| units(&format!("{}0{}", "(".repeat(n), ")".repeat(n)));
    let options = ParseOptions { max_nesting: LIMIT };
    for strict in [false, true] {
        let errors = parse_eval(&nested(2_000), strict, options).err().unwrap();
        assert_eq!(
            messages(&errors),
            vec!["nesting deeper than parser_nesting (1000)".to_string()],
            "eval, strict caller {strict}"
        );
        assert!(parse_eval(&nested(10), strict, options).is_ok());
    }
    let errors = parse_dynamic_function(&[], &nested(2_000), options)
        .err()
        .unwrap();
    assert_eq!(
        messages(&errors),
        vec!["nesting deeper than parser_nesting (1000)".to_string()]
    );
    assert!(parse_dynamic_function(&[], &nested(10), options).is_ok());
    // A parameter text of nested parentheses is one error, the grammar's.
    let errors = parse_dynamic_function(&nested(2_000), &[], options)
        .err()
        .unwrap();
    assert_eq!(errors.len(), 1);
    // At 0 every non-empty source fails, through either entry; an empty one parses.
    let zero = ParseOptions { max_nesting: 0 };
    assert!(parse_eval(&units("0"), false, zero).is_err());
    assert!(parse_eval(&[], false, zero).is_ok());
    assert!(parse_dynamic_function(&[], &[], zero).is_err());
}

/// PerformEval (`#sec-performeval`), steps 9 to 11: the eval code is strict when the caller is
/// strict or its own directive prologue holds a Use Strict Directive; `with` is an early error
/// in strict code (`#sec-with-statement-static-semantics-early-errors`), so it parses through
/// `parse_eval` with a non-strict caller and is a SyntaxError with a strict one.
#[test]
fn eval_entry_is_strict_after_a_strict_caller() {
    let options = ParseOptions { max_nesting: LIMIT };
    let script = parse_eval(&units("with (o) {}"), false, options).unwrap();
    assert!(!script.strict);
    let errors = parse_eval(&units("with (o) {}"), true, options)
        .err()
        .unwrap();
    assert_eq!(
        messages(&errors),
        vec!["with statement in strict mode code".to_string()]
    );
    let errors = parse_eval(&units("'use strict'; with (o) {}"), false, options)
        .err()
        .unwrap();
    assert_eq!(errors.len(), 1);
    assert!(parse_eval(&units("1;"), true, options).unwrap().strict);
    assert!(
        parse_eval(&units("'use strict'; 1;"), false, options)
            .unwrap()
            .strict
    );
    // A function of strict eval code is strict; of non-strict eval code it is not.
    let script = parse_eval(&units("function f() {}"), true, options).unwrap();
    assert!(script.ast.functions().iter().all(|f| f.strict));
    let script = parse_eval(&units("function f() {}"), false, options).unwrap();
    assert!(script.ast.functions().iter().all(|f| !f.strict));
    // Strict eval code may not declare `eval` or `arguments`
    // (`#sec-identifiers-static-semantics-early-errors`).
    assert!(parse_eval(&units("var eval;"), true, options).is_err());
    assert!(parse_eval(&units("var eval;"), false, options).is_ok());
}

/// EvalDeclarationInstantiation (`#sec-evaldeclarationinstantiation`) takes VarDeclaredNames
/// and the function declarations to instantiate from the answer of `parse_eval`: each name once
/// in source order, a function declaration's name among the var names, and the last declaration
/// of each function name.
#[test]
fn eval_entry_returns_var_names_and_hoisted_functions() {
    let options = ParseOptions { max_nesting: LIMIT };
    let source = "var a; function f() { var inner; } var b, a; if (x) { var c; } \
                  function f() { return 1; } function g() {} for (var d in o) {}";
    let script = parse_eval(&units(source), false, options).unwrap();
    assert_eq!(script.scope.var_names, vec!["a", "f", "b", "c", "g", "d"]);
    assert_eq!(script.scope.functions, vec![FunctionId(1), FunctionId(2)]);
    assert_eq!(script.ast.functions()[0].scope.var_names, vec!["inner"]);
    // An empty source is an empty script (PerformEval, step 13: its result is undefined).
    let script = parse_eval(&[], false, options).unwrap();
    assert!(script.body.is_empty() && script.scope.var_names.is_empty());
}

/// CreateDynamicFunction (`#sec-createdynamicfunction`) for kind normal: the parameter text is
/// parsed alone as FormalParameters, the body text alone as a FunctionBody, then the whole source
/// text as a FunctionExpression, so a parameter text that closes the list early, or a comment
/// that spans the two, is a SyntaxError although the whole text would parse.
#[test]
fn dynamic_function_entry_parses_parameters_and_body_apart() {
    let options = ParseOptions { max_nesting: LIMIT };
    let parsed = |p: &str, b: &str| parse_dynamic_function(&units(p), &units(b), options);
    let script = parsed("a, b", "return a + b").unwrap();
    let functions = script.ast.functions();
    let f = &functions[0];
    assert_eq!(f.kind, FunctionKind::Expression);
    assert!(f.name.is_none(), "the name anonymous is not bound inside");
    let names: Vec<&str> = f.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b"]);
    assert!(!f.strict);
    // The script holds the function expression alone.
    assert_eq!(script.body.len(), 1);
    let Some(Stmt::Expression(e)) = script.ast.stmt(script.body[0]).map(|s| &s.stmt) else {
        panic!("not an expression statement");
    };
    assert_eq!(
        script.ast.expr(*e).map(|n| &n.expr),
        Some(&Expr::Function(FunctionId(0)))
    );

    for (p, b) in [
        ("", ""),
        ("a,", "return a"),
        ("a\n,b", "return a + b"),
        ("/* c */ a", "// c"),
        ("", "'use strict'; return this"),
        ("a, a", "return a"),
        ("", "with (o) {}"),
    ] {
        assert!(parsed(p, b).is_ok(), "{p:?} {b:?}: {:?}", parsed(p, b));
    }
    assert!(parsed("", "'use strict'").unwrap().ast.functions()[0].strict);
    for (p, b) in [
        ("a) {", "}"),
        ("/*", "*/){"),
        ("a", "}); (function () {"),
        ("a, a", "'use strict';"),
        ("eval", "'use strict';"),
        ("", "'use strict'; with (o) {}"),
        ("a b", ""),
        ("a = 1", ""),
        ("", "}"),
        ("", "break;"),
    ] {
        assert!(parsed(p, b).is_err(), "{p:?} {b:?} parsed");
    }
    let errors = parsed("a = 1", "").err().unwrap();
    assert_eq!(
        messages(&errors),
        vec!["default parameter is not supported before slice 6".to_string()]
    );
}

/// `#sec-function-calls-runtime-semantics-evaluation`, step 6: a call whose callee, with its
/// parentheses removed, is the IdentifierReference `eval` is a direct eval; scope analysis marks
/// the function (or the script) whose own code holds one, and the function then gets its
/// arguments object (`docs/design/engine.md`, decision 3, "Names", item 4) unless a parameter or
/// a function declaration is named `arguments`.
#[test]
fn direct_eval_is_recorded_per_function() {
    let script = parse(
        "function a() { eval(x); } function b() { (eval)(x); } \
         function c() { o.eval(x); } function d() { (0, eval)(x); } \
         function e() { return function () { eval(x); }; } function g() { eval?.(x); } \
         function h(arguments) { eval(x); } function k() { new eval(x); } \
         function m() { var eval; eval(x); }",
    )
    .unwrap();
    assert!(!script.scope.contains_direct_eval);
    let functions = script.ast.functions();
    let named = |name: &str| {
        functions
            .iter()
            .find(|f| f.name.as_ref().is_some_and(|n| n.name == name))
            .unwrap_or_else(|| panic!("no function {name}"))
    };
    for (name, direct) in [
        ("a", true),
        ("b", true),
        ("c", false),
        ("d", false),
        ("e", false),
        ("g", false),
        ("h", true),
        ("k", false),
        ("m", true),
    ] {
        assert_eq!(named(name).scope.contains_direct_eval, direct, "{name}");
    }
    let inner = functions.iter().find(|f| f.name.is_none()).unwrap();
    assert!(inner.scope.contains_direct_eval);
    assert!(inner.scope.arguments_object);
    assert!(named("a").scope.arguments_object);
    assert!(!named("h").scope.arguments_object);
    assert!(!named("c").scope.arguments_object);
    assert!(parse("eval('1');").unwrap().scope.contains_direct_eval);
    assert!(
        parse_eval(
            &units("eval('1');"),
            false,
            ParseOptions { max_nesting: LIMIT }
        )
        .unwrap()
        .scope
        .contains_direct_eval
    );
}

#[test]
fn nesting_zero_refuses_any_non_empty_source() {
    for text in ["0", ";", "x", " ", "\n", "/* */", "// c", "var a = 1;"] {
        let errors = parse_units(&units(text), 0)
            .err()
            .unwrap_or_else(|| panic!("{text:?} parsed"));
        assert_eq!(
            messages(&errors),
            vec!["nesting deeper than parser_nesting (0)".to_string()],
            "{text:?}"
        );
    }
    assert!(parse_units(&[], 0).is_ok());
    assert!(parse_units(&units("0"), 1).is_err());
    assert!(parse_units(&units(";"), 3).is_ok());
}

#[test]
fn nesting_error_names_parser_nesting() {
    let nested = |n: usize| format!("{}0{};", "(".repeat(n), ")".repeat(n));
    let errors = parse(&nested(2_000)).err().unwrap();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("parser_nesting"), "{:?}", errors);
    assert_eq!(errors[0].line, 1);
    assert!(parse(&nested(10)).is_ok());
}

/// The string value assigned by the first statement, `x = "<string>";`; `None` when the first
/// statement is not of that form.
fn string_value_of_assignment(script: &Script) -> Option<Vec<u16>> {
    let ast = &script.ast;
    let Stmt::Expression(expr) = ast.stmt(*script.body.first()?)?.stmt else {
        return None;
    };
    let Expr::Assign { value, .. } = &ast.expr(expr)?.expr else {
        return None;
    };
    let Expr::String(literal) = &ast.expr(*value)?.expr else {
        return None;
    };
    Some(literal.value.clone())
}

#[test]
fn lone_surrogate_in_a_string_literal_is_kept() {
    let mut source = units("x = \"a");
    source.push(0xD800);
    source.extend(units("b\";"));
    let script = parse_units(&source, LIMIT).unwrap();
    assert_eq!(
        string_value_of_assignment(&script).unwrap(),
        vec![0x61, 0xD800, 0x62]
    );

    let mut source = units("x = '");
    source.extend([0xDC00, 0xD800]);
    source.extend(units("';"));
    let script = parse_units(&source, LIMIT).unwrap();
    assert_eq!(
        string_value_of_assignment(&script).unwrap(),
        vec![0xDC00, 0xD800]
    );

    let script = parse("x = \"\\uD800\";").unwrap();
    assert_eq!(string_value_of_assignment(&script).unwrap(), vec![0xD800]);
    let script = parse("x = \"\\uDC00\\uD800\";").unwrap();
    assert_eq!(
        string_value_of_assignment(&script).unwrap(),
        vec![0xDC00, 0xD800]
    );
    let script = parse("x = \"\\u{1F600}\";").unwrap();
    assert_eq!(
        string_value_of_assignment(&script).unwrap(),
        vec![0xD83D, 0xDE00]
    );
}

#[test]
fn use_strict_with_an_escape_is_not_a_directive() {
    assert!(parse("\"use strict\";").unwrap().strict);
    assert!(parse("'use strict';").unwrap().strict);
    assert!(parse("\"a\"; 'use strict';").unwrap().strict);
    assert!(parse("/* c */ \"use strict\"; // c\n").unwrap().strict);
    for text in [
        "\"use\\x20strict\";",
        "\"use\\u0020strict\";",
        "'use\\u{20}strict';",
        "\"use \\\nstrict\";",
        "(\"use strict\");",
        "\"use strict\" + 1;",
        "x; \"use strict\";",
        "\"use strict \";",
        "\"Use strict\";",
    ] {
        let script = parse(text).unwrap_or_else(|e| panic!("{text:?}: {e:?}"));
        assert!(!script.strict, "{text:?} made the script strict");
    }
    // The escaped form leaves the code non-strict, so `with` parses
    // (`#sec-with-statement-static-semantics-early-errors` holds for strict code only).
    let script = parse("\"use\\x20strict\"; with (o) {}")
        .unwrap_or_else(|e| panic!("the escaped directive and with: {e:?}"));
    assert!(!script.strict);
    let errors = parse("\"use strict\"; with (o) {}").err().unwrap();
    assert_eq!(
        messages(&errors),
        vec!["with statement in strict mode code".to_string()]
    );
}

/// `#sec-functiondeclarationinstantiation`, steps 15 to 18: a function whose own code references
/// `arguments` needs the object, unless a parameter or a function it declares is named
/// `arguments` (a `var` of that name does not stop it); a reference inside a nested function is
/// that function's own. A body whose own code holds a `with` statement says so.
#[test]
fn scope_marks_arguments_objects_and_with_statements() {
    let script = parse(
        "function a() { arguments; } function b(arguments) { arguments; } \
         function c() { function arguments() {} arguments; } \
         function d() { var arguments; arguments; } \
         function e() { return function () { return { arguments }; }; } \
         function g() { with (o) { arguments; } } \
         with (o) { x; }",
    )
    .unwrap();
    let functions = script.ast.functions();
    let named = |name: &str| {
        functions
            .iter()
            .find(|f| f.name.as_ref().is_some_and(|n| n.name == name))
            .unwrap_or_else(|| panic!("no function {name}"))
    };
    for (name, needs) in [
        ("a", true),
        ("b", false),
        ("c", false),
        ("arguments", false),
        ("d", true),
        ("e", false),
        ("g", true),
    ] {
        assert_eq!(named(name).scope.arguments_object, needs, "{name}");
    }
    let inner = functions.iter().find(|f| f.name.is_none()).unwrap();
    assert!(inner.scope.arguments_object);
    assert!(script.scope.contains_with);
    assert!(named("g").scope.contains_with);
    assert!(!named("a").scope.contains_with);
}

#[test]
fn function_bodies_inherit_and_set_strictness() {
    let text = "function a() { function b() {} }\n\
                function c() { \"use strict\"; function d() {} var e = function () {}; }\n\
                var o = { get p() { return 1; }, set p(v) { \"use strict\"; } };";
    let script = parse(text).unwrap();
    assert!(!script.strict);
    let strict: Vec<bool> = script.ast.functions().iter().map(|f| f.strict).collect();
    assert_eq!(strict, vec![false, false, true, true, true, false, true]);
    let names: Vec<Option<&str>> = script
        .ast
        .functions()
        .iter()
        .map(|f| f.name.as_ref().map(|n| n.name.as_str()))
        .collect();
    assert_eq!(
        names,
        vec![Some("a"), Some("b"), Some("c"), Some("d"), None, None, None]
    );

    let script = parse(&format!("'use strict';\n{text}")).unwrap();
    assert!(script.strict);
    assert!(script.ast.functions().iter().all(|f| f.strict));
}

#[test]
fn var_declared_names_and_hoisted_functions() {
    let text = "var a;\n\
                function f() { var inner; function g() { return h; } inner = q; }\n\
                var b, a;\n\
                for (var c in o) {}\n\
                if (x) { var d; }\n\
                try {} catch (e) { var h; }\n\
                function f() { return y + z(y); }\n\
                function k() {}\n\
                var u = function w() { var v; };";
    let script = parse(text).unwrap();
    assert_eq!(
        script.scope.var_names,
        vec!["a", "f", "b", "c", "d", "h", "k", "u"]
    );
    assert_eq!(script.scope.functions, vec![FunctionId(2), FunctionId(3)]);
    assert_eq!(script.scope.references, vec!["o", "x"]);

    let functions = script.ast.functions();
    assert_eq!(functions.len(), 5);
    let first_f = &functions[0];
    assert_eq!(first_f.scope.var_names, vec!["inner", "g"]);
    assert_eq!(first_f.scope.functions, vec![FunctionId(1)]);
    assert_eq!(first_f.scope.references, vec!["inner", "q"]);
    assert_eq!(functions[1].scope.references, vec!["h"]);
    assert_eq!(functions[2].scope.references, vec!["y", "z"]);
    assert!(functions[3].scope.var_names.is_empty());
    assert_eq!(functions[4].scope.var_names, vec!["v"]);
    assert!(functions[4].scope.functions.is_empty());
}

#[test]
fn every_zs_code_point_is_white_space() {
    let zs: Vec<u32> = (0..=0x10FFFF)
        .filter(|&c| swell_unicode::is_space_separator(c))
        .collect();
    assert_eq!(zs.len(), 17);
    let mut others = vec![0x09, 0x0B, 0x0C, 0xFEFF];
    others.extend(&zs);
    for c in others {
        let space = u16::try_from(c).unwrap();
        let mut source = units("var");
        source.push(space);
        source.push(u16::from(b'a'));
        source.push(space);
        source.push(u16::from(b'='));
        source.push(space);
        source.extend(units("1;"));
        let script = parse_units(&source, LIMIT).unwrap_or_else(|e| panic!("U+{c:04X}: {e:?}"));
        assert_eq!(script.scope.var_names, vec!["a"], "U+{c:04X}");
    }
    // White_Space code points outside `Zs` and the table above are not white space.
    for c in [0x180E_u16, 0x200B, 0x0085] {
        let mut source = units("var a =");
        source.push(c);
        source.extend(units("1;"));
        assert!(
            parse_units(&source, LIMIT).is_err(),
            "U+{c:04X} was white space"
        );
    }
}

#[test]
fn regexp_literal_is_lexed_to_its_end() {
    let regexp = |text: &str| -> (String, String) {
        let script = parse(text).unwrap_or_else(|e| panic!("{text:?}: {e:?}"));
        let ast = &script.ast;
        let Stmt::Expression(expr) = ast.stmt(script.body[0]).unwrap().stmt else {
            panic!("{text:?}: not an expression statement");
        };
        let Expr::Assign { value, .. } = &ast.expr(expr).unwrap().expr else {
            panic!("{text:?}: not an assignment");
        };
        let Expr::RegExp(literal) = &ast.expr(*value).unwrap().expr else {
            panic!("{text:?}: not a regular expression literal");
        };
        (
            String::from_utf16(&literal.body).unwrap(),
            String::from_utf16(&literal.flags).unwrap(),
        )
    };
    assert_eq!(regexp("x = /[/]\\//g;"), ("[/]\\/".into(), "g".into()));
    assert_eq!(
        regexp("x = /a\\/b[\\]/]c/gim;"),
        ("a\\/b[\\]/]c".into(), "gim".into())
    );
    assert_eq!(regexp("x = /=/;"), ("=".into(), "".into()));
    assert_eq!(regexp("x = /[]/ ;"), ("[]".into(), "".into()));
    for text in ["x = /[/;", "x = /a\\/;", "x = /a\nb/;", "x = /[a\n]/;"] {
        assert!(parse(text).is_err(), "{text:?} parsed");
    }
    // After an operand a slash divides.
    let script = parse("x = a / b / c;").unwrap();
    let Stmt::Expression(expr) = script.ast.stmt(script.body[0]).unwrap().stmt else {
        panic!("not an expression statement");
    };
    let Expr::Assign { value, .. } = &script.ast.expr(expr).unwrap().expr else {
        panic!("not an assignment");
    };
    assert!(matches!(
        script.ast.expr(*value).unwrap().expr,
        Expr::Binary { .. }
    ));
}

/// The text of each function's span, in source order; a span outside `source` reads as
/// `<out of the source>`, which no expectation names.
fn function_texts(script: &Script, source: &[u16]) -> Vec<String> {
    script
        .ast
        .functions()
        .iter()
        .map(
            |f| match source.get(f.span.start as usize..f.span.end as usize) {
                Some(units) => String::from_utf16_lossy(units),
                None => "<out of the source>".to_string(),
            },
        )
        .collect()
}

/// The span of a function is its source text, "the source text matched by" the
/// FunctionDeclaration, FunctionExpression or MethodDefinition
/// (`#sec-runtime-semantics-instantiateordinaryfunctionobject`,
/// `#sec-runtime-semantics-instantiateordinaryfunctionexpression`,
/// `#sec-runtime-semantics-definemethod`, `#sec-runtime-semantics-methoddefinitionevaluation`):
/// from its first token (`function`; `get` or `set` of an accessor; the property name of a plain
/// method) to its closing brace. A comment before or after it is outside; a comment and a line
/// end inside are kept. Through `parse_dynamic_function` the span is the whole text of the third
/// parse (`#sec-createdynamicfunction`), and a function in its body is placed in that text.
#[test]
fn function_spans_are_their_source_text() {
    let text = "/* a */ function d(a, /* in */ b) {\n  return a; // end\n} /* b */\n\
                var e = /* c */ function (x) { return x; } /* d */;\n\
                var o = { /* e */ get p() { return 1; } /* f */, /* g */ set p(v) {} /* h */,\n\
                /* i */ m(a) { return function () {}; } /* j */, \"q r\"() {} };";
    let source = units(text);
    let script = parse_units(&source, LIMIT).unwrap();
    let kinds: Vec<FunctionKind> = script.ast.functions().iter().map(|f| f.kind).collect();
    assert_eq!(
        kinds,
        vec![
            FunctionKind::Declaration,
            FunctionKind::Expression,
            FunctionKind::Getter,
            FunctionKind::Setter,
            FunctionKind::Method,
            FunctionKind::Expression,
            FunctionKind::Method,
        ]
    );
    assert_eq!(
        function_texts(&script, &source),
        vec![
            "function d(a, /* in */ b) {\n  return a; // end\n}",
            "function (x) { return x; }",
            "get p() { return 1; }",
            "set p(v) {}",
            "m(a) { return function () {}; }",
            "function () {}",
            "\"q r\"() {}",
        ]
    );

    let options = ParseOptions { max_nesting: LIMIT };
    let (parameters, body) = (units("a,\nb"), units("return function g() { /* x */ };"));
    let script = parse_dynamic_function(&parameters, &body, options).unwrap();
    let whole = units("function anonymous(a,\nb\n) {\nreturn function g() { /* x */ };\n}");
    assert_eq!(
        function_texts(&script, &whole),
        vec![
            String::from_utf16(&whole).unwrap(),
            "function g() { /* x */ }".to_string(),
        ]
    );
}

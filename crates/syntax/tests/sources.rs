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

use swell_syntax::ast::{Ast, Expr, ExprId, FunctionId, FunctionKind, Stmt};
use swell_syntax::{
    EvalContext, MessageKind, ParseOptions, Script, SyntaxError, message_kind,
    parse_dynamic_async_function, parse_dynamic_function, parse_dynamic_generator_function,
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

/// The checks of a source that parses, under `valid/` or under `held/` once nothing in it is
/// held: it parses as written and, with a `"use strict";` prologue, as strict code; a
/// `// sloppy-only` source is instead an early error (not a refusal) in strict code. Answers
/// whether the source is sloppy-only, or what failed.
fn parses_in_both_modes(path: &Path, text: &str) -> Result<bool, String> {
    let path = path.display();
    match parse(text) {
        Ok(script) if script.strict && !text.contains("\"use strict\"") => {
            return Err(format!("{path}: strict without a directive"));
        }
        Ok(_) => {}
        Err(errors) => return Err(format!("{path} as written: {errors:?}")),
    }
    let strict = with_use_strict(text);
    if sloppy_only(text) {
        let Err(errors) = parse(&strict) else {
            return Err(format!("{path}: parses as strict code"));
        };
        if !errors
            .iter()
            .all(|e| message_kind(&e.message) == MessageKind::Grammar)
        {
            return Err(format!("{path}: {errors:?}"));
        }
        return Ok(true);
    }
    match parse(&strict) {
        Ok(script) if !script.strict => Err(format!("{path}: not strict with the prologue")),
        Ok(_) => Ok(false),
        Err(errors) => Err(format!("{path} with \"use strict\": {errors:?}")),
    }
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
        if parses_in_both_modes(path, text).unwrap() {
            sloppy += 1;
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

/// The constructs a header result `held "<construct>", "<construct>", ...` names, in its
/// order, or what is wrong with it.
fn held_constructs(path: &Path, result: &str) -> Result<Vec<String>, String> {
    let list = result
        .strip_prefix("held ")
        .ok_or_else(|| format!("{}: header result is not held \"...\"", path.display()))?;
    list.split(", ")
        .map(|quoted| {
            quoted
                .strip_prefix('"')
                .and_then(|c| c.strip_suffix('"'))
                .filter(|c| !c.is_empty())
                .map(str::to_string)
                .ok_or_else(|| format!("{}: {quoted:?} is no quoted construct", path.display()))
        })
        .collect()
}

/// A source under `held/` holds constructs a story of slice 8 parses whole and refuses at the
/// end of the parse (`Parser::hold`; "Held sources" of `story:class-syntax-and-early-errors`).
/// Its header names each of them, `held "<construct>", ...`, in the order of their first
/// tokens. The first one that is still a line of [`swell_syntax::HOLDS`] is the one the source
/// answers, as written and with a `"use strict";` prologue: exactly `<construct> is not
/// supported before slice <n>`, a refusal of slice n for [`message_kind`]. A source none of
/// whose constructs is held any more parses as a source under `valid/` does. Every line of
/// `HOLDS` is named by a header, so a hold without a held source fails. Which construct is
/// still held is read from the parser's list, never from the fixture.
#[test]
fn held_sources_answer_their_first_construct_still_held() {
    let files = sources("held").unwrap();
    let mut named: Vec<String> = Vec::new();
    let (mut held, mut parsed) = (0, 0);
    for (path, text) in &files {
        let (_, result) = header(path, text).unwrap();
        let constructs = held_constructs(path, &result).unwrap();
        let still = constructs
            .iter()
            .find_map(|c| swell_syntax::HOLDS.iter().find(|(h, _)| h == c));
        named.extend(constructs);
        let Some(&(construct, slice)) = still else {
            parsed += 1;
            parses_in_both_modes(path, text).unwrap();
            continue;
        };
        held += 1;
        let expected = vec![format!("{construct} is not supported before slice {slice}")];
        for (mode, source) in [
            ("as written", text.to_string()),
            ("with \"use strict\"", with_use_strict(text)),
        ] {
            let errors = parse(&source).err().unwrap_or_else(|| {
                panic!("{} {mode}: parsed, expected {expected:?}", path.display())
            });
            assert_eq!(messages(&errors), expected, "{} {mode}", path.display());
            assert_eq!(
                message_kind(&errors[0].message),
                MessageKind::Refusal { slice },
                "{} {mode}",
                path.display()
            );
        }
    }
    for (construct, _) in swell_syntax::HOLDS {
        assert!(
            named.iter().any(|n| n == construct),
            "the hold {construct:?} of HOLDS has no held source"
        );
    }
    println!(
        "held: {} file(s), {held} answering a hold, {parsed} parsing; {} hold(s)",
        files.len(),
        swell_syntax::HOLDS.len()
    );
}

/// A ClassHeritage parses wherever its `extends` stands, in a declaration and in an
/// expression, and a heritage is no line of `HOLDS` (`story:derived-classes-and-super-call`).
#[test]
fn the_heritage_parses_at_extends() {
    assert!(
        !swell_syntax::HOLDS
            .iter()
            .any(|(c, _)| *c == "class heritage")
    );
    for text in [
        "class A\n  extends B {}",
        "x = class\n  extends B {};",
        "class A {}\nclass B extends C {}",
        "x = class extends (class {}) {};",
    ] {
        if let Err(errors) = parse(text) {
            panic!("{text:?}: {:?}", messages(&errors));
        }
    }
}

/// The answer of a class source that the parser reads whole: the one error of a source with a
/// grammar or an early error, or the refusal of the first construct it holds.
fn class_answer(text: &str) -> Result<String, String> {
    match parse(text) {
        Ok(_) => Ok("parses".to_string()),
        Err(errors) => match errors.as_slice() {
            [error] if error.message.contains("is not supported before slice") => {
                Ok(error.message.clone())
            }
            [error] => Err(error.message.clone()),
            _ => Err(format!("{errors:?}")),
        },
    }
}

/// A field ends at `;`, before `}`, or where `#sec-rules-of-automatic-semicolon-insertion` inserts
/// a `;`: before a token on a new line that its initializer or its name cannot take
/// (`language/statements/class/elements/fields-asi-*.js`). `static`, `get`, `set` and `async`
/// are field names where no element follows them on their line, and a name with an escape is
/// never the word (`escaped-*-field-*.js`). Each source of the first list parses to the hold of
/// its first field (a class is not held); each of the second is a grammar error.
#[test]
fn fields_end_where_a_semicolon_is_inserted() {
    let field = "class field is not supported before slice 8";
    let private = "private field is not supported before slice 8";
    for (text, held) in [
        ("class C { x\n y }", field),
        ("class C { x = 1\n ['a'] = 2 }", field),
        ("class C { x = a\n in\n z\n y = b }", field),
        ("class C { get\n *a() {} }", field),
        ("class C { \\u0067et\n *a() {} }", field),
        ("class C { static\n x }", field),
        ("class C { static }", field),
        ("class C { static = 1; get; set; async\n x }", field),
        ("class C { static\n async\n *m() {} }", field),
        ("class C { #x\n #y }", private),
        ("class C { static #x = 1; static y; z }", private),
        ("x = class { 'a'; 1 = 2; [b]; }", field),
    ] {
        assert_eq!(class_answer(text), Ok(held.to_string()), "{text:?}");
    }
    for (text, message) in [
        ("class C { x y }", "unexpected token y"),
        ("class C { #x #y }", "unexpected private name #y"),
        ("class C { x = 1 m() {} }", "unexpected token m"),
        ("class C { x = \"s\"\n [0]() {} }", "unexpected token {"),
        ("class C { x = 42\n *gen() {} }", "unexpected token {"),
        ("class C { g\\u0065t value() {} }", "unexpected token value"),
        (
            "class C { static st\\u0061tic value() {} }",
            "unexpected token value",
        ),
        ("class C { *x = 1 }", "unexpected token ="),
        ("class C { get x = 1 }", "unexpected token ="),
        ("class C { # x }", "unexpected character #"),
    ] {
        assert_eq!(class_answer(text), Err(message.to_string()), "{text:?}");
    }
}

/// `PrivateIdentifier in ShiftExpression` is a RelationalExpression
/// (`#sec-relational-operators`): a private name stands only directly before `in`, where `in`
/// is an operator, as the left operand at the precedence of `<`; its right operand is a
/// ShiftExpression. Every source of the first list parses to the hold of its first element, the
/// private field `#x`; each of the second is a grammar error at the private name.
#[test]
fn a_private_name_stands_only_before_in() {
    let held = "private field is not supported before slice 8";
    let class = |body: &str| format!("class C {{ #x; m(o, p) {{ {body} }} }}");
    for body in [
        "return #x in o;",
        "return #x in o in p;",
        "return #x in o < 1;",
        "return a == #x in o;",
        "return a && #x in o || #x in p;",
        "return #x in a + b * c;",
        "return #x in a ** 2;",
        "return (#x in o);",
        "for (; #x in o;);",
        "return a ? #x in o : #x in p;",
    ] {
        assert_eq!(class_answer(&class(body)), Ok(held.to_string()), "{body:?}");
    }
    for body in [
        "return #x in #x in o;",
        "return (#x) in o;",
        "return a < #x in o;",
        "return 1 + #x in o;",
        "return a in #x in o;",
        "return !#x in o;",
        "return #x;",
        "return #x + 1;",
        "for (#x in o;;);",
        "for (#x in o);",
        "return o[#x];",
    ] {
        assert_eq!(
            class_answer(&class(body)),
            Err("unexpected private name #x".to_string()),
            "{body:?}"
        );
    }
}

/// AllPrivateIdentifiersValid (`#sec-static-semantics-allprivateidentifiersvalid`) holds for a
/// script, eval code and `Function` code, none of which has an enclosing class body with names
/// for it (`#sec-performeval`, `#sec-createdynamicfunction`): through each entry a private
/// reference outside a class is the early error, and one inside a class that declares the name
/// answers the hold of that declaration, the private field `#x`.
#[test]
fn a_private_name_outside_a_class_is_an_early_error_through_every_entry() {
    let undeclared = "private name #x is not declared in an enclosing class";
    let held = "private field is not supported before slice 8";
    for (text, message) in [
        ("o.#x;", undeclared),
        ("o?.#x;", undeclared),
        ("o?.a.#x();", undeclared),
        ("#x in o;", undeclared),
        ("(function () { return this.#x; });", undeclared),
        ("x = () => o.#x;", undeclared),
        ("class C { #y; m() { o.#x; } }", undeclared),
        ("class C { #x; m() { class D { n() { o.#x; } } } }", held),
        ("class C { #x; static { o.#x; } }", held),
        ("class C { #x; y = o.#x; }", held),
    ] {
        for entry in ENTRIES {
            let answer = parse_through(entry, &units(text), LIMIT).map(|_| ());
            assert_eq!(
                answer.map_err(|errors| messages(&errors)),
                Err(vec![message.to_string()]),
                "{entry:?} {text:?}"
            );
        }
    }
}

/// A class is strict mode code from `class` to its `}` and no further (`#sec-strict-mode-code`):
/// the parser and the walk of the early errors both give the code after a class declaration or
/// expression the strictness of the code around it. So, through each entry, a source with a
/// class and, after it, code that only non-strict code admits parses (`None`);
/// an `if` clause that is a function declaration answers its refusal of slice 18, which
/// stops the parse at its token, as it does in non-strict code without a class.
#[test]
fn code_after_a_class_is_not_strict() {
    let parses = None;
    let if_clause = Some("function declaration as an if clause is not supported before slice 18");
    let mut cases: Vec<(String, Option<&str>)> = Vec::new();
    for class in ["class A {}", "x = class {};", "x = class { m() {} };"] {
        for (tail, message) in [
            ("with (a) {}", parses),
            ("x = 010;", parses),
            ("delete x;", parses),
            ("var eval;", parses),
            ("function f(a, a) {}", parses),
            ("if (a) function f() {}", if_clause),
        ] {
            cases.push((format!("{class} {tail}"), message));
            cases.push((format!("function g() {{ {class} {tail} }}"), message));
        }
    }
    let mut wrong: Vec<String> = Vec::new();
    for (text, message) in &cases {
        for entry in ENTRIES {
            let answer = parse_through(entry, &units(text), LIMIT).map(|_| ());
            let expected = message.map(|m| vec![m.to_string()]);
            if answer.as_ref().err().map(|errors| messages(errors)) != expected {
                wrong.push(format!("{entry:?} {text:?}: {answer:?}"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} answers wrong:\n{}",
        wrong.len(),
        cases.len() * ENTRIES.len(),
        wrong.join("\n")
    );
}

/// `async` functions and arrow functions parse whole (`#sec-async-function-definitions`,
/// `#sec-async-arrow-function-definitions`; `story:async-functions-and-await` lifted the refusal
/// of slice 10 that held them back to the end of the parse): through each entry, the dynamic ones
/// of `Function` and `%GeneratorFunction%` too; an early error or a grammar error anywhere ends
/// the source on that error alone. `await` is an identifier outside an `async` function, and
/// `async` one wherever no `async` function or arrow function starts.
#[test]
fn async_functions_parse_through_every_entry() {
    for text in [
        "async function f() {}",
        "x;\n  y = async (a) => a;",
        "y = async a => async function () {};",
        "y = async (a = async function () {}) => a;",
        "function* g() { yield* a; }\nasync function f() {}",
        // A generator method in an `async` function has [Await] cleared in its parameters and
        // body, and its computed name has the [Await] around.
        "async function f() { return { *[await a]() { await: yield; } }; }",
    ] {
        assert!(parse(text).is_ok(), "{text:?}: {:?}", parse(text).err());
    }
    for (text, message) in [
        (
            "async function f() {}\nlet a; let a;",
            "duplicate lexical declaration of a",
        ),
        (
            "async function f(await) {}",
            "await may not be an identifier in an async function",
        ),
        ("async function f() {}\n)", "unexpected token )"),
        ("async function f() { await; }", "unexpected token ;"),
    ] {
        assert_eq!(
            messages(&parse(text).err().unwrap()),
            vec![message.to_string()]
        );
    }
    let options = ParseOptions { max_nesting: LIMIT };
    assert!(parse_eval(&units("async x => x"), true, options).is_ok());
    assert!(parse_dynamic_function(&units("a"), &units("return async () => a;"), options).is_ok());
    let body = units("async function f(){}");
    assert!(parse_dynamic_function(&[], &body, options).is_ok());
    assert!(parse_dynamic_generator_function(&[], &body, options).is_ok());
    for text in [
        "await;",
        "var await = async;",
        "async\nfunction f() {}",
        "async(1);",
        "f = async => async;",
        "function* g() { await: yield; }",
        "async function f() { function g() { var await; } }",
    ] {
        assert!(parse(text).is_ok(), "{text:?}: {:?}", parse(text).err());
    }
}

/// `yield*` in an async generator (the async branch of YieldExpression : `yield *`
/// AssignmentExpression, `#sec-generator-function-definitions-runtime-semantics-evaluation`)
/// parses through each entry, the dynamic ones too (`story:yield-star-in-async-generators`
/// lifted the refusal of slice 10 that held it back to the end of the parse); an early error or
/// a grammar error anywhere ends the source on that error alone, and so does an early error
/// that stops the parse at its token (`super.x` in a plain function). An async generator
/// without it parses, and so does a `yield*` in a generator, also inside an async generator.
#[test]
fn yield_star_in_an_async_generator_parses_through_every_entry() {
    for text in [
        "async function* f() { yield* a; }",
        "x;\n  y = async function* () { yield* a; };",
        "async function* f() { yield* a; yield* b; }",
        "async function* f(a = async function* () { yield* b; }) { yield* a; }",
        "async function* f() { yield await 1; }\nasync function* g() { yield* a; }",
        "{ async function* f() { yield* a; } }\nswitch (0) { case 0: async function* g() { yield* b; } }",
        "function* g() { yield* a; }\nasync function* f() { function* h() { yield* a; } yield* b; }",
    ] {
        assert!(parse(text).is_ok(), "{text:?}: {:?}", parse(text).err());
    }
    for (text, expected) in [
        (
            "async function* f() { yield* a; }\nlet a; let a;",
            vec!["duplicate lexical declaration of a"],
        ),
        (
            "async function* f(x = yield) { yield* a; }\nlet a; let a;",
            vec![
                "yield expression in the parameters of a generator function",
                "duplicate lexical declaration of a",
            ],
        ),
        (
            "async function* f() { yield* a; }\n)",
            vec!["unexpected token )"],
        ),
        (
            "async function* f() { yield* a; await; }",
            vec!["unexpected token ;"],
        ),
        (
            "async function* f() { yield* a; }\nfunction g() { super.x; }",
            vec!["super property outside a method"],
        ),
    ] {
        assert_eq!(messages(&parse(text).err().unwrap()), expected, "{text:?}");
    }
    let options = ParseOptions { max_nesting: LIMIT };
    let parses = |answer: Result<Script, Vec<SyntaxError>>| {
        assert!(answer.is_ok(), "{:?}", answer.err());
    };
    parses(parse_eval(
        &units("x = async function* () { yield* a; };"),
        true,
        options,
    ));
    let body = units("return async function* () { yield* a; };");
    parses(parse_dynamic_function(&units("a"), &body, options));
    parses(parse_dynamic_generator_function(
        &units("a"),
        &body,
        options,
    ));
    parses(parse_dynamic_async_function(&units("a"), &body, options));
    parses(parse_dynamic_function(
        &units("a = async function* () { yield* b; }"),
        &[],
        options,
    ));
    for text in [
        "async function* f() {}",
        "x = async function* () { yield; yield 1; await 2; yield await 3; };",
        "async function* f() { function* g() { yield* a; } }",
        "function* g() { yield* a; }",
        // `async` before a line terminator is an identifier, and `function*` after it a generator.
        "async\nfunction* g() {}",
        "var async; async\nfunction* g() {}",
    ] {
        assert!(parse(text).is_ok(), "{text:?}: {:?}", parse(text).err());
    }
    let body = units("return async function* () { yield await a; };");
    assert!(parse_dynamic_function(&units("a"), &body, options).is_ok());
    assert!(parse_eval(&units("x = async function* () {};"), true, options).is_ok());
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

/// Every source under `eval/` is eval code: parsed through `parse_eval` as the code of a
/// non-strict caller with no flag set (`#sec-performeval`, step 9), it answers its header:
/// `parses`, `SyntaxError "<message>"` (the first error) or `refused "<message>"` (the one
/// error). The tests of scripts do not read these sources: what they show is the answer of eval
/// code, whose flags (`EvalContext`) decide what a script cannot state.
#[test]
fn eval_sources_answer_their_header() {
    let options = ParseOptions { max_nesting: LIMIT };
    let files = sources("eval").unwrap();
    for (path, text) in &files {
        let (_, result) = header(path, text).unwrap();
        let answer = parse_eval(&units(text), false, options);
        if result == "parses" {
            assert!(answer.is_ok(), "{}: {:?}", path.display(), answer.err());
            continue;
        }
        let errors = answer
            .err()
            .unwrap_or_else(|| panic!("{}: parsed, expected {result}", path.display()));
        if result.starts_with("SyntaxError") {
            let expected = quoted(path, &result, "SyntaxError").unwrap();
            assert_eq!(
                messages(&errors).first(),
                Some(&expected),
                "{}",
                path.display()
            );
        } else {
            let expected = quoted(path, &result, "refused").unwrap();
            assert_eq!(messages(&errors), vec![expected], "{}", path.display());
        }
    }
    println!("eval: {} file(s)", files.len());
}

/// `parse_script` parses a script, never eval code of a caller with every flag false: at the top
/// of a script and of such eval code `super.x` is the early error of
/// `#sec-scripts-static-semantics-early-errors` (PerformEval, steps 7 to 11: no flag of the
/// context is set); with `in_method` eval code admits a SuperProperty and not a SuperCall, with
/// `in_derived_constructor` both; `new.target` is the early error of its own rule through both
/// entries, and admitted in eval code only with `in_function`.
#[test]
fn a_script_is_not_parsed_as_eval_code() {
    let options = ParseOptions { max_nesting: LIMIT };
    let eval = |text: &str, context: EvalContext| {
        parse_eval(&units(text), context, options).map_err(|errors| messages(&errors))
    };
    let script = |text: &str| parse(text).map(|_| ()).map_err(|errors| messages(&errors));
    let super_property = vec!["super property outside a method".to_string()];
    let super_call = vec!["super call outside a derived class constructor".to_string()];
    let new_target = vec!["new.target outside a function".to_string()];
    assert_eq!(script("super.x;"), Err(super_property.clone()));
    assert_eq!(script("new.target;"), Err(new_target.clone()));
    assert_eq!(
        eval("super.x;", EvalContext::from(false)).map(|_| ()),
        Err(super_property.clone())
    );
    let in_method = EvalContext {
        in_function: true,
        in_method: true,
        ..EvalContext::from(false)
    };
    let in_derived_constructor = EvalContext {
        in_derived_constructor: true,
        ..in_method
    };
    assert!(eval("super.x; () => super.y;", in_method).is_ok());
    assert_eq!(
        eval("super();", in_method).map(|_| ()),
        Err(super_call.clone())
    );
    assert!(eval("super(); super.x; () => super();", in_derived_constructor).is_ok());
    assert_eq!(
        eval("super();", EvalContext::from(false)).map(|_| ()),
        Err(super_call)
    );
    assert_eq!(
        eval(
            "({ m() { super.x; } }); function f() { super.x; }",
            in_derived_constructor
        )
        .map(|_| ()),
        Err(super_property.clone())
    );
    assert_eq!(
        eval("new.target;", EvalContext::from(false)).map(|_| ()),
        Err(new_target)
    );
    let in_function = EvalContext {
        in_function: true,
        ..EvalContext::from(false)
    };
    assert!(eval("new.target;", in_function).is_ok());
    assert_eq!(
        script("var o = { m() { return super.x; } }; super.y;"),
        Err(super_property)
    );
}

/// The full path of a test in the crate's one test binary, `sources::<name>`, which a test
/// passes after `--exact` when it starts its own binary again
/// (`story:one-integration-test-binary-per-crate`). A bare name matches no test there: the
/// child would run nothing, exit 0 and print no "done" line. Made only by [`test_path!`]: write
/// `test_path!("<name>")` with the test's own name, never the name as a string.
#[derive(Clone, Debug)]
struct TestPath(String);

impl TestPath {
    /// Called by [`test_path!`] with the caller's `module_path!()`, whose first segment, the
    /// crate, is cut off; write `test_path!("<name>")`.
    fn from_module_path(module: &str, name: &str) -> Self {
        match module.split_once("::") {
            Some((_, path)) => Self(format!("{path}::{name}")),
            None => Self(name.to_owned()),
        }
    }

    /// The path, as `--exact` takes it.
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TestPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// `test_path!("<name>")`: the [`TestPath`] of the test `<name>` of the module the macro is
/// written in.
macro_rules! test_path {
    ($name:expr) => {
        TestPath::from_module_path(module_path!(), $name)
    };
}

/// Set in the environment of a child process a test starts: the nesting limit the child runs its
/// test body with.
const STACK_CHILD: &str = "SWELL_SYNTAX_STACK_CHILD";

/// Runs `body` with nesting limit `limit` on a thread whose stack is
/// `ParseOptions::stack_bytes` for that limit, in a child process that runs only the test
/// `test`. A stack overflow aborts the process, which no test can catch in its own process; in
/// the child it fails this test alone, with the child's output.
fn on_bounded_stack(test: TestPath, limit: u32, body: fn(u32)) -> Result<(), String> {
    let stack = ParseOptions { max_nesting: limit }.stack_bytes();
    on_stack(test, limit, stack, body)
}

/// As [`on_bounded_stack`], on a thread of `stack` bytes.
fn on_stack(test: TestPath, limit: u32, stack: usize, body: fn(u32)) -> Result<(), String> {
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
        .args(["--exact", test.as_str(), "--nocapture", "--test-threads=1"])
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
    let test = test_path!("nesting_of_100000_brackets_is_one_error");
    on_bounded_stack(test, LIMIT, |limit| {
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

/// The code units of a source of `n` nested `kind`s; each kind drives another recursion of the
/// parser, or, for the chains, a tree as deep as the limit that the parser builds by a loop.
///
/// The source is `before`, `n` times `open`, `middle`, `n` times `close` and `after`, put
/// together from the code units of each piece, which `[u16]::repeat` copies `n` times. A source
/// 100,000 deep is up to 2 MB, and encoding all of it as UTF-16 in the unoptimised test build
/// cost more than its parse, which stops at the limit: the child of
/// `every_nesting_kind_fits_the_stack_bound` at limit 10, which does little besides the
/// 100,000-deep check of its seven combinations, took 0.35 s of processor time that way and takes
/// 0.00 s this way; the child at 2,500 took 0.59 s and takes 0.21 s.
fn nested(kind: &str, n: usize) -> Option<Vec<u16>> {
    if kind == "labelled" {
        // Each label of a chain has a name of its own: a label inside the same label is an
        // early error.
        let labels: String = (0..n).map(|i| format!("l{i}: ")).collect();
        return Some(units(&format!("{labels};")));
    }
    // The innermost literal of `regexp_inside`: a pattern of 100 nested groups with a class and
    // an escape, which the early errors parse with ParsePattern (`docs/design/engine.md`,
    // decision 7, "No Rust recursion in the parser and the compiler of a pattern"). Its nesting
    // counts from zero against the limit, so at the limit 10 it is the nesting error.
    let regexp = format!("/a{}[/]b\\/{}/gi", "(".repeat(100), ")".repeat(100));
    let (before, open, middle, close, after) = match kind {
        "array" => ("", "[", "", "]", ";"),
        "paren" => ("", "(", "0", ")", ";"),
        "block" => ("", "{", "", "}", ""),
        "object" => ("x = ", "{a:", "0", "}", ";"),
        "arguments" => ("", "f(", "0", ")", ";"),
        "index" => ("", "a[", "0", "]", ";"),
        "new" => ("", "new ", "X;", "", ""),
        "not" => ("x = ", "!", "a;", "", ""),
        "typeof" => ("", "typeof ", "a;", "", ""),
        "assign" => ("", "a = ", "0;", "", ""),
        "conditional" => ("", "a ? b : ", "c;", "", ""),
        "conditional_middle" => ("", "a ? ", "b", " : c", ";"),
        "sum_chain" => ("x = a", "+a", ";", "", ""),
        "exp_chain" => ("x = a", " ** a", ";", "", ""),
        "member_chain" => ("x = a", ".b", ";", "", ""),
        "call_chain" => ("f", "()", ";", "", ""),
        "function_declaration" => ("", "function f(){", "", "}", ""),
        "function_expression" => ("x = ", "function(){ return ", "0", "; }", ";"),
        "getter" => ("x = ", "{get a(){ return ", "0", "; }}", ";"),
        "if" => ("", "if (a) ", ";", "", ""),
        "else" => ("", "if (a) ; else ", ";", "", ""),
        "while" => ("", "while (a) ", ";", "", ""),
        "for" => ("", "for (;;) ", ";", "", ""),
        "for_in" => ("", "for (x in o) ", ";", "", ""),
        "for_of" => ("", "for (x of a) ", ";", "", ""),
        "do_while" => ("", "do ", ";", " while (a);", ""),
        "try" => ("", "try {", "", "} finally {}", ""),
        "catch" => ("", "try {} catch (e) {", "", "}", ""),
        "switch" => ("", "switch (a) { case 0: ", "", "}", ""),
        "string_inside" => ("x = ", "(", "\"a\\u{1F600}\\x41\\101\"", ")", ";"),
        "regexp_inside" => ("x = ", "(", regexp.as_str(), ")", ";"),
        "optional_chain" => ("x = a", "?.b", ";", "", ""),
        "optional_index" => ("x = ", "a?.[", "0", "]", ";"),
        "optional_call" => ("x = ", "f?.(", "0", ")", ";"),
        "coalesce" => ("x = a", " ?? a", ";", "", ""),
        "method" => ("x = ", "{m(){ return ", "0", "; }}", ";"),
        "with" => ("", "with (o) ", ";", "", ""),
        "template" => ("x = ", "`${", "0", "}`", ";"),
        "arrow" => ("x = ", "() => ", "0", "", ";"),
        "arrow_block" => ("x = ", "() => { return ", "0", "; }", ";"),
        "default_parameter" => ("x = ", "function (a = ", "0", ") {}", ";"),
        "object_pattern" => ("var ", "{a:", "a", "}", " = o;"),
        "object_assignment_pattern" => ("(", "{a:", "a", "}", " = o);"),
        "computed_key" => ("x = ", "{[", "0", "]: 0}", ";"),
        "object_spread" => ("x = ", "{...", "0", "}", ";"),
        "block_function" => ("", "{ function f() {", "", "} }", ""),
        "array_pattern" => ("var ", "[", "a", "]", " = o;"),
        "array_assignment_pattern" => ("", "[", "a", "]", " = o;"),
        "spread" => ("", "[...", "a", "]", ";"),
        "tagged_template" => ("x = ", "t`${", "0", "}`", ";"),
        "generator_expression" => ("x = ", "function* (){ return ", "0", "; }", ";"),
        "yield" => ("function* g(){ ", "yield ", "0;", "", " }"),
        "yield_star" => ("function* g(){ ", "yield* ", "0;", "", " }"),
        "generator_method" => ("x = ", "{*m(){ return ", "0", "; }}", ";"),
        "async_function_expression" => ("x = ", "async function (){ return ", "0", "; }", ";"),
        "async_arrow" => ("x = ", "async x => ", "0", "", ";"),
        "await" => ("async function f(){ ", "await ", "0;", "", " }"),
        "async_method" => ("x = ", "{async m(){ return ", "0", "; }}", ";"),
        "async_generator_expression" => ("x = ", "async function* (){ return ", "0", "; }", ";"),
        "for_await" => ("async function f(){ ", "for await (x of y) ", ";", "", " }"),
        "async_generator_method" => ("x = ", "{async *m(){ return ", "0", "; }}", ";"),
        "class_computed_key" => ("x = ", "class { [", "0", "] () {} }", ";"),
        "class_heritage" => ("x = ", "class extends (", "0", ") {}", ";"),
        "class_field" => ("x = ", "class { x = ", "0", " }", ";"),
        "class_static_block" => ("x = ", "class { static { x = ", "0", " } }", ";"),
        _ => return None,
    };
    let mut source = units(before);
    source.extend_from_slice(&units(open).repeat(n));
    source.extend_from_slice(&units(middle));
    source.extend_from_slice(&units(close).repeat(n));
    source.extend_from_slice(&units(after));
    Some(source)
}

const NESTING_KINDS: [&str; 66] = [
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
    "for_of",
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
    "template",
    "arrow",
    "arrow_block",
    "default_parameter",
    "object_pattern",
    "object_assignment_pattern",
    "computed_key",
    "object_spread",
    "block_function",
    "array_pattern",
    "array_assignment_pattern",
    "spread",
    "tagged_template",
    "generator_expression",
    "yield",
    "yield_star",
    "generator_method",
    "async_function_expression",
    "async_arrow",
    "await",
    "async_method",
    "async_generator_expression",
    "for_await",
    "async_generator_method",
    "exp_chain",
    "class_computed_key",
    "class_heritage",
    "class_field",
    "class_static_block",
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
/// nesting error alone (`Some(false)`), or fail otherwise (`None`). A source whose one error is
/// the refusal of a line of [`swell_syntax::HOLDS`] counts as parsed: the parse and the early
/// errors reached the end of the source before the hold was answered (`Parser::hold`).
fn admitted(entry: Entry, kind: &str, n: usize, limit: u32) -> Option<bool> {
    let source = nested(kind, n)?;
    let held = |message: &str| {
        swell_syntax::HOLDS.iter().any(|(construct, slice)| {
            message == format!("{construct} is not supported before slice {slice}")
        })
    };
    match parse_through(entry, &source, limit) {
        Ok(_) => Some(true),
        Err(e) if e.len() == 1 && e.first()?.message.contains("parser_nesting") => Some(false),
        Err(e) if e.len() == 1 && held(&e.first()?.message) => Some(true),
        Err(_) => None,
    }
}

/// As [`admitted`], for the parser alone: `n` nested `kind`s with an unexpected `)` after them
/// reach that `)` (`Some(true)`), fail with the nesting error alone (`Some(false)`), or fail
/// otherwise (`None`). The parse is the one of [`admitted`] up to the end of the source, as deep
/// and with every node made and its depth checked; it stops at the `)` instead of going on to the
/// early errors and the scope analysis, which only a source that parses goes through.
fn admitted_by_the_parser(entry: Entry, kind: &str, n: usize, limit: u32) -> Option<bool> {
    let mut source = nested(kind, n)?;
    source.push(u16::from(b')'));
    match parse_through(entry, &source, limit) {
        Err(e) if e.len() == 1 && e.first()?.message.contains("parser_nesting") => Some(false),
        Err(e) if e.len() == 1 && e.first()?.message == "unexpected token )" => Some(true),
        _ => None,
    }
}

/// Every combination of a parse entry and a kind of nesting, kind by kind in the order of
/// `NESTING_KINDS`, each through the [`ENTRIES`] in order; so the three entries of one kind,
/// whose sources cost alike, fall into three parts ([`stack_bound_part`]).
fn nesting_combinations() -> Vec<(Entry, &'static str)> {
    NESTING_KINDS
        .iter()
        .flat_map(|&kind| ENTRIES.iter().map(move |&entry| (entry, kind)))
        .collect()
}

/// The combinations part `part` of [`STACK_BOUND_PARTS`] checks: every one whose index in
/// [`nesting_combinations`] leaves `part` when divided by the number of parts, 17: a prime, so
/// that the three entries of a kind fall into three parts and two kinds share parts only when
/// they stand 17 apart in the list. A kind added
/// to `NESTING_KINDS` lands in parts by its index, and no part is edited for it.
fn stack_bound_part(part: usize) -> Vec<(Entry, &'static str)> {
    nesting_combinations()
        .into_iter()
        .enumerate()
        .filter(|(i, _)| i % STACK_BOUND_PARTS.len() == part)
        .map(|(_, combination)| combination)
        .collect()
}

/// The deepest source of `kind` that `limit` admits through `entry`, found by a search on the
/// stack the test runs on, parses or fails with the nesting error alone, and so does one 100,000
/// deep ([`every_nesting_kind_fits_the_stack_bound`]).
///
/// The search bisects the depths between the deepest source the parser admitted and the
/// shallowest it refused with [`admitted_by_the_parser`], and only the deepest admitted source
/// goes through the early errors ([`admitted`]): they cost far more than the parse (a chain of
/// 5,000 labels: about 0.5 s against 0.03 s, as the early errors look each label up among those
/// around it), and a search whose every source that parses went through them made about five
/// such parses for labels at limit 10,000.
fn fits_the_stack_bound(entry: Entry, kind: &str, limit: u32) {
    let (mut parses, mut refused) = (0, limit as usize + 2);
    assert_eq!(
        admitted(entry, kind, refused, limit),
        Some(false),
        "{entry:?} {kind}"
    );
    while refused - parses > 1 {
        let mid = parses + (refused - parses) / 2;
        let fits = admitted_by_the_parser(entry, kind, mid, limit);
        assert!(
            fits.is_some(),
            "{entry:?} {kind} {mid}: an error other than the nesting error"
        );
        if fits == Some(true) {
            parses = mid;
        } else {
            refused = mid;
        }
    }
    // The whole text of a dynamic function holds the body a few levels deeper than the body
    // alone, whose parse the `)` stops, so through that entry the deepest source that parses
    // can be a few nestings shallower; a source that is refused costs a parse to the limit.
    while parses > 0 {
        let fits = admitted(entry, kind, parses, limit);
        assert!(
            fits.is_some(),
            "{entry:?} {kind} {parses}: an error other than the nesting error"
        );
        if fits == Some(true) {
            break;
        }
        parses -= 1;
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

/// The deep limit of the check of the stack bound, at which the per-level part of
/// `ParseOptions::stack_bytes` decides: 7,500 at the full count (the lane `slow`,
/// `SWELL_FULL_COUNTS=1`), 2,500 in the gate, each on the stack [`deep_stack`] gives. The full
/// count was 10,000, where the eighth part took 2.5 CPU seconds alone (measured on
/// 2026-11-22), at the lane's bound for a test; on its scaled stack 7,500 fails whenever 10,000
/// does ([`deep_stack`]).
///
/// A frame grows in steps of 16 bytes, but one nesting takes several levels (3 for an arrow
/// function, up to 14 for a getter or a method), so a growth of 16 bytes is 16 / k bytes a level
/// over `STACK_BYTES_PER_LEVEL`, and the stack overflows only once the levels have used up the
/// room the fixed part leaves of `STACK_BYTES_FIXED`: 21,564 bytes through `parse_script`, whose
/// fixed part took 11,204. Nested arrow functions (`b4c46f6`) took 688 bytes for a nesting of 3
/// levels, 229.3 bytes a level, 5.3 over: 13,300 bytes over at 2,500 levels, within that room,
/// and the gate's check passed on the stack `stack_bytes` gives; 53,300 at 10,000, where the
/// stack overflowed from limit 4,094 on (3,464 through the body of `parse_dynamic_function`,
/// whose fixed part is larger). So the gate's limit alone does not find a frame past the bound.
///
/// The bytes a level of the kinds closest to the bound, measured as the smallest stack that holds
/// the deepest source at limits 2,000 and 10,000: 181.3 (arrows), 164.6 (arrows with a block
/// body) and 160.0 (conditional expressions) in this build, dev without debug information;
/// 213.3, 196.6 and 192.0 in the profile `stack-debug`; 149.3 (`for`) at most in the release
/// profile. At 10,000 the 17 parts took 1.0 to 2.3 s of processor time each, 25.7 to 26.0 s
/// together (four runs), and in three runs of cargo-nextest up to 6.8, 11.6 and 28.8 s each at
/// loads of 74 to 101 on 20 cores; at 2,500, 0.24 to 0.60 s each and 6.7 s together (three
/// runs; 7.6 s with a search whose every source that parsed went through the early errors).
fn deep_limit() -> u32 {
    if std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1") {
        7_500
    } else {
        2_500
    }
}

/// The stack the child of the check at the deep limit `limit` runs on:
/// `ParseOptions::stack_bytes` less `STACK_BYTES_FIXED` × (10,000 − `limit`) / 10,000, that is
/// the stack of the check at 10,000 (2,272,768 bytes) scaled by `limit` / 10,000: at 10,000 the
/// stated stack, at 2,500 the stated 592,768 bytes less 24,576, a quarter of 2,272,768. With a
/// fixed part of f bytes and p bytes a level, the check at 10,000 overflows its stack S when
/// f + 10,000 p > S, and then f + 2,500 p ≥ (f + 10,000 p) / 4 > S / 4: the gate's check
/// overflows whenever the one at 10,000 does, whatever f is, and so does the lane's at 7,500,
/// f + 7,500 p ≥ 3 (f + 10,000 p) / 4 > 3 S / 4. The other way it is stricter, by
/// f × 3/4 over 2,500 levels: through `parse_script` (f = 11,204) by 3.4 bytes a level, so it
/// also fails a kind up to 1.2 bytes a level under the bound; the kinds closest to the bound
/// stand 42.7 bytes under it.
fn deep_stack(limit: u32) -> usize {
    let stated = ParseOptions { max_nesting: limit }.stack_bytes();
    let short = 10_000_usize.saturating_sub(limit as usize);
    stated.saturating_sub(swell_syntax::STACK_BYTES_FIXED * short / 10_000)
}

/// The tests of the stack bound, one per part, and the list of their parts,
/// [`STACK_BOUND_PARTS`], whose length is the number of parts. Each test checks the
/// combinations of its part ([`stack_bound_part`]) at the limit 10, on the stack
/// `ParseOptions::stack_bytes` gives for it, and at [`deep_limit`], on [`deep_stack`]; each in a
/// child process (the child inherits `SWELL_FULL_COUNTS`, so it runs the limit its parent asks
/// for).
macro_rules! stack_bound_parts {
    ($($name:ident = $part:literal),* $(,)?) => {
        /// The parts of the check of the stack bound, one test each ([`stack_bound_part`]).
        const STACK_BOUND_PARTS: &[usize] = &[$($part),*];
        $(
            #[test]
            fn $name() {
                let ten = ParseOptions { max_nesting: 10 }.stack_bytes();
                for (limit, stack) in [(10, ten), (deep_limit(), deep_stack(deep_limit()))] {
                    let test = test_path!(stringify!($name));
                    on_stack(test, limit, stack, |limit| {
                        for (entry, kind) in stack_bound_part($part) {
                            fits_the_stack_bound(entry, kind, limit);
                        }
                    })
                    .unwrap();
                }
            }
        )*
    };
}

// Every kind of nesting, at the deepest the limit admits and 100,000 deep, parses or fails on
// the stack `ParseOptions::stack_bytes` gives, in a child process, through each parse entry
// (`parse_script`, `parse_eval` and the body of `parse_dynamic_function`, whose three parses
// each stay within the limit): a test fails when a frame of the parser grows past
// `STACK_BYTES_PER_LEVEL`, or its fixed part past `STACK_BYTES_FIXED`, and the stack
// overflows. At the deep limit (`deep_limit`: 2,500 levels in the gate, on a quarter of the
// stack of 10,000, and 7,500 at the full count, on three quarters of it) the per-level bound
// decides; at 10 the fixed
// part does.
// The combinations of entry and kind are split over `every_nesting_kind_fits_the_stack_bound`,
// the first part, and `every_nesting_kind_fits_the_stack_bound_part_<n>`, so that under
// cargo-nextest each part runs in a process of its own and no part takes long; the parts
// together check each combination once (`the_parts_cover_every_combination`).
stack_bound_parts!(
    every_nesting_kind_fits_the_stack_bound = 0,
    every_nesting_kind_fits_the_stack_bound_part_2 = 1,
    every_nesting_kind_fits_the_stack_bound_part_3 = 2,
    every_nesting_kind_fits_the_stack_bound_part_4 = 3,
    every_nesting_kind_fits_the_stack_bound_part_5 = 4,
    every_nesting_kind_fits_the_stack_bound_part_6 = 5,
    every_nesting_kind_fits_the_stack_bound_part_7 = 6,
    every_nesting_kind_fits_the_stack_bound_part_8 = 7,
    every_nesting_kind_fits_the_stack_bound_part_9 = 8,
    every_nesting_kind_fits_the_stack_bound_part_10 = 9,
    every_nesting_kind_fits_the_stack_bound_part_11 = 10,
    every_nesting_kind_fits_the_stack_bound_part_12 = 11,
    every_nesting_kind_fits_the_stack_bound_part_13 = 12,
    every_nesting_kind_fits_the_stack_bound_part_14 = 13,
    every_nesting_kind_fits_the_stack_bound_part_15 = 14,
    every_nesting_kind_fits_the_stack_bound_part_16 = 15,
    every_nesting_kind_fits_the_stack_bound_part_17 = 16,
);

/// The parts of the check of the stack bound together check every combination of entry and kind
/// exactly once, each part is listed once, and the parts are numbered from 0 without a gap, so
/// a part is neither lost nor run twice ([`stack_bound_part`]).
#[test]
fn the_parts_cover_every_combination() {
    let all = nesting_combinations();
    assert_eq!(all.len(), ENTRIES.len() * NESTING_KINDS.len());
    let mut parts = STACK_BOUND_PARTS.to_vec();
    parts.sort_unstable();
    assert_eq!(parts, (0..STACK_BOUND_PARTS.len()).collect::<Vec<_>>());
    let mut checked: Vec<String> = Vec::new();
    for &part in STACK_BOUND_PARTS {
        let combinations = stack_bound_part(part);
        assert!(!combinations.is_empty(), "part {part} checks nothing");
        checked.extend(
            combinations
                .iter()
                .map(|(entry, kind)| format!("{entry:?} {kind}")),
        );
    }
    let mut expected: Vec<String> = all
        .iter()
        .map(|(entry, kind)| format!("{entry:?} {kind}"))
        .collect();
    checked.sort();
    expected.sort();
    assert_eq!(checked, expected);
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
    let names: Vec<&str> = f.parameter_names().map(|p| p.name.as_str()).collect();
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
        ("a = 1", "return a"),
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
        ("a <!-- x", ""),
        ("", "}"),
        ("", "break;"),
    ] {
        assert!(parsed(p, b).is_err(), "{p:?} {b:?} parsed");
    }
    let errors = parsed("a <!-- x", "").err().unwrap();
    assert_eq!(
        messages(&errors),
        vec!["HTML-like comment is not supported before slice 18".to_string()]
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

/// The shape of the expression `e`: each binary, logical or assignment operation in parentheses
/// with its `BinaryOp`, `LogicalOp` or `AssignOp` between its operands, a prefix operator by its `UnaryOp` or `UpdateOp` before its operand
/// in brackets, a postfix one after it, a literal by its text and an identifier by its name.
fn shape(ast: &Ast, e: ExprId) -> String {
    let Some(node) = ast.expr(e) else {
        return "<no node>".to_string();
    };
    match &node.expr {
        Expr::Binary { op, left, right } => {
            format!("({} {op:?} {})", shape(ast, *left), shape(ast, *right))
        }
        Expr::Logical { op, left, right } => {
            format!("({} {op:?} {})", shape(ast, *left), shape(ast, *right))
        }
        Expr::Assign { op, target, value } => {
            format!("({} {op:?} {})", shape(ast, *target), shape(ast, *value))
        }
        Expr::Unary { op, operand } => format!("{op:?}[{}]", shape(ast, *operand)),
        Expr::Update {
            op,
            prefix: true,
            operand,
        } => format!("{op:?}[{}]", shape(ast, *operand)),
        Expr::Update { op, operand, .. } => format!("[{}]{op:?}", shape(ast, *operand)),
        Expr::Number(n) => n.text.clone(),
        Expr::Identifier(name) => name.clone(),
        other => format!("{other:?}"),
    }
}

/// The shape ([`shape`]) of the expression of the first statement of `text`; the errors of a
/// parse, or the statement when it is no expression statement, as their debug text.
fn statement_shape(text: &str) -> String {
    let script = match parse(text) {
        Ok(script) => script,
        Err(errors) => return format!("{errors:?}"),
    };
    let first = script
        .body
        .first()
        .and_then(|&s| script.ast.stmt(s))
        .map(|s| &s.stmt);
    match first {
        Some(Stmt::Expression(e)) => shape(&script.ast, *e),
        other => format!("{other:?}"),
    }
}

/// `**` (`#sec-exp-operator`): ExponentiationExpression is `UpdateExpression **
/// ExponentiationExpression`, so the operator is right associative and binds more tightly than
/// `*`, `/` and `%`; a unary operator binds its own operand before `**` on the right and is a
/// SyntaxError as the left operand outside parentheses, while an update expression is a left
/// operand. A chain of 2,000 `**` builds a tree 2,001 nodes deep by a loop, as a chain of `+`
/// does: at `parser_nesting` 1,000 it is the nesting error alone, and at 2,100 it parses.
#[test]
fn exponentiation_is_right_associative_and_binds_tightest() {
    for (text, expected) in [
        ("2 ** 3 ** 2;", "(2 Exp (3 Exp 2))"),
        ("2 * 3 ** 2;", "(2 Mul (3 Exp 2))"),
        ("2 ** 3 * 2;", "((2 Exp 3) Mul 2)"),
        ("a ** b * c ** d;", "((a Exp b) Mul (c Exp d))"),
        ("a % b ** c ** d;", "(a Rem (b Exp (c Exp d)))"),
        ("a - b ** c;", "(a Sub (b Exp c))"),
        ("2 ** -2;", "(2 Exp Minus[2])"),
        ("(-2) ** 2;", "(Minus[2] Exp 2)"),
        ("-(2 ** 2);", "Minus[(2 Exp 2)]"),
        ("++a ** 2;", "(Increment[a] Exp 2)"),
        ("a-- ** 2;", "([a]Decrement Exp 2)"),
    ] {
        assert_eq!(statement_shape(text), expected, "{text}");
    }
    for text in [
        "-2 ** 2;",
        "2 ** -2 ** 2;",
        "a * typeof b ** 2;",
        "!a ** 2;",
    ] {
        assert_eq!(
            parse(text).err().map(|e| messages(&e)),
            Some(vec![
                "a unary expression as the left operand of ** needs parentheses".to_string()
            ]),
            "{text}"
        );
    }
    let chain = format!("x = a{};", " ** a".repeat(2_000));
    let errors = parse_units(&units(&chain), LIMIT).err().unwrap();
    assert_eq!(
        messages(&errors),
        vec!["nesting deeper than parser_nesting (1000)".to_string()]
    );
    let script = parse_units(&units(&chain), 2_100).unwrap();
    let Some(Stmt::Expression(e)) = script.ast.stmt(script.body[0]).map(|s| &s.stmt) else {
        panic!("not an expression statement");
    };
    let Expr::Assign { value, .. } = &script.ast.expr(*e).unwrap().expr else {
        panic!("not an assignment");
    };
    let depth = script.ast.expr(*value).unwrap().depth;
    println!("a chain of 2,000 **: depth {depth}");
    assert_eq!(depth, 2_001);
}

/// Sources ExponentiationExpression (`#sec-exp-operator`: UnaryExpression | UpdateExpression
/// `**` ExponentiationExpression) and `**=` (`#sec-assignment-operators`) admit, with their
/// neighbours of every precedence, `await` and `yield` as operands and as names, templates,
/// optional chains, arrow functions and regular expressions: each parses.
#[test]
fn exponentiation_sources_the_grammar_admits_parse() {
    let admitted = [
        "(-a) ** b;",
        "(+a) ** b;",
        "(!a) ** b;",
        "(~a) ** b;",
        "(typeof a) ** b;",
        "(void a) ** b;",
        "(delete a.b) ** c;",
        "async function f() { (await a) ** b; }",
        "async function f() { a ** await b; }",
        "async function f() { a ** -await b; }",
        "await ** 2;",
        "async function f() { function g() { return await ** 2; } }",
        "a ** -b;",
        "a ** typeof b;",
        "a ** void b;",
        "a ** !b;",
        "a ** ~b;",
        "a ** delete b.c;",
        "a ** b ** c;",
        "(a ** b) ** c;",
        "a * b ** c;",
        "a ** b * c;",
        "new a ** b;",
        "new a() ** b;",
        "a() ** b;",
        "a.b ** c;",
        "a[0] ** b;",
        "`t` ** 2;",
        "a`t` ** 2;",
        "2 ** `${a ** b}`;",
        "a?.b ** c;",
        "a ** b?.c;",
        "a ?? b ** c;",
        "a ** b ?? c;",
        "a ** b in c;",
        "a in b ** c;",
        "a ** b instanceof c;",
        "function* g() { yield a ** b; }",
        "function* g() { (yield) ** 2; }",
        "async () => a ** b;",
        "x => x ** 2;",
        "a ** b ? c : d;",
        "x = a ** b, c;",
        "a++ ** b;",
        "a-- ** b;",
        "++a ** b;",
        "--a ** b;",
        "a ** ++b;",
        "a ** b++;",
        "a ** (-b) ** c;",
        "a**b;",
        "2**-1;",
        "a ** /re/.source.length;",
        "a **= /re/.lastIndex;",
        ".5 ** 2;",
        "1. ** 2;",
        "a **= b **= c;",
        "a **= -b;",
        "a **= b ** c;",
        "x.y **= 2;",
        "x[i] **= 2;",
        "(a) **= 2;",
        "(x.y) **= 2;",
        "((a)) **= 2;",
        "for (x of a ** b);",
        "for (var i = 2 ** 3; i < 1; i++);",
        "with (o) a **= 2;",
        "eval **= 2;",
        "arguments **= 2;",
        "-(a ** b);",
        "typeof (a ** b);",
        "(a, b) ** c;",
        "(x = 2 ** 3) => x;",
        "async (x = 2 ** 3) => x;",
        "({ m() { return a ** b; } });",
        "a ** function () {};",
        "a ** (0);",
        "a ** [1];",
        "a ** ({}).b;",
    ];
    let mut wrong = Vec::new();
    for text in admitted {
        if let Err(errors) = parse(text) {
            wrong.push(format!("{text} -> {errors:?}"));
        }
    }
    assert!(wrong.is_empty(), "refused:\n{}", wrong.join("\n"));
}

/// Sources the grammar does not admit (`#sec-exp-operator`; for `**=` the early errors of
/// `#sec-assignment-operators-static-semantics-early-errors`): a unary expression or `await` as
/// the left operand, a `**` expression as an assignment target or arrow parameter, targets that
/// are not simple, `yield` as an operand of `**`, and broken tokens: each fails.
#[test]
fn exponentiation_sources_the_grammar_refuses_fail() {
    let refused = [
        "-a ** b;",
        "+a ** b;",
        "!a ** b;",
        "~a ** b;",
        "typeof a ** b;",
        "void a ** b;",
        "delete a.b ** c;",
        "async function f() { await a ** b; }",
        "async () => await a ** b;",
        "async function f() { a ** await b ** c; }",
        "a ** -b ** c;",
        "-a ** -b;",
        "- -a ** b;",
        "-(a) ** b;",
        "typeof (a) ** b;",
        "a * -b ** c;",
        "-++a ** b;",
        "!a-- ** b;",
        "a ** b = c;",
        "a ** b **= c;",
        "(a ** b) = c;",
        "(a ** b) **= c;",
        "a ** b => c;",
        "a ** () => b;",
        "a ** async () => b;",
        "a ** async x => b;",
        "a ?. b **= c;",
        "a?.[b] **= c;",
        "[a] **= b;",
        "({a} **= b);",
        "({a}) **= b;",
        "\"use strict\"; eval **= 2;",
        "\"use strict\"; arguments **= 2;",
        "\"use strict\"; (eval) **= 2;",
        "a++ **= 2;",
        "++a **= 2;",
        "a ** b++ **= 2;",
        "this **= 2;",
        "1 **= 2;",
        "'s' **= 2;",
        "function* g() { yield ** 2; }",
        "function* g() { a ** yield b; }",
        "a ** ** b;",
        "a *** b;",
        "a ** = b;",
        "a * * b;",
        "a ** b ** ;",
        "a **;",
        "** a;",
        "x = a ?? b ** c || d;",
        "for (a ** b of c);",
        "for (a ** b in c);",
        "(-a ** b) => 1;",
        "(x = -a ** b) => x;",
    ];
    let mut wrong = Vec::new();
    for text in refused {
        if parse(text).is_ok() {
            wrong.push(text.to_string());
        }
    }
    assert!(wrong.is_empty(), "accepted:\n{}", wrong.join("\n"));
}

/// `**=` takes exactly the targets `*=` takes (`#sec-assignment-operators-static-semantics-early-errors`:
/// AssignmentTargetType of the LeftHandSideExpression is simple for both): over 35 targets, in
/// sloppy and strict code, each source with `*=` and with `**=` parses or fails alike.
#[test]
fn exponent_assignment_takes_the_targets_of_multiplication_assignment() {
    let targets = [
        "a",
        "(a)",
        "((a))",
        "a.b",
        "a[b]",
        "(a.b)",
        "(a[b])",
        "a.b.c",
        "a()",
        "a().b",
        "new a",
        "new a().b",
        "a?.b",
        "a?.b.c",
        "(a?.b)",
        "[a]",
        "({a})",
        "this",
        "this.a",
        "1",
        "a++",
        "++a",
        "-a",
        "(a, b)",
        "a = b",
        "(a = b)",
        "eval",
        "arguments",
        "`t`",
        "a`t`",
        "async",
        "await",
        "yield",
        "let",
        "static",
    ];
    let mut wrong = Vec::new();
    for prefix in ["", "\"use strict\"; "] {
        for target in targets {
            let star = format!("{prefix}{target} *= 2;");
            let exp = format!("{prefix}{target} **= 2;");
            let (s, e) = (parse(&star).is_ok(), parse(&exp).is_ok());
            if s != e {
                wrong.push(format!("{star} -> {s}, {exp} -> {e}"));
            }
        }
    }
    assert!(wrong.is_empty(), "differ:\n{}", wrong.join("\n"));
}

/// The tree shapes ([`shape`]) of 24 sources (`#sec-exp-operator`, and the clauses of the
/// neighbouring operators 13.7 to 13.15): right associativity of `**` and `**=`, and the
/// precedence of `**` over every binary, logical and assignment neighbour.
#[test]
fn exponentiation_tree_shapes() {
    let cases = [
        ("a ** b ** c ** d;", "(a Exp (b Exp (c Exp d)))"),
        ("a * b ** c * d;", "((a Mul (b Exp c)) Mul d)"),
        ("a ** b * c ** d ** e;", "((a Exp b) Mul (c Exp (d Exp e)))"),
        ("a + b ** c * d;", "(a Add ((b Exp c) Mul d))"),
        ("a ** b + c;", "((a Exp b) Add c)"),
        ("a / b ** c / d;", "((a Div (b Exp c)) Div d)"),
        ("a ?? b ** c;", "(a Coalesce (b Exp c))"),
        ("a ** b ?? c;", "((a Exp b) Coalesce c)"),
        ("a ** b < c ** d;", "((a Exp b) Lt (c Exp d))"),
        ("a ** b == c;", "((a Exp b) Eq c)"),
        ("a | b ** c & d;", "(a BitOr ((b Exp c) BitAnd d))"),
        ("a ** b << c;", "((a Exp b) Shl c)"),
        ("a << b ** c;", "(a Shl (b Exp c))"),
        ("a ** -b;", "(a Exp Minus[b])"),
        ("a ** typeof b;", "(a Exp Typeof[b])"),
        ("a ** (b ** c);", "(a Exp (b Exp c))"),
        ("(a ** b) ** c;", "((a Exp b) Exp c)"),
        ("(-(a ** b)) ** c;", "(Minus[(a Exp b)] Exp c)"),
        ("a **= b ** c;", "(a Compound(Exp) (b Exp c))"),
        ("a **= b **= c;", "(a Compound(Exp) (b Compound(Exp) c))"),
        ("a = b ** c ** d;", "(a Assign (b Exp (c Exp d)))"),
        ("a || b ** c && d;", "(a Or ((b Exp c) And d))"),
        ("a ** b ** c * d ** e;", "((a Exp (b Exp c)) Mul (d Exp e))"),
        ("a - b ** c ** d - e;", "((a Sub (b Exp (c Exp d))) Sub e)"),
    ];
    let mut wrong = Vec::new();
    for (text, expected) in cases {
        let got = statement_shape(text);
        if got != expected {
            wrong.push(format!("{text}: got {got}, expected {expected}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Long chains end in one error each at `parser_nesting` 1,000 (`#sec-exp-operator`,
/// `#sec-assignment-operators`): a right-deep chain of 5,000 `**=`, a chain of 100,000 `**` as
/// the value of an assignment and the same chain as the argument of a call, in the nesting error
/// alone; a chain of 5,000 `**` with a unary right operand on each level in the unary-base
/// SyntaxError alone, at its second `**`, whose left operand is `-a`.
#[test]
fn exponent_chains_are_bounded() {
    let nesting = "nesting deeper than parser_nesting (1000)";
    let unary = "a unary expression as the left operand of ** needs parentheses";
    for (text, expected) in [
        (format!("x = a{};", " ** -a".repeat(5_000)), unary),
        (format!("{}0;", "a **= ".repeat(5_000)), nesting),
        (format!("x = a{};", " ** a".repeat(100_000)), nesting),
        (format!("f(a{});", " ** a".repeat(100_000)), nesting),
    ] {
        let result = parse(&text);
        let Err(errors) = result else {
            panic!("a chain of 5,000 or 100,000 parsed at {LIMIT}");
        };
        assert_eq!(messages(&errors), vec![expected.to_string()]);
    }
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

// The class elements of `story:class-element-syntax-and-early-errors`, each source against the
// verdict of the pinned ECMA-262 draft, through every parse entry. `H` is a source the standard
// admits, which parses or answers the hold of its first held element; `E` is a SyntaxError of
// the standard, a grammar or an early error and never a refusal. Every mismatch of a list is
// reported at once.

/// The expected answer of a source of these lists.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Verdict {
    /// Admitted by the standard: the source parses, or its one error is the refusal of a line of
    /// [`swell_syntax::HOLDS`] (a class is not held, so a class of methods alone parses).
    H,
    /// A SyntaxError of the standard: an error that is no refusal.
    E,
}

/// The mismatches of `cases` through `entries`, each with what the parser answered.
fn verdict_mismatches(entries: &[Entry], cases: &[(&str, Verdict)]) -> Vec<String> {
    let held = |message: &str| {
        swell_syntax::HOLDS.iter().any(|(construct, slice)| {
            message == format!("{construct} is not supported before slice {slice}")
        })
    };
    let mut wrong = Vec::new();
    for &(text, verdict) in cases {
        for &entry in entries {
            let answer = parse_through(entry, &units(text), LIMIT).map(|_| ());
            let ok = match (&answer, verdict) {
                (Ok(()), Verdict::H) => true,
                (Err(errors), Verdict::H) => {
                    matches!(errors.as_slice(), [error] if held(&error.message))
                }
                (Err(errors), Verdict::E) => errors
                    .iter()
                    .all(|e| !e.message.contains("is not supported before slice")),
                (Ok(()), Verdict::E) => false,
            };
            if !ok {
                let got = answer.map_err(|errors| messages(&errors));
                wrong.push(format!(
                    "{entry:?} {text:?}: expected {verdict:?}, got {got:?}"
                ));
            } else if verdict == Verdict::E && matches!(entry, Entry::Script) {
                let got = answer.map_err(|errors| messages(&errors));
                println!("E {text:?}: {got:?}");
            }
        }
    }
    wrong
}

fn check_verdicts(cases: &[(&str, Verdict)]) {
    let wrong = verdict_mismatches(&ENTRIES, cases);
    assert!(
        wrong.is_empty(),
        "{} mismatch(es):\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// Point 6 of the unit's notes: [Yield] and [Await] cleared in a field initializer and a static
/// block, then refused by an early error. `FieldDefinition[?Yield, ?Await]` with
/// `Initializer[+In, ?Yield, ?Await]`; `ClassStaticBlockStatementList : StatementList[~Yield,
/// +Await, ~Return]` and "Contains await" (`#sec-class-definitions-static-semantics-early-errors`),
/// where Contains looks into no function, arrow function included, for `await`
/// (`#sec-static-semantics-contains`); an arrow's ConciseBody is `[~Await]`
/// (`#sec-arrow-function-definitions`).
#[test]
fn await_and_yield_in_class_elements_answer_the_standard() {
    use Verdict::{E, H};
    check_verdicts(&[
        ("var await; class C { x = await; }", H),
        ("var await; class C { static x = await; }", H),
        ("var await; class C { [await] = 1; }", H),
        ("var await; class C { x = class { [await] = 1; }; }", H),
        ("async function f() { class C { [await 1] = 2; } }", H),
        ("function* g() { class C { [yield 1] = 2; } }", H),
        ("function* g() { class C { [yield] = 2; } }", H),
        ("async function f() { class C { x = () => await; } }", H),
        (
            "async function f() { class C { x = function () { await; }; } }",
            H,
        ),
        (
            "async function f() { class C { x = async () => await 1; } }",
            H,
        ),
        (
            "function* g() { class C { x = function* () { yield 1; }; } }",
            H,
        ),
        ("class C { static { (() => { class await {} }); } }", H),
        ("class C { static { (() => ({ await })); } }", H),
        ("class C { static { (() => await); } }", H),
        ("class C { static { () => { await: ; }; } }", H),
        ("class C { static { (function await(await) {}); } }", H),
        ("class C { static { ({ await: 0 }); } }", H),
        ("class C { static { ({ method(await) {} }); } }", H),
        ("class C { static { ({ get await() {} }); } }", H),
        (
            "var await = 0; class C { static { new (class { constructor(x = await) { await; } }); } }",
            H,
        ),
        ("class C { static { async () => await 1; } }", H),
        (
            "class C { static { (async function () { await 1; }); } }",
            H,
        ),
        ("async function f() { class C { x = await; } }", E),
        ("async function f() { class C { x = (await) => 1; } }", E),
        (
            "async function f() { class C { x = class { [await] = 1; }; } }",
            E,
        ),
        ("function* g() { class C { x = yield; } }", E),
        ("class C { x = yield; }", E),
        ("class C { static { await; } }", E),
        ("class C { static { (await); } }", E),
        ("class C { static { function await() {} } }", E),
        ("class C { static { function* await() {} } }", E),
        ("class C { static { async function await() {} } }", E),
        ("class C { static { class await {} } }", E),
        ("class C { static { ({ await }); } }", E),
        ("class C { static { (await) => 0; } }", E),
        ("class C { static { await => 0; } }", E),
        ("class C { static { (x = await) => 0; } }", E),
        ("class C { static { async (x = await 1) => x; } }", E),
        ("class C { static { class D { [await 1]; } } }", E),
        ("class C { static { class D { [await] = 1; } } }", E),
        ("class C { static { class D extends await {} } }", E),
        ("class C { static { await: ; } }", E),
        ("class C { static { x = { [await]: 1 }; } }", E),
        ("class C { static { var await; } }", E),
        ("class C { static { try {} catch (await) {} } }", E),
        ("class C { static { let [await] = []; } }", E),
        ("class C { static { for await (x of y); } }", E),
        ("function* g() { class C { static { yield; } } }", E),
    ]);
}

/// ContainsArguments (`#sec-static-semantics-containsarguments`) looks into everything but a
/// function and a method's body: into arrow functions, and into class bodies (a nested class's
/// computed keys and heritage) too; a SuperCall in an initializer or a static block is an
/// error, a SuperProperty and `new.target` are not (`#sec-class-definitions-static-semantics-early-errors`,
/// `#sec-static-semantics-contains`).
#[test]
fn arguments_and_super_in_class_elements_answer_the_standard() {
    use Verdict::{E, H};
    check_verdicts(&[
        ("class C { x = arguments; }", E),
        ("class C { static x = arguments; }", E),
        ("class C { #x = arguments; }", E),
        ("class C { x = () => arguments; }", E),
        ("class C { x = async () => arguments; }", E),
        ("class C { x = (a = arguments) => a; }", E),
        ("class C { x = { arguments }; }", E),
        ("class C { x = typeof arguments; }", E),
        ("class C { x = `${arguments}`; }", E),
        ("class C { x = argument\\u0073; }", E),
        ("class C { x = { [arguments]: 1 }; }", E),
        ("class C { x = { [arguments]() {} }; }", E),
        ("class C { x = class { [arguments] = 1; }; }", E),
        ("class C { x = class { [arguments]() {} }; }", E),
        ("class C { x = class extends arguments {}; }", E),
        ("class C { static { arguments; } }", E),
        ("class C { static { () => arguments; } }", E),
        ("class C { static { class D { [arguments] = 1; } } }", E),
        (
            "class C { static { class D { static [arguments]() {} } } }",
            E,
        ),
        ("class C { x = function () { arguments; }; }", H),
        ("class C { x = { m() { arguments; } }; }", H),
        ("class C { x = class { m() { arguments; } }; }", H),
        ("function f() { class C { [arguments] = 1; } }", H),
        ("function f() { class C { static [arguments] = 1; } }", H),
        ("class C { static { function f() { arguments; } } }", H),
        (
            "class C { static { (function () { return arguments; }); } }",
            H,
        ),
        ("class C extends B { x = super(); }", E),
        ("class C extends B { x = () => super(); }", E),
        ("class C extends B { static { super(); } }", E),
        ("class C extends B { static { () => super(); } }", E),
        ("class C { x = function () { super.y; }; }", E),
        ("class C { static { function f() { super.y; } } }", E),
        ("class C { x = super.y; }", H),
        ("class C { x = super[0]; }", H),
        // `super` in an arrow function answers the refusal of slice 8 of
        // `story:super-property-and-super-syntax` at its token, in an initializer and a static
        // block as in a method: no source of this list.
        ("class C { static { super.y; } }", H),
        ("class C { x = { m() { super.y; } }; }", H),
        ("class C { x = new.target; }", H),
        ("class C { x = () => new.target; }", H),
        ("class C { static { new.target; } }", H),
    ]);
}

/// The rules of `ClassStaticBlockBody` (`#sec-class-definitions-static-semantics-early-errors`):
/// its own var and top-level lexical names (TopLevelLexicallyDeclaredNames: a function
/// declaration at its top level is a var name, `#sec-static-semantics-toplevellexicallydeclarednames`),
/// labels, `break` and `continue` targets and no `return`; and class code is strict.
#[test]
fn static_block_statements_answer_the_standard() {
    use Verdict::{E, H};
    check_verdicts(&[
        ("class C { static { return; } }", E),
        ("function f() { class C { static { return; } } }", E),
        ("while (0) { class C { static { break; } } }", E),
        ("while (0) { class C { static { continue; } } }", E),
        ("L: { class C { static { break L; } } }", E),
        ("class C { static { var x; let x; } }", E),
        ("class C { static { let x; var x; } }", E),
        ("class C { static { let x; let x; } }", E),
        ("class C { static { L: L: ; } }", E),
        ("class C { static { function f() {} let f; } }", E),
        ("class C { static { class D {} var D; } }", E),
        ("class C { static { with (o) {} } }", E),
        ("class C { static { var eval; } }", E),
        ("class C { static { x = 010; } }", E),
        ("class C { x = 010; }", E),
        ("class C { x = function () { with (o) {} }; }", E),
        ("class C { 01 = 1; }", E),
        ("class C { static { L: ; } static { L: ; } }", H),
        ("L: { class C { static { L: ; } } }", H),
        ("class C { static { L: { break L; } } }", H),
        ("class C { static { L: while (0) { continue L; } } }", H),
        ("class C { static { var x; } static { var x; } }", H),
        ("class C { static { var x; { let x; } } }", H),
        ("let x; class C { static { var x; } }", H),
        ("class C { static { function f() {} var f; } }", H),
        ("class C { static { function f() {} function f() {} } }", H),
        ("class C { static { async function f() {} var f; } }", H),
        ("class C { static { function f() { return 1; } } }", H),
        ("class C { static { x = () => { return 1; }; } }", H),
    ]);
}

/// The names of fields and private names: `constructor` and `prototype`
/// (PropName, a StringValue, escapes included; a computed key has none), `#constructor`, and the
/// duplicates of PrivateBoundIdentifiers but for one getter and one setter of the same placement
/// (`#sec-class-definitions-static-semantics-early-errors`); `static` with a private name named
/// `#prototype` has no PropName "prototype".
#[test]
fn element_names_and_duplicates_answer_the_standard() {
    use Verdict::{E, H};
    check_verdicts(&[
        ("class C { constructor; }", E),
        ("class C { 'constructor' = 1; }", E),
        ("class C { \"constructor\"; }", E),
        ("class C { c\\u006fnstructor = 1; }", E),
        ("class C { static constructor = 1; }", E),
        ("class C { static 'constructor'; }", E),
        ("class C { static prototype; }", E),
        ("class C { static 'prototype' = 1; }", E),
        ("class C { static pr\\u006ftotype; }", E),
        ("class C { ['constructor'] = 1; }", H),
        ("class C { static ['prototype'] = 1; }", H),
        ("class C { static ['constructor']; }", H),
        ("class C { prototype = 1; }", H),
        ("class C { static constructor() {} }", H),
        ("class C { static #prototype; }", H),
        ("class C { static #prototype() {} }", H),
        ("class C { #constructor; }", E),
        ("class C { static #constructor; }", E),
        ("class C { #constructor() {} }", E),
        ("class C { get #constructor() {} }", E),
        ("class C { static async *#constructor() {} }", E),
        ("class C { #\\u0063onstructor; }", E),
        ("class C { #x; #x; }", E),
        ("class C { #x; #x() {} }", E),
        ("class C { #x() {} #x; }", E),
        ("class C { #x; static #x; }", E),
        ("class C { #x() {} static #x() {} }", E),
        ("class C { get #x() {} get #x() {} }", E),
        ("class C { set #x(v) {} set #x(v) {} }", E),
        ("class C { get #x() {} static set #x(v) {} }", E),
        ("class C { static get #x() {} set #x(v) {} }", E),
        ("class C { get #x() {} set #x(v) {} get #x() {} }", E),
        ("class C { get #x() {} set #x(v) {} #x; }", E),
        ("class C { #x; get #x() {} }", E),
        ("class C { get #x() {} #x() {} }", E),
        ("class C { #\\u0078; #x; }", E),
        ("class C { get #x() {} set #x(v) {} }", H),
        ("class C { set #x(v) {} get #x() {} }", H),
        ("class C { static get #x() {} static set #x(v) {} }", H),
        (
            "class C { get #x() {} set #x(v) {} static get #y() {} static set #y(v) {} }",
            H,
        ),
        ("class C { #x; m() { class D { #x; } } }", H),
        ("class C { #a; #b; }", H),
    ]);
}

/// AllPrivateIdentifiersValid (`#sec-static-semantics-allprivateidentifiersvalid`): the names of a
/// ClassBody are in scope in all of that body, its computed keys included, and in nothing outside
/// it, its own ClassHeritage included; a class in a heritage or a key brings its own.
#[test]
fn private_names_are_in_scope_in_their_class_body_only() {
    use Verdict::{E, H};
    check_verdicts(&[
        ("class C { m() { this.#x; } #x; }", H),
        ("class C { #x; m() { class D { n() { this.#x; } } } }", H),
        (
            "class C { #x; m() { class D extends (o => o.#x, Object) {} } }",
            H,
        ),
        ("class C { #x; [o.#x] = 1; }", H),
        ("class C { #x; static [o.#x]() {} }", H),
        ("class C { #x; [class { m() { o.#x; } }] = 1; }", H),
        ("class C { #x; m() { class D { [o.#x] = 1; } } }", H),
        ("class C { #x; static f = () => o.#x; }", H),
        ("class C { #x; static { (function () { o.#x; }); } }", H),
        ("class C { #x; m() { o?.a.#x; } }", H),
        ("class C extends (class { #x; m() { this.#x; } }) {}", H),
        ("class C extends (class { #x; m() { o.#x; } }) { #x; }", H),
        ("x = class { #x; m() { return #x in this; } };", H),
        ("class C extends (o => o.#x, Object) { #x; }", E),
        ("class C extends (class { m() { o.#x; } }) { #x; }", E),
        ("class C extends (function () { o.#x; }) { #x; }", E),
        ("x = class C extends (o => o.#x, Object) { #x; };", E),
        ("class C extends (class { #x; }) { m() { this.#x; } }", E),
        ("class A { #x; } class B { m() { this.#x; } }", E),
        ("class C { #x; } o.#x;", E),
        ("class C { #x; } (function () { o.#x; });", E),
        ("class C { [o.#x] = 1; }", E),
        ("class C { #x; m() { o?.#y; } }", E),
        ("class C { m() { return #y in o; } }", E),
        ("class C { #x; m() { class D { #y; } o.#y; } }", E),
    ]);
}

/// `delete` of a private reference, through parentheses and in an optional chain
/// (`#sec-delete-operator-static-semantics-early-errors`); `#x in` where a RelationalExpression
/// with [+In] may stand (`#sec-relational-operators`).
#[test]
fn delete_and_private_in_answer_the_standard() {
    use Verdict::{E, H};
    let class = |body: &str| format!("class C {{ #x; *m(o, p) {{ {body} }} }}");
    let mut cases: Vec<(String, Verdict)> = Vec::new();
    for (body, verdict) in [
        ("delete this.#x;", E),
        ("delete (this.#x);", E),
        ("delete ((this.#x));", E),
        ("delete this?.#x;", E),
        ("delete (this?.#x);", E),
        ("delete o?.b.#x;", E),
        ("delete this.#x.y;", H),
        ("delete this.#x?.y;", H),
        ("delete this.#x[0];", H),
        ("delete this.#x();", H),
        ("for ((#x in o);;);", H),
        ("for (var y = (#x in o);;);", H),
        ("for (var y = #x in o;;);", E),
        ("for (() => #x in o;;);", E),
        ("return () => #x in o;", H),
        ("return a | #x in o;", H),
        ("return a ?? #x in o;", H),
        ("return #x in o ?? a;", H),
        ("return #x in o instanceof p;", H),
        ("return (#x in o) in p;", H),
        ("return 1 == #x in o;", H),
        ("return 1 + (#x in o);", H),
        ("return !(#x in o);", H),
        ("return [#x in o];", H),
        ("return a(#x in o);", H),
        ("return { [#x in o]: 1 };", H),
        ("return `${#x in o}`;", H),
        ("return #x\nin o;", H),
        ("return #x in o, #x in p;", H),
        ("yield #x in o;", H),
        ("x += #x in o;", H),
        ("if (#x in o) {}", H),
        ("return #x in /re/g;", H),
        ("return typeof #x in o;", E),
        ("return 1 << #x in o;", E),
        ("return #x in;", E),
        ("return #x in #x;", E),
        ("return #x i\\u006e o;", E),
        ("return -#x in o;", E),
        ("return #x ** 2 in o;", E),
    ] {
        cases.push((class(body), verdict));
    }
    cases.push(("class C { #x; m(a = #x in this) {} }".to_string(), H));
    let cases: Vec<(&str, Verdict)> = cases.iter().map(|(t, v)| (t.as_str(), *v)).collect();
    check_verdicts(&cases);
}

/// The PrivateIdentifier token (`#sec-names-and-keywords`): `#` and an IdentifierName, escapes
/// included, with no white space between; the member forms and their assignment targets
/// (`#sec-left-hand-side-expressions`, `#sec-static-semantics-assignmenttargettype`), an optional
/// chain with a template (`#sec-left-hand-side-expressions-static-semantics-early-errors`) and no
/// PrivateIdentifier in an object literal, a pattern or after `super.`.
#[test]
fn private_name_tokens_and_member_forms_answer_the_standard() {
    use Verdict::{E, H};
    let class = |body: &str| format!("class C {{ #x; #xa; m(a, o) {{ {body} }} }}");
    let mut cases: Vec<(String, Verdict)> = Vec::new();
    for (body, verdict) in [
        ("a. /* c */ #x;", H),
        ("a.\n#x;", H),
        ("a.#x\\u0061;", H),
        ("a?.#x;", H),
        ("a?.[0].#x;", H),
        ("a.#x?.b;", H),
        ("a.#x.#x;", H),
        ("a?.#x.#x?.#x;", H),
        ("a.#x(); a?.#x();", H),
        ("(a?.#x)();", H),
        ("a.#x`t`;", H),
        ("new a.#x;", H),
        ("new a.#x();", H),
        ("a.#x++; --a.#x;", H),
        ("a.#x = 1; a.#x += 1; a.#x ??= 1;", H),
        ("[a.#x] = o;", H),
        ("({ y: a.#x } = o);", H),
        ("[...a.#x] = o;", H),
        ("for (a.#x of o);", H),
        ("for (a.#x in o);", H),
        ("return a.#x / 2 / 1;", H),
        ("return a.#x\n/1/g;", H),
        ("a?.#x`t`;", E),
        ("a?.b.#x`t`;", E),
        ("a?.#x = 1;", E),
        ("a?.#x++;", E),
        ("[a?.#x] = o;", E),
        ("a.# x;", E),
        ("a.#\nx;", E),
        ("a.#!x;", E),
        ("a.#1;", E),
        ("a.#;", E),
        ("super.#x;", E),
        ("({ #x: 1 });", E),
        ("({ #x });", E),
        ("({ #x() {} });", E),
        ("({ get #x() {} });", E),
        ("({ async #x() {} });", E),
        ("var { #x: y } = o;", E),
        ("#x;", E),
        ("a[#x];", E),
        ("a.#y;", E),
        ("a.#X;", E),
    ] {
        cases.push((class(body), verdict));
    }
    for (text, verdict) in [
        ("class C { #\\u{61}; m() { this.#a; } }", H),
        ("class C { #a; m() { this.#\\u0061; } }", H),
        ("class C { #\\u{1D49C}; m() { this.#\\u{1D49C}; } }", H),
        ("class C { #\u{e9}; m() { this.#\u{e9}; } }", H),
        ("class C { #a\\u200C; m() { this.#a\\u200C; } }", H),
        (
            "class C { #class; #in; m() { this.#class; return #in in this; } }",
            H,
        ),
        ("class C { #\\u0030; }", E),
        ("class C { #\\u200C; }", E),
        ("class C { #1; }", E),
        ("class C { # x; }", E),
        ("class C { #\nx; }", E),
        ("class C { #", E),
        ("class C { #x", E),
        ("class C { #x = ", E),
        ("class C { static {", E),
        ("class C { static { #!x } }", E),
    ] {
        cases.push((text.to_string(), verdict));
    }
    let cases: Vec<(&str, Verdict)> = cases.iter().map(|(t, v)| (t.as_str(), *v)).collect();
    check_verdicts(&cases);
}

/// Where a field ends (`#sec-rules-of-automatic-semicolon-insertion`): a semicolon is inserted
/// only before a token that no production allows, so a token that continues the element (`(`
/// after a name, a name after `get`, `static`, `*` after `static`) is no field end; `async` has
/// [no LineTerminator here] (`#sec-async-function-definitions`).
#[test]
fn field_ends_answer_the_standard() {
    use Verdict::{E, H};
    check_verdicts(&[
        ("class C { x\n() {} }", H),
        ("class C { get\nx() {} }", H),
        ("class C { static\nx() {} }", H),
        ("class C { static\n*x() {} }", H),
        ("class C { static\n{} }", H),
        ("class C { static\nstatic }", H),
        ("class C { async\nx() {} }", H),
        ("class C { static async\nx }", H),
        ("class C { x\n[y] }", H),
        ("class C { x\n*y() {} }", H),
        ("class C { x = a\n[b] }", H),
        ("class C { x\n= 1 }", H),
        ("class C { x =\n1 }", H),
        ("class C { x = 1;; y }", H),
        ("class C { x = function () {}\n(1) }", H),
        ("class C { x = () => {}\ny }", H),
        ("class C { x = /re/\ny }", H),
        ("class C { x = a in b }", H),
        ("class C { x = a\n?.b }", H),
        ("class C { 1 = 1; 'a' = 2; 1.5; 0x10 = 3 }", H),
        ("class C { if = 1; class; new = 2 }", H),
        ("class C { \\u0069f = 1 }", H),
        ("class C { get\nx }", E),
        ("class C { x = () => {} y }", E),
        ("class C { x = 1, y = 2 }", E),
        ("class C { x = 1\n++y }", E),
        ("class C { x = 1 y = 2 }", E),
        ("class C { static x = 1 static y }", E),
        ("class C { x = }", E),
        ("class C { x = ; }", E),
    ]);
}

/// The parameters of `Function` and the bodies of a generator and an `async` function made by the
/// constructors (`#sec-createdynamicfunction`): AllPrivateIdentifiersValid of the parameters, and
/// the [Yield] and [Await] of the body around a class.
#[test]
fn dynamic_function_entries_read_class_elements() {
    let options = ParseOptions { max_nesting: LIMIT };
    let held = vec!["class field is not supported before slice 8".to_string()];
    let undeclared = vec!["private name #x is not declared in an enclosing class".to_string()];
    let mut wrong = Vec::new();
    let mut check =
        |what: &str, got: Result<Script, Vec<SyntaxError>>, expected: Option<&Vec<String>>| {
            let got = got.map(|_| ()).map_err(|errors| messages(&errors));
            let ok = match (&got, expected) {
                (Err(m), Some(e)) => m == e,
                (Err(m), None) => m
                    .iter()
                    .all(|m| !m.contains("is not supported before slice")),
                (Ok(()), _) => false,
            };
            if !ok {
                wrong.push(format!("{what}: expected {expected:?}, got {got:?}"));
            }
        };
    check(
        "Function(\"a = o.#x\", \"\")",
        parse_dynamic_function(&units("a = o.#x"), &units(""), options),
        Some(&undeclared),
    );
    check(
        "Function(\"a = #x in o\", \"\")",
        parse_dynamic_function(&units("a = #x in o"), &units(""), options),
        None,
    );
    check(
        "AsyncFunction(\"\", \"class C { x = await; }\")",
        parse_dynamic_async_function(&units(""), &units("class C { x = await; }"), options),
        None,
    );
    check(
        "AsyncFunction(\"\", \"class C { [await 1] = 1; }\")",
        parse_dynamic_async_function(&units(""), &units("class C { [await 1] = 1; }"), options),
        Some(&held),
    );
    check(
        "AsyncFunction(\"\", \"class C { x = () => await; }\")",
        parse_dynamic_async_function(&units(""), &units("class C { x = () => await; }"), options),
        Some(&held),
    );
    check(
        "GeneratorFunction(\"\", \"class C { x = yield; }\")",
        parse_dynamic_generator_function(&units(""), &units("class C { x = yield; }"), options),
        None,
    );
    check(
        "GeneratorFunction(\"\", \"class C { [yield] = 1; }\")",
        parse_dynamic_generator_function(&units(""), &units("class C { [yield] = 1; }"), options),
        Some(&held),
    );
    assert!(
        wrong.is_empty(),
        "{} mismatch(es):\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// A ledger of the scale test of class elements: every byte charged, and the most held at once.
#[derive(Default)]
struct PeakLedger {
    charged: usize,
    held: usize,
    peak: usize,
}

impl swell_syntax::Ledger for PeakLedger {
    fn charge(&mut self, bytes: usize) -> Result<(), swell_syntax::Stop> {
        self.charged += bytes;
        self.held += bytes;
        self.peak = self.peak.max(self.held);
        Ok(())
    }

    fn credit(&mut self, bytes: usize) {
        self.held = self.held.saturating_sub(bytes);
    }

    fn poll(&mut self) -> Result<(), swell_syntax::Stop> {
        Ok(())
    }
}

/// The answer, the processor-independent cost (bytes charged, most held) and the wall time of
/// one parse of `text`.
fn parse_cost(text: &str) -> (Vec<String>, usize, usize, std::time::Duration) {
    let source = units(text);
    let mut ledger = PeakLedger::default();
    let start = std::time::Instant::now();
    let answer =
        swell_syntax::parse_script_with(&source, ParseOptions { max_nesting: LIMIT }, &mut ledger);
    let time = start.elapsed();
    let errors = match answer {
        Ok(_) => Vec::new(),
        Err(swell_syntax::Failure::Errors(errors)) => messages(&errors),
        Err(other) => vec![format!("{other:?}")],
    };
    (errors, ledger.charged, ledger.peak, time)
}

/// A named shape of a class source, the construct its first element holds, and the source of it
/// with n elements.
type Shape = (&'static str, &'static str, fn(usize) -> String);

/// Hostile sizes of class elements: a class of n private fields each used once, one private
/// name used n times, n public fields, n static blocks, n undeclared private references. Each
/// answers what a small one answers, and the bytes it charges grow linearly: four times the
/// elements charge at most five times the bytes, hold at most five times the peak, and take at
/// most ten times the time.
#[test]
fn class_elements_scale_linearly() {
    let shapes: [Shape; 5] = [
        ("private fields used once", "private field", |n| {
            let fields: String = (0..n).map(|i| format!("#p{i};")).collect();
            let uses: String = (0..n).map(|i| format!("this.#p{i};")).collect();
            format!("class C {{ {fields} m() {{ {uses} }} }}")
        }),
        ("one private name used n times", "private field", |n| {
            format!("class C {{ #x; m() {{ {} }} }}", "this.#x;".repeat(n))
        }),
        ("public fields with initializers", "class field", |n| {
            let fields: String = (0..n).map(|i| format!("f{i} = {i};")).collect();
            format!("class C {{ {fields} }}")
        }),
        ("static blocks", "class static block", |n| {
            format!("class C {{ {} }}", "static { var v = 1; }".repeat(n))
        }),
        ("private names in nested classes", "private field", |n| {
            let inner: String = (0..n)
                .map(|i| format!("class D{i} {{ #y; m() {{ this.#x; this.#y; }} }}"))
                .collect();
            format!("class C {{ #x; m() {{ {inner} }} }}")
        }),
    ];
    let mut wrong = Vec::new();
    for (name, construct, make) in shapes {
        let held = vec![format!("{construct} is not supported before slice 8")];
        let n = 2_500;
        let (small, small_charged, small_peak, small_time) = parse_cost(&make(n));
        let (large, large_charged, large_peak, large_time) = parse_cost(&make(4 * n));
        println!(
            "{name}: n {n}: charged {small_charged}, peak {small_peak}, {small_time:?}; \
             4n: charged {large_charged}, peak {large_peak}, {large_time:?}"
        );
        if small != held || large != held {
            wrong.push(format!("{name}: answers {small:?} and {large:?}"));
        }
        if large_charged > 5 * small_charged || large_peak > 5 * small_peak {
            wrong.push(format!(
                "{name}: charged {small_charged} -> {large_charged}, peak {small_peak} -> {large_peak}"
            ));
        }
        if large_time > 10 * small_time + std::time::Duration::from_millis(50) {
            wrong.push(format!("{name}: {small_time:?} -> {large_time:?}"));
        }
    }
    let undeclared = "this.#z;".repeat(10_000);
    let (errors, _, _, time) = parse_cost(&format!("class C {{ m() {{ {undeclared} }} }}"));
    println!(
        "10,000 undeclared references: {} error(s), {time:?}",
        errors.len()
    );
    if errors.first().map(String::as_str)
        != Some("private name #z is not declared in an enclosing class")
    {
        wrong.push(format!("undeclared: {:?}", errors.first()));
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The two nesting kinds of this story in turn with the class kinds before them, 100,000 deep:
/// through each entry the answer is the nesting error alone, on the stack the deep limit of the
/// stack-bound check gives ([`deep_stack`]), never an overflow of the stack.
#[test]
fn class_element_kinds_nest_in_turn_within_the_bound() {
    let test = test_path!("class_element_kinds_nest_in_turn_within_the_bound");
    let limit = deep_limit();
    on_stack(test, limit, deep_stack(limit), |limit| {
        let kinds = [
            ("class { x = ", " }"),
            ("class { static { x = ", " } }"),
            ("class { [", "] () {} }"),
            ("class extends (", ") {}"),
            ("class { static #p = ", "; m() { return this.#p; } }"),
            ("class { #q; static { x = #q in ", "; } }"),
        ];
        let depth = 100_000;
        let mut text = String::from("x = ");
        for i in 0..depth {
            text.push_str(kinds[i % kinds.len()].0);
        }
        text.push('0');
        for i in (0..depth).rev() {
            text.push_str(kinds[i % kinds.len()].1);
        }
        text.push(';');
        let source = units(&text);
        for entry in ENTRIES {
            let errors = parse_through(entry, &source, limit)
                .err()
                .unwrap_or_default();
            assert_eq!(
                messages(&errors),
                vec![format!("nesting deeper than parser_nesting ({limit})")],
                "{entry:?}"
            );
        }
    })
    .unwrap();
}

/// What a field initializer and a static block set for their code ([Yield], [Await],
/// `new.target`, SuperProperty, no `return`, ContainsArguments) is put back after them: the code
/// after the class, after the element and in the next element has the context around the class
/// (`#sec-class-definitions`, `#sec-static-semantics-containsarguments`). Each mutant that drops
/// one of the puts-back in `element_context_close`, `static_block_close` or the walk's `Restore`
/// turns a line of this list.
#[test]
fn the_context_is_put_back_after_class_elements() {
    use Verdict::{E, H};
    check_verdicts(&[
        ("async function f() { class C { x = 1; } await 1; }", H),
        ("async function f() { class C { static {} } await 1; }", H),
        (
            "async function f() { class C { x = 1; [await 1] = 2; } }",
            H,
        ),
        ("function* g() { class C { x = 1; } yield 1; }", H),
        ("function* g() { class C { static {} } yield 1; }", H),
        ("function* g() { class C { x = 1; [yield 1] = 2; } }", H),
        ("function f() { class C { static {} } return 1; }", H),
        (
            "function f() { class C { static { function g() {} } } return 1; }",
            H,
        ),
        ("class C { x = 1; } arguments;", H),
        ("class C { static {} } arguments;", H),
        ("function f() { class C { x = 1; } arguments; }", H),
        (
            "function f() { class C { static x = 1; static [arguments] = 2; } }",
            H,
        ),
        ("function f() { class C { static {} [arguments]() {} } }", H),
        (
            "function f() { class C { x = () => 1; [arguments] = 2; } }",
            H,
        ),
        ("var await; class C { static {} } await;", H),
        ("var await; class C { static {} [await] = 1; }", H),
    ]);
    // A script only: a dynamic function's body admits `new.target`, and eval code refuses
    // `super` at its token before slice 8.
    let wrong = verdict_mismatches(
        &[Entry::Script],
        &[
            ("class C { x = 1; } new.target;", E),
            ("class C { static {} } new.target;", E),
            ("class C { x = 1; } super.x;", E),
            ("class C { static {} } super.x;", E),
            ("class C { x = 1; [new.target] = 2; }", E),
            ("class C { static {} [super.x] = 2; }", E),
        ],
    );
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// More sources at the edges of the element grammar: `#x in` at the start of a statement and in
/// each element kind, functions and classes nested in a static block (their own [Await]), the
/// strict string keys of a field, the names of static methods, and an escaped `static`.
#[test]
fn element_edges_answer_the_standard() {
    use Verdict::{E, H};
    check_verdicts(&[
        ("class C { #x; m(o) { #x in o; } }", H),
        ("class C { #x; y = #x in this; }", H),
        ("class C { #x; [#x in o] = 1; }", H),
        ("class C { #x; static { #x in o; } }", H),
        (
            "class C { #x; m(o) { return async (a = #x in this) => a; } }",
            H,
        ),
        (
            "class C { #x; m(o) { return #x in o ? #x in o : #x in o; } }",
            H,
        ),
        ("class C { #x; m(o) { #x: ; } }", E),
        ("class C { static { class D { x = () => await; } } }", H),
        ("class C { static { class D { x = await; } } }", E),
        ("class C { static { (class { [await] = 1; }); } }", E),
        ("class C { static { function f(x = await) { await; } } }", H),
        ("class C { static { ({ *m() { await; } }); } }", H),
        (
            "class C { static { (async function () { var await; }); } }",
            E,
        ),
        ("class C { static { async => 1; } }", H),
        ("class C { static { var C; } }", H),
        ("class C { static { let C; } }", H),
        ("class C { x = async () => await 1; }", H),
        ("async () => { class C { x = await; } };", E),
        ("async () => class C { [await 1] = 1; };", H),
        ("L: while (0) { class C { static { continue L; } } }", E),
        ("switch (0) { case 0: class C { static { break; } } }", E),
        ("class C { '\\01' = 1; }", E),
        ("class C { '\\8' = 1; }", E),
        ("class C { static '\\01'() {} }", E),
        ("class C { static prototype() {} }", E),
        ("class C { static get prototype() {} }", E),
        ("class C { static async *prototype() {} }", E),
        ("class C { get constructor() {} }", E),
        ("class C { async constructor() {} }", E),
        ("class C { *constructor() {} }", E),
        ("class C { constructor() {} constructor() {} }", E),
        ("class C { constructor() {} 'constructor' = 1; }", E),
        ("class C { 'constructor'() {} }", H),
        ("class C { st\\u0061tic {} }", E),
        ("class C { st\\u0061tic x() {} }", E),
        ("class C { st\\u0061tic = 1; }", H),
        ("class C { static st\\u0061tic = 1; }", H),
    ]);
}

/// A YieldExpression or an AwaitExpression in a field initializer is a SyntaxError of this engine
/// at its `yield` or `await`, a stated difference from the pinned draft, whose
/// `FieldDefinition[?Yield, ?Await]` reads `Initializer[+In, ?Yield, ?Await]` and has no early
/// error against either (`#sec-class-definitions`). A field initializer is the body of a function
/// of its own that is neither a generator nor an `async` function, so no runtime of this engine
/// and no step of ClassFieldDefinitionEvaluation can run one there.
///
/// The code of the initializer is all of it but the functions in it: a class nested in it, its
/// computed keys and its own initializers included. A computed key beside the initializer reads
/// `yield` and `await` with the [Yield] and [Await] around the class, as the draft does; in an
/// arrow function or a function in the initializer they are that function's, as in the draft.
#[test]
fn a_field_initializer_refuses_yield_and_await_expressions() {
    use Verdict::{E, H};
    // Each source is refused at the first `yield` or `await` it holds, which names the message.
    let refused = [
        ("async function f() { class C { x = await 1; } }", "await"),
        (
            "async function f() { class C { static x = await 1; } }",
            "await",
        ),
        ("function* g() { class C { x = yield 1; } }", "yield"),
        ("function* g() { class C { #x = yield; } }", "yield"),
        ("async function f() { class C { x = await; } }", "await"),
        (
            "async function f() { class C { #x = 1; static y = (await 0); } }",
            "await",
        ),
        (
            "async function* g() { class C { x = yield await 1; } }",
            "yield",
        ),
        (
            "async function* g() { class C { x = await (yield); } }",
            "await",
        ),
        (
            "async function f() { class C { x = class { y = await 1; }; } }",
            "await",
        ),
        (
            "async function f() { class C { x = class { [await 1] = 0; }; } }",
            "await",
        ),
        (
            "function* g() { class C { x = class { static [yield] () {} }; } }",
            "yield",
        ),
        (
            "async function f() { class C { x = class extends (await 0) {}; } }",
            "await",
        ),
        (
            "async function f() { class C { x = (a = await 0) => a; } }",
            "await",
        ),
        ("function* g() { class C { x = [...yield]; } }", "yield"),
    ];
    let mut wrong = Vec::new();
    for (text, word) in refused {
        let column = text.find(word).map_or(0, |i| i + 1);
        let message = format!("{word} expression in a class field initializer");
        for entry in ENTRIES {
            let line = if matches!(entry, Entry::FunctionBody) {
                2
            } else {
                1
            };
            let got = parse_through(entry, &units(text), LIMIT).map(|_| ());
            let ok = matches!(&got, Err(errors) if errors.len() == 1
                && errors[0].message == message
                && (errors[0].line, errors[0].column) == (line, column as u32));
            if !ok {
                let got = got.map_err(|errors| {
                    errors
                        .iter()
                        .map(|e| format!("{}:{} {}", e.line, e.column, e.message))
                        .collect::<Vec<_>>()
                });
                wrong.push(format!(
                    "{entry:?} {text:?}: expected {line}:{column} {message:?}, got {got:?}"
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} mismatch(es):\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    check_verdicts(&[
        ("async function f() { class C { [await 1] = 0; } }", H),
        (
            "async function f() { class C { static [await 1] = await; } }",
            E,
        ),
        ("function* g() { class C { [yield 1] = 0; x = 1; } }", H),
        (
            "async function f() { class C { x = async () => await 1; } }",
            H,
        ),
        (
            "async function f() { class C { x = async function () { await 1; }; } }",
            H,
        ),
        (
            "function* g() { class C { x = function* () { yield 1; }; } }",
            H,
        ),
        (
            "async function f() { class C { x = class { m() { await; } }; } }",
            H,
        ),
        (
            "async function f() { class C { x = class { async m() { await 1; } }; } }",
            H,
        ),
        ("class C { x = await; }", H),
        ("async function f() { class C { x = () => await 1; } }", E),
        (
            "async function f() { class C { x = function () { await 1; }; } }",
            E,
        ),
        ("function* g() { class C { x = () => yield 1; } }", E),
        (
            "function* g() { class C { x = function () { yield 1; }; } }",
            E,
        ),
    ]);
}

/// `await` as an identifier where [Await] is set is named by the code that sets it: a static
/// block, through a class nested in the block and that class's field initializers and through an
/// arrow function that is not `async` (`ClassStaticBlockStatementList : StatementList[~Yield,
/// +Await, ~Return]`, `#sec-class-definitions-static-semantics-early-errors`), or else an `async`
/// function or arrow function (`#sec-identifiers-static-semantics-early-errors`). In a field
/// initializer of a class inside an `async` function, the pinned grammar reads `await` as the
/// start of an AwaitExpression, which the parser refuses at the token before the early errors
/// run ([`a_field_initializer_refuses_yield_and_await_expressions`]).
#[test]
fn await_as_an_identifier_names_the_code_that_reserves_it() {
    let block = "await may not be an identifier in a class static block";
    let function = "await may not be an identifier in an async function";
    let field = "await expression in a class field initializer";
    let cases = [
        ("class C { static { await; } }", block),
        ("class C { static { class D { x = await; } } }", block),
        (
            "class C { static { class D { static x = await; } } }",
            block,
        ),
        (
            "class C { static { class D { x = class { y = await; }; } } }",
            block,
        ),
        ("class C { static { class D { [await] = 1; } } }", block),
        ("class C { static { (x = await) => 0; } }", block),
        (
            "class C { static { class D { x = (y = await) => y; } } }",
            block,
        ),
        ("class C { static { async (x = await) => x; } }", function),
        (
            "class C { static { async () => { var await; }; } }",
            function,
        ),
        (
            "class C { static { (async function () { var await; }); } }",
            function,
        ),
        (
            "async function f() { class C { x = class await {}; } }",
            function,
        ),
        ("async function f() { class C { x = await; } }", field),
    ];
    let mut wrong = Vec::new();
    for (text, message) in cases {
        for entry in ENTRIES {
            let got = parse_through(entry, &units(text), LIMIT).map(|_| ());
            if got.as_ref().map_err(|errors| messages(errors)) != Err(vec![message.to_string()]) {
                let got = got.map_err(|errors| messages(&errors));
                wrong.push(format!(
                    "{entry:?} {text:?}: expected {message:?}, got {got:?}"
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} mismatch(es):\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

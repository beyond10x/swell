//! The tests of `swell-compiler` over source text.
//!
//! Every file `tests/sources/<name>.js` starts with a comment line `// <anchor>: <what it shows>`
//! naming the ECMA-262 clause it compiles, and has its expected disassembly beside it in
//! `<name>.disasm`. A directory with no `.js` file, or a `.js` file without its `.disasm`, fails
//! the test (`AGENTS.md`, "A check that finds nothing to read fails"). A file whose name starts
//! with `eval-code-` is compiled as the eval code of a non-strict caller (`parse_eval`, then
//! `compile_eval`); every other file as a script.

use std::fs;
use std::path::{Path, PathBuf};

use swell_compiler::bytecode::{Code, FunctionTemplate, HandlerKind, Instruction, Slot};
use swell_compiler::disasm::disassemble;
use swell_compiler::{compile_eval, compile_script};
use swell_syntax::{ParseOptions, Script, parse_eval, parse_script};

/// The host's `parser_nesting` these tests parse with.
const LIMIT: u32 = 1_000;

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn parse(text: &str) -> Result<Script, String> {
    parse_script(&units(text), ParseOptions { max_nesting: LIMIT })
        .map_err(|errors| format!("{text:?} does not parse: {errors:?}"))
}

fn compile(text: &str) -> Result<Code, String> {
    compile_script(&parse(text)?).map_err(|error| format!("{text:?} does not compile: {error}"))
}

/// `text` as the eval code of a non-strict caller: `parse_eval`, then `compile_eval`.
fn compile_eval_code(text: &str) -> Result<Code, String> {
    let script = parse_eval(&units(text), false, ParseOptions { max_nesting: LIMIT })
        .map_err(|errors| format!("{text:?} does not parse as eval code: {errors:?}"))?;
    compile_eval(&script).map_err(|error| format!("{text:?} does not compile: {error}"))
}

/// A source file compiled as its name says: `eval-code-*.js` as eval code, every other file as
/// a script.
fn compile_source(source: &Source) -> Result<Code, String> {
    let eval_code = source
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("eval-code-"));
    if eval_code {
        compile_eval_code(&source.text)
    } else {
        compile(&source.text)
    }
}

/// A source file: its path, its text and the text of its `.disasm`.
struct Source {
    path: PathBuf,
    text: String,
    expected: String,
}

/// Every `.js` file under `tests/sources/`, sorted, with its `.disasm`; an error when there is
/// none, when one has no `.disasm`, or when its first line names no clause.
fn sources() -> Result<Vec<Source>, String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/sources");
    let mut paths = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if path.extension().is_some_and(|x| x == "js") {
            paths.push(path);
        }
    }
    paths.sort();
    if paths.is_empty() {
        return Err(format!("{}: no .js file to read", dir.display()));
    }
    let mut sources = Vec::new();
    for path in paths {
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let first = text.lines().next().unwrap_or("");
        let anchor = first
            .strip_prefix("// ")
            .and_then(|rest| rest.split_once(": "))
            .map(|(anchor, _)| anchor);
        if !anchor.is_some_and(|a| a.starts_with("sec-")) {
            return Err(format!(
                "{}: the first line is not `// <sec- anchor>: <what it shows>`",
                path.display()
            ));
        }
        let disasm = path.with_extension("disasm");
        let expected = fs::read_to_string(&disasm)
            .map_err(|e| format!("{}: no expected disassembly: {e}", disasm.display()))?;
        sources.push(Source {
            path,
            text,
            expected,
        });
    }
    Ok(sources)
}

/// The instruction lists of a script: its own and each function's, with a name for messages.
fn bodies(code: &Code) -> Vec<(String, &[Instruction])> {
    let mut list = vec![("script".to_string(), code.instructions.as_slice())];
    for (i, f) in code.functions.iter().enumerate() {
        list.push((format!("function {i}"), f.instructions.as_slice()));
    }
    list
}

/// The string constant `index`; `"<none>"` when there is none, which no expectation names.
fn string(code: &Code, index: u32) -> String {
    code.strings
        .get(index as usize)
        .map(|units| String::from_utf16_lossy(units))
        .unwrap_or_else(|| "<none>".to_string())
}

fn function<'a>(code: &'a Code, name: &str) -> Result<&'a FunctionTemplate, String> {
    code.functions
        .iter()
        .find(|f| String::from_utf16_lossy(&f.name) == name)
        .ok_or_else(|| format!("no function {name:?}"))
}

#[test]
fn disassembly_matches_for_each_source() {
    let sources = sources().unwrap();
    let mut failures = Vec::new();
    for source in &sources {
        let actual = disassemble(&compile_source(source).unwrap());
        if actual != source.expected {
            failures.push(format!(
                "{}: the disassembly differs\n--- expected\n{}--- actual\n{}",
                source.path.display(),
                source.expected,
                actual
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!("disassembly: {} source(s) match", sources.len());
}

#[test]
fn numeric_literal_values() {
    // `#sec-numericvalue`: a hexadecimal literal, a legacy octal one in non-strict code, a decimal
    // one past the largest finite Number and one with no integer part.
    let code = compile("0x10; 010; 1e400; .5;").unwrap();
    let values: Vec<f64> = code
        .instructions
        .iter()
        .filter_map(|i| match i {
            Instruction::Number(k) => Some(
                *code
                    .numbers
                    .get(*k as usize)
                    .unwrap_or_else(|| panic!("no number constant {k}")),
            ),
            _ => None,
        })
        .collect();
    assert_eq!(values, vec![16.0, 8.0, f64::INFINITY, 0.5]);
}

#[test]
fn every_backward_jump_is_a_loop_jump() {
    let sources = sources().unwrap();
    let mut loop_jumps = 0;
    let mut jumps = 0;
    for source in &sources {
        let code = compile_source(source).unwrap();
        for (name, instructions) in bodies(&code) {
            for (index, instruction) in instructions.iter().enumerate() {
                let Some(target) = instruction.target() else {
                    continue;
                };
                let index = index as u32;
                assert!(
                    (target as usize) < instructions.len(),
                    "{} {name} {index}: {instruction:?} targets past the end",
                    source.path.display()
                );
                if let Instruction::LoopJump(_) = instruction {
                    loop_jumps += 1;
                    assert!(
                        target <= index,
                        "{} {name} {index}: a LoopJump to a later instruction",
                        source.path.display()
                    );
                } else {
                    jumps += 1;
                    assert!(
                        target > index,
                        "{} {name} {index}: {instruction:?} jumps back without being a LoopJump",
                        source.path.display()
                    );
                }
            }
        }
    }
    assert!(loop_jumps > 0, "no source has a loop");
    assert!(jumps > 0, "no source has a forward jump");
    println!(
        "jumps: {jumps} forward, {loop_jumps} LoopJump over {} source(s)",
        sources.len()
    );
}

#[test]
fn uncaptured_bindings_are_slots_and_captured_ones_are_environment() {
    let code = compile(
        "var g;\n\
         function outer(a, b) {\n\
           var c, d;\n\
           try {} catch (e) { return function () { return b + d + e; }; }\n\
           try {} catch (u) { u; }\n\
           return a + c;\n\
         }\n",
    )
    .unwrap();
    // The script's own names are global: no slot, no environment of its own.
    assert_eq!(code.locals, 0);
    let names: Vec<String> = code.var_names.iter().map(|k| string(&code, *k)).collect();
    assert_eq!(names, vec!["g".to_string()]);

    let outer = function(&code, "outer").unwrap();
    // `a`, `c` and the catch parameter `u` are slots of the frame; `b` and `d`, which the inner
    // function uses, are the function environment's.
    assert_eq!(outer.params, vec![Slot::Local(0), Slot::Env(0)]);
    assert_eq!(outer.locals, 3);
    let captured: Vec<String> = outer.captured.iter().map(|k| string(&code, *k)).collect();
    assert_eq!(captured, vec!["b".to_string(), "d".to_string()]);
    // The catch parameter `e` is captured: its catch clause makes an environment of one binding.
    assert!(outer.instructions.contains(&Instruction::PushEnv(1)));
    assert!(outer.instructions.contains(&Instruction::PopEnv));
    assert_eq!(
        outer
            .instructions
            .iter()
            .filter(|i| matches!(i, Instruction::PushEnv(_)))
            .count(),
        1
    );
    // `u` is a slot: stored and read as local 2.
    assert!(outer.instructions.contains(&Instruction::SetLocal(2)));
    assert!(outer.instructions.contains(&Instruction::GetLocal(2)));
    assert!(outer.instructions.contains(&Instruction::GetLocal(0)));
    assert!(outer.instructions.contains(&Instruction::GetLocal(1)));

    let inner = function(&code, "").unwrap();
    assert_eq!(inner.locals, 0);
    assert!(inner.captured.is_empty());
    // From the inner function: the catch environment is one hop out, the function environment of
    // `outer` two.
    let reads: Vec<&Instruction> = inner
        .instructions
        .iter()
        .filter(|i| matches!(i, Instruction::GetEnv { .. }))
        .collect();
    assert_eq!(
        reads,
        vec![
            &Instruction::GetEnv { hops: 1, slot: 0 },
            &Instruction::GetEnv { hops: 1, slot: 1 },
            &Instruction::GetEnv { hops: 0, slot: 0 },
        ]
    );
    assert!(
        !inner
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::GetGlobal(_) | Instruction::GetLocal(_)))
    );
}

/// `docs/design/engine.md`, decision 3, "Lexical bindings", item 1: a `let` of a block that no
/// closure refers to is a local slot of the frame, which needs no record; one a closure refers
/// to is a slot of the record the block makes at its entry and leaves at its end
/// (`#sec-blockdeclarationinstantiation`).
#[test]
fn block_bindings_are_local_slots_unless_a_closure_captures_them() {
    let code = compile(
        "function f() {\n\
           { let a = 1; a; }\n\
           { let b = 2; return function () { return b; }; }\n\
         }\n",
    )
    .unwrap();
    let f = function(&code, "f").unwrap();
    // `a` is local 0, written by its declaration and read after it with no check; `b` is not
    // a local slot, and no binding is the function environment's.
    assert_eq!(f.locals, 1);
    assert!(f.captured.is_empty());
    assert!(f.instructions.contains(&Instruction::SetLocal(0)));
    assert!(f.instructions.contains(&Instruction::GetLocal(0)));
    // `b` is slot 0 of the one record `f` makes, that of the second block, initialized there.
    assert!(
        f.instructions
            .contains(&Instruction::SetEnv { hops: 0, slot: 0 })
    );
    let text = disassemble(&code);
    assert_eq!(text.matches("PushLexicalEnv").count(), 1, "{text}");
    assert!(text.contains("PushLexicalEnv 0 \"b\""), "{text}");
    // The closure reads it from its environment, the block's record, with the check of a binding
    // that may be uninitialized (another function's code).
    let inner = function(&code, "").unwrap();
    assert_eq!(inner.locals, 0);
    let inner_text = text
        .split("function 1 ")
        .nth(1)
        .unwrap_or_else(|| panic!("no inner function in {text}"));
    assert!(inner_text.contains("GetEnvChecked 0 0"), "{text}");
}

#[test]
fn finally_runs_for_return_break_and_continue() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/sources/finally-exits.js");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let code = compile(&text).unwrap();
    let f = function(&code, "f").unwrap();
    let at = |i: u32| {
        f.instructions
            .get(i as usize)
            .unwrap_or_else(|| panic!("no instruction {i}"))
    };

    // One handler: a finally over the try block.
    assert_eq!(f.handlers.len(), 1, "{:?}", f.handlers);
    let handler = &f.handlers[0];
    assert_eq!(handler.kind, HandlerKind::Finally);
    assert_eq!(handler.items, 0);
    let entry = handler.target;

    // Each way out of the try block pushes its completion record, a value and a tag, and jumps
    // to the finally block: 2 return, 3 break, 4 continue.
    for tag in [2, 3, 4] {
        let site = f
            .instructions
            .iter()
            .position(|i| *i == Instruction::Tag(tag))
            .unwrap_or_else(|| panic!("no exit with tag {tag}")) as u32;
        assert!(handler.start <= site && site < handler.end, "tag {tag}");
        assert_eq!(*at(site + 1), Instruction::Jump(entry), "tag {tag}");
    }
    // The return value is the Number 1.
    let ret = f
        .instructions
        .iter()
        .position(|i| *i == Instruction::Tag(2))
        .unwrap_or(0);
    assert!(matches!(at(ret as u32 - 1), Instruction::Number(_)));

    // The finally block reads `d` and then dispatches on the tag.
    assert!(matches!(at(entry), Instruction::GetGlobal(k) if string(&code, *k) == "d"));
    let dispatch = |tag: u32| {
        f.instructions
            .iter()
            .find_map(|i| match i {
                Instruction::JumpIfTag { tag: t, target } if *t == tag => Some(*target),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no dispatch for tag {tag}"))
    };
    // Return: the value is returned.
    assert_eq!(*at(dispatch(2)), Instruction::Return);
    // Break: past the loop, whose test jumps to the same place when false.
    assert_eq!(*at(dispatch(3)), Instruction::Pop);
    let Instruction::Jump(after) = *at(dispatch(3) + 1) else {
        panic!("break does not jump: {:?}", at(dispatch(3) + 1));
    };
    assert!(f.instructions.contains(&Instruction::JumpIfFalse(after)));
    // Continue: back to the loop's test, a LoopJump.
    assert_eq!(*at(dispatch(3) + 2), Instruction::Pop);
    assert_eq!(dispatch(4), dispatch(3) + 2);
    assert_eq!(*at(dispatch(4) + 1), Instruction::LoopJump(0));
    // Normal and throw completions end the finally block through EndFinally.
    assert!(f.instructions.contains(&Instruction::EndFinally));

    // A return from a for-in inside the try block takes the value over the enumerator (`Nip`).
    let g = function(&code, "g").unwrap();
    let nip = g
        .instructions
        .iter()
        .position(|i| *i == Instruction::Nip(1))
        .unwrap_or_else(|| panic!("no Nip in g: {:?}", g.instructions));
    assert_eq!(g.instructions.get(nip + 1), Some(&Instruction::Tag(2)));
}

#[test]
fn compile_is_deterministic() {
    let sources = sources().unwrap();
    for source in &sources {
        let first = compile_source(source).unwrap();
        let second = compile_source(source).unwrap();
        assert_eq!(first, second, "{}", source.path.display());
        assert_eq!(disassemble(&first), disassemble(&second));
    }
    println!("deterministic: {} source(s)", sources.len());
}

/// The deepest `n` for which `shape(n)` parses at [`LIMIT`], by bisection over `1..=4 * LIMIT`.
fn deepest(shape: &dyn Fn(usize) -> String) -> Result<usize, String> {
    let ok = |n: usize| parse(&shape(n)).is_ok();
    if !ok(1) {
        return Err(format!("{:?} does not parse", shape(1)));
    }
    let (mut good, mut bad) = (1, 4 * LIMIT as usize);
    if ok(bad) {
        return Err(format!("{bad} levels parse"));
    }
    while bad - good > 1 {
        let mid = good + (bad - good) / 2;
        if ok(mid) {
            good = mid;
        } else {
            bad = mid;
        }
    }
    Ok(good)
}

/// The depth of the deepest statement of a script, as the parser recorded it.
fn depth(script: &Script) -> u32 {
    script
        .body
        .iter()
        .filter_map(|s| script.ast.stmt(*s).map(|n| n.depth))
        .max()
        .unwrap_or(0)
}

#[test]
fn deepest_accepted_tree_compiles_on_a_256_kib_stack() {
    type Shape = fn(usize) -> String;
    let shapes: Vec<(&str, Shape)> = vec![
        ("binary chain", |n| format!("x = a{};", "+a".repeat(n))),
        ("member chain", |n| format!("x = a{};", ".b".repeat(n))),
        ("call chain", |n| format!("f{};", "()".repeat(n))),
        ("unary chain", |n| format!("x = {}a;", "!".repeat(n))),
        ("parentheses", |n| {
            format!("{}0{};", "(".repeat(n), ")".repeat(n))
        }),
        ("arrays", |n| format!("{}{};", "[".repeat(n), "]".repeat(n))),
        ("conditionals", |n| {
            format!("x = {}0;", "a ? b : ".repeat(n))
        }),
        ("assignments", |n| format!("{}0;", "a = ".repeat(n))),
        ("blocks", |n| format!("{}{}", "{".repeat(n), "}".repeat(n))),
        ("loops", |n| format!("{}break;", "while (a) ".repeat(n))),
        ("try finally", |n| {
            format!("{}x;{}", "try { ".repeat(n), " } finally { y; }".repeat(n))
        }),
        ("functions", |n| {
            format!(
                "{}return a;{}",
                "function f(a) { ".repeat(n),
                " }".repeat(n)
            )
        }),
        ("function expressions", |n| {
            format!(
                "x = {}a{};",
                "function (a) { return ".repeat(n),
                "; }".repeat(n)
            )
        }),
        ("labels", |n| {
            format!(
                "{}while (a) {{ break l0; continue l{}; }}",
                (0..n).map(|i| format!("l{i}: ")).collect::<String>(),
                n.saturating_sub(1)
            )
        }),
        ("optional chain", |n| format!("x = a{}();", "?.b".repeat(n))),
        ("coalesce chain", |n| format!("x = a{};", " ?? a".repeat(n))),
        ("withs", |n| {
            format!("{}x = arguments;", "with (o) ".repeat(n))
        }),
        ("functions in withs", |n| {
            format!(
                "{}x = arguments[0] + a;{}",
                "function f(a) { with (a) { ".repeat(n),
                " } }".repeat(n)
            )
        }),
        ("logical assignments", |n| {
            format!("{}0;", "a ??= ".repeat(n))
        }),
        ("methods", |n| {
            format!(
                "x = {}a{};",
                "{ m(a) { return ".repeat(n),
                "; } }".repeat(n)
            )
        }),
    ];
    let mut deepest_depth = 0;
    let mut cases = Vec::new();
    for (name, shape) in &shapes {
        let n = deepest(shape).unwrap();
        let text = shape(n);
        let d = depth(&parse(&text).unwrap());
        deepest_depth = deepest_depth.max(d);
        println!("{name}: {n} level(s), tree depth {d}");
        cases.push((name.to_string(), text));
    }
    // At least one tree is as deep as the limit admits.
    assert_eq!(deepest_depth, LIMIT);

    let worker = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            for (name, text) in cases {
                let script = parse_script(&units(&text), ParseOptions { max_nesting: LIMIT })
                    .unwrap_or_else(|e| panic!("{name}: {e:?}"));
                let code = compile_script(&script).unwrap_or_else(|e| panic!("{name}: {e:?}"));
                assert!(!code.instructions.is_empty(), "{name}");
                let text = disassemble(&code);
                assert!(text.starts_with("script"), "{name}");
                drop(code);
                drop(script);
            }
        })
        .unwrap_or_else(|e| panic!("spawn: {e}"));
    worker
        .join()
        .unwrap_or_else(|_| panic!("the compile on a 256 KiB stack failed"));
}

/// The instructions of `f` that read or write a binding, as text, in order.
fn binding_instructions(code: &Code, f: &FunctionTemplate) -> Vec<String> {
    f.instructions
        .iter()
        .filter_map(|i| match i {
            Instruction::GetLocal(n) => Some(format!("local {n}")),
            Instruction::GetEnv { hops, slot } => Some(format!("env {hops} {slot}")),
            Instruction::GetName(k) => Some(format!("name {}", string(code, *k))),
            Instruction::GetGlobal(k) => Some(format!("global {}", string(code, *k))),
            _ => None,
        })
        .collect()
}

/// `docs/design/engine.md`, decision 3, "Names": a reference with a `with` body between it and
/// its binding is looked up by name, a function nested in the body included; the same binding
/// read outside the body keeps its slot, now in the named function environment; a catch
/// parameter declared inside the body and a parameter of a function nested in it keep their
/// slots; code that no `with` reaches is compiled as before.
#[test]
fn with_bodies_look_up_by_name_and_everything_else_keeps_its_slot() {
    let code = compile(
        "function outer(o) { var a = 1; \
           with (o) { a; try { throw 1; } catch (c) { c; } (function inner(p) { return p + a; }); } \
           return a; } \
         function plain(q) { var b = q; return b; }",
    )
    .unwrap();
    let outer = function(&code, "outer").unwrap();
    assert_eq!(
        binding_instructions(&code, outer),
        ["env 0 0", "name a", "local 0", "env 0 1"]
    );
    let names: Vec<String> = outer.captured.iter().map(|k| string(&code, *k)).collect();
    assert_eq!(names, ["o", "a"]);
    assert_eq!(outer.params, [Slot::Env(0)]);
    let inner = function(&code, "inner").unwrap();
    assert_eq!(binding_instructions(&code, inner), ["local 0", "name a"]);
    let plain = function(&code, "plain").unwrap();
    assert_eq!(binding_instructions(&code, plain), ["local 0", "local 1"]);
    assert!(plain.captured.is_empty() && plain.arguments.is_none());
}

/// `#sec-functiondeclarationinstantiation`, steps 15 to 22: an arguments object only for a
/// function whose own code references `arguments`; mapped in non-strict code, with every
/// parameter in the function environment; unmapped in strict code, the parameters local.
#[test]
fn arguments_objects_are_made_where_the_code_references_them() {
    let code = compile(
        "function none(a) { return a; } \
         function sloppy(a, a, b) { return arguments; } \
         function strict(a) { \"use strict\"; return arguments; } \
         function param(arguments) { return arguments; }",
    )
    .unwrap();
    assert_eq!(function(&code, "none").unwrap().arguments, None);
    assert_eq!(function(&code, "param").unwrap().arguments, None);
    let sloppy = function(&code, "sloppy").unwrap();
    let arguments = sloppy.arguments.unwrap();
    assert!(arguments.mapped);
    assert_eq!(sloppy.params, [Slot::Env(0), Slot::Env(0), Slot::Env(1)]);
    let strict = function(&code, "strict").unwrap();
    assert!(!strict.arguments.unwrap().mapped);
    assert_eq!(strict.params, [Slot::Local(0)]);
}

/// `#sec-regular-expression-literals-runtime-semantics-evaluation` (`docs/design/engine.md`,
/// decision 7, "A regular expression literal: checked at parse time, compiled at each
/// evaluation", rule 2): a literal compiles to one instruction that makes a RegExp from two
/// String constants, its body and its flags as written, and to nothing that throws.
#[test]
fn a_regexp_literal_compiles_to_one_creation_of_its_body_and_flags() {
    let code = compile("x = /a(b)c/gi; y = /\\//;").unwrap();
    let made: Vec<(String, String)> = code
        .instructions
        .iter()
        .filter_map(|i| match i {
            Instruction::NewRegExp { body, flags } => {
                Some((string(&code, *body), string(&code, *flags)))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        made,
        vec![
            ("a(b)c".to_string(), "gi".to_string()),
            ("\\/".to_string(), String::new())
        ]
    );
    assert!(
        !code
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::ThrowSyntaxError(_))),
        "{:?}",
        code.instructions
    );
}

/// How many direct-eval calls `instructions` hold.
fn direct_eval_calls(instructions: &[Instruction]) -> usize {
    instructions
        .iter()
        .filter(|i| matches!(i, Instruction::CallEval(_)))
        .count()
}

/// `docs/design/engine.md`, decision 3, "Names", items 1 to 4 for a direct `eval`: a reference
/// with a non-strict function scope that contains a direct `eval` between it and its binding
/// (the global scope when none binds it) is looked up by name, a function nested in that scope
/// included; a strict function keeps its slots, also when it calls `eval`; every binding of a
/// scope that encloses a direct `eval` call lives in its named environment; a function with a
/// direct `eval` makes its arguments object; the call itself is a `CallEval`. Code that no
/// direct `eval` reaches is compiled as before. Eval code looks up by name every name it does
/// not bind in a function of its own.
#[test]
fn direct_eval_looks_up_by_name_what_eval_code_may_shadow() {
    let code = compile(
        "var g = 1; \
         function f(a) { var b = a; eval(\"b\"); function inner() { return a + b + g; } return b + g; } \
         function s(a) { \"use strict\"; var b = a; eval(\"b\"); return b + g; } \
         function outer(c) { eval(\"\"); function strict() { \"use strict\"; return c + g; } return strict; } \
         function plain(d) { return d + g; }",
    )
    .unwrap();
    let names = |f: &FunctionTemplate| -> Vec<String> {
        f.captured.iter().map(|k| string(&code, *k)).collect()
    };
    let f = function(&code, "f").unwrap();
    assert_eq!(
        binding_instructions(&code, f),
        ["env 0 0", "env 0 2", "name g"]
    );
    assert_eq!(names(f), ["a", "arguments", "b", "inner"]);
    assert_eq!(f.params, [Slot::Env(0)]);
    assert!(
        f.arguments
            .is_some_and(|a| a.mapped && a.slot == Slot::Env(1))
    );
    assert_eq!(direct_eval_calls(&f.instructions), 1);
    let inner = function(&code, "inner").unwrap();
    assert_eq!(
        binding_instructions(&code, inner),
        ["env 0 0", "env 0 2", "name g"]
    );
    assert_eq!(direct_eval_calls(&inner.instructions), 0);
    let s = function(&code, "s").unwrap();
    assert_eq!(
        binding_instructions(&code, s),
        ["env 0 0", "global eval", "env 0 2", "global g"]
    );
    assert_eq!(names(s), ["a", "arguments", "b"]);
    assert!(
        s.arguments
            .is_some_and(|a| !a.mapped && a.slot == Slot::Env(1))
    );
    assert_eq!(direct_eval_calls(&s.instructions), 1);
    let strict = function(&code, "strict").unwrap();
    assert_eq!(binding_instructions(&code, strict), ["env 0 0", "name g"]);
    assert!(strict.arguments.is_none());
    let outer = function(&code, "outer").unwrap();
    assert_eq!(names(outer), ["c", "arguments", "strict"]);
    let plain = function(&code, "plain").unwrap();
    assert_eq!(binding_instructions(&code, plain), ["local 0", "global g"]);
    assert!(plain.captured.is_empty() && plain.arguments.is_none());
    assert_eq!(direct_eval_calls(&code.instructions), 0);

    let code = compile_eval_code("var x = 1; function h(p) { return p + x; } x;").unwrap();
    let script: Vec<String> = code
        .instructions
        .iter()
        .filter_map(|i| match i {
            Instruction::GetName(k) => Some(format!("name {}", string(&code, *k))),
            Instruction::GetGlobal(k) => Some(format!("global {}", string(&code, *k))),
            Instruction::GetLocal(n) => Some(format!("local {n}")),
            _ => None,
        })
        .collect();
    assert_eq!(script, ["name x"]);
    let h = function(&code, "h").unwrap();
    assert_eq!(binding_instructions(&code, h), ["local 0", "name x"]);
    assert_eq!(code.var_names.len(), 1);
    assert_eq!(code.function_declarations, [0]);
}

/// Each template carries the span of its function's source text, the [[SourceText]] that
/// OrdinaryFunctionCreate gives the function object
/// (`#sec-runtime-semantics-instantiateordinaryfunctionobject`,
/// `#sec-runtime-semantics-definemethod`): the span the parser gave the function, from its first
/// token to its closing brace, in code units of the source; in eval code the same.
#[test]
fn templates_carry_the_span_of_their_source_text() {
    let text = "// lead\nfunction f(a) { return a; } /* x */\n\
                var o = { get p() { return 1; }, m() {} };\n\
                var g = function () { /* in */ };";
    let script = parse(text).unwrap();
    let code = compile_script(&script).unwrap();
    let source = units(text);
    let spans: Vec<_> = script.ast.functions().iter().map(|f| f.span).collect();
    let carried: Vec<_> = code.functions.iter().map(|f| f.source_text).collect();
    assert_eq!(carried, spans);
    let texts: Vec<String> = carried
        .iter()
        .map(|span| String::from_utf16(&source[span.start as usize..span.end as usize]).unwrap())
        .collect();
    assert_eq!(
        texts,
        [
            "function f(a) { return a; }",
            "get p() { return 1; }",
            "m() {}",
            "function () { /* in */ }",
        ]
    );

    let text = " (function h() {});";
    let code = compile_eval_code(text).unwrap();
    let span = code.functions[0].source_text;
    assert_eq!((span.start, span.end), (2, 17));
}

/// A generator template ("Suspended frames", items 2 and 6, `docs/design/engine.md`) has one
/// local slot more than the same function written without `*`, the last, which no binding
/// uses: its generator object. Its code binds the parameters and instantiates the function
/// declarations first and only then starts the generator in that slot
/// (`#sec-runtime-semantics-evaluategeneratorbody`), and every way out of it is the return of a
/// generator, never a plain `Return`.
#[test]
fn a_generator_reserves_its_last_slot_and_starts_after_its_parameters() {
    // The same bindings, `yield x` in the generator where the function has `x`.
    let head = "(a = 1, [b] = [a]) { var x = b; function inner() {}";
    let code = compile(&format!(
        "function* g{head} yield x; return inner; }}\nfunction f{head} x; return inner; }}"
    ))
    .unwrap();
    let g = function(&code, "g").unwrap();
    let f = function(&code, "f").unwrap();
    assert!(g.generator && !f.generator);
    assert_eq!(g.locals, f.locals + 1);
    let slot = g.locals - 1;
    assert!(
        g.params
            .iter()
            .all(|p| !matches!(p, Slot::Local(s) if *s == slot)),
        "a parameter in the reserved slot: {:?}",
        g.params
    );
    let start = g
        .instructions
        .iter()
        .position(|i| *i == Instruction::GeneratorStart(slot))
        .unwrap_or_else(|| panic!("no GeneratorStart {slot}: {:?}", g.instructions));
    let last_before = g
        .instructions
        .iter()
        .rposition(|i| {
            matches!(
                i,
                Instruction::Argument(_)
                    | Instruction::JumpIfNotUndefined(_)
                    | Instruction::PatternStep(_)
                    | Instruction::Closure(_)
            )
        })
        .unwrap_or_else(|| panic!("no binding of the parameters: {:?}", g.instructions));
    let first_yield = g
        .instructions
        .iter()
        .position(|i| *i == Instruction::Yield(slot))
        .unwrap_or_else(|| panic!("no Yield {slot}: {:?}", g.instructions));
    assert!(
        last_before < start && start < first_yield,
        "{:?}",
        g.instructions
    );
    assert_eq!(
        g.instructions.get(start + 1),
        Some(&Instruction::EndFinally)
    );
    for i in &g.instructions {
        assert!(
            !matches!(i, Instruction::Return),
            "a plain Return in a generator: {:?}",
            g.instructions
        );
        if let Instruction::GetLocal(s) | Instruction::SetLocal(s) = i {
            assert_ne!(*s, slot, "a binding in the reserved slot");
        }
    }
    assert!(g.instructions.contains(&Instruction::GeneratorReturn(slot)));
    assert!(g.instructions.contains(&Instruction::GeneratorDone(slot)));
    assert!(
        g.handlers
            .iter()
            .any(|h| h.kind == HandlerKind::Finally && h.start as usize == start + 2),
        "no finally from the start of the body: {:?}",
        g.handlers
    );
}

/// An `async` template ("`async` functions on suspended frames", item 1, `docs/design/engine.md`)
/// has one local slot more than the same function written without `async`, the last, which no
/// binding uses: its activation. Its code starts the activation in that slot before it binds
/// the parameters (`#sec-runtime-semantics-evaluateasyncfunctionbody`, step 1, before
/// FunctionDeclarationInstantiation), and from right after the start to its end it is a catch
/// region whose block rejects the promise of the call (step 3 and `#sec-asyncblockstart`); every
/// way out of it is the return of an `async` template, never a plain `Return`.
#[test]
fn an_async_function_reserves_its_last_slot_and_starts_before_its_parameters() {
    // The same bindings, `await x` in the async function where the function has `x`.
    let head = "(a = 1, [b] = [a]) { var x = b; function inner() {}";
    let code = compile(&format!(
        "async function g{head} await x; return inner; }}\nfunction f{head} x; return inner; }}"
    ))
    .unwrap();
    let g = function(&code, "g").unwrap();
    let f = function(&code, "f").unwrap();
    assert!(g.asynchronous && !f.asynchronous && !g.generator);
    assert_eq!(g.locals, f.locals + 1);
    let slot = g.locals - 1;
    assert!(
        g.params
            .iter()
            .all(|p| !matches!(p, Slot::Local(s) if *s == slot)),
        "a parameter in the reserved slot: {:?}",
        g.params
    );
    assert_eq!(
        g.instructions.first(),
        Some(&Instruction::AsyncStart(slot)),
        "the start is not the first instruction: {:?}",
        g.instructions
    );
    let first_binding = g
        .instructions
        .iter()
        .position(|i| {
            matches!(
                i,
                Instruction::Argument(_)
                    | Instruction::JumpIfNotUndefined(_)
                    | Instruction::PatternStep(_)
                    | Instruction::Closure(_)
            )
        })
        .unwrap_or_else(|| panic!("no binding of the parameters: {:?}", g.instructions));
    let first_await = g
        .instructions
        .iter()
        .position(|i| *i == Instruction::Await(slot))
        .unwrap_or_else(|| panic!("no Await {slot}: {:?}", g.instructions));
    assert!(0 < first_binding && first_binding < first_await);
    for i in &g.instructions {
        assert!(
            !matches!(i, Instruction::Return),
            "a plain Return in an async function: {:?}",
            g.instructions
        );
        if let Instruction::GetLocal(s) | Instruction::SetLocal(s) = i {
            assert_ne!(*s, slot, "a binding in the reserved slot");
        }
    }
    assert!(g.instructions.contains(&Instruction::AsyncReturn(slot)));
    let reject = g
        .instructions
        .iter()
        .position(|i| *i == Instruction::AsyncReject(slot))
        .unwrap_or_else(|| panic!("no AsyncReject {slot}: {:?}", g.instructions));
    assert_eq!(
        reject + 1,
        g.instructions.len(),
        "the rejection is not last"
    );
    assert!(
        g.handlers.iter().any(|h| h.kind == HandlerKind::Catch
            && h.start == 1
            && h.end as usize == reject
            && h.target as usize == reject),
        "no catch from right after the start to the rejection: {:?}",
        g.handlers
    );
}

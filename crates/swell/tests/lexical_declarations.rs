//! `let` and `const` (`#sec-let-and-const-declarations`) through the host interface: the
//! temporal dead zone, the `TypeError` of every assignment to a `const` binding, block scopes and
//! the closures that outlive them, the per-iteration bindings of `for (let ...)`
//! (`#sec-createperiterationenvironment`), the lexical bindings of scripts in the global
//! environment's [[DeclarativeRecord]] (`#sec-globaldeclarationinstantiation`, HOST-156) and
//! those of eval code in the record of PerformEval (`#sec-performeval`,
//! `#sec-evaldeclarationinstantiation`), `delete` of a lexical binding, and the memory a
//! per-iteration record takes (`docs/design/engine.md`, decision 3, "Lexical bindings"). What
//! needs one script is also in `tests/language/`; what needs two scripts or a budget is here.
//! Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{agent, agent_with, limits, realm};
use support::script::{Outcome, describe, eval_text, parse, run, run_both};
use swell::Termination;

fn normal(text: &str) -> Outcome {
    Outcome::Normal(text.to_string())
}

fn thrown(name: &str) -> Outcome {
    Outcome::Throw(name.to_string())
}

/// `source` in sloppy and in strict mode answers `expected` in both.
fn both(source: &str, expected: &Outcome) {
    let (sloppy, strict) = run_both(source);
    assert_eq!(&sloppy, expected, "sloppy: {source}");
    assert_eq!(&strict, expected, "strict: {source}");
}

/// HOST-156 (`#sec-globaldeclarationinstantiation`, step 3): one parsed script `let b = 1;`
/// evaluated twice answers `Throw` of a `SyntaxError` the second time, before it binds anything:
/// its lexical name is a lexical binding of the realm already, which keeps its value.
#[test]
fn host_156_evaluating_a_let_script_twice_throws_a_syntax_error() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let script = parse(&mut agent, &realm, "let b = 1;");
    let first = agent.evaluate_script(&script).unwrap();
    assert_eq!(describe(&mut agent, &realm, first), normal("undefined"));
    let second = agent.evaluate_script(&script).unwrap();
    assert_eq!(describe(&mut agent, &realm, second), thrown("SyntaxError"));
    assert_eq!(eval_text(&mut agent, &realm, "b"), normal("1"));
}

/// `#sec-declarative-environment-records-getbindingvalue-n-s` step 2,
/// `#sec-declarative-environment-records-setmutablebinding-n-v-s` step 3 and
/// `#sec-typeof-operator-runtime-semantics-evaluation`: a read, a write (plain, compound and
/// `++`) and `typeof` of a binding before its declaration ran throw a `ReferenceError`, for a
/// binding of the script, of a block, of a function body, of a closure and of a block entered
/// again; the write throws after its right side was evaluated.
#[test]
fn a_binding_read_written_or_typeof_before_its_initialization_throws_reference_error() {
    for source in [
        "x; let x;",
        "x = 1; let x;",
        "typeof x; let x;",
        "x += 1; let x = 0;",
        "function f() { return x; } f(); let x;",
        "{ x; let x; }",
        "{ x = 1; let x; }",
        "{ typeof x; let x; }",
        "{ x++; let x = 0; }",
        "{ let x = x; }",
        "function f() { x; let x; } f();",
        "function f() { var g = function () { return x; }; g(); let x; } f();",
        "for (var i = 0; i < 2; i++) { if (i === 1) { x; } let x = i; }",
        "for (let k in k) {}",
        "switch (1) { case x: let x; }",
    ] {
        both(source, &thrown("ReferenceError"));
    }
    both(
        "var right = 0; try { x = (right = 1); } catch (e) {} let x; right",
        &normal("1"),
    );
    both(
        "var m; try { x; } catch (e) { m = e.message; } let x; m === \"x is not initialized\"",
        &normal("true"),
    );
}

/// The design's finding on FunctionDeclarationInstantiation (`#sec-functiondeclarationinstantiation`):
/// the top level of a function body is a scope of its own, entered before the function
/// declarations of the body are instantiated, so a hoisted function sees the body's `let`; called
/// before the declaration runs, it finds the binding uninitialized.
#[test]
fn a_hoisted_function_sees_the_lexical_bindings_of_its_function_body() {
    both(
        "function f() { let x = 1; function g() { return x; } return g(); } f();",
        &normal("1"),
    );
    both(
        "function f() { var r = g(); let x = 1; function g() { return x; } return r; } f();",
        &thrown("ReferenceError"),
    );
}

/// `#sec-template-literals-runtime-semantics-evaluation`: a substitution of a template literal
/// is an expression like any other, so it reads the block-scoped binding of its own scope, in a
/// local slot or captured, and before the declaration ran it throws the `ReferenceError` of the
/// temporal dead zone.
#[test]
fn a_template_substitution_sees_block_bindings_and_their_dead_zone() {
    both(
        "let a = 1; { let a = 2; const b = `${a}-${(function () { return a; })()}`; b; }",
        &normal("\"2-2\""),
    );
    both("{ `${x}`; let x = 1; }", &thrown("ReferenceError"));
    both(
        "function f() { return `${y}`; } var r = []; try { f(); } catch (e) { r.push(e.name); } \
         let y = 3; r.push(f()); r.join()",
        &normal("\"ReferenceError,3\""),
    );
}

/// `#sec-declarative-environment-records-setmutablebinding-n-v-s`, steps 2 and 5: every
/// assignment to a `const` binding throws a `TypeError`, in strict and in non-strict code,
/// through its slot and through a lookup by name; before its declaration the `ReferenceError` of
/// step 3 comes first. The binding keeps its value.
#[test]
fn assignment_to_const_throws_type_error_in_both_modes() {
    for source in [
        "const c = 1; c = 2;",
        "const c = 1; c += 1;",
        "const c = 1; c++;",
        "const c = 1; (function () { c = 2; })();",
        "{ const c = 1; c = 2; }",
        "function f() { const c = 1; c = 2; } f();",
        "function f() { const c = 1; return function () { c = 2; }; } f()();",
        "function f() { const c = 1; eval(\"c = 2\"); } f();",
        "for (const i = 0; i < 1; i++) {}",
        "for (const k in { a: 1 }) { k = 2; }",
    ] {
        both(source, &thrown("TypeError"));
    }
    both("c = 0; const c = 1;", &thrown("ReferenceError"));
    both("{ c = 0; const c = 1; }", &thrown("ReferenceError"));
    both(
        "const c = 1; var m; try { c = 2; } catch (e) { m = e.message; } \
         m === \"Assignment to the constant binding c\" && c === 1",
        &normal("true"),
    );
}

/// `#sec-block-runtime-semantics-evaluation`: a closure keeps the record of the block it was
/// made in after the block is left, and each entry of a block makes a new one.
#[test]
fn a_block_binding_a_closure_captured_outlives_the_block() {
    both(
        "var get; { let a = 1; get = function () { return a; }; a = 2; } get();",
        &normal("2"),
    );
    both(
        "var fs = []; for (var i = 0; i < 3; i++) { let j = i; fs[i] = function () { return j; }; } \
         fs[0]() + \",\" + fs[1]() + \",\" + fs[2]()",
        &normal("\"0,1,2\""),
    );
    both(
        "var get; try { let a = 3; get = function () { return a; }; throw 0; } catch (e) {} get();",
        &normal("3"),
    );
}

/// `#sec-forbodyevaluation` and `#sec-createperiterationenvironment`: each iteration of
/// `for (let ...)` has a copy of the head's bindings, made before the first test and after each
/// body before the update, so a change in the body is the value the next iteration starts from
/// and a closure keeps its own iteration's binding; `for`-`in` with `let` or `const` binds anew
/// each iteration (`#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`).
#[test]
fn for_let_closures_see_one_binding_per_iteration() {
    both(
        "var fs = []; for (let i = 0; i < 3; i++) { fs[i] = function () { return i; }; } \
         fs[0]() + \",\" + fs[1]() + \",\" + fs[2]()",
        &normal("\"0,1,2\""),
    );
    both(
        "var fs = []; for (let i = 0; i < 6; i++) { fs.push(function () { return i; }); i++; } \
         fs[0]() + \",\" + fs[1]() + \",\" + fs[2]()",
        &normal("\"1,3,5\""),
    );
    both(
        "var fs = []; for (let i = 0; i < 3; i++) { if (i === 1) continue; fs.push(function () { return i; }); } \
         fs[0]() + \",\" + fs[1]()",
        &normal("\"0,2\""),
    );
    both(
        "var fs = []; for (const k in { a: 1, b: 2 }) { fs.push(function () { return k; }); } \
         fs[0]() + fs[1]()",
        &normal("\"ab\""),
    );
    both(
        "var fs = []; for (var i = 0; i < 3; i++) { fs[i] = function () { return i; }; } \
         fs[0]() + \",\" + fs[2]()",
        &normal("\"3,3\""),
    );
}

/// `#sec-globaldeclarationinstantiation`, step 15, and the global environment's
/// [[DeclarativeRecord]]: a script's `let` and `const` bindings are no properties of the global
/// object; a lexical name over a configurable property of the global object shadows it.
#[test]
fn script_lexical_bindings_are_no_properties_of_the_global_object() {
    both(
        "let a = 1; const b = 2; var g = this; \
         [typeof g.a, \"a\" in g, \"b\" in g, a, b].join(\",\")",
        &normal("\"undefined,false,false,1,2\""),
    );
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(eval_text(&mut agent, &realm, "this.c = 5; c"), normal("5"));
    assert_eq!(eval_text(&mut agent, &realm, "let c = 6; c"), normal("6"));
    assert_eq!(eval_text(&mut agent, &realm, "c + this.c"), normal("11"));
}

/// GlobalDeclarationInstantiation, steps 3 and 4: a later script of the realm sees the lexical
/// bindings of an earlier one by name; it throws a `SyntaxError` before any of its own bindings
/// exists when one of its `var`, function or lexical names is a lexical binding of the realm or
/// one of its lexical names a restricted global property (`#sec-hasrestrictedglobalproperty`:
/// `undefined`, a `var` of an earlier script). A binding whose script threw before its
/// declaration ran stays uninitialized.
#[test]
fn a_later_script_sees_and_conflicts_with_lexical_bindings() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let mut run = |source: &str| eval_text(&mut agent, &realm, source);
    assert_eq!(
        run("let x = 1; var v = 2; function f() { return x; }"),
        normal("undefined")
    );
    assert_eq!(run("x + f()"), normal("2"));
    assert_eq!(run("x = 3; f()"), normal("3"));
    for source in [
        "var fresh1; var x;",
        "var fresh1; let x;",
        "var fresh1; function x() {}",
        "var fresh1; const x = 0;",
        "var fresh1; let undefined;",
        "var fresh1; let v;",
        "let fresh2; var x;",
        "let fresh2; let undefined;",
    ] {
        assert_eq!(run(source), thrown("SyntaxError"), "{source}");
    }
    assert_eq!(
        run("typeof fresh1 + typeof fresh2"),
        normal("\"undefinedundefined\"")
    );
    assert_eq!(run("\"fresh1\" in this"), normal("false"));
    assert_eq!(
        run("let late = (function () { throw 0; })();"),
        Outcome::Throw("0".to_string())
    );
    assert_eq!(run("late"), thrown("ReferenceError"));
    assert_eq!(run("late = 1"), thrown("ReferenceError"));
    assert_eq!(run("typeof late"), thrown("ReferenceError"));
    assert_eq!(run("let late = 2;"), thrown("SyntaxError"));
}

/// PerformEval, steps 17 to 19, and EvalDeclarationInstantiation, steps 3 and 16: the lexical
/// bindings of eval code, direct or indirect, strict or not, live in a record of their own that
/// ends with it (a closure of the eval code keeps it); non-strict eval code whose `var` or
/// function name is a lexical binding of the global environment or of a record between its
/// LexicalEnvironment and its VariableEnvironment throws a `SyntaxError` before it binds
/// anything; a `catch` parameter lets it pass (B.3.4).
#[test]
fn lexical_declarations_of_eval_code_live_in_their_own_record() {
    for (source, expected) in [
        ("eval(\"let a = 1; a + 1\")", "2"),
        ("eval(\"let a = 1;\"); typeof a", "\"undefined\""),
        ("(0, eval)(\"let q = 1; q\")", "1"),
        ("(0, eval)(\"const q = 1;\"); typeof q", "\"undefined\""),
        ("eval(\"let a = 4; (function () { return a; })\")()", "4"),
        ("eval(\"'use strict'; var s = 1; let t = 2; s + t\")", "3"),
        (
            "eval(\"'use strict'; var s = 1;\"); typeof s",
            "\"undefined\"",
        ),
        ("eval(\"let a;\"); eval(\"let a = 5; a\")", "5"),
        (
            "function f() { eval(\"let a = 1;\"); return typeof a; } f();",
            "\"undefined\"",
        ),
    ] {
        both(source, &normal(expected));
    }
    // Non-strict eval code adds its `var` bindings to the caller's VariableEnvironment.
    for (source, expected) in [
        (
            "function f() { eval(\"let a = 1; var b = 2;\"); return typeof a + b; } f();",
            "\"undefined2\"",
        ),
        (
            "function f() { { let a; } eval(\"var a = 1;\"); return a; } f();",
            "1",
        ),
        (
            "try { throw 1; } catch (e) { eval(\"var e = 2;\"); e }",
            "2",
        ),
    ] {
        assert_eq!(run(limits(), source), normal(expected), "{source}");
    }
    for source in [
        "let g1 = 1; eval(\"var g1;\");",
        "let g1 = 1; eval(\"function g1() {}\");",
        "let g2 = 1; (0, eval)(\"var g2;\");",
        "function f() { let a = 1; eval(\"var a;\"); } f();",
        "function f() { { const a = 1; eval(\"var a;\"); } } f();",
        "function f() { for (let i = 0; i < 1; i++) { eval(\"var i;\"); } } f();",
    ] {
        assert_eq!(run(limits(), source), thrown("SyntaxError"), "{source}");
    }
    assert_eq!(
        run(
            limits(),
            "let g1 = 1; try { eval(\"var fresh = 1; var g1;\"); } catch (e) {} typeof fresh"
        ),
        normal("\"undefined\"")
    );
}

/// `#sec-delete-operator-runtime-semantics-evaluation` and
/// `#sec-declarative-environment-records-deletebinding-n`: `delete` of a lexical binding (non-strict
/// code only) answers `false`, before its initialization too, and checks nothing.
#[test]
fn delete_of_a_lexical_binding_answers_false_and_checks_nothing() {
    for source in [
        "let a = 1; [delete a, a].join(\",\")",
        "var r = delete b; let b = 2; [r, b].join(\",\")",
        "{ var r = delete c; let c = 3; } String(r)",
        "function f() { var r = (function () { return delete d; })(); let d; return String(r); } f();",
        "function f() { let e = 1; return String(eval(\"delete e\")) + e; } f();",
        "function f() { var r = eval(\"delete e\"); let e = 1; return String(r); } f();",
    ] {
        let expected = match source.chars().next() {
            Some('l') => "\"false,1\"",
            Some('v') => "\"false,2\"",
            _ => "",
        };
        let found = run(limits(), source);
        if expected.is_empty() {
            let Outcome::Normal(text) = &found else {
                panic!("{source}: {found:?}");
            };
            assert!(text.starts_with("\"false"), "{source}: {found:?}");
        } else {
            assert_eq!(found, normal(expected), "{source}");
        }
    }
}

/// The loop of [`the_per_iteration_record_is_charged`]: [`iterations`] closures kept in an
/// array, each over the loop's binding, declared with `kind`.
fn kept_closures(kind: &str) -> String {
    let n = iterations();
    format!(
        "var fs = []; for ({kind} i = 0; i < {n}; i++) {{ fs[i] = function () {{ return i; }}; }} \
         fs.length"
    )
}

/// The iterations of [`the_per_iteration_record_is_charged`]: 4,000, the full count, in the
/// default mode's lane `slow` (`SWELL_FULL_COUNTS=1`); 500 in the default mode's gate; 50
/// with the feature `collect-every-safe-point`, in the gate and in the lane alike, where every
/// instruction collects the whole heap, so the loop's cost grows with the square of its
/// length. At 4,000 the test took 1.00 CPU seconds alone in the default mode (measured on
/// 2026-11-23). The budget is a reading of the two loops at the same count and half their
/// difference, so it scales with the count: 500 records of more than 32 bytes each still
/// put the `let` loop past it and leave the `var` loop under it, and a record that was not
/// charged leaves both loops with the same bytes, which the assertion of more than 32 bytes a
/// record refuses at any count, as the collect mode's 50 show.
fn iterations() -> u32 {
    if cfg!(feature = "collect-every-safe-point") {
        50
    } else if std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1") {
        4_000
    } else {
        500
    }
}

/// The bytes an agent with `limits` holds after `source` ran in a fresh realm, collected.
fn bytes_after(limits: swell::Limits, source: &str) -> (Outcome, u64) {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let outcome = eval_text(&mut agent, &realm, source);
    agent.collect().unwrap();
    (outcome, agent.heap_bytes().unwrap())
}

/// Decision 3, "Lexical bindings", item 6: the record CreatePerIterationEnvironment makes is
/// charged as any record. A `for (let i ...)` loop whose closures capture `i`, all kept, holds a
/// record per iteration that the same loop over `var i` does not: under a budget between the two
/// the first ends in `Terminated(MemoryBudget)`, the second completes. The budget is taken from
/// the bytes each holds after one run with the test limits.
#[test]
fn the_per_iteration_record_is_charged() {
    let (var_outcome, var_bytes) = bytes_after(limits(), &kept_closures("var"));
    let (let_outcome, let_bytes) = bytes_after(limits(), &kept_closures("let"));
    let n = iterations();
    let count = normal(&n.to_string());
    assert_eq!(var_outcome, count);
    assert_eq!(let_outcome, count);
    println!("n {n}, var loop {var_bytes} bytes, let loop {let_bytes} bytes");
    // A record per iteration is more than 32 bytes.
    assert!(
        let_bytes > var_bytes + u64::from(n) * 32,
        "{let_bytes} against {var_bytes}"
    );
    let mut bounded = limits();
    bounded.memory_bytes = var_bytes + (let_bytes - var_bytes) / 2;
    println!("budget {}", bounded.memory_bytes);
    assert_eq!(run(bounded, &kept_closures("var")), count);
    assert_eq!(
        run(bounded, &kept_closures("let")),
        Outcome::Terminated(Termination::MemoryBudget)
    );
}

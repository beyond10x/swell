//! Function declarations in a Block and in the CaseBlock of a `switch`
//! (`#sec-blockdeclarationinstantiation`, `#sec-block-runtime-semantics-evaluation`,
//! `#sec-switch-statement-runtime-semantics-evaluation`): a lexical binding of that scope, made
//! with a new function object at each entry of the scope before its first statement, and no
//! `var` of the function, the script or the eval code (`docs/design/engine.md`, decision 3,
//! "Function declarations in a block"). Written by `story:block-function-declarations`. Run
//! also with the feature `collect-every-safe-point`.
//!
//! Annex B, B.3.2 (`#sec-block-level-function-declarations-web-legacy-compatibility-semantics`),
//! is slice 18: until then non-strict code answers as strict code does, and the non-strict half
//! of [`a_block_function_is_not_visible_after_the_block`] changes its answer with that slice
//! (item 4 of the design's section).
#![cfg(test)]

use crate::support;

use support::host::{agent_with, limits, realm};
use support::script::{Outcome, describe, eval, run, run_both};
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

/// `#sec-blockdeclarationinstantiation`, step 3.d: every function of a Block is made and its
/// binding initialized before the Block's first statement, so it can be called above its
/// declaration: in a block of a script and of a function body, in a `try`, `catch` and `finally`
/// block, by itself (recursion) and by another function of the same block, and it sees the
/// block's `let` bindings and a direct `eval` in the block sees it.
#[test]
fn a_block_function_is_callable_before_its_declaration_in_the_block() {
    for (source, expected) in [
        ("var r; { r = f(); function f() { return 1; } } r", "1"),
        (
            "var r; { function fact(n) { return n <= 1 ? 1 : n * fact(n - 1); } r = fact(5); } r",
            "120",
        ),
        (
            "var r; { r = even(10); function even(n) { return n === 0 ? true : odd(n - 1); } \
             function odd(n) { return n === 0 ? false : even(n - 1); } } r",
            "true",
        ),
        (
            "var r; { let x = 2; r = f(); function f() { return x; } } r",
            "2",
        ),
        (
            "function g() { var out = []; { out.push(b()); function b() { return \"block\"; } } \
             try { out.push(t()); function t() { return \"try\"; } } catch (e) {} \
             try { throw 0; } catch (e) { out.push(c(e)); function c(x) { return \"catch\" + x; } } \
             try {} finally { out.push(z()); function z() { return \"finally\"; } } \
             return out.join(); } g()",
            "\"block,try,catch0,finally\"",
        ),
        (
            "function g() { { function f() { return 3; } return eval(\"f()\"); } } g()",
            "3",
        ),
        (
            "var r; { function f(a, b) {} r = [f.name, f.length, typeof f.prototype].join(); } r",
            "\"f,2,object\"",
        ),
    ] {
        both(source, &normal(expected));
    }
}

/// `#sec-block-runtime-semantics-evaluation`, step 4: after the Block the running execution
/// context's LexicalEnvironment is the one around it again, so a function the Block declared is
/// not visible after it: reading it throws a `ReferenceError`, `typeof` answers `"undefined"`,
/// an outer binding of the same name keeps its value, and the script's global object has no
/// property of that name; in a script, in a function body and after a `switch`. In non-strict
/// code this changes with slice 18, when B.3.2 gives the function a `var` binding as well
/// (`docs/design/engine.md`, decision 3, "Function declarations in a block", item 4).
#[test]
fn a_block_function_is_not_visible_after_the_block() {
    both("{ function f() {} } f;", &thrown("ReferenceError"));
    both("{ function f() {} } f();", &thrown("ReferenceError"));
    both(
        "function g() { { function f() {} } return f; } g();",
        &thrown("ReferenceError"),
    );
    both(
        "switch (0) { case 0: function f() {} } f;",
        &thrown("ReferenceError"),
    );
    for (source, expected) in [
        ("{ function f() {} } typeof f", "\"undefined\""),
        (
            "function g() { { function f() {} } return typeof f; } g()",
            "\"undefined\"",
        ),
        (
            "function f() { return \"outer\"; } { function f() { return \"inner\"; } } f()",
            "\"outer\"",
        ),
        ("var f = \"outer\"; { function f() {} } f", "\"outer\""),
        ("{ function f() {} } \"f\" in this", "false"),
    ] {
        both(source, &normal(expected));
    }
}

/// `#sec-blockdeclarationinstantiation`, step 3.d.iv: InstantiateFunctionObject runs at each
/// entry of the Block, so two iterations of a loop body, and two calls of a function whose body
/// holds the Block, give two objects that are not `===`; each closes over its own entry's
/// bindings.
#[test]
fn each_entry_of_a_block_makes_a_new_function_object() {
    for (source, expected) in [
        (
            "var fs = []; for (var i = 0; i < 2; i++) { fs[i] = f; function f() {} } \
             fs[0] !== fs[1] && typeof fs[0] === \"function\" && typeof fs[1] === \"function\"",
            "true",
        ),
        (
            "var fs = []; for (var i = 0; i < 3; i++) { let j = i; fs[i] = f; \
             function f() { return j; } } fs[0]() + \",\" + fs[1]() + \",\" + fs[2]()",
            "\"0,1,2\"",
        ),
        (
            "function g() { { return f; function f() {} } } g() !== g()",
            "true",
        ),
        (
            "var fs = []; for (var i = 0; i < 2; i++) { switch (i) { default: fs[i] = f; \
             function f() {} } } fs[0] !== fs[1]",
            "true",
        ),
    ] {
        both(source, &normal(expected));
    }
}

/// `#sec-switch-statement-runtime-semantics-evaluation`, steps 4 to 6: the CaseBlock is one
/// scope, entered after the discriminant is evaluated, whose functions are made before the first
/// clause's test; every clause sees the same object, a clause before the declaration can call it,
/// and the discriminant still sees the binding around the `switch`.
#[test]
fn a_case_clause_function_is_shared_by_every_clause() {
    for (source, expected) in [
        (
            "var seen = []; switch (1) { case 0: function f() {} case 1: seen.push(f); \
             default: seen.push(f); } seen[0] === seen[1] && typeof seen[0] === \"function\"",
            "true",
        ),
        (
            "var r; switch (0) { case 0: r = later(); break; case 1: \
             function later() { return \"later\"; } } r",
            "\"later\"",
        ),
        (
            "var r; switch (2) { case 0: function f() { return 0; } default: r = f(); } r",
            "0",
        ),
        (
            "let g = \"outer\"; var r = []; switch (typeof g) { case \"string\": \
             r.push(typeof g); function g() {} } r.push(g); r.join()",
            "\"function,outer\"",
        ),
        (
            "var r; switch (1) { case (r = typeof f, 1): function f() {} } r",
            "\"function\"",
        ),
    ] {
        both(source, &normal(expected));
    }
}

/// `#sec-performeval` and `#sec-evaldeclarationinstantiation`: a function of a Block of eval
/// code, direct or indirect, belongs to that Block, never to the record PerformEval makes, so
/// it is visible neither after the Block in the eval code nor in the code that called `eval`;
/// a top-level function of eval code stays a `var` declaration of it (decision 3, "Function
/// declarations in a block", item 3): of the caller's variable environment for non-strict
/// direct eval code, of the global object for indirect eval code, and of the eval code's own
/// record for strict eval code.
#[test]
fn a_block_function_in_eval_code_belongs_to_its_block() {
    for (source, expected) in [
        (
            "eval(\"{ function f() { return 1; } } typeof f\")",
            "\"undefined\"",
        ),
        ("eval(\"{ function f() { return 1; } f(); }\")", "1"),
        ("eval(\"{ function f() {} }\"); typeof f", "\"undefined\""),
        (
            "function g() { eval(\"{ function f() {} }\"); return typeof f; } g()",
            "\"undefined\"",
        ),
        (
            "(0, eval)(\"{ function f() {} } typeof f\")",
            "\"undefined\"",
        ),
        (
            "(0, eval)(\"{ function h() {} }\"); typeof h",
            "\"undefined\"",
        ),
        (
            "eval(\"let a = 1; switch (a) { case 1: function f() { return a; } } typeof f\")",
            "\"undefined\"",
        ),
        ("(0, eval)(\"function top2() { return 3; }\"); top2()", "3"),
        (
            "eval(\"'use strict'; function top3() {} { function inner() {} } \
             typeof top3 + typeof inner\")",
            "\"functionundefined\"",
        ),
    ] {
        both(source, &normal(expected));
    }
    // Non-strict direct eval code adds its top-level function to the caller's variable
    // environment; the strict caller's eval code is strict and keeps it.
    let (sloppy, strict) = run_both(
        "function g() { eval(\"function top() { return 2; } { function inner() {} }\"); \
         return typeof top + typeof inner; } g()",
    );
    assert_eq!(sloppy, normal("\"functionundefined\""));
    assert_eq!(strict, normal("\"undefinedundefined\""));
}

/// The iterations of [`a_block_entered_in_a_loop_ends_at_the_memory_and_the_step_budget`]:
/// 4,000; 50 with the feature `collect-every-safe-point`, where every instruction collects the
/// whole heap, so the loop's cost grows with the square of its length.
const ITERATIONS: u32 = if cfg!(feature = "collect-every-safe-point") {
    50
} else {
    4_000
};

/// A loop of `n` iterations whose body is a Block that declares a function, each object kept in
/// an array (`declared` true), or the same loop whose Block declares nothing and keeps a Number
/// in its place (`declared` false); the script answers the array's length.
fn block_loop(n: u32, declared: bool) -> String {
    let body = if declared {
        "fs[i] = f; function f() { return i; }"
    } else {
        "fs[i] = 0;"
    };
    format!("var fs = []; for (var i = 0; i < {n}; i++) {{ {body} }} fs.length")
}

/// The outcome of `source` in a fresh agent with `limits`, the steps of its entry (read before
/// the outcome is described, which is an entry of its own) and the bytes the agent holds after
/// it, collected.
fn measured(limits: swell::Limits, source: &str) -> (Outcome, u64, u64) {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let completion = eval(&mut agent, &realm, source);
    let steps = agent.last_entry_steps().unwrap();
    let outcome = describe(&mut agent, &realm, completion);
    agent.collect().unwrap();
    (outcome, steps, agent.heap_bytes().unwrap())
}

/// Decision 3, "Function declarations in a block", item 5: each function object a Block makes
/// is the closure instruction's allocation and step, at each entry. A loop of [`ITERATIONS`]
/// whose Block makes a function it keeps spends a step and holds more than 32 bytes an
/// iteration beyond the same loop whose Block declares nothing and keeps a Number. Each budget
/// is the reading of the loop of none of those iterations, measured on an agent with the test
/// limits, plus a room of half what the loop of [`ITERATIONS`] added to it: the loop then ends
/// in `Terminated(StepBudget)` and in `Terminated(MemoryBudget)`, never in a hang, while the
/// loop of none completes under the same budget.
#[test]
fn a_block_entered_in_a_loop_ends_at_the_memory_and_the_step_budget() {
    let n = u64::from(ITERATIONS);
    let count = normal(&ITERATIONS.to_string());
    let (none_outcome, none_steps, none_bytes) = measured(limits(), &block_loop(0, true));
    let (kept_outcome, kept_steps, kept_bytes) = measured(limits(), &block_loop(ITERATIONS, true));
    let (plain_outcome, plain_steps, plain_bytes) =
        measured(limits(), &block_loop(ITERATIONS, false));
    assert_eq!(none_outcome, normal("0"));
    assert_eq!(kept_outcome, count);
    assert_eq!(plain_outcome, count);
    println!(
        "n {n}: none {none_steps} steps {none_bytes} bytes; kept {kept_steps} steps \
         {kept_bytes} bytes; a Number kept {plain_steps} steps {plain_bytes} bytes"
    );
    assert!(
        kept_steps >= plain_steps + n,
        "{kept_steps} steps against {plain_steps}"
    );
    assert!(
        kept_bytes > plain_bytes + n * 32,
        "{kept_bytes} bytes against {plain_bytes}"
    );

    let step_room = (kept_steps - none_steps) / 2;
    let mut stepped = limits();
    stepped.steps_per_entry = none_steps + step_room;
    println!("step budget {} (room {step_room})", stepped.steps_per_entry);
    assert_eq!(
        run(stepped, &block_loop(ITERATIONS, true)),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(run(stepped, &block_loop(0, true)), normal("0"));

    let memory_room = (kept_bytes - none_bytes) / 2;
    let mut bounded = limits();
    bounded.memory_bytes = none_bytes + memory_room;
    println!(
        "memory budget {} (room {memory_room})",
        bounded.memory_bytes
    );
    assert_eq!(
        run(bounded, &block_loop(ITERATIONS, true)),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    assert_eq!(run(bounded, &block_loop(0, true)), normal("0"));
}

/// A ledger with no bound that reads what it holds and its high-water mark.
#[derive(Default)]
struct HighWater {
    held: usize,
    high: usize,
}

impl swell_syntax::Ledger for HighWater {
    fn charge(&mut self, bytes: usize) -> Result<(), swell_syntax::Stop> {
        self.held += bytes;
        self.high = self.high.max(self.held);
        Ok(())
    }

    fn credit(&mut self, bytes: usize) {
        self.held -= bytes;
    }

    fn poll(&mut self) -> Result<(), swell_syntax::Stop> {
        Ok(())
    }
}

/// The frontend's ledger over the parse and compile of `text`: when the parse returns the
/// ledger holds exactly what the tree holds by capacity, and while the compile step runs its
/// high-water mark covers the tree and the code, after which it holds exactly both.
fn charged_as_read(text: &str) {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut ledger = HighWater::default();
    let options = swell_syntax::ParseOptions { max_nesting: 100 };
    let Ok(tree) = swell_syntax::parse_script_with(&units, options, &mut ledger) else {
        panic!("the source did not parse");
    };
    assert_eq!(ledger.held, tree.capacity_bytes(), "held after the parse");
    assert_eq!(tree.charged, tree.capacity_bytes());
    ledger.high = ledger.held;
    let Ok(code) = swell_compiler::compile_script_with(&tree, &mut ledger) else {
        panic!("the source did not compile");
    };
    let held = tree.charged + code.charged;
    println!(
        "tree {} code {} high-water {}",
        tree.charged, code.charged, ledger.high
    );
    assert_eq!(code.charged, code.capacity_bytes());
    assert!(
        ledger.high >= held,
        "high-water {} below {held}",
        ledger.high
    );
    assert_eq!(ledger.held, held, "the compile step credited its scratch");
}

/// The declarations of [`many_block_functions_are_charged_as_they_are_read`]: 10,000, the full
/// count, in the default mode and in the lane `slow` (`SWELL_FULL_COUNTS=1`); 1,000 in the gate
/// with the feature `collect-every-safe-point`, whose build is slower (with 10,000 the test took
/// 6.8 s to 28.2 s there at a load of 105 to 120 on 20 cores, cargo-nextest). The test runs no
/// script, so the feature changes nothing it checks; 1,000 declarations still grow every list
/// of the parse and the compile step past its first capacity many times over.
fn declarations() -> usize {
    if cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
    {
        1_000
    } else {
        10_000
    }
}

/// The frontend's ledger over a script of [`declarations`] Blocks that each declare a
/// function, one Block that declares as many, and a `switch` of as many clauses that each
/// declare one: the tree, the early errors' frames, the scope records of the resolver and the
/// code that makes each function at the entry of its scope are charged as they grow.
#[test]
fn many_block_functions_are_charged_as_they_are_read() {
    let n = declarations();
    let blocks: Vec<String> = (0..n)
        .map(|i| format!("{{ function f{i}() {{ return {i}; }} }}"))
        .collect();
    let functions: Vec<String> = (0..n).map(|i| format!("function f{i}() {{}}")).collect();
    let clauses: Vec<String> = (0..n)
        .map(|i| format!("case {i}: function f{i}() {{ return f{i}; }}"))
        .collect();
    charged_as_read(&blocks.join("\n"));
    charged_as_read(&format!("{{ {} }}", functions.join(" ")));
    charged_as_read(&format!("switch (x) {{ {} }}", clauses.join(" ")));
}

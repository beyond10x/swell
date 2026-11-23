//! Array patterns: IteratorBindingInitialization
//! (`#sec-runtime-semantics-iteratorbindinginitialization`) of an ArrayBindingPattern in a `var`,
//! `let` or `const` declaration, a parameter, a `catch` parameter and the head of a `for`,
//! `for`-`in` and `for`-`of`, and IteratorDestructuringAssignmentEvaluation
//! (`#sec-runtime-semantics-iteratordestructuringassignmentevaluation`) of an
//! ArrayAssignmentPattern. The value's iterator is got by GetIterator (`#sec-getiterator`), each
//! element and elision is one IteratorStepValue (`#sec-iteratorstepvalue`), a default is
//! evaluated only for `undefined`, a rest element collects the remaining values into a new
//! Array, and the iterator is closed by IteratorClose (`#sec-iteratorclose`) when the pattern
//! ends before the iterator is done and when a target, a default or a nested pattern throws,
//! never after a throw of `next` (`docs/design/engine.md`, decision 3, "The iterator of
//! `for`-`of`, of an array pattern and of a spread", item 5). Written by
//! `story:array-destructuring-and-spread`. Run also with the feature `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`.
#![cfg(test)]

use crate::support;

use std::cell::Cell;

use support::host::{agent, realm};
use support::script::{Outcome, define_function, eval_text, run_both};
use swell::{Completion, Cx, HostCall, InterruptHandle, Termination, Value};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `source` answers `true` in sloppy and in strict code.
fn passes(source: &str) {
    assert_eq!(run_both(source), (passed(), passed()), "{source}");
}

/// `source` after the declarations of [`ITERABLE`].
fn with_iterable(source: &str) -> String {
    [ITERABLE, source].concat()
}

/// `iterable(n)` answers 1 to `n`, then done; its iterator writes "next" to `log` for each call
/// of `next` and "return" for each call of `return`, which answers an object. `throwingNext()`
/// is an iterable whose `next` throws "next", also with a `return` that logs.
const ITERABLE: &str = "var log = [];\n\
     function iterable(n) {\n\
       var o = {};\n\
       o[Symbol.iterator] = function () {\n\
         var i = 0;\n\
         return {\n\
           next: function () { log.push(\"next\"); i++; return { value: i, done: i > n }; },\n\
           \"return\": function () { log.push(\"return\"); return {}; }\n\
         };\n\
       };\n\
       return o;\n\
     }\n\
     function throwingNext() {\n\
       var o = {};\n\
       o[Symbol.iterator] = function () {\n\
         return {\n\
           next: function () { log.push(\"next\"); throw \"next\"; },\n\
           \"return\": function () { log.push(\"return\"); return {}; }\n\
         };\n\
       };\n\
       return o;\n\
     }\n";

/// `#sec-runtime-semantics-iteratorbindinginitialization`: an array pattern binds its names from
/// an Array, a String (its code points) and a user iterable, with elisions, defaults and nested
/// patterns of both kinds, in a `var`, `let` and `const` declaration, a parameter of a
/// function, an arrow, a method and a setter, a rest parameter that is a pattern, a `catch`
/// parameter, and the heads of `for`, `for`-`in` and `for`-`of`, with a binding per iteration
/// for `let`.
#[test]
fn array_patterns_bind_in_declarations_parameters_catch_and_for_heads() {
    passes(
        "var [a, , b] = [1, 2, 3];\n\
         let [c, [d, e] = [4, 5], { f }] = [\"c\", undefined, { f: 6 }];\n\
         const [g = 7, h = 8] = [undefined, null];\n\
         var [s, t] = \"x\\u{1F600}\";\n\
         var [] = [];\n\
         a === 1 && b === 3 && c === \"c\" && d === 4 && e === 5 && f === 6\n\
           && g === 7 && h === null && s === \"x\" && t === \"\\u{1F600}\";",
    );
    passes(
        "function fn([a, b], [c] = [3], ...[d, e]) { return a + b + c + d + e; }\n\
         var arrow = ([a, [b]], { c: [c] }) => a + b + c;\n\
         var o = { m([a] = [\"m\"]) { return a; }, set s([v]) { this.v = v; } };\n\
         o.s = [\"set\"];\n\
         fn([1, 2], undefined, 4, 5) === 15 && arrow([1, [2]], { c: [3] }) === 6\n\
           && o.m() === \"m\" && o.v === \"set\" && fn.length === 1 && arrow.length === 2;",
    );
    passes(
        "var caught;\n\
         try { throw [1, [2]]; } catch ([x, [y]]) { caught = x + y; }\n\
         var r = [];\n\
         for (var [k, v] = [\"k\", \"v\"]; r.length < 1; ) r.push(k + v);\n\
         for (let [i] = [0]; i < 2; i++) r.push(i);\n\
         for (var [p, q] in { ab: 1 }) r.push(p + q);\n\
         for (let [m, n] of [[1, 2], [3, 4]]) r.push(m * n);\n\
         for (const [name, value] of Object.entries({ x: 1 })) r.push(name + value);\n\
         var fs = [];\n\
         for (let [z] of [[\"a\"], [\"b\"]]) fs.push(function () { return z; });\n\
         caught === 3 && r.join() === \"kv,0,1,ab,2,12,x1\" && fs[0]() + fs[1]() === \"ab\";",
    );
}

/// `#sec-runtime-semantics-iteratordestructuringassignmentevaluation`, step 1: the reference of
/// each target (a property access with a computed key) is evaluated before the step that reads
/// its value, and PutValue follows the default; the value of the assignment is its right side;
/// a rest target is evaluated before the remaining steps; the head of a `for`-`of` and of a
/// `for`-`in` assigns each value; a nested object pattern and a parenthesized target work.
#[test]
fn an_array_assignment_pattern_evaluates_its_targets_before_each_step() {
    passes(&with_iterable(
        "var o = {}, order = [];\n\
         function key(k) { order.push(\"key \" + k); return k; }\n\
         function target() { order.push(\"target\"); return o; }\n\
         var it = {}; it[Symbol.iterator] = function () { var i = 0; return { next: function () {\n\
           i++; order.push(\"step \" + i); return { value: i, done: i > 2 }; } }; };\n\
         var result = ([target()[key(\"a\")], target()[key(\"b\")] = 9, ...target()[key(\"r\")]] = it);\n\
         var x, y, z, w = {};\n\
         [x, [y], { z }, (w.p)] = [1, [2], { z: 3 }, 4];\n\
         var r = [];\n\
         for ([x, y] of [[5, 6]]) r.push(x + y);\n\
         for ([x] in { q: 0 }) r.push(x);\n\
         order.join() === \"target,key a,step 1,target,key b,step 2,target,key r,step 3\"\n\
           && result === it && o.a === 1 && o.b === 2 && o.r.length === 0\n\
           && x === \"q\" && y === 6 && z === 3 && w.p === 4 && r.join() === \"11,q\";",
    ));
}

/// IteratorClose (`#sec-iteratorclose`) of an array pattern: a pattern that ends before the
/// iterator is done closes it once (also the empty pattern and one of elisions), with a normal
/// completion, so a throw of `return` is the pattern's throw; a throw of a target's reference,
/// of a default and of a PutValue closes it once and the target's throw stands, whatever
/// `return` throws. A pattern whose last step found the iterator done does not close it.
#[test]
fn a_pattern_closes_its_iterator_when_it_ends_early_and_on_a_throw_of_a_target() {
    passes(&with_iterable(
        "var r = [];\n\
         var [a] = iterable(3); r.push(log.join()); log = [];\n\
         var [] = iterable(3); r.push(log.join()); log = [];\n\
         [, ,] = iterable(3); r.push(log.join()); log = [];\n\
         var [b, c, d] = iterable(2); r.push(log.join()); log = [];\n\
         r.join(\";\") === \"next,return;return;next,next,return;next,next,next\"\n\
           && a === 1 && b === 1 && c === 2 && d === undefined;",
    ));
    passes(&with_iterable(
        "var r = [];\n\
         function attempt(f) { try { f(); r.push(\"none\"); } catch (e) { r.push(e); } r.push(log.join()); log = []; }\n\
         var frozen = Object.freeze({});\n\
         attempt(function () { var [x = (function () { throw \"default\"; })()] = [undefined, 2]; });\n\
         attempt(function () { var o; [o.p] = iterable(3); });\n\
         attempt(function () { \"use strict\"; [frozen.p] = iterable(3); });\n\
         attempt(function () { var [{ p }] = [null]; });\n\
         var badReturn = {}; badReturn[Symbol.iterator] = function () { return {\n\
           next: function () { return { value: 1, done: false }; },\n\
           \"return\": function () { log.push(\"return\"); throw \"from return\"; } }; };\n\
         attempt(function () { var [x] = badReturn; });\n\
         attempt(function () { var [x = (function () { throw \"default\"; })()] = badReturn.nothing || iterable(0); });\n\
         var it = {}; it[Symbol.iterator] = function () { return { next: function () { return { value: undefined, done: false }; },\n\
           \"return\": function () { log.push(\"return\"); throw \"from return\"; } }; };\n\
         attempt(function () { var [x = (function () { throw \"default\"; })()] = it; });\n\
         r[0] === \"default\" && r[1] === \"\" && r[2] instanceof TypeError && r[3] === \"next,return\"\n\
           && r[4] instanceof TypeError && r[5] === \"next,return\"\n\
           && r[6] instanceof TypeError && r[7] === \"\"\n\
           && r[8] === \"from return\" && r[9] === \"return\"\n\
           && r[10] === \"default\" && r[11] === \"next\"\n\
           && r[12] === \"default\" && r[13] === \"return\";",
    ));
}

/// `#sec-runtime-semantics-iteratorbindinginitialization` with IteratorStepValue: a pattern is
/// not closed after a throw of `next`, of reading `done` or `value`, or a result that is no
/// object, each of which the pattern throws; nor when its steps found the iterator done, nor
/// after a rest element, which runs it to the end.
#[test]
fn a_pattern_does_not_close_an_iterator_that_is_done_or_whose_next_threw() {
    passes(&with_iterable(
        "var r = [];\n\
         function attempt(f) { try { f(); r.push(\"none\"); } catch (e) { r.push(e instanceof TypeError ? \"TypeError\" : e); } r.push(log.join()); log = []; }\n\
         function bad(next) { var o = {}; o[Symbol.iterator] = function () { return { next: next,\n\
           \"return\": function () { log.push(\"return\"); return {}; } }; }; return o; }\n\
         attempt(function () { var [x] = throwingNext(); });\n\
         attempt(function () { var [x] = bad(function () { return 1; }); });\n\
         attempt(function () { var [x] = bad(function () { return { get done() { throw \"done\"; } }; }); });\n\
         attempt(function () { var [x] = bad(function () { return { done: false, get value() { throw \"value\"; } }; }); });\n\
         attempt(function () { var [x, y] = iterable(1); });\n\
         attempt(function () { var [x, ...y] = iterable(3); });\n\
         attempt(function () { var [, x] = throwingNext(); });\n\
         r.join(\";\") === \"next;next;TypeError;;done;;value;;none;next,next;none;next,next,next,next;next;next\";",
    ));
}

/// `#sec-destructuring-binding-patterns-runtime-semantics-iteratorbindinginitialization` for a
/// BindingRestElement and `#sec-runtime-semantics-iteratordestructuringassignmentevaluation`
/// for an AssignmentRestElement: the rest collects the remaining values into a new Array of the
/// current realm, empty when none remain, after elisions and elements, its target a name, a
/// property or a nested pattern; the iterator is done after it and not closed.
#[test]
fn rest_elements_collect_the_remaining_values() {
    passes(&with_iterable(
        "var [a, , ...rest] = [1, 2, 3, 4, 5];\n\
         var [...all] = \"ab\";\n\
         let [x, ...none] = [1];\n\
         var [...[p, q]] = [6, 7];\n\
         var [...{ length }] = iterable(3);\n\
         var o = {};\n\
         [, ...o.list] = iterable(2);\n\
         Array.isArray(rest) && rest.join() === \"3,4,5\" && Object.getPrototypeOf(rest) === Array.prototype\n\
           && all.join() === \"a,b\" && none.length === 0 && Array.isArray(none)\n\
           && p === 6 && q === 7 && length === 3 && o.list.join() === \"2\"\n\
           && log.join() === \"next,next,next,next,next,next,next\";",
    ));
}

/// Item 5 of the design: a nested pattern holds an iterator of its own, closed before the outer
/// one on a throw inside it; at its end the inner pattern closes its own iterator when it is
/// not done, and the outer pattern goes on with its next element.
#[test]
fn a_nested_pattern_closes_the_inner_iterator_then_the_outer() {
    passes(
        "var log = [];\n\
         function named(name, values) {\n\
           var o = {};\n\
           o[Symbol.iterator] = function () {\n\
             var i = 0;\n\
             return {\n\
               next: function () { return { value: values[i++], done: i > values.length }; },\n\
               \"return\": function () { log.push(name); return {}; }\n\
             };\n\
           };\n\
           return o;\n\
         }\n\
         var caught;\n\
         try { var [[a, b = (function () { throw \"inner\"; })()]] = named(\"outer\", [named(\"inner\", [1, undefined])]); }\n\
         catch (e) { caught = e; }\n\
         var first = log.join(); log = [];\n\
         var [[c], d] = named(\"outer\", [named(\"inner\", [2, 3]), 4, 5]);\n\
         var second = log.join(); log = [];\n\
         var x, y;\n\
         try { [[x, { y = (function () { throw \"deep\"; })() }]] = named(\"outer\", [named(\"inner\", [1, {}])]); }\n\
         catch (e) { caught += e; }\n\
         caught === \"innerdeep\" && first === \"inner,outer\" && second === \"inner,outer\"\n\
           && c === 2 && d === 4 && log.join() === \"inner,outer\";",
    );
}

thread_local! {
    /// The handle `interruptNow` sets.
    static HANDLE: Cell<Option<InterruptHandle>> = const { Cell::new(None) };
}

/// A host function that sets its agent's interrupt flag once.
fn interrupt_now(
    _cx: &mut Cx<'_, support::host::TestHost>,
    _call: HostCall<'_>,
) -> Completion<Value> {
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Ok(Value::Undefined)
}

/// Item 4 of the design: a termination enters no handler, so an interrupt set by a default of
/// an array pattern ends the run in `Terminated(Interrupted)` and `return` of the pattern's
/// iterator is not called; the agent runs the next entry.
#[test]
fn a_termination_in_a_default_calls_no_return() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    let source = with_iterable(
        "var reached = false; var [a = (interruptNow(), 0), b] = [undefined, 2]; reached = true;",
    );
    assert_eq!(
        eval_text(&mut agent, &realm, &source),
        Outcome::Terminated(Termination::Interrupted)
    );
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    let source = "var u = {}; u[Symbol.iterator] = function () { return {\n\
         next: function () { log.push(\"next\"); return { value: undefined, done: false }; },\n\
         \"return\": function () { log.push(\"return\"); return {}; } }; };\n\
         var [x = (interruptNow(), 0), y] = u;";
    assert_eq!(
        eval_text(&mut agent, &realm, source),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "reached + \";\" + log.join()"),
        Outcome::Normal("\"false;next\"".to_string())
    );
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

/// The elements of [`a_large_pattern_and_spread_are_charged_as_they_are_read`]: 10,000, the
/// full count, in the lane `slow` (`SWELL_FULL_COUNTS=1`); 1,000 in the gate, in either mode.
/// The test runs no script, so the feature `collect-every-safe-point` changes nothing it
/// checks; at 10,000 it took 0.59 CPU seconds alone in the default mode (measured on
/// 2026-11-23). 1,000 elements still grow every list of the parse and the compile step past
/// its first capacity many times over, and each assertion compares the ledger with what the
/// lists hold at that size (a list that grew uncharged leaves the ledger below it at any
/// size).
fn elements() -> usize {
    if std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1") {
        10_000
    } else {
        1_000
    }
}

/// The frontend's ledger over one script of an array binding pattern, an array assignment
/// pattern and an array literal of [`elements`] elements each, with elisions, defaults, a rest
/// element and spreads, and a call with as many spread arguments: the tree, the reading of the
/// literal again as a pattern (the cover grammar, credited), the early errors, the scope
/// records, the code, the handler of each pattern and the compile lists are charged as they
/// grow.
#[test]
fn a_large_pattern_and_spread_are_charged_as_they_are_read() {
    let n = elements();
    let names: Vec<String> = (0..n)
        .map(|i| match i % 3 {
            0 => format!("a{i}"),
            1 => format!("a{i} = {i}"),
            _ => String::new(),
        })
        .collect();
    let spreads: Vec<String> = (0..n)
        .map(|i| {
            if i % 2 == 0 {
                format!("...s{i}")
            } else {
                i.to_string()
            }
        })
        .collect();
    let source = format!(
        "var [{0}, ...rest] = o;\n[{0}, ...rest] = o;\nvar s = [{1}];\nf({1});\n",
        names.join(", "),
        spreads.join(", ")
    );
    charged_as_read(&source);
}

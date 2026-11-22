//! Spread: ArrayAccumulation of a SpreadElement (`#sec-runtime-semantics-arrayaccumulation`)
//! and ArgumentListEvaluation of `...` (`#sec-runtime-semantics-argumentlistevaluation`) in an
//! array literal, a call, a direct `eval`, an optional call and `new`. The value's iterator is
//! got by GetIterator (`#sec-getiterator`) and stepped by IteratorStepValue
//! (`#sec-iteratorstepvalue`) until it is done, in one instruction that keeps the record on the
//! kept stack, charges one step per element and never closes the iterator
//! (`docs/design/engine.md`, decision 3, "The iterator of `for`-`of`, of an array pattern and of
//! a spread", item 5). Written by `story:array-destructuring-and-spread`. Run also with the
//! feature `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`.
#![cfg(test)]

use crate::support;

use support::host::{agent, agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, eval, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{Outcome as HostOutcome, Termination};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `source` answers `true` in sloppy and in strict code.
fn passes(source: &str) {
    assert_eq!(run_both(source), (passed(), passed()), "{source}");
}

/// `source` after the declarations of [`COUNTED`].
fn with_counted(source: &str) -> String {
    [COUNTED, source].concat()
}

/// `counted(values)` is an iterable of `values` whose `[Symbol.iterator]` writes "iter" to
/// `log`, whose `next` writes "next" and whose `return` writes "return"; `args()` answers its
/// arguments joined by `,`; `mark(v)` writes `v` to `log` and answers it.
const COUNTED: &str = "var log = [];\n\
     function counted(values) {\n\
       var o = {};\n\
       o[Symbol.iterator] = function () {\n\
         log.push(\"iter\");\n\
         var i = 0;\n\
         return {\n\
           next: function () { log.push(\"next\"); i++; return { value: values[i - 1], done: i > values.length }; },\n\
           \"return\": function () { log.push(\"return\"); return {}; }\n\
         };\n\
       };\n\
       return o;\n\
     }\n\
     function args() { return Array.prototype.join.call(arguments); }\n\
     function mark(v) { log.push(v); return v; }\n";

/// `#sec-runtime-semantics-arrayaccumulation` and `#sec-runtime-semantics-argumentlistevaluation`:
/// a spread in an array literal (among elements, elisions and a trailing comma, of an Array, a
/// String by code points, an empty Array and a user iterable), in the arguments of a call, a
/// method call, an optional call of both kinds (not iterated when the chain stops), `new` and
/// a direct `eval`, gets the iterator once and evaluates in source order; the Array's elements
/// are own data properties, defined past a setter of `Array.prototype`.
#[test]
fn spread_in_arrays_calls_and_new_iterates_once_in_order() {
    passes(&with_counted(
        "var a = [mark(0), ...counted([1, 2]), mark(3), ...[], ...[4]];\n\
         var holes = [, ...[1], , ];\n\
         var s = [...\"a\\u{1F600}\"];\n\
         var called = args(mark(\"x\"), ...counted([1, 2]), 3, ...counted([]));\n\
         function C() { this.v = args.apply(null, arguments); this.t = this instanceof C; }\n\
         var made = new C(...[1, 2], ...counted([3]));\n\
         var o = { m: function () { return this === o && args.apply(null, arguments); } };\n\
         var opt = o?.m(...[1]), opt2 = o.m?.(...[2]), none = o.nothing?.(...counted([9]));\n\
         var direct = (function () { var local = 5; return eval(...[\"local\"], 0); })();\n\
         var empty = eval(...[]);\n\
         Object.defineProperty(Array.prototype, \"1\", { set: function () { throw \"setter\"; }, configurable: true });\n\
         var defined = [...\"pq\"];\n\
         delete Array.prototype[1];\n\
         a.join() === \"0,1,2,3,4\" && holes.length === 3 && !(0 in holes) && holes[1] === 1 && !(2 in holes)\n\
           && s.length === 2 && s[1] === \"\\u{1F600}\" && called === \"x,1,2,3\"\n\
           && made.v === \"1,2,3\" && made.t && opt === \"1\" && opt2 === \"2\" && none === undefined\n\
           && direct === 5 && empty === undefined && defined.join() === \"p,q\"\n\
           && Object.getOwnPropertyDescriptor(defined, \"1\").writable\n\
           && log.join() === \"0,iter,next,next,next,3,x,iter,next,next,next,iter,next,iter,next,next\";",
    ));
}

/// IteratorStepValue (`#sec-iteratorstepvalue`) under a spread: `next`, read once by
/// GetIterator, is called on the iterator with no arguments until a result is done; `value` is
/// read only from a result that is not done. A throw of `next`, of `done` or of `value`, a
/// result that is no object and a value that is not iterable throw out of the spread.
#[test]
fn a_spread_over_a_user_iterator_calls_next_until_done() {
    passes(
        "var log = [];\n\
         var it = {};\n\
         it[Symbol.iterator] = function () {\n\
           var i = 0, self = {\n\
             next: function () {\n\
               log.push(\"next \" + arguments.length + (this === self));\n\
               self.next = null;\n\
               i++;\n\
               return { get done() { log.push(\"done\"); return i > 2; }, get value() { log.push(\"value\"); return i; } };\n\
             }\n\
           };\n\
           return self;\n\
         };\n\
         var values = [...it];\n\
         var r = [];\n\
         function attempt(f) { try { f(); r.push(\"none\"); } catch (e) { r.push(e instanceof TypeError ? \"TypeError\" : e); } }\n\
         function bad(next) { var o = {}; o[Symbol.iterator] = function () { return { next: next }; }; return o; }\n\
         attempt(function () { [...bad(function () { throw \"next\"; })]; });\n\
         attempt(function () { args(...bad(function () { return 1; })); });\n\
         attempt(function () { new Object(...bad(function () { return { get done() { throw \"done\"; } }; })); });\n\
         attempt(function () { [...bad(function () { return { done: false, get value() { throw \"value\"; } }; })]; });\n\
         attempt(function () { [...1]; });\n\
         attempt(function () { args(...undefined); });\n\
         attempt(function () { [...{}]; });\n\
         function args() { return arguments.length; }\n\
         values.join() === \"1,2\"\n\
           && log.join() === \"next 0true,done,value,next 0true,done,value,next 0true,done\"\n\
           && r.join() === \"next,TypeError,done,value,TypeError,TypeError,TypeError\";",
    );
}

/// Item 5 of the design: a spread runs its iterator to the end and never calls `return`, not
/// when it completes, not when `next` or `value` throws, not when the callee of the call it
/// fills is not callable (its arguments are evaluated first) and not when `new` of it throws.
#[test]
fn a_spread_never_closes_its_iterator() {
    passes(&with_counted(
        "var r = [];\n\
         function attempt(f) { try { f(); r.push(\"none\"); } catch (e) { r.push(e instanceof TypeError ? \"TypeError\" : e); } r.push(log.join()); log = []; }\n\
         function throwing(at) { var o = {}; o[Symbol.iterator] = function () { var i = 0; return {\n\
           next: function () { i++; if (i === at) throw \"next\"; return { value: i, done: false }; },\n\
           \"return\": function () { log.push(\"return\"); return {}; } }; }; return o; }\n\
         attempt(function () { [...counted([1])]; });\n\
         attempt(function () { args(...throwing(2)); });\n\
         attempt(function () { var notCallable = 0; notCallable(...counted([1])); });\n\
         attempt(function () { new args(...throwing(1)); });\n\
         attempt(function () { new Math.max(...counted([])); });\n\
         r.join(\";\") === \"none;iter,next,next;next;;TypeError;iter,next,next;next;;TypeError;iter,next\";",
    ));
}

/// The record and the values a spread has read are held on the kept stack while `next` runs
/// (`docs/design/engine.md`, decision 3, "Iterators held by a builtin"): a `next` that makes
/// garbage and fresh Strings, with a collection at every safe point in the mode
/// `collect-every-safe-point`, leaves every value read, into an Array and into the arguments of
/// a call and of `new`. Ten values with one piece of garbage each: in the collect mode 30 values
/// with four took 8.3 s.
#[test]
fn a_spread_keeps_its_values_across_a_collection_in_next() {
    passes(
        "function fresh(n) {\n\
           var o = {};\n\
           o[Symbol.iterator] = function () {\n\
             var i = 0;\n\
             return { next: function () {\n\
               [{}, \"garbage \" + i];\n\
               i++;\n\
               return { value: { text: \"v\" + i }, done: i > n };\n\
             } };\n\
           };\n\
           return o;\n\
         }\n\
         function texts() { var t = []; for (var k = 0; k < arguments.length; k++) t.push(arguments[k].text); return t.join(); }\n\
         function C() { this.t = texts.apply(null, arguments); }\n\
         var a = [...fresh(10)];\n\
         var t = []; for (var k = 0; k < a.length; k++) t.push(a[k].text);\n\
         var expected = []; for (var k = 1; k <= 10; k++) expected.push(\"v\" + k);\n\
         t.join() === expected.join() && texts(...fresh(10)) === expected.join()\n\
           && new C(...fresh(10)).t === expected.join();",
    );
}

/// Whether a test takes its gate count: with the feature `collect-every-safe-point`, unless
/// `SWELL_FULL_COUNTS=1` (the lane `slow`) asks for the full one.
fn gate_count() -> bool {
    cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

/// The elements of the large Array of [`spread_charges_each_element`]: 1,000, the full count;
/// 100 in the gate with the feature `collect-every-safe-point`, where every instruction
/// collects: there the full count took 34 s of wall time.
fn large() -> u64 {
    if gate_count() { 100 } else { 1_000 }
}

/// Item 5 of the design: a spread charges one step per element read, beside the steps of
/// `next`: `last_entry_steps()` after `[...a]` over an Array of 1,000 elements ([`large`])
/// exceeds that over an Array of 10 by at least 990 (the difference of the lengths), and so
/// does a call `f(...a)`; both Arrays are made by an earlier script, as Arrays of holes, which
/// the Array iterator reads as `undefined`.
#[test]
fn spread_charges_each_element() {
    let n = large();
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = format!(
        "var small = new Array(10), large = new Array({n});\n\
         function f() {{}}\n\
         true;"
    );
    assert_eq!(eval_text(&mut agent, &realm, &setup), passed());
    let mut steps_of = |source: &str| {
        let outcome = eval(&mut agent, &realm, source);
        assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
        agent.last_entry_steps().unwrap()
    };
    for (small, large) in [
        ("[...small];", "[...large];"),
        ("f(...small);", "f(...large);"),
    ] {
        let few = steps_of(small);
        let many = steps_of(large);
        println!("{small} {few} steps, {large} {many} steps");
        assert!(many >= few + (n - 10), "{small} {few}, {large} {many}");
    }
}

/// `Limits::min_stack_bytes` through a spread (decision 3, "Depth bounds"): the spread calls
/// `next` from its instruction, a re-entry, so `f` re-enters itself through a user iterator's
/// `next` until the re-entry limit, on seven eighths of `min_stack_bytes()`, in a child process;
/// into an Array, a call and `new`. In the collect mode the depth is [`BUILTINS_REENTRY`]. The
/// argument list of a call and of `new` is read by the same instruction as an Array
/// (`SpreadArray`), so the gate count ([`gate_count`]) runs the Array alone: the three took
/// 3.0 s of wall time in the collect mode beside the other tests of the package.
#[test]
fn a_spread_of_a_user_iterator_re_enters_on_the_promised_stack() {
    fn body() {
        let spreads: &[&str] = if gate_count() {
            &["[...o]"]
        } else {
            &["[...o]", "Math.max(...o)", "new Object(...o)"]
        };
        for spread in spreads {
            reach_through(
                &format!(
                    "var o = {{}}; o[Symbol.iterator] = function () {{ return {{ next: function () {{ \
                     f(); return {{ done: true }}; }} }}; }}; {spread}"
                ),
                BUILTINS_REENTRY,
            );
        }
    }
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("a_spread_of_a_user_iterator_re_enters_on_the_promised_stack"),
        stack,
        body,
    )
    .unwrap();
}

/// The iterable of [`an_endless_spread_ends_at_the_step_and_the_memory_budget`]: `endless(n,
/// work)` answers `n` values (`Infinity` for no end), each the answer of `work()`, and its
/// `return` counts its calls in `returns`.
const ENDLESS: &str = "var returns = 0;\n\
     function endless(n, work) {\n\
       var o = {};\n\
       o[Symbol.iterator] = function () {\n\
         var i = 0;\n\
         return {\n\
           next: function () { var v = work(); i++; return { value: v, done: i > n }; },\n\
           \"return\": function () { returns++; return {}; }\n\
         };\n\
       };\n\
       return o;\n\
     }\n\
     function nothing() {}\n\
     function count() { return arguments.length; }\n\
     true;";

/// The String the growing spread of [`an_endless_spread_ends_at_the_step_and_the_memory_budget`]
/// repeats, made by a script of its own: it runs where the step budget is not the bound.
const CHUNK: &str = "var chunk = \"x\".repeat(1024); true;";

/// A spread of `endless(n, work)` into an Array or into the arguments of a call, in a function
/// so that what it made is garbage once the run has ended; it answers how many values it read.
fn budget_spread(n: &str, work: &str, into_call: bool) -> String {
    if into_call {
        format!("(function () {{ return count(...endless({n}, {work})); }})();")
    } else {
        format!("(function () {{ return [...endless({n}, {work})].length; }})();")
    }
}

/// The step room of [`an_endless_spread_ends_at_the_step_and_the_memory_budget`] above the
/// control spread's steps: 1,000; 100 with the feature `collect-every-safe-point`.
const STEP_ROOM: u64 = if cfg!(feature = "collect-every-safe-point") {
    100
} else {
    1_000
};

/// The memory room above the reading of an agent that ran the control spreads and collected:
/// 1 MiB, which the Strings of a growing spread (a flat String of 65,536 code units, 128 KiB,
/// each) pass after about 8 values.
const MEMORY_ROOM: u64 = 1 << 20;

/// Item 5 of the design and "Iterators held by a builtin", item 2: a spread of an iterator that
/// is never done, into an Array and into a call, ends in `Terminated(StepBudget)` when `next`
/// does nothing and in `Terminated(MemoryBudget)` when each value is a new String of 128 KiB,
/// never in a hang, and calls no `return`. The step budget is the reading of the control spread
/// (3 values) plus [`STEP_ROOM`], the memory budget the heap after the control spreads and a
/// collection plus [`MEMORY_ROOM`]; the control spreads complete under each. After each
/// termination the agent runs again.
#[test]
fn an_endless_spread_ends_at_the_step_and_the_memory_budget() {
    let mut measure = agent().unwrap();
    let measured = realm(&mut measure);
    for setup in [ENDLESS, CHUNK] {
        assert_eq!(eval_text(&mut measure, &measured, setup), passed());
    }
    let mut steps = 0;
    for into_call in [false, true] {
        let control = budget_spread("3", "nothing", into_call);
        let outcome = eval(&mut measure, &measured, &control);
        assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
        steps = steps.max(measure.last_entry_steps().unwrap());
    }
    measure.collect().unwrap();
    let heap = measure.heap_bytes().unwrap();
    println!("control: {steps} steps, heap {heap} bytes after a collection");

    for (grows, termination) in [
        (false, Termination::StepBudget),
        (true, Termination::MemoryBudget),
    ] {
        let mut budgeted = limits();
        if grows {
            budgeted.memory_bytes = heap + MEMORY_ROOM;
        } else {
            budgeted.steps_per_entry = steps + STEP_ROOM;
        }
        let mut agent = agent_with(budgeted).unwrap();
        let realm = realm(&mut agent);
        let setups: &[&str] = if grows { &[ENDLESS, CHUNK] } else { &[ENDLESS] };
        for setup in setups {
            assert_eq!(eval_text(&mut agent, &realm, setup), passed());
        }
        let work = if grows {
            "function () { return chunk.repeat(64); }"
        } else {
            "nothing"
        };
        for into_call in [false, true] {
            let source = budget_spread("Infinity", work, into_call);
            assert_eq!(
                eval_text(&mut agent, &realm, &source),
                Outcome::Terminated(termination),
                "{source}"
            );
            agent.collect().unwrap();
            assert_eq!(
                eval_text(&mut agent, &realm, "returns"),
                Outcome::Normal("0".to_string()),
                "{source}"
            );
            let control = budget_spread("3", "nothing", into_call);
            assert_eq!(
                eval_text(&mut agent, &realm, &control),
                Outcome::Normal("3".to_string()),
                "the control after {source}"
            );
        }
    }
}

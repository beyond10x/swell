//! The instructions of the loop of `buildString` (the test262 harness, `harness/regExpUtils.js`)
//! answer as they did before `story:property-escape-cases-spend-less-in-the-interpreter-loop`
//! made them cheaper: a local read and written in and after its temporal dead zone
//! (`GetLocalChecked`, `SetLocalChecked`), the numeric operators on a Number, on an object with
//! `valueOf` and on a Symbol (`ToNumeric`, `Increment`, `Negate`, `BitNot`), an indexed put
//! (`PutIndex`) past the end, on a frozen Array, through a setter and through a Proxy of the
//! chain, and the steps an indexed put spends in the walk of its prototypes. Every expectation
//! here was read on the base of that story, before its first cut. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{agent_with, get, global, limits, object, realm};
use support::script::{Outcome, eval, run, run_both};
use swell::{Outcome as HostOutcome, Termination, Value};

/// A String answer as [`support::script::show`] writes it.
fn normal(text: &str) -> Outcome {
    Outcome::Normal(format!("\"{text}\""))
}

/// A `let` binding read and written by the checked instructions: in its temporal dead zone (a
/// `switch` whose case runs before the declaration's) both throw a `ReferenceError` that names
/// it, and the binding stays uninitialized; after its declaration, in the head of a `for`
/// statement as in `buildString`, both read and write it. The defects it holds: a checked read
/// that answers the slot without looking at its state, and a checked write that stores into an
/// uninitialized slot.
#[test]
fn a_local_in_and_after_its_temporal_dead_zone_reads_and_writes_as_before() {
    let source = "function g(n) { \
          switch (n) { \
            case 0: let x = 'init'; return x; \
            case 1: return x; \
            case 2: x = 2; return 'stored'; \
            case 3: x = 3; return x; \
          } \
        } \
        function caught(n) { \
          try { return g(n); } \
          catch (e) { return e.name + ':' + (e.message.indexOf('x') >= 0); } \
        } \
        function loop() { \
          var a = []; \
          for (let length = 0, codePoint = 5; codePoint <= 9; codePoint++) { \
            a[length++] = codePoint; \
            if (length === 3) { a[length++] = 'c'; } \
          } \
          return a.join(); \
        } \
        [caught(0), caught(1), caught(2), caught(3), caught(1), loop()].join(';');";
    let answer = normal(
        "init;ReferenceError:true;ReferenceError:true;ReferenceError:true;ReferenceError:true;\
         5,6,7,c,8,9",
    );
    assert_eq!(run_both(source), (answer.clone(), answer));
}

/// `++`, `--`, unary `-` and `~` on a Number answer ToNumeric of it unchanged (-0 and NaN
/// included), on an object with `valueOf` call it once each, and on a Symbol throw a
/// `TypeError` and leave the binding as it was. The defects it holds: a ToNumeric of a Number
/// that changes it (-0 to +0), one of an object that does not call `valueOf`, and one of a
/// Symbol that answers instead of throwing.
#[test]
fn numeric_operators_on_a_number_an_object_and_a_symbol_answer_as_before() {
    let source = "var log = ''; \
        var n = 1; var r1 = n++; \
        var z = -0; var r2 = z--; \
        var nan = NaN; var r3 = nan++; \
        var o = { valueOf: function () { log += 'v'; return 41; } }; var r4 = o++; \
        var q = { valueOf: function () { log += 'q'; return 5; } }; var r5 = -q; var r6 = ~q; \
        var s = Symbol('s'); var thrown = ''; \
        try { s++; } catch (e) { thrown += e.name; } \
        try { -s; } catch (e) { thrown += ',' + e.name; } \
        var big = 9007199254740992; big++; \
        [r1, n, Object.is(r2, -0), z, r3 !== r3, nan !== nan, r4, o, r5, r6, log, thrown, \
         typeof s, big, -n, ~n].join(';');";
    let answer = normal(
        "1;2;true;-1;true;true;41;42;-5;-6;vqq;TypeError,TypeError;symbol;9007199254740992;-2;-3",
    );
    assert_eq!(run_both(source), (answer.clone(), answer));
}

/// An indexed put as `buildString` makes it, at the end of an Array, past its end, after a
/// truncation, on a frozen Array (refused: false in sloppy code, a `TypeError` in strict code),
/// with a setter for the index on `Array.prototype` (called, the Array keeps its length) and
/// with a Proxy in the chain (its `set` trap called with the Array as the receiver). The defects
/// it holds: a put at the end that does not raise the length or does not look at the chain, and
/// one on a frozen Array that stores.
#[test]
fn an_indexed_put_past_the_end_on_a_frozen_array_and_through_the_chain_answers_as_before() {
    let source = "var a = []; \
        for (var i = 0; i < 5; i++) a[i] = i * 2; \
        a[7] = 'h'; \
        var t = [1, 2, 3]; t.length = 0; t[0] = 'z'; t[1] = 'y'; \
        var f = Object.freeze([1]); \
        var strict = (function () { 'use strict'; \
          try { f[1] = 2; return 'stored'; } catch (e) { return e.name; } })(); \
        var strict0 = (function () { 'use strict'; \
          try { f[0] = 2; return 'stored'; } catch (e) { return e.name; } })(); \
        var sloppy = (function () { f[1] = 3; f[0] = 4; return f.length + ':' + f[0]; })(); \
        var seen = ''; \
        Object.defineProperty(Array.prototype, '0', \
          { set: function (v) { seen += v + (this === b); }, configurable: true }); \
        var b = []; b[0] = 'x'; \
        delete Array.prototype[0]; \
        var trapped = ''; \
        var proxy = new Proxy({}, { set: function (target, key, value, receiver) { \
          trapped += key + '=' + value + (receiver === c); return true; } }); \
        var c = []; Object.setPrototypeOf(c, proxy); c[0] = 'y'; \
        [a.join(), a.length, t.join(), t.length, strict, strict0, sloppy, f.join(), seen, \
         b.length, trapped, c.length, Object.keys(c).length].join(';');";
    assert_eq!(
        run(limits(), source),
        normal("0,2,4,6,8,,,h;8;z,y;2;TypeError;TypeError;1:1;1;xtrue;0;0=ytrue;0;0")
    );
}

/// The script of [`an_indexed_put_ends_at_the_step_budget_inside_its_prototype_walk`]: a put at
/// the end of an Array whose chain has two prototypes, so that OrdinarySet passes to two of
/// them, a step each, as the last statement.
const PUT: &str = "var a = []; a[0] = 1";

/// [`PUT`] in a fresh agent with `steps` per entry: whether it ended at the step budget (and
/// otherwise how it ended), the steps it spent, and what the host reads after it of the global
/// `a`: its `length` and its element 0, or the value it holds when it is no object.
fn put_with_budget(steps: u64) -> (String, u64, String) {
    let mut limits = limits();
    limits.steps_per_entry = steps;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let answer = match eval(&mut agent, &realm, PUT) {
        HostOutcome::Terminated(Termination::StepBudget) => "budget",
        HostOutcome::Terminated(_) => "other termination",
        HostOutcome::Normal(_) => "normal",
        HostOutcome::Throw(_) => "throw",
    };
    let spent = agent.last_entry_steps().unwrap();
    let global = global(&mut agent, &realm);
    let after = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global);
            match get(cx, global, "a") {
                Value::Object(array) => {
                    let length = get(cx, array, "length");
                    let first = get(cx, array, "0");
                    format!("{length:?} {first:?}")
                }
                other => format!("{other:?}"),
            }
        })
        .unwrap();
    (answer.to_string(), spent, after)
}

/// The steps of [`PUT`] with no budget in the way, read on the base.
const PUT_STEPS: u64 = 17;

/// [`PUT`] with each budget from 8 steps below [`PUT_STEPS`] to one below it, read on the base:
/// the budget, how the run ended, the steps spent and what the host reads of `a` after it.
/// Step 12 is the put's own, 13 and 14 those of its two prototypes: a budget of 11, 12 or 13
/// ends the run at one of them, before the store; from 14 on the element is stored and the
/// run ends at the steps of the statement's completion and the script's return.
const SWEEP: [(u64, &str, u64, &str); 8] = [
    (9, "budget", 9, "Number(0.0) Undefined"),
    (10, "budget", 10, "Number(0.0) Undefined"),
    (11, "budget", 11, "Number(0.0) Undefined"),
    (12, "budget", 12, "Number(0.0) Undefined"),
    (13, "budget", 13, "Number(0.0) Undefined"),
    (14, "budget", 14, "Number(1.0) Number(1.0)"),
    (15, "budget", 15, "Number(1.0) Number(1.0)"),
    (16, "budget", 16, "Number(1.0) Number(1.0)"),
];

/// The step budget ends [`PUT`] inside the walk of its put's prototypes at each of the last
/// three steps of the put (the put's own step and the step of each prototype it passes to)
/// and at the steps around them: the run ends in `Terminated(StepBudget)` with the counter at
/// the budget, and the Array holds the element only when every step of the walk was spent.
/// The expectations ([`PUT_STEPS`], [`SWEEP`]) were read on the base. The defects it holds:
/// the steps of the walk charged in one call (`charge_steps(2)` spends none at one step left,
/// where two calls spend one), a step charged after the store, and a walk that charges no step.
#[test]
fn an_indexed_put_ends_at_the_step_budget_inside_its_prototype_walk() {
    let (answer, spent, after) = put_with_budget(1_000_000);
    let seen: Vec<(u64, String, u64, String)> = (spent.saturating_sub(8)..spent)
        .map(|budget| {
            let (answer, spent, after) = put_with_budget(budget);
            (budget, answer, spent, after)
        })
        .collect();
    let expected: Vec<(u64, String, u64, String)> = SWEEP
        .iter()
        .map(|&(budget, answer, spent, after)| {
            (budget, answer.to_string(), spent, after.to_string())
        })
        .collect();
    assert_eq!(
        (answer.as_str(), spent, after.as_str(), seen),
        ("normal", PUT_STEPS, "Number(1.0) Number(1.0)", expected)
    );
}

/// A write in the temporal dead zone throws and leaves the binding uninitialized: a read, an
/// update and a `typeof` after it in the same activation throw as well, and the declaration
/// initializes it. The defect it holds: a checked write that stores before it throws.
#[test]
fn a_write_in_the_temporal_dead_zone_leaves_the_binding_uninitialized() {
    let source = "function f() { var r = []; \
          try { x = 5 } catch (e) { r.push(e.name) } \
          try { r.push(x) } catch (e) { r.push(e.name) } \
          try { x++ } catch (e) { r.push(e.name) } \
          try { r.push(typeof x) } catch (e) { r.push(e.name) } \
          let x; r.push(x === undefined); return r.join() } f()";
    let answer = normal("ReferenceError,ReferenceError,ReferenceError,ReferenceError,true");
    assert_eq!(run_both(source), (answer.clone(), answer));
}

/// The `ReferenceError` of a checked read, of a checked write and of a compound assignment in
/// the temporal dead zone names its own binding among three. The defect it holds: a message that
/// names another String constant of the code.
#[test]
fn a_read_and_a_write_in_the_temporal_dead_zone_name_their_own_binding() {
    let source = "function f() { var r = []; \
          try { alpha } catch (e) { r.push(e.message) } \
          try { beta = 1 } catch (e) { r.push(e.message) } \
          try { gamma += 1 } catch (e) { r.push(e.message) } \
          let alpha = 1, beta = 2, gamma = 3; return r.join('|') } f()";
    let answer =
        normal("alpha is not initialized|beta is not initialized|gamma is not initialized");
    assert_eq!(run_both(source), (answer.clone(), answer));
}

//! SuperProperty outside classes (`#sec-super-keyword`): `super.name` and `super[expression]` in
//! the methods, getters and setters of object literals, generator and `async` methods included.
//! A SuperProperty reads [[HomeObject]] through the running function
//! (`#sec-super-keyword-runtime-semantics-evaluation`, MakeSuperPropertyReference,
//! `#sec-makesuperpropertyreference`, GetSuperBase, `#sec-getsuperbase`): the receiver is the
//! `this` of the method, the base the [[Prototype]] of the home object when the reference is made;
//! `delete` of one throws a `ReferenceError` (`#sec-delete-operator-runtime-semantics-evaluation`).
//! Written by `story:super-property-and-super-syntax`. Run also with the feature
//! `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`, in sloppy and in strict mode.
#![cfg(test)]

use crate::support;

use support::host::{agent, limits, realm};
use support::script::{Outcome, eval_text, run, run_both};
use swell::{Outcome as HostOutcome, Termination};

/// `source` answers `true` in sloppy and in strict mode.
fn both_pass(source: &str) {
    let passed = Outcome::Normal("true".to_string());
    assert_eq!(run_both(source), (passed.clone(), passed), "{source}");
}

/// `#sec-super-keyword-runtime-semantics-evaluation`, `SuperProperty : super . IdentifierName`:
/// the property is read from the [[Prototype]] of the method's home object, not from the
/// object itself, with the method's `this` as the receiver (a getter there sees it), also when
/// the method is called on another object.
#[test]
fn super_member_reads_the_prototype_of_the_home_object() {
    both_pass(
        "var proto = { x: 1, get g() { return this.tag; } };\n\
         var o = { __proto__: proto, x: 2, tag: 'o', m() { return [super.x, this.x, super.g]; } };\n\
         var r = o.m(), s = o.m.call({ x: 3, tag: 'other' });\n\
         r[0] === 1 && r[1] === 2 && r[2] === 'o' &&\n\
         s[0] === 1 && s[1] === 3 && s[2] === 'other' && o.m.call(o)[0] === 1;",
    );
}

/// `SuperProperty : super [ Expression ]`: the key is the value of the expression, converted by
/// ToPropertyKey once, a String, a Number or a Symbol; a key the base lacks reads `undefined`.
#[test]
fn super_index_reads_with_a_computed_key() {
    both_pass(
        "var log = [], s = Symbol('s');\n\
         var proto = { a: 'pa', 1: 'p1' };\n\
         proto[s] = 'ps';\n\
         var o = { __proto__: proto, a: 'oa', 1: 'o1', m(k) { return super[k]; } };\n\
         var key = { toString() { log.push('key'); return 'a'; } };\n\
         o.m('a') === 'pa' && o.m(1) === 'p1' && o.m(s) === 'ps' && o.m(key) === 'pa' &&\n\
         log.join() === 'key' && o.m('missing') === undefined;",
    );
}

/// EvaluateCall (`#sec-evaluatecall`) of a Super Reference: GetThisValue gives the method's
/// `this`, so `super.m()`, `super[k]()`, a tag `super.t` and `super.m?.()` call the function of
/// the prototype with the receiver as `this`; a missing one with `?.()` gives `undefined`.
#[test]
fn super_method_call_gets_this() {
    both_pass(
        "var proto = {\n\
           who() { return this.name; },\n\
           add(a, b) { return this.base + a + b; },\n\
           t(strings, v) { return this === o && strings[0] === 'x' && v === 1; }\n\
         };\n\
         var o = { __proto__: proto, name: 'o', base: 10,\n\
           who() { return 'own ' + super.who(); },\n\
           sum() { return super['add'](1, 2); },\n\
           tagged() { return super.t`x${1}`; },\n\
           optional() { return [super.who?.(), super.missing?.()]; } };\n\
         var r = o.optional();\n\
         o.who() === 'own o' && o.sum() === 13 && o.tagged() === true &&\n\
         r[0] === 'o' && r[1] === undefined;",
    );
}

/// PutValue (`#sec-putvalue`) of a Super Reference: [[Set]] on the base with the receiver, so an
/// ordinary data property of the prototype is created on the receiver and the prototype keeps
/// its own; the value of the assignment is the value assigned; a destructuring pattern and a
/// for-in head assign to a SuperProperty the same way. A [[Set]] that fails (a frozen receiver)
/// throws a `TypeError` in strict code and does nothing in sloppy code.
#[test]
fn super_assignment_sets_on_the_receiver() {
    both_pass(
        "var strict = (function () { return this === undefined; })();\n\
         var proto = { x: 1 };\n\
         var o = { __proto__: proto, m() {\n\
           var v = (super.x = 2);\n\
           super['y'] = 3;\n\
           [super.a, super['b']] = [4, 5];\n\
           ({ c: super.c } = { c: 6 });\n\
           for (super.k in { p: 0 }) ;\n\
           return v;\n\
         } };\n\
         var frozen = Object.freeze({ __proto__: proto, m() {\n\
           try { super.x = 9; return 'no throw ' + this.x; }\n\
           catch (e) { return e.constructor === TypeError ? 'TypeError' : 'other'; }\n\
         } });\n\
         o.m() === 2 && o.x === 2 && o.y === 3 && o.a === 4 && o.b === 5 && o.c === 6 &&\n\
         o.k === 'p' && proto.x === 1 && !proto.hasOwnProperty('y') &&\n\
         !proto.hasOwnProperty('a') && Object.keys(proto).join() === 'x' &&\n\
         frozen.m() === (strict ? 'TypeError' : 'no throw 1');",
    );
}

/// A compound assignment, a logical assignment and an update of a SuperProperty read the base
/// and put on the receiver through one reference: the key is converted once, the base is the
/// one read before the key was converted (`prop-expr-getsuperbase-before-topropertykey-*`), and
/// a logical assignment the left value decides puts nothing.
#[test]
fn super_compound_and_logical_assignment_and_update() {
    both_pass(
        "var count = 0, key = { toString() { count++; return 'n'; } };\n\
         var proto = { n: 1, t: 1 };\n\
         var o = { __proto__: proto, m() {\n\
           return [super.n += 2, super.n++, ++super['n'], super[key] += 1, super[key]--,\n\
             super.u ??= 5, super.t ||= 9, super.z &&= 4, super.t ??= 7];\n\
         } };\n\
         var r = o.m();\n\
         var p1 = { v: 1 }, p2 = { v: -1 }, k2;\n\
         var o2 = { __proto__: p1, m() { return super[k2] += 1; }, n() { return ++super[k2]; } };\n\
         k2 = { toString() { Object.setPrototypeOf(o2, p2); return 'v'; } };\n\
         var first = o2.m();\n\
         Object.setPrototypeOf(o2, p1);\n\
         var second = o2.n();\n\
         r.join() === '3,1,2,2,1,5,1,,1' && count === 2 && o.n === 0 && o.u === 5 &&\n\
         !o.hasOwnProperty('t') && !o.hasOwnProperty('z') && proto.n === 1 &&\n\
         first === 2 && second === 2 && p1.v === 1;",
    );
}

/// A getter and a setter of an object literal are MethodDefinitions with the object as
/// [[HomeObject]] (`#sec-makemethod`): `super.v` in them reaches the accessor of the prototype
/// with the receiver, also under a computed name.
#[test]
fn super_in_a_getter_and_a_setter() {
    both_pass(
        "var log = [];\n\
         var proto = { get v() { return 'pv:' + this.id; }, set v(x) { log.push('ps:' + this.id + ':' + x); } };\n\
         var o = { __proto__: proto, id: 'o',\n\
           get v() { return 'ov ' + super.v; },\n\
           set v(x) { super.v = x + 1; },\n\
           get ['w']() { return super['v']; } };\n\
         o.v = 1;\n\
         o.v === 'ov pv:o' && o.w === 'pv:o' && log.join() === 'ps:o:2' &&\n\
         !Object.prototype.hasOwnProperty.call(o, 'x');",
    );
}

/// A generator method keeps its function, and so its home object, while it is suspended: the
/// object literal is reachable only through the generator's [[HomeObject]], and after a
/// `collect()` between two scripts the resumed method still reads its prototype.
#[test]
fn super_in_a_generator_method_after_a_collection() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var it = ({ __proto__: { x: 'p' }, *g() { yield super.x; yield super.x + '!'; } }).g();\n\
             it.next().value === 'p';",
        ),
        Outcome::Normal("true".to_string())
    );
    agent.collect().unwrap();
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var r = it.next(); r.value === 'p!' && r.done === false && it.next().done;"
        ),
        Outcome::Normal("true".to_string())
    );
}

/// An `async` method reads `super.x` before and after an `await`: the resumed frame has the
/// same function, so the same home object, after a `collect()` before the checkpoint.
#[test]
fn super_in_an_async_method_across_an_await() {
    for prologue in ["", "\"use strict\";\n"] {
        let mut agent = agent().unwrap();
        let realm = realm(&mut agent);
        let source = format!(
            "{prologue}var seen = 'pending';\n\
             ({{ __proto__: {{ x: 'p' }}, async m() {{ var a = super.x; await null; return a + super.x; }} }})\n\
               .m().then(function (v) {{ seen = v; }});\n\
             seen;"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Outcome::Normal("\"pending\"".to_string())
        );
        agent.collect().unwrap();
        let checkpoint = agent.perform_microtask_checkpoint().unwrap();
        assert!(
            matches!(checkpoint, HostOutcome::Normal(_)),
            "{checkpoint:?}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "seen"),
            Outcome::Normal("\"pp\"".to_string()),
            "{prologue}"
        );
    }
}

/// GetSuperBase (`#sec-getsuperbase`) runs at each evaluation: after `Object.setPrototypeOf` of
/// the home object the same method reads the new prototype, whatever its `this` is.
#[test]
fn the_base_is_read_at_each_evaluation_after_set_prototype_of() {
    both_pass(
        "var p1 = { x: 1 }, p2 = { x: 2 };\n\
         var o = { __proto__: p1, m() { return super.x; } };\n\
         var a = o.m();\n\
         Object.setPrototypeOf(o, p2);\n\
         var b = o.m(), c = o.m.call({ __proto__: p1 });\n\
         a === 1 && b === 2 && c === 2;",
    );
}

/// A home object whose [[Prototype]] is `null`: GetValue and PutValue of the reference throw
/// the `TypeError` of ToObject of a `null` base, for a read, a write, a computed key and a call.
#[test]
fn a_null_prototype_throws_a_type_error() {
    both_pass(
        "var o = { m() { return super.x; }, n() { super.y = 1; }, k() { return super['z']; },\n\
           c() { return super.f(); } };\n\
         Object.setPrototypeOf(o, null);\n\
         var names = ['m', 'n', 'k', 'c'].map(function (name) {\n\
           try { o[name](); return 'none'; } catch (e) { return e.constructor === TypeError; }\n\
         });\n\
         names.join() === 'true,true,true,true' &&\n\
         !Object.prototype.hasOwnProperty.call(o, 'y');",
    );
}

/// `#sec-delete-operator-runtime-semantics-evaluation`, step 5.a: `delete` of a Super Reference
/// throws a `ReferenceError`; the key expression is evaluated first and its value is not
/// converted (`delete/super-property-topropertykey.js`), and the prototype keeps its property.
#[test]
fn delete_super_index_throws_a_reference_error_without_converting_the_key() {
    both_pass(
        "var converted = false, evaluated = false;\n\
         var key = { toString() { converted = true; return 'x'; } };\n\
         var o = { __proto__: { x: 1 },\n\
           m() { delete super[(evaluated = true, key)]; },\n\
           n() { delete super.x; } };\n\
         var thrown = [o.m, o.n].map(function (f) {\n\
           try { f.call(o); return 'none'; } catch (e) { return e.constructor === ReferenceError; }\n\
         });\n\
         thrown.join() === 'true,true' && evaluated && !converted &&\n\
         Object.getPrototypeOf(o).x === 1;",
    );
}

/// [[HomeObject]] is set once, by MakeMethod: a method copied to another object, by assignment
/// or by `Object.assign`, still reads the prototype of the object it was defined in.
#[test]
fn a_copied_method_keeps_its_home_object() {
    both_pass(
        "var p1 = { who: 'p1' }, p2 = { who: 'p2' };\n\
         var a = { __proto__: p1, m() { return super.who; } };\n\
         var b = { __proto__: p2, m: a.m };\n\
         var c = Object.assign({ __proto__: p2 }, a);\n\
         b.m() === 'p1' && c.m() === 'p1' && b.m === a.m && c.m === a.m;",
    );
}

/// The calls of the step budget test: 10,000 in the default mode, 100 with the feature
/// `collect-every-safe-point`, where every instruction collects the whole heap (at 1,000 the
/// test took 11.7 s there).
fn super_calls() -> u32 {
    if cfg!(feature = "collect-every-safe-point") {
        100
    } else {
        10_000
    }
}

/// The script of the step budget test: a loop of `n` calls `super.m()`, which answers whether
/// it ran them all.
fn super_call_loop(n: u32) -> String {
    format!(
        "var o = {{ __proto__: {{ m() {{ return 1; }} }},\n\
           run(n) {{ var s = 0; for (var i = 0; i < n; i++) s += super.m(); return s; }} }};\n\
         o.run({n}) === {n};"
    )
}

/// `docs/design/engine.md`, "The step budget": a loop of [`super_calls`] calls of `super.m()`
/// completes under the test limits and ends in `Terminated(StepBudget)` with a step budget of
/// one step per call, never in a hang.
#[test]
fn a_super_call_loop_ends_at_the_step_budget() {
    let n = super_calls();
    let source = super_call_loop(n);
    assert_eq!(run(limits(), &source), Outcome::Normal("true".to_string()));
    let mut small = limits();
    small.steps_per_entry = u64::from(n);
    assert_eq!(
        run(small, &source),
        Outcome::Terminated(Termination::StepBudget)
    );
}

/// The `frame_depth` of the recursion test.
const FRAME_DEPTH: u32 = 100;

/// "Depth bounds" (decision 2): a recursion through `super.down()` and `this.again()` stops at
/// `frame_depth` with a `RangeError` the script catches, and the agent runs on.
#[test]
fn a_super_recursion_stops_at_frame_depth() {
    let mut bounded = limits();
    bounded.frame_depth = FRAME_DEPTH;
    assert_eq!(
        run(
            bounded,
            "var depth = 0;\n\
             var o = { __proto__: { down(n) { depth = n; return this.again(n); } },\n\
               again(n) { return super.down(n + 1); } };\n\
             var caught;\n\
             try { o.again(0); caught = 'none'; } catch (e) { caught = e.constructor === RangeError; }\n\
             caught === true && depth > 10 && Object.getPrototypeOf(o).down.call({ again() { return 'ran'; } }, 1) === 'ran';"
        ),
        Outcome::Normal("true".to_string())
    );
}

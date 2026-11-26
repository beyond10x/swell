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

// ---- Order of steps, receivers, unwinding, suspension, home objects and parse answers -------
// Each case answers a string that the test compares in full, so a wrong step shows which one.

/// `source` answers the String `expected` in sloppy and in strict mode.
fn both_answer(source: &str, expected: &str) {
    let want = Outcome::Normal(format!("\"{expected}\""));
    assert_eq!(run_both(source), (want.clone(), want), "{source}");
}

/// `source` answers the String `sloppy` in sloppy mode and `strict` in strict mode.
fn modes_answer(source: &str, sloppy: &str, strict: &str) {
    assert_eq!(
        run_both(source),
        (
            Outcome::Normal(format!("\"{sloppy}\"")),
            Outcome::Normal(format!("\"{strict}\""))
        ),
        "{source}"
    );
}

/// GetValue and PutValue of `super[k]` (`#sec-getvalue`, `#sec-putvalue`): ToObject of a `null`
/// base throws before ToPropertyKey of the key, for a read, a compound assignment, an update, a
/// logical assignment and a plain assignment; the plain assignment evaluates its right side
/// first.
#[test]
fn null_base_throws_before_the_key_is_converted() {
    both_answer(
        r#"var log = [];
var key = { toString() { log.push('key'); return 'x'; } };
var o = { m() { return super[key]; }, n() { return super[key] += 1; },
  p() { return super[key]++; }, q() { return super[key] ??= 1; },
  r() { super[key] = (log.push('rhs'), 1); } };
Object.setPrototypeOf(o, null);
['m', 'n', 'p', 'q', 'r'].forEach(function (name) {
  log.push(name);
  try { o[name](); log.push('none'); } catch (e) { log.push(e.name); }
});
log.join();"#,
        "m,TypeError,n,TypeError,p,TypeError,q,TypeError,r,rhs,TypeError",
    );
}

/// `super[k] = v` converts the key after the right side (the NOTE of
/// `#sec-super-keyword-runtime-semantics-evaluation`, PutValue); a compound assignment
/// converts it once, before the right side (GetValue).
#[test]
fn put_converts_the_key_after_the_right_side() {
    both_answer(
        r#"var log = [];
var key = { toString() { log.push('key'); return 'x'; } };
var o = { __proto__: { x: 10 }, m() { super[key] = (log.push('rhs'), 1); },
  n() { super[key] += (log.push('rhs2'), 1); } };
o.m(); log.push(o.x); o.n(); log.push(o.x);
log.join();"#,
        "rhs,key,1,key,rhs2,11",
    );
}

/// The base of a Super Reference is read when the reference is made, before the right side of
/// an assignment is evaluated: a prototype set in the right side is not the one written to.
#[test]
fn the_base_is_read_before_the_right_side() {
    both_answer(
        r#"var log = [];
var p1 = { get x() { log.push('g1'); return 1; }, set x(v) { log.push('s1:' + v); } };
var p2 = { get x() { log.push('g2'); return 2; }, set x(v) { log.push('s2:' + v); } };
var o = { __proto__: p1,
  a() { super.x = (Object.setPrototypeOf(o, p2), 'a'); },
  b() { super['x'] = (Object.setPrototypeOf(o, p2), 'b'); },
  c() { super.x += (Object.setPrototypeOf(o, p2), 10); },
  d() { super['x'] += (Object.setPrototypeOf(o, p2), 20); },
  e() { return super[(Object.setPrototypeOf(o, p2), 'x')]; } };
['a', 'b', 'c', 'd'].forEach(function (n) { Object.setPrototypeOf(o, p1); o[n](); });
Object.setPrototypeOf(o, p1);
log.push(o.e());
log.join();"#,
        "s1:a,s1:b,g1,s1:11,g1,s1:21,g2,2",
    );
}

/// `delete` of a Super Reference (`#sec-delete-operator-runtime-semantics-evaluation`, 5.a):
/// a `ReferenceError` also with a `null` prototype, also in parentheses, the key evaluated
/// and not converted; `typeof super.x` reads the value, so a `null` base throws a `TypeError`.
#[test]
fn delete_and_typeof_of_a_super_reference() {
    both_answer(
        r#"var log = [];
var key = { toString() { log.push('conv'); return 'x'; } };
var o = { __proto__: { x: 1 },
  a() { return delete super[(log.push('eval'), key)]; },
  b() { return delete (super.x); },
  c() { return typeof super.x; },
  d() { return typeof super.nothing; } };
function t(n) { try { return String(o[n]()); } catch (e) { return e.name; } }
log.push(t('a'), t('b'), t('c'), t('d'));
Object.setPrototypeOf(o, null);
log.push(t('a'), t('b'), t('c'));
log.join();"#,
        "eval,ReferenceError,ReferenceError,number,undefined,eval,ReferenceError,ReferenceError,TypeError",
    );
}

/// GetThisValue of a Super Reference in a call: `(super.m)()` keeps `this`; `new super.C()`
/// makes a new object; `super.m.call(x)` calls with `x`.
#[test]
fn calls_through_a_super_reference() {
    modes_answer(
        r#"var proto = { who() { return this === o ? 'o' : typeof this; },
  C: function () { this.made = this !== o; } };
var o = { __proto__: proto,
  a() { return (super.who)(); },
  b() { return (super['who'])(); },
  c() { return new super.C().made; },
  d() { return super.who.call(1); } };
[o.a(), o.b(), o.c(), o.d()].join();"#,
        "o,o,true,object",
        "o,o,true,number",
    );
}

/// [[Get]] and [[Set]] with a primitive receiver: in strict code `this` stays a Number, a
/// setter and a getter reached through `super` see it, and a data write to it fails with a
/// `TypeError`; in sloppy code `this` is a Number object.
#[test]
fn a_primitive_receiver() {
    modes_answer(
        r#"var log = [];
var proto = { set s(v) { log.push(typeof this + ':' + v); }, get g() { return typeof this; } };
var o = { __proto__: proto, w() { super.x = 1; return 'ok'; }, v() { super.s = 2; },
  r() { return super.g; } };
try { log.push(o.w.call(5)); } catch (e) { log.push(e.name); }
o.v.call(5);
log.push(o.r.call(5));
log.join();"#,
        "ok,object:2,object",
        "TypeError,number:2,number",
    );
}

/// OrdinarySetWithOwnDescriptor with a receiver other than the base: the receiver's own
/// writable property keeps its attributes, its own non-writable property and its own accessor
/// make the [[Set]] answer false (no setter of the receiver is called), as do a frozen
/// receiver and a non-writable property of the base.
#[test]
fn the_receiver_s_own_properties_decide_the_set() {
    modes_answer(
        r#"var r1 = {}; Object.defineProperty(r1, 'x', { value: 0, writable: true, enumerable: false, configurable: true });
var r2 = {}; Object.defineProperty(r2, 'x', { value: 0, writable: false, configurable: true });
var called = false; var r3 = { set x(v) { called = true; }, get x() { return 'r3'; } };
var r4 = Object.freeze({});
var pr = {}; Object.defineProperty(pr, 'x', { value: 'ro', writable: false });
var o = { __proto__: {}, m() { super.x = 5; return 'ok'; } };
var q = { __proto__: pr, m() { super.x = 5; return 'ok'; } };
function t(f, recv) { try { return f.call(recv); } catch (e) { return e.name; } }
var qr = {};
[t(o.m, r1), r1.x, Object.keys(r1).length, t(o.m, r2), r2.x, t(o.m, r3), called,
  t(o.m, r4), Object.keys(r4).length, t(q.m, qr), Object.keys(qr).length].join();"#,
        "ok,5,0,ok,0,ok,false,ok,0,ok,0",
        "ok,5,0,TypeError,0,TypeError,false,TypeError,0,TypeError,0",
    );
}

/// Every assignment target form with a SuperProperty: array and object destructuring with
/// defaults and rest, `for`-`of` and `for`-`in` heads; the prototype gets nothing.
#[test]
fn destructuring_rest_and_loop_heads() {
    both_answer(
        r#"var proto = {};
var o = { __proto__: proto, m() {
  [super.a = 'da', super['b'] = 'db'] = [undefined];
  [...super.r] = [1, 2];
  ({ x: super['c'] = 'dc', ...super.rest } = { y: 3 });
  for (super.f of ['f1', 'f2']) ;
  for (super['g'] of ['g1']) ;
  for (super['h'] in { h1: 0 }) ;
  return [this.a, this.b, this.r.join('+'), this.c, Object.keys(this.rest).join(), this.f,
    this.g, this.h, Object.keys(proto).length].join();
} };
o.m();"#,
        "da,db,1+2,dc,y,f2,g1,h1,0",
    );
}

/// The target of a destructuring assignment is evaluated before the value is read
/// (`#sec-runtime-semantics-keyeddestructuringassignmentevaluation` step 1,
/// IteratorDestructuringAssignmentEvaluation), and its key converted after.
#[test]
fn destructuring_evaluates_the_target_first() {
    both_answer(
        r#"var log = [];
var key = { toString() { log.push('key'); return 'k'; } };
var src = { get v() { log.push('get'); return 1; } };
var iter = { [Symbol.iterator]() { return { next() { log.push('next'); return { value: 2, done: false }; },
  return() { log.push('ret'); return {}; } }; } };
var o = { __proto__: {}, m() {
  ({ v: super[(log.push('expr'), key)] } = src);
  log.push('|');
  [super[(log.push('expr'), key)]] = iter;
  return this.k;
} };
log.push(o.m());
log.join();"#,
        "expr,get,key,|,expr,next,key,ret,2",
    );
}

/// The rounds of a loop of a case below: `default` in the default mode, `collect` with the
/// feature `collect-every-safe-point`, where every instruction collects the whole heap (in the
/// whole test binary under load two cases took 5.5 s and 6.6 s there before this bound).
fn rounds(default: u32, collect: u32) -> u32 {
    if cfg!(feature = "collect-every-safe-point") {
        collect
    } else {
        default
    }
}

/// Each assignment form of a SuperProperty as a statement whose value is dropped, fifty times
/// (twenty under `collect-every-safe-point`): the reference's two or three stack items are
/// consumed on both sides of a short circuit.
#[test]
fn dropped_assignment_forms_in_a_loop() {
    let n = rounds(50, 20);
    let source = r#"var o = { __proto__: { t: 1 }, m() {
  for (var i = 0; i < ROUNDS; i++) {
    super.u ??= i; super['t'] ||= i; super.z &&= i; super[i % 2] ??= i;
    super.c = (this.c | 0) + 1; super['d'] += 1; super.e++; --super['f'];
    super.g; super[i];
  }
  var a = [1, 2, 3];
  return [this.u, this.t, this.z, this[0], this[1], this.c, this.d, this.e, this.f,
    a.length, a[2]].join();
} };
o.m();"#
        .replace("ROUNDS", &n.to_string());
    both_answer(
        &source,
        &format!("{},1,,{},{},{n},NaN,NaN,NaN,3,3", n - 1, n - 2, n - 1),
    );
}

/// A getter or setter reached through `super` that throws, caught in the method, a hundred
/// times (fifty under `collect-every-safe-point`): the reference's stack items are unwound and
/// the method computes on after.
#[test]
fn a_throwing_accessor_unwinds_the_reference() {
    let n = rounds(20, 10);
    let source = r#"var proto = { get x() { throw new RangeError('g'); }, set y(v) { throw new EvalError('s'); } };
var o = { __proto__: proto, m() {
  var caught = [];
  for (var i = 0; i < ROUNDS; i++) {
    try { var a = [1, 2, super.x]; } catch (e) { caught.push(e.name[0]); }
    try { super['x'] += 1; } catch (e) { caught.push(e.name[0]); }
    try { super.y = 1; } catch (e) { caught.push(e.name[0]); }
    try { super['y'] = super.x; } catch (e) { caught.push(e.name[0]); }
    try { [super['y']] = [1]; } catch (e) { caught.push(e.name[0]); }
  }
  var after = [1, 2, 3].concat([4]).length + caught.length;
  return caught.slice(0, 5).join('') + ':' + after;
} };
o.m();"#
        .replace("ROUNDS", &n.to_string());
    both_answer(&source, &format!("RRERE:{}", 5 * n + 4));
}

/// A generator suspended between the key and the value of `super[yield] = yield`: the base
/// read before the suspension is the one written to, after the home object's prototype changed.
#[test]
fn a_generator_keeps_the_reference_across_yield() {
    both_answer(
        r#"var log = [];
var p1 = { set a(v) { log.push('p1:' + v); } }, p2 = { set a(v) { log.push('p2:' + v); } };
var o = { __proto__: p1, *g() { super[yield 'k'] = yield 'v'; super.a = yield 'w'; } };
var it = o.g();
it.next(); it.next('a'); Object.setPrototypeOf(o, p2); it.next(1);
Object.setPrototypeOf(o, p1); it.next(2);
log.join();"#,
        "p1:1,p2:2",
    );
}

/// An `async` method suspended in the right side of an assignment to a SuperProperty writes to
/// the base read before the `await`.
#[test]
fn an_async_method_keeps_the_reference_across_await() {
    for prologue in ["", "\"use strict\";\n"] {
        let mut agent = agent().unwrap();
        let realm = realm(&mut agent);
        let source = format!(
            "{prologue}var log = [];\n\
             var p1 = {{ set a(v) {{ log.push('p1:' + v); }} }}, p2 = {{ set a(v) {{ log.push('p2:' + v); }} }};\n\
             var o = {{ __proto__: p1, async m() {{\n\
               super['a'] = await (Object.setPrototypeOf(o, p2), 'v1');\n\
               super.a = await (Object.setPrototypeOf(o, p1), 'v2'); }} }};\n\
             o.m(); log.join();"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Outcome::Normal("\"\"".to_string())
        );
        let checkpoint = agent.perform_microtask_checkpoint().unwrap();
        assert!(
            matches!(checkpoint, HostOutcome::Normal(_)),
            "{checkpoint:?}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "log.join()"),
            Outcome::Normal("\"p1:v1,p2:v2\"".to_string()),
            "{prologue}"
        );
    }
}

/// [[HomeObject]] belongs to the closure: one per evaluation of a literal, kept through a
/// getter read off its descriptor and called on another object, through `bind` and through an
/// object spread.
#[test]
fn each_closure_has_its_own_home_object() {
    both_answer(
        r#"var protos = [{ v: 'a' }, { v: 'b' }, { v: 'c' }];
var ms = [];
for (var i = 0; i < 3; i++) ms.push({ __proto__: protos[i], m() { return super.v; } }.m);
function make(p) { return { __proto__: p, get g() { return super.v + ':' + this.tag; } }; }
var x = make(protos[0]), y = make(protos[1]);
var bound = ms[2].bind({});
var spread = { ...{ __proto__: protos[1], m() { return super.v; } } };
[ms[0](), ms[1](), ms[2](), Object.getOwnPropertyDescriptor(x, 'g').get.call({ tag: 'other' }),
  Object.getOwnPropertyDescriptor(y, 'g').get.call({ tag: 't' }), bound(), spread.m(),
  ms[0] !== ms[1]].join();"#,
        "a,b,c,a:other,b:t,c,b,true",
    );
}

/// No script reaches a frame with no home object: a getter run by `JSON.stringify`, a method as
/// a `sort` comparator, `new` of a method, the source text of a method given back to `eval` and
/// `Function`, and a method of a literal in a parameter's default.
#[test]
fn no_script_reaches_a_frame_without_home_object() {
    both_answer(
        r#"var o = { __proto__: { x: 'p', cmp(a, b) { return a - b; } },
  get y() { return super.x; }, cmp(a, b) { return super.cmp(a, b); }, m() { return super.x; } };
var r = [];
r.push(JSON.stringify(o));
r.push([3, 1, 2].sort(o.cmp).join(''));
try { new o.m(); r.push('constructed'); } catch (e) { r.push(e.name); }
try { r.push(eval('(' + o.m + ')')); } catch (e) { r.push(e.name); }
try { r.push(Function('return ' + o.m)); } catch (e) { r.push(e.name); }
try { r.push(Function(String(o.m).slice(String(o.m).indexOf('{') + 1, -1))()); } catch (e) { r.push(e.name); }
r.push(eval('({ __proto__: { x: 9 }, ' + o.m + ' })').m());
function f(a = { __proto__: { x: 'd' }, m() { return super.x; } }) { return a.m(); }
r.push(f());
r.join();"#,
        "{\"y\":\"p\"},123,TypeError,SyntaxError,SyntaxError,SyntaxError,9,d",
    );
}

/// A SuperProperty in the parameters of a method, a generator method and an `async` method
/// reads the home object's prototype (`#sec-method-definitions-static-semantics-early-errors`
/// admits it; the parameters are code of the method).
#[test]
fn super_in_method_parameters_runs() {
    both_answer(
        r#"var o = { __proto__: { x: 4 },
  m(a = super.x, b = super['x'] + 1) { return a + b; },
  *g(a = super.x) { yield a; },
  set s(v = super.x) { this.seen = v; } };
o.s = undefined;
[o.m(), o.g().next().value, o.seen].join();"#,
        "9,4,4",
    );
}

/// A computed key of an object literal inside a method is code of the method: `super` there is
/// admitted, and reads the method's home object.
#[test]
fn super_in_a_computed_key_inside_a_method() {
    both_answer(
        r#"var o = { __proto__: { x: 'k' }, m() {
  var inner = { [super.x]: 1, [super['x'] + '2']() { return 2; }, get [super.x + '3']() { return 3; } };
  return [inner.k, inner.k2(), inner.k3].join();
} };
o.m();"#,
        "1,2,3",
    );
}

/// The parse answers: `super` outside a method is an early error wherever the code around is
/// no method (a computed key at the top, a function as a property value, after a method and
/// its nested functions and arrows closed); `super` with an escape, alone, before a template,
/// and before `?.` is a SyntaxError in a method.
#[test]
fn where_super_is_an_error() {
    let refused = [
        "var o = { [super.x]: 1 };",
        "var o = { [super.x]() {} };",
        "var o = { get [super.x]() { return 1; } };",
        "var o = { f: function () { return super.x; } };",
        "var o = { m() { return function () { return super.x; }; } };",
        "var o = { m() { function f() {} var g = x => 1; var h = (y) => { return y; }; return super.x; } };\nsuper.y;",
        "var o = { m() { var f = { n() { return super.x; } }; } };\nsuper.z;",
        "var o = { m() { return s\\u0075per.x; } };",
        "var o = { m() { return super; } };",
        "var o = { m() { return super`x`; } };",
        "var o = { m() { return super?.x; } };",
        "var o = { m() { return new super; } };",
        "var o = { m() { return new super(); } };",
        "function f(a = { m() {} }) { return super.x; }",
    ];
    let mut wrong = Vec::new();
    for source in refused {
        let outcome = run(limits(), source);
        if !matches!(&outcome, Outcome::Errors(e) if e.len() == 1 && !e[0].contains("not supported"))
        {
            wrong.push(format!("{source:?} -> {outcome:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The parse answers: the flag is restored after a nested plain function, a concise arrow body
/// and a nested method, so `super` after them in the same method is admitted; `(super.x)` in a
/// method is no arrow's parameters.
#[test]
fn where_super_is_admitted_after_nesting() {
    both_answer(
        r#"var o = { __proto__: { x: 'p' }, m() {
  function f() { return 1; }
  var g = function () { return { n() { return 2; } }; };
  var h = x => x;
  var k = (y) => { return y; };
  var a = (super.x);
  return a + super.x + f() + g().n() + h(3) + k(4);
} };
o.m();"#,
        "pp1234",
    );
}

/// Direct and indirect `eval` of `super.x` inside a method and at the top: what the host reads.
/// ECMA-262 (`#sec-performeval`) answers the direct one in a method with the property and the
/// other three with a SyntaxError a script can catch.
#[test]
fn eval_of_super_answers_a_catchable_error() {
    both_answer(
        r#"var o = { __proto__: { x: 1 },
  d() { try { return eval('super.x'); } catch (e) { return e.name; } },
  i() { try { return (0, eval)('super.x'); } catch (e) { return e.name; } } };
var top;
try { top = eval('super.x'); } catch (e) { top = e.name; }
var ind;
try { ind = (0, eval)('super.x'); } catch (e) { ind = e.name; }
[o.d(), o.i(), top, ind].join();"#,
        "SyntaxError,SyntaxError,SyntaxError,SyntaxError",
    );
}

/// A method whose home object is reachable only through the method keeps it across a
/// collection; the home object and the method, which point at each other, are freed together
/// once nothing else reaches them.
#[test]
fn the_home_object_is_traced_and_its_cycle_freed() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var m = ({ __proto__: { x: 'p' }, m() { return super.x; } }).m; 1"
        ),
        Outcome::Normal("1".to_string())
    );
    agent.collect().unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "m.call(null)"),
        Outcome::Normal("\"p\"".to_string())
    );
    agent.collect().unwrap();
    let base = agent.live_bytes().unwrap();
    // 2,000 literals; 20 under `collect-every-safe-point` (2,000 ran past 60 s there).
    let methods: u32 = if cfg!(feature = "collect-every-safe-point") {
        20
    } else {
        2_000
    };
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "var keep = []; for (var i = 0; i < {methods}; i++) keep.push({{ m() {{ return super.x; }} }}); keep.length"
            )
        ),
        Outcome::Normal(methods.to_string())
    );
    agent.collect().unwrap();
    let with = agent.live_bytes().unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "keep = null; 0"),
        Outcome::Normal("0".to_string())
    );
    agent.collect().unwrap();
    let after = agent.live_bytes().unwrap();
    assert!(
        with > base && after.saturating_sub(base) < (with - base) / 10,
        "live_bytes: base {base}, with {methods} methods {with}, after the drop {after}"
    );
}

/// The interrupt ends a loop of `super.m()` calls run with a step budget of 2^62.
#[test]
fn an_interrupt_ends_a_super_call_loop() {
    let mut big = limits();
    big.steps_per_entry = 1 << 62;
    let mut agent = support::host::agent_with(big).unwrap();
    let realm = realm(&mut agent);
    let handle = agent.interrupt_handle();
    let interrupter = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(50));
        handle.interrupt();
    });
    let outcome = eval_text(
        &mut agent,
        &realm,
        "var o = { __proto__: { m() { return 1; } }, run() { for (;;) { super.m(); super.x = super.y; } } }; o.run();",
    );
    interrupter.join().unwrap();
    assert_eq!(outcome, Outcome::Terminated(Termination::Interrupted));
}

/// A loop of writes through `super` that grows the receiver ends at the memory budget, and a
/// loop of reads at the step budget.
#[test]
fn super_loops_end_at_the_memory_and_step_budgets() {
    let realm_bytes = {
        let mut probe = agent().unwrap();
        let _realm = realm(&mut probe);
        probe.collect().unwrap();
        probe.heap_bytes().unwrap()
    };
    let mut small = limits();
    small.memory_bytes = realm_bytes
        + if cfg!(feature = "collect-every-safe-point") {
            48 << 10
        } else {
            512 << 10
        };
    small.steps_per_entry = 1 << 40;
    assert_eq!(
        run(
            small,
            "var o = { __proto__: {}, run() { for (var i = 0; ; i++) super['k' + i] = { a: i }; } }; o.run();"
        ),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    let mut steps = limits();
    steps.steps_per_entry = u64::from(rounds(10_000, 2_000));
    assert_eq!(
        run(
            steps,
            "var o = { __proto__: { x: 1 }, run() { var s = 0; for (;;) s += super.x + super['x']; } }; o.run();"
        ),
        Outcome::Terminated(Termination::StepBudget)
    );
}

/// An anonymous function assigned to a SuperProperty gets no name (NamedEvaluation is for an
/// IdentifierReference only, `#sec-assignment-operators-runtime-semantics-evaluation`); a
/// shorthand default `{ a = super.x }` in a destructuring assignment reads through the home
/// object.
#[test]
fn function_names_and_shorthand_defaults() {
    both_answer(
        r#"var o = { __proto__: { x: 'px' }, m() {
  super.f = function () {};
  super['g'] = function () {};
  super.h ||= function () {};
  var a;
  ({ a = super.x } = {});
  return [JSON.stringify(this.f.name), JSON.stringify(this.g.name), JSON.stringify(this.h.name), a].join();
} };
o.m();"#,
        "\"\",\"\",\"\",px",
    );
}

// ---- Suspension and unwinding inside a reference, SuperCall, re-entry limits ---------------

/// A generator suspended inside a Super Reference (`super[yield] = yield`, `super[k] += yield`)
/// and resumed by `return()` and `throw()`: the reference's items are unwound, a `finally`
/// that reads `super` and yields again runs, a `catch` continues with `super`, and the frame
/// computes on after.
#[test]
fn generator_return_and_throw_while_suspended_in_a_super_reference() {
    both_answer(
        r#"var log = [];
var p1 = { set a(v) { log.push('p1:' + v); }, x: 'p1x' };
var o = { __proto__: p1,
  *g() {
    try { super[yield 'k'] = yield 'v'; log.push('after'); }
    finally { log.push('fin:' + super.x); var y = yield 'f'; super.a = y; }
  },
  *h() {
    try { super[yield 'k'] = yield 'v'; } catch (e) { log.push('caught:' + e); }
    super.a = yield 'w';
    return [1, 2, super.x].length;
  },
  *c() { try { super['n'] += yield 'c'; } finally { log.push('cf'); } } };
var it = o.g(); it.next(); it.next('a');
var r1 = it.return('R'); log.push(r1.value + r1.done);
var r2 = it.next('Y'); log.push(r2.value + r2.done);
var it2 = o.h(); it2.next(); it2.next('a');
var t1 = it2.throw('T'); log.push(t1.value + t1.done);
var t2 = it2.next('W'); log.push(t2.value + ':' + t2.done);
var it3 = o.h(); it3.next();
var r3 = it3.return('early'); log.push(r3.value + r3.done);
var it4 = o.c(); it4.next();
try { it4.throw('E'); } catch (e) { log.push(e); }
log.join();"#,
        "fin:p1x,ffalse,p1:Y,Rtrue,caught:T,wfalse,p1:W,3:true,earlytrue,cf,E",
    );
}

/// Super References inside `try`/`finally` with a `break`, a `continue` and a `return` through
/// the `finally`: the value returned is read before the `finally` changes the prototype, a
/// right side that throws leaves the reference's items behind, and nested `finally` blocks
/// that write through `super` run in order.
#[test]
fn try_finally_around_super_references_with_return_and_break() {
    both_answer(
        r#"var log = [];
var p1 = { x: 'p1' }, p2 = { x: 'p2' };
var o = { __proto__: p1,
  m() {
    for (var i = 0; i < 3; i++) {
      try { if (i === 1) break; super['k' + i] = super.x + i; }
      finally { super.f = (this.f || '') + super['x'] + i; super.x; }
    }
    try { return super.x + super['x']; }
    finally { Object.setPrototypeOf(o, p2); log.push(this.f); }
  },
  n() {
    outer: for (var j = 0; j < 2; j++) {
      try {
        super[(function () { return 'q' + j; })()] = (function () { if (j === 1) throw 'x'; return j; })();
      } catch (e) { continue outer; }
      finally { log.push('fin' + j); }
    }
    try { try { return super.x; } finally { super.y = 'set'; } } finally { log.push(this.y); }
  } };
var r = o.m();
[r, o.n(), o.k0, o.q0, 'q1' in o].join() + '|' + log.join();"#,
        "p1p1,p2,p10,0,false|p10p11,fin0,fin1,set",
    );
}

/// An `async` method whose `await` inside a Super Reference rejects: in the key, in the right
/// side of `=` and of `+=`, each caught, then a write through `super` after them; an uncaught
/// rejection rejects the method's promise and calls no setter; the base read before an `await`
/// is the one written to after the home object's prototype became `null`, and the key is
/// converted after the `await`.
#[test]
fn async_rejection_inside_a_super_reference() {
    for prologue in ["", "\"use strict\";\n"] {
        let mut agent = agent().unwrap();
        let realm = realm(&mut agent);
        let source = format!(
            "{prologue}var lm = [], ln = [], lq = [];\n\
             var p1 = {{ set a(v) {{ lm.push('p1:' + v); }} }};\n\
             var o = {{ __proto__: p1, async m() {{\n\
               try {{ super[await Promise.reject('rk')] = 1; }} catch (e) {{ lm.push('c1:' + e); }}\n\
               try {{ super['a'] = await Promise.reject('rv'); }} catch (e) {{ lm.push('c2:' + e); }}\n\
               try {{ super.a += await Promise.reject('rc'); }} catch (e) {{ lm.push('c3:' + e); }}\n\
               super.a = await 'ok';\n\
               return [1, 2, 3].length; }} }};\n\
             var on = {{ __proto__: {{ set a(v) {{ ln.push('set'); }} }}, async n() {{ super.a = await Promise.reject('out'); }} }};\n\
             var p3 = {{ set a(v) {{ lq.push('p3:' + v); }} }};\n\
             var oq = {{ __proto__: p3, async q() {{\n\
               var k = {{ toString() {{ lq.push('conv'); return 'a'; }} }}; super[k] = await null; }} }};\n\
             o.m().then(function (v) {{ lm.push('m:' + v); }});\n\
             on.n().catch(function (e) {{ ln.push('n:' + e); }});\n\
             oq.q(); Object.setPrototypeOf(oq, null);\n\
             lm.join() + '|' + ln.join() + '|' + lq.join();"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Outcome::Normal("\"||\"".to_string()),
            "{prologue}"
        );
        let checkpoint = agent.perform_microtask_checkpoint().unwrap();
        assert!(
            matches!(checkpoint, HostOutcome::Normal(_)),
            "{checkpoint:?}"
        );
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                "lm.join() + '|' + ln.join() + '|' + lq.join()"
            ),
            Outcome::Normal("\"c1:rk,c2:rv,c3:rc,p1:ok,m:3|n:out|conv,p3:null\"".to_string()),
            "{prologue}"
        );
    }
}

/// `#sec-function-definitions-static-semantics-early-errors` and
/// `#sec-arrow-function-definitions-static-semantics-early-errors`: a SuperProperty belongs to
/// the nearest function that is not an arrow function. In a method nested in the parameters of
/// an arrow function inside a method, that function is the nested method; no source here has
/// `super` in an arrow's own code. Today the parser scans the whole cover of an arrow's
/// parameters inside a method, the nested method included, and the script answers the refusal
/// `super in an arrow function is not supported before slice 8` (the base before this story
/// refused the same source with the refusal of `super`). `story:derived-classes-and-super-call`
/// lifts that refusal with the refusal of `super` in an arrow; the source must then answer
/// `"t,b,p"` in both modes, the value of `super.x` read through each nested method's home object.
#[test]
fn a_method_in_an_arrow_parameter_inside_a_method_is_refused_until_derived_classes() {
    let source = r#"var top = (a = { __proto__: { x: 't' }, n() { return super.x; } }) => a.n();
var o = { __proto__: { x: 'o' },
  body() { var f = () => ({ __proto__: { x: 'b' }, n() { return super.x; } }).n(); return f(); },
  params() { var f = (a = { __proto__: { x: 'p' }, n() { return super.x; } }) => a.n(); return f(); } };
[top(), o.body(), o.params()].join();"#;
    let refused = |line: u32| {
        Outcome::Errors(vec![format!(
            "super in an arrow function is not supported before slice 8 ({line}:65)"
        )])
    };
    assert_eq!(run_both(source), (refused(4), refused(5)), "{source}");
}

/// Every SuperCall is an early error before derived classes, wherever it stands (a method, an
/// accessor, the parameters of a method, an arrow and a plain function in a method, a
/// generator and an `async` method, the top, `Function` code), so no `Expr::SuperCall` reaches
/// the compiler; in eval code it is the refusal of `super` in eval code, a SyntaxError a script
/// catches.
#[test]
fn no_source_reaches_a_super_call() {
    let sources = [
        "var o = { m() { super(); } };",
        "var o = { get g() { return super(); } };",
        "var o = { set s(v) { super(v); } };",
        "var o = { m(a = super()) {} };",
        "var o = { m() { return () => super(); } };",
        "var o = { m() { return (a = super()) => a; } };",
        "var o = { m() { function f() { super(); } } };",
        "var o = { *g() { yield super(); } };",
        "var o = { async m() { await super(); } };",
        "var o = { async *m() { yield super(); } };",
        "var o = { m() { new super(); } };",
        "super();",
        "function f() { super(); }",
    ];
    let mut wrong = Vec::new();
    for source in sources {
        let outcome = run(limits(), source);
        if !matches!(&outcome, Outcome::Errors(e) if e.len() == 1
            && e[0].contains("super call outside a derived class constructor"))
        {
            wrong.push(format!("{source:?} -> {outcome:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    both_answer(
        r#"var r = [];
var o = { m() { try { eval('super()'); } catch (e) { r.push(e.name); } } };
o.m();
try { Function('super()'); } catch (e) { r.push(e.name); }
try { Function('a = super()', ''); } catch (e) { r.push(e.name); }
r.join();"#,
        "SyntaxError,SyntaxError,SyntaxError",
    );
}

/// The re-entry limit through accessors and keys reached through `super`: a getter whose
/// prototype getter reads it again, a setter that writes it again and a key whose `toString`
/// calls the method again each end in a `RangeError` the script catches, and the agent runs on.
#[test]
fn super_recursions_through_accessors_and_keys_end_in_a_range_error() {
    let mut bounded = limits();
    bounded.frame_depth = 100;
    assert_eq!(
        run(
            bounded,
            r#"var o = { __proto__: { get x() { return this.y; }, set x(v) { this.z = v; } },
  get y() { return super.x; }, set z(v) { super.x = v; },
  k() { return super[key]; }, ok() { return super.hasOwnProperty === Object.prototype.hasOwnProperty; } };
var key = { toString() { return o.k(); } };
function t(f) { try { f(); return 'none'; } catch (e) { return e.constructor === RangeError ? 'RangeError' : String(e); } }
[t(function () { return o.y; }), t(function () { o.z = 1; }), t(function () { return o.k(); }), o.ok()].join();"#
        ),
        Outcome::Normal("\"RangeError,RangeError,RangeError,true\"".to_string())
    );
}

/// The body of the stack test: re-entries through a getter and a setter reached through
/// `super`, and through the `toString` of the key of `super[k]` and of `super[k] += 1`, one
/// agent each.
fn reach_through_super() {
    for through in [
        "({ __proto__: { get a() { return f(); } }, m() { return super.a; } }).m()",
        "({ __proto__: { set a(v) { f(); } }, m() { super['a'] = 1; } }).m()",
        "({ __proto__: {}, m() { return super[{ toString: function () { f(); return 'a'; } }]; } }).m()",
        "({ __proto__: { a: 1 }, m() { super[{ toString: function () { f(); return 'a'; } }] += 1; } }).m()",
    ] {
        support::stack::reach_through(through, support::stack::BUILTINS_REENTRY);
    }
}

/// `Limits::min_stack_bytes` through `super` (decision 3): on seven eighths of the promised
/// stack, a script reaches `reentry_depth` re-entries through the accessors and keys of Super
/// References and parses 1,000 nested brackets at the deepest, without a stack overflow.
#[test]
fn super_re_enters_the_interpreter_on_the_promised_stack() {
    let stack = support::host::stack_with_room(support::stack::stack_limits(
        support::stack::BUILTINS_REENTRY,
    ));
    support::stack::on_bounded_stack(
        crate::support::stack::test_path!("super_re_enters_the_interpreter_on_the_promised_stack"),
        stack,
        reach_through_super,
    )
    .unwrap();
}

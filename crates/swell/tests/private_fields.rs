//! Private fields and `#x in` (`#sec-privateelement-specification-type`, [[Kind]] field):
//! PrivateFieldAdd (`#sec-privatefieldadd`) of an instance's private fields by its class's
//! initializer, in pass 2 and in source order with the public fields, and of the static ones by
//! the runs of static fields on F; PrivateGet and PrivateSet of a field (`#sec-privateget`,
//! `#sec-privateset`) through every form of a private reference; `#x in o`
//! (`#sec-relational-operators-runtime-semantics-evaluation`, RelationalExpression :
//! PrivateIdentifier `in` ShiftExpression); and the cases where a private field meets the other
//! parts of slice 8 (a static block, a derived class, private methods, public fields). Written
//! by `story:private-fields-and-private-in`. Run also with the feature
//! `collect-every-safe-point`.
//!
//! Each script answers `true`, or the empty string when it lists what went wrong, as written and
//! with a `"use strict";` prologue (class code is strict either way).
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, agent_with, get_object, global, key, limits, object, realm};
use support::script::{Outcome, describe, eval, eval_text, parse, run, run_both};
use swell::{Agent, Outcome as HostOutcome, Parsed, Source, Termination, Value};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `source` answers `true` as written and as strict code.
fn both_pass(source: &str) {
    assert_eq!(run_both(source), (passed(), passed()), "{source}");
}

/// `source` answers the empty string, the list of what went wrong, as written and as strict
/// code; with a collection at every safe point as written only, where a script of this size
/// run twice comes near the 2 s bound of a test.
fn lists_nothing(source: &str) {
    let nothing = Outcome::Normal("\"\"".to_string());
    if cfg!(feature = "collect-every-safe-point") {
        assert_eq!(run(limits(), source), nothing, "{source}");
    } else {
        assert_eq!(run_both(source), (nothing.clone(), nothing), "{source}");
    }
}

/// The script helper `te(f)`: whether `f` throws a `TypeError`.
const TE: &str =
    "function te(f) { try { f(); } catch (e) { return e instanceof TypeError; } return false; }\n";

/// ClassFieldDefinitionEvaluation and InitializeInstanceElements (`#sec-initializeinstanceelements`,
/// step 4): each construction adds the private fields in source order with the public ones, each
/// initializer evaluated with the instance as `this` (or `undefined` without one), so a later
/// initializer reads an earlier field; PrivateGet answers the field's value and PrivateSet
/// replaces it, on `this` and on another instance; `o?.#x` and `o?.a.#x` read a field and
/// short-circuit on `null`. A direct `eval` that names a field (`this.#f`, `#f in this`) in a
/// method, a field initializer and a static block reaches the refusal of direct eval code
/// (`Agent::private_names_of_direct_eval`, until `story:private-names-in-eval-code`), and a name
/// no class declares is the early `SyntaxError` there.
#[test]
fn a_private_field_is_read_and_written() {
    both_pass(
        r##"var log = [];
class C {
  a = log.push("a");
  #x = (log.push("#x"), 1);
  b = log.push("b");
  #y;
  #z = this.#x + 1;
  get() { return [this.#x, this.#y, this.#z].join(); }
  set(v) { this.#x = v; this.#y = "y"; return this.#x; }
  static sum(a, b) { return a.#x + b.#x; }
  static chain(o) { var none = null, wrap = { a: o }; return [o?.#x, none?.#x, none?.a.#x, wrap?.a.#x].join(); }
}
var c = new C(), d = new C();
c.get() === "1,,2" && c.set(5) === 5 && c.get() === "5,y,2" && C.sum(c, d) === 6
  && log.join() === "a,#x,b,a,#x,b" && C.chain(d) === "1,,,1"
  && Object.getOwnPropertyNames(c).join() === "a,b";"##,
    );
    let script = r##"function said(e) { return (e instanceof SyntaxError ? "SyntaxError: " : "other: ") + e.message; }
function tried(f) { try { f(); } catch (e) { return said(e); } return "none"; }
var block;
class C {
  #f = 1;
  method() { return [tried(() => eval("this.#f")), tried(() => eval("#f in this")), tried(() => eval("this.#zz"))].join("|"); }
  field = [tried(() => eval("this.#f")), tried(() => eval("#f in this")), tried(() => eval("#zz in this"))].join("|");
  static { block = [tried(() => eval("this.#f")), tried(() => eval("#f in this"))].join("|"); }
}
var c = new C();
[c.method(), c.field, block].join("|");"##;
    let refused = |at: &str| {
        format!(
            "SyntaxError: private name #f in direct eval code is not supported before slice 8 ({at})"
        )
    };
    let early = |at: &str| {
        format!("SyntaxError: private name #zz is not declared in an enclosing class ({at})")
    };
    let wanted = [
        refused("1:6"),
        refused("1:1"),
        early("1:6"),
        refused("1:6"),
        refused("1:1"),
        early("1:1"),
        refused("1:6"),
        refused("1:1"),
    ]
    .join("|");
    for text in [script.to_string(), format!("\"use strict\";\n{script}")] {
        let mut agent = agent().unwrap();
        let r = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &r, &text),
            Outcome::Normal(format!("\"{wanted}\"")),
            "{text}"
        );
    }
}

/// The compound, logical and update assignments of `Target::Private` on a field
/// (`#sec-assignment-operators-runtime-semantics-evaluation`,
/// `#sec-prefix-increment-operator-runtime-semantics-evaluation` and its twins): the reference
/// is evaluated first, the field read where the form reads (PrivateGet), the right-hand side
/// after it, then PrivateSet; `&&=`, `||=` and `??=` short-circuit and write nothing; a postfix
/// form answers the old value; on a primitive base and on an object without the field the
/// form throws a `TypeError` before the right-hand side. Read by the log of the side effects of
/// the reference (`ref`) and of the right-hand side (`rhs`), with the field's value after.
#[test]
fn compound_logical_and_update_assignments_of_a_private_field() {
    lists_nothing(
        r##"var log = [], failures = [];
function rhs(v) { log.push("rhs"); return v; }
function check(name, f, expected) {
  log = [];
  try { log.push("=" + String(f())); } catch (e) { log.push(e instanceof TypeError ? "TypeError" : "other " + e); }
  if (log.join() !== expected) failures.push(name + ": " + log.join());
}
class C {
  #f = 0;
  static run(o) {
    function ref() { log.push("ref"); return o; }
    function f(v) { o.#f = v; return true; }
    function after(r) { return r + "/" + o.#f; }
    f(1); check("=", () => after(ref().#f = rhs(5)), "ref,rhs,=5/5");
    f(1); check("+=", () => after(ref().#f += rhs(2)), "ref,rhs,=3/3");
    f("a"); check("+= string", () => after(ref().#f += rhs("b")), "ref,rhs,=ab/ab");
    f(7); check("-=", () => after(ref().#f -= rhs(2)), "ref,rhs,=5/5");
    f(3); check("*=", () => after(ref().#f *= rhs(4)), "ref,rhs,=12/12");
    f(2); check("**=", () => after(ref().#f **= rhs(10)), "ref,rhs,=1024/1024");
    f(7); check("%=", () => after(ref().#f %= rhs(4)), "ref,rhs,=3/3");
    f(1); check("<<=", () => after(ref().#f <<= rhs(3)), "ref,rhs,=8/8");
    f(5); check("|=", () => after(ref().#f |= rhs(2)), "ref,rhs,=7/7");
    f(1); check("&&=", () => after(ref().#f &&= rhs(4)), "ref,rhs,=4/4");
    f(0); check("&&= short", () => after(ref().#f &&= rhs(4)), "ref,=0/0");
    f(0); check("||=", () => after(ref().#f ||= rhs(4)), "ref,rhs,=4/4");
    f(1); check("||= short", () => after(ref().#f ||= rhs(4)), "ref,=1/1");
    f(null); check("??=", () => after(ref().#f ??= rhs(4)), "ref,rhs,=4/4");
    f(0); check("??= short", () => after(ref().#f ??= rhs(4)), "ref,=0/0");
    f(1); check("++x", () => after(++ref().#f), "ref,=2/2");
    f(1); check("x++", () => after(ref().#f++), "ref,=1/2");
    f(1); check("--x", () => after(--ref().#f), "ref,=0/0");
    f(1); check("x--", () => after(ref().#f--), "ref,=1/0");
    f("2"); check("x++ string", () => after(ref().#f++), "ref,=2/3");
    return true;
  }
  static wrong(o) {
    function ref() { log.push("ref"); return o; }
    check("+= " + typeof o, () => ref().#f += rhs(1), "ref,TypeError");
    check("&&= " + typeof o, () => ref().#f &&= rhs(1), "ref,TypeError");
    check("++ " + typeof o, () => ++ref().#f, "ref,TypeError");
    check("= " + typeof o, () => ref().#f = rhs(1), "ref,rhs,TypeError");
  }
}
C.run(new C());
[1, "s", undefined, null, {}].forEach(C.wrong);
failures.join("; ");"##,
    );
}

/// A private field as the target of a destructuring assignment and of a `for`-`of` and a
/// `for`-`in` head, through `Target::Private`
/// (`#sec-runtime-semantics-iteratordestructuringassignmentevaluation`,
/// `#sec-runtime-semantics-keyeddestructuringassignmentevaluation`,
/// `#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`): an
/// element of an array pattern (the iterator closed after it), with a default, a rest element,
/// a property of an object pattern, an object rest, a `for`-`of` head (closed by `break`) and a
/// `for`-`in` head; the reference is evaluated where the clause evaluates it, the field takes
/// the value; on an object without the field each form throws a `TypeError` at PutValue.
#[test]
fn a_private_field_as_a_destructuring_and_for_of_target() {
    lists_nothing(
        r##"var log = [], failures = [];
function rhs(v) { log.push("rhs"); return v; }
function iter(v) {
  var it = { next() { log.push("next"); return { value: v, done: false }; },
             return() { log.push("return"); return {}; } };
  return { [Symbol.iterator]() { return it; } };
}
function src(v) { return { get x() { log.push("x"); return v; } }; }
function check(name, f, expected) {
  log = [];
  try { log.push("=" + String(f())); } catch (e) { log.push(e instanceof TypeError ? "TypeError" : "other " + e); }
  if (log.join() !== expected) failures.push(name + ": " + log.join());
}
class C {
  #f = 0;
  static run(o) {
    function ref() { log.push("ref"); return o; }
    function value() { var v = o.#f; return typeof v === "object" ? JSON.stringify(v) : v; }
    check("[] =", () => { [ref().#f] = iter(7); return value(); }, "ref,next,return,=7");
    check("[] = default", () => { [ref().#f = rhs(3)] = [undefined]; return value(); }, "ref,rhs,=3");
    check("[...] =", () => { [, ...ref().#f] = [1, 2, 3]; return value(); }, "ref,=[2,3]");
    check("{} =", () => { ({ x: ref().#f } = src(8)); return value(); }, "ref,x,=8");
    check("{...} =", () => { ({ y: o.y, ...ref().#f } = { y: 1, a: 2 }); return value(); }, "ref,={\"a\":2}");
    check("for of", () => { for (ref().#f of iter(9)) break; return value(); }, "next,ref,return,=9");
    check("for in", () => { for (ref().#f in { k: 1 }); return value(); }, "ref,=k");
    check("for of twice", () => { var n = 0; for (ref().#f of [4, 5]) n += o.#f; return n + "/" + value(); }, "ref,ref,=9/5");
    var other = {};
    function wrong() { log.push("ref"); return other; }
    check("[] = other", () => { [wrong().#f] = iter(7); }, "ref,next,return,TypeError");
    check("{} = other", () => { ({ x: wrong().#f } = src(8)); }, "ref,x,TypeError");
    check("for of other", () => { for (wrong().#f of iter(9)) break; }, "next,ref,return,TypeError");
    check("for in other", () => { for (wrong().#f in { k: 1 }); }, "ref,TypeError");
    return true;
  }
}
C.run(new C());
failures.join("; ");"##,
    );
}

/// The static private fields (ClassDefinitionEvaluation, step 31: the static elements in source
/// order): each run of static fields adds its private fields to F, its initializers evaluated
/// with F as `this` and able to read an earlier static field; a static block between two runs
/// reads the fields added before it and writes them, and a read of a field added after the
/// block throws a `TypeError` in the block; an instance has no static field, and a subclass
/// (whose [[Prototype]] is F) has none either: PrivateGet does not walk prototypes.
#[test]
fn static_private_fields_and_a_static_block_that_reads_them() {
    both_pass(&format!(
        r##"{TE}var log = [];
class C {{
  static #a = (log.push("#a"), 1);
  static {{ log.push("block " + C.#a); }}
  static b = C.#a + 1;
  static #c = this.#a + this.b;
  static {{ log.push("block2 " + this.#c); C.#a = 10; try {{ C.#d; }} catch (e) {{ log.push(e.constructor.name); }} }}
  static #d = "d";
  static get() {{ return [C.#a, C.b, C.#c, C.#d].join(); }}
  static read(o) {{ return o.#a; }}
}}
class E extends C {{}}
log.join() === "#a,block 1,block2 3,TypeError" && C.get() === "10,2,3,d" && C.read(C) === 10
  && te(function () {{ C.read(new C()); }}) && te(function () {{ C.read(E); }})
  && Object.getOwnPropertyNames(C).join() === "length,name,prototype,get,read,b";"##
    ));
}

/// `#x in o` (`#sec-relational-operators-runtime-semantics-evaluation`): PrivateElementFind of
/// the Private Name `#x` resolves to on `o`, for a field, a method, a getter, a setter, and the
/// static ones on F; a field not yet added (an initializer before it) is not found, a method is
/// (pass 1 adds the methods first); a `Proxy` whose target has the name answers `false` and no
/// trap runs; an instance of a second evaluation of the same class source has other Private
/// Names; inside a nested class `#x` resolves to the innermost class that declares it, the
/// outer class's name included; `#x in o` binds as tightly as `in`.
#[test]
fn private_in_for_a_field_a_method_and_an_accessor() {
    both_pass(
        r##"var traps = [];
class C {
  early = #f in this;
  earlyMethod = #m in this;
  #f = 1; #m() {} get #g() { return 1; } set #s(v) {}
  static #sf = 2; static #sm() {}
  late = #f in this;
  static test(o) { return [#f in o, #m in o, #g in o, #s in o, #sf in o, #sm in o].join(); }
  static precedence(o) { return #f in o === true && !(#f in {}) && (#f in o ? 1 : 2) === 1; }
}
var c = new C(), handler = {};
["has", "get", "getOwnPropertyDescriptor", "ownKeys", "getPrototypeOf", "set", "defineProperty"].forEach(function (t) {
  handler[t] = function () { traps.push(t); return Reflect[t].apply(null, arguments); };
});
var p = new Proxy(c, handler);
function make() { return class { #x; static has(o) { return #x in o; } }; }
var A = make(), B = make();
class O {
  #o; #x;
  static inner() { return class { #i; static has(v) { return #o in v; } static mine(v) { return #i in v; } }; }
  static shadow() { return class { #x; static has(v) { return #x in v; } }; }
}
var I = O.inner(), S = O.shadow();
C.test(c) === "true,true,true,true,false,false" && C.test(C) === "false,false,false,false,true,true"
  && C.test(p) === "false,false,false,false,false,false" && traps.length === 0
  && c.early === false && c.earlyMethod === true && c.late === true && C.precedence(c)
  && A.has(new A()) && !A.has(new B()) && B.has(new B())
  && I.has(new O()) && !I.has(new I()) && I.mine(new I()) && !S.has(new O()) && S.has(new S());"##,
    );
}

/// `#x in o`, step 4: a right operand that is no Object is a `TypeError`, for every primitive,
/// after the operand was evaluated (its side effect seen first); an error the operand's own
/// evaluation throws is that error, not a `TypeError`.
#[test]
fn private_in_on_a_primitive_throws_a_type_error() {
    both_pass(&format!(
        r##"{TE}var log = [];
class C {{
  #x;
  static has(v) {{ return #x in (log.push("rhs"), v); }}
  static thrown() {{ try {{ return #x in (function () {{ throw new RangeError("r"); }})(); }} catch (e) {{ return e.constructor.name; }} }}
}}
[1, 0, NaN, "s", "", true, false, Symbol(), undefined, null].every(function (v) {{
  return te(function () {{ C.has(v); }});
}}) && log.length === 10 && C.thrown() === "RangeError" && C.has(new C()) === true;"##
    ));
}

/// PrivateElementFind finds no element of `#x` on an object its class did not initialize, so
/// `#x in o` is `false` there: a plain object, an object whose prototype is an instance (no
/// prototype is read), an array, a function, F itself for an instance field, an instance of
/// another class with a field of the same text, a frozen object, and an object with an own
/// property named `"#x"`; the same object after a stamp answers `true`.
#[test]
fn private_in_on_an_object_without_the_name_is_false() {
    both_pass(
        r##"class C { #x = 1; static has(o) { return #x in o; } }
class D { #x = 1; static has(o) { return #x in o; } }
class Base { constructor(o) { return o; } }
class Stamp extends Base { #x; static has(o) { return #x in o; } }
var c = new C(), o = {};
var none = [{}, Object.create(c), [], function () {}, C, new D(), Object.freeze({}), { "#x": 1 }, o];
var answers = none.map(C.has);
new Stamp(o);
answers.every(function (a) { return a === false; }) && Stamp.has(o) && !C.has(o) && C.has(c) && !Stamp.has(c);"##,
    );
}

/// PrivateFieldAdd, step 3 (`#sec-privatefieldadd`): a base constructor that returns another
/// object makes the derived class's fields land on that object (the stamp); a second
/// construction through the same object evaluates the first field's initializer and then
/// throws a `TypeError` at its add, so no later initializer runs and the fields keep their
/// values; with a private method in the class the method's add (pass 1) throws first and no
/// initializer runs; objects of other kinds (an array, a function, a `Proxy`, whose traps see
/// nothing) take the stamp alike.
#[test]
fn a_second_add_through_a_returned_object_throws_a_type_error() {
    both_pass(&format!(
        r##"{TE}var log = [], traps = [];
class Base {{ constructor(o) {{ return o; }} }}
class Two extends Base {{
  #a = (log.push("a"), 42);
  #b = (log.push("b"), "b");
  static read(o) {{ return o.#a + o.#b; }}
  static write(o, v) {{ o.#a = v; }}
}}
class Both extends Base {{ #m() {{ return "m"; }} #c = (log.push("c"), "c"); static read(o) {{ return o.#m() + o.#c; }} }}
class Fields extends Base {{ #f = (log.push("f"), 1); static read(o) {{ return o.#f; }} }}
var o = {{}};
new Two(o);
var first = Two.read(o) === "42b" && log.join() === "a,b";
log = [];
var again = te(function () {{ new Two(o); }}) && log.join() === "a" && Two.read(o) === "42b";
Two.write(o, 1);
log = [];
new Both(o);
var both = Both.read(o) === "mc" && te(function () {{ new Both(o); }}) && log.join() === "c";
log = [];
var fields = te(function () {{ new Fields(o); new Fields(o); }}) && log.join() === "f,f" && Fields.read(o) === 1;
var handler = {{}};
["defineProperty", "set", "get", "has", "getOwnPropertyDescriptor", "preventExtensions", "isExtensible"].forEach(function (t) {{
  handler[t] = function () {{ traps.push(t); return Reflect[t].apply(null, arguments); }};
}});
var kinds = [[], function () {{}}, new Proxy({{}}, handler)];
var stamped = kinds.every(function (k) {{ new Fields(k); return Fields.read(k) === 1 && te(function () {{ new Fields(k); }}); }});
first && again && Two.read(o) === "1b" && both && fields && stamped && traps.length === 0;"##
    ));
}

/// PrivateFieldAdd does not read [[Extensible]] (`#sec-privatefieldadd`; the feature
/// `nonextensible-applies-to-private` is excluded): a field is added to a frozen, a sealed and a
/// non-extensible object through a returned object, and to an instance that the base
/// constructor made non-extensible before the derived class's fields were added; PrivateSet of
/// a field on a frozen object writes it; the object stays frozen and lists nothing new.
#[test]
fn a_private_field_is_added_to_a_frozen_and_a_non_extensible_object() {
    both_pass(
        r##"class Base { constructor(o) { return o; } }
class Stamp extends Base { #f = 1; static read(o) { return o.#f; } static write(o, v) { o.#f = v; return o.#f; } }
var frozen = Object.freeze({ a: 1 }), sealed = Object.seal([]), closed = Object.preventExtensions({});
[frozen, sealed, closed].forEach(function (o) { new Stamp(o); });
class Closed { constructor() { Object.freeze(this); } }
class Late extends Closed { #g = 2; static read(o) { return o.#g; } }
var late = new Late();
Stamp.read(frozen) === 1 && Stamp.read(sealed) === 1 && Stamp.read(closed) === 1
  && Stamp.write(frozen, 5) === 5 && Stamp.read(frozen) === 5
  && Object.isFrozen(frozen) && Object.isSealed(sealed) && !Object.isExtensible(closed)
  && Object.getOwnPropertyNames(frozen).join() === "a" && Late.read(late) === 2 && Object.isFrozen(late);"##,
    );
}

/// "Private elements are no properties": no listing of a script sees a private field
/// (`Object.getOwnPropertyNames`, `Object.getOwnPropertySymbols`, `Reflect.ownKeys`,
/// `Object.keys`, `Object.entries`, `Object.getOwnPropertyDescriptors`, `JSON.stringify`,
/// `for`-`in`, object spread), `Object.assign` and spread copy none, and the static fields are
/// not properties of F. The host interface has no listing of an object's keys: its view is
/// `has_property` and `get_property` of the name `#f`, which find nothing.
#[test]
fn private_fields_are_invisible_to_listings() {
    let source = r##"class C {
  #f = 1; x = 2; static #s = 3;
  static has(o) { return #f in o; }
  static read(o) { return o.#f; }
}
var c = new C();
var copy = Object.assign({}, c), spread = { ...c }, keys = [];
for (var k in c) keys.push(k);
Object.getOwnPropertyNames(c).join() === "x" && Object.getOwnPropertySymbols(c).length === 0
  && Reflect.ownKeys(c).join() === "x" && Object.keys(c).join() === "x"
  && JSON.stringify(Object.entries(c)) === '[["x",2]]'
  && Object.keys(Object.getOwnPropertyDescriptors(c)).join() === "x"
  && JSON.stringify(c) === '{"x":2}' && keys.join() === "x"
  && !C.has(copy) && !C.has(spread) && Object.keys(spread).join() === "x" && C.read(c) === 1
  && Object.getOwnPropertyNames(C).join() === "length,name,prototype,has,read";"##;
    both_pass(source);
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(eval_text(&mut agent, &realm, source), passed());
    let global_root = global(&mut agent, &realm);
    let (has, value) = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let c = get_object(cx, global, "c");
            let name = key(cx, "#f");
            let has = cx.has_property(c, name).unwrap().unwrap();
            let value = cx.get_property(c, name).unwrap().unwrap();
            (has, value)
        })
        .unwrap();
    assert!(!has);
    assert_eq!(value, Value::Undefined);
}

/// Whether the variable `SWELL_FULL_COUNTS` asks for the full counts.
fn full_counts() -> bool {
    std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

/// How many constructions each round of [`instances_with_100_private_fields_are_charged_and_freed`]
/// makes (two rounds): 5,000 under `SWELL_FULL_COUNTS=1` and 500 in the gate; with a collection
/// at every safe point, where each of the about 200 instructions of a construction marks the
/// whole heap (about 0.09 CPU s a construction), 8 under `SWELL_FULL_COUNTS=1` and 2 in the
/// gate. The story names 10,000 and 1,000: 5,000 took 1.97 CPU s alone in the default build and
/// 500 took 0.49 s (about 0.16 ms a construction, so 10,000 would take about 3.6 s, over the
/// 2.5 s of a test at the full count), and 8 took 1.49 s with the collections and 2 took 0.43 s,
/// so 1,000 would take far over 60 s. Each construction
/// runs the same instructions and the reading after the round must equal the one before it to
/// the byte, so a construction whose list is charged and not credited fails at every count, 2
/// included.
fn constructions() -> u32 {
    match (cfg!(feature = "collect-every-safe-point"), full_counts()) {
        (false, true) => 5_000,
        (false, false) => 500,
        (true, true) => 8,
        (true, false) => 2,
    }
}

/// The source of a class expression with `n` private fields `#f0 = 0` to `#f<n-1> = <n-1>`, a
/// method `last()` that reads the last and `first()` that reads the first.
fn class_with_private_fields(n: u32) -> String {
    let fields: String = (0..n).map(|i| format!("#f{i} = {i}; ")).collect();
    let last = n.saturating_sub(1);
    format!("class {{ {fields}last() {{ return this.#f{last}; }} first() {{ return this.#f0; }} }}")
}

/// The rise of `heap_bytes()` with `n` objects of `make()` held in an array, from the agent
/// without them to the agent holding them, both after a collection; the first of two rounds
/// grows the arenas. Two setups' rises differ by what their objects differ in, the array and
/// the arenas' growth cancelling out.
fn rise_of(agent: &mut Agent<TestHost>, r: &swell::Realm, setup: &str, n: u64) -> u64 {
    assert_eq!(
        eval_text(agent, r, &format!("{setup} var keep = null; 0;")),
        Outcome::Normal("0".to_string()),
        "{setup}"
    );
    let fill = format!("keep = []; for (var i = 0; i < {n}; i++) keep.push(make()); 0;");
    let mut rise = 0;
    for _ in 0..2 {
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        let empty = agent.heap_bytes().unwrap();
        assert_eq!(eval_text(agent, r, &fill), Outcome::Normal("0".to_string()));
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        rise = agent.heap_bytes().unwrap() - empty;
        assert_eq!(
            eval_text(agent, r, "keep = null; 0;"),
            Outcome::Normal("0".to_string())
        );
    }
    rise
}

/// The box of an object's list of private elements, `size_of::<PrivateElements>()`, a `Vec`
/// (pointer, capacity and length).
const BOX_BYTES: u64 = 24;

/// One entry of the list, `size_of::<PrivateElement>()`: the Private Name's handle (12 bytes) and
/// a `PrivateValue`, whose `Field` holds a `Value` (16 bytes, aligned to 8), padded to 8.
const ENTRY_BYTES: u64 = 48;

/// "Every allocation is charged before it exists" (decision 4, "Memory budget"): a loop of
/// [`constructions`] constructions of a class with 100 private fields, each instance read, leaves,
/// once the script dropped what it held, the reading of `heap_bytes()` after a `collect()` it had
/// before the loop (taken after a first round, which grows the arenas): the list and its box are
/// credited by the sweep as they were charged. The room of the list, in the default build (200
/// objects a reading): an instance with f private fields costs [`BOX_BYTES`] + f
/// [`ENTRY_BYTES`] over a plain one for f = 1, 4, 17 and 100, so the first add of a level
/// reserves room for all of its elements once; a level of 3 methods and 2 fields, a base of 2
/// fields under a derived level of 3, and an object stamped by a class of 2 and one of 3
/// fields through return overrides each cost what one level of 5 fields does. A loop that keeps
/// instances under a small budget ends in `Terminated(MemoryBudget)`, and leaks nothing.
#[test]
fn instances_with_100_private_fields_are_charged_and_freed() {
    let mut rounds = limits();
    rounds.steps_per_entry = 50_000_000;
    let mut agent = agent_with(rounds).unwrap();
    let r = realm(&mut agent);
    let class = class_with_private_fields(100);
    assert_eq!(
        eval_text(
            &mut agent,
            &r,
            &format!("var i = 0, o = null, C = {class}; 0;")
        ),
        Outcome::Normal("0".to_string())
    );
    let count = constructions();
    let looped = parse(
        &mut agent,
        &r,
        &format!(
            "for (i = 0; i < {count}; i++) {{ o = new C(); if (o.last() !== 99) throw 0; }} i;"
        ),
    );
    let dropped = parse(&mut agent, &r, "o = null; 0;");
    let round = |agent: &mut Agent<TestHost>| {
        let outcome = agent.evaluate_script(&looped).unwrap();
        assert_eq!(
            describe(agent, &r, outcome),
            Outcome::Normal(count.to_string())
        );
        let outcome = agent.evaluate_script(&dropped).unwrap();
        assert_eq!(
            describe(agent, &r, outcome),
            Outcome::Normal("0".to_string())
        );
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        agent.heap_bytes().unwrap()
    };
    let before = round(&mut agent);
    let after = round(&mut agent);
    println!(
        "heap_bytes: {before} before the loop, {after} after {count} constructions and a collection"
    );
    assert_eq!(after, before);
    // The room and the memory budget: in the default build only, where a construction of 100
    // fields is cheap; with a collection at every safe point each of its instructions marks the
    // heap.
    if cfg!(feature = "collect-every-safe-point") {
        return;
    }
    let n = 200;
    let plain = rise_of(
        &mut agent,
        &r,
        "var K = class {}; var make = function () { return new K(); };",
        n,
    );
    // The bytes per object over a plain instance.
    let over = |agent: &mut Agent<TestHost>, setup: &str| {
        let rise = rise_of(agent, &r, setup, n) - plain;
        assert_eq!(rise % n, 0, "{setup}: {rise} over {n}");
        rise / n
    };
    let fields = |f: u32| {
        format!(
            "var K = {}; var make = function () {{ return new K(); }};",
            class_with_private_fields(f)
        )
    };
    let mut table = Vec::new();
    for f in [1u32, 4, 17, 100] {
        table.push((f, over(&mut agent, &fields(f))));
    }
    let five = over(&mut agent, &fields(5));
    let shapes = [
        (
            "3 methods and 2 fields",
            "var K = class { #a() {} #b() {} #c() {} #d = 1; #e = 2; }; var make = function () { return new K(); };",
        ),
        (
            "2 fields under 3",
            "var make = (function () { class A { #a; #b; } class B extends A { #c; #d; #e; } return function () { return new B(); }; })();",
        ),
        (
            "stamps of 2 and 3",
            "var make = (function () { class Base { constructor(o) { return o; } }\n\
             class S2 extends Base { #a; #b; } class S3 extends Base { #c; #d; #e; }\n\
             return function () { var o = new K(); new S2(o); new S3(o); return o; }; })(); var K = class {};",
        ),
    ];
    let mut found = Vec::new();
    for (name, setup) in shapes {
        found.push((name, over(&mut agent, setup)));
    }
    println!(
        "bytes per instance over a plain one, by private fields: {table:?}; \
         5 fields {five}, shapes {found:?}"
    );
    for (f, bytes) in &table {
        assert_eq!(
            *bytes,
            BOX_BYTES + u64::from(*f) * ENTRY_BYTES,
            "{f} fields: {table:?}"
        );
    }
    for (name, per) in found {
        assert_eq!(per, five, "{name}: {per} against 5 fields {five}");
    }
    // The memory budget: 256 KiB above what a fresh agent and realm hold.
    let fresh = {
        let mut agent = support::host::agent().unwrap();
        let _realm = support::host::realm(&mut agent);
        agent.heap_bytes().unwrap()
    };
    let mut small = limits();
    small.memory_bytes = fresh + 256 * 1024;
    let mut agent = agent_with(small).unwrap();
    let small_realm = support::host::realm(&mut agent);
    let kept = parse(
        &mut agent,
        &small_realm,
        &format!(
            "(function () {{ var K = {}; var keep = []; for (;;) keep.push(new K()); }})();",
            class_with_private_fields(100)
        ),
    );
    let mut readings = Vec::new();
    for _ in 0..3 {
        let outcome = agent.evaluate_script(&kept).unwrap();
        assert!(
            matches!(outcome, HostOutcome::Terminated(Termination::MemoryBudget)),
            "{outcome:?}"
        );
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        readings.push(agent.heap_bytes().unwrap());
    }
    println!("heap_bytes after each memory budget and a collection: {readings:?}");
    assert_eq!(readings[1], readings[2]);
}

/// How many objects each round of [`a_private_field_survives_a_collection_between_add_and_read`]
/// makes and drops: 50, and 5 with a collection at every safe point (for the time): garbage
/// whose records a collection that freed what a field holds would hand out again.
const GARBAGE: u32 = if cfg!(feature = "collect-every-safe-point") {
    5
} else {
    50
};

/// A field's value is reached through the object alone (design, "Private names", item 6: each
/// entry of [[PrivateElements]] marks its values at the object record's site): an instance
/// field and a static field whose values are the only references to their objects keep them
/// across the host's collections and the garbage made after them, and, with a collection at
/// every safe point, across the instructions between the add and the read in one script. Unlike
/// a method, no private record holds a field's value, so a list whose field values are not
/// traced fails here.
#[test]
fn a_private_field_survives_a_collection_between_add_and_read() {
    let mut agent = agent().unwrap();
    let r = realm(&mut agent);
    let garbage = format!(
        "var garbage = []; for (var j = 0; j < {GARBAGE}; j++) garbage.push({{ v: j }}, [j]); garbage = null;"
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &r,
            &format!(
                "var o = (function () {{\n\
                   class C {{ #f = {{ v: 42 }}; static #s = [7]; read() {{ return this.#f.v; }} static read() {{ return C.#s[0]; }} }}\n\
                   var c = new C(); {garbage} if (c.read() !== 42) throw 0;\n\
                   return [c, C];\n\
                 }})(); 0;"
            )
        ),
        Outcome::Normal("0".to_string())
    );
    for _ in 0..3 {
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        assert_eq!(
            eval_text(
                &mut agent,
                &r,
                &format!("{garbage} o[0].read() + o[1].read();")
            ),
            Outcome::Normal("49".to_string())
        );
    }
}

/// How many steps an entry of [`a_private_increment_loop_ends_at_the_step_budget`] may spend
/// in its loop: 100,000, and 1,000 with a collection at every safe point, where every step is an
/// instruction that marks the heap (5,000 took 3.6 CPU s alone there with the step readings,
/// over the 2 s bound). A loop that does not end at the step budget fails at either.
const LOOP_STEPS: u64 = if cfg!(feature = "collect-every-safe-point") {
    1_000
} else {
    100_000
};

/// The steps a construction of 100 private fields spends past its instructions: its adds find
/// each key first (PrivateFieldAdd, step 2), comparing with every entry so far, a step per
/// comparison past `SMALL` (8): the sum over k = 1..100 of max(0, k - 1 - 8), (100 - 9)(100 - 8) / 2.
const ADD_STEPS_100: u64 = 4_186;

/// The steps a read of the 100th field spends past a read of the first: it compares 100
/// entries, 100 - 8 past `SMALL`.
const READ_STEPS_100: u64 = 92;

/// The step budget reaches a loop of private field updates and `#x in` checks
/// (`Termination::StepBudget`), as written and in strict code; the agent runs again after it.
/// And the step charge of PrivateElementFind on fields: with 1, 7, 8 and 100 private fields, the
/// steps of one construction beyond the instructions of its fields are 0 up to 9 fields and
/// [`ADD_STEPS_100`] at 100, and one read of the last of 100 fields spends [`READ_STEPS_100`]
/// more than a read of the first, as `last_entry_steps()` reads them.
#[test]
fn a_private_increment_loop_ends_at_the_step_budget() {
    let mut small = limits();
    small.steps_per_entry = LOOP_STEPS;
    let source = "class C { #i = 0; #j; run() { for (;;) { this.#i++; this.#j = #i in this; } } }\n\
                  new C().run();";
    for text in [source.to_string(), format!("\"use strict\";\n{source}")] {
        let mut agent = agent_with(small).unwrap();
        let r = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &r, &text),
            Outcome::Terminated(Termination::StepBudget),
            "{text}"
        );
        assert_eq!(
            eval_text(&mut agent, &r, "1 + 1;"),
            Outcome::Normal("2".to_string())
        );
    }
    let mut agent = agent().unwrap();
    let r = realm(&mut agent);
    let mut steps = std::collections::BTreeMap::new();
    for n in [1u64, 7, 8, 100] {
        let class = class_with_private_fields(u32::try_from(n).unwrap());
        assert_eq!(
            eval_text(&mut agent, &r, &format!("var K = {class}; 0;")),
            Outcome::Normal("0".to_string())
        );
        let outcome = eval(&mut agent, &r, "new K(); 0;");
        assert!(matches!(outcome, HostOutcome::Normal(_)), "{n}");
        steps.insert(n, agent.last_entry_steps().unwrap());
    }
    let per = steps[&8] - steps[&7];
    let extra = steps[&100] - steps[&1] - 99 * per;
    assert_eq!(
        eval_text(&mut agent, &r, "var o = new K(); 0;"),
        Outcome::Normal("0".to_string())
    );
    let mut read = |method: &str| {
        let outcome = eval(&mut agent, &r, &format!("o.{method}(); 0;"));
        assert!(matches!(outcome, HostOutcome::Normal(_)), "{method}");
        agent.last_entry_steps().unwrap()
    };
    let (first, last) = (read("first"), read("last"));
    println!(
        "steps of `new K(); 0;` by private fields: {steps:?}, {per} a field, {extra} past them at 100; \
         `o.first(); 0;` {first}, `o.last(); 0;` {last}"
    );
    assert_eq!(steps[&7] - steps[&1], 6 * per, "{steps:?}");
    assert_eq!(extra, ADD_STEPS_100, "{steps:?}");
    assert_eq!(last - first, READ_STEPS_100);
}

/// The private fields of a derived class are added by InitializeInstanceElements after
/// `super()` returns (`#sec-super-keyword-runtime-semantics-evaluation`, SuperCall, steps 9 and
/// 10): the parent's constructor finds no field on the new object (PrivateGet throws a
/// `TypeError`, `#x in` answers `false`); after `super()` the derived constructor reads the
/// value; a second `super()` runs the parent again and throws a `ReferenceError` at
/// BindThisValue before any second add (the initializer is not evaluated again); the default
/// derived constructor and a `super()` in an arrow add the fields the same way; three levels
/// add their fields level by level, each initializer seeing the levels below.
#[test]
fn private_fields_of_a_derived_class_are_added_after_super() {
    both_pass(
        r##"var log = [];
class A {
  constructor() {
    try { D.read(this); log.push("read"); } catch (e) { log.push(e.constructor.name); }
    log.push("in " + D.has(this));
  }
}
class D extends A {
  #f = (log.push("init"), 1);
  static read(o) { return o.#f; }
  static has(o) { return #f in o; }
  constructor() {
    super();
    log.push("after " + D.read(this));
    try { super(); } catch (e) { log.push(e.constructor.name + " " + D.read(this)); }
  }
}
class Default extends A { #g = 2; static read(o) { return o.#g; } }
class Arrow extends A { #h = 3; constructor() { var s = () => super(); s(); } static read(o) { return o.#h; } }
new D();
var derived = log.join() === "TypeError,in false,init,after 1,TypeError,in false,ReferenceError 1";
class L1 { #a = 1; static a(o) { return o.#a; } }
class L2 extends L1 { #b = L1.a(this) + 1; static b(o) { return o.#b; } }
class L3 extends L2 { #c = L2.b(this) + 1; static c(o) { return o.#c; } }
var l = new L3();
derived && Default.read(new Default()) === 2 && Arrow.read(new Arrow()) === 3
  && L1.a(l) + L2.b(l) + L3.c(l) === 6;"##,
    );
}

/// `#x in o` charges its find as a read does (PrivateElementFind, a step per comparison past
/// `SMALL`): on an object of 100 private fields, `#f99 in o` compares all 100 entries and `#f0 in
/// o` one, so the first spends 100 - 8 = [`READ_STEPS_100`] steps more; both run the same
/// instructions.
#[test]
fn private_in_charges_its_find_as_a_read_does() {
    let mut agent = agent().unwrap();
    let r = realm(&mut agent);
    let fields: String = (0..100).map(|i| format!("#f{i}; ")).collect();
    assert_eq!(
        eval_text(
            &mut agent,
            &r,
            &format!(
                "var K = class {{ {fields}static first(o) {{ return #f0 in o; }} static last(o) {{ return #f99 in o; }} }}; var o = new K(); 0;"
            )
        ),
        Outcome::Normal("0".to_string())
    );
    let mut steps = |call: &str| {
        let outcome = eval(&mut agent, &r, &format!("K.{call}(o); 0;"));
        assert!(matches!(outcome, HostOutcome::Normal(_)), "{call}");
        agent.last_entry_steps().unwrap()
    };
    let (first, last) = (steps("first"), steps("last"));
    println!(
        "steps of `K.first(o); 0;` ({{#f0 in o}}) {first}, of `K.last(o); 0;` ({{#f99 in o}}) {last}"
    );
    assert_eq!(last - first, READ_STEPS_100);
}

/// The room of the instance level counts the instance's private elements only
/// (`Resolution::private_rooms`): an instance of a class with one private field and three static
/// private fields costs one entry and the box over a plain instance, as one with the private
/// field alone does; F's static fields do not widen the instance's list. In the default build
/// (200 objects a reading, as [`instances_with_100_private_fields_are_charged_and_freed`]).
#[test]
fn the_instance_room_leaves_out_the_static_fields() {
    if cfg!(feature = "collect-every-safe-point") {
        return;
    }
    let mut agent = agent().unwrap();
    let r = realm(&mut agent);
    let n = 200;
    let plain = rise_of(
        &mut agent,
        &r,
        "var K = class {}; var make = function () { return new K(); };",
        n,
    );
    let mixed = rise_of(
        &mut agent,
        &r,
        "var K = class { #a = 1; static #s = 1; static #t = 2; static #u = 3; }; var make = function () { return new K(); };",
        n,
    );
    println!(
        "heap rise of {n} instances: plain {plain}, one private field and three static {mixed}"
    );
    assert_eq!((mixed - plain) % n, 0);
    assert_eq!((mixed - plain) / n, BOX_BYTES + ENTRY_BYTES);
}

/// The static private fields of a class are one level of F's own list, reserved once by the
/// first add of the level (`AddPrivateField` of a static field carries the room of F's level,
/// keyed by the constructor). The list's bytes are the box and three entries whether it grows
/// once or at each add, so the test reads where a memory budget ends the run instead: each
/// initializer calls `Date.now()`, which the test host counts with no charge to the heap, and the
/// ballast made first puts the class past the peak of the script's parse. Then the run either
/// ends before the level's first add is done, after one initializer, or gets past all three
/// adds: the smallest budget with which two initializers run lets all three run, and the budget
/// one byte lower lets one run. A level that grew one entry at each add, its second add charged,
/// would end there after two.
#[test]
fn the_static_fields_of_a_class_reserve_their_level_once() {
    let source = "var ballast = new Array(4096).fill(0);\n\
                  var C = class { static #s = Date.now(); static #t = Date.now(); static #u = Date.now(); };\n\
                  0;";
    let parsed = {
        let mut agent = agent().unwrap();
        let r = realm(&mut agent);
        let _script = parse(&mut agent, &r, source);
        agent.heap_bytes().unwrap()
    };
    // The initializers that ran and the bytes held at the end, under `budget`.
    let run_at = |budget: u64| {
        let mut room = limits();
        room.memory_bytes = budget;
        let mut agent = agent_with(room).unwrap();
        let r = realm(&mut agent);
        if let Parsed::Parsed(script) = agent.parse_script(&r, Source::Utf8(source), None).unwrap()
        {
            let outcome = agent.evaluate_script(&script).unwrap();
            assert!(
                matches!(
                    outcome,
                    HostOutcome::Normal(_) | HostOutcome::Terminated(Termination::MemoryBudget)
                ),
                "{budget}: {outcome:?}"
            );
        }
        (
            agent.host().clock_globals.len(),
            agent.heap_bytes().unwrap(),
        )
    };
    // The smallest budget with which two initializers run, by halving.
    let (mut short, mut enough) = (parsed, parsed + 256 * 1024);
    assert_eq!(run_at(enough).0, 3);
    while enough - short > 1 {
        let mid = short + (enough - short) / 2;
        if run_at(mid).0 >= 2 {
            enough = mid;
        } else {
            short = mid;
        }
    }
    let (ran, _) = run_at(enough);
    let (ran_below, held) = run_at(enough - 1);
    println!(
        "from a budget of {enough} bytes {ran} static initializers run; at {} {ran_below} run, \
         the run holding {held}",
        enough - 1
    );
    assert_eq!((ran_below, ran), (1, 3));
}

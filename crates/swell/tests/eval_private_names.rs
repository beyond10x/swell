//! Private names in direct eval code (PerformEval, `#sec-performeval`): the eval code is parsed
//! with the names of the running PrivateEnvironment and its outer ones (privateIdentifiers, for
//! AllPrivateIdentifiersValid), and `#x`, `o.#x`, `#x in o` and a private method's call in it
//! evaluate against the Private Names of the classes around the call, as other code does: in
//! methods, constructors, field initializers, static blocks, the arrows and functions in them,
//! computed keys, default parameters and inner classes, each evaluation of a class seeing its own
//! names. An indirect eval and `Function` code have no private names. The walk that lists the
//! names (`Agent::eval_private_names`, `docs/design/engine.md`, "Private names", item 4) spends a
//! step per record and lists texts and positions only. Written by
//! `story:private-names-in-eval-code`. Run also with the feature `collect-every-safe-point`.
//!
//! Each script lists what went wrong and answers the empty string, as written and with a
//! `"use strict";` prologue (class code is strict either way).
#![cfg(test)]

use crate::support;

use support::host::{agent, agent_with, limits, realm};
use support::script::{Outcome, eval_text, parse, run_both};
use swell::{Outcome as HostOutcome, Termination};

/// What a script that lists what went wrong answers when nothing did.
fn nothing() -> Outcome {
    Outcome::Normal("\"\"".to_string())
}

/// `source` answers the empty string as written and as strict code.
fn lists_nothing(source: &str) {
    assert_eq!(run_both(source), (nothing(), nothing()), "{source}");
}

/// The script helper `syntax(f)`: the message of the `SyntaxError` `f()` throws, `"other: "` and
/// the error for another throw, `"none"` when it throws nothing.
const SYNTAX: &str = "function syntax(f) { try { f(); } catch (e) { return e instanceof SyntaxError ? e.message : \"other: \" + e; } return \"none\"; }\n";

/// The message AllPrivateIdentifiersValid gives for `#x`, without its position.
fn undeclared(name: &str) -> String {
    format!("private name {name} is not declared in an enclosing class (")
}

/// `#sec-performeval` with a method as the caller: `this.#f`, `this.#m()` and `o.#f` of another
/// instance in eval code; through a block whose `let` binding lives in a record (a hop more), a
/// nested function declaration, and a default parameter's initializer; an object without the
/// field is the `TypeError` of PrivateGet.
#[test]
fn eval_reads_a_private_field_in_a_method() {
    lists_nothing(
        r##"var wrong = [];
class C {
  #f = 1;
  #m() { return 2; }
  read() { return eval("this.#f"); }
  call() { return eval("this.#m()"); }
  other(o) { return eval("o.#f"); }
  block() { var o = this; { let b = 3; return eval("o.#f + b"); } }
  nested() { function f(o) { return eval("o.#f + 10"); } return f(this); }
  param(o, v = eval("o.#f + 20")) { return v; }
}
var c = new C();
if (c.read() !== 1) wrong.push("read " + c.read());
if (c.call() !== 2) wrong.push("call");
if (c.other(new C()) !== 1) wrong.push("other");
if (c.block() !== 4) wrong.push("block");
if (c.nested() !== 11) wrong.push("nested function");
if (c.param(c) !== 21) wrong.push("default parameter");
try { c.other({}); wrong.push("no TypeError"); } catch (e) { if (!(e instanceof TypeError)) wrong.push("other error " + e); }
wrong.join();"##,
    );
}

/// The constructor as the caller, of a base class and of a derived class after `super()`
/// (inDerivedConstructor: `this` in the eval code is looked up by name).
#[test]
fn eval_reads_a_private_field_in_a_constructor() {
    lists_nothing(
        r##"var wrong = [];
class C { #f; constructor(v) { this.#f = v; this.seen = eval("this.#f * 2"); } }
class B {}
class D extends B { #g = 5; constructor() { super(); this.seen = eval("this.#g + 1"); } }
if (new C(4).seen !== 8) wrong.push("base");
if (new D().seen !== 6) wrong.push("derived");
wrong.join();"##,
    );
}

/// A field initializer as the caller (inClassFieldInitializer set): an instance field reads a
/// field added before it, a static field a static one, and an arrow in an initializer passes
/// the names on.
#[test]
fn eval_reads_a_private_field_in_a_field_initializer() {
    lists_nothing(
        r##"var wrong = [];
class C {
  #f = 7;
  g = eval("this.#f + 1");
  h = (() => eval("this.#f"))();
  static #s = 2;
  static t = eval("this.#s * 3");
}
var c = new C();
if (c.g !== 8) wrong.push("instance");
if (c.h !== 7) wrong.push("arrow");
if (C.t !== 6) wrong.push("static");
wrong.join();"##,
    );
}

/// A static block as the caller: a static field and a static method, read and called with F as
/// `this`.
#[test]
fn eval_reads_a_private_field_in_a_static_block() {
    lists_nothing(
        r##"var wrong = [];
class C {
  static #s = 3;
  static #m() { return 4; }
  static { this.r = eval("this.#s + this.#m()"); }
}
if (C.r !== 7) wrong.push("block " + C.r);
wrong.join();"##,
    );
}

/// An arrow function, and an arrow in an arrow, in a method: the eval in it sees the names of
/// the class around the arrow.
#[test]
fn an_arrow_in_a_method_passes_the_names_to_its_eval() {
    lists_nothing(
        r##"var wrong = [];
class C {
  #f = 5;
  m() { return (() => eval("this.#f"))(); }
  n() { var a = () => () => eval("#f in this"); return a()(); }
}
var c = new C();
if (c.m() !== 5) wrong.push("arrow");
if (c.n() !== true) wrong.push("arrow in an arrow");
wrong.join();"##,
    );
}

/// The case the rejected design missed (`docs/design/engine.md`, "Private names", item 4): an
/// arrow in a computed key of an inner class takes its `this` from the outer method, but its
/// PrivateEnvironment is the inner class's, so `#x` there is the inner class's name, which the
/// outer instance does not have.
#[test]
fn an_arrow_in_an_inner_computed_key_sees_the_inner_names() {
    lists_nothing(
        r##"var wrong = [];
class O {
  #x = "outer";
  m() {
    var seen = [];
    class I {
      #x = "inner";
      static probe(o) { return o.#x; }
      [(() => { seen.push(eval("#x in this"), typeof this); return "k"; })()]() {}
    }
    return [seen.join(), eval("#x in this"), I.probe(new I())].join("|");
  }
}
var answer = new O().m();
if (answer !== "false,object|true|inner") wrong.push(answer);
wrong.join();"##,
    );
}

/// `#x in o` in eval code, for a field, a method, an accessor and a static field; a primitive
/// right operand is the `TypeError`.
#[test]
fn eval_of_private_in() {
    lists_nothing(
        r##"var wrong = [];
class C {
  #f; #m() {} get #a() { return 1; } static #s;
  has(o) { return [eval("#f in o"), eval("#m in o"), eval("#a in o"), eval("#s in o")].join(); }
  static sh(o) { return eval("#s in o"); }
  prim() { return eval("#f in 1"); }
}
var c = new C();
if (c.has(c) !== "true,true,true,false") wrong.push("instance " + c.has(c));
if (c.has({}) !== "false,false,false,false") wrong.push("object");
if (C.sh(C) !== true || C.sh(c) !== false) wrong.push("static");
try { c.prim(); wrong.push("no TypeError"); } catch (e) { if (!(e instanceof TypeError)) wrong.push("other " + e); }
wrong.join();"##,
    );
}

/// Every write form on a private field in eval code (plain, compound, update, destructuring,
/// logical), the call of a private method with its `this`, an accessor's setter and getter, and
/// the `TypeError` of a write to a method.
#[test]
fn eval_writes_a_private_field_and_calls_a_private_method_and_accessor() {
    lists_nothing(
        r##"var wrong = [];
class C {
  #f = 1; #v = 0;
  #m(a) { return this.#f + a; }
  get #a() { return this.#v; } set #a(v) { this.#v = v * 10; }
  run() {
    var r = [];
    r.push(eval("this.#f = 5"));
    r.push(eval("this.#f += 2, this.#f"));
    r.push(eval("this.#f++"));
    r.push(eval("this.#m(1)"));
    r.push(eval("this.#a = 3, this.#a"));
    r.push(eval("[this.#f] = [11], this.#f"));
    r.push(eval("this.#f ??= 0"));
    try { eval("this.#m = 1"); r.push("no throw"); } catch (e) { r.push(e instanceof TypeError); }
    return r.join();
  }
}
var answer = new C().run();
if (answer !== "5,7,7,9,30,11,11,true") wrong.push(answer);
wrong.join();"##,
    );
}

/// The names of every class around the call: a method of an inner class reads the outer
/// class's field and calls its static method; eval inside eval code sees the names the outer
/// eval saw, and those of a class the outer eval code declares; a class declared in eval code
/// runs a direct eval in its method after the outer eval returned.
#[test]
fn an_inner_class_eval_reads_an_outer_classes_name() {
    lists_nothing(
        r##"var wrong = [];
class O {
  #o = "o";
  static #s() { return "s"; }
  static make() { return class { #i = "i"; m(x) { return eval("x.#o + O.#s() + this.#i"); } }; }
  outer() { return eval("var K = class { #k = 'k'; r(x) { return eval('x.#o + this.#k'); } }; eval('this.#o') + new K().r(this)"); }
  later() { return eval("(class { #k = 'k2'; r(x) { return eval('x.#o + this.#k'); } })"); }
}
var I = O.make(), o = new O();
if (new I().m(o) !== "osi") wrong.push("inner " + new I().m(o));
if (o.outer() !== "ook") wrong.push("eval in eval " + o.outer());
var K = o.later();
if (new K().r(o) !== "ok2") wrong.push("later");
wrong.join();"##,
    );
}

/// An inner class's name hides an outer one of the same text, for eval code in the inner class
/// and for a class the eval code declares.
#[test]
fn an_inner_name_hides_an_outer_name_of_the_same_text() {
    lists_nothing(
        r##"var wrong = [];
class O {
  #x = "outer";
  static inner(o) { return new (class { #x = "inner"; m(p) { return [eval("this.#x"), eval("#x in p")].join(); } })().m(o); }
  own() { return eval("new (class { #x = 'own'; r(o) { return [this.#x, #x in o].join(); } })().r(this)"); }
}
var o = new O();
if (O.inner(o) !== "inner,false") wrong.push("inner " + O.inner(o));
if (o.own() !== "own,false") wrong.push("own " + o.own());
wrong.join();"##,
    );
}

/// Two evaluations of one class make two Private Names of one text; the eval code of each
/// class reads its own, also after a collection between the definitions and the evals.
#[test]
fn two_evaluations_of_one_class_each_read_their_own_name_through_eval() {
    let setup = r##"function make() { return class { #x; constructor(v) { this.#x = v; } static read(o) { return eval("o.#x"); } static has(o) { return eval("#x in o"); } }; }
var A = make(), B = make(), a = new A(1), b = new B(2);
0;"##;
    let check = r##"var wrong = [];
if (A.read(a) !== 1) wrong.push("A");
if (B.read(b) !== 2) wrong.push("B");
if (A.has(b) !== false || B.has(a) !== false) wrong.push("has");
if (A.has(a) !== true || B.has(b) !== true) wrong.push("own");
try { A.read(b); wrong.push("no TypeError"); } catch (e) { if (!(e instanceof TypeError)) wrong.push("other " + e); }
wrong.join();"##;
    for prologue in ["", "\"use strict\";\n"] {
        let mut agent = agent().unwrap();
        let r = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &r, &format!("{prologue}{setup}")),
            Outcome::Normal("0".to_string())
        );
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        assert_eq!(
            eval_text(&mut agent, &r, &format!("{prologue}{check}")),
            nothing()
        );
    }
}

/// An indirect eval (`(0, eval)`, an alias, `eval?.()`) and `Function` code have no private
/// environment: a private name in them is the `SyntaxError` of AllPrivateIdentifiersValid, also
/// inside a class that declares it.
#[test]
fn indirect_eval_and_function_code_have_no_private_names() {
    let script = format!(
        r##"{SYNTAX}var wrong = [];
class C {{
  #x = 1;
  tries() {{
    var e = eval;
    return [
      syntax(() => (0, eval)("this.#x")),
      syntax(() => e("this.#x")),
      syntax(() => eval?.("this.#x")),
      syntax(() => Function("return this.#x;")),
      syntax(() => new Function("o", "return o.#x;")),
      syntax(() => (0, eval)("#x in {{}}")),
    ];
  }}
}}
new C().tries().forEach(function (m, i) {{ if (m.indexOf("{}") !== 0) wrong.push(i + ": " + m); }});
wrong.join();"##,
        undeclared("#x")
    );
    lists_nothing(&script);
}

/// A name no class around the call declares is the `SyntaxError` of AllPrivateIdentifiersValid,
/// counted as no refusal: in a method, beside a declared name, outside every class, in a
/// heritage (which runs before the class's private record is entered), and in a class the eval
/// code declares.
#[test]
fn an_undeclared_private_name_in_eval_is_a_syntax_error() {
    let script = format!(
        r##"{SYNTAX}var wrong = [], heritage;
class C {{
  #x;
  a() {{ return syntax(() => eval("this.#y")); }}
  b() {{ return syntax(() => eval("this.#x; this.#y")); }}
  c() {{ return syntax(() => eval("(class {{ m() {{ return this.#z; }} }})")); }}
}}
class D extends (function () {{ heritage = syntax(() => eval("this.#d")); return Object; }})() {{ #d; }}
var c = new C(), want = {{ a: "#y", b: "#y", c: "#z" }};
["a", "b", "c"].forEach(function (k) {{
  var m = c[k](), name = want[k];
  if (m.indexOf("private name " + name + " is not declared in an enclosing class (") !== 0) wrong.push(k + ": " + m);
}});
if (syntax(() => eval("this.#x")).indexOf("{}") !== 0) wrong.push("outside");
if (heritage.indexOf("{}") !== 0) wrong.push("heritage: " + heritage);
wrong.join();"##,
        undeclared("#x"),
        undeclared("#d")
    );
    for text in [script.clone(), format!("\"use strict\";\n{script}")] {
        let mut agent = agent().unwrap();
        let r = realm(&mut agent);
        let before = agent.refusals();
        assert_eq!(eval_text(&mut agent, &r, &text), nothing(), "{text}");
        assert_eq!(agent.refusals(), before, "{text}");
    }
}

/// The code units of the String literal of the eval code of
/// [`a_collection_between_the_walk_and_the_run_keeps_the_names`]: its copy for the parse and its
/// constant pass the collector's smallest threshold (1 MiB) between the walk and the run.
const LONG_LITERAL: usize = 600_000;

/// The list of the walk holds texts and positions, no handle: a collection between the walk and
/// the run of the eval code changes nothing it reads. The eval code's own source passes the
/// collector's threshold while it is parsed and compiled, so the safe point at the first
/// instruction of the eval code collects (in the default build by bytes, after a collection
/// before the entry; with a collection at every safe point always), and the code then reads a
/// field and calls a method through the positions the walk gave.
#[test]
fn a_collection_between_the_walk_and_the_run_keeps_the_names() {
    let setup = format!(
        "class C {{ #x = 41; #m() {{ return 1; }} read(s) {{ return eval(s); }} }}\n\
         var c = new C(), src = '\"' + \"x\".repeat({LONG_LITERAL}) + '\".length + this.#x + this.#m()';\n0;"
    );
    // The source is charged a step per code unit as it is made, copied and parsed.
    let mut l = limits();
    l.steps_per_entry = 20 * LONG_LITERAL as u64;
    let mut agent = agent_with(l).unwrap();
    let r = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &r, &setup),
        Outcome::Normal("0".to_string())
    );
    let read = parse(&mut agent, &r, "c.read(src);");
    for _ in 0..2 {
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        let outcome = agent.evaluate_script(&read).unwrap();
        assert_eq!(
            support::script::describe(&mut agent, &r, outcome),
            Outcome::Normal((LONG_LITERAL + 42).to_string())
        );
    }
}

/// The levels of [`nested_eval_with_private_names_is_bounded`]: the frame depth, whose
/// `RangeError` ends the nesting; 100 with a collection at every safe point.
const LEVELS: u32 = if cfg!(feature = "collect-every-safe-point") {
    100
} else {
    1_000
};

/// Eval code that calls eval again, each level reading the field at the bottom: each level's
/// walk passes the records of the levels above it (strict eval code makes a record of its own).
/// Half the frame depth answers the field; twice it ends in the `RangeError` of `frame_depth`,
/// which the script catches; no panic, and the agent runs on.
#[test]
fn nested_eval_with_private_names_is_bounded() {
    let mut l = limits();
    l.frame_depth = LEVELS;
    l.steps_per_entry = 50_000_000;
    let mut agent = agent_with(l).unwrap();
    let r = realm(&mut agent);
    let setup = "class C { #x = 7; deep(n) { var k = 0, src = \"k++ < n ? eval(src) : this.#x\"; return eval(src); } }\nvar c = new C();\n0;";
    assert_eq!(
        eval_text(&mut agent, &r, setup),
        Outcome::Normal("0".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &r, &format!("c.deep({});", LEVELS / 2)),
        Outcome::Normal("7".to_string())
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &r,
            &format!(
                "var e; try {{ c.deep({}); }} catch (x) {{ e = x; }} e instanceof RangeError && e.message;",
                LEVELS * 2
            )
        ),
        Outcome::Normal("\"frame_depth exceeded\"".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &r, "c.deep(3);"),
        Outcome::Normal("7".to_string())
    );
}

/// The loop bound of [`an_eval_loop_reading_a_private_field_ends_at_the_step_budget`]: 10,000
/// evals, 1,000 with a collection at every safe point. The step budget ends the loop first.
const EVALS: u32 = if cfg!(feature = "collect-every-safe-point") {
    1_000
} else {
    10_000
};

/// The step budget of that loop: 20,000 steps, 1,000 with a collection at every safe point.
const LOOP_STEPS: u64 = if cfg!(feature = "collect-every-safe-point") {
    1_000
} else {
    20_000
};

/// The steps of `c.m(true)` (a direct eval) and of `c.m(false)` (an indirect one) at the same
/// place: in a method of a class with `names` private fields, under `depth` nested blocks whose
/// `let` bindings live in records (the direct eval names every record around it).
fn eval_steps(names: usize, depth: usize) -> (u64, u64) {
    let fields: String = (0..names).map(|k| format!("#p{k} = {k}; ")).collect();
    let open: String = (0..depth).map(|k| format!("{{ let b{k} = {k}; ")).collect();
    let close = "}".repeat(depth);
    let setup = format!(
        "class C {{ {fields}m(direct) {{ {open}if (direct) eval(\"1\"); else (0, eval)(\"1\"); {close}return 0; }} }}\nvar c = new C();\n0;"
    );
    let mut agent = agent().unwrap();
    let r = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &r, &setup),
        Outcome::Normal("0".to_string())
    );
    // The script is run without `describe`, whose reading of the answer is an entry of its own.
    let mut steps = |source: &str| {
        let script = parse(&mut agent, &r, source);
        assert!(matches!(
            agent.evaluate_script(&script).unwrap(),
            HostOutcome::Normal(_)
        ));
        agent.last_entry_steps().unwrap()
    };
    (steps("c.m(true);"), steps("c.m(false);"))
}

/// "Hostile input must never hang": a loop of up to 10,000 direct evals, each reading a private
/// field, ends in `Terminated(StepBudget)` before its end; the list of each walk is credited
/// once the eval code is compiled, so `heap_bytes()` after the run and a collection equals its
/// reading after the same run before. The walk spends a step per record: the steps of one direct
/// eval less those of an indirect one at the same place grow by one per record of depth (63
/// from 1 to 64 blocks), and by the names a record holds past the first 8; the table is printed
/// (the default build reads 0, 1, 8 and 100 private names at 1, 8 and 64 blocks).
#[test]
fn an_eval_loop_reading_a_private_field_ends_at_the_step_budget() {
    let mut l = limits();
    l.steps_per_entry = LOOP_STEPS;
    let mut agent = agent_with(l).unwrap();
    let r = realm(&mut agent);
    let setup = format!(
        "class C {{ #x = 1; loop() {{ for (var i = 0; i < {EVALS}; i++) {{ done = i; eval(\"this.#x\"); }} }} }}\nvar c = new C(), done = 0;\n0;"
    );
    assert_eq!(
        eval_text(&mut agent, &r, &setup),
        Outcome::Normal("0".to_string())
    );
    let run = parse(&mut agent, &r, "c.loop();");
    let mut readings = Vec::new();
    for _ in 0..2 {
        assert!(matches!(
            agent.evaluate_script(&run).unwrap(),
            HostOutcome::Terminated(Termination::StepBudget)
        ));
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        readings.push(agent.heap_bytes().unwrap());
    }
    assert_eq!(readings[0], readings[1], "the walk's list leaked");
    let done = eval_text(&mut agent, &r, "done;");
    let Outcome::Normal(done) = done else {
        panic!("{done:?}");
    };
    let done: u32 = done.parse().unwrap();
    assert!(done > 10 && done < EVALS, "{done} evals before the budget");

    let walk = |names: usize, depth: usize| {
        let (direct, indirect) = eval_steps(names, depth);
        direct as i64 - indirect as i64
    };
    let shallow = walk(1, 1);
    let deep = walk(1, 64);
    assert!(
        deep - shallow >= 63,
        "one direct eval spends {shallow} steps over an indirect one at 1 block, {deep} at 64"
    );
    if cfg!(feature = "collect-every-safe-point") {
        return;
    }
    let mut table = Vec::new();
    for names in [0, 1, 8, 100] {
        for depth in [1, 8, 64] {
            table.push(format!(
                "{names} names, {depth} blocks: {}",
                walk(names, depth)
            ));
        }
    }
    println!(
        "steps of a direct eval less an indirect one: {}",
        table.join("; ")
    );
    assert!(walk(100, 1) - walk(8, 1) >= 92, "{}", table.join("; "));
}

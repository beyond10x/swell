//! Private names in direct eval code (`#sec-evaldeclarationinstantiation`, steps 4 to 7, with
//! the privateEnv of PerformEval, `#sec-performeval`): the eval code is parsed with the names of
//! the running PrivateEnvironment and its outer ones (privateIdentifiers, for
//! AllPrivateIdentifiersValid), and `#x`, `o.#x`, `#x in o` and a private method's call in it
//! evaluate against the Private Names of the classes around the call, as other code does: in
//! methods, constructors, field initializers, static blocks, the arrows and functions in them,
//! computed keys, default parameters and inner classes, each evaluation of a class seeing its own
//! names. An indirect eval and `Function` code have no private names. The walk that lists the
//! names (`Agent::eval_private_names`, `docs/design/engine.md`, "Private names", item 4) spends a
//! step per record and lists texts and positions only; a direct eval whose String holds no `#`
//! does not walk. Written by
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
/// `let` bindings live in records (the direct eval names every record around it). The eval
/// text is `'#'`, a string literal: a String with a `#` walks, though it names no private name.
fn eval_steps(names: usize, depth: usize) -> (u64, u64) {
    eval_steps_of(names, depth, "'#'")
}

/// [`eval_steps`] with the eval text `text`, written in a JavaScript String literal in `"`.
fn eval_steps_of(names: usize, depth: usize, text: &str) -> (u64, u64) {
    let fields: String = (0..names).map(|k| format!("#p{k} = {k}; ")).collect();
    let open: String = (0..depth).map(|k| format!("{{ let b{k} = {k}; ")).collect();
    let close = "}".repeat(depth);
    let setup = format!(
        "class C {{ {fields}m(direct) {{ {open}if (direct) eval(\"{text}\"); else (0, eval)(\"{text}\"); {close}return 0; }} }}\nvar c = new C();\n0;"
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

/// `source` defines `cases`, a list of `[eval text, wanted answer]`, and a
/// class `C` whose method `run(src)` returns `eval(src)`; each text runs on a fresh instance and
/// the script lists every answer that is not the wanted one.
fn case_list(prelude: &str, cases: &str) -> String {
    format!(
        r##"var wrong = [];
{prelude}
var cases = [
{cases}
];
cases.forEach(function (k) {{
  var got;
  try {{ got = new C().run(k[0]); }} catch (e) {{ got = "threw " + e; }}
  if (got !== k[1]) wrong.push(JSON.stringify(k[0]) + " => " + String(got));
}});
wrong.join(" ;; ");"##
    )
}

/// Every shape of eval code between a private reference and the caller's
/// record (`#sec-performeval` step 16, `#sec-evaldeclarationinstantiation`): top-level `let`,
/// `const`, `var`, function and class declarations, captured or not; a function declaration of
/// eval code beside a captured `let` (its closure is the eval code's own record); blocks,
/// `for (let ...)`, `switch`, `catch` with and without a binding, a generator after a `yield`,
/// parameter expressions, computed keys, heritage, static blocks, field initializers, nested
/// classes with their own private names and nested direct evals; and every write form.
#[test]
fn eval_code_shapes_count_their_records() {
    let prelude = r##"class C {
  #x = 1;
  #m() { return 2; }
  get #g() { return 3; }
  set #g(v) { this.v = v; }
  static #s = 4;
  run(src) { return eval(src); }
}"##;
    let cases = r##"["this.#x", 1],
["let a = 10; this.#x + a", 11],
["let a = 10; (() => a)(); this.#x + a", 11],
["const a = 10; (() => a)(); (() => this.#x)() + a", 11],
["let a = 10; function f(o) { return o.#x + a; } f(this)", 11],
["let a = 10; (() => a)(); function f(o) { return o.#x; } f(this)", 1],
["function f(o) { return o.#x; } f(this)", 1],
["var v = 5; function f(o) { return o.#x + v; } f(this)", 6],
["var v = 5; (() => v)(); this.#x + v", 6],
["let a = 1; (() => a)(); var v = 5; function f(o) { return o.#x + v + a; } f(this)", 7],
["const k = 3; class K { #k = k; r(o) { return o.#x + this.#k; } } new K().r(this)", 4],
["class K { static #y = 1; static r(o) { return o.#x + o.#m() + K.#y; } } K.r(this)", 4],
["class K { r(o) { return o.#x; } } new K().r(this)", 1],
["let r = []; for (let i = 0; i < 2; i++) r.push(() => this.#x + i); r[0]() + r[1]()", 3],
["let s = 0; switch (1) { case 1: let z = 2; s = (() => this.#x + z)(); } s", 3],
["let s; try { throw 5; } catch (e) { s = (() => this.#x + e)(); } s", 6],
["let s; try { throw 5; } catch { let q = 1; s = (() => this.#x + q)(); } s", 2],
["function* g(o) { let q = 1; yield q; return (() => o.#x + q)(); } var it = g(this); it.next(); it.next().value", 2],
["(function (a = () => this.#x) { let q = 1; return a() + (() => q)(); }).call(this)", 2],
["class K { [this.#x + 'k']() { return 1; } } typeof K.prototype['1k']", "function"],
["class K extends (this.#x, Object) { #z = 1; } typeof K", "function"],
["let o = this; class L { static #w = 1; static { L.got = o.#x + L.#w; } } L.got", 2],
["class K { f = (o) => o.#x; } new K().f(this)", 1],
["class K { #x = 5; f(o) { return this.#x; } } new K().f(this)", 5],
["class K { #x = 5; f(o) { try { return o.#x; } catch (e) { return e instanceof TypeError; } } } new K().f(this)", true],
["eval('this.#x')", 1],
["eval('let z = 1; (() => z)(); this.#x + z')", 2],
["{ let b = 1; (() => b)(); eval('this.#x + b'); }", 2],
["let a = 1; (() => a)(); class K { #k = 2; m(o) { return eval('o.#x + this.#k + a'); } } new K().m(this)", 4],
["this.#g = 7; this.v + this.#g", 10],
["[this.#x] = [9]; ({ a: this.#x } = { a: 8 }); this.#x", 8],
["this.#x += 4; this.#x", 5],
["this.#x &&= 6; this.#x", 6],
["#s in C", true],
["C.#s", 4],
["#m in this", true],
["#g in this", true],
["this.#m.call(this)", 2],
["let f = this.#m; f()", 2],
["`${this.#x}`", "1"],
["this?.#x", 1],
["(() => { let a = 1; return () => this.#x + a; })()()", 2],
["label: { let a = 1; (() => a)(); break label; } this.#x", 1],
["let a = 1, b = 2; (() => a + b)(); { const c = 3; { let d = 4; (() => c + d)(); this.#x; } }", 1],
["new (class { #x = 'k'; m(o) { return [this.#x, o.#m()].join(); } })().m(this)", "k,2"],
["class K extends (class { #h = 1; }) { #x = 2; m(o) { return this.#x + o.#m(); } } new K().m(this)", 4],
["class K { static { this.z = 0; } } let q = 1; (() => q)(); (() => this.#x + q)()", 2],
["let a = 1; (() => a)(); (function () { 'use strict'; return this.#x + a; }).call(this)", 2],
["var o = this; let a = 1; (() => a)(); class K extends (o.#x, Object) { #x = 3; } typeof K", "function"],
["let a = 1; (() => a)(); class K { static f = (() => a)() + (o => o.#x); } typeof K.f", "string"],
["(eval)('this.#x')", 1],
["eval(...['this.#x'])", 1],
["class K { #x = 5; } this.#x", 1],
["class K { get #x() { return 5; } set #x(v) {} } this.#x", 1],
["C.#s = 9; C.#s", 9]"##;
    // Class code is strict either way: one run, not two, keeps the collect build in its bound.
    let source = case_list(prelude, cases);
    assert_eq!(
        support::script::run(limits(), &source),
        nothing(),
        "{source}"
    );
}

/// The places of the call in the class (the caller's records between the
/// eval and the private record): blocks, `for (let ...)`, `catch`, `switch`, parameter
/// expressions, a generator after a `yield`, an inner class with other and with the same names,
/// computed keys of an inner class, a class expression's name binding, a static block and a field
/// initializer with lexical bindings of their own, and a closure eval code returns called after
/// the method returned (and after a collection).
#[test]
fn caller_shapes_count_their_records() {
    let source = r##"var wrong = [];
class D {
  #x = 1;
  f = (() => { let q = 1; (() => q)(); return eval("this.#x + q"); })();
  static { let q = 1; (() => q)(); D.sb = eval("new D().#x + q"); }
  blk() { let b = 2; (() => b)(); { let c = 3; (() => c)(); return eval("this.#x + b + c"); } }
  forl() { var r = []; for (let i = 0; i < 2; i++) { r.push(eval("(() => this.#x + i)")); } return r[0]() + r[1](); }
  cat() { try { throw 2; } catch (e) { (() => e)(); return eval("this.#x + e"); } }
  catnb() { try { throw 2; } catch { let q = 1; (() => q)(); return eval("this.#x + q"); } }
  sw() { switch (1) { case 1: let z = 2; (() => z)(); return eval("this.#x + z"); } }
  par(a, f = () => eval("this.#x + a")) { var a2 = 5; (() => a2)(); return f(); }
  def(a = eval("this.#x")) { return a; }
  *gen() { let q = 1; (() => q)(); yield 0; yield eval("this.#x + q"); }
  nested() { class N { #y = 2; m(o) { return eval("o.#x + this.#y"); } } return new N().m(this); }
  same() { class N { #x = 5; m() { let a = 1; (() => a)(); return eval("this.#x + a"); } } return new N().m(); }
  ck() { var got; class N { #y; [(got = eval("this.#x"), "k")]() {} } return got; }
  ckInner() { var got; class N { #x = 9; static [(got = eval("#x in this"), "k")]() {} } return got; }
  closure() { let a = 1; (() => a)(); return eval("() => this.#x + a"); }
  self() { var E = class Named { #n = 3; m(o) { return eval("o.#x + this.#n + typeof Named"); } }; return new E().m(this); }
  her() { class I extends (eval("#x in this") ? Object : Array) { #x; } return Object.getPrototypeOf(I) === Object; }
  herRead() { var got; class I extends (got = eval("this.#x"), Object) { #x = 7; } return got; }
}
var d = new D(), g = d.gen(), c = d.closure();
g.next();
var answers = [d.f, D.sb, d.blk(), d.forl(), d.cat(), d.catnb(), d.sw(), d.par(1), d.def(), g.next().value,
  d.nested(), d.same(), d.ck(), d.ckInner(), c(), d.self(), d.her(), d.herRead()].join();
var want = "2,2,6,3,3,2,3,2,1,2,3,6,1,false,2,4function,true,1";
if (answers !== want) wrong.push(answers);
wrong.join();"##;
    lists_nothing(source);
}

/// A direct `eval` in the heritage of a class that declares the name
/// (`#sec-runtime-semantics-classdefinitionevaluation`: the running PrivateEnvironment while
/// ClassHeritage is evaluated is outerPrivateEnvironment) is the `SyntaxError` of
/// AllPrivateIdentifiersValid, with the call directly in the heritage and not inside a function.
#[test]
fn a_direct_eval_in_the_heritage_does_not_see_the_class_names() {
    let source = r##"var wrong = [], h;
try { class D extends (eval("#d in {}") ? Object : Object) { #d; } h = "none"; }
catch (e) { h = e instanceof SyntaxError ? "SE" : "other " + e; }
if (h !== "SE") wrong.push("heritage " + h);
wrong.join();"##;
    lists_nothing(source);
}

/// What must stay an early error in direct eval code, thrown by the call as a
/// `SyntaxError` before any of the eval text runs, and the forms of a call that are no direct
/// `eval` (`#sec-function-calls-runtime-semantics-evaluation`).
#[test]
fn early_errors_of_direct_eval_code_stay_errors() {
    let source = r##"var wrong = [];
globalThis.side = 0;
class E {
  #x = 1;
  m(src) { try { eval(src); return "none"; } catch (e) { return e instanceof SyntaxError ? "SE" : "other " + e; } }
  alias(src) { var e = eval; try { e(src); return "none"; } catch (x) { return x instanceof SyntaxError ? "SE" : "other " + x; } }
  opt(src) { try { eval?.(src); return "none"; } catch (x) { return x instanceof SyntaxError ? "SE" : "other " + x; } }
  glob(src) { try { globalThis.eval(src); return "none"; } catch (x) { return x instanceof SyntaxError ? "SE" : "other " + x; } }
  fn(src) { try { Function(src); return "none"; } catch (x) { return x instanceof SyntaxError ? "SE" : "other " + x; } }
  fnEval(src) { try { return Function("s", "return eval(s);").call(this, src), "none"; } catch (x) { return x instanceof SyntaxError ? "SE" : "other " + x; } }
}
var e = new E();
var cases = [
  ["m", "side = 1; delete this.#x", "SE"],
  ["m", "side = 1; class K { #x; #x; }", "SE"],
  ["m", "side = 1; class K { #constructor; }", "SE"],
  ["m", "side = 1; class K { #y; } this.#y", "SE"],
  ["m", "side = 1; class K { #y; } (class { static m(o) { return o.#y; } })", "SE"],
  ["m", "side = 1; #x", "SE"],
  ["m", "side = 1; #y in this", "SE"],
  ["m", "side = 1; super.#x", "SE"],
  ["m", "side = 1; this.#x; let a; let a;", "SE"],
  ["m", "class K { #x; get #x() {} }", "SE"],
  ["m", "class K { #x; m() { return this.#x; } } 0", "none"],
  ["m", "class K { m(o) { return o.#x; } } 0", "none"],
  ["alias", "side = 1; this.#x", "SE"],
  ["opt", "side = 1; this.#x", "SE"],
  ["glob", "side = 1; this.#x", "SE"],
  ["fn", "side = 1; return this.#x", "SE"],
  ["fnEval", "side = 1; this.#x", "SE"],
  ["m", "side = 1; (0, eval)('this.#x')", "other SyntaxError"],
];
cases.forEach(function (k) {
  var got = e[k[0]](k[1]);
  if (k[2] === "other SyntaxError") { if (side !== 1) wrong.push("no run " + k[1]); side = 0; return; }
  if (got !== k[2]) wrong.push(k[0] + " " + JSON.stringify(k[1]) + " => " + got);
  if (side !== 0) { wrong.push("ran " + k[0] + " " + k[1]); side = 0; }
});
wrong.join(" ;; ");"##;
    lists_nothing(source);
}

/// An async method reads the name in a direct eval after an `await`, and an
/// async function of eval code reads it after an `await` (the job runs after the eval returned).
#[test]
fn eval_after_an_await_reads_the_name() {
    let source = r##"var got = [];
class A {
  #x = 1;
  async m() { let q = 1; (() => q)(); await 0; return eval("this.#x + q"); }
  run() { return eval("let a = 2; (() => a)(); (async () => { await 0; return this.#x + a; })()"); }
}
var a = new A();
a.m().then(v => got.push(v), e => got.push("threw " + e));
a.run().then(v => got.push(v), e => got.push("threw " + e));
0;"##;
    let mut agent = agent().unwrap();
    let r = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &r, source),
        Outcome::Normal("0".to_string())
    );
    assert!(matches!(
        agent.perform_microtask_checkpoint(),
        Ok(HostOutcome::Normal(_))
    ));
    assert_eq!(
        eval_text(&mut agent, &r, "got.sort().join();"),
        Outcome::Normal("\"2,3\"".to_string())
    );
}

/// The steps of the walk past the unit's table, and its wall time: one direct eval of `'#'`
/// less an indirect one at 1,000 names and 1 block, 1 name and 300 blocks, 64 and 100 classes
/// of one name each, and the cost of a direct `eval("1")`, which does not walk, in a function
/// whose record has many named slots and under a script with many top-level `let`s.
#[test]
fn walk_cost_beyond_the_table() {
    // A measurement: the default build only.
    if cfg!(feature = "collect-every-safe-point") {
        return;
    }
    let walk = |names: usize, depth: usize| {
        let (direct, indirect) = eval_steps(names, depth);
        direct as i64 - indirect as i64
    };
    let start = std::time::Instant::now();
    let cells = [(0, 1), (8, 8), (100, 64), (1000, 1), (1, 300), (1000, 300)];
    let measured: Vec<String> = cells
        .iter()
        .map(|&(n, d)| {
            let t = std::time::Instant::now();
            let w = walk(n, d);
            format!("{n} names {d} blocks: {w} steps ({:?})", t.elapsed())
        })
        .collect();
    println!("walk: {}", measured.join("; "));
    // 64 and 100 nested classes, one name each, distinct texts: the eval in the innermost.
    let nested = |levels: usize| {
        let mut s = String::from("function chain(d) { ");
        for k in 0..levels {
            s.push_str(&format!("class K{k} {{ #n{k} = {k}; static m(d) {{ "));
        }
        s.push_str("return d ? eval(\"'#'\") : (0, eval)(\"'#'\");");
        for k in (0..levels).rev() {
            s.push_str(&format!(" }} }} return K{k}.m(d);"));
        }
        s.push_str(" }\n0;");
        s
    };
    for levels in [64usize, 100] {
        let mut agent = agent().unwrap();
        let r = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &r, &nested(levels)),
            Outcome::Normal("0".to_string())
        );
        let mut steps = |source: &str| {
            let script = parse(&mut agent, &r, source);
            let t = std::time::Instant::now();
            assert!(matches!(
                agent.evaluate_script(&script).unwrap(),
                HostOutcome::Normal(_)
            ));
            (agent.last_entry_steps().unwrap() as i64, t.elapsed())
        };
        let (direct, td) = steps("chain(true);");
        let (indirect, ti) = steps("chain(false);");
        println!(
            "{levels} nested classes: direct less indirect {} steps ({td:?} against {ti:?})",
            direct - indirect
        );
        // Each class's text is compared with every text an inner class listed: those compares
        // are steps past the first 8 of a record (`env/mod.rs`, `eval_private_names`).
        let levels = levels as i64;
        assert!(
            direct - indirect >= levels * (levels - 1) / 2 - 8 * levels,
            "{levels} nested classes: the compares of the walk are not charged ({})",
            direct - indirect
        );
    }
    // A sloppy function whose 1,000 `var`s live in a named record (a direct eval stands in it).
    let vars: String = (0..1000).map(|k| format!("var v{k} = {k}; ")).collect();
    let lets: String = (0..1000).map(|k| format!("let g{k} = {k}; ")).collect();
    let mut agent = agent().unwrap();
    let r = realm(&mut agent);
    let setup = format!(
        "{lets}\nfunction sloppy(direct) {{ {vars} return direct ? eval(\"1\") : (0, eval)(\"1\"); }}\nfunction top(direct) {{ return direct ? eval(\"1\") : (0, eval)(\"1\"); }}\n0;"
    );
    assert_eq!(
        eval_text(&mut agent, &r, &setup),
        Outcome::Normal("0".to_string())
    );
    let mut steps = |source: &str| {
        let script = parse(&mut agent, &r, source);
        assert!(matches!(
            agent.evaluate_script(&script).unwrap(),
            HostOutcome::Normal(_)
        ));
        agent.last_entry_steps().unwrap() as i64
    };
    let sloppy = steps("sloppy(true);") - steps("sloppy(false);");
    let top = steps("top(true);") - steps("top(false);");
    println!(
        "no class: a direct eval less an indirect one in a function of 1,000 named vars {sloppy}, under 1,000 global lets {top}; total {:?}",
        start.elapsed()
    );
}

/// The class of [`step_budget_sweep`] and [`a_loop_of_500_evals_with_100_names_keeps_nothing`]:
/// 100 private names, a method whose direct eval stands under 64 blocks, and a loop of evals.
fn sweep_setup() -> String {
    let fields: String = (0..100).map(|k| format!("#p{k} = {k}; ")).collect();
    let open: String = (0..64)
        .map(|k| format!("{{ let b{k} = {k}; (() => b{k}); "))
        .collect();
    let close = "}".repeat(64);
    format!(
        "class C {{ {fields}m() {{ {open}return eval(\"this.#p99 + this.#p0\"); {close} }} loop(n) {{ var s = 0; for (var i = 0; i < n; i++) s += eval(\"this.#p1\"); return s; }} }}\nvar c;\n0;"
    )
}

/// The step budget lowered one step at a time across the walk, the parse
/// and the compile of one direct eval in a class of 100 private names under 64 blocks: each run
/// ends in `Terminated(StepBudget)` or answers the field, and a second run at the same budget
/// leaves `heap_bytes()` after a collection where the first left it (nothing of a stopped walk,
/// parse or compile is kept). The default build only (a fresh agent per budget); in three tests
/// of one third of the budgets each (`part` 0, 1, 2), so that each stays in its time.
fn step_budget_sweep(part: u64) {
    const RUN: &str = "c = new C(); c.m();";
    // The walk (157 steps), the parse of the eval text and the run of the eval code.
    const SWEEP: u64 = 171;
    if cfg!(feature = "collect-every-safe-point") {
        return;
    }
    let setup = sweep_setup();
    let (setup_steps, total) = {
        let mut agent = agent().unwrap();
        let r = realm(&mut agent);
        let mut steps = |source: &str| {
            let script = parse(&mut agent, &r, source);
            assert!(matches!(
                agent.evaluate_script(&script).unwrap(),
                HostOutcome::Normal(_)
            ));
            agent.last_entry_steps().unwrap()
        };
        (steps(&setup), steps(RUN))
    };
    assert!(setup_steps + 10 < total - SWEEP, "{setup_steps} {total}");
    println!("setup {setup_steps} steps, run {total}");
    let mut ended = 0;
    let mut answered = 0;
    let third = SWEEP / 3;
    let first = total - SWEEP + part * third;
    let last = if part == 2 { total } else { first + third - 1 };
    let budgets = first..=last;
    for budget in budgets {
        let mut l = limits();
        l.steps_per_entry = budget;
        let mut agent = agent_with(l).unwrap();
        let r = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &r, &setup),
            Outcome::Normal("0".to_string())
        );
        let run = parse(&mut agent, &r, RUN);
        let mut readings = Vec::new();
        for _ in 0..2 {
            match agent.evaluate_script(&run).unwrap() {
                HostOutcome::Terminated(Termination::StepBudget) => ended += 1,
                HostOutcome::Normal(value) => {
                    assert_eq!(
                        support::script::describe(&mut agent, &r, HostOutcome::Normal(value)),
                        Outcome::Normal("99".to_string()),
                        "{budget}"
                    );
                    answered += 1;
                }
                other => panic!("{budget}: {other:?}"),
            }
            assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
            readings.push(agent.heap_bytes().unwrap());
        }
        assert_eq!(
            readings[0], readings[1],
            "{budget}: a stopped eval kept bytes"
        );
    }
    println!("total {total}: {ended} runs ended at the budget, {answered} answered");
    assert!(ended > 0, "{ended} {answered}");
    if part == 2 {
        assert!(answered > 0, "{ended} {answered}");
    }
}

/// [`step_budget_sweep`], the lowest third of the budgets (the walk).
#[test]
fn step_budget_across_the_walk_keeps_nothing() {
    step_budget_sweep(0);
}

/// [`step_budget_sweep`], the middle third of the budgets (the end of the walk).
#[test]
fn step_budget_across_the_end_of_the_walk_keeps_nothing() {
    step_budget_sweep(1);
}

/// [`step_budget_sweep`], the top third of the budgets (the parse, the compile and the run).
#[test]
fn step_budget_across_the_parse_keeps_nothing() {
    step_budget_sweep(2);
}

/// 500 direct evals in a loop, each walking a class of 100 private names, leave `heap_bytes()`
/// after a collection where it was before the loop (the default build; the collect build's
/// loop is `an_eval_loop_reading_a_private_field_ends_at_the_step_budget`).
#[test]
fn a_loop_of_500_evals_with_100_names_keeps_nothing() {
    if cfg!(feature = "collect-every-safe-point") {
        return;
    }
    let setup = sweep_setup();
    let mut agent = agent().unwrap();
    let r = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &r, &setup),
        Outcome::Normal("0".to_string())
    );
    let run = parse(&mut agent, &r, "c = new C(); c.loop(500);");
    let mut readings = Vec::new();
    for _ in 0..2 {
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        readings.push(agent.heap_bytes().unwrap());
        let outcome = agent.evaluate_script(&run).unwrap();
        assert_eq!(
            support::script::describe(&mut agent, &r, outcome),
            Outcome::Normal("500".to_string())
        );
    }
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    readings.push(agent.heap_bytes().unwrap());
    assert_eq!(
        readings[1], readings[2],
        "500 evals kept bytes: {readings:?}"
    );
}

/// No private name exists outside a class body, so a direct `eval("1")` in a
/// non-strict function with no class around it should cost about what it cost before
/// `story:private-names-in-eval-code` (a step per record at most, as `docs/design/engine.md`,
/// "Private names", item 4, states the walk). The walk reads every slot of every record whose
/// slots are named, and a function with a direct `eval` names all of its slots: with 1,000
/// `var`s one direct eval spends 992 steps more than an indirect one at the same place, for a
/// list it always finds empty.
#[test]
fn a_direct_eval_outside_every_class_pays_no_step_per_slot() {
    // 1,000 `var`s; 200 with a collection at every safe point.
    let count = if cfg!(feature = "collect-every-safe-point") {
        200
    } else {
        1000
    };
    let vars: String = (0..count).map(|k| format!("var v{k} = {k}; ")).collect();
    let mut agent = agent().unwrap();
    let r = realm(&mut agent);
    let setup = format!(
        "function sloppy(direct) {{ {vars} return direct ? eval(\"1\") : (0, eval)(\"1\"); }}\n0;"
    );
    assert_eq!(
        eval_text(&mut agent, &r, &setup),
        Outcome::Normal("0".to_string())
    );
    let mut steps = |source: &str| {
        let script = parse(&mut agent, &r, source);
        assert!(matches!(
            agent.evaluate_script(&script).unwrap(),
            HostOutcome::Normal(_)
        ));
        agent.last_entry_steps().unwrap() as i64
    };
    let extra = steps("sloppy(true);") - steps("sloppy(false);");
    assert!(
        extra <= 16,
        "a direct eval in a function of {count} vars and no class spends {extra} steps more than an indirect one"
    );
}

/// The steps one direct eval of `text` spends more than an indirect one at the same place, in a
/// non-strict function of `count` `var`s (a function with a direct eval names all of its slots)
/// and no class around it; `text` is written in a JavaScript String literal in `"`.
fn sloppy_extra_steps(count: usize, text: &str) -> i64 {
    let vars: String = (0..count).map(|k| format!("var v{k} = {k}; ")).collect();
    let mut agent = agent().unwrap();
    let r = realm(&mut agent);
    let setup = format!(
        "function sloppy(direct) {{ {vars} return direct ? eval(\"{text}\") : (0, eval)(\"{text}\"); }}\n0;"
    );
    assert_eq!(
        eval_text(&mut agent, &r, &setup),
        Outcome::Normal("0".to_string())
    );
    let mut steps = |source: &str| {
        let script = parse(&mut agent, &r, source);
        assert!(matches!(
            agent.evaluate_script(&script).unwrap(),
            HostOutcome::Normal(_)
        ));
        agent.last_entry_steps().unwrap() as i64
    };
    steps("sloppy(true);") - steps("sloppy(false);")
}

/// A direct eval walks the caller's records whenever its String holds a `#` code unit, also
/// when the `#` stands only in a string literal or a comment (the scan looks at code units, not
/// tokens): in a function of 1,000 named `var`s (100 with a collection at every safe point) and
/// no class, `eval("'#'")` and `eval("1 // #")` each spend a step per slot past the first 8 of
/// the record and a step per record more than an indirect eval of the same text, where
/// `eval("1")` spends none of them. A String without `#` costs the same at any number of
/// private names and blocks around it.
#[test]
fn a_hash_in_a_string_literal_or_a_comment_walks_the_records() {
    let count = if cfg!(feature = "collect-every-safe-point") {
        100
    } else {
        1000
    };
    let plain = sloppy_extra_steps(count, "1");
    let literal = sloppy_extra_steps(count, "'#'");
    let comment = sloppy_extra_steps(count, "1 // #");
    println!(
        "{count} vars, no class: a direct eval less an indirect one: \"1\" {plain}, \"'#'\" {literal}, \"1 // #\" {comment}"
    );
    let floor = count as i64 - 8;
    assert!(plain <= 16, "{plain}");
    assert!(literal - plain >= floor, "{literal} against {plain}");
    assert!(comment - plain >= floor, "{comment} against {plain}");
    let no_hash = |names: usize, depth: usize| {
        let (direct, indirect) = eval_steps_of(names, depth, "1");
        direct as i64 - indirect as i64
    };
    let (near, far) = (no_hash(0, 1), no_hash(100, 64));
    assert_eq!(
        near, far,
        "eval(\"1\") at 0 names, 1 block and 100 names, 64 blocks"
    );
}

/// A direct eval whose text holds no `#` does not walk, and the direct eval its code makes with
/// a text built at run time walks at its own time: the outer texts below have no `#`, the inner
/// String gets it from `String.fromCharCode(35)`, and the inner eval reads, writes and tests
/// the field of the class around the outer call, also from a function the outer eval code made
/// and that runs after the outer eval returned. A `#` written as an escape (`\u{23}`) is no `#`:
/// `this.\u{23}x` names no private name and is a `SyntaxError` in the class too.
#[test]
fn an_inner_eval_of_a_text_built_at_run_time_walks_at_its_own_time() {
    lists_nothing(
        r##"var wrong = [];
class C {
  #x = 1;
  read() { return eval("eval(['this.', String.fromCharCode(35), 'x'].join(''))"); }
  write() { return eval("var h = String.fromCharCode(35); eval('this.' + h + 'x = 5, this.' + h + 'x')"); }
  has(o) { return eval("eval(String.fromCharCode(35) + 'x in o')"); }
  later() { return eval("(function (o) { return eval('o.' + String.fromCharCode(35) + 'x'); })"); }
  escaped() { try { eval("this.\\u{23}x"); return "no error"; } catch (e) { return e instanceof SyntaxError; } }
}
var c = new C(), f = c.later();
if (c.escaped() !== true) wrong.push("escaped " + c.escaped());
if (c.read() !== 1) wrong.push("read " + c.read());
if (c.write() !== 5) wrong.push("write");
if (c.has(c) !== true || c.has({}) !== false) wrong.push("has");
if (f(c) !== 5) wrong.push("later " + f(c));
wrong.join();"##,
    );
}

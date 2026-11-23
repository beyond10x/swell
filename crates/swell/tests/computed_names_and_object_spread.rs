//! Computed property names (`#sec-object-initializer`, ComputedPropertyName) and object rest and
//! spread: PropertyDefinitionEvaluation (`#sec-runtime-semantics-propertydefinitionevaluation`)
//! evaluates a computed key and converts it by ToPropertyKey (`#sec-topropertykey`) before the
//! value, in source order, and names an anonymous function, a method or an accessor under it by
//! SetFunctionName (`#sec-setfunctionname`) at run time; a spread property copies by
//! CopyDataProperties (`#sec-copydataproperties`); a rest property of an object pattern copies
//! every own enumerable property but the keys read before it
//! (`#sec-destructuring-binding-patterns-runtime-semantics-restbindinginitialization`,
//! `#sec-runtime-semantics-restdestructuringassignmentevaluation`). Written by
//! `story:computed-names-and-object-spread`. Run also with the feature
//! `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`; no script writes a Symbol
//! key, an array pattern or a spread in an array or a call.
#![cfg(test)]

use crate::support;

use support::host::{agent, agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{Outcome as HostOutcome, Parsed, Source, Termination};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `#sec-runtime-semantics-propertydefinitionevaluation`: each computed key is evaluated and
/// converted by ToPropertyKey (its object's `toString` runs) before its value is evaluated, the
/// properties in source order; a Number key is its ToString, a later key of the same name
/// replaces the value, and a computed `"__proto__"` is a data property, never the prototype.
#[test]
fn a_computed_key_is_evaluated_and_converted_before_its_value() {
    let source = "var log = [];\n\
                  function key(name) {\n\
                    return { toString: function () { log.push(\"key \" + name); return name; } };\n\
                  }\n\
                  function value(name) { log.push(\"value \" + name); return name; }\n\
                  var o = { a: value(\"a\"), [key(\"b\")]: value(\"b\"), [key(\"c\")]() {},\n\
                            get [key(\"d\")]() { return 1; }, [key(\"e\")]: value(\"e\") };\n\
                  var n = { [1 + 1]: \"two\", [0.5]: \"half\", a: 1, [\"a\"]: 2 };\n\
                  var p = { [\"__proto__\"]: 1 };\n\
                  log.join() === \"value a,key b,value b,key c,key d,key e,value e\"\n\
                    && Object.keys(o).join() === \"a,b,c,d,e\" && o.b === \"b\" && o.d === 1\n\
                    && n[\"2\"] === \"two\" && n[\"0.5\"] === \"half\" && n.a === 2\n\
                    && Object.keys(n).join() === \"2,0.5,a\"\n\
                    && Object.getPrototypeOf(p) === Object.prototype\n\
                    && p.hasOwnProperty(\"__proto__\");";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-setfunctionname` at run time: a method under a computed key is named by its key, a
/// getter and a setter by `get ` and `set ` and the key, an anonymous function or arrow value
/// by its key (NamedEvaluation), a named function expression keeps its own name; a method has
/// no `prototype`, a function expression has one, after `length` and `name`.
#[test]
fn a_computed_method_and_accessor_get_their_names_at_run_time() {
    let source = "var k = \"m\";\n\
                  var o = { [k]() {}, get [k + \"g\"]() { return 0; }, set [k + \"s\"](v) {},\n\
                            [k + \"f\"]: function () {}, [k + \"a\"]: () => 0,\n\
                            [k + \"n\"]: function named() {}, [1]: function () {} };\n\
                  var g = Object.getOwnPropertyDescriptor(o, \"mg\").get;\n\
                  var s = Object.getOwnPropertyDescriptor(o, \"ms\").set;\n\
                  var d = Object.getOwnPropertyDescriptor(o.mf, \"name\");\n\
                  o.m.name === \"m\" && g.name === \"get mg\" && s.name === \"set ms\"\n\
                    && o.mf.name === \"mf\" && o.ma.name === \"ma\" && o.mn.name === \"named\"\n\
                    && o[1].name === \"1\" && !o.m.hasOwnProperty(\"prototype\")\n\
                    && Object.getOwnPropertyNames(o.mf).join() === \"length,name,prototype\"\n\
                    && !d.writable && !d.enumerable && d.configurable;";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-copydataproperties`: a spread copies the own enumerable properties of its source, in
/// the order of [[OwnPropertyKeys]], each read once by [[Get]] with the source as receiver, as
/// data properties of the literal (CreateDataProperty: an own `__proto__` stays a property);
/// no inherited and no non-enumerable one. A later property replaces a spread one and the other
/// way round; a String spreads its indices.
#[test]
fn an_object_spread_copies_own_enumerable_properties_once_through_getters() {
    let source = "var log = [];\n\
                  var source = Object.create({ inherited: 1 },\n\
                    { hidden: { value: 2, enumerable: false } });\n\
                  source.b = 2;\n\
                  Object.defineProperty(source, \"a\", { enumerable: true, configurable: true,\n\
                    get: function () { log.push(this === source ? \"a\" : \"wrong\"); return 1; } });\n\
                  source[0] = \"zero\";\n\
                  var copy = { before: 0, ...source, after: 3 };\n\
                  var d = Object.getOwnPropertyDescriptor(copy, \"a\");\n\
                  var proto = { ...JSON.parse('{\"__proto__\": 5}') };\n\
                  var s = { ...\"ab\" };\n\
                  log.join() === \"a\" && Object.keys(copy).join() === \"0,before,b,a,after\"\n\
                    && d.value === 1 && d.writable && d.enumerable && d.configurable\n\
                    && !(\"hidden\" in copy) && !(\"inherited\" in copy)\n\
                    && Object.getPrototypeOf(proto) === Object.prototype\n\
                    && proto.hasOwnProperty(\"__proto__\")\n\
                    && ({ a: 0, ...{ a: 1 } }).a === 1 && ({ ...{ a: 1 }, a: 2 }).a === 2\n\
                    && Object.keys(s).join() === \"0,1\" && s[1] === \"b\";";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-copydataproperties`, step 1: a spread of `undefined` or `null` copies nothing and
/// throws nothing; a Number or a Boolean has no own property to copy.
#[test]
fn an_object_spread_of_undefined_or_null_copies_nothing() {
    let source = "var o = { ...undefined, ...null, ...0, ...true, a: 1, ...function () {} };\n\
                  Object.keys(o).join() === \"a\";";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// RestBindingInitialization and RestDestructuringAssignmentEvaluation: the rest object is a
/// new ordinary object with the own enumerable properties of the value but the keys the
/// properties before it read (a static key, a Number key, a computed key), in a `var`, `let`
/// and `const` declaration, a parameter and an assignment (to a name and to a property); a key
/// read before is not read again; a computed key of a pattern is evaluated once, in source
/// order, before its property is read and before its default.
#[test]
fn an_object_rest_excludes_the_keys_read_before_it() {
    let source = "var src = { a: 1, b: 2, c: 3, 1: 4, [\"d\"]: 5 };\n\
                  var { a, ...rest } = src;\n\
                  let { b, [\"c\"]: c, 1: one, ...others } = src;\n\
                  var x, r2;\n\
                  ({ x, ...r2 } = src);\n\
                  function f({ a, ...more }) { return more; }\n\
                  var m = f(src);\n\
                  const { length, ...chars } = \"xyz\";\n\
                  var name = \"a\";\n\
                  var { [name]: viaKey, ...afterKey } = src;\n\
                  var r3 = {};\n\
                  ({ a: r3.a, ...r3.rest } = src);\n\
                  var count = 0;\n\
                  var { a: ga, ...gr } = { get a() { count++; return 1; }, b: 2 };\n\
                  var order = [];\n\
                  var { [(order.push(\"key\"), \"a\")]: q = (order.push(\"default\"), 0), ...qr } =\n\
                    { get a() { order.push(\"get\"); }, z: 1 };\n\
                  Object.keys(rest).join() === \"1,b,c,d\" && a === 1\n\
                    && Object.keys(others).join() === \"a,d\" && b === 2 && c === 3 && one === 4\n\
                    && Object.keys(r2).join() === \"1,a,b,c,d\" && x === undefined\n\
                    && Object.keys(m).join() === \"1,b,c,d\" && Object.keys(chars).join() === \"0,1,2\"\n\
                    && length === 3 && Object.keys(afterKey).join() === \"1,b,c,d\" && viaKey === 1\n\
                    && r3.a === 1 && Object.keys(r3.rest).join() === \"1,b,c,d\"\n\
                    && rest !== src && Object.getPrototypeOf(rest) === Object.prototype\n\
                    && count === 1 && ga === 1 && Object.keys(gr).join() === \"b\"\n\
                    && order.join() === \"key,get,default\" && q === 0\n\
                    && Object.keys(qr).join() === \"z\";";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// The properties of `large` in the tests of the budgets of a spread: 10,000, the full count, in
/// the default mode; 1,000 with the feature `collect-every-safe-point`, where every instruction
/// that makes `large` collects, in the gate and in the lane `slow` (`SWELL_FULL_COUNTS=1`)
/// alike (with 10,000 the budget test took 3.1 CPU seconds alone in the lane, measured on
/// 2026-11-22). The budgets are readings of an agent that ran the same scripts plus a room
/// that scales with the count, so 1,000 still end the spread in each budget.
fn large_properties() -> usize {
    if cfg!(feature = "collect-every-safe-point") {
        1_000
    } else {
        10_000
    }
}

/// The parts `large` is made in: 20 scripts, each `Object.assign` of `JSON.parse` of a twentieth
/// of its properties, so that each fits the step budget
/// [`a_spread_of_many_properties_ends_at_the_step_and_the_memory_budget`] sets, and each made of
/// a few instructions, as few collections in the collect mode.
const LARGE_PARTS: usize = 20;

/// The globals `small` and `large`, objects of 10 and [`large_properties`] own enumerable
/// properties `k0`, `k1` and so on, made by scripts of their own in `agent`, with a collection
/// after each, so the heap holds no garbage of one part when the next is made.
fn small_and_large(agent: &mut swell::Agent<support::host::TestHost>, realm: &swell::Realm) {
    let make = "var small = {}, large = {};\n\
                for (var i = 0; i < 10; i++) small[\"k\" + i] = i;\n\
                true;";
    assert_eq!(eval_text(agent, realm, make), passed());
    let n = large_properties();
    let part = n / LARGE_PARTS;
    for from in (0..n).step_by(part) {
        let fields: Vec<String> = (from..from + part)
            .map(|i| format!("\"k{i}\":{i}"))
            .collect();
        let fill = format!(
            "Object.assign(large, JSON.parse('{{{}}}')); true;",
            fields.join(",")
        );
        assert_eq!(eval_text(agent, realm, &fill), passed(), "part at {from}");
        agent.collect().unwrap();
    }
}

/// `docs/design/engine.md`, "The step budget": a spread charges a step per key, beyond the
/// step per key [[OwnPropertyKeys]] charges: a spread of an object of [`large_properties`] own
/// properties spends at least that many steps less 10 more than one of 10, the two scripts
/// differing only in a name.
#[test]
fn an_object_spread_charges_a_step_per_key() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    small_and_large(&mut agent, &realm);
    assert_eq!(
        eval_text(&mut agent, &realm, "({ ...small }); true;"),
        passed()
    );
    let small = agent.last_entry_steps().unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "({ ...large }); true;"),
        passed()
    );
    let large = agent.last_entry_steps().unwrap();
    let n = u64::try_from(large_properties()).unwrap();
    println!("steps: spread of 10 keys {small}, of {n} keys {large}");
    assert!(large >= small + n - 10, "{large} steps against {small}");
}

/// The step room above the measured steps of the spread of `small` that
/// [`a_spread_of_many_properties_ends_at_the_step_and_the_memory_budget`] gives, per property of
/// `large`: a step (10,000 at the full count), which a part of `large` fits in; the spread of
/// `large` takes about two steps a key (one of [[OwnPropertyKeys]], one copied).
const STEP_ROOM_PER_PROPERTY: u64 = 1;

/// The memory room above the measured heap after `large` was made, as a fraction of the bytes
/// the spread of `large` kept on the agent that measures (the copy's properties): three quarters.
/// Making `large` in the bounded agent passes through a peak above its end reading, when the
/// list of its properties grows and the old list is still held (at most about half of the copy's
/// bytes); the spread itself holds all of the copy's bytes and its key list before it ends.
const MEMORY_ROOM_FRACTION: (u64, u64) = (3, 4);

/// The step and the memory budget end a spread of an object of [`large_properties`] own
/// properties in `Terminated(StepBudget)` and `Terminated(MemoryBudget)`, never in a hang,
/// where each budget is a reading measured on an agent that ran the same scripts plus a room:
/// the steps of the spread of 10 properties, and the heap after the objects were made. The
/// spread of 10 still completes after each.
#[test]
fn a_spread_of_many_properties_ends_at_the_step_and_the_memory_budget() {
    let n = u64::try_from(large_properties()).unwrap();
    let step_room = n * STEP_ROOM_PER_PROPERTY;
    let mut measure = agent().unwrap();
    let measured = realm(&mut measure);
    small_and_large(&mut measure, &measured);
    let heap = measure.heap_bytes().unwrap();
    assert_eq!(
        eval_text(&mut measure, &measured, "({ ...small }); true;"),
        passed()
    );
    let steps = measure.last_entry_steps().unwrap();
    let before = measure.heap_bytes().unwrap();
    assert_eq!(
        eval_text(&mut measure, &measured, "var copy = { ...large }; true;"),
        passed()
    );
    let (spread_steps, spread_bytes) = (
        measure.last_entry_steps().unwrap(),
        measure.heap_bytes().unwrap() - before,
    );
    println!(
        "measured: heap {heap} bytes, spread of 10 keys {steps} steps, of {n} keys \
         {spread_steps} steps and {spread_bytes} bytes"
    );
    assert!(
        steps + step_room < spread_steps,
        "the step room is too wide"
    );
    let memory_room = spread_bytes * MEMORY_ROOM_FRACTION.0 / MEMORY_ROOM_FRACTION.1;
    println!("rooms: {step_room} steps, {memory_room} bytes");

    let mut stepped = limits();
    stepped.steps_per_entry = steps + step_room;
    let mut steps_agent = agent_with(stepped).unwrap();
    let steps_realm = realm(&mut steps_agent);
    small_and_large(&mut steps_agent, &steps_realm);
    assert_eq!(
        eval_text(&mut steps_agent, &steps_realm, "({ ...large }); true;"),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(
        eval_text(&mut steps_agent, &steps_realm, "({ ...small }); true;"),
        passed()
    );

    let mut bounded = limits();
    bounded.memory_bytes = heap + memory_room;
    let mut memory_agent = agent_with(bounded).unwrap();
    let memory_realm = realm(&mut memory_agent);
    small_and_large(&mut memory_agent, &memory_realm);
    assert_eq!(
        eval_text(&mut memory_agent, &memory_realm, "({ ...large }); true;"),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    assert_eq!(
        eval_text(&mut memory_agent, &memory_realm, "({ ...small }); true;"),
        passed()
    );
}

/// HOST-173 (decision 13): a throw of ToPropertyKey of a computed key is sited at the key, line
/// 4, column 4, where `bad` stands; a throw in a getter a spread reads is sited in the getter,
/// line 2, column 10, where `null.x` starts.
#[test]
fn a_throw_of_a_computed_key_and_of_a_spread_getter_is_sited_there() {
    let cases = [
        (
            "var bad = { toString: null, valueOf: null };\nvar o = {\n  a: 1,\n  [bad]: 2\n};",
            (4, 4),
        ),
        (
            "var src = { get a() {\n  return null.x; } };\nvar o = { ...src };",
            (2, 10),
        ),
    ];
    for (source, at) in cases {
        let mut agent = agent().unwrap();
        let realm = realm(&mut agent);
        let Parsed::Parsed(script) = agent
            .parse_script(&realm, Source::Utf8(source), None)
            .unwrap()
        else {
            panic!("{source:?} did not parse");
        };
        assert!(
            matches!(agent.evaluate_script(&script), Ok(HostOutcome::Throw(_))),
            "{source:?}"
        );
        let site = agent.last_entry_error_site().unwrap().unwrap();
        assert_eq!((site.line, site.column), at, "{source:?}");
    }
}

/// The body of the stack test: re-entries through the `toString` of a computed key (the one
/// instruction `ToPropertyKey` of a literal and of a pattern), and through a getter a spread
/// and a rest property read (`CopyDataProperties` and `ObjectRest`), one agent each.
fn reach_through_keys_and_spreads() {
    for through in [
        "({ [{ toString: function () { f(); return \"k\"; } }]: 0 })",
        "({ ...{ get a() { return f(); } } })",
        "var { ...r } = { get a() { return f(); } }",
    ] {
        reach_through(through, BUILTINS_REENTRY);
    }
}

/// `Limits::min_stack_bytes` through a computed key and a spread (decision 3): on a thread with
/// seven eighths of `min_stack_bytes()` for `frame_depth` 10,000, `reentry_depth` 1,000 and
/// `parser_nesting` 1,000, a script reaches `reentry_depth` re-entries through the `toString`
/// ToPropertyKey calls and through a getter CopyDataProperties reads, and parses 1,000 nested
/// brackets at the deepest, without a stack overflow. In the collect mode the depth is
/// [`BUILTINS_REENTRY`].
#[test]
fn a_computed_key_and_a_spread_re_enter_the_interpreter_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("a_computed_key_and_a_spread_re_enter_the_interpreter_on_the_promised_stack"),
        stack,
        reach_through_keys_and_spreads,
    )
    .unwrap();
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
        panic!("the literal did not parse");
    };
    assert_eq!(ledger.held, tree.capacity_bytes(), "held after the parse");
    assert_eq!(tree.charged, tree.capacity_bytes());
    ledger.high = ledger.held;
    let Ok(code) = swell_compiler::compile_script_with(&tree, &mut ledger) else {
        panic!("the literal did not compile");
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

/// The properties of [`many_computed_keys_and_spreads_are_charged_as_they_are_read`]: 10,000,
/// the full count, in the lane `slow` (`SWELL_FULL_COUNTS=1`); 1,000 in the gate, in either
/// mode. The test runs no script, so the feature `collect-every-safe-point` changes nothing it
/// does; at 10,000 its parses and compiles took 1.19 CPU seconds alone in each mode (measured
/// on 2026-11-23). 1,000 properties still grow every list of the parse and the compile step
/// past its first capacity many times over, as the large pattern of `object_patterns.rs` does
/// at its gate count, and each assertion compares the ledger with what the lists hold at that
/// size (a list that grew uncharged leaves the ledger below it at any size).
fn literal_properties() -> usize {
    if std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1") {
        10_000
    } else {
        1_000
    }
}

/// The frontend's ledger over an object literal of [`literal_properties`] computed keys, one of
/// as many spreads, and a pattern of as many computed keys with a rest property, as a `var`
/// declaration and an assignment pattern (read first as an object literal): the tree, the key
/// expressions and the code are charged as they grow.
#[test]
fn many_computed_keys_and_spreads_are_charged_as_they_are_read() {
    let n = literal_properties();
    let keys: Vec<String> = (0..n).map(|i| format!("[\"k{i}\"]: {i}")).collect();
    let spreads: Vec<String> = (0..n).map(|i| format!("...o{}", i % 10)).collect();
    let pattern: Vec<String> = (0..n).map(|i| format!("[\"k{i}\"]: p{i} = {i}")).collect();
    charged_as_read(&format!("x = {{ {} }};", keys.join(", ")));
    charged_as_read(&format!("x = {{ {} }};", spreads.join(", ")));
    charged_as_read(&format!(
        "var {{ {}, ...rest }} = {{}};",
        pattern.join(", ")
    ));
    charged_as_read(&format!(
        "var p0; ({{ {}, ...rest }} = {{}});",
        pattern.join(", ")
    ));
}

//! Symbol objects (ECMA-262 20.4, `#sec-symbol-objects`), `story:symbol-constructor-and-well-known-symbols`:
//! the `Symbol` function (`#sec-symbol-description`), `Symbol.for` and `Symbol.keyFor`
//! (`#sec-symbol.for`, `#sec-symbol.keyfor`) over the agent's GlobalSymbolRegistry
//! (`#sec-globalsymbolregistry-records`), the twelve well-known symbols of slice 6 as properties
//! of the constructor (`#sec-well-known-symbols`), `%Symbol.prototype%`
//! (`#sec-properties-of-the-symbol-prototype-object`), ToObject of a Symbol (`#sec-toobject`) and
//! the TypeError of ToString and ToNumber of one (`#sec-tostring`, `#sec-tonumber`). The design
//! is `docs/design/engine.md`, decision 4, "Symbols": the registry is the agent's, a root of the
//! collector, charged to the memory budget and never freed; `Symbol.for` charges a step per code
//! unit of its key.
//!
//! Scripts run in sloppy and in strict code. No test uses an iterator, `for`-`of`, a spread or
//! a computed property name. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, agent_with, get, get_object, global, limits, object, realm};
use support::script::{Outcome, eval, eval_text, run_both};
use swell::{Agent, Outcome as HostOutcome, Realm, Root, Termination, Value};

/// `source` answers the String `expected` in sloppy and in strict code.
fn answers(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(format!("\"{expected}\""));
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// The value the script `source` completes with in `realm`; the root of the outcome is
/// dropped, so a value that nothing else holds may be collected at the next collection.
fn value_of(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> Value {
    rooted_value_of(agent, realm, source).1
}

/// The value the script `source` completes with in `realm`, with the root that holds it.
fn rooted_value_of(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> (Root, Value) {
    match eval(agent, realm, source) {
        HostOutcome::Normal(root) => {
            let value = agent.with_cx(realm, |cx| cx.get(&root).unwrap()).unwrap();
            (root, value)
        }
        HostOutcome::Throw(_) => panic!("{source} threw"),
        HostOutcome::Terminated(termination) => panic!("{source} ended in {termination:?}"),
    }
}

/// `#sec-symbol-description`: each call of `Symbol` answers a new Symbol, whatever its
/// description; the [[Description]] is ToString of the argument, or undefined for an absent or
/// undefined one; `new Symbol()` throws a `TypeError` (step 1), and so does a construct with an
/// object description before ToString runs. `Symbol` has "length" 0 and "name" "Symbol", the
/// [[Prototype]] `%Function.prototype%` (`#sec-properties-of-the-symbol-constructor`), and its
/// "prototype" is not writable, not enumerable and not configurable (`#sec-symbol.prototype`).
#[test]
fn symbol_makes_a_new_symbol_each_call_and_new_symbol_throws() {
    answers(
        "var a = Symbol(), b = Symbol(), c = Symbol(\"x\"), d = Symbol(\"x\");\n\
         [typeof a, a === b, c === d, c === c, a !== b].join();",
        "symbol,false,false,true,true",
    );
    answers(
        "var o = { toString: function () { return \"from toString\"; } };\n\
         [Symbol().description === undefined, Symbol(undefined).description === undefined,\n\
          Symbol(\"\").description === \"\", Symbol(null).description,\n\
          Symbol(1.5).description, Symbol(o).description].join();",
        "true,true,true,null,1.5,from toString",
    );
    answers(
        "var called = false, r = [];\n\
         try { new Symbol(); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { new Symbol({ toString: function () { called = true; return \"\"; } }); }\n\
         catch (e) { r.push(e.constructor === TypeError); }\n\
         r.push(called);\n\
         r.join();",
        "true,true,false",
    );
    answers(
        "var p = Object.getOwnPropertyDescriptor(Symbol, \"prototype\");\n\
         [Symbol.length, Symbol.name, typeof Symbol,\n\
          Object.getPrototypeOf(Symbol) === Function.prototype,\n\
          p.writable, p.enumerable, p.configurable,\n\
          Object.getPrototypeOf(Symbol.prototype) === Object.prototype,\n\
          Symbol.prototype.constructor === Symbol].join();",
        "0,Symbol,function,true,false,false,false,true,true",
    );
}

/// `#sec-symbol.for`: one Symbol per key, made the first time and found after; the key is
/// ToString of the argument, so `1` and `"1"` name one Symbol and a Symbol argument throws a
/// `TypeError`. The registry is the agent's (`#sec-agents`, [[GlobalSymbolRegistry]]): realm B
/// answers the Symbol realm A registered, and `Symbol.keyFor` of B answers its key.
#[test]
fn symbol_for_answers_one_symbol_per_key_in_every_realm_of_the_agent() {
    answers(
        "var r = [Symbol.for(\"app\") === Symbol.for(\"app\"), Symbol.for(\"x\") === Symbol.for(\"y\"),\n\
          Symbol.for(1) === Symbol.for(\"1\"), Symbol.for() === Symbol.for(\"undefined\"),\n\
          Symbol.for(\"app\").description, Symbol.for(\"app\") === Symbol(\"app\")];\n\
         try { Symbol.for(Symbol()); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.join();",
        "true,false,true,true,app,false,true",
    );
    let mut agent = agent().unwrap();
    let a = realm(&mut agent);
    let b = realm(&mut agent);
    let in_a = value_of(&mut agent, &a, "Symbol.for(\"app\")");
    assert!(matches!(in_a, Value::Symbol(_)), "{in_a:?}");
    let global_b = global(&mut agent, &b);
    agent
        .with_cx(&b, |cx| {
            let global = object(cx, &global_b);
            let name = support::host::key(cx, "fromA");
            assert!(cx.set(global, name, in_a, true).unwrap().unwrap());
        })
        .unwrap();
    let in_b = value_of(&mut agent, &b, "Symbol.for(\"app\")");
    assert_eq!(in_a, in_b);
    assert_eq!(
        eval_text(
            &mut agent,
            &b,
            "[Symbol.for(\"app\") === fromA, Symbol.keyFor(fromA)].join();"
        ),
        Outcome::Normal("\"true,app\"".to_string())
    );
}

/// `#sec-symbol.keyfor` with KeyForSymbol (`#sec-keyforsymbol`): the key of a Symbol
/// `Symbol.for` made; undefined for a Symbol `Symbol` made, whatever its description, and for a
/// well-known symbol, which is in no registry; a value that is not a Symbol, a Symbol object
/// included, throws a `TypeError`. "length" 1, "name" "keyFor"; `Symbol.for` has "length" 1.
#[test]
fn symbol_key_for_answers_the_key_of_a_registered_symbol_only() {
    answers(
        "var r = [Symbol.keyFor(Symbol.for(\"k\")), Symbol.keyFor(Symbol(\"k\")) === undefined,\n\
          Symbol.keyFor(Symbol.for(\"\")) === \"\", Symbol.keyFor(Symbol.iterator) === undefined,\n\
          Symbol.keyFor(Symbol.hasInstance) === undefined];\n\
         var values = [\"k\", undefined, null, 1, {}, Object(Symbol.for(\"k\"))];\n\
         for (var i = 0; i < values.length; i++) {\n\
           try { Symbol.keyFor(values[i]); r.push(\"answered\"); }\n\
           catch (e) { r.push(e.constructor === TypeError); }\n\
         }\n\
         r.push(Symbol.keyFor.length, Symbol.keyFor.name, Symbol.for.length, Symbol.for.name);\n\
         r.join();",
        "k,true,true,true,true,true,true,true,true,true,true,1,keyFor,1,for",
    );
}

/// `#sec-well-known-symbols` and `#sec-properties-of-the-symbol-constructor`: the constructor
/// has twelve own properties whose values are Symbols, each a well-known symbol whose
/// [[Description]] is "Symbol." and its name (Table 1), not writable, not enumerable and not
/// configurable; `Symbol.asyncIterator` (slice 10) is not one of them. They are "shared by all
/// realms" (6.1.5.1): two realms of one agent read the same twelve Symbols. The script runs in
/// sloppy code only, in the first of the two realms: no step of it differs in strict code, and
/// with the feature `collect-every-safe-point` each of its instructions collects.
#[test]
fn the_well_known_symbols_are_twelve_and_the_same_in_every_realm() {
    const NAMES: [&str; 12] = [
        "hasInstance",
        "isConcatSpreadable",
        "iterator",
        "match",
        "matchAll",
        "replace",
        "search",
        "species",
        "split",
        "toPrimitive",
        "toStringTag",
        "unscopables",
    ];
    let listed = NAMES.join(",");
    let each: Vec<String> = NAMES
        .iter()
        .map(|name| format!("Symbol.{name}:false:false:false:true"))
        .collect();
    let mut agent = agent().unwrap();
    let a = realm(&mut agent);
    let b = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &a,
            "var names = Object.getOwnPropertyNames(Symbol), found = [], each = [];\n\
             for (var i = 0; i < names.length; i++)\n\
               if (typeof Symbol[names[i]] === \"symbol\") found.push(names[i]);\n\
             found.sort();\n\
             for (i = 0; i < found.length; i++) {\n\
               var d = Object.getOwnPropertyDescriptor(Symbol, found[i]);\n\
               each.push([d.value.description, d.writable, d.enumerable, d.configurable,\n\
                          Symbol.keyFor(d.value) === undefined].join(\":\"));\n\
             }\n\
             [found.join(), each.join(), Symbol.asyncIterator === undefined,\n\
              \"asyncIterator\" in Symbol, Symbol.iterator === Symbol(\"Symbol.iterator\")].join(\"|\");"
        ),
        Outcome::Normal(format!("\"{listed}|{}|true|false|false\"", each.join(",")))
    );
    let read = |agent: &mut Agent<TestHost>, realm: &Realm| -> Vec<Value> {
        let global_root = global(agent, realm);
        agent
            .with_cx(realm, |cx| {
                let global = object(cx, &global_root);
                let symbol = get_object(cx, global, "Symbol");
                NAMES.iter().map(|name| get(cx, symbol, name)).collect()
            })
            .unwrap()
    };
    let in_a = read(&mut agent, &a);
    let in_b = read(&mut agent, &b);
    assert_eq!(in_a, in_b);
    for (index, value) in in_a.iter().enumerate() {
        assert!(matches!(value, Value::Symbol(_)), "{value:?}");
        assert!(!in_a[index + 1..].contains(value), "{}", NAMES[index]);
    }
}

/// `#sec-toobject`, step for a Symbol: "Return a new Symbol object whose [[SymbolData]]
/// internal slot is set to argument" (`#sec-properties-of-symbol-instances`), whose
/// [[Prototype]] is `%Symbol.prototype%`. Its `valueOf` (thisSymbolValue,
/// `#sec-thissymbolvalue`) answers the Symbol; `toString` and `description` read it; its class
/// string is "Symbol" (`#sec-symbol.prototype-%symbol.tostringtag%`); each ToObject makes a new
/// object. The host's `to_object` of a Symbol makes one too.
#[test]
fn to_object_of_a_symbol_is_a_wrapper_whose_value_of_is_the_symbol() {
    answers(
        "var s = Symbol(\"w\"), o = Object(s);\n\
         [typeof o, o.valueOf() === s, Object.getPrototypeOf(o) === Symbol.prototype,\n\
          o instanceof Symbol, o.toString(), o.description,\n\
          Object.prototype.toString.call(o), Object(s) === o, o[Symbol.toPrimitive](\"number\") === s,\n\
          s.constructor === Symbol, Object.prototype.toString.call(s)].join();",
        "object,true,true,true,Symbol(w),w,[object Symbol],false,true,true,[object Symbol]",
    );
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let (_held, symbol) = rooted_value_of(&mut agent, &realm, "Symbol(\"host\")");
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let wrapper = cx.to_object(symbol).unwrap().unwrap();
            let global = object(cx, &global_root);
            let constructor = get_object(cx, global, "Symbol");
            let prototype = get_object(cx, constructor, "prototype");
            assert_eq!(cx.get_prototype_of(wrapper).unwrap(), Ok(Some(prototype)));
            let value_of = get(cx, prototype, "valueOf");
            assert_eq!(
                cx.call(value_of, Value::Object(wrapper), &[]).unwrap(),
                Ok(symbol)
            );
        })
        .unwrap();
}

/// `#sec-tostring` and `#sec-tonumber`: ToString and ToNumber of a Symbol throw a `TypeError`,
/// so `"" + s`, a template with `s`, `+s`, `s < 1` and `s * 1` throw; `String(s)`
/// (`#sec-string-constructor-string-value`, step 2.a) and `s.toString()`
/// (SymbolDescriptiveString, `#sec-symboldescriptivestring`) answer "Symbol(desc)"; a Symbol
/// is a property key as it is (`#sec-topropertykey`).
#[test]
fn to_string_and_to_number_of_a_symbol_throw_type_error() {
    answers(
        "var s = Symbol(\"desc\"), r = [];\n\
         var tries = [function () { return \"\" + s; }, function () { return `${s}`; },\n\
                      function () { return +s; }, function () { return s < 1; },\n\
                      function () { return s * 1; }, function () { return new String(s); }];\n\
         for (var i = 0; i < tries.length; i++) {\n\
           try { tries[i](); r.push(\"answered\"); } catch (e) { r.push(e.constructor === TypeError); }\n\
         }\n\
         var o = {}; o[s] = 1;\n\
         r.push(String(s), s.toString(), String(Symbol()), o[s], Object.keys(o).length);\n\
         r.join();",
        "true,true,true,true,true,true,Symbol(desc),Symbol(desc),Symbol(),1,0",
    );
}

/// The code units of a key of `length` units, as a script writes it.
fn long_key(letter: char, length: usize) -> String {
    format!("\"{letter}\".repeat({length})")
}

/// `docs/design/engine.md`, "Symbols", item 2: the registry is a root of the collector. A
/// Symbol `Symbol.for` made for a key of 10,000 code units, which nothing else holds, survives
/// `collect()`: the charged reading does not fall by its key's 10,000 bytes, and `Symbol.for`
/// of the same key after the collection answers the same Symbol (its handle, with the slot's
/// generation, is the one before). The control, a Symbol `Symbol` made with a description of
/// 10,000 code units and held by nothing, is freed: the charged and the live reading each fall
/// by at least the 10,000 bytes of its description.
#[test]
fn a_registered_symbol_survives_a_collection_and_an_unregistered_one_is_freed() {
    const UNITS: u64 = 10_000;
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let registered = value_of(
        &mut agent,
        &realm,
        &format!("Symbol.for({})", long_key('r', 10_000)),
    );
    assert!(matches!(registered, Value::Symbol(_)), "{registered:?}");
    let before = agent.heap_bytes().unwrap();
    agent.collect().unwrap();
    let after = agent.heap_bytes().unwrap();
    assert!(
        before - after < UNITS,
        "the registered Symbol's key was freed: {before} before, {after} after"
    );
    let again = value_of(
        &mut agent,
        &realm,
        &format!("Symbol.for({})", long_key('r', 10_000)),
    );
    assert_eq!(again, registered, "a new Symbol after the collection");

    let unregistered = value_of(
        &mut agent,
        &realm,
        &format!("Symbol({})", long_key('u', 10_000)),
    );
    assert!(matches!(unregistered, Value::Symbol(_)), "{unregistered:?}");
    let (charged, live) = (agent.heap_bytes().unwrap(), agent.live_bytes().unwrap());
    agent.collect().unwrap();
    let (charged_after, live_after) = (agent.heap_bytes().unwrap(), agent.live_bytes().unwrap());
    assert!(
        charged - charged_after >= UNITS,
        "heap_bytes {charged} before, {charged_after} after"
    );
    assert!(
        live - live_after >= UNITS,
        "live_bytes {live} before, {live_after} after"
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "Symbol.keyFor(Symbol.for({})).length",
                long_key('r', 10_000)
            )
        ),
        Outcome::Normal("10000".to_string())
    );
}

/// `docs/design/engine.md`, "Symbols", item 3: `Symbol.for` charges a step per code unit of
/// its key. With both keys made by an earlier script, `last_entry_steps()` of the entry that
/// runs `Symbol.for` of a key of 1,000 code units exceeds that of a key of 10 by at least 990;
/// each is read right after `evaluate_script`, before another entry.
#[test]
fn symbol_for_charges_a_step_per_code_unit_of_its_key() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "var long = {}, short = {}; 1",
                long_key('l', 1_000),
                long_key('s', 10)
            )
        ),
        Outcome::Normal("1".to_string())
    );
    let mut steps = Vec::new();
    for source in ["Symbol.for(long); 1", "Symbol.for(short); 1"] {
        let outcome = eval(&mut agent, &realm, source);
        assert!(matches!(outcome, HostOutcome::Normal(_)), "{source}");
        steps.push(agent.last_entry_steps().unwrap());
    }
    assert!(
        steps[0] >= steps[1] + 990,
        "{} steps for 1,000 units, {} for 10",
        steps[0],
        steps[1]
    );
}

/// The loop of [`the_registry_is_charged_to_the_memory_budget`]: `keys` Symbols made by `make`
/// (`Symbol.for` or `Symbol`), each for `long + String(i)`, a key of 1,024 code units and the
/// digits of `i`, so that a registration keeps about 3 KiB and the loop is short.
fn registry_loop(make: &str, keys: u64) -> String {
    format!(
        "var long = {}; for (var i = 0; i < {keys}; i++) {make}(long + String(i)); i",
        long_key('k', 1_024)
    )
}

/// The agent and realm of [`the_registry_is_charged_to_the_memory_budget`] with `memory_bytes`
/// `budget` and no step budget to speak of, after a collection.
fn registry_agent(budget: u64) -> (Agent<TestHost>, Realm) {
    let mut budgeted = limits();
    budgeted.memory_bytes = budget;
    budgeted.steps_per_entry = 1 << 40;
    let mut agent = agent_with(budgeted).unwrap();
    let realm = realm(&mut agent);
    agent.collect().unwrap();
    (agent, realm)
}

/// `docs/design/engine.md`, "Symbols", item 3: the registry is charged to the memory budget and
/// no sweep credits it. Measured first, each in an agent of its own: the charged reading C0 and
/// the live reading L0 after a realm and its collection; the bytes a registration keeps, from
/// the live reading after 100 registrations ([`registry_loop`]) and a collection (10 with the
/// feature `collect-every-safe-point`); and the charged reading C of the control, the loop over
/// `Symbol`, whose Symbols nothing keeps, run to its end with a budget of 1 GiB, which holds the
/// capacity the arenas grew to. The room R past C is the collection threshold, the garbage a
/// safe point lets grow before it collects: L0, at least 1 MiB (`engine.md`, "Collector"); with
/// the feature, where every safe point collects, 16 KiB. The loop runs over as many keys as
/// keep 3 R. With `memory_bytes` C + R the loop over `Symbol.for` ends in
/// `Terminated(MemoryBudget)` and `heap_bytes()` then reads at most the budget; the control
/// completes normally under the same budget.
#[test]
fn the_registry_is_charged_to_the_memory_budget() {
    let collect_mode = cfg!(feature = "collect-every-safe-point");
    let probe_keys: u64 = if collect_mode { 10 } else { 100 };
    let (mut probe, probe_realm) = registry_agent(1 << 30);
    let (charged, live) = (probe.heap_bytes().unwrap(), probe.live_bytes().unwrap());
    assert_eq!(
        eval_text(
            &mut probe,
            &probe_realm,
            &registry_loop("Symbol.for", probe_keys)
        ),
        Outcome::Normal(probe_keys.to_string())
    );
    probe.collect().unwrap();
    let per_key = (probe.live_bytes().unwrap() - live) / probe_keys;
    assert!(per_key > 2 * 1_024, "a registration keeps {per_key} bytes");

    let room = if collect_mode {
        16 * 1024
    } else {
        live.max(1 << 20)
    };
    let keys = 3 * room / per_key;
    let (mut measure, measure_realm) = registry_agent(1 << 30);
    assert_eq!(
        eval_text(&mut measure, &measure_realm, &registry_loop("Symbol", keys)),
        Outcome::Normal(keys.to_string())
    );
    let control = measure.heap_bytes().unwrap();
    let budget = control + room;
    assert!(
        keys * per_key > budget - charged,
        "{keys} registrations of {per_key} bytes fit {} bytes past the realm",
        budget - charged
    );
    for (make, expected) in [
        ("Symbol.for", Outcome::Terminated(Termination::MemoryBudget)),
        ("Symbol", Outcome::Normal(keys.to_string())),
    ] {
        let (mut agent, realm) = registry_agent(budget);
        assert_eq!(agent.heap_bytes().unwrap(), charged, "the same realm");
        assert_eq!(
            eval_text(&mut agent, &realm, &registry_loop(make, keys)),
            expected,
            "{make}: {keys} keys, {per_key} bytes a key, budget {budget}, realm {charged}"
        );
        let bytes = agent.heap_bytes().unwrap();
        assert!(bytes <= budget, "{make}: {bytes} bytes, budget {budget}");
    }
}

/// `#sec-object.getownpropertysymbols` with GetOwnPropertyKeys (`#sec-getownpropertykeys`,
/// type symbol) and OrdinaryOwnPropertyKeys (`#sec-ordinaryownpropertykeys`): the Symbol keys of
/// an object in the order they were defined, non-enumerable ones too, none of its String keys
/// and a key deleted and defined again at its new place; an empty Array for an object without
/// one; for a primitive the keys of the object ToObject makes of it (none for a String, a
/// Number, a Boolean or a Symbol); a `TypeError` for undefined and null. The answer is a new
/// Array each call. "length" 1, "name" "getOwnPropertySymbols", writable, not enumerable,
/// configurable (clause 18). The design has no host hook for own keys
/// (`crates/swell/src/host/host_object.rs`), and no test of `getOwnPropertyNames` reads one, so
/// no host object is asked here.
#[test]
fn get_own_property_symbols_answers_the_symbol_keys_in_the_order_they_were_defined() {
    answers(
        "var a = Symbol(\"a\"), b = Symbol(\"b\"), c = Symbol(\"c\"), d = Symbol(\"d\");\n\
         var o = { x: 1 };\n\
         o[b] = 1; o[\"1\"] = 2; o[a] = 3; o.y = 4;\n\
         Object.defineProperty(o, c, { value: 5, enumerable: false });\n\
         o[d] = 6; delete o[a]; o[a] = 7;\n\
         var k = Object.getOwnPropertySymbols(o);\n\
         [k.length, k[0] === b, k[1] === c, k[2] === d, k[3] === a, Array.isArray(k),\n\
          Object.getOwnPropertySymbols(o) !== k].join();",
        "4,true,true,true,true,true,true",
    );
    answers(
        "var e = Object.getOwnPropertySymbols({ x: 1, 0: 2 });\n\
         [Array.isArray(e), e.length, Object.getOwnPropertySymbols([1, 2]).length,\n\
          Object.getOwnPropertySymbols(\"ab\").length, Object.getOwnPropertySymbols(5).length,\n\
          Object.getOwnPropertySymbols(true).length,\n\
          Object.getOwnPropertySymbols(Symbol(\"p\")).length].join();",
        "true,0,0,0,0,0,0",
    );
    answers(
        "var r = [];\n\
         try { Object.getOwnPropertySymbols(undefined); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { Object.getOwnPropertySymbols(null); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { Object.getOwnPropertySymbols(); } catch (e) { r.push(e instanceof TypeError); }\n\
         var f = Object.getOwnPropertySymbols;\n\
         var p = Object.getOwnPropertyDescriptor(Object, \"getOwnPropertySymbols\");\n\
         r.push(typeof f, f.length, f.name, p.writable, p.enumerable, p.configurable);\n\
         r.join();",
        "true,true,true,function,1,getOwnPropertySymbols,true,false,true",
    );
}

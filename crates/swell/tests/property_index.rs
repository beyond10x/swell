//! The object model of `docs/design/engine.md`, decision 4, "Objects", as scripts see it: a
//! property is found through an index map above a small size and an Array's elements are
//! stored densely, so that a lookup's time does not grow with the object; the bindings a
//! non-strict direct `eval` adds are found the same way; a scan that stays is charged a step per
//! entry it visits; and nothing a script can observe depends on the map (its order or its
//! hash key). Written by `story:property-index-and-dense-elements`.
//!
//! The ratio of times the story's acceptance holds is measured by hand with the release runner
//! (`tests/probes/property-index/run.sh`), not here. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::host::{TestHost, agent_with, global, key, limits, object, realm, seeds};
use support::script::{Outcome, describe, eval_text, parse};
use swell::{
    Agent, Brand, HostObjectSpec, Limits, Outcome as HostOutcome, Prototype, Realm, Seeds,
    Termination, Value,
};

/// HOST-10 with a budget a debug build fills in seconds: `var a = []; while (true)
/// a.push({});` with `memory_bytes` 8 MiB ends in `Terminated(MemoryBudget)` and `heap_bytes()`
/// then reads at most the budget. Before the index map and the dense storage each `push`
/// scanned the Array's properties, so the run took time that grows with the square of what it
/// held (64 MiB did not end in nine minutes, `docs/plan/slice-01-wave-6.md`). With the feature
/// `collect-every-safe-point` the budget is the realm and 64 KiB, as HOST-10 has it there: a
/// collection at every safe point traces the whole heap, so that mode's time grows with the
/// square of the heap whatever the lookup costs (1 MiB ran over 60 s there, measured).
///
/// The room `1 + 1` then has is bounded from below in both modes: the host holds 4 KiB charged
/// (`charge_host_bytes`) during the loop, on top of the budget, and gives them back after it
/// (`credit_host_bytes`); `heap_bytes()` is at most `memory_bytes` after the termination
/// (HOST-171), so at least 4 KiB are free, and `1 + 1` needs 2,322 bytes of room above
/// `heap_bytes()`. Before, the room was what the loop's last refused growth happened to leave:
/// with a collection at every safe point `1 + 1` had 3,549 bytes there and needed 3,895. The
/// test took 0.53 s in the default mode and 0.47 s with a collection at every safe point
/// (cargo-nextest, 2026-11-22).
#[test]
fn memory_budget_of_8_mib_ends_a_push_loop() {
    const HELD: u64 = 4 * 1024;
    let mut limits = limits();
    limits.memory_bytes = HELD
        + if cfg!(feature = "collect-every-safe-point") {
            let mut agent = agent_with(support::host::limits()).unwrap();
            let _realm = realm(&mut agent);
            assert!(agent.collect().is_ok());
            agent.heap_bytes().unwrap() + 65_536
        } else {
            8 << 20
        };
    limits.steps_per_entry = u64::MAX;
    let budget = limits.memory_bytes;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let held = agent
        .with_cx(&realm, |cx| cx.charge_host_bytes(HELD))
        .unwrap()
        .unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "var a = []; while (true) a.push({});"),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    assert!(agent.heap_bytes().unwrap() <= budget);
    assert_eq!(agent.credit_host_bytes(held).unwrap(), HELD);
    assert!(budget - agent.heap_bytes().unwrap() >= HELD);
    assert_eq!(
        eval_text(&mut agent, &realm, "1 + 1"),
        Outcome::Normal("2".to_string())
    );
}

/// OrdinaryOwnPropertyKeys (`#sec-ordinaryownpropertykeys`) past the small size: the integer
/// keys ascending, then the String keys in creation order, after additions and deletions on
/// both sides of the size; on an Array, the dense and the sparse elements merged ascending.
#[test]
fn own_keys_keep_their_order_past_the_small_size() {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    let script = "var o = {};
        for (var i = 0; i < 20; i++) { o['s' + i] = i; o[39 - i] = i; }
        delete o.s3; delete o[38]; delete o.s15; delete o[22];
        for (var i = 0; i < 20; i++) o['t' + i] = i;
        o[100] = 0; o[50] = 0; delete o.t0;
        var a = [];
        for (var i = 0; i < 20; i++) a[i] = i;
        a.x = 1; a[4000000000] = 1; a[100] = 1; a.y = 2; delete a[3];
        Object.getOwnPropertyNames(o).join(',') + '|' + Object.getOwnPropertyNames(a).join(',')";
    let mut object: Vec<String> = (20..40)
        .filter(|i| *i != 38 && *i != 22)
        .map(|i| i.to_string())
        .collect();
    object.extend(["50".to_string(), "100".to_string()]);
    object.extend(
        (0..20)
            .filter(|i| *i != 3 && *i != 15)
            .map(|i| format!("s{i}")),
    );
    object.extend((1..20).map(|i| format!("t{i}")));
    let mut array: Vec<String> = (0..20).filter(|i| *i != 3).map(|i| i.to_string()).collect();
    array.extend(["100", "4000000000", "length", "x", "y"].map(String::from));
    // 4000000000 is an array index, below 2^32 - 1.
    let expected = format!("\"{}|{}\"", object.join(","), array.join(","));
    assert_eq!(
        eval_text(&mut agent, &realm, script),
        Outcome::Normal(expected)
    );
}

/// EvalDeclarationInstantiation (`#sec-evaldeclarationinstantiation`, steps 17 and 18): the
/// bindings non-strict direct `eval` adds to a function's record, 100 of them, past the small
/// size, are each found by name, deletable, and a deleted one is not found again.
#[test]
fn eval_added_bindings_are_found_past_the_small_size() {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    let script = "function f() {
            for (var i = 0; i < 100; i++) eval('var x' + i + ' = ' + i);
            var s = 0;
            for (var i = 0; i < 100; i++) s += eval('x' + i);
            var deleted = delete x5;
            eval('var x5 = 500');
            var again = x5;
            delete x7;
            return s + ',' + deleted + ',' + again + ',' + typeof x7 + ',' + x99;
        }
        f()";
    assert_eq!(
        eval_text(&mut agent, &realm, script),
        Outcome::Normal("\"4950,true,500,undefined,99\"".to_string())
    );
}

/// The names of a record's slots stay a scan of a table of the compiled code: up to the small
/// size (8) a lookup's comparisons are covered by the step the lookup pays for the record, and
/// each comparison past it is a step. A function whose body holds a `with` statement has its 30
/// variables in a named record, and a name in the `with` body is looked up by name: the first
/// variable is found at the first comparison, the last at the 30th, 22 past the small size.
/// ResolveBinding scans the record once and GetBindingValue once more, so reading the last costs
/// 44 steps more than reading the first.
#[test]
fn slot_names_past_the_small_size_charge_a_step_per_comparison() {
    let names: Vec<String> = (10..40).map(|i| format!("v{i}")).collect();
    let steps = |name: &str| -> u64 {
        let mut agent = agent_with(limits()).unwrap();
        let realm = realm(&mut agent);
        let script = format!(
            "function f() {{ var {}; with ({{}}) {{ return {name}; }} }} f()",
            names.join(", ")
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &script),
            Outcome::Normal("undefined".to_string())
        );
        agent.last_entry_steps().unwrap()
    };
    let first = steps("v10");
    let last = steps("v39");
    assert!(last > first, "the scan was not charged: {first} and {last}");
    assert_eq!(last - first, 44, "{first} and {last}");
}

/// Determinism (`docs/design/host-interface.md`, decision 3): a script of 10,000 properties,
/// with deletions and an Array of 10,000 elements, spends the same steps and holds the same
/// heap bytes in two agents of the same seeds, and in one of another `hash_key`: no order or
/// charge comes from the map's iteration or its hash. With the feature `collect-every-safe-point`
/// the script has 50 of each, still past the small size: that mode collects at every safe point,
/// and every iteration of a loop is one (10,000 ran over 60 s there and 1,000 took 56.4 s,
/// measured). In the default mode's gate it has 2,000 of each (10,000 took 26.5 s there at a
/// load of 100 to 110 on 17 cores, cargo-nextest): the map still grows from the small size
/// through several sizes, with every third property deleted; the lane `slow`
/// (`SWELL_FULL_COUNTS=1`) runs 10,000.
#[test]
fn the_same_seeds_give_the_same_steps_and_heap_bytes() {
    let full = std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1");
    let (n, expected) = if cfg!(feature = "collect-every-safe-point") {
        (50, "\"33,p1,p49,49\"")
    } else if full {
        (10_000, "\"6666,p1,p9998,9999\"")
    } else {
        (2_000, "\"1333,p1,p1999,1999\"")
    };
    let script = format!(
        "var o = {{}};
        for (var i = 0; i < {n}; i++) o['p' + i] = i;
        for (var i = 0; i < {n}; i += 3) delete o['p' + i];
        var a = [];
        for (var i = 0; i < {n}; i++) a.push(i);
        var names = Object.getOwnPropertyNames(o);
        names.length + ',' + names[0] + ',' + names[names.length - 1] + ',' + a[{n} - 1]"
    );
    let run = |seeds: Seeds| {
        let mut agent = Agent::new(support::host::TestHost::default(), limits(), seeds).unwrap();
        let realm = realm(&mut agent);
        let outcome = eval_text(&mut agent, &realm, &script);
        (
            outcome,
            agent.last_entry_steps().unwrap(),
            agent.heap_bytes().unwrap(),
        )
    };
    let one = run(seeds());
    let two = run(seeds());
    let mut other = seeds();
    other.hash_key[0] ^= 1;
    let three = run(other);
    assert_eq!(one.0, Outcome::Normal(expected.to_string()));
    assert_eq!(one, two);
    assert_eq!(one, three);
}

/// The code units of the long key of HOST-182.
const LONG_KEY: u64 = 1_000_000;

/// An agent with `limits` and a realm whose global object has `o`, an object the host made
/// (its [[Prototype]] `%Object.prototype%`), `k`, a String of [`LONG_KEY`] code units, and
/// `q`, a String of one code unit, each defined by the host.
fn agent_with_long_key(limits: Limits) -> (Agent<TestHost>, Realm) {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let o = cx
                .new_host_object(HostObjectSpec {
                    prototype: Prototype::ObjectPrototypeOf(realm_ref),
                    brand: Brand(1),
                    data: None,
                    error_kind: None,
                })
                .unwrap()
                .unwrap();
            let k = cx.new_string_utf16(&vec![0x6B; LONG_KEY as usize]).unwrap();
            let q = cx.new_string_utf16(&[0x71]).unwrap();
            for (name, value) in [
                ("o", Value::Object(o)),
                ("k", Value::String(k)),
                ("q", Value::String(q)),
            ] {
                let name = key(cx, name);
                cx.set(global, name, value, true).unwrap().unwrap();
            }
        })
        .unwrap();
    (agent, realm)
}

/// HOST-182: a String made a property key charges a step per code unit past 32 before it is
/// hashed (`docs/design/engine.md`, "The step budget"; ToPropertyKey, `#sec-topropertykey`).
/// `o[k]` with the host's key `k` of 1,000,000 code units spends at least 999,968 steps more
/// than `o[q]` with the key `q` of one; with `steps_per_entry` 10,000 and a `collect()` before
/// the entry it ends in `Terminated(StepBudget)`, and `heap_bytes()` after the entry reads what
/// it read before: no atom was made.
#[test]
fn host_182_a_long_key_is_charged_before_it_is_interned() {
    let mut roomy = limits();
    roomy.steps_per_entry = 10_000_000;
    let (mut agent, realm) = agent_with_long_key(roomy);
    let mut counted = |source: &str| {
        let script = parse(&mut agent, &realm, source);
        let outcome = agent.evaluate_script(&script).unwrap();
        let steps = agent.last_entry_steps().unwrap();
        (describe(&mut agent, &realm, outcome), steps)
    };
    let undefined = Outcome::Normal("undefined".to_string());
    let (outcome, short) = counted("o[q]");
    assert_eq!(outcome, undefined);
    let (outcome, long) = counted("o[k]");
    assert_eq!(outcome, undefined);
    assert!(
        long >= short + (LONG_KEY - 32),
        "o[k] spent {long} steps, o[q] {short}"
    );

    let mut tight = limits();
    tight.steps_per_entry = 10_000;
    let (mut agent, realm) = agent_with_long_key(tight);
    let script = parse(&mut agent, &realm, "o[k]");
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    let before = agent.heap_bytes().unwrap();
    let outcome = agent.evaluate_script(&script).unwrap();
    let after = agent.heap_bytes().unwrap();
    assert_eq!(
        describe(&mut agent, &realm, outcome),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(after, before, "an atom was made");
}

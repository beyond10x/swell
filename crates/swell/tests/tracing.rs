//! The trace functions and root sites of `story:interpreter-and-script-entry-points`
//! (`docs/design/engine.md`, decision 4): each test reads, after collections, a value that only
//! the trace or root site it names keeps alive, so it fails when that site reports nothing (a
//! freed object read again ends the run in `Terminated(EngineFault)`). Run also with the feature
//! `collect-every-safe-point`, where every instruction boundary collects.
#![cfg(test)]

mod support;

use support::host::{agent, agent_with, limits, realm};
use support::script::{Outcome, eval, eval_text, parse};
use swell::{AgentState, Outcome as HostOutcome};

/// A collection between two entries, which must answer `Normal`.
fn collect(agent: &mut swell::Agent<support::host::TestHost>) {
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
}

/// The iterations of a garbage loop: enough to pass the collection threshold (1 MiB) several
/// times in the default build, and few where every safe point collects.
const GARBAGE: u32 = if cfg!(feature = "collect-every-safe-point") {
    20
} else {
    100_000
};

/// `closure_environment_survives_collect`: an object reachable only through the environment a
/// returned closure captured is read after collections. The trace of environment records
/// (`env/mod.rs`) and of the ECMAScript function kind, its [[Environment]]
/// (`object/ecmascript_function.rs`).
#[test]
fn closure_environment_survives_collect() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function outer() { var o = { v: 40 }; \
               return function middle() { var p = { w: 2 }; \
                 return function inner() { return o.v + p.w; }; }; } \
             var get = outer()(); typeof get;"
        ),
        Outcome::Normal("\"function\"".to_string())
    );
    collect(&mut agent);
    collect(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "get() + get();"),
        Outcome::Normal("84".to_string())
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// `operand_stack_and_local_values_survive_collect`: an object held only on the operand stack,
/// as the pending argument of a call whose next argument allocates, and one held only in a local
/// slot are read after the collections at the safe points between. The root site of the frame
/// stack (`interp/mod.rs`).
///
/// The memory budget, 4 MiB, is smaller than the garbage the default build allocates (100,000
/// arrays of four elements), so the run completes only if collections ran during it.
#[test]
fn operand_stack_and_local_values_survive_collect() {
    let mut limits = limits();
    limits.memory_bytes = 4 * 1024 * 1024;
    limits.steps_per_entry = 100_000_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let source = format!(
        "function garbage(n) {{ var i = 0; while (i < n) {{ [i, i, i, i]; i++; }} return n; }} \
         function pair(a, b) {{ return a.v + b; }} \
         function local() {{ var kept = {{ v: 2 }}; garbage({GARBAGE}); return kept.v; }} \
         pair({{ v: 1 }}, garbage({GARBAGE})) + local() + garbage({GARBAGE});"
    );
    assert_eq!(
        eval_text(&mut agent, &realm, &source),
        Outcome::Normal((3 + 2 * GARBAGE).to_string())
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// `global_environment_survives_collect`: with only the `Realm` root kept, a collection, then a
/// second script reads a global `var` binding and a global function the first declared. The
/// global environment record (`env/global.rs`) and the realm record.
#[test]
fn global_environment_survives_collect() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    {
        let script = parse(
            &mut agent,
            &realm,
            "var kept = { v: 5 }; function read() { return kept.v; }",
        );
        let outcome = agent.evaluate_script(&script).unwrap();
        assert!(matches!(outcome, HostOutcome::Normal(_)));
    }
    collect(&mut agent);
    collect(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "typeof read + (read() + kept.v);"),
        Outcome::Normal("\"function10\"".to_string())
    );
}

/// `script_record_survives_collect`: a parsed `Script` evaluated after a collection; the trace
/// of script records (`script.rs`), its realm and the strings of its constants.
#[test]
fn script_record_survives_collect() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let script = parse(
        &mut agent,
        &realm,
        "var x = { key: \"constant\" + 1 }; x.key + x[\"key\"].length;",
    );
    // Garbage that the collection frees, so that the slots of the script's strings could be
    // reused if the script did not report them.
    assert!(matches!(
        eval(
            &mut agent,
            &realm,
            "var g = 0; while (g < 50) { [\"a\" + g]; g++; }"
        ),
        HostOutcome::Normal(_)
    ));
    for _ in 0..2 {
        collect(&mut agent);
        let outcome = agent.evaluate_script(&script).unwrap();
        assert_eq!(
            support::script::describe(&mut agent, &realm, outcome),
            Outcome::Normal("\"constant19\"".to_string())
        );
    }
}

/// `object_environment_survives_collect`: an object reachable only through the Object
/// Environment Record of a `with` statement that a closure inside its body captured is read
/// after collections. The trace of environment records (`env/mod.rs`): the binding object and
/// the outer record, and the names of a function environment's slots.
#[test]
fn object_environment_survives_collect() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function make() { var local = 2; \
               with ({ v: 40 }) { return function () { return v + local; }; } } \
             var read = make(); typeof read;"
        ),
        Outcome::Normal("\"function\"".to_string())
    );
    assert!(matches!(
        eval(
            &mut agent,
            &realm,
            "var g = 0; while (g < 50) { ({ v: g }); g++; }"
        ),
        HostOutcome::Normal(_)
    ));
    collect(&mut agent);
    collect(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "read() + read();"),
        Outcome::Normal("84".to_string())
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// `mapped_arguments_survive_collect`: a mapped arguments object returned from its function
/// still aliases the parameter, after collections: `alone`'s environment is reachable only
/// through the object's parameter map, `shared`'s also through a closure that captured the
/// parameter. The trace of the arguments object (`object/arguments.rs`): the environment of
/// its parameter map.
#[test]
fn mapped_arguments_survive_collect() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function f(a) { return arguments; } \
             var get; \
             function g(a) { get = function () { return a; }; return arguments; } \
             var alone = f({ n: 1 }); var shared = g({ n: 2 }); typeof alone + typeof shared;"
        ),
        Outcome::Normal("\"objectobject\"".to_string())
    );
    assert!(matches!(
        eval(
            &mut agent,
            &realm,
            "var garbage = 0; while (garbage < 50) { ({ n: garbage }); garbage++; }"
        ),
        HostOutcome::Normal(_)
    ));
    collect(&mut agent);
    collect(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var before = alone[0].n + shared[0].n; \
             alone[0] = { n: 10 }; shared[0] = { n: 20 }; \
             before + alone[0].n + get().n;"
        ),
        Outcome::Normal("33".to_string())
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

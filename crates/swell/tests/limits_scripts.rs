//! The limits and realm requirements whose test needs the builtins and scripts together
//! (`docs/design/host-interface.md`, decisions 2, 7 and 8): the memory budget reached by
//! `push` (HOST-10), a zero `reentry_depth` and the `map` callback (HOST-16), the deepest
//! re-entry parsing at full nesting (HOST-17), the setup closure and the clause 19 globals
//! (HOST-49), a parse that defines no global binding (HOST-53), and the Rust stack
//! `Limits::min_stack_bytes` promises through the builtins that call back into script.
//!
//! A test of a stack bound runs its subject in a child process on a thread of the stated size
//! (`AGENTS.md`, "Commands and the gate"). Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

mod support;

use std::cell::Cell;

use support::host::{
    TestHost, agent_with, global, is_error_of, limits, object, realm, stack_with_room,
};
use support::script::{Outcome, define_function, eval, eval_text, parse};
use swell::{
    Agent, AgentState, Brand, Completion, Cx, GlobalObjects, HookCx, HostCall, HostObjectSpec,
    Limits, Outcome as HostOutcome, Parsed, Prototype, Realm, RealmRef, Root, Source, Termination,
    Value,
};

/// The bytes an agent with the test limits holds after `create_realm`.
fn realm_bytes() -> u64 {
    let mut agent = support::host::agent().unwrap();
    let _realm = realm(&mut agent);
    agent.collect().unwrap();
    agent.heap_bytes().unwrap()
}

/// Whether `outcome` is a throw of a `RangeError` of `realm`.
fn is_range_error_of(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    outcome: HostOutcome<Root>,
) -> bool {
    let HostOutcome::Throw(thrown) = outcome else {
        panic!("expected a throw, got {outcome:?}");
    };
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let global = object(cx, &global_root);
            let thrown = cx.get(&thrown).unwrap();
            is_error_of(cx, global, thrown, "RangeError")
        })
        .unwrap()
}

/// HOST-10: `var a = []; while (true) a.push({});` with `memory_bytes` M and a step budget
/// large enough answers `Terminated(MemoryBudget)`, and `heap_bytes()` then reads at most M;
/// the agent stays `Alive` and answers the next script. For three budgets beyond the realm in
/// the default mode; with the feature `collect-every-safe-point` for the two smaller ones: a
/// collection at every safe point traces the whole heap, so filling 1 MiB costs time that
/// grows with the square of the heap (this test took 29.1 s in that mode with it, 0.27 s without
/// it, measured), and
/// a budget is reached the same way at any size.
#[test]
fn host_10_endless_allocation_ends_in_memory_budget() {
    let base = realm_bytes();
    let rooms: &[u64] = if cfg!(feature = "collect-every-safe-point") {
        &[4_096, 65_536]
    } else {
        &[4_096, 65_536, 1 << 20]
    };
    for &room in rooms {
        let mut limits = limits();
        limits.memory_bytes = base + room;
        limits.steps_per_entry = u64::MAX;
        let budget = limits.memory_bytes;
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, "var a = []; while (true) a.push({});"),
            Outcome::Terminated(Termination::MemoryBudget),
            "room {room}"
        );
        let bytes = agent.heap_bytes().unwrap();
        assert!(bytes <= budget, "{bytes} > {budget}");
        assert_eq!(agent.state(), AgentState::Alive);
        assert_eq!(
            eval_text(&mut agent, &realm, "1;"),
            Outcome::Normal("1".to_string())
        );
        assert!(agent.heap_bytes().unwrap() <= budget);
    }
}

/// HOST-16: with `reentry_depth` 0, `[1].map(function (x) { return x; })` answers `Throw` of a
/// `RangeError` of its realm: no callback runs from a builtin. With 1 the same script answers
/// an array.
#[test]
fn host_16_zero_reentry_depth_refuses_the_map_callback() {
    let source = "var ran = 0; [1].map(function (x) { ran++; return x; });";
    let mut limits = limits();
    limits.reentry_depth = 0;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let outcome = eval(&mut agent, &realm, source);
    assert!(is_range_error_of(&mut agent, &realm, outcome));
    assert_eq!(
        eval_text(&mut agent, &realm, "ran;"),
        Outcome::Normal("0".to_string())
    );

    limits.reentry_depth = 1;
    let mut agent = agent_with(limits).unwrap();
    let realm = support::host::realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, &format!("{source} ran;")),
        Outcome::Normal("1".to_string())
    );
}

thread_local! {
    /// The realm the host functions of these tests parse in.
    static REALM: Cell<Option<RealmRef>> = const { Cell::new(None) };
}

/// Make `realm` the realm the host functions of these tests parse in.
fn remember(agent: &mut Agent<TestHost>, realm: &Realm) {
    let realm_ref = agent
        .with_cx(realm, |cx| cx.realm_ref(realm).unwrap())
        .unwrap();
    REALM.with(|cell| cell.set(Some(realm_ref)));
}

/// A host function that parses 1,000 nested brackets through `cx.parse_script`, as deep as
/// `parser_nesting` lets the parser go, and counts it in the host's `count`; the parse must
/// answer `Parsed` or `Errors`, never `Terminated`.
fn parse_deep(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    let realm = REALM.with(Cell::get).unwrap();
    let source = format!("{}0{};", "(".repeat(1_000), ")".repeat(1_000));
    let parsed = cx.parse_script(realm, Source::Utf8(&source), None).unwrap();
    assert!(!matches!(parsed, Parsed::Terminated(_)), "{parsed:?}");
    cx.host_mut().count += 1;
    Ok(Value::Undefined)
}

/// A host function that calls its first argument through `cx.call`: one re-entry.
fn reenter(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let function = call.args.first().copied().unwrap_or(Value::Undefined);
    cx.call(function, Value::Undefined, &[]).unwrap()
}

/// The variable that tells a child process which stack test to run.
const STACK_CHILD: &str = "SWELL_LIMITS_SCRIPTS_STACK_CHILD";

/// Run `body` on a thread of `stack` bytes in a child process that runs only the test `test`
/// (the test binary started again). A stack overflow aborts the process, which no test can
/// catch in its own process; in the child it fails this test alone, with the child's output.
fn on_bounded_stack(test: &str, stack: usize, body: fn()) -> Result<(), String> {
    let done = format!("{STACK_CHILD}: {test} on {stack} bytes done");
    if std::env::var_os(STACK_CHILD).is_some() {
        std::thread::Builder::new()
            .stack_size(stack)
            .spawn(body)
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| format!("{test}: the body failed"))?;
        println!("{done}");
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let out = std::process::Command::new(exe)
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(STACK_CHILD, "1")
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    if out.status.success() && stdout.contains(&done) {
        return Ok(());
    }
    Err(format!(
        "{test} on a stack of {stack} bytes: {}\n{stdout}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    ))
}

/// The `reentry_depth` of the stack tests: the full depth the gate's limits state.
const FULL_REENTRY: u32 = 1_000;

/// The limits of the stack tests: `frame_depth` 10,000, `reentry_depth` `reentry_depth` and
/// `parser_nesting` 1,000, with step and memory budgets no run of them reaches.
fn stack_limits(reentry_depth: u32) -> Limits {
    let mut limits = limits();
    limits.frame_depth = 10_000;
    limits.reentry_depth = reentry_depth;
    limits.parser_nesting = 1_000;
    limits.steps_per_entry = 1 << 40;
    limits.memory_bytes = 1 << 30;
    limits
}

/// An agent with the stack limits of `reentry_depth` and a realm with `reenter` and
/// `parseDeep`.
fn stack_agent(reentry_depth: u32) -> (Agent<TestHost>, Realm) {
    let mut agent = agent_with(stack_limits(reentry_depth)).unwrap();
    let realm = realm(&mut agent);
    remember(&mut agent, &realm);
    define_function(&mut agent, &realm, "reenter", 1, reenter, false);
    define_function(&mut agent, &realm, "parseDeep", 0, parse_deep, false);
    (agent, realm)
}

/// A script in which `f` calls itself through `through` (the expression that re-enters `f`
/// from Rust) until the re-entry that would pass `reentry_depth` throws a `RangeError`; the
/// deepest `f` then calls `parseDeep` once and throws on. `n` counts the runs of `f`.
fn deepest(through: &str) -> String {
    format!(
        "var n = 0, parsed = false;\n\
         function f(k, v) {{\n\
           n++;\n\
           try {{ {through}; }}\n\
           catch (e) {{ if (!parsed) {{ parsed = true; parseDeep(); }} throw e; }}\n\
           return v;\n\
         }}\n\
         f();"
    )
}

/// Run `deepest(through)` with the stack limits of `reentry_depth`: it throws a `RangeError`,
/// `f` ran `reentry_depth + 1` times (the first call is the script's own, each other a
/// re-entry), and `parseDeep` parsed at the deepest.
fn reach_through(through: &str, reentry_depth: u32) {
    let (mut agent, realm) = stack_agent(reentry_depth);
    let outcome = eval(&mut agent, &realm, &deepest(through));
    assert!(
        is_range_error_of(&mut agent, &realm, outcome),
        "through {through}"
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "n;"),
        Outcome::Normal((reentry_depth + 1).to_string()),
        "through {through}"
    );
    assert_eq!(
        agent.host().count,
        1,
        "parsed once at the deepest, {through}"
    );
}

/// The `reentry_depth` of HOST-17: the full depth in the default mode and in the lane `slow`
/// (`SWELL_FULL_COUNTS=1`); in the gate with the feature `collect-every-safe-point`, 100, as for
/// [`BUILTINS_REENTRY`] and for the same reason. At 100 every safe point of the deepest levels
/// still collects while 100 re-entries are live, and the deepest still parses at full nesting;
/// at 1,000 the test took 15.7 s in that mode (13.7 CPU s) against 0.11 s in the default one
/// (measured).
fn host_17_reentry() -> u32 {
    if cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
    {
        100
    } else {
        FULL_REENTRY
    }
}

/// The body of HOST-17: through a host function that calls back with `cx.call`.
fn reach_through_host_function() {
    reach_through("reenter(f)", host_17_reentry());
}

/// HOST-17: on a thread whose stack has `limits.min_stack_bytes()` bytes (`frame_depth`
/// 10,000, `reentry_depth` 1,000, `parser_nesting` 1,000), a script that reaches
/// `reentry_depth` re-entries and at the deepest one calls a host function that parses a source
/// nested `parser_nesting` levels through `cx.parse_script` ends without a stack overflow. The
/// thread has seven eighths of it ([`stack_with_room`]), which proves the room as well. In the
/// gate's collect mode the depth is [`host_17_reentry`], and the thread has seven eighths of
/// `min_stack_bytes()` of that depth.
#[test]
fn host_17_deepest_reentry_parses_at_full_nesting() {
    let stack = stack_with_room(stack_limits(host_17_reentry()));
    on_bounded_stack(
        "host_17_deepest_reentry_parses_at_full_nesting",
        stack,
        reach_through_host_function,
    )
    .unwrap();
}

/// The four re-entries through builtins whose Rust frames differ: `Function.prototype.call`
/// (`#sec-function.prototype.call`), the callback of `Array.prototype.map`
/// (`#sec-array.prototype.map`), `toJSON` (`#sec-serializejsonproperty`, step 2.b) and the
/// replacer of `JSON.stringify` (`#sec-serializejsonproperty`, step 3.a).
const THROUGH_BUILTINS: [&str; 4] = [
    "f.call(null)",
    "[0].map(f)",
    "JSON.stringify({ toJSON: f })",
    "JSON.stringify(0, f)",
];

/// The `reentry_depth` of the stack test through builtins: the full depth in the default mode;
/// with the feature `collect-every-safe-point`, 100, which still re-enters through each of the
/// four builtins at every level. The stack bound is a property of the default mode's frames
/// (the collector marks from a worklist, so its own stack does not grow with the depth); what
/// the collect mode looks for, a value
/// held without a root, shows at any depth, while a collection at every safe point traces every
/// activation, so the cost grows with the square of the depth: at 1,000 the test took 99.5 s in
/// that mode against 0.16 s in the default one, and at 100 it takes 1.4 s (measured).
const BUILTINS_REENTRY: u32 = if cfg!(feature = "collect-every-safe-point") {
    100
} else {
    FULL_REENTRY
};

/// The body of the stack test through builtins: the four runs, one agent each.
fn reach_through_builtins() {
    for through in THROUGH_BUILTINS {
        reach_through(through, BUILTINS_REENTRY);
    }
}

/// `Limits::min_stack_bytes` through the builtins that call back into script (decision 2;
/// `crates/swell/src/host/limits.rs` names this test beside its constants): on a thread with
/// seven eighths of `min_stack_bytes()` ([`stack_with_room`]) for `frame_depth` 10,000,
/// `reentry_depth` 1,000 and `parser_nesting` 1,000, a script reaches `reentry_depth`
/// re-entries through each of `Function.prototype.call`, a `map` callback, `toJSON` and a
/// replacer of `JSON.stringify` (four runs) and parses 1,000 nested brackets at the deepest,
/// without a stack overflow: `min_stack_bytes()` is enough, with an eighth of it to spare. In
/// the collect mode the depth is [`BUILTINS_REENTRY`], and the thread has seven eighths of
/// `min_stack_bytes()` of that depth.
#[test]
fn min_stack_bytes_is_enough_through_builtins() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        "min_stack_bytes_is_enough_through_builtins",
        stack,
        reach_through_builtins,
    )
    .unwrap();
}

/// The same scripts with a small `reentry_depth`, on the test's own thread: each reaches the
/// bound exactly, so the stack tests measure what they claim.
#[test]
fn each_builtin_reentry_counts_against_reentry_depth() {
    for depth in [0_u32, 1, 7] {
        for through in THROUGH_BUILTINS.iter().copied().chain(["reenter(f)"]) {
            let mut limits = limits();
            limits.reentry_depth = depth;
            let mut agent = agent_with(limits).unwrap();
            let realm = realm(&mut agent);
            remember(&mut agent, &realm);
            define_function(&mut agent, &realm, "reenter", 1, reenter, false);
            define_function(&mut agent, &realm, "parseDeep", 0, parse_deep, false);
            let outcome = eval(&mut agent, &realm, &deepest(through));
            assert!(
                is_range_error_of(&mut agent, &realm, outcome),
                "{through} at {depth}"
            );
            assert_eq!(
                eval_text(&mut agent, &realm, "n;"),
                Outcome::Normal((depth + 1).to_string()),
                "{through} at {depth}"
            );
            assert_eq!(agent.host().count, 1, "{through} at {depth}");
        }
    }
}

/// HOST-49 (`#sec-initializehostdefinedrealm`): the setup closure is called once per
/// `create_realm`, makes the global by `new_host_object` with `Prototype::ObjectPrototypeOf` of
/// the realm it receives, so that its prototype is the new realm's `%Object.prototype%`, and
/// the global it returns has the clause 19 properties the engine defines: with that object
/// returned as the global and no `this`, the script
/// `this.Array === Array && Object.getPrototypeOf(this) === Object.prototype` answers `true` in
/// the realm, and so do the other globals of this slice.
#[test]
fn host_49_setup_closure_and_clause_19_globals() {
    let mut agent = support::host::agent().unwrap();
    for round in 1..=3 {
        let mut calls = 0;
        let mut setup =
            |cx: &mut HookCx<'_, TestHost>, realm: RealmRef| -> Completion<GlobalObjects> {
                calls += 1;
                let global = cx
                    .new_host_object(HostObjectSpec {
                        prototype: Prototype::ObjectPrototypeOf(realm),
                        brand: Brand(round),
                        data: None,
                    })
                    .unwrap()?;
                Ok(GlobalObjects { global, this: None })
            };
        let HostOutcome::Normal(realm) = agent.create_realm(Some(&mut setup), None).unwrap() else {
            panic!("create_realm did not answer Normal");
        };
        assert_eq!(calls, 1, "the setup closure ran once in round {round}");
        let global_root = global(&mut agent, &realm);
        let brand = agent
            .with_cx(&realm, |cx| cx.brand(object(cx, &global_root)))
            .unwrap();
        assert_eq!(brand, Ok(Some(Brand(round))), "the global is the closure's");
        for name in [
            "Array",
            "Object",
            "String",
            "Math",
            "JSON",
            "Error",
            "TypeError",
            "RangeError",
            "SyntaxError",
            "ReferenceError",
            "EvalError",
            "URIError",
            "NaN",
            "Infinity",
            "undefined",
        ] {
            let source = if name == "NaN" {
                "this.NaN !== this.NaN && NaN !== NaN;".to_string()
            } else {
                format!("this.{name} === {name} && \"{name}\" in this;")
            };
            assert_eq!(
                eval_text(&mut agent, &realm, &source),
                Outcome::Normal("true".to_string()),
                "{name}"
            );
        }
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                "this.Array === Array && Object.getPrototypeOf(this) === Object.prototype;"
            ),
            Outcome::Normal("true".to_string())
        );
    }
}

/// A host function that counts its calls in the host's `count`.
fn counting(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().count += 1;
    Ok(Value::Undefined)
}

/// HOST-53 (`#sec-parse-script`): `parse_script` calls no hook, runs no code and defines no
/// global binding: after `parse_script` of a script that declares `var x` (and a function, and
/// calls a host function), and before it is evaluated, the script
/// `Object.prototype.hasOwnProperty.call(this, "x")` answers `false` in a realm whose setup
/// returned no `this`; the host function did not run, the parse spent no step, and no hook was
/// called. After the evaluation it answers `true`.
#[test]
fn host_53_parse_defines_no_global_binding() {
    let mut agent = support::host::agent().unwrap();
    let mut setup =
        |cx: &mut HookCx<'_, TestHost>, _realm: RealmRef| -> Completion<GlobalObjects> {
            let global = cx
                .new_host_object(HostObjectSpec {
                    prototype: Prototype::Null,
                    brand: Brand(5),
                    data: None,
                })
                .unwrap()?;
            Ok(GlobalObjects { global, this: None })
        };
    let HostOutcome::Normal(realm) = agent.create_realm(Some(&mut setup), None).unwrap() else {
        panic!("create_realm did not answer Normal");
    };
    define_function(&mut agent, &realm, "counting", 0, counting, false);
    let script = parse(&mut agent, &realm, "var x = counting(); function g() {} x;");
    assert_eq!(agent.last_entry_steps(), Ok(0), "the parse spent no step");
    assert_eq!(agent.host().count, 0, "no code ran");
    assert!(agent.host().calls.is_empty(), "{:?}", agent.host().calls);
    for name in ["x", "g"] {
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                &format!("Object.prototype.hasOwnProperty.call(this, \"{name}\");")
            ),
            Outcome::Normal("false".to_string()),
            "{name}"
        );
    }
    assert!(matches!(
        agent.evaluate_script(&script),
        Ok(HostOutcome::Normal(_))
    ));
    assert_eq!(agent.host().count, 1);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "Object.prototype.hasOwnProperty.call(this, \"x\");"
        ),
        Outcome::Normal("true".to_string())
    );
    assert!(agent.host().calls.is_empty(), "{:?}", agent.host().calls);
}

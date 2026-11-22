//! The limits and realm requirements whose test needs the builtins and scripts together
//! (`docs/design/host-interface.md`, decisions 2, 7 and 8): the memory budget reached by
//! `push` (HOST-10), a zero `reentry_depth` and the `map` callback (HOST-16), the deepest
//! re-entry parsing at full nesting (HOST-17), the setup closure and the clause 19 globals
//! (HOST-49), a parse that defines no global binding (HOST-53), and the Rust stack
//! `Limits::min_stack_bytes` promises through the builtins that call back into script.
//!
//! A test of a stack bound runs its subject in a child process on a thread of the stated size
//! (`AGENTS.md`, "Commands and the gate"), with the helpers of `tests/support/stack.rs`. Run
//! also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent_with, global, limits, object, realm, stack_with_room};
use support::script::{Outcome, define_function, eval, eval_text, parse};
use support::stack::{
    BUILTINS_REENTRY, FULL_REENTRY, THROUGH_BUILTINS, deepest, is_range_error_of, on_bounded_stack,
    parse_deep, reach_through, reenter, remember, stack_limits, test_path,
};
use swell::{
    AgentState, Brand, Completion, Cx, GlobalObjects, HookCx, HostCall, HostObjectSpec,
    Outcome as HostOutcome, Prototype, RealmRef, Termination, Value,
};

/// The bytes an agent with the test limits holds after `create_realm`.
fn realm_bytes() -> u64 {
    let mut agent = support::host::agent().unwrap();
    let _realm = realm(&mut agent);
    agent.collect().unwrap();
    agent.heap_bytes().unwrap()
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
        test_path!("host_17_deepest_reentry_parses_at_full_nesting"),
        stack,
        reach_through_host_function,
    )
    .unwrap();
}

/// The `reentry_depth` of [`min_stack_bytes_is_enough_through_builtins`]: [`BUILTINS_REENTRY`],
/// the full depth, in the default mode and in the lane `slow` (`SWELL_FULL_COUNTS=1`); 25 in the
/// gate with the feature `collect-every-safe-point`, where every safe point collects and traces
/// every live activation (at 100 the four runs took 23.9 s there at a load of 100 to 110 on 17
/// cores, cargo-nextest). At 25 the script still re-enters through each of the four builtins
/// at every level, every collection of the deepest levels runs with 25 re-entries live, and the
/// deepest still parses at full nesting; the stack bound is a property of the default mode's
/// frames ([`BUILTINS_REENTRY`]).
fn builtins_reentry() -> u32 {
    if cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
    {
        25
    } else {
        BUILTINS_REENTRY
    }
}

/// The body of the stack test through builtins: the four runs, one agent each.
fn reach_through_builtins() {
    for through in THROUGH_BUILTINS {
        reach_through(through, builtins_reentry());
    }
}

/// `Limits::min_stack_bytes` through the builtins that call back into script (decision 2;
/// `crates/swell/src/host/limits.rs` names this test beside its constants): on a thread with
/// seven eighths of `min_stack_bytes()` ([`stack_with_room`]) for `frame_depth` 10,000,
/// `reentry_depth` 1,000 and `parser_nesting` 1,000, a script reaches `reentry_depth`
/// re-entries through each of `Function.prototype.call`, a `map` callback, `toJSON` and a
/// replacer of `JSON.stringify` (four runs) and parses 1,000 nested brackets at the deepest,
/// without a stack overflow: `min_stack_bytes()` is enough, with an eighth of it to spare. In
/// the collect mode the depth is [`builtins_reentry`], and the thread has seven eighths of
/// `min_stack_bytes()` of that depth.
#[test]
fn min_stack_bytes_is_enough_through_builtins() {
    let stack = stack_with_room(stack_limits(builtins_reentry()));
    on_bounded_stack(
        test_path!("min_stack_bytes_is_enough_through_builtins"),
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
                        error_kind: None,
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
            .with_cx(&realm, |cx| {
                let global = object(cx, &global_root);
                cx.brand(global)
            })
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
                    error_kind: None,
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

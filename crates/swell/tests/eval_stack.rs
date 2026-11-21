//! The Rust stack `Limits::min_stack_bytes` promises (decision 2, "What `min_stack_bytes()`
//! covers") when code compiled at run time is parsed at the deepest re-entry: `eval` (HOST-152)
//! and `Function` (HOST-163) of sources nested `parser_nesting` levels, as HOST-17 tests
//! `cx.parse_script` there.
//!
//! Each test runs its subject in a child process on a thread of seven eighths of
//! `min_stack_bytes()` (`support::host::stack_with_room`), so a stack overflow fails the test
//! alone (`AGENTS.md`, "Commands and the gate"). Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::host::{
    TestHost, agent_with, global, is_error_of, limits, object, realm, stack_with_room,
};
use support::script::{Outcome, define_function, eval, eval_text};
use swell::{Completion, Cx, HostCall, Limits, Outcome as HostOutcome, Value};

/// A host function that calls its first argument through `cx.call`: one re-entry.
fn reenter(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let function = call.args.first().copied().unwrap_or(Value::Undefined);
    cx.call(function, Value::Undefined, &[]).unwrap()
}

/// A host function that counts its calls in the host's `count`.
fn counting(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().count += 1;
    Ok(Value::Undefined)
}

/// The variable that tells a child process which stack test to run.
const STACK_CHILD: &str = "SWELL_EVAL_STACK_CHILD";

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

/// The `reentry_depth` of the stack tests: the full depth the gate's limits state; with the
/// feature `collect-every-safe-point`, 100, as `limits_scripts.rs` reasons for its test through
/// builtins (a collection at every safe point traces every activation).
const FULL_REENTRY: u32 = if cfg!(feature = "collect-every-safe-point") {
    100
} else {
    1_000
};

/// The `parser_nesting` of the stack tests.
const NESTING: u32 = 1_000;

/// The limits of the stack tests: `frame_depth` 10,000, `reentry_depth` [`FULL_REENTRY`] and
/// `parser_nesting` 1,000, with step and memory budgets no run of them reaches.
fn stack_limits() -> Limits {
    let mut limits = limits();
    limits.frame_depth = 10_000;
    limits.reentry_depth = FULL_REENTRY;
    limits.parser_nesting = NESTING;
    limits.steps_per_entry = 1 << 40;
    limits.memory_bytes = 1 << 30;
    limits
}

/// `0` inside `n` nested parentheses, as the text of a String literal.
fn nested(n: u32) -> String {
    let n = n as usize;
    format!("{}0{}", "(".repeat(n), ")".repeat(n))
}

/// A script in which `f` calls itself through the host function `reenter` until the re-entry
/// that would pass `reentry_depth` throws a `RangeError`; the deepest `f` then runs `at_deepest`
/// once, calls `counting` and throws on. `n` counts the runs of `f`; `deep` is the source nested
/// `parser_nesting` levels.
fn deepest(at_deepest: &str) -> String {
    format!(
        "var n = 0, done = false, kinds = \"\";\n\
         var deep = \"{}\";\n\
         function kind(run) {{\n\
           try {{ return typeof run(); }} catch (e) {{ return e.name; }}\n\
         }}\n\
         function f() {{\n\
           n++;\n\
           try {{ reenter(f); }}\n\
           catch (e) {{\n\
             if (!done) {{ done = true; {at_deepest} counting(); }}\n\
             throw e;\n\
           }}\n\
         }}\n\
         f();",
        nested(NESTING)
    )
}

/// Run `deepest(at_deepest)`: it throws a `RangeError`, `f` ran `reentry_depth + 1` times, and
/// the code at the deepest ran once; answers `kinds`, what that code recorded.
fn reach(at_deepest: &str) -> String {
    let mut agent = agent_with(stack_limits()).unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "reenter", 1, reenter, false);
    define_function(&mut agent, &realm, "counting", 0, counting, false);
    let outcome = eval(&mut agent, &realm, &deepest(at_deepest));
    let HostOutcome::Throw(thrown) = outcome else {
        panic!("expected a throw, got {outcome:?}");
    };
    let global_root = global(&mut agent, &realm);
    let is_range_error = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let thrown = cx.get(&thrown).unwrap();
            is_error_of(cx, global, thrown, "RangeError")
        })
        .unwrap();
    assert!(is_range_error, "{at_deepest}");
    assert_eq!(
        eval_text(&mut agent, &realm, "n;"),
        Outcome::Normal((FULL_REENTRY + 1).to_string())
    );
    assert_eq!(agent.host().count, 1, "the code at the deepest ran once");
    let Outcome::Normal(kinds) = eval_text(&mut agent, &realm, "kinds;") else {
        panic!("kinds");
    };
    kinds
}

/// The body of HOST-152: at the deepest re-entry, a direct `eval` of `deep` (its parse and its
/// code on the running loop) and an indirect one (a builtin's activation more, whose code then
/// passes `reentry_depth`); each answers a Number or throws a `SyntaxError` (the nesting limit)
/// or a `RangeError`.
fn eval_at_the_deepest() {
    let kinds = reach(
        "kinds += kind(function () { return eval(deep); }) + \" \"; \
         kinds += kind(function () { return (0, eval)(deep); });",
    );
    let allowed = ["number", "SyntaxError", "RangeError"];
    for kind in kinds.trim_matches('"').split(' ') {
        assert!(allowed.contains(&kind), "{kinds}");
    }
}

/// HOST-152: on a thread whose stack has seven eighths of `limits.min_stack_bytes()` bytes
/// (`frame_depth` 10,000, `reentry_depth` 1,000, `parser_nesting` 1,000), a script that reaches
/// `reentry_depth` re-entries and at the deepest one calls `eval` of a source nested
/// `parser_nesting` levels, directly and indirectly, ends without a stack overflow.
#[test]
fn host_152_deepest_reentry_evals_at_full_nesting() {
    on_bounded_stack(
        "host_152_deepest_reentry_evals_at_full_nesting",
        stack_with_room(stack_limits()),
        eval_at_the_deepest,
    )
    .unwrap();
}

/// The body of HOST-163: at the deepest re-entry, `Function(p, b)` with a parameter text `p`
/// and a body text `b` each nested `parser_nesting` levels, and `Function("a", "return " + b)`,
/// whose body and whole source text the parser reads to the limit; each answers a function or
/// throws a `SyntaxError`.
fn function_at_the_deepest() {
    let kinds = reach(
        "kinds += kind(function () { return Function(deep, \"return \" + deep); }) + \" \"; \
         kinds += kind(function () { return Function(\"a\", \"return \" + deep); });",
    );
    for kind in kinds.trim_matches('"').split(' ') {
        assert!(["function", "SyntaxError"].contains(&kind), "{kinds}");
    }
}

/// HOST-163: on a thread whose stack has seven eighths of `limits.min_stack_bytes()` bytes, a
/// script that reaches `reentry_depth` re-entries and at the deepest one calls `Function(p, b)`
/// with `p` and `b` each nested `parser_nesting` levels ends without a stack overflow, in
/// `Normal` or in `Throw` of a `SyntaxError`.
#[test]
fn host_163_deepest_reentry_calls_function_with_parameters_and_body_at_full_nesting() {
    on_bounded_stack(
        "host_163_deepest_reentry_calls_function_with_parameters_and_body_at_full_nesting",
        stack_with_room(stack_limits()),
        function_at_the_deepest,
    )
    .unwrap();
}

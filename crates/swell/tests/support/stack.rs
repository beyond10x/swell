//! The helpers of the stack tests through builtins that call back into script
//! (`story:builtin-group-modules`, moved from `tests/limits_scripts.rs`): a child process on a
//! thread of a stated stack size ([`on_bounded_stack`]), the limits and the agent of the stack
//! tests, and the script that recurses through an expression until the re-entry limit
//! ([`deepest`], [`reach_through`]). A story that delivers a builtin which calls back into
//! script writes its stack test in a file of its own with these helpers, at the re-entry limit
//! on seven eighths of `Limits::min_stack_bytes()` (`AGENTS.md`, "Commands and the gate").
//! `tests/budgets.rs` and `tests/eval_stack.rs` keep their own copies.
//!
//! The helpers stop the test with a panic when a call the test needs does not answer as it
//! must; `cfg(test)` lets `clippy.toml` treat them as test code.
#![cfg(test)]

use std::cell::Cell;

use super::host::{TestHost, agent_with, global, is_error_of, limits, object, realm};
use super::script::{Outcome, define_function, eval, eval_text};
use swell::{
    Agent, Completion, Cx, HostCall, Limits, Outcome as HostOutcome, Parsed, Realm, RealmRef, Root,
    Source, Value,
};

/// Whether `outcome` is a throw of a `RangeError` of `realm`.
pub fn is_range_error_of(
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

thread_local! {
    /// The realm the host functions of the stack tests parse in.
    static REALM: Cell<Option<RealmRef>> = const { Cell::new(None) };
}

/// Make `realm` the realm the host functions of the stack tests parse in.
pub fn remember(agent: &mut Agent<TestHost>, realm: &Realm) {
    let realm_ref = agent
        .with_cx(realm, |cx| cx.realm_ref(realm).unwrap())
        .unwrap();
    REALM.with(|cell| cell.set(Some(realm_ref)));
}

/// A host function that parses 1,000 nested brackets through `cx.parse_script`, as deep as
/// `parser_nesting` lets the parser go, and counts it in the host's `count`; the parse must
/// answer `Parsed` or `Errors`, never `Terminated`.
pub fn parse_deep(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    let realm = REALM.with(Cell::get).unwrap();
    let source = format!("{}0{};", "(".repeat(1_000), ")".repeat(1_000));
    let parsed = cx.parse_script(realm, Source::Utf8(&source), None).unwrap();
    assert!(!matches!(parsed, Parsed::Terminated(_)), "{parsed:?}");
    cx.host_mut().count += 1;
    Ok(Value::Undefined)
}

/// A host function that calls its first argument through `cx.call`: one re-entry.
pub fn reenter(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let function = call.args.first().copied().unwrap_or(Value::Undefined);
    cx.call(function, Value::Undefined, &[]).unwrap()
}

/// The full path of a test in the crate's one test binary, `<module>::<name>` (with any inline
/// module between), which a test passes after `--exact` when it starts its own binary again
/// (`story:one-integration-test-binary-per-crate`). A bare name matches no test there: the
/// child would run nothing, exit 0 and print no "done" line. Made only by [`test_path!`]: write
/// `test_path!("<name>")` with the test's own name, never the name as a string.
#[derive(Clone, Debug)]
pub struct TestPath(String);

impl TestPath {
    /// Called by [`test_path!`] with the caller's `module_path!()`, whose first segment, the
    /// crate, is cut off; write `test_path!("<name>")`.
    #[doc(hidden)]
    pub fn from_module_path(module: &str, name: &str) -> Self {
        match module.split_once("::") {
            Some((_, path)) => Self(format!("{path}::{name}")),
            None => Self(name.to_owned()),
        }
    }

    /// The path, as `--exact` takes it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TestPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// `test_path!("<name>")`: the [`TestPath`] of the test `<name>` of the module the macro is
/// written in.
macro_rules! test_path {
    ($name:expr) => {
        $crate::support::stack::TestPath::from_module_path(module_path!(), $name)
    };
}
pub(crate) use test_path;

/// The variable that tells a child process which stack test to run.
const STACK_CHILD: &str = "SWELL_SUPPORT_STACK_CHILD";

/// Run `body` on a thread of `stack` bytes in a child process that runs only the test `test`
/// (the test binary started again). A stack overflow aborts the process, which no test can
/// catch in its own process; in the child it fails this test alone, with the child's output.
pub fn on_bounded_stack(test: TestPath, stack: usize, body: fn()) -> Result<(), String> {
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
        .args(["--exact", test.as_str(), "--nocapture", "--test-threads=1"])
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
pub const FULL_REENTRY: u32 = 1_000;

/// The four re-entries through builtins whose Rust frames differ: `Function.prototype.call`
/// (`#sec-function.prototype.call`), the callback of `Array.prototype.map`
/// (`#sec-array.prototype.map`), `toJSON` (`#sec-serializejsonproperty`, step 2.b) and the
/// replacer of `JSON.stringify` (`#sec-serializejsonproperty`, step 3.a). The stack test of
/// `tests/limits_scripts.rs` runs them; a story of slice 3 runs its own expressions in its
/// own test with [`reach_through`] and leaves this list as it is.
pub const THROUGH_BUILTINS: [&str; 4] = [
    "f.call(null)",
    "[0].map(f)",
    "JSON.stringify({ toJSON: f })",
    "JSON.stringify(0, f)",
];

/// The `reentry_depth` of a stack test through builtins: the full depth in the default mode;
/// with the feature `collect-every-safe-point`, 50, which still re-enters through each
/// builtin at every level. The stack bound is a property of the default mode's frames (the
/// collector marks from a worklist, so its own stack does not grow with the depth); what the
/// collect mode looks for, a value held without a root, shows at any depth, while a collection
/// at every safe point traces every activation, so the cost grows with the square of the
/// depth: at 1,000 the test through the four builtins of [`THROUGH_BUILTINS`] took 99.5 s in
/// that mode against 0.16 s in the default one, and at 100 it took 1.4 s. Measured in that mode,
/// each test run alone: the 21 tests `*_reach*_the_reentry_limit_on_the_promised_stack` took
/// 15.8 CPU seconds together at 100 and 7.0 at 50; the slowest, through arrows, 2.2 and 0.8.
pub const BUILTINS_REENTRY: u32 = if cfg!(feature = "collect-every-safe-point") {
    50
} else {
    FULL_REENTRY
};

/// The limits of the stack tests: `frame_depth` 10,000, `reentry_depth` `reentry_depth` and
/// `parser_nesting` 1,000, with step and memory budgets no run of them reaches.
pub fn stack_limits(reentry_depth: u32) -> Limits {
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
pub fn stack_agent(reentry_depth: u32) -> (Agent<TestHost>, Realm) {
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
pub fn deepest(through: &str) -> String {
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
pub fn reach_through(through: &str, reentry_depth: u32) {
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

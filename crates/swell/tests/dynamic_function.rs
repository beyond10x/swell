//! The `Function` constructor (`#sec-function-p1-p2-pn-body`) and CreateDynamicFunction
//! (`#sec-createdynamicfunction`) through the host interface
//! (`docs/design/host-interface.md`): HostEnsureCanCompileStrings with the parts of the source
//! (HOST-117), the nesting limit (decision 9, item 9; HOST-141), and each limit reached through
//! `Function`, as the decision that gives `Function` to slice 2 asks (`AGENTS.md`): the
//! `reentry_depth`, `frame_depth`, the step budget and the memory budget. Run also with the
//! feature `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::host::{agent_with, limits, realm};
use support::script::{Outcome, eval_text, run};
use swell::{
    Agent, CompilationType, CompileStrings, Completion, Cx, HookCx, Host, Job, LoadRequest,
    ObjectRef, Outcome as HostOutcome, Parsed, Source, StringRef, Termination, Value,
};

/// A host that records each call of `host_ensure_can_compile_strings`; the required hooks do
/// nothing.
#[derive(Default)]
struct CompileHost {
    calls: Vec<Request>,
}

/// One call of `host_ensure_can_compile_strings`, with its Strings as text.
#[derive(Debug, PartialEq)]
struct Request {
    params: Vec<String>,
    body: String,
    code: String,
    kind: CompilationType,
    param_args: Vec<String>,
    body_arg: String,
    same_realm: bool,
}

impl Host for CompileHost {
    fn host_ensure_can_compile_strings(
        cx: &mut Cx<'_, Self>,
        request: &CompileStrings<'_>,
    ) -> Completion<()> {
        let text = |cx: &Cx<'_, Self>, s: StringRef| cx.to_rust_string_lossy(s).unwrap();
        let value = |cx: &Cx<'_, Self>, v: Value| match v {
            Value::String(s) => text(cx, s),
            other => format!("{other:?}"),
        };
        let request = Request {
            params: request.param_strings.iter().map(|s| text(cx, *s)).collect(),
            body: text(cx, request.body_string),
            code: text(cx, request.code_string),
            kind: request.compilation_type,
            param_args: request.param_args.iter().map(|v| value(cx, *v)).collect(),
            body_arg: value(cx, request.body_arg),
            same_realm: REALM.with(std::cell::Cell::get) == Some(request.callee_realm),
        };
        cx.host_mut().calls.push(request);
        Ok(())
    }

    fn host_enqueue_generic_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}

    fn host_enqueue_timeout_job(_cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {}

    fn host_enqueue_finalization_registry_cleanup_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}

    fn host_load_imported_module(_cx: &mut HookCx<'_, Self>, _request: LoadRequest) {}

    fn host_system_utc_epoch_nanoseconds(_cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        0
    }
}

thread_local! {
    /// The realm of the agent of HOST-117, which the hook compares `callee_realm` with.
    static REALM: std::cell::Cell<Option<swell::RealmRef>> = const { std::cell::Cell::new(None) };
}

/// HOST-117 (`#sec-createdynamicfunction`, step 5): `Function("a", "return a")` calls
/// `host_ensure_can_compile_strings` once, before it parses, with `param_strings` `["a"]`,
/// `body_string` `"return a"`, the whole source text as `code_string`, `compilation_type`
/// `Function`, the arguments as `param_args` and `body_arg`, and the current realm; the
/// function it makes then answers its argument.
#[test]
fn host_117_function_asks_the_host_with_its_parts() {
    let mut agent = Agent::new(CompileHost::default(), limits(), support::host::seeds()).unwrap();
    let HostOutcome::Normal(realm) = agent.create_realm(None, None).unwrap() else {
        panic!("create_realm did not answer Normal");
    };
    let realm_ref = agent
        .with_cx(&realm, |cx| cx.realm_ref(&realm).unwrap())
        .unwrap();
    REALM.with(|cell| cell.set(Some(realm_ref)));
    let Parsed::Parsed(script) = agent
        .parse_script(
            &realm,
            Source::Utf8(
                "Function(\"a\", \"return a\")(7) + \
                 (new Function(\"x, y\", 1)(1) === undefined ? 0 : 1);",
            ),
            None,
        )
        .unwrap()
    else {
        panic!("the script did not parse");
    };
    let HostOutcome::Normal(result) = agent.evaluate_script(&script).unwrap() else {
        panic!("the script did not complete normally");
    };
    let result = agent
        .with_cx(&realm, |cx| cx.get(&result).unwrap())
        .unwrap();
    assert_eq!(result, Value::Number(7.0));
    assert_eq!(
        agent.host().calls,
        [
            Request {
                params: vec!["a".to_string()],
                body: "return a".to_string(),
                code: "function anonymous(a\n) {\nreturn a\n}".to_string(),
                kind: CompilationType::Function,
                param_args: vec!["a".to_string()],
                body_arg: "return a".to_string(),
                same_realm: true,
            },
            Request {
                params: vec!["x, y".to_string()],
                body: "1".to_string(),
                code: "function anonymous(x, y\n) {\n1\n}".to_string(),
                kind: CompilationType::Function,
                param_args: vec!["x, y".to_string()],
                body_arg: "Number(1.0)".to_string(),
                same_realm: true,
            },
        ]
    );
}

/// `0` inside `n` nested parentheses.
fn nested(n: usize) -> String {
    format!("{}0{}", "(".repeat(n), ")".repeat(n))
}

/// HOST-141 (decision 9, item 9): `Function("return " + s)` with `s` nested deeper than
/// `parser_nesting` (1,000) throws a `SyntaxError` whose message names `parser_nesting`, which
/// the script catches; with 10 levels the function answers 0.
#[test]
fn host_141_function_past_parser_nesting_throws_a_catchable_syntax_error() {
    let source = format!(
        "try {{ Function(\"return \" + \"{}\"); }} catch (e) {{ e.name + \": \" + e.message; }}",
        nested(2_000)
    );
    let Outcome::Normal(caught) = run(limits(), &source) else {
        panic!("the script did not complete normally");
    };
    assert!(
        caught.starts_with("\"SyntaxError: ") && caught.contains("parser_nesting (1000)"),
        "{caught}"
    );
    assert_eq!(
        run(
            limits(),
            &format!("Function(\"return \" + \"{}\")();", nested(10))
        ),
        Outcome::Normal("0".to_string())
    );
}

/// `reentry_depth` through `Function`: with `reentry_depth` 16, an object whose `toString`
/// calls `Function(o, "")` on itself re-enters from Rust at each ToString of a parameter
/// (`#sec-createdynamicfunction`, step 9), until the re-entry past the bound throws a
/// `RangeError` whose message is `reentry_depth exceeded`, which the script catches.
#[test]
fn function_past_reentry_depth_throws_a_range_error() {
    let mut limits = limits();
    limits.reentry_depth = 16;
    assert_eq!(
        run(
            limits,
            "var o = { toString: function () { return Function(o, \"\"); } };\n\
             var r;\n\
             try { Function(o, \"\"); } catch (e) { r = e.name + \": \" + e.message; }\n\
             r;"
        ),
        Outcome::Normal("\"RangeError: reentry_depth exceeded\"".to_string())
    );
}

/// The step budget through `Function`: a dynamic function whose body loops forever ends the run
/// in `Terminated(StepBudget)`.
#[test]
fn step_budget_ends_an_endless_dynamic_function() {
    for source in [
        "Function(\"while (true) {}\")();",
        "new Function(\"a\", \"for (;;) { a++; }\")(0);",
    ] {
        let mut limits = limits();
        limits.steps_per_entry = 100_000;
        assert_eq!(
            run(limits, source),
            Outcome::Terminated(Termination::StepBudget),
            "{source}"
        );
    }
}

/// The bytes an agent with `limits` holds after `create_realm`, collected.
fn realm_bytes(limits: swell::Limits) -> u64 {
    let mut agent = agent_with(limits).unwrap();
    let _realm = realm(&mut agent);
    agent.collect().unwrap();
    agent.heap_bytes().unwrap()
}

/// The memory budget through `Function`: the code CreateDynamicFunction compiles is charged as
/// `parse_script` charges it, so a body of 32,768 statements `0;` (64 Ki code units, which
/// fit) whose code does not fit in what is left of the budget ends the run in
/// `Terminated(MemoryBudget)`; `heap_bytes()` then reads at most the budget, and the agent stays
/// `Alive` and makes the next function.
#[test]
fn memory_budget_ends_a_dynamic_function_whose_code_does_not_fit() {
    let mut limits = limits();
    limits.memory_bytes = realm_bytes(limits) + 512 * 1024;
    let budget = limits.memory_bytes;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let build = "var s = \"0;\"; for (var i = 0; i < 15; i++) s = s + s; ";
    assert_eq!(
        eval_text(&mut agent, &realm, &format!("{build} s.length;")),
        Outcome::Normal("65536".to_string()),
        "the source fits"
    );
    assert_eq!(
        eval_text(&mut agent, &realm, &format!("{build} Function(s);")),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    assert!(agent.heap_bytes().unwrap() <= budget);
    assert_eq!(agent.state(), swell::AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "Function(\"return 2\")();"),
        Outcome::Normal("2".to_string())
    );
}

/// `frame_depth` through `Function`: a function that makes a dynamic function calling it again
/// pushes a frame for each call, with no Rust recursion, until `frame_depth` throws a
/// `RangeError`.
#[test]
fn dynamic_function_recursion_ends_in_a_range_error() {
    let mut limits = limits();
    limits.frame_depth = 500;
    assert_eq!(
        run(limits, "function g() { Function(\"g()\")(); } g();"),
        Outcome::Throw("RangeError".to_string())
    );
    assert_eq!(
        run(
            limits,
            "var n = 0; function g() { n++; Function(\"g()\")(); } try { g(); } catch (e) { n; }"
        ),
        Outcome::Normal("250".to_string())
    );
}

/// `#sec-createdynamicfunction`, step 33: the function's [[Environment]] is the global
/// environment of the current realm, whatever the caller's scope; its `var`s are its own, and a
/// name the global object gets later is seen.
#[test]
fn dynamic_function_scope_is_the_global_environment() {
    assert_eq!(
        run(
            limits(),
            "var x = \"global\";\n\
             function f() { var x = \"local\"; return Function(\"return x\")(); }\n\
             f();"
        ),
        Outcome::Normal("\"global\"".to_string())
    );
    assert_eq!(
        run(
            limits(),
            "var g = Function(\"var v = 1; return typeof later + typeof v;\");\n\
             var later = 0;\n\
             g() + typeof v;"
        ),
        Outcome::Normal("\"numbernumberundefined\"".to_string())
    );
    assert_eq!(
        run(
            limits(),
            "function f(a) { return Function(\"return typeof a\")(); } f(1);"
        ),
        Outcome::Normal("\"undefined\"".to_string())
    );
}

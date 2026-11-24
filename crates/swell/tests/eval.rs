//! `eval` (`#sec-eval-x`, `#sec-performeval`, `#sec-evaldeclarationinstantiation` and the
//! direct call of `#sec-function-calls-runtime-semantics-evaluation`, step 6) through the host
//! interface (`docs/design/host-interface.md`): the hooks HostEnsureCanCompileStrings and
//! HostGetCodeForEval and the check of what they answer (decision 6), the nesting limit through
//! a direct and an indirect `eval` (decision 9, item 9), the count of refusals (section 13), and
//! each limit reached through `eval`: `reentry_depth`, `frame_depth`, the step budget and the
//! memory budget. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::cell::Cell;

use support::host::{agent_with, limits, realm};
use support::script::{Outcome, define_function, eval_text, run};
use swell::{
    Abrupt, Agent, AgentState, Brand, CompileStrings, Completion, Cx, HookCx, Host, HostCall,
    HostError, HostObjectSpec, Job, LoadRequest, ObjectRef, Outcome as HostOutcome, Parsed,
    PropertyKey, Prototype, Realm, Root, Source, StringRef, Termination, Value,
};

/// What [`EvalHost`]'s `host_ensure_can_compile_strings` answers.
#[derive(Clone, Copy, Debug, Default)]
enum Compile {
    /// `Ok(())`, the default.
    #[default]
    Allow,
    /// `Err(Abrupt::Throw)` of a new String "refused".
    Refuse,
    /// `Err(Abrupt::Throw)` of this value, which a test makes stale or foreign.
    Throw(Value),
}

/// What [`EvalHost`]'s `host_get_code_for_eval` answers.
#[derive(Clone, Copy, Debug, Default)]
enum Code {
    /// `None`, the default ("no-code").
    #[default]
    Absent,
    /// `Some` of a new String of this text.
    Text(&'static str),
    /// `Some` of this String, which a test makes stale or foreign.
    Handle(StringRef),
}

/// A host that records the calls of the two hooks of dynamic code and answers them as a test
/// sets; the required hooks do nothing.
#[derive(Default)]
struct EvalHost {
    /// One line per call of either hook, in order.
    calls: Vec<String>,
    compile: Compile,
    code: Code,
    /// Counted by the host function `mark`.
    marks: u32,
}

impl Host for EvalHost {
    fn host_ensure_can_compile_strings(
        cx: &mut Cx<'_, Self>,
        request: &CompileStrings<'_>,
    ) -> Completion<()> {
        let text = |cx: &Cx<'_, Self>, s: StringRef| cx.to_rust_string_lossy(s).unwrap();
        let params: Vec<String> = request.param_strings.iter().map(|s| text(cx, *s)).collect();
        let line = format!(
            "compile {:?} params {params:?} body {:?} code {:?} param args {} body arg {}",
            request.compilation_type,
            text(cx, request.body_string),
            text(cx, request.code_string),
            request.param_args.len(),
            match request.body_arg {
                Value::String(s) => format!("{:?}", text(cx, s)),
                Value::Object(_) => "an object".to_string(),
                other => format!("{other:?}"),
            },
        );
        cx.host_mut().calls.push(line);
        match cx.host().compile {
            Compile::Allow => Ok(()),
            Compile::Refuse => Err(Abrupt::Throw(Value::String(cx.new_string("refused")?))),
            Compile::Throw(value) => Err(Abrupt::Throw(value)),
        }
    }

    fn host_get_code_for_eval(cx: &mut HookCx<'_, Self>, _arg: ObjectRef) -> Option<StringRef> {
        cx.host_mut().calls.push("code for eval".to_string());
        match cx.host().code {
            Code::Absent => None,
            Code::Text(text) => cx.new_string(text).ok(),
            Code::Handle(string) => Some(string),
        }
    }

    fn host_enqueue_generic_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}

    fn host_enqueue_timeout_job(_cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {}

    fn host_enqueue_finalization_registry_cleanup_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}

    fn host_load_imported_module(_cx: &mut HookCx<'_, Self>, _request: LoadRequest) {}

    fn host_system_utc_epoch_nanoseconds(_cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        0
    }
}

/// An agent with [`EvalHost`] and the test limits, a realm, and the host function `mark`, which
/// counts its calls in `marks`.
fn eval_agent() -> (Agent<EvalHost>, Realm) {
    let mut agent = Agent::new(EvalHost::default(), limits(), support::host::seeds()).unwrap();
    let HostOutcome::Normal(realm) = support::host::create_realm(&mut agent, None, None).unwrap()
    else {
        panic!("create_realm did not answer Normal");
    };
    let global = agent.global_object(&realm).unwrap();
    agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let mark = cx
                .new_host_function(realm_ref, "mark", 0, mark, false, None)
                .unwrap()
                .unwrap();
            let Value::Object(global) = cx.get(&global).unwrap() else {
                panic!("the global is no object");
            };
            let key = PropertyKey::String(cx.new_string("mark").unwrap());
            cx.set(global, key, Value::Object(mark), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
    (agent, realm)
}

/// The host function `mark`: counts its calls.
fn mark(cx: &mut Cx<'_, EvalHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().marks += 1;
    Ok(Value::Undefined)
}

/// The outcome of the script `source` in `realm` of an [`EvalHost`] agent, as text: `normal`
/// or `throw` with the value as ToString writes it (a String as is), a termination, or
/// `errors` of a parse.
fn outcome(agent: &mut Agent<EvalHost>, realm: &Realm, source: &str) -> String {
    let script = match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => script,
        Parsed::Errors(_) => return "errors".to_string(),
        Parsed::Terminated(termination) => return format!("{termination:?}"),
    };
    let (kind, root): (&str, Root) = match agent.evaluate_script(&script).unwrap() {
        HostOutcome::Normal(root) => ("normal", root),
        HostOutcome::Throw(root) => ("throw", root),
        HostOutcome::Terminated(termination) => return format!("{termination:?}"),
    };
    agent
        .with_cx(realm, |cx| {
            let value = cx.get(&root).unwrap();
            let string = cx.to_string(value).unwrap().unwrap();
            format!("{kind} {}", cx.to_rust_string_lossy(string).unwrap())
        })
        .unwrap()
}

/// HOST-116 (`#sec-hostensurecancompilestrings`, `#sec-performeval` step 5): `eval` of a String
/// calls `host_ensure_can_compile_strings` once, before it parses, with the String as
/// `body_string` and `code_string`, no parameters, and `DirectEval` or `IndirectEval` as it was
/// called; `eval` of a value that is not a String calls no hook. A `Throw` from the hook is
/// thrown by `eval`, and no code of the string runs: a string that would not parse throws the
/// hook's value, not a `SyntaxError`.
#[test]
fn host_116_eval_asks_the_host_before_parsing() {
    let (mut agent, realm) = eval_agent();
    assert_eq!(outcome(&mut agent, &realm, "eval(\"1\");"), "normal 1");
    assert_eq!(
        agent.host().calls,
        [
            "compile DirectEval params [] body \"1\" code \"1\" param args 0 body arg \"1\""
                .to_string()
        ]
    );
    agent.host_mut().calls.clear();
    assert_eq!(outcome(&mut agent, &realm, "(0, eval)(\"1\");"), "normal 1");
    assert_eq!(
        agent.host().calls,
        [
            "compile IndirectEval params [] body \"1\" code \"1\" param args 0 body arg \"1\""
                .to_string()
        ]
    );
    agent.host_mut().calls.clear();
    assert_eq!(outcome(&mut agent, &realm, "eval(2);"), "normal 2");
    assert!(agent.host().calls.is_empty(), "{:?}", agent.host().calls);

    agent.host_mut().compile = Compile::Refuse;
    for source in [
        "try { eval(\"mark(); (\"); } catch (e) { e; }",
        "try { (0, eval)(\"mark(); (\"); } catch (e) { e; }",
        "try { eval(\"mark();\"); } catch (e) { e; }",
        "function f() { return eval(\"mark();\"); } try { f(); } catch (e) { e; }",
    ] {
        agent.host_mut().calls.clear();
        assert_eq!(
            outcome(&mut agent, &realm, source),
            "normal refused",
            "{source}"
        );
        assert_eq!(agent.host().calls.len(), 1, "{source}");
    }
    assert_eq!(agent.host().marks, 0, "no code of the strings ran");
    assert_eq!(agent.state(), AgentState::Alive);
}

/// HOST-118 (`#sec-hostgetcodeforeval`, `#sec-performeval` step 2): `eval` of an object asks
/// `host_get_code_for_eval`; `None` ("no-code") makes `eval(o)` return `o`, without asking
/// HostEnsureCanCompileStrings, and `Some(s)` evaluates `s` as `eval(s)` would, asking
/// HostEnsureCanCompileStrings with `s` as `body_string` and `code_string` and the object
/// itself as `body_arg` (step 6: bodyArg is source). `eval` of a String does not ask it.
#[test]
fn host_118_eval_of_an_object_asks_host_get_code_for_eval() {
    let (mut agent, realm) = eval_agent();
    assert_eq!(
        outcome(
            &mut agent,
            &realm,
            "var o = {}; eval(o) === o && (0, eval)(o) === o;"
        ),
        "normal true"
    );
    assert_eq!(agent.host().calls, ["code for eval", "code for eval"]);
    agent.host_mut().calls.clear();
    agent.host_mut().code = Code::Text("var k = 20; k + 1");
    assert_eq!(
        outcome(
            &mut agent,
            &realm,
            "function f() { var k = 1; return eval({}) + k; } f();"
        ),
        "normal 41"
    );
    assert_eq!(
        agent.host().calls,
        [
            "code for eval".to_string(),
            "compile DirectEval params [] body \"var k = 20; k + 1\" code \"var k = 20; k + 1\" \
             param args 0 body arg an object"
                .to_string()
        ]
    );
    agent.host_mut().calls.clear();
    assert_eq!(outcome(&mut agent, &realm, "eval(\"3\");"), "normal 3");
    assert!(!agent.host().calls.iter().any(|c| c == "code for eval"));
}

/// A String handle that is stale: made in one scope, never rooted, and freed by a collection.
fn stale_string(agent: &mut Agent<EvalHost>, realm: &Realm) -> StringRef {
    let string = agent
        .with_cx(realm, |cx| cx.new_string("stale code").unwrap())
        .unwrap();
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    string
}

/// A String handle of another agent, which the caller keeps alive.
fn foreign_string(other: &mut Agent<EvalHost>, other_realm: &Realm) -> StringRef {
    other
        .with_cx(other_realm, |cx| cx.new_string("foreign code").unwrap())
        .unwrap()
}

/// Every entry point and read of a faulted agent but `state()` and `refusals()` answers
/// `Err(HostError::Faulted)`.
fn assert_faulted(agent: &mut Agent<EvalHost>, realm: &Realm, case: &str) {
    assert_eq!(agent.state(), AgentState::Faulted, "{case}");
    let _: u64 = agent.refusals();
    assert!(
        matches!(agent.with_cx(realm, |_| ()), Err(HostError::Faulted)),
        "{case}"
    );
    assert!(
        matches!(
            agent.parse_script(realm, Source::Utf8("1;"), None),
            Err(HostError::Faulted)
        ),
        "{case}"
    );
    assert!(matches!(agent.collect(), Err(HostError::Faulted)), "{case}");
    assert!(
        matches!(agent.heap_bytes(), Err(HostError::Faulted)),
        "{case}"
    );
    assert!(
        matches!(agent.last_entry_steps(), Err(HostError::Faulted)),
        "{case}"
    );
    assert!(
        matches!(agent.global_object(realm), Err(HostError::Faulted)),
        "{case}"
    );
    assert!(
        matches!(agent.create_realm(None, None), Err(HostError::Faulted)),
        "{case}"
    );
}

/// HOST-162 (decision 6, both return sites): a `Some` of `host_get_code_for_eval` holding a
/// stale or a foreign String handle, called by `eval(o)`, and an `Abrupt::Throw` of
/// `host_ensure_can_compile_strings` holding one, called by a direct `eval`, an indirect `eval`
/// and `Function`: each ends the run with `Terminated(EngineFault)`, no code of the string runs,
/// `state()` then reads `Faulted`, every entry point and read but `state()` and `refusals()`
/// answers `Err(HostError::Faulted)`, and nothing panics.
#[test]
fn host_162_a_stale_or_foreign_handle_from_either_hook_faults_the_agent() {
    let (mut other, other_realm) = eval_agent();
    for stale in [true, false] {
        // The Some of host_get_code_for_eval.
        let (mut agent, realm) = eval_agent();
        let bad = if stale {
            stale_string(&mut agent, &realm)
        } else {
            foreign_string(&mut other, &other_realm)
        };
        agent.host_mut().code = Code::Handle(bad);
        assert_eq!(
            outcome(
                &mut agent,
                &realm,
                "try { eval({}); } catch (e) { mark(); }"
            ),
            "EngineFault",
            "code for eval, stale {stale}"
        );
        assert_eq!(agent.host().marks, 0);
        assert_faulted(&mut agent, &realm, "code for eval");

        // The Throw of host_ensure_can_compile_strings.
        for call in [
            "eval(\"mark()\")",
            "(0, eval)(\"mark()\")",
            "Function(\"mark()\")()",
        ] {
            let (mut agent, realm) = eval_agent();
            let bad = if stale {
                stale_string(&mut agent, &realm)
            } else {
                foreign_string(&mut other, &other_realm)
            };
            agent.host_mut().compile = Compile::Throw(Value::String(bad));
            assert_eq!(
                outcome(
                    &mut agent,
                    &realm,
                    &format!("try {{ {call}; }} catch (e) {{ mark(); }}")
                ),
                "EngineFault",
                "{call}, stale {stale}"
            );
            assert_eq!(agent.host().marks, 0, "{call}");
            assert_faulted(&mut agent, &realm, call);
        }
    }
}

/// `0` inside `n` nested parentheses.
fn nested(n: usize) -> String {
    format!("{}0{}", "(".repeat(n), ")".repeat(n))
}

/// HOST-139 (decision 9, item 9): a direct `eval` of a source nested deeper than
/// `parser_nesting` (1,000) throws a `SyntaxError` whose message names `parser_nesting`, which
/// the script catches; 10 levels evaluate.
#[test]
fn host_139_direct_eval_past_parser_nesting_throws_a_catchable_syntax_error() {
    let source = format!(
        "function f() {{ try {{ return eval(\"{}\"); }} catch (e) {{ return e.name + \": \" + \
         e.message; }} }} f();",
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
        run(limits(), &format!("eval(\"{}\");", nested(10))),
        Outcome::Normal("0".to_string())
    );
}

/// HOST-140 (decision 9, item 9): an indirect `eval` of a source nested deeper than
/// `parser_nesting` throws a `SyntaxError` naming `parser_nesting`: uncaught it is the script's
/// `Throw`, caught the script reads its message.
#[test]
fn host_140_indirect_eval_past_parser_nesting_throws_a_syntax_error() {
    assert_eq!(
        run(limits(), &format!("(0, eval)(\"{}\");", nested(2_000))),
        Outcome::Throw("SyntaxError".to_string())
    );
    let source = format!(
        "try {{ (0, eval)(\"{}\"); }} catch (e) {{ e.message; }}",
        nested(2_000)
    );
    let Outcome::Normal(message) = run(limits(), &source) else {
        panic!("the script did not complete normally");
    };
    assert!(message.contains("parser_nesting (1000)"), "{message}");
}

thread_local! {
    /// The stale object the host function `fault` answers.
    static STALE: Cell<Option<ObjectRef>> = const { Cell::new(None) };
}

/// A host function that answers a stale object, which faults the agent (decision 6).
fn fault(_cx: &mut Cx<'_, support::host::TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Object(STALE.with(Cell::get).unwrap()))
}

/// HOST-164 (section 13, "A count of refusals"): `refusals()` reads 0 on a new agent and one
/// more for each refusal the engine raises, caught or not, through each entry: `parse_script`
/// of `<!-- x` (an HTML-like comment, slice 18), and `eval` and `Function` of it caught by the
/// script. A grammar error and the nesting limit are no refusals. The count is a `u64` read
/// after the agent faulted too.
#[test]
fn host_164_refusals_counts_every_refusal_caught_or_not() {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(agent.refusals(), 0);
    let Outcome::Errors(errors) = eval_text(&mut agent, &realm, "<!-- x") else {
        panic!("<!-- x parsed");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(agent.refusals(), 1);
    for (source, after) in [
        ("try { eval(\"<!-- x\"); } catch (e) {}", 2),
        ("try { Function(\"<!-- x\"); } catch (e) {}", 3),
        ("try { (0, eval)(\"<!-- x\"); } catch (e) {}", 4),
    ] {
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Normal("undefined".to_string()),
            "{source}"
        );
        assert_eq!(agent.refusals(), after, "{source}");
    }
    assert!(matches!(
        eval_text(&mut agent, &realm, "var;"),
        Outcome::Errors(_)
    ));
    assert!(matches!(
        eval_text(&mut agent, &realm, &format!("{};", nested(2_000))),
        Outcome::Errors(_)
    ));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "try {{ eval(\"var;\"); }} catch (e) {{}} try {{ eval(\"{}\"); }} catch (e) {{}}",
                nested(2_000)
            )
        ),
        Outcome::Normal("undefined".to_string())
    );
    assert_eq!(
        agent.refusals(),
        4,
        "grammar errors and nesting are no refusals"
    );

    // Read after the agent faulted.
    let stale = agent
        .with_cx(&realm, |cx| {
            cx.new_host_object(HostObjectSpec {
                prototype: Prototype::Null,
                brand: Brand(1),
                data: None,
                error_kind: None,
            })
            .unwrap()
            .unwrap()
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    STALE.with(|cell| cell.set(Some(stale)));
    define_function(&mut agent, &realm, "fault", 0, fault, false);
    assert_eq!(
        eval_text(&mut agent, &realm, "fault();"),
        Outcome::Terminated(Termination::EngineFault)
    );
    assert_eq!(agent.state(), AgentState::Faulted);
    let count: u64 = agent.refusals();
    assert_eq!(count, 4);
}

/// `reentry_depth` through `eval`: with `reentry_depth` 16, a getter that evaluates
/// `(0, eval)("o.x")` on its own object re-enters from Rust at each read (the getter's call and
/// the indirect `eval`'s code), until the re-entry past the bound throws a `RangeError` whose
/// message is `reentry_depth exceeded`, which the script catches.
#[test]
fn eval_past_reentry_depth_throws_a_range_error() {
    let mut limits = limits();
    limits.reentry_depth = 16;
    assert_eq!(
        run(
            limits,
            "var o = { get x() { return (0, eval)(\"o.x\"); } };\n\
             var r;\n\
             try { o.x; } catch (e) { r = e.name + \": \" + e.message; }\n\
             r;"
        ),
        Outcome::Normal("\"RangeError: reentry_depth exceeded\"".to_string())
    );
}

/// The step budget of [`ends_in_step_budget`]: 100,000 steps; 2,000 with the collector at
/// every safe point, where every step collects, in the gate and in the lane `slow`
/// (`SWELL_FULL_COUNTS=1`) alike. With 10,000 the three tests took 17.9 to 25.2 s in that
/// mode at a load of 100 to 110 on 17 cores (cargo-nextest), and with the lane's 100,000 17.6
/// to 18.1 CPU seconds each alone (2,000: 0.37 to 0.39, measured on 2026-11-22). The loop in
/// eval code never ends, and the eval code is compiled in far fewer than 2,000 steps, so every
/// budget ends it the same way, in `Terminated(StepBudget)`.
fn step_budget() -> u64 {
    if cfg!(feature = "collect-every-safe-point") {
        2_000
    } else {
        100_000
    }
}

/// `source`, an endless loop in eval code, ends the run in `Terminated(StepBudget)` at the budget
/// [`step_budget`] gives.
fn ends_in_step_budget(source: &str) {
    let mut limits = limits();
    limits.steps_per_entry = step_budget();
    assert_eq!(
        run(limits, source),
        Outcome::Terminated(Termination::StepBudget),
        "{source}"
    );
}

/// The step budget through `eval`: an endless loop in eval code, direct or indirect, ends the
/// run in `Terminated(StepBudget)`; here direct eval at the top level. The indirect eval and the
/// direct eval in a function are the two tests after it, each in a process of its own under
/// cargo-nextest (in the mode collect-every-safe-point each of the three takes seconds).
#[test]
fn step_budget_ends_an_endless_eval() {
    ends_in_step_budget("eval(\"while (true) {}\");");
}

/// The step budget through an indirect `eval` ([`step_budget_ends_an_endless_eval`]).
#[test]
fn step_budget_ends_an_endless_indirect_eval() {
    ends_in_step_budget("(0, eval)(\"for (;;) {}\");");
}

/// The step budget through a direct `eval` in a function ([`step_budget_ends_an_endless_eval`]).
#[test]
fn step_budget_ends_an_endless_eval_in_a_function() {
    ends_in_step_budget("function f() { eval(\"do {} while (true)\"); } f();");
}

/// The bytes an agent with `limits` holds after `create_realm`, collected.
fn realm_bytes(limits: swell::Limits) -> u64 {
    let mut agent = agent_with(limits).unwrap();
    let _realm = realm(&mut agent);
    agent.collect().unwrap();
    agent.heap_bytes().unwrap()
}

/// A script that builds a source `s` of 32,768 statements `0;` (64 Ki code units) by doubling,
/// then runs `tail`.
fn large_code(tail: &str) -> String {
    format!(
        "var s = \"0;\";\n\
         for (var i = 0; i < 15; i++) s = s + s;\n\
         {tail}"
    )
}

/// The memory budget through `eval`: the code an `eval` compiles is charged as `parse_script`
/// charges it, so a source whose code does not fit in what is left of the budget ends the run in
/// `Terminated(MemoryBudget)`, while the source itself fits; `heap_bytes()` then reads at most
/// the budget and the agent stays `Alive`.
#[test]
fn memory_budget_ends_an_eval_whose_code_does_not_fit() {
    for call in ["eval", "(0, eval)"] {
        let mut limits = limits();
        limits.memory_bytes = realm_bytes(limits) + 512 * 1024;
        let budget = limits.memory_bytes;
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, &large_code("s.length;")),
            Outcome::Normal("65536".to_string()),
            "the source fits"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &large_code(&format!("{call}(s);"))),
            Outcome::Terminated(Termination::MemoryBudget),
            "{call}"
        );
        assert!(agent.heap_bytes().unwrap() <= budget);
        assert_eq!(agent.state(), AgentState::Alive);
        assert_eq!(
            eval_text(&mut agent, &realm, "eval(\"1 + 1\");"),
            Outcome::Normal("2".to_string())
        );
    }
}

/// The `frame_depth` of [`eval_recursion_ends_in_a_range_error`]: 500, the full count, in the
/// default mode; 100 with the collector at every safe point, in the gate and in the lane `slow`
/// (`SWELL_FULL_COUNTS=1`) alike, where every step collects and every collection traces every
/// frame, so the cost grows with the square of the depth (500 took 29.8 s there at a load of
/// 100 to 110 on 17 cores, cargo-nextest, and 2.1 to 2.4 CPU seconds alone in the lane,
/// measured on 2026-11-22). At 100 the frames of 50 calls and 50 eval codes are live across
/// those collections and the depth still throws the `RangeError`; the default mode shows at
/// 500 that no Rust call is made per level.
fn eval_frame_depth() -> u32 {
    if cfg!(feature = "collect-every-safe-point") {
        100
    } else {
        500
    }
}

/// `frame_depth` through `eval`: `function f() { eval("f()"); } f()` pushes a frame for each
/// call and each eval code, with no Rust recursion, until `frame_depth` ([`eval_frame_depth`])
/// throws a `RangeError`, after `frame_depth / 2` calls.
#[test]
fn eval_recursion_ends_in_a_range_error() {
    let mut limits = limits();
    limits.frame_depth = eval_frame_depth();
    assert_eq!(
        run(limits, "function f() { eval(\"f()\"); } f();"),
        Outcome::Throw("RangeError".to_string())
    );
    assert_eq!(
        run(
            limits,
            "var n = 0; function f() { n++; eval(\"f()\"); } try { f(); } catch (e) { n; }"
        ),
        Outcome::Normal((limits.frame_depth / 2).to_string())
    );
}

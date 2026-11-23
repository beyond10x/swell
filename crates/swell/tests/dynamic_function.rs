//! The `Function` constructor (`#sec-function-p1-p2-pn-body`) and CreateDynamicFunction
//! (`#sec-createdynamicfunction`) through the host interface
//! (`docs/design/host-interface.md`): HostEnsureCanCompileStrings with the parts of the source
//! (HOST-117), the nesting limit (decision 9, item 9; HOST-141), and each limit reached through
//! `Function`, as the decision that gives `Function` to slice 2 asks (`AGENTS.md`): the
//! `reentry_depth`, `frame_depth`, the step budget and the memory budget. Then
//! `%GeneratorFunction%` (`#sec-generatorfunction-constructor`), CreateDynamicFunction of kind
//! generator through the same hook, with the compilation type `Function`, and the same nesting
//! limit. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent_with, get, get_object, global, key, limits, object, realm};
use support::script::{Outcome, eval, eval_text, run};
use swell::{
    Abrupt, Agent, CompilationType, CompileStrings, Completion, Cx, HookCx, Host, HostCall, Job,
    LoadRequest, ObjectRef, Outcome as HostOutcome, Parsed, Source, StringRef, Termination, Value,
};

/// A host that records each call of `host_ensure_can_compile_strings`, and answers it with a
/// throw of the String "refused" when `refuse` is set; the required hooks do nothing.
#[derive(Default)]
struct CompileHost {
    calls: Vec<Request>,
    refuse: bool,
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
        if cx.host().refuse {
            return Err(Abrupt::Throw(Value::String(cx.new_string("refused")?)));
        }
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

/// The step budget of [`ends_in_step_budget`]: 100,000 steps; 2,000 with the collector at
/// every safe point, where every step collects, in the gate and in the lane `slow`
/// (`SWELL_FULL_COUNTS=1`) alike. With 10,000 the two tests took 16.3 and 10.6 s in that mode
/// at a load of 100 to 110 on 17 cores (cargo-nextest), and with the lane's 100,000 17.5 and
/// 17.7 CPU seconds each alone (2,000: 0.35 and 0.36, measured on 2026-11-22). The loop of the
/// function body never ends, and the function is made in far fewer than 2,000 steps, so every
/// budget ends it the same way, in `Terminated(StepBudget)`.
fn step_budget() -> u64 {
    if cfg!(feature = "collect-every-safe-point") {
        2_000
    } else {
        100_000
    }
}

/// `source`, a dynamic function whose body loops forever, ends the run in
/// `Terminated(StepBudget)` at the budget [`step_budget`] gives.
fn ends_in_step_budget(source: &str) {
    let mut limits = limits();
    limits.steps_per_entry = step_budget();
    assert_eq!(
        run(limits, source),
        Outcome::Terminated(Termination::StepBudget),
        "{source}"
    );
}

/// The step budget through `Function`: a dynamic function whose body loops forever ends the run
/// in `Terminated(StepBudget)`; here `Function` called. The test after it constructs one with
/// `new`, in a process of its own under cargo-nextest (in the mode collect-every-safe-point each
/// takes seconds).
#[test]
fn step_budget_ends_an_endless_dynamic_function() {
    ends_in_step_budget("Function(\"while (true) {}\")();");
}

/// The step budget through `new Function` ([`step_budget_ends_an_endless_dynamic_function`]).
#[test]
fn step_budget_ends_an_endless_constructed_dynamic_function() {
    ends_in_step_budget("new Function(\"a\", \"for (;;) { a++; }\")(0);");
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

/// The `frame_depth` of [`dynamic_function_recursion_ends_in_a_range_error`]: 500, the full
/// count, in the default mode; 100 with the collector at every safe point, in the gate and in
/// the lane `slow` (`SWELL_FULL_COUNTS=1`) alike, where every step collects and every
/// collection traces every frame, so the cost grows with the square of the depth (500 took
/// 26.2 s there at a load of 100 to 110 on 17 cores, cargo-nextest, and 3.2 CPU seconds alone
/// in the lane, measured on 2026-11-22). At 100 the frames of 50 calls of `g` and 50 dynamic
/// functions are live across those collections and the depth still throws the `RangeError`;
/// the default mode shows at 500 that no Rust call is made per level.
fn dynamic_frame_depth() -> u32 {
    if cfg!(feature = "collect-every-safe-point") {
        100
    } else {
        500
    }
}

/// `frame_depth` through `Function`: a function that makes a dynamic function calling it again
/// pushes a frame for each call, with no Rust recursion, until `frame_depth`
/// ([`dynamic_frame_depth`]) throws a `RangeError`, after `frame_depth / 2` calls of `g`.
#[test]
fn dynamic_function_recursion_ends_in_a_range_error() {
    let mut limits = limits();
    limits.frame_depth = dynamic_frame_depth();
    assert_eq!(
        run(limits, "function g() { Function(\"g()\")(); } g();"),
        Outcome::Throw("RangeError".to_string())
    );
    assert_eq!(
        run(
            limits,
            "var n = 0; function g() { n++; Function(\"g()\")(); } try { g(); } catch (e) { n; }"
        ),
        Outcome::Normal((limits.frame_depth / 2).to_string())
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

/// `%GeneratorFunction%`, which no global names: the `constructor` of
/// `%GeneratorFunction.prototype%`, the [[Prototype]] of a generator function.
const GENERATOR_FUNCTION: &str = "var GF = Object.getPrototypeOf(function* () {}).constructor;\n";

/// `source` after [`GENERATOR_FUNCTION`], with the test limits, in a fresh agent and realm.
fn run_with_generator_function(source: &str) -> Outcome {
    run(limits(), &format!("{GENERATOR_FUNCTION}{source}"))
}

/// The String `source` answers after [`GENERATOR_FUNCTION`], run by an agent of `host` with the
/// test limits, and the calls of its hook; the realm the hook compares with is the agent's.
fn run_on_compile_host(host: CompileHost, source: &str) -> (String, Vec<Request>) {
    let mut agent = Agent::new(host, limits(), support::host::seeds()).unwrap();
    let HostOutcome::Normal(realm) = agent.create_realm(None, None).unwrap() else {
        panic!("create_realm did not answer Normal");
    };
    let realm_ref = agent
        .with_cx(&realm, |cx| cx.realm_ref(&realm).unwrap())
        .unwrap();
    REALM.with(|cell| cell.set(Some(realm_ref)));
    let source = format!("{GENERATOR_FUNCTION}{source}");
    let Parsed::Parsed(script) = agent
        .parse_script(&realm, Source::Utf8(&source), None)
        .unwrap()
    else {
        panic!("the script did not parse");
    };
    let HostOutcome::Normal(result) = agent.evaluate_script(&script).unwrap() else {
        panic!("the script did not complete normally");
    };
    let answer = agent
        .with_cx(&realm, |cx| match cx.get(&result).unwrap() {
            Value::String(string) => cx.to_rust_string_lossy(string).unwrap(),
            other => format!("{other:?}"),
        })
        .unwrap();
    (answer, std::mem::take(&mut agent.host_mut().calls))
}

/// `#sec-generatorfunction` and `#sec-createdynamicfunction` of kind generator: called or
/// constructed, `%GeneratorFunction%` makes a generator function of its parameter arguments and
/// its body argument (none: the empty body), named "anonymous", of the length its parameters
/// give, whose [[Prototype]] is `%GeneratorFunction.prototype%`, with no [[Construct]], and with
/// a `prototype` { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: false } that
/// holds a new object whose [[Prototype]] is `%GeneratorPrototype%` and that has no
/// `constructor` (step 32). Its generators run its body; `yield` in it is a YieldExpression.
/// Its [[SourceText]] is the whole text `function* anonymous(<P>\n) {\n<body>\n}`.
#[test]
fn generator_function_makes_a_generator_function_from_its_arguments() {
    assert_eq!(
        run_with_generator_function(
            "var g = GF(\"a\", \"b\", \"yield a; yield b; return a + b;\");\n\
             var it = g(1, 2);\n\
             var r = [it.next().value, it.next().value, it.next().value, it.next().done];\n\
             var d = Object.getOwnPropertyDescriptor(g, \"prototype\");\n\
             var GP = Object.getPrototypeOf(g.prototype);\n\
             r.push(g.name, g.length, Object.getPrototypeOf(g) === GF.prototype,\n\
                    GP === GF.prototype.prototype, Object.getPrototypeOf(it) === g.prototype,\n\
                    d.writable, d.enumerable, d.configurable,\n\
                    Object.prototype.hasOwnProperty.call(g.prototype, \"constructor\"),\n\
                    g instanceof GF);\n\
             var made;\n\
             try { new g(); made = \"constructed\"; } catch (e) { made = e.name; }\n\
             r.push(made, new GF(\"yield 4\")().next().value, GF()().next().done,\n\
                    GF(\"x = yield\")().next().done,\n\
                    g.toString() === \"function* anonymous(a,b\\n) {\\nyield a; yield b; return a + b;\\n}\");\n\
             r.join();"
        ),
        Outcome::Normal(
            "\"1,2,3,true,anonymous,2,true,true,true,true,false,false,false,true,TypeError,4,true,false,true\""
                .to_string()
        )
    );
}

/// HOST-117 for `%GeneratorFunction%` (`#sec-createdynamicfunction`, step 16;
/// `#sec-hostensurecancompilestrings`): `GF("a", "yield a")` calls
/// `host_ensure_can_compile_strings` once, before it parses, with the compilation type
/// `Function` the standard gives every kind, `param_strings` `["a"]`, `body_string`
/// `"yield a"`, the generator's whole source text `function* anonymous(a\n) {\nyield a\n}` as
/// `code_string`, the arguments as `param_args` and `body_arg`, and the current realm; the
/// generator function it makes then yields its argument.
#[test]
fn generator_function_asks_the_host_hook_with_the_source_text() {
    let (answer, calls) = run_on_compile_host(
        CompileHost::default(),
        "String(GF(\"a\", \"yield a\")(7).next().value);",
    );
    assert_eq!(answer, "7");
    assert_eq!(
        calls,
        [Request {
            params: vec!["a".to_string()],
            body: "yield a".to_string(),
            code: "function* anonymous(a\n) {\nyield a\n}".to_string(),
            kind: CompilationType::Function,
            param_args: vec!["a".to_string()],
            body_arg: "yield a".to_string(),
            same_realm: true,
        }]
    );
}

/// HOST-117 for `%GeneratorFunction%`: a throw of `host_ensure_can_compile_strings` is thrown
/// by `GF`, called or constructed, before anything is parsed: a source that would not parse
/// throws the hook's value, not a `SyntaxError`, and no generator function is made.
#[test]
fn a_refusal_of_the_host_hook_is_thrown_by_generator_function() {
    let host = CompileHost {
        refuse: true,
        ..CompileHost::default()
    };
    let (answer, calls) = run_on_compile_host(
        host,
        "var r = [];\n\
         try { GF(\"yield 1\"); r.push(\"made\"); } catch (e) { r.push(e); }\n\
         try { new GF(\"a\", \"yield 1 +\"); r.push(\"made\"); } catch (e) { r.push(e); }\n\
         r.join();",
    );
    assert_eq!(answer, "refused,refused");
    assert_eq!(calls.len(), 2);
    assert!(
        calls
            .iter()
            .all(|call| call.kind == CompilationType::Function
                && call.code.starts_with("function* ")),
        "{calls:?}"
    );
}

/// `#sec-createdynamicfunction` of kind generator: the parameters are
/// `FormalParameters[+Yield, ~Await]` and the source a `GeneratorExpression` with its early
/// errors (`#sec-generator-function-definitions-static-semantics-early-errors`): a
/// YieldExpression in the parameters, and `yield` as a parameter's or a variable's name, are
/// each a `SyntaxError`, where the `Function` constructor, whose kind is normal, takes `yield`
/// as a name.
#[test]
fn yield_in_the_parameters_of_generator_function_is_a_syntax_error() {
    assert_eq!(
        run_with_generator_function(
            "var r = [];\n\
             var sources = [[\"x = yield\", \"\"], [\"x = yield 1\", \"\"], [\"yield\", \"\"],\n\
                            [\"var yield;\"], [\"a\", \"(function* () {})(yield)\"]];\n\
             for (var i = 0; i < sources.length; i++) {\n\
               try { GF.apply(null, sources[i]); r.push(\"made\"); } catch (e) { r.push(e.name); }\n\
             }\n\
             r.push(Function(\"yield\", \"return yield\")(5));\n\
             r.join();"
        ),
        Outcome::Normal("\"SyntaxError,SyntaxError,SyntaxError,SyntaxError,made,5\"".to_string())
    );
}

/// A host constructor whose "prototype" the test sets; it is only used as a new target.
fn unused(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Undefined)
}

/// `#sec-createdynamicfunction`, step 26: GetPrototypeFromConstructor(newTarget,
/// "%GeneratorFunction.prototype%"): constructed with a new target whose `prototype` is an
/// object, the generator function's [[Prototype]] is that object; with one whose `prototype` is
/// no object, `%GeneratorFunction.prototype%` of the new target's realm; called (no new
/// target, so `%GeneratorFunction%` itself), `%GeneratorFunction.prototype%`.
#[test]
fn generator_function_takes_its_prototype_from_new_target() {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let HostOutcome::Normal(generator_function) = eval(
        &mut agent,
        &realm,
        "Object.getPrototypeOf(function* () {}).constructor;",
    ) else {
        panic!("no %GeneratorFunction%");
    };
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let gf = object(cx, &generator_function);
            let gf_prototype = get_object(cx, gf, "prototype");
            let object_constructor = get_object(cx, global, "Object");
            let get_prototype_of = get(cx, object_constructor, "getPrototypeOf");
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let prototype_key = key(cx, "prototype");
            let body = Value::String(cx.new_string("yield 1").unwrap());
            for (prototype, expected) in [(true, None), (false, Some(gf_prototype))] {
                let new_target = cx
                    .new_host_function(realm_ref, "N", 0, unused, true, None)
                    .unwrap()
                    .unwrap();
                let own = cx.to_object(Value::Number(1.0)).unwrap().unwrap();
                let value = if prototype {
                    Value::Object(own)
                } else {
                    Value::Number(1.0)
                };
                cx.set(new_target, prototype_key, value, true)
                    .unwrap()
                    .unwrap();
                let made = cx
                    .construct(Value::Object(gf), &[body], Some(Value::Object(new_target)))
                    .unwrap()
                    .unwrap();
                let found = cx
                    .call(get_prototype_of, Value::Undefined, &[made])
                    .unwrap()
                    .unwrap();
                assert_eq!(found, Value::Object(expected.unwrap_or(own)), "{prototype}");
            }
            let made = cx
                .call(Value::Object(gf), Value::Undefined, &[body])
                .unwrap()
                .unwrap();
            let found = cx
                .call(get_prototype_of, Value::Undefined, &[made])
                .unwrap()
                .unwrap();
            assert_eq!(found, Value::Object(gf_prototype));
        })
        .unwrap();
}

/// `#sec-generatorfunction.prototype.constructor` and
/// `#sec-properties-of-the-generatorfunction-constructor`: the `constructor` of
/// `%GeneratorFunction.prototype%` is `%GeneratorFunction%`, { [[Writable]]: false,
/// [[Enumerable]]: false, [[Configurable]]: true }; `%GeneratorFunction%` is a function named
/// "GeneratorFunction" of length 1 whose [[Prototype]] is `%Function%` and whose `prototype`
/// is `%GeneratorFunction.prototype%`, { [[Writable]]: false, [[Enumerable]]: false,
/// [[Configurable]]: false }; it is no property of the global object, and every generator
/// function is an instance of it.
#[test]
fn the_constructor_of_generator_function_prototype_is_generator_function() {
    assert_eq!(
        run_with_generator_function(
            "var GFP = Object.getPrototypeOf(function* () {});\n\
             var c = Object.getOwnPropertyDescriptor(GFP, \"constructor\");\n\
             var p = Object.getOwnPropertyDescriptor(GF, \"prototype\");\n\
             [typeof GF, GF.name, GF.length, Object.getPrototypeOf(GF) === Function,\n\
              GF.prototype === GFP, c.value === GF, c.writable, c.enumerable, c.configurable,\n\
              p.writable, p.enumerable, p.configurable, typeof GeneratorFunction,\n\
              \"GeneratorFunction\" in this, (function* () {}) instanceof GF,\n\
              Object.isExtensible(GF)].join();"
        ),
        Outcome::Normal(
            "\"function,GeneratorFunction,1,true,true,true,false,false,true,false,false,false,\
             undefined,false,true,true\""
                .to_string()
        )
    );
}

/// HOST-139 to HOST-141 through `%GeneratorFunction%` (decision 9, item 9): a body, and a
/// parameter's initializer, nested deeper than `parser_nesting` (1,000) throw a `SyntaxError`
/// whose message names `parser_nesting`, which the script catches; with 10 levels the generator
/// yields 0.
#[test]
fn generator_function_reaches_parser_nesting_as_a_syntax_error() {
    let deep = nested(2_000);
    let caught = run_with_generator_function(&format!(
        "var r = [];\n\
         try {{ GF(\"yield \" + \"{deep}\"); }} catch (e) {{ r.push(e.name + \": \" + e.message); }}\n\
         try {{ GF(\"a = \" + \"{deep}\", \"\"); }} catch (e) {{ r.push(e.name + \": \" + e.message); }}\n\
         r.join(\"|\");"
    ));
    let Outcome::Normal(caught) = caught else {
        panic!("the script did not complete normally: {caught:?}");
    };
    let caught = caught.trim_matches('"');
    let parts: Vec<&str> = caught.split('|').collect();
    assert_eq!(parts.len(), 2, "{caught}");
    for part in parts {
        assert!(
            part.starts_with("SyntaxError: ") && part.contains("parser_nesting (1000)"),
            "{part}"
        );
    }
    assert_eq!(
        run_with_generator_function(&format!(
            "GF(\"yield \" + \"{}\")().next().value;",
            nested(10)
        )),
        Outcome::Normal("0".to_string())
    );
}

/// `#sec-createdynamicfunction`, steps 9 and 10, ToString of each argument: a Symbol as a
/// parameter argument or as the body argument throws a `TypeError` before the hook is asked
/// (the choice for `%GeneratorFunction%`, which the sweep of every function with a Symbol,
/// walking from the global object, does not reach). The constructor called is no `Function`.
#[test]
fn generator_function_with_a_symbol_throws_a_type_error() {
    assert_eq!(
        run_with_generator_function(
            "var r = [GF === Function];\n\
             var args = [[Symbol()], [Symbol(), \"\"], [\"a\", Symbol(), \"\"], [\"a\", Symbol()]];\n\
             for (var i = 0; i < args.length; i++) {\n\
               try { GF.apply(null, args[i]); r.push(\"made\"); } catch (e) { r.push(e.name); }\n\
               try { new GF(args[i][0]); r.push(\"made\"); } catch (e) { r.push(e.name); }\n\
             }\n\
             r.join();"
        ),
        Outcome::Normal(
            "\"false,TypeError,TypeError,TypeError,TypeError,TypeError,made,TypeError,made\""
                .to_string()
        )
    );
}

//! `Function.prototype.toString` (`#sec-function.prototype.tostring`) with the source text the
//! engine keeps of every script, eval code and dynamic function (`docs/design/engine.md`,
//! decision 4, "Source text."), HostHasSourceTextAvailable (`#sec-hosthassourcetextavailable`,
//! `HOST-120`), and `Function.prototype.apply` (`#sec-function.prototype.apply`) with
//! CreateListFromArrayLike (`#sec-createlistfromarraylike`). Written by
//! `story:function-apply-and-source-text`. Run also with the feature `collect-every-safe-point`.
//!
//! A text is read out of the String a script answers and compared here; an argument list is an
//! array literal, an `arguments` object or an object with `length`, and what a callee received
//! is read element by element: no builtin of another story of the wave is called.
#![cfg(test)]

use crate::support;

use support::host::{agent, agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, define_function, eval_text, nothing, parse, run};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{
    Agent, CollectReport, HookCx, Host, Job, LoadRequest, ObjectRef, Outcome as HostOutcome,
    Parsed, PropertyKey, Realm, Root, Seeds, Source, Termination, Value,
};

/// What `eval_text` answers for a String result `text`.
fn string(text: &str) -> Outcome {
    Outcome::Normal(format!("\"{text}\""))
}

/// Evaluate each `(expression, expected text)` in `realm`.
fn expect_texts(agent: &mut Agent<support::host::TestHost>, realm: &Realm, cases: &[(&str, &str)]) {
    for (expression, expected) in cases {
        assert_eq!(
            eval_text(agent, realm, expression),
            string(expected),
            "{expression}"
        );
    }
}

/// A collection, which must complete.
fn collect<H: Host>(agent: &mut Agent<H>) -> CollectReport {
    match agent.collect().unwrap() {
        HostOutcome::Normal(report) => report,
        other => panic!("collect answered {other:?}"),
    }
}

/// `#sec-function.prototype.tostring`, step 2: the [[SourceText]] of an ECMAScript function,
/// the source text matched by its FunctionDeclaration, FunctionExpression or MethodDefinition
/// (`#sec-runtime-semantics-instantiateordinaryfunctionobject`,
/// `#sec-runtime-semantics-definemethod`): comments and line ends inside it are kept, the text
/// around it is not.
#[test]
fn to_string_of_a_declaration_expression_method_getter_and_setter_is_its_source_text() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let source = "/* before */ function d(a, /* in */ b) {\n  return a; // end\n} /* after */\n\
                  var e = /* lead */ function (x) {\r\n  return x;\u{2028}} /* tail */;\n\
                  var o = { /* 1 */ get p() { return 1; } /* 2 */, set p(v) { /* set */ },\n\
                  m(a) { return function () {}; } /* 3 */ };\n\
                  var desc = Object.getOwnPropertyDescriptor(o, \"p\");";
    assert!(matches!(
        eval_text(&mut agent, &realm, source),
        Outcome::Normal(_)
    ));
    expect_texts(
        &mut agent,
        &realm,
        &[
            (
                "d.toString();",
                "function d(a, /* in */ b) {\n  return a; // end\n}",
            ),
            ("e.toString();", "function (x) {\r\n  return x;\u{2028}}"),
            ("desc.get.toString();", "get p() { return 1; }"),
            ("desc.set.toString();", "set p(v) { /* set */ }"),
            ("o.m.toString();", "m(a) { return function () {}; }"),
            ("o.m().toString();", "function () {}"),
            (
                "Function.prototype.toString.call(d);",
                "function d(a, /* in */ b) {\n  return a; // end\n}",
            ),
        ],
    );
}

/// `#sec-createdynamicfunction`: the [[SourceText]] of a function `Function` makes is the text of
/// its third parse, `function anonymous(` P `\n) {\n` body `\n}`, P the parameter arguments
/// joined with ","; a function in its body has its own text out of that one.
#[test]
fn to_string_of_a_dynamic_function_is_the_text_of_its_third_parse() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    expect_texts(
        &mut agent,
        &realm,
        &[
            (
                "Function(\"a\", \"b\", \"return a\").toString();",
                "function anonymous(a,b\n) {\nreturn a\n}",
            ),
            ("Function().toString();", "function anonymous(\n) {\n\n}"),
            (
                "new Function(\"a,\\nb\", \"/* c */ return a\").toString();",
                "function anonymous(a,\nb\n) {\n/* c */ return a\n}",
            ),
            (
                "Function(\"return function inner(x) { return x; }\")().toString();",
                "function inner(x) { return x; }",
            ),
        ],
    );
}

/// The text of eval code stays as long as a function made from it (decision 4): a function a
/// direct and an indirect `eval` made answers its text after the eval returned, the script that
/// called it was released and a collection ran.
#[test]
fn to_string_of_a_function_made_by_eval_outlives_the_eval() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let made = "var f = eval(\"/* lead */ (function g(a) { return a; }) /* tail */\");\n\
                var h = (0, eval)(\"var unused = 1; (function k() { /* k */ })\");";
    assert!(matches!(
        eval_text(&mut agent, &realm, made),
        Outcome::Normal(_)
    ));
    collect(&mut agent);
    collect(&mut agent);
    expect_texts(
        &mut agent,
        &realm,
        &[
            ("f.toString();", "function g(a) { return a; }"),
            ("h.toString();", "function k() { /* k */ }"),
        ],
    );
}

/// `#sec-function.prototype.tostring`, steps 3 and 4: a built-in function, a host function
/// (CreateBuiltinFunction, `docs/design/host-interface.md`) and a bound function answer a String
/// with the syntax of a NativeFunction (`prod-NativeFunction`); a built-in one names its
/// [[InitialName]] where the grammar has its PropertyName, `%Function.prototype%` the empty one.
#[test]
fn to_string_of_builtin_bound_and_host_functions_is_a_native_function_string() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "hostFn", 0, nothing, false);
    expect_texts(
        &mut agent,
        &realm,
        &[
            (
                "Function.prototype.call.toString();",
                "function call() { [native code] }",
            ),
            (
                "Function.prototype.apply.toString();",
                "function apply() { [native code] }",
            ),
            (
                "Function.prototype.toString.toString();",
                "function toString() { [native code] }",
            ),
            (
                "Function.toString();",
                "function Function() { [native code] }",
            ),
            ("Math.max.toString();", "function max() { [native code] }"),
            (
                "Function.prototype.toString.call(Function.prototype);",
                "function () { [native code] }",
            ),
            ("hostFn.toString();", "function hostFn() { [native code] }"),
            (
                "(function f() {}).bind(null).toString();",
                "function () { [native code] }",
            ),
            (
                "Function.prototype.call.bind(null).toString();",
                "function () { [native code] }",
            ),
        ],
    );
}

/// `#sec-function.prototype.tostring`, step 5: a this value that is not callable throws a
/// TypeError.
#[test]
fn to_string_of_a_non_callable_throws_type_error() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    for this in ["{}", "1", "\"s\"", "undefined", "null", "true", "[]"] {
        let source = format!("Function.prototype.toString.call({this});");
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Outcome::Throw("TypeError".to_string()),
            "{source}"
        );
    }
}

/// A host whose HostHasSourceTextAvailable answers false for the function `hidden` and true for
/// any other, and records each function it is asked about.
#[derive(Default)]
struct SourceHost {
    hidden: Option<ObjectRef>,
    asked: Vec<ObjectRef>,
}

impl Host for SourceHost {
    fn host_has_source_text_available(cx: &mut HookCx<'_, Self>, func: ObjectRef) -> bool {
        let host = cx.host_mut();
        host.asked.push(func);
        host.hidden != Some(func)
    }

    fn host_enqueue_generic_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}

    fn host_enqueue_timeout_job(_cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {}

    fn host_enqueue_finalization_registry_cleanup_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}

    fn host_load_imported_module(_cx: &mut HookCx<'_, Self>, _request: LoadRequest) {}

    fn host_system_utc_epoch_nanoseconds(_cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        0
    }
}

/// The value the script `source` answers in `realm`, which must complete normally.
fn evaluated(agent: &mut Agent<SourceHost>, realm: &Realm, source: &str) -> Root {
    let Parsed::Parsed(script) = agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    else {
        panic!("{source:?} did not parse");
    };
    match agent.evaluate_script(&script).unwrap() {
        HostOutcome::Normal(root) => root,
        other => panic!("{source:?} answered {other:?}"),
    }
}

/// The text of the String the script `source` answers.
fn text_of(agent: &mut Agent<SourceHost>, realm: &Realm, source: &str) -> String {
    let root = evaluated(agent, realm, source);
    agent
        .with_cx(realm, |cx| match cx.get(&root).unwrap() {
            Value::String(string) => cx.to_rust_string_lossy(string).unwrap(),
            other => panic!("{source:?} answered {other:?}"),
        })
        .unwrap()
}

/// The function object the global variable `name` holds.
fn global_function(agent: &mut Agent<SourceHost>, realm: &Realm, name: &str) -> ObjectRef {
    let global = agent.global_object(realm).unwrap();
    agent
        .with_cx(realm, |cx| {
            let Value::Object(global) = cx.get(&global).unwrap() else {
                panic!("the global object is not an object");
            };
            let key = PropertyKey::String(cx.new_string(name).unwrap());
            match cx.get_property(global, key).unwrap().unwrap() {
                Value::Object(function) => function,
                other => panic!("{name} is {other:?}"),
            }
        })
        .unwrap()
}

/// `HOST-120` (`#sec-hosthassourcetextavailable`, `#sec-function.prototype.tostring`, step 2):
/// for a function for which `host_has_source_text_available` answers false,
/// `Function.prototype.toString` returns a NativeFunction string; for one for which it answers
/// true, the source text. The hook is called once per `toString` of an ECMAScript function, with
/// that function, and not for a built-in or a bound function.
#[test]
fn host_120_no_source_text_gives_a_native_function_string() {
    let mut agent = Agent::new(
        SourceHost::default(),
        limits(),
        Seeds {
            hash_key: [7; 16],
            random_seed: [11; 32],
        },
    )
    .unwrap();
    let realm = match agent.create_realm(None, None).unwrap() {
        HostOutcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    };
    evaluated(
        &mut agent,
        &realm,
        "var shown = function () { return 1; };\nvar hidden = function () { return 2; };",
    );
    let shown = global_function(&mut agent, &realm, "shown");
    let hidden = global_function(&mut agent, &realm, "hidden");
    agent.host_mut().hidden = Some(hidden);
    assert!(agent.host().asked.is_empty());

    assert_eq!(
        text_of(&mut agent, &realm, "shown.toString();"),
        "function () { return 1; }"
    );
    assert_eq!(agent.host().asked, [shown]);
    assert_eq!(
        text_of(&mut agent, &realm, "hidden.toString();"),
        "function () { [native code] }"
    );
    assert_eq!(agent.host().asked, [shown, hidden]);
    assert_eq!(
        text_of(
            &mut agent,
            &realm,
            "Function.prototype.toString.call(hidden) + '|' + hidden.bind(null).toString() + '|' + \
             Function.prototype.call.toString();"
        ),
        "function () { [native code] }|function () { [native code] }|\
         function call() { [native code] }"
    );
    assert_eq!(agent.host().asked, [shown, hidden, hidden]);
}

/// Decision 4, "What a test reads": the source text of a script is charged when `parse_script`
/// ends, with the compiled code, and stays charged as long as the host holds the script or a
/// function made from it is reachable; the collection after both are gone credits it. With a
/// source of a comment of 100,000 code units and one function, in both orders of release. A
/// text that does not fit what is left of the budget ends the parse in
/// `Terminated(MemoryBudget)`.
#[test]
fn the_source_text_is_charged_and_credited() {
    const UNITS: u64 = 100_000;
    let source = format!("/*{}*/ (function () {{ return 1; }});", "x".repeat(100_000));
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);

    // The host releases the script first, then the function.
    collect(&mut agent);
    let base = agent.heap_bytes().unwrap();
    let script = parse(&mut agent, &realm, &source);
    let parsed = agent.heap_bytes().unwrap();
    assert!(
        parsed >= base + UNITS,
        "{parsed} after the parse, {base} before"
    );
    let HostOutcome::Normal(function) = agent.evaluate_script(&script).unwrap() else {
        panic!("the script did not complete normally");
    };
    drop(script);
    let report = collect(&mut agent);
    assert!(
        report.heap_bytes_after >= base + UNITS,
        "{report:?} with the function reachable, {base} before the parse"
    );
    drop(function);
    let report = collect(&mut agent);
    assert!(
        report.heap_bytes_before - report.heap_bytes_after >= UNITS,
        "{report:?}: the text was not given back"
    );

    // The host releases the function first, then the script.
    let base = agent.heap_bytes().unwrap();
    let script = parse(&mut agent, &realm, &source);
    let HostOutcome::Normal(function) = agent.evaluate_script(&script).unwrap() else {
        panic!("the script did not complete normally");
    };
    drop(function);
    let report = collect(&mut agent);
    assert!(
        report.heap_bytes_after >= base + UNITS,
        "{report:?} with the script held, {base} before the parse"
    );
    drop(script);
    let report = collect(&mut agent);
    assert!(
        report.heap_bytes_before - report.heap_bytes_after >= UNITS,
        "{report:?}: the text was not given back"
    );

    // The budget: what a realm takes, then room for less than the text.
    let mut small = limits();
    let mut probe = agent_with(small).unwrap();
    support::host::realm(&mut probe);
    let taken = probe.heap_bytes().unwrap();
    small.memory_bytes = taken + UNITS / 2 * 3;
    let mut agent = agent_with(small).unwrap();
    let realm = support::host::realm(&mut agent);
    assert!(matches!(
        agent
            .parse_script(&realm, Source::Utf8(&source), None)
            .unwrap(),
        Parsed::Terminated(Termination::MemoryBudget)
    ));
}

/// The callee of the tests of `apply`: what it received, `this.k`, the number of arguments and
/// the first three, joined with "|".
const RECEIVER: &str = "var t = { k: \"t\" };\n\
                        function f(a, b, c) {\n\
                          return this.k + \"|\" + arguments.length + \"|\" + a + \"|\" + b + \"|\" + c;\n\
                        }\n";

/// `#sec-function.prototype.apply`, steps 4 to 6, with CreateListFromArrayLike
/// (`#sec-createlistfromarraylike`): the elements of an array, of an `arguments` object and of
/// an object with `length` (ToLength of it, `#sec-lengthofarraylike`), read in index order by
/// Get, each a value of its own kept across the next Get, a hole read as `undefined`; a built-in
/// function as the callee.
#[test]
fn apply_with_an_array_arguments_and_an_array_like() {
    for (call, expected) in [
        ("f.apply(t, [1, 2]);", "t|2|1|2|undefined"),
        (
            "(function () { return f.apply(t, arguments); })(7, 8, 9);",
            "t|3|7|8|9",
        ),
        (
            "f.apply(t, { length: 2, 0: \"x\", 1: \"y\", 2: \"z\" });",
            "t|2|x|y|undefined",
        ),
        (
            "f.apply(t, { length: \"2.9\", 0: \"x\", 1: \"y\" });",
            "t|2|x|y|undefined",
        ),
        (
            "f.apply(t, { length: -1, 0: \"x\" });",
            "t|0|undefined|undefined|undefined",
        ),
        ("f.apply(t, [, 5]);", "t|2|undefined|5|undefined"),
        (
            "var log = \"\";\n\
             var like = { length: 3,\n\
               get 0() { log += \"0\"; return { v: \"a\" }; },\n\
               get 1() { log += \"1\"; return { v: \"b\" }; },\n\
               get 2() { log += \"2\"; return { v: \"c\" }; } };\n\
             (function (a, b, c) { return a.v + b.v + c.v; }).apply(null, like) + \"|\" + log;",
            "abc|012",
        ),
    ] {
        let source = format!("{RECEIVER}{call}");
        assert_eq!(run(limits(), &source), string(expected), "{call}");
    }
    assert_eq!(
        run(limits(), "Math.max.apply(null, [1, 3, 2]);"),
        Outcome::Normal("3".to_string())
    );
}

/// `#sec-function.prototype.apply`, step 3: an argument list that is undefined or null calls the
/// function with no argument.
#[test]
fn apply_with_undefined_and_null_calls_with_no_argument() {
    for call in ["f.apply(t);", "f.apply(t, undefined);", "f.apply(t, null);"] {
        let source = format!("{RECEIVER}{call}");
        assert_eq!(
            run(limits(), &source),
            string("t|0|undefined|undefined|undefined"),
            "{call}"
        );
    }
}

/// `#sec-createlistfromarraylike`, step 2: an argument list that is not an object throws a
/// TypeError; `#sec-function.prototype.apply`, step 2: so does a this value that is not
/// callable, before the argument list is read.
#[test]
fn apply_refuses_a_primitive_argument_list() {
    for call in [
        "f.apply(t, 1);",
        "f.apply(t, \"ab\");",
        "f.apply(t, true);",
        "Function.prototype.apply.call({}, t, []);",
        "Function.prototype.apply.call(1, t, { get length() { throw 1; } });",
    ] {
        let source = format!("{RECEIVER}{call}");
        assert_eq!(
            run(limits(), &source),
            Outcome::Throw("TypeError".to_string()),
            "{call}"
        );
    }
}

/// The steps the entry `g.apply(null, o)` spends, `o` an array-like of `length` elements that
/// are its own data properties (made by an entry before it, so that a Get of one walks no
/// prototype chain, which OrdinaryGet charges a step per object of).
fn apply_steps(length: u32) -> u64 {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = format!(
        "function g() {{}}\nvar o = {{ length: {length} }};\n\
         for (var i = 0; i < {length}; i++) {{ o[i] = i; }}"
    );
    assert!(matches!(
        eval_text(&mut agent, &realm, &setup),
        Outcome::Normal(_)
    ));
    assert_eq!(
        eval_text(&mut agent, &realm, "g.apply(null, o);"),
        Outcome::Normal("undefined".to_string())
    );
    agent.last_entry_steps().unwrap()
}

/// CreateListFromArrayLike is a loop a script runs: a step per element (`AGENTS.md`, "Rules"),
/// so n elements more cost n steps more: 100 and 200 elements in the default mode, 10 and 20
/// with the feature `collect-every-safe-point` (the file has no full count), where the loop
/// that makes the elements collects at every instruction; a list that charged no step per
/// element costs both the same at any length. (1,000 and 2,000 took 7.2 s there, measured;
/// 100 and 200 took 1.13 CPU seconds alone, against 0.02 in the default mode, measured on
/// 2026-11-23.)
#[test]
fn apply_charges_a_step_per_element() {
    let n = if cfg!(feature = "collect-every-safe-point") {
        10
    } else {
        100
    };
    let (one, two) = (apply_steps(n), apply_steps(2 * n));
    assert_eq!(
        two - one,
        u64::from(n),
        "{one} steps for {n}, {two} for {}",
        2 * n
    );
}

/// A length of 2^53 (ToLength makes it 2^53 - 1, and so it does of `Infinity`) ends in a budget,
/// never in a panic: the step budget of 10,000 before 10,000 elements are read, or with steps
/// to spare the memory budget, which the list is charged to as it grows.
#[test]
fn apply_of_a_length_of_2_53_ends_in_a_budget() {
    let script =
        |length: &str| format!("function g() {{}}\ng.apply(null, {{ length: {length} }});");
    let mut steps = limits();
    steps.steps_per_entry = 10_000;
    for length in ["9007199254740992", "Infinity"] {
        assert_eq!(
            run(steps, &script(length)),
            Outcome::Terminated(Termination::StepBudget),
            "{length}"
        );
    }

    let mut memory = limits();
    memory.steps_per_entry = 1 << 40;
    let mut probe = agent_with(memory).unwrap();
    realm(&mut probe);
    memory.memory_bytes = probe.heap_bytes().unwrap() + (1 << 20);
    assert_eq!(
        run(memory, &script("9007199254740992")),
        Outcome::Terminated(Termination::MemoryBudget)
    );
}

/// The body of the stack test through `apply`.
fn reach_through_apply() {
    reach_through("f.apply(null, [])", BUILTINS_REENTRY);
}

/// `Limits::min_stack_bytes` through `Function.prototype.apply`, which calls back into script
/// (`tests/support/stack.rs`): on seven eighths of `min_stack_bytes()` a script reaches
/// `reentry_depth` re-entries through `f.apply(null, [])` and parses 1,000 nested brackets at the
/// deepest, without a stack overflow.
#[test]
fn apply_reaches_the_reentry_limit_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("apply_reaches_the_reentry_limit_on_the_promised_stack"),
        stack,
        reach_through_apply,
    )
    .unwrap();
}

//! The Function builtins the harness files call (ECMA-262 20.2, `#sec-function-objects`):
//! `Function.prototype.call` (`#sec-function.prototype.call`), `Function.prototype.bind`
//! (`#sec-function.prototype.bind`) and the bound function exotic object it makes (10.4.1,
//! `#sec-bound-function-exotic-objects`, `#sec-boundfunctioncreate`). No script runs: the
//! targets are host functions that record the `this` value, the arguments and the new target
//! they receive. `Function.prototype[@@hasInstance]` needs a well-known symbol, which the host
//! interface does not hand out, so its test is a unit test of `builtins/function.rs`. Run also
//! with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{
    TestHost, agent, agent_with, data, get, get_object, global, is_error_of, key, limits, object,
    realm, string, text, thrown,
};
use support::stack::{TestPath, test_path};
use swell::{Completion, Cx, HostCall, ObjectRef, Outcome, Value};

/// How a recorded value reads: a String by its text, a Number by its value, an object by its
/// "name", anything else by its debug form. A String whose record is gone reads as the error.
fn describe(cx: &mut Cx<'_, TestHost>, value: Value) -> String {
    match value {
        Value::Undefined => "undefined".to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(string) => cx
            .to_rust_string_lossy(string)
            .unwrap_or_else(|error| format!("{error:?}")),
        Value::Object(object) => match get(cx, object, "name") {
            Value::String(name) => cx.to_rust_string_lossy(name).unwrap(),
            _ => "object".to_string(),
        },
        other => format!("{other:?}"),
    }
}

/// A host function that records its `this`, its arguments and, when constructed, its new
/// target, as one line in the host. Called, it answers undefined; constructed, itself.
fn record(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let mut parts = vec![describe(cx, call.this)];
    for &arg in call.args {
        parts.push(describe(cx, arg));
    }
    if let Some(new_target) = call.new_target {
        let name = describe(cx, Value::Object(new_target));
        parts.push(format!("new.target={name}"));
    }
    cx.host_mut().calls.push(parts.join(" "));
    match call.new_target {
        Some(_) => Ok(Value::Object(call.callee)),
        None => Ok(Value::Undefined),
    }
}

/// A host function `name` of `length` that records its calls, a constructor when `constructor`.
fn recorder(
    cx: &mut Cx<'_, TestHost>,
    realm: &swell::Realm,
    name: &str,
    length: u32,
    constructor: bool,
) -> ObjectRef {
    let realm = cx.realm_ref(realm).unwrap();
    cx.new_host_function(realm, name, length, record, constructor, None)
        .unwrap()
        .unwrap()
}

/// `%Function.prototype%`, read as the prototype of `Error` (a builtin function of the realm).
fn function_prototype(cx: &mut Cx<'_, TestHost>, global: ObjectRef) -> ObjectRef {
    let object_constructor = get_object(cx, global, "Object");
    let get_prototype_of = get(cx, object_constructor, "getPrototypeOf");
    let error = get(cx, global, "Error");
    match cx
        .call(get_prototype_of, Value::Undefined, &[error])
        .unwrap()
        .unwrap()
    {
        Value::Object(prototype) => prototype,
        other => panic!("{other:?}"),
    }
}

/// `function.bind(this, ...args)` through `Function.prototype.bind`.
fn bind(
    cx: &mut Cx<'_, TestHost>,
    global: ObjectRef,
    function: Value,
    args: &[Value],
) -> Completion<Value> {
    let prototype = function_prototype(cx, global);
    let bind = get(cx, prototype, "bind");
    cx.call(bind, function, args).unwrap()
}

/// `#sec-function.prototype.call`: the target receives the first argument as its `this` and the
/// rest as its arguments, unchanged; with no argument, `this` is undefined; `call` of `call`
/// passes through; a this value that is not callable throws a TypeError. `call` is a builtin of
/// length 1.
#[test]
fn call_passes_this_and_arguments() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let calls = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let f = Value::Object(recorder(cx, &realm, "f", 0, false));
            let prototype = function_prototype(cx, global);
            let call = get(cx, prototype, "call");
            let this = string(cx, "T");
            assert_eq!(
                cx.call(call, f, &[this, Value::Number(1.0), Value::Number(2.0)])
                    .unwrap()
                    .unwrap(),
                Value::Undefined
            );
            cx.call(call, f, &[]).unwrap().unwrap();
            cx.call(call, call, &[f, this, Value::Number(3.0)])
                .unwrap()
                .unwrap();
            let not_callable = Value::Object(cx.to_object(Value::Number(1.0)).unwrap().unwrap());
            let error = thrown(cx.call(call, not_callable, &[]).unwrap());
            assert!(is_error_of(cx, global, error, "TypeError"));
            let error = thrown(cx.call(call, Value::Undefined, &[]).unwrap());
            assert!(is_error_of(cx, global, error, "TypeError"));
            let Value::Object(call_object) = call else {
                panic!("{call:?}");
            };
            assert_eq!(get(cx, call_object, "length"), Value::Number(1.0));
            let name = get(cx, call_object, "name");
            assert_eq!(text(cx, name), "call");
            cx.host().calls.clone()
        })
        .unwrap();
    assert_eq!(calls, ["T 1 2", "undefined", "T 3"]);
}

/// `#sec-function.prototype.bind` with BoundFunctionCreate (`#sec-boundfunctioncreate`),
/// SetFunctionLength and SetFunctionName ("bound " and the target's name): the length is the
/// target's own Number length less the bound arguments, at least 0, +∞ for +∞, 0 for -∞, for a
/// length that is not a Number and for no own length; a name that is not a String is the empty
/// String. A bound function calls its target with the bound `this` and the bound arguments
/// before its own (10.4.1.1); a bound function of a bound function adds its arguments after the
/// inner ones and keeps the inner `this`. Constructed (10.4.1.2), the new target that is the
/// bound function becomes the target; another new target passes through; a bound function of a
/// function that is not a constructor is not one. Its [[Prototype]] is the target's and it has
/// no "prototype". A this value that is not callable throws a TypeError.
#[test]
fn bind_name_length_and_construct() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let calls = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let object_constructor = get_object(cx, global, "Object");
            let f = recorder(cx, &realm, "f", 3, false);
            let this = string(cx, "T");
            let bound = bind(cx, global, Value::Object(f), &[this, Value::Number(1.0)]).unwrap();
            let Value::Object(bound_object) = bound else {
                panic!("{bound:?}");
            };
            let name = get(cx, bound_object, "name");
            assert_eq!(text(cx, name), "bound f");
            assert_eq!(get(cx, bound_object, "length"), Value::Number(2.0));
            let x = string(cx, "x");
            assert_eq!(
                cx.call(bound, Value::Null, &[x]).unwrap().unwrap(),
                Value::Undefined
            );

            let other_this = string(cx, "U");
            let twice = bind(cx, global, bound, &[other_this, Value::Number(2.0)]).unwrap();
            let Value::Object(twice_object) = twice else {
                panic!("{twice:?}");
            };
            let name = get(cx, twice_object, "name");
            assert_eq!(text(cx, name), "bound bound f");
            assert_eq!(get(cx, twice_object, "length"), Value::Number(1.0));
            let y = string(cx, "y");
            cx.call(twice, Value::Undefined, &[y]).unwrap().unwrap();

            // Prototype and "prototype".
            let get_prototype_of = get(cx, object_constructor, "getPrototypeOf");
            let prototype = cx
                .call(get_prototype_of, Value::Undefined, &[bound])
                .unwrap()
                .unwrap();
            assert_eq!(prototype, Value::Object(function_prototype(cx, global)));
            let object_prototype = get_object(cx, object_constructor, "prototype");
            let has_own = get(cx, object_prototype, "hasOwnProperty");
            let prototype_name = string(cx, "prototype");
            assert_eq!(
                cx.call(has_own, bound, &[prototype_name]).unwrap().unwrap(),
                Value::Boolean(false)
            );

            // The lengths.
            let length_key = key(cx, "length");
            let cases = [
                (Value::Number(f64::INFINITY), 1, f64::INFINITY),
                (Value::Number(f64::NEG_INFINITY), 0, 0.0),
                (Value::Number(2.5), 1, 1.0),
                (Value::Number(1.0), 3, 0.0),
                (string(cx, "3"), 0, 0.0),
            ];
            for (length, bound_args, expected) in cases {
                let g = recorder(cx, &realm, "g", 0, false);
                cx.define_property_or_throw(g, length_key, data(length, false, false, true))
                    .unwrap()
                    .unwrap();
                let mut args = vec![Value::Undefined];
                args.extend(std::iter::repeat_n(Value::Null, bound_args));
                let Value::Object(made) = bind(cx, global, Value::Object(g), &args).unwrap() else {
                    panic!("bind made no object");
                };
                assert_eq!(
                    get(cx, made, "length"),
                    Value::Number(expected),
                    "{length:?}"
                );
            }
            let g = recorder(cx, &realm, "g", 4, false);
            cx.delete_property_or_throw(g, length_key).unwrap().unwrap();
            let Value::Object(made) = bind(cx, global, Value::Object(g), &[]).unwrap() else {
                panic!("bind made no object");
            };
            assert_eq!(get(cx, made, "length"), Value::Number(0.0));

            // A name that is not a String.
            let name_key = key(cx, "name");
            cx.define_property_or_throw(g, name_key, data(Value::Number(5.0), false, false, true))
                .unwrap()
                .unwrap();
            let Value::Object(made) = bind(cx, global, Value::Object(g), &[]).unwrap() else {
                panic!("bind made no object");
            };
            let name = get(cx, made, "name");
            assert_eq!(text(cx, name), "bound ");

            // Construct.
            let constructor = recorder(cx, &realm, "C", 1, true);
            let ignored = string(cx, "ignored");
            let bound_constructor = bind(
                cx,
                global,
                Value::Object(constructor),
                &[ignored, Value::Number(1.0)],
            )
            .unwrap();
            let z = string(cx, "z");
            let made = cx
                .construct(bound_constructor, &[z], None)
                .unwrap()
                .unwrap();
            assert_eq!(made, Value::Object(constructor));
            let other = recorder(cx, &realm, "N", 0, true);
            cx.construct(bound_constructor, &[], Some(Value::Object(other)))
                .unwrap()
                .unwrap();
            let error = thrown(cx.construct(bound, &[], None).unwrap());
            assert!(is_error_of(cx, global, error, "TypeError"));

            // Not callable.
            let plain = cx.to_object(Value::Boolean(true)).unwrap().unwrap();
            let error = thrown(bind(cx, global, Value::Object(plain), &[]));
            assert!(is_error_of(cx, global, error, "TypeError"));
            let prototype = function_prototype(cx, global);
            let bind_function = get_object(cx, prototype, "bind");
            assert_eq!(get(cx, bind_function, "length"), Value::Number(1.0));
            cx.host().calls.clone()
        })
        .unwrap();
    assert_eq!(
        calls,
        [
            "T 1 x",
            "T 1 2 y",
            "undefined 1 z new.target=C",
            "undefined 1 new.target=N"
        ]
    );
}

/// The trace function of the bound function kind (`object/bound_function.rs`) reports
/// [[BoundTargetFunction]], [[BoundThis]] and [[BoundArguments]]: a bound function rooted alone,
/// whose target, `this` and arguments nothing else holds, survives two collections, and a call
/// shows the target receiving them (10.4.1.1).
#[test]
fn bound_target_this_and_arguments_survive_collect() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let (bound, undefined, third) = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let f = recorder(cx, &realm, "f", 0, false);
            let this = string(cx, "bound this");
            let first = string(cx, "first");
            let second = string(cx, "second");
            let bound = bind(cx, global, Value::Object(f), &[this, first, second]).unwrap();
            let third = string(cx, "third");
            (
                cx.root(bound).unwrap(),
                cx.root(Value::Undefined).unwrap(),
                cx.root(third).unwrap(),
            )
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let outcome = agent.call(&realm, &bound, &undefined, &[&third]).unwrap();
    assert!(matches!(outcome, Outcome::Normal(_)), "{outcome:?}");
    assert_eq!(agent.host().calls, ["bound this first second third"]);
}

/// Set in the environment of the child process a test starts.
const STACK_CHILD: &str = "SWELL_BUILTINS_FUNCTION_STACK_CHILD";

/// The stack of the thread the child runs its body on: 256 KiB, far less than a chain of
/// 100,000 links would take if each link were a Rust call.
const STACK_BYTES: usize = 256 * 1024;

/// Runs `body` on a thread of [`STACK_BYTES`] in a child process that runs only the test
/// `test`. A stack overflow aborts the process, which no test can catch in its own process; in
/// the child it fails this test alone, with the child's output.
fn on_small_stack(test: TestPath, body: fn()) {
    let done = format!("{STACK_CHILD}: {test} done");
    if std::env::var_os(STACK_CHILD).is_some() {
        crate::support::host::stack_thread(STACK_BYTES)
            .spawn(body)
            .unwrap()
            .join()
            .unwrap();
        println!("{done}");
        return;
    }
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--exact", test.as_str(), "--nocapture", "--test-threads=1"])
        .env(STACK_CHILD, "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && stdout.contains(&done),
        "{test} on a stack of {STACK_BYTES} bytes: {}\n{stdout}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The number of links of the chain: 100,000; 2,000 with the feature `collect-every-safe-point`,
/// where each `bind` collects every link made so far, so the cost grows with the square of the
/// links (5,000 took 6.9 s there, so 100,000 would take about 46 minutes). The default mode
/// shows the stack bound; the collect mode shows that the chain stays rooted across every call.
const LINKS: u32 = if cfg!(feature = "collect-every-safe-point") {
    2_000
} else {
    100_000
};

/// The links of the chain in this run: [`LINKS`] in the collect mode; 50,000 in the default
/// mode's lane `slow` (`SWELL_FULL_COUNTS=1`; 100,000 took 2.7 CPU seconds alone there,
/// measured on 2026-11-22), 25,000 in its gate (100,000 took 19.0 s there at a load of 100 to
/// 110 on 17 cores, cargo-nextest). 25,000 still shows the stack bound: a Rust call per link
/// takes at least 16 bytes (a return address and a saved frame pointer), 400,000 bytes for the
/// chain, more than the [`STACK_BYTES`] (262,144) the chain is called on, and 50,000 three
/// times it.
fn links() -> u32 {
    if cfg!(feature = "collect-every-safe-point") {
        LINKS
    } else if std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1") {
        50_000
    } else {
        25_000
    }
}

/// A chain of 100,000 bound functions ([`links`] in this run), each bound to the one before
/// by `bind` through `cx.call`, is called and constructed on a 256 KiB stack: [[Call]] and
/// [[Construct]] (10.4.1.1, 10.4.1.2) gather the bound arguments in a loop over the chain, a step per link,
/// not by a Rust call per link. The target receives the innermost `this` and the arguments
/// innermost first; constructed, the new target that is the outermost link becomes the target.
/// Each link's "name" is set to undefined before the next bind, so that the names stay short
/// ("bound " and the empty String) instead of growing by six code units a link.
#[test]
fn bound_chain_of_100000_links_needs_no_rust_stack() {
    let test = test_path!("bound_chain_of_100000_links_needs_no_rust_stack");
    on_small_stack(test, || {
        let mut limits = limits();
        limits.memory_bytes = 512 * 1024 * 1024;
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        let global_root = global(&mut agent, &realm);
        let calls = agent
            .with_cx(&realm, |cx| {
                let global = object(cx, &global_root);
                let target = recorder(cx, &realm, "C", 0, true);
                let prototype = function_prototype(cx, global);
                let bind_function = get(cx, prototype, "bind");
                let name_key = key(cx, "name");
                let inner_this = string(cx, "inner");
                let outer_this = string(cx, "outer");
                let first = string(cx, "first");
                let last = string(cx, "last");
                let mut chain = Value::Object(target);
                let links = links();
                for link in 0..links {
                    let args = match link {
                        0 => vec![inner_this, first],
                        _ if link == links - 1 => vec![outer_this, last],
                        _ => vec![outer_this],
                    };
                    chain = cx.call(bind_function, chain, &args).unwrap().unwrap();
                    let Value::Object(made) = chain else {
                        panic!("{chain:?}");
                    };
                    cx.define_property_or_throw(
                        made,
                        name_key,
                        data(Value::Undefined, false, false, true),
                    )
                    .unwrap()
                    .unwrap();
                }
                let argument = string(cx, "argument");
                assert_eq!(
                    cx.call(chain, Value::Undefined, &[argument])
                        .unwrap()
                        .unwrap(),
                    Value::Undefined
                );
                assert_eq!(
                    cx.construct(chain, &[], None).unwrap().unwrap(),
                    Value::Object(target)
                );
                cx.host().calls.clone()
            })
            .unwrap();
        assert_eq!(
            calls,
            [
                "inner first last argument",
                "undefined first last new.target=C"
            ]
        );
    });
}

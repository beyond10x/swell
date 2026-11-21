//! Helpers that parse and evaluate scripts in the tests (`docs/design/host-interface.md`,
//! decision 8): [`run`] with a fresh agent and realm, [`run_both`] in sloppy and strict mode,
//! and what an outcome reads as text, so that a test compares it with an expectation.
//!
//! The helpers stop the test with a panic when a call the test needs does not answer as it
//! must; `cfg(test)` lets `clippy.toml` treat them as test code.
#![cfg(test)]

use swell::{
    Agent, Cx, HostCall, HostFn, Limits, Outcome as HostOutcome, Parsed, PropertyKey, Realm, Root,
    Script, Source, Termination, Value,
};

use super::host::{TestHost, agent_with, global, limits, object, realm};

/// What an entry answered, as text: a value of a normal completion as [`show`] writes it, a
/// thrown error by its `name` (another thrown value as [`show`] writes it), a termination, or
/// the messages of the errors of a parse.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Normal(String),
    Throw(String),
    Terminated(Termination),
    Errors(Vec<String>),
}

/// A value as text: `undefined`, `null`, `true`, a Number as ToString writes it, a String in
/// double quotes, `object` for an object (a script that needs to tell a function asks
/// `typeof`).
pub fn show(cx: &mut Cx<'_, TestHost>, value: Value) -> String {
    match value {
        Value::Undefined => "undefined".to_string(),
        Value::Null => "null".to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Number(_) => {
            let string = cx.to_string(value).unwrap().unwrap();
            cx.to_rust_string_lossy(string).unwrap()
        }
        Value::String(string) => format!("\"{}\"", cx.to_rust_string_lossy(string).unwrap()),
        Value::Object(_) => "object".to_string(),
        other => format!("{other:?}"),
    }
}

/// A thrown value as text: the `name` of an object that has a String `name`, otherwise as
/// [`show`] writes it.
pub fn show_thrown(cx: &mut Cx<'_, TestHost>, value: Value) -> String {
    if let Value::Object(object) = value {
        let key = PropertyKey::String(cx.new_string("name").unwrap());
        if let Ok(Ok(Value::String(name))) = cx.get_property(object, key) {
            return cx.to_rust_string_lossy(name).unwrap();
        }
        return "object".to_string();
    }
    show(cx, value)
}

/// The text of an entry's outcome; `realm` names the realm the values are read in.
pub fn describe(agent: &mut Agent<TestHost>, realm: &Realm, outcome: HostOutcome<Root>) -> Outcome {
    match outcome {
        HostOutcome::Normal(root) => agent
            .with_cx(realm, |cx| {
                let value = cx.get(&root).unwrap();
                Outcome::Normal(show(cx, value))
            })
            .unwrap(),
        HostOutcome::Throw(root) => agent
            .with_cx(realm, |cx| {
                let value = cx.get(&root).unwrap();
                Outcome::Throw(show_thrown(cx, value))
            })
            .unwrap(),
        HostOutcome::Terminated(termination) => Outcome::Terminated(termination),
    }
}

/// The messages of the errors of a parse.
pub fn messages(agent: &mut Agent<TestHost>, realm: &Realm, errors: &[Root]) -> Vec<String> {
    agent
        .with_cx(realm, |cx| {
            errors
                .iter()
                .map(|error| {
                    let error = object(cx, error);
                    let key = PropertyKey::String(cx.new_string("message").unwrap());
                    match cx.get_property(error, key).unwrap().unwrap() {
                        Value::String(message) => cx.to_rust_string_lossy(message).unwrap(),
                        other => format!("{other:?}"),
                    }
                })
                .collect()
        })
        .unwrap()
}

/// `parse_script` of `source` in `realm`, which must parse.
pub fn parse(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> Script {
    match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => script,
        Parsed::Errors(errors) => {
            let messages = messages(agent, realm, &errors);
            panic!("{source:?} did not parse: {messages:?}")
        }
        Parsed::Terminated(termination) => panic!("parsing {source:?} ended in {termination:?}"),
    }
}

/// The outcome of the script `source` in `realm`: parsed, then evaluated.
pub fn eval(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> HostOutcome<Root> {
    let script = parse(agent, realm, source);
    agent.evaluate_script(&script).unwrap()
}

/// The script `source` in `realm`, described: a parse error is [`Outcome::Errors`].
pub fn eval_text(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> Outcome {
    match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => {
            let outcome = agent.evaluate_script(&script).unwrap();
            describe(agent, realm, outcome)
        }
        Parsed::Errors(errors) => Outcome::Errors(messages(agent, realm, &errors)),
        Parsed::Terminated(termination) => Outcome::Terminated(termination),
    }
}

/// The outcome of `source` with `limits`, in a fresh agent and realm.
pub fn run(limits: Limits, source: &str) -> Outcome {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    eval_text(&mut agent, &realm, source)
}

/// `source` as written and with a `"use strict";` prologue, each with the test limits.
pub fn run_both(source: &str) -> (Outcome, Outcome) {
    (
        run(limits(), source),
        run(limits(), &format!("\"use strict\";\n{source}")),
    )
}

/// Define the host function `steps` as the global property `name` of `realm`.
pub fn define_function(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    name: &str,
    length: u32,
    steps: HostFn<TestHost>,
    constructor: bool,
) {
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let realm_ref = cx.realm_ref(realm).unwrap();
            let function = cx
                .new_host_function(realm_ref, name, length, steps, constructor, None)
                .unwrap()
                .unwrap();
            let global = object(cx, &global_root);
            let key = PropertyKey::String(cx.new_string(name).unwrap());
            cx.set(global, key, Value::Object(function), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
}

/// A host function that answers `undefined`, for tests that only need a function.
pub fn nothing(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> swell::Completion<Value> {
    Ok(Value::Undefined)
}

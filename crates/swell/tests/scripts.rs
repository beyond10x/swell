//! The script entry points (`docs/design/host-interface.md`, decision 8; ECMA-262 16.1,
//! `#sec-scripts`): `parse_script` performs ParseScript (`#sec-parse-script`) and answers its
//! errors as `SyntaxError` objects of the realm, `evaluate_script` performs ScriptEvaluation
//! (`#sec-runtime-semantics-scriptevaluation`). Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::host::{TestHost, agent, agent_with, global, is_error_of, limits, object, realm};
use support::script::{Outcome, describe, eval, eval_text, messages, parse};
use swell::{
    Brand, Completion, GlobalObjects, HookCx, HostError, HostObjectSpec, Outcome as HostOutcome,
    Parsed, Prototype, RealmRef, Root, Source, Value,
};

/// `0` inside `n` nested parentheses, ended by a `;` (automatic semicolon insertion is slice
/// 2's).
fn nested(n: usize) -> String {
    format!("{}0{};", "(".repeat(n), ")".repeat(n))
}

/// HOST-18: a source of 100,000 nested brackets with `parser_nesting` 1,000 makes
/// `parse_script` answer `Parsed::Errors` with one error object, and no stack overflow.
#[test]
fn host_18_100000_brackets_are_one_parse_error() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    for source in [
        format!("{}{}", "[".repeat(100_000), "]".repeat(100_000)),
        nested(100_000),
    ] {
        let Parsed::Errors(errors) = agent
            .parse_script(&realm, Source::Utf8(&source), None)
            .unwrap()
        else {
            panic!("100,000 brackets parsed");
        };
        assert_eq!(errors.len(), 1);
    }
}

/// HOST-19: with `parser_nesting` 0, `parse_script` of any non-empty source answers
/// `Parsed::Errors`.
#[test]
fn host_19_zero_parser_nesting_refuses_any_source() {
    let mut limits = limits();
    limits.parser_nesting = 0;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    for source in ["1;", ";", " ", "// c", "var a = 1;"] {
        assert!(
            matches!(
                agent.parse_script(&realm, Source::Utf8(source), None),
                Ok(Parsed::Errors(errors)) if !errors.is_empty()
            ),
            "{source:?}"
        );
    }
}

/// HOST-137 (`#sec-parse-script`; decision 2, `parser_nesting`): with `parser_nesting` 1,000,
/// `0` inside 2,000 nested parentheses answers `Parsed::Errors` with one object whose prototype
/// is the realm's `%SyntaxError.prototype%` and whose `message` names `parser_nesting`, and `0`
/// inside 10 nested parentheses (with its `;`) parses.
#[test]
fn host_137_nesting_error_names_parser_nesting_and_10_levels_parse() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let Parsed::Errors(errors) = agent
        .parse_script(&realm, Source::Utf8(&nested(2_000)), None)
        .unwrap()
    else {
        panic!("2,000 levels parsed");
    };
    assert_eq!(errors.len(), 1);
    let found = messages(&mut agent, &realm, &errors);
    assert!(found[0].contains("parser_nesting"), "{found:?}");
    assert!(
        agent
            .with_cx(&realm, |cx| {
                let global = object(cx, &global_root);
                let error = cx.get(&errors[0]).unwrap();
                is_error_of(cx, global, error, "SyntaxError")
            })
            .unwrap()
    );
    let script = parse(&mut agent, &realm, &nested(10));
    let outcome = agent.evaluate_script(&script).unwrap();
    assert_eq!(
        describe(&mut agent, &realm, outcome),
        Outcome::Normal("0".to_string())
    );
}

/// HOST-35 (`#sec-source-text`): the same text as `Source::Utf8` and as `Source::Utf16` gives
/// scripts that answer the same value, and a UTF-16 source with a lone surrogate inside a
/// string literal yields a String holding that code unit.
#[test]
fn host_35_utf8_and_utf16_sources_agree() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let text = "var s = 'h\u{e9}\u{1F600}'; s + s.length;";
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut answers = Vec::new();
    for source in [Source::Utf8(text), Source::Utf16(&units)] {
        let Parsed::Parsed(script) = agent.parse_script(&realm, source, None).unwrap() else {
            panic!("the source did not parse");
        };
        let outcome = agent.evaluate_script(&script).unwrap();
        answers.push(describe(&mut agent, &realm, outcome));
    }
    assert_eq!(
        answers[0],
        Outcome::Normal("\"h\u{e9}\u{1F600}4\"".to_string())
    );
    assert_eq!(answers[0], answers[1]);

    let lone = [u16::from(b'"'), 0xD800, u16::from(b'"'), u16::from(b';')];
    let Parsed::Parsed(script) = agent
        .parse_script(&realm, Source::Utf16(&lone), None)
        .unwrap()
    else {
        panic!("the lone surrogate did not parse");
    };
    let HostOutcome::Normal(root) = agent.evaluate_script(&script).unwrap() else {
        panic!("the lone surrogate did not evaluate");
    };
    let units = agent
        .with_cx(&realm, |cx| match cx.get(&root).unwrap() {
            Value::String(string) => cx.to_utf16(string).unwrap(),
            other => panic!("{other:?}"),
        })
        .unwrap();
    assert_eq!(units, vec![0xD800]);
}

/// HOST-36 (`#sec-parse-script`): `var 1;` answers `Parsed::Errors` with at least one object
/// whose prototype is the realm's `%SyntaxError.prototype%`, and no code runs: no hook is
/// called, no step is spent, and the global object gets no binding.
#[test]
fn host_36_parse_error_has_the_realm_syntax_error_prototype() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let other = realm_of(&mut agent);
    let global_root = global(&mut agent, &realm);
    let Parsed::Errors(errors) = agent
        .parse_script(&realm, Source::Utf8("x = 1; var 1;"), None)
        .unwrap()
    else {
        panic!("`var 1;` parsed");
    };
    assert!(!errors.is_empty());
    assert_eq!(agent.last_entry_steps(), Ok(0));
    assert!(agent.host().calls.is_empty());
    let (of_realm, of_other) = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let error = cx.get(&errors[0]).unwrap();
            (
                is_error_of(cx, global, error, "SyntaxError"),
                is_error_of(cx, global, error, "TypeError"),
            )
        })
        .unwrap();
    assert!(of_realm && !of_other);
    assert_eq!(
        eval_text(&mut agent, &realm, "typeof x;"),
        Outcome::Normal("\"undefined\"".to_string())
    );
    // The errors are objects of the realm given, not of another realm of the agent.
    let other_global = global(&mut agent, &other);
    let in_other = agent
        .with_cx(&other, |cx| {
            let global = object(cx, &other_global);
            let error = cx.get(&errors[0]).unwrap();
            is_error_of(cx, global, error, "SyntaxError")
        })
        .unwrap();
    assert!(!in_other);
}

/// A second realm of `agent`.
fn realm_of(agent: &mut swell::Agent<TestHost>) -> swell::Realm {
    realm(agent)
}

/// HOST-37 (`#sec-runtime-semantics-scriptevaluation`): the script `throw 1` answers `Throw`
/// whose root reads the Number 1, and `1 + 1` answers `Normal` whose root reads 2.
#[test]
fn host_37_throw_and_normal_completion_values() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let HostOutcome::Throw(thrown) = eval(&mut agent, &realm, "throw 1;") else {
        panic!("`throw 1` did not throw");
    };
    let HostOutcome::Normal(normal) = eval(&mut agent, &realm, "1 + 1;") else {
        panic!("`1 + 1` did not complete normally");
    };
    agent
        .with_cx(&realm, |cx| {
            assert_eq!(cx.get(&thrown), Ok(Value::Number(1.0)));
            assert_eq!(cx.get(&normal), Ok(Value::Number(2.0)));
        })
        .unwrap();
}

/// HOST-50 (`#sec-initializehostdefinedrealm`): when the setup closure returns a `this`
/// object, the script `this` answers that object.
#[test]
fn host_50_setup_this_is_script_this() {
    let mut agent = agent().unwrap();
    let mut kept: Option<Root> = None;
    let mut setup =
        |cx: &mut HookCx<'_, TestHost>, _realm: RealmRef| -> Completion<GlobalObjects> {
            let spec = |brand| HostObjectSpec {
                prototype: Prototype::Null,
                brand: Brand(brand),
                data: None,
            };
            let global = cx.new_host_object(spec(1)).unwrap()?;
            let this = cx.new_host_object(spec(2)).unwrap()?;
            kept = Some(cx.root(Value::Object(this)).unwrap());
            Ok(GlobalObjects {
                global,
                this: Some(this),
            })
        };
    let HostOutcome::Normal(realm) = agent.create_realm(Some(&mut setup), None).unwrap() else {
        panic!("create_realm did not answer Normal");
    };
    let kept = kept.unwrap();
    let HostOutcome::Normal(found) = eval(&mut agent, &realm, "this;") else {
        panic!("`this` did not complete normally");
    };
    agent
        .with_cx(&realm, |cx| {
            let this = cx.get(&kept).unwrap();
            assert_eq!(cx.get(&found).unwrap(), this);
            assert_eq!(
                cx.brand(object(cx, &found)),
                Ok(Some(Brand(2))),
                "the global `this`, not the global object"
            );
        })
        .unwrap();
}

/// HOST-54 (`#sec-globaldeclarationinstantiation`): one parsed script `var a = 1;` evaluated
/// twice answers `Normal` twice.
#[test]
fn host_54_var_script_evaluates_twice() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let script = parse(&mut agent, &realm, "var a = 1;");
    for _ in 0..2 {
        assert!(matches!(
            agent.evaluate_script(&script),
            Ok(HostOutcome::Normal(_))
        ));
    }
    assert_eq!(
        eval_text(&mut agent, &realm, "a;"),
        Outcome::Normal("1".to_string())
    );
}

/// `script_host_defined` answers the `Rc` given to `parse_script`; a `Script` of another
/// agent answers `Err(ForeignHandle)` (HOST-45) and a `Realm` of another agent given to
/// `parse_script` too.
#[test]
fn script_host_defined_and_foreign_handles() {
    struct Data;
    impl swell::HostData for Data {
        fn trace(&self, _tracer: &mut swell::Tracer<'_>) {}
    }
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let data: std::rc::Rc<dyn swell::HostData> = std::rc::Rc::new(Data);
    let Parsed::Parsed(script) = agent
        .parse_script(&realm, Source::Utf8("1;"), Some(std::rc::Rc::clone(&data)))
        .unwrap()
    else {
        panic!("`1;` did not parse");
    };
    let held = agent.script_host_defined(&script).unwrap().unwrap();
    assert!(std::rc::Rc::ptr_eq(&held, &data));
    let plain = parse(&mut agent, &realm, "2;");
    assert!(agent.script_host_defined(&plain).unwrap().is_none());

    let mut other = support::host::agent().unwrap();
    let other_realm = support::host::realm(&mut other);
    assert!(matches!(
        other.evaluate_script(&script),
        Err(HostError::ForeignHandle)
    ));
    assert!(matches!(
        other.script_host_defined(&script),
        Err(HostError::ForeignHandle)
    ));
    assert!(matches!(
        agent.parse_script(&other_realm, Source::Utf8("1;"), None),
        Err(HostError::ForeignHandle)
    ));
}

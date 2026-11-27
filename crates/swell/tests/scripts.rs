//! The script entry points (`docs/design/host-interface.md`, decision 8; ECMA-262 16.1,
//! `#sec-scripts`): `parse_script` performs ParseScript (`#sec-parse-script`) and answers its
//! errors as `SyntaxError` objects of the realm, `evaluate_script` performs ScriptEvaluation
//! (`#sec-runtime-semantics-scriptevaluation`). Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::rc::Rc;
use support::host::{TestHost, agent, agent_with, global, is_error_of, limits, object, realm};
use support::script::{Outcome, describe, eval, eval_text, messages, parse};

use swell::{
    Brand, Completion, ErrorSite, GlobalObjects, HookCx, HostData, HostDefined, HostError,
    HostObjectSpec, Outcome as HostOutcome, Parsed, Prototype, RealmRef, Root, Source, Termination,
    Tracer, Value,
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
                error_kind: None,
                overrides: None,
            };
            let global = cx.new_host_object(spec(1)).unwrap()?;
            let this = cx.new_host_object(spec(2)).unwrap()?;
            kept = Some(cx.root(Value::Object(this)).unwrap());
            Ok(GlobalObjects {
                global,
                this: Some(this),
            })
        };
    let HostOutcome::Normal(realm) =
        support::host::create_realm(&mut agent, Some(&mut setup), None).unwrap()
    else {
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
            let global_this = object(cx, &found);
            assert_eq!(
                cx.brand(global_this),
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

/// The [[HostDefined]] of the tests of the error site: a fresh `Rc` names one script.
struct Named;

impl HostData for Named {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

fn fresh() -> HostDefined {
    Rc::new(Named)
}

/// The `(<line>:<column>)` suffix of a parse error's message.
fn suffix(message: &str) -> (u32, u32) {
    let open = message.rfind('(').unwrap();
    let inner = message[open + 1..].strip_suffix(')').unwrap();
    let (line, column) = inner.split_once(':').unwrap();
    (line.parse().unwrap(), column.parse().unwrap())
}

/// Whether `site` names `host_defined`: the same `Rc`, or none for none.
fn names(site: &ErrorSite, host_defined: Option<&HostDefined>) -> bool {
    match (&site.host_defined, host_defined) {
        (Some(held), Some(given)) => Rc::ptr_eq(held, given),
        (None, None) => true,
        _ => false,
    }
}

/// What a read answered, to compare two reads: line, column, `dynamic` and the address of the
/// [[HostDefined]].
type SiteKey = Option<(u32, u32, bool, Option<*const ()>)>;

fn key_of(site: &Option<ErrorSite>) -> SiteKey {
    site.as_ref().map(|site| {
        (
            site.line,
            site.column,
            site.dynamic,
            site.host_defined
                .as_ref()
                .map(|held| Rc::as_ptr(held).cast::<()>()),
        )
    })
}

/// HOST-172 (decision 13): `parse_script` of `var a;\n  var 1;` answers `Parsed::Errors`, and
/// `last_entry_error_site()` then reads line 2, the column the error's message ends with,
/// `dynamic` false and the `Rc` given to the parse, from a UTF-8 and from a UTF-16 source.
#[test]
fn host_172_a_parse_error_names_its_line_and_column() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let text = "var a;\n  var 1;";
    let units: Vec<u16> = text.encode_utf16().collect();
    for source in [Source::Utf8(text), Source::Utf16(&units)] {
        let host_defined = fresh();
        let Parsed::Errors(errors) = agent
            .parse_script(&realm, source, Some(Rc::clone(&host_defined)))
            .unwrap()
        else {
            panic!("`var 1;` parsed");
        };
        // The read comes first: every outermost entry, `with_cx` included, clears the record.
        let site = agent.last_entry_error_site().unwrap();
        let message = messages(&mut agent, &realm, &errors).remove(0);
        let site = site.unwrap_or_else(|| panic!("no site for {message:?}"));
        assert_eq!(site.line, 2, "{message}");
        assert_eq!((site.line, site.column), suffix(&message), "{message}");
        assert!(!site.dynamic);
        assert!(names(&site, Some(&host_defined)));
    }
}

/// Parse `source` with a fresh [[HostDefined]], evaluate it, and answer the site of its `Throw`
/// with that `Rc`. Between the entry and the read every root of the script and of the thrown
/// value is dropped and `collect()` runs; the read answers the same before and after.
fn site_of_throw(
    agent: &mut swell::Agent<TestHost>,
    realm: &swell::Realm,
    source: &str,
) -> (ErrorSite, HostDefined) {
    let host_defined = fresh();
    let Parsed::Parsed(script) = agent
        .parse_script(realm, Source::Utf8(source), Some(Rc::clone(&host_defined)))
        .unwrap()
    else {
        panic!("{source:?} did not parse");
    };
    let HostOutcome::Throw(thrown) = agent.evaluate_script(&script).unwrap() else {
        panic!("{source:?} did not throw");
    };
    let before = agent.last_entry_error_site().unwrap();
    drop((script, thrown));
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    let site = agent.last_entry_error_site().unwrap();
    assert_eq!(key_of(&before), key_of(&site), "{source:?}");
    let site = site.unwrap_or_else(|| panic!("no site for {source:?}"));
    (site, host_defined)
}

/// HOST-173 (decision 13): an uncaught throw is sited where it arose: in a called function, in
/// a `map` callback (a throw of a value that is not an error, at its `throw` statement), after a
/// `catch` that throws again, through a `finally` block, in eval code (`dynamic`) and in
/// `Function` code (`dynamic`, no [[HostDefined]]); a function defined by one script and called
/// from another names the first script's `Rc`.
#[test]
fn host_173_an_uncaught_throw_names_where_it_arose() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let in_function = "function f() {\n  null.x;\n}\nf();";
    assert_eq!(
        eval_text(&mut agent, &realm, in_function),
        Outcome::Throw("TypeError".to_string())
    );
    let cases: [(&str, u32, u32, bool); 6] = [
        (in_function, 2, 3, false),
        ("[1].map(function (x) {\n  throw x;\n});", 2, 3, false),
        ("try { throw 1; } catch (e) {\n  throw e;\n}", 2, 3, false),
        ("try { null.x } finally {}", 1, 7, false),
        ("eval(\"1;\\n  throw 2;\");", 2, 3, true),
        ("throw 1;", 1, 1, false),
    ];
    for (source, line, column, dynamic) in cases {
        let (site, host_defined) = site_of_throw(&mut agent, &realm, source);
        assert_eq!(
            (site.line, site.column, site.dynamic),
            (line, column, dynamic),
            "{source:?}"
        );
        assert!(names(&site, Some(&host_defined)), "{source:?}");
    }
    let (site, _) = site_of_throw(&mut agent, &realm, "new Function(\"\\n  null.x;\")();");
    assert_eq!((site.line, site.column, site.dynamic), (4, 3, true));
    assert!(names(&site, None));

    let first = fresh();
    let Parsed::Parsed(definer) = agent
        .parse_script(
            &realm,
            Source::Utf8("function g() {\n  null.y;\n}"),
            Some(Rc::clone(&first)),
        )
        .unwrap()
    else {
        panic!("the definer did not parse");
    };
    assert!(matches!(
        agent.evaluate_script(&definer),
        Ok(HostOutcome::Normal(_))
    ));
    drop(definer);
    let (site, second) = site_of_throw(&mut agent, &realm, "1;\ng();");
    assert_eq!((site.line, site.column, site.dynamic), (2, 3, false));
    assert!(names(&site, Some(&first)));
    assert!(!names(&site, Some(&second)));
}

/// An entry of `throw 1;` whose site the read gives.
fn thrown_entry(agent: &mut swell::Agent<TestHost>, realm: &swell::Realm) {
    assert!(matches!(
        eval(agent, realm, "throw 1;"),
        HostOutcome::Throw(_)
    ));
    assert!(agent.last_entry_error_site().unwrap().is_some());
}

/// HOST-174 (decision 13): after a `Throw` entry, the read gives `None` after an entry that
/// answered `Normal`, `Terminated(StepBudget)` or `Parsed::Parsed`, and after a script that
/// caught its throw; it reads the same value twice and after `collect()`, and leaves
/// `last_entry_steps()`, `heap_bytes()` and the hook calls as they were.
#[test]
fn host_174_the_error_site_is_a_read() {
    // A small step budget: `while (true) {}` collects at each of its steps in the mode
    // `collect-every-safe-point` (100,000 steps took 9.7 s there).
    let mut limits = limits();
    limits.steps_per_entry = 2_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert!(agent.last_entry_error_site().unwrap().is_none());

    thrown_entry(&mut agent, &realm);
    assert!(matches!(
        eval(&mut agent, &realm, "1;"),
        HostOutcome::Normal(_)
    ));
    assert!(agent.last_entry_error_site().unwrap().is_none(), "Normal");

    thrown_entry(&mut agent, &realm);
    assert!(matches!(
        eval(&mut agent, &realm, "while (true) {}"),
        HostOutcome::Terminated(Termination::StepBudget)
    ));
    assert!(
        agent.last_entry_error_site().unwrap().is_none(),
        "StepBudget"
    );

    thrown_entry(&mut agent, &realm);
    let script = parse(&mut agent, &realm, "2;");
    assert!(agent.last_entry_error_site().unwrap().is_none(), "Parsed");
    drop(script);

    thrown_entry(&mut agent, &realm);
    assert!(matches!(
        eval(&mut agent, &realm, "try { throw 1; } catch (e) {} 1"),
        HostOutcome::Normal(_)
    ));
    assert!(agent.last_entry_error_site().unwrap().is_none(), "caught");

    thrown_entry(&mut agent, &realm);
    let steps = agent.last_entry_steps().unwrap();
    let heap = agent.heap_bytes().unwrap();
    let calls = agent.host().calls.len();
    let first = key_of(&agent.last_entry_error_site().unwrap());
    let second = key_of(&agent.last_entry_error_site().unwrap());
    assert!(first.is_some());
    assert_eq!(first, second);
    assert_eq!(agent.last_entry_steps().unwrap(), steps);
    assert_eq!(agent.heap_bytes().unwrap(), heap);
    assert_eq!(agent.host().calls.len(), calls);
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    assert_eq!(key_of(&agent.last_entry_error_site().unwrap()), first);
}

/// The next number of a linear congruential generator over `seed`, below `bound`.
fn next(seed: &mut u64, bound: u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*seed >> 33) % bound
}

/// A guard of HOST-172: 50 sources with one grammar error each, an identifier, a literal or a
/// keyword placed where the grammar refuses it, at a line and column a generator seeded here
/// picks; the site equals the `(<line>:<column>)` suffix of the first error's message, and the
/// position of the token placed.
#[test]
fn a_parse_error_site_matches_its_message() {
    // Each refused statement with the offset of the token the grammar refuses.
    let refused: [(&str, u64); 7] = [
        ("var 1;", 4),
        ("var if;", 4),
        ("var 'x';", 4),
        ("a b;", 2),
        ("1 2;", 2),
        ("a.b c;", 4),
        ("x = if;", 4),
    ];
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let mut seed = 0x5EED_0172_u64;
    for _ in 0..50 {
        let lines = next(&mut seed, 6);
        let indent = next(&mut seed, 9);
        let refused_len = u64::try_from(refused.len()).unwrap();
        let (statement, offset) = refused[usize::try_from(next(&mut seed, refused_len)).unwrap()];
        let mut source = String::new();
        for line in 0..lines {
            source.push_str(&format!("var v{line} = {line};\n"));
        }
        source.push_str(&" ".repeat(usize::try_from(indent).unwrap()));
        source.push_str(statement);
        let Parsed::Errors(errors) = agent
            .parse_script(&realm, Source::Utf8(&source), None)
            .unwrap()
        else {
            panic!("{source:?} parsed");
        };
        let site = agent.last_entry_error_site().unwrap();
        let message = messages(&mut agent, &realm, &errors).remove(0);
        let site = site.unwrap_or_else(|| panic!("no site for {source:?}: {message}"));
        assert_eq!(
            (site.line, site.column),
            suffix(&message),
            "{source:?}: {message}"
        );
        let placed = (
            u32::try_from(lines + 1).unwrap(),
            u32::try_from(indent + offset + 1).unwrap(),
        );
        assert_eq!((site.line, site.column), placed, "{source:?}: {message}");
        assert!(!site.dynamic && site.host_defined.is_none());
    }
}

//! HostEnsureCanAddPrivateElement (`#sec-hostensurecanaddprivateelement`; HOST-119, HOST-223):
//! PrivateFieldAdd (`#sec-privatefieldadd`) and PrivateMethodOrAccessorAdd
//! (`#sec-privatemethodoraccessoradd`) call `host_ensure_can_add_private_element` before the
//! element is added, only for a host object with an override table (`docs/design/engine.md`,
//! "Private names", item 2, step 1); a `Throw` from it is the add's throw and nothing is added;
//! a `Throw` value with a stale handle or a handle of another agent faults the agent (decision
//! 6). An ordinary object, a function, a `Proxy`, a host object without a table and a host
//! function call no hook. The objects reach a class as `this` through the return override of a
//! base constructor (`function Base() { return target; }`). Written by
//! `story:host-ensure-can-add-private-element`. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{create_realm, limits, seeds};
use swell::{
    Abrupt, Agent, AgentState, Brand, Completion, Cx, ErrorKind, HookCx, Host, HostCall, HostError,
    HostObjectSpec, InterruptHandle, Job, LoadRequest, ObjectRef, Outcome as HostOutcome,
    Overrides, Parsed, PropertyKey, Prototype, Realm, Root, Source, Value,
};

/// What [`HookHost`]'s `host_ensure_can_add_private_element` answers at the call it answers.
#[derive(Clone, Copy, Debug, Default)]
enum Answer {
    /// `Ok(())`, the default.
    #[default]
    Allow,
    /// `Ok(())` after it made a String and a host object, which it drops.
    AllocateThenAllow,
    /// `Err(Abrupt::Throw)` of this value.
    Throw(Value),
    /// `Err(Abrupt::Throw)` of a new host object it makes, whose prototype is the object the
    /// hook was given and which has an [[ErrorData]] slot of kind `TypeError`; nothing roots it.
    ThrowNew,
    /// `Ok(())` after it set the interrupt flag through [`HookHost::interrupt`].
    Interrupt,
}

/// A host that records each call of `host_ensure_can_add_private_element` and of the host
/// function `log` and of the one override of [`TABLE`], and answers the hook as a test sets.
#[derive(Default)]
struct HookHost {
    /// One line per call, in order: `hook` for the hook, the argument of `log`, and
    /// `is_extensible` for the override.
    calls: Vec<String>,
    /// The object each call of the hook was given, in order.
    objects: Vec<ObjectRef>,
    /// What the hook answers at call number `at`.
    answer: Answer,
    /// The call of the hook, counted from 1 over [`HookHost::objects`], that answers `answer`;
    /// the others answer `Ok(())`. `0` answers `answer` at every call.
    at: usize,
    /// The handle [`Answer::Interrupt`] sets.
    interrupt: Option<InterruptHandle>,
}

impl Host for HookHost {
    fn host_ensure_can_add_private_element(
        cx: &mut HookCx<'_, Self>,
        obj: ObjectRef,
    ) -> Completion<()> {
        let host = cx.host_mut();
        host.calls.push("hook".to_string());
        host.objects.push(obj);
        if host.at != 0 && host.objects.len() != host.at {
            return Ok(());
        }
        match host.answer {
            Answer::Allow => Ok(()),
            Answer::AllocateThenAllow => {
                cx.new_string("made in the hook")?;
                cx.new_host_object(spec(Prototype::Null, None)).unwrap()?;
                Ok(())
            }
            Answer::Throw(value) => Err(Abrupt::Throw(value)),
            Answer::ThrowNew => {
                let mut made = spec(Prototype::Object(obj), None);
                made.error_kind = Some(ErrorKind::TypeError);
                let made = cx.new_host_object(made).unwrap()?;
                Err(Abrupt::Throw(Value::Object(made)))
            }
            Answer::Interrupt => {
                if let Some(handle) = &host.interrupt {
                    handle.interrupt();
                }
                Ok(())
            }
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

/// The override table of the host objects of these tests: one override, `is_extensible`, which
/// records its call and answers `true`, so a test sees that no add reads [[IsExtensible]].
static TABLE: Overrides<HookHost> = Overrides {
    get_prototype_of: None,
    set_prototype_of: None,
    is_extensible: Some(is_extensible),
    prevent_extensions: None,
    get_own_property: None,
    define_own_property: None,
    has_property: None,
    get: None,
    set: None,
    delete: None,
    own_property_keys: None,
};

/// The override `is_extensible` of [`TABLE`].
fn is_extensible(cx: &mut Cx<'_, HookHost>, _object: ObjectRef) -> Completion<bool> {
    cx.host_mut().calls.push("is_extensible".to_string());
    Ok(true)
}

/// A host object of brand 223 with no data and no [[ErrorData]].
fn spec(
    prototype: Prototype,
    overrides: Option<&'static Overrides<HookHost>>,
) -> HostObjectSpec<HookHost> {
    HostObjectSpec {
        prototype,
        brand: Brand(223),
        data: None,
        error_kind: None,
        overrides,
    }
}

/// The host function `log(s)`: records `s` and answers it.
fn log(cx: &mut Cx<'_, HookHost>, call: HostCall<'_>) -> Completion<Value> {
    let value = call.args.first().copied().unwrap_or(Value::Undefined);
    let Value::String(string) = value else {
        panic!("log takes a String");
    };
    let text = cx.to_rust_string_lossy(string).unwrap();
    cx.host_mut().calls.push(text);
    Ok(value)
}

/// An agent with [`HookHost`] and the test limits, a realm, and the host function `log`.
fn hook_agent() -> (Agent<HookHost>, Realm) {
    let mut agent = Agent::new(HookHost::default(), limits(), seeds()).unwrap();
    let HostOutcome::Normal(realm) = create_realm(&mut agent, None, None).unwrap() else {
        panic!("create_realm did not answer Normal");
    };
    bind(&mut agent, &realm, "log", |cx, realm_ref| {
        let made = cx.new_host_function(realm_ref, "log", 1, log, false, None);
        Value::Object(made.unwrap().unwrap())
    });
    (agent, realm)
}

/// Bind the value `make` makes as the global property `name`, and answer it: it lives as long as
/// the binding.
fn bind(
    agent: &mut Agent<HookHost>,
    realm: &Realm,
    name: &str,
    make: impl FnOnce(&mut Cx<'_, HookHost>, swell::RealmRef) -> Value,
) -> Value {
    let global = agent.global_object(realm).unwrap();
    agent
        .with_cx(realm, |cx| {
            let realm_ref = cx.realm_ref(realm).unwrap();
            let value = make(cx, realm_ref);
            let Value::Object(global) = cx.get(&global).unwrap() else {
                panic!("the global is no object");
            };
            let key = PropertyKey::String(cx.new_string(name).unwrap());
            cx.set(global, key, value, true).unwrap().unwrap();
            value
        })
        .unwrap()
}

/// Bind a new host object of `%Object.prototype%` with the table `overrides` as the global
/// property `name`, and answer it.
fn bind_host(
    agent: &mut Agent<HookHost>,
    realm: &Realm,
    name: &str,
    overrides: Option<&'static Overrides<HookHost>>,
) -> ObjectRef {
    let made = bind(agent, realm, name, |cx, realm_ref| {
        let made = cx.new_host_object(spec(Prototype::ObjectPrototypeOf(realm_ref), overrides));
        Value::Object(made.unwrap().unwrap())
    });
    let Value::Object(made) = made else {
        panic!("a host object is an object");
    };
    made
}

/// The script `source` parsed in `realm`.
fn parse(agent: &mut Agent<HookHost>, realm: &Realm, source: &str) -> swell::Script {
    match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => script,
        other => panic!("{source} did not parse: {other:?}"),
    }
}

/// The outcome of the script `source` in `realm`, as text: `normal` or `throw` with the value as
/// ToString writes it, or the termination.
fn outcome(agent: &mut Agent<HookHost>, realm: &Realm, source: &str) -> String {
    let script = parse(agent, realm, source);
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

/// The Number the parsed `script` answers, read with no allocation in the heap.
fn number(agent: &mut Agent<HookHost>, realm: &Realm, script: &swell::Script) -> f64 {
    let HostOutcome::Normal(root) = agent.evaluate_script(script).unwrap() else {
        panic!("the script did not complete normally");
    };
    match agent.with_cx(realm, |cx| cx.get(&root)).unwrap() {
        Ok(Value::Number(number)) => number,
        other => panic!("the script answered {other:?}"),
    }
}

/// The calls the host recorded since the last read, which empties both lists: the lines and the
/// objects the hook was given.
fn take_calls(agent: &mut Agent<HookHost>) -> (Vec<String>, Vec<ObjectRef>) {
    let host = agent.host_mut();
    (
        std::mem::take(&mut host.calls),
        std::mem::take(&mut host.objects),
    )
}

/// The heap's reading after a collection.
fn collected_bytes(agent: &mut Agent<HookHost>) -> u64 {
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    agent.heap_bytes().unwrap()
}

/// The base constructor that returns the global `target`, and a class that adds `#x` to it.
const FIELD_CLASS: &str = "var target; function Base() { return target; }
class C extends Base {
  #x = 1;
  static has(o) { return #x in o; }
}
0;";

/// HOST-119 (`#sec-privatefieldadd`, step 1): the field `#x` of `class C extends Base`, whose
/// base constructor returns `h`, a host object with an override table, calls the hook once,
/// with `h`, before the field is added; when it throws, `new C` throws that value and `#x in h`
/// is `false`. A value made in the hook and held by nothing else (a new host object) is thrown
/// to the script's `catch`, also with a collection at every safe point. The refused add leaves
/// `h` as it was: `heap_bytes()` after a collection reads the same before and after the
/// refused construction of a second host object `h2`, made and rooted before the first
/// reading, which an answer of `Ok(())` raises by the box and one entry of the list (24 and 48
/// bytes): the hook runs before the list's room is charged, and no override of `h` runs.
#[test]
fn host_119_a_throw_from_the_hook_is_the_field_adds_throw() {
    let (mut agent, realm) = hook_agent();
    let h = bind_host(&mut agent, &realm, "h", Some(&TABLE));
    let thrown = bind(&mut agent, &realm, "thrown", |cx, _| {
        Value::String(cx.new_string("refused").unwrap())
    });
    assert_eq!(outcome(&mut agent, &realm, FIELD_CLASS), "normal 0");
    agent.host_mut().answer = Answer::Throw(thrown);
    assert_eq!(
        outcome(
            &mut agent,
            &realm,
            "target = h; try { new C(); 'added'; } catch (e) { (e === thrown) + ' ' + C.has(h); }"
        ),
        "normal true false"
    );
    assert_eq!(take_calls(&mut agent), (vec!["hook".to_string()], vec![h]));

    agent.host_mut().answer = Answer::ThrowNew;
    assert_eq!(
        outcome(
            &mut agent,
            &realm,
            "try { new C(); 'added'; } catch (e) { \
             [e !== h, Object.getPrototypeOf(e) === h, Error.isError(e), C.has(h)].join(); }"
        ),
        "normal true,true,true,false"
    );
    assert_eq!(take_calls(&mut agent), (vec!["hook".to_string()], vec![h]));

    // The reading: the second host object and the scripts exist before it; a first round
    // grows the arenas the construction uses.
    let h2 = bind_host(&mut agent, &realm, "h2", Some(&TABLE));
    let h3 = bind_host(&mut agent, &realm, "h3", Some(&TABLE));
    agent.host_mut().answer = Answer::Throw(Value::Number(7.0));
    let refused = parse(
        &mut agent,
        &realm,
        "target = h; try { new C(); 0; } catch (e) { e; }",
    );
    let refused_2 = parse(
        &mut agent,
        &realm,
        "target = h2; try { new C(); 0; } catch (e) { e; }",
    );
    let allowed_3 = parse(
        &mut agent,
        &realm,
        "target = h3; try { new C(); 0; } catch (e) { e; }",
    );
    assert_eq!(number(&mut agent, &realm, &refused), 7.0);
    let before = collected_bytes(&mut agent);
    assert_eq!(number(&mut agent, &realm, &refused_2), 7.0);
    let after = collected_bytes(&mut agent);
    assert_eq!(after, before, "a refused add left bytes charged");
    agent.host_mut().answer = Answer::Allow;
    assert_eq!(number(&mut agent, &realm, &allowed_3), 0.0);
    let added = collected_bytes(&mut agent);
    assert_eq!(added - after, 24 + 48, "the list of one field");
    assert_eq!(
        take_calls(&mut agent),
        (vec!["hook".to_string(); 3], vec![h, h2, h3])
    );
    assert_eq!(
        outcome(&mut agent, &realm, "[C.has(h2), C.has(h3)].join();"),
        "normal false,true"
    );
}

/// HOST-119: when the hook answers `Ok(())` the field is added: `new C` answers `h`, `#x in h` is
/// `true` and the field reads back. The hook makes a String and a host object before it answers,
/// while the field's value is a new object the initializer made: with a collection at every
/// safe point the value is still the field's afterwards (no `HookCx` method is a safe point).
#[test]
fn host_119_an_ok_answer_adds_the_field() {
    let (mut agent, realm) = hook_agent();
    let h = bind_host(&mut agent, &realm, "h", Some(&TABLE));
    agent.host_mut().answer = Answer::AllocateThenAllow;
    assert_eq!(
        outcome(
            &mut agent,
            &realm,
            "var target = h; function Base() { return target; }
             class C extends Base {
               #x = { tag: 7 };
               static has(o) { return #x in o; }
               static read(o) { return o.#x.tag; }
             }
             [new C() === h, C.has(h), C.read(h)].join();"
        ),
        "normal true,true,7"
    );
    assert_eq!(take_calls(&mut agent), (vec!["hook".to_string()], vec![h]));
}

/// HOST-223 (`#sec-privatemethodoraccessoradd`, step 2; `#sec-initializeinstanceelements`):
/// a private method and a private accessor call the hook before they are added, and a `Throw`
/// from it is the add's throw: the element is not added. A getter and a setter of one name are
/// one element and call the hook once. At one construction the hook runs once per element in
/// the order of the adds, the methods and accessors first, then the fields in source order,
/// each after its initializer; a throw at the n-th call keeps the elements added before it and
/// adds none after it.
#[test]
fn host_223_a_throw_from_the_hook_is_the_method_adds_throw() {
    let (mut agent, realm) = hook_agent();
    let h = bind_host(&mut agent, &realm, "h", Some(&TABLE));
    let thrown = bind(&mut agent, &realm, "thrown", |cx, _| {
        Value::String(cx.new_string("refused").unwrap())
    });
    assert_eq!(
        outcome(
            &mut agent,
            &realm,
            "var target = h; function Base() { return target; }
             class M extends Base { #m() {} static has(o) { return #m in o; } }
             class A extends Base {
               get #a() { return 1; }
               set #a(v) {}
               static has(o) { return #a in o; }
             }
             0;"
        ),
        "normal 0"
    );
    agent.host_mut().answer = Answer::Throw(thrown);
    for class in ["M", "A"] {
        assert_eq!(
            outcome(
                &mut agent,
                &realm,
                &format!(
                    "try {{ new {class}(); 'added'; }} catch (e) {{ (e === thrown) + ' ' + {class}.has(h); }}"
                )
            ),
            "normal true false",
            "{class}"
        );
        assert_eq!(
            take_calls(&mut agent),
            (vec!["hook".to_string()], vec![h]),
            "{class}"
        );
    }

    assert_eq!(
        outcome(
            &mut agent,
            &realm,
            "class E extends Base {
               #m() {}
               get #a() { return 1; }
               set #a(v) {}
               #x = log('x');
               #y = log('y');
               static probe(o) { return [#m in o, #a in o, #x in o, #y in o].join(); }
             }
             0;"
        ),
        "normal 0"
    );
    let all = ["hook", "hook", "x", "hook", "y", "hook"];
    agent.host_mut().answer = Answer::Allow;
    let target = bind_host(&mut agent, &realm, "target", Some(&TABLE));
    assert_eq!(
        outcome(&mut agent, &realm, "new E() === target && E.probe(target);"),
        "normal true,true,true,true"
    );
    assert_eq!(
        take_calls(&mut agent),
        (all.map(String::from).to_vec(), vec![target; 4])
    );
    for (at, probe, calls) in [
        (1_u8, "false,false,false,false", 1),
        (2, "true,false,false,false", 2),
        (3, "true,true,false,false", 4),
        (4, "true,true,true,false", 6),
    ] {
        let target = bind_host(&mut agent, &realm, "target", Some(&TABLE));
        let host = agent.host_mut();
        host.answer = Answer::Throw(Value::Number(f64::from(at)));
        host.at = usize::from(at);
        assert_eq!(
            outcome(
                &mut agent,
                &realm,
                "try { new E(); 'added'; } catch (e) { e + ' ' + E.probe(target); }"
            ),
            format!("normal {at} {probe}"),
            "a throw at call {at}"
        );
        let expected: Vec<String> = all[..calls].iter().map(|c| c.to_string()).collect();
        assert_eq!(
            take_calls(&mut agent),
            (expected, vec![target; usize::from(at)]),
            "a throw at call {at}"
        );
    }
}

/// A host object of `agent` that is stale: made in one scope, never rooted, and freed by a
/// collection.
fn stale_object(agent: &mut Agent<HookHost>, realm: &Realm) -> ObjectRef {
    let made = agent
        .with_cx(realm, |cx| {
            cx.new_host_object(spec(Prototype::Null, None))
                .unwrap()
                .unwrap()
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    made
}

/// Every entry point and read of a faulted agent but `state()` and `refusals()` answers
/// `Err(HostError::Faulted)`; `script` was parsed before the fault.
fn assert_faulted(agent: &mut Agent<HookHost>, realm: &Realm, script: &swell::Script) {
    assert_eq!(agent.state(), AgentState::Faulted);
    let _: u64 = agent.refusals();
    assert!(matches!(
        agent.with_cx(realm, |_| ()),
        Err(HostError::Faulted)
    ));
    assert!(matches!(
        agent.parse_script(realm, Source::Utf8("1;"), None),
        Err(HostError::Faulted)
    ));
    assert!(matches!(
        agent.evaluate_script(script),
        Err(HostError::Faulted)
    ));
    assert!(matches!(agent.collect(), Err(HostError::Faulted)));
    assert!(matches!(agent.heap_bytes(), Err(HostError::Faulted)));
    assert!(matches!(agent.last_entry_steps(), Err(HostError::Faulted)));
    assert!(matches!(
        agent.global_object(realm),
        Err(HostError::Faulted)
    ));
    assert!(matches!(
        agent.create_realm(None, None),
        Err(HostError::Faulted)
    ));
}

/// The run of `new C` (a field) and of `new M` (a method) on `h` when the hook throws `bad`:
/// `Terminated(EngineFault)`, the hook called once with `h`, no code after the add (the
/// `catch`, the `log` after it) run, and then the faulted agent's answers. Whether the element
/// is on `h` cannot be read afterwards: every entry point answers `Err(Faulted)`, so the test
/// reads what the run did before the fault, through the calls.
fn assert_bad_throw_faults(bad: impl Fn(&mut Agent<HookHost>, &Realm) -> Value) {
    for (class, element) in [("C", "#x = log('x');"), ("M", "#m() {}")] {
        let (mut agent, realm) = hook_agent();
        let h = bind_host(&mut agent, &realm, "h", Some(&TABLE));
        let source = format!(
            "var target = h; function Base() {{ return target; }}
             class {class} extends Base {{ {element} }}
             0;"
        );
        assert_eq!(outcome(&mut agent, &realm, &source), "normal 0");
        let script = parse(
            &mut agent,
            &realm,
            &format!(
                "try {{ new {class}(); log('added'); }} catch (e) {{ log('caught'); }} log('after');"
            ),
        );
        let value = bad(&mut agent, &realm);
        agent.host_mut().answer = Answer::Throw(value);
        assert!(matches!(
            agent.evaluate_script(&script),
            Ok(HostOutcome::Terminated(swell::Termination::EngineFault))
        ));
        let initializer: &[&str] = if class == "C" { &["x"] } else { &[] };
        let mut expected: Vec<String> = initializer.iter().map(|c| c.to_string()).collect();
        expected.push("hook".to_string());
        assert_eq!(take_calls(&mut agent), (expected, vec![h]), "{class}");
        assert_faulted(&mut agent, &realm, &script);
    }
}

/// HOST-223 (decision 6): a `Throw` from the hook whose value is a stale handle (an object made
/// in a scope, never rooted, freed by a collection) ends the run of the field's add and of the
/// method's add with `Terminated(EngineFault)`; no element is added and no code after the add
/// runs; `state()` then reads `Faulted`, every entry point and read but `state()` and
/// `refusals()` answers `Err(HostError::Faulted)`, and nothing panics.
#[test]
fn host_223_a_stale_handle_in_the_throw_faults_the_agent() {
    assert_bad_throw_faults(|agent, realm| Value::Object(stale_object(agent, realm)));
}

/// HOST-223 (decision 6): the same for a `Throw` value that is an object of another agent,
/// alive there.
#[test]
fn host_223_a_handle_of_another_agent_in_the_throw_faults_the_agent() {
    let (mut other, other_realm) = hook_agent();
    let foreign = bind_host(&mut other, &other_realm, "kept", None);
    assert_bad_throw_faults(|_, _| Value::Object(foreign));
    assert_eq!(other.state(), AgentState::Alive);
}

/// HOST-119: an object of any kind but a host object with an override table calls no hook (the
/// hook here would throw): an ordinary object a base constructor returns, an instance of a base
/// class with private fields, methods and accessors, and the class itself, which its static
/// private elements are added to (a function).
#[test]
fn an_ordinary_object_calls_no_hook() {
    let (mut agent, realm) = hook_agent();
    agent.host_mut().answer = Answer::Throw(Value::Number(1.0));
    assert_eq!(
        outcome(
            &mut agent,
            &realm,
            "var o = {}; function Base() { return o; }
             class C extends Base { #x = 1; #m() {} static has(v) { return #x in v && #m in v; } }
             class D {
               #x = 1;
               #m() {}
               get #a() { return 1; }
               set #a(v) {}
               static #s = 1;
               static #t() {}
               static has(v) { return #x in v && #m in v && #a in v; }
               static hasStatic(v) { return #s in v && #t in v; }
             }
             [new C() === o, C.has(o), D.has(new D()), D.hasStatic(D)].join();"
        ),
        "normal true,true,true,true"
    );
    assert_eq!(take_calls(&mut agent), (vec![], vec![]));
}

/// HOST-119: a `Proxy` a base constructor returns gets the field and the method on itself: no
/// hook and no trap runs (every trap of its handler logs), and `#x in p` is `true`.
#[test]
fn a_proxy_calls_no_hook() {
    let (mut agent, realm) = hook_agent();
    agent.host_mut().answer = Answer::Throw(Value::Number(1.0));
    assert_eq!(
        outcome(
            &mut agent,
            &realm,
            "var handler = {};
             ['getPrototypeOf', 'setPrototypeOf', 'isExtensible', 'preventExtensions',
              'getOwnPropertyDescriptor', 'defineProperty', 'has', 'get', 'set',
              'deleteProperty', 'ownKeys', 'apply', 'construct'].forEach(function (name) {
               handler[name] = function () { log('trap ' + name); return Reflect[name].apply(null, arguments); };
             });
             var p = new Proxy({}, handler);
             function Base() { return p; }
             class C extends Base { #x = 1; #m() {} static has(v) { return #x in v && #m in v; } }
             [new C() === p, C.has(p)].join();"
        ),
        "normal true,true"
    );
    assert_eq!(take_calls(&mut agent), (vec![], vec![]));
}

/// HOST-119: a host object without an override table, and a host function (no host function
/// has an override table: `new_host_function` takes none), get the field with no hook call.
#[test]
fn a_host_object_without_an_override_table_calls_no_hook() {
    let (mut agent, realm) = hook_agent();
    bind_host(&mut agent, &realm, "plain", None);
    agent.host_mut().answer = Answer::Throw(Value::Number(1.0));
    assert_eq!(
        outcome(
            &mut agent,
            &realm,
            "var target; function Base() { return target; }
             class C extends Base { #x = 1; #m() {} static has(v) { return #x in v && #m in v; } }
             target = plain; var a = new C() === plain;
             target = log; var b = new C() === log;
             [a, b, C.has(plain), C.has(log)].join();"
        ),
        "normal true,true,true,true"
    );
    assert_eq!(take_calls(&mut agent), (vec![], vec![]));
}

/// `#sec-privatefieldadd`, steps 1 to 3: the hook runs before the check for an element already
/// there. A second `new C` on the same host object, after a first that the hook allowed, calls
/// the hook once more and then throws the `TypeError` of the duplicate; `#x` stays.
#[test]
fn a_second_add_after_an_ok_answer_is_a_type_error_and_calls_the_hook_once_more() {
    let (mut agent, realm) = hook_agent();
    let h = bind_host(&mut agent, &realm, "h", Some(&TABLE));
    assert_eq!(outcome(&mut agent, &realm, FIELD_CLASS), "normal 0");
    assert_eq!(
        outcome(
            &mut agent,
            &realm,
            "target = h; var first = new C() === h;
             try { new C(); 'added twice'; } catch (e) { [first, e instanceof TypeError, C.has(h)].join(); }"
        ),
        "normal true,true,true"
    );
    assert_eq!(
        take_calls(&mut agent),
        (vec!["hook".to_string(); 2], vec![h, h])
    );
}

/// Decision 12: a hook that sets the interrupt flag and answers `Ok(())` ends the run in
/// `Terminated(Interrupted)` before the element is added: no code after the add runs, and the
/// next entry reads `#x in h` as `false`.
#[test]
fn an_interrupt_set_in_the_hook_ends_the_run() {
    let (mut agent, realm) = hook_agent();
    let h = bind_host(&mut agent, &realm, "h", Some(&TABLE));
    assert_eq!(outcome(&mut agent, &realm, FIELD_CLASS), "normal 0");
    let handle = agent.interrupt_handle();
    let host = agent.host_mut();
    host.interrupt = Some(handle);
    host.answer = Answer::Interrupt;
    assert_eq!(
        outcome(
            &mut agent,
            &realm,
            "target = h; try { new C(); log('added'); } catch (e) { log('caught'); } log('after');"
        ),
        "Interrupted"
    );
    assert_eq!(take_calls(&mut agent), (vec!["hook".to_string()], vec![h]));
    agent.host_mut().answer = Answer::Allow;
    assert_eq!(outcome(&mut agent, &realm, "C.has(h);"), "normal false");
    assert_eq!(agent.state(), AgentState::Alive);
}

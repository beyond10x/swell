//! The value property `globalThis` of the global object (`#sec-globalthis`): the
//! [[GlobalThisValue]] of the realm's global environment, { [[Writable]]: true, [[Enumerable]]:
//! false, [[Configurable]]: true }, defined by SetDefaultGlobalBindings
//! (`#sec-setdefaultglobalbindings`) with the four URI functions of 19.2.6
//! (`#sec-uri-handling-functions`) after the properties the global object had before them.
//! HOST-155 (`#sec-initializehostdefinedrealm`): when the setup closure returns a `this` object,
//! `globalThis` answers that object. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::host::{TestHost, agent, global, object, realm};
use support::script::{Outcome, eval, eval_text, run_both};
use swell::{
    Agent, Brand, Completion, GlobalObjects, HookCx, HostObjectSpec, Outcome as HostOutcome,
    Prototype, Realm, RealmRef, Root, Value,
};

/// The five properties this story defines, in the order they are defined.
const NEW_NAMES: [&str; 5] = [
    "globalThis",
    "decodeURI",
    "decodeURIComponent",
    "encodeURI",
    "encodeURIComponent",
];

/// Both modes of `source` answer `expected`, a normal completion as `show` writes it.
fn both_answer(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(expected.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// The root of `source`'s normal completion in `realm`: the value is read through it inside
/// the next `with_cx`, since a collection may run between two entries.
fn root_of(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> Root {
    let HostOutcome::Normal(found) = eval(agent, realm, source) else {
        panic!("{source} did not complete normally");
    };
    found
}

/// A realm whose setup closure makes a host object of brand 1 the global object, with the
/// realm's `%Object.prototype%` as its prototype, and, when `with_this`, a host object of brand
/// 2 the global `this`; with a root of that `this`.
fn host_made_realm(agent: &mut Agent<TestHost>, with_this: bool) -> (Realm, Option<Root>) {
    let mut kept: Option<Root> = None;
    let mut setup = |cx: &mut HookCx<'_, TestHost>, realm: RealmRef| -> Completion<GlobalObjects> {
        let spec = |brand| HostObjectSpec {
            prototype: Prototype::ObjectPrototypeOf(realm),
            brand: Brand(brand),
            data: None,
            error_kind: None,
        };
        let global = cx.new_host_object(spec(1)).unwrap()?;
        let this = if with_this {
            let this = cx.new_host_object(spec(2)).unwrap()?;
            kept = Some(cx.root(Value::Object(this)).unwrap());
            Some(this)
        } else {
            None
        };
        Ok(GlobalObjects { global, this })
    };
    let HostOutcome::Normal(realm) = agent.create_realm(Some(&mut setup), None).unwrap() else {
        panic!("create_realm did not answer Normal");
    };
    (realm, kept)
}

/// HOST-155 (`#sec-initializehostdefinedrealm`, `#sec-globalthis`): with a setup closure that
/// returns a global object and a `this` object, `globalThis` in a script of the realm is that
/// `this` object (`globalThis === this`, brand 2) and not the global object (brand 1), whose
/// own property it is.
#[test]
fn host_155_global_this_is_the_setup_this_object() {
    let mut agent = agent().unwrap();
    let (realm, kept) = host_made_realm(&mut agent, true);
    let kept = kept.unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "globalThis === this;"),
        Outcome::Normal("true".to_string())
    );
    let found = root_of(&mut agent, &realm, "globalThis;");
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let found = cx.get(&found).unwrap();
            let this = cx.get(&kept).unwrap();
            let global = object(cx, &global_root);
            assert_eq!(found, this);
            assert_ne!(found, Value::Object(global), "not the global object");
            let Value::Object(found) = found else {
                panic!("globalThis is {found:?}");
            };
            assert_eq!(cx.brand(found), Ok(Some(Brand(2))));
            let key = support::host::key(cx, "globalThis");
            assert_eq!(cx.get_property(global, key).unwrap(), Ok(this));
        })
        .unwrap();
}

/// `#sec-globalthis` without a setup closure: the global `this` value is the global object
/// (`#sec-setrealmglobalobject`), so `globalThis` is the global object, in sloppy and strict
/// code and inside a function.
#[test]
fn global_this_without_a_setup_is_the_global_object() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let found = root_of(&mut agent, &realm, "globalThis;");
    let global_root = global(&mut agent, &realm);
    let (found, global) = agent
        .with_cx(&realm, |cx| {
            (cx.get(&found).unwrap(), object(cx, &global_root))
        })
        .unwrap();
    assert_eq!(found, Value::Object(global));
    both_answer(
        "var probe = 1; [globalThis === this, globalThis.probe, \
          (function () { return globalThis; })() === this, typeof globalThis].join();",
        "\"true,1,true,object\"",
    );
}

/// `#sec-globalthis`: { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }; an
/// assignment replaces it, in strict code too, and `delete` removes it.
#[test]
fn global_this_is_writable_and_configurable() {
    both_answer(
        "var d = Object.getOwnPropertyDescriptor(this, 'globalThis'); \
         [d.value === this, d.writable, d.enumerable, d.configurable, \
          this.propertyIsEnumerable('globalThis')].join();",
        "\"true,true,false,true,false\"",
    );
    both_answer(
        "globalThis = 7; var replaced = globalThis; \
         var deleted = delete this.globalThis; \
         [replaced, deleted, typeof globalThis, 'globalThis' in this].join();",
        "\"7,true,undefined,false\"",
    );
}

/// SetDefaultGlobalBindings (`#sec-setdefaultglobalbindings`) on a global object the setup
/// closure made (no `this` object, so `globalThis` is that global): `globalThis` and the four
/// URI functions of 19.2.6 are own properties with the attributes of clause 18 and 19.1.1, each
/// function of length 1 with its name and no constructor, and they are the last five own
/// String keys, in the order `globalThis`, then 19.2.6.1 to 19.2.6.4, after every key the
/// global object had before them.
#[test]
fn the_five_new_properties_are_defined_on_a_host_made_global() {
    let mut agent = agent().unwrap();
    let (realm, _) = host_made_realm(&mut agent, false);
    assert_eq!(
        eval_text(&mut agent, &realm, "globalThis === this;"),
        Outcome::Normal("true".to_string())
    );
    for name in &NEW_NAMES[1..] {
        // In a function, so that the script adds no key to the global object.
        let source = format!(
            "(function (global) {{ \
               var d = Object.getOwnPropertyDescriptor(global, '{name}'); \
               return [typeof d.value, d.value === {name}, d.writable, d.enumerable, \
                 d.configurable, d.value.length, d.value.name, \
                 Object.getPrototypeOf(d.value) === Function.prototype].join(); \
             }})(this);"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Outcome::Normal(format!("\"function,true,true,false,true,1,{name},true\"")),
            "{name}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &format!("new {name}('a');")),
            Outcome::Throw("TypeError".to_string()),
            "{name} is no constructor"
        );
    }
    let names = root_of(
        &mut agent,
        &realm,
        "Object.getOwnPropertyNames(this).join();",
    );
    let names = agent
        .with_cx(&realm, |cx| {
            let names = cx.get(&names).unwrap();
            support::host::text(cx, names)
        })
        .unwrap();
    let names: Vec<&str> = names.split(',').collect();
    let first = names.len() - NEW_NAMES.len();
    assert_eq!(names[first..], NEW_NAMES, "{names:?}");
    for before in [
        "Infinity",
        "NaN",
        "undefined",
        "Error",
        "URIError",
        "eval",
        "Function",
    ] {
        let at = names.iter().position(|name| *name == before);
        assert!(at.is_some_and(|at| at < first), "{before} in {names:?}");
    }
}

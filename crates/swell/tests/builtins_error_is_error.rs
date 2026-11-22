//! `Error.isError ( arg )` (`#sec-error.iserror`): true exactly for an object with an
//! [[ErrorData]] internal slot, an error of another realm included; false for every other value.
//! A built-in function of length 1, not a constructor, defined on `Error` after the keys it had
//! before (clause 18, `#sec-properties-of-the-error-constructor`). Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, realm};
use support::script::{Outcome, eval, run_both};
use swell::{Agent, ErrorKind, Outcome as HostOutcome, Realm, Root, Value};

/// Both modes of `source` answer `expected`, a normal completion as `show` writes it.
fn both_answer(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(expected.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// The root of what `source` completes with in `realm`, normal or thrown.
fn completion_of(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> Root {
    match eval(agent, realm, source) {
        HostOutcome::Normal(root) | HostOutcome::Throw(root) => root,
        HostOutcome::Terminated(termination) => panic!("{source} ended in {termination:?}"),
    }
}

/// `Error.isError` of realm `realm` called from the host with `arg`.
fn is_error_in(agent: &mut Agent<TestHost>, realm: &Realm, arg: &Root) -> Value {
    let function = completion_of(agent, realm, "Error.isError;");
    let undefined = agent
        .with_cx(realm, |cx| cx.root(Value::Undefined).unwrap())
        .unwrap();
    let HostOutcome::Normal(answer) = agent.call(realm, &function, &undefined, &[arg]).unwrap()
    else {
        panic!("Error.isError did not answer normally");
    };
    agent.with_cx(realm, |cx| cx.get(&answer).unwrap()).unwrap()
}

/// `#sec-error.iserror`, step 3: an Error and each NativeError made by its constructor, with
/// and without `new`, and the errors the engine throws answer true; so do an error of another realm made by a script, one the engine threw there,
/// and one the host made there with `cx.new_error`, asked of either realm's `Error.isError`.
/// The function: length 1, name "isError", { [[Writable]]: true, [[Enumerable]]: false,
/// [[Configurable]]: true }, the last own key of `Error`, and not a constructor.
#[test]
fn is_error_of_errors_native_errors_and_an_error_of_another_realm() {
    both_answer(
        "var kinds = [Error, EvalError, RangeError, ReferenceError, SyntaxError, TypeError, \
           URIError]; var out = []; \
         for (var i = 0; i < kinds.length; i++) { \
           out.push(Error.isError(new kinds[i]('m'))); out.push(Error.isError(kinds[i]())); \
         } \
         var thrown; try { null.x; } catch (e) { thrown = e; } out.push(Error.isError(thrown)); \
         try { undefinedName; } catch (e) { thrown = e; } out.push(Error.isError(thrown)); \
         out.join();",
        "\"true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,true\"",
    );
    both_answer(
        "var d = Object.getOwnPropertyDescriptor(Error, 'isError'); \
         var names = Object.getOwnPropertyNames(Error); \
         [typeof d.value, d.writable, d.enumerable, d.configurable, Error.isError.length, \
          Error.isError.name, names[names.length - 1], names.length, \
          Object.getPrototypeOf(Error.isError) === Function.prototype, \
          'isError' in TypeError, TypeError.hasOwnProperty('isError')].join();",
        "\"function,true,false,true,1,isError,isError,4,true,true,false\"",
    );
    let (sloppy, strict) = run_both("new Error.isError(new Error());");
    assert_eq!(sloppy, Outcome::Throw("TypeError".to_string()));
    assert_eq!(strict, Outcome::Throw("TypeError".to_string()));

    let mut agent = agent().unwrap();
    let a = realm(&mut agent);
    let b = realm(&mut agent);
    let made = completion_of(&mut agent, &b, "new RangeError('of b');");
    let thrown = completion_of(&mut agent, &b, "null.x;");
    let hosted = agent
        .with_cx(&b, |cx| {
            let realm_ref = cx.realm_ref(&b).unwrap();
            let error = cx
                .new_error(realm_ref, ErrorKind::UriError, "of the host")
                .unwrap()
                .unwrap();
            cx.root(Value::Object(error)).unwrap()
        })
        .unwrap();
    for error in [&made, &thrown, &hosted] {
        assert_eq!(is_error_in(&mut agent, &a, error), Value::Boolean(true));
        assert_eq!(is_error_in(&mut agent, &b, error), Value::Boolean(true));
    }
    let of_a = completion_of(&mut agent, &a, "new Error('of a');");
    assert_eq!(is_error_in(&mut agent, &b, &of_a), Value::Boolean(true));
}

/// `#sec-error.iserror`, steps 1 and 2: no argument, every primitive, an ordinary object, an
/// object made by a constructor whose prototype is `Error.prototype`, an object with `name`,
/// `message` and `constructor` of an error, `Error.prototype` itself (an ordinary object,
/// `#sec-properties-of-the-error-prototype-object`), the constructors, a function and an
/// array answer false; so does an object of another realm that is no error.
#[test]
fn is_error_of_a_fake_error_and_of_primitives_is_false() {
    both_answer(
        "function F() {} F.prototype = Error.prototype; \
         var values = [undefined, null, true, false, 0, NaN, '', 'Error', {}, new F(), \
           { name: 'Error', message: 'm', constructor: Error, stack: '' }, Error.prototype, \
           TypeError.prototype, Error, TypeError, F, [], [new Error()], new String('e'), \
           new Number(1), new Boolean(false), Math, JSON]; \
         var out = [Error.isError()]; \
         for (var i = 0; i < values.length; i++) out.push(Error.isError(values[i])); \
         [new F() instanceof Error, out.length, out.join()].join('|');",
        "\"true|24|false,false,false,false,false,false,false,false,false,false,false,false,\
         false,false,false,false,false,false,false,false,false,false,false,false\"",
    );
    let mut agent = agent().unwrap();
    let a = realm(&mut agent);
    let b = realm(&mut agent);
    for source in [
        "({});",
        "Error.prototype;",
        "(function () { function F() {} F.prototype = Error.prototype; return new F(); })();",
    ] {
        let value = completion_of(&mut agent, &b, source);
        assert_eq!(
            is_error_in(&mut agent, &a, &value),
            Value::Boolean(false),
            "{source}"
        );
    }
}

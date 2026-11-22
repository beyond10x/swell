//! The integrity and prototype methods of `Object` and `Object.prototype`
//! (`story:object-integrity-and-prototype-methods`): `Object.create` (`#sec-object.create`),
//! `Object.defineProperties` (`#sec-object.defineproperties`) with ObjectDefineProperties
//! (`#sec-objectdefineproperties`), `Object.freeze` (`#sec-object.freeze`), `Object.is`
//! (`#sec-object.is`), `Object.isExtensible` (`#sec-object.isextensible`), `Object.isFrozen`
//! (`#sec-object.isfrozen`), `Object.isSealed` (`#sec-object.issealed`),
//! `Object.preventExtensions` (`#sec-object.preventextensions`), `Object.seal`
//! (`#sec-object.seal`), `Object.setPrototypeOf` (`#sec-object.setprototypeof`), and
//! `Object.prototype.isPrototypeOf` (`#sec-object.prototype.isprototypeof`), `.toLocaleString`
//! (`#sec-object.prototype.tolocalestring`) and `.valueOf` (`#sec-object.prototype.valueof`).
//!
//! Each builtin has a test of its property (clause 18: writable, not enumerable, configurable;
//! its "length" and "name"; not a constructor) and of its steps, in sloppy and in strict code;
//! then the step budget of the loops, and the re-entry limit through a getter of
//! ObjectDefineProperties and the `toString` of `toLocaleString` on the stack
//! `Limits::min_stack_bytes` promises. Own keys are read by `Object.getOwnPropertyNames`, and
//! attributes by `Object.getOwnPropertyDescriptor`. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::host::{
    agent, agent_with, get, get_object, global, key, limits, object, realm, stack_with_room,
};
use support::script::{Outcome, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits};
use swell::{Termination, Value};

/// `source` answers the String `expected` in sloppy and in strict code.
fn answers(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(format!("\"{expected}\""));
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// The method `name` of `holder` is a property as clause 18 makes it: a function, writable,
/// not enumerable and configurable; its "length" is `length` and its "name" is `name`, each
/// read-only, not enumerable and configurable; `new` of it throws a `TypeError`, it has no
/// "prototype" and its prototype is `%Function.prototype%`.
fn is_a_method(holder: &str, name: &str, length: u32) {
    let source = format!(
        "var d = Object.getOwnPropertyDescriptor({holder}, \"{name}\");\n\
         var f = d.value;\n\
         var l = Object.getOwnPropertyDescriptor(f, \"length\");\n\
         var n = Object.getOwnPropertyDescriptor(f, \"name\");\n\
         var refused = false;\n\
         try {{ new f(); }} catch (e) {{ refused = e instanceof TypeError; }}\n\
         [typeof f, d.writable, d.enumerable, d.configurable,\n\
          l.value, l.writable, l.enumerable, l.configurable,\n\
          n.value, n.writable, n.enumerable, n.configurable,\n\
          refused, f.hasOwnProperty(\"prototype\"),\n\
          Object.getPrototypeOf(f) === Function.prototype].join();"
    );
    answers(
        &source,
        &format!(
            "function,true,false,true,{length},false,false,true,{name},false,false,true,true,false,true"
        ),
    );
}

/// `#sec-object.preventextensions`: an object is made not extensible and answered, after which
/// a new property is refused and an old one may still be deleted; a primitive is answered as
/// it is.
#[test]
fn prevent_extensions_property_and_steps() {
    is_a_method("Object", "preventExtensions", 1);
    answers(
        "var o = { a: 1 };\n\
         var r = [Object.preventExtensions(o) === o, Object.isExtensible(o), o.a];\n\
         try { Object.defineProperty(o, \"b\", { value: 1 }); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.push(o.hasOwnProperty(\"b\"), delete o.a, o.hasOwnProperty(\"a\"));\n\
         var a = Object.preventExtensions([1]);\n\
         try { Object.defineProperty(a, \"1\", { value: 2 }); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.push(a.length, Object.preventExtensions(o) === o);\n\
         r.push(Object.preventExtensions(1), Object.preventExtensions(\"s\"),\n\
                Object.preventExtensions(undefined) === undefined, Object.preventExtensions(null) === null);\n\
         r.join();",
        "true,false,1,true,false,true,false,true,1,true,1,s,true,true",
    );
}

/// `#sec-object.isextensible`: true for a new object, false after `preventExtensions`, `seal`
/// or `freeze`, and false for a primitive, which throws nothing.
#[test]
fn is_extensible_property_and_steps() {
    is_a_method("Object", "isExtensible", 1);
    answers(
        "[Object.isExtensible({}), Object.isExtensible([]), Object.isExtensible(new String(\"a\")),\n\
          Object.isExtensible(Object.preventExtensions({})), Object.isExtensible(Object.seal({})),\n\
          Object.isExtensible(Object.freeze({})), Object.isExtensible(1),\n\
          Object.isExtensible(undefined), Object.isExtensible(null), Object.isExtensible()].join();",
        "true,true,true,false,false,false,false,false,false,false",
    );
}

/// `#sec-object.seal`: the object is answered, not extensible, every own property not
/// configurable; a primitive is answered as it is.
#[test]
fn seal_property_and_steps() {
    is_a_method("Object", "seal", 1);
    answers(
        "var o = { a: 1 };\n\
         var d = Object.getOwnPropertyDescriptor(Object.seal(o), \"a\");\n\
         [Object.seal(o) === o, Object.isExtensible(o), d.configurable, d.enumerable,\n\
          Object.seal(2), Object.seal(\"t\"), Object.seal(undefined) === undefined].join();",
        "true,false,false,true,2,t,true",
    );
}

/// `#sec-object.seal` with SetIntegrityLevel sealed (`#sec-setintegritylevel`, step 4): a sealed
/// object keeps its values writable and its accessors, so an assignment still changes a value,
/// in strict code too; an Array keeps its elements and its "length" writable.
#[test]
fn seal_keeps_values_writable() {
    answers(
        "var o = Object.seal({ a: 1, get g() { return 2; } });\n\
         o.a = 3;\n\
         var d = Object.getOwnPropertyDescriptor(o, \"a\");\n\
         var g = Object.getOwnPropertyDescriptor(o, \"g\");\n\
         var a = Object.seal([1, 2]);\n\
         a[0] = 5;\n\
         var e = Object.getOwnPropertyDescriptor(a, \"0\");\n\
         var l = Object.getOwnPropertyDescriptor(a, \"length\");\n\
         [o.a, d.writable, d.configurable, typeof g.get, g.configurable, o.g,\n\
          a[0], e.writable, e.configurable, l.writable, l.configurable,\n\
          Object.isSealed(o), Object.isFrozen(o), Object.isSealed(a), Object.isFrozen(a)].join();",
        "3,true,false,function,false,2,5,true,false,true,false,true,false,true,false",
    );
}

/// `#sec-object.issealed`: TestIntegrityLevel sealed (`#sec-testintegritylevel`).
#[test]
fn is_sealed_property_and_steps() {
    is_a_method("Object", "isSealed", 1);
    answers(
        "var o = Object.preventExtensions({ a: 1 });\n\
         var r = [Object.isSealed({}), Object.isSealed(o)];\n\
         Object.defineProperty(o, \"a\", { configurable: false });\n\
         r.push(Object.isSealed(o), Object.isSealed(Object.seal({ b: 1 })), Object.isSealed(Object.freeze({ c: 1 })));\n\
         r.join();",
        "false,false,true,true,true",
    );
}

/// `#sec-object.freeze`: the object is answered, not extensible, every data property read-only
/// and not configurable, every accessor not configurable; a primitive is answered as it is.
#[test]
fn freeze_property_and_steps() {
    is_a_method("Object", "freeze", 1);
    answers(
        "var o = { a: 1, get g() { return 2; } };\n\
         var r = [Object.freeze(o) === o, Object.isExtensible(o)];\n\
         var d = Object.getOwnPropertyDescriptor(o, \"a\");\n\
         var g = Object.getOwnPropertyDescriptor(o, \"g\");\n\
         r.push(d.writable, d.configurable, d.value, typeof g.get, g.configurable, o.g);\n\
         try { Object.defineProperty(o, \"a\", { value: 2 }); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.push(o.a, Object.freeze(3), Object.freeze(\"u\"), Object.freeze(null) === null);\n\
         r.join();",
        "true,false,false,false,1,function,false,2,true,1,3,u,true",
    );
}

/// `#sec-object.freeze` of an Array and of the arguments objects (`#sec-arguments-exotic-objects`):
/// each element and "length" read-only and not configurable, `push` refused with a
/// `TypeError`; a mapped index is no longer mapped once it is read-only, so an assignment to
/// the parameter does not show through; a String object is frozen with its indices.
#[test]
fn freeze_of_an_array_and_of_arguments() {
    answers(
        "var a = Object.freeze([1, 2]);\n\
         var e = Object.getOwnPropertyDescriptor(a, \"1\");\n\
         var l = Object.getOwnPropertyDescriptor(a, \"length\");\n\
         var r = [Object.isFrozen(a), e.value, e.writable, e.configurable, l.value, l.writable, l.configurable];\n\
         try { a.push(3); } catch (x) { r.push(x instanceof TypeError); }\n\
         r.push(a.length, a.join(\"-\"));\n\
         function f(x, y) {\n\
           var args = Object.freeze(arguments);\n\
           x = 9;\n\
           var d = Object.getOwnPropertyDescriptor(args, \"0\");\n\
           return [Object.isFrozen(args), args[0], args.length, d.writable, d.configurable].join(\"/\");\n\
         }\n\
         r.push(f(1, 2));\n\
         var s = Object.freeze(new String(\"ab\"));\n\
         r.push(Object.isFrozen(s), Object.getOwnPropertyNames(s).join(\"/\"));\n\
         r.join();",
        "true,2,false,false,2,false,false,true,2,1-2,true/1/2/false/false,true,0/1/length",
    );
}

/// `#sec-object.isfrozen`: TestIntegrityLevel frozen (`#sec-testintegritylevel`).
#[test]
fn is_frozen_property_and_steps() {
    is_a_method("Object", "isFrozen", 1);
    answers(
        "var o = Object.preventExtensions({ a: 1, get g() { return 1; } });\n\
         var r = [Object.isFrozen({}), Object.isFrozen(o)];\n\
         Object.defineProperty(o, \"a\", { configurable: false });\n\
         Object.defineProperty(o, \"g\", { configurable: false });\n\
         r.push(Object.isFrozen(o));\n\
         Object.defineProperty(o, \"a\", { writable: false });\n\
         r.push(Object.isFrozen(o), Object.isFrozen(Object.seal({ b: 1 })), Object.isFrozen(Object.freeze([1])));\n\
         r.join();",
        "false,false,false,true,false,true",
    );
}

/// `#sec-object.issealed` and `#sec-object.isfrozen` (step 1: a primitive is sealed and
/// frozen, and nothing throws for undefined or null), and TestIntegrityLevel of an object with
/// no own property (step 3: what is not extensible is sealed and frozen); an empty Array is not
/// frozen, its "length" being writable, and a String object with its indices is frozen.
#[test]
fn integrity_tests_of_primitives_and_of_empty_objects() {
    answers(
        "[Object.isSealed(1), Object.isFrozen(\"s\"), Object.isSealed(undefined), Object.isFrozen(null),\n\
          Object.isFrozen(), Object.isSealed(true),\n\
          Object.isSealed({}), Object.isFrozen({}),\n\
          Object.isSealed(Object.preventExtensions({})), Object.isFrozen(Object.preventExtensions({})),\n\
          Object.isSealed(Object.preventExtensions([])), Object.isFrozen(Object.preventExtensions([])),\n\
          Object.isFrozen(Object.preventExtensions(new String(\"ab\"))),\n\
          Object.isSealed(Object.preventExtensions({ a: 1 }))].join();",
        "true,true,true,true,true,true,false,false,true,true,true,false,true,false",
    );
}

/// `#sec-object.setprototypeof`: the prototype is set and the object answered; null is a
/// prototype; a primitive is answered as it is once its prototype argument passes; undefined
/// and null as the object, and a prototype that is neither an object nor null, throw a
/// `TypeError`.
#[test]
fn set_prototype_of_property_and_steps() {
    is_a_method("Object", "setPrototypeOf", 2);
    answers(
        "var a = {}, b = {};\n\
         var r = [Object.setPrototypeOf(a, b) === a, Object.getPrototypeOf(a) === b];\n\
         r.push(Object.getPrototypeOf(Object.setPrototypeOf(a, null)) === null);\n\
         r.push(Object.setPrototypeOf(1, null), Object.setPrototypeOf(\"s\", {}));\n\
         try { Object.setPrototypeOf(undefined, null); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { Object.setPrototypeOf(null, {}); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { Object.setPrototypeOf({}, 1); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { Object.setPrototypeOf(1); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.join();",
        "true,true,true,1,s,true,true,true,true",
    );
}

/// `#sec-object.setprototypeof`, step 5: a false [[SetPrototypeOf]] throws a `TypeError`: a
/// cycle (`#sec-ordinarysetprototypeof`, step 8), an object that is not extensible, and
/// `%Object.prototype%`, an immutable prototype exotic object
/// (`#sec-immutable-prototype-exotic-objects-setprototypeof-v`), which accepts only the
/// prototype it has.
#[test]
fn set_prototype_of_refuses_a_cycle_and_the_immutable_prototype() {
    answers(
        "var a = {}, b = Object.create(a), c = Object.create(b);\n\
         var r = [];\n\
         try { Object.setPrototypeOf(a, c); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { Object.setPrototypeOf(a, a); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.push(Object.getPrototypeOf(a) === Object.prototype);\n\
         try { Object.setPrototypeOf(Object.prototype, {}); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.push(Object.setPrototypeOf(Object.prototype, null) === Object.prototype);\n\
         var n = Object.preventExtensions({});\n\
         try { Object.setPrototypeOf(n, a); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.push(Object.setPrototypeOf(n, Object.prototype) === n);\n\
         r.join();",
        "true,true,true,true,true,true,true",
    );
}

/// `#sec-object.is`: SameValue (`#sec-samevalue`): NaN is itself, +0 and -0 differ, Strings
/// compare by their code units, objects by identity; missing arguments are undefined.
#[test]
fn is_property_and_steps() {
    is_a_method("Object", "is", 2);
    answers(
        "[Object.is(NaN, NaN), Object.is(0, -0), Object.is(-0, -0), Object.is(\"ab\", \"a\" + \"b\"),\n\
          Object.is({}, {}), Object.is(), Object.is(undefined, null), Object.is(1, \"1\"),\n\
          Object.is(Object, Object), Object.is(true, true)].join();",
        "true,false,true,true,false,true,false,false,true,true",
    );
}

/// `#sec-object.create`: an ordinary object with the given prototype; its properties defined
/// by ObjectDefineProperties when the argument is not undefined; a prototype that is neither
/// an object nor null throws a `TypeError`, as properties of null do (ToObject).
#[test]
fn create_property_and_steps() {
    is_a_method("Object", "create", 2);
    answers(
        "var p = { q: 1 };\n\
         var o = Object.create(p);\n\
         var r = [Object.getPrototypeOf(o) === p, o.q, Object.getOwnPropertyNames(o).length,\n\
                  Object.isExtensible(o), Object.prototype.toString.call(o),\n\
                  Object.getOwnPropertyNames(Object.create(p, undefined)).length];\n\
         try { Object.create(1); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { Object.create(); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { Object.create({}, null); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.join();",
        "true,1,0,true,[object Object],0,true,true,true",
    );
}

/// `#sec-object.create` with a null prototype (`#sec-ordinaryobjectcreate`) and properties:
/// the object inherits nothing, and each enumerable own property of the second argument is a
/// descriptor, a missing attribute false.
#[test]
fn create_with_a_null_prototype_and_properties() {
    answers(
        "var o = Object.create(null, { a: { value: 1, enumerable: true }, b: { get: function () { return 2; } } });\n\
         var d = Object.getOwnPropertyDescriptor(o, \"a\");\n\
         var g = Object.getOwnPropertyDescriptor(o, \"b\");\n\
         [Object.getPrototypeOf(o) === null, \"toString\" in o, o.a, d.writable, d.enumerable, d.configurable,\n\
          o.b, g.enumerable, g.configurable, g.set === undefined,\n\
          Object.getOwnPropertyNames(o).join(\"/\"),\n\
          Object.getPrototypeOf(Object.create(null)) === null].join();",
        "true,false,1,false,true,false,2,false,false,true,a/b,true",
    );
}

/// `#sec-object.defineproperties`: an object that is not an Object throws a `TypeError`, the
/// target is answered; ObjectDefineProperties (`#sec-objectdefineproperties`) takes ToObject of
/// the properties (a String's indices are its enumerable keys, undefined throws), reads only
/// the enumerable own keys, and defines an Array index as [[DefineOwnProperty]] of an Array
/// does.
#[test]
fn define_properties_property_and_steps() {
    is_a_method("Object", "defineProperties", 2);
    answers(
        "var t = {};\n\
         var props = { a: { value: 1, writable: true } };\n\
         Object.defineProperty(props, \"hidden\", { value: { value: 2 }, enumerable: false });\n\
         var r = [Object.defineProperties(t, props) === t, t.a, t.hasOwnProperty(\"hidden\"),\n\
                  Object.getOwnPropertyDescriptor(t, \"a\").writable];\n\
         try { Object.defineProperties(1, {}); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { Object.defineProperties({}); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { Object.defineProperties({}, \"ab\"); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.push(Object.getOwnPropertyNames(Object.defineProperties({}, \"\")).length);\n\
         r.push(Object.getOwnPropertyNames(Object.defineProperties({}, 5)).length);\n\
         var a = Object.defineProperties([], { 2: { value: 7, enumerable: true, writable: true, configurable: true } });\n\
         r.push(a.length, a[2]);\n\
         r.join();",
        "true,1,false,true,true,true,true,0,0,3,7",
    );
}

/// ObjectDefineProperties (`#sec-objectdefineproperties`), steps 4 and 5: every descriptor is
/// read (Get, then ToPropertyDescriptor) before any is defined, in the order of the keys, so
/// a getter sees no property defined yet, and one descriptor that is not valid leaves the
/// target as it was.
#[test]
fn define_properties_reads_every_descriptor_before_it_defines_one() {
    answers(
        "var t = {};\n\
         var log = [];\n\
         var seen = function () { return \"\" + t.hasOwnProperty(\"a\") + t.hasOwnProperty(\"b\"); };\n\
         var props = {\n\
           get a() { log.push(\"a\" + seen()); return { get value() { log.push(\"va\" + seen()); return 1; } }; },\n\
           get b() { log.push(\"b\" + seen()); return { value: 2 }; }\n\
         };\n\
         Object.defineProperties(t, props);\n\
         log.push(seen(), t.a, t.b);\n\
         var u = {};\n\
         try { Object.defineProperties(u, { a: { value: 1 }, b: { get: 1 } }); } catch (e) { log.push(e instanceof TypeError); }\n\
         try { Object.defineProperties(u, { a: { value: 1 }, b: 1 }); } catch (e) { log.push(e instanceof TypeError); }\n\
         log.push(Object.getOwnPropertyNames(u).length);\n\
         log.join();",
        "afalsefalse,vafalsefalse,bfalsefalse,truetrue,1,2,true,true,0",
    );
}

/// `#sec-object.prototype.isprototypeof`: a value that is not an object is false before the
/// this value is read (so an undefined this does not throw then); otherwise ToObject of the
/// this value, then the walk up the value's prototypes, the value itself not included.
#[test]
fn is_prototype_of_property_and_steps() {
    is_a_method("Object.prototype", "isPrototypeOf", 1);
    answers(
        "var p = {}, o = Object.create(p), q = Object.create(o);\n\
         var r = [p.isPrototypeOf(o), p.isPrototypeOf(q), o.isPrototypeOf(p), o.isPrototypeOf(o),\n\
                  Object.prototype.isPrototypeOf(q), Object.prototype.isPrototypeOf(Object.create(null)),\n\
                  Array.prototype.isPrototypeOf([]), Function.prototype.isPrototypeOf(Object),\n\
                  Object.prototype.isPrototypeOf.call(undefined, 1), p.isPrototypeOf(),\n\
                  Object.prototype.isPrototypeOf.call(1, {}), String.prototype.isPrototypeOf(\"s\")];\n\
         try { Object.prototype.isPrototypeOf.call(undefined, {}); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { Object.prototype.isPrototypeOf.call(null, o); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.join();",
        "true,true,false,false,true,false,true,true,false,false,false,false,true,true",
    );
}

/// `#sec-object.prototype.tolocalestring`: Invoke (`#sec-invoke`) of "toString" on the this
/// value, a primitive as it is (a strict `toString` sees it unwrapped); a `toString` that is
/// not callable, and an undefined this, throw a `TypeError`.
#[test]
fn to_locale_string_property_and_steps() {
    is_a_method("Object.prototype", "toLocaleString", 0);
    answers(
        "var o = { toString: function () { return \"t\" + (this === o) + arguments.length; } };\n\
         var seen;\n\
         Boolean.prototype.toString = function () { \"use strict\"; seen = typeof this; return \"b\"; };\n\
         var r = [o.toLocaleString(1, 2), ({}).toLocaleString(), Object.prototype.toLocaleString.call(true) + seen];\n\
         try { ({ toString: 1 }).toLocaleString(); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { Object.prototype.toLocaleString.call(undefined); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.join();",
        "ttrue0,[object Object],bboolean,true,true",
    );
}

/// `#sec-object.prototype.valueof`: ToObject of the this value: an object as it is, a
/// primitive wrapped, undefined and null a `TypeError`.
#[test]
fn value_of_property_and_steps() {
    is_a_method("Object.prototype", "valueOf", 0);
    answers(
        "var o = {};\n\
         var v = Object.prototype.valueOf;\n\
         var r = [o.valueOf() === o, typeof v.call(1), v.call(\"s\") instanceof String, v.call(true) instanceof Boolean,\n\
                  [].valueOf === v];\n\
         try { v.call(undefined); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { v.call(null); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.join();",
        "true,object,true,true,true,true,true",
    );
}

/// OrdinaryToPrimitive (`#sec-ordinarytoprimitive`) of an ordinary object: with the hint
/// number and with the default hint "valueOf" is called first, and `%Object.prototype.valueOf%`
/// answers the object, which is not a primitive, so "toString" is called next.
#[test]
fn to_primitive_of_an_ordinary_object_calls_value_of_first() {
    answers(
        "var log = \"\";\n\
         var v = Object.prototype.valueOf, s = Object.prototype.toString;\n\
         Object.prototype.valueOf = function () { log += \"v\"; return v.call(this); };\n\
         Object.prototype.toString = function () { log += \"s\"; return s.call(this); };\n\
         var text = {} + \"\";\n\
         var number = +{};\n\
         text + \"|\" + number + \"|\" + log;",
        "[object Object]|NaN|vsvs",
    );
}

/// The steps an entry of `source` spends in `agent`, which must answer `expected`.
fn steps_of(
    agent: &mut swell::Agent<support::host::TestHost>,
    realm: &swell::Realm,
    source: &str,
    expected: &str,
) -> u64 {
    assert_eq!(
        eval_text(agent, realm, source),
        Outcome::Normal(expected.to_string()),
        "{source}"
    );
    agent.last_entry_steps().unwrap()
}

/// The walk of `isPrototypeOf` (`#sec-object.prototype.isprototypeof`, step 3) is charged a
/// step per object it passes: the test through a chain of 1,000 prototypes more costs at least
/// 1,000 steps more than the test of the nearest prototype, both answering true.
#[test]
fn is_prototype_of_charges_a_step_per_object() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var root = {}, test = Object.prototype.isPrototypeOf; 0;"
        ),
        Outcome::Normal("0".to_string())
    );
    // The chain is made through `Cx`, one entry per object: a script loop would collect at
    // each instruction with the feature `collect-every-safe-point`.
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let constructor = get_object(cx, global, "Object");
            let create = get(cx, constructor, "create");
            let mut prototype = get(cx, global, "root");
            for _ in 0..1_000 {
                prototype = cx
                    .call(create, Value::Undefined, &[prototype])
                    .unwrap()
                    .unwrap();
            }
            let deep = cx
                .call(create, Value::Undefined, &[prototype])
                .unwrap()
                .unwrap();
            for (name, value) in [("near", prototype), ("deep", deep)] {
                let name = key(cx, name);
                assert!(cx.set(global, name, value, true).unwrap().unwrap());
            }
        })
        .unwrap();
    // The method is called through `call`, so that its Get, which walks the prototypes of the
    // this value too (`#sec-ordinaryget`), costs the same for both.
    let far = steps_of(&mut agent, &realm, "test.call(root, deep);", "true");
    let near = steps_of(&mut agent, &realm, "test.call(near, deep);", "true");
    assert!(far >= near + 1_000, "far {far}, near {near}");
}

/// `Object.freeze`, `Object.seal`, `Object.isFrozen` and `Object.isSealed` are charged a step
/// per own key on top of the steps of [[OwnPropertyKeys]] (`#sec-setintegritylevel`,
/// `#sec-testintegritylevel`), so the cost over an empty object grows with the keys: at least
/// two steps per key, and the same per key for 10 and for 100 keys.
#[test]
fn freeze_charges_a_step_per_key() {
    // A test answers true only after it has read every key, so it reads objects frozen or
    // sealed before; the empty one is then not extensible, so it answers true too.
    for (call, prepare, answer) in [
        ("Object.freeze", "0;", "object"),
        ("Object.seal", "0;", "object"),
        (
            "Object.isFrozen",
            "Object.freeze(a); Object.freeze(b); Object.freeze(c); 0;",
            "true",
        ),
        (
            "Object.isSealed",
            "Object.seal(a); Object.seal(b); Object.seal(c); 0;",
            "true",
        ),
    ] {
        let mut agent = agent().unwrap();
        let realm = realm(&mut agent);
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                "function make(n) { var o = {}; for (var i = 0; i < n; i++) o[\"k\" + i] = i; return o; }\n\
                 var a = make(0), b = make(10), c = make(100); 0;"
            ),
            Outcome::Normal("0".to_string())
        );
        assert_eq!(
            eval_text(&mut agent, &realm, prepare),
            Outcome::Normal("0".to_string())
        );
        let empty = steps_of(&mut agent, &realm, &format!("{call}(a);"), answer);
        let ten = steps_of(&mut agent, &realm, &format!("{call}(b);"), answer);
        let hundred = steps_of(&mut agent, &realm, &format!("{call}(c);"), answer);
        let (d10, d100) = (ten - empty, hundred - empty);
        assert!(d10 >= 20, "{call}: 10 keys cost {d10} steps more");
        assert_eq!(d100, 10 * d10, "{call}: 10 keys {d10}, 100 keys {d100}");
    }
}

/// The step budget ends `Object.freeze` and `Object.seal` of an object with many keys in
/// `Terminated(StepBudget)`, whatever the number of keys, with `last_entry_steps()` at most the
/// budget of 1,000 steps; the agent answers the next script. The object is made through `Cx`,
/// one entry per key, so that making it spends no step of the entry that freezes it.
#[test]
fn step_budget_ends_a_freeze_of_many_keys() {
    for count in [600_u32, 2_000] {
        for call in ["Object.freeze", "Object.seal"] {
            let mut limits = limits();
            limits.steps_per_entry = 1_000;
            let mut agent = agent_with(limits).unwrap();
            let realm = realm(&mut agent);
            assert_eq!(
                eval_text(&mut agent, &realm, "var o = {}; 0;"),
                Outcome::Normal("0".to_string())
            );
            let global_root = global(&mut agent, &realm);
            agent
                .with_cx(&realm, |cx| {
                    let global = object(cx, &global_root);
                    let target = get_object(cx, global, "o");
                    for index in 0..count {
                        let name = key(cx, &format!("k{index}"));
                        assert!(
                            cx.set(target, name, Value::Number(f64::from(index)), true)
                                .unwrap()
                                .unwrap()
                        );
                    }
                })
                .unwrap();
            assert_eq!(
                eval_text(&mut agent, &realm, &format!("{call}(o);")),
                Outcome::Terminated(Termination::StepBudget),
                "{call} of {count} keys"
            );
            let spent = agent.last_entry_steps().unwrap();
            assert!(spent <= 1_000, "{call} of {count} keys spent {spent}");
            assert_eq!(
                eval_text(&mut agent, &realm, "1;"),
                Outcome::Normal("1".to_string())
            );
        }
    }
}

/// The setup of the memory test: a target `t` with 1,000 keys and the properties `p` that
/// redefine each of them as it is, so that the definitions grow nothing.
/// Its keys are set through `Cx`, one entry per key: a script loop would collect at each
/// instruction with the feature `collect-every-safe-point`.
const REDEFINE_ALL: &str = "var d = { value: 1, writable: true, enumerable: true, configurable: true };\n\
     var t = {}, p = {}; 0;";

/// An agent with `memory_bytes` and a realm in which [`REDEFINE_ALL`] ran and its 1,000 keys
/// were set, collected: its heap bytes.
fn redefine_agent(memory_bytes: u64) -> (swell::Agent<support::host::TestHost>, swell::Realm, u64) {
    let mut limits = limits();
    limits.memory_bytes = memory_bytes;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, REDEFINE_ALL),
        Outcome::Normal("0".to_string())
    );
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let target = get_object(cx, global, "t");
            let properties = get_object(cx, global, "p");
            let descriptor = get(cx, global, "d");
            for index in 0..1_000 {
                let name = key(cx, &format!("k{index}"));
                assert!(
                    cx.set(target, name, Value::Number(1.0), true)
                        .unwrap()
                        .unwrap()
                );
                assert!(cx.set(properties, name, descriptor, true).unwrap().unwrap());
            }
        })
        .unwrap();
    agent.collect().unwrap();
    let bytes = agent.heap_bytes().unwrap();
    (agent, realm, bytes)
}

/// ObjectDefineProperties (`#sec-objectdefineproperties`) charges its lists (the keys, the
/// descriptors read and their room on the kept stack) to the memory budget: with 16 KiB of
/// the budget left, `Object.defineProperties(t, {})` answers, and the redefinition of 1,000
/// keys, which grows no object, ends in `Terminated(MemoryBudget)`; with 1 MiB left it
/// answers. The room is left by a String the test holds.
#[test]
fn define_properties_charges_its_lists_to_the_memory_budget() {
    let (_, _, base) = redefine_agent(64 * 1024 * 1024);
    for (room, outcome) in [
        (
            16 * 1024_u64,
            Outcome::Terminated(Termination::MemoryBudget),
        ),
        (1024 * 1024, Outcome::Normal("object".to_string())),
    ] {
        let budget = base + 2 * 1024 * 1024;
        let (mut agent, realm, bytes) = redefine_agent(budget);
        assert_eq!(bytes, base, "the same setup charges the same");
        // A String of `units` code units takes the room down to `room`: measured on one of
        // 1,024 units first.
        let probe = agent
            .with_cx(&realm, |cx| {
                let value = Value::String(cx.new_string(&"x".repeat(1024)).unwrap());
                cx.root(value).unwrap()
            })
            .unwrap();
        agent.collect().unwrap();
        let per_unit = (agent.heap_bytes().unwrap() - base) / 1024;
        assert!(per_unit > 0);
        let left = budget - agent.heap_bytes().unwrap();
        let units = usize::try_from(left.saturating_sub(room + 1024) / per_unit).unwrap();
        let filler = agent
            .with_cx(&realm, |cx| {
                let value = Value::String(cx.new_string(&"y".repeat(units)).unwrap());
                cx.root(value).unwrap()
            })
            .unwrap();
        agent.collect().unwrap();
        let left = budget - agent.heap_bytes().unwrap();
        assert!(
            left >= room && left < room + 4096,
            "{left} bytes left for {room}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "Object.defineProperties(t, {});"),
            Outcome::Normal("object".to_string()),
            "{room}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "Object.defineProperties(t, p);"),
            outcome,
            "{room}"
        );
        drop((probe, filler));
    }
}

/// The body of the stack test: ObjectDefineProperties re-enters `f` through the getter of a
/// descriptor's "value" (`#sec-topropertydescriptor`, step 7), from `Object.defineProperties` and
/// from `Object.create`, and `toLocaleString` through the `toString` it invokes, each to the
/// re-entry limit.
fn reach_through_define_properties_and_to_locale_string() {
    for through in [
        "Object.defineProperties({}, { p: { get value() { return f(); } } })",
        "Object.create(null, { p: { get value() { return f(); } } })",
        "({ toString: f }).toLocaleString()",
    ] {
        reach_through(through, BUILTINS_REENTRY);
    }
}

/// `Limits::min_stack_bytes` through `Object.defineProperties`, `Object.create` and
/// `Object.prototype.toLocaleString` (`AGENTS.md`, "Commands and the gate"): on a thread with
/// seven eighths of `min_stack_bytes()` for `frame_depth` 10,000, `reentry_depth`
/// [`BUILTINS_REENTRY`] and `parser_nesting` 1,000, each script re-enters until the
/// `RangeError` of the re-entry limit and parses at full nesting at the deepest, without a
/// stack overflow.
#[test]
fn define_properties_getter_reaches_the_reentry_limit_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        "define_properties_getter_reaches_the_reentry_limit_on_the_promised_stack",
        stack,
        reach_through_define_properties_and_to_locale_string,
    )
    .unwrap();
}

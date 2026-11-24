//! Symbol values and symbol keys in the builtins of slice 3,
//! `story:symbol-values-in-core-builtins`: the hooks a builtin reads under a well-known symbol (IsConcatSpreadable,
//! `#sec-isconcatspreadable`; ArraySpeciesCreate, `#sec-arrayspeciescreate`), the copy of
//! symbol keys by `Object.assign` (`#sec-object.assign`) in the order of OrdinaryOwnPropertyKeys
//! (`#sec-ordinaryownpropertykeys`), `Object.getOwnPropertySymbols`
//! (`#sec-object.getownpropertysymbols`), and the TypeError of ToString of a Symbol
//! (`#sec-tostring`, step 2) that every method of `String.prototype` meets when its `this` or an
//! argument is a Symbol; then Symbol keys, Symbol values, a Symbol `this` and Symbol objects
//! through the other builtins of slice 3, and a walk that calls every function of the realm
//! with a Symbol in each place.
//!
//! Scripts run in sloppy and in strict code. No test uses an iterator, `for`-`of` or a spread.
//! Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, get, get_object, global, object, realm, text};
use support::script::{Outcome, eval, run_both};
use swell::{Abrupt, AgentState, Cx, ObjectRef, Outcome as HostOutcome, Value};

/// `source` answers the String `expected` in sloppy and in strict code.
fn answers(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(format!("\"{expected}\""));
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// `#sec-array.prototype.concat`, step 5.a, with IsConcatSpreadable (`#sec-isconcatspreadable`):
/// an object that is no Array is spread when its @@isConcatSpreadable is truthy, by its
/// "length" and holes skipped; an Array whose @@isConcatSpreadable is falsy but not undefined
/// is added as one element; an undefined one falls back to IsArray. A getter of the property is
/// read once per item, and its error ends `concat`.
#[test]
fn concat_spreads_an_object_whose_is_concat_spreadable_is_true_and_not_an_array_whose_is_false() {
    answers(
        "var like = { length: 3, 0: \"a\", 2: \"c\" };\n\
         like[Symbol.isConcatSpreadable] = true;\n\
         var array = [\"x\", \"y\"];\n\
         array[Symbol.isConcatSpreadable] = false;\n\
         var plain = [\"p\"];\n\
         plain[Symbol.isConcatSpreadable] = undefined;\n\
         var r = [0].concat(like, array, plain);\n\
         [r.length, r[0], r[1], 2 in r, r[3], r[4] === array, r[5]].join();",
        "6,0,a,false,c,true,p",
    );
    answers(
        "var reads = 0, item = { length: 1, 0: \"z\" };\n\
         Object.defineProperty(item, Symbol.isConcatSpreadable,\n\
           { get: function () { reads++; return 1; } });\n\
         var r = [].concat(item);\n\
         var thrower = {};\n\
         Object.defineProperty(thrower, Symbol.isConcatSpreadable,\n\
           { get: function () { throw new RangeError(\"spreadable\"); } });\n\
         var caught;\n\
         try { [].concat(thrower); } catch (e) { caught = e.constructor === RangeError; }\n\
         [r.length, r[0], reads, caught].join();",
        "1,z,1,true",
    );
}

/// `#sec-arrayspeciescreate`, steps 4 and 5: `map`, `filter`, `slice`, `splice` and `concat`
/// read "constructor" of an Array and then its @@species, once per call; the species
/// constructor is called with `new` and the length, and its result is the result's object. A
/// species of null or undefined makes an Array; one that is not a constructor throws a
/// TypeError.
#[test]
fn species_of_an_array_subclass_like_constructor_is_read_once() {
    answers(
        "var reads = 0, made = [];\n\
         function Species(length) { made.push(length); this.tag = \"species\"; }\n\
         var C = function () {};\n\
         Object.defineProperty(C, Symbol.species,\n\
           { get: function () { reads++; return Species; } });\n\
         var a = [1, 2, 3];\n\
         a.constructor = C;\n\
         var results = [a.map(function (x) { return x; }), a.filter(function () { return true; }),\n\
           a.slice(1), a.splice(0, 1), a.concat()];\n\
         var tags = [];\n\
         for (var i = 0; i < results.length; i++) { tags.push(results[i].tag); }\n\
         [reads, made.join(\"/\"), tags.join(\"/\")].join();",
        "5,3/0/2/1/0,species/species/species/species/species",
    );
    answers(
        "var C = function () {}, r = [];\n\
         var a = [1];\n\
         a.constructor = C;\n\
         C[Symbol.species] = null;\n\
         r.push(Array.isArray(a.map(function (x) { return x; })));\n\
         C[Symbol.species] = undefined;\n\
         r.push(Array.isArray(a.slice()));\n\
         C[Symbol.species] = Math.max;\n\
         try { a.filter(function () { return true; }); r.push(\"answered\"); }\n\
         catch (e) { r.push(e.constructor === TypeError); }\n\
         r.join();",
        "true,true,true",
    );
}

/// `#sec-object.assign`, step 3.a.iii: the keys of a source come from [[OwnPropertyKeys]],
/// which answers the String keys before the Symbol keys (`#sec-ordinaryownpropertykeys`), each
/// group in the order of creation; a Symbol key is copied like a String key, an enumerable one
/// only, and a Symbol value is copied as it is.
#[test]
fn object_assign_copies_symbol_keys_after_string_keys() {
    answers(
        "var order = [], a = Symbol(\"a\"), b = Symbol(\"b\"), hidden = Symbol(\"hidden\");\n\
         var source = {};\n\
         function record(name, value) {\n\
           return { get: function () { order.push(name); return value; }, enumerable: true };\n\
         }\n\
         Object.defineProperty(source, b, record(\"b\", 2));\n\
         Object.defineProperty(source, \"x\", record(\"x\", 1));\n\
         Object.defineProperty(source, a, record(\"a\", a));\n\
         Object.defineProperty(source, \"y\", record(\"y\", 3));\n\
         Object.defineProperty(source, hidden, { value: 4, enumerable: false });\n\
         var target = Object.assign({}, source);\n\
         [order.join(\"\"), target[b], target.x, target[a] === a, target.y,\n\
          hidden in target, Object.getOwnPropertySymbols(target).length].join();",
        "xyba,2,1,true,3,false,2",
    );
}

/// `#sec-object.getownpropertysymbols` with GetOwnPropertyKeys (`#sec-getownpropertykeys`):
/// the Symbol keys of an object in the order they were made, enumerable or not, after none of
/// its String keys; ToObject of a primitive first, so a String answers none and a Symbol
/// object answers its own.
#[test]
fn get_own_property_symbols_answers_the_symbol_keys_in_order() {
    answers(
        "var a = Symbol(\"a\"), b = Symbol(\"b\"), c = Symbol(\"c\"), o = {};\n\
         o[c] = 1; o.x = 2; o[a] = 3;\n\
         Object.defineProperty(o, b, { value: 4, enumerable: false });\n\
         o[1] = 5;\n\
         var keys = Object.getOwnPropertySymbols(o);\n\
         var wrapped = Object(Symbol(\"w\"));\n\
         wrapped[a] = 1;\n\
         [keys.length, keys[0] === c, keys[1] === a, keys[2] === b,\n\
          Object.getOwnPropertySymbols(\"text\").length,\n\
          Object.getOwnPropertySymbols(wrapped)[0] === a].join();",
        "3,true,true,true,0,true",
    );
}

/// `#sec-tostring`, step 2: ToString of a Symbol throws a TypeError, so each method of
/// `String.prototype` whose `this` or String argument is a Symbol throws one, and so does
/// `String.fromCodePoint` (ToNumber, `#sec-tonumber`, step 2) and a method whose integer
/// argument is a Symbol. One call per method, each a TypeError and no other error.
#[test]
fn string_methods_given_a_symbol_throw_type_error() {
    answers(
        "var s = Symbol(\"s\"), p = String.prototype, r = [];\n\
         var calls = [\n\
           function () { return p.codePointAt.call(s); },\n\
           function () { return \"a\".codePointAt(s); },\n\
           function () { return p.endsWith.call(s, \"a\"); },\n\
           function () { return \"a\".endsWith(s); },\n\
           function () { return \"a\".endsWith(\"a\", s); },\n\
           function () { return p.includes.call(s, \"a\"); },\n\
           function () { return \"a\".includes(s); },\n\
           function () { return \"a\".includes(\"a\", s); },\n\
           function () { return p.indexOf.call(s, \"a\"); },\n\
           function () { return \"a\".indexOf(s); },\n\
           function () { return \"a\".padEnd(3, s); },\n\
           function () { return \"a\".padStart(3, s); },\n\
           function () { return p.repeat.call(s, 1); },\n\
           function () { return \"a\".repeat(s); },\n\
           function () { return p.startsWith.call(s, \"a\"); },\n\
           function () { return \"a\".startsWith(s); },\n\
           function () { return \"a\".startsWith(\"a\", s); },\n\
           function () { return p.trimEnd.call(s); },\n\
           function () { return p.trimStart.call(s); },\n\
           function () { return String.fromCodePoint(s); }\n\
         ];\n\
         for (var i = 0; i < calls.length; i++) {\n\
           try { calls[i](); r.push(\"answered \" + i); }\n\
           catch (e) { if (e.constructor !== TypeError) r.push(\"other \" + i); }\n\
         }\n\
         [calls.length, r.join(\" \")].join();",
        "20,",
    );
}

/// Symbol keys and Symbol values through the builtins of slice 3, each answer the one its
/// clause gives: a Symbol key is found, defined and described by the methods of `Object`
/// (`#sec-object.defineproperties` and `#sec-object.create` read the Symbol keys of their
/// properties object too) and left out of `keys`, `values`, `entries` and of `JSON.stringify`
/// (`#sec-serializejsonobject`), which writes `null` for a Symbol element and leaves out a
/// Symbol value, a Symbol element of a property list and a Symbol `space`
/// (`#sec-json.stringify`, steps 4.b and 6); `toLocaleString` of an Array calls the
/// `toString` of a Symbol element (`#sec-array.prototype.tolocalestring`); `bind` names a
/// function whose "name" is a Symbol "bound " (`#sec-function.prototype.bind`, step 8); a
/// Symbol reaches ToString or ToNumber in `join`, `sort`, `Error`, the `String` methods, `Math`,
/// the URI functions, `Number` and `toFixed` and throws a TypeError there
/// (`#sec-tostring`, `#sec-tonumber`), and is kept by `Array`, `Array.of`, `reduce`, `Boolean`,
/// `eval` and `Object.is`. Each probe is one call; a TypeError answers its name.
#[test]
fn symbol_keys_and_values_through_the_builtins_of_slice_3() {
    answers(
        "var s = Symbol(\"d\"), t = Symbol(\"t\"), r = [];\n\
         function k(f) { try { return String(f()); } catch (e) { return e.name; } }\n\
         var o = { a: 1 }; o[s] = 2;\n\
         r.push(k(function () { return o.hasOwnProperty(s); }));\n\
         r.push(k(function () { return o.propertyIsEnumerable(s); }));\n\
         r.push(k(function () { return Object.getOwnPropertyDescriptor(o, s).value; }));\n\
         r.push(k(function () { return Object.getOwnPropertyDescriptor(o, Object(s)).value; }));\n\
         r.push(k(function () { var p = {}; p[t] = { value: 7, enumerable: true }; return Object.defineProperties({}, p)[t]; }));\n\
         r.push(k(function () { var p = {}; p[t] = { value: 8 }; return Object.create(null, p)[t]; }));\n\
         r.push(k(function () { return Object.getOwnPropertyDescriptors(o)[s].value; }));\n\
         r.push(k(function () { return Object.keys(o).length + Object.values(o).length + Object.entries(o).length; }));\n\
         r.push(k(function () { return Object.isFrozen(Object.freeze(o)) && Object.getOwnPropertyDescriptor(o, s).writable; }));\n\
         r.push(k(function () { return JSON.stringify(o); }));\n\
         r.push(k(function () { return JSON.stringify([s, 1]); }));\n\
         r.push(k(function () { return JSON.stringify({ a: s, b: 2 }); }));\n\
         r.push(k(function () { return JSON.stringify(s); }));\n\
         r.push(k(function () { return JSON.stringify(Object(s)); }));\n\
         r.push(k(function () { return JSON.stringify({ a: 1, b: 2 }, [\"a\", s, Object(s)]); }));\n\
         r.push(k(function () { return JSON.stringify([1], null, s); }));\n\
         r.push(k(function () { return JSON.stringify({ toJSON: function () { return s; } }); }));\n\
         r.push(k(function () { return JSON.stringify({ a: 1 }, function (key, v) { return key === \"a\" ? s : v; }); }));\n\
         r.push(k(function () { return [s].toLocaleString(); }));\n\
         r.push(k(function () { return [s].join(); }));\n\
         r.push(k(function () { return [s].toString(); }));\n\
         r.push(k(function () { return [s, t].sort(); }));\n\
         r.push(k(function () { return [1, s].indexOf(s) + [1, s].lastIndexOf(s) + [s].includes(s); }));\n\
         r.push(k(function () { return Object.prototype.toString.call(s); }));\n\
         r.push(k(function () { return Object.prototype.toString.call(Object(s)); }));\n\
         r.push(k(function () { var f = function () {}; Object.defineProperty(f, \"name\", { value: s }); return \"[\" + f.bind().name + \"]\"; }));\n\
         r.push(k(function () { var e = new Error(\"m\"); e.name = s; return e.toString(); }));\n\
         r.push(k(function () { var e = new Error(\"m\"); e.message = s; return e.toString(); }));\n\
         r.push(k(function () { return new Error(s).message; }));\n\
         r.push(k(function () { return Error(\"m\", { cause: s }).cause === s; }));\n\
         r.push(k(function () { return Object.getOwnPropertyNames(Object(s)).length; }));\n\
         r.push(k(function () { return Object.keys(s).length + Object.entries(s).length; }));\n\
         r.push(k(function () { return Object.is(s, s) && !Object.is(s, t); }));\n\
         r.push(k(function () { return Object(s) == s; }));\n\
         r.push(k(function () { return String.prototype.concat.call(\"a\", s); }));\n\
         r.push(k(function () { return \"a\".localeCompare(s); }));\n\
         r.push(k(function () { return \"a\".normalize(s); }));\n\
         r.push(k(function () { return \"a\".split(s); }));\n\
         r.push(k(function () { return \"a\".at(s); }));\n\
         r.push(k(function () { return \"abc\".substring(0, s); }));\n\
         r.push(k(function () { return \"abc\".slice(s); }));\n\
         r.push(k(function () { return \"abc\".lastIndexOf(\"a\", s); }));\n\
         r.push(k(function () { return String.fromCharCode(s); }));\n\
         r.push(k(function () { return String.raw({ raw: [\"a\", \"b\"] }, s); }));\n\
         r.push(k(function () { return [1, 2].at(s); }));\n\
         r.push(k(function () { return Array(s).length; }));\n\
         r.push(k(function () { return Array.of(s)[0] === s; }));\n\
         r.push(k(function () { return new Array(s)[0] === s; }));\n\
         r.push(k(function () { var a = [1]; a.length = s; return a.length; }));\n\
         r.push(k(function () { return Function(\"a\", s); }));\n\
         r.push(k(function () { return Function.prototype.apply.call(function () { return arguments.length; }, null, { length: s }); }));\n\
         r.push(k(function () { return [3, 1].sort(function (a, b) { return s; }); }));\n\
         r.push(k(function () { return [1].reduce(function (a, b) { return a; }, s) === s; }));\n\
         r.push(k(function () { return Math.max(1, s); }));\n\
         r.push(k(function () { return isNaN(s); }));\n\
         r.push(k(function () { return encodeURIComponent(s); }));\n\
         r.push(k(function () { return (1).toFixed(s); }));\n\
         r.push(k(function () { return Number(s); }));\n\
         r.push(k(function () { return Boolean(s); }));\n\
         r.push(k(function () { return eval(s) === s; }));\n\
         r.push(k(function () { return s in o && Object(s) in o; }));\n\
         r.join(\" \");",
        "true true 2 2 7 8 2 3 false {\"a\":1} [null,1] {\"b\":2} undefined {} {\"a\":1} [1] undefined {} Symbol(d) TypeError TypeError TypeError 3 [object Symbol] [object Symbol] [bound ] TypeError TypeError TypeError true 0 0 true true TypeError TypeError TypeError TypeError TypeError TypeError TypeError TypeError TypeError TypeError TypeError 1 true true TypeError TypeError TypeError TypeError true TypeError TypeError TypeError TypeError TypeError true true true",
    );
}

/// A Symbol as the `this` or the object of a builtin of slice 3 and a Symbol object as its
/// argument, each answer the one its clause gives: ToObject of a Symbol is a Symbol object
/// (`#sec-toobject`), with no own property, not an Array and not spreadable, whose
/// [[Prototype]] is `%Symbol.prototype%`; the integrity methods of `Object` answer a primitive
/// as it is (`#sec-object.freeze`, step 1, and the others); `JSON.stringify` reads no `toJSON`
/// of a Symbol (`#sec-serializejsonproperty`, step 2) and writes a Symbol object as an object
/// without properties; OrdinaryHasInstance answers false for a primitive and for a `this` that
/// is not callable and asks the target of a bound function (`#sec-ordinaryhasinstance`); a
/// getter on `%Symbol.prototype%` gets the Symbol as its strict `this`; ToString of a Symbol
/// object reaches its Symbol through `%Symbol.prototype%[%Symbol.toPrimitive%]` and throws a
/// TypeError.
#[test]
fn symbol_this_and_symbol_objects_through_the_builtins_of_slice_3() {
    answers(
        "var s = Symbol(\"d\"), r = [];\n\
         function k(f) { try { return String(f()); } catch (e) { return e.name; } }\n\
         function F() {}\n\
         r.push(k(function () { return Object.prototype.toString.call(Symbol.prototype); }));\n\
         r.push(k(function () { Symbol.prototype.toJSON = function () { return \"x\"; };\n\
           try { return JSON.stringify(s) + JSON.stringify([s]); }\n\
           finally { delete Symbol.prototype.toJSON; } }));\n\
         r.push(k(function () { return JSON.stringify({ a: Object(s) }); }));\n\
         r.push(k(function () { return JSON.stringify({}, null, Object(s)); }));\n\
         r.push(k(function () { return JSON.parse(\"{\\\"a\\\":1}\", function (key, v) { return key === \"a\" ? s : v; }).a === s; }));\n\
         r.push(k(function () { return Object.prototype.toLocaleString.call(s); }));\n\
         r.push(k(function () { return typeof Object.prototype.valueOf.call(s); }));\n\
         r.push(k(function () { return Object.entries(Object(s)).length + Object.getOwnPropertySymbols(s).length; }));\n\
         r.push(k(function () { return Error.prototype.toString.call(s); }));\n\
         r.push(k(function () { return (s instanceof Symbol) + \"/\" + (Object(s) instanceof Symbol); }));\n\
         r.push(k(function () { return Symbol.prototype.toString.call(Object(s)); }));\n\
         r.push(k(function () { return Function.prototype[Symbol.hasInstance].call({}, {}); }));\n\
         r.push(k(function () { return F.bind()[Symbol.hasInstance](new F()); }));\n\
         r.push(k(function () { return Function.prototype[Symbol.hasInstance].call(F, s); }));\n\
         r.push(k(function () { return Array.prototype.toString.call(s); }));\n\
         r.push(k(function () { return Array.prototype.includes.call({ length: 1, 0: s }, s); }));\n\
         r.push(k(function () { return Object.prototype.hasOwnProperty.call(s, \"description\") + \"/\" + Object.prototype.propertyIsEnumerable.call(s, \"description\"); }));\n\
         r.push(k(function () { return Object.getOwnPropertyDescriptor(s, \"description\"); }));\n\
         r.push(k(function () { return Object.prototype.isPrototypeOf.call(Symbol.prototype, Object(s)) + \"/\" + Object.prototype.isPrototypeOf.call(Symbol.prototype, s); }));\n\
         r.push(k(function () { return Object.setPrototypeOf(s, {}) === s && Object.getPrototypeOf(s) === Symbol.prototype; }));\n\
         r.push(k(function () { return [Object.isExtensible(s), Object.isFrozen(s), Object.isSealed(s), Object.preventExtensions(s) === s, Object.freeze(s) === s, Object.seal(s) === s].join(\"/\"); }));\n\
         r.push(k(function () { return Object.defineProperty(s, \"x\", {}); }));\n\
         r.push(k(function () { return String(Object(s)); }));\n\
         r.push(k(function () { return new String(s); }));\n\
         r.push(k(function () { return String.prototype.at.call(Object(s)); }));\n\
         r.push(k(function () { var m = Array.prototype.map.call(s, function () {}); return Array.isArray(m) + \"/\" + m.length; }));\n\
         r.push(k(function () { var c = Array.prototype.concat.call(s); return c.length + \"/\" + typeof c[0] + \"/\" + (c[0].valueOf() === s); }));\n\
         r.push(k(function () { Object.defineProperty(Symbol.prototype, \"self\", { get: function () { \"use strict\"; return this; }, configurable: true });\n\
           try { return s.self === s; } finally { delete Symbol.prototype.self; } }));\n\
         r.push(k(function () { return s.toString() + \"/\" + s.description + \"/\" + Object(s).description; }));\n\
         r.push(k(function () { return (Object(s) == Object(s)) + \"/\" + Object.is(Object(s), s) + \"/\" + Object.hasOwn(Object(s), s); }));\n\
         r.join(\" \");",
        "[object Symbol] undefined[null] {\"a\":{}} {} true Symbol(d) object 0 TypeError false/true Symbol(d) false true false [object Symbol] true false/false undefined true/false true false/true/true/true/true/true TypeError TypeError TypeError TypeError true/0 1/object/true true Symbol(d)/d/d false/false/false",
    );
}

/// The further roots of the walk of [`functions_of_the_realm`]: objects of the realm that no
/// property reached from the global object leads to, each the source of an expression the test
/// evaluates once before the walk, with the path printed for it. The stories of slice 10 append
/// theirs; from `%AsyncFunction.prototype%` the walk reaches `%AsyncFunction%` through
/// `constructor`.
const FURTHER_ROOTS: &[(&str, &str)] = &[(
    "Object.getPrototypeOf(async function () {})",
    "%AsyncFunction.prototype%",
)];

/// The functions of the realm that a walk from the global object and from `roots` (each with
/// its path) reaches through their properties, String-keyed and Symbol-keyed: the value of each
/// data property and the getter and setter of each accessor that is a function, each once, with
/// its place and the object that holds it. Callable is asked of `Function.prototype.toString`,
/// which throws a TypeError for a `this` that is not callable
/// (`#sec-function.prototype.tostring`, step 4). The walk runs no script, so that with the
/// feature `collect-every-safe-point` it pays a collection per call of a builtin, not one per
/// instruction of a walk in script.
fn functions_of_the_realm(
    cx: &mut Cx<'_, TestHost>,
    global: ObjectRef,
    roots: &[(ObjectRef, String)],
) -> Vec<(String, ObjectRef, Value)> {
    let object_constructor = get_object(cx, global, "Object");
    let names_of = get(cx, object_constructor, "getOwnPropertyNames");
    let symbols_of = get(cx, object_constructor, "getOwnPropertySymbols");
    let descriptors_of = get(cx, object_constructor, "getOwnPropertyDescriptors");
    let prototype_of = get(cx, object_constructor, "getPrototypeOf");
    let string = get(cx, global, "String");
    let function_constructor = get_object(cx, global, "Function");
    let function_prototype = get_object(cx, function_constructor, "prototype");
    let to_string = get(cx, function_prototype, "toString");
    let call = |cx: &mut Cx<'_, TestHost>, function: Value, argument: Value| {
        cx.call(function, Value::Undefined, &[argument])
            .unwrap()
            .unwrap()
    };
    let mut queue = vec![(global, "global".to_string())];
    let mut seen = vec![global];
    for (root, path) in roots {
        if !seen.contains(root) {
            seen.push(*root);
            queue.push((*root, path.clone()));
        }
    }
    let mut functions: Vec<(String, ObjectRef, Value)> = Vec::new();
    let mut not_callable = Vec::new();
    let mut next = 0;
    while let Some((o, path)) = queue.get(next).cloned() {
        next += 1;
        let Value::Object(descriptors) = call(cx, descriptors_of, Value::Object(o)) else {
            panic!("{path}: no descriptors");
        };
        let mut keys = Vec::new();
        for (list_of, symbols) in [(names_of, false), (symbols_of, true)] {
            let Value::Object(list) = call(cx, list_of, Value::Object(o)) else {
                panic!("{path}: no keys");
            };
            let Value::Number(length) = get(cx, list, "length") else {
                panic!("{path}: no length");
            };
            for index in 0..length as u32 {
                let element = get(cx, list, &index.to_string());
                let name = match symbols {
                    false => format!(".{}", text(cx, element)),
                    true => {
                        let description = call(cx, string, element);
                        format!("[{}]", text(cx, description))
                    }
                };
                let key = cx.to_property_key(element).unwrap().unwrap();
                let Value::Object(descriptor) = cx.get_property(descriptors, key).unwrap().unwrap()
                else {
                    panic!("{path}{name}: no descriptor");
                };
                keys.push((name, descriptor));
            }
        }
        let mut values = Vec::new();
        for (name, descriptor) in keys {
            // A data descriptor has a "value" and an accessor descriptor has none, so only an
            // accessor's "get" and "set" are read.
            match get(cx, descriptor, "value") {
                Value::Undefined => {
                    for field in ["get", "set"] {
                        let value = get(cx, descriptor, field);
                        values.push((format!("{path}{name} {field}"), value, false));
                    }
                }
                value => values.push((format!("{path}{name}"), value, true)),
            }
        }
        let prototype = call(cx, prototype_of, Value::Object(o));
        values.push((format!("{path}.[[Prototype]]"), prototype, true));
        for (place, value, walk) in values {
            let Value::Object(reached) = value else {
                continue;
            };
            if !functions.iter().any(|(_, _, function)| *function == value)
                && !not_callable.contains(&value)
            {
                match cx.call(to_string, value, &[]) {
                    Ok(Ok(_)) => functions.push((place.clone(), o, value)),
                    _ => not_callable.push(value),
                }
            }
            if walk && !seen.contains(&reached) {
                seen.push(reached);
                queue.push((reached, place));
            }
        }
    }
    functions
}

/// What a call answered: `normal`, the `name` of a thrown error, or the termination.
fn answer(cx: &mut Cx<'_, TestHost>, completion: swell::Completion<Value>) -> String {
    match completion {
        Ok(_) => "normal".to_string(),
        Err(Abrupt::Throw(Value::Object(thrown))) => {
            let name = get(cx, thrown, "name");
            match name {
                Value::String(_) => text(cx, name),
                other => format!("{other:?}"),
            }
        }
        Err(Abrupt::Throw(other)) => format!("threw {other:?}"),
        Err(Abrupt::Terminated(termination)) => format!("{termination:?}"),
    }
}

/// How many of the four places of a Symbol the walk of
/// [`every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_error`] tries per
/// function: all four, or the first two (`this` and the first argument) in the gate
/// with the feature `collect-every-safe-point`, where every call of a builtin collects, unless
/// `SWELL_FULL_COUNTS=1` (the lane `slow`) asks for all four. With all four that mode took
/// 1.8 CPU seconds and 15 to 30 s of wall time at a load of 75 to 100 on 18 cores.
fn places_per_function() -> usize {
    if cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
    {
        2
    } else {
        4
    }
}

/// `#sec-tostring` and `#sec-tonumber`, step 2 of each: a builtin that converts a Symbol to a
/// String or a Number throws a TypeError; one that converts it to an object or a property key,
/// or keeps it, answers. Every function of the realm a walk reaches under a String or a Symbol
/// key from the global object and from the [`FURTHER_ROOTS`] (144 functions on the base of
/// `story:symbol-values-in-core-builtins`; the count is printed) is called with a Symbol as
/// `this` and with a Symbol as its first, second and third argument (`this` the object that
/// holds it, 0 before the Symbol), 576 calls (288 at the gate count of
/// [`places_per_function`]), and each call answers or throws a TypeError, but for the eight
/// calls below whose clauses throw another error before or without the Symbol: no termination,
/// and the agent stays alive. `%AsyncFunction%` is among the functions reached.
#[test]
fn every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_error() {
    let places = places_per_function();
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let further: Vec<(swell::Root, String)> = FURTHER_ROOTS
        .iter()
        .map(|&(source, path)| {
            let HostOutcome::Normal(root) = eval(&mut agent, &realm, &format!("{source};")) else {
                panic!("{path}: {source} did not complete normally");
            };
            (root, path.to_string())
        })
        .collect();
    let mut reached = Vec::new();
    let (called, found) = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let roots: Vec<(ObjectRef, String)> = further
                .iter()
                .map(|(root, path)| (object(cx, root), path.clone()))
                .collect();
            let symbol_constructor = get(cx, global, "Symbol");
            let description = Value::String(cx.new_string("probe").unwrap());
            let symbol = cx
                .call(symbol_constructor, Value::Undefined, &[description])
                .unwrap()
                .unwrap();
            let symbol_root = cx.root(symbol).unwrap();
            let functions = functions_of_the_realm(cx, global, &roots);
            reached = functions
                .iter()
                .map(|(place, _, _)| place.clone())
                .collect();
            let mut found = Vec::new();
            let mut called = 0;
            for (place, holder, function) in functions {
                let symbol = cx.get(&symbol_root).unwrap();
                let zero = Value::Number(0.0);
                let holder = Value::Object(holder);
                let calls: [(&str, Value, Vec<Value>); 4] = [
                    ("this", symbol, vec![]),
                    ("argument 1", holder, vec![symbol]),
                    ("argument 2", holder, vec![zero, symbol]),
                    ("argument 3", holder, vec![zero, zero, symbol]),
                ];
                for (what, this, args) in calls.into_iter().take(places) {
                    called += 1;
                    let completion = match cx.call(function, this, &args) {
                        Ok(completion) => completion,
                        Err(error) => {
                            found.push(format!("{place} ({what}): {error:?}"));
                            return (called, found);
                        }
                    };
                    let answer = answer(cx, completion);
                    if answer != "normal" && answer != "TypeError" {
                        found.push(format!("{place} ({what}): {answer}"));
                    }
                }
            }
            (called, found)
        })
        .unwrap();
    println!("functions of the realm: {}, calls: {called}", reached.len());
    assert_eq!(agent.state(), AgentState::Alive, "{found:#?}");
    assert!(
        reached
            .iter()
            .any(|place| place == "%AsyncFunction.prototype%.constructor"),
        "{reached:#?}"
    );
    // The other errors the clauses give, none of them from the Symbol: `RegExp` with the flags
    // 0, whose ToString "0" is no flag, so the SyntaxError of RegExpInitialize comes before any
    // Symbol is read (`#sec-regexpinitialize`, steps 5 and 6); `JSON.parse` and
    // `JSON.rawJSON` with no argument parse ToString(undefined), "undefined", which is no JSON
    // text (`#sec-json.parse`, step 2; `#sec-json.rawjson`, step 4); `with` of a Symbol `this`
    // reads the length 0 of its Symbol object, and the index 0 is not below it
    // (`#sec-array.prototype.with`, step 7); `normalize` of `String.prototype` with the form 0,
    // whose ToString "0" is no form name, so the RangeError of the form comes before any Symbol
    // is read (`#sec-string.prototype.normalize`, step 4; `#sec-validatestringoption`); and
    // `toString` of `Number.prototype` with the radix 0 (`#sec-number.prototype.tostring`, step
    // 4).
    let expected: Vec<&str> = [
        ("global.RegExp (argument 3): SyntaxError", 4),
        ("global.JSON.parse (this): SyntaxError", 1),
        ("global.JSON.rawJSON (this): SyntaxError", 1),
        ("global.Array.prototype.with (this): RangeError", 1),
        (
            "global.String.prototype.normalize (argument 2): RangeError",
            3,
        ),
        (
            "global.String.prototype.normalize (argument 3): RangeError",
            4,
        ),
        (
            "global.Number.prototype.toString (argument 2): RangeError",
            3,
        ),
        (
            "global.Number.prototype.toString (argument 3): RangeError",
            4,
        ),
    ]
    .into_iter()
    .filter(|&(_, place)| place <= places)
    .map(|(line, _)| line)
    .collect();
    assert_eq!(found, expected);
    assert!(called >= 100 * places, "only {called} calls");
}

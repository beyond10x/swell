//! Symbol values and symbol keys in the builtins of slice 3,
//! `story:symbol-values-in-core-builtins`: the hooks a builtin reads under a well-known symbol (IsConcatSpreadable,
//! `#sec-isconcatspreadable`; ArraySpeciesCreate, `#sec-arrayspeciescreate`), the copy of
//! symbol keys by `Object.assign` (`#sec-object.assign`) in the order of OrdinaryOwnPropertyKeys
//! (`#sec-ordinaryownpropertykeys`), `Object.getOwnPropertySymbols`
//! (`#sec-object.getownpropertysymbols`), and the TypeError of ToString of a Symbol
//! (`#sec-tostring`, step 2) that every method of `String.prototype` meets when its `this` or an
//! argument is a Symbol; then Symbol keys, Symbol values, a Symbol `this` and Symbol objects
//! through the other builtins of slice 3, and a walk that calls every function of the realm
//! with a Symbol in each place, in parts that read the places of the functions from
//! `tests/fixtures/realm_places.txt`.
//!
//! Scripts run in sloppy and in strict code. No test uses an iterator, `for`-`of` or a spread.
//! Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, get, get_object, global, object, text};
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
/// `constructor`; from `%AsyncGeneratorFunction.prototype%` it reaches
/// `%AsyncGeneratorPrototype%` through `prototype` and `%AsyncIteratorPrototype%` through its
/// [[Prototype]].
const FURTHER_ROOTS: &[(&str, &str)] = &[
    (
        "Object.getPrototypeOf(async function () {})",
        "%AsyncFunction.prototype%",
    ),
    (
        "Object.getPrototypeOf(async function* () {})",
        "%AsyncGeneratorFunction.prototype%",
    ),
];

/// The functions of the realm that a walk from the global object and from `roots` (each with
/// its path) reaches through their properties, String-keyed and Symbol-keyed: the value of each
/// data property and the getter and setter of each accessor that is a function, each once, with
/// its place and the object that holds it. Callable is asked of `Function.prototype.toString`,
/// which throws a TypeError for a `this` that is not callable
/// (`#sec-function.prototype.tostring`, step 4). The walk runs no script, so that with the
/// feature `collect-every-safe-point` it pays a collection per call of a builtin, not one per
/// instruction of a walk in script. Only [`the_realm_places_file_equals_the_walk`] makes it,
/// in the default mode: in the collect mode the walk alone is over that mode's bound, so the
/// parts read its places from [`REALM_PLACES`] instead.
#[cfg(not(feature = "collect-every-safe-point"))]
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

/// How many of the four places of a Symbol a part of the walk ([`walk_part`]) tries per
/// function: all four, or the first two (`this` and the first argument) in the gate with the
/// feature `collect-every-safe-point`, where every call of a builtin collects, unless
/// `SWELL_FULL_COUNTS=1` (the lane `slow`) asks for all four. On `557206e`, 344 places in 2
/// parts, the slowest part took 0.021 CPU seconds alone in the default mode, 0.319 in the
/// collect mode at this gate count and 0.687 there at the full count.
fn places_per_function() -> usize {
    if cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
    {
        2
    } else {
        4
    }
}

/// The file of the places of the walk's functions, as the messages name it: two comment lines,
/// then one place per line in the order of the list of `functions_of_the_realm`. The task
/// `realm-places-expected` writes it and nothing else does;
/// [`the_realm_places_file_equals_the_walk`] holds it equal to the walk.
const REALM_PLACES: &str = "crates/swell/tests/fixtures/realm_places.txt";

/// The least number of places the file holds (the walk's old floor of 100 functions).
const LEAST_PLACES: usize = 100;

/// The text of [`REALM_PLACES`], read at run time.
fn realm_places_text() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/realm_places.txt"
    );
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// The place lines of [`REALM_PLACES`], lines 3 to the end, each with its number, counted from
/// 1 with the two comment lines.
fn realm_places() -> Vec<(usize, String)> {
    realm_places_text()
        .lines()
        .enumerate()
        .skip(2)
        .map(|(index, line)| (index + 1, line.to_string()))
        .collect()
}

/// A step of a place after its root.
enum Step {
    /// `.<name>`: a Get by the String key `<name>`.
    Name(String),
    /// `[Symbol(Symbol.<name>)]`: a Get by the well-known symbol `Symbol.<name>`.
    Symbol(String),
    /// `.[[Prototype]]`: [[GetPrototypeOf]].
    Prototype,
}

impl core::fmt::Display for Step {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Step::Name(name) => write!(f, ".{name}"),
            Step::Symbol(name) => write!(f, "[Symbol(Symbol.{name})]"),
            Step::Prototype => write!(f, ".[[Prototype]]"),
        }
    }
}

/// A place parsed: its root (`None` for `global`, else the index of its entry of
/// [`FURTHER_ROOTS`]), its steps, at least one, and the field of the accessor its last step
/// names, `get` or `set`, if it ends at an accessor's function.
struct Place {
    root: Option<usize>,
    steps: Vec<Step>,
    accessor: Option<&'static str>,
}

/// `line` read into its root, its steps and its accessor suffix, or why it cannot be.
fn parse_place(line: &str) -> Result<Place, String> {
    let (body, accessor) = match line.rsplit_once(' ') {
        Some((body, "get")) => (body, Some("get")),
        Some((body, "set")) => (body, Some("set")),
        _ => (line, None),
    };
    let (root, mut rest) = if let Some(rest) = body.strip_prefix("global") {
        (None, rest)
    } else if let Some((index, (_, path))) = FURTHER_ROOTS
        .iter()
        .enumerate()
        .find(|(_, (_, path))| body.starts_with(path))
    {
        (Some(index), &body[path.len()..])
    } else {
        return Err("it starts neither with global nor with a path of FURTHER_ROOTS".to_string());
    };
    let mut steps = Vec::new();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix(".[[Prototype]]") {
            steps.push(Step::Prototype);
            rest = after;
        } else if let Some(after) = rest.strip_prefix('[') {
            let Some((inner, after)) = after.split_once(']') else {
                return Err(format!("the step {rest} has no ]"));
            };
            let Some(name) = inner
                .strip_prefix("Symbol(Symbol.")
                .and_then(|name| name.strip_suffix(')'))
            else {
                return Err(format!("the key [{inner}] is not Symbol(Symbol.<name>)"));
            };
            steps.push(Step::Symbol(name.to_string()));
            rest = after;
        } else if let Some(after) = rest.strip_prefix('.') {
            let end = after.find(['.', '[']).unwrap_or(after.len());
            if end == 0 {
                return Err(format!("an empty name at {rest}"));
            }
            steps.push(Step::Name(after[..end].to_string()));
            rest = &after[end..];
        } else {
            return Err(format!("no step at {rest}"));
        }
    }
    match (steps.last(), accessor) {
        (None, _) => Err("no step after the root".to_string()),
        (Some(Step::Prototype), Some(field)) => Err(format!("a {field} of .[[Prototype]]")),
        _ => Ok(Place {
            root,
            steps,
            accessor,
        }),
    }
}

/// How a value a step reached is named in a message.
fn described(cx: &Cx<'_, TestHost>, value: Value) -> String {
    match value {
        Value::Undefined => "undefined".to_string(),
        Value::Null => "null".to_string(),
        Value::Boolean(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(_) => format!("the String {:?}", text(cx, value)),
        Value::Symbol(_) => "a Symbol".to_string(),
        Value::BigInt(_) => "a BigInt".to_string(),
        Value::Object(_) => "an object".to_string(),
    }
}

/// What [`resolve_place`] reads in a realm: its global object, the objects of
/// [`FURTHER_ROOTS`], `Symbol`, `Object.getOwnPropertyDescriptor` and
/// `Function.prototype.toString`.
struct Resolver {
    global: ObjectRef,
    roots: Vec<ObjectRef>,
    symbol_constructor: ObjectRef,
    descriptor_of: Value,
    to_string: Value,
}

impl Resolver {
    fn new(cx: &mut Cx<'_, TestHost>, global: ObjectRef, roots: &[(ObjectRef, String)]) -> Self {
        let symbol_constructor = get_object(cx, global, "Symbol");
        let object_constructor = get_object(cx, global, "Object");
        let descriptor_of = get(cx, object_constructor, "getOwnPropertyDescriptor");
        let function_constructor = get_object(cx, global, "Function");
        let function_prototype = get_object(cx, function_constructor, "prototype");
        let to_string = get(cx, function_prototype, "toString");
        Resolver {
            global,
            roots: roots.iter().map(|&(root, _)| root).collect(),
            symbol_constructor,
            descriptor_of,
            to_string,
        }
    }

    /// The key of a step that is a Get: a String, or the well-known symbol read as the Get of
    /// its name on `Symbol`.
    fn key(&self, cx: &mut Cx<'_, TestHost>, step: &Step) -> Value {
        match step {
            Step::Name(name) => Value::String(cx.new_string(name).unwrap()),
            Step::Symbol(name) => get(cx, self.symbol_constructor, name),
            Step::Prototype => Value::Undefined,
        }
    }

    /// The value one step reaches from `from`; a Get that throws is a reason.
    fn step(
        &self,
        cx: &mut Cx<'_, TestHost>,
        from: ObjectRef,
        step: &Step,
    ) -> Result<Value, String> {
        if let Step::Prototype = step {
            return Ok(match cx.get_prototype_of(from).unwrap() {
                Ok(Some(prototype)) => Value::Object(prototype),
                Ok(None) => Value::Null,
                Err(_) => return Err(format!("{step} threw")),
            });
        }
        let key = self.key(cx, step);
        let key = cx.to_property_key(key).unwrap().unwrap();
        cx.get_property(from, key)
            .unwrap()
            .map_err(|_| format!("{step} threw"))
    }
}

/// The holder and the function of `place` in the realm of `resolver`: the object its last step
/// starts from, and what that step reaches, checked callable as the walk checks it (a call of
/// `Function.prototype.toString` with it as `this`, which throws a TypeError for a `this` that
/// is not callable, `#sec-function.prototype.tostring`; the host interface has no call that
/// asks IsCallable). An accessor's function is read with one call of
/// `Object.getOwnPropertyDescriptor` and the Get of `get` or `set` on the descriptor.
fn resolve_place(
    cx: &mut Cx<'_, TestHost>,
    resolver: &Resolver,
    place: &Place,
) -> Result<(ObjectRef, Value), String> {
    let mut at = match place.root {
        None => resolver.global,
        Some(index) => resolver.roots[index],
    };
    let Some((last, before)) = place.steps.split_last() else {
        return Err("no step after the root".to_string());
    };
    for step in before {
        match resolver.step(cx, at, step)? {
            Value::Object(next) => at = next,
            other => {
                return Err(format!(
                    "{step} reached {}, not an object",
                    described(cx, other)
                ));
            }
        }
    }
    let (reached, step) = match place.accessor {
        None => (resolver.step(cx, at, last)?, last.to_string()),
        Some(field) => {
            let key = resolver.key(cx, last);
            let descriptor = cx
                .call(
                    resolver.descriptor_of,
                    Value::Undefined,
                    &[Value::Object(at), key],
                )
                .unwrap()
                .map_err(|_| format!("{last} {field}: Object.getOwnPropertyDescriptor threw"))?;
            let Value::Object(descriptor) = descriptor else {
                return Err(format!(
                    "{last} {field} reached the descriptor {}, not an object",
                    described(cx, descriptor)
                ));
            };
            (get(cx, descriptor, field), format!("{last} {field}"))
        }
    };
    if !matches!(reached, Value::Object(_)) {
        return Err(format!(
            "{step} reached {}, not an object",
            described(cx, reached)
        ));
    }
    match cx.call(resolver.to_string, reached, &[]) {
        Ok(Ok(_)) => Ok((at, reached)),
        Ok(Err(_)) => Err("reached an object that is not callable".to_string()),
        Err(error) => Err(format!("the callable check ended with {error:?}")),
    }
}

/// The agent, its bare realm, the objects of [`FURTHER_ROOTS`], and `f(cx, global, roots)` in
/// the one entry of the realm; its answer is given back with the state of the agent after it.
/// Every part of the walk ([`walk_part`]) and [`the_realm_places_file_equals_the_walk`] make
/// their realm here, since [`REALM_PLACES`] holds the places of the bare realm.
fn in_a_bare_realm<R>(
    f: impl FnOnce(&mut Cx<'_, TestHost>, ObjectRef, &[(ObjectRef, String)]) -> R,
) -> (AgentState, R) {
    let mut agent = agent().unwrap();
    // The bare realm, never padded (an exception of `support::host::create_realm`): the file of
    // places holds the functions of the bare realm, and the walk asserts over every one of them.
    let HostOutcome::Normal(realm) = agent.create_realm(None, None).unwrap() else {
        panic!("create_realm did not answer Normal");
    };
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
    let answer = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let roots: Vec<(ObjectRef, String)> = further
                .iter()
                .map(|(root, path)| (object(cx, root), path.clone()))
                .collect();
            f(cx, global, &roots)
        })
        .unwrap();
    (agent.state(), answer)
}

/// The places of the walk, made again, equal [`REALM_PLACES`] line for line, and every place
/// line resolves ([`resolve_place`]) to the holder and the function the walk holds it on. In the
/// default mode only, where the walk takes about 0.05 CPU seconds; in the collect mode the walk
/// alone is over that mode's bound. With `SWELL_REALM_PLACES_OUT` set it writes the walk's
/// places to that path, one per line, and compares nothing: the task `realm-places-expected`
/// writes the file from it. It prints `realm places: <t> lines of <file> equal the walk`.
#[cfg(not(feature = "collect-every-safe-point"))]
#[test]
fn the_realm_places_file_equals_the_walk() {
    let out = std::env::var("SWELL_REALM_PLACES_OUT").ok();
    let file: Vec<(usize, String)> = match out {
        Some(_) => Vec::new(),
        None => realm_places(),
    };
    let (state, (walk, failure)) = in_a_bare_realm(|cx, global, roots| {
        let functions = functions_of_the_realm(cx, global, roots);
        let walk: Vec<String> = functions
            .iter()
            .map(|(place, _, _)| place.clone())
            .collect();
        if out.is_some() {
            return (walk, None);
        }
        let lines: Vec<&String> = file.iter().map(|(_, line)| line).collect();
        if lines.len() != walk.len() || lines.iter().zip(&walk).any(|(line, place)| *line != place)
        {
            let missing: Vec<&String> =
                walk.iter().filter(|place| !lines.contains(place)).collect();
            let unreached: Vec<&String> = lines
                .iter()
                .copied()
                .filter(|line| !walk.contains(*line))
                .collect();
            let first = (0..walk.len().max(lines.len()))
                .find(|&index| walk.get(index) != lines.get(index).copied())
                .unwrap_or(0);
            let side = |line: Option<&String>| line.map_or("none".to_string(), String::clone);
            let failure = format!(
                "realm places: the walk reaches {} places, the file holds {}; missing from the \
                 file: {missing:?}; not reached by the walk: {unreached:?}; first line that \
                 differs: {} (walk {}, file {}); slot --timeout 30m task realm-places-expected \
                 writes the file again",
                walk.len(),
                lines.len(),
                first + 3,
                side(walk.get(first)),
                side(lines.get(first).copied()),
            );
            return (walk, Some(failure));
        }
        let resolver = Resolver::new(cx, global, roots);
        for ((k, line), (_, holder, function)) in file.iter().zip(&functions) {
            let resolved = parse_place(line).and_then(|place| resolve_place(cx, &resolver, &place));
            let (found_holder, found_function) = match resolved {
                Ok(found) => found,
                Err(reason) => {
                    return (
                        walk,
                        Some(format!("realm places: line {k} \"{line}\": {reason}")),
                    );
                }
            };
            if found_holder != *holder || found_function != *function {
                let same = |same: bool| if same { "same" } else { "other" };
                let failure = format!(
                    "realm places: line {k} \"{line}\" resolves to another holder or function \
                     than the walk's (holder {}, function {})",
                    same(found_holder == *holder),
                    same(found_function == *function),
                );
                return (walk, Some(failure));
            }
        }
        (walk, None)
    });
    assert_eq!(state, AgentState::Alive);
    if let Some(path) = out {
        let mut text = walk.join("\n");
        text.push('\n');
        std::fs::write(&path, text).unwrap_or_else(|error| panic!("{path}: {error}"));
        println!("realm places: {} places written to {path}", walk.len());
        return;
    }
    if let Some(failure) = failure {
        panic!("{failure}");
    }
    println!(
        "realm places: {} lines of {REALM_PLACES} equal the walk",
        walk.len()
    );
}

/// FNV-1a, 64 bit, over the UTF-8 bytes of `line`: offset basis `0xcbf29ce484222325`, prime
/// `0x100000001b3`, each byte XORed in and then the product taken modulo 2^64.
fn place_hash(line: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in line.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The place lines of part `part` of [`WALK_PARTS`], in the order of the file: each line whose
/// [`place_hash`] leaves `part` when divided by the number of parts. A function the realm gains
/// or loses moves no other place to another part; only a change of the number of parts does.
fn walk_part_places(part: usize, places: &[(usize, String)]) -> Vec<(usize, String)> {
    places
        .iter()
        .filter(|(_, line)| place_hash(line) % WALK_PARTS.len() as u64 == part as u64)
        .cloned()
        .collect()
}

/// When a row of [`WALK_EXPECTED`] is met.
#[derive(Clone, Copy, Debug)]
enum When {
    /// Always: the clause throws before the Symbol is read, whatever earlier calls left in the
    /// realm.
    Always,
    /// When the index is at least the length of the call's `this`, the holder, read before the
    /// call as LengthOfArrayLike reads it (`#sec-lengthofarraylike`): the length is what earlier
    /// calls of the same realm left in the holder.
    IndexAtLeastLength(u32),
}

/// A call of the walk whose clause throws an error other than a TypeError.
struct WalkRow {
    /// The place of the function.
    function: &'static str,
    /// The place of the Symbol: `this` or `argument <i>`.
    symbol: &'static str,
    /// The name of the error.
    error: &'static str,
    /// The least number of places of [`places_per_function`] that reaches the call.
    least: usize,
    /// When the row is met.
    when: When,
}

impl core::fmt::Display for WalkRow {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} ({}): {}", self.function, self.symbol, self.error)
    }
}

/// The calls of the walk whose clauses throw an error other than a TypeError, none of them from
/// the Symbol. `RegExp` with the flags 0, whose ToString "0" is no flag, so the SyntaxError of
/// RegExpInitialize comes before any Symbol is read (`#sec-regexpinitialize`, steps 5 and 6);
/// `JSON.parse` and `JSON.rawJSON` with no argument parse ToString(undefined), "undefined",
/// which is no JSON text (`#sec-json.parse`, step 2; `#sec-json.rawjson`, step 4); `with` of a
/// Symbol `this` reads the length 0 of its Symbol object, and the index 0 is not below it
/// (`#sec-array.prototype.with`, step 7); `normalize` of `String.prototype` with the form 0,
/// whose ToString "0" is no form name, so the RangeError of the form comes before any Symbol is
/// read (`#sec-string.prototype.normalize`, step 4; `#sec-validatestringoption`); and
/// `toString` of `Number.prototype` with the radix 0 (`#sec-number.prototype.tostring`, step 4)
/// and its `toPrecision` with the precision 0 (`#sec-number.prototype.toprecision`, step 6).
///
/// The two rows of `with` with the Symbol as argument 2 or 3 carry a condition: `with` with the
/// index 0 throws a RangeError when the index is at least the length of its `this`
/// (`#sec-array.prototype.with`, step 7: "If actualIndex ≥ len, throw a RangeError"), before
/// the Symbol is read. That `this` is %Array.prototype%, whose length is what earlier calls left:
/// `global.Array.prototype.push`, earlier in the file, stores the Symbol in it when it is called
/// with it as `this` and the Symbol as argument 1, in the same part.
const WALK_EXPECTED: &[WalkRow] = &[
    WalkRow {
        function: "global.RegExp",
        symbol: "argument 3",
        error: "SyntaxError",
        least: 4,
        when: When::Always,
    },
    WalkRow {
        function: "global.JSON.parse",
        symbol: "this",
        error: "SyntaxError",
        least: 1,
        when: When::Always,
    },
    WalkRow {
        function: "global.JSON.rawJSON",
        symbol: "this",
        error: "SyntaxError",
        least: 1,
        when: When::Always,
    },
    WalkRow {
        function: "global.Array.prototype.with",
        symbol: "this",
        error: "RangeError",
        least: 1,
        when: When::Always,
    },
    WalkRow {
        function: "global.Array.prototype.with",
        symbol: "argument 2",
        error: "RangeError",
        least: 3,
        when: When::IndexAtLeastLength(0),
    },
    WalkRow {
        function: "global.Array.prototype.with",
        symbol: "argument 3",
        error: "RangeError",
        least: 4,
        when: When::IndexAtLeastLength(0),
    },
    WalkRow {
        function: "global.String.prototype.normalize",
        symbol: "argument 2",
        error: "RangeError",
        least: 3,
        when: When::Always,
    },
    WalkRow {
        function: "global.String.prototype.normalize",
        symbol: "argument 3",
        error: "RangeError",
        least: 4,
        when: When::Always,
    },
    WalkRow {
        function: "global.Number.prototype.toString",
        symbol: "argument 2",
        error: "RangeError",
        least: 3,
        when: When::Always,
    },
    WalkRow {
        function: "global.Number.prototype.toString",
        symbol: "argument 3",
        error: "RangeError",
        least: 4,
        when: When::Always,
    },
    WalkRow {
        function: "global.Number.prototype.toPrecision",
        symbol: "argument 2",
        error: "RangeError",
        least: 3,
        when: When::Always,
    },
    WalkRow {
        function: "global.Number.prototype.toPrecision",
        symbol: "argument 3",
        error: "RangeError",
        least: 4,
        when: When::Always,
    },
];

/// ToLength (`#sec-tolength`) of a Number: its integer part, clamped to 0 and 2^53 - 1.
fn to_length(number: f64) -> f64 {
    const MOST: f64 = 9_007_199_254_740_991.0;
    if number.is_nan() || number <= 0.0 {
        0.0
    } else if number >= MOST {
        MOST
    } else {
        number as u64 as f64
    }
}

/// `#sec-tostring` and `#sec-tonumber`, step 2 of each: a builtin that converts a Symbol to a
/// String or a Number throws a TypeError; one that converts it to an object or a property key,
/// or keeps it, answers. Part `part` of [`WALK_PARTS`] takes the place lines of
/// [`REALM_PLACES`] that [`walk_part_places`] gives it, resolves each and checks it callable
/// ([`resolve_place`]) before its first call, since a call can change a property on a later
/// place's path, and then calls each function, in the order of the file, with a Symbol as
/// `this` and with a Symbol as its first, second and third argument (`this` the object that
/// holds it, 0 before the Symbol), in as many of these places as [`places_per_function`] says.
/// Each call answers or throws a TypeError, but for the calls of [`WALK_EXPECTED`] whose clauses
/// throw another error before or without the Symbol, each met exactly when its condition holds:
/// no termination, and the agent stays alive. The count of functions and of calls is printed:
/// `walk part <p> of <n>: <f> of <t> places, <c> calls`.
fn walk_part(part: usize) {
    let n = WALK_PARTS.len();
    let places = places_per_function();
    let all = realm_places();
    let mine = walk_part_places(part, &all);
    let (state, (called, found, expected, differences)) = in_a_bare_realm(|cx, global, roots| {
        let resolver = Resolver::new(cx, global, roots);
        let description = Value::String(cx.new_string("probe").unwrap());
        let symbol = cx
            .call(
                Value::Object(resolver.symbol_constructor),
                Value::Undefined,
                &[description],
            )
            .unwrap()
            .unwrap();
        let symbol_root = cx.root(symbol).unwrap();
        let mut functions = Vec::new();
        for (k, line) in &mine {
            let resolved = parse_place(line).and_then(|place| resolve_place(cx, &resolver, &place));
            match resolved {
                Ok((holder, function)) => functions.push((line.as_str(), holder, function)),
                Err(reason) => panic!("walk part {part} of {n}: line {k} \"{line}\": {reason}"),
            }
        }
        let mut found = Vec::new();
        let mut expected = Vec::new();
        let mut differences = Vec::new();
        let mut called = 0;
        'functions: for (place, holder, function) in functions {
            let symbol = cx.get(&symbol_root).unwrap();
            let zero = Value::Number(0.0);
            let this_holder = Value::Object(holder);
            let calls: [(&str, Value, Vec<Value>); 4] = [
                ("this", symbol, vec![]),
                ("argument 1", this_holder, vec![symbol]),
                ("argument 2", this_holder, vec![zero, symbol]),
                ("argument 3", this_holder, vec![zero, zero, symbol]),
            ];
            for (what, this, args) in calls.into_iter().take(places) {
                let row = WALK_EXPECTED
                    .iter()
                    .find(|row| row.function == place && row.symbol == what && row.least <= places);
                // The condition of a row, read from the realm before the call.
                let condition = match row {
                    Some(
                        row @ WalkRow {
                            when: When::IndexAtLeastLength(index),
                            ..
                        },
                    ) => {
                        let length = get(cx, holder, "length");
                        let Value::Number(number) = length else {
                            panic!(
                                "walk part {part} of {n}: the row {row} under {:?} read length \
                                 {length:?}, not a Number",
                                row.when
                            );
                        };
                        let length = to_length(number);
                        Some((length, f64::from(*index) >= length))
                    }
                    _ => None,
                };
                called += 1;
                let completion = match cx.call(function, this, &args) {
                    Ok(completion) => completion,
                    Err(error) => {
                        let line = format!("{place} ({what}): {error:?}");
                        differences.push(format!(
                            "walk part {part} of {n}: a row not in WALK_EXPECTED: {line}"
                        ));
                        found.push(line);
                        break 'functions;
                    }
                };
                let answer = answer(cx, completion);
                let met = row.is_some_and(|row| row.error == answer);
                let holds = condition.is_none_or(|(_, holds)| holds);
                if let Some(row) = row {
                    if holds {
                        expected.push(row.to_string());
                    }
                    match condition {
                        Some((length, holds)) if met != holds => differences.push(format!(
                            "walk part {part} of {n}: the row {row} under {:?} read length \
                             {length}: the condition is {holds} and the call answered {answer}",
                            row.when
                        )),
                        None if !met => differences.push(format!(
                            "walk part {part} of {n}: the row {row} was not met"
                        )),
                        _ => {}
                    }
                }
                if answer != "normal" && answer != "TypeError" {
                    let line = format!("{place} ({what}): {answer}");
                    if !met {
                        differences.push(format!(
                            "walk part {part} of {n}: a row not in WALK_EXPECTED: {line}"
                        ));
                    }
                    found.push(line);
                }
            }
        }
        (called, found, expected, differences)
    });
    println!(
        "walk part {part} of {n}: {} of {} places, {called} calls",
        mine.len(),
        all.len()
    );
    assert_eq!(state, AgentState::Alive, "{found:#?}");
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    assert_eq!(
        found, expected,
        "walk part {part} of {n}: the rows in the order of its calls"
    );
    assert_eq!(
        called,
        mine.len() * places,
        "walk part {part} of {n}: calls"
    );
}

/// The parts of the walk, [`WALK_PARTS`] and one test each. The invocation writes each test as
/// `#[test]` and `fn <name> = <part>;`, so that `taskfiles/realm-room-tests.awk` sees each part
/// by its name.
macro_rules! walk_parts {
    ($(#[$meta:meta])* const WALK_PARTS; $(#[test] fn $name:ident = $part:literal;)*) => {
        $(#[$meta])*
        const WALK_PARTS: &[usize] = &[$($part),*];
        $(
            #[test]
            fn $name() {
                walk_part($part);
            }
        )*
    };
}

walk_parts!(
    /// The parts of the walk, one test each ([`walk_part`]); part 0 keeps the name of the walk
    /// before it was cut, the others are `..._part_<q>`, `<q>` the part's number plus 1.
    ///
    /// The number of parts follows two rules. There are at least 2 parts, always, so that the
    /// partition, its cover test and the line that adds a part are held by the gate from the
    /// start. Above 2, it is the smallest number from 2 to 8 for which the slowest part, each
    /// measured alone, is at most four fifths of its bound in each of three cells: 1.2 CPU
    /// seconds in the default mode, 1.6 in the collect mode at its gate count, 2.0 there at its
    /// full count. A unit whose part is over adds lines here, measuring every part again at each
    /// number (a hash gives no balance); past 8 parts, or with
    /// `the_realm_places_file_equals_the_walk` over 1.2, the walk is a defect that gets a story.
    /// The readings that chose 2 on `557206e`, the slowest part in CPU seconds per cell (default,
    /// collect at the gate count, collect at the full count), 2 runs each, the higher kept, on the
    /// pool's build node of 32 cores at a load of 7 to 19: 2 parts (181 + 163 places) 0.021,
    /// 0.319 and 0.687; 3 parts (119 + 104 + 121) 0.016, 0.196 and 0.414.
    const WALK_PARTS;
    #[test]
    fn every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_error = 0;
    #[test]
    fn every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_error_part_2 = 1;
);

/// The parts of the walk cover the file of places: [`REALM_PLACES`] has its two comment lines
/// first and no other, no blank line and a final newline; [`WALK_PARTS`] is numbered from 0
/// without a gap and has at least 2 parts; every place line parses ([`parse_place`]) and is on
/// one line only; the place lines of [`walk_part_places`] of all parts are the place lines of
/// the file, each once, and no part holds more than twice the ceiling of their mean; each row
/// of [`WALK_EXPECTED`] names a place the file holds exactly once, so that no part drops a row
/// unseen, and a row with a condition names a call `argument <i>`, whose `this` is the holder;
/// `%AsyncFunction.prototype%.constructor` is a place; and the file holds at least
/// [`LEAST_PLACES`] places. It reads the file and makes no agent, and prints `places of the
/// realm: <t>, in <n> parts: <f0> + <f1> + ...`.
#[test]
fn the_parts_of_the_walk_cover_every_function_of_the_realm() {
    let text = realm_places_text();
    let file = REALM_PLACES;
    for (index, line) in text.lines().enumerate() {
        let k = index + 1;
        if k <= 2 {
            assert!(
                line.starts_with('#'),
                "line {k} of {file} does not start with #"
            );
        } else {
            assert!(
                !line.starts_with('#'),
                "line {k} of {file} starts with #; only lines 1 and 2 may"
            );
        }
        assert!(!line.trim().is_empty(), "line {k} of {file} is blank");
    }
    assert!(text.ends_with('\n'), "{file} does not end with a newline");
    let n = WALK_PARTS.len();
    assert_eq!(
        WALK_PARTS,
        (0..n).collect::<Vec<usize>>(),
        "WALK_PARTS is {WALK_PARTS:?}, not 0 to {}",
        n.saturating_sub(1)
    );
    assert!(n >= 2, "WALK_PARTS has {n} part(s), at least 2");
    let places = realm_places();
    for (k, line) in &places {
        if let Err(reason) = parse_place(line) {
            panic!("line {k} of {file}: \"{line}\": {reason}");
        }
    }
    for (_, place) in &places {
        let lines = places.iter().filter(|(_, line)| line == place).count();
        assert_eq!(
            lines, 1,
            "the place \"{place}\" is on {lines} lines of {file}"
        );
    }
    let parts: Vec<Vec<(usize, String)>> = WALK_PARTS
        .iter()
        .map(|&part| walk_part_places(part, &places))
        .collect();
    let sizes: Vec<String> = parts.iter().map(|part| part.len().to_string()).collect();
    println!(
        "places of the realm: {}, in {n} parts: {}",
        places.len(),
        sizes.join(" + ")
    );
    let owners = |place: &(usize, String)| parts.iter().filter(|part| part.contains(place)).count();
    let in_no_part: Vec<&String> = places
        .iter()
        .filter(|place| owners(place) == 0)
        .map(|(_, line)| line)
        .collect();
    let in_more_parts: Vec<&String> = places
        .iter()
        .filter(|place| owners(place) > 1)
        .map(|(_, line)| line)
        .collect();
    assert!(
        in_no_part.is_empty() && in_more_parts.is_empty(),
        "places of the file in no part: {in_no_part:?}; in more than one part: {in_more_parts:?}"
    );
    let ceiling = places.len().div_ceil(n);
    for (part, held) in parts.iter().enumerate() {
        assert!(
            held.len() <= 2 * ceiling,
            "part {part} of {n} holds {} places, more than twice {ceiling}, the ceiling of {} / {n}",
            held.len(),
            places.len()
        );
    }
    for row in WALK_EXPECTED {
        let lines = places
            .iter()
            .filter(|(_, line)| line == row.function)
            .count();
        assert_eq!(
            lines, 1,
            "the row \"{row}\" of WALK_EXPECTED names a place the file holds {lines} times"
        );
        if let When::IndexAtLeastLength(_) = row.when {
            assert!(
                row.symbol.starts_with("argument "),
                "the row {row} carries a condition on a call whose this is not the holder"
            );
        }
    }
    assert!(
        places
            .iter()
            .any(|(_, line)| line == "%AsyncFunction.prototype%.constructor"),
        "the file has no line %AsyncFunction.prototype%.constructor"
    );
    assert!(
        places.len() >= LEAST_PLACES,
        "the file holds {} places, at least {LEAST_PLACES}",
        places.len()
    );
}

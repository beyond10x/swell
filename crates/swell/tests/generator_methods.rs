//! Generator methods of object literals (MethodDefinition : GeneratorMethod,
//! `#sec-generator-function-definitions`, `#sec-runtime-semantics-methoddefinitionevaluation`):
//! the template of a method whose `generator` is set is a generator function of kind method,
//! made by the instantiation every generator template takes, and defined on its object by
//! DefineMethod. Written by `story:generator-methods-in-object-literals`. Run also with the
//! feature `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`, in sloppy and in strict mode.
#![cfg(test)]

use crate::support;

use support::script::{Outcome, run_both};

/// `source` answers `true` in sloppy and in strict mode.
fn both_pass(source: &str) {
    let passed = Outcome::Normal("true".to_string());
    assert_eq!(run_both(source), (passed.clone(), passed), "{source}");
}

/// `#sec-runtime-semantics-methoddefinitionevaluation` (GeneratorMethod): the closure's
/// [[Prototype]] is `%GeneratorFunction.prototype%`, it has a `prototype` { [[Writable]]: true,
/// [[Enumerable]]: false, [[Configurable]]: false } whose [[Prototype]] is
/// `%GeneratorPrototype%` and which has no `constructor`, each method its own; a generator
/// object the method makes inherits from that `prototype`; and the method has no [[Construct]],
/// so `new` throws a `TypeError` (`#sec-evaluatenew`, step 7).
#[test]
fn a_generator_method_is_a_generator_function_with_its_prototype_object() {
    both_pass(
        "var generatorFunctionPrototype = Object.getPrototypeOf(function* () {});\n\
         var o = { *g() { yield 1; }, *h() {} }, f = o.g;\n\
         var d = Object.getOwnPropertyDescriptor(f, \"prototype\");\n\
         var thrown = false;\n\
         try { new o.g(); } catch (e) { thrown = e instanceof TypeError; }\n\
         Object.getPrototypeOf(f) === generatorFunctionPrototype &&\n\
         d.writable === true && d.enumerable === false && d.configurable === false &&\n\
         Object.getPrototypeOf(d.value) === generatorFunctionPrototype.prototype &&\n\
         !Object.prototype.hasOwnProperty.call(d.value, \"constructor\") &&\n\
         o.h.prototype !== d.value && Object.getPrototypeOf(o.g()) === d.value && thrown;",
    );
}

/// A generator method runs as every generator does (`#sec-generator.prototype.next`,
/// `#sec-generator.prototype.return`, `#sec-generator.prototype.throw`): its parameters and
/// `this` are those of the call, `next` runs to each `yield` and gives it a value, `return` runs
/// its `finally` and completes it, and `throw` throws at its `yield`.
#[test]
fn a_generator_method_runs_as_a_generator() {
    both_pass(
        "var log = [];\n\
         var o = { base: 10, *g(a) {\n\
           try { var b = yield a + this.base; log.push(b); yield b + arguments.length; }\n\
           finally { log.push(\"f\"); }\n\
         } };\n\
         var it = o.g(1), x = it.next(), y = it.next(5), r = it[\"return\"](9), z = it.next();\n\
         var t = o.g(2), caught = false;\n\
         t.next();\n\
         try { t[\"throw\"](new RangeError(\"e\")); } catch (e) { caught = e instanceof RangeError; }\n\
         x.value === 11 && x.done === false && y.value === 6 && y.done === false &&\n\
         r.value === 9 && r.done === true && z.value === undefined && z.done === true &&\n\
         caught && t.next().done === true && log.join() === \"5,f,f\";",
    );
}

/// SetFunctionName (`#sec-setfunctionname`) through DefineMethod: a generator method is named
/// by its key, a computed one by the key computed when the literal is evaluated, a Symbol key as
/// `[description]`; `length` counts its parameters before the first with a default.
#[test]
fn a_generator_method_with_a_computed_name_gets_its_name() {
    both_pass(
        "var k = \"dyn\", s = Symbol(\"sym\");\n\
         var o = { *[k + 1]() {}, *[s](a, b = 0) {}, *plain(a, b) {}, *[0]() {}, *\"str\"() {} };\n\
         o.dyn1.name === \"dyn1\" && o[s].name === \"[sym]\" && o.plain.name === \"plain\" &&\n\
         o[0].name === \"0\" && o.str.name === \"str\" && o.plain.length === 2 &&\n\
         o[s].length === 1;",
    );
}

/// The ComputedPropertyName of a generator method is read with the [Yield] of the code around
/// the literal (`#sec-generator-function-definitions`: `ClassElementName[?Yield, ?Await]`), so in
/// a generator a `yield` there is a YieldExpression of that generator: it suspends the outer
/// generator while the literal is evaluated, and the value given to `next` is the key. A `yield`
/// in the method's body belongs to the method.
#[test]
fn yield_in_a_computed_name_inside_a_generator_belongs_to_the_outer_generator() {
    both_pass(
        "function* outer() { return { *[yield \"key?\"]() { yield \"inner\"; } }; }\n\
         var it = outer(), a = it.next(), b = it.next(\"k\"), o = b.value;\n\
         a.value === \"key?\" && a.done === false && b.done === true &&\n\
         Object.keys(o).join() === \"k\" && o.k.name === \"k\" &&\n\
         o.k().next().value === \"inner\";",
    );
}

/// DefineMethodProperty (`#sec-definemethodproperty`) with enumerable true: a generator method
/// is a data property of its object, writable, enumerable and configurable, in the order of the
/// literal among the other properties.
#[test]
fn a_generator_method_is_an_enumerable_property() {
    both_pass(
        "var o = { a: 1, *g() {}, m() {} };\n\
         var d = Object.getOwnPropertyDescriptor(o, \"g\");\n\
         d.writable === true && d.enumerable === true && d.configurable === true &&\n\
         typeof d.value === \"function\" && Object.keys(o).join() === \"a,g,m\";",
    );
}

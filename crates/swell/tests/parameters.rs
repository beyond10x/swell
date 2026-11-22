//! Default and rest parameters: FunctionDeclarationInstantiation of a parameter list that is not
//! simple (`#sec-functiondeclarationinstantiation`, steps 19 to 30; `docs/design/engine.md`,
//! decision 3, "A parameter list that is not simple"). A default is evaluated only for
//! `undefined`, left to right, each parameter uninitialized until its turn; a rest parameter is a
//! new Array of the remaining arguments, a step per element; `length` is ExpectedArgumentCount;
//! the arguments object is unmapped; a closure in a default does not see the body's `var`
//! bindings, and a direct `eval` in a default declares its `var` names outside the parameters.
//! Written by `story:default-and-rest-parameters`. Run also with the feature
//! `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`; none uses a template literal
//! or a destructuring pattern.
#![cfg(test)]

use crate::support;

use support::host::{agent, limits, realm};
use support::script::{Outcome, eval_text, run, run_both};
use swell::{Outcome as HostOutcome, Parsed, Source};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `#sec-runtime-semantics-iteratorbindinginitialization` (SingleNameBinding, step 4): a default
/// is evaluated only when its argument is `undefined` or missing, never for `null`, `0` or a
/// value given; the defaults run left to right, once each; an anonymous function in a default is
/// named after its parameter (NamedEvaluation).
#[test]
fn a_default_is_evaluated_only_for_undefined_and_left_to_right() {
    let source = "var log = [];\n\
                  function note(name, value) { log.push(name); return value; }\n\
                  function f(a = note(\"a\", 1), b = note(\"b\", 2), c = note(\"c\", 3)) {\n\
                    return [a, b, c].join();\n\
                  }\n\
                  var first = f();\n\
                  var firstLog = log.join(); log = [];\n\
                  var second = f(undefined, null, 0);\n\
                  var secondLog = log.join(); log = [];\n\
                  var third = f(7, undefined);\n\
                  var thirdLog = log.join();\n\
                  function g(h = function () {}, k = () => 0) { return h.name + k.name; }\n\
                  var m = { m(x = 1) { return x; } }.m;\n\
                  first === \"1,2,3\" && firstLog === \"a,b,c\"\n\
                    && second === \"1,,0\" && secondLog === \"a\"\n\
                    && third === \"7,2,3\" && thirdLog === \"b,c\"\n\
                    && g() === \"hk\" && m() === 1 && m(2) === 2;";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-functiondeclarationinstantiation`, steps 21 and 26: each parameter is uninitialized
/// until its turn, so a default reads an earlier parameter, and reading the parameter itself or
/// a later one throws the `ReferenceError` of an uninitialized binding, before the body runs.
#[test]
fn a_default_reads_an_earlier_parameter_and_a_later_one_throws_reference_error() {
    let source = "var ran = 0;\n\
                  function earlier(a, b = a + 1, c = b * 2) { return [a, b, c].join(); }\n\
                  function later(a = b, b) { ran++; }\n\
                  function self(a = a) { ran++; }\n\
                  function closure(a = (() => b)(), b = 1) { ran++; }\n\
                  var arrow = (x, y = x) => y;\n\
                  var kinds = [];\n\
                  [later, self, closure].forEach(function (f) {\n\
                    try { f(); kinds.push(\"none\"); } catch (e) { kinds.push(e.constructor.name); }\n\
                  });\n\
                  earlier(1) === \"1,2,4\" && earlier(1, 5) === \"1,5,10\" && arrow(3) === 3\n\
                    && kinds.join() === \"ReferenceError,ReferenceError,ReferenceError\"\n\
                    && later(1, 2) === undefined && ran === 1;";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-static-semantics-expectedargumentcount` and `#sec-setfunctionlength`: `length` counts
/// the parameters before the first one with a default and before a rest parameter, for
/// declarations, expressions, methods, setters and arrows.
#[test]
fn length_counts_the_parameters_before_the_first_default_or_rest() {
    let source = "function a(x, y = 1, z) {}\n\
                  function b(x, ...r) {}\n\
                  var c = function (x = 1) {};\n\
                  var d = (x, y, ...r) => 0;\n\
                  var o = { m(x, y, z = 0) {}, set s(v = 0) {} };\n\
                  var setter = Object.getOwnPropertyDescriptor(o, \"s\").set;\n\
                  function e(...r) {}\n\
                  a.length === 1 && b.length === 1 && c.length === 0 && d.length === 2\n\
                    && o.m.length === 2 && setter.length === 0 && e.length === 0;";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-runtime-semantics-iteratorbindinginitialization` (FunctionRestParameter): a rest
/// parameter is a new Array of the arguments after the parameters before it, empty when there
/// are none, a new one on each call.
#[test]
fn rest_collects_the_remaining_arguments_into_a_new_array() {
    let source = "function f(a, ...r) { return r; }\n\
                  var one = f(1, 2, 3);\n\
                  var none = f(1);\n\
                  var again = f(1);\n\
                  var all = ((...x) => x)(4, 5);\n\
                  var m = { m(...r) { return r.length; } };\n\
                  Array.isArray(one) && one.length === 2 && one[0] === 2 && one[1] === 3\n\
                    && Array.isArray(none) && none.length === 0 && none !== again\n\
                    && Object.getPrototypeOf(one) === Array.prototype\n\
                    && all.join() === \"4,5\" && m.m(1, 2, 3) === 3;";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// The steps of a script that calls `name` once with 1,000 arguments written out.
fn steps_of_a_call_with_a_thousand_arguments(declaration: &str, name: &str) -> u64 {
    let arguments: Vec<String> = (0..1_000).map(|i| i.to_string()).collect();
    let source = format!("{declaration}\n{name}({});", arguments.join(", "));
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, &source),
        Outcome::Normal("undefined".to_string())
    );
    agent.last_entry_steps().unwrap()
}

/// Decision 3, "A parameter list that is not simple", item 3: the rest Array charges a step per
/// element. `f(...r)` and `g()` are each called by a script of its own with the same 1,000
/// arguments written out; the two calls differ only in the rest Array, and the script that calls
/// `f` spends at least 1,000 steps more.
#[test]
fn rest_charges_a_step_per_element() {
    let rest = steps_of_a_call_with_a_thousand_arguments("function f(...r) {}", "f");
    let none = steps_of_a_call_with_a_thousand_arguments("function g() {}", "g");
    println!("steps with the rest Array {rest}, without {none}");
    assert!(
        rest >= none + 1_000,
        "{rest} steps with the rest Array, {none} without"
    );
}

/// `#sec-functiondeclarationinstantiation`, step 22 (CreateUnmappedArgumentsObject,
/// `#sec-createunmappedargumentsobject`): the arguments object of a list that is not simple is
/// unmapped also in non-strict code; a write to it does not change a parameter, nor the other
/// way round.
#[test]
fn the_arguments_object_of_a_list_that_is_not_simple_is_unmapped() {
    let source = "function dflt(a, b = 0) {\n\
                    arguments[0] = 9; var viaArguments = a;\n\
                    a = 8; return [viaArguments, arguments[0], arguments.length].join();\n\
                  }\n\
                  function rest(a, ...r) { arguments[0] = 9; return a; }\n\
                  function reads(x = arguments[1], y) { return x; }\n\
                  dflt(1) === \"1,9,1\" && rest(1, 2) === 1 && reads(undefined, 5) === 5;";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-functiondeclarationinstantiation`, step 28: a closure made in a default reads the
/// parameter of a name, not the body's `var` of that name, which is a binding of its own that
/// starts with the parameter's value; a name the body declares with `var` and no parameter
/// binds is not seen by the closure either.
#[test]
fn a_closure_in_a_default_does_not_see_the_var_bindings_of_the_body() {
    let source = "var x = \"outside\";\n\
                  function f(a, read = function () { return [a, x].join(); }) {\n\
                    var a; var start = a;\n\
                    var x = \"inside\";\n\
                    a = \"changed\";\n\
                    return [start, a, read()].join(\"|\");\n\
                  }\n\
                  f(\"param\") === \"param|changed|param,outside\";";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-functiondeclarationinstantiation`, step 20, and `#sec-evaldeclarationinstantiation`,
/// step 3: a direct `eval` in a default of non-strict code declares its `var` names in a record
/// outside the parameters, which closures made in the defaults see, and which the body's own
/// `var` of that name shadows, starting as `undefined` (no parameter has the name); a `var` of
/// a parameter's name, or of `arguments`, is the
/// `SyntaxError` of EvalDeclarationInstantiation and the body does not run. In strict code the
/// eval code's `var` names stay in its own record.
#[test]
fn a_direct_eval_in_a_default_declares_outside_the_parameters() {
    let sloppy = "var x = \"outside\", probe, ran = 0;\n\
                  function f(a = eval(\"var x = 'from eval'\"), b = function () { return x; }) {\n\
                    var inBody = x;\n\
                    var x = \"body\";\n\
                    return [b(), inBody, x].join();\n\
                  }\n\
                  function g(a = eval(\"var a = 1\")) { ran++; }\n\
                  function h(p = eval(\"var arguments = 1\")) { ran++; }\n\
                  var kinds = [];\n\
                  [g, h].forEach(function (k) {\n\
                    try { k(); kinds.push(\"none\"); } catch (e) { kinds.push(e.constructor.name); }\n\
                  });\n\
                  f() === \"from eval,,body\" && x === \"outside\"\n\
                    && kinds.join() === \"SyntaxError,SyntaxError\" && ran === 0;";
    assert_eq!(run(limits(), sloppy), passed());
    let strict = "\"use strict\";\n\
                  var x = \"outside\";\n\
                  function f(a = eval(\"var x = 'from eval'; x\"), b = function () { return x; }) {\n\
                    return [a, b()].join();\n\
                  }\n\
                  f() === \"from eval,outside\";";
    assert_eq!(run(limits(), strict), passed());
}

/// `#sec-declarative-environment-records-setmutablebinding-n-v-s`, step 3: a write to a later
/// parameter from a default, directly, from a closure or through a direct `eval`, finds it
/// uninitialized and throws a `ReferenceError` that names it, in sloppy and in strict code.
#[test]
fn a_write_to_a_later_parameter_in_a_default_throws_reference_error_naming_it() {
    let source = "var seen = [];\n\
                  function direct(a = (b = 1), b) {}\n\
                  function closure(a = (() => { b = 1; })(), b) {}\n\
                  function byEval(a = eval(\"b = 1\"), b) {}\n\
                  [direct, closure, byEval].forEach(function (f) {\n\
                    try { f(); seen.push(\"none\"); }\n\
                    catch (e) { seen.push(e.constructor.name + \" \" + e.message); }\n\
                  });\n\
                  seen.join();";
    let thrown = "ReferenceError b is not initialized";
    let expected = Outcome::Normal(format!("\"{thrown},{thrown},{thrown}\""));
    assert_eq!(run_both(source), (expected.clone(), expected));
}

/// HOST-173 (decision 13): a throw from a parameter's initializer is sited at the instruction
/// that threw, in the function's own code: line 2, column 7, where `null.x` starts.
#[test]
fn a_throw_in_a_default_is_sited_there() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let source = "function f(a,\n  b = null.x) {}\nf(1);";
    let Parsed::Parsed(script) = agent
        .parse_script(&realm, Source::Utf8(source), None)
        .unwrap()
    else {
        panic!("{source:?} did not parse");
    };
    assert!(matches!(
        agent.evaluate_script(&script),
        Ok(HostOutcome::Throw(_))
    ));
    let site = agent.last_entry_error_site().unwrap().unwrap();
    assert_eq!((site.line, site.column), (2, 7));
}

/// A ledger with no bound that reads what it holds and its high-water mark.
#[derive(Default)]
struct HighWater {
    held: usize,
    high: usize,
}

impl swell_syntax::Ledger for HighWater {
    fn charge(&mut self, bytes: usize) -> Result<(), swell_syntax::Stop> {
        self.held += bytes;
        self.high = self.high.max(self.held);
        Ok(())
    }

    fn credit(&mut self, bytes: usize) {
        self.held -= bytes;
    }

    fn poll(&mut self) -> Result<(), swell_syntax::Stop> {
        Ok(())
    }
}

/// The frontend's ledger over the parse and compile of `text`, whose first function must have
/// `parameters` parameters: when the parse returns the ledger holds exactly what the tree holds
/// by capacity (a first reading of an arrow's parameters is credited), and while the compile
/// step runs its high-water mark covers the tree and the code, after which it holds exactly both.
fn charged_as_read(text: &str, parameters: usize) {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut ledger = HighWater::default();
    let options = swell_syntax::ParseOptions { max_nesting: 100 };
    let Ok(tree) = swell_syntax::parse_script_with(&units, options, &mut ledger) else {
        panic!("the list of {parameters} parameters did not parse");
    };
    let count = tree.ast.functions().last().map(|f| f.params.len());
    assert_eq!(count, Some(parameters));
    assert_eq!(ledger.held, tree.capacity_bytes(), "held after the parse");
    assert_eq!(tree.charged, tree.capacity_bytes());
    ledger.high = ledger.held;
    let Ok(code) = swell_compiler::compile_script_with(&tree, &mut ledger) else {
        panic!("the list of {parameters} parameters did not compile");
    };
    let held = tree.charged + code.charged;
    println!(
        "tree {} code {} high-water {}",
        tree.charged, code.charged, ledger.high
    );
    assert_eq!(code.charged, code.capacity_bytes());
    assert!(
        ledger.high >= held,
        "high-water {} below {held}",
        ledger.high
    );
    assert_eq!(ledger.held, held, "the compile step credited its scratch");
}

/// The frontend's ledger over a function with 10,000 parameters, each with a default and the
/// last a rest parameter, and over an arrow with as many, read first as the parenthesized
/// expression it covers: the tree, the initializers and the code are charged as they grow, and
/// the first reading of the arrow's list is credited.
#[test]
fn ten_thousand_parameters_with_defaults_are_charged_as_they_are_read() {
    let mut parameters: Vec<String> = (0..9_999).map(|i| format!("p{i} = {i}")).collect();
    parameters.push("...rest".to_string());
    charged_as_read(
        &format!("function f({}) {{ return p9998; }}", parameters.join(", ")),
        10_000,
    );
    charged_as_read(
        &format!("var f = ({}) => p9998;", parameters.join(", ")),
        10_000,
    );
}

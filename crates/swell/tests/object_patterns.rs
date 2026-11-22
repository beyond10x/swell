//! Object patterns: BindingInitialization (`#sec-runtime-semantics-bindinginitialization`) of an
//! ObjectBindingPattern in a `var`, `let` or `const` declaration, a parameter and a `catch`
//! parameter, and DestructuringAssignmentEvaluation
//! (`#sec-runtime-semantics-destructuringassignmentevaluation`) of an ObjectAssignmentPattern.
//! The value is checked by RequireObjectCoercible, each property is read once by GetV in source
//! order, a default is evaluated only for `undefined`, and the reference of an assignment target
//! is evaluated before its value is read. Written by `story:object-destructuring`. Run also with
//! the feature `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`; none uses a template literal,
//! an array pattern, a spread or rest element or a computed property name.
#![cfg(test)]

use crate::support;

use support::host::{agent, realm};
use support::script::{Outcome, run_both};
use swell::{Outcome as HostOutcome, Parsed, Source};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `#sec-requireobjectcoercible`: a pattern of `undefined` or `null` throws a `TypeError`
/// before any property is read, also the empty pattern `{}`, in a declaration, an assignment,
/// a parameter and a `catch` parameter.
#[test]
fn a_pattern_of_undefined_or_null_throws_type_error() {
    let source = "var kinds = [];\n\
                  function attempt(f) {\n\
                    try { f(); kinds.push(\"none\"); } catch (e) { kinds.push(e.constructor.name); }\n\
                  }\n\
                  var a;\n\
                  attempt(function () { var { a } = undefined; });\n\
                  attempt(function () { var { a } = null; });\n\
                  attempt(function () { var {} = null; });\n\
                  attempt(function () { let { a } = null; });\n\
                  attempt(function () { const {} = undefined; });\n\
                  attempt(function () { ({ a } = null); });\n\
                  attempt(function () { ({} = undefined); });\n\
                  attempt(function () { (function ({ a }) {})(); });\n\
                  attempt(function () { (({}) => 0)(null); });\n\
                  attempt(function () { try { throw null; } catch ({ a }) {} });\n\
                  attempt(function () { var {} = 0; var {} = \"\"; var {} = false; });\n\
                  kinds.join() === \"TypeError,TypeError,TypeError,TypeError,TypeError,\"\n\
                    + \"TypeError,TypeError,TypeError,TypeError,TypeError,none\";";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-runtime-semantics-keyedbindinginitialization` and
/// `#sec-runtime-semantics-keyeddestructuringassignmentevaluation`: each property of the
/// pattern is read once, by GetV, in source order, through getters that record their calls,
/// including a key read twice and a nested pattern.
#[test]
fn properties_are_read_once_in_source_order() {
    let source = "var log = [];\n\
                  var o = {\n\
                    get c() { log.push(\"c\"); return { d: 4 }; },\n\
                    get a() { log.push(\"a\"); return 1; },\n\
                    get b() { log.push(\"b\"); return 2; }\n\
                  };\n\
                  var { b, a, c: { d }, a: again } = o;\n\
                  var first = log.join(); log = [];\n\
                  var x, y;\n\
                  ({ a: x, b: y } = o);\n\
                  var second = log.join();\n\
                  first === \"b,a,c,a\" && second === \"a,b\"\n\
                    && a === 1 && b === 2 && d === 4 && again === 1 && x === 1 && y === 2;";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-runtime-semantics-keyedbindinginitialization`, step 4: a default is evaluated only
/// when the property's value is `undefined` (missing or present), never for `null`, `0` or
/// `false`; an anonymous function in a default takes the name of its binding or of a name it
/// is assigned to (NamedEvaluation).
#[test]
fn a_default_is_evaluated_only_for_undefined() {
    let source = "var log = [];\n\
                  function note(name, value) { log.push(name); return value; }\n\
                  var { a = note(\"a\", 1), b = note(\"b\", 2), c = note(\"c\", 3),\n\
                        d = note(\"d\", 4), e = note(\"e\", 5) } =\n\
                    { b: undefined, c: null, d: 0, e: false };\n\
                  var defaults = log.join();\n\
                  var { f = function () {}, g: h = () => 0 } = {};\n\
                  var k;\n\
                  ({ m: k = function () {} } = {});\n\
                  defaults === \"a,b\" && a === 1 && b === 2 && c === null && d === 0\n\
                    && e === false && f.name === \"f\" && h.name === \"h\" && k.name === \"k\";";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-runtime-semantics-propertydestructuringassignmentevaluation` and
/// `#sec-runtime-semantics-keyeddestructuringassignmentevaluation`, step 1: the reference of
/// an assignment target (its object and its key) is evaluated before the property's value is
/// read.
#[test]
fn an_assignment_target_is_evaluated_before_its_value_is_read() {
    let source = "var log = [];\n\
                  var target = {};\n\
                  function where() { log.push(\"target\"); return target; }\n\
                  function key() { log.push(\"key\"); return \"k\"; }\n\
                  var source = {\n\
                    get p() { log.push(\"p\"); return 1; },\n\
                    get q() { log.push(\"q\"); return 2; }\n\
                  };\n\
                  ({ p: where().m, q: where()[key()] } = source);\n\
                  log.join() === \"target,p,target,key,q\" && target.m === 1 && target.k === 2;";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-getv`: a pattern of a primitive reads the properties through its wrapper's
/// prototype, and the value of the pattern is the primitive itself.
#[test]
fn a_pattern_of_a_primitive_reads_through_its_wrapper() {
    let source = "var { length } = \"abc\";\n\
                  var { toFixed } = 1;\n\
                  var { valueOf: v } = true;\n\
                  var r;\n\
                  var s = ({ length: r } = \"de\");\n\
                  length === 3 && toFixed === Number.prototype.toFixed\n\
                    && v === Boolean.prototype.valueOf && r === 2 && s === \"de\";";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// The three object patterns of the measurement (two arrows and a function of the scripts of
/// the consumer): a pattern as a parameter binds its names in the function's scope.
#[test]
fn a_parameter_pattern_of_an_arrow_and_a_function_binds_its_names() {
    let source = "var run = ({ iframe, t, id, input, expected, record }) =>\n\
                    [iframe, t, id, input, expected, record].join();\n\
                  var check = ({ iframe, t, id, input, expected, record }) => {\n\
                    return id + input + expected;\n\
                  };\n\
                  function printDiff({ id, input, expected, actual, container }) {\n\
                    return [id, input, expected, actual, container].join();\n\
                  }\n\
                  var all = { iframe: 1, t: 2, id: 3, input: 4, expected: 5, record: 6,\n\
                              actual: 7, container: 8 };\n\
                  run(all) === \"1,2,3,4,5,6\" && check(all) === 12\n\
                    && printDiff(all) === \"3,4,5,7,8\" && printDiff.length === 1\n\
                    && run.length === 1;";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-let-and-const-declarations` with `#sec-getbindingvalue`: the names of a `let` or
/// `const` pattern are uninitialized until the pattern binds them, so a default or a getter
/// that reads one first throws a `ReferenceError`.
#[test]
fn a_let_pattern_reads_before_initialization_throws_reference_error() {
    let source = "var kinds = [];\n\
                  function attempt(f) {\n\
                    try { f(); kinds.push(\"none\"); } catch (e) { kinds.push(e.constructor.name); }\n\
                  }\n\
                  attempt(function () { let { a = b, b } = {}; });\n\
                  attempt(function () { const { a = a } = {}; });\n\
                  attempt(function () {\n\
                    let { x } = { get x() { return y; } }, y = 1;\n\
                  });\n\
                  attempt(function () { let { a, b = a } = { a: 1 }; if (b !== 1) throw 0; });\n\
                  kinds.join() === \"ReferenceError,ReferenceError,ReferenceError,none\";";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// HOST-173 (decision 13): the `TypeError` of a pattern of `null` is sited at the pattern, line
/// 2, column 5, where its `{` stands.
#[test]
fn a_throw_of_a_pattern_of_null_is_sited_at_the_pattern() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let source = "var o = null;\nvar { a } = o;";
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
    assert_eq!((site.line, site.column), (2, 5));
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

/// The frontend's ledger over the parse and compile of `text`: when the parse returns the
/// ledger holds exactly what the tree holds by capacity (a first reading of a cover is
/// credited), and while the compile step runs its high-water mark covers the tree and the code,
/// after which it holds exactly both.
fn charged_as_read(text: &str) {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut ledger = HighWater::default();
    let options = swell_syntax::ParseOptions { max_nesting: 100 };
    let Ok(tree) = swell_syntax::parse_script_with(&units, options, &mut ledger) else {
        panic!("the pattern did not parse");
    };
    assert_eq!(ledger.held, tree.capacity_bytes(), "held after the parse");
    assert_eq!(tree.charged, tree.capacity_bytes());
    ledger.high = ledger.held;
    let Ok(code) = swell_compiler::compile_script_with(&tree, &mut ledger) else {
        panic!("the pattern did not compile");
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

/// The properties of the pattern of [`the_properties_of_a_large_pattern_are_charged_as_they_are_read`]:
/// 10,000, the full count, in the default mode and in the lane `slow` (`SWELL_FULL_COUNTS=1`);
/// 1,000 in the gate with the feature `collect-every-safe-point`, whose build is slower (with
/// 10,000 the test took 5.5 s there at a load of 55 to 67 on 20 cores, cargo-nextest). The test
/// runs no script, so the feature changes nothing it checks; 1,000 properties still grow every
/// list of the parse and the compile step past its first capacity many times over.
fn pattern_properties() -> usize {
    if cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
    {
        1_000
    } else {
        10_000
    }
}

/// The frontend's ledger over an object pattern of [`pattern_properties`] properties, each with
/// a default, as a `var` and a `let` declaration, a parameter of a function and of an arrow
/// (read first as the parenthesized expression it covers) and an assignment pattern (read first
/// as an object literal): the tree, the names, the defaults and the code are charged as they
/// grow.
#[test]
fn the_properties_of_a_large_pattern_are_charged_as_they_are_read() {
    let n = pattern_properties();
    let properties: Vec<String> = (0..n).map(|i| format!("p{i} = {i}")).collect();
    let pattern = properties.join(", ");
    let last = n - 1;
    charged_as_read(&format!("var {{ {pattern} }} = {{}};"));
    charged_as_read(&format!("let {{ {pattern} }} = {{}};"));
    charged_as_read(&format!(
        "function f({{ {pattern} }}) {{ return p{last}; }}"
    ));
    charged_as_read(&format!("var f = ({{ {pattern} }}) => p{last};"));
    charged_as_read(&format!("var p0; ({{ {pattern} }} = {{}});"));
}

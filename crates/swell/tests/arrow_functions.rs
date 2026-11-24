//! Arrow functions (`#sec-arrow-function-definitions`): the `this` and `arguments` of the code
//! around them (`docs/design/engine.md`, decision 3, "`this` and `arguments` in an arrow
//! function"), no [[Construct]] and no `prototype`, their source text, the collector keeping
//! the `this` an arrow took, the re-entry limit reached through calls of arrows, the site of a
//! throw in an arrow's body, the ledger over a wide parameter list, and the forms of an arrow's
//! parameters, `async` arrows among them. Written by `story:arrow-functions`. Run also with the
//! feature
//! `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`; none uses a template literal,
//! a default or rest parameter or a destructuring pattern.
#![cfg(test)]

use crate::support;

use support::host::{agent, limits, realm, stack_with_room};
use support::script::{Outcome, eval_text, run, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{Outcome as HostOutcome, Parsed, Source};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `#sec-arrow-function-definitions-runtime-semantics-evaluation` and
/// `#sec-ordinarycallbindthis`, step 2: an arrow's `this` is the `this` of the code that made it,
/// a method's, a function's called with `call`, the script's; the `this` argument of `call`,
/// `apply` and `bind` is ignored, and an arrow in an arrow and eval code in an arrow see the same
/// value.
#[test]
fn an_arrow_takes_this_from_the_code_that_made_it() {
    let source = "var o = { m: function () { return () => this; } };\n\
                  var f = o.m();\n\
                  var other = {};\n\
                  function g() { return (() => () => this)()(); }\n\
                  var top = (() => this)();\n\
                  var viaEval = (function () { return () => eval(\"this\"); }).call(other)();\n\
                  var made = (function () { return () => this; }).call(other);\n\
                  f() === o && f.call(other) === o && f.apply(other, []) === o\n\
                    && f.bind(other)() === o && g.call(other) === other && top === this\n\
                    && viaEval === other && made() === other && made.call(o) === other;";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-functiondeclarationinstantiation`, step 16, and decision 3, "Names", item 4: an arrow
/// has no arguments object; its `arguments` is the object of the nearest enclosing function that
/// is not an arrow, read and written through it, at any depth of arrows and from eval code in an
/// arrow; at the top of a script it is a global name. An arrow with a parameter named
/// `arguments` (non-strict code only) binds that name itself.
#[test]
fn an_arrow_reads_and_writes_the_arguments_of_the_enclosing_function() {
    let source = "function f(a, b) {\n\
                    var read = () => arguments[0] + arguments.length;\n\
                    var write = () => { arguments[1] = 10; };\n\
                    var r = read();\n\
                    write();\n\
                    var nested = () => () => arguments[1];\n\
                    var count = (x) => arguments.length;\n\
                    return [r, arguments[1], nested()(), count(1, 2, 3, 4)].join();\n\
                  }\n\
                  var viaEval = (function () { return () => eval(\"arguments.length\"); })(1, 2)();\n\
                  f(1, 2, 3) === \"4,10,10,3\" && viaEval === 2\n\
                    && (() => typeof arguments)() === \"undefined\";";
    assert_eq!(run_both(source), (passed(), passed()));
    // A mapped arguments object aliases the parameters an arrow reads through it.
    let mapped = "function f(a) { var set = () => { arguments[0] = 5; }; set(); return a; }\n\
                  function g(a) { var get = () => arguments[0]; a = 6; return get(); }\n\
                  var own = ((arguments) => arguments)(7);\n\
                  f(1) === 5 && g(1) === 6 && own === 7;";
    assert_eq!(run(limits(), mapped), passed());
}

/// `#sec-arrow-function-definitions-runtime-semantics-evaluation`: an arrow is made by
/// OrdinaryFunctionCreate with no MakeConstructor, so `new` of it, or of a bound function of it,
/// throws the `TypeError` of a value that is not a constructor, and it has no own `prototype`.
#[test]
fn an_arrow_is_not_a_constructor_and_has_no_prototype() {
    let source = "var f = () => 1;\n\
                  var direct = false, bound = false;\n\
                  try { new f(); } catch (e) { direct = e instanceof TypeError; }\n\
                  try { new (f.bind(null))(); } catch (e) { bound = e instanceof TypeError; }\n\
                  direct && bound && !f.hasOwnProperty(\"prototype\") && f.prototype === undefined\n\
                    && typeof f === \"function\" && Object.getPrototypeOf(f) === Function.prototype\n\
                    && f.length === 0 && f.name === \"f\";";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-runtime-semantics-instantiatearrowfunctionexpression` and
/// `#sec-function.prototype.tostring`: the source text of an arrow is the text its ArrowFunction
/// matched, from the first token of its parameters to the end of its body, comments inside
/// included.
#[test]
fn the_source_text_of_an_arrow_is_its_text() {
    let source = "var f = (a, b) => a + b;\n\
                  var g = x => { return x; };\n\
                  var h = () => () => 0;\n\
                  var k = /* before */( /* a */ a /* b */ ) /* c */ => /* d */ a /* after */;\n\
                  f.toString() === \"(a, b) => a + b\" && g.toString() === \"x => { return x; }\"\n\
                    && h.toString() === \"() => () => 0\" && h().toString() === \"() => 0\"\n\
                    && k.toString() === \"( /* a */ a /* b */ ) /* c */ => /* d */ a\";";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// Decision 3, item 1: the function object keeps the `this` its arrow took, which the collector
/// traces: an object reachable only through an arrow's `this` survives a collection between the
/// script that made the arrow and the script that calls it.
#[test]
fn an_arrow_kept_after_its_function_returned_still_sees_its_this() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let make = "var make = function () { var local = { n: 1 }; return () => [this.name, local.n]; };\n\
                var kept = make.call({ name: \"only through the arrow\" });\n\
                make = null;\n\
                typeof kept;";
    assert_eq!(
        eval_text(&mut agent, &realm, make),
        Outcome::Normal("\"function\"".to_string())
    );
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "kept().join() === \"only through the arrow,1\";"
        ),
        passed()
    );
}

/// The body of the stack test: calls of arrows through three builtins that call back into
/// script, one agent each.
fn reach_through_arrows() {
    for through in [
        "[0].map(() => f())",
        "(() => f()).call(null)",
        "JSON.stringify({ toJSON: () => f() })",
    ] {
        reach_through(through, BUILTINS_REENTRY);
    }
}

/// `Limits::min_stack_bytes` through calls of arrows (decision 3): on a thread with seven eighths
/// of `min_stack_bytes()` for `frame_depth` 10,000, `reentry_depth` 1,000 and `parser_nesting`
/// 1,000, a script reaches `reentry_depth` re-entries through an arrow that `map`, `call` and
/// `toJSON` call, and parses 1,000 nested brackets at the deepest, without a stack overflow. In
/// the collect mode the depth is [`BUILTINS_REENTRY`].
#[test]
fn calls_of_arrows_reach_the_reentry_limit_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("calls_of_arrows_reach_the_reentry_limit_on_the_promised_stack"),
        stack,
        reach_through_arrows,
    )
    .unwrap();
}

/// HOST-173 (decision 13): a throw from the expression body of an arrow is sited at the
/// instruction that threw, in the arrow's own code: line 3, column 3, where `null.x` starts.
#[test]
fn a_throw_in_the_expression_body_of_an_arrow_is_sited_there() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let source = "var f = () =>\n\
                  \n  null.x;\n\
                  f();";
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
    assert_eq!((site.line, site.column), (3, 3));
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

/// The frontend's ledger over an arrow with 10,000 parameters, read first as the
/// parenthesized expression it covers and then again as its parameters: when the parse returns
/// the ledger holds exactly what the tree holds by capacity (the first reading is credited), and
/// while the compile step runs its high-water mark covers the tree and the code, after which it
/// holds exactly both.
#[test]
fn an_arrow_with_ten_thousand_parameters_is_charged_as_it_is_read() {
    let parameters: Vec<String> = (0..10_000).map(|i| format!("p{i}")).collect();
    let text = format!("var f = ({}) => p9999;", parameters.join(", "));
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut ledger = HighWater::default();
    let options = swell_syntax::ParseOptions { max_nesting: 100 };
    let Ok(tree) = swell_syntax::parse_script_with(&units, options, &mut ledger) else {
        panic!("the arrow with 10,000 parameters did not parse");
    };
    let arrow = tree.ast.functions().first().map(|f| f.params.len());
    assert_eq!(arrow, Some(10_000));
    assert_eq!(ledger.held, tree.capacity_bytes(), "held after the parse");
    assert_eq!(tree.charged, tree.capacity_bytes());
    ledger.high = ledger.held;
    let Ok(code) = swell_compiler::compile_script_with(&tree, &mut ledger) else {
        panic!("the arrow with 10,000 parameters did not compile");
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

/// Defaults and a rest element parse since `story:default-and-rest-parameters`, an object
/// pattern since `story:object-destructuring`, an array pattern since
/// `story:array-destructuring-and-spread`; an `async` arrow evaluates to a function since
/// `story:async-functions-and-await`.
#[test]
fn arrow_defaults_rest_and_patterns_run_and_async_arrows_run() {
    for source in [
        "var f = (a = 1) => a;",
        "var f = (a, b = 1) => a;",
        "var f = (...a) => a;",
        "var f = (a, ...b) => a;",
        "var f = ({ a }) => a;",
        "var f = ([a]) => a;",
    ] {
        assert_eq!(
            run(limits(), source),
            Outcome::Normal("undefined".into()),
            "{source}"
        );
    }
    for source in [
        "var f = async (a) => a; typeof f;",
        "var f = async a => a; typeof f;",
    ] {
        assert_eq!(
            run(limits(), source),
            Outcome::Normal("\"function\"".into()),
            "{source}"
        );
    }
}

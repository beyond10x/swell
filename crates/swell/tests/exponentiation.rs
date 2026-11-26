//! The exponentiation operator `**` (`#sec-exp-operator`) and its assignment `**=`
//! (`#sec-assignment-operators`), `story:exponentiation-operator`: Number::exponentiate
//! (`#sec-numeric-types-number-exponentiate`) through the operator answers what `Math.pow`
//! answers over a table of pairs, the operands are evaluated before either is converted, the
//! operator is right associative and binds more tightly than `*`, `**=` converts the key of a
//! computed property once and assigns nothing when its value throws, a unary operator as the
//! left operand is the parser's SyntaxError through `Function` and `eval`, and a loop of `**`
//! costs the steps of a loop of `*`, ends in the step budget and is ended by an interrupt from
//! another thread at its loop jump. Also: values of the clause's table written as literals, not
//! compared with `Math.pow`; Symbols and `Symbol.toPrimitive` as operands; the order of the
//! conversions in a chain; `**=` on unusual targets; and the deepest chain the test limits
//! admit run to its value (the cases of the unit's first adversary pass). Run also with the
//! feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent_with, limits, realm};
use support::script::{Outcome, describe, eval_text, parse, run};
use swell::{AgentState, Termination};

/// The message of the parser for a unary expression as the left operand of `**`; the parser's
/// own test (`crates/syntax/tests/sources.rs`) and its fixtures name the same text.
const UNARY_BASE: &str = "a unary expression as the left operand of ** needs parentheses";

/// `source` answers the String `expected` with the test limits.
fn answers(source: &str, expected: &str) {
    assert_eq!(
        run(limits(), source),
        Outcome::Normal(format!("\"{expected}\"")),
        "{source}"
    );
}

/// `x` as a literal of the script: NaN and the infinities by name, every other Number by the
/// shortest digits that read back as it (`-0.0` for -0).
fn js(x: f64) -> String {
    if x.is_nan() {
        "NaN".to_string()
    } else if x == f64::INFINITY {
        "Infinity".to_string()
    } else if x == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else {
        format!("{x:?}")
    }
}

/// The reference data of `story:swell-num-math-exp-and-log`; its `pow` lines whose two enclosing
/// Numbers differ are the pairs near the edge of correct rounding.
const REFERENCE: &str = include_str!("../../../data/math/reference-exp-log.txt");

/// The 120 pairs of [`each_pair_answers_as_math_pow`]: 64 of the special values (each base of
/// eight with each exponent of eight), 12 of integers up to 2^31, the first 40 `pow` lines of
/// the reference data whose two enclosing Numbers differ, and 4 of a subnormal and the largest
/// Number.
fn pairs() -> Vec<(f64, f64)> {
    let inf = f64::INFINITY;
    let bases = [f64::NAN, 0.0, -0.0, 1.0, -1.0, inf, -inf, -1.5];
    let exponents = [f64::NAN, -0.0, 3.0, -3.0, 2.0, 0.5, inf, -inf];
    let mut pairs: Vec<(f64, f64)> = bases
        .iter()
        .flat_map(|&x| exponents.iter().map(move |&y| (x, y)))
        .collect();
    pairs.extend([
        (2.0, 31.0),
        (2.0, -31.0),
        (2.0, 30.0),
        (3.0, 19.0),
        (10.0, 9.0),
        (7.0, 11.0),
        (2_147_483_647.0, 1.0),
        (2.0, -2_147_483_648.0),
        (1.0, -2_147_483_648.0),
        (-2.0, 31.0),
        (-3.0, 7.0),
        (46_341.0, 2.0),
    ]);
    let hex = |field: &str| f64::from_bits(u64::from_str_radix(field, 16).unwrap());
    pairs.extend(
        REFERENCE
            .lines()
            .filter_map(|line| match line.split(' ').collect::<Vec<_>>()[..] {
                ["pow", x, y, lower, upper] if lower != upper => Some((hex(x), hex(y))),
                _ => None,
            })
            .take(40),
    );
    pairs.extend([
        (5e-324, 0.5),
        (2.0, -1074.0),
        (f64::MAX, 1.0),
        (f64::MAX, 0.5),
    ]);
    pairs
}

/// `a ** b` answers the Number `Math.pow(a, b)` answers, by `Object.is`, for each of the 120
/// pairs of [`pairs`]; a pair that differs is named. The script is one statement per pair with
/// the operands as literals and no loop: with a collection at every safe point a loop over the
/// pairs that read them from an array took 3.7 CPU seconds (the bound of that mode is 2).
#[test]
fn each_pair_answers_as_math_pow() {
    let pairs = pairs();
    assert_eq!(pairs.len(), 120);
    let checks: Vec<String> = pairs
        .iter()
        .map(|&(x, y)| {
            let (x, y) = (js(x), js(y));
            format!("is(({x}) ** ({y}), p({x}, {y})) || bad.push('{x} ** {y}');\n")
        })
        .collect();
    let source = format!(
        "var is = Object.is, p = Math.pow, bad = [];\n{}bad.length + ' ' + bad.join(';');",
        checks.concat()
    );
    assert_eq!(source.matches("|| bad.push").count(), 120);
    answers(&source, "0 ");
}

/// EvaluateStringOrNumericBinaryExpression (`#sec-evaluatestringornumericbinaryexpression`):
/// the left operand, then the right one, then ToNumeric of the left value and of the right value
/// (`#sec-applystringornumericbinaryoperator`, steps 3 and 4); `**=` reads its property before
/// it evaluates its value and puts the result after both conversions.
#[test]
fn the_operands_are_evaluated_before_either_is_converted() {
    answers(
        "var log = [];\n\
         function v(name, value) {\n\
           return { valueOf: function () { log.push('valueOf ' + name); return value; } };\n\
         }\n\
         function a() { log.push('a'); return v('a', 2); }\n\
         function b() { log.push('b'); return v('b', 3); }\n\
         log.push(a() ** b());\n\
         var o = {\n\
           get p() { log.push('get'); return v('p', 2); },\n\
           set p(x) { log.push('set ' + x); }\n\
         };\n\
         o.p **= b();\n\
         log.join();",
        "a,b,valueOf a,valueOf b,8,get,b,valueOf p,valueOf b,set 8",
    );
}

/// `**` is right associative and binds more tightly than the multiplicative operators; a
/// parenthesized unary expression is a base, and a unary expression is an exponent.
#[test]
fn right_associativity_and_precedence() {
    answers(
        "[2 ** 3 ** 2, 2 * 3 ** 2, -(2 ** 2), (-2) ** 2, 2 ** -1, (2 ** 3) ** 2, 2 ** 3 * 2,\n\
          10 ** 2 % 7, 2 ** 2 ** -1, (-8) ** (1 / 3)].join();",
        "512,18,-4,4,0.5,64,16,2,1.4142135623730951,NaN",
    );
}

/// `**=` on a variable, a property and a computed property: the key of the computed property is
/// converted once (`#sec-assignment-operators-runtime-semantics-evaluation`, step 1 of the
/// compound form evaluates the LeftHandSideExpression once); a value that throws, or whose
/// `valueOf` throws, leaves the target as it was.
#[test]
fn compound_assignment_targets() {
    answers(
        "var x = 2; x **= 10;\n\
         var o = { p: 3, k: 3 }; o.p **= 3;\n\
         var conversions = 0;\n\
         var key = { toString: function () { conversions++; return 'k'; } };\n\
         o[key] **= 2;\n\
         var y = 5, names = [];\n\
         try { y **= (function () { throw new EvalError(); })(); } catch (e) { names.push(e.name); }\n\
         try { y **= { valueOf: function () { throw new RangeError(); } }; }\n\
         catch (e) { names.push(e.name); }\n\
         [x, o.p, o.k, conversions, y, names.join(' ')].join();",
        "1024,27,9,1,5,EvalError RangeError",
    );
}

/// `-2 ** 2` is the parser's SyntaxError (`#sec-exp-operator`, ExponentiationExpression has no
/// UnaryExpression before `**`) through `new Function` and `eval`, which a script catches, and
/// as a script of its own; each message starts with the parser's.
#[test]
fn a_unary_base_is_a_syntax_error_through_function_and_eval() {
    let Outcome::Normal(caught) = run(
        limits(),
        "var messages = [];\n\
         try { new Function('-2 ** 2'); } catch (e) { messages.push(e.name + ': ' + e.message); }\n\
         try { eval('-2 ** 2'); } catch (e) { messages.push(e.name + ': ' + e.message); }\n\
         messages.join('|');",
    ) else {
        panic!("the script did not complete normally");
    };
    let caught = caught.trim_matches('"');
    let messages: Vec<&str> = caught.split('|').collect();
    assert_eq!(messages.len(), 2, "{caught}");
    for message in messages {
        assert!(
            message.starts_with(&format!("SyntaxError: {UNARY_BASE}")),
            "{message}"
        );
    }
    let Outcome::Errors(errors) = run(limits(), "-2 ** 2;") else {
        panic!("the script parsed");
    };
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].starts_with(UNARY_BASE), "{errors:?}");
}

/// The iterations of the loop of `**` that completes and ends in the step budget: in the default
/// mode 20,000, the story's count; with the feature `collect-every-safe-point` the reduced count
/// 50, accepted by the coordinator for that mode, where every instruction collects the whole
/// heap (200 took 3.5 CPU seconds there, measured in a job of the pool, against the bound of 2).
const ITERATIONS: u32 = if cfg!(feature = "collect-every-safe-point") {
    50
} else {
    20_000
};

/// The iterations of the two loops whose steps are compared, of `*` and of `**`: 1,000; 50 with
/// the feature `collect-every-safe-point`. Both loops of 20,000 took 1.6 CPU seconds in the
/// default mode against its bound of 1.5; the steps of a loop grow by the same count with each
/// iteration, so equal steps at 1,000 are equal steps per operator.
const COMPARED: u32 = if cfg!(feature = "collect-every-safe-point") {
    50
} else {
    1_000
};

/// A script whose loop applies `op` to two Numbers `n` times.
fn loop_of(op: &str, n: u32) -> String {
    format!(
        "var a = 3, b = 2, r;\n\
         for (var i = 0; i < {n}; i++) {{ r = a {op} b; }}\n\
         r;"
    )
}

/// The outcome of the script `source` in `realm` with the steps its evaluation spent, read
/// before the outcome is described (a read through `with_cx` is an entry of its own).
fn evaluated(
    agent: &mut swell::Agent<TestHost>,
    realm: &swell::Realm,
    source: &str,
) -> (Outcome, u64) {
    let script = parse(agent, realm, source);
    let outcome = agent.evaluate_script(&script).unwrap();
    let spent = agent.last_entry_steps().unwrap();
    (describe(agent, realm, outcome), spent)
}

/// The loop of `**` takes exactly the steps of the same loop of `*` ([`COMPARED`] iterations
/// each): the operator costs what the other binary operators cost, and the two differ by the
/// cost of the loop body only, zero.
#[test]
fn a_loop_of_the_operator_costs_the_steps_of_multiplication() {
    let mut big = limits();
    big.steps_per_entry = 1 << 40;
    let mut agent = agent_with(big).unwrap();
    let realm = realm(&mut agent);
    let mut steps = Vec::new();
    for op in ["*", "**"] {
        let expected = if op == "*" { "6" } else { "9" };
        let (outcome, spent) = evaluated(&mut agent, &realm, &loop_of(op, COMPARED));
        assert_eq!(outcome, Outcome::Normal(expected.to_string()), "{op}");
        steps.push(spent);
    }
    println!("steps of {COMPARED} iterations of * and of **: {steps:?}");
    assert_eq!(steps[0], steps[1]);
    assert!(steps[1] > u64::from(COMPARED));
}

/// The loop of [`ITERATIONS`] `**` answers 9 under an ample budget; under 1,000 steps per entry
/// it ends in `Terminated(StepBudget)` with no more than its budget spent, and the agent runs
/// the next entry.
#[test]
fn a_loop_of_the_operator_ends_in_the_step_budget() {
    let mut big = limits();
    big.steps_per_entry = 1 << 40;
    let mut agent = agent_with(big).unwrap();
    let realm = realm(&mut agent);
    let (outcome, spent) = evaluated(&mut agent, &realm, &loop_of("**", ITERATIONS));
    assert_eq!(outcome, Outcome::Normal("9".to_string()));
    println!("steps of {ITERATIONS} iterations of **: {spent}");
    assert!(spent > 1_000);

    let mut small = limits();
    small.steps_per_entry = 1_000;
    let mut agent = agent_with(small).unwrap();
    let realm = support::host::realm(&mut agent);
    let (outcome, spent) = evaluated(&mut agent, &realm, &loop_of("**", ITERATIONS));
    assert_eq!(outcome, Outcome::Terminated(Termination::StepBudget));
    assert!(spent <= 1_000, "{spent}");
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "2 ** 10;"),
        Outcome::Normal("1024".to_string())
    );
}

/// The step budget of the endless loop of
/// [`an_interrupt_from_another_thread_ends_a_loop_of_the_operator`]: 4,000,000 steps, about 8
/// CPU seconds of the loop in the default mode; 20,000 with the feature
/// `collect-every-safe-point` (about 0.4 ms a step there). The interrupt comes after 100 ms, far
/// inside either; a run that does not read it ends at this budget instead.
const INTERRUPT_ROOM: u64 = if cfg!(feature = "collect-every-safe-point") {
    20_000
} else {
    4_000_000
};

/// HOST-90 for a loop of `**` (decision 12): in `for (;;) { r = a ** b; }` the interrupt flag
/// is read at the loop jump and nowhere else, since the loop calls no function, returns from
/// none and runs no builtin that charges steps. `interrupt()` called from another thread 100 ms
/// after the run started ends it in `Terminated(Interrupted)`, with steps spent and fewer than
/// [`INTERRUPT_ROOM`]; without the check at the loop jump the run ends in
/// `Terminated(StepBudget)` (shown once by removing `check_interrupt` from `loop_jump`). The
/// agent runs the next entry.
#[test]
fn an_interrupt_from_another_thread_ends_a_loop_of_the_operator() {
    let mut room = limits();
    room.steps_per_entry = INTERRUPT_ROOM;
    let mut agent = agent_with(room).unwrap();
    let realm = realm(&mut agent);
    let script = parse(
        &mut agent,
        &realm,
        "var a = 3, b = 2, r; for (;;) { r = a ** b; }",
    );
    let handle = agent.interrupt_handle();
    let interrupter = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        handle.interrupt();
    });
    let outcome = agent.evaluate_script(&script).unwrap();
    let spent = agent.last_entry_steps().unwrap();
    interrupter.join().unwrap();
    let outcome = describe(&mut agent, &realm, outcome);
    println!("steps before the interrupt: {spent} of {INTERRUPT_ROOM}");
    assert_eq!(outcome, Outcome::Terminated(Termination::Interrupted));
    assert!(spent > 0 && spent < INTERRUPT_ROOM, "{spent}");
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "3 ** 3;"),
        Outcome::Normal("27".to_string())
    );
}

/// Number::exponentiate (`#sec-numeric-types-number-exponentiate`) through `**`, its operands
/// converted by ToNumeric (`#sec-applystringornumericbinaryoperator`): each of the 42 rows, the
/// Number `source` evaluates to, written as a literal from the clause's cases and not compared
/// with `Math.pow`, shown by `String` with -0 as `-0`.
#[test]
fn values_of_the_clause_written_as_literals() {
    let rows: [(&str, &str); 42] = [
        ("'2' ** '3'", "8"),
        ("null ** 0", "1"),
        ("undefined ** 0", "1"),
        ("NaN ** 0", "1"),
        ("NaN ** -0", "1"),
        ("1 ** NaN", "NaN"),
        ("1 ** Infinity", "NaN"),
        ("1 ** -Infinity", "NaN"),
        ("(-1) ** Infinity", "NaN"),
        ("(-1) ** -Infinity", "NaN"),
        ("0.5 ** Infinity", "0"),
        ("0.5 ** -Infinity", "Infinity"),
        ("(-0.5) ** -Infinity", "Infinity"),
        ("1.5 ** -Infinity", "0"),
        ("(-0) ** 3", "-0"),
        ("(-0) ** -3", "-Infinity"),
        ("(-0) ** 2", "0"),
        ("(-0) ** -2", "Infinity"),
        ("(-0) ** 0.5", "0"),
        ("(-0) ** -0.5", "Infinity"),
        ("0 ** -1", "Infinity"),
        ("0 ** 1", "0"),
        ("(-Infinity) ** 3", "-Infinity"),
        ("(-Infinity) ** 2", "Infinity"),
        ("(-Infinity) ** -3", "-0"),
        ("(-Infinity) ** -2", "0"),
        ("(-Infinity) ** 0.5", "Infinity"),
        ("Infinity ** -1", "0"),
        ("Infinity ** 0.1", "Infinity"),
        ("(-2) ** 0.5", "NaN"),
        ("2 ** 1024", "Infinity"),
        ("2 ** -1075", "0"),
        ("2 ** -1074", "5e-324"),
        ("(-2) ** 3", "-8"),
        ("(-2) ** 1023", "-8.98846567431158e+307"),
        ("[] ** []", "1"),
        ("true ** 2", "1"),
        ("'0x10' ** 2", "256"),
        ("'abc' ** 0", "1"),
        ("({}) ** 0", "1"),
        ("2 ** 2 ** 2 ** 2", "65536"),
        ("typeof (2 ** 2)", "number"),
    ];
    let checks: Vec<String> = rows
        .iter()
        .map(|(source, expected)| {
            format!(
                "s(({source})) === '{expected}' || bad.push({source:?} + ' = ' + s(({source})));\n"
            )
        })
        .collect();
    let source = format!(
        "var bad = [];\n\
         function s(x) {{ return x === 0 && 1 / x < 0 ? '-0' : String(x); }}\n\
         {}bad.join('; ');",
        checks.concat()
    );
    answers(&source, "");
}

/// A Symbol operand throws a TypeError from ToNumeric (`#sec-tonumeric`, `#sec-tonumber`); the
/// left operand is converted first (`#sec-applystringornumericbinaryoperator`, steps 3 and 4),
/// so a Symbol on the right throws after the left `valueOf` ran, and a Symbol on the left throws
/// before the right one is converted. `Symbol.toPrimitive` is called with the hint `number`
/// (`#sec-toprimitive`); `**=` on a Symbol throws and leaves it.
#[test]
fn symbol_operands_and_to_primitive() {
    answers(
        "var log = [];\n\
         function v(name, value) {\n\
           return { valueOf: function () { log.push(name); return value; } };\n\
         }\n\
         try { v('l', 2) ** Symbol(); } catch (e) { log.push(e.name); }\n\
         try { Symbol() ** v('r', 2); } catch (e) { log.push(e.name); }\n\
         var o = {}; o[Symbol.toPrimitive] = function (hint) { log.push(hint); return 3; };\n\
         log.push(o ** 2);\n\
         var x = Symbol(); try { x **= 2; } catch (e) { log.push(e.name, typeof x); }\n\
         log.join();",
        "l,TypeError,TypeError,number,9,TypeError,symbol",
    );
}

/// The conversions of a chain (`#sec-exp-operator-runtime-semantics-evaluation`,
/// `#sec-evaluatestringornumericbinaryexpression`): `a ** b ** c` evaluates `a`, `b` and `c`,
/// converts `b` and `c` for the inner operator, then `a` for the outer one.
#[test]
fn the_conversions_of_a_chain_run_inner_operator_first() {
    answers(
        "var log = [];\n\
         function f(name, value) {\n\
           log.push(name);\n\
           return { valueOf: function () { log.push('v' + name); return value; } };\n\
         }\n\
         log.push(f('a', 2) ** f('b', 3) ** f('c', 2));\n\
         log.join();",
        "a,b,c,vb,vc,va,512",
    );
}

/// `**=` (`#sec-assignment-operators-runtime-semantics-evaluation`, the compound form: the
/// reference, GetValue, the right side, the operator, PutValue) in `with` reads and writes the
/// object's property through its getter and setter; on a frozen object it throws in strict code
/// and assigns nothing in sloppy code; on a `const` it evaluates the right side and throws a
/// TypeError; on an unresolvable name it throws a ReferenceError before the right side is
/// evaluated (GetValue of an unresolvable reference); on a property of `null` a TypeError before
/// the right side.
#[test]
fn compound_assignment_on_unusual_targets() {
    answers(
        "var log = [];\n\
         function r(v) { log.push('rhs'); return v; }\n\
         var o = { get x() { log.push('get'); return 2; }, set x(v) { log.push('set ' + v); } };\n\
         with (o) { x **= r(3); }\n\
         var p = { y: 3 }; with (p) { y **= 2; } log.push(p.y);\n\
         var f = Object.freeze({ q: 2 });\n\
         f.q **= r(3); log.push(f.q);\n\
         (function () { 'use strict'; try { f.q **= r(3); } catch (e) { log.push(e.name); } })();\n\
         const c = 2; try { c **= r(3); } catch (e) { log.push(e.name, c); }\n\
         try { notDeclaredAnywhere **= r(3); } catch (e) { log.push(e.name); }\n\
         var n = null; try { n.p **= r(3); } catch (e) { log.push(e.name); }\n\
         log.join();",
        "get,rhs,set 8,9,rhs,2,rhs,TypeError,rhs,TypeError,2,ReferenceError,TypeError",
    );
}

/// The entries the deepest chain of
/// [`the_deepest_chain_runs_through_a_script_eval_and_function`] runs through: a script, `eval`
/// and `Function` in the default mode; with the feature `collect-every-safe-point` `Function`
/// alone, the entry whose wrappers bind the depth, as every instruction collects there (the
/// three took 3.7 s of wall time there under load in the adversary's run).
const DEEPEST_CHAIN_ENTRIES: &[&str] = if cfg!(feature = "collect-every-safe-point") {
    &["Function"]
} else {
    &["script", "eval", "Function"]
};

/// The deepest chain of `**` the test limits admit (`parser_nesting` 1,000: 990 operators,
/// under the wrappers of `Function`) compiles and runs to its value
/// (`#sec-exp-operator-runtime-semantics-evaluation`) through each of
/// [`DEEPEST_CHAIN_ENTRIES`]: a right-deep tree keeps every operand on the operand stack until
/// the innermost operator runs, and with a collection at every safe point the 990 operands stay
/// held across each collection.
#[test]
fn the_deepest_chain_runs_through_a_script_eval_and_function() {
    let chain = format!("1{}", " ** 1".repeat(990));
    let statements: String = DEEPEST_CHAIN_ENTRIES
        .iter()
        .map(|entry| match *entry {
            "script" => format!("var x = {chain};\nvalues.push(x);\n"),
            "eval" => format!("var y = eval('{chain}');\nvalues.push(y);\n"),
            _ => format!("var z = Function('return {chain};')();\nvalues.push(z);\n"),
        })
        .collect();
    let expected = vec!["1"; DEEPEST_CHAIN_ENTRIES.len()].join(",");
    answers(
        &format!("var values = [];\n{statements}values.join();"),
        &expected,
    );
}

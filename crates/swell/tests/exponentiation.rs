//! The exponentiation operator `**` (`#sec-exp-operator`) and its assignment `**=`
//! (`#sec-assignment-operators`), `story:exponentiation-operator`: Number::exponentiate
//! (`#sec-numeric-types-number-exponentiate`) through the operator answers what `Math.pow`
//! answers over a table of pairs, the operands are evaluated before either is converted, the
//! operator is right associative and binds more tightly than `*`, `**=` converts the key of a
//! computed property once and assigns nothing when its value throws, a unary operator as the
//! left operand is the parser's SyntaxError through `Function` and `eval`, and a loop of `**`
//! costs the steps of a loop of `*`, ends in the step budget and is ended by an interrupt. Run
//! also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::cell::Cell;

use support::host::{TestHost, agent_with, limits, realm};
use support::script::{Outcome, define_function, describe, eval_text, parse, run};
use swell::{AgentState, Completion, Cx, HostCall, InterruptHandle, Termination, Value};

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

/// The iterations of the loop of `**` that ends in the step budget, completes and is
/// interrupted: 20,000, the story's count; 50 with the feature `collect-every-safe-point`, where
/// every instruction collects the whole heap (200 took 3.5 CPU seconds in that mode, measured
/// in a job of the pool, against the bound of 2).
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

/// A script whose loop applies `op` to two Numbers `n` times and, when `interrupt`, calls
/// `interruptNow` in its middle iteration.
fn loop_of(op: &str, n: u32, interrupt: bool) -> String {
    let middle = n / 2;
    let call = if interrupt {
        format!("if (i === {middle}) interruptNow();")
    } else {
        String::new()
    };
    format!(
        "var a = 3, b = 2, r;\n\
         for (var i = 0; i < {n}; i++) {{ r = a {op} b; {call} }}\n\
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
        let (outcome, spent) = evaluated(&mut agent, &realm, &loop_of(op, COMPARED, false));
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
    let (outcome, spent) = evaluated(&mut agent, &realm, &loop_of("**", ITERATIONS, false));
    assert_eq!(outcome, Outcome::Normal("9".to_string()));
    println!("steps of {ITERATIONS} iterations of **: {spent}");
    assert!(spent > 1_000);

    let mut small = limits();
    small.steps_per_entry = 1_000;
    let mut agent = agent_with(small).unwrap();
    let realm = support::host::realm(&mut agent);
    let (outcome, spent) = evaluated(&mut agent, &realm, &loop_of("**", ITERATIONS, false));
    assert_eq!(outcome, Outcome::Terminated(Termination::StepBudget));
    assert!(spent <= 1_000, "{spent}");
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "2 ** 10;"),
        Outcome::Normal("1024".to_string())
    );
}

thread_local! {
    /// The handle `interrupt_now` sets.
    static HANDLE: Cell<Option<InterruptHandle>> = const { Cell::new(None) };
}

/// A host function that sets its agent's interrupt flag.
fn interrupt_now(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Ok(Value::Undefined)
}

/// An interrupt set between two `**` of the loop ends the entry as `Terminated(Interrupted)`,
/// and the agent runs the next entry.
#[test]
fn an_interrupt_between_two_operations_ends_the_loop() {
    let mut big = limits();
    big.steps_per_entry = 1 << 40;
    let mut agent = agent_with(big).unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(&mut agent, &realm, &loop_of("**", ITERATIONS, true)),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "3 ** 3;"),
        Outcome::Normal("27".to_string())
    );
}

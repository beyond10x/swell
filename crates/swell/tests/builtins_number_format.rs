//! `Number.prototype.toFixed` (`#sec-number.prototype.tofixed`), `toExponential`
//! (`#sec-number.prototype.toexponential`), `toPrecision` (`#sec-number.prototype.toprecision`)
//! and `toLocaleString` (`#sec-number.prototype.tolocalestring`), `story:number-format`: their
//! lengths and names, the texts of a table of calls, the order of the coercions, the TypeError
//! of a `this` that is not a Number, and the charges: one step per code unit of the text, a
//! temporary of 2,048 bytes charged before `swell-num` is called and credited after, the step
//! budget and the memory budget reached through them. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent_with, limits, realm};
use support::script::{Outcome, describe, eval_text, parse, run};
use swell::{AgentState, Termination};

/// Whether this run is the gate's with the collector at every safe point: the feature
/// `collect-every-safe-point`, without `SWELL_FULL_COUNTS=1` (which the lane `slow` sets).
fn in_the_collect_gate() -> bool {
    cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

/// `source` answers the String `expected` with the test limits.
fn answers(source: &str, expected: &str) {
    assert_eq!(
        run(limits(), source),
        Outcome::Normal(format!("\"{expected}\"")),
        "{source}"
    );
}

/// The four methods are functions of `%Number.prototype%` with the lengths the standard gives
/// (1, 1, 1 and 0) and their names, each property { [[Writable]]: true, [[Enumerable]]: false,
/// [[Configurable]]: true }.
#[test]
fn the_four_methods_have_their_lengths_and_names() {
    answers(
        "var p = Number.prototype;\n\
         ['toFixed', 'toExponential', 'toPrecision', 'toLocaleString'].map(function (name) {\n\
           var d = Object.getOwnPropertyDescriptor(p, name);\n\
           return [typeof d.value, d.value.length, d.value.name, d.writable, d.enumerable,\n\
             d.configurable].join(' ');\n\
         }).join(';');",
        "function 1 toFixed true false true;function 1 toExponential true false true;\
         function 1 toPrecision true false true;function 0 toLocaleString true false true",
    );
}

/// Sixty calls and their texts: the cases of the standard's notes, of the pinned test262 cases
/// of the four methods (`toFixed/exactness.js`, `range.js`, `S15.7.4.5_A1.1_T01` to
/// `A1.4_T01`, `toExponential/return-values.js`, `tointeger-fractiondigits.js`,
/// `undefined-fractiondigits.js`, `toPrecision/return-values.js`, `exponential.js`,
/// `tointeger-precision.js`, `nan.js`, `infinity.js`), and ties, zeros and signs. One script
/// makes all sixty texts, one per line.
#[test]
fn a_table_of_sixty_calls() {
    let hundred_zeros = "0".repeat(100);
    let table: Vec<(&str, String)> = vec![
        ("(25).toExponential(0)", "3e+1".into()),
        ("(12345).toExponential(3)", "1.235e+4".into()),
        ("(0.000001).toPrecision(2)", "0.0000010".into()),
        ("(1e21).toFixed(2)", "1e+21".into()),
        (
            "(1000000000000000128).toFixed(0)",
            "1000000000000000128".into(),
        ),
        (
            "(1000000000000000128).toString()",
            "1000000000000000100".into(),
        ),
        ("(-1.5).toFixed(0)", "-2".into()),
        ("(-0).toFixed(2)", "0.00".into()),
        ("NaN.toFixed(2)", "NaN".into()),
        ("Infinity.toFixed(2)", "Infinity".into()),
        ("(-Infinity).toExponential(1000)", "-Infinity".into()),
        ("NaN.toPrecision(1000)", "NaN".into()),
        ("Infinity.toPrecision(undefined)", "Infinity".into()),
        ("(3).toFixed(-0)", "3".into()),
        ("(3).toFixed(100)", format!("3.{hundred_zeros}")),
        ("(3).toExponential(100)", format!("3.{hundred_zeros}e+0")),
        ("(3).toPrecision(1)", "3".into()),
        ("(123.456).toExponential(0)", "1e+2".into()),
        ("(123.456).toExponential(2)", "1.23e+2".into()),
        (
            "(123.456).toExponential(17)",
            "1.23456000000000003e+2".into(),
        ),
        (
            "(123.456).toExponential(20)",
            "1.23456000000000003070e+2".into(),
        ),
        ("(-123.456).toExponential(3)", "-1.235e+2".into()),
        (
            "(0.0001).toExponential(17)",
            "1.00000000000000005e-4".into(),
        ),
        ("(0.9999).toExponential(2)", "1.00e+0".into()),
        ("(0.9999).toExponential(3)", "9.999e-1".into()),
        ("(7).toPrecision(21)", "7.00000000000000000000".into()),
        ("(10).toPrecision(2)", "10".into()),
        ("(100).toPrecision(7)", "100.0000".into()),
        ("(100000).toPrecision(7)", "100000.0".into()),
        ("(0.000001).toPrecision(3)", "0.00000100".into()),
        ("(10).toPrecision(1)", "1e+1".into()),
        ("(17).toPrecision(1)", "2e+1".into()),
        (
            "(1.2345e+27).toPrecision(18)",
            "1.23449999999999996e+27".into(),
        ),
        (
            "(-1.2345e+27).toPrecision(21)",
            "-1.23449999999999996184e+27".into(),
        ),
        ("new Number('1e21').toPrecision(3)", "1.00e+21".into()),
        ("(1e-21).toPrecision(16)", "9.999999999999999e-22".into()),
        ("(0.00000001).toPrecision(1)", "1e-8".into()),
        ("(-0.00000001).toPrecision(1)", "-1e-8".into()),
        ("(123.456).toExponential()", "1.23456e+2".into()),
        ("(1.1e-32).toExponential()", "1.1e-32".into()),
        ("(0).toExponential()", "0e+0".into()),
        ("(0).toExponential(2)", "0.00e+0".into()),
        ("(0).toPrecision(1)", "0".into()),
        ("(0).toPrecision(3)", "0.00".into()),
        ("(0.5).toFixed(0)", "1".into()),
        ("(2.5).toFixed(0)", "3".into()),
        ("(1.25).toFixed(1)", "1.3".into()),
        ("(1.005).toFixed(2)", "1.00".into()),
        ("(2.5).toExponential(0)", "3e+0".into()),
        ("(2.5).toPrecision(1)", "3".into()),
        ("Number.prototype.toFixed(1)", "0.0".into()),
        ("new Number(1).toFixed(0.9)", "1".into()),
        ("new Number(1).toFixed('1.1')", "1.0".into()),
        ("new Number(1).toFixed(-0.1)", "1".into()),
        ("(123.456).toExponential(true)", "1.2e+2".into()),
        ("(123.456).toExponential([2])", "1.23e+2".into()),
        ("(123.456).toPrecision('2')", "1.2e+2".into()),
        ("(123.456).toPrecision(1.9)", "1e+2".into()),
        ("(-1e-7).toFixed(2)", "-0.00".into()),
        ("(1234.5678).toFixed(2)", "1234.57".into()),
    ];
    assert_eq!(table.len(), 60);
    let calls: Vec<&str> = table.iter().map(|(call, _)| *call).collect();
    let source = format!("[{}].join('\\n');", calls.join(",\n"));
    let Outcome::Normal(text) = run(limits(), &source) else {
        panic!("the table did not answer");
    };
    let texts: Vec<&str> = text.trim_matches('"').split('\n').collect();
    assert_eq!(texts.len(), 60, "{text}");
    for ((call, expected), ours) in table.iter().zip(texts) {
        assert_eq!(ours, expected, "{call}");
    }
}

/// The order of the steps, from the pinned text of each clause: `this` first (ThisNumberValue),
/// then ToIntegerOrInfinity of the digits, whose throw goes first; `toFixed` then throws the
/// RangeError of digits that are not finite or outside 0 to 100 before it looks at x, while
/// `toExponential` and `toPrecision` answer `Number::toString(x)` for an x that is not finite
/// before their range test; `toPrecision(undefined)` converts nothing.
#[test]
fn the_coercions_run_in_the_order_of_the_steps() {
    let log = "var log = [];\n\
               function digits(value) {\n\
                 return { valueOf: function () { log.push('valueOf ' + value); return value; } };\n\
               }\n\
               function call(f) {\n\
                 try { log.push(f()); } catch (e) { log.push(e.name); }\n\
               }\n";
    answers(
        &format!(
            "{log}call(function () {{ return (1).toFixed(digits(101)); }});\n\
             call(function () {{ return NaN.toFixed(digits(101)); }});\n\
             call(function () {{ return NaN.toFixed(digits(Infinity)); }});\n\
             call(function () {{ return (1).toFixed(digits(2)); }});\n\
             log.join();"
        ),
        "valueOf 101,RangeError,valueOf 101,RangeError,valueOf Infinity,RangeError,valueOf 2,1.00",
    );
    answers(
        &format!(
            "{log}call(function () {{ return Infinity.toExponential(digits(101)); }});\n\
             call(function () {{ return (1).toExponential(digits(101)); }});\n\
             call(function () {{ return (1).toExponential(digits(-1)); }});\n\
             call(function () {{ return NaN.toPrecision(digits(0)); }});\n\
             call(function () {{ return (1).toPrecision(digits(0)); }});\n\
             call(function () {{ return (1).toPrecision(digits(Infinity)); }});\n\
             log.join();"
        ),
        "valueOf 101,Infinity,valueOf 101,RangeError,valueOf -1,RangeError,\
         valueOf 0,NaN,valueOf 0,RangeError,valueOf Infinity,RangeError",
    );
    // A throw of the conversion goes first, and `this` is checked before the argument is
    // converted.
    answers(
        &format!(
            "{log}var thrower = {{ valueOf: function () {{ log.push('thrower'); throw new EvalError(); }} }};\n\
             call(function () {{ return NaN.toExponential(thrower); }});\n\
             call(function () {{ return (1).toFixed(thrower); }});\n\
             call(function () {{ return Number.prototype.toFixed.call('1', thrower); }});\n\
             call(function () {{ return Number.prototype.toPrecision.call({{}}, thrower); }});\n\
             call(function () {{ return (1).toPrecision(undefined); }});\n\
             log.join();"
        ),
        "thrower,EvalError,thrower,EvalError,TypeError,TypeError,1",
    );
}

/// ThisNumberValue (`#sec-thisnumbervalue`): a `this` that is not a Number and not a Number
/// object is a TypeError for each of the four methods; a Number object is read.
#[test]
fn a_this_that_is_not_a_number_is_a_type_error() {
    answers(
        "var names = ['toFixed', 'toExponential', 'toPrecision', 'toLocaleString'];\n\
         var others = ['1', true, null, undefined, {}, Symbol(), new String('1'), [1]];\n\
         var out = [];\n\
         names.forEach(function (name) {\n\
           var f = Number.prototype[name];\n\
           others.forEach(function (value) {\n\
             try { f.call(value, 1); out.push('answered'); }\n\
             catch (e) { out.push(e instanceof TypeError ? 'T' : e.name); }\n\
           });\n\
           out.push(f.call(new Number(2.5), 1));\n\
         });\n\
         out.join();",
        "T,T,T,T,T,T,T,T,2.5,T,T,T,T,T,T,T,T,2.5e+0,T,T,T,T,T,T,T,T,3,T,T,T,T,T,T,T,T,2.5",
    );
}

/// The step budget of [`a_value_of_that_ends_the_run_charges_nothing_more`]: 500, which the
/// endless loop of the `valueOf` reaches in a few hundred instructions. With 10,000 the test
/// took 6.5 CPU seconds with a collection at every safe point (measured 2026-11-24).
const VALUE_OF_STEPS: u64 = 500;

/// An object whose `valueOf` ends the run in the step budget: the run answers
/// `Terminated(StepBudget)`, the counter reads no more than the budget (no step of the text is
/// charged after the termination), and the agent runs the next entry.
#[test]
fn a_value_of_that_ends_the_run_charges_nothing_more() {
    let mut limits = limits();
    limits.steps_per_entry = VALUE_OF_STEPS;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    for method in ["toFixed", "toExponential", "toPrecision"] {
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                &format!("(1).{method}({{ valueOf: function () {{ while (true) {{}} }} }});")
            ),
            Outcome::Terminated(Termination::StepBudget),
            "{method}"
        );
        assert!(agent.last_entry_steps().unwrap() <= VALUE_OF_STEPS);
        assert_eq!(agent.state(), AgentState::Alive);
        assert_eq!(
            eval_text(&mut agent, &realm, "(2.5).toFixed(0);"),
            Outcome::Normal("\"3\"".to_string())
        );
    }
}

/// The steps of `source` parsed and evaluated in `agent`, and what it answered.
fn steps_of(
    agent: &mut swell::Agent<TestHost>,
    realm: &swell::Realm,
    source: &str,
) -> (Outcome, u64) {
    let script = parse(agent, realm, source);
    let outcome = agent.evaluate_script(&script).unwrap();
    let steps = agent.last_entry_steps().unwrap();
    (describe(agent, realm, outcome), steps)
}

/// `toFixed(100)` of 2^-1074 charges one step per code unit of its text, 102 ("0." and 100
/// zeros): one more than 101 over `toFixed(0)`, whose text is "0", in scripts of the same
/// instructions. The bytes the heap is charged after the call, read in a second round with a
/// collection after each (the arenas keep their capacity after the first), equal those before
/// it: the temporary of the call is credited.
#[test]
fn the_text_is_charged_by_its_length_and_the_temporary_is_credited() {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    let (long, long_steps) = steps_of(&mut agent, &realm, "(5e-324).toFixed(100);");
    assert_eq!(long, Outcome::Normal(format!("\"0.{}\"", "0".repeat(100))));
    let (short, short_steps) = steps_of(&mut agent, &realm, "(5e-324).toFixed(0);");
    assert_eq!(short, Outcome::Normal("\"0\"".to_string()));
    assert_eq!(long_steps - short_steps, 102 - 1);

    let script = parse(&mut agent, &realm, "(5e-324).toFixed(100);");
    let mut readings = Vec::new();
    for _ in 0..3 {
        assert!(matches!(agent.collect(), Ok(swell::Outcome::Normal(_))));
        let before = agent.heap_bytes().unwrap();
        let outcome = agent.evaluate_script(&script).unwrap();
        drop(outcome);
        assert!(matches!(agent.collect(), Ok(swell::Outcome::Normal(_))));
        readings.push((before, agent.heap_bytes().unwrap()));
    }
    println!("heap_bytes before and after each round: {readings:?}");
    let (before, after) = readings[1];
    assert_eq!(after, before, "the second round");
    assert_eq!(readings[2], readings[1], "the third round");
}

/// The count of calls of the loop below: 2,000 in the default mode; with the collector at every
/// safe point, where every instruction collects, 400 at the full count (`SWELL_FULL_COUNTS=1`)
/// and 50 in the gate. Measured on 2026-11-24 with a collection at every safe point: 2,000
/// calls took 8.6 CPU seconds and 200 took 1.1, about 4.2 milliseconds a call.
fn loop_calls() -> u32 {
    if in_the_collect_gate() {
        50
    } else if cfg!(feature = "collect-every-safe-point") {
        400
    } else {
        2_000
    }
}

/// A loop of [`loop_calls`] calls of `(1e-300).toPrecision(100)`, each charged 106 steps for
/// its text ("1.", 99 digits and "e-300"): under 400,000 steps per entry it ends normally; under
/// 5,000, which even 50 calls pass, it ends in `Terminated(StepBudget)` and the agent runs the
/// next entry.
#[test]
fn a_loop_of_calls_ends_in_the_step_budget() {
    let calls = loop_calls();
    let source = format!(
        "var text; for (var i = 0; i < {calls}; i++) text = (1e-300).toPrecision(100); text.length;"
    );
    let mut limits = limits();
    limits.steps_per_entry = 400_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, &source),
        Outcome::Normal("106".to_string())
    );
    let mut limits = support::host::limits();
    limits.steps_per_entry = 5_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = support::host::realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, &source),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "1;"),
        Outcome::Normal("1".to_string())
    );
}

/// The host charges what is free of `budget` past `room` (`charge_host_bytes`), as the test of
/// HOST-188 does, so that `memory_bytes - heap_bytes()` reads exactly `room`; the charge.
fn leave_room(
    agent: &mut swell::Agent<TestHost>,
    realm: &swell::Realm,
    budget: u64,
    room: u64,
) -> swell::HostBytes {
    let free = budget - agent.heap_bytes().unwrap();
    let charge = agent
        .with_cx(realm, |cx| cx.charge_host_bytes(free - room))
        .unwrap()
        .unwrap();
    assert_eq!(budget - agent.heap_bytes().unwrap(), room, "the room left");
    charge
}

/// The parsed `script` evaluated after a collection with `room` bytes left: what it answered
/// and its steps. The host gives its charge back afterwards.
fn with_room(
    agent: &mut swell::Agent<TestHost>,
    realm: &swell::Realm,
    script: &swell::Script,
    room: u64,
) -> (Outcome, u64) {
    let budget = limits().memory_bytes;
    assert!(matches!(agent.collect(), Ok(swell::Outcome::Normal(_))));
    let held = leave_room(agent, realm, budget, room);
    let outcome = agent.evaluate_script(script).unwrap();
    let steps = agent.last_entry_steps().unwrap();
    let outcome = describe(agent, realm, outcome);
    agent.credit_host_bytes(held).unwrap();
    (outcome, steps)
}

/// The temporary of 2,048 bytes is charged before `swell-num` is called. The entry of the
/// parsed script `(1e-300).toPrecision(100)` holds some bytes of its own when it calls the
/// method (`held`, read here as the least room whose entry is charged the 106 steps of the text,
/// less 2,048); with a room of `held` + 2,047 bytes, set as the test of HOST-188 sets one, it
/// ends in `Terminated(MemoryBudget)` before the steps of the text are charged, so before the
/// text is made; with `held` + 2,048 it passes the charge and is charged those 106 steps. With a
/// room of 0 it ends in the memory budget too, and with 64 KiB it answers the text. The agent
/// runs the next entry.
#[test]
fn a_room_below_the_temporary_ends_in_the_memory_budget() {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    let script = parse(&mut agent, &realm, "(1e-300).toPrecision(100);");
    // A first round with a room of 1 MiB grows the arenas once; the rooms below are read after
    // it. At some sizes of the realm the first object the script makes grows an arena by more
    // than 64 KiB (at 128 padding functions with the set methods of `story:set-methods`).
    with_room(&mut agent, &realm, &script, 1 << 20);
    let (ample, ample_steps) = with_room(&mut agent, &realm, &script, 64 * 1024);
    let Outcome::Normal(text) = &ample else {
        panic!("{ample:?}");
    };
    assert_eq!(text.len(), 2 + 106, "{text}");
    let (stopped, stopped_steps) = with_room(&mut agent, &realm, &script, 0);
    assert_eq!(stopped, Outcome::Terminated(Termination::MemoryBudget));
    assert!(
        stopped_steps + 106 <= ample_steps,
        "{stopped_steps} and {ample_steps}"
    );
    // An entry that passed the charge was charged the 106 steps of the text, so its steps are
    // above `ample_steps - 106`; one that stopped at or before the charge is at or below
    // `ample_steps - 106` less the steps after the text. The least room that passes: a binary
    // search, the outcome being monotone in the room.
    let passed = |steps: u64| steps + 106 > ample_steps;
    let (mut low, mut high) = (0u64, 64 * 1024);
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        if passed(with_room(&mut agent, &realm, &script, middle).1) {
            high = middle;
        } else {
            low = middle;
        }
    }
    assert!(high >= 2_048, "{high}");
    let held = high - 2_048;
    let (below, below_steps) = with_room(&mut agent, &realm, &script, held + 2_047);
    let (at, at_steps) = with_room(&mut agent, &realm, &script, held + 2_048);
    println!(
        "held at the call {held} bytes; room 0: {stopped_steps} steps; room {}: {below:?} at \
         {below_steps} steps; room {}: {at:?} at {at_steps} steps; room 65536: {ample_steps} steps",
        held + 2_047,
        held + 2_048
    );
    assert_eq!(below, Outcome::Terminated(Termination::MemoryBudget));
    assert!(!passed(below_steps), "{below_steps}");
    // Past the charge the 106 steps of the text are spent, and the String then fits the room
    // the credit gave back; the entry may also end after it.
    assert!(at_steps >= below_steps + 106, "{at:?} at {at_steps} steps");
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "1;"),
        Outcome::Normal("1".to_string())
    );
}

/// `toLocaleString` gives the text of `toString()` of the same Number
/// (`docs/design/engine.md`, decision 6, "The text of toLocale*String"), with its arguments
/// read by nothing.
#[test]
fn to_locale_string_is_the_text_of_to_string() {
    answers(
        "[1234.5, -0, NaN, 1e21, -1.5e-7, Infinity].map(function (x) {\n\
           return x.toLocaleString() + (x.toLocaleString() === x.toString() ? '' : ' differs');\n\
         }).join() + ';' + (1.5).toLocaleString('de-DE', { useGrouping: true });",
        "1234.5,0,NaN,1e+21,-1.5e-7,Infinity;1.5",
    );
}

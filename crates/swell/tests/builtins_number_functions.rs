//! `Number.isFinite` (`#sec-number.isfinite`), `Number.isInteger` (`#sec-number.isinteger`),
//! `Number.isNaN` (`#sec-number.isnan`), `Number.isSafeInteger` (`#sec-number.issafeinteger`),
//! `parseFloat` (`#sec-parsefloat-string`) and `parseInt` (`#sec-parseint-string-radix`), and
//! `Number.parseFloat` and `Number.parseInt` (`#sec-number.parsefloat`,
//! `#sec-number.parseint`), the same function objects, `story:number-functions`: their lengths,
//! names and descriptors, the predicates over a table with no conversion, the values of the two
//! global functions through the engine, the order of their conversions, and the charges: a step
//! per code unit past 32 of the String read, before it is read, and the ASCII buffer of the
//! digits charged to the memory budget before it is made and credited after. Run also with the
//! feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::cell::Cell;

use support::host::{TestHost, agent_with, limits, realm};
use support::script::{Outcome, define_function, describe, eval_text, parse, run};
use swell::{AgentState, Completion, Cx, HostCall, InterruptHandle, Termination, Value};

/// `source` answers the String `expected` with the test limits.
fn answers(source: &str, expected: &str) {
    assert_eq!(
        run(limits(), source),
        Outcome::Normal(format!("\"{expected}\"")),
        "{source}"
    );
}

/// The six functions with their lengths (1 for each predicate and `parseFloat`, 2 for
/// `parseInt`) and names; each property { [[Writable]]: true, [[Enumerable]]: false,
/// [[Configurable]]: true } (clause 18); `Number.parseFloat === parseFloat` and
/// `Number.parseInt === parseInt` (21.1.2.12 and 21.1.2.13), on the constructor as on the
/// global object; none is a constructor.
#[test]
fn the_six_functions_have_their_lengths_names_and_descriptors() {
    answers(
        "function describe(object, name) {\n\
           var d = Object.getOwnPropertyDescriptor(object, name);\n\
           var constructs = true;\n\
           try { new d.value('1'); } catch (e) { constructs = !(e instanceof TypeError); }\n\
           return [name, typeof d.value, d.value.length, d.value.name, d.writable, d.enumerable,\n\
             d.configurable, constructs].join(' ');\n\
         }\n\
         var global = this;\n\
         ['isFinite', 'isInteger', 'isNaN', 'isSafeInteger', 'parseFloat', 'parseInt']\n\
           .map(function (name) { return describe(Number, name); })\n\
           .concat(['parseFloat', 'parseInt'].map(function (name) { return describe(global, name); }))\n\
           .join(';') + ';' + (Number.parseFloat === parseFloat) + ' ' + (Number.parseInt === parseInt)\n\
           + ' ' + (Number.isNaN !== isNaN) + ' ' + (Number.isFinite !== isFinite);",
        "isFinite function 1 isFinite true false true false;\
         isInteger function 1 isInteger true false true false;\
         isNaN function 1 isNaN true false true false;\
         isSafeInteger function 1 isSafeInteger true false true false;\
         parseFloat function 1 parseFloat true false true false;\
         parseInt function 2 parseInt true false true false;\
         parseFloat function 1 parseFloat true false true false;\
         parseInt function 2 parseInt true false true false;true true true true",
    );
}

/// The four predicates over 20 values, one digit per value (1 for true): a value that is not a
/// Number answers false at once, with no conversion, so the `valueOf` and `toString` of the
/// poisoned object are never called; -0, 2^53 - 1 and -(2^53 - 1) are safe integers, ±2^53
/// integers that are not safe, 1.5 and 2^-1074 no integers.
#[test]
fn the_four_predicates_over_a_table() {
    answers(
        "var log = [];\n\
         var poisoned = {\n\
           valueOf: function () { log.push('valueOf'); throw new EvalError(); },\n\
           toString: function () { log.push('toString'); throw new EvalError(); }\n\
         };\n\
         var values = [NaN, Infinity, -Infinity, -0, 0, 9007199254740991, 9007199254740992,\n\
           -9007199254740991, -9007199254740992, 1.5, -1.5, 5e-324, 1.7976931348623157e308,\n\
           '1', true, null, undefined, poisoned, new Number(1), Symbol()];\n\
         ['isFinite', 'isInteger', 'isNaN', 'isSafeInteger'].map(function (name) {\n\
           return name + ' ' + values.map(function (v) { return Number[name](v) ? 1 : 0; }).join('');\n\
         }).join(';') + ';' + Number.isNaN() + Number.isInteger() + ';' + log.length;",
        "isFinite 00011111111110000000;isInteger 00011111100010000000;\
         isNaN 10000000000000000000;isSafeInteger 00011101000000000000;falsefalse;0",
    );
}

/// `parseInt` and `parseFloat` through the engine: ToString of the argument (null, a Number, an
/// object), ToInt32 of the radix (a String, NaN, 2^32 + 2 and -2^32 + 2 both 2), a radix of 1
/// that is NaN, -0 kept, the white space of `StringToNumber`, `Number.parseInt` the same
/// function, and a Symbol argument a TypeError of ToString.
#[test]
fn parse_int_and_parse_float_through_the_engine() {
    answers(
        "function type(f) { try { f(); return 'answered'; } catch (e) { return e.name; } }\n\
         [parseInt('  0x1F'), parseInt('11', 2), parseInt('11', 4294967298),\n\
          parseInt('11', -4294967294), parseInt('11', '16'), parseInt('11', NaN),\n\
          parseInt('11', 1), parseInt(null, 36), parseInt(1e21), 1 / parseInt('-0'),\n\
          parseInt(undefined), Number.parseInt('12px'), parseInt('\\u00a0\\ufeff-7'),\n\
          parseFloat('  .5e1x'), 1 / parseFloat(-0), 1 / parseFloat('-0'),\n\
          parseFloat('\\u180e1'), parseFloat({ toString: function () { return '7.5'; } }),\n\
          parseFloat('Infinityx'), parseFloat('-Infinity'), parseFloat('0x10'),\n\
          parseFloat('1e1000'), Number.parseFloat('\\u2029 3.25'),\n\
          type(function () { parseInt(Symbol()); }), type(function () { parseFloat(Symbol()); }),\n\
          type(function () { parseInt('1', Symbol()); })].join();",
        "31,3,3,3,17,11,NaN,1112745,1,-Infinity,NaN,12,-7,5,Infinity,-Infinity,NaN,7.5,\
         Infinity,-Infinity,0,Infinity,3.25,TypeError,TypeError,TypeError",
    );
}

/// `parseInt` converts its string with ToString before its radix with ToInt32 (19.2.5, steps 1
/// and 2), also when the radix then makes the answer NaN; a throw of ToString converts no
/// radix; `parseFloat` converts its argument once.
#[test]
fn parse_int_converts_the_string_before_the_radix() {
    answers(
        "var log = [];\n\
         var s = { toString: function () { log.push('string'); return '11'; } };\n\
         function radix(value) {\n\
           return { valueOf: function () { log.push('radix ' + value); return value; } };\n\
         }\n\
         log.push(parseInt(s, radix(2)));\n\
         log.push(parseInt(s, radix(37)));\n\
         try {\n\
           parseInt({ toString: function () { log.push('thrower'); throw new EvalError(); } },\n\
             radix(16));\n\
         } catch (e) { log.push(e.name); }\n\
         log.push(parseFloat(s));\n\
         log.join();",
        "string,radix 2,3,string,radix 37,NaN,thrower,EvalError,string,11",
    );
}

/// The code units of the long Strings of the tests below.
const LONG: usize = 100_000;

/// A String of 100,000 digits read by `parseInt` or `parseFloat` under 2,000 steps per entry
/// ends in `Terminated(StepBudget)`: 99,968 steps for its code units past 32 are charged before
/// they are read. The same literal alone, its length read, answers under the same budget, so the
/// steps are the call's. The entry spends no more than its budget, and the agent runs the next
/// entry.
#[test]
fn a_long_string_ends_in_the_step_budget() {
    let mut budget = limits();
    budget.steps_per_entry = 2_000;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    let digits = "1".repeat(LONG);
    assert_eq!(
        eval_text(&mut agent, &realm, &format!("'{digits}'.length;")),
        Outcome::Normal(LONG.to_string())
    );
    for call in ["parseInt", "parseFloat", "Number.parseInt"] {
        assert_eq!(
            eval_text(&mut agent, &realm, &format!("{call}('{digits}');")),
            Outcome::Terminated(Termination::StepBudget),
            "{call}"
        );
        assert!(agent.last_entry_steps().unwrap() <= 2_000);
        assert_eq!(agent.state(), AgentState::Alive);
        assert_eq!(
            eval_text(&mut agent, &realm, &format!("{call}('12.5');")),
            Outcome::Normal(if call == "parseFloat" { "12.5" } else { "12" }.to_string()),
            "{call}"
        );
    }
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

/// An interrupt raised by a host function from the `toString` of the argument ends the call of
/// `parseInt` and of `parseFloat` as `Terminated(Interrupted)`, with a String of 40 code units
/// returned (past the 32 that are read without a step) and of 2; the agent runs the next entry.
#[test]
fn an_interrupt_from_to_string_ends_the_call() {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    let forty = "1".repeat(40);
    for call in ["parseInt", "parseFloat"] {
        for text in [forty.as_str(), "12"] {
            HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
            assert_eq!(
                eval_text(
                    &mut agent,
                    &realm,
                    &format!(
                        "{call}({{ toString: function () {{ interruptNow(); return '{text}'; }} }});"
                    )
                ),
                Outcome::Terminated(Termination::Interrupted),
                "{call} of {text}"
            );
            assert_eq!(agent.state(), AgentState::Alive);
            assert_eq!(
                eval_text(&mut agent, &realm, &format!("{call}('12');")),
                Outcome::Normal("12".to_string())
            );
        }
    }
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

/// The code units of the String of [`a_million_code_units_end_in_the_memory_budget`].
const MILLION: usize = 1_000_000;

/// A String of 1,000,000 digits, made by an entry before: `parseFloat` and `parseInt` of it with
/// 100 KiB of room end in `Terminated(MemoryBudget)`, since the buffer of its digits, charged
/// at the String's length before it is made, does not fit; with 2 MiB of room each answers
/// Infinity. The bytes the heap is charged after a call, read in a second round with a
/// collection after each (the arenas keep their capacity after the first), equal those before
/// it: the buffer is credited. The agent runs the next entry.
#[test]
fn a_million_code_units_end_in_the_memory_budget() {
    let mut budget = limits();
    budget.steps_per_entry = 4_000_000;
    let memory = budget.memory_bytes;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!("var s = '{}'; s.length;", "1".repeat(MILLION))
        ),
        Outcome::Normal(MILLION.to_string())
    );
    for call in ["parseFloat", "parseInt"] {
        let script = parse(&mut agent, &realm, &format!("{call}(s);"));
        let mut readings = Vec::new();
        for room in [100 * 1024, 2 * 1024 * 1024, 2 * 1024 * 1024] {
            assert!(matches!(agent.collect(), Ok(swell::Outcome::Normal(_))));
            let held = leave_room(&mut agent, &realm, memory, room);
            let outcome = agent.evaluate_script(&script).unwrap();
            let outcome = describe(&mut agent, &realm, outcome);
            agent.credit_host_bytes(held).unwrap();
            let expected = if room == 100 * 1024 {
                Outcome::Terminated(Termination::MemoryBudget)
            } else {
                Outcome::Normal("Infinity".to_string())
            };
            assert_eq!(outcome, expected, "{call} with {room} bytes of room");
            assert_eq!(agent.state(), AgentState::Alive);
            let before = agent.heap_bytes().unwrap();
            assert!(matches!(agent.collect(), Ok(swell::Outcome::Normal(_))));
            readings.push((room, before, agent.heap_bytes().unwrap()));
        }
        println!("{call}: room, heap_bytes after the call and after a collection: {readings:?}");
        assert_eq!(readings[2].2, readings[1].2, "{call}: the second round");
    }
    assert_eq!(
        eval_text(&mut agent, &realm, "parseInt('7');"),
        Outcome::Normal("7".to_string())
    );
}

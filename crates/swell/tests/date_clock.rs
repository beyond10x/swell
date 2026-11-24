//! The clock half of `Date` (ECMA-262 21.4, `#sec-date-objects`;
//! `story:date-object-clock-and-time-value`): `%Date%` with `Date.now`, `new Date()` and
//! `new Date(value)` over the host clock (HOST-125, HOST-126), the time value of a Date
//! (`getTime`, `valueOf`, `setTime`), the step charged before each call of the clock hook, and
//! the two `Date.now()` reads of the busy wait of the first consumer.
//!
//! `story:date-text-formats` rewrote the rows and tests that stated an interim: the row of a
//! String in `new_date_over_a_table_of_values` (the row `new Date(1, 2)` was rewritten by
//! `story:date-object-constructor-now-utc-and-getters`) and
//! `date_called_as_a_function_answers_the_text_of_to_string`; the test of the interim bodies
//! went with the last of them, `Date.parse`.
#![cfg(test)]

use crate::support;

use std::cell::Cell;

use support::host::{TestHost, agent, agent_with, global, limits, object, realm};
use support::script::{Outcome, define_function, describe, eval, eval_text};
use swell::{
    Agent, Completion, Cx, HostCall, InterruptHandle, Outcome as HostOutcome, Realm, Termination,
    Value,
};

/// The name `TestHost` records for a call of the clock hook.
const CLOCK: &str = "host_system_utc_epoch_nanoseconds";

/// The number of calls of the clock hook `agent`'s host recorded.
fn clock_calls(agent: &Agent<TestHost>) -> usize {
    agent
        .host()
        .calls
        .iter()
        .filter(|call| call.as_str() == CLOCK)
        .count()
}

/// The text of a normal completion whose value is a String, as `show` writes it.
fn quoted(text: &str) -> Outcome {
    Outcome::Normal(format!("\"{text}\""))
}

/// The text of a normal completion whose value is a Number or a Boolean.
fn normal(text: &str) -> Outcome {
    Outcome::Normal(text.to_string())
}

/// HOST-125: `Date.now()` calls `host_system_utc_epoch_nanoseconds` once with the current
/// realm's global object and answers the nanoseconds divided by 10^6, rounded toward negative
/// infinity: -1 gives -1 and 1,999,999 gives 1. `new Date()` reads the clock the same way.
#[test]
fn host_125_date_now_calls_the_clock_hook_once_and_floors_to_milliseconds() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    agent.host_mut().now = -1;
    assert_eq!(eval_text(&mut agent, &realm, "Date.now();"), normal("-1"));
    assert_eq!(agent.host().calls, [CLOCK]);
    let global_root = global(&mut agent, &realm);
    let global = agent
        .with_cx(&realm, |cx| object(cx, &global_root))
        .unwrap();
    assert_eq!(agent.host().clock_globals, [global]);

    agent.host_mut().now = 1_999_999;
    assert_eq!(eval_text(&mut agent, &realm, "Date.now();"), normal("1"));
    agent.host_mut().now = 1_999_999;
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date().getTime();"),
        normal("1")
    );
    assert_eq!(clock_calls(&agent), 3);
    assert_eq!(agent.host().clock_globals, [global, global, global]);
}

/// HOST-126: an answer of the clock outside ±8.64 × 10^21 nanoseconds is clamped to that range:
/// `i128::MAX` makes `Date.now()` answer 8.64 × 10^15 and `i128::MIN` -8.64 × 10^15.
#[test]
fn host_126_a_clock_answer_out_of_range_is_clamped() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    agent.host_mut().now = i128::MAX;
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.now();"),
        normal("8640000000000000")
    );
    agent.host_mut().now = i128::MIN;
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.now();"),
        normal("-8640000000000000")
    );
    agent.host_mut().now = 8_640_000_000_000_000_999_999;
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date().getTime();"),
        normal("8640000000000000")
    );
    assert_eq!(clock_calls(&agent), 3);
}

/// A host function that does nothing: the call `Date.idle()` spends what `Date.now()` spends
/// without the clock.
fn idle(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Undefined)
}

/// A host function that asks for more steps than any budget has: the entry ends in
/// `Terminated(StepBudget)` with the steps it had spent when the call began, the step of the
/// call included.
fn stop(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.charge_steps(u64::MAX)?;
    Ok(Value::Undefined)
}

/// The outcome of `source` in `realm` and the steps its entry spent, read before the outcome
/// is described (a read of a value through `with_cx` begins an entry of its own).
fn steps_of(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> (Outcome, u64) {
    let outcome = eval(agent, realm, source);
    let steps = agent.last_entry_steps().unwrap();
    (describe(agent, realm, outcome), steps)
}

/// The clock hook is charged one step before it is called (HOST-211's rule, for the clock):
/// `last_entry_steps()` after `Date.now()` is one more than after `Date.idle()`, a host function
/// of the same call that does nothing, and with `steps_per_entry` at the steps spent when the
/// call began the entry ends in `Terminated(StepBudget)` before the hook is called.
#[test]
fn a_clock_hook_call_is_charged_one_step_before_it_is_made() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "idle", 0, idle, false);
    define_function(&mut agent, &realm, "stop", 0, stop, false);
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.idle = idle; Date.stop = stop; 0;"),
        normal("0")
    );
    let (outcome, without) = steps_of(&mut agent, &realm, "Date.idle();");
    assert_eq!(outcome, normal("undefined"));
    let (outcome, with) = steps_of(&mut agent, &realm, "Date.now();");
    assert_eq!(outcome, normal("0"));
    assert_eq!(
        with,
        without + 1,
        "Date.now() {with}, Date.idle() {without}"
    );
    let (outcome, begun) = steps_of(&mut agent, &realm, "Date.stop();");
    assert_eq!(outcome, Outcome::Terminated(Termination::StepBudget));
    println!("steps: Date.idle() {without}, Date.now() {with}, at the call {begun}");

    // With the steps spent when the call began, the step of the hook is past the budget.
    let mut tight = limits();
    tight.steps_per_entry = begun;
    let mut agent = agent_with(tight).unwrap();
    let realm = support::host::realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.now();"),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert!(agent.host().calls.is_empty(), "{:?}", agent.host().calls);
    assert_eq!(agent.last_entry_steps(), Ok(begun));

    // One step more and the hook is called.
    let mut room = limits();
    room.steps_per_entry = begun + 1;
    let mut agent = agent_with(room).unwrap();
    let realm = support::host::realm(&mut agent);
    let _ = eval_text(&mut agent, &realm, "Date.now();");
    assert_eq!(agent.host().calls, [CLOCK]);
}

/// `new Date(value)` (`#sec-date-constructor`, `numberOfArgs` = 1) over a table: a Date is read
/// by its [[DateValue]] with no call of its methods; another value goes through ToPrimitive
/// with no hint and ToNumber, then TimeClip; a String is parsed as `Date.parse` parses it
/// (`tests/date_text.rs` holds the forms); two or more arguments are the components, whose
/// coercions run and whose time value `new Date(1, 2)` is 1901-03-01T00:00:00.000Z in the
/// zone "UTC" (`tests/date_zone.rs` holds the rest). None of them reads the clock. The object
/// is made after the coercion, from `new.target`'s realm when its `prototype` is not an
/// object.
#[test]
fn new_date_over_a_table_of_values() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let rows: [(&str, Outcome); 14] = [
        ("new Date(1234.9).getTime();", normal("1234")),
        ("new Date(-1234.9).getTime();", normal("-1234")),
        (
            "var log = ''; var d = new Date(5); \
             d.valueOf = function () { log += 'v'; return 7; }; \
             d.toString = function () { log += 's'; return '7'; }; \
             new Date(d).getTime() + ':' + log;",
            quoted("5:"),
        ),
        ("new Date(8.64e15).getTime();", normal("8640000000000000")),
        ("new Date(-8.64e15).getTime();", normal("-8640000000000000")),
        ("new Date(8.64e15 + 1).getTime();", normal("NaN")),
        ("1 / new Date(-0).getTime();", normal("Infinity")),
        ("new Date(NaN).getTime();", normal("NaN")),
        (
            "new Date(\"1970-01-01T00:00:00.000Z\").getTime();",
            normal("0"),
        ),
        ("new Date(\"garbage\").getTime();", normal("NaN")),
        (
            "new Date({ valueOf: function () { throw new RangeError('poisoned'); } });",
            Outcome::Throw("RangeError".to_string()),
        ),
        (
            "new Date({ [Symbol.toPrimitive]: function (hint) { \
               return hint === 'default' ? 4 : 5; } }).getTime();",
            normal("4"),
        ),
        ("new Date(1, 2).getTime();", normal("-2172355200000")),
        (
            "var made = new Date(0); \
             (Object.getPrototypeOf(made) === Date.prototype) + ',' + typeof made;",
            quoted("true,object"),
        ),
    ];
    for (source, expected) in rows {
        assert_eq!(eval_text(&mut agent, &realm, source), expected, "{source}");
    }
    assert_eq!(clock_calls(&agent), 0, "{:?}", agent.host().calls);

    // `new.target` of another realm whose `prototype` is not an object: the Date takes that
    // realm's %Date.prototype%, read after the argument's `valueOf` ran.
    let other = support::host::realm(&mut agent);
    define_function(&mut agent, &other, "Maker", 0, idle, true);
    assert_eq!(
        eval_text(
            &mut agent,
            &other,
            "var log = ''; \
             Object.defineProperty(Maker, 'prototype', \
               { get: function () { log += 'p'; return undefined; } }); \
             var arg = { valueOf: function () { log += 'v'; return 3; } }; 0;"
        ),
        normal("0")
    );
    let here = global(&mut agent, &realm);
    let there = global(&mut agent, &other);
    agent
        .with_cx(&realm, |cx| {
            let here = object(cx, &here);
            let there = object(cx, &there);
            let date = support::host::get(cx, here, "Date");
            let maker = support::host::get(cx, there, "Maker");
            let arg = support::host::get(cx, there, "arg");
            let made = cx.construct(date, &[arg], Some(maker)).unwrap().unwrap();
            let key = support::host::key(cx, "made");
            cx.set(there, key, made, true).unwrap().unwrap();
        })
        .unwrap();
    assert_eq!(
        eval_text(
            &mut agent,
            &other,
            "(Object.getPrototypeOf(made) === Date.prototype) + ',' + made.getTime() + ',' + log;"
        ),
        quoted("true,3,vp")
    );
}

/// `Date ( ...values )` called as a function (`#sec-date-constructor`, step 1): the clock is
/// read once and the arguments are not read; the text is ToDateString of the clock's value,
/// the text of `toString` for it, with 3 hook calls: the clock, the identifier and the offset.
#[test]
fn date_called_as_a_function_answers_the_text_of_to_string() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    agent.host_mut().now = 1_234_567_890_123_000_000;
    for (source, calls) in [
        ("Date();", 1),
        ("Date(0);", 2),
        ("Date(1, 2);", 3),
        (
            "Date({ valueOf: function () { throw new RangeError('poisoned'); } });",
            4,
        ),
    ] {
        let before = agent.host().calls.len();
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            quoted("Fri Feb 13 2009 23:31:30 GMT+0000"),
            "{source}"
        );
        assert_eq!(clock_calls(&agent), calls, "{source}");
        let made: Vec<&str> = agent.host().calls[before..]
            .iter()
            .map(|call| call.split(' ').next().unwrap())
            .collect();
        assert_eq!(
            made,
            [
                CLOCK,
                "system_time_zone_identifier",
                "get_named_time_zone_offset_nanoseconds"
            ],
            "{source}"
        );
    }
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "typeof Date() + ',' + (Date() === new Date(1234567890123).toString());"
        ),
        quoted("string,true")
    );
}

/// `Date.prototype.setTime ( time )` (`#sec-date.prototype.settime`): the [[DateValue]] check
/// first, then ToNumber, TimeClip, the store and the value returned; a `this` that is not a
/// Date throws a TypeError before `time` is read.
#[test]
fn set_time_checks_this_then_clips_stores_and_returns_the_time_value() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let rows: [(&str, Outcome); 6] = [
        (
            "var d = new Date(0); var r = d.setTime(1.9); r + ',' + d.getTime();",
            quoted("1,1"),
        ),
        (
            "var d = new Date(0); var r = d.setTime(8.64e15 + 1); r + ',' + d.getTime();",
            quoted("NaN,NaN"),
        ),
        (
            "var d = new Date(0); var r = d.setTime(-0); (1 / r) + ',' + (1 / d.valueOf());",
            quoted("Infinity,Infinity"),
        ),
        ("new Date(7).setTime();", normal("NaN")),
        (
            "var log = ''; \
             var time = { valueOf: function () { log += 'v'; return 1; } }; \
             try { Date.prototype.setTime.call({}, time); } catch (e) { log += e.name; } log;",
            quoted("TypeError"),
        ),
        (
            "Date.prototype.setTime.call(Date.prototype, 1);",
            Outcome::Throw("TypeError".to_string()),
        ),
    ];
    for (source, expected) in rows {
        assert_eq!(eval_text(&mut agent, &realm, source), expected, "{source}");
    }
}

/// `getTime` and `valueOf` (`#sec-date.prototype.gettime`, `#sec-date.prototype.valueof`)
/// throw a TypeError for a `this` without [[DateValue]]: an ordinary object, a Number, and
/// `%Date.prototype%`, which is not a Date (`#sec-properties-of-the-date-prototype-object`).
#[test]
fn get_time_and_value_of_throw_for_a_this_that_is_not_a_date() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    for source in [
        "Date.prototype.getTime.call({});",
        "Date.prototype.getTime();",
        "Date.prototype.getTime.call(5);",
        "Date.prototype.valueOf.call({});",
        "Date.prototype.valueOf();",
        "Date.prototype.valueOf.call(new Number(5));",
    ] {
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Throw("TypeError".to_string()),
            "{source}"
        );
    }
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date(42).valueOf();"),
        normal("42")
    );
}

/// The lengths, names and property attributes of `Date` (`#sec-properties-of-the-date-constructor`:
/// `length` 7, `prototype` not writable, enumerable or configurable) and of every function of
/// this story and its interim functions (clause 18: writable, not enumerable, configurable;
/// `length` and `name` not writable, not enumerable, configurable).
#[test]
fn lengths_names_and_descriptors_of_date_and_its_functions() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let source = "\
        function attributes(d) { return '' + d.writable + d.enumerable + d.configurable; } \
        function row(object, key) { \
          var d = Object.getOwnPropertyDescriptor(object, key); \
          var f = d.value; \
          return key + ' ' + typeof f + ' ' + attributes(d) + ' ' + f.length + ' ' + f.name + ' ' \
            + attributes(Object.getOwnPropertyDescriptor(f, 'length')) + ' ' \
            + attributes(Object.getOwnPropertyDescriptor(f, 'name')); \
        } \
        var rows = [row(this, 'Date'), \
          attributes(Object.getOwnPropertyDescriptor(Date, 'prototype')), \
          String(Object.getPrototypeOf(Date) === Function.prototype), \
          String(Object.getPrototypeOf(Date.prototype) === Object.prototype), \
          String(Date.prototype.constructor === Date), \
          attributes(Object.getOwnPropertyDescriptor(Date.prototype, 'constructor'))]; \
        ['now', 'parse', 'UTC'].forEach(function (k) { rows.push(row(Date, k)); }); \
        ['getTime', 'setDate', 'setFullYear', 'setHours', 'setMilliseconds', 'setMinutes', \
         'setMonth', 'setSeconds', 'setTime', 'setUTCDate', 'setUTCFullYear', 'setUTCHours', \
         'setUTCMilliseconds', 'setUTCMinutes', 'setUTCMonth', 'setUTCSeconds', 'valueOf'] \
          .forEach(function (k) { rows.push(row(Date.prototype, k)); }); \
        try { new Date.now(); rows.push('now constructs'); } catch (e) { rows.push(e.name); } \
        rows.join('|');";
    let functions = [
        ("now", 0),
        ("parse", 1),
        ("UTC", 7),
        ("getTime", 0),
        ("setDate", 1),
        ("setFullYear", 3),
        ("setHours", 4),
        ("setMilliseconds", 1),
        ("setMinutes", 3),
        ("setMonth", 2),
        ("setSeconds", 2),
        ("setTime", 1),
        ("setUTCDate", 1),
        ("setUTCFullYear", 3),
        ("setUTCHours", 4),
        ("setUTCMilliseconds", 1),
        ("setUTCMinutes", 3),
        ("setUTCMonth", 2),
        ("setUTCSeconds", 2),
        ("valueOf", 0),
    ];
    let mut expected = vec![
        "Date function truefalsetrue 7 Date falsefalsetrue falsefalsetrue".to_string(),
        "falsefalsefalse".to_string(),
        "true".to_string(),
        "true".to_string(),
        "true".to_string(),
        "truefalsetrue".to_string(),
    ];
    for (name, length) in functions {
        expected.push(format!(
            "{name} function truefalsetrue {length} {name} falsefalsetrue falsefalsetrue"
        ));
    }
    expected.push("TypeError".to_string());
    assert_eq!(
        eval_text(&mut agent, &realm, source),
        quoted(&expected.join("|"))
    );
}

/// `Object.prototype.toString` (`#sec-object.prototype.tostring`, step 12) of a Date is
/// `[object Date]`; of `%Date.prototype%`, which has no [[DateValue]], `[object Object]`.
#[test]
fn object_prototype_to_string_of_a_date_is_object_date() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "Object.prototype.toString.call(new Date(0)) + ',' \
             + Object.prototype.toString.call(Date.prototype);"
        ),
        quoted("[object Date],[object Object]")
    );
}

/// `%Date%` and `%Date.prototype%` are traced by the realm's intrinsics: with the global `Date`
/// deleted, the intrinsics are the only path to them, and after two collections (and one at
/// every safe point with `collect-every-safe-point`) a Date made with a `new.target` of that
/// realm whose `prototype` is not an object gets that realm's `%Date.prototype%`
/// (`#sec-getprototypefromconstructor`), whose `constructor.now` answers the clock.
#[test]
fn a_collected_realm_still_has_date_and_its_prototype() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "Maker", 0, idle, true);
    assert_eq!(
        eval_text(&mut agent, &realm, "delete this.Date; typeof Date;"),
        quoted("undefined")
    );
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    agent.host_mut().now = 3_000_000;
    let other = support::host::realm(&mut agent);
    let here = global(&mut agent, &realm);
    let there = global(&mut agent, &other);
    agent
        .with_cx(&other, |cx| {
            let here = object(cx, &here);
            let there = object(cx, &there);
            let date = support::host::get(cx, there, "Date");
            let maker = support::host::get(cx, here, "Maker");
            let made = cx
                .construct(date, &[Value::Number(0.0)], Some(maker))
                .unwrap()
                .unwrap();
            let key = support::host::key(cx, "made");
            cx.set(there, key, made, true).unwrap().unwrap();
        })
        .unwrap();
    assert_eq!(
        eval_text(
            &mut agent,
            &other,
            "var proto = Object.getPrototypeOf(made); \
             (proto !== Date.prototype) + ',' + Object.prototype.toString.call(made) + ',' \
             + made.getTime() + ',' + proto.constructor.now() + ',' + proto.constructor.name;"
        ),
        quoted("true,[object Date],0,3,Date")
    );
}

/// The busy wait of `fake_set_timeout` in surf's `resources/testharness.js`, the two `Date`
/// references of the first consumer: a clock that advances one millisecond per read answers
/// T, T + 1, ..., T + 5, so `end - Date.now()` is 4, 3, 2, 1 and then 0, and the loop runs 4
/// times.
#[test]
fn consumer_date_reads_of_surf_resolve_as_the_standard_says() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    agent.host_mut().now = 1_700_000_000_000_000_000;
    agent.host_mut().clock_step_ns = 1_000_000;
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var start = Date.now(); var end = start + 5; var n = 0; \
             while ((end - Date.now()) > 0) { n++; } n;"
        ),
        normal("4")
    );
    assert_eq!(clock_calls(&agent), 6);
}

/// The step budget of [`a_clock_that_does_not_advance_ends_the_busy_wait_at_the_step_budget`]:
/// 10,000 steps, the full count, which the lane `slow` (`SWELL_FULL_COUNTS=1`) runs in both
/// modes; a quarter, 2,500, in the gate with the collector at every safe point, where every
/// step collects (1.83 s of wall time at the full count there, against 0.017 s in the default
/// mode, on the pool's build node).
fn busy_wait_steps() -> u64 {
    let collect_gate = cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1");
    if collect_gate { 2_500 } else { 10_000 }
}

/// The busy wait over a clock that does not advance ends at the step budget, as any loop of
/// script code does: `Terminated(StepBudget)` at 10,000 steps (a quarter in the collect gate,
/// [`busy_wait_steps`]), with the hook called fewer times than that.
#[test]
fn a_clock_that_does_not_advance_ends_the_busy_wait_at_the_step_budget() {
    let steps = busy_wait_steps();
    let mut budget = limits();
    budget.steps_per_entry = steps;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    agent.host_mut().now = 1_700_000_000_000_000_000;
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var start = Date.now(); var end = start + 5; var n = 0; \
             while ((end - Date.now()) > 0) { n++; } n;"
        ),
        Outcome::Terminated(Termination::StepBudget)
    );
    let calls = clock_calls(&agent);
    println!("clock calls before the step budget of {steps}: {calls}");
    assert!(
        calls > 1 && u64::try_from(calls).unwrap() < steps,
        "{calls}"
    );
    assert_eq!(agent.last_entry_steps(), Ok(steps));
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

/// The busy wait over a clock that does not advance ends at the interrupt: a host function
/// between two clock reads sets it, and the run ends in `Terminated(Interrupted)` after the
/// second read and before the third.
#[test]
fn an_interrupt_ends_the_busy_wait() {
    let mut budget = limits();
    budget.steps_per_entry = 1 << 62;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var start = Date.now(); var end = start + 5; var n = 0; \
             while ((end - Date.now()) > 0) { n++; interruptNow(); } n;"
        ),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(clock_calls(&agent), 2);
    assert!(!agent.interrupt_handle().is_set());
    assert_eq!(eval_text(&mut agent, &realm, "n;"), normal("1"));
}

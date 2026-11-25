//! The zone half of `Date` (ECMA-262 21.4, `#sec-date-objects`;
//! `story:date-object-constructor-now-utc-and-getters`): the constructor with components
//! (`#sec-date-constructor`, step 5), `Date.UTC` (`#sec-date.utc`) and the 17 getters in local
//! time and in UTC (21.4.4) over the three time zone hooks (`#sec-localtime`, `#sec-utc-t`):
//! HOST-127 (its first half), HOST-128, HOST-209, HOST-210 and HOST-211
//! (`docs/design/host-interface.md`, decision 17).
#![cfg(test)]

use crate::support;

use std::cell::Cell;
use std::time::Instant;

use support::host::{
    ProvidedHost, TestHost, ZONE_EPOCHS, ZONE_IDENTIFIER, ZONE_OFFSET, agent, agent_with, global,
    limits, object, realm, seeds,
};
use support::script::{Outcome, define_function, describe, eval, eval_text};
use swell::{
    Agent, AgentState, Completion, Cx, HostCall, InterruptHandle, IsoDateTime,
    Outcome as HostOutcome, Parsed, Realm, Source, Termination, Value,
};

/// The text of a normal completion whose value is a Number or a Boolean.
fn normal(text: &str) -> Outcome {
    Outcome::Normal(text.to_string())
}

/// The text of a normal completion whose value is a String, as `show` writes it.
fn quoted(text: &str) -> Outcome {
    Outcome::Normal(format!("\"{text}\""))
}

/// The calls of the three time zone hooks `agent`'s host recorded, in order.
fn zone_calls(agent: &Agent<TestHost>) -> Vec<String> {
    agent
        .host()
        .calls
        .iter()
        .filter(|call| {
            call.as_str() == ZONE_IDENTIFIER
                || call.starts_with(ZONE_OFFSET)
                || call.starts_with(ZONE_EPOCHS)
        })
        .cloned()
        .collect()
}

/// The number of recorded calls whose text starts with `prefix`.
fn count_of(agent: &Agent<TestHost>, prefix: &str) -> usize {
    agent
        .host()
        .calls
        .iter()
        .filter(|call| call.starts_with(prefix))
        .count()
}

/// The text `TestHost` records for a call of the offset hook with `id` and `epoch_ns`.
fn offset_call(id: &str, epoch_ns: i128) -> String {
    format!("{ZONE_OFFSET} {id} {epoch_ns}")
}

/// An agent of the test host whose offset hook answers `offset_ns`, and a realm.
fn zone_agent(offset_ns: i128) -> (Agent<TestHost>, Realm) {
    let mut agent = agent().unwrap();
    agent.host_mut().zone_offset_ns = offset_ns;
    let realm = realm(&mut agent);
    (agent, realm)
}

/// The outcome of `source` in `realm` and the steps its entry spent, read before the outcome
/// is described (a read of a value through `with_cx` begins an entry of its own).
fn steps_of(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> (Outcome, u64) {
    let outcome = eval(agent, realm, source);
    let steps = agent.last_entry_steps().unwrap();
    (describe(agent, realm, outcome), steps)
}

/// Nanoseconds in an hour.
const HOUR_NS: i128 = 3_600_000_000_000;

/// The first half of HOST-127: with the provided time zone methods (`ProvidedHost`, a host
/// that answers only the required hooks), `new Date(0).getTimezoneOffset()` answers 0. The
/// second half, the
/// `GMT+0000` of `toString`, is `story:date-text-formats`'s.
#[test]
fn host_127_with_the_provided_zone_get_timezone_offset_answers_zero() {
    let mut agent = Agent::new(ProvidedHost, limits(), seeds()).unwrap();
    let realm = match agent.create_realm(None, None).unwrap() {
        HostOutcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    };
    for (source, expected) in [
        ("new Date(0).getTimezoneOffset();", 0.0),
        ("new Date(0).getHours();", 0.0),
        ("new Date(2016, 6).getTime();", 1_467_331_200_000.0),
    ] {
        let Parsed::Parsed(script) = agent
            .parse_script(&realm, Source::Utf8(source), None)
            .unwrap()
        else {
            panic!("{source} did not parse");
        };
        let HostOutcome::Normal(root) = agent.evaluate_script(&script).unwrap() else {
            panic!("{source} did not complete normally");
        };
        let value = agent.with_cx(&realm, |cx| cx.get(&root).unwrap()).unwrap();
        assert_eq!(value, Value::Number(expected), "{source}");
    }
}

/// HOST-128: a host answering an offset of 3,600 × 10^9 ns makes `new Date(0).getHours()`
/// answer 1, `getUTCHours()` 0 and `getTimezoneOffset()` -60.
#[test]
fn host_128_an_offset_of_one_hour_makes_get_hours_answer_one() {
    let (mut agent, realm) = zone_agent(HOUR_NS);
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date(0).getHours();"),
        normal("1")
    );
    assert_eq!(
        zone_calls(&agent),
        [ZONE_IDENTIFIER.to_string(), offset_call("UTC", 0)]
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date(0).getUTCHours();"),
        normal("0")
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date(0).getTimezoneOffset();"),
        normal("-60")
    );
    assert_eq!(zone_calls(&agent).len(), 4);
}

/// HOST-209: an offset outside ±86,399,999,999,999 ns is clamped to that range: `i128::MAX`
/// makes `new Date(0).getHours()` 23 and `getMilliseconds()` 999, `i128::MIN` makes
/// `new Date(0).getDate()` 31 and `getMilliseconds()` 1.
#[test]
fn host_209_an_offset_out_of_range_is_clamped() {
    let (mut agent, realm) = zone_agent(i128::MAX);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var d = new Date(0); d.getHours() + ',' + d.getMilliseconds();"
        ),
        quoted("23,999")
    );
    let (mut agent, realm) = zone_agent(i128::MIN);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var d = new Date(0); d.getDate() + ',' + d.getMilliseconds();"
        ),
        quoted("31,1")
    );
}

/// A host with no instant for any local time.
fn no_instants(_local: &IsoDateTime) -> Vec<i128> {
    Vec::new()
}

/// HOST-210: a host whose epoch hook answers the empty list for every local time makes
/// `new Date(2020, 0, 1).getTime()` answer NaN after exactly 31 calls of zone hooks (one of the
/// identifier, 30 of the epoch hook, none of the offset hook), and nothing is thrown.
#[test]
fn host_210_a_host_with_no_instant_for_any_local_time_gets_nan_after_31_calls() {
    let mut agent = agent().unwrap();
    agent.host_mut().zone_epochs = Some(no_instants);
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date(2020, 0, 1).getTime();"),
        normal("NaN")
    );
    assert_eq!(zone_calls(&agent).len(), 31, "{:?}", zone_calls(&agent));
    assert_eq!(count_of(&agent, ZONE_IDENTIFIER), 1);
    assert_eq!(count_of(&agent, ZONE_EPOCHS), 30);
    assert_eq!(count_of(&agent, ZONE_OFFSET), 0);
}

/// A host function that asks for more steps than any budget has: the entry ends in
/// `Terminated(StepBudget)` with the steps it had spent when the call began, the step of the
/// call included.
fn stop(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.charge_steps(u64::MAX)?;
    Ok(Value::Undefined)
}

/// HOST-211: every call of a zone hook is charged one step before it is made.
/// `new Date(0).getHours()` calls the identifier and then the offset hook once each, and
/// `last_entry_steps()` after it is 2 more than after `new Date(0).getUTCHours()`. The step at
/// which the getter's call begins is measured with a host function of the same call shape
/// (`new Date(0).stop()`); with `steps_per_entry` one above it the identifier is called and the
/// entry ends in `Terminated(StepBudget)` before the offset hook is called, and at it neither
/// hook is called.
#[test]
fn host_211_every_zone_hook_call_is_charged_one_step_before_it_is_made() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "stop", 0, stop, false);
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.prototype.stop = stop; 0;"),
        normal("0")
    );
    let (outcome, utc) = steps_of(&mut agent, &realm, "new Date(0).getUTCHours();");
    assert_eq!(outcome, normal("0"));
    assert!(zone_calls(&agent).is_empty(), "{:?}", zone_calls(&agent));
    let (outcome, local) = steps_of(&mut agent, &realm, "new Date(0).getHours();");
    assert_eq!(outcome, normal("0"));
    assert_eq!(
        zone_calls(&agent),
        [ZONE_IDENTIFIER.to_string(), offset_call("UTC", 0)]
    );
    assert_eq!(local, utc + 2, "getHours {local}, getUTCHours {utc}");
    let (outcome, begun) = steps_of(&mut agent, &realm, "new Date(0).stop();");
    assert_eq!(outcome, Outcome::Terminated(Termination::StepBudget));
    println!("steps: getUTCHours() {utc}, getHours() {local}, at the call {begun}");

    // The step of the identifier is the first after the call's own, that of the offset hook
    // the second.
    for (budget, calls) in [
        (begun, Vec::new()),
        (begun + 1, vec![ZONE_IDENTIFIER.to_string()]),
    ] {
        let mut tight = limits();
        tight.steps_per_entry = budget;
        let mut agent = agent_with(tight).unwrap();
        let realm = support::host::realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, "new Date(0).getHours();"),
            Outcome::Terminated(Termination::StepBudget),
            "{budget}"
        );
        assert_eq!(zone_calls(&agent), calls, "{budget}");
        assert_eq!(agent.last_entry_steps(), Ok(budget));
    }
    let mut room = limits();
    room.steps_per_entry = begun + 2;
    let mut agent = agent_with(room).unwrap();
    let realm = support::host::realm(&mut agent);
    let outcome = eval_text(&mut agent, &realm, "new Date(0).getHours();");
    assert_eq!(
        zone_calls(&agent),
        [ZONE_IDENTIFIER.to_string(), offset_call("UTC", 0)]
    );
    println!("budget {}: {outcome:?}", begun + 2);
}

/// `new.target` of another realm for the constructor with components: the Date takes that
/// realm's `%Date.prototype%`, read after the coercions of every argument.
fn new_target_of_another_realm(agent: &mut Agent<TestHost>, realm: &Realm) {
    let other = support::host::realm(agent);
    define_function(agent, &other, "Maker", 0, support::script::nothing, true);
    assert_eq!(
        eval_text(
            agent,
            &other,
            "var log = ''; \
             Object.defineProperty(Maker, 'prototype', \
               { get: function () { log += 'p'; return undefined; } }); \
             var year = { valueOf: function () { log += 'y'; return 2016; } }; \
             var month = { valueOf: function () { log += 'm'; return 6; } }; 0;"
        ),
        normal("0")
    );
    let here = global(agent, realm);
    let there = global(agent, &other);
    agent
        .with_cx(realm, |cx| {
            let here = object(cx, &here);
            let there = object(cx, &there);
            let date = support::host::get(cx, here, "Date");
            let maker = support::host::get(cx, there, "Maker");
            let year = support::host::get(cx, there, "year");
            let month = support::host::get(cx, there, "month");
            let made = cx
                .construct(date, &[year, month], Some(maker))
                .unwrap()
                .unwrap();
            let key = support::host::key(cx, "made");
            cx.set(there, key, made, true).unwrap().unwrap();
        })
        .unwrap();
    assert_eq!(
        eval_text(
            agent,
            &other,
            "(Object.getPrototypeOf(made) === Date.prototype) + ',' + made.getTime() + ',' + log;"
        ),
        quoted("true,1467331200000,ymp")
    );
}

/// The constructor with two or more arguments (`#sec-date-constructor`, step 5) over a table,
/// in the zone "UTC": ToNumber of each argument once and in order, even after a NaN, and none
/// of an eighth; MakeFullYear (0 to 99 is 1900 to 1999); MakeDay, MakeTime, MakeDate,
/// `UTC ( t )` and TimeClip; the common path of `UTC ( t )` makes three zone hook calls and
/// the clock is not read.
#[test]
fn the_constructor_with_components_over_a_table() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date(2016, 6).getTime();"),
        normal("1467331200000")
    );
    assert_eq!(
        zone_calls(&agent),
        [
            ZONE_IDENTIFIER.to_string(),
            format!("{ZONE_EPOCHS} UTC 2016-07-01T00:00:00.000"),
            offset_call("UTC", 1_467_331_200_000_000_000),
        ]
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var log = ''; \
             function v(c, n) { return { valueOf: function () { log += c; return n; } }; } 0;"
        ),
        normal("0")
    );
    let rows: [(&str, Outcome); 16] = [
        ("new Date(99, 0).getTime();", normal("915148800000")),
        ("new Date(99, 0).getFullYear();", normal("1999")),
        ("new Date(0, 0).getFullYear();", normal("1900")),
        ("new Date(99.9, 0).getFullYear();", normal("1999")),
        ("new Date(100, 0).getTime();", normal("-59011459200000")),
        ("new Date(1, 2).getTime();", normal("-2172355200000")),
        ("new Date(NaN, 0).getTime();", normal("NaN")),
        ("new Date(2016, Infinity).getTime();", normal("NaN")),
        (
            "log = ''; new Date(v('a', 2000), v('b', 1), v('c', 2), v('d', 3), v('e', 4), \
             v('f', 5), v('g', 6), v('h', 7)).getTime() + ':' + log;",
            quoted("949460645006:abcdefg"),
        ),
        (
            "log = ''; new Date(v('a', NaN), v('b', 0), v('c', 1), v('d', 0)).getTime() \
             + ':' + log;",
            quoted("NaN:abcd"),
        ),
        (
            "log = ''; try { new Date(v('a', 1), v('b', 2), \
               { valueOf: function () { throw new RangeError('poisoned'); } }, v('d', 4)); \
             } catch (e) { log += e.name; } log;",
            quoted("abRangeError"),
        ),
        (
            "new Date(1970, 0, 1, 0, 0, 0, 8.64e15).getTime();",
            normal("8640000000000000"),
        ),
        (
            "new Date(1970, 0, 1, 0, 0, 0, 8.64e15 + 1).getTime();",
            normal("NaN"),
        ),
        ("new Date(2000, 0, 106752348147).getTime();", normal("NaN")),
        (
            "1 / new Date(1970, 0, 1, 0, 0, 0, -0).getTime();",
            normal("Infinity"),
        ),
        (
            "var made = new Date(2016, 6); \
             (Object.getPrototypeOf(made) === Date.prototype) + ',' + typeof made;",
            quoted("true,object"),
        ),
    ];
    for (source, expected) in rows {
        assert_eq!(eval_text(&mut agent, &realm, source), expected, "{source}");
    }
    assert_eq!(count_of(&agent, "host_system_utc_epoch_nanoseconds"), 0);
    new_target_of_another_realm(&mut agent, &realm);
}

/// `Date.UTC` (`#sec-date.utc`) over the 14 cases of `test/built-ins/Date/UTC`: no argument,
/// one argument, two-digit years, -0, the overflow of month and day, NaN and the infinities,
/// TimeClip, the order of the floating-point arithmetic, non-integral values and the order of
/// the coercions; no hook is called.
#[test]
fn date_utc_over_a_table() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var log = ''; \
             function v(c, n) { return { valueOf: function () { log += c; return n; } }; } 0;"
        ),
        normal("0")
    );
    let rows: [(&str, Outcome); 21] = [
        ("Date.UTC();", normal("NaN")),
        ("Date.UTC(2017);", normal("1483228800000")),
        ("Date.UTC(2016, 6);", normal("1467331200000")),
        ("Date.UTC(99, 0);", normal("915148800000")),
        ("Date.UTC(0);", normal("-2208988800000")),
        ("Date.UTC(-0);", normal("-2208988800000")),
        ("Date.UTC(100);", normal("-59011459200000")),
        ("1 / Date.UTC(1970, 0, 1, 0, 0, 0, -0);", normal("Infinity")),
        ("Date.UTC(2016, 12);", normal("1483228800000")),
        ("Date.UTC(2016, 0, 32);", normal("1454284800000")),
        ("Date.UTC(2016, -1);", normal("1448928000000")),
        ("Date.UTC(NaN);", normal("NaN")),
        ("Date.UTC(2016, NaN);", normal("NaN")),
        ("Date.UTC(2016, 0, Infinity);", normal("NaN")),
        ("Date.UTC(275760, 8, 13);", normal("8640000000000000")),
        ("Date.UTC(275760, 8, 13, 0, 0, 0, 1);", normal("NaN")),
        (
            "Date.UTC(1970, 0, 1, 80063993375, 29, 1, -288230376151711740);",
            normal("29312"),
        ),
        (
            "Date.UTC(1970, 0, 213503982336, 0, 0, 0, -18446744073709552000);",
            normal("34447360"),
        ),
        (
            "Date.UTC(1970.9, 0.9, 1.9, 1.9, 1.9, 1.9, 1.9);",
            normal("3661001"),
        ),
        (
            "log = ''; Date.UTC(v('a', 2000), v('b', 1), v('c', 2), v('d', 3), v('e', 4), \
             v('f', 5), v('g', 6), v('h', 7)) + ':' + log;",
            quoted("949460645006:abcdefg"),
        ),
        (
            "log = ''; try { Date.UTC(v('a', 1), \
               { valueOf: function () { throw new RangeError('poisoned'); } }, v('c', 2)); \
             } catch (e) { log += e.name; } log;",
            quoted("aRangeError"),
        ),
    ];
    for (source, expected) in rows {
        assert_eq!(eval_text(&mut agent, &realm, source), expected, "{source}");
    }
    assert!(agent.host().calls.is_empty(), "{:?}", agent.host().calls);
}

/// The 17 getters, in the order of the expected fields of [`TABLE`]: the eight local fields,
/// `getTimezoneOffset`, the eight UTC fields.
const GETTERS: [&str; 17] = [
    "getFullYear",
    "getMonth",
    "getDate",
    "getDay",
    "getHours",
    "getMinutes",
    "getSeconds",
    "getMilliseconds",
    "getTimezoneOffset",
    "getUTCFullYear",
    "getUTCMonth",
    "getUTCDate",
    "getUTCDay",
    "getUTCHours",
    "getUTCMinutes",
    "getUTCSeconds",
    "getUTCMilliseconds",
];

/// The three zones of [`TABLE`]: the offset the host answers (ns) and what
/// `getTimezoneOffset` answers for it.
const ZONES: [(i128, f64); 3] = [
    (0, 0.0),
    (HOUR_NS, -60.0),
    (-(5 * HOUR_NS + HOUR_NS / 2), 330.0),
];

/// A row of [`TABLE`]: a time value, its UTC fields, and its local fields in each of
/// [`ZONES`]; the fields are year, month (0 to 11), date, weekday (0 is Sunday), hours,
/// minutes, seconds and milliseconds.
type Row = (f64, [i32; 8], [[i32; 8]; 3]);

/// 24 time values and their fields, written from GNU `date -u -d @<seconds>` of the time value
/// plus each zone's offset (an oracle outside the engine), not computed by the engine.
const TABLE: [Row; 24] = [
    (
        -8640000000000000.0,
        [-271821, 3, 20, 2, 0, 0, 0, 0],
        [
            [-271821, 3, 20, 2, 0, 0, 0, 0],
            [-271821, 3, 20, 2, 1, 0, 0, 0],
            [-271821, 3, 19, 1, 18, 30, 0, 0],
        ],
    ),
    (
        -8639999999999999.0,
        [-271821, 3, 20, 2, 0, 0, 0, 1],
        [
            [-271821, 3, 20, 2, 0, 0, 0, 1],
            [-271821, 3, 20, 2, 1, 0, 0, 1],
            [-271821, 3, 19, 1, 18, 30, 0, 1],
        ],
    ),
    (
        -62167219200001.0,
        [-1, 11, 31, 5, 23, 59, 59, 999],
        [
            [-1, 11, 31, 5, 23, 59, 59, 999],
            [0, 0, 1, 6, 0, 59, 59, 999],
            [-1, 11, 31, 5, 18, 29, 59, 999],
        ],
    ),
    (
        -62167219200000.0,
        [0, 0, 1, 6, 0, 0, 0, 0],
        [
            [0, 0, 1, 6, 0, 0, 0, 0],
            [0, 0, 1, 6, 1, 0, 0, 0],
            [-1, 11, 31, 5, 18, 30, 0, 0],
        ],
    ),
    (
        -2208988800000.0,
        [1900, 0, 1, 1, 0, 0, 0, 0],
        [
            [1900, 0, 1, 1, 0, 0, 0, 0],
            [1900, 0, 1, 1, 1, 0, 0, 0],
            [1899, 11, 31, 0, 18, 30, 0, 0],
        ],
    ),
    (
        -2147483648.0,
        [1969, 11, 7, 0, 3, 28, 36, 352],
        [
            [1969, 11, 7, 0, 3, 28, 36, 352],
            [1969, 11, 7, 0, 4, 28, 36, 352],
            [1969, 11, 6, 6, 21, 58, 36, 352],
        ],
    ),
    (
        -86400000.0,
        [1969, 11, 31, 3, 0, 0, 0, 0],
        [
            [1969, 11, 31, 3, 0, 0, 0, 0],
            [1969, 11, 31, 3, 1, 0, 0, 0],
            [1969, 11, 30, 2, 18, 30, 0, 0],
        ],
    ),
    (
        -1.0,
        [1969, 11, 31, 3, 23, 59, 59, 999],
        [
            [1969, 11, 31, 3, 23, 59, 59, 999],
            [1970, 0, 1, 4, 0, 59, 59, 999],
            [1969, 11, 31, 3, 18, 29, 59, 999],
        ],
    ),
    (
        0.0,
        [1970, 0, 1, 4, 0, 0, 0, 0],
        [
            [1970, 0, 1, 4, 0, 0, 0, 0],
            [1970, 0, 1, 4, 1, 0, 0, 0],
            [1969, 11, 31, 3, 18, 30, 0, 0],
        ],
    ),
    (
        1.0,
        [1970, 0, 1, 4, 0, 0, 0, 1],
        [
            [1970, 0, 1, 4, 0, 0, 0, 1],
            [1970, 0, 1, 4, 1, 0, 0, 1],
            [1969, 11, 31, 3, 18, 30, 0, 1],
        ],
    ),
    (
        86399999.0,
        [1970, 0, 1, 4, 23, 59, 59, 999],
        [
            [1970, 0, 1, 4, 23, 59, 59, 999],
            [1970, 0, 2, 5, 0, 59, 59, 999],
            [1970, 0, 1, 4, 18, 29, 59, 999],
        ],
    ),
    (
        951782400000.0,
        [2000, 1, 29, 2, 0, 0, 0, 0],
        [
            [2000, 1, 29, 2, 0, 0, 0, 0],
            [2000, 1, 29, 2, 1, 0, 0, 0],
            [2000, 1, 28, 1, 18, 30, 0, 0],
        ],
    ),
    (
        951868799999.0,
        [2000, 1, 29, 2, 23, 59, 59, 999],
        [
            [2000, 1, 29, 2, 23, 59, 59, 999],
            [2000, 2, 1, 3, 0, 59, 59, 999],
            [2000, 1, 29, 2, 18, 29, 59, 999],
        ],
    ),
    (
        1456704000000.0,
        [2016, 1, 29, 1, 0, 0, 0, 0],
        [
            [2016, 1, 29, 1, 0, 0, 0, 0],
            [2016, 1, 29, 1, 1, 0, 0, 0],
            [2016, 1, 28, 0, 18, 30, 0, 0],
        ],
    ),
    (
        1456790399999.0,
        [2016, 1, 29, 1, 23, 59, 59, 999],
        [
            [2016, 1, 29, 1, 23, 59, 59, 999],
            [2016, 2, 1, 2, 0, 59, 59, 999],
            [2016, 1, 29, 1, 18, 29, 59, 999],
        ],
    ),
    (
        2147483647.0,
        [1970, 0, 25, 0, 20, 31, 23, 647],
        [
            [1970, 0, 25, 0, 20, 31, 23, 647],
            [1970, 0, 25, 0, 21, 31, 23, 647],
            [1970, 0, 25, 0, 15, 1, 23, 647],
        ],
    ),
    (
        2147483648.0,
        [1970, 0, 25, 0, 20, 31, 23, 648],
        [
            [1970, 0, 25, 0, 20, 31, 23, 648],
            [1970, 0, 25, 0, 21, 31, 23, 648],
            [1970, 0, 25, 0, 15, 1, 23, 648],
        ],
    ),
    (
        4294967296.0,
        [1970, 1, 19, 4, 17, 2, 47, 296],
        [
            [1970, 1, 19, 4, 17, 2, 47, 296],
            [1970, 1, 19, 4, 18, 2, 47, 296],
            [1970, 1, 19, 4, 11, 32, 47, 296],
        ],
    ),
    (
        1234567890123.0,
        [2009, 1, 13, 5, 23, 31, 30, 123],
        [
            [2009, 1, 13, 5, 23, 31, 30, 123],
            [2009, 1, 14, 6, 0, 31, 30, 123],
            [2009, 1, 13, 5, 18, 1, 30, 123],
        ],
    ),
    (
        253402300799999.0,
        [9999, 11, 31, 5, 23, 59, 59, 999],
        [
            [9999, 11, 31, 5, 23, 59, 59, 999],
            [10000, 0, 1, 6, 0, 59, 59, 999],
            [9999, 11, 31, 5, 18, 29, 59, 999],
        ],
    ),
    (
        253402300800000.0,
        [10000, 0, 1, 6, 0, 0, 0, 0],
        [
            [10000, 0, 1, 6, 0, 0, 0, 0],
            [10000, 0, 1, 6, 1, 0, 0, 0],
            [9999, 11, 31, 5, 18, 30, 0, 0],
        ],
    ),
    (
        8639999999999999.0,
        [275760, 8, 12, 5, 23, 59, 59, 999],
        [
            [275760, 8, 12, 5, 23, 59, 59, 999],
            [275760, 8, 13, 6, 0, 59, 59, 999],
            [275760, 8, 12, 5, 18, 29, 59, 999],
        ],
    ),
    (
        8640000000000000.0,
        [275760, 8, 13, 6, 0, 0, 0, 0],
        [
            [275760, 8, 13, 6, 0, 0, 0, 0],
            [275760, 8, 13, 6, 1, 0, 0, 0],
            [275760, 8, 12, 5, 18, 30, 0, 0],
        ],
    ),
    (
        1000000000000000.0,
        [33658, 8, 27, 5, 1, 46, 40, 0],
        [
            [33658, 8, 27, 5, 1, 46, 40, 0],
            [33658, 8, 27, 5, 2, 46, 40, 0],
            [33658, 8, 26, 4, 20, 16, 40, 0],
        ],
    ),
];

/// What the 17 getters answer for the Date of time value `t` in `realm`, each called from the
/// host with the Date as `this`: one outermost entry per call.
fn getters_of(agent: &mut Agent<TestHost>, realm: &Realm, times: &[f64]) -> Vec<[f64; 17]> {
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let global = object(cx, &global_root);
            let date = support::host::get(cx, global, "Date");
            let Value::Object(constructor) = date else {
                panic!("Date is {date:?}");
            };
            let prototype = support::host::get_object(cx, constructor, "prototype");
            let getters = GETTERS.map(|name| support::host::get(cx, prototype, name));
            times
                .iter()
                .map(|&t| {
                    let made = cx
                        .construct(date, &[Value::Number(t)], None)
                        .unwrap()
                        .unwrap();
                    getters.map(
                        |getter| match cx.call(getter, made, &[]).unwrap().unwrap() {
                            Value::Number(answer) => answer,
                            other => panic!("{other:?}"),
                        },
                    )
                })
                .collect()
        })
        .unwrap()
}

/// The 17 getters (21.4.4.2 to 21.4.4.19 but `getTime`) over 24 time values at three zone
/// offsets (0, +1 hour, -5.5 hours): 1,224 calls, each answer compared with the table's; each
/// local getter makes two zone hook calls (the identifier and the offset), the UTC getters
/// none. A Date whose time value is NaN answers NaN from all 17 with no hook call.
#[test]
fn seventeen_getters_over_a_table_of_24_time_values_at_three_offsets() {
    let started = Instant::now();
    let times = TABLE.map(|(t, _, _)| t);
    let mut calls = 0;
    for (zone, (offset_ns, timezone_offset)) in ZONES.into_iter().enumerate() {
        let (mut agent, realm) = zone_agent(offset_ns);
        let answers = getters_of(&mut agent, &realm, &times);
        for ((t, utc, locals), answer) in TABLE.iter().zip(&answers) {
            let mut expected = [0.0; 17];
            for (slot, field) in expected.iter_mut().zip(locals[zone]) {
                *slot = f64::from(field);
            }
            expected[8] = timezone_offset;
            for (slot, field) in expected[9..].iter_mut().zip(utc) {
                *slot = f64::from(*field);
            }
            assert_eq!(answer, &expected, "t {t}, offset {offset_ns} ns");
            calls += 17;
        }
        assert_eq!(
            zone_calls(&agent).len(),
            TABLE.len() * 9 * 2,
            "offset {offset_ns}"
        );
        assert_eq!(count_of(&agent, ZONE_IDENTIFIER), TABLE.len() * 9);

        let nan = getters_of(&mut agent, &realm, &[f64::NAN]);
        assert!(nan[0].iter().all(|answer| answer.is_nan()), "{nan:?}");
        assert_eq!(zone_calls(&agent).len(), TABLE.len() * 9 * 2);
    }
    assert_eq!(calls, 1_224);
    println!(
        "getters: {calls} calls over {} time values and {} zones in {} ms",
        TABLE.len(),
        ZONES.len(),
        started.elapsed().as_millis()
    );
}

/// At the edge of the time range: `new Date(8.64e15)` with the offset `i128::MAX` (clamped to
/// 23:59:59.999999999) answers `getFullYear()` 275760, `getDate()` 13 and `getHours()` 23: the
/// local time stays within `LOCAL_TIME_LIMIT`.
#[test]
fn the_last_time_value_with_the_largest_offset_has_fields() {
    let (mut agent, realm) = zone_agent(i128::MAX);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var d = new Date(8.64e15); \
             d.getFullYear() + ',' + d.getMonth() + ',' + d.getDate() + ',' + d.getHours() \
             + ',' + d.getMilliseconds();"
        ),
        quoted("275760,8,13,23,999")
    );
}

/// An identifier that is an offset string (`#sec-istimezoneoffsetstring`) gives the offset
/// itself: one hook call per conversion, the identifier, for a getter and for the constructor.
/// "+24:00" and an identifier of 20 code units whose first 19 are an offset string are named
/// zones: the offset hook is called with the identifier as it was answered.
#[test]
fn an_identifier_that_is_an_offset_string_makes_no_offset_or_epoch_call() {
    let (mut agent, realm) = zone_agent(2 * HOUR_NS);
    agent.host_mut().zone_identifier = "+01:30".to_string();
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var d = new Date(0); \
             d.getHours() + ',' + d.getMinutes() + ',' + d.getTimezoneOffset() + ',' \
             + new Date(2016, 6).getTime();"
        ),
        quoted("1,30,-90,1467325800000")
    );
    assert_eq!(zone_calls(&agent), [ZONE_IDENTIFIER; 4]);

    let (mut agent, realm) = zone_agent(2 * HOUR_NS);
    agent.host_mut().zone_identifier = "-23:59:59.999999999".to_string();
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var d = new Date(0); \
             d.getDate() + ',' + d.getHours() + ',' + d.getMilliseconds();"
        ),
        quoted("31,0,1")
    );
    assert_eq!(zone_calls(&agent), [ZONE_IDENTIFIER; 3]);

    let (mut agent, realm) = zone_agent(2 * HOUR_NS);
    agent.host_mut().zone_identifier = "+01:00:00.123456789".to_string();
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var d = new Date(0); d.getHours() + ',' + d.getMilliseconds();"
        ),
        quoted("1,123")
    );
    assert_eq!(zone_calls(&agent), [ZONE_IDENTIFIER; 2]);

    for named in ["+24:00", "+01:00:00.123456789Z", "Europe/Berlin"] {
        let (mut agent, realm) = zone_agent(2 * HOUR_NS);
        agent.host_mut().zone_identifier = named.to_string();
        assert_eq!(
            eval_text(&mut agent, &realm, "new Date(0).getHours();"),
            normal("2"),
            "{named}"
        );
        assert_eq!(
            zone_calls(&agent),
            [ZONE_IDENTIFIER.to_string(), offset_call(named, 0)],
            "{named}"
        );
    }
}

/// A host function that changes the identifier the host answers.
fn next_zone(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().zone_identifier = "+02:00".to_string();
    Ok(Value::Undefined)
}

/// The identifier is read again by each conversion and kept by none: two reads of `getHours`
/// on one Date answer two hours when the host changes its answer between them.
#[test]
fn the_identifier_is_read_again_by_each_conversion() {
    let (mut agent, realm) = zone_agent(0);
    agent.host_mut().zone_identifier = "+01:00".to_string();
    define_function(&mut agent, &realm, "nextZone", 0, next_zone, false);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var d = new Date(0); var a = d.getHours(); nextZone(); a + ',' + d.getHours();"
        ),
        quoted("1,2")
    );
    assert_eq!(zone_calls(&agent), [ZONE_IDENTIFIER; 2]);
}

/// Milliseconds in an hour.
const HOUR_MS: i128 = 3_600_000;

/// 2021-03-28T01:00Z, when the zone of [`daylight_saving`] moves from +1 to +2 hours.
const SPRING_MS: i128 = 1_616_893_200_000;

/// 2021-10-31T01:00Z, when it moves back.
const AUTUMN_MS: i128 = 1_635_642_000_000;

/// The local time of `local` read as UTC, in milliseconds.
fn local_ms(local: &IsoDateTime) -> i128 {
    let date = swell_num::date::IsoDateTime {
        year: i64::from(local.year),
        month: local.month,
        day: local.day,
        hour: local.hour,
        minute: local.minute,
        second: local.second,
        millisecond: local.millisecond,
        microsecond: 0,
        nanosecond: 0,
    };
    swell_num::date::get_utc_epoch_nanoseconds(&date).unwrap() / 1_000_000
}

/// A zone of +1 hour with daylight saving time of +2 hours from [`SPRING_MS`] to
/// [`AUTUMN_MS`]: the local hour from 02:00 on 2021-03-28 is skipped, the local hour from
/// 02:00 on 2021-10-31 is repeated. The instants of a local time, in ascending order.
fn daylight_saving(local: &IsoDateTime) -> Vec<i128> {
    let ms = local_ms(local);
    let summer = ms - 2 * HOUR_MS;
    let winter = ms - HOUR_MS;
    let mut instants = Vec::new();
    if (SPRING_MS..AUTUMN_MS).contains(&summer) {
        instants.push(summer * 1_000_000);
    }
    if !(SPRING_MS..AUTUMN_MS).contains(&winter) {
        instants.push(winter * 1_000_000);
    }
    instants
}

/// 2021-03-28T00:00 local, where the gaps of [`gap_24`], [`gap_48`] and [`gap_80`] begin.
const GAP_START_MS: i128 = 1_616_889_600_000;

/// The instants of a zone of offset 0 whose local times from [`GAP_START_MS`] for `hours`
/// hours have none.
fn gap(local: &IsoDateTime, hours: i128) -> Vec<i128> {
    let ms = local_ms(local);
    if (GAP_START_MS..GAP_START_MS + hours * HOUR_MS).contains(&ms) {
        Vec::new()
    } else {
        vec![ms * 1_000_000]
    }
}

/// A gap of 24 hours.
fn gap_24(local: &IsoDateTime) -> Vec<i128> {
    gap(local, 24)
}

/// A gap of 48 hours, the longest offsets below 24 hours make (decision 17, item 4).
fn gap_48(local: &IsoDateTime) -> Vec<i128> {
    gap(local, 48)
}

/// A gap of 80 hours, beyond the search's 2^28 ms.
fn gap_80(local: &IsoDateTime) -> Vec<i128> {
    gap(local, 80)
}

/// The search for `tBefore` (`#sec-utc-t`; `docs/design/engine.md` decision 6) over hosts whose
/// rule is written here: a repeated local time takes the first instant, a skipped one the last
/// instant of the last local time before it; a gap of 24 and of 48 hours is crossed, the
/// latter in exactly 59 calls, and a gap of 80 hours gives NaN after 31. Every count is at most
/// 59.
#[test]
fn the_epoch_search_over_a_daylight_saving_rule_and_gaps() {
    type Case = (
        fn(&IsoDateTime) -> Vec<i128>,
        i128,
        &'static str,
        &'static str,
        Option<i128>,
        Option<usize>,
    );
    let cases: [Case; 6] = [
        (
            daylight_saving,
            HOUR_NS,
            "new Date(2021, 0, 15, 12).getTime();",
            "1610708400000",
            Some(1_610_708_400_000),
            Some(3),
        ),
        (
            daylight_saving,
            HOUR_NS,
            "new Date(2021, 9, 31, 2, 30).getTime();",
            "1635643800000",
            Some(1_635_640_200_000),
            Some(3),
        ),
        (
            daylight_saving,
            HOUR_NS,
            "new Date(2021, 2, 28, 2, 30).getTime();",
            "1616895000000",
            Some(1_616_893_199_999),
            None,
        ),
        (
            gap_24,
            0,
            "new Date(2021, 2, 28, 23, 59, 59, 999).getTime();",
            "1616975999999",
            Some(1_616_889_599_999),
            None,
        ),
        (
            gap_48,
            0,
            "new Date(2021, 2, 29, 23, 59, 59, 999).getTime();",
            "1617062399999",
            Some(1_616_889_599_999),
            Some(59),
        ),
        (
            gap_80,
            0,
            "new Date(2021, 2, 31, 7, 59, 59, 999).getTime();",
            "NaN",
            None,
            Some(31),
        ),
    ];
    for (epochs, offset_ns, source, expected, instant_ms, count) in cases {
        let mut agent = agent().unwrap();
        agent.host_mut().zone_epochs = Some(epochs);
        agent.host_mut().zone_offset_ns = offset_ns;
        let realm = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            normal(expected),
            "{source}"
        );
        let calls = zone_calls(&agent);
        println!("{source} {} zone hook calls", calls.len());
        assert!(calls.len() <= 59, "{source}: {}", calls.len());
        assert_eq!(count_of(&agent, ZONE_IDENTIFIER), 1, "{source}");
        let offsets: Vec<&String> = calls
            .iter()
            .filter(|call| call.starts_with(ZONE_OFFSET))
            .collect();
        match instant_ms {
            Some(ms) => assert_eq!(offsets, [&offset_call("UTC", ms * 1_000_000)], "{source}"),
            None => assert!(offsets.is_empty(), "{source}"),
        }
        if let Some(count) = count {
            assert_eq!(calls.len(), count, "{source}");
        }
    }
}

/// The lengths, names and property attributes of the 17 getters and of `Date.UTC` (clause 18:
/// writable, not enumerable, configurable; `length` and `name` not writable, not enumerable,
/// configurable; `Date.UTC` has `length` 7, 21.4.3.4).
#[test]
fn lengths_names_and_descriptors_of_the_getters_and_utc() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let names = GETTERS.map(|name| format!("'{name}'")).join(", ");
    let source = format!(
        "function attributes(d) {{ return '' + d.writable + d.enumerable + d.configurable; }} \
         function row(object, key) {{ \
           var d = Object.getOwnPropertyDescriptor(object, key); \
           var f = d.value; \
           return key + ' ' + typeof f + ' ' + attributes(d) + ' ' + f.length + ' ' + f.name \
             + ' ' + attributes(Object.getOwnPropertyDescriptor(f, 'length')) + ' ' \
             + attributes(Object.getOwnPropertyDescriptor(f, 'name')); \
         }} \
         var rows = [row(Date, 'UTC')]; \
         [{names}].forEach(function (k) {{ rows.push(row(Date.prototype, k)); }}); \
         try {{ new Date.prototype.getHours(); rows.push('constructs'); }} \
         catch (e) {{ rows.push(e.name); }} \
         try {{ new Date.UTC(); rows.push('constructs'); }} catch (e) {{ rows.push(e.name); }} \
         rows.join('|');"
    );
    let mut expected =
        vec!["UTC function truefalsetrue 7 UTC falsefalsetrue falsefalsetrue".to_string()];
    for name in GETTERS {
        expected.push(format!(
            "{name} function truefalsetrue 0 {name} falsefalsetrue falsefalsetrue"
        ));
    }
    expected.push("TypeError".to_string());
    expected.push("TypeError".to_string());
    assert_eq!(
        eval_text(&mut agent, &realm, &source),
        quoted(&expected.join("|"))
    );
}

/// A getter throws a TypeError for a `this` without [[DateValue]] (`#sec-thistimevalue`)
/// before any hook call, `%Date.prototype%` included; a Date of another realm is a Date.
#[test]
fn getters_check_this_and_take_a_date_of_another_realm() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    for name in GETTERS {
        for this in ["{}", "Date.prototype", "5", "undefined"] {
            let source = format!("Date.prototype.{name}.call({this});");
            assert_eq!(
                eval_text(&mut agent, &realm, &source),
                Outcome::Throw("TypeError".to_string()),
                "{source}"
            );
        }
    }
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.prototype.getHours();"),
        Outcome::Throw("TypeError".to_string())
    );
    assert!(zone_calls(&agent).is_empty(), "{:?}", zone_calls(&agent));

    let other = support::host::realm(&mut agent);
    let here = global(&mut agent, &realm);
    let there = global(&mut agent, &other);
    agent
        .with_cx(&realm, |cx| {
            let here = object(cx, &here);
            let there = object(cx, &there);
            let date = support::host::get(cx, there, "Date");
            let made = cx
                .construct(date, &[Value::Number(86_400_000.0)], None)
                .unwrap()
                .unwrap();
            let key = support::host::key(cx, "foreign");
            cx.set(here, key, made, true).unwrap().unwrap();
        })
        .unwrap();
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "(Object.getPrototypeOf(foreign) !== Date.prototype) + ',' \
             + Date.prototype.getDate.call(foreign) + ',' \
             + Date.prototype.getUTCDay.call(foreign);"
        ),
        quoted("true,2,5")
    );
}

/// A search over a host with no instant ends at the step budget: `steps_per_entry` 10 ends
/// `new Date(2020, 0, 1)` in `Terminated(StepBudget)` with fewer than 31 hook calls, the agent
/// alive, and the next entry runs.
#[test]
fn a_search_ends_at_the_step_budget() {
    let mut budget = limits();
    budget.steps_per_entry = 10;
    let mut agent = agent_with(budget).unwrap();
    agent.host_mut().zone_epochs = Some(no_instants);
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date(2020, 0, 1);"),
        Outcome::Terminated(Termination::StepBudget)
    );
    let calls = zone_calls(&agent);
    println!(
        "zone hook calls before the step budget of 10: {}",
        calls.len()
    );
    assert!(!calls.is_empty() && calls.len() < 31, "{calls:?}");
    assert_eq!(agent.last_entry_steps(), Ok(10));
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.UTC(1970);"),
        normal("0")
    );
}

thread_local! {
    /// The handle `interrupting` sets.
    static HANDLE: Cell<Option<InterruptHandle>> = const { Cell::new(None) };
    /// The length of the list `sized` answers.
    static LENGTH: Cell<usize> = const { Cell::new(0) };
}

/// An epoch hook that sets its agent's interrupt flag and answers no instant.
fn interrupting(_local: &IsoDateTime) -> Vec<i128> {
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Vec::new()
}

/// A search ends at an interrupt the host raises from inside the epoch hook: the step before
/// the next hook call reads it, and the entry ends in `Terminated(Interrupted)` after one call
/// of the epoch hook; the next entry runs.
#[test]
fn a_search_ends_at_an_interrupt() {
    let mut agent = agent().unwrap();
    agent.host_mut().zone_epochs = Some(interrupting);
    let realm = realm(&mut agent);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date(2020, 0, 1);"),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(
        zone_calls(&agent),
        [
            ZONE_IDENTIFIER.to_string(),
            format!("{ZONE_EPOCHS} UTC 2020-01-01T00:00:00.000")
        ]
    );
    assert!(!agent.interrupt_handle().is_set());
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date(0).getUTCHours();"),
        normal("0")
    );
}

/// An epoch hook that answers a million instants.
fn a_million(local: &IsoDateTime) -> Vec<i128> {
    vec![local_ms(local) * 1_000_000; 1_000_000]
}

/// An epoch hook that answers [`LENGTH`] instants.
fn sized(local: &IsoDateTime) -> Vec<i128> {
    vec![local_ms(local) * 1_000_000; LENGTH.with(Cell::get)]
}

/// The list of the epoch hook is charged 16 bytes per element before an element is read: a
/// million instants under a budget of 1 MiB end the entry in `Terminated(MemoryBudget)` with no
/// offset call, and the next entry runs. A list that fits is credited when it is dropped: a
/// list of more than half the room left is answered three times in a row.
#[test]
fn a_vec_over_the_memory_budget_ends_before_it_is_read() {
    let mut budget = limits();
    budget.memory_bytes = 1 << 20;
    let mut agent = agent_with(budget).unwrap();
    agent.host_mut().zone_epochs = Some(a_million);
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date(2020, 0, 1);"),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    assert_eq!(
        zone_calls(&agent),
        [
            ZONE_IDENTIFIER.to_string(),
            format!("{ZONE_EPOCHS} UTC 2020-01-01T00:00:00.000")
        ]
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date(0).getUTCHours();"),
        normal("0")
    );

    agent.host_mut().zone_epochs = Some(sized);
    LENGTH.with(|length| length.set(1));
    let source = "new Date(2020, 0, 1).getTime();";
    assert_eq!(
        eval_text(&mut agent, &realm, source),
        normal("1577836800000")
    );
    let used = agent.heap_bytes().unwrap();
    let room = (1 << 20) - used;
    assert!(room > 256 * 1024, "{used} bytes used");
    let length = usize::try_from((room - 64 * 1024) / 16).unwrap();
    LENGTH.with(|cell| cell.set(length));
    for round in 0..3 {
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            normal("1577836800000"),
            "round {round}, {length} instants, {used} bytes used before"
        );
    }
    println!("a list of {length} instants with {used} of 1048576 bytes used, three times");
}

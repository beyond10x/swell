//! The 14 setters of `Date.prototype` but `setTime` (ECMA-262 21.4.4.20 to 21.4.4.34,
//! `#sec-date.prototype.setdate` to `#sec-date.prototype.setutcseconds`;
//! `story:date-setters`): the local ones over `LocalTime` (`#sec-localtime`) and `UTC`
//! (`#sec-utc-t`) of the zone story, the UTC ones with no hook call, the order of the reads
//! (the time value first, then ToNumber of each argument present), TimeClip
//! (`#sec-timeclip`) and the hook calls a setter makes (`docs/design/host-interface.md`,
//! decision 17, items 1, 4 and 5; HOST-210, HOST-211).
#![cfg(test)]

use crate::support;

use std::cell::Cell;
use std::time::Instant;

use support::host::{
    TestHost, ZONE_EPOCHS, ZONE_IDENTIFIER, ZONE_OFFSET, agent, agent_with, global, limits, object,
    realm,
};
use support::script::{Outcome, define_function, describe, eval, eval_text};
use swell::{
    Agent, AgentState, Completion, Cx, HostCall, InterruptHandle, IsoDateTime, Realm, Termination,
    Value,
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

/// The three zone offsets of the table, as in `tests/date_zone.rs`: 0, +1 hour and
/// -5.5 hours, in nanoseconds.
const ZONES: [i128; 3] = [0, HOUR_NS, -(5 * HOUR_NS + HOUR_NS / 2)];

/// 2016-07-04T05:06:07.008Z, the time value of every Date of the table.
const T0: f64 = 1_467_608_767_008.0;

/// The 14 setters: each local name and its UTC twin, with the `length` of both.
const SETTERS: [(&str, &str, u32); 7] = [
    ("setMilliseconds", "setUTCMilliseconds", 1),
    ("setSeconds", "setUTCSeconds", 2),
    ("setMinutes", "setUTCMinutes", 3),
    ("setHours", "setUTCHours", 4),
    ("setDate", "setUTCDate", 1),
    ("setMonth", "setUTCMonth", 2),
    ("setFullYear", "setUTCFullYear", 3),
];

/// A row of [`TABLE`]: the local setter (its UTC twin is the name with `UTC` after `set`), the
/// arguments, and the time value stored and answered: by the UTC twin, then by the local
/// setter at each of [`ZONES`].
type Row = (&'static str, &'static [f64], [f64; 4]);

const NAN: f64 = f64::NAN;
const INF: f64 = f64::INFINITY;

/// 37 rows over the 14 setters: each with one to four arguments as its `length` allows, NaN,
/// Infinity, a fractional value (truncated toward zero, -0.5 to +0), and values that carry
/// into the next or the previous field. The time values are written from GNU `date -u` (the
/// epoch seconds of the first day of the new month) and the integer sums of MakeDay and
/// MakeTime over the fields of 2016-07-04T05:06:07.008 in each zone (local 06:06:07.008 at
/// +1 hour, 2016-07-03T23:36:07.008 at -5.5 hours), an oracle outside the engine, not
/// computed by the engine.
const TABLE: [Row; 37] = [
    ("setMilliseconds", &[999.0], [1467608767999.0; 4]),
    ("setMilliseconds", &[1.9], [1467608767001.0; 4]),
    ("setMilliseconds", &[-1.0], [1467608766999.0; 4]),
    ("setMilliseconds", &[NAN], [NAN; 4]),
    ("setSeconds", &[59.0], [1467608819008.0; 4]),
    ("setSeconds", &[1.0, 2.0], [1467608761002.0; 4]),
    ("setSeconds", &[30.9, -0.5], [1467608790000.0; 4]),
    ("setSeconds", &[INF], [NAN; 4]),
    (
        "setMinutes",
        &[1.0],
        [
            1467608467008.0,
            1467608467008.0,
            1467608467008.0,
            1467606667008.0,
        ],
    ),
    (
        "setMinutes",
        &[1.0, 2.0],
        [
            1467608462008.0,
            1467608462008.0,
            1467608462008.0,
            1467606662008.0,
        ],
    ),
    (
        "setMinutes",
        &[1.0, 2.0, 3.0],
        [
            1467608462003.0,
            1467608462003.0,
            1467608462003.0,
            1467606662003.0,
        ],
    ),
    (
        "setMinutes",
        &[59.99, 0.0, 1000.0],
        [
            1467611941000.0,
            1467611941000.0,
            1467611941000.0,
            1467610141000.0,
        ],
    ),
    ("setMinutes", &[NAN, 1.0], [NAN; 4]),
    (
        "setHours",
        &[1.0],
        [
            1467594367008.0,
            1467594367008.0,
            1467590767008.0,
            1467529567008.0,
        ],
    ),
    (
        "setHours",
        &[1.0, 2.0],
        [
            1467594127008.0,
            1467594127008.0,
            1467590527008.0,
            1467527527008.0,
        ],
    ),
    (
        "setHours",
        &[1.0, 2.0, 3.0],
        [
            1467594123008.0,
            1467594123008.0,
            1467590523008.0,
            1467527523008.0,
        ],
    ),
    (
        "setHours",
        &[1.0, 2.0, 3.0, 4.0],
        [
            1467594123004.0,
            1467594123004.0,
            1467590523004.0,
            1467527523004.0,
        ],
    ),
    (
        "setHours",
        &[23.99],
        [
            1467673567008.0,
            1467673567008.0,
            1467669967008.0,
            1467608767008.0,
        ],
    ),
    (
        "setHours",
        &[24.0],
        [
            1467677167008.0,
            1467677167008.0,
            1467673567008.0,
            1467612367008.0,
        ],
    ),
    ("setHours", &[1.0, INF], [NAN; 4]),
    (
        "setDate",
        &[1.0],
        [
            1467349567008.0,
            1467349567008.0,
            1467349567008.0,
            1467435967008.0,
        ],
    ),
    (
        "setDate",
        &[31.0],
        [
            1469941567008.0,
            1469941567008.0,
            1469941567008.0,
            1470027967008.0,
        ],
    ),
    (
        "setDate",
        &[15.5],
        [
            1468559167008.0,
            1468559167008.0,
            1468559167008.0,
            1468645567008.0,
        ],
    ),
    (
        "setDate",
        &[32.0],
        [
            1470027967008.0,
            1470027967008.0,
            1470027967008.0,
            1470114367008.0,
        ],
    ),
    ("setDate", &[NAN], [NAN; 4]),
    ("setMonth", &[0.0], [1451883967008.0; 4]),
    (
        "setMonth",
        &[1.0, 29.0],
        [
            1456722367008.0,
            1456722367008.0,
            1456722367008.0,
            1456808767008.0,
        ],
    ),
    ("setMonth", &[11.5], [1480827967008.0; 4]),
    ("setMonth", &[12.0], [1483506367008.0; 4]),
    (
        "setMonth",
        &[-1.0, 31.0],
        [
            1451538367008.0,
            1451538367008.0,
            1451538367008.0,
            1451624767008.0,
        ],
    ),
    ("setMonth", &[INF], [NAN; 4]),
    ("setFullYear", &[2000.0], [962687167008.0; 4]),
    ("setFullYear", &[2000.0, 1.0], [949640767008.0; 4]),
    (
        "setFullYear",
        &[2000.0, 1.0, 29.0],
        [
            951800767008.0,
            951800767008.0,
            951800767008.0,
            951887167008.0,
        ],
    ),
    ("setFullYear", &[1999.5], [931064767008.0; 4]),
    (
        "setFullYear",
        &[1.0, 0.0, 1.0],
        [
            -62135578432992.0,
            -62135578432992.0,
            -62135578432992.0,
            -62135492032992.0,
        ],
    ),
    ("setFullYear", &[2000.0, NAN], [NAN; 4]),
];

/// The UTC twin of the local setter `name`.
fn utc_name(name: &str) -> String {
    format!("setUTC{}", name.trim_start_matches("set"))
}

/// Whether two Numbers are the same value for this table: NaN is NaN.
fn same(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || a == b
}

/// What `setter` answers and what `getTime` reads afterwards, called from the host on a new
/// Date of time value `t` with `args`, in `realm`; and the number of zone hook calls it made.
fn set_from_host(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    setter: &str,
    t: f64,
    args: &[f64],
) -> (f64, f64, usize) {
    let before = zone_calls(agent).len();
    let global_root = global(agent, realm);
    let (answer, stored) = agent
        .with_cx(realm, |cx| {
            let global = object(cx, &global_root);
            let date = support::host::get(cx, global, "Date");
            let Value::Object(constructor) = date else {
                panic!("Date is {date:?}");
            };
            let prototype = support::host::get_object(cx, constructor, "prototype");
            let function = support::host::get(cx, prototype, setter);
            let get_time = support::host::get(cx, prototype, "getTime");
            let made = cx
                .construct(date, &[Value::Number(t)], None)
                .unwrap()
                .unwrap();
            let values: Vec<Value> = args.iter().map(|&x| Value::Number(x)).collect();
            let answer = match cx.call(function, made, &values).unwrap().unwrap() {
                Value::Number(x) => x,
                other => panic!("{setter} answered {other:?}"),
            };
            let stored = match cx.call(get_time, made, &[]).unwrap().unwrap() {
                Value::Number(x) => x,
                other => panic!("getTime answered {other:?}"),
            };
            (answer, stored)
        })
        .unwrap();
    (answer, stored, zone_calls(agent).len() - before)
}

/// The 14 setters over [`TABLE`] at the three zone offsets: 222 calls, each answer and each
/// stored time value compared with the table's. A local setter whose new date is finite makes
/// five zone hook calls (two of `LocalTime`, three of `UTC` on the common path), one whose new
/// date is NaN two; a UTC setter none.
#[test]
fn fourteen_setters_over_a_table_at_three_offsets() {
    let started = Instant::now();
    let mut calls = 0;
    for (zone, offset_ns) in ZONES.into_iter().enumerate() {
        let (mut agent, realm) = zone_agent(offset_ns);
        for (name, args, expected) in TABLE {
            let utc = utc_name(name);
            let (answer, stored, hooks) = set_from_host(&mut agent, &realm, &utc, T0, args);
            assert!(
                same(answer, expected[0]) && same(stored, expected[0]),
                "{utc}{args:?} at {offset_ns}: {answer} {stored}, expected {}",
                expected[0]
            );
            assert_eq!(hooks, 0, "{utc}{args:?}");
            let (answer, stored, hooks) = set_from_host(&mut agent, &realm, name, T0, args);
            let want = expected[zone + 1];
            assert!(
                same(answer, want) && same(stored, want),
                "{name}{args:?} at {offset_ns}: {answer} {stored}, expected {want}"
            );
            assert_eq!(hooks, if want.is_nan() { 2 } else { 5 }, "{name}{args:?}");
            calls += 2;
        }
    }
    assert_eq!(calls, 222);
    println!(
        "setters: {calls} calls over {} rows and {} zones in {} ms",
        TABLE.len(),
        ZONES.len(),
        started.elapsed().as_millis()
    );
}

/// A Date whose time value is NaN: `setFullYear` starts from +0 with no `LocalTime` call and
/// gives the year with month 0 and date 1 at 00:00:00.000 local (three hook calls, those of
/// `UTC`), `setUTCFullYear` the same in UTC with none; every other setter answers NaN, stores
/// nothing and makes no hook call.
#[test]
fn a_nan_date_gives_a_year_to_set_full_year_and_nan_to_the_others() {
    let firsts: [[f64; 3]; 2] = [
        [946684800000.0, 946681200000.0, 946704600000.0],
        [961027200000.0, 961023600000.0, 961047000000.0],
    ];
    for (zone, offset_ns) in ZONES.into_iter().enumerate() {
        let (mut agent, realm) = zone_agent(offset_ns);
        for (args, expected) in [
            (&[2000.0][..], firsts[0]),
            (&[2000.0, 5.0, 15.0], firsts[1]),
        ] {
            let (answer, stored, hooks) =
                set_from_host(&mut agent, &realm, "setFullYear", NAN, args);
            assert_eq!(
                (answer, stored),
                (expected[zone], expected[zone]),
                "{args:?}"
            );
            assert_eq!(hooks, 3, "setFullYear{args:?}");
            let (answer, stored, hooks) =
                set_from_host(&mut agent, &realm, "setUTCFullYear", NAN, args);
            assert_eq!((answer, stored), (expected[0], expected[0]), "{args:?}");
            assert_eq!(hooks, 0);
        }
        assert_eq!(count_of(&agent, ZONE_IDENTIFIER), 2);
        for (local, utc, _) in &SETTERS[..6] {
            for name in [*local, *utc] {
                let (answer, stored, hooks) =
                    set_from_host(&mut agent, &realm, name, NAN, &[1.0, 2.0, 3.0, 4.0]);
                assert!(answer.is_nan() && stored.is_nan(), "{name}");
                assert_eq!(hooks, 0, "{name}");
            }
        }
    }
}

/// The reads of a setter in the order of its clause: the time value first, then ToNumber of
/// each argument present, each once and in order, also when the time value is NaN; an argument
/// past the setter's `length` is not read; a ToNumber that throws ends the call with the Date
/// unchanged; a `valueOf` that calls `setTime` on the same Date does not change the time value
/// the setter computes from (the cases `date-value-read-before-tonumber-*`).
#[test]
fn the_time_value_is_read_first_then_each_argument_once_in_order() {
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
    for (local, utc, length) in SETTERS {
        for name in [local, utc] {
            for t in ["0", "NaN"] {
                let source = format!(
                    "log = ''; var d = new Date({t}); \
                     d.{name}(v('a', 1), v('b', 2), v('c', 3), v('d', 4), v('e', 5)); log;"
                );
                let read = &"abcd"[..usize::try_from(length).unwrap()];
                assert_eq!(
                    eval_text(&mut agent, &realm, &source),
                    quoted(read),
                    "{source}"
                );
            }
            let source = format!(
                "log = ''; var d = new Date(0); try {{ d.{name}(v('a', 1), \
                   {{ valueOf: function () {{ throw new RangeError('poisoned'); }} }}, v('c', 3)); \
                 }} catch (e) {{ log += e.name; }} log + ',' + d.getTime();"
            );
            // A setter of `length` 1 does not read the second argument: `setMilliseconds(1)`
            // stores 1, `setDate(1)` stores 0.
            let expected = match local {
                "setMilliseconds" => "a,1",
                "setDate" => "a,0",
                _ => "aRangeError,0",
            };
            assert_eq!(
                eval_text(&mut agent, &realm, &source),
                quoted(expected),
                "{source}"
            );
        }
    }

    // The time value is read before ToNumber: a Date of 0 whose `valueOf` sets it to NaN is
    // set from 0, and a Date of NaN whose `valueOf` sets it to 0 answers NaN, but for the two
    // full-year setters, which start from +0.
    for (local, utc, _) in SETTERS {
        for name in [local, utc] {
            let valid = format!(
                "var d = new Date(0); \
                 var r = d.{name}({{ valueOf: function () {{ d.setTime(NaN); return 1; }} }}); \
                 (r === d.getTime()) + ',' + isNaN(r);"
            );
            assert_eq!(
                eval_text(&mut agent, &realm, &valid),
                quoted("true,false"),
                "{valid}"
            );
            let invalid = format!(
                "var d = new Date(NaN); \
                 var r = d.{name}({{ valueOf: function () {{ d.setTime(0); return 1; }} }}); \
                 String(r) + ',' + d.getTime();"
            );
            let expected = if local == "setFullYear" {
                "-62135596800000,-62135596800000"
            } else {
                "NaN,0"
            };
            assert_eq!(
                eval_text(&mut agent, &realm, &invalid),
                quoted(expected),
                "{invalid}"
            );
        }
    }
}

/// A host function that records a mark among the host's calls.
fn mark(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().calls.push("mark".to_string());
    Ok(Value::Undefined)
}

/// The hook calls and the coercions interleave as the clauses give them: `setHours` reads all
/// its arguments before `LocalTime`; `setFullYear` reads `year`, then calls `LocalTime`, then
/// reads `month` and `day` (21.4.4.21, steps 4 to 7).
#[test]
fn the_conversions_come_where_the_clauses_put_them() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "mark", 0, mark, false);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var m = { valueOf: function () { mark(); return 1; } }; \
             new Date(0).setHours(1, m);"
        ),
        normal("3660000")
    );
    let marked = |agent: &Agent<TestHost>| -> Vec<String> {
        agent
            .host()
            .calls
            .iter()
            .map(|call| call.split(' ').next().unwrap_or("").to_string())
            .collect()
    };
    assert_eq!(
        marked(&agent),
        [
            "mark",
            ZONE_IDENTIFIER,
            ZONE_OFFSET,
            ZONE_IDENTIFIER,
            ZONE_EPOCHS,
            ZONE_OFFSET
        ]
    );
    agent.host_mut().calls.clear();
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date(0).setFullYear(2016, m, 4);"),
        normal("1454544000000")
    );
    assert_eq!(
        marked(&agent),
        [
            ZONE_IDENTIFIER,
            ZONE_OFFSET,
            "mark",
            ZONE_IDENTIFIER,
            ZONE_EPOCHS,
            ZONE_OFFSET
        ]
    );
}

/// A `this` that is not a Date (`#sec-thistimevalue`) throws a TypeError before any argument is
/// read and before any hook call, for each of the 14; `%Date.prototype%` is not a Date.
#[test]
fn a_this_that_is_not_a_date_throws_before_any_coercion() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var log = ''; var arg = { valueOf: function () { log += 'v'; return 1; } }; 0;"
        ),
        normal("0")
    );
    for (local, utc, _) in SETTERS {
        for name in [local, utc] {
            for this in ["{}", "Date.prototype", "5", "undefined", "new Number(0)"] {
                let source = format!(
                    "log = ''; try {{ Date.prototype.{name}.call({this}, arg, arg); 'no throw'; \
                     }} catch (e) {{ e.name + ':' + log; }}"
                );
                assert_eq!(
                    eval_text(&mut agent, &realm, &source),
                    quoted("TypeError:"),
                    "{source}"
                );
            }
        }
    }
    assert!(zone_calls(&agent).is_empty(), "{:?}", zone_calls(&agent));
}

/// TimeClip (`#sec-timeclip`; the cases `new-value-time-clip`): a new time value beyond
/// ±8.64 × 10^15 stores NaN and answers it, for the local and the UTC setters.
#[test]
fn a_new_value_beyond_the_time_range_stores_nan() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let rows: [(&str, &str); 12] = [
        ("new Date(8.64e15)", "setHours(24)"),
        ("new Date(8.64e15)", "setHours(0, 24 * 60)"),
        ("new Date(8.64e15)", "setUTCHours(0, 0, 24 * 60 * 60)"),
        (
            "new Date(8.64e15)",
            "setUTCHours(0, 0, 0, 24 * 60 * 60 * 1000)",
        ),
        ("new Date(8.64e15)", "setMilliseconds(1)"),
        ("new Date(8.64e15)", "setUTCMilliseconds(1)"),
        ("new Date(8.64e15)", "setDate(14)"),
        ("new Date(8.64e15)", "setUTCMonth(9)"),
        ("new Date(8.64e15)", "setFullYear(275761)"),
        ("new Date(-8.64e15)", "setUTCSeconds(-1)"),
        ("new Date(-8.64e15)", "setMinutes(-1)"),
        ("new Date(-8.64e15)", "setUTCFullYear(-271822)"),
    ];
    for (made, call) in rows {
        let source = format!("var d = {made}; var r = d.{call}; String(r) + ',' + d.getTime();");
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            quoted("NaN,NaN"),
            "{source}"
        );
    }
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "new Date(8.64e15 - 1).setUTCMilliseconds(1000);"
        ),
        normal("8640000000000000")
    );
}

/// A host function that asks for more steps than any budget has.
fn stop(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.charge_steps(u64::MAX)?;
    Ok(Value::Undefined)
}

/// `setHours` on an ordinary Date reads the identifier twice (once per conversion) and makes
/// 2 + 3 = 5 zone hook calls, each charged one step: its entry spends 5 steps more than
/// `setUTCHours`, which makes none.
#[test]
fn set_hours_reads_the_identifier_twice_and_makes_five_calls() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "stop", 0, stop, false);
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.prototype.stop = stop; 0;"),
        normal("0")
    );
    let (outcome, utc) = steps_of(&mut agent, &realm, "new Date(0).setUTCHours(1);");
    assert_eq!(outcome, normal("3600000"));
    assert!(zone_calls(&agent).is_empty(), "{:?}", zone_calls(&agent));
    let (outcome, local) = steps_of(&mut agent, &realm, "new Date(0).setHours(1);");
    assert_eq!(outcome, normal("3600000"));
    assert_eq!(
        zone_calls(&agent),
        [
            ZONE_IDENTIFIER.to_string(),
            offset_call("UTC", 0),
            ZONE_IDENTIFIER.to_string(),
            format!("{ZONE_EPOCHS} UTC 1970-01-01T01:00:00.000"),
            offset_call("UTC", 3_600_000_000_000),
        ]
    );
    assert_eq!(count_of(&agent, ZONE_IDENTIFIER), 2);
    let (outcome, begun) = steps_of(&mut agent, &realm, "new Date(0).stop(1);");
    assert_eq!(outcome, Outcome::Terminated(Termination::StepBudget));
    println!("steps: setUTCHours(1) {utc}, setHours(1) {local}, at the call {begun}");
    assert_eq!(local, utc + 5, "setHours {local}, setUTCHours {utc}");
}

/// 2021-03-28T00:00 local, where the gap of [`gap_48`] begins.
const GAP_START_MS: i128 = 1_616_889_600_000;

/// Milliseconds in an hour.
const HOUR_MS: i128 = 3_600_000;

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

/// A zone of offset 0 whose local times from [`GAP_START_MS`] for 48 hours have no instant,
/// the longest gap offsets below 24 hours make (decision 17, item 4).
fn gap_48(local: &IsoDateTime) -> Vec<i128> {
    let ms = local_ms(local);
    if (GAP_START_MS..GAP_START_MS + 48 * HOUR_MS).contains(&ms) {
        Vec::new()
    } else {
        vec![ms * 1_000_000]
    }
}

/// A local setter over a host with a gap of 48 hours at the new local time: `LocalTime` makes
/// 2 calls and `UTC` 59, 61 in all (decision 17, item 5: "at most 61"), and the time value is
/// the last instant before the gap's local time read with its offset.
#[test]
fn a_local_setter_into_a_48_hour_gap_makes_at_most_61_calls() {
    let mut agent = agent().unwrap();
    agent.host_mut().zone_epochs = Some(gap_48);
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var d = new Date(Date.UTC(2021, 2, 29, 12)); d.setHours(23, 59, 59, 999);"
        ),
        normal("1617062399999")
    );
    let calls = zone_calls(&agent);
    println!(
        "setHours into a gap of 48 hours: {} zone hook calls",
        calls.len()
    );
    assert!(calls.len() <= 61, "{}", calls.len());
    assert_eq!(calls.len(), 61);
    assert_eq!(count_of(&agent, ZONE_IDENTIFIER), 2);
    assert_eq!(count_of(&agent, ZONE_OFFSET), 2);
    assert_eq!(count_of(&agent, ZONE_EPOCHS), 57);
}

/// A host function that changes the identifier the host answers to "-02:00".
fn next_zone(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().zone_identifier = "-02:00".to_string();
    Ok(Value::Undefined)
}

/// The identifier is read by each conversion (decision 17, item 1): a host that answers
/// "+01:00" to the read of `LocalTime` and "-02:00" to that of `UTC` gives a time value
/// computed from the two answers as given. `setFullYear` reads `month` between the two
/// conversions, so its `valueOf` changes the host's answer: local 1970-01-01T01:00 at "+01:00",
/// new local 2016-07-04T01:00, read at "-02:00" as 2016-07-04T03:00Z.
#[test]
fn each_conversion_uses_the_identifier_it_read() {
    let (mut agent, realm) = zone_agent(0);
    agent.host_mut().zone_identifier = "+01:00".to_string();
    define_function(&mut agent, &realm, "nextZone", 0, next_zone, false);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var d = new Date(0); \
             d.setFullYear(2016, { valueOf: function () { nextZone(); return 6; } }, 4);"
        ),
        normal("1467601200000")
    );
    assert_eq!(zone_calls(&agent), [ZONE_IDENTIFIER; 2]);
}

/// A host with no instant for any local time.
fn no_instants(_local: &IsoDateTime) -> Vec<i128> {
    Vec::new()
}

/// HOST-210 for a setter: a host whose epoch hook answers the empty list for every local time
/// makes `setHours` store NaN after 2 + 31 = 33 zone hook calls, and nothing is thrown.
#[test]
fn host_210_a_local_setter_with_no_instant_stores_nan_after_33_calls() {
    let mut agent = agent().unwrap();
    agent.host_mut().zone_epochs = Some(no_instants);
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var d = new Date(0); var r = d.setHours(1); String(r) + ',' + d.getTime();"
        ),
        quoted("NaN,NaN")
    );
    assert_eq!(zone_calls(&agent).len(), 33, "{:?}", zone_calls(&agent));
    assert_eq!(count_of(&agent, ZONE_IDENTIFIER), 2);
    assert_eq!(count_of(&agent, ZONE_OFFSET), 1);
    assert_eq!(count_of(&agent, ZONE_EPOCHS), 30);
}

/// The lengths, names and property attributes of the 14 (clause 18: writable, not enumerable,
/// configurable; `length` and `name` not writable, not enumerable, configurable), and none is
/// a constructor (the cases `not-a-constructor`).
#[test]
fn lengths_names_descriptors_and_not_a_constructor_of_the_14() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let mut names = Vec::new();
    let mut expected = Vec::new();
    for (local, utc, length) in SETTERS {
        for name in [local, utc] {
            names.push(format!("'{name}'"));
            expected.push(format!(
                "{name} function truefalsetrue {length} {name} falsefalsetrue falsefalsetrue \
                 TypeError"
            ));
        }
    }
    let source = format!(
        "function attributes(d) {{ return '' + d.writable + d.enumerable + d.configurable; }} \
         function row(object, key) {{ \
           var d = Object.getOwnPropertyDescriptor(object, key); \
           var f = d.value; \
           var made; \
           try {{ new f(); made = 'constructs'; }} catch (e) {{ made = e.name; }} \
           return key + ' ' + typeof f + ' ' + attributes(d) + ' ' + f.length + ' ' + f.name \
             + ' ' + attributes(Object.getOwnPropertyDescriptor(f, 'length')) + ' ' \
             + attributes(Object.getOwnPropertyDescriptor(f, 'name')) + ' ' + made; \
         }} \
         var rows = []; \
         [{}].forEach(function (k) {{ rows.push(row(Date.prototype, k)); }}); \
         rows.join('|');",
        names.join(", ")
    );
    assert_eq!(
        eval_text(&mut agent, &realm, &source),
        quoted(&expected.join("|"))
    );
}

/// Makes the global `d` of `realm` a Date of time value 0, from the host.
fn date_from_host(agent: &mut Agent<TestHost>, realm: &Realm) {
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let global = object(cx, &global_root);
            let date = support::host::get(cx, global, "Date");
            let made = cx
                .construct(date, &[Value::Number(0.0)], None)
                .unwrap()
                .unwrap();
            let key = support::host::key(cx, "d");
            cx.set(global, key, made, true).unwrap().unwrap();
        })
        .unwrap();
}

/// A step budget ends a local setter over a host with no instant with `Terminated(StepBudget)`
/// and the Date unchanged, since the store comes last; the agent stays alive. The step at which
/// the call of `d.setHours(1)` begins is measured with a host function of the same call shape
/// (`d.stop(1)`, 8 steps): `steps_per_entry` 10 ends it after the two calls of `LocalTime`,
/// before `UTC` reads the identifier; that step plus 6 ends it inside the search of `UTC`,
/// after the identifier and three calls of the epoch hook.
#[test]
fn a_step_budget_inside_the_search_leaves_the_date_unchanged() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "stop", 0, stop, false);
    date_from_host(&mut agent, &realm);
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.prototype.stop = stop; 0;"),
        normal("0")
    );
    let (outcome, begun) = steps_of(&mut agent, &realm, "d.stop(1);");
    assert_eq!(outcome, Outcome::Terminated(Termination::StepBudget));
    println!("steps at the call of d.stop(1): {begun}");

    for (steps, calls, epochs) in [(10, 2, 0), (begun + 6, 6, 3)] {
        let mut budget = limits();
        budget.steps_per_entry = steps;
        let mut agent = agent_with(budget).unwrap();
        agent.host_mut().zone_epochs = Some(no_instants);
        let realm = support::host::realm(&mut agent);
        date_from_host(&mut agent, &realm);
        assert_eq!(
            eval_text(&mut agent, &realm, "d.setHours(1);"),
            Outcome::Terminated(Termination::StepBudget),
            "{steps}"
        );
        let made = zone_calls(&agent);
        println!(
            "zone hook calls before the step budget of {steps}: {}",
            made.len()
        );
        assert_eq!(made.len(), calls, "{steps}: {made:?}");
        assert_eq!(count_of(&agent, ZONE_EPOCHS), epochs, "{steps}: {made:?}");
        assert_eq!(agent.last_entry_steps(), Ok(steps));
        assert_eq!(agent.state(), AgentState::Alive);
        assert_eq!(eval_text(&mut agent, &realm, "d.getTime();"), normal("0"));
    }
}

thread_local! {
    /// The handle `interrupting` sets.
    static HANDLE: Cell<Option<InterruptHandle>> = const { Cell::new(None) };
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

/// An interrupt raised from inside the epoch hook ends a local setter in
/// `Terminated(Interrupted)` after one call of that hook, the Date unchanged.
#[test]
fn an_interrupt_from_the_epoch_hook_leaves_the_date_unchanged() {
    let mut agent = agent().unwrap();
    agent.host_mut().zone_epochs = Some(interrupting);
    let realm = realm(&mut agent);
    date_from_host(&mut agent, &realm);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(&mut agent, &realm, "d.setHours(1);"),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(
        zone_calls(&agent),
        [
            ZONE_IDENTIFIER.to_string(),
            offset_call("UTC", 0),
            ZONE_IDENTIFIER.to_string(),
            format!("{ZONE_EPOCHS} UTC 1970-01-01T01:00:00.000"),
        ]
    );
    assert!(!agent.interrupt_handle().is_set());
    assert_eq!(eval_text(&mut agent, &realm, "d.getTime();"), normal("0"));
}

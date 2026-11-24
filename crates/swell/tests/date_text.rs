//! The text half of `Date` (ECMA-262 21.4, `#sec-date-objects`; `story:date-text-formats`):
//! `Date.parse` (`#sec-date.parse`) and the String argument of the constructor
//! (`#sec-date-constructor`, step 4.c.ii), the call without `new` (step 1), `toString`
//! (`#sec-date.prototype.tostring`) and its family, `toISOString`
//! (`#sec-date.prototype.toisostring`), `toJSON` (`#sec-date.prototype.tojson`) and
//! `[Symbol.toPrimitive]` (`#sec-date.prototype-%symbol.toprimitive%`), over the parser and the
//! writers of `swell_num::date_text` (`docs/design/engine.md`, decision 6, "`Date.parse` and the
//! String argument of `new Date`" and "The text of toLocale*String"), and the second half of
//! HOST-127 (`docs/design/host-interface.md`, decision 17).
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
    Outcome as HostOutcome, Parsed, PropertyKey, Realm, Source, Termination, Value,
};

/// The name `TestHost` records for a call of the clock hook.
const CLOCK: &str = "host_system_utc_epoch_nanoseconds";

/// Nanoseconds in an hour.
const HOUR_NS: i128 = 3_600_000_000_000;

/// The text of a normal completion whose value is a Number or a Boolean.
fn normal(text: &str) -> Outcome {
    Outcome::Normal(text.to_string())
}

/// The text of a normal completion whose value is a String, as `show` writes it.
fn quoted(text: &str) -> Outcome {
    Outcome::Normal(format!("\"{text}\""))
}

/// A throw of an error named `name`.
fn throws(name: &str) -> Outcome {
    Outcome::Throw(name.to_string())
}

/// The calls of the clock and the three time zone hooks `agent`'s host recorded, in order.
fn hook_calls(agent: &Agent<TestHost>) -> Vec<String> {
    agent
        .host()
        .calls
        .iter()
        .filter(|call| {
            call.as_str() == CLOCK
                || call.as_str() == ZONE_IDENTIFIER
                || call.starts_with(ZONE_OFFSET)
                || call.starts_with(ZONE_EPOCHS)
        })
        .cloned()
        .collect()
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

/// The global property `name` of `realm` set to a String of `text`, made by the host.
fn host_string(agent: &mut Agent<TestHost>, realm: &Realm, name: &str, text: &str) {
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let global = object(cx, &global_root);
            let string = Value::String(cx.new_string(text).unwrap());
            let key = PropertyKey::String(cx.new_string(name).unwrap());
            cx.set(global, key, string, true).unwrap().unwrap();
        })
        .unwrap();
}

/// HOST-127, both halves: with the provided time zone methods (`ProvidedHost`, a host that
/// answers only the required hooks), `new Date(0).getTimezoneOffset()` is 0 and
/// `new Date(0).toString()` is `Thu Jan 01 1970 00:00:00 GMT+0000`, which contains `GMT+0000`.
#[test]
fn host_127_with_the_provided_zone_methods_get_timezone_offset_is_zero_and_to_string_contains_gmt_0000()
 {
    let mut agent = Agent::new(ProvidedHost, limits(), seeds()).unwrap();
    let realm = match agent.create_realm(None, None).unwrap() {
        HostOutcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    };
    let mut answers = Vec::new();
    for source in [
        "new Date(0).getTimezoneOffset();",
        "new Date(0).toString();",
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
        // The value is read while the root holds it.
        let answer = agent
            .with_cx(&realm, |cx| match cx.get(&root).unwrap() {
                Value::Number(number) => format!("number {number}"),
                Value::String(text) => cx.to_rust_string_lossy(text).unwrap(),
                other => format!("{other:?}"),
            })
            .unwrap();
        answers.push(answer);
    }
    assert_eq!(answers[0], "number 0");
    assert!(answers[1].contains("GMT+0000"), "{}", answers[1]);
    assert_eq!(answers[1], "Thu Jan 01 1970 00:00:00 GMT+0000");
}

/// The three zones of [`TABLE`]: the offset the host answers (ns).
const ZONES: [i128; 3] = [0, HOUR_NS, -(5 * HOUR_NS + HOUR_NS / 2)];

/// A row of [`TABLE`]: a time value and what `toString` answers for it in each of [`ZONES`].
type Row = (f64, [&'static str; 3]);

/// The 24 time values of the getters' table of `tests/date_zone.rs` and the text of
/// `toString` for each in the three zones: written from that table's fields (from GNU
/// `date -u -d @<seconds>` of the time value plus each zone's offset, an oracle outside the
/// engine) as `DateString`, `TimeString` and `TimeZoneString` write them, not computed by the
/// engine.
const TABLE: [Row; 24] = [
    (
        -8640000000000000.0,
        [
            "Tue Apr 20 -271821 00:00:00 GMT+0000",
            "Tue Apr 20 -271821 01:00:00 GMT+0100",
            "Mon Apr 19 -271821 18:30:00 GMT-0530",
        ],
    ),
    (
        -8639999999999999.0,
        [
            "Tue Apr 20 -271821 00:00:00 GMT+0000",
            "Tue Apr 20 -271821 01:00:00 GMT+0100",
            "Mon Apr 19 -271821 18:30:00 GMT-0530",
        ],
    ),
    (
        -62167219200001.0,
        [
            "Fri Dec 31 -0001 23:59:59 GMT+0000",
            "Sat Jan 01 0000 00:59:59 GMT+0100",
            "Fri Dec 31 -0001 18:29:59 GMT-0530",
        ],
    ),
    (
        -62167219200000.0,
        [
            "Sat Jan 01 0000 00:00:00 GMT+0000",
            "Sat Jan 01 0000 01:00:00 GMT+0100",
            "Fri Dec 31 -0001 18:30:00 GMT-0530",
        ],
    ),
    (
        -2208988800000.0,
        [
            "Mon Jan 01 1900 00:00:00 GMT+0000",
            "Mon Jan 01 1900 01:00:00 GMT+0100",
            "Sun Dec 31 1899 18:30:00 GMT-0530",
        ],
    ),
    (
        -2147483648.0,
        [
            "Sun Dec 07 1969 03:28:36 GMT+0000",
            "Sun Dec 07 1969 04:28:36 GMT+0100",
            "Sat Dec 06 1969 21:58:36 GMT-0530",
        ],
    ),
    (
        -86400000.0,
        [
            "Wed Dec 31 1969 00:00:00 GMT+0000",
            "Wed Dec 31 1969 01:00:00 GMT+0100",
            "Tue Dec 30 1969 18:30:00 GMT-0530",
        ],
    ),
    (
        -1.0,
        [
            "Wed Dec 31 1969 23:59:59 GMT+0000",
            "Thu Jan 01 1970 00:59:59 GMT+0100",
            "Wed Dec 31 1969 18:29:59 GMT-0530",
        ],
    ),
    (
        0.0,
        [
            "Thu Jan 01 1970 00:00:00 GMT+0000",
            "Thu Jan 01 1970 01:00:00 GMT+0100",
            "Wed Dec 31 1969 18:30:00 GMT-0530",
        ],
    ),
    (
        1.0,
        [
            "Thu Jan 01 1970 00:00:00 GMT+0000",
            "Thu Jan 01 1970 01:00:00 GMT+0100",
            "Wed Dec 31 1969 18:30:00 GMT-0530",
        ],
    ),
    (
        86399999.0,
        [
            "Thu Jan 01 1970 23:59:59 GMT+0000",
            "Fri Jan 02 1970 00:59:59 GMT+0100",
            "Thu Jan 01 1970 18:29:59 GMT-0530",
        ],
    ),
    (
        951782400000.0,
        [
            "Tue Feb 29 2000 00:00:00 GMT+0000",
            "Tue Feb 29 2000 01:00:00 GMT+0100",
            "Mon Feb 28 2000 18:30:00 GMT-0530",
        ],
    ),
    (
        951868799999.0,
        [
            "Tue Feb 29 2000 23:59:59 GMT+0000",
            "Wed Mar 01 2000 00:59:59 GMT+0100",
            "Tue Feb 29 2000 18:29:59 GMT-0530",
        ],
    ),
    (
        1456704000000.0,
        [
            "Mon Feb 29 2016 00:00:00 GMT+0000",
            "Mon Feb 29 2016 01:00:00 GMT+0100",
            "Sun Feb 28 2016 18:30:00 GMT-0530",
        ],
    ),
    (
        1456790399999.0,
        [
            "Mon Feb 29 2016 23:59:59 GMT+0000",
            "Tue Mar 01 2016 00:59:59 GMT+0100",
            "Mon Feb 29 2016 18:29:59 GMT-0530",
        ],
    ),
    (
        2147483647.0,
        [
            "Sun Jan 25 1970 20:31:23 GMT+0000",
            "Sun Jan 25 1970 21:31:23 GMT+0100",
            "Sun Jan 25 1970 15:01:23 GMT-0530",
        ],
    ),
    (
        2147483648.0,
        [
            "Sun Jan 25 1970 20:31:23 GMT+0000",
            "Sun Jan 25 1970 21:31:23 GMT+0100",
            "Sun Jan 25 1970 15:01:23 GMT-0530",
        ],
    ),
    (
        4294967296.0,
        [
            "Thu Feb 19 1970 17:02:47 GMT+0000",
            "Thu Feb 19 1970 18:02:47 GMT+0100",
            "Thu Feb 19 1970 11:32:47 GMT-0530",
        ],
    ),
    (
        1234567890123.0,
        [
            "Fri Feb 13 2009 23:31:30 GMT+0000",
            "Sat Feb 14 2009 00:31:30 GMT+0100",
            "Fri Feb 13 2009 18:01:30 GMT-0530",
        ],
    ),
    (
        253402300799999.0,
        [
            "Fri Dec 31 9999 23:59:59 GMT+0000",
            "Sat Jan 01 10000 00:59:59 GMT+0100",
            "Fri Dec 31 9999 18:29:59 GMT-0530",
        ],
    ),
    (
        253402300800000.0,
        [
            "Sat Jan 01 10000 00:00:00 GMT+0000",
            "Sat Jan 01 10000 01:00:00 GMT+0100",
            "Fri Dec 31 9999 18:30:00 GMT-0530",
        ],
    ),
    (
        8639999999999999.0,
        [
            "Fri Sep 12 275760 23:59:59 GMT+0000",
            "Sat Sep 13 275760 00:59:59 GMT+0100",
            "Fri Sep 12 275760 18:29:59 GMT-0530",
        ],
    ),
    (
        8640000000000000.0,
        [
            "Sat Sep 13 275760 00:00:00 GMT+0000",
            "Sat Sep 13 275760 01:00:00 GMT+0100",
            "Fri Sep 12 275760 18:30:00 GMT-0530",
        ],
    ),
    (
        1000000000000000.0,
        [
            "Fri Sep 27 33658 01:46:40 GMT+0000",
            "Fri Sep 27 33658 02:46:40 GMT+0100",
            "Thu Sep 26 33658 20:16:40 GMT-0530",
        ],
    ),
];

/// `toString` (`#sec-date.prototype.tostring`, `ToDateString`) over the 24 time values of
/// [`TABLE`] at the three offsets of [`ZONES`] (0, +1 hour, -5.5 hours): 72 calls, each made
/// from the host with the Date as `this` and each text compared with the table's; each call
/// makes two hook calls (the identifier and the offset) and the text ends with the offset of
/// the local time it printed. A Date whose time value is NaN answers `Invalid Date` with no
/// hook call.
#[test]
fn to_string_over_a_table_of_24_time_values_at_three_offsets() {
    let started = Instant::now();
    let mut calls = 0;
    for (zone, offset_ns) in ZONES.into_iter().enumerate() {
        let (mut agent, realm) = zone_agent(offset_ns);
        let global_root = global(&mut agent, &realm);
        let texts = agent
            .with_cx(&realm, |cx| {
                let global = object(cx, &global_root);
                let date = support::host::get(cx, global, "Date");
                let Value::Object(constructor) = date else {
                    panic!("Date is {date:?}");
                };
                let prototype = support::host::get_object(cx, constructor, "prototype");
                let to_string = support::host::get(cx, prototype, "toString");
                TABLE
                    .iter()
                    .map(|&(t, _)| t)
                    .chain([f64::NAN])
                    .map(|t| {
                        let made = cx
                            .construct(date, &[Value::Number(t)], None)
                            .unwrap()
                            .unwrap();
                        let text = cx.call(to_string, made, &[]).unwrap().unwrap();
                        support::host::text(cx, text)
                    })
                    .collect::<Vec<String>>()
            })
            .unwrap();
        for ((t, expected), text) in TABLE.iter().zip(&texts) {
            assert_eq!(text, expected[zone], "t {t}, offset {offset_ns} ns");
            calls += 1;
        }
        assert_eq!(texts.last().map(String::as_str), Some("Invalid Date"));
        assert_eq!(
            hook_calls(&agent).len(),
            TABLE.len() * 2,
            "offset {offset_ns}"
        );
    }
    assert_eq!(calls, 72);
    println!(
        "toString: {calls} calls over {} time values and {} zones in {} ms",
        TABLE.len(),
        ZONES.len(),
        started.elapsed().as_millis()
    );
}

/// `toString` of `new Date(0)` with a host offset of 3,600 × 10^9 ns is
/// `Thu Jan 01 1970 01:00:00 GMT+0100`: the local time and the `GMT+0100` of one offset answer.
#[test]
fn to_string_of_the_epoch_at_an_offset_of_one_hour() {
    let (mut agent, realm) = zone_agent(HOUR_NS);
    assert_eq!(
        eval_text(&mut agent, &realm, "new Date(0).toString();"),
        quoted("Thu Jan 01 1970 01:00:00 GMT+0100")
    );
    assert_eq!(
        hook_calls(&agent),
        [ZONE_IDENTIFIER.to_string(), offset_call("UTC", 0)]
    );
}

/// The hook calls of each text and of the parse (decision 17, items 1, 2 and 5): `toString`,
/// `toDateString`, `toTimeString` and the three `toLocale*String` make one conversion, 2 calls
/// (the identifier and the offset), 1 with an offset-string identifier, none for NaN;
/// `toUTCString` and `toISOString` none; `Date()` 3 (the clock, the identifier, the offset);
/// `Date.parse` and `new Date(string)` of a date-time form without an offset 3 on the common
/// path and 1 with an offset-string identifier (at most 59: 59 across a gap of 48 hours), of a
/// form with `Z` or an offset and of the date-only form none.
#[test]
fn hook_calls_of_each_text_and_of_the_parse() {
    let identifier = || ZONE_IDENTIFIER.to_string();
    let conversion = vec![identifier(), offset_call("UTC", 0)];
    let rows: [(&str, Vec<String>); 16] = [
        ("new Date(0).toString();", conversion.clone()),
        ("new Date(0).toDateString();", conversion.clone()),
        ("new Date(0).toTimeString();", conversion.clone()),
        ("new Date(0).toLocaleString();", conversion.clone()),
        ("new Date(0).toLocaleDateString();", conversion.clone()),
        ("new Date(0).toLocaleTimeString();", conversion.clone()),
        ("new Date(NaN).toString();", Vec::new()),
        ("new Date(NaN).toTimeString();", Vec::new()),
        ("new Date(0).toUTCString();", Vec::new()),
        ("new Date(0).toISOString();", Vec::new()),
        ("new Date(0).toJSON();", Vec::new()),
        (
            "Date();",
            vec![CLOCK.to_string(), identifier(), offset_call("UTC", 0)],
        ),
        (
            "Date.parse('2020-01-01T00:00:00');",
            vec![
                identifier(),
                format!("{ZONE_EPOCHS} UTC 2020-01-01T00:00:00.000"),
                offset_call("UTC", 1_577_836_800_000_000_000),
            ],
        ),
        (
            "new Date('2020-01-01T00:00');",
            vec![
                identifier(),
                format!("{ZONE_EPOCHS} UTC 2020-01-01T00:00:00.000"),
                offset_call("UTC", 1_577_836_800_000_000_000),
            ],
        ),
        (
            "Date.parse('2020-01-01T00:00:00Z') + Date.parse('2020-01-01T01:00+01:00');",
            Vec::new(),
        ),
        (
            "Date.parse('2020-01-01') + new Date('2020').getTime();",
            Vec::new(),
        ),
    ];
    let (mut agent, realm) = zone_agent(0);
    for (source, expected) in rows {
        agent.host_mut().calls.clear();
        let outcome = eval_text(&mut agent, &realm, source);
        assert!(
            matches!(outcome, Outcome::Normal(_)),
            "{source}: {outcome:?}"
        );
        let calls = hook_calls(&agent);
        assert!(calls.len() <= 59, "{source}: {}", calls.len());
        assert_eq!(calls, expected, "{source}");
    }

    let (mut agent, realm) = zone_agent(2 * HOUR_NS);
    agent.host_mut().zone_identifier = "+01:00".to_string();
    for (source, expected) in [
        (
            "new Date(0).toString();",
            "\"Thu Jan 01 1970 01:00:00 GMT+0100\"",
        ),
        ("Date.parse('1970-01-01T01:00:00');", "0"),
    ] {
        agent.host_mut().calls.clear();
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            normal(expected),
            "{source}"
        );
        assert_eq!(hook_calls(&agent), [identifier()], "{source}");
    }

    let mut agent = support::host::agent().unwrap();
    agent.host_mut().zone_epochs = Some(gap_48);
    let realm = support::host::realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.parse('2021-03-29T23:59:59.999');"),
        normal("1617062399999")
    );
    assert_eq!(hook_calls(&agent).len(), 59, "{:?}", hook_calls(&agent));
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

/// A host function that changes the identifier the host answers.
fn next_zone(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().zone_identifier = "+02:00".to_string();
    Ok(Value::Undefined)
}

/// Each conversion reads the identifier anew (decision 17, item 1): `toString` after
/// `getHours` on one Date, with the host's identifier changed between them, prints the time
/// and the offset of the new identifier.
#[test]
fn to_string_after_get_hours_reads_the_identifier_anew() {
    let (mut agent, realm) = zone_agent(0);
    agent.host_mut().zone_identifier = "+01:00".to_string();
    define_function(&mut agent, &realm, "nextZone", 0, next_zone, false);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var d = new Date(0); var a = d.getHours(); nextZone(); a + ',' + d.toString();"
        ),
        quoted("1,Thu Jan 01 1970 02:00:00 GMT+0200")
    );
    assert_eq!(hook_calls(&agent), [ZONE_IDENTIFIER; 2]);
}

/// `Date.parse` and `new Date(string)` over every form and rejection of decision 6 ("`Date.parse`
/// and the String argument of `new Date`"), in the zone "UTC": the Date Time String Format
/// with each of its lengths, expanded years, the length of each month, hour 24, an offset; the
/// forms of `toString` (with and without a name) and `toUTCString`; the instants at both ends of
/// the time range and one beyond each; and the rejections (a space before or after, a lower case
/// letter, a second space, out-of-range elements, a missing or an extra element, the forms of
/// `toDateString` and `toTimeString`). `Date.parse` of a value that is not a String reads its
/// ToString; `new Date(string)` answers what `Date.parse` answers.
#[test]
fn date_parse_and_new_date_over_every_form_and_rejection() {
    let rows: [(&str, &str); 58] = [
        ("2020", "1577836800000"),
        ("2020-02", "1580515200000"),
        ("2020-02-29", "1582934400000"),
        ("2020-01-01T00:00", "1577836800000"),
        ("2020-01-01T00:00:01", "1577836801000"),
        ("2020-01-01T00:00:00.123", "1577836800123"),
        ("2020-01-01T00:00:00.123Z", "1577836800123"),
        ("2020-01-01T01:00:00+01:00", "1577836800000"),
        ("2020-01-01T00:00:00-05:30", "1577856600000"),
        ("2020-01-01T24:00:00Z", "1577923200000"),
        ("2020-01-01T24:00Z", "1577923200000"),
        ("+002020-01-01T00:00:00Z", "1577836800000"),
        ("-000001-01-01T00:00:00Z", "-62198755200000"),
        ("+275760-09-13T00:00:00.000Z", "8640000000000000"),
        ("+275760-09-13T01:00:00+01:00", "8640000000000000"),
        ("-271821-04-20T00:00:00.000Z", "-8640000000000000"),
        ("+275760-09-13T00:00:00.001Z", "NaN"),
        ("-271821-04-19T23:59:59.999Z", "NaN"),
        ("-000000-01-01T00:00:00Z", "NaN"),
        ("2021-02-29", "NaN"),
        ("2020-04-31", "NaN"),
        ("2020-13-01", "NaN"),
        ("2020-00-01", "NaN"),
        ("2020-01-00", "NaN"),
        ("2020-01-01Z", "NaN"),
        ("2020-01-01T24:00:01Z", "NaN"),
        ("2020-01-01T23:60Z", "NaN"),
        ("2020-01-01T00:00:60Z", "NaN"),
        ("2020-01-01T00:00:00.12Z", "NaN"),
        ("2020-01-01T00:00:00+24:00", "NaN"),
        ("2020-01-01T00:00:00+0100", "NaN"),
        ("2020-01-01T00:00:00z", "NaN"),
        ("2020-01-01T00", "NaN"),
        ("20-01-01", "NaN"),
        (" 2020-01-01", "NaN"),
        ("2020-01-01 ", "NaN"),
        ("", "NaN"),
        ("garbage", "NaN"),
        ("Thu Jan 01 1970 00:00:00 GMT+0000", "0"),
        ("Thu Jan 01 1970 01:00:00 GMT+0100", "0"),
        ("Wed Dec 31 1969 18:30:00 GMT-0530", "0"),
        (
            "Thu Jan 01 1970 00:00:00 GMT+0000 (Coordinated Universal Time)",
            "0",
        ),
        ("Mon Jan 01 1970 00:00:00 GMT+0000", "0"),
        ("Fri Dec 31 -0001 00:00:00 GMT+0000", "-62167305600000"),
        ("Sat Sep 13 275760 00:00:00 GMT+0000", "8640000000000000"),
        ("Tue Apr 20 -271821 00:00:00 GMT+0000", "-8640000000000000"),
        ("Sat Sep 13 275760 00:00:01 GMT+0000", "NaN"),
        ("Thu Jan 01 1970 00:00:00 GMT+0000 ()", "NaN"),
        ("Thu Jan 01 1970 00:00:00 GMT+0000 (a(b))", "NaN"),
        ("Thu Jan 01 -0000 00:00:00 GMT+0000", "NaN"),
        ("Thu Jan  1 1970 00:00:00 GMT+0000", "NaN"),
        ("thu Jan 01 1970 00:00:00 GMT+0000", "NaN"),
        ("Thu Feb 30 2020 00:00:00 GMT+0000", "NaN"),
        ("Thu Jan 01 1970 00:00:00 GMT+2400", "NaN"),
        ("Thu, 01 Jan 1970 00:00:00 GMT", "0"),
        ("Thu, 01 Jan 1970 00:00:00 GMT+0000", "NaN"),
        ("Thu Jan 01 1970", "NaN"),
        ("00:00:00 GMT+0000", "NaN"),
    ];
    let (mut agent, realm) = zone_agent(0);
    for (text, expected) in rows {
        assert_eq!(
            eval_text(&mut agent, &realm, &format!("Date.parse('{text}');")),
            normal(expected),
            "Date.parse('{text}')"
        );
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                &format!("new Date('{text}').getTime();")
            ),
            normal(expected),
            "new Date('{text}')"
        );
    }
    for (source, expected) in [
        (
            "Date.parse({ toString: function () { return '1970-01-01'; } });",
            normal("0"),
        ),
        ("Date.parse();", normal("NaN")),
        ("Date.parse(2020);", normal("1577836800000")),
        (
            "new Date(new String('1970-01-01T00:00:00.000Z')).getTime();",
            normal("0"),
        ),
        (
            "Date.parse({ toString: function () { throw new RangeError('poisoned'); } });",
            throws("RangeError"),
        ),
        ("Date.parse(Symbol());", throws("TypeError")),
    ] {
        assert_eq!(eval_text(&mut agent, &realm, source), expected, "{source}");
    }
}

/// `Date.parse` reads the text of `toString`, `toUTCString`, `toISOString` and
/// `toLocaleString` (decision 6, "The text of toLocale*String"): `Date.parse(d.toString())` is
/// `d` to the second, and so are the texts of `toUTCString` and `toLocaleString`;
/// `Date.parse(d.toISOString())` is `d`; the texts of `toDateString`, `toTimeString`,
/// `toLocaleDateString` and `toLocaleTimeString` are NaN; and each `toLocale*String` is the
/// text of its counterpart. Over eight time values at the three offsets of [`ZONES`].
#[test]
fn date_parse_reads_back_the_text_of_each_writer() {
    let source = "\
        var times = [0, -1, 1234567890123, 951868799999, -62167219200001, 8.64e15, -8.64e15, \
          253402300800000]; \
        var rows = []; \
        times.forEach(function (t) { \
          var d = new Date(t); var s = t - (((t % 1000) + 1000) % 1000); \
          rows.push([Date.parse(d.toString()) === s, Date.parse(d.toUTCString()) === s, \
            Date.parse(d.toISOString()) === t, Date.parse(d.toLocaleString()) === s, \
            isNaN(Date.parse(d.toDateString())), isNaN(Date.parse(d.toTimeString())), \
            isNaN(Date.parse(d.toLocaleDateString())), \
            isNaN(Date.parse(d.toLocaleTimeString())), \
            d.toLocaleString() === d.toString(), d.toLocaleDateString() === d.toDateString(), \
            d.toLocaleTimeString() === d.toTimeString()].join()); \
        }); \
        rows.join('|');";
    let row = ["true"; 11].join(",");
    let expected = vec![row; 8].join("|");
    for offset_ns in ZONES {
        let (mut agent, realm) = zone_agent(offset_ns);
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            quoted(&expected),
            "offset {offset_ns}"
        );
    }
}

/// The writers but `toString` (`#sec-date.prototype.todatestring`,
/// `#sec-date.prototype.totimestring`, `#sec-date.prototype.toutcstring`,
/// `#sec-date.prototype.toisostring`): their texts for the epoch, the ends of the time range and
/// years past four digits and below 0; `Invalid Date` for NaN from all but `toISOString`, which
/// throws a RangeError; and a TypeError for a `this` that is not a Date from each of the nine
/// that are not generic, `%Date.prototype%` included.
#[test]
fn the_writers_over_a_table() {
    let rows: [(&str, Outcome); 21] = [
        ("new Date(0).toDateString();", quoted("Thu Jan 01 1970")),
        ("new Date(0).toTimeString();", quoted("00:00:00 GMT+0000")),
        (
            "new Date(0).toUTCString();",
            quoted("Thu, 01 Jan 1970 00:00:00 GMT"),
        ),
        (
            "new Date(0).toISOString();",
            quoted("1970-01-01T00:00:00.000Z"),
        ),
        (
            "new Date(-1).toISOString();",
            quoted("1969-12-31T23:59:59.999Z"),
        ),
        (
            "new Date(8.64e15).toISOString();",
            quoted("+275760-09-13T00:00:00.000Z"),
        ),
        (
            "new Date(-8.64e15).toISOString();",
            quoted("-271821-04-20T00:00:00.000Z"),
        ),
        (
            "new Date(-62167219200001).toISOString();",
            quoted("-000001-12-31T23:59:59.999Z"),
        ),
        (
            "new Date(253402300800000).toISOString();",
            quoted("+010000-01-01T00:00:00.000Z"),
        ),
        (
            "new Date(-8.64e15).toUTCString();",
            quoted("Tue, 20 Apr -271821 00:00:00 GMT"),
        ),
        (
            "new Date(8.64e15).toUTCString();",
            quoted("Sat, 13 Sep 275760 00:00:00 GMT"),
        ),
        (
            "new Date(-62167219200001).toDateString();",
            quoted("Fri Dec 31 -0001"),
        ),
        ("new Date(NaN).toISOString();", throws("RangeError")),
        ("new Date(NaN).toString();", quoted("Invalid Date")),
        ("new Date(NaN).toDateString();", quoted("Invalid Date")),
        ("new Date(NaN).toTimeString();", quoted("Invalid Date")),
        ("new Date(NaN).toUTCString();", quoted("Invalid Date")),
        ("new Date(NaN).toLocaleString();", quoted("Invalid Date")),
        (
            "new Date(NaN).toLocaleDateString();",
            quoted("Invalid Date"),
        ),
        (
            "new Date(NaN).toLocaleTimeString();",
            quoted("Invalid Date"),
        ),
        ("String(new Date(NaN));", quoted("Invalid Date")),
    ];
    let (mut agent, realm) = zone_agent(0);
    for (source, expected) in rows {
        assert_eq!(eval_text(&mut agent, &realm, source), expected, "{source}");
    }
    for name in [
        "toString",
        "toDateString",
        "toTimeString",
        "toUTCString",
        "toISOString",
        "toLocaleString",
        "toLocaleDateString",
        "toLocaleTimeString",
    ] {
        for this in ["{}", "0", "Date.prototype", "undefined"] {
            let source = format!("Date.prototype.{name}.call({this});");
            assert_eq!(
                eval_text(&mut agent, &realm, &source),
                throws("TypeError"),
                "{source}"
            );
        }
    }
    agent.host_mut().calls.clear();
    let source = "Date.prototype[Symbol.toPrimitive].call(1, 'number');";
    assert_eq!(eval_text(&mut agent, &realm, source), throws("TypeError"));
    assert!(hook_calls(&agent).is_empty(), "{:?}", hook_calls(&agent));
}

/// `Date.prototype.toJSON ( key )` (`#sec-date.prototype.tojson`): ToObject(this),
/// ToPrimitive(O, number), null for a Number that is not finite, else Invoke(O,
/// "toISOString") with no argument. It is generic; an abrupt ToObject, ToPrimitive or
/// `toISOString` goes up; the key is not read.
#[test]
fn to_json_over_its_steps() {
    let rows: [(&str, Outcome); 12] = [
        ("new Date(0).toJSON();", quoted("1970-01-01T00:00:00.000Z")),
        ("new Date(NaN).toJSON();", normal("null")),
        (
            "JSON.stringify(new Date(0));",
            quoted("\"1970-01-01T00:00:00.000Z\""),
        ),
        (
            "JSON.stringify({ d: new Date(NaN) });",
            quoted("{\"d\":null}"),
        ),
        (
            "Date.prototype.toJSON.call({ valueOf: function () { return Infinity; }, \
               toISOString: function () { return 'iso'; } });",
            normal("null"),
        ),
        (
            "Date.prototype.toJSON.call({ toISOString: function () { \
               return arguments.length + ',' + typeof this; } }, 'key');",
            quoted("0,object"),
        ),
        (
            "var log = ''; try { Date.prototype.toJSON.call({ toISOString: function () { \
               log += arguments.length; throw new RangeError('poisoned'); } }); } \
             catch (e) { log += e.name; } log;",
            quoted("0RangeError"),
        ),
        (
            "var called = false; var r = 'no throw'; try { Date.prototype.toJSON.call({ \
               valueOf: function () { throw new SyntaxError('valueOf'); }, \
               toISOString: function () { called = true; } }); } \
             catch (e) { r = e.name + ',' + called; } r;",
            quoted("SyntaxError,false"),
        ),
        (
            "Number.prototype.toISOString = function () { return typeof this + ',' + this; }; \
             var r = Date.prototype.toJSON.call(5); delete Number.prototype.toISOString; r;",
            quoted("object,5"),
        ),
        (
            "Date.prototype.toJSON.call(undefined);",
            throws("TypeError"),
        ),
        ("Date.prototype.toJSON.call({});", throws("TypeError")),
        (
            "var d = new Date(0); d.toISOString = 1; Date.prototype.toJSON.call(d);",
            throws("TypeError"),
        ),
    ];
    let (mut agent, realm) = zone_agent(0);
    for (source, expected) in rows {
        assert_eq!(eval_text(&mut agent, &realm, source), expected, "{source}");
    }
}

/// `Date.prototype [ %Symbol.toPrimitive% ] ( hint )`
/// (`#sec-date.prototype-%symbol.toprimitive%`): "string" and "default" try `toString` and then
/// `valueOf`, "number" the reverse (OrdinaryToPrimitive); a hint that is not one of the three
/// Strings, a value that is not a String included, throws a TypeError; a `this` that is not an
/// Object throws a TypeError before the hint is read; it is generic over objects; and the
/// operators reach it: `+` with no hint, `-` with "number".
#[test]
fn symbol_to_primitive_with_each_hint_and_an_invalid_one() {
    let setup = "var d = new Date(0); \
        d.toString = function () { return 's'; }; d.valueOf = function () { return 'v'; }; \
        var p = Date.prototype[Symbol.toPrimitive]; 0;";
    let rows: [(&str, Outcome); 20] = [
        ("d[Symbol.toPrimitive]('string');", quoted("s")),
        ("d[Symbol.toPrimitive]('default');", quoted("s")),
        ("d[Symbol.toPrimitive]('number');", quoted("v")),
        (
            "var o = { toString: function () { return {}; }, valueOf: function () { return 7; } }; \
             p.call(o, 'string');",
            normal("7"),
        ),
        (
            "var o = { toString: function () { return 's'; }, valueOf: function () { return {}; } }; \
             p.call(o, 'number');",
            quoted("s"),
        ),
        (
            "p.call({ toString: function () { return {}; }, valueOf: function () { return {}; } }, \
               'default');",
            throws("TypeError"),
        ),
        ("d[Symbol.toPrimitive]();", throws("TypeError")),
        ("d[Symbol.toPrimitive](undefined);", throws("TypeError")),
        ("d[Symbol.toPrimitive](null);", throws("TypeError")),
        ("d[Symbol.toPrimitive]('');", throws("TypeError")),
        ("d[Symbol.toPrimitive]('String');", throws("TypeError")),
        (
            "d[Symbol.toPrimitive]('defaultnumber');",
            throws("TypeError"),
        ),
        ("d[Symbol.toPrimitive](1);", throws("TypeError")),
        (
            "d[Symbol.toPrimitive]({ toString: function () { return 'number'; } });",
            throws("TypeError"),
        ),
        (
            "d[Symbol.toPrimitive](new String('number'));",
            throws("TypeError"),
        ),
        ("p.call(undefined, 'number');", throws("TypeError")),
        ("p.call('x', 'string');", throws("TypeError")),
        (
            "var log = ''; try { p.call(1, { toString: function () { log += 'h'; return 'number'; } }); } \
             catch (e) { log += e.name; } log;",
            quoted("TypeError"),
        ),
        (
            "new Date(0) + 1;",
            quoted("Thu Jan 01 1970 00:00:00 GMT+00001"),
        ),
        (
            "(new Date(5) - 1) + ',' + `${new Date(0)}`;",
            quoted("4,Thu Jan 01 1970 00:00:00 GMT+0000"),
        ),
    ];
    let (mut agent, realm) = zone_agent(0);
    assert_eq!(eval_text(&mut agent, &realm, setup), normal("0"));
    for (source, expected) in rows {
        assert_eq!(eval_text(&mut agent, &realm, source), expected, "{source}");
    }
}

/// The lengths, names and property attributes of the 10 functions of this story (clause 18:
/// writable, not enumerable, configurable; `length` and `name` not writable, not enumerable,
/// configurable; `toJSON` has `length` 1, the others 0) and of `[Symbol.toPrimitive]`
/// (`length` 1, `name` "[Symbol.toPrimitive]", { [[Writable]]: false, [[Enumerable]]: false,
/// [[Configurable]]: true }); none is a constructor.
#[test]
fn lengths_names_and_descriptors_of_the_ten_functions() {
    let source = "\
        function attributes(d) { return '' + d.writable + d.enumerable + d.configurable; } \
        function row(key) { \
          var d = Object.getOwnPropertyDescriptor(Date.prototype, key); \
          var f = d.value; var made; \
          try { new f(); made = 'constructs'; } catch (e) { made = e.name; } \
          return typeof f + ' ' + attributes(d) + ' ' + f.length + ' ' + f.name + ' ' \
            + attributes(Object.getOwnPropertyDescriptor(f, 'length')) + ' ' \
            + attributes(Object.getOwnPropertyDescriptor(f, 'name')) + ' ' + made; \
        } \
        ['toString', 'toDateString', 'toTimeString', 'toUTCString', 'toISOString', 'toJSON', \
         'toLocaleString', 'toLocaleDateString', 'toLocaleTimeString', Symbol.toPrimitive] \
          .map(row).join('|');";
    let functions = [
        ("toString", 0),
        ("toDateString", 0),
        ("toTimeString", 0),
        ("toUTCString", 0),
        ("toISOString", 0),
        ("toJSON", 1),
        ("toLocaleString", 0),
        ("toLocaleDateString", 0),
        ("toLocaleTimeString", 0),
    ];
    let mut expected: Vec<String> = functions
        .iter()
        .map(|(name, length)| {
            format!(
                "function truefalsetrue {length} {name} falsefalsetrue falsefalsetrue TypeError"
            )
        })
        .collect();
    expected.push(
        "function falsefalsetrue 1 [Symbol.toPrimitive] falsefalsetrue falsefalsetrue TypeError"
            .to_string(),
    );
    let (mut agent, realm) = zone_agent(0);
    assert_eq!(
        eval_text(&mut agent, &realm, source),
        quoted(&expected.join("|"))
    );
}

/// A String is charged before it is read (decision 6; `docs/design/engine.md`, "The step
/// budget"): `Date.parse` of a String of up to 32 code units costs the steps of the call, each
/// code unit past 32 one step more (33 units 1, 1,000 units 968), whatever the text. A String of
/// 100,000 code units that the host made, of the form of `toString` with a name, answers 0 under
/// the test limits; under `steps_per_entry` 2,000 its 99,968 steps end `Date.parse` in
/// `Terminated(StepBudget)` before a unit is read: no hook is called and no value is answered,
/// and the agent runs the next entry.
#[test]
fn a_long_string_ends_date_parse_at_the_step_budget_before_it_is_read() {
    let name = "x".repeat(100_000 - 36);
    let long = format!("Thu Jan 01 1970 00:00:00 GMT+0000 ({name})");
    assert_eq!(long.len(), 100_000);
    let (mut agent, realm) = zone_agent(0);
    for (global_name, length) in [("s1", 1), ("s32", 32), ("s33", 33), ("s1000", 1_000)] {
        host_string(&mut agent, &realm, global_name, &"x".repeat(length));
    }
    host_string(&mut agent, &realm, "long", &long);
    let mut steps = Vec::new();
    for source in [
        "Date.parse(s1);",
        "Date.parse(s32);",
        "Date.parse(s33);",
        "Date.parse(s1000);",
    ] {
        let (outcome, spent) = steps_of(&mut agent, &realm, source);
        assert_eq!(outcome, normal("NaN"), "{source}");
        steps.push(spent);
    }
    println!("steps of Date.parse of 1, 32, 33 and 1,000 code units: {steps:?}");
    assert_eq!(steps[1], steps[0]);
    assert_eq!(steps[2], steps[0] + 1);
    assert_eq!(steps[3], steps[0] + 968);
    let (outcome, spent) = steps_of(&mut agent, &realm, "Date.parse(long);");
    assert_eq!(outcome, normal("0"));
    assert_eq!(spent, steps[0] + 99_968);

    let mut tight = limits();
    tight.steps_per_entry = 2_000;
    let mut agent = agent_with(tight).unwrap();
    let realm = support::host::realm(&mut agent);
    host_string(&mut agent, &realm, "long", &long);
    agent.host_mut().calls.clear();
    assert_eq!(
        eval_text(&mut agent, &realm, "var answer = Date.parse(long); answer;"),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert!(hook_calls(&agent).is_empty(), "{:?}", hook_calls(&agent));
    // The charge of the 99,968 steps is refused whole: the entry keeps the steps it had spent.
    let spent = agent.last_entry_steps().unwrap();
    println!("steps of the entry ended at the budget of 2,000: {spent}");
    assert!(spent < 2_000, "{spent}");
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "typeof answer + ',' + Date.parse('1970-01-01');"
        ),
        quoted("undefined,0")
    );
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

/// An interrupt the host raises from inside the epoch hook during
/// `Date.parse("2020-01-01T00:00:00")` ends the entry in `Terminated(Interrupted)` after one
/// call of the epoch hook: the step before the next hook call reads it. The next entry runs.
#[test]
fn an_interrupt_from_the_epoch_hook_ends_date_parse() {
    let mut agent = agent().unwrap();
    agent.host_mut().zone_epochs = Some(interrupting);
    let realm = realm(&mut agent);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.parse('2020-01-01T00:00:00');"),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(
        hook_calls(&agent),
        [
            ZONE_IDENTIFIER.to_string(),
            format!("{ZONE_EPOCHS} UTC 2020-01-01T00:00:00.000")
        ]
    );
    assert!(!agent.interrupt_handle().is_set());
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.parse('2020-01-01T00:00:00Z');"),
        normal("1577836800000")
    );
}

/// An epoch hook that answers a million instants.
fn a_million(local: &IsoDateTime) -> Vec<i128> {
    vec![local_ms(local) * 1_000_000; 1_000_000]
}

/// The bytes an agent with the test limits holds after `create_realm`, which leaves nothing
/// for a collection to free: the reading the memory limit of
/// [`a_million_instants_end_date_parse_at_the_memory_budget`] is fitted to, whatever the size
/// of the realm.
fn realm_bytes() -> u64 {
    let mut agent = agent().unwrap();
    let _realm = realm(&mut agent);
    let made = agent.heap_bytes().unwrap();
    agent.collect().unwrap();
    assert_eq!(agent.heap_bytes().unwrap(), made);
    made
}

/// The list of the epoch hook is charged 16 bytes per instant before one is read (decision 17,
/// item 7): with a memory limit of the realm's bytes and 1 MiB of room, the 16,000,000 bytes of
/// a million instants end `Date.parse("2020-01-01T00:00:00")` in `Terminated(MemoryBudget)`
/// with no offset call, and the next entry runs.
#[test]
fn a_million_instants_end_date_parse_at_the_memory_budget() {
    let realm_bytes = realm_bytes();
    let mut budget = limits();
    budget.memory_bytes = realm_bytes + (1 << 20);
    let mut agent = agent_with(budget).unwrap();
    agent.host_mut().zone_epochs = Some(a_million);
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.parse('2020-01-01T00:00:00');"),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    assert_eq!(
        hook_calls(&agent),
        [
            ZONE_IDENTIFIER.to_string(),
            format!("{ZONE_EPOCHS} UTC 2020-01-01T00:00:00.000")
        ]
    );
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "Date.parse('2020-01-01T00:00:00Z');"),
        normal("1577836800000")
    );
    println!(
        "realm {realm_bytes} bytes, limit {} bytes",
        realm_bytes + (1 << 20)
    );
}

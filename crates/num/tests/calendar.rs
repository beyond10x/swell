//! The calendar of a time value and the conversions between local time and UTC
//! (`#sec-time-values-and-time-range` to `#sec-makefullyear`, `#sec-localtime`, `#sec-utc-t`,
//! `#sec-istimezoneoffsetstring`, `#sec-parsetimezoneoffsetstring`), tested without an engine.
//!
//! Where the numbers come from:
//! - -8.64 × 10^15 and 8.64 × 10^15 are the ends of the time value range
//!   (`#sec-time-values-and-time-range`); their dates and weekdays, and those of one day beyond
//!   each, were printed by `date -u -d @<seconds> '+%Y-%m-%d %a'` (`@-8640000000000`
//!   -271821-04-20 Tue, `@8640000000000` 275760-09-13 Sat, `@8640000086400` 275760-09-14 Sun,
//!   `@-8640000086400` -271821-04-19 Mon, `@0` Thu);
//! - the two New York cases are the notes of `#sec-utc-t`; their instants were printed by
//!   `date -u -d '<instant>' +%s` (`2017-03-12T07:00:00Z` 1489302000, `2017-11-05T06:00:00Z`
//!   1509861600, `2017-03-12T02:30:00Z` 1489285800, `2017-11-05T01:30:00Z` 1509845400);
//! - the bounds of the search (at most 57 asks, 30 with no instant, 2^28 ms) are those of
//!   `docs/design/engine.md` decision 6, "`UTC ( t )` and the local time that does not exist";
//!   the clamp is HOST-209 (`docs/design/host-interface.md` decision 17, item 3);
//! - the walk's day counts: 201 years of 365 days and 49 leap days from 1900 to 2100 (73,414);
//!   4,001 years and 970 leap days from -1000 to 3000 (1,461,335).

use std::cell::Cell;
use std::time::Instant;

use swell_num::date::{
    DateFields, IsoDateTime, LOCAL_TIME_LIMIT, MS_PER_DAY, date_fields, get_utc_epoch_nanoseconds,
    local_time, make_day, make_full_year, offset_milliseconds, parse_time_zone_offset_string,
    utc_from_local,
};

/// Whether the walk runs its full count (the lane `slow` sets `SWELL_FULL_COUNTS=1`).
fn full_counts() -> bool {
    std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

const NS_PER_MS: i128 = 1_000_000;
const HOUR: i64 = 3_600_000;

// ---------------------------------------------------------------------------------------------
// The fields of a time value.

/// The oracle's leap year rule, the Gregorian one, written apart from the crate's.
fn oracle_is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// The oracle's month lengths, by month 0 to 11.
fn oracle_month_length(year: i64, month: u8) -> i64 {
    match month {
        1 if oracle_is_leap(year) => 29,
        1 => 28,
        3 | 5 | 8 | 10 => 30,
        _ => 31,
    }
}

/// The day number of January first of `year`, by counting the days of every year between it and
/// 1970 one year at a time.
fn oracle_day_of_year_start(year: i64) -> i64 {
    let mut day = 0;
    if year >= 1970 {
        for y in 1970..year {
            day += if oracle_is_leap(y) { 366 } else { 365 };
        }
    } else {
        for y in year..1970 {
            day -= if oracle_is_leap(y) { 366 } else { 365 };
        }
    }
    day
}

/// Walks every day from January first of `first` to December 31 of `last` by looping over the
/// months, and checks `date_fields` of a time of each day against the oracle. Returns the number
/// of days walked.
fn walk(first: i64, last: i64) -> i64 {
    let mut day = oracle_day_of_year_start(first);
    // 1970-01-01 is a Thursday (4); the weekday of the first day is counted back from it.
    let mut weekday = (4 + day % 7 + 7) % 7;
    let mut walked = 0;
    for year in first..=last {
        for month in 0..12u8 {
            for date in 1..=oracle_month_length(year, month) {
                // A time of day that moves through every hour, minute, second and millisecond.
                let within = (walked * 7_919_993 + 12_345) % 86_400_000;
                let t = day * 86_400_000 + within;
                let expected = DateFields {
                    year,
                    month,
                    date: u8::try_from(date).unwrap_or(0),
                    weekday: u8::try_from(weekday).unwrap_or(9),
                    hour: u8::try_from(within / HOUR).unwrap_or(99),
                    minute: u8::try_from(within / 60_000 % 60).unwrap_or(99),
                    second: u8::try_from(within / 1_000 % 60).unwrap_or(99),
                    millisecond: u16::try_from(within % 1_000).unwrap_or(9_999),
                };
                assert_eq!(date_fields(t as f64), Some(expected), "t = {t}");
                day += 1;
                weekday = (weekday + 1) % 7;
                walked += 1;
            }
        }
    }
    walked
}

#[test]
fn the_fields_of_every_day_match_a_walk_over_the_months() {
    let start = Instant::now();
    let (first, last, expected) = if full_counts() {
        (-1000, 3000, 1_461_335)
    } else {
        (1900, 2100, 73_414)
    };
    let walked = walk(first, last);
    println!(
        "calendar walk: {walked} days from {first}-01-01 to {last}-12-31 in {:?}",
        start.elapsed()
    );
    assert_eq!(walked, expected);
}

fn fields(
    year: i64,
    month: u8,
    date: u8,
    weekday: u8,
    hms: (u8, u8, u8),
    millisecond: u16,
) -> Option<DateFields> {
    Some(DateFields {
        year,
        month,
        date,
        weekday,
        hour: hms.0,
        minute: hms.1,
        second: hms.2,
        millisecond,
    })
}

#[test]
fn the_fields_at_the_edges_of_the_range() {
    const MS_PER_DAY_I64: i64 = 86_400_000;
    assert_eq!(MS_PER_DAY, 86_400_000.0);
    assert_eq!(LOCAL_TIME_LIMIT, 8_640_000_086_400_000.0);
    assert_eq!(date_fields(0.0), fields(1970, 0, 1, 4, (0, 0, 0), 0));
    assert_eq!(date_fields(-0.0), fields(1970, 0, 1, 4, (0, 0, 0), 0));
    assert_eq!(
        date_fields(-1.0),
        fields(1969, 11, 31, 3, (23, 59, 59), 999)
    );
    assert_eq!(
        date_fields(-8.64e15),
        fields(-271_821, 3, 20, 2, (0, 0, 0), 0)
    );
    assert_eq!(
        date_fields(-8.64e15 + 1.0),
        fields(-271_821, 3, 20, 2, (0, 0, 0), 1)
    );
    assert_eq!(
        date_fields(-8.64e15 - 1.0),
        fields(-271_821, 3, 19, 1, (23, 59, 59), 999)
    );
    assert_eq!(
        date_fields(8.64e15),
        fields(275_760, 8, 13, 6, (0, 0, 0), 0)
    );
    assert_eq!(
        date_fields(8.64e15 - 1.0),
        fields(275_760, 8, 12, 5, (23, 59, 59), 999)
    );
    assert_eq!(
        date_fields(8.64e15 + 1.0),
        fields(275_760, 8, 13, 6, (0, 0, 0), 1)
    );
    assert_eq!(
        date_fields(LOCAL_TIME_LIMIT),
        fields(275_760, 8, 14, 0, (0, 0, 0), 0)
    );
    assert_eq!(
        date_fields(-LOCAL_TIME_LIMIT),
        fields(-271_821, 3, 19, 1, (0, 0, 0), 0)
    );
    assert_eq!(
        date_fields(8.64e15 + (MS_PER_DAY_I64 - 1) as f64),
        fields(275_760, 8, 13, 6, (23, 59, 59), 999)
    );
    assert_eq!(date_fields(LOCAL_TIME_LIMIT + 1.0), None);
    assert_eq!(date_fields(-LOCAL_TIME_LIMIT - 1.0), None);
    assert_eq!(date_fields(f64::NAN), None);
    assert_eq!(date_fields(f64::INFINITY), None);
    assert_eq!(date_fields(f64::NEG_INFINITY), None);
    assert_eq!(date_fields(0.5), None);
    assert_eq!(date_fields(-1.5), None);
    assert_eq!(date_fields(1e300), None);
}

#[test]
fn make_full_year_reads_two_digit_years_as_years_after_1900() {
    assert!(make_full_year(f64::NAN).is_nan());
    assert!(make_full_year(f64::INFINITY).is_nan());
    assert!(make_full_year(f64::NEG_INFINITY).is_nan());
    let cases: [(f64, f64); 8] = [
        (-0.5, 1900.0),
        (-0.0, 1900.0),
        (0.0, 1900.0),
        (99.0, 1999.0),
        (99.9, 1999.0),
        (100.0, 100.0),
        (-1.0, -1.0),
        (2020.7, 2020.0),
    ];
    for (year, expected) in cases {
        let full = make_full_year(year);
        assert_eq!(full, expected, "MakeFullYear({year})");
        assert!(
            full.is_sign_positive() || expected < 0.0,
            "MakeFullYear({year})"
        );
    }
}

#[test]
fn make_day_answers_up_to_a_balanced_year_of_2_to_the_53() {
    const TWO_53: f64 = 9_007_199_254_740_992.0;
    assert!(make_day(TWO_53 - 65.0, 0.0, 1.0).is_finite());
    assert!(make_day(TWO_53, 0.0, 1.0).is_nan());
    assert!(make_day(TWO_53 + 2.0, 0.0, 1.0).is_nan());
    assert!(make_day(-(TWO_53 + 2.0), 0.0, 1.0).is_nan());
    assert!(
        make_day(TWO_53 - 65.0 - 1.0, 12.0, 1.0).is_finite(),
        "balanced by the month"
    );
}

// ---------------------------------------------------------------------------------------------
// The offset of a zone.

#[test]
fn an_offset_is_clamped_and_truncated_to_milliseconds() {
    let cases: [(i128, i64); 10] = [
        (0, 0),
        (86_399_999_999_999, 86_399_999),
        (-86_399_999_999_999, -86_399_999),
        (86_400_000_000_000, 86_399_999),
        (-86_400_000_000_000, -86_399_999),
        (i128::MAX, 86_399_999),
        (i128::MIN, -86_399_999),
        (1_999_999, 1),
        (-1_999_999, -1),
        (3_600_000_000_000, 3_600_000),
    ];
    for (ns, ms) in cases {
        assert_eq!(offset_milliseconds(ns), ms, "{ns} ns");
    }
}

#[test]
fn offset_strings_are_read_as_the_grammar_states() {
    let cases: [(&str, Option<i128>); 24] = [
        ("+01:00", Some(3_600_000_000_000)),
        ("-23:59:59.999999999", Some(-86_399_999_999_999)),
        ("+24:00", None),
        ("+0100", Some(3_600_000_000_000)),
        ("+01:00:00.0000000000", None),
        ("UTC", None),
        ("Europe/Berlin", None),
        ("+01:00:00,5", Some(3_600_500_000_000)),
        ("+01:00:00.5", Some(3_600_500_000_000)),
        ("\u{2212}01:00", None),
        ("+01", Some(3_600_000_000_000)),
        ("-00", Some(0)),
        ("+23", Some(82_800_000_000_000)),
        ("+010000.5", Some(3_600_500_000_000)),
        ("-12:34:56.789", Some(-45_296_789_000_000)),
        ("+01:0000", None),
        ("+0100:00", None),
        ("+01:60", None),
        ("+01:00.5", None),
        ("+01:00:00.", None),
        ("+1", None),
        ("01:00", None),
        ("", None),
        ("+01:00 ", None),
    ];
    for (identifier, expected) in cases {
        assert_eq!(
            parse_time_zone_offset_string(identifier),
            expected,
            "{identifier:?}"
        );
    }
    assert_eq!("+01:00:00.0000000000".len(), 20);
    assert_eq!("-23:59:59.999999999".len(), 19);
}

// ---------------------------------------------------------------------------------------------
// The conversions over a host given as closures.

#[derive(Debug, PartialEq)]
struct Stop;

const ZONE: &str = "Test/Zone";

/// A zone with one transition at the UTC instant `transition` (ms): the offset is `before` ms
/// before it and `after` ms from it on.
#[derive(Clone, Copy)]
struct Zone {
    transition: i64,
    before: i64,
    after: i64,
}

impl Zone {
    fn offset_at(self, utc_ms: i64) -> i64 {
        if utc_ms < self.transition {
            self.before
        } else {
            self.after
        }
    }

    /// The first and the last instant (ns) whose local time is `local` ms, or `None`.
    fn instants(self, local: i64) -> Option<(i128, i128)> {
        let mut list = Vec::new();
        let early = local - self.before;
        if early < self.transition {
            list.push(early);
        }
        let late = local - self.after;
        if late >= self.transition {
            list.push(late);
        }
        list.sort_unstable();
        let first = *list.first()?;
        let last = *list.last()?;
        Some((i128::from(first) * NS_PER_MS, i128::from(last) * NS_PER_MS))
    }
}

/// The local time, in ms, of the fields the conversion hands the host.
fn local_ms(fields: &IsoDateTime) -> i64 {
    assert!((1..=12).contains(&fields.month), "{fields:?}");
    assert_eq!((fields.microsecond, fields.nanosecond), (0, 0));
    let ns = get_utc_epoch_nanoseconds(fields).unwrap_or(i128::MAX);
    i64::try_from(ns / NS_PER_MS).unwrap_or(i64::MAX)
}

/// `UTC ( t )` over `zone`, with the asks of each closure counted: (result, asks of the
/// possible instants, asks of the offset).
fn utc_over(zone: Zone, t: f64) -> (f64, u32, u32) {
    let asks = Cell::new(0u32);
    let offsets = Cell::new(0u32);
    let result = utc_from_local(
        t,
        ZONE,
        &mut |identifier, fields| -> Result<_, Stop> {
            assert_eq!(identifier, ZONE);
            asks.set(asks.get() + 1);
            Ok(zone.instants(local_ms(fields)))
        },
        &mut |identifier, ns| {
            assert_eq!(identifier, ZONE);
            offsets.set(offsets.get() + 1);
            let ms = i64::try_from(ns / NS_PER_MS).unwrap_or(0);
            Ok(i128::from(zone.offset_at(ms)) * NS_PER_MS)
        },
    );
    (result.unwrap_or(f64::INFINITY), asks.get(), offsets.get())
}

/// `LocalTime ( t )` with a host whose offset is always `offset_ns`: (result, asks).
fn local_over(identifier: &str, offset_ns: i128, t: f64) -> (f64, u32) {
    let asks = Cell::new(0u32);
    let result = local_time(t, identifier, &mut |id, ns| -> Result<_, Stop> {
        assert_eq!(id, identifier);
        assert_eq!(ns, t as i128 * NS_PER_MS);
        asks.set(asks.get() + 1);
        Ok(offset_ns)
    });
    (result.unwrap_or(f64::INFINITY), asks.get())
}

fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

#[test]
fn local_time_asks_the_offset_once_or_reads_an_offset_string() {
    assert_eq!(local_over(ZONE, 3_600_000_000_000, 0.0), (3_600_000.0, 1));
    assert_eq!(
        local_over(ZONE, -3_600_000_000_000, 1e12),
        (1e12 - 3_600_000.0, 1)
    );
    assert_eq!(local_over("+01:00", 0, 0.0), (3_600_000.0, 0));
    assert_eq!(local_over("-05:30", 0, 0.0), (-19_800_000.0, 0));
    let (nan, asks) = local_over(ZONE, 0, f64::NAN);
    assert!(nan.is_nan());
    assert_eq!(asks, 0);
    assert_eq!(local_over(ZONE, 0, f64::INFINITY), (f64::INFINITY, 0));
    assert_eq!(
        local_over(ZONE, 0, f64::NEG_INFINITY),
        (f64::NEG_INFINITY, 0)
    );
    // The clamp (HOST-209): an answer of i128::MAX at 0 is 23:59:59.999 of 1970-01-01.
    assert_eq!(local_over(ZONE, i128::MAX, 0.0), (86_399_999.0, 1));
    assert_eq!(local_over(ZONE, i128::MIN, 0.0), (-86_399_999.0, 1));
    assert_eq!(local_over(ZONE, 0, 8.64e15), (8.64e15, 1));
}

#[test]
fn utc_takes_the_one_instant_of_an_ordinary_local_time() {
    let zone = Zone {
        transition: 1_000_000_000_000,
        before: 0,
        after: HOUR,
    };
    assert_eq!(utc_over(zone, 0.0), (0.0, 1, 1));
    assert_eq!(utc_over(zone, 2e12), (2e12 - HOUR as f64, 1, 1));
    assert_eq!(utc_over(zone, -1.0), (-1.0, 1, 1));
    // An offset string asks neither closure.
    let mut never = |_: &str, _: &IsoDateTime| -> Result<Option<(i128, i128)>, Stop> { Err(Stop) };
    let mut no_offset = |_: &str, _: i128| -> Result<i128, Stop> { Err(Stop) };
    assert_eq!(
        utc_from_local(3_600_000.0, "+01:00", &mut never, &mut no_offset),
        Ok(0.0)
    );
    assert_eq!(
        utc_from_local(0.0, "-23:59:59.999999999", &mut never, &mut no_offset),
        Ok(86_399_999.0)
    );
}

#[test]
fn utc_of_a_repeated_local_time_takes_the_first_instant() {
    // 1:30 AM on 5 November 2017 in America/New_York is 1:30 AM UTC-04 (`#sec-utc-t`, note 1).
    let new_york_fall = Zone {
        transition: 1_509_861_600_000,
        before: -4 * HOUR,
        after: -5 * HOUR,
    };
    let t = 1_509_845_400_000.0;
    assert_eq!(utc_over(new_york_fall, t), (t + 4.0 * HOUR as f64, 1, 1));
    let zone = Zone {
        transition: 0,
        before: HOUR,
        after: 0,
    };
    assert_eq!(utc_over(zone, 1_800_000.0), (-1_800_000.0, 1, 1));
}

#[test]
fn utc_of_a_skipped_local_time_takes_the_offset_before_the_transition() {
    // 2:30 AM on 12 March 2017 in America/New_York is 2:30 AM UTC-05 (`#sec-utc-t`, note 1).
    // tBefore is 1:59:59.999, 1,800,001 ms below t: the gallop asks at 1, 2, ..., 2^21 (22
    // probes), the bisection halves (2^20, 2^21] (20 probes), and the first ask is t's.
    let new_york_spring = Zone {
        transition: 1_489_302_000_000,
        before: -5 * HOUR,
        after: -4 * HOUR,
    };
    let t = 1_489_285_800_000.0;
    assert_eq!(utc_over(new_york_spring, t), (t + 5.0 * HOUR as f64, 43, 1));
    // A 24-hour gap: the last skipped local time is 86,400,000 ms above tBefore (gallop to
    // 2^27, 28 probes; bisection of (2^26, 2^27], 26 probes).
    let day_gap = Zone {
        transition: 0,
        before: 0,
        after: 86_400_000,
    };
    assert_eq!(utc_over(day_gap, 86_399_999.0), (86_399_999.0, 55, 1));
    assert_eq!(utc_over(day_gap, 0.0), (0.0, 2, 1));
}

#[test]
fn the_search_asks_at_most_57_times_and_gives_up_beyond_2_to_the_28_ms() {
    const TWO_28: i64 = 1 << 28;
    // tBefore exactly 2^28 below t: 1 + 29 + 27 asks, the most.
    let widest = Zone {
        transition: 0,
        before: 0,
        after: TWO_28,
    };
    let t = (TWO_28 - 1) as f64;
    assert_eq!(utc_over(widest, t), (t, 57, 1));
    // A gap of 80 hours: no tBefore within 2^28 ms, NaN after 1 + 29 asks and no offset.
    let eighty_hours = Zone {
        transition: 0,
        before: 0,
        after: 80 * HOUR,
    };
    let (result, asks, offsets) = utc_over(eighty_hours, (80 * HOUR - 1) as f64);
    assert!(result.is_nan());
    assert_eq!((asks, offsets), (30, 0));
    let (result, asks, offsets) = utc_over(
        Zone {
            transition: 0,
            before: 0,
            after: TWO_28 + 1,
        },
        TWO_28 as f64,
    );
    assert!(result.is_nan());
    assert_eq!((asks, offsets), (30, 0));
    // Across gaps of every size and every place in them, the count stays at most 57, and the
    // result is t minus the offset before the transition when tBefore is within 2^28 ms.
    let gaps: [i64; 14] = [
        1,
        2,
        3,
        1_000,
        HOUR,
        86_400_000,
        (1 << 27) - 1,
        1 << 27,
        (1 << 27) + 1,
        TWO_28 - 1,
        TWO_28,
        TWO_28 + 1,
        172_800_000,
        300_000_000,
    ];
    let mut most = 0;
    for gap in gaps {
        let zone = Zone {
            transition: 1_000_000_000_000,
            before: -2 * HOUR,
            after: -2 * HOUR + gap,
        };
        let gap_start = zone.transition + zone.before;
        for place in [0, 1, gap / 3, gap / 2, gap - 2, gap - 1] {
            if !(0..gap).contains(&place) {
                continue;
            }
            let t = gap_start + place;
            let (result, asks, offsets) = utc_over(zone, t as f64);
            most = most.max(asks);
            assert!(asks <= 57, "gap {gap}, place {place}: {asks} asks");
            if place < TWO_28 {
                assert!(
                    same(result, (t - zone.before) as f64),
                    "gap {gap}, place {place}"
                );
                assert_eq!(offsets, 1);
            } else {
                assert!(result.is_nan(), "gap {gap}, place {place}");
                assert_eq!((asks, offsets), (30, 0));
            }
        }
    }
    assert_eq!(most, 57);
}

#[test]
fn a_host_with_no_instant_for_any_local_time_gets_30_asks_and_nan() {
    let asks = Cell::new(0u32);
    let result = utc_from_local(
        1_577_836_800_000.0,
        ZONE,
        &mut |_, _| -> Result<_, Stop> {
            asks.set(asks.get() + 1);
            Ok(None)
        },
        &mut |_, _| Err(Stop),
    );
    assert!(result.is_ok_and(f64::is_nan));
    assert_eq!(asks.get(), 30);
}

#[test]
fn utc_at_the_edges_of_the_local_range() {
    // The lowest t: only t and t - 1 lie within the limit; every lower probe makes no call.
    let asks = Cell::new(0u32);
    let lowest = -LOCAL_TIME_LIMIT + 1.0;
    let result = utc_from_local(
        lowest,
        ZONE,
        &mut |_, fields| -> Result<_, Stop> {
            asks.set(asks.get() + 1);
            assert!(fields.year == -271_821 && fields.month == 4 && fields.day == 19);
            Ok(None)
        },
        &mut |_, _| Err(Stop),
    );
    assert!(result.is_ok_and(f64::is_nan));
    assert_eq!(asks.get(), 2);
    // The highest t, with a zone of +1 hour: 8.64e15 + 3.6e6 is the time value 8.64e15
    // (`#sec-utc-t`, its note), and LOCAL_TIME_LIMIT itself is converted.
    let plus_one = Zone {
        transition: i64::MIN,
        before: 0,
        after: HOUR,
    };
    assert_eq!(utc_over(plus_one, 8.64e15 + 3.6e6), (8.64e15, 1, 1));
    assert_eq!(
        utc_over(plus_one, LOCAL_TIME_LIMIT),
        (LOCAL_TIME_LIMIT - 3.6e6, 1, 1)
    );
    let utc = Zone {
        transition: 0,
        before: 0,
        after: 0,
    };
    assert_eq!(utc_over(utc, -LOCAL_TIME_LIMIT), (-LOCAL_TIME_LIMIT, 1, 1));
    // Beyond the limit, a Number that is not integral, NaN and the infinities: NaN, no call.
    for t in [
        LOCAL_TIME_LIMIT + 1.0,
        -LOCAL_TIME_LIMIT - 1.0,
        0.5,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1e300,
    ] {
        let (result, asks, offsets) = utc_over(utc, t);
        assert!(result.is_nan(), "{t}");
        assert_eq!((asks, offsets), (0, 0), "{t}");
    }
}

#[test]
fn an_error_of_a_closure_goes_up_unchanged() {
    let mut fails = |_: &str, _: &IsoDateTime| -> Result<Option<(i128, i128)>, Stop> { Err(Stop) };
    let mut zero = |_: &str, _: i128| -> Result<i128, Stop> { Ok(0) };
    assert_eq!(utc_from_local(0.0, ZONE, &mut fails, &mut zero), Err(Stop));
    // An error in the search, on the third ask.
    let asks = Cell::new(0u32);
    let result = utc_from_local(
        0.0,
        ZONE,
        &mut |_, _| {
            asks.set(asks.get() + 1);
            if asks.get() == 3 { Err(Stop) } else { Ok(None) }
        },
        &mut zero,
    );
    assert_eq!((result, asks.get()), (Err(Stop), 3));
    let mut one =
        |_: &str, _: &IsoDateTime| -> Result<Option<(i128, i128)>, Stop> { Ok(Some((0, 0))) };
    let mut offset_fails = |_: &str, _: i128| -> Result<i128, Stop> { Err(Stop) };
    assert_eq!(
        utc_from_local(0.0, ZONE, &mut one, &mut offset_fails),
        Err(Stop)
    );
    assert_eq!(local_time(0.0, ZONE, &mut offset_fails), Err(Stop));
}

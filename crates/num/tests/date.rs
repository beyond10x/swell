//! `GetUTCEpochNanoseconds` (`#sec-getutcepochnanoseconds`) and the operations it calls,
//! `MakeDay` (`#sec-makeday`), `MakeTime` (`#sec-maketime`) and `MakeDate` (`#sec-makedate`).
//!
//! Where the numbers come from:
//! - the seconds of the ordinary dates were printed by `date -u -d '<date>' +%s`
//!   (`2000-02-29T00:00:00Z` 951782400, `2000-02-29T23:59:59Z` 951868799,
//!   `1969-12-31T23:59:59Z` -1, `1900-03-01T00:00:00Z` -2203891200);
//! - -271821-04-20 and 275760-09-13 are the ends of the time value range, -100,000,000 and
//!   100,000,000 days (`#sec-time-values-and-time-range`; test262
//!   `test/built-ins/Date/parse/time-value-maximum-range.js`), so -8.64 × 10^21 and
//!   8.64 × 10^21 nanoseconds (`MinEpochNanoseconds`, `MaxEpochNanoseconds`,
//!   `#sec-time-related-constants`);
//! - the `MakeDay`, `MakeTime` and `MakeDate` values are those of test262
//!   `test/built-ins/Date/UTC/overflow-make-day.js` and `fp-evaluation-order.js`.

use swell_num::date::{IsoDateTime, get_utc_epoch_nanoseconds, make_date, make_day, make_time};

fn at(year: i64, month: u8, day: u8) -> IsoDateTime {
    IsoDateTime {
        year,
        month,
        day,
        hour: 0,
        minute: 0,
        second: 0,
        millisecond: 0,
        microsecond: 0,
        nanosecond: 0,
    }
}

/// `Date.UTC` without `TimeClip`: `MakeDate(MakeDay(y, m, d), MakeTime(h, min, s, ms))`.
fn utc(y: f64, m: f64, d: f64, h: f64, min: f64, s: f64, ms: f64) -> f64 {
    make_date(make_day(y, m, d), make_time(h, min, s, ms))
}

#[test]
fn epoch_nanoseconds_of_known_dates() {
    const SECOND: i128 = 1_000_000_000;
    let last_nanosecond = IsoDateTime {
        hour: 23,
        minute: 59,
        second: 59,
        millisecond: 999,
        microsecond: 999,
        nanosecond: 999,
        ..at(2000, 2, 29)
    };
    let last_second_of_1969 = IsoDateTime {
        hour: 23,
        minute: 59,
        second: 59,
        ..at(1969, 12, 31)
    };
    let cases: [(IsoDateTime, i128); 7] = [
        (at(1970, 1, 1), 0),
        (at(2000, 2, 29), 951_782_400 * SECOND),
        (last_nanosecond, 951_868_799 * SECOND + 999_999_999),
        (last_second_of_1969, -SECOND),
        (at(1900, 3, 1), -2_203_891_200 * SECOND),
        (at(-271_821, 4, 20), -8_640_000_000_000_000_000_000),
        (at(275_760, 9, 13), 8_640_000_000_000_000_000_000),
    ];
    for (date, expected) in cases {
        assert_eq!(get_utc_epoch_nanoseconds(&date), Some(expected), "{date:?}");
    }

    assert_eq!(utc(2016.0, 12.0, 1.0, 0.0, 0.0, 0.0, 0.0), 1483228800000.0);
    assert_eq!(utc(2016.0, 13.0, 1.0, 0.0, 0.0, 0.0, 0.0), 1485907200000.0);
    assert_eq!(utc(2016.0, 144.0, 1.0, 0.0, 0.0, 0.0, 0.0), 1830297600000.0);
    assert_eq!(utc(2016.0, 0.0, 33.0, 0.0, 0.0, 0.0, 0.0), 1454371200000.0);
    assert_eq!(utc(2016.0, 2.0, -27.0, 0.0, 0.0, 0.0, 0.0), 1454371200000.0);
    let t = utc(
        1970.0,
        0.0,
        1.0,
        80063993375.0,
        29.0,
        1.0,
        -288230376151711740.0,
    );
    assert_eq!(t, 29312.0, "order of operations and precision in MakeTime");
    let t = utc(
        1970.0,
        0.0,
        213503982336.0,
        0.0,
        0.0,
        0.0,
        -18446744073709552000.0,
    );
    assert_eq!(t, 34447360.0, "precision in MakeDate");
    assert!(make_day(f64::INFINITY, 0.0, 1.0).is_nan());
    assert!(make_day(2016.0, f64::NAN, 1.0).is_nan());
    assert!(make_time(0.0, 0.0, f64::NEG_INFINITY, 0.0).is_nan());
    assert!(make_date(f64::MAX, 0.0).is_nan());
    assert!(make_day(1e300, 0.0, 1.0).is_nan());
}

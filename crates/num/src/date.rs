//! The date arithmetic that `GetUTCEpochNanoseconds` (`#sec-getutcepochnanoseconds`) needs:
//! `MakeTime` (`#sec-maketime`), `MakeDay` (`#sec-makeday`) and `MakeDate` (`#sec-makedate`),
//! with `DayFromYear` (`#sec-dayfromyear`) and `IsLeapYear` (`#sec-isleapyear`) inside.
//!
//! The calendar is integer arithmetic: the day number of the first day of a month is computed
//! exactly, in `i128`. `MakeTime` and `MakeDate` are Number arithmetic, because their clauses say
//! so ("The arithmetic in MakeTime is floating-point arithmetic, which is not associative, so the
//! operations must be performed in the correct order"; `MakeDate` is `day × 𝔽(msPerDay) + time`):
//! IEEE 754 `+` and `×` in the order the clause gives, which every platform computes alike.
//! test262 holds that order (`test/built-ins/Date/UTC/fp-evaluation-order.js`).
//!
//! `MakeDay` finds "a finite time value tv such that YearFromTime(tv) = ℝ(balancedYear),
//! MonthFromTime(tv) = balancedMonthMV, and DateFromTime(tv) = 1", NaN when there is none. A
//! finite time value is read here as an integral Number, the definition of
//! `#sec-time-values-and-time-range`, without the range of ±8.64 × 10^15 that `TimeClip` applies
//! later: `GetUTCEpochNanoseconds` of -271821-04-20, the first day of the range, goes through the
//! first of that April, 19 days before it. Such a tv exists when some Number lies in the
//! milliseconds of that day, which this module decides exactly for |balancedYear| ≤ 2^53; for a
//! balancedYear of magnitude above 2^53 `MakeDay` answers NaN (`docs/design/engine.md`
//! decision 6, "`MakeDay` far outside the time range": `MakeDay(2^53, 0, 1)` is NaN,
//! `MakeDay(2^53 - 65, 0, 1)` is not).
//!
//! The calendar of a time value: [`date_fields`] gives Day, TimeWithinDay, YearFromTime,
//! MonthFromTime, DateFromTime, WeekDay, HourFromTime, MinuteFromTime, SecondFromTime and
//! msFromTime (`#sec-time-values-and-time-range` to `#sec-ms-from-time`) in one integer
//! decomposition; [`make_full_year`] is `MakeFullYear` (`#sec-makefullyear`). The conversions
//! [`local_time`] (`#sec-localtime`) and [`utc_from_local`] (`#sec-utc-t`) ask the host through
//! closures their caller passes, read an identifier that is an offset string themselves
//! ([`parse_time_zone_offset_string`], `#sec-istimezoneoffsetstring`,
//! `#sec-parsetimezoneoffsetstring`) and clamp every offset ([`offset_milliseconds`];
//! `docs/design/host-interface.md` decision 17).

/// MillisecondsPerDay (`#sec-time-related-constants`), as an integer.
const MILLISECONDS_PER_DAY: i128 = 86_400_000;

use crate::fmath;

/// The first day of each month within a common year, January first.
const MONTH_STARTS: [i128; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

/// ToIntegerOrInfinity (`#sec-tointegerorinfinity`) of a finite Number, as a Number: the
/// truncated value, with -0 made +0.
fn to_integer(x: f64) -> f64 {
    fmath::trunc(x) + 0.0
}

/// `MakeTime ( hour, minute, second, millisecond )`.
pub fn make_time(hour: f64, minute: f64, second: f64, millisecond: f64) -> f64 {
    if !(hour.is_finite() && minute.is_finite() && second.is_finite() && millisecond.is_finite()) {
        return f64::NAN;
    }
    let h = to_integer(hour);
    let m = to_integer(minute);
    let s = to_integer(second);
    let milli = to_integer(millisecond);
    ((h * 3_600_000.0 + m * 60_000.0) + s * 1_000.0) + milli
}

/// `MakeDay ( year, month, day )`.
pub fn make_day(year: f64, month: f64, day: f64) -> f64 {
    if !(year.is_finite() && month.is_finite() && day.is_finite()) {
        return f64::NAN;
    }
    let year_mv = to_integer(year);
    let month_mv = to_integer(month);
    let day_mv = to_integer(day);
    // 𝔽(floor(monthMV / 12)) and monthMV modulo 12, exactly.
    let (years_of_months, balanced_month) = match exact_integer(month_mv) {
        Some(m) => (m.div_euclid(12) as f64, m.rem_euclid(12)),
        None => {
            // |monthMV| ≥ 2^127: a multiple of 2^75, so monthMV / 12 is within 1 of an
            // integer only when it is one, and its rounding is that of floor(monthMV / 12).
            let rest = fmath::rem_euclid(month_mv, 12.0);
            (month_mv / 12.0, rest as i128)
        }
    };
    let balanced_year = year_mv + years_of_months;
    if !balanced_year.is_finite() {
        return f64::NAN;
    }
    let Some(y) = exact_integer(balanced_year).filter(|y| y.unsigned_abs() <= 1 << 53) else {
        return f64::NAN;
    };
    let Some(first) = month_start(y, balanced_month) else {
        return f64::NAN;
    };
    if !day_holds_a_number(first) {
        return f64::NAN;
    }
    (first as f64 + day_mv) - 1.0
}

/// `MakeDate ( day, time )`.
pub fn make_date(day: f64, time: f64) -> f64 {
    if !(day.is_finite() && time.is_finite()) {
        return f64::NAN;
    }
    let tv = day * 86_400_000.0 + time;
    if !tv.is_finite() {
        return f64::NAN;
    }
    tv
}

/// The arguments of `GetUTCEpochNanoseconds`: an ISO 8601 calendar date and wall-clock time.
/// The clause gives month 1 to 12, day 1 to 31, hour 0 to 23, minute and second 0 to 59,
/// millisecond, microsecond and nanosecond 0 to 999.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IsoDateTime {
    pub year: i64,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub millisecond: u16,
    pub microsecond: u16,
    pub nanosecond: u16,
}

/// `GetUTCEpochNanoseconds ( year, month, day, hour, minute, second, millisecond, microsecond,
/// nanosecond )`: the epoch nanoseconds count of `date` in UTC.
///
/// `None` where the clause's step 4, "Assert: epochMilliseconds is an integral Number", would
/// fail, which a year of a time value never makes it do.
pub fn get_utc_epoch_nanoseconds(date: &IsoDateTime) -> Option<i128> {
    let day = make_day(
        date.year as f64,
        f64::from(i16::from(date.month) - 1),
        f64::from(date.day),
    );
    let time = make_time(
        f64::from(date.hour),
        f64::from(date.minute),
        f64::from(date.second),
        f64::from(date.millisecond),
    );
    let epoch_milliseconds = exact_integer(make_date(day, time))?;
    epoch_milliseconds
        .checked_mul(1_000_000)?
        .checked_add(i128::from(date.microsecond) * 1_000)?
        .checked_add(i128::from(date.nanosecond))
}

/// The integer `x` is, when it is finite, integral and below 2^127 in magnitude.
fn exact_integer(x: f64) -> Option<i128> {
    const LIMIT: f64 = 170_141_183_460_469_231_731_687_303_715_884_105_728.0; // 2^127
    if x.is_finite() && fmath::trunc(x) == x && x.abs() < LIMIT {
        Some(x as i128)
    } else {
        None
    }
}

/// `IsLeapYear ( y )`.
fn is_leap_year(y: i128) -> bool {
    y.rem_euclid(400) == 0 || (y.rem_euclid(100) != 0 && y.rem_euclid(4) == 0)
}

/// `DayFromYear ( y )`.
fn day_from_year(y: i128) -> i128 {
    365 * (y - 1970) + (y - 1969).div_euclid(4) - (y - 1901).div_euclid(100)
        + (y - 1601).div_euclid(400)
}

/// The day number of the first day of month `month` (0 to 11) of year `y`.
fn month_start(y: i128, month: i128) -> Option<i128> {
    let index = usize::try_from(month).ok()?;
    let start = *MONTH_STARTS.get(index)?;
    let leap_day = i128::from(month >= 2 && is_leap_year(y));
    Some(day_from_year(y) + start + leap_day)
}

/// Whether some Number is a time value of day `day`: lies in
/// [`day` × msPerDay, (`day` + 1) × msPerDay).
fn day_holds_a_number(day: i128) -> bool {
    let low = day * MILLISECONDS_PER_DAY;
    let high = low + MILLISECONDS_PER_DAY;
    let mut x = low as f64;
    if (x as i128) < low {
        x = x.next_up();
    }
    x.is_finite() && (x as i128) < high
}

/// MillisecondsPerDay (`#sec-time-related-constants`).
pub const MS_PER_DAY: f64 = 86_400_000.0;

/// The largest magnitude of a time value whose fields this module reads: the time value range
/// of `#sec-time-values-and-time-range` plus one day (`docs/design/engine.md` decision 6,
/// "`UTC ( t )` and the local time that does not exist"). A local time of a time value lies
/// within it, since an offset is below one day ([`offset_milliseconds`]).
pub const LOCAL_TIME_LIMIT: f64 = 8.64e15 + 86_400_000.0;

/// [`LOCAL_TIME_LIMIT`] as an integer: the constant is integral and below 2^53, so the cast
/// is exact.
const LOCAL_TIME_LIMIT_MS: i64 = LOCAL_TIME_LIMIT as i64;

/// What the host's list of possible instants of a local time is reduced to: `None` for an
/// empty list, else its first and its last instant (nanoseconds).
type Instants = Option<(i128, i128)>;

/// The largest offset magnitude, in nanoseconds: 23:59:59.999999999, the largest the UTC offset
/// format writes (`docs/design/host-interface.md` decision 17, item 3; HOST-209).
const MAX_OFFSET_NS: i128 = 86_399_999_999_999;

/// The longest identifier that can be an offset string, `+23:59:59.999999999`, in code units.
const MAX_OFFSET_STRING_LENGTH: usize = 19;

/// The distance of the last probe of the search for `tBefore` is 2^`SEARCH_EXPONENT` ms
/// (`docs/design/engine.md` decision 6, "`UTC ( t )` and the local time that does not exist").
const SEARCH_EXPONENT: u32 = 28;

const NS_PER_MS: i128 = 1_000_000;

/// The fields of a time value: YearFromTime (`#sec-year-number`), MonthFromTime (0 to 11,
/// `#sec-month-number`), DateFromTime (1 to 31, `#sec-date-number`), WeekDay (0 is Sunday,
/// `#sec-week-day`), HourFromTime, MinuteFromTime, SecondFromTime and msFromTime
/// (`#sec-hour-minute-second-and-millisecond`, `#sec-ms-from-time`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateFields {
    pub year: i64,
    pub month: u8,
    pub date: u8,
    pub weekday: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub millisecond: u16,
}

/// The fields of the time value `t`. `None` for NaN, an infinity, a Number that is not
/// integral and a magnitude above [`LOCAL_TIME_LIMIT`].
pub fn date_fields(t: f64) -> Option<DateFields> {
    fields_of(time_milliseconds(t)?)
}

/// `t` as an integer, when it is finite, integral and of magnitude at most
/// [`LOCAL_TIME_LIMIT`].
fn time_milliseconds(t: f64) -> Option<i64> {
    if t.is_finite() && fmath::trunc(t) == t && t.abs() <= LOCAL_TIME_LIMIT {
        // The range test is the condition above: `t` is integral and of magnitude at most
        // LOCAL_TIME_LIMIT, below 2^63, so the cast (truncation toward zero) is exact.
        Some(t as i64)
    } else {
        None
    }
}

/// The fields of the time value `ms` (integer milliseconds), `None` beyond
/// [`LOCAL_TIME_LIMIT`]. Day and TimeWithinDay are the floored quotient and the remainder of
/// msPerDay; the year, month and date come from the days in eras of 400 years
/// (146,097 days), each counted from the first of March so that the leap day ends the year.
/// Every intermediate value is below 2^40 in magnitude.
fn fields_of(ms: i64) -> Option<DateFields> {
    if ms.unsigned_abs() > LOCAL_TIME_LIMIT_MS.unsigned_abs() {
        return None;
    }
    let day = ms.div_euclid(86_400_000);
    let within = ms.rem_euclid(86_400_000);
    // Days since 0000-03-01.
    let shifted = day + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    // Months counted from March: 0 is March, 11 is February.
    let month_from_march = (5 * day_of_year + 2) / 153;
    let date = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 2
    } else {
        month_from_march - 10
    };
    let year = era * 400 + year_of_era + i64::from(month < 2);
    // 1970-01-01, day 0, is a Thursday (`#sec-week-day`: WeekDay(+0𝔽) is 4).
    let weekday = (day + 4).rem_euclid(7);
    Some(DateFields {
        year,
        month: u8::try_from(month).ok()?,
        date: u8::try_from(date).ok()?,
        weekday: u8::try_from(weekday).ok()?,
        hour: u8::try_from(within / 3_600_000).ok()?,
        minute: u8::try_from(within / 60_000 % 60).ok()?,
        second: u8::try_from(within / 1_000 % 60).ok()?,
        millisecond: u16::try_from(within % 1_000).ok()?,
    })
}

/// `MakeFullYear ( year )`: NaN for NaN and the infinities; else the integer part of `year`,
/// read as a year after 1900 when it is 0 to 99.
pub fn make_full_year(year: f64) -> f64 {
    if !year.is_finite() {
        return f64::NAN;
    }
    let truncated = to_integer(year);
    if (0.0..=99.0).contains(&truncated) {
        1900.0 + truncated
    } else {
        truncated
    }
}

/// The offset `offset_ns` (nanoseconds, as a host answers it) clamped to
/// ±86,399,999,999,999 ns (HOST-209) and then truncated toward zero to whole milliseconds, as
/// `LocalTime` and `UTC` take it ("truncate(offsetNanoseconds / NanosecondsPerMillisecond)").
pub fn offset_milliseconds(offset_ns: i128) -> i64 {
    let clamped = offset_ns.clamp(-MAX_OFFSET_NS, MAX_OFFSET_NS);
    // Integer division truncates toward zero; the quotient is at most 86,399,999 in magnitude.
    (clamped / NS_PER_MS) as i64
}

/// `IsTimeZoneOffsetString` and `ParseTimeZoneOffsetString` (`#sec-istimezoneoffsetstring`,
/// `#sec-parsetimezoneoffsetstring`): the offset in nanoseconds when `identifier` conforms to
/// the grammar `UTCOffset` (`#sec-time-zone-offset-string-format`), `None` for a named zone.
/// The grammar: an ASCII sign, an hour 00 to 23, then nothing, or minutes, or minutes and
/// seconds and an optional fraction of 1 to 9 digits after `.` or `,`, with `:` between the
/// parts in every place or in none. An identifier longer than 19 code units is a named zone.
pub fn parse_time_zone_offset_string(identifier: &str) -> Option<i128> {
    let bytes = identifier.as_bytes();
    if bytes.len() > MAX_OFFSET_STRING_LENGTH {
        return None;
    }
    let (sign, rest) = match bytes {
        [b'+', rest @ ..] => (1, rest),
        [b'-', rest @ ..] => (-1, rest),
        _ => return None,
    };
    let (hours, rest) = two_digits(rest, 23)?;
    let (minutes, seconds, nanoseconds) = match rest {
        [] => (0, 0, 0),
        [b':', rest @ ..] => hour_subcomponents(rest, true)?,
        _ => hour_subcomponents(rest, false)?,
    };
    Some(sign * (((hours * 60 + minutes) * 60 + seconds) * 1_000_000_000 + nanoseconds))
}

/// `HourSubcomponents[Extended]` after the first separator: minutes, then nothing, or a
/// separator, seconds and an optional fraction.
fn hour_subcomponents(text: &[u8], extended: bool) -> Option<(i128, i128, i128)> {
    let (minutes, rest) = two_digits(text, 59)?;
    if rest.is_empty() {
        return Some((minutes, 0, 0));
    }
    let rest = if extended {
        rest.strip_prefix(b":")?
    } else {
        rest
    };
    let (seconds, rest) = two_digits(rest, 59)?;
    let nanoseconds = match rest {
        [] => 0,
        [b'.' | b',', digits @ ..] => fraction(digits)?,
        _ => return None,
    };
    Some((minutes, seconds, nanoseconds))
}

/// Two ASCII digits at the start of `text` whose value is at most `max`, and the rest.
fn two_digits(text: &[u8], max: i128) -> Option<(i128, &[u8])> {
    let [tens @ b'0'..=b'9', ones @ b'0'..=b'9', rest @ ..] = text else {
        return None;
    };
    let value = i128::from(tens - b'0') * 10 + i128::from(ones - b'0');
    (value <= max).then_some((value, rest))
}

/// `TemporalDecimalFraction` after its separator: 1 to 9 digits, as nanoseconds (the digits
/// padded with zeros to nine).
fn fraction(digits: &[u8]) -> Option<i128> {
    if digits.is_empty() || digits.len() > 9 {
        return None;
    }
    let mut value = 0;
    for digit in digits {
        if !digit.is_ascii_digit() {
            return None;
        }
        value = value * 10 + i128::from(digit - b'0');
    }
    for _ in digits.len()..9 {
        value *= 10;
    }
    Some(value)
}

/// `LocalTime ( t )` (`#sec-localtime`). `identifier` is the one the caller read from
/// `SystemTimeZoneIdentifier` for this conversion. NaN and the infinities answer themselves
/// with no call; a finite Number that is not integral or lies beyond [`LOCAL_TIME_LIMIT`]
/// answers NaN with no call (no time value does); an identifier that is an offset string
/// answers `t` plus that offset with no call; else `offset_ns` is called once with the
/// identifier and the epoch nanoseconds of `t`, and its answer goes through
/// [`offset_milliseconds`]. An error of the closure goes up unchanged.
pub fn local_time<E>(
    t: f64,
    identifier: &str,
    offset_ns: &mut dyn FnMut(&str, i128) -> Result<i128, E>,
) -> Result<f64, E> {
    if !t.is_finite() {
        return Ok(t);
    }
    let Some(ms) = time_milliseconds(t) else {
        return Ok(f64::NAN);
    };
    let offset = match parse_time_zone_offset_string(identifier) {
        Some(offset) => offset,
        None => offset_ns(identifier, i128::from(ms) * NS_PER_MS)?,
    };
    // The offset is below 86,400,000 ms and `t` below 2^53 - 2^27, so the sum is exact.
    Ok(t + offset_milliseconds(offset) as f64)
}

/// `UTC ( t )` (`#sec-utc-t`). `identifier` is the one the caller read from
/// `SystemTimeZoneIdentifier` for this conversion. `possible` stands for
/// `GetNamedTimeZoneEpochNanoseconds`: given the identifier and the fields of a local time, it
/// answers `None` for an empty list of instants and else the first and the last instant of the
/// list (nanoseconds); the list itself is the closure's, charged and dropped inside it.
/// `offset_ns` stands for `GetNamedTimeZoneOffsetNanoseconds`. An error of either closure goes
/// up unchanged.
///
/// A `t` that [`date_fields`] refuses (NaN, an infinity, not integral, beyond
/// [`LOCAL_TIME_LIMIT`]) answers NaN with no call. An offset string answers `t` minus that
/// offset with no call. Else `possible` is asked for the fields of `t`; its first instant, or
/// for a skipped local time the last instant of `tBefore` (found by `search_before`), goes to
/// `offset_ns` once, and the answer is `t` minus [`offset_milliseconds`] of it. With no
/// `tBefore` within 2^28 ms the answer is NaN, with no offset call. `possible` is asked at most
/// 57 times.
pub fn utc_from_local<E>(
    t: f64,
    identifier: &str,
    possible: &mut dyn FnMut(&str, &IsoDateTime) -> Result<Instants, E>,
    offset_ns: &mut dyn FnMut(&str, i128) -> Result<i128, E>,
) -> Result<f64, E> {
    let Some(ms) = time_milliseconds(t) else {
        return Ok(f64::NAN);
    };
    if let Some(offset) = parse_time_zone_offset_string(identifier) {
        return Ok(t - offset_milliseconds(offset) as f64);
    }
    let instant = match probe(ms, identifier, possible)? {
        Some((first, _)) => first,
        None => match search_before(ms, identifier, possible)? {
            Some(last) => last,
            None => return Ok(f64::NAN),
        },
    };
    let answer = offset_ns(identifier, instant)?;
    // The offset is below 86,400,000 ms and `t` below 2^53 - 2^27, so the difference is exact.
    Ok(t - offset_milliseconds(answer) as f64)
}

/// The search for `tBefore`, "the largest integral Number < t for which possibleInstantsBefore
/// is not empty", bounded as `docs/design/engine.md` decision 6 states: a gallop at the
/// distances 1, 2, 4, ... 2^28 ms below `t` (29 probes; the first that is not empty ends it),
/// then a bisection between the last empty distance and the first that is not, one probe per
/// halving of an interval of at most 2^27 ms (27 probes), keeping the last instant of the
/// lowest distance that answered a list. `None` when no probe within 2^28 ms answered one.
fn search_before<E>(
    ms: i64,
    identifier: &str,
    possible: &mut dyn FnMut(&str, &IsoDateTime) -> Result<Instants, E>,
) -> Result<Option<i128>, E> {
    let mut empty: i64 = 0;
    let mut found = None;
    for exponent in 0..=SEARCH_EXPONENT {
        let distance = 1_i64 << exponent;
        if let Some((_, last)) = probe(ms - distance, identifier, possible)? {
            found = Some((distance, last));
            break;
        }
        empty = distance;
    }
    let Some((mut full, mut last)) = found else {
        return Ok(None);
    };
    while full - empty > 1 {
        let middle = empty + (full - empty) / 2;
        match probe(ms - middle, identifier, possible)? {
            Some((_, instant)) => {
                full = middle;
                last = instant;
            }
            None => empty = middle,
        }
    }
    Ok(Some(last))
}

/// One ask of `possible` for the local time `ms`. A local time beyond [`LOCAL_TIME_LIMIT`]
/// counts as an empty list and makes no call.
fn probe<E>(
    ms: i64,
    identifier: &str,
    possible: &mut dyn FnMut(&str, &IsoDateTime) -> Result<Instants, E>,
) -> Result<Option<(i128, i128)>, E> {
    let Some(fields) = fields_of(ms) else {
        return Ok(None);
    };
    let local = IsoDateTime {
        year: fields.year,
        month: fields.month + 1,
        day: fields.date,
        hour: fields.hour,
        minute: fields.minute,
        second: fields.second,
        millisecond: fields.millisecond,
        microsecond: 0,
        nanosecond: 0,
    };
    possible(identifier, &local)
}

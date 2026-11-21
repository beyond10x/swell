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
//! milliseconds of that day, which this module decides exactly for |balancedYear| ≤ 2^53. Beyond
//! that it answers NaN without the search: not decided, and reported for slice 4, which builds
//! `Date`; no year of a time value comes near it.
//!
//! The rest of the date arithmetic is slice 4's (design, section 12, "A part before its slice").

/// MillisecondsPerDay (`#sec-time-related-constants`).
const MS_PER_DAY: i128 = 86_400_000;

/// The first day of each month within a common year, January first.
const MONTH_STARTS: [i128; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

/// ToIntegerOrInfinity (`#sec-tointegerorinfinity`) of a finite Number, as a Number: the
/// truncated value, with -0 made +0.
fn to_integer(x: f64) -> f64 {
    x.trunc() + 0.0
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
            let rest = month_mv.rem_euclid(12.0);
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
    if x.is_finite() && x.trunc() == x && x.abs() < LIMIT {
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
    let low = day * MS_PER_DAY;
    let high = low + MS_PER_DAY;
    let mut x = low as f64;
    if (x as i128) < low {
        x = x.next_up();
    }
    x.is_finite() && (x as i128) < high
}

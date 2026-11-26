//! The text of a date: the three forms `Date.parse` and the String argument of `new Date` read
//! (`docs/design/engine.md` decision 6, "`Date.parse` and the String argument of `new Date`"),
//! and the strings `Date.prototype.toString`, `toDateString`, `toTimeString`, `toUTCString` and
//! `toISOString` are made of.
//!
//! [`parse_date_text`] reads exactly three forms, each the whole text, told apart by the first
//! code unit (an ASCII digit, `+` or `-` the first; an ASCII letter the second or third, by the
//! fourth code unit: a space or a comma):
//!
//! 1. The Date Time String Format (`#sec-date-time-string-format`, `#sec-expanded-years`):
//!    `YYYY`, `YYYY-MM` or `YYYY-MM-DD`, optionally `THH:mm`, `THH:mm:ss` or `THH:mm:ss.sss`,
//!    and after a time optionally `Z`, `+HH:mm` or `-HH:mm`.
//! 2. The form of `ToDateString` (`#sec-todatestring`): `Www Mmm DD Y HH:mm:ss GMT±HHmm`,
//!    optionally followed by ` (name)`.
//! 3. The form of `toUTCString` (`#sec-date.prototype.toutcstring`): `Www, DD Mmm Y HH:mm:ss GMT`.
//!
//! Every rule of range and of NaN is the design's; anything else is [`DateText::Invalid`]. The
//! read is one pass over the text with no backtracking and no allocation; the fields are `i64`
//! and small integers on the stack. A time value is `MakeDate(MakeDay, MakeTime)` of the
//! calendar ([`crate::date`]) minus the offset of the text; the caller applies `TimeClip`.
//!
//! The writers [`date_string`], [`time_string`], [`time_zone_string`], [`utc_string`] and
//! [`iso_string`] write ASCII into one `String` each, reserved at its final length first.

use alloc::string::String;

use crate::date::{DateFields, make_date, make_day, make_time};

/// What [`parse_date_text`] read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DateText {
    /// Not one of the three forms, or an element out of its range: `Date.parse` answers NaN.
    Invalid,
    /// The time value of a text that fixes its instant (a date-only form, a date-time form
    /// with `Z` or an offset, the forms of `toString` and `toUTCString`), the offset taken off,
    /// not clipped: the caller applies `TimeClip` (`#sec-timeclip`).
    Utc(f64),
    /// A date-time form without an offset: the fields read as a local time value, which the
    /// caller converts with [`crate::date::utc_from_local`] and then clips.
    Local(f64),
}

/// The names of Table 61 (`#sec-todatestring-day-names`), by WeekDay index (0 is Sunday).
const WEEKDAY_NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// The names of Table 62 (`#sec-todatestring-month-names`), by MonthFromTime index.
const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The days of each month of a common year, January first.
const MONTH_LENGTHS: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

const MS_PER_MINUTE: i64 = 60_000;

/// Reads `text` as one of the three forms of `Date.parse` (the module header). `text` is the
/// whole String as code units.
pub fn parse_date_text<U: Copy + Into<u32>>(text: &[U]) -> DateText {
    let mut reader = Reader { text, at: 0 };
    let read = match reader.peek() {
        Some(c) if is_digit(c) || c == u32::from(b'+') || c == u32::from(b'-') => {
            date_time_string(&mut reader)
        }
        Some(c) if is_ascii_letter(c) => date_string_form(&mut reader),
        _ => None,
    };
    read.unwrap_or(DateText::Invalid)
}

/// The text and the index of the next code unit to read. Every read moves forward; nothing
/// is read twice.
struct Reader<'a, U> {
    text: &'a [U],
    at: usize,
}

impl<U: Copy + Into<u32>> Reader<'_, U> {
    /// The next code unit, not read.
    fn peek(&self) -> Option<u32> {
        self.text.get(self.at).map(|&unit| unit.into())
    }

    /// The next code unit, read.
    fn next(&mut self) -> Option<u32> {
        let unit = self.peek()?;
        // `at` is below the length of a slice, so the sum does not overflow.
        self.at += 1;
        Some(unit)
    }

    /// Whether every code unit is read.
    fn at_end(&self) -> bool {
        self.at >= self.text.len()
    }

    /// Reads `expected`; `None` for any other code unit or the end.
    fn eat(&mut self, expected: u8) -> Option<()> {
        (self.next()? == u32::from(expected)).then_some(())
    }

    /// Reads the ASCII code units of `expected`, one after the other.
    fn eat_all(&mut self, expected: &[u8]) -> Option<()> {
        for &unit in expected {
            self.eat(unit)?;
        }
        Some(())
    }

    /// Reads the next code unit when it is `expected`; whether it was.
    fn eat_if(&mut self, expected: u8) -> bool {
        let found = self.peek() == Some(u32::from(expected));
        if found {
            self.at += 1;
        }
        found
    }

    /// Reads one ASCII digit.
    fn digit(&mut self) -> Option<i64> {
        let unit = self.next()?;
        is_digit(unit).then(|| i64::from(unit - u32::from(b'0')))
    }

    /// Reads exactly `count` ASCII digits (`count` at most 6, so the value is below 10^6).
    fn digits(&mut self, count: usize) -> Option<i64> {
        let mut value = 0_i64;
        for _ in 0..count {
            value = value.checked_mul(10)?.checked_add(self.digit()?)?;
        }
        Some(value)
    }

    /// Reads two ASCII digits whose value is `low` to `high`.
    fn two_digits(&mut self, low: i64, high: i64) -> Option<i64> {
        self.digits(2).filter(|value| (low..=high).contains(value))
    }

    /// Reads three code units that are one of `names` and answers its index.
    fn name(&mut self, names: &[&str]) -> Option<usize> {
        let read = [self.next()?, self.next()?, self.next()?];
        names.iter().position(|name| {
            name.len() == read.len()
                && name
                    .bytes()
                    .zip(read)
                    .all(|(expected, unit)| u32::from(expected) == unit)
        })
    }
}

fn is_digit(unit: u32) -> bool {
    (u32::from(b'0')..=u32::from(b'9')).contains(&unit)
}

fn is_ascii_letter(unit: u32) -> bool {
    (u32::from(b'A')..=u32::from(b'Z')).contains(&unit)
        || (u32::from(b'a')..=u32::from(b'z')).contains(&unit)
}

/// `IsLeapYear ( y )` (`#sec-isleapyear`) of a year as an integer.
fn is_leap_year(year: i64) -> bool {
    year.rem_euclid(400) == 0 || (year.rem_euclid(100) != 0 && year.rem_euclid(4) == 0)
}

/// The number of days of month `month` (1 to 12) of `year`; `None` for another month.
fn month_length(year: i64, month: i64) -> Option<i64> {
    let index = usize::try_from(month.checked_sub(1)?).ok()?;
    let days = *MONTH_LENGTHS.get(index)?;
    Some(if month == 2 && is_leap_year(year) {
        days + 1
    } else {
        days
    })
}

/// The fields of a date and a time, as a text gives them.
struct Fields {
    year: i64,
    /// 1 to 12.
    month: i64,
    /// 1 to the length of the month.
    day: i64,
    /// 0 to 24; 24 only with the other three 0.
    hour: i64,
    minute: i64,
    second: i64,
    millisecond: i64,
}

/// A small integer as a Number: `None` beyond the range of `i32`, which no field of a text
/// reaches (a year is below 10^6), so the conversion is exact.
fn number(value: i64) -> Option<f64> {
    i32::try_from(value).ok().map(f64::from)
}

/// `MakeDate(MakeDay(year, month - 1, day), MakeTime(hour, minute, second, millisecond))` minus
/// `offset_minutes` minutes, as the Number arithmetic of the calendar computes it; `None` when
/// it is not finite (no text of the three forms makes it so).
fn time_value(fields: &Fields, offset_minutes: i64) -> Option<f64> {
    let day = make_day(
        number(fields.year)?,
        number(fields.month.checked_sub(1)?)?,
        number(fields.day)?,
    );
    let time = make_time(
        number(fields.hour)?,
        number(fields.minute)?,
        number(fields.second)?,
        number(fields.millisecond)?,
    );
    // An offset is below 24 hours: at most 86,340,000 ms in magnitude.
    let offset = number(offset_minutes.checked_mul(MS_PER_MINUTE)?)?;
    let value = make_date(day, time) - offset;
    value.is_finite().then_some(value)
}

/// The first form, the Date Time String Format, from its first code unit.
fn date_time_string<U: Copy + Into<u32>>(reader: &mut Reader<'_, U>) -> Option<DateText> {
    let year = if reader.eat_if(b'+') {
        reader.digits(6)?
    } else if reader.eat_if(b'-') {
        // `-000000` is not a year (`#sec-expanded-years`).
        let magnitude = reader.digits(6)?;
        if magnitude == 0 {
            return None;
        }
        -magnitude
    } else {
        reader.digits(4)?
    };
    let mut month = 1;
    let mut day = 1;
    if reader.eat_if(b'-') {
        month = reader.two_digits(1, 12)?;
        if reader.eat_if(b'-') {
            day = reader.two_digits(1, month_length(year, month)?)?;
        }
    }
    let mut fields = Fields {
        year,
        month,
        day,
        hour: 0,
        minute: 0,
        second: 0,
        millisecond: 0,
    };
    if reader.at_end() {
        // A date-only form is read as UTC and takes no offset.
        return Some(DateText::Utc(time_value(&fields, 0)?));
    }
    reader.eat(b'T')?;
    fields.hour = reader.two_digits(0, 24)?;
    reader.eat(b':')?;
    fields.minute = reader.two_digits(0, 59)?;
    if reader.eat_if(b':') {
        fields.second = reader.two_digits(0, 59)?;
        if reader.eat_if(b'.') {
            fields.millisecond = reader.digits(3)?;
        }
    }
    // HH of 24 is 00:00 of the next day, and only with every other element 0 (Note 1 of
    // `#sec-date-time-string-format`); MakeTime of hour 24 is that instant.
    if fields.hour == 24 && (fields.minute, fields.second, fields.millisecond) != (0, 0, 0) {
        return None;
    }
    let Some(unit) = reader.next() else {
        return Some(DateText::Local(time_value(&fields, 0)?));
    };
    let offset_minutes = if unit == u32::from(b'Z') {
        0
    } else {
        let sign = if unit == u32::from(b'+') {
            1
        } else if unit == u32::from(b'-') {
            -1
        } else {
            return None;
        };
        let hours = reader.two_digits(0, 23)?;
        reader.eat(b':')?;
        let minutes = reader.two_digits(0, 59)?;
        sign * (hours * 60 + minutes)
    };
    if !reader.at_end() {
        return None;
    }
    Some(DateText::Utc(time_value(&fields, offset_minutes)?))
}

/// The second and third forms, from their first code unit: the weekday, read and not checked,
/// then a space (the form of `toString`) or a comma (the form of `toUTCString`).
fn date_string_form<U: Copy + Into<u32>>(reader: &mut Reader<'_, U>) -> Option<DateText> {
    reader.name(&WEEKDAY_NAMES)?;
    let separator = reader.next()?;
    let (month, day, year) = if separator == u32::from(b' ') {
        let month = reader.name(&MONTH_NAMES)?;
        reader.eat(b' ')?;
        let day = reader.digits(2)?;
        reader.eat(b' ')?;
        (month, day, year_of_date_string(reader)?)
    } else if separator == u32::from(b',') {
        reader.eat(b' ')?;
        let day = reader.digits(2)?;
        reader.eat(b' ')?;
        let month = reader.name(&MONTH_NAMES)?;
        reader.eat(b' ')?;
        (month, day, year_of_date_string(reader)?)
    } else {
        return None;
    };
    // The index of a name of Table 62 is 0 to 11.
    let month = i64::try_from(month).ok()?.checked_add(1)?;
    if !(1..=month_length(year, month)?).contains(&day) {
        return None;
    }
    reader.eat(b' ')?;
    let hour = reader.two_digits(0, 23)?;
    reader.eat(b':')?;
    let minute = reader.two_digits(0, 59)?;
    reader.eat(b':')?;
    let second = reader.two_digits(0, 59)?;
    reader.eat_all(b" GMT")?;
    let fields = Fields {
        year,
        month,
        day,
        hour,
        minute,
        second,
        millisecond: 0,
    };
    if separator == u32::from(b',') {
        if !reader.at_end() {
            return None;
        }
        return Some(DateText::Utc(time_value(&fields, 0)?));
    }
    let sign = match reader.next()? {
        unit if unit == u32::from(b'+') => 1,
        unit if unit == u32::from(b'-') => -1,
        _ => return None,
    };
    let hours = reader.two_digits(0, 23)?;
    let minutes = reader.two_digits(0, 59)?;
    if !reader.at_end() {
        time_zone_name(reader)?;
    }
    Some(DateText::Utc(time_value(
        &fields,
        sign * (hours * 60 + minutes),
    )?))
}

/// The year of `DateString`: four to six ASCII digits, with a `-` before them for a year below
/// 0; a `-` before a year of 0 is no year.
fn year_of_date_string<U: Copy + Into<u32>>(reader: &mut Reader<'_, U>) -> Option<i64> {
    let negative = reader.eat_if(b'-');
    let mut magnitude = reader.digits(4)?;
    for _ in 4..6 {
        if !reader.peek().is_some_and(is_digit) {
            break;
        }
        magnitude = magnitude.checked_mul(10)?.checked_add(reader.digit()?)?;
    }
    match (negative, magnitude) {
        (true, 0) => None,
        (true, _) => Some(-magnitude),
        (false, _) => Some(magnitude),
    }
}

/// The name after the offset of the form of `toString`, ignored: a space, `(`, one or more
/// code units other than `(` and `)`, and `)` as the last code unit of the text.
fn time_zone_name<U: Copy + Into<u32>>(reader: &mut Reader<'_, U>) -> Option<()> {
    reader.eat_all(b" (")?;
    let mut length = 0_usize;
    loop {
        let unit = reader.next()?;
        if unit == u32::from(b')') {
            break;
        }
        if unit == u32::from(b'(') {
            return None;
        }
        length = length.saturating_add(1);
    }
    (length > 0 && reader.at_end()).then_some(())
}

// ---------------------------------------------------------------------------------------------
// The writers.

/// The number of decimal digits of `value`.
fn decimal_length(value: u64) -> usize {
    let mut length = 1;
    let mut rest = value / 10;
    while rest > 0 {
        length += 1;
        rest /= 10;
    }
    length
}

/// The length [`push_padded`] writes for `value` and `width`.
fn padded_length(value: u64, width: usize) -> usize {
    decimal_length(value).max(width)
}

/// Writes `value` in decimal, padded with zeros on the left to `width` digits
/// (`ToZeroPaddedDecimalString`, `#sec-tozeropaddeddecimalstring`).
fn push_padded(buf: &mut String, value: u64, width: usize) {
    let length = decimal_length(value);
    for _ in length..width {
        buf.push('0');
    }
    for place in (0..length).rev() {
        // `place` is below 20, the digits of a u64, so the power is at most 10^19.
        let power = u32::try_from(place)
            .ok()
            .and_then(|exponent| 10_u64.checked_pow(exponent))
            .unwrap_or(1);
        let digit = value / power % 10;
        buf.push(char::from_digit(u32::try_from(digit).unwrap_or(0), 10).unwrap_or('0'));
    }
}

/// The name of index `index` of `names`; a name of three letters. An index outside the table,
/// which [`crate::date::date_fields`] never gives, writes `"???"`.
fn name_of(names: &[&'static str], index: u8) -> &'static str {
    names.get(usize::from(index)).copied().unwrap_or("???")
}

/// The length of the year as `DateString` and `toUTCString` write it.
fn padded_year_length(year: i64) -> usize {
    usize::from(year < 0) + padded_length(year.unsigned_abs(), 4)
}

/// The year as `DateString` and `toUTCString` write it: `-` when negative, then the magnitude
/// padded to four digits (`ToZeroPaddedDecimalString(abs(yv), 4)`).
fn push_padded_year(buf: &mut String, year: i64) {
    if year < 0 {
        buf.push('-');
    }
    push_padded(buf, year.unsigned_abs(), 4);
}

/// `DateString ( tv )` (`#sec-datestring`) of the fields of `tv`: `Www Mmm DD YYYY`, the year
/// padded to four digits with a `-` only when it is negative. At most 18 bytes for the fields
/// of a time value.
pub fn date_string(fields: &DateFields) -> String {
    let date = u64::from(fields.date);
    let length = 3 + 1 + 3 + 1 + padded_length(date, 2) + 1 + padded_year_length(fields.year);
    let mut buf = String::with_capacity(length);
    buf.push_str(name_of(&WEEKDAY_NAMES, fields.weekday));
    buf.push(' ');
    buf.push_str(name_of(&MONTH_NAMES, fields.month));
    buf.push(' ');
    push_padded(&mut buf, date, 2);
    buf.push(' ');
    push_padded_year(&mut buf, fields.year);
    buf
}

/// The length of `HH:mm:ss` for `fields`.
fn clock_length(fields: &DateFields) -> usize {
    padded_length(u64::from(fields.hour), 2)
        + 1
        + padded_length(u64::from(fields.minute), 2)
        + 1
        + padded_length(u64::from(fields.second), 2)
}

/// Writes `HH:mm:ss`.
fn push_clock(buf: &mut String, fields: &DateFields) {
    push_padded(buf, u64::from(fields.hour), 2);
    buf.push(':');
    push_padded(buf, u64::from(fields.minute), 2);
    buf.push(':');
    push_padded(buf, u64::from(fields.second), 2);
}

/// `TimeString ( tv )` (`#sec-timestring`) of the fields of `tv` without its last part:
/// `HH:mm:ss`. The clause ends the string with a space and `GMT`; here [`time_zone_string`]
/// writes `GMT`, so `ToDateString` is `date_string`, a space, `time_string`, a space and
/// `time_zone_string`, and `toTimeString` is `time_string`, a space and `time_zone_string`.
/// HourFromTime is 0 to 23, so the hour is never 24. 8 bytes for the fields of a time value.
pub fn time_string(fields: &DateFields) -> String {
    let mut buf = String::with_capacity(clock_length(fields));
    push_clock(&mut buf, fields);
    buf
}

/// `GMT` and `TimeZoneString`'s offset (`#sec-timezoneestring`) for an offset of `offset_ms`
/// whole milliseconds, as [`crate::date::offset_milliseconds`] gives it: `GMT`, `+` for an
/// offset of 0 or more and else `-`, then HourFromTime and MinuteFromTime of its magnitude,
/// two digits each. The seconds of an offset are not written; no name is written (no hook
/// answers one). 8 bytes.
pub fn time_zone_string(offset_ms: i64) -> String {
    let magnitude = offset_ms.unsigned_abs();
    let hours = magnitude / 3_600_000 % 24;
    let minutes = magnitude / 60_000 % 60;
    let mut buf = String::with_capacity(8);
    buf.push_str("GMT");
    buf.push(if offset_ms >= 0 { '+' } else { '-' });
    push_padded(&mut buf, hours, 2);
    push_padded(&mut buf, minutes, 2);
    buf
}

/// `Date.prototype.toUTCString` (`#sec-date.prototype.toutcstring`) of the fields of `tv`:
/// `Www, DD Mmm YYYY HH:mm:ss GMT`, the year as [`date_string`] writes it. At most 32 bytes
/// for the fields of a time value.
pub fn utc_string(fields: &DateFields) -> String {
    let date = u64::from(fields.date);
    let length = 3
        + 2
        + padded_length(date, 2)
        + 1
        + 3
        + 1
        + padded_year_length(fields.year)
        + 1
        + clock_length(fields)
        + 4;
    let mut buf = String::with_capacity(length);
    buf.push_str(name_of(&WEEKDAY_NAMES, fields.weekday));
    buf.push_str(", ");
    push_padded(&mut buf, date, 2);
    buf.push(' ');
    buf.push_str(name_of(&MONTH_NAMES, fields.month));
    buf.push(' ');
    push_padded_year(&mut buf, fields.year);
    buf.push(' ');
    push_clock(&mut buf, fields);
    buf.push_str(" GMT");
    buf
}

/// `Date.prototype.toISOString` (`#sec-date.prototype.toisostring`) of the fields of `tv`: the
/// Date Time String Format with every element and `Z`, `YYYY-MM-DDTHH:mm:ss.sssZ`; the year as
/// four digits for 0 to 9999 and else as a sign and six digits (`#sec-expanded-years`). At
/// most 27 bytes for the fields of a time value.
pub fn iso_string(fields: &DateFields) -> String {
    let expanded = !(0..=9_999).contains(&fields.year);
    let magnitude = fields.year.unsigned_abs();
    let year_length = if expanded {
        1 + padded_length(magnitude, 6)
    } else {
        4
    };
    let month = u64::from(fields.month) + 1;
    let date = u64::from(fields.date);
    let length = year_length
        + 1
        + padded_length(month, 2)
        + 1
        + padded_length(date, 2)
        + 1
        + clock_length(fields)
        + 1
        + padded_length(u64::from(fields.millisecond), 3)
        + 1;
    let mut buf = String::with_capacity(length);
    if expanded {
        buf.push(if fields.year < 0 { '-' } else { '+' });
        push_padded(&mut buf, magnitude, 6);
    } else {
        push_padded(&mut buf, magnitude, 4);
    }
    buf.push('-');
    push_padded(&mut buf, month, 2);
    buf.push('-');
    push_padded(&mut buf, date, 2);
    buf.push('T');
    push_clock(&mut buf, fields);
    buf.push('.');
    push_padded(&mut buf, u64::from(fields.millisecond), 3);
    buf.push('Z');
    buf
}

//! Date objects (ECMA-262 21.4, `#sec-date-objects`), the clock half, written by
//! `story:date-object-clock-and-time-value`: `%Date%` (`#sec-date-constructor`) with
//! `Date.now` (`#sec-date.now`), `%Date.prototype%`
//! (`#sec-properties-of-the-date-prototype-object`) with `getTime`
//! (`#sec-date.prototype.gettime`), `setTime` (`#sec-date.prototype.settime`) and `valueOf`
//! (`#sec-date.prototype.valueof`), the time value of a Date ([[DateValue]],
//! `#sec-properties-of-date-instances`, held as `OrdinaryData::Date`), and the read of the host
//! clock (`current_time_value`, `docs/design/host-interface.md`, HOST-125 and HOST-126).
//!
//! The zone half, written by `story:date-object-constructor-now-utc-and-getters`: the
//! constructor with two or more arguments (step 5 of `#sec-date-constructor`), `Date.UTC`
//! (`#sec-date.utc`) and the 17 getters of 21.4.4 but `getTime` (`#sec-date.prototype.getdate`
//! to `#sec-date.prototype.getutcseconds`), over the two conversions `local_time_of`
//! (`#sec-localtime`) and `utc_of_local` (`#sec-utc-t`). Each conversion reads the host's
//! identifier once (`system_time_zone_identifier`) and asks the offset and the epoch hooks
//! through closures that `swell_num::date::local_time` and `utc_from_local` call, which clamp
//! every offset and bound the search for `tBefore` (`docs/design/host-interface.md`, decision
//! 17; HOST-128, HOST-209, HOST-210, HOST-211).
//!
//! The setters half, written by `story:date-setters`: the 14 setters of 21.4.4 but `setTime`
//! (`#sec-date.prototype.setdate` to `#sec-date.prototype.setutcseconds`), one function over a
//! table row each (`setter_of`): the time value read first, ToNumber of each argument present,
//! then for a local setter `LocalTime` and `UTC` of the zone half (at most 2 + 59 = 61 hook
//! calls, decision 17, item 5), TimeClip, and the store last.
//!
//! The text half, written by `story:date-text-formats` over `swell_num::date_text`
//! (`docs/design/engine.md`, decision 6, "`Date.parse` and the String argument of `new Date`"
//! and "The text of toLocale*String"): `Date.parse` (`#sec-date.parse`) and the String argument
//! of the constructor (step 4.c.ii of `#sec-date-constructor`, `time_value_of_text`), read in
//! place after a step per code unit past 32; ToDateString (`#sec-todatestring`) of the call
//! without `new` (step 1) and of `toString` (`#sec-date.prototype.tostring`), `toDateString`
//! (`#sec-date.prototype.todatestring`), `toTimeString` (`#sec-date.prototype.totimestring`),
//! `toUTCString` (`#sec-date.prototype.toutcstring`), `toISOString`
//! (`#sec-date.prototype.toisostring`) and the three `toLocale*String`
//! (`#sec-date.prototype.tolocalestring`), each the text of one conversion (`date_text`):
//! `LocalTime` once (2 hook calls, 1 for an offset-string identifier, none for NaN), whose one
//! offset gives both the local time and the `GMT±hhmm` (decision 17, item 1); `toJSON`
//! (`#sec-date.prototype.tojson`) and `[Symbol.toPrimitive]`
//! (`#sec-date.prototype-%symbol.toprimitive%`).
//!
//! The builtins re-enter script only through ToPrimitive, ToString, ToNumber and the Invoke of
//! `toJSON`, as `Number` does. The loops are the search of `swell-num` (a charged hook call
//! per probe), the ToNumber of at most seven arguments, the fields of a setter (at most 4
//! arguments, 7 fields) and the one pass of the parse over a String, charged a step per code
//! unit past 32 before the first is read. Every call of a clock or time zone hook is charged
//! one step before it is made (decision 17, item 6).

use std::cell::RefCell;

use swell_num::date::{
    DateFields, date_fields, local_time, make_date, make_day, make_full_year, make_time,
    utc_from_local,
};
use swell_num::date_text::{
    DateText, date_string, iso_string, parse_date_text, time_string, time_zone_string, utc_string,
};
use swell_num::time_value::time_clip;

use crate::builtins::Method;
use crate::builtins::object::argument;
use crate::heap::string::CodeUnits;
use crate::host::agent::Agent;
use crate::host::outcome::{Completion, Termination};
use crate::host::{ErrorKind, HookCx, Host, IsoDateTime, PreferredType};
use crate::object::ObjectKind;
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Key, Property};
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, StringRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DateBuiltin {
    /// `Date ( ...values )` (21.4.2.1).
    Constructor,
    /// `Date.now ( )` (21.4.3.1).
    Now,
    /// `Date.parse ( string )` (21.4.3.2).
    Parse,
    /// `Date.UTC ( year [ , month [ , ... ] ] )` (21.4.3.4).
    Utc,
    /// `Date.prototype.getDate ( )` (21.4.4.2).
    GetDate,
    /// `Date.prototype.getDay ( )` (21.4.4.3).
    GetDay,
    /// `Date.prototype.getFullYear ( )` (21.4.4.4).
    GetFullYear,
    /// `Date.prototype.getHours ( )` (21.4.4.5).
    GetHours,
    /// `Date.prototype.getMilliseconds ( )` (21.4.4.6).
    GetMilliseconds,
    /// `Date.prototype.getMinutes ( )` (21.4.4.7).
    GetMinutes,
    /// `Date.prototype.getMonth ( )` (21.4.4.8).
    GetMonth,
    /// `Date.prototype.getSeconds ( )` (21.4.4.9).
    GetSeconds,
    /// `Date.prototype.getTime ( )` (21.4.4.10).
    GetTime,
    /// `Date.prototype.getTimezoneOffset ( )` (21.4.4.11).
    GetTimezoneOffset,
    /// `Date.prototype.getUTCDate ( )` (21.4.4.12).
    GetUtcDate,
    /// `Date.prototype.getUTCDay ( )` (21.4.4.13).
    GetUtcDay,
    /// `Date.prototype.getUTCFullYear ( )` (21.4.4.14).
    GetUtcFullYear,
    /// `Date.prototype.getUTCHours ( )` (21.4.4.15).
    GetUtcHours,
    /// `Date.prototype.getUTCMilliseconds ( )` (21.4.4.16).
    GetUtcMilliseconds,
    /// `Date.prototype.getUTCMinutes ( )` (21.4.4.17).
    GetUtcMinutes,
    /// `Date.prototype.getUTCMonth ( )` (21.4.4.18).
    GetUtcMonth,
    /// `Date.prototype.getUTCSeconds ( )` (21.4.4.19).
    GetUtcSeconds,
    /// `Date.prototype.setDate ( date )` (21.4.4.20).
    SetDate,
    /// `Date.prototype.setFullYear ( year [ , month [ , date ] ] )` (21.4.4.21).
    SetFullYear,
    /// `Date.prototype.setHours ( hour [ , min [ , sec [ , ms ] ] ] )` (21.4.4.22).
    SetHours,
    /// `Date.prototype.setMilliseconds ( ms )` (21.4.4.23).
    SetMilliseconds,
    /// `Date.prototype.setMinutes ( min [ , sec [ , ms ] ] )` (21.4.4.24).
    SetMinutes,
    /// `Date.prototype.setMonth ( month [ , date ] )` (21.4.4.25).
    SetMonth,
    /// `Date.prototype.setSeconds ( sec [ , ms ] )` (21.4.4.26).
    SetSeconds,
    /// `Date.prototype.setTime ( time )` (21.4.4.27).
    SetTime,
    /// `Date.prototype.setUTCDate ( date )` (21.4.4.28).
    SetUtcDate,
    /// `Date.prototype.setUTCFullYear ( year [ , month [ , date ] ] )` (21.4.4.29).
    SetUtcFullYear,
    /// `Date.prototype.setUTCHours ( hour [ , min [ , sec [ , ms ] ] ] )` (21.4.4.30).
    SetUtcHours,
    /// `Date.prototype.setUTCMilliseconds ( ms )` (21.4.4.31).
    SetUtcMilliseconds,
    /// `Date.prototype.setUTCMinutes ( min [ , sec [ , ms ] ] )` (21.4.4.32).
    SetUtcMinutes,
    /// `Date.prototype.setUTCMonth ( month [ , date ] )` (21.4.4.33).
    SetUtcMonth,
    /// `Date.prototype.setUTCSeconds ( sec [ , ms ] )` (21.4.4.34).
    SetUtcSeconds,
    /// `Date.prototype.toDateString ( )` (21.4.4.35).
    ToDateString,
    /// `Date.prototype.toISOString ( )` (21.4.4.36).
    ToIsoString,
    /// `Date.prototype.toJSON ( key )` (21.4.4.37).
    ToJson,
    /// `Date.prototype.toLocaleDateString ( [ reserved1 [ , reserved2 ] ] )` (21.4.4.38).
    ToLocaleDateString,
    /// `Date.prototype.toLocaleString ( [ reserved1 [ , reserved2 ] ] )` (21.4.4.39).
    ToLocaleString,
    /// `Date.prototype.toLocaleTimeString ( [ reserved1 [ , reserved2 ] ] )` (21.4.4.40).
    ToLocaleTimeString,
    /// `Date.prototype.toString ( )` (21.4.4.41).
    ToString,
    /// `Date.prototype.toTimeString ( )` (21.4.4.42).
    ToTimeString,
    /// `Date.prototype.toUTCString ( )` (21.4.4.43).
    ToUtcString,
    /// `Date.prototype.valueOf ( )` (21.4.4.44).
    ValueOf,
    /// `Date.prototype [ %Symbol.toPrimitive% ] ( hint )` (21.4.4.45).
    ToPrimitive,
}

/// The functions of `%Date%` (21.4.3) but `prototype`, in the order of the clauses; `length`
/// as each clause's parameters give it (21.4.3.4: "The "length" property of this function is
/// 7𝔽").
const CONSTRUCTOR_METHODS: [Method<DateBuiltin>; 3] = [
    ("now", 0, DateBuiltin::Now),
    ("parse", 1, DateBuiltin::Parse),
    ("UTC", 7, DateBuiltin::Utc),
];

/// The methods of `%Date.prototype%` (21.4.4) keyed by a String, in the order of the clauses;
/// `length` as each clause's required parameters give it, and as its "length" sentence says
/// where it has one. `[Symbol.toPrimitive]` (21.4.4.45) is defined after them.
const PROTOTYPE_METHODS: [Method<DateBuiltin>; 43] = [
    ("getDate", 0, DateBuiltin::GetDate),
    ("getDay", 0, DateBuiltin::GetDay),
    ("getFullYear", 0, DateBuiltin::GetFullYear),
    ("getHours", 0, DateBuiltin::GetHours),
    ("getMilliseconds", 0, DateBuiltin::GetMilliseconds),
    ("getMinutes", 0, DateBuiltin::GetMinutes),
    ("getMonth", 0, DateBuiltin::GetMonth),
    ("getSeconds", 0, DateBuiltin::GetSeconds),
    ("getTime", 0, DateBuiltin::GetTime),
    ("getTimezoneOffset", 0, DateBuiltin::GetTimezoneOffset),
    ("getUTCDate", 0, DateBuiltin::GetUtcDate),
    ("getUTCDay", 0, DateBuiltin::GetUtcDay),
    ("getUTCFullYear", 0, DateBuiltin::GetUtcFullYear),
    ("getUTCHours", 0, DateBuiltin::GetUtcHours),
    ("getUTCMilliseconds", 0, DateBuiltin::GetUtcMilliseconds),
    ("getUTCMinutes", 0, DateBuiltin::GetUtcMinutes),
    ("getUTCMonth", 0, DateBuiltin::GetUtcMonth),
    ("getUTCSeconds", 0, DateBuiltin::GetUtcSeconds),
    ("setDate", 1, DateBuiltin::SetDate),
    ("setFullYear", 3, DateBuiltin::SetFullYear),
    ("setHours", 4, DateBuiltin::SetHours),
    ("setMilliseconds", 1, DateBuiltin::SetMilliseconds),
    ("setMinutes", 3, DateBuiltin::SetMinutes),
    ("setMonth", 2, DateBuiltin::SetMonth),
    ("setSeconds", 2, DateBuiltin::SetSeconds),
    ("setTime", 1, DateBuiltin::SetTime),
    ("setUTCDate", 1, DateBuiltin::SetUtcDate),
    ("setUTCFullYear", 3, DateBuiltin::SetUtcFullYear),
    ("setUTCHours", 4, DateBuiltin::SetUtcHours),
    ("setUTCMilliseconds", 1, DateBuiltin::SetUtcMilliseconds),
    ("setUTCMinutes", 3, DateBuiltin::SetUtcMinutes),
    ("setUTCMonth", 2, DateBuiltin::SetUtcMonth),
    ("setUTCSeconds", 2, DateBuiltin::SetUtcSeconds),
    ("toDateString", 0, DateBuiltin::ToDateString),
    ("toISOString", 0, DateBuiltin::ToIsoString),
    ("toJSON", 1, DateBuiltin::ToJson),
    ("toLocaleDateString", 0, DateBuiltin::ToLocaleDateString),
    ("toLocaleString", 0, DateBuiltin::ToLocaleString),
    ("toLocaleTimeString", 0, DateBuiltin::ToLocaleTimeString),
    ("toString", 0, DateBuiltin::ToString),
    ("toTimeString", 0, DateBuiltin::ToTimeString),
    ("toUTCString", 0, DateBuiltin::ToUtcString),
    ("valueOf", 0, DateBuiltin::ValueOf),
];

/// The bound of the clock's answer: ±8.64 × 10^21 nanoseconds, the time values' range
/// (`#sec-time-values-and-time-range`) in nanoseconds (HOST-126).
const MAX_EPOCH_NANOSECONDS: i128 = 8_640_000_000_000_000_000_000;

/// Nanoseconds in a millisecond.
const NANOSECONDS_PER_MILLISECOND: i128 = 1_000_000;

/// msPerMinute (`#sec-time-related-constants`), as `getTimezoneOffset` divides by it.
const MS_PER_MINUTE: f64 = 60_000.0;

/// The code units of an identifier a conversion hands to `swell-num` to tell an offset string
/// from a named zone: the longest offset string has 19 (decision 17, item 2), and a twentieth
/// shows that the identifier is longer.
const IDENTIFIER_PREFIX_UNITS: usize = 20;

/// The bytes the memory budget is charged for each instant of the list the epoch hook answers
/// (decision 17, item 7): an `i128`.
const INSTANT_BYTES: u64 = 16;

/// A field of a time value a getter reads (`swell_num::date::DateFields`).
#[derive(Clone, Copy)]
enum Field {
    Year,
    Month,
    Date,
    WeekDay,
    Hour,
    Minute,
    Second,
    Millisecond,
}

/// What a getter answers: a field of the local time, a field of the time value itself, or
/// the offset in minutes.
#[derive(Clone, Copy)]
enum Getter {
    Local(Field),
    Utc(Field),
    TimezoneOffset,
}

/// The name and the reading of the getter `id`; `None` for a builtin that is not one.
fn getter_of(id: DateBuiltin) -> Option<(&'static str, Getter)> {
    Some(match id {
        DateBuiltin::GetDate => ("Date.prototype.getDate", Getter::Local(Field::Date)),
        DateBuiltin::GetDay => ("Date.prototype.getDay", Getter::Local(Field::WeekDay)),
        DateBuiltin::GetFullYear => ("Date.prototype.getFullYear", Getter::Local(Field::Year)),
        DateBuiltin::GetHours => ("Date.prototype.getHours", Getter::Local(Field::Hour)),
        DateBuiltin::GetMilliseconds => (
            "Date.prototype.getMilliseconds",
            Getter::Local(Field::Millisecond),
        ),
        DateBuiltin::GetMinutes => ("Date.prototype.getMinutes", Getter::Local(Field::Minute)),
        DateBuiltin::GetMonth => ("Date.prototype.getMonth", Getter::Local(Field::Month)),
        DateBuiltin::GetSeconds => ("Date.prototype.getSeconds", Getter::Local(Field::Second)),
        DateBuiltin::GetTimezoneOffset => {
            ("Date.prototype.getTimezoneOffset", Getter::TimezoneOffset)
        }
        DateBuiltin::GetUtcDate => ("Date.prototype.getUTCDate", Getter::Utc(Field::Date)),
        DateBuiltin::GetUtcDay => ("Date.prototype.getUTCDay", Getter::Utc(Field::WeekDay)),
        DateBuiltin::GetUtcFullYear => ("Date.prototype.getUTCFullYear", Getter::Utc(Field::Year)),
        DateBuiltin::GetUtcHours => ("Date.prototype.getUTCHours", Getter::Utc(Field::Hour)),
        DateBuiltin::GetUtcMilliseconds => (
            "Date.prototype.getUTCMilliseconds",
            Getter::Utc(Field::Millisecond),
        ),
        DateBuiltin::GetUtcMinutes => ("Date.prototype.getUTCMinutes", Getter::Utc(Field::Minute)),
        DateBuiltin::GetUtcMonth => ("Date.prototype.getUTCMonth", Getter::Utc(Field::Month)),
        DateBuiltin::GetUtcSeconds => ("Date.prototype.getUTCSeconds", Getter::Utc(Field::Second)),
        _ => return None,
    })
}

/// The field `field` of the time value `t` as a Number: YearFromTime, MonthFromTime,
/// DateFromTime, WeekDay, HourFromTime, MinuteFromTime, SecondFromTime or msFromTime
/// (`#sec-year-number` to `#sec-ms-from-time`). NaN where `date_fields` has no fields, which a
/// time value and its local time never are. Every field is an integer below 2^53 in magnitude
/// (a year of a local time is at most 275,761), so the conversions are exact.
fn field_of(t: f64, field: Field) -> f64 {
    let Some(fields) = date_fields(t) else {
        return f64::NAN;
    };
    match field {
        Field::Year => i32::try_from(fields.year).map_or(f64::NAN, f64::from),
        Field::Month => f64::from(fields.month),
        Field::Date => f64::from(fields.date),
        Field::WeekDay => f64::from(fields.weekday),
        Field::Hour => f64::from(fields.hour),
        Field::Minute => f64::from(fields.minute),
        Field::Second => f64::from(fields.second),
        Field::Millisecond => f64::from(fields.millisecond),
    }
}

/// A setter of `%Date.prototype%` (21.4.4.20 to 21.4.4.34 but `setTime`): its name, the
/// position in [`fields_of_time`] of the first field it writes, the number of arguments it
/// reads (its "length"; the fields it writes follow the first), and whether it works on the
/// local time (`LocalTime` before, `UTC` after) or on the time value itself.
#[derive(Clone, Copy)]
struct Setter {
    name: &'static str,
    first: usize,
    length: usize,
    local: bool,
}

/// The position of the year in [`fields_of_time`]: the full-year setters start from +0 for a
/// time value of NaN (21.4.4.21 step 5, 21.4.4.29 step 4).
const YEAR: usize = 0;

/// The row of the setter `id`; `None` for a builtin that is not one.
fn setter_of(id: DateBuiltin) -> Option<Setter> {
    let (name, first, length, local) = match id {
        DateBuiltin::SetDate => ("Date.prototype.setDate", 2, 1, true),
        DateBuiltin::SetFullYear => ("Date.prototype.setFullYear", YEAR, 3, true),
        DateBuiltin::SetHours => ("Date.prototype.setHours", 3, 4, true),
        DateBuiltin::SetMilliseconds => ("Date.prototype.setMilliseconds", 6, 1, true),
        DateBuiltin::SetMinutes => ("Date.prototype.setMinutes", 4, 3, true),
        DateBuiltin::SetMonth => ("Date.prototype.setMonth", 1, 2, true),
        DateBuiltin::SetSeconds => ("Date.prototype.setSeconds", 5, 2, true),
        DateBuiltin::SetUtcDate => ("Date.prototype.setUTCDate", 2, 1, false),
        DateBuiltin::SetUtcFullYear => ("Date.prototype.setUTCFullYear", YEAR, 3, false),
        DateBuiltin::SetUtcHours => ("Date.prototype.setUTCHours", 3, 4, false),
        DateBuiltin::SetUtcMilliseconds => ("Date.prototype.setUTCMilliseconds", 6, 1, false),
        DateBuiltin::SetUtcMinutes => ("Date.prototype.setUTCMinutes", 4, 3, false),
        DateBuiltin::SetUtcMonth => ("Date.prototype.setUTCMonth", 1, 2, false),
        DateBuiltin::SetUtcSeconds => ("Date.prototype.setUTCSeconds", 5, 2, false),
        _ => return None,
    };
    Some(Setter {
        name,
        first,
        length,
        local,
    })
}

/// The seven fields of the time value `t` a setter keeps or writes, as Numbers: YearFromTime,
/// MonthFromTime, DateFromTime, HourFromTime, MinuteFromTime, SecondFromTime and msFromTime.
/// MakeDay over the first three is Day(`t`), MakeTime over the last four is
/// TimeWithinDay(`t`), both exactly (integers below 2^53). NaN for every field where
/// `date_fields` has none, which a time value and its local time never are.
fn fields_of_time(t: f64) -> [f64; 7] {
    let Some(fields) = date_fields(t) else {
        return [f64::NAN; 7];
    };
    [
        i32::try_from(fields.year).map_or(f64::NAN, f64::from),
        f64::from(fields.month),
        f64::from(fields.date),
        f64::from(fields.hour),
        f64::from(fields.minute),
        f64::from(fields.second),
        f64::from(fields.millisecond),
    ]
}

/// The first [`IDENTIFIER_PREFIX_UNITS`] code units of `identifier`, never more (a character
/// of two code units that crosses the bound is taken whole), or all of it when it is shorter:
/// what a conversion hands to `swell-num` to decide whether the identifier is an offset string
/// (decision 17, item 2).
fn offset_prefix(identifier: &str) -> &str {
    let mut units = 0;
    for (index, character) in identifier.char_indices() {
        if units >= IDENTIFIER_PREFIX_UNITS {
            return identifier.get(..index).unwrap_or(identifier);
        }
        units += character.len_utf16();
    }
    identifier
}

/// The text of a Date whose time value is NaN (`#sec-todatestring`, step 1; 21.4.4.35 step 4,
/// 21.4.4.42 step 4, 21.4.4.43 step 4).
const INVALID_DATE: &str = "Invalid Date";

/// The capacity of the one buffer a text is written into: the longest text, that of `toString`
/// for a year of six digits and a sign, is 18 + 1 + 8 + 1 + 8 = 36 bytes (`DateString`,
/// `TimeString`, `TimeZoneString`); `toUTCString` writes at most 32, `toISOString` 27.
const TEXT_BYTES: usize = 40;

/// What the TypeError of ThisTimeValue says after the name of the method.
const REQUIRES: &str = " requires that 'this' be a Date";

/// msPerDay (`#sec-time-related-constants`): an offset `swell-num` gives is below it in
/// magnitude.
const MS_PER_DAY: f64 = 86_400_000.0;

/// What a writer of `%Date.prototype%` writes.
#[derive(Clone, Copy)]
enum Form {
    /// ToDateString (`#sec-todatestring`): `DateString`, a space, `TimeString`, a space and
    /// `TimeZoneString` of the local time.
    Full,
    /// `DateString` of the local time.
    Date,
    /// `TimeString`, a space and `TimeZoneString` of the local time.
    Time,
    /// The form of `toUTCString`, of the time value itself.
    Utc,
    /// The Date Time String Format, of the time value itself.
    Iso,
}

/// The name and the form of the writer `id`; `None` for a builtin that is not one. The three
/// `toLocale*String` write what `toString`, `toDateString` and `toTimeString` write (decision
/// 6, "The text of toLocale*String").
fn writer_of(id: DateBuiltin) -> Option<(&'static str, Form)> {
    Some(match id {
        DateBuiltin::ToDateString => ("Date.prototype.toDateString", Form::Date),
        DateBuiltin::ToIsoString => ("Date.prototype.toISOString", Form::Iso),
        DateBuiltin::ToLocaleDateString => ("Date.prototype.toLocaleDateString", Form::Date),
        DateBuiltin::ToLocaleString => ("Date.prototype.toLocaleString", Form::Full),
        DateBuiltin::ToLocaleTimeString => ("Date.prototype.toLocaleTimeString", Form::Time),
        DateBuiltin::ToString => ("Date.prototype.toString", Form::Full),
        DateBuiltin::ToTimeString => ("Date.prototype.toTimeString", Form::Time),
        DateBuiltin::ToUtcString => ("Date.prototype.toUTCString", Form::Utc),
        _ => return None,
    })
}

/// Whether `units` are the ASCII code units of `name`, compared in place.
fn units_are(units: CodeUnits<'_>, name: &[u8]) -> bool {
    match units {
        CodeUnits::Latin1(bytes) => bytes == name,
        CodeUnits::Utf16(units) => {
            units.len() == name.len()
                && units
                    .iter()
                    .zip(name)
                    .all(|(&unit, &byte)| unit == u16::from(byte))
        }
    }
}

/// The tryFirst of the hint `units` of `[Symbol.toPrimitive]` (21.4.4.45, steps 3 and 4):
/// "string" and "default" give string, "number" gives number; any other String is not one of
/// the allowed values. At most 7 code units are compared, in place.
fn try_first_of(units: CodeUnits<'_>) -> Option<PreferredType> {
    if units_are(units, b"string") || units_are(units, b"default") {
        Some(PreferredType::String)
    } else if units_are(units, b"number") {
        Some(PreferredType::Number)
    } else {
        None
    }
}

impl<H: Host> Agent<H> {
    /// `%Date%` (21.4.2) and `%Date.prototype%` (21.4.4): the prototype is an ordinary object,
    /// not a Date ("does not have a [[DateValue]] internal slot"), whose [[Prototype]] is
    /// `%Object.prototype%`; the constructor's "length" is 7 (21.4.3), its "prototype"
    /// (21.4.3.3) { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false }, and
    /// the prototype's "constructor" is `%Date%` (21.4.4.1).
    pub(crate) fn create_date_constructor(
        &mut self,
        making: Making,
    ) -> Completion<(ObjectRef, ObjectRef)> {
        let prototype =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        let constructor = self.create_builtin_constructor(
            making,
            "Date",
            7,
            BuiltinId::Date(DateBuiltin::Constructor),
            prototype,
        )?;
        self.define_group_methods(making, constructor, &CONSTRUCTOR_METHODS, BuiltinId::Date)?;
        self.define_group_methods(making, prototype, &PROTOTYPE_METHODS, BuiltinId::Date)?;
        // 21.4.4.45: named "[Symbol.toPrimitive]", length 1, with { [[Writable]]: false,
        // [[Enumerable]]: false, [[Configurable]]: true }.
        let to_primitive = self.create_builtin(
            making,
            "[Symbol.toPrimitive]",
            1,
            BuiltinId::Date(DateBuiltin::ToPrimitive),
        )?;
        self.add_property(
            prototype,
            Key::Symbol(making.symbols.to_primitive),
            Property::data(Value::Object(to_primitive), false, false, true),
        )?;
        Ok((constructor, prototype))
    }

    /// The steps of the builtins of this module. It is called out of line from the dispatch
    /// (`run_builtin`, on the path of every re-entry through a builtin) and takes `this` and
    /// `new_target` by reference, so that its arm there builds no copy of either in the
    /// dispatch's frame and that frame stays at its size without `Date`.
    #[inline(never)]
    pub(crate) fn run_date_builtin(
        &mut self,
        id: DateBuiltin,
        this: &Value,
        args: &[Value],
        new_target: &Option<ObjectRef>,
    ) -> Completion<Value> {
        let (this, new_target) = (*this, *new_target);
        match id {
            DateBuiltin::Constructor => self.date_constructor(args, new_target),
            // `#sec-date.now`: "the time value designating the UTC date and time of the
            // occurrence of the call to it".
            DateBuiltin::Now => self.current_time_value().map(Value::Number),
            DateBuiltin::Utc => self.date_utc(args),
            DateBuiltin::GetDate
            | DateBuiltin::GetDay
            | DateBuiltin::GetFullYear
            | DateBuiltin::GetHours
            | DateBuiltin::GetMilliseconds
            | DateBuiltin::GetMinutes
            | DateBuiltin::GetMonth
            | DateBuiltin::GetSeconds
            | DateBuiltin::GetTimezoneOffset
            | DateBuiltin::GetUtcDate
            | DateBuiltin::GetUtcDay
            | DateBuiltin::GetUtcFullYear
            | DateBuiltin::GetUtcHours
            | DateBuiltin::GetUtcMilliseconds
            | DateBuiltin::GetUtcMinutes
            | DateBuiltin::GetUtcMonth
            | DateBuiltin::GetUtcSeconds => self.date_prototype_get(id, this),
            // `#sec-date.prototype.gettime`: 1. Let dateObj be the this value. 2. Perform ?
            // RequireInternalSlot(dateObj, [[DateValue]]). 3. Return dateObj.[[DateValue]].
            DateBuiltin::GetTime => self
                .this_time_value(this, "Date.prototype.getTime")
                .map(Value::Number),
            DateBuiltin::SetDate
            | DateBuiltin::SetFullYear
            | DateBuiltin::SetHours
            | DateBuiltin::SetMilliseconds
            | DateBuiltin::SetMinutes
            | DateBuiltin::SetMonth
            | DateBuiltin::SetSeconds
            | DateBuiltin::SetTime
            | DateBuiltin::SetUtcDate
            | DateBuiltin::SetUtcFullYear
            | DateBuiltin::SetUtcHours
            | DateBuiltin::SetUtcMilliseconds
            | DateBuiltin::SetUtcMinutes
            | DateBuiltin::SetUtcMonth
            | DateBuiltin::SetUtcSeconds => self.date_prototype_set(id, this, args),
            // `#sec-date.prototype.valueof`: the steps of `getTime`.
            DateBuiltin::ValueOf => self
                .this_time_value(this, "Date.prototype.valueOf")
                .map(Value::Number),
            DateBuiltin::Parse
            | DateBuiltin::ToDateString
            | DateBuiltin::ToIsoString
            | DateBuiltin::ToJson
            | DateBuiltin::ToLocaleDateString
            | DateBuiltin::ToLocaleString
            | DateBuiltin::ToLocaleTimeString
            | DateBuiltin::ToPrimitive
            | DateBuiltin::ToString
            | DateBuiltin::ToTimeString
            | DateBuiltin::ToUtcString => self.date_text_builtin(id, this, args),
        }
    }

    /// The builtins of the text half: `Date.parse`, the eight writers, `toJSON` and
    /// `[Symbol.toPrimitive]`, one arm of the dispatch, out of line so that their locals (a
    /// text, the fields of a date) are in no frame of the dispatch.
    #[inline(never)]
    fn date_text_builtin(
        &mut self,
        id: DateBuiltin,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        match id {
            DateBuiltin::Parse => self.date_parse(args),
            DateBuiltin::ToJson => self.date_prototype_to_json(this),
            DateBuiltin::ToPrimitive => self.date_prototype_to_primitive(this, args),
            _ => self.date_prototype_write(id, this),
        }
    }

    /// `Date.parse ( string )` (`#sec-date.parse`): ToString of the argument, before any code
    /// unit is read, then the time value of the text (`time_value_of_text`).
    fn date_parse(&mut self, args: &[Value]) -> Completion<Value> {
        let string = self.to_string(argument(args, 0))?;
        Ok(Value::Number(self.time_value_of_text(string)?))
    }

    /// The writers of `%Date.prototype%` (`#sec-date.prototype.todatestring`,
    /// `#sec-date.prototype.toisostring`, `#sec-date.prototype.tolocaledatestring` to
    /// `#sec-date.prototype.tolocaletimestring`, `#sec-date.prototype.tostring`,
    /// `#sec-date.prototype.totimestring`, `#sec-date.prototype.toutcstring`): 1. Let dateObj
    /// be the this value. 2. Perform ? RequireInternalSlot(dateObj, [[DateValue]]). 3. Let tv
    /// be dateObj.[[DateValue]]. Then the text of tv in the writer's form (`date_text`).
    fn date_prototype_write(&mut self, id: DateBuiltin, this: Value) -> Completion<Value> {
        let Some((name, form)) = writer_of(id) else {
            return Err(self.fault());
        };
        let tv = self.this_time_value(this, name)?;
        self.date_text(tv, form).map(Value::String)
    }

    /// The text of the time value `tv` in `form`, as one String of the heap, charged as every
    /// String is; the text is written into one buffer of [`TEXT_BYTES`] first.
    ///
    /// - NaN: `toISOString` throws a RangeError (21.4.4.36 step 4); every other form is
    ///   [`INVALID_DATE`], with no hook call.
    /// - `Iso` and `Utc` read the fields of tv itself, with no hook call. A time value always
    ///   has fields, and its year, -271,821 to 275,760, has at most the six digits of an
    ///   expanded year, so the RangeError of 21.4.4.36 step 6 is never thrown; a [[DateValue]]
    ///   without fields is an engine fault.
    /// - `Full`, `Date` and `Time` read the fields of LocalTime(tv) and the offset that
    ///   conversion used (`local_fields_with_offset`): one conversion, 2 hook calls, 1 with an
    ///   offset-string identifier; the `GMT±hhmm` is the offset of the local time it prints,
    ///   and the draft's second read of the offset (`TimeZoneString`, step 2) is not made
    ///   (`docs/design/host-interface.md`, decision 17, item 1).
    #[inline(never)]
    fn date_text(&mut self, tv: f64, form: Form) -> Completion<StringRef> {
        if tv.is_nan() {
            if let Form::Iso = form {
                return Err(self.throw_error(
                    ErrorKind::RangeError,
                    "Date.prototype.toISOString of a Date whose time value is NaN",
                ));
            }
            return self.new_string_str(INVALID_DATE);
        }
        let mut text = String::with_capacity(TEXT_BYTES);
        match form {
            Form::Iso | Form::Utc => {
                let Some(fields) = date_fields(tv) else {
                    return Err(self.fault());
                };
                text.push_str(&match form {
                    Form::Iso => iso_string(&fields),
                    _ => utc_string(&fields),
                });
            }
            Form::Full | Form::Date | Form::Time => {
                let (fields, offset_ms) = self.local_fields_with_offset(tv)?;
                if let Form::Full | Form::Date = form {
                    text.push_str(&date_string(&fields));
                }
                if let Form::Full = form {
                    text.push(' ');
                }
                if let Form::Full | Form::Time = form {
                    text.push_str(&time_string(&fields));
                    text.push(' ');
                    text.push_str(&time_zone_string(offset_ms));
                }
            }
        }
        self.new_string_str(&text)
    }

    /// The fields of LocalTime(`tv`) (`#sec-localtime`) and the offset in milliseconds that
    /// conversion added, for a `tv` that is not NaN: one `local_time_of`, so one read of the
    /// identifier and at most one of the offset hook. The offset is the local time minus `tv`,
    /// both integral Numbers below 2^53, so the difference is exact, and `swell-num` clamps it
    /// below msPerDay in magnitude. Neither a time value nor its local time is without
    /// fields, and an offset outside that bound is not made: either is an engine fault.
    fn local_fields_with_offset(&mut self, tv: f64) -> Completion<(DateFields, i64)> {
        let local = self.local_time_of(tv)?;
        let offset = local - tv;
        // Range test of the cast: |offset| < msPerDay and integral, so `as i64` is exact.
        if !(offset.abs() < MS_PER_DAY && offset.trunc() == offset) {
            return Err(self.fault());
        }
        let offset_ms = offset as i64;
        match date_fields(local) {
            Some(fields) => Ok((fields, offset_ms)),
            None => Err(self.fault()),
        }
    }

    /// `Date.prototype.toJSON ( key )` (`#sec-date.prototype.tojson`). It is generic; the key
    /// is not read. O is new for a primitive `this`, so it is kept, the kept stack's room for
    /// it charged before, while ToPrimitive and Invoke run code.
    #[inline(never)]
    fn date_prototype_to_json(&mut self, this: Value) -> Completion<Value> {
        // 1. Let O be ? ToObject(this value).
        let object = self.to_object(this)?;
        let mark = self.kept.len();
        let mut room = self.kept_room();
        let answer = match self.reserve_kept(&mut room, 1) {
            Ok(()) => {
                self.keep(Value::Object(object));
                self.date_json_steps(object)
            }
            Err(abrupt) => Err(abrupt),
        };
        self.end_kept_room(mark, &room);
        answer
    }

    /// Steps 2 to 4 of `toJSON` for the kept object `object`.
    fn date_json_steps(&mut self, object: ObjectRef) -> Completion<Value> {
        // 2. Let tv be ? ToPrimitive(O, number).
        let tv = self.to_primitive(Value::Object(object), Some(PreferredType::Number))?;
        // 3. If tv is a Number and tv is not finite, return null.
        if let Value::Number(number) = tv
            && !number.is_finite()
        {
            return Ok(Value::Null);
        }
        // 4. Return ? Invoke(O, "toISOString").
        let key = self.name_key("toISOString")?;
        self.invoke(Value::Object(object), key, &[])
    }

    /// `Date.prototype [ %Symbol.toPrimitive% ] ( hint )`
    /// (`#sec-date.prototype-%symbol.toprimitive%`): 1. Let obj be the this value. 2. If obj is
    /// not an Object, throw a TypeError exception. 3. Set hint to ? ValidateStringOption(hint,
    /// reject, « "string", "number", "default" », required) (`#sec-validatestringoption`):
    /// undefined and any value that is not a String throw a TypeError with no conversion, and
    /// so does a String that is not one of the three. 4. If hint is "number", let tryFirst be
    /// number; otherwise string. 5. Return ? OrdinaryToPrimitive(obj, tryFirst).
    #[inline(never)]
    fn date_prototype_to_primitive(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        let Value::Object(object) = this else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Date.prototype [ @@toPrimitive ] requires that 'this' be an Object",
            ));
        };
        let try_first = match argument(args, 0) {
            Value::String(hint) => match self.heap.strings.get(hint.handle) {
                Ok(record) => try_first_of(record.units()),
                Err(_) => return Err(self.fault()),
            },
            _ => None,
        };
        let Some(try_first) = try_first else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the hint of Date.prototype [ @@toPrimitive ] is not \"string\", \"number\" or \"default\"",
            ));
        };
        self.ordinary_to_primitive(object, try_first)
    }

    /// `Date ( ...values )` (`#sec-date-constructor`).
    fn date_constructor(
        &mut self,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. If NewTarget is undefined, then
        let Some(new_target) = new_target else {
            // a. Let now be the time value (UTC) identifying the current time.
            let now = self.current_time_value()?;
            // b. Return ToDateString(now). The arguments are not read.
            return self.date_text(now, Form::Full).map(Value::String);
        };
        // 2. Let numberOfArgs be the number of elements in values.
        let dv = match args {
            // 3. If numberOfArgs = 0, then a. Let dv be the time value (UTC) identifying the
            // current time.
            [] => self.current_time_value()?,
            // 4. Else if numberOfArgs = 1, then
            [value] => self.date_value_of_one(*value)?,
            // 5. Else, the components.
            _ => self.date_value_of_components(args)?,
        };
        // 6. Let O be ? OrdinaryCreateFromConstructor(NewTarget, "%Date.prototype%", «
        // [[DateValue]] »), after the coercions, so a Getter of "prototype" runs after them.
        // 7. Set O.[[DateValue]] to dv. 8. Return O. dv is a Number, which needs no root.
        let object = self.ordinary_create_from_constructor(
            new_target,
            Intrinsic::DatePrototype,
            OrdinaryData::Date(dv),
        )?;
        Ok(Value::Object(object))
    }

    /// Step 4 of `Date ( ...values )` (`#sec-date-constructor`): the time value of the one
    /// argument `value`.
    fn date_value_of_one(&mut self, value: Value) -> Completion<f64> {
        // a. Let value be values[0].
        // b. If value is an Object and value has a [[DateValue]] internal slot, then i. Let tv
        // be value.[[DateValue]]. No method of value is read.
        let tv = match self.date_value(value)? {
            Some(time) => time,
            // c. Else,
            None => match self.to_primitive(value, None)? {
                // i. Let v be ? ToPrimitive(value). ii. If v is a String, then 1. Assert: The
                // next step never returns an abrupt completion because v is a String. 2. Let
                // tv be the result of parsing v as a date, in exactly the same manner as for
                // the parse method (21.4.3.2).
                Value::String(text) => self.time_value_of_text(text)?,
                // iii. Else, 1. Let tv be ? ToNumber(v).
                primitive => self.to_number(primitive)?,
            },
        };
        // d. Let dv be TimeClip(tv).
        Ok(time_clip(tv))
    }

    /// [[DateValue]] of `value` when it is an Object that has that slot.
    fn date_value(&mut self, value: Value) -> Completion<Option<f64>> {
        if let Value::Object(object) = value
            && let ObjectKind::Ordinary(ordinary) = &self.engine_object(object)?.kind
            && let OrdinaryData::Date(time) = ordinary.data
        {
            return Ok(Some(time));
        }
        Ok(None)
    }

    /// RequireInternalSlot(`value`, [[DateValue]]) (`#sec-requireinternalslot`), then its
    /// [[DateValue]]: ThisTimeValue (`#sec-thistimevalue`) of the method `method`. A value that
    /// is not a Date throws a TypeError.
    pub(crate) fn this_time_value(&mut self, value: Value, method: &str) -> Completion<f64> {
        if let Some(time) = self.date_value(value)? {
            return Ok(time);
        }
        let mut message = String::with_capacity(method.len() + REQUIRES.len());
        message.push_str(method);
        message.push_str(REQUIRES);
        Err(self.throw_error(ErrorKind::TypeError, &message))
    }

    /// `Date.prototype.setTime ( time )` (`#sec-date.prototype.settime`).
    fn date_prototype_set_time(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let dateObj be the this value. 2. Perform ? RequireInternalSlot(dateObj,
        // [[DateValue]]): before `time` is read.
        self.this_time_value(this, "Date.prototype.setTime")?;
        let Value::Object(date) = this else {
            return Err(self.fault());
        };
        // 3. Let t be ? ToNumber(time). dateObj is held by this builtin's activation while
        // ToNumber runs code.
        let t = self.to_number(argument(args, 0))?;
        // 4. Let v be TimeClip(t).
        let v = time_clip(t);
        // 5. Set dateObj.[[DateValue]] to v.
        let stored = match &mut self.engine_object_mut(date)?.kind {
            ObjectKind::Ordinary(ordinary) => match &mut ordinary.data {
                OrdinaryData::Date(time) => {
                    *time = v;
                    true
                }
                _ => false,
            },
            _ => false,
        };
        // The object was a Date at step 2, and an object's slots do not change kind.
        if !stored {
            return Err(self.fault());
        }
        // 6. Return v.
        Ok(Value::Number(v))
    }

    /// The time value (UTC) identifying the current time: one step is charged (HOST-211's rule
    /// for every hook of the clock and the time zone, `docs/design/host-interface.md`,
    /// decision 17, item 5), then `host_system_utc_epoch_nanoseconds` is called once with the
    /// current realm's global object (HOST-125). Its answer is clamped to ±8.64 × 10^21
    /// (HOST-126) and divided by 10^6 rounding toward negative infinity (`div_euclid` by a
    /// positive divisor); the quotient is at most 8.64 × 10^15 in magnitude, below 2^53, so
    /// `as f64` reads it exactly, and it is a time value with no TimeClip. `new Date()`,
    /// `Date()` and `Date.now()` call it.
    pub(crate) fn current_time_value(&mut self) -> Completion<f64> {
        self.charge_steps(1)?;
        let realm = self.current_realm()?;
        let Some(global) = self.realm_record(realm)?.global else {
            return Err(self.fault());
        };
        let start = self.scope.len();
        let answer = H::host_system_utc_epoch_nanoseconds(&mut HookCx { agent: self }, global);
        self.scope.truncate(start);
        // A terminated run ignores what the hook answered (decision 10).
        self.running()?;
        let clamped = answer.clamp(-MAX_EPOCH_NANOSECONDS, MAX_EPOCH_NANOSECONDS);
        let milliseconds = clamped.div_euclid(NANOSECONDS_PER_MILLISECOND);
        Ok(milliseconds as f64)
    }

    /// The time value of the String `text` as `Date.parse` reads it (`#sec-date.parse`; step
    /// 4.c.ii of `#sec-date-constructor`, "parsing v as a date, in exactly the same manner as
    /// for the parse method"), by `swell_num::date_text::parse_date_text` over the code units
    /// of the String's own storage, read in place: no copy is made. A step per code unit past
    /// 32 is charged before the first is read (`charge_code_units`; decision 6: the name of the
    /// form of `toString` has no length bound, so no length refuses a String unread). A text
    /// that fixes its instant gives TimeClip of its time value with no hook call; a date-time
    /// form without an offset gives TimeClip(UTC(t)) over the zone conversion (at most 59 hook
    /// calls, each charged a step first); anything else NaN, with no fallback.
    #[inline(never)]
    fn time_value_of_text(&mut self, text: StringRef) -> Completion<f64> {
        let length = self.string_length(text)?;
        self.charge_code_units(length)?;
        let read = match self.heap.strings.get(text.handle) {
            Ok(record) => match record.units() {
                CodeUnits::Latin1(bytes) => parse_date_text(bytes),
                CodeUnits::Utf16(units) => parse_date_text(units),
            },
            Err(_) => return Err(self.fault()),
        };
        Ok(match read {
            DateText::Utc(t) => time_clip(t),
            DateText::Local(t) => time_clip(self.utc_of_local(t)?),
            DateText::Invalid => f64::NAN,
        })
    }

    /// Step 5 of `Date ( ...values )` (`#sec-date-constructor`), two or more arguments: the
    /// components read as a local time (`date_args_to_time`), then `dv` is
    /// TimeClip(UTC(finalDate)). A local time with no instant gives NaN and throws nothing.
    #[inline(never)]
    fn date_value_of_components(&mut self, args: &[Value]) -> Completion<f64> {
        let final_date = self.date_args_to_time(args)?;
        Ok(time_clip(self.utc_of_local(final_date)?))
    }

    /// The steps the constructor with components (`#sec-date-constructor`, step 5.b to 5.j)
    /// and `Date.UTC` (`#sec-date.utc`, steps 1 to 8) share: ToNumber of the year, then of
    /// month, day, hour, minute, second and millisecond where present (else +0, 1 for the
    /// day), each once and in this order, even after a NaN; an eighth argument is not read.
    /// Then MakeFullYear of the year and MakeDate(MakeDay(year, month, day), MakeTime(hour,
    /// minute, second, millisecond)). Each Number is kept as it is read: no list of values.
    pub(crate) fn date_args_to_time(&mut self, args: &[Value]) -> Completion<f64> {
        // Let yearNumber be ? ToNumber(year).
        let year = self.to_number(argument(args, 0))?;
        // If <name> is present, let <name>Number be ? ToNumber(<name>); else +0𝔽, and 1𝔽 for
        // the day.
        let mut rest = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        for (index, number) in rest.iter_mut().enumerate() {
            if let Some(value) = args.get(index + 1) {
                *number = self.to_number(*value)?;
            }
        }
        let [month, day, hour, minute, second, millisecond] = rest;
        // Set yearNumber to MakeFullYear(yearNumber).
        let year = make_full_year(year);
        // MakeDate(MakeDay(yearNumber, monthNumber, dayNumber), MakeTime(hourNumber,
        // minuteNumber, secondNumber, millisecondNumber)).
        Ok(make_date(
            make_day(year, month, day),
            make_time(hour, minute, second, millisecond),
        ))
    }

    /// `Date.UTC ( year [ , month [ , day [ , hour [ , minute [ , second [ , millisecond ] ] ]
    /// ] ] ] )` (`#sec-date.utc`): 9. Return TimeClip(MakeDate(...)). No hook is called.
    #[inline(never)]
    fn date_utc(&mut self, args: &[Value]) -> Completion<Value> {
        let time = self.date_args_to_time(args)?;
        Ok(Value::Number(time_clip(time)))
    }

    /// The getters of `%Date.prototype%` but `getTime` (`#sec-date.prototype.getdate` to
    /// `#sec-date.prototype.getutcseconds`): 1. Let dateObj be the this value. 2. Perform ?
    /// RequireInternalSlot(dateObj, [[DateValue]]). 3. Let tv be dateObj.[[DateValue]]. 4. If
    /// tv is NaN, return NaN, with no hook call. 5. Return the field of LocalTime(tv) or of tv,
    /// or, for `getTimezoneOffset`, (tv - LocalTime(tv)) / msPerMinute: two integral Numbers
    /// below 2^53 divided as IEEE 754 divides, exact when the quotient is an integer.
    #[inline(never)]
    fn date_prototype_get(&mut self, id: DateBuiltin, this: Value) -> Completion<Value> {
        let Some((name, getter)) = getter_of(id) else {
            return Err(self.fault());
        };
        let tv = self.this_time_value(this, name)?;
        if tv.is_nan() {
            return Ok(Value::Number(f64::NAN));
        }
        let answer = match getter {
            Getter::Utc(field) => field_of(tv, field),
            Getter::Local(field) => field_of(self.local_time_of(tv)?, field),
            Getter::TimezoneOffset => (tv - self.local_time_of(tv)?) / MS_PER_MINUTE,
        };
        Ok(Value::Number(answer))
    }

    /// The 14 setters of `%Date.prototype%` but `setTime` (`#sec-date.prototype.setdate` to
    /// `#sec-date.prototype.setutcseconds`), over the row of `id` ([`setter_of`]); `setTime`
    /// goes to `date_prototype_set_time`, the clock story's:
    ///
    /// - Let dateObj be the this value; perform ? RequireInternalSlot(dateObj, [[DateValue]]);
    ///   let tv be dateObj.[[DateValue]], read before any argument.
    /// - ? ToNumber of the first argument (undefined when absent), then ? ToNumber of each
    ///   further argument present, up to the setter's "length", each once and in order, also
    ///   when tv is NaN; an argument past it is not read. Numbers are kept, not values.
    /// - `setFullYear` (21.4.4.21 step 5) and `setUTCFullYear` (21.4.4.29 step 4): if tv is
    ///   NaN, set tv to +0𝔽, with no hook call; else `setFullYear` sets tv to LocalTime(tv),
    ///   after ToNumber(year) and before ToNumber(month), as the clause orders them (step 4
    ///   of `setUTCFullYear` reads nothing, so its place before ToNumber(year) is not
    ///   observable). Every other setter: if tv is NaN, return NaN, with no store and no hook
    ///   call; else a local one sets tv to LocalTime(tv) (2 hook calls).
    /// - The fields of tv the arguments do not give are kept; the new date is
    ///   MakeDate(MakeDay(year, month, date), MakeTime(hour, minute, second, ms)), which is
    ///   MakeDate(Day(tv), MakeTime(...)) for the time setters and MakeDate(MakeDay(...),
    ///   TimeWithinDay(tv)) for the date setters.
    /// - A local setter takes UTC(newDate) (at most 59 hook calls; NaN, and nothing thrown,
    ///   when the host has no instant, HOST-210), then TimeClip; the result is stored into
    ///   [[DateValue]] last and returned, so a termination inside a hook leaves the Date
    ///   unchanged.
    ///
    /// The Date is held by this builtin's activation while ToNumber runs code and while the
    /// hooks are called.
    #[inline(never)]
    fn date_prototype_set(
        &mut self,
        id: DateBuiltin,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        // `setTime` shares this arm of the dispatch, so that the setters add no call to the
        // dispatch's frame; its steps are its own.
        if id == DateBuiltin::SetTime {
            return self.date_prototype_set_time(this, args);
        }
        let Some(setter) = setter_of(id) else {
            return Err(self.fault());
        };
        let tv = self.this_time_value(this, setter.name)?;
        let Value::Object(date) = this else {
            return Err(self.fault());
        };
        let present = args.len().clamp(1, setter.length);
        let first = self.to_number(argument(args, 0))?;
        let full_year = setter.first == YEAR;
        let mut t = tv;
        if full_year {
            t = if tv.is_nan() {
                0.0
            } else if setter.local {
                self.local_time_of(tv)?
            } else {
                tv
            };
        }
        let mut rest = [f64::NAN; 3];
        for (index, number) in rest.iter_mut().enumerate().take(present.saturating_sub(1)) {
            *number = self.to_number(argument(args, index + 1))?;
        }
        if !full_year {
            if tv.is_nan() {
                return Ok(Value::Number(f64::NAN));
            }
            if setter.local {
                t = self.local_time_of(tv)?;
            }
        }
        let mut fields = fields_of_time(t);
        let numbers = std::iter::once(first).chain(rest);
        for (field, number) in fields
            .iter_mut()
            .skip(setter.first)
            .zip(numbers.take(present))
        {
            *field = number;
        }
        let [year, month, day, hour, minute, second, millisecond] = fields;
        let new_date = make_date(
            make_day(year, month, day),
            make_time(hour, minute, second, millisecond),
        );
        let u = if setter.local {
            self.utc_of_local(new_date)?
        } else {
            new_date
        };
        let v = time_clip(u);
        let stored = match &mut self.engine_object_mut(date)?.kind {
            ObjectKind::Ordinary(ordinary) => match &mut ordinary.data {
                OrdinaryData::Date(time) => {
                    *time = v;
                    true
                }
                _ => false,
            },
            _ => false,
        };
        // The object was a Date when tv was read, and an object's slots do not change kind.
        if !stored {
            return Err(self.fault());
        }
        Ok(Value::Number(v))
    }

    /// `LocalTime ( t )` (`#sec-localtime`) over the hooks: a `t` that is not finite answers
    /// itself with no call; else the identifier is read once (one step, then
    /// `system_time_zone_identifier`), its first 20 code units go to
    /// `swell_num::date::local_time`, and an identifier that is not an offset string makes one
    /// call of `get_named_time_zone_offset_nanoseconds` (one step before it) with the
    /// identifier as it was answered. The offset is clamped and truncated by `swell-num`
    /// (HOST-209). A termination of a step or a hook goes up unchanged.
    pub(crate) fn local_time_of(&mut self, t: f64) -> Completion<f64> {
        if !t.is_finite() {
            return Ok(t);
        }
        let identifier = self.time_zone_identifier()?;
        local_time(
            t,
            offset_prefix(&identifier),
            &mut |_: &str, epoch_ns: i128| self.time_zone_offset_nanoseconds(&identifier, epoch_ns),
        )
    }

    /// `UTC ( t )` (`#sec-utc-t`) over the hooks: a `t` that is not finite answers NaN with no
    /// call; else the identifier is read once, as `local_time_of` reads it, and
    /// `swell_num::date::utc_from_local` asks the epoch hook (`possible_instants`) and the
    /// offset hook through two closures, each call charged one step before it is made: at most
    /// 59 calls (the identifier, 57 of the epoch hook, the offset), 3 on the common path, 1
    /// for an offset string, 31 for a host with no instant for any local time (HOST-210). The
    /// two closures share the agent through a `RefCell`; `swell-num` calls them one after the
    /// other, never inside each other, and a borrow that is refused anyway is an engine fault.
    pub(crate) fn utc_of_local(&mut self, t: f64) -> Completion<f64> {
        if !t.is_finite() {
            return Ok(f64::NAN);
        }
        let identifier = self.time_zone_identifier()?;
        let answer = {
            let agent = RefCell::new(&mut *self);
            utc_from_local(
                t,
                offset_prefix(&identifier),
                &mut |_: &str, local: &swell_num::date::IsoDateTime| match agent.try_borrow_mut() {
                    Ok(mut agent) => agent.possible_instants(&identifier, local).map_err(Some),
                    Err(_) => Err(None),
                },
                &mut |_: &str, epoch_ns: i128| match agent.try_borrow_mut() {
                    Ok(mut agent) => agent
                        .time_zone_offset_nanoseconds(&identifier, epoch_ns)
                        .map_err(Some),
                    Err(_) => Err(None),
                },
            )
        };
        match answer {
            Ok(utc) => Ok(utc),
            Err(Some(abrupt)) => Err(abrupt),
            Err(None) => Err(self.fault()),
        }
    }

    /// SystemTimeZoneIdentifier (`#sec-systemtimezoneidentifier`): one step, then
    /// `system_time_zone_identifier` once. The String is the host's; it is charged nothing
    /// more and is dropped when the conversion that read it ends (decision 17, item 7).
    fn time_zone_identifier(&mut self) -> Completion<String> {
        self.charge_steps(1)?;
        let start = self.scope.len();
        let identifier = H::system_time_zone_identifier(&mut HookCx { agent: self });
        self.scope.truncate(start);
        // A terminated run ignores what the hook answered (decision 10).
        self.running()?;
        Ok(identifier)
    }

    /// GetNamedTimeZoneOffsetNanoseconds (`#sec-getnamedtimezoneoffsetnanoseconds`) of the
    /// zone `identifier` at `epoch_ns`: one step, then the hook once. The answer is `swell-num`'s
    /// to clamp (`offset_milliseconds`).
    fn time_zone_offset_nanoseconds(
        &mut self,
        identifier: &str,
        epoch_ns: i128,
    ) -> Completion<i128> {
        self.charge_steps(1)?;
        let start = self.scope.len();
        let answer = H::get_named_time_zone_offset_nanoseconds(
            &mut HookCx { agent: self },
            identifier,
            epoch_ns,
        );
        self.scope.truncate(start);
        self.running()?;
        Ok(answer)
    }

    /// GetNamedTimeZoneEpochNanoseconds (`#sec-getnamedtimezoneepochnanoseconds`) of the zone
    /// `identifier` at the local time `local`, reduced to what `utc_from_local` reads: `None`
    /// for an empty list, else its first and its last instant. The record of the hook takes
    /// the year as an `i32`; a year that does not fit answers no instant with no call (no
    /// year of a time value or of a local time a day beyond it is over 275,761 in magnitude).
    /// One step, then the hook once; the list is charged 16 bytes per instant to the memory
    /// budget before an instant is read (`Terminated(MemoryBudget)` when refused) and credited
    /// when it is dropped, before this returns (decision 17, item 7).
    fn possible_instants(
        &mut self,
        identifier: &str,
        local: &swell_num::date::IsoDateTime,
    ) -> Completion<Option<(i128, i128)>> {
        let Ok(year) = i32::try_from(local.year) else {
            return Ok(None);
        };
        let record = IsoDateTime {
            year,
            month: local.month,
            day: local.day,
            hour: local.hour,
            minute: local.minute,
            second: local.second,
            millisecond: local.millisecond,
            microsecond: local.microsecond,
            nanosecond: local.nanosecond,
        };
        self.charge_steps(1)?;
        let start = self.scope.len();
        let list = H::get_named_time_zone_epoch_nanoseconds(
            &mut HookCx { agent: self },
            identifier,
            record,
        );
        self.scope.truncate(start);
        self.running()?;
        let bytes = u64::try_from(list.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(INSTANT_BYTES);
        if self.heap.accounting.charge(bytes).is_err() {
            drop(list);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let instants = list.first().copied().zip(list.last().copied());
        drop(list);
        self.heap.accounting.credit(bytes);
        Ok(instants)
    }
}

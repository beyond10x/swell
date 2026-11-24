//! Date objects (ECMA-262 21.4, `#sec-date-objects`), the clock half, written by
//! `story:date-object-clock-and-time-value`: `%Date%` (`#sec-date-constructor`) with
//! `Date.now` (`#sec-date.now`), `%Date.prototype%`
//! (`#sec-properties-of-the-date-prototype-object`) with `getTime`
//! (`#sec-date.prototype.gettime`), `setTime` (`#sec-date.prototype.settime`) and `valueOf`
//! (`#sec-date.prototype.valueof`), the time value of a Date ([[DateValue]],
//! `#sec-properties-of-date-instances`, held as `OrdinaryData::Date`), and the read of the host
//! clock (`current_time_value`, `docs/design/host-interface.md`, HOST-125 and HOST-126).
//!
//! Every other own property of `%Date%` and `%Date.prototype%` that a later story builds is
//! defined here with its name and length and an interim body, `date_not_built`, which throws a
//! TypeError that names the function: `Date.UTC` (the zone story), `Date.parse` (the text
//! story) and the 14 setters other than `setTime` (the setters story). Each interim is one line
//! of the dispatch, the arm of its variant calling the interim body with the function's name as
//! a string literal, which the story that builds the function replaces. Three steps are seams
//! until their story: the String argument of the constructor (`parse_date_string`, NaN), the
//! constructor with two or more arguments (`date_from_components`, NaN with no coercion) and
//! ToDateString of the call without `new` (`date_to_string`, "Invalid Date").
//!
//! The builtins re-enter script only through ToPrimitive and ToNumber, as `Number` does. No
//! loop runs here; the one step charged beyond the call's own is the one before the clock hook.

use swell_num::time_value::time_clip;

use crate::builtins::Method;
use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, HookCx, Host};
use crate::object::ObjectKind;
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, StringRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DateBuiltin {
    /// `Date ( ...values )` (21.4.2.1).
    Constructor,
    /// `Date.now ( )` (21.4.3.1).
    Now,
    /// `Date.parse ( string )` (21.4.3.2): interim.
    Parse,
    /// `Date.UTC ( year [ , month [ , ... ] ] )` (21.4.3.4): interim.
    Utc,
    /// `Date.prototype.getTime ( )` (21.4.4.10).
    GetTime,
    /// `Date.prototype.setDate ( date )` (21.4.4.20): interim.
    SetDate,
    /// `Date.prototype.setFullYear ( year [ , month [ , date ] ] )` (21.4.4.21): interim.
    SetFullYear,
    /// `Date.prototype.setHours ( hour [ , min [ , sec [ , ms ] ] ] )` (21.4.4.22): interim.
    SetHours,
    /// `Date.prototype.setMilliseconds ( ms )` (21.4.4.23): interim.
    SetMilliseconds,
    /// `Date.prototype.setMinutes ( min [ , sec [ , ms ] ] )` (21.4.4.24): interim.
    SetMinutes,
    /// `Date.prototype.setMonth ( month [ , date ] )` (21.4.4.25): interim.
    SetMonth,
    /// `Date.prototype.setSeconds ( sec [ , ms ] )` (21.4.4.26): interim.
    SetSeconds,
    /// `Date.prototype.setTime ( time )` (21.4.4.27).
    SetTime,
    /// `Date.prototype.setUTCDate ( date )` (21.4.4.28): interim.
    SetUtcDate,
    /// `Date.prototype.setUTCFullYear ( year [ , month [ , date ] ] )` (21.4.4.29): interim.
    SetUtcFullYear,
    /// `Date.prototype.setUTCHours ( hour [ , min [ , sec [ , ms ] ] ] )` (21.4.4.30): interim.
    SetUtcHours,
    /// `Date.prototype.setUTCMilliseconds ( ms )` (21.4.4.31): interim.
    SetUtcMilliseconds,
    /// `Date.prototype.setUTCMinutes ( min [ , sec [ , ms ] ] )` (21.4.4.32): interim.
    SetUtcMinutes,
    /// `Date.prototype.setUTCMonth ( month [ , date ] )` (21.4.4.33): interim.
    SetUtcMonth,
    /// `Date.prototype.setUTCSeconds ( sec [ , ms ] )` (21.4.4.34): interim.
    SetUtcSeconds,
    /// `Date.prototype.valueOf ( )` (21.4.4.44).
    ValueOf,
}

/// The functions of `%Date%` (21.4.3) but `prototype`, in the order of the clauses; `length`
/// as each clause's parameters give it (21.4.3.4: "The "length" property of this function is
/// 7𝔽").
const CONSTRUCTOR_METHODS: [Method<DateBuiltin>; 3] = [
    ("now", 0, DateBuiltin::Now),
    ("parse", 1, DateBuiltin::Parse),
    ("UTC", 7, DateBuiltin::Utc),
];

/// The methods of `%Date.prototype%` (21.4.4) this story defines, in the order of the clauses;
/// `length` as each clause's required parameters give it, and as its "length" sentence says
/// where it has one.
const PROTOTYPE_METHODS: [Method<DateBuiltin>; 17] = [
    ("getTime", 0, DateBuiltin::GetTime),
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
    ("valueOf", 0, DateBuiltin::ValueOf),
];

/// The bound of the clock's answer: ±8.64 × 10^21 nanoseconds, the time values' range
/// (`#sec-time-values-and-time-range`) in nanoseconds (HOST-126).
const MAX_EPOCH_NANOSECONDS: i128 = 8_640_000_000_000_000_000_000;

/// Nanoseconds in a millisecond.
const NANOSECONDS_PER_MILLISECOND: i128 = 1_000_000;

/// What an arm of the dispatch answers: the completion of the builtin's steps, or, for an
/// interim body, the name of the function that is not built yet.
type Ran = Result<Completion<Value>, &'static str>;

/// The interim body of a function of `Date` that a later story builds: the dispatch throws a
/// TypeError that names `name` and says it is not built yet.
fn date_not_built(name: &'static str) -> Ran {
    Err(name)
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
        let ran: Ran = match id {
            DateBuiltin::Constructor => Ok(self.date_constructor(args, new_target)),
            // `#sec-date.now`: "the time value designating the UTC date and time of the
            // occurrence of the call to it".
            DateBuiltin::Now => Ok(self.current_time_value().map(Value::Number)),
            DateBuiltin::Parse => date_not_built("parse"),
            DateBuiltin::Utc => date_not_built("UTC"),
            // `#sec-date.prototype.gettime`: 1. Let dateObj be the this value. 2. Perform ?
            // RequireInternalSlot(dateObj, [[DateValue]]). 3. Return dateObj.[[DateValue]].
            DateBuiltin::GetTime => Ok(self
                .this_time_value(this, "Date.prototype.getTime")
                .map(Value::Number)),
            DateBuiltin::SetDate => date_not_built("setDate"),
            DateBuiltin::SetFullYear => date_not_built("setFullYear"),
            DateBuiltin::SetHours => date_not_built("setHours"),
            DateBuiltin::SetMilliseconds => date_not_built("setMilliseconds"),
            DateBuiltin::SetMinutes => date_not_built("setMinutes"),
            DateBuiltin::SetMonth => date_not_built("setMonth"),
            DateBuiltin::SetSeconds => date_not_built("setSeconds"),
            DateBuiltin::SetTime => Ok(self.date_prototype_set_time(this, args)),
            DateBuiltin::SetUtcDate => date_not_built("setUTCDate"),
            DateBuiltin::SetUtcFullYear => date_not_built("setUTCFullYear"),
            DateBuiltin::SetUtcHours => date_not_built("setUTCHours"),
            DateBuiltin::SetUtcMilliseconds => date_not_built("setUTCMilliseconds"),
            DateBuiltin::SetUtcMinutes => date_not_built("setUTCMinutes"),
            DateBuiltin::SetUtcMonth => date_not_built("setUTCMonth"),
            DateBuiltin::SetUtcSeconds => date_not_built("setUTCSeconds"),
            // `#sec-date.prototype.valueof`: the steps of `getTime`.
            DateBuiltin::ValueOf => Ok(self
                .this_time_value(this, "Date.prototype.valueOf")
                .map(Value::Number)),
        };
        ran.unwrap_or_else(|name| {
            Err(self.throw_error(
                ErrorKind::TypeError,
                &format!("the Date function {name} is not built yet"),
            ))
        })
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
            // b. Return ToDateString(now).
            let text = self.date_to_string(now)?;
            return Ok(Value::String(self.new_string_str(&text)?));
        };
        // 2. Let numberOfArgs be the number of elements in values.
        let dv = match args {
            // 3. If numberOfArgs = 0, then a. Let dv be the time value (UTC) identifying the
            // current time.
            [] => self.current_time_value()?,
            // 4. Else if numberOfArgs = 1, then
            [value] => self.date_value_of_one(*value)?,
            // 5. Else, the components: the seam of the zone story.
            _ => self.date_from_components(args)?,
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
                Value::String(text) => self.parse_date_string(text)?,
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
        Err(self.throw_error(
            ErrorKind::TypeError,
            &format!("{method} requires that 'this' be a Date"),
        ))
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

    /// The seam of the String argument of the constructor (step 4.c.ii of
    /// `#sec-date-constructor`, "parsing v as a date, in exactly the same manner as for the
    /// parse method"): NaN, until `story:date-text-formats` replaces it.
    fn parse_date_string(&mut self, _text: StringRef) -> Completion<f64> {
        Ok(f64::NAN)
    }

    /// The seam of the constructor with two or more arguments (step 5 of
    /// `#sec-date-constructor`): NaN, with no coercion of the arguments, until
    /// `story:date-object-constructor-now-utc-and-getters` replaces it.
    fn date_from_components(&mut self, _args: &[Value]) -> Completion<f64> {
        Ok(f64::NAN)
    }

    /// The seam of ToDateString (`#sec-todatestring`) for the call without `new`: "Invalid
    /// Date", the text of a time value of NaN, until `story:date-text-formats` replaces it.
    fn date_to_string(&mut self, _time: f64) -> Completion<String> {
        Ok("Invalid Date".to_string())
    }
}

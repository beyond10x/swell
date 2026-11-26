//! The matching of RegExp objects (ECMA-262 22.2.7,
//! `#sec-abstract-operations-for-regexp-matching`), written by
//! `story:regexp-objects-exec-and-literals`: `RegExp.prototype.exec`
//! (`#sec-regexp.prototype.exec`) and `RegExp.prototype.test` (`#sec-regexp.prototype.test`),
//! RegExpExec (`#sec-regexpexec`), RegExpBuiltinExec (`#sec-regexpbuiltinexec`) with
//! MakeMatchIndicesIndexPairArray (`#sec-makematchindicesindexpairarray`), and
//! AdvanceStringIndex (`#sec-advancestringindex`). RegExpExec, RegExpBuiltinExec and
//! AdvanceStringIndex are `pub(crate)` for the methods keyed by well-known symbols of the later
//! stories.
//!
//! The search of `swell-regexp` reads the subject in place in the arena of Strings and the
//! program in the object's record, while it charges the agent's budget (`regexp/budget.rs`); a
//! stop of the budget ends the entry. Positions are code unit indices in every mode, `u`
//! included, and become `index`, `lastIndex` and `indices` as they are: nothing converts
//! (`docs/design/engine.md`, decision 7, "How `swell-regexp` charges its work", item 3).
//!
//! The captures of a match are copied out of the search into a list charged to the memory
//! budget, credited when the match array is made; the names of the groups are copied the same
//! way. The match array, its `groups` object and the `indices` array are charged as any Array
//! and object; each captured String is read with a step per code unit; each group costs a step.
//! No code runs while the array is made, so nothing in it needs a root until it is returned.
//!
//! Duplicate group names: a name is defined on `groups` by the first group that has it, and a
//! later group of the name that captured replaces the value; one that did not capture leaves the
//! value of an earlier one that did. This is what the steps over `matchedGroupNames` answer,
//! since at most one group of a name captures in a match.

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::ops::Range;

use swell_regexp::{Budget, Stop, Subject, search};

use super::budget::AgentBudget;
use crate::builtins::object::argument;
use crate::heap::Heap;
use crate::heap::arena::Arena;
use crate::heap::string::HeapString;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Key;
use crate::object::{ObjectKind, ObjectRecord};
use crate::value::{ObjectRef, StringRef, Value};

/// The captures of a match, copied out of the search: the index the try that matched started
/// at, each group's range, group 0 first, and the bytes the list was charged.
struct Matched {
    index: usize,
    captures: Vec<Option<Range<usize>>>,
    bytes: u64,
}

/// What the search in place answered.
enum Searched {
    /// The object or the String names no record of its kind: an engine bug.
    Stale,
    Stopped(Stop),
    Done(Option<Matched>),
}

/// The names of the groups of a program, copied out of the record, with the bytes they were
/// charged.
struct GroupNames {
    names: Vec<(u32, String)>,
    bytes: u64,
}

impl GroupNames {
    /// The name of group `group`, read from the position `cursor`, which moves past it: the
    /// names are in ascending order of their groups.
    fn name_of(&self, group: u32, cursor: &mut usize) -> Option<&str> {
        while let Some((number, name)) = self.names.get(*cursor) {
            if *number > group {
                return None;
            }
            *cursor = cursor.saturating_add(1);
            if *number == group {
                return Some(name);
            }
        }
        None
    }
}

/// The search of the program of `regexp` over `string` from `start`, read in place, charged to
/// `budget`; a match's captures are copied into a list charged before it is made.
fn search_in_place(
    objects: &Arena<ObjectRecord>,
    strings: &Arena<HeapString>,
    regexp: ObjectRef,
    string: StringRef,
    start: usize,
    budget: &mut AgentBudget<'_>,
) -> Searched {
    let program = match objects.get(regexp.handle) {
        Ok(ObjectRecord {
            kind: ObjectKind::RegExp(regexp),
            ..
        }) => &regexp.program,
        _ => return Searched::Stale,
    };
    let subject = match strings.get(string.handle) {
        Ok(HeapString::Latin1(bytes)) => Subject::Latin1(bytes),
        Ok(HeapString::Utf16(units)) => Subject::Utf16(units),
        Err(_) => return Searched::Stale,
    };
    let found = match search(program, subject, start, budget) {
        Ok(Some(found)) => found,
        Ok(None) => return Searched::Done(None),
        Err(stop) => return Searched::Stopped(stop),
    };
    let count = found.captures.len();
    let bytes = count.saturating_mul(size_of::<Option<Range<usize>>>());
    if let Err(stop) = budget.charge(bytes) {
        return Searched::Stopped(stop);
    }
    let mut captures = Vec::new();
    if captures.try_reserve_exact(count).is_err() {
        budget.credit(bytes);
        return Searched::Stopped(Stop::MemoryBudget);
    }
    captures.extend(found.captures.iter());
    Searched::Done(Some(Matched {
        index: found.index,
        captures,
        bytes: u64::try_from(bytes).unwrap_or(u64::MAX),
    }))
}

impl<H: Host> Agent<H> {
    /// `RegExp.prototype.exec ( string )` (`#sec-regexp.prototype.exec`).
    pub(super) fn regexp_prototype_exec(
        &mut self,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        // 1. Let R be the this value. 2. Perform ? RequireInternalSlot(R, [[RegExpMatcher]]).
        let regexp = match this {
            Value::Object(object) if self.regexp_slots(object)?.is_some() => object,
            _ => {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "RegExp.prototype.exec requires that 'this' be a RegExp object",
                ));
            }
        };
        // 3. Let S be ? ToString(string).
        let string = self.to_string(argument(args, 0))?;
        // 4. Return ? RegExpBuiltinExec(R, S).
        let mark = self.keep(Value::String(string));
        let answer = self.regexp_builtin_exec(regexp, string);
        self.release(mark);
        answer
    }

    /// `RegExp.prototype.test ( S )` (`#sec-regexp.prototype.test`).
    pub(super) fn regexp_prototype_test(
        &mut self,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        // 1. Let R be the this value. 2. If R is not an Object, throw a TypeError exception.
        let Value::Object(regexp) = this else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "RegExp.prototype.test requires that 'this' be an Object",
            ));
        };
        // 3. Let string be ? ToString(S).
        let string = self.to_string(argument(args, 0))?;
        // 4. Let match be ? RegExpExec(R, string). 5. If match is null, return false. 6.
        // Return true.
        let mark = self.keep(Value::String(string));
        let answer = self.regexp_exec(regexp, string);
        self.release(mark);
        Ok(Value::Boolean(!matches!(answer?, Value::Null)))
    }

    /// RegExpExec ( R, S ) (`#sec-regexpexec`): an Object or null. The caller keeps `regexp`
    /// and `string`.
    pub(crate) fn regexp_exec(
        &mut self,
        regexp: ObjectRef,
        string: StringRef,
    ) -> Completion<Value> {
        // 1. Let exec be ? Get(R, "exec").
        let key = self.name_key("exec")?;
        let exec = self.get(regexp, key)?;
        // 2. If IsCallable(exec) is true, then a. Let result be ? Call(exec, R, « S »). b. If
        // result is not an Object and result is not null, throw a TypeError exception. c.
        // Return result.
        if self.is_callable(exec)? {
            let result = self.call_value(exec, Value::Object(regexp), &[Value::String(string)])?;
            if !matches!(result, Value::Object(_) | Value::Null) {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "the result of exec is neither an object nor null",
                ));
            }
            return Ok(result);
        }
        // 3. Perform ? RequireInternalSlot(R, [[RegExpMatcher]]).
        if self.regexp_slots(regexp)?.is_none() {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "RegExpExec requires a RegExp object when exec is not callable",
            ));
        }
        // 4. Return ? RegExpBuiltinExec(R, S).
        self.regexp_builtin_exec(regexp, string)
    }

    /// RegExpBuiltinExec ( R, S ) (`#sec-regexpbuiltinexec`): the match array or null. The
    /// caller keeps `regexp`, which has the slots, and `string`.
    pub(crate) fn regexp_builtin_exec(
        &mut self,
        regexp: ObjectRef,
        string: StringRef,
    ) -> Completion<Value> {
        // 1. Let length be the length of S.
        let length = self.string_length(string)?;
        // 2. Let lastIndex be ℝ(? ToLength(! Get(R, "lastIndex"))).
        let key = self.name_key("lastIndex")?;
        let value = self.get(regexp, key)?;
        let last_index = self.to_length(value)?;
        // 3 to 6. The flags g, y and d of R.[[OriginalFlags]], as its program holds them.
        let Some((_, _, flags)) = self.regexp_slots(regexp)? else {
            return Err(self.fault());
        };
        let moves = flags.global() || flags.sticky();
        // 7. If global is false and sticky is false, set lastIndex to 0.
        let last_index = if moves { last_index } else { 0.0 };
        // 13.a. If lastIndex > length, then i. If global is true or sticky is true, then 1.
        // Perform ? Set(R, "lastIndex", +0𝔽, true). ii. Return null.
        if last_index > length as f64 {
            if moves {
                self.set(regexp, key, Value::Number(0.0), true)?;
            }
            return Ok(Value::Null);
        }
        // 8 to 13. The matcher from lastIndex, moved by AdvanceStringIndex until a match or the
        // end (only at lastIndex under `y`): the search of the crate, in place.
        let start = last_index as usize;
        let limit = self.limits.steps_per_entry;
        let searched = {
            let Agent {
                heap,
                steps,
                interrupt,
                ..
            } = self;
            let Heap {
                accounting,
                strings,
                objects,
                ..
            } = heap;
            let mut budget = AgentBudget::new(accounting, steps, limit, interrupt);
            search_in_place(objects, strings, regexp, string, start, &mut budget)
        };
        let matched = match searched {
            Searched::Stale => return Err(self.fault()),
            Searched::Stopped(stop) => return Err(self.regexp_stopped(stop)),
            // 13.d.i. If r is failure and sticky is true, then 1. Perform ? Set(R, "lastIndex",
            // +0𝔽, true). 2. Return null; past the end, 13.a.
            Searched::Done(None) => {
                if moves {
                    self.set(regexp, key, Value::Number(0.0), true)?;
                }
                return Ok(Value::Null);
            }
            Searched::Done(Some(matched)) => matched,
        };
        let made = self.match_array(regexp, string, key, moves, flags.has_indices(), &matched);
        self.heap.accounting.credit(matched.bytes);
        made
    }

    /// Steps 14 to 35 of RegExpBuiltinExec for the match `matched`: `lastIndex` set under `g`
    /// or `y`, then the match array.
    fn match_array(
        &mut self,
        regexp: ObjectRef,
        string: StringRef,
        last_index_key: Key,
        moves: bool,
        has_indices: bool,
        matched: &Matched,
    ) -> Completion<Value> {
        // 14. Let endIndex be result.[[EndIndex]]; 15. under `u` it is a code unit index already.
        let Some(Some(whole)) = matched.captures.first() else {
            return Err(self.fault());
        };
        let end = whole.end;
        // 16. If global is true or sticky is true, then a. Perform ? Set(R, "lastIndex", 𝔽(e),
        // true).
        if moves {
            self.set(regexp, last_index_key, Value::Number(end as f64), true)?;
        }
        let names = self.group_names(regexp)?;
        let mark = self.kept.len();
        let made = self.match_array_named(string, has_indices, matched, end, &names);
        self.release(mark);
        self.heap.accounting.credit(names.bytes);
        made
    }

    /// The names of the groups of the program of `regexp`, copied and charged.
    fn group_names(&mut self, regexp: ObjectRef) -> Completion<GroupNames> {
        let (count, text) =
            match &self.engine_object(regexp)?.kind {
                ObjectKind::RegExp(record) => record.program.group_names().fold(
                    (0usize, 0usize),
                    |(count, text), (_, name)| {
                        (count.saturating_add(1), text.saturating_add(name.len()))
                    },
                ),
                _ => return Err(self.fault()),
            };
        let bytes = count
            .saturating_mul(size_of::<(u32, String)>())
            .saturating_add(text);
        let bytes = u64::try_from(bytes).unwrap_or(u64::MAX);
        self.charge_bytes(bytes)?;
        let mut names = Vec::new();
        if names.try_reserve_exact(count).is_err() {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(crate::host::Termination::MemoryBudget));
        }
        if let Ok(ObjectRecord {
            kind: ObjectKind::RegExp(record),
            ..
        }) = self.heap.objects.get(regexp.handle)
        {
            names.extend(
                record
                    .program
                    .group_names()
                    .map(|(number, name)| (number, name.to_string())),
            );
        }
        Ok(GroupNames { names, bytes })
    }

    /// Steps 20 to 35 of RegExpBuiltinExec: the Array of the match with `index`, `input`, the
    /// captures, `groups` and, under `d`, `indices`. The objects made are kept.
    fn match_array_named(
        &mut self,
        string: StringRef,
        has_indices: bool,
        matched: &Matched,
        end: usize,
        names: &GroupNames,
    ) -> Completion<Value> {
        let count = matched.captures.len();
        // 20. Let A be ! ArrayCreate(n + 1).
        let array = self.array_create(count as f64, None)?;
        self.keep(Value::Object(array));
        // 22. Perform ! CreateDataPropertyOrThrow(A, "index", 𝔽(lastIndex)).
        let key = self.name_key("index")?;
        self.create_data_property_or_throw(array, key, Value::Number(matched.index as f64))?;
        // 23. Perform ! CreateDataPropertyOrThrow(A, "input", S).
        let key = self.name_key("input")?;
        self.create_data_property_or_throw(array, key, Value::String(string))?;
        // 24 to 29. The match from lastIndex to e, "0".
        let whole = self.match_string(string, matched.index..end)?;
        self.create_data_property_or_throw(array, Key::Index(0), whole)?;
        // 30. groups: an object with a null prototype when R has a GroupName.
        let has_groups = !names.names.is_empty();
        let groups = if has_groups {
            let groups = self.ordinary_object_create(None, OrdinaryData::Plain)?;
            self.keep(Value::Object(groups));
            Value::Object(groups)
        } else {
            Value::Undefined
        };
        // 31. Perform ! CreateDataPropertyOrThrow(A, "groups", groups).
        let key = self.name_key("groups")?;
        self.create_data_property_or_throw(array, key, groups)?;
        // 33. Each capture i from 1 on, a step each.
        let mut cursor = 0;
        for (i, capture) in matched.captures.iter().enumerate().skip(1) {
            self.charge_steps(1)?;
            let Ok(group) = u32::try_from(i) else {
                return Err(self.fault());
            };
            let value = match capture {
                Some(range) => self.match_string(string, range.clone())?,
                None => Value::Undefined,
            };
            self.create_data_property_or_throw(array, Key::Index(group), value)?;
            if let (Value::Object(groups), Some(name)) = (groups, names.name_of(group, &mut cursor))
            {
                self.define_group(groups, name, value)?;
            }
        }
        // 34. If hasIndices is true, then a. Let indicesArray be
        // MakeMatchIndicesIndexPairArray(S, indices, groupNames, hasGroups). b. Perform !
        // CreateDataPropertyOrThrow(A, "indices", indicesArray).
        if has_indices {
            let indices = self.match_indices_array(matched, has_groups, names)?;
            let key = self.name_key("indices")?;
            self.create_data_property_or_throw(array, key, Value::Object(indices))?;
        }
        // 35. Return A.
        Ok(Value::Object(array))
    }

    /// The value of the group name `name` on `groups`, by the steps over `matchedGroupNames`:
    /// a captured value is defined; an undefined one only when no group of the name defined
    /// the property before.
    fn define_group(&mut self, groups: ObjectRef, name: &str, value: Value) -> Completion<()> {
        let key = self.name_key(name)?;
        if matches!(value, Value::Undefined) && self.has_own_property(groups, key)? {
            return Ok(());
        }
        self.create_data_property_or_throw(groups, key, value)
    }

    /// MakeMatchIndicesIndexPairArray ( S, indices, groupNames, hasGroups )
    /// (`#sec-makematchindicesindexpairarray`), with GetMatchIndexPair
    /// (`#sec-getmatchindexpair`) of each capture.
    fn match_indices_array(
        &mut self,
        matched: &Matched,
        has_groups: bool,
        names: &GroupNames,
    ) -> Completion<ObjectRef> {
        // 1. Let n be the number of elements in indices. 5. Let A be ! ArrayCreate(n).
        let count = matched.captures.len();
        let array = self.array_create(count as f64, None)?;
        self.keep(Value::Object(array));
        // 6. groups: an object with a null prototype when hasGroups is true.
        let groups = if has_groups {
            let groups = self.ordinary_object_create(None, OrdinaryData::Plain)?;
            self.keep(Value::Object(groups));
            Value::Object(groups)
        } else {
            Value::Undefined
        };
        // 7. Perform ! CreateDataPropertyOrThrow(A, "groups", groups).
        let key = self.name_key("groups")?;
        self.create_data_property_or_throw(array, key, groups)?;
        // 8. For each integer i such that 0 ≤ i < n, in ascending order.
        let mut cursor = 0;
        for (i, capture) in matched.captures.iter().enumerate() {
            self.charge_steps(1)?;
            let Ok(index) = u32::try_from(i) else {
                return Err(self.fault());
            };
            // a to c. matchIndexPair: GetMatchIndexPair(S, matchIndices), or undefined; the
            // first of indices is the match, from lastIndex (RegExpBuiltinExec, step 24).
            let capture = match (i, capture) {
                (0, Some(whole)) => Some(matched.index..whole.end),
                (_, capture) => capture.clone(),
            };
            let pair = match capture {
                Some(range) => Value::Object(self.create_array_from_list(&[
                    Value::Number(range.start as f64),
                    Value::Number(range.end as f64),
                ])?),
                None => Value::Undefined,
            };
            // d. Perform ! CreateDataPropertyOrThrow(A, ! ToString(𝔽(i)), matchIndexPair).
            self.create_data_property_or_throw(array, Key::Index(index), pair)?;
            // e. If i > 0 and the name of the group is not undefined, the pair on groups.
            if index > 0
                && let (Value::Object(groups), Some(name)) =
                    (groups, names.name_of(index, &mut cursor))
            {
                self.define_group(groups, name, pair)?;
            }
        }
        // 9. Return A.
        Ok(array)
    }

    /// GetMatchString ( S, match ) (`#sec-getmatchstring`): the substring of `string` over
    /// `range`, a step per code unit read.
    fn match_string(&mut self, string: StringRef, range: Range<usize>) -> Completion<Value> {
        let units = self.read_units(string, range.start, range.end)?;
        Ok(Value::String(self.new_string_value(&units)?))
    }

    /// AdvanceStringIndex ( S, index, unicode ) (`#sec-advancestringindex`).
    pub(crate) fn advance_string_index(
        &mut self,
        string: StringRef,
        index: u64,
        unicode: bool,
    ) -> Completion<u64> {
        let next = index.saturating_add(1);
        // 2. If unicode is false, return index + 1.
        if !unicode {
            return Ok(next);
        }
        // 3. Let length be the length of S. 4. If index + 1 ≥ length, return index + 1.
        let length = u64::try_from(self.string_length(string)?).unwrap_or(u64::MAX);
        if next >= length {
            return Ok(next);
        }
        // 5. Let cp be CodePointAt(S, index). 6. Return index + cp.[[CodeUnitCount]].
        let at = usize::try_from(index).unwrap_or(usize::MAX);
        let lead = self.code_unit_at(string, at)?;
        let trail = self.code_unit_at(string, at.saturating_add(1))?;
        let paired = lead.is_some_and(|unit| (0xD800..=0xDBFF).contains(&unit))
            && trail.is_some_and(|unit| (0xDC00..=0xDFFF).contains(&unit));
        Ok(if paired {
            index.saturating_add(2)
        } else {
            next
        })
    }
}

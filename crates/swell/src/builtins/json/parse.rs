//! `JSON.parse` (`#sec-json.parse`) with ParseJSON (`#sec-ParseJSON`), the JSON Parse Record
//! (`#sec-json-parse-record`), CreateJSONParseRecord (`#sec-createjsonparserecord`),
//! InternalizeJSONProperty (`#sec-internalizejsonproperty`), ShallowestContainedJSONValue
//! (`#sec-shallowestcontainedjsonvalue`) and JSONArrayLiteralContentNodes
//! (`#sec-static-semantics-jsonarrayliteralcontentnodes`); `JSON.rawJSON` (`#sec-json.rawjson`)
//! and `JSON.isRawJSON` (`#sec-json.israwjson`). Written by `story:json-parse-and-raw-json`.
//!
//! ParseJSON reads the text by the grammar of ECMA-404, as the draft cites it, in a loop over an
//! explicit stack with one entry per open array or object (decision 3, "Depth bounds"): a
//! nesting of any depth takes no Rust stack, and each open entry is charged to the memory
//! budget while it is open. The value is made as the text is read, as the evaluation of the
//! text as a Script would make it (13.2.5.6 in its JSON form: a duplicate key keeps the last
//! value, `__proto__` is an own data property). A step is charged per code unit of the text
//! before the text is copied, and per value made, in batches of `STEP_CHUNK` (decision 2, "per
//! node for `JSON.parse`"). Every String the parse makes, a key included, is bounded by
//! `string_length`.
//!
//! The parse node of ParseJSON is not kept: with a reviver, the parse makes the JSON Parse
//! Records CreateJSONParseRecord would make from it as it goes, the record of a primitive value
//! holding the range of its source text, which is what ShallowestContainedJSONValue finds (the
//! literal, or the unary minus with its number) and JSONArrayLiteralContentNodes lists (an
//! element of an array). InternalizeJSONProperty walks the values in a loop over a stack of
//! its own, one entry per open object or array, charged to the memory budget. Each record, its
//! value and its key are kept on the agent's kept stack (decision 4) until `JSON.parse`
//! returns, so that a value the reviver detached is still there for SameValue; each open entry
//! of the walk keeps its value, its context object and its keys. The call of the reviver is
//! made from a function that holds only the call's arguments: the walk's steps around it are
//! functions that return before it.

use crate::builtins::object::argument;
use crate::heap::accounting::ChargedVec;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::ObjectKind;
use crate::object::builtin_function::KeptRoom;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Key, array_index};
use crate::ops::objects::IntegrityLevel;
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, StringRef, Value};
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

/// The values made between two charges of the step budget.
const STEP_CHUNK: u64 = 4_096;

/// A JSON Parse Record (`#sec-json-parse-record`): [[Value]], the source text of [[ParseNode]]
/// for a primitive value (a range of code units of the text), and [[Elements]] or [[Entries]]
/// as the key and the index of each record. [[Key]] is the key its parent lists it under.
struct Record {
    value: Value,
    start: usize,
    end: usize,
    children: ChargedVec<(Key, usize)>,
}

/// An array or object the parse has opened and not closed: an element of its stack.
struct Open {
    object: ObjectRef,
    is_array: bool,
    /// The index of the next element of an array.
    length: f64,
    /// The key of the member being read, of an object, and the kept stack's length before it.
    key: Option<(Key, usize)>,
    /// The index of its record, when the parse makes records.
    record: usize,
    /// The kept stack's length before the object was kept, when the parse makes no records.
    mark: Option<usize>,
}

/// The state of ParseJSON.
struct Parser<'t> {
    text: &'t [u16],
    position: usize,
    /// Steps read and not yet charged.
    unpaid: u64,
    stack: Vec<Open>,
    /// The records, when a reviver will walk them.
    records: Option<ChargedVec<Record>>,
    /// The bytes charged for the open entries and the records, credited when they go.
    charged: u64,
}

impl Parser<'_> {
    /// The code unit at the position.
    fn peek(&self) -> Option<u16> {
        self.text.get(self.position).copied()
    }

    /// Read `count` code units: their steps, one each, were charged with the text before it
    /// was copied ([`Agent::json_text_units`]).
    fn advance(&mut self, count: usize) {
        self.position = self.position.saturating_add(count);
    }
}

/// The members of a value the walk has opened.
enum Members {
    /// An array: its length and the next index.
    Array { length: f64, next: f64 },
    /// An object: its keys of EnumerableOwnProperties, the next one, and the entry of its
    /// record that the next key is looked for first.
    Object {
        keys: Vec<Key>,
        next: usize,
        hint: usize,
    },
}

/// An object or array InternalizeJSONProperty is walking: an element of the walk's stack.
struct Walk {
    holder: ObjectRef,
    name: Key,
    value: ObjectRef,
    context: ObjectRef,
    /// Its record, when the record's [[Value]] is the value.
    record: Option<usize>,
    members: Members,
    /// The key of the member being internalized.
    current: Option<Key>,
    /// The kept stack's length before its value was kept.
    mark: usize,
    /// The bytes charged for it.
    charged: u64,
}

/// An InternalizeJSONProperty that has not started: holder, name and parseRecord.
#[derive(Clone, Copy)]
struct Enter {
    holder: ObjectRef,
    name: Key,
    record: Option<usize>,
}

/// The last step of an InternalizeJSONProperty: Call(reviver, holder, « name, val, context »),
/// and the kept stack's length to release to after it.
#[derive(Clone, Copy)]
struct ReviverCall {
    holder: ObjectRef,
    name: Key,
    value: Value,
    context: ObjectRef,
    mark: usize,
}

/// The state of the walk of InternalizeJSONProperty.
struct Walker<'p> {
    reviver: Value,
    text: &'p [u16],
    records: &'p [Record],
    frames: ChargedVec<Walk>,
    pending: Option<Enter>,
    /// The bytes charged for the open frames and their lists of keys, credited when they
    /// close.
    charged: u64,
    /// The room the kept stack had when the walk began, and the growth charged for the keys
    /// the frames keep, given back when the walk ends.
    room: KeptRoom,
}

/// The value ParseJSON made and the index of its record.
struct Parsed {
    value: Value,
    record: usize,
}

/// Whether `unit` is JSON white space (ECMA-404: tab, line feed, carriage return, space).
fn is_json_space(unit: u16) -> bool {
    matches!(unit, 0x09 | 0x0A | 0x0D | 0x20)
}

/// Whether `unit` is an ASCII decimal digit.
fn is_digit(unit: u16) -> bool {
    (0x30..=0x39).contains(&unit)
}

/// The value of an ASCII hexadecimal digit.
fn hex_value(unit: u16) -> Option<u16> {
    match unit {
        0x30..=0x39 => Some(unit - 0x30),
        0x41..=0x46 => Some(unit - 0x41 + 10),
        0x61..=0x66 => Some(unit - 0x61 + 10),
        _ => None,
    }
}

/// The bytes of `count` elements of `T`.
fn bytes_of<T>(count: usize) -> u64 {
    u64::try_from(count.saturating_mul(size_of::<T>())).unwrap_or(u64::MAX)
}

impl<H: Host> Agent<H> {
    /// `JSON.parse ( text [ , reviver ] )` (`#sec-json.parse`). The arguments are held by this
    /// builtin's activation; the parse and the walk run in a room of the kept stack, ended here
    /// on every way out, and what they keep is released and credited here.
    pub(super) fn json_parse(&mut self, args: &[Value]) -> Completion<Value> {
        // The room is ended on every way out, as `Agent::with_kept_room` ends it; written out
        // here so that no closure's frame stands between this call and the reviver's re-entry.
        let mark = self.kept.len();
        let room = self.kept_room();
        let mut charged = 0;
        let result = self.json_parse_with(args, &mut charged);
        self.end_kept_room(mark, &room);
        self.heap.accounting.credit(charged);
        result
    }

    /// The steps of `JSON.parse`; `charged` is what stays charged when it returns.
    fn json_parse_with(&mut self, args: &[Value], charged: &mut u64) -> Completion<Value> {
        let reviver = argument(args, 1);
        // 1. Let jsonString be ? ToString(text).
        let string = self.to_string(argument(args, 0))?;
        self.keep(Value::String(string));
        let units = self.json_text_units(string)?;
        // 4. If IsCallable(reviver) is false, return unfiltered: the records are made only
        // for a reviver (IsCallable runs no code, so it is asked before the parse).
        let callable = self.is_callable(reviver)?;
        // 2, 3. Let parseResult be ? ParseJSON(jsonString); let unfiltered be its [[Value]].
        let mut parser = Parser {
            text: &units,
            position: 0,
            unpaid: 0,
            stack: Vec::new(),
            records: callable.then(ChargedVec::new),
            charged: 0,
        };
        let parsed = self.json_parse_text(&mut parser);
        // The parse has ended (a parse that succeeds leaves its stack of open values empty): the
        // stack's room is freed here, before InternalizeJSONProperty calls the reviver, since a
        // close credits the charge of its entry and leaves the room.
        parser.stack = Vec::new();
        *charged = charged.saturating_add(parser.charged);
        let parsed = parsed?;
        let Some(records) = parser.records else {
            return Ok(parsed.value);
        };
        // 5. Let root be OrdinaryObjectCreate(%Object.prototype%).
        let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let root = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        self.keep(Value::Object(root));
        // 6, 7. Let rootName be the empty String; ! CreateDataPropertyOrThrow(root, rootName,
        // unfiltered).
        let root_name = self.name_key("")?;
        self.keep_key(root_name);
        self.create_data_property_or_throw(root, root_name, parsed.value)?;
        // 8. Let snapshot be CreateJSONParseRecord(parseResult.[[ParseNode]], rootName,
        // unfiltered): the record the parse made of the whole text.
        // 9. Return ? InternalizeJSONProperty(root, rootName, reviver, snapshot).
        let walk_mark = self.kept.len();
        let mut walker = Walker {
            reviver,
            text: &units,
            records: &records,
            frames: ChargedVec::new(),
            pending: Some(Enter {
                holder: root,
                name: root_name,
                record: Some(parsed.record),
            }),
            charged: 0,
            room: self.kept_room(),
        };
        let result = self.json_internalize(&mut walker);
        *charged = charged.saturating_add(walker.charged);
        self.end_kept_room(walk_mark, &walker.room);
        result
    }

    /// The code units of `string`, held charged to the memory budget while the parse and the
    /// walk read them, and a step per code unit, the step of reading it, charged before they
    /// are copied ([`Agent::charged_units`]): a parse that reads the text to its end costs what
    /// it costs to read it unit by unit, and one that fails early has paid for the whole text.
    fn json_text_units(&mut self, string: StringRef) -> Completion<crate::object::string::Units> {
        self.charged_units(string)
    }

    /// Charge `bytes` of the parse to the memory budget.
    fn json_parse_charge(&mut self, parser: &mut Parser<'_>, bytes: u64) -> Completion<()> {
        if self.heap.accounting.charge(bytes).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        parser.charged = parser.charged.saturating_add(bytes);
        Ok(())
    }

    /// Charge the steps read so far.
    fn json_pay(&mut self, parser: &mut Parser<'_>) -> Completion<()> {
        let unpaid = core::mem::take(&mut parser.unpaid);
        self.charge_steps(unpaid)
    }

    /// Charge the steps read so far once a chunk of them is read.
    fn json_pay_chunk(&mut self, parser: &mut Parser<'_>) -> Completion<()> {
        if parser.unpaid >= STEP_CHUNK {
            self.json_pay(parser)?;
        }
        Ok(())
    }

    /// The `SyntaxError` of a text that is not JSON (`#sec-ParseJSON`, step 1).
    fn json_syntax_error(&mut self, parser: &Parser<'_>) -> crate::host::Abrupt {
        let message = match parser.peek() {
            Some(_) => format!(
                "JSON.parse: the text is not JSON at code unit {}",
                parser.position
            ),
            None => "JSON.parse: the text ends before its JSON value".to_string(),
        };
        self.throw_error(ErrorKind::SyntaxError, &message)
    }

    /// ParseJSON ( text ) (`#sec-ParseJSON`): step 1 by the grammar of ECMA-404 and the
    /// evaluation of steps 2 to 8 at once, in a loop over the stack of open arrays and objects.
    fn json_parse_text(&mut self, parser: &mut Parser<'_>) -> Completion<Parsed> {
        let mut top = None;
        loop {
            // A value; an array or object that is not empty asks for its first value next.
            if self.json_value(parser, &mut top)? {
                continue;
            }
            // What follows a value: a comma before the next one, or the ends of the arrays
            // and objects that close here.
            if !self.json_after_value(parser)? {
                break;
            }
        }
        // Nothing but white space follows the value of the text.
        self.json_space(parser)?;
        if parser.position != parser.text.len() {
            return Err(self.json_syntax_error(parser));
        }
        self.json_pay(parser)?;
        top.ok_or_else(|| self.fault())
    }

    /// White space, read.
    fn json_space(&mut self, parser: &mut Parser<'_>) -> Completion<()> {
        while parser.peek().is_some_and(is_json_space) {
            parser.advance(1);
            self.json_pay_chunk(parser)?;
        }
        Ok(())
    }

    /// One JSON value, after white space: a primitive is made and placed; an array or object
    /// is made, placed and opened. Answers whether a value is wanted next (the first element
    /// or member of an array or object that is not empty).
    fn json_value(
        &mut self,
        parser: &mut Parser<'_>,
        top: &mut Option<Parsed>,
    ) -> Completion<bool> {
        self.json_space(parser)?;
        let start = parser.position;
        let value = match parser.peek() {
            Some(open @ (0x5B | 0x7B)) => {
                parser.advance(1);
                let is_array = open == 0x5B;
                return self.json_open_value(parser, top, is_array, start);
            }
            Some(0x22) => Value::String(self.json_string(parser)?),
            Some(0x2D | 0x30..=0x39) => Value::Number(self.json_number(parser)?),
            Some(0x74) => self.json_literal(parser, "true", Value::Boolean(true))?,
            Some(0x66) => self.json_literal(parser, "false", Value::Boolean(false))?,
            Some(0x6E) => self.json_literal(parser, "null", Value::Null)?,
            _ => return Err(self.json_syntax_error(parser)),
        };
        let record = self.json_node(parser, value, start, parser.position)?;
        self.json_place(parser, top, value, record)?;
        Ok(false)
    }

    /// An array or object whose bracket was read: made, placed, opened, and closed again at
    /// once when it is empty. Answers whether its first value is wanted next.
    fn json_open_value(
        &mut self,
        parser: &mut Parser<'_>,
        top: &mut Option<Parsed>,
        is_array: bool,
        start: usize,
    ) -> Completion<bool> {
        let object = if is_array {
            self.array_create(0.0, None)?
        } else {
            let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
            self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?
        };
        let value = Value::Object(object);
        let record = self.json_node(parser, value, start, start)?;
        self.json_place(parser, top, value, record)?;
        let mark = parser.records.is_none().then(|| self.keep(value));
        let bytes = bytes_of::<Open>(1).saturating_add(bytes_of::<Value>(1));
        self.json_parse_charge(parser, bytes)?;
        #[expect(
            clippy::disallowed_methods,
            reason = "each entry is charged by `json_parse_charge` before the stack grows; `try_reserve(1)` leaves a capacity of at most max(4, 2(d - 1)) entries of `size_of::<Open>()` bytes for a deepest nesting d, so the spare room, uncharged while the parse runs, is at most the charged entries again or 4 entries; no script runs during the parse, and `json_parse_with` frees the stack before the reviver is called"
        )]
        if parser.stack.try_reserve(1).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        parser.stack.push(Open {
            object,
            is_array,
            length: 0.0,
            key: None,
            record,
            mark,
        });
        self.json_space(parser)?;
        let close = if is_array { 0x5D } else { 0x7D };
        if parser.peek() == Some(close) {
            parser.advance(1);
            self.json_parse_close(parser)?;
            return Ok(false);
        }
        if !is_array {
            self.json_member_name(parser)?;
        }
        Ok(true)
    }

    /// The end of the innermost array or object: it leaves the stack, and what it kept goes.
    fn json_parse_close(&mut self, parser: &mut Parser<'_>) -> Completion<()> {
        let Some(open) = parser.stack.pop() else {
            return Err(self.fault());
        };
        if let Some(mark) = open.mark {
            self.release(mark);
        }
        let bytes = bytes_of::<Open>(1).saturating_add(bytes_of::<Value>(1));
        self.heap.accounting.credit(bytes);
        parser.charged = parser.charged.saturating_sub(bytes);
        Ok(())
    }

    /// A member's name of the innermost object and its colon, after white space: the key is
    /// kept until its value is placed.
    fn json_member_name(&mut self, parser: &mut Parser<'_>) -> Completion<()> {
        self.json_space(parser)?;
        if parser.peek() != Some(0x22) {
            return Err(self.json_syntax_error(parser));
        }
        let units = self.json_string_units(parser)?;
        self.json_string_fits(units.len())?;
        let key = match array_index(&units) {
            Some(index) => Key::Index(index),
            None => match self.heap.intern(&units) {
                Ok(atom) => Key::String(atom),
                Err(_) => return Err(self.terminate(Termination::MemoryBudget)),
            },
        };
        let mark = self.keep_key(key);
        self.json_space(parser)?;
        if parser.peek() != Some(0x3A) {
            return Err(self.json_syntax_error(parser));
        }
        parser.advance(1);
        match parser.stack.last_mut() {
            Some(open) if !open.is_array => open.key = Some((key, mark)),
            _ => return Err(self.fault()),
        }
        Ok(())
    }

    /// After a value, the commas and closing brackets up to the next value: answers whether
    /// one is wanted, false when the stack is empty and the text's value is complete.
    fn json_after_value(&mut self, parser: &mut Parser<'_>) -> Completion<bool> {
        loop {
            self.json_space(parser)?;
            let Some(is_array) = parser.stack.last().map(|open| open.is_array) else {
                return Ok(false);
            };
            match parser.peek() {
                Some(0x2C) => {
                    parser.advance(1);
                    if !is_array {
                        self.json_member_name(parser)?;
                    }
                    return Ok(true);
                }
                Some(0x5D) if is_array => {
                    parser.advance(1);
                    self.json_parse_close(parser)?;
                }
                Some(0x7D) if !is_array => {
                    parser.advance(1);
                    self.json_parse_close(parser)?;
                }
                _ => return Err(self.json_syntax_error(parser)),
            }
        }
    }

    /// A value made: a step, and with a reviver CreateJSONParseRecord's record of it
    /// (`#sec-createjsonparserecord`), its value kept and its bytes charged. Answers the
    /// record's index (0 without records).
    fn json_node(
        &mut self,
        parser: &mut Parser<'_>,
        value: Value,
        start: usize,
        end: usize,
    ) -> Completion<usize> {
        parser.unpaid = parser.unpaid.saturating_add(1);
        self.json_pay_chunk(parser)?;
        if parser.records.is_none() {
            return Ok(0);
        }
        // The record and its entry in its parent are in lists charged by their capacity; the
        // two slots of the kept stack are charged here.
        self.json_parse_charge(parser, bytes_of::<Value>(2))?;
        #[expect(
            clippy::disallowed_methods,
            reason = "charged by `json_parse_charge`, the two slots of the kept stack, before it grows"
        )]
        if self.kept.try_reserve(2).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        self.keep(value);
        let Some(records) = parser.records.as_mut() else {
            return Err(self.fault());
        };
        self.reserve_vec(records, 1)?;
        let index = records.len();
        let pushed = records.push(Record {
            value,
            start,
            end,
            children: ChargedVec::new(),
        });
        if pushed.is_err() {
            return Err(self.fault());
        }
        Ok(index)
    }

    /// Place a value made: as the text's value when no array or object is open; as the next
    /// element of the innermost array (CreateDataProperty, as ArrayAccumulation does); as the
    /// member of the innermost object under the key read for it (CreateDataProperty, as
    /// PropertyDefinitionEvaluation does for JSON: a later duplicate replaces the value in the
    /// place of the first, and `__proto__` is an own property). With records, the value's
    /// record becomes an element or an entry of the record of its array or object; a duplicate
    /// key's record replaces the earlier one, as the last pair is the one that is kept.
    fn json_place(
        &mut self,
        parser: &mut Parser<'_>,
        top: &mut Option<Parsed>,
        value: Value,
        record: usize,
    ) -> Completion<()> {
        let recording = parser.records.is_some();
        let Some(open) = parser.stack.last_mut() else {
            *top = Some(Parsed { value, record });
            return Ok(());
        };
        let (object, parent) = (open.object, open.record);
        let (key, duplicate) = if open.is_array {
            let index = open.length;
            open.length += 1.0;
            (self.index_key(index)?, false)
        } else {
            let Some((key, mark)) = open.key.take() else {
                return Err(self.fault());
            };
            let duplicate = recording && self.get_own_property(object, key)?.is_some();
            if !recording {
                self.json_define(object, key, value)?;
                self.release(mark);
                return Ok(());
            }
            (key, duplicate)
        };
        self.json_define(object, key, value)?;
        let Some(records) = parser.records.as_mut() else {
            return Ok(());
        };
        let Some(children) = records.get_mut(parent).map(|parent| &mut parent.children) else {
            return Err(self.fault());
        };
        if duplicate {
            let mut compared = 0_u64;
            let mut found = false;
            for child in children.iter_mut() {
                compared += 1;
                if child.0 == key {
                    child.1 = record;
                    found = true;
                    break;
                }
            }
            self.charge_steps(compared)?;
            return if found { Ok(()) } else { Err(self.fault()) };
        }
        self.reserve_vec(children, 1)?;
        if children.push((key, record)).is_err() {
            return Err(self.fault());
        }
        Ok(())
    }

    /// CreateDataProperty of a parsed value, which cannot fail on the fresh, extensible
    /// objects the parse makes.
    fn json_define(&mut self, object: ObjectRef, key: Key, value: Value) -> Completion<()> {
        if !self.create_data_property(object, key, value)? {
            return Err(self.fault());
        }
        Ok(())
    }

    /// A literal `true`, `false` or `null`.
    fn json_literal(
        &mut self,
        parser: &mut Parser<'_>,
        word: &str,
        value: Value,
    ) -> Completion<Value> {
        for expected in word.encode_utf16() {
            if parser.peek() != Some(expected) {
                return Err(self.json_syntax_error(parser));
            }
            parser.advance(1);
        }
        Ok(value)
    }

    /// A number of ECMA-404: `-`? (`0` | a digit 1 to 9 and digits), then `.` and digits, then
    /// `e` or `E`, a sign and digits, each part optional but the first; its value is that of
    /// the same text as a StringNumericLiteral, which this grammar is a part of.
    fn json_number(&mut self, parser: &mut Parser<'_>) -> Completion<f64> {
        let start = parser.position;
        if parser.peek() == Some(0x2D) {
            parser.advance(1);
        }
        match parser.peek() {
            Some(0x30) => parser.advance(1),
            Some(0x31..=0x39) => self.json_digits(parser)?,
            _ => return Err(self.json_syntax_error(parser)),
        }
        if parser.peek() == Some(0x2E) {
            parser.advance(1);
            if !parser.peek().is_some_and(is_digit) {
                return Err(self.json_syntax_error(parser));
            }
            self.json_digits(parser)?;
        }
        if matches!(parser.peek(), Some(0x45 | 0x65)) {
            parser.advance(1);
            if matches!(parser.peek(), Some(0x2B | 0x2D)) {
                parser.advance(1);
            }
            if !parser.peek().is_some_and(is_digit) {
                return Err(self.json_syntax_error(parser));
            }
            self.json_digits(parser)?;
        }
        let Some(text) = parser.text.get(start..parser.position) else {
            return Err(self.fault());
        };
        Ok(swell_num::literal::string_to_number_trimmed(text))
    }

    /// A run of decimal digits.
    fn json_digits(&mut self, parser: &mut Parser<'_>) -> Completion<()> {
        while parser.peek().is_some_and(is_digit) {
            parser.advance(1);
            self.json_pay_chunk(parser)?;
        }
        Ok(())
    }

    /// A String of ECMA-404 as a String value, at most `string_length` code units long.
    fn json_string(&mut self, parser: &mut Parser<'_>) -> Completion<StringRef> {
        let units = self.json_string_units(parser)?;
        self.json_new_string(&units)
    }

    /// A new String of `units`, which must not be longer than `string_length`.
    fn json_new_string(&mut self, units: &[u16]) -> Completion<StringRef> {
        self.json_string_fits(units.len())?;
        self.new_string_value(units)
    }

    /// A `RangeError` for a String longer than `string_length`
    /// (`docs/design/host-interface.md`, section 2).
    fn json_string_fits(&mut self, length: usize) -> Completion<()> {
        if u64::try_from(length).unwrap_or(u64::MAX) > self.limits.string_length {
            return Err(self.throw_error(
                ErrorKind::RangeError,
                "the string would be longer than string_length",
            ));
        }
        Ok(())
    }

    /// The code units of a String of ECMA-404 at the position, its quotes read: no code unit
    /// below U+0020 unescaped, and the escapes `\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t` and
    /// `\u` with four hexadecimal digits. The units are bounded by the text, which is charged.
    fn json_string_units(&mut self, parser: &mut Parser<'_>) -> Completion<Vec<u16>> {
        parser.advance(1);
        let mut units = Vec::new();
        loop {
            self.json_pay_chunk(parser)?;
            let Some(unit) = parser.peek() else {
                return Err(self.json_syntax_error(parser));
            };
            let decoded = match unit {
                0x22 => {
                    parser.advance(1);
                    return Ok(units);
                }
                0x5C => {
                    parser.advance(1);
                    match parser.peek() {
                        Some(0x22) => 0x22,
                        Some(0x5C) => 0x5C,
                        Some(0x2F) => 0x2F,
                        Some(0x62) => 0x08,
                        Some(0x66) => 0x0C,
                        Some(0x6E) => 0x0A,
                        Some(0x72) => 0x0D,
                        Some(0x74) => 0x09,
                        Some(0x75) => {
                            parser.advance(1);
                            let mut code = 0_u16;
                            for _ in 0..4 {
                                let Some(digit) = parser.peek().and_then(hex_value) else {
                                    return Err(self.json_syntax_error(parser));
                                };
                                code = code * 16 + digit;
                                parser.advance(1);
                            }
                            #[expect(
                                clippy::disallowed_methods,
                                reason = "bounded by the charged text, one unit per escape read, dropped before any call into script"
                            )]
                            if units.try_reserve(1).is_err() {
                                return Err(self.terminate(Termination::MemoryBudget));
                            }
                            units.push(code);
                            continue;
                        }
                        _ => return Err(self.json_syntax_error(parser)),
                    }
                }
                _ if unit < 0x20 => return Err(self.json_syntax_error(parser)),
                _ => unit,
            };
            parser.advance(1);
            #[expect(
                clippy::disallowed_methods,
                reason = "bounded by the charged text, one unit per unit read, dropped before any call into script"
            )]
            if units.try_reserve(1).is_err() {
                return Err(self.terminate(Termination::MemoryBudget));
            }
            units.push(decoded);
        }
    }

    /// InternalizeJSONProperty ( holder, name, reviver, parseRecord )
    /// (`#sec-internalizejsonproperty`) of the root, as a loop: each property is entered
    /// (steps 1 to 4, a frame opened for an object or array), its members entered one after the
    /// other, and its last step, the Call of the reviver, made when its members are done; the
    /// answer is placed in its holder (step 5, the Delete or CreateDataProperty of the
    /// caller's loop). The answer for the root is the answer of `JSON.parse`.
    fn json_internalize(&mut self, walker: &mut Walker<'_>) -> Completion<Value> {
        loop {
            let call = self.json_internalize_next(walker)?;
            let answer = self.json_reviver_call(walker.reviver, call);
            self.release(call.mark);
            let answer = answer?;
            if walker.frames.is_empty() {
                return Ok(answer);
            }
            self.json_internalize_place(walker, answer)?;
        }
    }

    /// Call(reviver, holder, « name, val, context ») (`#sec-internalizejsonproperty`, the last
    /// step): name as a String. The arguments are held by the activation of the call.
    fn json_reviver_call(&mut self, reviver: Value, call: ReviverCall) -> Completion<Value> {
        let name = self.key_value(call.name)?;
        self.call_value(
            reviver,
            Value::Object(call.holder),
            &[name, call.value, Value::Object(call.context)],
        )
    }

    /// The walk up to the next Call of the reviver: the property waiting to be entered, else
    /// the next member of the innermost frame, else the end of that frame.
    fn json_internalize_next(&mut self, walker: &mut Walker<'_>) -> Completion<ReviverCall> {
        loop {
            if let Some(enter) = walker.pending.take() {
                if let Some(call) = self.json_internalize_enter(walker, enter)? {
                    return Ok(call);
                }
                continue;
            }
            let Some(frame) = walker.frames.last_mut() else {
                return Err(self.fault());
            };
            let member = match &mut frame.members {
                Members::Array { length, next } => {
                    // 5.a.iv. Repeat, while I < len: ! ToString(𝔽(I)).
                    let index = *next;
                    *next += 1.0;
                    (index < *length).then_some(Err(index))
                }
                Members::Object { keys, next, .. } => {
                    // 5.b.ii. For each String P of keys.
                    let key = keys.get(*next).copied();
                    *next += 1;
                    key.map(Ok)
                }
            };
            let Some(member) = member else {
                // The members are done: the Call of the frame's own property.
                let Some(frame) = walker.frames.pop() else {
                    return Err(self.fault());
                };
                self.heap.accounting.credit(frame.charged);
                walker.charged = walker.charged.saturating_sub(frame.charged);
                return Ok(ReviverCall {
                    holder: frame.holder,
                    name: frame.name,
                    value: Value::Object(frame.value),
                    context: frame.context,
                    mark: frame.mark,
                });
            };
            let key = match member {
                Ok(key) => key,
                Err(index) => {
                    let key = self.index_key(index)?;
                    self.keep_key(key);
                    key
                }
            };
            let record = self.json_member_record(walker, key)?;
            let Some(frame) = walker.frames.last_mut() else {
                return Err(self.fault());
            };
            frame.current = Some(key);
            walker.pending = Some(Enter {
                holder: frame.value,
                name: key,
                record,
            });
        }
    }

    /// The record of the member `key` of the innermost frame: the element of that index of
    /// elementRecords (5.a.iv.2), or the entry of entryRecords whose [[Key]] is `key`
    /// (5.b.ii.1), looked for first where the last one was found, a step per entry compared
    /// beyond it.
    fn json_member_record(
        &mut self,
        walker: &mut Walker<'_>,
        key: Key,
    ) -> Completion<Option<usize>> {
        let records = walker.records;
        let Some(frame) = walker.frames.last_mut() else {
            return Err(self.fault());
        };
        let Some(children) = frame
            .record
            .and_then(|record| records.get(record))
            .map(|record| &record.children)
        else {
            return Ok(None);
        };
        match &mut frame.members {
            Members::Array { .. } => Ok(match key {
                Key::Index(index) => usize::try_from(index)
                    .ok()
                    .and_then(|index| children.get(index))
                    .map(|child| child.1),
                _ => None,
            }),
            Members::Object { hint, .. } => {
                if let Some(child) = children.get(*hint)
                    && child.0 == key
                {
                    *hint += 1;
                    return Ok(Some(child.1));
                }
                let found = children.iter().position(|child| child.0 == key);
                let compared = found.map_or(children.len(), |at| at + 1);
                if let Some(at) = found {
                    *hint = at + 1;
                }
                let child = found.and_then(|at| children.get(at)).map(|child| child.1);
                self.charge_steps(u64::try_from(compared).unwrap_or(u64::MAX))?;
                Ok(child)
            }
        }
    }

    /// Steps 1 to 4 of InternalizeJSONProperty, and the start of step 5: val, context, the
    /// source of an unchanged primitive, and the frame of an object or array. A step per
    /// property. Answers the Call of a primitive's property, or `None` when a frame opened.
    fn json_internalize_enter(
        &mut self,
        walker: &mut Walker<'_>,
        enter: Enter,
    ) -> Completion<Option<ReviverCall>> {
        self.charge_steps(1)?;
        // 1. Let val be ? Get(holder, name).
        let value = self.get(enter.holder, enter.name)?;
        let mark = self.keep(value);
        // 2. Let context be OrdinaryObjectCreate(%Object.prototype%).
        let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let context = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        self.keep(Value::Object(context));
        // 3. If parseRecord is a JSON Parse Record and SameValue(parseRecord.[[Value]], val)
        // is true; else elementRecords and entryRecords are empty.
        let mut record = None;
        if let Some(index) = enter.record {
            let Some(parsed) = walker.records.get(index) else {
                return Err(self.fault());
            };
            let (parsed_value, start, end) = (parsed.value, parsed.start, parsed.end);
            if self.same_value(parsed_value, value)? {
                record = Some(index);
                // 3.a. If val is not an Object: context.source is the source text matched by
                // the parse node, as a String.
                if !matches!(value, Value::Object(_)) {
                    self.json_context_source(walker, context, start, end)?;
                }
            }
        }
        // 4. If val is an Object: its members are walked in a frame of their own.
        let Value::Object(object) = value else {
            return Ok(Some(ReviverCall {
                holder: enter.holder,
                name: enter.name,
                value,
                context,
                mark,
            }));
        };
        self.json_open_walk(walker, enter, object, context, record, mark)?;
        Ok(None)
    }

    /// Step 3.a.iv of InternalizeJSONProperty: ! CreateDataPropertyOrThrow(context,
    /// "source", CodePointsToString(sourceText)).
    fn json_context_source(
        &mut self,
        walker: &Walker<'_>,
        context: ObjectRef,
        start: usize,
        end: usize,
    ) -> Completion<()> {
        let Some(units) = walker.text.get(start..end) else {
            return Err(self.fault());
        };
        let source = self.json_new_string(units)?;
        let key = self.name_key("source")?;
        self.create_data_property_or_throw(context, key, Value::String(source))
    }

    /// Step 5 of InternalizeJSONProperty up to its loop: ? IsArray(val), then
    /// ? LengthOfArrayLike(val) or ? EnumerableOwnProperties(val, key) (`ops/objects.rs`),
    /// whose keys stay kept with the frame; the frame is charged and pushed.
    fn json_open_walk(
        &mut self,
        walker: &mut Walker<'_>,
        enter: Enter,
        object: ObjectRef,
        context: ObjectRef,
        record: Option<usize>,
        mark: usize,
    ) -> Completion<()> {
        let (members, list_bytes) = if self.is_array(Value::Object(object))? {
            let length = self.length_of_array_like(object)?;
            (Members::Array { length, next: 0.0 }, 0)
        } else {
            // Charged before it exists, each key kept in room charged to the walk; the frame
            // takes the list's charge over.
            let list = self.enumerable_own_keys(object, &mut walker.room)?;
            walker.charged = walker.charged.saturating_add(list.bytes);
            (
                Members::Object {
                    keys: list.keys,
                    next: 0,
                    hint: 0,
                },
                list.bytes,
            )
        };
        let frame = bytes_of::<Walk>(1).saturating_add(bytes_of::<Value>(2));
        if self.heap.accounting.charge(frame).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        walker.charged = walker.charged.saturating_add(frame);
        let charged = frame.saturating_add(list_bytes);
        self.reserve_vec(&mut walker.frames, 1)?;
        let pushed = walker.frames.push(Walk {
            holder: enter.holder,
            name: enter.name,
            value: object,
            context,
            record,
            members,
            current: None,
            mark,
            charged,
        });
        if pushed.is_err() {
            return Err(self.fault());
        }
        Ok(())
    }

    /// Step 5 of InternalizeJSONProperty for the member just internalized, newElement: if it
    /// is undefined, ? val.[[Delete]](P), else ? CreateDataProperty(val, P, newElement); a
    /// false answer of either is no error (the note of the clause).
    fn json_internalize_place(&mut self, walker: &mut Walker<'_>, answer: Value) -> Completion<()> {
        let Some(frame) = walker.frames.last() else {
            return Err(self.fault());
        };
        let Some(key) = frame.current else {
            return Err(self.fault());
        };
        let object = frame.value;
        if answer == Value::Undefined {
            self.internal_delete(object, key)?;
        } else {
            self.create_data_property(object, key, answer)?;
        }
        Ok(())
    }

    /// `JSON.rawJSON ( text )` (`#sec-json.rawjson`).
    pub(super) fn json_raw_json(&mut self, args: &[Value]) -> Completion<Value> {
        let mark = self.kept.len();
        let mut charged = 0;
        let result = self.json_raw_json_with(args, &mut charged);
        self.heap.accounting.credit(charged);
        self.release(mark);
        result
    }

    /// The steps of `JSON.rawJSON`; `charged` is what stays charged when it returns.
    fn json_raw_json_with(&mut self, args: &[Value], charged: &mut u64) -> Completion<Value> {
        // 1. Let jsonString be ? ToString(text).
        let string = self.to_string(argument(args, 0))?;
        self.keep(Value::String(string));
        let units = self.json_text_units(string)?;
        // 2. If jsonString is the empty String, throw a SyntaxError exception.
        // 3. The first code unit: an ASCII lowercase letter, an ASCII digit, 0x0022 or 0x002D.
        // 4. The last code unit: an ASCII lowercase letter, an ASCII digit or 0x0022.
        let starts = units
            .first()
            .is_some_and(|&unit| matches!(unit, 0x61..=0x7A | 0x30..=0x39 | 0x22 | 0x2D));
        let ends = units
            .last()
            .is_some_and(|&unit| matches!(unit, 0x61..=0x7A | 0x30..=0x39 | 0x22));
        if !starts || !ends {
            return Err(self.throw_error(
                ErrorKind::SyntaxError,
                "JSON.rawJSON: the text is not one JSON primitive value",
            ));
        }
        // 5. Let parseResult be ? ParseJSON(jsonString).
        let mut parser = Parser {
            text: &units,
            position: 0,
            unpaid: 0,
            stack: Vec::new(),
            records: None,
            charged: 0,
        };
        let parsed = self.json_parse_text(&mut parser);
        *charged = charged.saturating_add(parser.charged);
        // 6. Assert: parseResult.[[Value]] is a String, a Number, a Boolean or null: the first
        // and last code units leave no array or object.
        if let Value::Object(_) = parsed?.value {
            return Err(self.fault());
        }
        // 7, 8. Let obj be OrdinaryObjectCreate(null, « [[IsRawJSON]] »).
        let object = self.ordinary_object_create(None, OrdinaryData::RawJson)?;
        self.keep(Value::Object(object));
        // 9. Perform ! CreateDataPropertyOrThrow(obj, "rawJSON", jsonString).
        let key = self.name_key("rawJSON")?;
        self.create_data_property_or_throw(object, key, Value::String(string))?;
        // 10. Perform ! SetIntegrityLevel(obj, frozen) (`ops/objects.rs`).
        if !self.set_integrity_level(object, IntegrityLevel::Frozen)? {
            return Err(self.fault());
        }
        // 11. Return obj.
        Ok(Value::Object(object))
    }

    /// `JSON.isRawJSON ( obj )` (`#sec-json.israwjson`): 1. If obj is an Object and obj has an
    /// [[IsRawJSON]] internal slot, return true. 2. Return false.
    pub(super) fn json_is_raw_json(&mut self, args: &[Value]) -> Completion<Value> {
        let Value::Object(object) = argument(args, 0) else {
            return Ok(Value::Boolean(false));
        };
        Ok(Value::Boolean(self.json_has_raw_json(object)?))
    }

    /// Whether `object` has an [[IsRawJSON]] internal slot.
    pub(super) fn json_has_raw_json(&mut self, object: ObjectRef) -> Completion<bool> {
        Ok(match &self.engine_object(object)?.kind {
            ObjectKind::Ordinary(ordinary) => ordinary.data == OrdinaryData::RawJson,
            _ => false,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::Termination;
    use crate::builtins::string::transform::tests::{Answer, Counting, LONG};
    use crate::host::hooks::test_host::{agent_with_realm, enter};
    use crate::ops::objects::IntegrityLevel;
    use crate::value::Value;
    use alloc::vec;

    /// `#sec-json.rawjson` step 10: the object `JSON.rawJSON` answers is frozen, by
    /// TestIntegrityLevel (`#sec-testintegritylevel`), and has the slot [[IsRawJSON]].
    #[test]
    fn raw_json_object_is_frozen() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let text = agent.new_string_str("\"a\"").unwrap();
        let made = agent.json_raw_json(&[Value::String(text)]).unwrap();
        let Value::Object(object) = made else {
            panic!("{made:?}");
        };
        assert_eq!(
            agent.test_integrity_level(object, IntegrityLevel::Frozen),
            Ok(true)
        );
        assert_eq!(agent.json_has_raw_json(object), Ok(true));
        let plain = agent
            .ordinary_object_create(None, crate::object::ordinary::OrdinaryData::Plain)
            .unwrap();
        assert_eq!(agent.json_has_raw_json(plain), Ok(false));
    }

    /// The steps of an entry below: a tenth of the step per code unit of the long text.
    const PARSE_STEPS: u64 = 100_000;

    /// An agent with the global `t`: "}" and then 999,999 "0", a text that is not JSON at its
    /// first code unit.
    fn invalid_at_its_first_unit() -> Counting {
        let mut counting = Counting::new(1 << 21);
        let mut text = vec![0x30; LONG];
        text[0] = 0x7D;
        counting.string("t", &text);
        counting
    }

    /// `JSON.parse` (`#sec-json.parse`) charges the step per code unit of its text before it
    /// copies the text, so a text that fails at its first unit has paid for all of it: with
    /// 100,000 steps the entry ends at the step budget and no copy of the text was made.
    #[test]
    fn a_text_invalid_at_its_first_unit_pays_for_its_copy() {
        let mut counting = invalid_at_its_first_unit();
        assert_eq!(
            counting.counted(PARSE_STEPS, "JSON.parse(t)"),
            (Answer::Terminated(Termination::StepBudget), 0)
        );
    }

    /// `JSON.rawJSON` (`#sec-json.rawjson`) charges the same before its copy, so a text its
    /// step 3 refuses by the first code unit has paid for all of it.
    #[test]
    fn a_raw_json_text_refused_by_its_first_unit_pays_for_its_copy() {
        let mut counting = invalid_at_its_first_unit();
        assert_eq!(
            counting.counted(PARSE_STEPS, "JSON.rawJSON(t)"),
            (Answer::Terminated(Termination::StepBudget), 0)
        );
    }
}

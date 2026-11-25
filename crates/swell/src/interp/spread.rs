//! The instructions of spread elements (`swell_compiler::bytecode`, `SpreadArray`,
//! `ArrayAppend`, `ArrayGrow`, `CallSpread`, `CallEvalSpread` and `NewSpread`), each in a
//! function of its own that one arm of `interp/dispatch.rs` calls, so the frame of the
//! instruction loop stays as it was. Written by `story:array-destructuring-and-spread`.
//!
//! The design is `docs/design/engine.md`, decision 3, "The iterator of `for`-`of`, of an array
//! pattern and of a spread", item 5: a spread is one instruction that iterates in Rust, the
//! Iterator Record's [[Iterator]] and [[NextMethod]] on the kept stack while `next` runs, one
//! step charged per value beside the steps of `next`; it never closes the iterator, since
//! ArrayAccumulation and ArgumentListEvaluation of a SpreadElement have no IteratorClose
//! (`#sec-runtime-semantics-arrayaccumulation`, `#sec-runtime-semantics-argumentlistevaluation`).
//! A termination ends the run where it is. The Array the values go to is on the operand stack,
//! which the collector scans; a value read is kept until it is defined there.
//!
//! A call with a spread gets its argument list as that Array, built by the same instructions,
//! and reads it into a list charged to the memory budget before it is made, one step per
//! argument; the call then goes the way `Call(n)`, `CallEval(n)` and `New(n)` go.

use super::dispatch::Flow;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::property::Key;
use crate::ops::iteration::IteratorRecord;
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

impl<H: Host> Agent<H> {
    /// `SpreadArray`: GetIterator of the value on top, which stays there, rooted, while the
    /// iterator runs; each value appended to the Array under it; then the value is dropped.
    #[inline(never)]
    pub(super) fn spread_array(&mut self) -> Completion<Flow> {
        let array = self.peek_value(1)?;
        let array = self.expect_object(array)?;
        let value = self.peek_value(0)?;
        let mut record = self.get_iterator(value)?;
        let mark = self.keep(Value::Object(record.iterator));
        self.keep(record.next_method);
        let spread = self.spread_values(array, &mut record);
        self.release(mark);
        spread?;
        self.drop_entries(1)?;
        Ok(Flow::Next)
    }

    /// The values of `record`, from IteratorStepValue (`#sec-iteratorstepvalue`) until it is
    /// done, each defined at the next index of `array` (CreateDataPropertyOrThrow, ArrayAccumulation
    /// step 4 of `SpreadElement`), one step charged before each call of `next`.
    fn spread_values(&mut self, array: ObjectRef, record: &mut IteratorRecord) -> Completion<()> {
        let mut index = self.length_of_array_like(array)?;
        loop {
            self.charge_steps(1)?;
            let Some(value) = self.iterator_step_value(record)? else {
                return Ok(());
            };
            let mark = self.keep(value);
            let defined = self
                .index_key(index)
                .and_then(|key| self.create_data_property_or_throw(array, key, value));
            self.release(mark);
            defined?;
            index += 1.0;
        }
    }

    /// `ArrayAppend`: the value on top defined at the length of the Array under it, then
    /// dropped.
    #[inline(never)]
    pub(super) fn array_append(&mut self) -> Completion<Flow> {
        let array = self.peek_value(1)?;
        let array = self.expect_object(array)?;
        let value = self.peek_value(0)?;
        let index = self.length_of_array_like(array)?;
        let key = self.index_key(index)?;
        self.create_data_property_or_throw(array, key, value)?;
        self.drop_entries(1)?;
        Ok(Flow::Next)
    }

    /// `ArrayGrow(n)`: Set(a, "length", len + n, true) of the Array on top
    /// (ArrayAccumulation of `Elision`).
    #[inline(never)]
    pub(super) fn array_grow(&mut self, count: u32) -> Completion<Flow> {
        let array = self.peek_value(0)?;
        let array = self.expect_object(array)?;
        let length = self.length_of_array_like(array)? + f64::from(count);
        let key = self.name_key("length")?;
        self.set(array, key, Value::Number(length), true)?;
        Ok(Flow::Next)
    }

    /// The elements of the argument Array `list` a spread built, read into a list whose bytes
    /// are charged before it is made, one step per element; and those bytes, which the caller
    /// credits when the call no longer needs the list.
    fn spread_arguments(&mut self, list: Value) -> Completion<(Vec<Value>, u64)> {
        let list = self.expect_object(list)?;
        let length = self.length_of_array_like(list)?;
        let Ok(count) = u32::try_from(length as u64) else {
            return Err(self.throw_error(ErrorKind::RangeError, "too many arguments"));
        };
        self.charge_steps(u64::from(count))?;
        let size = u64::try_from(size_of::<Value>()).unwrap_or(u64::MAX);
        let bytes = u64::from(count).saturating_mul(size);
        self.charge_bytes(bytes)?;
        let mut args = Vec::new();
        if args
            .try_reserve_exact(usize::try_from(count).unwrap_or(usize::MAX))
            .is_err()
        {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        for index in 0..count {
            match self.get(list, Key::Index(index)) {
                Ok(value) => args.push(value),
                Err(abrupt) => {
                    drop(args);
                    self.heap.accounting.credit(bytes);
                    return Err(abrupt);
                }
            }
        }
        Ok((args, bytes))
    }

    /// `CallSpread`: `Call(n)` of `[this, f, args]` with the elements of `args`.
    #[inline(never)]
    pub(super) fn call_spread(&mut self) -> Completion<Flow> {
        let list = self.peek_value(0)?;
        let (args, bytes) = self.spread_arguments(list)?;
        let called = self.call_with_list(&args);
        drop(args);
        self.heap.accounting.credit(bytes);
        called
    }

    /// The call of `[this, f, args]` with `args`, the list read from the Array: as
    /// `Call(n)`, its three entries replaced.
    fn call_with_list(&mut self, args: &[Value]) -> Completion<Flow> {
        let this = self.peek_value(2)?;
        let function = self.peek_value(1)?;
        let Value::Object(object) = function else {
            return Err(self.throw_error(ErrorKind::TypeError, "the value is not a function"));
        };
        if !self.has_call(object)? {
            return Err(self.throw_error(ErrorKind::TypeError, "the object is not a function"));
        }
        if self.is_ecmascript_function(object)? {
            return self.push_call_frame(object, this, args, 1);
        }
        let result = self.internal_call(object, this, args);
        let result = self.check_interrupt_after(result)?;
        self.replace(3, result)
    }

    /// `CallEvalSpread`: as `CallEval(n)`, with the elements of `args`: when the function is
    /// `%eval%` of the current realm, the direct `eval` of the first element, or `undefined`
    /// when there is none; otherwise `CallSpread`.
    #[inline(never)]
    pub(super) fn call_eval_spread(&mut self, strict: bool) -> Completion<Flow> {
        let function = self.peek_value(1)?;
        let realm = self.top_frame()?.realm;
        let eval = self.intrinsic(realm, Intrinsic::Eval)?;
        if function != Value::Object(eval) {
            return self.call_spread();
        }
        let list = self.peek_value(0)?;
        let (args, bytes) = self.spread_arguments(list)?;
        let source = args.first().copied();
        drop(args);
        self.heap.accounting.credit(bytes);
        let Some(source) = source else {
            return self.replace(3, Value::Undefined);
        };
        self.drop_entries(3)?;
        // As for `CallEval(n)`: the source stays held by the activation the hooks run under,
        // then by the eval code's frame.
        let keep = self.keep(source);
        let answer = self.direct_eval(eval, source, strict);
        self.release(keep);
        if let Some(value) = answer? {
            self.push_value(value)?;
        }
        Ok(Flow::Next)
    }

    /// `NewSpread`: `New(n)` of `[f, args]` with the elements of `args`.
    #[inline(never)]
    pub(super) fn new_spread(&mut self) -> Completion<Flow> {
        let list = self.peek_value(0)?;
        let (args, bytes) = self.spread_arguments(list)?;
        let made = self.construct_with_list(&args);
        drop(args);
        self.heap.accounting.credit(bytes);
        made
    }

    /// The construction of `[f, args]` with `args`: as `New(n)`, its two entries replaced.
    fn construct_with_list(&mut self, args: &[Value]) -> Completion<Flow> {
        let constructor = self.peek_value(1)?;
        if !self.is_constructor(constructor)? {
            return Err(self.throw_error(ErrorKind::TypeError, "the value is not a constructor"));
        }
        let object = self.expect_object(constructor)?;
        if self.is_ecmascript_function(object)? {
            return self.push_construct_frame(object, args, 1);
        }
        let result = self.construct_value(object, args, None);
        let result = self.check_interrupt_after(result)?;
        self.replace(2, result)
    }
}

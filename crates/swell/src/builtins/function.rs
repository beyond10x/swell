//! Function objects (ECMA-262 20.2, `#sec-function-objects`): the methods of
//! `%Function.prototype%` the harness files call, `Function.prototype.call`
//! (`#sec-function.prototype.call`), `Function.prototype.bind` (`#sec-function.prototype.bind`)
//! and `Function.prototype[@@hasInstance]` (`#sec-function.prototype-%symbol.hasinstance%`),
//! which `InstanceofOperator` reads first (`#sec-instanceofoperator`). The bound function
//! exotic object `bind` makes is `object/bound_function.rs`. Written by
//! `story:harness-builtins`. The `Function` constructor (`#sec-function-constructor`), given
//! to slice 2 by decision (`AGENTS.md`), with CreateDynamicFunction in `dynamic_code.rs`, was
//! added by `story:eval-and-function-constructor`. `story:function-apply-and-source-text` adds
//! the rest of slice 3:
//!
//! - `Function.prototype.apply` (`#sec-function.prototype.apply`) with CreateListFromArrayLike
//!   (`#sec-createlistfromarraylike`): a step per element; each element is kept on the kept
//!   stack across the next Get and charged to the memory budget as it is read, for its kept
//!   slot and its slot of the argument list, until the call returns. A length that passes what
//!   the budgets allow ends in `Terminated(StepBudget)` or `Terminated(MemoryBudget)`.
//! - `Function.prototype.toString` (`#sec-function.prototype.tostring`): for an ECMAScript
//!   function for which HostHasSourceTextAvailable (`#sec-hosthassourcetextavailable`) answers
//!   true, its [[SourceText]], the span its template carries out of the source kept with its
//!   compiled code (`script.rs`; `docs/design/engine.md`, decision 4, "Source text."); for a
//!   built-in function, host functions included, `function <[[InitialName]]>() { [native code]
//!   }`; for any other function object, a bound one or an ECMAScript one whose text the host
//!   withholds, `function () { [native code] }`: each a String with the syntax of a
//!   NativeFunction (`prod-NativeFunction`), made by a `StringBuilder`, so bounded by
//!   `string_length` and a step per code unit.
//!
//! The work of each is a function of its own, not inlined into the dispatch, whose frame every
//! re-entry through a builtin of this module pays.

use alloc::rc::Rc;
use alloc::vec::Vec;

use crate::builtins::object::argument;
use crate::builtins::string::StringBuilder;
use crate::host::agent::Agent;
use crate::host::outcome::{Completion, Termination};
use crate::host::{ErrorKind, HookCx, Host};
use crate::object::ObjectKind;
use crate::object::bound_function::bound_function_create;
use crate::object::builtin_function::{BuiltinId, KeptRoom};
use crate::object::property::{Key, Property};
use crate::realm::intrinsics::Making;
use crate::script::{Compiled, vec_bytes};
use crate::value::{ObjectRef, StringRef, Value};

/// What `Function.prototype.toString` reads of a function object.
enum Shape {
    /// An ECMAScript function, its compiled code and the start and end of its [[SourceText]] in
    /// the source kept with the code.
    Ecmascript(ObjectRef, Rc<Compiled>, u32, u32),
    /// A built-in function object and its [[InitialName]].
    Builtin(Option<StringRef>),
    /// Any other object.
    Other,
}

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FunctionBuiltin {
    /// `Function.prototype.bind ( thisArg, ...args )` (20.2.3.2).
    Bind,
    /// `Function.prototype.call ( thisArg, ...args )` (20.2.3.3).
    Call,
    /// `Function.prototype [ @@hasInstance ] ( V )` (20.2.3.6).
    HasInstance,
    /// `Function ( ...parameterArgs, bodyArg )` (20.2.1.1).
    Constructor,
    /// `Function.prototype.apply ( thisArg, argArray )` (20.2.3.1).
    Apply,
    /// `Function.prototype.toString ( )` (20.2.3.5).
    ToString,
}

impl<H: Host> Agent<H> {
    /// `%Function%` (20.2.1, 20.2.2): a built-in constructor of length 1 whose [[Prototype]] is
    /// `%Function.prototype%`, with "prototype" `%Function.prototype%` ({ [[Writable]]: false,
    /// [[Enumerable]]: false, [[Configurable]]: false }, 20.2.2.1) and
    /// `Function.prototype.constructor` (20.2.3.4).
    pub(crate) fn create_function_constructor(&mut self, making: Making) -> Completion<ObjectRef> {
        self.create_builtin_constructor(
            making,
            "Function",
            1,
            BuiltinId::Function(FunctionBuiltin::Constructor),
            making.function_prototype,
        )
    }

    /// `bind`, `call` and `[@@hasInstance]` of `%Function.prototype%`, then `apply` (20.2.3.1,
    /// length 2) and `toString` (20.2.3.5, length 0), in the order of their clauses.
    pub(crate) fn install_function_prototype_methods(&mut self, making: Making) -> Completion<()> {
        let prototype = making.function_prototype;
        self.define_builtin_method(
            making,
            prototype,
            "bind",
            1,
            BuiltinId::Function(FunctionBuiltin::Bind),
        )?;
        self.define_builtin_method(
            making,
            prototype,
            "call",
            1,
            BuiltinId::Function(FunctionBuiltin::Call),
        )?;
        // 20.2.3.6: named "[Symbol.hasInstance]" (SetFunctionName of a Symbol), length 1, with
        // { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false }.
        let has_instance = self.create_builtin(
            making,
            "[Symbol.hasInstance]",
            1,
            BuiltinId::Function(FunctionBuiltin::HasInstance),
        )?;
        self.add_property(
            prototype,
            Key::Symbol(making.symbols.has_instance),
            Property::data(Value::Object(has_instance), false, false, false),
        )?;
        self.define_builtin_method(
            making,
            prototype,
            "apply",
            2,
            BuiltinId::Function(FunctionBuiltin::Apply),
        )?;
        self.define_builtin_method(
            making,
            prototype,
            "toString",
            0,
            BuiltinId::Function(FunctionBuiltin::ToString),
        )?;
        Ok(())
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_function_builtin(
        &mut self,
        id: FunctionBuiltin,
        function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            // `#sec-function-p1-p2-pn-body` with CreateDynamicFunction (`dynamic_code.rs`).
            FunctionBuiltin::Constructor => self.function_constructor(function, args, new_target),
            FunctionBuiltin::Bind => self.function_bind(this, args),
            FunctionBuiltin::Call => self.function_call(this, args),
            // "Let F be the this value. Return ? OrdinaryHasInstance(F, V)."
            FunctionBuiltin::HasInstance => Ok(Value::Boolean(
                self.ordinary_has_instance(this, argument(args, 0))?,
            )),
            FunctionBuiltin::Apply => self.function_apply(this, args),
            FunctionBuiltin::ToString => self.function_to_string(this),
        }
    }

    /// `Function.prototype.apply ( thisArg, argArray )` (`#sec-function.prototype.apply`). The
    /// list is kept in a room of the kept stack from its first element until the call returns,
    /// then released and credited.
    #[inline(never)]
    fn function_apply(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let func be the this value. 2. If IsCallable(func) is false, throw a TypeError.
        if !self.is_callable(this)? {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Function.prototype.apply called on a value that is not a function",
            ));
        }
        let this_arg = argument(args, 0);
        let arg_array = argument(args, 1);
        // 3. If argArray is either undefined or null: PrepareForTailCall() (no frame to
        // release here), then return ? Call(func, thisArg).
        if matches!(arg_array, Value::Undefined | Value::Null) {
            return self.call_value(this, this_arg, &[]);
        }
        // 4. Let argList be ? CreateListFromArrayLike(argArray). 5. PrepareForTailCall().
        // 6. Return ? Call(func, thisArg, argList).
        // The room is ended on every way out, as `Agent::with_kept_room` ends it; written out
        // here so that no closure's frame stands between this call and the callee's re-entry.
        let mark = self.kept.len();
        let mut room = self.kept_room();
        let mut charged = 0;
        let result = match self.create_list_from_array_like(arg_array, &mut charged, &mut room) {
            Ok(list) => self.call_value(this, this_arg, &list),
            Err(abrupt) => Err(abrupt),
        };
        self.end_kept_room(mark, &room);
        self.heap.accounting.credit(charged);
        result
    }

    /// CreateListFromArrayLike (`#sec-createlistfromarraylike`) of `value`, validElementTypes
    /// all: each element is pushed on the kept stack as it is read, in `room`, and stays there
    /// until the caller releases it; `charged` counts the bytes charged for the elements' slots
    /// of the list, which the caller credits.
    #[inline(never)]
    pub(crate) fn create_list_from_array_like(
        &mut self,
        value: Value,
        charged: &mut u64,
        room: &mut KeptRoom,
    ) -> Completion<Vec<Value>> {
        // 1. validElementTypes is all. 2. If obj is not an Object, throw a TypeError.
        let Value::Object(object) = value else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "CreateListFromArrayLike called on a value that is not an object",
            ));
        };
        // 3. Let len be ? LengthOfArrayLike(obj).
        let length = self.length_of_array_like(object)?;
        let start = self.kept.len();
        // 4. Let list be a new empty List. 5. Let index be 0. 6. Repeat, while index < len.
        let mut index = 0.0;
        while index < length {
            self.charge_steps(1)?;
            // 6.a. Let indexName be ! ToString(𝔽(index)). 6.b. Let next be ? Get(obj, indexName).
            let key = self.index_key(index)?;
            let next = self.get(object, key)?;
            // 6.c does not apply to all. 6.d. Append next to list: kept in the room, and charged
            // for its slot of the argument list.
            let bytes = vec_bytes::<Value>(1);
            if self.heap.accounting.charge(bytes).is_err() {
                return Err(self.terminate(Termination::MemoryBudget));
            }
            *charged = charged.saturating_add(bytes);
            self.reserve_kept(room, 1)?;
            self.keep(next);
            // 6.e. Set index to index + 1.
            index += 1.0;
        }
        // 7. Return list.
        let mut list = Vec::new();
        #[expect(
            clippy::disallowed_methods,
            reason = "charged by the loop above, a `Value` per element into `charged`, which the caller credits"
        )]
        if list
            .try_reserve_exact(self.kept.len().saturating_sub(start))
            .is_err()
        {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        list.extend_from_slice(self.kept.get(start..).unwrap_or_default());
        Ok(list)
    }

    /// What `Function.prototype.toString` reads of its this value.
    fn shape_of(&mut self, this: Value) -> Completion<Shape> {
        let Value::Object(func) = this else {
            return Ok(Shape::Other);
        };
        let shape = match &self.engine_object(func)?.kind {
            ObjectKind::EcmascriptFunction(function) => {
                function.compiled.as_ref().and_then(|compiled| {
                    let position = usize::try_from(function.template).unwrap_or(usize::MAX);
                    let span = compiled.code.functions.get(position)?.source_text;
                    Some(Shape::Ecmascript(
                        func,
                        Rc::clone(compiled),
                        span.start,
                        span.end,
                    ))
                })
            }
            ObjectKind::BuiltinFunction(builtin) => Some(Shape::Builtin(builtin.initial_name)),
            _ => Some(Shape::Other),
        };
        match shape {
            Some(shape) => Ok(shape),
            None => Err(self.fault()),
        }
    }

    /// `Function.prototype.toString ( )` (`#sec-function.prototype.tostring`).
    #[inline(never)]
    fn function_to_string(&mut self, this: Value) -> Completion<Value> {
        // 1. Let func be the this value.
        let shape = self.shape_of(this)?;
        let mut builder = StringBuilder::new();
        match shape {
            // 2. If func is an Object, func has a [[SourceText]] internal slot, func.[[SourceText]]
            // is a sequence of Unicode code points, and HostHasSourceTextAvailable(func) is true,
            // return CodePointsToString(func.[[SourceText]]).
            Shape::Ecmascript(func, compiled, start, end) => {
                if self.source_text_available(func)? {
                    let span = usize::try_from(start).unwrap_or(usize::MAX)
                        ..usize::try_from(end).unwrap_or(usize::MAX);
                    let Some(text) = compiled.source.get(span) else {
                        return Err(self.fault());
                    };
                    self.append_units(&mut builder, text)?;
                } else {
                    // 4: a function object whose text the host withholds.
                    self.append_str(&mut builder, "function () { [native code] }")?;
                }
            }
            // 3. If func is a built-in function object, return a NativeFunction whose
            // NativeFunctionAccessor opt PropertyName part is func.[[InitialName]].
            Shape::Builtin(name) => {
                self.append_str(&mut builder, "function ")?;
                if let Some(name) = name {
                    self.append_string(&mut builder, name)?;
                }
                self.append_str(&mut builder, "() { [native code] }")?;
            }
            // 4. If func is an Object and IsCallable(func) is true, return a NativeFunction.
            Shape::Other if self.is_callable(this)? => {
                self.append_str(&mut builder, "function () { [native code] }")?;
            }
            // 5. Throw a TypeError exception.
            Shape::Other => {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "Function.prototype.toString called on a value that is not a function",
                ));
            }
        }
        let text = self.finish_string(builder)?;
        Ok(Value::String(text))
    }

    /// HostHasSourceTextAvailable(`func`) (`#sec-hosthassourcetextavailable`): the host's
    /// answer, through a `HookCx`, which runs no code; a termination of the run while the hook
    /// ran is the answer.
    fn source_text_available(&mut self, func: ObjectRef) -> Completion<bool> {
        let start = self.scope.len();
        let answer = H::host_has_source_text_available(&mut HookCx { agent: self }, func);
        self.scope.truncate(start);
        self.running()?;
        Ok(answer)
    }

    /// `Function.prototype.call ( thisArg, ...args )` (`#sec-function.prototype.call`). The
    /// arguments are held by this builtin's activation while the call runs.
    fn function_call(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let func be the this value. 2. If IsCallable(func) is false, throw a TypeError.
        if !self.is_callable(this)? {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Function.prototype.call called on a value that is not a function",
            ));
        }
        // 3. Perform PrepareForTailCall(): no frame to release here. 4. Return ? Call(func,
        // thisArg, args).
        let rest = args.get(1..).unwrap_or(&[]);
        self.call_value(this, argument(args, 0), rest)
    }

    /// `Function.prototype.bind ( thisArg, ...args )` (`#sec-function.prototype.bind`).
    fn function_bind(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let Target be the this value. 2. If IsCallable(Target) is false, throw a TypeError.
        let target = match this {
            Value::Object(target) if self.is_callable(this)? => target,
            _ => {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "Function.prototype.bind called on a value that is not a function",
                ));
            }
        };
        // 3. Let F be ? BoundFunctionCreate(Target, thisArg, args).
        let rest = args.get(1..).unwrap_or(&[]);
        let bound = bound_function_create(self, target, argument(args, 0), rest)?;
        // Steps 4 to 9 read "length" and "name" of the target, which may run code: F is kept.
        let keep = self.keep(Value::Object(bound));
        let done = self.bind_length_and_name(bound, target, rest.len());
        self.release(keep);
        done?;
        // 10. Return F.
        Ok(Value::Object(bound))
    }

    /// Steps 4 to 9 of `bind`: SetFunctionLength (`#sec-setfunctionlength`) and SetFunctionName
    /// (`#sec-setfunctionname`) with the prefix "bound".
    fn bind_length_and_name(
        &mut self,
        bound: ObjectRef,
        target: ObjectRef,
        bound_count: usize,
    ) -> Completion<()> {
        // 4. Let L be 0.
        let mut length = 0.0;
        // 5. Let targetHasLength be ? HasOwnProperty(Target, "length").
        let key = self.name_key("length")?;
        if self.has_own_property(target, key)? {
            // 6.a. Let targetLen be ? Get(Target, "length").
            let key = self.name_key("length")?;
            // 6.b. If targetLen is a Number, then
            if let Value::Number(target_length) = self.get(target, key)? {
                // 6.b.i. Let targetLenAsInt be ! ToIntegerOrInfinity(targetLen).
                let as_int = self.to_integer_or_infinity(Value::Number(target_length))?;
                length = if as_int == f64::INFINITY {
                    // 6.b.ii. +∞ gives +∞.
                    f64::INFINITY
                } else if as_int == f64::NEG_INFINITY {
                    // 6.b.iii. -∞ gives 0.
                    0.0
                } else {
                    // 6.b.iv. max(targetLenAsInt - argCount, 0). The count is far below 2^53.
                    (as_int - bound_count as f64).max(0.0)
                };
            }
        }
        // 7. Perform SetFunctionLength(F, L): { [[Writable]]: false, [[Enumerable]]: false,
        // [[Configurable]]: true }.
        let key = self.name_key("length")?;
        self.add_property(
            bound,
            key,
            Property::data(Value::Number(length), false, false, true),
        )?;
        // 8. Let targetName be ? Get(Target, "name"). If targetName is not a String, set
        // targetName to the empty String.
        let key = self.name_key("name")?;
        let target_name = self.get(target, key)?;
        // 9. Perform SetFunctionName(F, targetName, "bound"): "bound", a space, and the name,
        // a String made from another value, so it is bounded by string_length.
        let mut builder = StringBuilder::new();
        self.append_str(&mut builder, "bound ")?;
        if let Value::String(name) = target_name {
            self.append_string(&mut builder, name)?;
        }
        let name = self.finish_string(builder)?;
        let key = self.name_key("name")?;
        self.add_property(
            bound,
            key,
            Property::data(Value::String(name), false, false, true),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm, enter};
    use crate::host::outcome::{Abrupt, Completion};
    use crate::host::{Cx, HostCall};
    use crate::object::ordinary::OrdinaryData;
    use crate::object::property::Key;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::Value;

    /// A constructor that is never called.
    fn constructor(_cx: &mut Cx<'_, NoHooks>, call: HostCall<'_>) -> Completion<Value> {
        Ok(Value::Object(call.callee))
    }

    /// `#sec-function.prototype-%symbol.hasinstance%` with OrdinaryHasInstance
    /// (`#sec-ordinaryhasinstance`), step 2: for a bound function, InstanceofOperator of its
    /// target (`#sec-instanceofoperator`), through a chain of bound functions too, in a loop.
    /// An object whose chain holds the target's "prototype" is an instance; another is not; a
    /// this value that is not callable answers false; a bound function whose target's
    /// "prototype" is not an object throws a TypeError. The property is { [[Writable]]: false,
    /// [[Enumerable]]: false, [[Configurable]]: false }. The method is read through
    /// @@hasInstance, which the host interface does not hand out, so this is a unit test.
    #[test]
    fn has_instance_of_a_bound_function() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let function_prototype = agent
            .intrinsic(handle, Intrinsic::FunctionPrototype)
            .unwrap();
        let has_instance_key = Key::Symbol(agent.well_known().unwrap().has_instance);
        let property = agent
            .get_own_property(function_prototype, has_instance_key)
            .unwrap()
            .unwrap();
        assert!(!property.enumerable && !property.configurable);
        let crate::object::property::Slot::Data {
            value: has_instance,
            writable: false,
        } = property.slot
        else {
            panic!("{property:?}");
        };

        let target = agent
            .create_host_function(handle, "F", 0, constructor, true, None)
            .unwrap();
        let prototype = agent
            .ordinary_object_create(None, OrdinaryData::Plain)
            .unwrap();
        let prototype_key = agent.name_key("prototype").unwrap();
        agent
            .create_data_property(target, prototype_key, Value::Object(prototype))
            .unwrap();
        let instance = agent
            .ordinary_object_create(Some(prototype), OrdinaryData::Plain)
            .unwrap();
        let other = agent
            .ordinary_object_create(None, OrdinaryData::Plain)
            .unwrap();
        // Nothing but these locals holds the objects: they are kept, as a builtin keeps what it
        // holds across a call (`object/builtin_function.rs`), since every call below is a safe
        // point that may collect.
        for object in [target, instance, other] {
            agent.keep(Value::Object(object));
        }
        let bind_key = agent.name_key("bind").unwrap();
        let bind = agent.get(function_prototype, bind_key).unwrap();
        let bound = agent
            .call_value(bind, Value::Object(target), &[Value::Null])
            .unwrap();
        agent.keep(bound);
        let twice = agent.call_value(bind, bound, &[Value::Null]).unwrap();
        agent.keep(twice);
        for (this, value, expected) in [
            (bound, Value::Object(instance), true),
            (twice, Value::Object(instance), true),
            (bound, Value::Object(other), false),
            (twice, Value::Number(1.0), false),
            (Value::Object(other), Value::Object(instance), false),
            (Value::Number(1.0), Value::Object(instance), false),
        ] {
            assert_eq!(
                agent.call_value(has_instance, this, &[value]),
                Ok(Value::Boolean(expected)),
                "{this:?} {value:?}"
            );
        }
        agent
            .create_data_property(target, prototype_key, Value::Number(1.0))
            .unwrap();
        assert!(matches!(
            agent.call_value(has_instance, twice, &[Value::Object(instance)]),
            Err(Abrupt::Throw(_))
        ));
    }
}

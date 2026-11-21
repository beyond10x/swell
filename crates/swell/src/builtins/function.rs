//! Function objects (ECMA-262 20.2, `#sec-function-objects`): the methods of
//! `%Function.prototype%` the harness files call, `Function.prototype.call`
//! (`#sec-function.prototype.call`), `Function.prototype.bind` (`#sec-function.prototype.bind`)
//! and `Function.prototype[@@hasInstance]` (`#sec-function.prototype-%symbol.hasinstance%`),
//! which `InstanceofOperator` reads first (`#sec-instanceofoperator`). The bound function
//! exotic object `bind` makes is `object/bound_function.rs`. Written by
//! `story:harness-builtins`. The `Function` constructor (`#sec-function-constructor`), given
//! to slice 2 by decision (`AGENTS.md`), with CreateDynamicFunction in `dynamic_code.rs`, was
//! added by `story:eval-and-function-constructor`; the other methods are slice 3.

use crate::builtins::object::argument;
use crate::builtins::string::StringBuilder;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::bound_function::bound_function_create;
use crate::object::builtin_function::BuiltinId;
use crate::object::property::{Key, Property};
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, Value};

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

    /// `bind`, `call` and `[@@hasInstance]` of `%Function.prototype%`.
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
        )
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
        }
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

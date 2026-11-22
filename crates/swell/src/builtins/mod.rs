//! The builtins, one module per clause of ECMA-262 (`docs/design/engine.md`, decision 2). Slice
//! 1 has the ones the harness files call: `global`, `error`, `boolean` and `number` are
//! `story:object-model-and-realms`; `object`, `function`, `array`, `string`, `math` and `json`
//! are `story:harness-builtins`. Slice 2 takes parts of later slices early
//! (`docs/design/engine.md`, section 12, "A part before its slice";
//! `story:number-boolean-and-global-functions-early`): the `Number` constructor with its value
//! properties, `valueOf` and `toString` in `number`, all of `Boolean` in `boolean`, `isNaN` and
//! `isFinite` in `global`, and `String.prototype.toString` and `valueOf` in `string`. Later
//! slices add their files here.
//!
//! `array`, `object` and `string` are directories (`story:builtin-group-modules`): `mod.rs`
//! holds the constructor and the builtins of the base, and each group of builtins a story of
//! slice 3 delivers has a file of its own beside it (`array/iteration.rs`, `array/search.rs`,
//! `array/mutation.rs`, `object/integrity.rs`, `object/enumeration.rs`, `string/access.rs`,
//! `string/transform.rs`). A group file holds the id type of its builtins, the lists of the
//! methods it defines on the constructor and on the prototype ([`Method`]), the data
//! properties keyed by a well-known symbol it defines on the prototype ([`SymbolData`]) and
//! the function that runs one of its builtins; `mod.rs` reads the lists after its own
//! properties and calls the run function, so a builtin of a group is one variant, one entry of
//! a list and one arm, all in the group's file. On each object the keys of the base come
//! first, then those of the groups in the order `mod.rs` reads them, inside a group in the
//! order of the clauses. The paths `crate::builtins::array::ArrayBuiltin`,
//! `crate::builtins::object::ObjectBuiltin` and `crate::builtins::string::StringBuiltin` are
//! as before.

pub(crate) mod array;
pub(crate) mod boolean;
pub(crate) mod error;
pub(crate) mod function;
pub(crate) mod global;
pub(crate) mod json;
pub(crate) mod math;
pub(crate) mod number;
pub(crate) mod object;
pub(crate) mod string;

use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::object::builtin_function::BuiltinId;
use crate::object::property::{Key, Property};
use crate::realm::WellKnown;
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, SymbolRef, Value};

/// A method a group defines: its name, its "length" and the id of its steps.
pub(crate) type Method<Id> = (&'static str, u32, Id);

/// A data property keyed by a well-known symbol that a group defines, as
/// `Array.prototype[%Symbol.unscopables%]` (`#sec-array.prototype-%symbol.unscopables%`) is:
/// the symbol of its key, the steps that make its value in the realm being made, and its
/// attributes.
pub(crate) struct SymbolData<H: Host> {
    /// The well-known symbol of the key.
    pub(crate) symbol: fn(&WellKnown) -> SymbolRef,
    /// The steps that make the value; they run while the realm is made, before any code.
    pub(crate) value: fn(&mut Agent<H>, Making) -> Completion<Value>,
    pub(crate) writable: bool,
    pub(crate) enumerable: bool,
    pub(crate) configurable: bool,
}

impl<H: Host> Agent<H> {
    /// Define the methods of a group on `object`, in the order of `methods`, each a built-in
    /// function whose steps are `id` of the method's id (clause 18: { [[Writable]]: true,
    /// [[Enumerable]]: false, [[Configurable]]: true }).
    pub(crate) fn define_group_methods<Id: Copy>(
        &mut self,
        making: Making,
        object: ObjectRef,
        methods: &[Method<Id>],
        id: fn(Id) -> BuiltinId,
    ) -> Completion<()> {
        for &(name, length, method) in methods {
            self.define_builtin_method(making, object, name, length, id(method))?;
        }
        Ok(())
    }

    /// Define the data properties keyed by a well-known symbol of a group on `object`, in the
    /// order of `properties`. The object is new and no code runs, so each is added as given;
    /// a value is defined before the next is made, with no safe point between.
    pub(crate) fn define_symbol_data(
        &mut self,
        making: Making,
        object: ObjectRef,
        properties: &[SymbolData<H>],
    ) -> Completion<()> {
        for property in properties {
            let value = (property.value)(self, making)?;
            self.add_property(
                object,
                Key::Symbol((property.symbol)(&making.symbols)),
                Property::data(
                    value,
                    property.writable,
                    property.enumerable,
                    property.configurable,
                ),
            )?;
        }
        Ok(())
    }
}

/// What the tests of the group modules share: the check of the order of keys of an object with
/// groups.
#[cfg(test)]
pub(crate) mod group_testing {
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::NoHooks;
    use crate::host::outcome::Abrupt;
    use crate::object::ObjectKind;
    use crate::object::builtin_function::{Behaviour, BuiltinId};
    use crate::object::property::{Key, Slot};
    use crate::value::{ObjectRef, Value};

    use super::Method;

    /// A method of a group: its name, its "length" and its id as a `BuiltinId`.
    pub(crate) type GroupMethod = (&'static str, u32, BuiltinId);

    /// The methods of a group's list, with their ids made a `BuiltinId` by `id`.
    pub(crate) fn group_methods<Id: Copy>(
        methods: &[Method<Id>],
        id: fn(Id) -> BuiltinId,
    ) -> Vec<GroupMethod> {
        methods
            .iter()
            .map(|&(name, length, method)| (name, length, id(method)))
            .collect()
    }

    /// The text of a String key or value.
    fn text(agent: &mut Agent<NoHooks>, value: Value) -> String {
        let Value::String(string) = value else {
            panic!("{value:?} is not a String");
        };
        String::from_utf16_lossy(&agent.string_units(string).unwrap())
    }

    /// The own String keys of `object` are `base`, in that order, then the names of `groups`,
    /// in that order, and nothing else; each group key is a method (clause 18: writable,
    /// not enumerable, configurable) whose steps are its id, with its "length" and "name", and
    /// a call of it with no this value and no argument ends in a result or a throw, not in a
    /// termination; each of `symbols` is still an own key.
    pub(crate) fn check_group_keys(
        agent: &mut Agent<NoHooks>,
        object: ObjectRef,
        base: &[&str],
        groups: &[GroupMethod],
        symbols: &[Key],
    ) {
        let keys = agent.own_property_keys(object).unwrap().keys;
        let mut names = Vec::new();
        for &key in &keys {
            if matches!(key, Key::Symbol(_)) {
                continue;
            }
            let value = agent.key_value(key).unwrap();
            names.push(text(agent, value));
        }
        let expected: Vec<&str> = base
            .iter()
            .copied()
            .chain(groups.iter().map(|&(name, _, _)| name))
            .collect();
        assert_eq!(names, expected);
        for symbol in symbols {
            assert!(keys.contains(symbol), "{symbol:?}");
        }
        for &(name, length, id) in groups {
            let key = agent.name_key(name).unwrap();
            let property = agent.get_own_property(object, key).unwrap().unwrap();
            let Slot::Data {
                value: Value::Object(function),
                writable: true,
            } = property.slot
            else {
                panic!("{name}: {property:?}");
            };
            assert!(!property.enumerable && property.configurable, "{name}");
            let ObjectKind::BuiltinFunction(builtin) = &agent.engine_object(function).unwrap().kind
            else {
                panic!("{name} is not a built-in function");
            };
            assert!(
                matches!(builtin.behaviour, Behaviour::Builtin(steps) if steps == id),
                "{name}"
            );
            agent.keep(Value::Object(function));
            let length_key = agent.name_key("length").unwrap();
            assert_eq!(
                agent.get(function, length_key),
                Ok(Value::Number(f64::from(length))),
                "{name}"
            );
            let name_key = agent.name_key("name").unwrap();
            let own_name = agent.get(function, name_key).unwrap();
            assert_eq!(text(agent, own_name), name);
            let ran = agent.call_value(Value::Object(function), Value::Undefined, &[]);
            assert!(
                !matches!(ran, Err(Abrupt::Terminated(_))),
                "{name}: {ran:?}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SymbolData;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm, enter};
    use crate::host::outcome::Completion;
    use crate::object::ordinary::OrdinaryData;
    use crate::object::property::{Key, Property, Slot};
    use crate::realm::intrinsics::{Intrinsic, Making};
    use crate::value::Value;

    /// The value of the test group's property: an object with a null prototype whose "at" is
    /// true, as `Array.prototype[%Symbol.unscopables%]` is made.
    fn unscopable_list(agent: &mut Agent<NoHooks>, _making: Making) -> Completion<Value> {
        let list = agent.ordinary_object_create(None, OrdinaryData::Plain)?;
        let key = agent.name_key("at")?;
        agent.create_data_property_or_throw(list, key, Value::Boolean(true))?;
        Ok(Value::Object(list))
    }

    /// The hook of a group for a data property keyed by a well-known symbol (the form
    /// `#sec-array.prototype-%symbol.unscopables%` needs): a test group of two such properties
    /// defines both on an object, in its order, each with the value its steps made and the
    /// attributes it gives, read back by [[GetOwnProperty]].
    #[test]
    fn a_group_installs_a_property_keyed_by_a_well_known_symbol() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let symbols = agent.well_known().unwrap();
        let making = Making {
            realm: handle,
            object_prototype: agent.intrinsic(handle, Intrinsic::ObjectPrototype).unwrap(),
            function_prototype: agent
                .intrinsic(handle, Intrinsic::FunctionPrototype)
                .unwrap(),
            symbols,
        };
        let group: [SymbolData<NoHooks>; 2] = [
            SymbolData {
                symbol: |symbols| symbols.unscopables,
                value: unscopable_list,
                writable: false,
                enumerable: false,
                configurable: true,
            },
            SymbolData {
                symbol: |symbols| symbols.is_concat_spreadable,
                value: |_, _| Ok(Value::Number(1.0)),
                writable: true,
                enumerable: true,
                configurable: false,
            },
        ];
        let object = agent
            .ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)
            .unwrap();
        agent.keep(Value::Object(object));
        agent.define_symbol_data(making, object, &group).unwrap();

        let unscopables = Key::Symbol(symbols.unscopables);
        let spreadable = Key::Symbol(symbols.is_concat_spreadable);
        assert_eq!(
            agent.own_property_keys(object).map(|list| list.keys),
            Ok(vec![unscopables, spreadable])
        );
        let property = agent
            .get_own_property(object, unscopables)
            .unwrap()
            .unwrap();
        let Slot::Data {
            value: Value::Object(list),
            writable: false,
        } = property.slot
        else {
            panic!("{property:?}");
        };
        assert!(!property.enumerable && property.configurable);
        assert_eq!(agent.get_prototype_of(list), Ok(None));
        let at = agent.name_key("at").unwrap();
        assert_eq!(agent.get(list, at), Ok(Value::Boolean(true)));
        assert_eq!(
            agent.get_own_property(object, spreadable),
            Ok(Some(Property::data(Value::Number(1.0), true, true, false)))
        );
    }
}

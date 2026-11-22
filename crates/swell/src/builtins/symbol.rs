//! Symbol objects (ECMA-262 20.4, `#sec-symbol-objects`), written by
//! `story:symbol-constructor-and-well-known-symbols`: the `Symbol` function
//! (`#sec-symbol-description`), `Symbol.for` (`#sec-symbol.for`) and `Symbol.keyFor`
//! (`#sec-symbol.keyfor`, with KeyForSymbol, `#sec-keyforsymbol`) over the agent's
//! GlobalSymbolRegistry (`#sec-globalsymbolregistry-records`; `heap/mod.rs`,
//! `SymbolRegistry`), the well-known symbol properties of the constructor
//! (`#sec-properties-of-the-symbol-constructor`), and `%Symbol.prototype%`
//! (`#sec-properties-of-the-symbol-prototype-object`) with `constructor`, the `description`
//! getter (`#sec-symbol.prototype.description`), `toString` (`#sec-symbol.prototype.tostring`,
//! with SymbolDescriptiveString, `#sec-symboldescriptivestring`), `valueOf`
//! (`#sec-symbol.prototype.valueof`, with ThisSymbolValue, `#sec-thissymbolvalue`),
//! `[%Symbol.toPrimitive%]` (`#sec-symbol.prototype-%symbol.toprimitive%`) and
//! `[%Symbol.toStringTag%]` (`#sec-symbol.prototype-%symbol.tostringtag%`).
//!
//! Twelve of the well-known symbols of Table 1 are properties of the constructor in slice 6;
//! `Symbol.asyncIterator` comes with slice 10 (`docs/design/engine.md`, decision 4, "Symbols").
//! A Symbol object is an ordinary object whose [[SymbolData]] is its Symbol
//! (`#sec-properties-of-symbol-instances`; `OrdinaryData::Symbol`), made by ToObject
//! (`ops/conversion.rs`).

use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::ObjectKind;
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Key, Property, Slot};
use crate::realm::WellKnown;
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, SymbolRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SymbolBuiltin {
    /// `Symbol ( [ description ] )` (20.4.1.1).
    Constructor,
    /// `Symbol.for ( key )` (20.4.2.4).
    For,
    /// `Symbol.keyFor ( symbol )` (20.4.2.8).
    KeyFor,
    /// `get Symbol.prototype.description` (20.4.3.2).
    Description,
    /// `Symbol.prototype.toString ( )` (20.4.3.3).
    ToString,
    /// `Symbol.prototype.valueOf ( )` (20.4.3.4).
    ValueOf,
    /// `Symbol.prototype [ %Symbol.toPrimitive% ] ( hint )` (20.4.3.5).
    ToPrimitive,
}

/// A well-known symbol property of the constructor: its name and its symbol.
type WellKnownProperty = (&'static str, fn(&WellKnown) -> SymbolRef);

/// The well-known symbol properties of the constructor before "keyFor", in the order of 20.4.2
/// (20.4.2.5 to 20.4.2.7).
const BEFORE_KEY_FOR: [WellKnownProperty; 3] = [
    ("hasInstance", |symbols| symbols.has_instance),
    ("isConcatSpreadable", |symbols| symbols.is_concat_spreadable),
    ("iterator", |symbols| symbols.iterator),
];

/// The well-known symbol properties of the constructor after "keyFor", in the order of 20.4.2
/// (20.4.2.9 to 20.4.2.18, "prototype" left out: `create_builtin_constructor` defines it).
const AFTER_KEY_FOR: [WellKnownProperty; 9] = [
    ("match", |symbols| symbols.r#match),
    ("matchAll", |symbols| symbols.match_all),
    ("replace", |symbols| symbols.replace),
    ("search", |symbols| symbols.search),
    ("species", |symbols| symbols.species),
    ("split", |symbols| symbols.split),
    ("toPrimitive", |symbols| symbols.to_primitive),
    ("toStringTag", |symbols| symbols.to_string_tag),
    ("unscopables", |symbols| symbols.unscopables),
];

impl<H: Host> Agent<H> {
    /// `%Symbol%` (20.4.1) and `%Symbol.prototype%` (20.4.3), answered in that order: the
    /// constructor's "length" 0 (20.4.2), its "prototype" { [[Writable]]: false,
    /// [[Enumerable]]: false, [[Configurable]]: false } (20.4.2.11), "for", "keyFor" and the
    /// twelve well-known symbols, each { [[Writable]]: false, [[Enumerable]]: false,
    /// [[Configurable]]: false }; the prototype, an ordinary object whose [[Prototype]] is
    /// `%Object.prototype%` with no [[SymbolData]] (20.4.3), with its properties in the order of
    /// 20.4.3.
    pub(crate) fn create_symbol_constructor(
        &mut self,
        making: Making,
    ) -> Completion<(ObjectRef, ObjectRef)> {
        let prototype =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        let constructor = self.create_builtin_constructor(
            making,
            "Symbol",
            0,
            BuiltinId::Symbol(SymbolBuiltin::Constructor),
            prototype,
        )?;
        self.define_builtin_method(
            making,
            constructor,
            "for",
            1,
            BuiltinId::Symbol(SymbolBuiltin::For),
        )?;
        self.define_well_known_properties(making, constructor, &BEFORE_KEY_FOR)?;
        self.define_builtin_method(
            making,
            constructor,
            "keyFor",
            1,
            BuiltinId::Symbol(SymbolBuiltin::KeyFor),
        )?;
        self.define_well_known_properties(making, constructor, &AFTER_KEY_FOR)?;

        // 20.4.3.2: an accessor whose set accessor function is undefined; the getter is named
        // "get description"; { [[Enumerable]]: false, [[Configurable]]: true } (clause 18).
        let description = self.create_builtin(
            making,
            "get description",
            0,
            BuiltinId::Symbol(SymbolBuiltin::Description),
        )?;
        let key = self.name_key("description")?;
        self.add_property(
            prototype,
            key,
            Property {
                slot: Slot::Accessor {
                    get: Some(description),
                    set: None,
                },
                enumerable: false,
                configurable: true,
            },
        )?;
        self.define_builtin_method(
            making,
            prototype,
            "toString",
            0,
            BuiltinId::Symbol(SymbolBuiltin::ToString),
        )?;
        self.define_builtin_method(
            making,
            prototype,
            "valueOf",
            0,
            BuiltinId::Symbol(SymbolBuiltin::ValueOf),
        )?;
        // 20.4.3.5: named "[Symbol.toPrimitive]", length 1, with { [[Writable]]: false,
        // [[Enumerable]]: false, [[Configurable]]: true }.
        let to_primitive = self.create_builtin(
            making,
            "[Symbol.toPrimitive]",
            1,
            BuiltinId::Symbol(SymbolBuiltin::ToPrimitive),
        )?;
        self.add_property(
            prototype,
            Key::Symbol(making.symbols.to_primitive),
            Property::data(Value::Object(to_primitive), false, false, true),
        )?;
        // 20.4.3.6: the String "Symbol", { [[Writable]]: false, [[Enumerable]]: false,
        // [[Configurable]]: true }.
        self.define_to_string_tag(making, prototype, "Symbol")?;
        Ok((constructor, prototype))
    }

    /// The well-known symbol properties `properties` of the constructor, each { [[Writable]]:
    /// false, [[Enumerable]]: false, [[Configurable]]: false } (20.4.2).
    fn define_well_known_properties(
        &mut self,
        making: Making,
        constructor: ObjectRef,
        properties: &[WellKnownProperty],
    ) -> Completion<()> {
        for &(name, symbol) in properties {
            let key = self.name_key(name)?;
            let value = Value::Symbol(symbol(&making.symbols));
            self.add_property(constructor, key, Property::data(value, false, false, false))?;
        }
        Ok(())
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_symbol_builtin(
        &mut self,
        id: SymbolBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            SymbolBuiltin::Constructor => self.symbol_constructor(args, new_target),
            SymbolBuiltin::For => self.symbol_for(args),
            SymbolBuiltin::KeyFor => self.symbol_key_for(args),
            // `#sec-symbol.prototype.description`: 1. Let s be the this value. 2. Let sym be ?
            // ThisSymbolValue(s). 3. Return sym.[[Description]].
            SymbolBuiltin::Description => {
                let symbol = self.this_symbol_value(this, "Symbol.prototype.description")?;
                Ok(self
                    .symbol_description(symbol)?
                    .map_or(Value::Undefined, Value::String))
            }
            // `#sec-symbol.prototype.tostring`: 1. Let sym be ? ThisSymbolValue(this value).
            // 2. Return SymbolDescriptiveString(sym).
            SymbolBuiltin::ToString => {
                let symbol = self.this_symbol_value(this, "Symbol.prototype.toString")?;
                Ok(Value::String(self.symbol_descriptive_string(symbol)?))
            }
            // `#sec-symbol.prototype.valueof`: 1. Return ? ThisSymbolValue(this value).
            SymbolBuiltin::ValueOf => self
                .this_symbol_value(this, "Symbol.prototype.valueOf")
                .map(Value::Symbol),
            // `#sec-symbol.prototype-%symbol.toprimitive%`: 1. Return ? ThisSymbolValue(this
            // value). The argument is ignored.
            SymbolBuiltin::ToPrimitive => self
                .this_symbol_value(this, "Symbol.prototype [ @@toPrimitive ]")
                .map(Value::Symbol),
        }
    }

    /// `Symbol ( [ description ] )` (`#sec-symbol-description`).
    fn symbol_constructor(
        &mut self,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. If NewTarget is not undefined, throw a TypeError exception.
        if new_target.is_some() {
            return Err(self.throw_error(ErrorKind::TypeError, "Symbol is not a constructor"));
        }
        // 2. If description is undefined, let descString be undefined. 3. Else, let descString
        // be ? ToString(description).
        let description = match argument(args, 0) {
            Value::Undefined => None,
            value => Some(self.to_string(value)?),
        };
        // 4. Return a new Symbol whose [[Description]] is descString. The String is stored in
        // the record before any safe point.
        match self.heap.new_symbol(description) {
            Ok(symbol) => Ok(Value::Symbol(symbol)),
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    /// `Symbol.for ( key )` (`#sec-symbol.for`). A step per code unit of the key, which the
    /// registry hashes and compares, is charged before the lookup (`docs/design/engine.md`,
    /// "Symbols", item 3); a new entry is charged by the heap (`Heap::symbol_for`).
    fn symbol_for(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. Let stringKey be ? ToString(key).
        let key = self.to_string(argument(args, 0))?;
        let length = self.string_length(key)?;
        self.charge_steps(u64::try_from(length).unwrap_or(u64::MAX))?;
        // 2 to 4. For each element e of the GlobalSymbolRegistry, if e.[[Key]] is stringKey,
        // return e.[[Symbol]]. 5 to 7. Otherwise a new Symbol whose [[Description]] is
        // stringKey, appended with it and returned.
        match self.heap.symbol_for(key) {
            Ok(Ok(symbol)) => Ok(Value::Symbol(symbol)),
            Ok(Err(_)) => Err(self.terminate(Termination::MemoryBudget)),
            Err(_) => Err(self.fault()),
        }
    }

    /// `Symbol.keyFor ( symbol )` (`#sec-symbol.keyfor`) with KeyForSymbol
    /// (`#sec-keyforsymbol`): the record of a Symbol `Symbol.for` made says it is registered,
    /// and its [[Description]] is its key (`docs/design/engine.md`, "Symbols", item 2).
    fn symbol_key_for(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If symbol is not a Symbol, throw a TypeError exception.
        let Value::Symbol(symbol) = argument(args, 0) else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Symbol.keyFor requires that its argument be a Symbol",
            ));
        };
        // 2. Return KeyForSymbol(sym).
        let answer = match self.heap.symbols.get(symbol.handle) {
            Ok(record) if record.registered => record.description.map(Value::String),
            Ok(_) => Some(Value::Undefined),
            Err(_) => None,
        };
        answer.ok_or_else(|| self.fault())
    }

    /// The [[Description]] of `symbol`.
    fn symbol_description(
        &mut self,
        symbol: SymbolRef,
    ) -> Completion<Option<crate::value::StringRef>> {
        match self.heap.symbols.get(symbol.handle) {
            Ok(record) => Ok(record.description),
            Err(_) => Err(self.fault()),
        }
    }

    /// ThisSymbolValue ( arg ) (`#sec-thissymbolvalue`), for the method `method`.
    fn this_symbol_value(&mut self, value: Value, method: &str) -> Completion<SymbolRef> {
        match value {
            // 1. If arg is a Symbol, return arg.
            Value::Symbol(symbol) => return Ok(symbol),
            // 2. If arg is an Object and arg has a [[SymbolData]] internal slot, then return
            // arg.[[SymbolData]].
            Value::Object(object) => {
                if let ObjectKind::Ordinary(ordinary) = &self.engine_object(object)?.kind
                    && let OrdinaryData::Symbol(symbol) = ordinary.data
                {
                    return Ok(symbol);
                }
            }
            _ => {}
        }
        // 3. Throw a TypeError exception.
        Err(self.throw_error(
            ErrorKind::TypeError,
            &format!("{method} requires that 'this' be a Symbol"),
        ))
    }
}

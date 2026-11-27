//! Host objects (`docs/design/host-interface.md`, decision 15; `docs/design/engine.md`, decision 9,
//! "Host objects for the DOM"): objects the host makes with a brand (the Web IDL interface they
//! implement), host data of any Rust type that implements [`HostData`], and the host data's
//! `trace`, which the collector calls so that what the data holds stays alive exactly as long
//! as the object. The host data is dropped by the sweep that frees its object.
//!
//! A host object may have an override table, [`Overrides`] (section 15, "overrides";
//! `story:host-object-overrides`): each internal method it gives runs in place of the ordinary
//! one (`object/host_overrides.rs`), and the facts its answers fixed are kept in a box of the
//! object, which its kind site traces.
//!
//! Kind site of the collector: [`HostObject::trace`]. `is_html_dda` (slice 18) is added to
//! [`HostObjectSpec`] by the slice of its requirement. Written by
//! `story:heap-values-and-host-surface`.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::any::Any;

use crate::heap::collect::Mark;
use crate::host::outcome::Completion;
use crate::host::{Cx, ErrorKind, Host, PropertyDescriptor};
use crate::object::host_overrides::Overridden;
use crate::realm::WellKnown;
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, PropertyKey, RealmRef, SymbolRef, Value};

/// The brand of a host object: which Web IDL interface it implements, in the host's numbering
/// (`webidl:is-a-platform-object`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Brand(pub u32);

/// Host data: Rust state a host object owns. `trace` lists every value the data holds, and is
/// called by every collection that reaches the object; a value it does not list may be freed,
/// and a listed handle that is stale ends the collection in `Terminated(EngineFault)`.
pub trait HostData: Any {
    fn trace(&self, tracer: &mut Tracer<'_>);
}

/// What `HostData::trace` reports to: the collector's worklist.
pub struct Tracer<'t> {
    mark: &'t mut Mark,
}

impl Tracer<'_> {
    /// Keep the record `value` holds, if any.
    pub fn value(&mut self, value: Value) {
        self.mark.value(value);
    }

    /// Keep `object`.
    pub fn object(&mut self, object: ObjectRef) {
        self.mark.object(object);
    }
}

/// The [[Prototype]] of a new host object (`docs/design/host-interface.md`, decision 15).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Prototype {
    Object(ObjectRef),
    Null,
    /// The `%Object.prototype%` of that realm, without a call (decision 7).
    ObjectPrototypeOf(RealmRef),
}

/// The intrinsics a host can name (Web IDL's need for its interface objects, prototypes and
/// exceptions); a closed list, grown only by a decision of the design with a changelog line
/// (`docs/design/host-interface.md`, decision 7, "The closed list"). Each is the realm's
/// original object, whatever a script did to the globals since.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HostIntrinsic {
    /// `%Object.prototype%`: the [[Prototype]] of an interface prototype object
    /// (`webidl:interface-prototype-object`).
    ObjectPrototype,
    /// `%Function.prototype%`: the [[Prototype]] of an interface object whose interface has no
    /// parent (`webidl:interface-object`).
    FunctionPrototype,
    /// `%Error.prototype%`: the [[Prototype]] of `DOMException`'s interface prototype object
    /// (`webidl:interface-prototype-object`).
    ErrorPrototype,
    /// `%Array.prototype.values%` (`#sec-array.prototype.values`): the `@@iterator` of an
    /// interface with an indexed property getter, and the `values` of one that also has a value
    /// iterator (`webidl:define-the-iteration-methods`).
    ArrayPrototypeValues,
    /// `%Array.prototype.entries%` (`#sec-array.prototype.entries`): the `entries` of an
    /// interface with an indexed property getter and a value iterator
    /// (`webidl:define-the-iteration-methods`).
    ArrayPrototypeEntries,
    /// `%Array.prototype.keys%` (`#sec-array.prototype.keys`): its `keys`, as `entries`.
    ArrayPrototypeKeys,
    /// `%Array.prototype.forEach%` (`#sec-array.prototype.foreach`): its `forEach`, as
    /// `entries`.
    ArrayPrototypeForEach,
}

impl HostIntrinsic {
    /// The realm's intrinsic (`#sec-well-known-intrinsic-objects`, Table 6) this entry names.
    pub(crate) fn intrinsic(self) -> Intrinsic {
        match self {
            HostIntrinsic::ObjectPrototype => Intrinsic::ObjectPrototype,
            HostIntrinsic::FunctionPrototype => Intrinsic::FunctionPrototype,
            HostIntrinsic::ErrorPrototype => Intrinsic::ErrorPrototype(ErrorKind::Error),
            HostIntrinsic::ArrayPrototypeValues => Intrinsic::ArrayPrototypeValues,
            HostIntrinsic::ArrayPrototypeEntries => Intrinsic::ArrayPrototypeEntries,
            HostIntrinsic::ArrayPrototypeKeys => Intrinsic::ArrayPrototypeKeys,
            HostIntrinsic::ArrayPrototypeForEach => Intrinsic::ArrayPrototypeForEach,
        }
    }
}

/// The well-known symbols a host can name (`#sec-well-known-symbols`), shared by every realm
/// of the agent; a closed list, grown as [`HostIntrinsic`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WellKnownSymbol {
    /// `%Symbol.toStringTag%`: the key of an object's class string (`webidl:dfn-class-string`).
    ToStringTag,
    /// `%Symbol.unscopables%`: the key of the object of an interface's `[Unscopable]` members
    /// (`webidl:interface-prototype-object`).
    Unscopables,
    /// `%Symbol.iterator%`: the key under which an interface with an indexed property getter
    /// gets `%Array.prototype.values%` (`webidl:define-the-iteration-methods`).
    Iterator,
    /// `%Symbol.asyncIterator%`: the key of an interface's asynchronous iteration methods
    /// (`webidl:define-the-asynchronous-iteration-methods`).
    AsyncIterator,
}

impl WellKnownSymbol {
    /// The symbol of `symbols` this entry names.
    pub(crate) fn of(self, symbols: &WellKnown) -> SymbolRef {
        match self {
            WellKnownSymbol::ToStringTag => symbols.to_string_tag,
            WellKnownSymbol::Unscopables => symbols.unscopables,
            WellKnownSymbol::Iterator => symbols.iterator,
            WellKnownSymbol::AsyncIterator => symbols.async_iterator,
        }
    }
}

/// The internal methods a host object overrides (`docs/design/host-interface.md`, section 15):
/// each one given runs in place of the ordinary method of that name, and each one left `None`
/// runs the ordinary method. A table is shared by every object of one interface, so it is
/// `&'static`. Each call is one re-entry and spends a step before it runs; the handles of its
/// answer are checked, then the answer against the invariants of the essential internal
/// methods (`#sec-invariants-of-the-essential-internal-methods`) with the facts the object's
/// earlier answers fixed; a broken one ends the run in `Terminated(EngineFault)`.
#[expect(
    clippy::type_complexity,
    reason = "each field's type as docs/design/host-interface.md section 15 states it; an alias would be a public name the design does not have"
)]
pub struct Overrides<H: Host> {
    pub get_prototype_of: Option<fn(&mut Cx<'_, H>, ObjectRef) -> Completion<Option<ObjectRef>>>,
    pub set_prototype_of:
        Option<fn(&mut Cx<'_, H>, ObjectRef, Option<ObjectRef>) -> Completion<bool>>,
    pub is_extensible: Option<fn(&mut Cx<'_, H>, ObjectRef) -> Completion<bool>>,
    pub prevent_extensions: Option<fn(&mut Cx<'_, H>, ObjectRef) -> Completion<bool>>,
    pub get_own_property: Option<
        fn(&mut Cx<'_, H>, ObjectRef, PropertyKey) -> Completion<Option<PropertyDescriptor>>,
    >,
    pub define_own_property:
        Option<fn(&mut Cx<'_, H>, ObjectRef, PropertyKey, PropertyDescriptor) -> Completion<bool>>,
    pub has_property: Option<fn(&mut Cx<'_, H>, ObjectRef, PropertyKey) -> Completion<bool>>,
    pub get: Option<fn(&mut Cx<'_, H>, ObjectRef, PropertyKey, Value) -> Completion<Value>>,
    pub set: Option<fn(&mut Cx<'_, H>, ObjectRef, PropertyKey, Value, Value) -> Completion<bool>>,
    pub delete: Option<fn(&mut Cx<'_, H>, ObjectRef, PropertyKey) -> Completion<bool>>,
    pub own_property_keys: Option<fn(&mut Cx<'_, H>, ObjectRef) -> Completion<Vec<PropertyKey>>>,
}

/// What `new_host_object` makes.
pub struct HostObjectSpec<H: Host> {
    /// [[Prototype]].
    pub prototype: Prototype,
    pub brand: Brand,
    pub data: Option<Box<dyn HostData>>,
    /// `Some(kind)` gives the object an [[ErrorData]] slot of that kind, as Web IDL gives a
    /// `DOMException` (`webidl:js-DOMException-specialness`): `Error.isError` answers true for
    /// it, `Object.prototype.toString` gives it the builtin tag "Error", and `error_summary`
    /// reads the kind (decisions 13 and 15).
    pub error_kind: Option<ErrorKind>,
    /// The internal methods the object overrides, or `None` for the ordinary ones.
    pub overrides: Option<&'static Overrides<H>>,
}

/// The kind part of a host object's record.
pub(crate) struct HostObject {
    pub(crate) brand: Brand,
    pub(crate) data: Option<Box<dyn HostData>>,
    /// The kind of its [[ErrorData]] slot, when it has one.
    pub(crate) error_kind: Option<ErrorKind>,
}

impl HostObject {
    /// Kind site of the collector: what the host data lists. For an object with an override
    /// table the data is the box of the table ([`Overridden`]), whose `trace` lists the host's
    /// own data and the values, keys and prototype of the facts.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("host/host_object.rs HostObject::trace");
        if let Some(data) = &self.data {
            data.trace(&mut Tracer { mark });
        }
    }

    /// The box of its override table, with the facts and the host's data, when it has a table:
    /// the record of a host object keeps its size (`object/generator.rs`, `RECORD_BYTES`) and
    /// so do the records that keep a [[HostDefined]] as a host object's data.
    pub(crate) fn overridden(&self) -> Option<&Overridden> {
        let data: &dyn Any = self.data.as_deref()?;
        data.downcast_ref::<Overridden>()
    }

    /// As [`HostObject::overridden`], to change.
    pub(crate) fn overridden_mut(&mut self) -> Option<&mut Overridden> {
        let data: &mut dyn Any = self.data.as_deref_mut()?;
        data.downcast_mut::<Overridden>()
    }

    /// The host's data: the data, or the data in the box of the override table.
    fn host_data(&self) -> Option<&dyn HostData> {
        match self.overridden() {
            Some(overridden) => overridden.data(),
            None => self.data.as_deref(),
        }
    }

    /// The data, when the brand is `brand` and the data is a `T`.
    pub(crate) fn data<T: HostData>(&self, brand: Brand) -> Option<&T> {
        if self.brand != brand {
            return None;
        }
        let data: &dyn Any = self.host_data()?;
        data.downcast_ref::<T>()
    }

    /// The data to change, when the brand is `brand` and the data is a `T`.
    pub(crate) fn data_mut<T: HostData>(&mut self, brand: Brand) -> Option<&mut T> {
        if self.brand != brand {
            return None;
        }
        let data: &mut dyn HostData = if self.overridden().is_some() {
            self.overridden_mut()?.data_mut()?
        } else {
            self.data.as_deref_mut()?
        };
        let data: &mut dyn Any = data;
        data.downcast_mut::<T>()
    }

    /// A host object with no data, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> HostObject {
        HostObject {
            brand: Brand(0),
            data: None,
            error_kind: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::object::ordinary::Ordinary;
    use crate::object::{ObjectKind, ObjectRecord};
    use crate::{Agent, Brand, HostData, Limits, Seeds, Tracer, Value};

    use crate::host::hooks::test_host::{NoHooks, new_realm};

    struct Data;

    impl HostData for Data {
        fn trace(&self, _tracer: &mut Tracer<'_>) {}
    }

    /// HOST-108's last case, which no script can make before `story:object-model-and-realms`:
    /// an object that is not a host object has no brand and no host data.
    #[test]
    fn an_object_that_is_not_a_host_object_has_no_brand_and_no_data() {
        let limits = Limits {
            steps_per_entry: 1_000,
            memory_bytes: 1 << 20,
            frame_depth: 100,
            reentry_depth: 10,
            parser_nesting: 100,
            string_length: 1 << 20,
            bigint_bits: 1 << 10,
        };
        let mut agent = Agent::new(
            NoHooks,
            limits,
            Seeds {
                hash_key: [1; 16],
                random_seed: [2; 32],
            },
        )
        .unwrap();
        let record = ObjectRecord::new(None, ObjectKind::Ordinary(Ordinary::for_test()));
        let object = agent.heap.new_object(record, 0).unwrap();
        let _root = agent.heap.roots.add(Value::Object(object));
        let realm = new_realm(&mut agent);
        agent
            .with_cx(&realm, |cx| {
                assert_eq!(cx.brand(object), Ok(None));
                assert!(cx.host_data::<Data>(object, Brand(0)).unwrap().is_none());
                assert!(
                    cx.host_data_mut::<Data>(object, Brand(0))
                        .unwrap()
                        .is_none()
                );
            })
            .unwrap();
    }
}

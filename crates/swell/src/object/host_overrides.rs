//! Host objects with an override table (`docs/design/host-interface.md`, section 15, "Host
//! objects: brand, host data, `trace`, overrides"; `docs/design/engine.md`, decision 9, "Host
//! objects for the DOM"), `story:host-object-overrides`: the arms of the 11 internal methods the
//! dispatch of `object/mod.rs` calls for a host object. Each runs the override of its name where
//! the object's table has one and the ordinary method where it has none (HOST-114).
//!
//! A call of an override: the handles of its answer, in its `Ok` value or its `Abrupt::Throw`
//! value, are checked first (decision 6, "A handle the host returns"; HOST-222), then the answer
//! against the invariants of the essential internal methods
//! (`#sec-invariants-of-the-essential-internal-methods`), read as the clause words them: facts
//! of the object's own earlier answers ("previously observed"), not a live target's state
//! (HOST-112, HOST-113). The engine records, in a box of the object, the facts an invariant holds
//! it to:
//!
//! - each key `get_own_property` reported non-configurable, with its descriptor (it stays: it is
//!   not reported absent, not deleted, `has_property` does not answer false for it, a
//!   non-writable data value does not change, and every key list holds it);
//! - that the object reported itself not extensible (`is_extensible` false or
//!   [[PreventExtensions]] true), then the prototype it reported;
//! - once it is not extensible, each key reported non-existent: absent from `get_own_property`,
//!   deleted by a [[Delete]] that answered true, or left out of a key list. It stays
//!   non-existent: a later descriptor for it, a `define_own_property` that answers true for it,
//!   or a key list that holds it is a broken invariant. Before that report an absent answer
//!   binds nothing, and a key reported present and configurable may be reported absent, deleted
//!   or answered false later, as an ordinary object's. A false from `has_property` records
//!   nothing and breaks an invariant only for a non-configurable property, fixed or of the
//!   record, whether or not the object is extensible: the clause binds [[HasProperty]] only for
//!   a key observed non-configurable;
//! - once it is not extensible, the keys of the first key list: each later list holds only keys
//!   of the one before it (a key it leaves out is non-existent).
//!
//! An answer is the object's whoever gives it: where the table lacks `prevent_extensions`,
//! `set_prototype_of`, `delete` or `own_property_keys`, the record's answer (OrdinaryPreventExtensions,
//! OrdinarySetPrototypeOf, OrdinaryDelete, OrdinaryOwnPropertyKeys) is checked against the facts
//! and recorded as the override's would be. A key list never holds a key twice. What a method
//! without an override answers is a fact as it stands, the record being the target the Proxy
//! checks of `object/invariants.rs` compare with: the record's own properties while the table
//! has no `get_own_property` (a key list must hold each non-configurable one, and, while the
//! object is not extensible, only the record's keys), its [[Extensible]] while it has no
//! `is_extensible`, its [[Prototype]] while it has no `get_prototype_of`. The record's answers of
//! the other methods are made from these and from the answers of the overrides the ordinary
//! methods call (OrdinaryDefineOwnProperty, OrdinaryHasProperty, OrdinaryGet and OrdinarySet
//! read [[GetOwnProperty]], [[IsExtensible]] and [[GetPrototypeOf]] through the object's own
//! methods), so they are checked there. A broken invariant, a descriptor that is not complete or a stale or
//! foreign handle ends the run in `Terminated(EngineFault)` and faults the agent (HOST-48), never
//! a `TypeError` (section 15, "Rejected"). An override that answers `Abrupt::Throw` throws its
//! value after the check of its handles.
//!
//! Bounds. Steps: each override call spends one step before it runs. Depth: each is one
//! re-entry for its duration, counted in `Interp::proxy_nesting` beside the methods of proxies:
//! at its entry the activations plus that count plus this one passing `reentry_depth` throw the
//! RangeError of "Depth bounds" (`docs/design/engine.md`, decision 3) before the override runs,
//! so overrides that call each other through `Cx` end there, not in a Rust stack overflow. A
//! `Cx` call inside an override spends from the entry that called the internal method
//! (`Agent::begin`). Memory: the box of facts and the growth of each of its tables are charged
//! before they are made and recorded in the object's `Grown`, credited when the object is freed;
//! a fact is recorded only from an answer of the object, at most one per call of an override (a
//! key list's keys once, when it is the first after the report, a step per key; a key the
//! record's OrdinaryDelete makes non-existent follows a call of the `get_own_property`
//! override), so the step budget bounds the facts one entry adds and the memory budget all of
//! them;
//! the map of the keys of an answer of `own_property_keys` is charged before it is made and
//! credited when the method returns, and the list answered is charged as every list of keys is
//! ([`Agent::new_key_list`]). Each check of `own_property_keys` spends a step per key it visits.
//! What a method holds across the call (the object, the key, the value, the receiver, and the
//! answer) is kept until it returns.
//!
//! The table, the box of facts and the host's data are one box ([`Overridden`]), which is the
//! data of the host object, so that its record and every record that keeps a [[HostDefined]] as
//! a host object's data keep their sizes. Kind site of the collector: `HostObject::trace`
//! (`host/host_object.rs`) traces the data, whose `trace` ([`Overridden`]'s) lists the host's
//! data and, through `Facts::trace`, the keys, values and accessors of the fixed properties, the
//! prototype, the keys listed and the keys recorded non-existent.
//!
//! What a host-object arm holds across a call that may run an override (the object, the key,
//! the descriptor given, the descriptor answered) is kept until it returns, also where it runs
//! the ordinary method because the table lacks the override of its name
//! ([`Agent::held_ordinary_define_own_property`]).

use alloc::boxed::Box;
use core::any::Any;
use core::hash::Hash;

use super::invariants::{self, Next};
use super::property::{Descriptor, Key, Property, Slot};
use super::{KeyList, ObjectKind};
use crate::heap::map::KeyedMap;
use crate::host::agent::Agent;
use crate::host::host_object::{HostData, Overrides, Tracer};
use crate::host::outcome::{Abrupt, Completion, Termination};
use crate::host::{Cx, ErrorKind, Host, PropertyDescriptor};
use crate::value::{ObjectRef, PropertyKey, Value};
use swell_num::siphash::SipBuildHasher;

/// The walk of the chain that may hand over to a parent's own method
/// (`object/ordinary.rs`): OrdinaryHasProperty, OrdinaryGet or OrdinarySet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Walk {
    Has,
    Get,
    Set,
}

/// The properties an answer reported non-configurable, by key, each with its key for the
/// tracer.
type FixedMap = KeyedMap<Key, (Key, Property)>;

/// A set of keys, each with itself for the tracer.
type KeySet = KeyedMap<Key, Key>;

/// The facts the answers of a host object's overrides fixed (the module's documentation).
pub(crate) struct Facts {
    /// Each property `get_own_property` reported non-configurable, as last reported.
    fixed: FixedMap,
    /// Whether `is_extensible` answered false or `prevent_extensions` true.
    sealed: bool,
    /// The prototype reported while the object was not extensible.
    prototype: Option<Option<ObjectRef>>,
    /// Whether `keys` holds the keys of the last list `own_property_keys` answered while the
    /// object was not extensible, less those reported non-existent since: every other key is
    /// non-existent.
    listed: bool,
    keys: KeySet,
    /// The keys reported non-existent while the object was not extensible, before `listed`.
    absent: KeySet,
}

impl Facts {
    /// No fact.
    fn new(hasher: SipBuildHasher) -> Facts {
        Facts {
            fixed: KeyedMap::with_hasher(hasher),
            sealed: false,
            prototype: None,
            listed: false,
            keys: KeyedMap::with_hasher(hasher),
            absent: KeyedMap::with_hasher(hasher),
        }
    }

    /// What the facts hold, for the kind site of host objects: the key and the values of each
    /// property, the prototype, the keys listed and the keys non-existent.
    fn trace(&self, tracer: &mut Tracer<'_>) {
        for (key, property) in self.fixed.values() {
            trace_key(*key, tracer);
            match property.slot {
                Slot::Data { value, .. } => tracer.value(value),
                Slot::Accessor { get, set } => {
                    for function in [get, set].into_iter().flatten() {
                        tracer.object(function);
                    }
                }
            }
        }
        if let Some(Some(prototype)) = self.prototype {
            tracer.object(prototype);
        }
        for key in self.keys.values() {
            trace_key(*key, tracer);
        }
        for key in self.absent.values() {
            trace_key(*key, tracer);
        }
    }
}

/// The atom or the Symbol of `key`, for the tracer.
fn trace_key(key: Key, tracer: &mut Tracer<'_>) {
    match key {
        Key::Index(_) => {}
        Key::String(string) => tracer.value(Value::String(string)),
        Key::Symbol(symbol) => tracer.value(Value::Symbol(symbol)),
    }
}

/// The override table of a host object, the facts its answers fixed and the host's data, in one
/// box: the data of the host object (`HostObject::overridden`), owned by its record and charged
/// with it, so that the record keeps its size.
pub(crate) struct Overridden {
    /// An `Overrides<H>` of the agent's host, kept without its type as the steps of a host
    /// function are (`object/builtin_function.rs`).
    table: &'static dyn Any,
    /// The facts, `None` until the first.
    facts: Option<Box<Facts>>,
    /// The host's data.
    data: Option<Box<dyn HostData>>,
}

impl Overridden {
    /// The box of `table` and the host's `data`, with no fact.
    pub(crate) fn boxed<H: Host>(
        table: &'static Overrides<H>,
        data: Option<Box<dyn HostData>>,
    ) -> Box<dyn HostData> {
        Box::new(Overridden {
            table,
            facts: None,
            data,
        })
    }

    /// The host's data.
    pub(crate) fn data(&self) -> Option<&dyn HostData> {
        self.data.as_deref()
    }

    /// The host's data, to change.
    pub(crate) fn data_mut(&mut self) -> Option<&mut dyn HostData> {
        match &mut self.data {
            Some(data) => Some(&mut **data),
            None => None,
        }
    }
}

impl HostData for Overridden {
    /// What the host's data lists, and what the facts hold.
    fn trace(&self, tracer: &mut Tracer<'_>) {
        if let Some(data) = &self.data {
            data.trace(tracer);
        }
        if let Some(facts) = &self.facts {
            facts.trace(tracer);
        }
    }
}

/// The table of the properties of the facts.
fn fixed_of(facts: &mut Facts) -> &mut FixedMap {
    &mut facts.fixed
}

/// The table of the keys listed of the facts.
fn keys_of(facts: &mut Facts) -> &mut KeySet {
    &mut facts.keys
}

/// The table of the keys non-existent of the facts.
fn absent_of(facts: &mut Facts) -> &mut KeySet {
    &mut facts.absent
}

/// The bytes of the box of facts, charged when it is made.
fn facts_bytes() -> u64 {
    u64::try_from(size_of::<Facts>()).unwrap_or(u64::MAX)
}

/// What the engine knows of an own property of a host object with a table.
#[derive(Clone, Copy, Debug)]
enum Known {
    /// The record's property, or that it has none: the table has no `get_own_property`, so the
    /// record answers [[GetOwnProperty]] and is the target of the Proxy checks.
    Record(Option<Property>),
    /// The property an answer reported non-configurable.
    Fixed(Property),
    /// Non-existent: reported so once the object was not extensible.
    Absent,
    /// The object is not extensible and listed the key, with no fixed descriptor.
    Listed,
    /// Nothing.
    Unknown,
}

/// The host's descriptor of `descriptor`, an argument of `define_own_property`.
fn descriptor_out(descriptor: &Descriptor) -> PropertyDescriptor {
    let accessor = |field: Option<Option<ObjectRef>>| {
        field.map(|function| function.map_or(Value::Undefined, Value::Object))
    };
    PropertyDescriptor {
        value: descriptor.value,
        writable: descriptor.writable,
        get: accessor(descriptor.get),
        set: accessor(descriptor.set),
        enumerable: descriptor.enumerable,
        configurable: descriptor.configurable,
    }
}

/// The value a host's key holds.
fn key_handle(key: PropertyKey) -> Value {
    match key {
        PropertyKey::String(string) => Value::String(string),
        PropertyKey::Symbol(symbol) => Value::Symbol(symbol),
    }
}

impl<H: Host> Agent<H> {
    /// The override table of `object`, a host object, with the agent's host type; a table of
    /// another type is a broken invariant.
    fn override_table(&mut self, object: ObjectRef) -> Completion<Option<&'static Overrides<H>>> {
        let table = match &self.engine_object(object)?.kind {
            ObjectKind::Host(host) => host.overridden().map(|overridden| overridden.table),
            _ => None,
        };
        match table {
            None => Ok(None),
            Some(table) => match table.downcast_ref::<Overrides<H>>() {
                Some(table) => Ok(Some(table)),
                None => Err(self.fault()),
            },
        }
    }

    /// Whether a walk of the chain hands over to the method of `object`, a host object, that
    /// `walk` makes: its table has that override.
    pub(crate) fn host_overrides_walk(
        &mut self,
        object: ObjectRef,
        walk: Walk,
    ) -> Completion<bool> {
        let Some(table) = self.override_table(object)? else {
            return Ok(false);
        };
        Ok(match walk {
            Walk::Has => table.has_property.is_some(),
            Walk::Get => table.get.is_some(),
            Walk::Set => table.set.is_some(),
        })
    }

    /// Whether the internal methods of `object` may run code: a Proxy, or a host object with an
    /// override table.
    pub(crate) fn methods_may_run_code(&mut self, object: ObjectRef) -> Completion<bool> {
        Ok(match &self.engine_object(object)?.kind {
            ObjectKind::Proxy(_) => true,
            ObjectKind::Host(host) => host.overridden().is_some(),
            _ => false,
        })
    }

    /// Whether the [[GetPrototypeOf]] of `object` is not the ordinary one
    /// (`#sec-ordinarysetprototypeof`, step 7.c.i): a Proxy, or a host object whose table has
    /// `get_prototype_of`.
    pub(crate) fn get_prototype_of_is_exotic(&mut self, object: ObjectRef) -> Completion<bool> {
        if matches!(self.engine_object(object)?.kind, ObjectKind::Proxy(_)) {
            return Ok(true);
        }
        Ok(self
            .override_table(object)?
            .is_some_and(|table| table.get_prototype_of.is_some()))
    }

    /// The entry of an override (the module's documentation, "Bounds"): the sticky
    /// termination, a step, then the re-entry count, whose bound throws the RangeError of
    /// "Depth bounds" before the override runs; then the call counts itself. As for a Proxy,
    /// a builtin's activation innermost is the re-entry this call makes; otherwise it adds one.
    fn enter_override(&mut self) -> Completion<()> {
        self.running()?;
        self.charge_steps(1)?;
        let depth = self.activations.len();
        let own = u32::from(depth <= self.interp.running.unwrap_or(0));
        let activations = u32::try_from(depth).unwrap_or(u32::MAX);
        let reentries = activations
            .saturating_add(self.interp.proxy_nesting)
            .saturating_add(own);
        if reentries > self.limits.reentry_depth {
            return Err(self.throw_error(ErrorKind::RangeError, "reentry_depth exceeded"));
        }
        self.interp.proxy_nesting = self.interp.proxy_nesting.saturating_add(1);
        Ok(())
    }

    /// Run `steps`, the steps of an override of `object`, between [`Agent::enter_override`] and
    /// its exit: `object` and what the steps kept are released and the count dropped on every
    /// way out.
    fn in_override<T>(
        &mut self,
        object: ObjectRef,
        steps: impl FnOnce(&mut Self) -> Completion<T>,
    ) -> Completion<T> {
        self.enter_override()?;
        let mark = self.keep(Value::Object(object));
        let answer = steps(self);
        self.release(mark);
        self.interp.proxy_nesting = self.interp.proxy_nesting.saturating_sub(1);
        answer
    }

    /// The call of an override by `call`, which hands its arguments out and calls it, as a host
    /// function is called (`object/builtin_function.rs`, `run_host_function`): the values handed
    /// out in it are dropped from the scope when it returns; a terminated run ignores the
    /// answer (decision 10); a thrown value with a stale or foreign handle faults the agent
    /// (HOST-222) and is kept otherwise; a termination the override answered ends the run; then
    /// the interrupt is checked. The `Ok` value is the caller's to check.
    fn call_override<T>(&mut self, call: impl FnOnce(&mut Self) -> Completion<T>) -> Completion<T> {
        let start = self.scope.len();
        let answer = call(self);
        self.scope.truncate(start);
        self.running()?;
        let answer = match answer {
            Ok(answer) => Ok(answer),
            Err(Abrupt::Throw(value)) => {
                self.checked_answer(value)?;
                Err(Abrupt::Throw(value))
            }
            Err(Abrupt::Terminated(termination)) => return Err(self.terminate(termination)),
        };
        self.check_interrupt()?;
        answer
    }

    /// A value of an override's answer: a handle that reaches no record, stale or of another
    /// agent, faults the agent (HOST-222); otherwise it is kept until the method returns.
    fn checked_answer(&mut self, value: Value) -> Completion<()> {
        if self.heap.check_value(value).is_err() {
            return Err(self.fault());
        }
        self.keep(value);
        Ok(())
    }

    /// The values `descriptor` holds, kept until the method returns.
    fn keep_given(&mut self, descriptor: &PropertyDescriptor) {
        for value in [descriptor.value, descriptor.get, descriptor.set]
            .into_iter()
            .flatten()
        {
            self.keep(value);
        }
    }

    /// The property a `get_own_property` override answered: its handles checked (HOST-222),
    /// then a complete descriptor, data or accessor (HOST-113), each accessor a function or
    /// undefined; anything else faults the agent.
    fn answered_property(&mut self, descriptor: PropertyDescriptor) -> Completion<Property> {
        for value in [descriptor.value, descriptor.get, descriptor.set]
            .into_iter()
            .flatten()
        {
            self.checked_answer(value)?;
        }
        let (Some(enumerable), Some(configurable)) =
            (descriptor.enumerable, descriptor.configurable)
        else {
            return Err(self.fault());
        };
        let slot = match (
            descriptor.value,
            descriptor.writable,
            descriptor.get,
            descriptor.set,
        ) {
            (Some(value), Some(writable), None, None) => Slot::Data { value, writable },
            (None, None, Some(get), Some(set)) => Slot::Accessor {
                get: self.answered_accessor(get)?,
                set: self.answered_accessor(set)?,
            },
            _ => return Err(self.fault()),
        };
        Ok(Property {
            slot,
            enumerable,
            configurable,
        })
    }

    /// A getter or setter of an answered descriptor: undefined or a function.
    fn answered_accessor(&mut self, value: Value) -> Completion<Option<ObjectRef>> {
        match value {
            Value::Undefined => Ok(None),
            Value::Object(function) if self.is_callable(value)? => Ok(Some(function)),
            _ => Err(self.fault()),
        }
    }

    /// The facts of `object`, if it has any.
    fn facts(&self, object: ObjectRef) -> Option<&Facts> {
        match &self.heap.objects.get(object.handle).ok()?.kind {
            ObjectKind::Host(host) => host.overridden()?.facts.as_deref(),
            _ => None,
        }
    }

    /// Make the box of facts of `object` if it has none: its bytes charged before it is made
    /// and recorded, to be credited when the object is freed.
    fn ensure_facts(&mut self, object: ObjectRef) -> Completion<()> {
        let made = match &self.engine_object(object)?.kind {
            ObjectKind::Host(host) => host
                .overridden()
                .map(|overridden| overridden.facts.is_some()),
            _ => None,
        };
        match made {
            Some(true) => return Ok(()),
            Some(false) => {}
            None => return Err(self.fault()),
        }
        let bytes = facts_bytes();
        self.charge_bytes(bytes)?;
        self.record_grown(object, bytes)?;
        let facts = Box::new(Facts::new(self.heap.hasher));
        if let ObjectKind::Host(host) = &mut self.engine_object_mut(object)?.kind
            && let Some(overridden) = host.overridden_mut()
        {
            overridden.facts = Some(facts);
        }
        Ok(())
    }

    /// Change the facts of `object` by `change`, after the box is made.
    fn with_facts<T>(
        &mut self,
        object: ObjectRef,
        change: impl FnOnce(&mut Facts) -> T,
    ) -> Completion<T> {
        self.ensure_facts(object)?;
        let facts = match &mut self.engine_object_mut(object)?.kind {
            ObjectKind::Host(host) => host
                .overridden_mut()
                .and_then(|overridden| overridden.facts.as_deref_mut()),
            _ => None,
        };
        match facts {
            Some(facts) => Ok(change(facts)),
            // `ensure_facts` made the box.
            None => Err(Abrupt::Terminated(Termination::EngineFault)),
        }
    }

    /// Room for `additional` more entries in the table `table` picks of the facts of `object`:
    /// the growth is charged before it is made and recorded, to be credited when the object is
    /// freed.
    fn room_in<K: Hash + Eq, V>(
        &mut self,
        object: ObjectRef,
        table: fn(&mut Facts) -> &mut KeyedMap<K, V>,
        additional: usize,
    ) -> Completion<()> {
        let (growth, before) = self.with_facts(object, |facts| {
            let table = table(facts);
            (table.growth_for(additional), table.table_bytes())
        })?;
        let Ok(growth) = growth else {
            return Err(self.terminate(Termination::MemoryBudget));
        };
        let mut charged = 0;
        if let Some(capacity) = growth {
            charged = KeyedMap::<K, V>::table_bytes_for(capacity).saturating_sub(before);
            self.charge_bytes(charged)?;
            self.record_grown(object, charged)?;
        }
        let (reserved, after) = self.with_facts(object, |facts| {
            let table = table(facts);
            (table.reserve(additional), table.table_bytes())
        })?;
        if reserved.is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let more = after.saturating_sub(before.saturating_add(charged));
        if more > 0 {
            // The table took more than its layout says: charged now.
            self.charge_bytes(more)?;
            self.record_grown(object, more)?;
        }
        Ok(())
    }

    /// Record `property`, reported non-configurable for `key`, as a fact of `object`.
    fn record_fixed(&mut self, object: ObjectRef, key: Key, property: Property) -> Completion<()> {
        let known = self
            .facts(object)
            .is_some_and(|facts| facts.fixed.get(&key).is_some());
        if !known {
            // A key fixed before is replaced in its place; a new one may need room.
            self.room_in(object, fixed_of, 1)?;
        }
        if self
            .with_facts(object, |facts| facts.fixed.insert(key, (key, property)))?
            .is_err()
        {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(())
    }

    /// Record `key` non-existent on `object`, which is not extensible: dropped from the keys
    /// listed, or added to the keys non-existent before a list.
    fn record_absent(&mut self, object: ObjectRef, key: Key) -> Completion<()> {
        let (listed, known) = self.facts(object).map_or((false, false), |facts| {
            (facts.listed, facts.absent.get(&key).is_some())
        });
        if listed {
            self.with_facts(object, |facts| facts.keys.remove(&key))?;
            return Ok(());
        }
        if known {
            return Ok(());
        }
        self.room_in(object, absent_of, 1)?;
        if self
            .with_facts(object, |facts| facts.absent.insert(key, key))?
            .is_err()
        {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(())
    }

    /// After an answer that says `object` has no own property `key` (absent from
    /// `get_own_property`, true from [[Delete]]), with `known` what was known before: a fixed key
    /// breaks the invariant; otherwise, once the object is not extensible, the key is
    /// non-existent from now on.
    fn answered_absent(
        &mut self,
        object: ObjectRef,
        table: &Overrides<H>,
        key: Key,
        known: Known,
    ) -> Completion<()> {
        match known {
            Known::Fixed(_) | Known::Record(_) => Err(self.fault()),
            Known::Absent => Ok(()),
            Known::Listed | Known::Unknown => {
                if self.known_extensible(object, table)? == Some(false) {
                    self.record_absent(object, key)?;
                }
                Ok(())
            }
        }
    }

    /// Record `keys`, reported while `object` is not extensible, as its own keys.
    fn record_keys(&mut self, object: ObjectRef, keys: &[Key]) -> Completion<()> {
        self.room_in(object, keys_of, keys.len())?;
        let inserted = self.with_facts(object, |facts| {
            facts.listed = true;
            keys.iter()
                .try_for_each(|&key| facts.keys.insert(key, key).map(|_| ()))
        })?;
        if inserted.is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(())
    }

    /// Whether `object` is extensible as far as the engine knows: its [[Extensible]] while its
    /// table has no `is_extensible`; else false once an answer said so, unknown before.
    fn known_extensible(
        &mut self,
        object: ObjectRef,
        table: &Overrides<H>,
    ) -> Completion<Option<bool>> {
        if table.is_extensible.is_none() {
            return Ok(Some(self.engine_object(object)?.extensible));
        }
        Ok(self
            .facts(object)
            .is_some_and(|facts| facts.sealed)
            .then_some(false))
    }

    /// The prototype of `object` as far as the engine knows: its [[Prototype]] while its table
    /// has no `get_prototype_of`; else the one recorded while it was not extensible.
    fn known_prototype(
        &mut self,
        object: ObjectRef,
        table: &Overrides<H>,
    ) -> Completion<Option<Option<ObjectRef>>> {
        if table.get_prototype_of.is_none() {
            return Ok(Some(self.engine_object(object)?.prototype));
        }
        Ok(self.facts(object).and_then(|facts| facts.prototype))
    }

    /// What the engine knows of the own property `key` of `object`: its record's while its
    /// table has no `get_own_property`; else the fixed fact, or, once the object is not
    /// extensible, whether `key` is non-existent or, after a list, listed.
    fn known_property(
        &mut self,
        object: ObjectRef,
        table: &Overrides<H>,
        key: Key,
    ) -> Completion<Known> {
        if table.get_own_property.is_none() {
            return Ok(Known::Record(self.ordinary_get_own_property(object, key)?));
        }
        let extensible = self.known_extensible(object, table)?;
        let Some(facts) = self.facts(object) else {
            return Ok(Known::Unknown);
        };
        if let Some(&(_, property)) = facts.fixed.get(&key) {
            return Ok(Known::Fixed(property));
        }
        if extensible != Some(false) {
            return Ok(Known::Unknown);
        }
        if facts.listed {
            return Ok(if facts.keys.get(&key).is_some() {
                Known::Listed
            } else {
                Known::Absent
            });
        }
        Ok(if facts.absent.get(&key).is_some() {
            Known::Absent
        } else {
            Known::Unknown
        })
    }

    /// The descriptor the Proxy checks compare with: the record's, or the fixed one.
    fn known_descriptor(known: Known) -> Option<Option<Property>> {
        match known {
            Known::Record(current) => Some(current),
            Known::Fixed(property) => Some(Some(property)),
            Known::Absent | Known::Listed | Known::Unknown => None,
        }
    }

    /// [[GetPrototypeOf]] of a host object.
    pub(crate) fn host_get_prototype_of(
        &mut self,
        object: ObjectRef,
    ) -> Completion<Option<ObjectRef>> {
        match self.override_table(object)? {
            Some(table) if table.get_prototype_of.is_some() => self.in_override(object, |agent| {
                agent.override_get_prototype_of(object, table)
            }),
            _ => self.ordinary_get_prototype_of(object),
        }
    }

    /// The `get_prototype_of` override: while the object is not extensible, the answer is
    /// the prototype recorded (I2), or is recorded.
    #[inline(never)]
    fn override_get_prototype_of(
        &mut self,
        object: ObjectRef,
        table: &'static Overrides<H>,
    ) -> Completion<Option<ObjectRef>> {
        let Some(call) = table.get_prototype_of else {
            return Err(self.fault());
        };
        let answer = self.call_override(|agent| {
            agent.hand_out(Value::Object(object));
            call(&mut Cx { agent }, object)
        })?;
        if let Some(prototype) = answer {
            self.checked_answer(Value::Object(prototype))?;
        }
        if self.known_extensible(object, table)? == Some(false) {
            match self.known_prototype(object, table)? {
                Some(recorded) => {
                    if invariants::get_prototype_of_after_prototype(answer, recorded).is_err() {
                        return Err(self.fault());
                    }
                }
                None => self.with_facts(object, |facts| facts.prototype = Some(answer))?,
            }
        }
        Ok(answer)
    }

    /// [[SetPrototypeOf]] of a host object.
    pub(crate) fn host_set_prototype_of(
        &mut self,
        object: ObjectRef,
        prototype: Option<ObjectRef>,
    ) -> Completion<bool> {
        match self.override_table(object)? {
            Some(table) if table.set_prototype_of.is_some() => self.in_override(object, |agent| {
                agent.override_set_prototype_of(object, table, prototype)
            }),
            Some(table) => {
                // The record answers; what the object said before the answer changed it.
                let extensible = self.known_extensible(object, table)?;
                let recorded = self.known_prototype(object, table)?;
                let answer = self.ordinary_set_prototype_of(object, prototype)?;
                if answer {
                    self.set_prototype_of_true(object, extensible, recorded, prototype)?;
                }
                Ok(answer)
            }
            None => self.ordinary_set_prototype_of(object, prototype),
        }
    }

    /// The `set_prototype_of` override: true while the object is not extensible only for the
    /// prototype recorded (I3), or the value is recorded.
    #[inline(never)]
    fn override_set_prototype_of(
        &mut self,
        object: ObjectRef,
        table: &'static Overrides<H>,
        prototype: Option<ObjectRef>,
    ) -> Completion<bool> {
        let Some(call) = table.set_prototype_of else {
            return Err(self.fault());
        };
        let value = prototype.map_or(Value::Null, Value::Object);
        self.keep(value);
        let answer = self.call_override(|agent| {
            agent.hand_out(Value::Object(object));
            agent.hand_out(value);
            call(&mut Cx { agent }, object, prototype)
        })?;
        if let Next::Answer(answer) = invariants::set_prototype_of_after_trap(answer) {
            return Ok(answer);
        }
        let extensible = self.known_extensible(object, table)?;
        let recorded = self.known_prototype(object, table)?;
        self.set_prototype_of_true(object, extensible, recorded, prototype)?;
        Ok(true)
    }

    /// [[SetPrototypeOf]] of `object` answered true for `prototype`, from the override or the
    /// record, with `extensible` and `recorded` what was known before the record answered:
    /// while the object is not extensible, true only for the prototype recorded (I3), or the
    /// value is recorded.
    fn set_prototype_of_true(
        &mut self,
        object: ObjectRef,
        extensible: Option<bool>,
        recorded: Option<Option<ObjectRef>>,
        prototype: Option<ObjectRef>,
    ) -> Completion<()> {
        if extensible != Some(false) {
            return Ok(());
        }
        match recorded {
            Some(recorded) => {
                if invariants::set_prototype_of_after_prototype(prototype, recorded).is_err() {
                    return Err(self.fault());
                }
                Ok(())
            }
            None => self.with_facts(object, |facts| facts.prototype = Some(prototype)),
        }
    }

    /// [[IsExtensible]] of a host object.
    pub(crate) fn host_is_extensible(&mut self, object: ObjectRef) -> Completion<bool> {
        match self.override_table(object)? {
            Some(table) if table.is_extensible.is_some() => {
                self.in_override(object, |agent| agent.override_is_extensible(object, table))
            }
            _ => self.ordinary_is_extensible(object),
        }
    }

    /// The `is_extensible` override: once it answered false, or `prevent_extensions` true, it
    /// answers false (I4); a first false is recorded.
    #[inline(never)]
    fn override_is_extensible(
        &mut self,
        object: ObjectRef,
        table: &'static Overrides<H>,
    ) -> Completion<bool> {
        let Some(call) = table.is_extensible else {
            return Err(self.fault());
        };
        let answer = self.call_override(|agent| {
            agent.hand_out(Value::Object(object));
            call(&mut Cx { agent }, object)
        })?;
        match self.known_extensible(object, table)? {
            Some(known) => {
                if invariants::is_extensible_after_target(answer, known).is_err() {
                    return Err(self.fault());
                }
            }
            None if !answer => self.with_facts(object, |facts| facts.sealed = true)?,
            None => {}
        }
        Ok(answer)
    }

    /// [[PreventExtensions]] of a host object.
    pub(crate) fn host_prevent_extensions(&mut self, object: ObjectRef) -> Completion<bool> {
        match self.override_table(object)? {
            Some(table) if table.prevent_extensions.is_some() => self
                .in_override(object, |agent| {
                    agent.override_prevent_extensions(object, table)
                }),
            Some(table) => {
                let answer = self.ordinary_prevent_extensions(object)?;
                if answer {
                    self.prevent_extensions_true(object, table)?;
                }
                Ok(answer)
            }
            None => self.ordinary_prevent_extensions(object),
        }
    }

    /// The `prevent_extensions` override: true for an object known to be extensible breaks I5;
    /// a true is recorded.
    #[inline(never)]
    fn override_prevent_extensions(
        &mut self,
        object: ObjectRef,
        table: &'static Overrides<H>,
    ) -> Completion<bool> {
        let Some(call) = table.prevent_extensions else {
            return Err(self.fault());
        };
        let answer = self.call_override(|agent| {
            agent.hand_out(Value::Object(object));
            call(&mut Cx { agent }, object)
        })?;
        if let Next::Answer(answer) = invariants::prevent_extensions_after_trap(answer) {
            return Ok(answer);
        }
        self.prevent_extensions_true(object, table)?;
        Ok(true)
    }

    /// [[PreventExtensions]] of `object` answered true, from the override or the record: an
    /// object known to be extensible breaks I5; otherwise the object is not extensible from now
    /// on.
    fn prevent_extensions_true(
        &mut self,
        object: ObjectRef,
        table: &Overrides<H>,
    ) -> Completion<()> {
        match self.known_extensible(object, table)? {
            Some(known) => {
                if invariants::prevent_extensions_after_extensible(known).is_err() {
                    return Err(self.fault());
                }
                Ok(())
            }
            None => self.with_facts(object, |facts| facts.sealed = true),
        }
    }

    /// [[GetOwnProperty]] of a host object.
    pub(crate) fn host_get_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
    ) -> Completion<Option<Property>> {
        match self.override_table(object)? {
            Some(table) if table.get_own_property.is_some() => self.in_override(object, |agent| {
                agent.override_get_own_property(object, table, key)
            }),
            _ => self.ordinary_get_own_property(object, key),
        }
    }

    /// The `get_own_property` override: a complete descriptor (HOST-113); absent for a fixed
    /// property breaks the invariant (HOST-112) and is recorded non-existent once the object is
    /// not extensible; a descriptor compatible with the fixed one, and none for a key
    /// non-existent. A non-configurable answer is recorded.
    #[inline(never)]
    fn override_get_own_property(
        &mut self,
        object: ObjectRef,
        table: &'static Overrides<H>,
        key: Key,
    ) -> Completion<Option<Property>> {
        let Some(call) = table.get_own_property else {
            return Err(self.fault());
        };
        self.keep_key(key);
        let answer = self.call_override(|agent| {
            agent.hand_out(Value::Object(object));
            let key = agent.property_key_out(key)?;
            call(&mut Cx { agent }, object, key)
        })?;
        let answer = match answer {
            Some(descriptor) => Some(self.answered_property(descriptor)?),
            None => None,
        };
        let known = self.known_property(object, table, key)?;
        let Some(result) = answer else {
            self.answered_absent(object, table, key, known)?;
            return Ok(None);
        };
        match known {
            Known::Fixed(current) => {
                let extensible = self.known_extensible(object, table)?.unwrap_or(true);
                let descriptor = invariants::descriptor_of(result);
                if !invariants::is_compatible_property_descriptor(
                    self,
                    extensible,
                    descriptor,
                    Some(current),
                )? {
                    return Err(self.fault());
                }
            }
            Known::Absent | Known::Record(_) => return Err(self.fault()),
            Known::Listed | Known::Unknown => {}
        }
        if !result.configurable {
            self.record_fixed(object, key, result)?;
        }
        Ok(Some(result))
    }

    /// [[DefineOwnProperty]] of a host object.
    pub(crate) fn host_define_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        match self.override_table(object)? {
            Some(table) if table.define_own_property.is_some() => self
                .in_override(object, |agent| {
                    agent.override_define_own_property(object, table, key, descriptor)
                }),
            Some(_) => self.held_ordinary_define_own_property(object, key, descriptor),
            None => self.ordinary_define_own_property(object, key, descriptor),
        }
    }

    /// OrdinaryDefineOwnProperty (`#sec-ordinarydefineownproperty`) of a host object whose table
    /// has no `define_own_property`: its [[GetOwnProperty]] and [[IsExtensible]] may run an
    /// override, which may run script, so the object, the key, the values of the descriptor
    /// given and those of the property answered are kept until it returns.
    pub(crate) fn held_ordinary_define_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        let mark = self.keep(Value::Object(object));
        self.keep_key(key);
        self.keep_given(&descriptor_out(&descriptor));
        let answer = self.held_define_steps(object, key, descriptor);
        self.release(mark);
        answer
    }

    /// The steps of [`Agent::held_ordinary_define_own_property`] once it kept what it holds.
    fn held_define_steps(
        &mut self,
        object: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        let current = self.get_own_property(object, key)?;
        if let Some(property) = current {
            match property.slot {
                Slot::Data { value, .. } => {
                    self.keep(value);
                }
                Slot::Accessor { get, set } => {
                    for function in [get, set].into_iter().flatten() {
                        self.keep(Value::Object(function));
                    }
                }
            }
        }
        let extensible = self.is_extensible(object)?;
        self.validate_and_apply_property_descriptor(
            Some(object),
            key,
            extensible,
            descriptor,
            current,
        )
    }

    /// The `define_own_property` override: true is checked against what the engine knows of
    /// the property, the Proxy's checks (I12 to I16) against the record or a fixed property,
    /// and false for a key non-existent.
    #[inline(never)]
    fn override_define_own_property(
        &mut self,
        object: ObjectRef,
        table: &'static Overrides<H>,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        let Some(call) = table.define_own_property else {
            return Err(self.fault());
        };
        self.keep_key(key);
        let given = descriptor_out(&descriptor);
        self.keep_given(&given);
        let answer = self.call_override(|agent| {
            agent.hand_out(Value::Object(object));
            for value in [given.value, given.get, given.set].into_iter().flatten() {
                agent.hand_out(value);
            }
            let key = agent.property_key_out(key)?;
            call(&mut Cx { agent }, object, key, given)
        })?;
        if let Next::Answer(answer) = invariants::define_own_property_after_trap(answer) {
            return Ok(answer);
        }
        let known = self.known_property(object, table, key)?;
        if let Known::Absent = known {
            return Err(self.fault());
        }
        if let Some(current) = Self::known_descriptor(known) {
            let extensible = self.known_extensible(object, table)?.unwrap_or(true);
            if invariants::define_own_property(self, extensible, descriptor, current)?.is_err() {
                return Err(self.fault());
            }
        }
        Ok(true)
    }

    /// [[HasProperty]] of a host object.
    pub(crate) fn host_has_property(&mut self, object: ObjectRef, key: Key) -> Completion<bool> {
        match self.override_table(object)? {
            Some(table) if table.has_property.is_some() => self.in_override(object, |agent| {
                agent.override_has_property(object, table, key)
            }),
            _ => self.ordinary_has_property(object, key),
        }
    }

    /// The `has_property` override: false for a non-configurable property, fixed or of the
    /// record, breaks the invariant; any other false, also on an object that is not extensible,
    /// is allowed and records nothing (the clause binds [[HasProperty]] only for a key observed
    /// non-configurable).
    #[inline(never)]
    fn override_has_property(
        &mut self,
        object: ObjectRef,
        table: &'static Overrides<H>,
        key: Key,
    ) -> Completion<bool> {
        let Some(call) = table.has_property else {
            return Err(self.fault());
        };
        self.keep_key(key);
        let answer = self.call_override(|agent| {
            agent.hand_out(Value::Object(object));
            let key = agent.property_key_out(key)?;
            call(&mut Cx { agent }, object, key)
        })?;
        if let Next::Answer(answer) = invariants::has_after_trap(answer) {
            return Ok(answer);
        }
        match self.known_property(object, table, key)? {
            Known::Record(Some(property)) | Known::Fixed(property) if !property.configurable => {
                Err(self.fault())
            }
            _ => Ok(false),
        }
    }

    /// [[Get]] of a host object.
    pub(crate) fn host_get(
        &mut self,
        object: ObjectRef,
        key: Key,
        receiver: Value,
    ) -> Completion<Value> {
        match self.override_table(object)? {
            Some(table) if table.get.is_some() => self.in_override(object, |agent| {
                agent.override_get(object, table, key, receiver)
            }),
            _ => self.ordinary_get(object, key, receiver),
        }
    }

    /// The `get` override: the value of a fixed non-writable data property, and undefined for
    /// a fixed accessor without a getter.
    #[inline(never)]
    fn override_get(
        &mut self,
        object: ObjectRef,
        table: &'static Overrides<H>,
        key: Key,
        receiver: Value,
    ) -> Completion<Value> {
        let Some(call) = table.get else {
            return Err(self.fault());
        };
        self.keep_key(key);
        self.keep(receiver);
        let answer = self.call_override(|agent| {
            agent.hand_out(Value::Object(object));
            agent.hand_out(receiver);
            let key = agent.property_key_out(key)?;
            call(&mut Cx { agent }, object, key, receiver)
        })?;
        self.checked_answer(answer)?;
        let known = self.known_property(object, table, key)?;
        if let Some(current) = Self::known_descriptor(known)
            && invariants::get(self, current, answer)?.is_err()
        {
            return Err(self.fault());
        }
        Ok(answer)
    }

    /// [[Set]] of a host object.
    pub(crate) fn host_set(
        &mut self,
        object: ObjectRef,
        key: Key,
        value: Value,
        receiver: Value,
    ) -> Completion<bool> {
        match self.override_table(object)? {
            Some(table) if table.set.is_some() => self.in_override(object, |agent| {
                agent.override_set(object, table, key, value, receiver)
            }),
            _ => self.ordinary_set(object, key, value, receiver),
        }
    }

    /// The `set` override: true for a fixed non-writable data property only with its value,
    /// and never for a fixed accessor without a setter.
    #[inline(never)]
    fn override_set(
        &mut self,
        object: ObjectRef,
        table: &'static Overrides<H>,
        key: Key,
        value: Value,
        receiver: Value,
    ) -> Completion<bool> {
        let Some(call) = table.set else {
            return Err(self.fault());
        };
        self.keep_key(key);
        self.keep(value);
        self.keep(receiver);
        let answer = self.call_override(|agent| {
            agent.hand_out(Value::Object(object));
            agent.hand_out(value);
            agent.hand_out(receiver);
            let key = agent.property_key_out(key)?;
            call(&mut Cx { agent }, object, key, value, receiver)
        })?;
        if let Next::Answer(answer) = invariants::set_after_trap(answer) {
            return Ok(answer);
        }
        let known = self.known_property(object, table, key)?;
        if let Some(current) = Self::known_descriptor(known)
            && invariants::set(self, current, value)?.is_err()
        {
            return Err(self.fault());
        }
        Ok(true)
    }

    /// [[Delete]] of a host object.
    pub(crate) fn host_delete(&mut self, object: ObjectRef, key: Key) -> Completion<bool> {
        match self.override_table(object)? {
            Some(table) if table.delete.is_some() => {
                self.in_override(object, |agent| agent.override_delete(object, table, key))
            }
            Some(table) => {
                let answer = self.ordinary_delete(object, key)?;
                if answer {
                    self.delete_true(object, table, key)?;
                }
                Ok(answer)
            }
            None => self.ordinary_delete(object, key),
        }
    }

    /// The `delete` override: true for a fixed property breaks the invariant, and for a
    /// property of the record the Proxy's checks; otherwise true makes the key non-existent
    /// once the object is not extensible.
    #[inline(never)]
    fn override_delete(
        &mut self,
        object: ObjectRef,
        table: &'static Overrides<H>,
        key: Key,
    ) -> Completion<bool> {
        let Some(call) = table.delete else {
            return Err(self.fault());
        };
        self.keep_key(key);
        let answer = self.call_override(|agent| {
            agent.hand_out(Value::Object(object));
            let key = agent.property_key_out(key)?;
            call(&mut Cx { agent }, object, key)
        })?;
        if let Next::Answer(answer) = invariants::delete_after_trap(answer) {
            return Ok(answer);
        }
        self.delete_true(object, table, key)?;
        Ok(true)
    }

    /// [[Delete]] of `object` answered true for `key`, from the override or the record: true
    /// for a fixed property breaks the invariant, and for a property of the record the Proxy's
    /// checks (the record after its own OrdinaryDelete no longer holds a configurable one);
    /// otherwise the key is non-existent from now on once the object is not extensible.
    fn delete_true(&mut self, object: ObjectRef, table: &Overrides<H>, key: Key) -> Completion<()> {
        match self.known_property(object, table, key)? {
            Known::Record(current) => match invariants::delete_after_descriptor(current) {
                Ok(Next::ReadExtensible) => {
                    let extensible = self.known_extensible(object, table)?.unwrap_or(true);
                    if invariants::delete_after_extensible(extensible).is_err() {
                        return Err(self.fault());
                    }
                }
                Ok(_) => {}
                Err(_) => return Err(self.fault()),
            },
            known => self.answered_absent(object, table, key, known)?,
        }
        Ok(())
    }

    /// [[OwnPropertyKeys]] of a host object. Where the table has `get_own_property` and no
    /// `own_property_keys`, the record's list is checked as an override's answer is; with
    /// neither, the record answers both and its list is its own keys.
    pub(crate) fn host_own_property_keys(&mut self, object: ObjectRef) -> Completion<KeyList> {
        match self.override_table(object)? {
            Some(table) if table.own_property_keys.is_some() => self.in_override(object, |agent| {
                agent.override_own_property_keys(object, table)
            }),
            Some(table) if table.get_own_property.is_some() => {
                let list = self.ordinary_own_property_keys(object)?;
                let checked = self.with_seen(list.keys.len(), |agent, seen| {
                    agent.key_list_check(object, table, &list.keys, seen)
                });
                match checked {
                    Ok(()) => Ok(list),
                    Err(abrupt) => {
                        self.drop_key_list(list);
                        Err(abrupt)
                    }
                }
            }
            _ => self.ordinary_own_property_keys(object),
        }
    }

    /// The `own_property_keys` override: each key's handle checked (HOST-222), then the list
    /// answered made, charged as every list of keys is; then the checks, a step per key, the
    /// list credited on an abrupt completion.
    #[inline(never)]
    fn override_own_property_keys(
        &mut self,
        object: ObjectRef,
        table: &'static Overrides<H>,
    ) -> Completion<KeyList> {
        let Some(call) = table.own_property_keys else {
            return Err(self.fault());
        };
        let answer = self.call_override(|agent| {
            agent.hand_out(Value::Object(object));
            call(&mut Cx { agent }, object)
        })?;
        for &key in &answer {
            if self.heap.check_value(key_handle(key)).is_err() {
                return Err(self.fault());
            }
        }
        let mut list = self.new_key_list(answer.len())?;
        let keys = &mut list.keys;
        let checked = self.with_seen(answer.len(), |agent, seen| {
            for &key in &answer {
                let key = agent.key_of(key)?;
                keys.push(key);
            }
            agent.key_list_check(object, table, keys, seen)
        });
        match checked {
            Ok(()) => Ok(list),
            Err(abrupt) => {
                self.drop_key_list(list);
                Err(abrupt)
            }
        }
    }

    /// Run `check` with a map for the keys of a list of `count` keys, its bytes charged before
    /// it is made and credited when `check` returns.
    fn with_seen(
        &mut self,
        count: usize,
        check: impl FnOnce(&mut Self, &mut KeyedMap<Key, ()>) -> Completion<()>,
    ) -> Completion<()> {
        let bytes = KeyedMap::<Key, ()>::table_bytes_for(count);
        self.charge_bytes(bytes)?;
        let mut seen = KeyedMap::<Key, ()>::with_hasher(self.heap.hasher);
        let mut charged = bytes;
        let checked = match self.reserve_seen(&mut seen, count, &mut charged) {
            Ok(()) => check(self, &mut seen),
            Err(abrupt) => Err(abrupt),
        };
        self.heap.accounting.credit(charged);
        checked
    }

    /// Room for `count` keys in `seen`, `charged` bytes so far.
    fn reserve_seen(
        &mut self,
        seen: &mut KeyedMap<Key, ()>,
        count: usize,
        charged: &mut u64,
    ) -> Completion<()> {
        if seen.reserve(count).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let table_bytes = seen.table_bytes();
        if table_bytes > *charged {
            // The map took more than its layout says: charged now.
            self.charge_bytes(table_bytes - *charged)?;
            *charged = table_bytes;
        }
        Ok(())
    }

    /// The checks of a key list of `object`, from the override or the record, with `seen`, an
    /// empty map with room for its keys: a key listed twice breaks the invariant (I18), a step
    /// per key; each fixed key must be listed (I19), a step per key; an object that is not
    /// extensible lists exactly the keys it listed first (I20, I21), a step per key, or its
    /// first list is recorded.
    fn key_list_check(
        &mut self,
        object: ObjectRef,
        table: &Overrides<H>,
        keys: &[Key],
        seen: &mut KeyedMap<Key, ()>,
    ) -> Completion<()> {
        for &key in keys {
            self.charge_steps(1)?;
            match seen.insert(key, ()) {
                Ok(None) => {}
                Ok(Some(())) => return Err(self.fault()),
                Err(_) => return Err(self.terminate(Termination::MemoryBudget)),
            }
        }
        let seen = &*seen;
        let sealed = self.known_extensible(object, table)? == Some(false);
        if table.get_own_property.is_none() {
            return self.record_keys_check(object, sealed, keys, seen);
        }
        let fixed = self.facts(object).map_or(0, |facts| facts.fixed.len());
        self.charge_steps(u64::try_from(fixed).unwrap_or(u64::MAX))?;
        let missing = self
            .facts(object)
            .is_some_and(|facts| facts.fixed.values().any(|(key, _)| seen.get(key).is_none()));
        if missing || !sealed {
            return if missing { Err(self.fault()) } else { Ok(()) };
        }
        // Not extensible: no key non-existent, and, after a first list, only keys of the list
        // before; a step per key of the answer, then per key of that list.
        self.charge_steps(u64::try_from(keys.len()).unwrap_or(u64::MAX))?;
        let (listed, count) = self
            .facts(object)
            .map_or((false, 0), |facts| (facts.listed, facts.keys.len()));
        let unknown = self.facts(object).is_some_and(|facts| {
            keys.iter().any(|key| {
                facts.absent.get(key).is_some() || (facts.listed && facts.keys.get(key).is_none())
            })
        });
        if unknown {
            return Err(self.fault());
        }
        if !listed {
            return self.record_keys(object, keys);
        }
        self.charge_steps(u64::try_from(count).unwrap_or(u64::MAX))?;
        self.with_facts(object, |facts| {
            facts.keys.retain(|key, _| seen.get(key).is_some());
        })
    }

    /// The checks of an answer of `own_property_keys` of an object whose table has no
    /// `get_own_property`, so that its record answers [[GetOwnProperty]]: each non-configurable
    /// property of the record is listed, and, while the object is not extensible (`sealed`),
    /// each key listed is a property of the record; a step per key of the record, then per key
    /// of the answer.
    fn record_keys_check(
        &mut self,
        object: ObjectRef,
        sealed: bool,
        keys: &[Key],
        seen: &KeyedMap<Key, ()>,
    ) -> Completion<()> {
        let record = self.ordinary_own_property_keys(object)?;
        let mut broken = false;
        for &key in &record.keys {
            let property = self.ordinary_get_own_property(object, key)?;
            if property.is_some_and(|property| !property.configurable) && seen.get(&key).is_none() {
                broken = true;
                break;
            }
        }
        self.drop_key_list(record);
        if broken {
            return Err(self.fault());
        }
        if !sealed {
            return Ok(());
        }
        self.charge_steps(u64::try_from(keys.len()).unwrap_or(u64::MAX))?;
        for &key in keys {
            if self.ordinary_get_own_property(object, key)?.is_none() {
                return Err(self.fault());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Facts, Overridden};
    use crate::host::host_object::HostObject;
    use crate::object::property::{Key, Property};

    /// The bytes of one fact in the table of fixed properties: a bucket's entry, the key and
    /// the property with its key; `tests/host_overrides.rs` adds the control byte of the bucket
    /// (`FACT_BYTES`, 73). A key non-existent is an entry of the key and itself (`ABSENT_BYTES`,
    /// 33 with the control byte). The boxes are charged with their own bytes, printed here with
    /// the kind part of a host object's record.
    #[test]
    fn a_fact_has_the_bytes_the_tests_name() {
        std::println!(
            "fact {} bytes, key non-existent {} bytes, box of facts {} bytes, box of a table {} bytes, HostObject {} bytes",
            size_of::<Option<(Key, (Key, Property))>>(),
            size_of::<Option<(Key, Key)>>(),
            size_of::<Facts>(),
            size_of::<Overridden>(),
            size_of::<HostObject>()
        );
        assert_eq!(size_of::<Option<(Key, (Key, Property))>>(), 72);
        assert_eq!(size_of::<Option<(Key, Key)>>(), 32);
    }
}

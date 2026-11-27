//! Host objects with an override table (`docs/design/host-interface.md`, section 15, "Host
//! objects: brand, host data, `trace`, overrides"; `docs/design/engine.md`, decision 9, "Host
//! objects for the DOM"), `story:host-object-overrides`: the arms of the 11 internal methods the
//! dispatch of `object/mod.rs` calls for a host object. Each runs the override of its name where
//! the object's table has one and the ordinary method where it has none (HOST-114).
//!
//! A call of an override: the handles of its answer, in its `Ok` value or its `Abrupt::Throw`
//! value, are checked first (decision 6, "A handle the host returns"; HOST-222), then the answer
//! against the invariants of the essential internal methods
//! (`#sec-invariants-of-the-essential-internal-methods`) with the checks of the Proxy internal
//! methods (`object/invariants.rs`), the facts the engine recorded of the object's earlier
//! answers in place of a target's (HOST-112, HOST-113). A host object has no target, so the
//! engine records, in a box of the object, the facts an invariant holds it to: each property an
//! answer of `get_own_property` reported non-configurable, with its descriptor; that it reported
//! itself not extensible; then the prototype and the own keys it reported. What a method without
//! an override answers is a fact as it stands: the record's own properties while the table has
//! no `get_own_property`, its [[Extensible]] while it has no `is_extensible`, its [[Prototype]]
//! while it has no `get_prototype_of`. A broken invariant, a descriptor that is not complete or
//! a stale or foreign handle ends the run in `Terminated(EngineFault)` and faults the agent
//! (HOST-48), never a `TypeError` (section 15, "Rejected"). An override that answers
//! `Abrupt::Throw` throws its value after the check of its handles.
//!
//! Bounds. Steps: each override call spends one step before it runs. Depth: each is one
//! re-entry for its duration, counted in `Interp::proxy_nesting` beside the methods of proxies:
//! at its entry the activations plus that count plus this one passing `reentry_depth` throw the
//! RangeError of "Depth bounds" (`docs/design/engine.md`, decision 3) before the override runs,
//! so overrides that call each other through `Cx` end there, not in a Rust stack overflow. A
//! `Cx` call inside an override spends from the entry that called the internal method
//! (`Agent::begin`). Memory: the box of facts and the growth of each of its tables are charged
//! before they are made and recorded in the object's `Grown`, credited when the object is freed;
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
//! prototype and the keys recorded.

use alloc::boxed::Box;
use alloc::vec::Vec;
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

/// The own keys the object reported once it was not extensible, each with itself for the
/// tracer.
type KeySet = KeyedMap<Key, Key>;

/// The facts the answers of a host object's overrides fixed (the module's documentation).
pub(crate) struct Facts {
    /// Each property `get_own_property` reported non-configurable, as last reported.
    fixed: FixedMap,
    /// Whether `is_extensible` answered false or `prevent_extensions` true.
    sealed: bool,
    /// The prototype reported while the object was not extensible.
    prototype: Option<Option<ObjectRef>>,
    /// Whether `keys` holds the own keys reported while the object was not extensible.
    listed: bool,
    keys: KeySet,
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
        }
    }

    /// What the facts hold, for the kind site of host objects: the key and the values of each
    /// property, the prototype, and the keys.
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

/// The table of the keys of the facts.
fn keys_of(facts: &mut Facts) -> &mut KeySet {
    &mut facts.keys
}

/// The bytes of the box of facts, charged when it is made.
fn facts_bytes() -> u64 {
    u64::try_from(size_of::<Facts>()).unwrap_or(u64::MAX)
}

/// What the engine knows of an own property of a host object with a table.
#[derive(Clone, Copy, Debug)]
enum Known {
    /// The property, or that there is none.
    Is(Option<Property>),
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
        self.room_in(object, fixed_of, 1)?;
        if self
            .with_facts(object, |facts| facts.fixed.insert(key, (key, property)))?
            .is_err()
        {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(())
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
    /// extensible and listed its keys, whether it listed `key`.
    fn known_property(
        &mut self,
        object: ObjectRef,
        table: &Overrides<H>,
        key: Key,
    ) -> Completion<Known> {
        if table.get_own_property.is_none() {
            return Ok(Known::Is(self.ordinary_get_own_property(object, key)?));
        }
        let extensible = self.known_extensible(object, table)?;
        let Some(facts) = self.facts(object) else {
            return Ok(Known::Unknown);
        };
        if let Some(&(_, property)) = facts.fixed.get(&key) {
            return Ok(Known::Is(Some(property)));
        }
        if extensible == Some(false) && facts.listed {
            return Ok(if facts.keys.get(&key).is_some() {
                Known::Listed
            } else {
                Known::Is(None)
            });
        }
        Ok(Known::Unknown)
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
            _ => self.ordinary_set_prototype_of(object, prototype),
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
        if self.known_extensible(object, table)? == Some(false) {
            match self.known_prototype(object, table)? {
                Some(recorded) => {
                    if invariants::set_prototype_of_after_prototype(prototype, recorded).is_err() {
                        return Err(self.fault());
                    }
                }
                None => self.with_facts(object, |facts| facts.prototype = Some(prototype))?,
            }
        }
        Ok(true)
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
            _ => self.ordinary_prevent_extensions(object),
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
        match self.known_extensible(object, table)? {
            Some(known) => {
                if invariants::prevent_extensions_after_extensible(known).is_err() {
                    return Err(self.fault());
                }
            }
            None => self.with_facts(object, |facts| facts.sealed = true)?,
        }
        Ok(true)
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
    /// property (I7, HOST-112) or a listed key of an object that is not extensible (I8); a
    /// descriptor compatible with the fixed one, or new only while the object may be extended
    /// (I9). A non-configurable answer is recorded.
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
            let current = match known {
                Known::Is(current) => current,
                Known::Listed => return Err(self.fault()),
                Known::Unknown => return Ok(None),
            };
            match invariants::get_own_property_undefined_after_descriptor(current) {
                Ok(Next::ReadExtensible) => {
                    let extensible = self.known_extensible(object, table)?.unwrap_or(true);
                    if invariants::get_own_property_undefined_after_extensible(extensible).is_err()
                    {
                        return Err(self.fault());
                    }
                }
                Ok(_) => {}
                Err(_) => return Err(self.fault()),
            }
            return Ok(None);
        };
        if let Known::Is(current) = known {
            let extensible = self.known_extensible(object, table)?.unwrap_or(true);
            let descriptor = invariants::descriptor_of(result);
            if !invariants::is_compatible_property_descriptor(
                self, extensible, descriptor, current,
            )? {
                return Err(self.fault());
            }
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
            _ => self.ordinary_define_own_property(object, key, descriptor),
        }
    }

    /// The `define_own_property` override: true is checked by I12 to I16 against what the
    /// engine knows of the property.
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
        if let Known::Is(current) = self.known_property(object, table, key)? {
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

    /// The `has_property` override: false for a fixed property, or for a property of an object
    /// that is not extensible, breaks the invariant.
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
            Known::Is(current) => match invariants::has_after_descriptor(current) {
                Ok(Next::ReadExtensible) => {
                    let extensible = self.known_extensible(object, table)?.unwrap_or(true);
                    if invariants::has_after_extensible(extensible).is_err() {
                        return Err(self.fault());
                    }
                }
                Ok(_) => {}
                Err(_) => return Err(self.fault()),
            },
            Known::Listed => return Err(self.fault()),
            Known::Unknown => {}
        }
        Ok(false)
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
        if let Known::Is(current) = self.known_property(object, table, key)?
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
        if let Known::Is(current) = self.known_property(object, table, key)?
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
            _ => self.ordinary_delete(object, key),
        }
    }

    /// The `delete` override: true for a fixed property, or for a property of an object that
    /// is not extensible, breaks the invariant.
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
        match self.known_property(object, table, key)? {
            Known::Is(current) => match invariants::delete_after_descriptor(current) {
                Ok(Next::ReadExtensible) => {
                    let extensible = self.known_extensible(object, table)?.unwrap_or(true);
                    if invariants::delete_after_extensible(extensible).is_err() {
                        return Err(self.fault());
                    }
                }
                Ok(_) => {}
                Err(_) => return Err(self.fault()),
            },
            Known::Listed => return Err(self.fault()),
            Known::Unknown => {}
        }
        Ok(true)
    }

    /// [[OwnPropertyKeys]] of a host object.
    pub(crate) fn host_own_property_keys(&mut self, object: ObjectRef) -> Completion<KeyList> {
        match self.override_table(object)? {
            Some(table) if table.own_property_keys.is_some() => self.in_override(object, |agent| {
                agent.override_own_property_keys(object, table)
            }),
            _ => self.ordinary_own_property_keys(object),
        }
    }

    /// The `own_property_keys` override: each key's handle checked (HOST-222), then the list
    /// answered made, charged as every list of keys is, a step per key before it is read and
    /// made a key; then the checks, credited on an abrupt completion.
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
        let bytes = KeyedMap::<Key, ()>::table_bytes_for(answer.len());
        let checked = match self.charge_bytes(bytes) {
            Ok(()) => {
                let mut seen = KeyedMap::<Key, ()>::with_hasher(self.heap.hasher);
                let mut charged = bytes;
                let checked = self.override_keys_check(
                    object,
                    table,
                    &answer,
                    &mut seen,
                    &mut charged,
                    &mut list.keys,
                );
                self.heap.accounting.credit(charged);
                checked
            }
            Err(abrupt) => Err(abrupt),
        };
        match checked {
            Ok(()) => Ok(list),
            Err(abrupt) => {
                self.drop_key_list(list);
                Err(abrupt)
            }
        }
    }

    /// The checks of an answer of `own_property_keys`, with `seen`, the map of its keys,
    /// `charged` bytes so far: each key made a key and pushed to `keys`, a step before each, a
    /// key listed twice breaks the invariant (I18); each fixed key must be listed (I19), a step
    /// per key; an object that is not extensible lists exactly the keys it listed first (I20,
    /// I21), a step per key, or its first list is recorded.
    fn override_keys_check(
        &mut self,
        object: ObjectRef,
        table: &'static Overrides<H>,
        answer: &[PropertyKey],
        seen: &mut KeyedMap<Key, ()>,
        charged: &mut u64,
        keys: &mut Vec<Key>,
    ) -> Completion<()> {
        if seen.reserve(answer.len()).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let table_bytes = seen.table_bytes();
        if table_bytes > *charged {
            // The map took more than its layout says: charged now.
            self.charge_bytes(table_bytes - *charged)?;
            *charged = table_bytes;
        }
        for &key in answer {
            self.charge_steps(1)?;
            let key = self.key_of(key)?;
            match seen.insert(key, ()) {
                Ok(None) => keys.push(key),
                Ok(Some(())) => return Err(self.fault()),
                Err(_) => return Err(self.terminate(Termination::MemoryBudget)),
            }
        }
        let fixed = self.facts(object).map_or(0, |facts| facts.fixed.len());
        self.charge_steps(u64::try_from(fixed).unwrap_or(u64::MAX))?;
        let missing = self
            .facts(object)
            .is_some_and(|facts| facts.fixed.values().any(|(key, _)| seen.get(key).is_none()));
        if missing {
            return Err(self.fault());
        }
        if self.known_extensible(object, table)? != Some(false) {
            return Ok(());
        }
        let listed = self
            .facts(object)
            .map(|facts| (facts.listed, facts.keys.len()));
        match listed {
            Some((true, count)) => {
                self.charge_steps(u64::try_from(count).unwrap_or(u64::MAX))?;
                let same = count == seen.len()
                    && self.facts(object).is_some_and(|facts| {
                        facts.keys.values().all(|key| seen.get(key).is_some())
                    });
                if !same {
                    return Err(self.fault());
                }
                Ok(())
            }
            _ => self.record_keys(object, keys),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Facts, Overridden};
    use crate::host::host_object::HostObject;
    use crate::object::property::{Key, Property};

    /// The bytes of one fact in the table of fixed properties: a bucket's entry, the key and
    /// the property with its key; `tests/host_overrides.rs` adds the control byte of the bucket
    /// (`FACT_BYTES`, 73). The boxes are charged with their own bytes, printed here with the
    /// kind part of a host object's record.
    #[test]
    fn a_fact_has_the_bytes_the_tests_name() {
        std::println!(
            "fact {} bytes, box of facts {} bytes, box of a table {} bytes, HostObject {} bytes",
            size_of::<Option<(Key, (Key, Property))>>(),
            size_of::<Facts>(),
            size_of::<Overridden>(),
            size_of::<HostObject>()
        );
        assert_eq!(size_of::<Option<(Key, (Key, Property))>>(), 72);
    }
}

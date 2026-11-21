//! Objects (ECMA-262 10.1 to 10.4, `#sec-ordinary-and-exotic-objects-behaviours`): the record
//! every object has, its kind, and the internal-method dispatch over the kinds (6.1.7.2,
//! `#sec-object-internal-methods-and-internal-slots`). `story:heap-values-and-host-surface` wrote
//! the record with [[Prototype]], the kinds, and the collector's kind site with its host object
//! arm; `story:object-model-and-realms` adds [[Extensible]], the properties, the internal methods
//! and their dispatch arms, and one marking arm per kind beside them.
//!
//! Every kind but the Array and String exotic objects uses the ordinary internal methods
//! (`object/ordinary.rs`); host objects too, until the overrides of slice 9. [[Call]] and
//! [[Construct]] have an arm for each function kind, the bound function and ECMAScript function
//! arms calling into files their own stories fill.
//!
//! The growth of an object's property map is charged to the memory budget when it grows and
//! credited when the object is freed: the record keeps what it was charged in [`Grown`], whose
//! drop queues the bytes for the agent to credit after the sweep.

pub(crate) mod array;
pub(crate) mod bound_function;
pub(crate) mod builtin_function;
pub(crate) mod ecmascript_function;
pub(crate) mod ordinary;
pub(crate) mod property;
pub(crate) mod string;

use std::cell::Cell;
use std::rc::Rc;

use crate::heap::collect::Mark;
use crate::host::agent::Agent;
use crate::host::host_object::HostObject;
use crate::host::{Abrupt, Completion, Host, Termination};
use crate::value::{ObjectRef, Value};
use array::Array;
use bound_function::BoundFunction;
use builtin_function::BuiltinFunction;
use ecmascript_function::EcmascriptFunction;
use ordinary::Ordinary;
use property::{Descriptor, Key, Properties, Property};
use string::StringObject;

/// The bytes an object's property map was charged as it grew, credited when the record is
/// dropped: the drop adds them to the agent's queue, which the agent credits after a sweep.
#[derive(Default)]
pub(crate) struct Grown {
    bytes: u64,
    queue: Option<Rc<Cell<u64>>>,
}

impl Grown {
    /// Record `bytes` more, to be credited through `queue`.
    fn add(&mut self, bytes: u64, queue: &Rc<Cell<u64>>) {
        self.bytes = self.bytes.saturating_add(bytes);
        if self.queue.is_none() {
            self.queue = Some(Rc::clone(queue));
        }
    }
}

impl Drop for Grown {
    fn drop(&mut self) {
        if let Some(queue) = &self.queue {
            queue.set(queue.get().saturating_add(self.bytes));
        }
    }
}

/// The record of every object.
pub(crate) struct ObjectRecord {
    /// [[Prototype]].
    pub(crate) prototype: Option<ObjectRef>,
    /// [[Extensible]].
    pub(crate) extensible: bool,
    pub(crate) properties: Properties,
    pub(crate) kind: ObjectKind,
    grown: Grown,
}

/// What kind of object a record is, with the slots of that kind.
pub(crate) enum ObjectKind {
    /// An ordinary object (10.1), and the Boolean, Number and Error objects.
    Ordinary(Ordinary),
    /// An Array exotic object (10.4.2).
    Array(Array),
    /// A String exotic object (10.4.3).
    String(StringObject),
    /// A built-in function object (10.3), host functions included.
    BuiltinFunction(BuiltinFunction),
    /// A bound function exotic object (10.4.1).
    #[cfg_attr(not(test), expect(dead_code, reason = "story:harness-builtins"))]
    BoundFunction(BoundFunction),
    /// An ECMAScript function object (10.2).
    EcmascriptFunction(EcmascriptFunction),
    /// A host object (`docs/design/host-interface.md`, decision 15).
    Host(HostObject),
}

/// Which kind an object is, for the dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KindTag {
    Ordinary,
    ImmutablePrototype,
    Array,
    String,
    BuiltinFunction,
    BoundFunction,
    EcmascriptFunction,
    Host,
}

impl ObjectRecord {
    /// A new extensible object with no property.
    pub(crate) fn new(prototype: Option<ObjectRef>, kind: ObjectKind) -> ObjectRecord {
        ObjectRecord {
            prototype,
            extensible: true,
            properties: Properties::default(),
            kind,
            grown: Grown::default(),
        }
    }

    /// Kind site of the collector: [[Prototype]], the property keys and values, then the slots
    /// of the object's kind.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/mod.rs ObjectRecord::trace");
        if let Some(prototype) = self.prototype {
            mark.object(prototype);
        }
        self.properties.trace(mark);
        match &self.kind {
            ObjectKind::Ordinary(ordinary) => ordinary.trace(mark),
            ObjectKind::Array(array) => array.trace(mark),
            ObjectKind::String(string) => string.trace(mark),
            ObjectKind::BuiltinFunction(function) => function.trace(mark),
            ObjectKind::BoundFunction(function) => function.trace(mark),
            ObjectKind::EcmascriptFunction(function) => function.trace(mark),
            ObjectKind::Host(host) => host.trace(mark),
        }
    }

    /// The kind, for the dispatch.
    pub(crate) fn tag(&self) -> KindTag {
        match &self.kind {
            ObjectKind::Ordinary(ordinary) if ordinary.has_immutable_prototype() => {
                KindTag::ImmutablePrototype
            }
            ObjectKind::Ordinary(_) => KindTag::Ordinary,
            ObjectKind::Array(_) => KindTag::Array,
            ObjectKind::String(_) => KindTag::String,
            ObjectKind::BuiltinFunction(_) => KindTag::BuiltinFunction,
            ObjectKind::BoundFunction(_) => KindTag::BoundFunction,
            ObjectKind::EcmascriptFunction(_) => KindTag::EcmascriptFunction,
            ObjectKind::Host(_) => KindTag::Host,
        }
    }
}

impl<H: Host> Agent<H> {
    /// The record of `object`, for the engine's own use inside an entry. A handle that reaches
    /// no record here is an engine bug: the run ends in `Terminated(EngineFault)` and the agent
    /// is faulted (`docs/design/engine.md`, decision 4).
    pub(crate) fn engine_object(&mut self, object: ObjectRef) -> Completion<&ObjectRecord> {
        if self.heap.objects.get(object.handle).is_err() {
            return Err(self.fault());
        }
        // The check above passed, so this answers the record.
        self.heap
            .objects
            .get(object.handle)
            .map_err(|_| Abrupt::Terminated(Termination::EngineFault))
    }

    /// As [`Agent::engine_object`], to change.
    pub(crate) fn engine_object_mut(&mut self, object: ObjectRef) -> Completion<&mut ObjectRecord> {
        if self.heap.objects.get(object.handle).is_err() {
            return Err(self.fault());
        }
        self.heap
            .objects
            .get_mut(object.handle)
            .map_err(|_| Abrupt::Terminated(Termination::EngineFault))
    }

    /// The kind of `object`.
    pub(crate) fn kind_of(&mut self, object: ObjectRef) -> Completion<KindTag> {
        Ok(self.engine_object(object)?.tag())
    }

    /// Store a new object, ending the run in `Terminated(MemoryBudget)` when it does not fit.
    pub(crate) fn allocate_object(
        &mut self,
        prototype: Option<ObjectRef>,
        kind: ObjectKind,
    ) -> Completion<ObjectRef> {
        match self.heap.new_object(ObjectRecord::new(prototype, kind), 0) {
            Ok(object) => Ok(object),
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    /// Add a property `object` does not have, growing its map first when it is full. The growth
    /// is charged before it is made; over the budget the run ends in
    /// `Terminated(MemoryBudget)`.
    pub(crate) fn add_property(
        &mut self,
        object: ObjectRef,
        key: Key,
        property: Property,
    ) -> Completion<()> {
        let (full, capacity) = {
            let record = self.engine_object(object)?;
            (record.properties.is_full(), record.properties.capacity())
        };
        if full {
            let wanted = capacity.saturating_mul(2).max(4);
            let added = u64::try_from(wanted.saturating_sub(capacity)).unwrap_or(u64::MAX);
            let bytes = added.saturating_mul(Properties::entry_bytes());
            if self.heap.accounting.charge(bytes).is_err() {
                return Err(self.terminate(Termination::MemoryBudget));
            }
            let queue = Rc::clone(&self.credit);
            let reserved = match self.heap.objects.get_mut(object.handle) {
                Ok(record) => {
                    let additional = wanted.saturating_sub(record.properties.len());
                    let reserved = record.properties.reserve(additional).is_ok();
                    if reserved {
                        record.grown.add(bytes, &queue);
                    }
                    reserved
                }
                Err(_) => false,
            };
            if !reserved {
                self.heap.accounting.credit(bytes);
                return Err(self.terminate(Termination::MemoryBudget));
            }
        }
        self.engine_object_mut(object)?
            .properties
            .push(key, property);
        Ok(())
    }

    /// [[GetPrototypeOf]] (`#sec-ordinary-object-internal-methods-and-internal-slots-getprototypeof`).
    pub(crate) fn get_prototype_of(&mut self, object: ObjectRef) -> Completion<Option<ObjectRef>> {
        self.ordinary_get_prototype_of(object)
    }

    /// [[SetPrototypeOf]]: SetImmutablePrototype for `%Object.prototype%` (10.4.7.1,
    /// `#sec-immutable-prototype-exotic-objects-setprototypeof-v`), ordinary for the others.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "Object.setPrototypeOf: story:harness-builtins")
    )]
    pub(crate) fn set_prototype_of(
        &mut self,
        object: ObjectRef,
        prototype: Option<ObjectRef>,
    ) -> Completion<bool> {
        if self.kind_of(object)? == KindTag::ImmutablePrototype {
            // SetImmutablePrototype: "Let current be ? O.[[GetPrototypeOf]](). If SameValue(V,
            // current) is true, return true. Return false."
            return Ok(self.get_prototype_of(object)? == prototype);
        }
        self.ordinary_set_prototype_of(object, prototype)
    }

    /// [[IsExtensible]].
    pub(crate) fn is_extensible(&mut self, object: ObjectRef) -> Completion<bool> {
        self.ordinary_is_extensible(object)
    }

    /// [[PreventExtensions]].
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "Object.preventExtensions: story:harness-builtins")
    )]
    pub(crate) fn prevent_extensions(&mut self, object: ObjectRef) -> Completion<bool> {
        self.ordinary_prevent_extensions(object)
    }

    /// [[GetOwnProperty]]: the String exotic method (10.4.3.1) for String objects, ordinary for
    /// the others.
    pub(crate) fn get_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
    ) -> Completion<Option<Property>> {
        if self.kind_of(object)? == KindTag::String {
            return self.string_get_own_property_method(object, key);
        }
        self.ordinary_get_own_property(object, key)
    }

    /// [[DefineOwnProperty]]: the Array (10.4.2.1) and String (10.4.3.2) exotic methods, ordinary
    /// for the others.
    pub(crate) fn define_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        match self.kind_of(object)? {
            KindTag::Array => self.array_define_own_property(object, key, descriptor),
            KindTag::String => self.string_define_own_property(object, key, descriptor),
            _ => self.ordinary_define_own_property(object, key, descriptor),
        }
    }

    /// [[HasProperty]].
    pub(crate) fn has_property(&mut self, object: ObjectRef, key: Key) -> Completion<bool> {
        self.ordinary_has_property(object, key)
    }

    /// [[Get]].
    pub(crate) fn internal_get(
        &mut self,
        object: ObjectRef,
        key: Key,
        receiver: Value,
    ) -> Completion<Value> {
        self.ordinary_get(object, key, receiver)
    }

    /// [[Set]].
    pub(crate) fn internal_set(
        &mut self,
        object: ObjectRef,
        key: Key,
        value: Value,
        receiver: Value,
    ) -> Completion<bool> {
        self.ordinary_set(object, key, value, receiver)
    }

    /// [[Delete]].
    pub(crate) fn internal_delete(&mut self, object: ObjectRef, key: Key) -> Completion<bool> {
        self.ordinary_delete(object, key)
    }

    /// [[OwnPropertyKeys]]: the String exotic method (10.4.3.3) for String objects, ordinary for
    /// the others.
    pub(crate) fn own_property_keys(&mut self, object: ObjectRef) -> Completion<Vec<Key>> {
        if self.kind_of(object)? == KindTag::String {
            return self.string_own_property_keys(object);
        }
        self.ordinary_own_property_keys(object)
    }

    /// [[Call]] of a function object. The caller has checked IsCallable.
    pub(crate) fn internal_call(
        &mut self,
        function: ObjectRef,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        match self.kind_of(function)? {
            KindTag::BuiltinFunction => self.builtin_call(function, this, args, None),
            KindTag::BoundFunction => bound_function::call(self, function, this, args),
            KindTag::EcmascriptFunction => ecmascript_function::call(self, function, this, args),
            _ => Err(self.fault()),
        }
    }

    /// [[Construct]] of a constructor. The caller has checked IsConstructor of `function` and
    /// of `new_target`.
    pub(crate) fn internal_construct(
        &mut self,
        function: ObjectRef,
        args: &[Value],
        new_target: ObjectRef,
    ) -> Completion<Value> {
        match self.kind_of(function)? {
            KindTag::BuiltinFunction => {
                self.builtin_call(function, Value::Undefined, args, Some(new_target))
            }
            KindTag::BoundFunction => bound_function::construct(self, function, args, new_target),
            KindTag::EcmascriptFunction => {
                ecmascript_function::construct(self, function, args, new_target)
            }
            _ => Err(self.fault()),
        }
    }

    /// Whether `object` has a [[Call]] internal method.
    pub(crate) fn has_call(&mut self, object: ObjectRef) -> Completion<bool> {
        Ok(match &self.engine_object(object)?.kind {
            ObjectKind::BuiltinFunction(_) => true,
            ObjectKind::BoundFunction(function) => function.has_call(),
            ObjectKind::EcmascriptFunction(function) => function.has_call(),
            _ => false,
        })
    }

    /// Whether `object` has a [[Construct]] internal method.
    pub(crate) fn has_construct(&mut self, object: ObjectRef) -> Completion<bool> {
        Ok(match &self.engine_object(object)?.kind {
            ObjectKind::BuiltinFunction(function) => function.constructor,
            ObjectKind::BoundFunction(function) => function.has_construct(),
            ObjectKind::EcmascriptFunction(function) => function.has_construct(),
            _ => false,
        })
    }
}

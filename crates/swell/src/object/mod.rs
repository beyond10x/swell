//! Objects (ECMA-262 10.1 to 10.4, `#sec-ordinary-and-exotic-objects-behaviours`): the record
//! every object has, its kind, and the internal-method dispatch over the kinds (6.1.7.2,
//! `#sec-object-internal-methods-and-internal-slots`). `story:heap-values-and-host-surface` wrote
//! the record with [[Prototype]], the kinds, and the collector's kind site with its host object
//! arm; `story:object-model-and-realms` adds [[Extensible]], the properties, the internal methods
//! and their dispatch arms, and one marking arm per kind beside them.
//! `story:regexp-objects-exec-and-literals` adds the RegExp kind (`object/regexp.rs`), which owns
//! the program of its pattern. `story:promise-constructor-then-and-jobs` adds the Promise kind
//! (`object/promise.rs`), which takes every internal method by its ordinary arm, and
//! `story:async-functions-and-await` the activation of an `async` function
//! (`object/async_function.rs`), which does too, as does the activation of a built-in async
//! function (`object/async_builtin.rs`, `story:array-from-async`).
//!
//! Every kind but the Array, String, arguments and Proxy exotic objects uses the ordinary internal
//! methods (`object/ordinary.rs`); a host object runs the overrides its table gives and the
//! ordinary methods it does not (`object/host_overrides.rs`). [[Call]] and
//! [[Construct]] have an arm for each function kind, the bound function and ECMAScript function
//! arms calling into files their own stories fill.
//!
//! The growth of an object's property map (its list and, above the small size, its index map,
//! `object/property.rs`) and of an Array's dense elements (`object/array.rs`) is charged to the
//! memory budget before it is made and credited when the object is freed: the record keeps what
//! it was charged in [`Grown`], whose drop queues the bytes for the agent to credit after the
//! sweep. `story:property-index-and-dense-elements` added the map, the dense elements and the
//! Array arms of [[GetOwnProperty]], [[Delete]] and [[OwnPropertyKeys]].
//!
//! A list of property keys is charged before it exists and credited when it drops
//! (`docs/design/engine.md`, "The step budget" and "Memory budget";
//! `story:own-keys-charged-before-the-list-exists`): each [[OwnPropertyKeys]] charges a step per
//! key it will answer, counted from lengths it holds, then the list's bytes ([`key_list_bytes`])
//! before it reserves the list, and answers a [`KeyList`], the keys with the bytes they were
//! charged. The code that holds the list credits them when it drops it
//! ([`Agent::drop_key_list`]) or takes them over into a charge of its own. Index keys added out
//! of order are sorted by a bottom-up merge sort over a second buffer charged with the list, n
//! steps charged before each pass ([`Agent::listed_indices`]).

use alloc::vec::Vec;

pub(crate) mod arguments;
pub(crate) mod array;
pub(crate) mod async_builtin;
pub(crate) mod async_function;
pub(crate) mod async_generator;
pub(crate) mod bound_function;
pub(crate) mod builtin_function;
pub(crate) mod class_constructor;
pub(crate) mod collection;
pub(crate) mod ecmascript_function;
pub(crate) mod generator;
pub(crate) mod host_overrides;
pub(crate) mod invariants;
pub(crate) mod keyed_entries;
pub(crate) mod ordinary;
pub(crate) mod private_names;
pub(crate) mod promise;
pub(crate) mod property;
pub(crate) mod proxy;
pub(crate) mod regexp;
pub(crate) mod string;
pub(crate) mod weak_collection;

use alloc::rc::Rc;
use core::cell::Cell;

use crate::heap::collect::Mark;
use crate::host::agent::Agent;
use crate::host::host_object::HostObject;
use crate::host::{Abrupt, Completion, Host, Termination};
use crate::value::{ObjectRef, Value};
use arguments::ArgumentsObject;
use array::Array;
use async_builtin::AsyncBuiltinActivation;
use async_function::AsyncActivation;
use async_generator::AsyncGenerator;
use bound_function::BoundFunction;
use builtin_function::BuiltinFunction;
use ecmascript_function::EcmascriptFunction;
use generator::Generator;
use ordinary::Ordinary;
use promise::Promise;
use property::{Descriptor, Key, Properties, Property, Room, sort_indices};
use proxy::ProxyObject;
use regexp::RegExpObject;
use string::StringObject;

/// The bytes of a list of `count` property keys: what every list of keys is charged.
pub(crate) fn key_list_bytes(count: usize) -> u64 {
    u64::try_from(count.saturating_mul(size_of::<Key>())).unwrap_or(u64::MAX)
}

/// A list of property keys with the bytes it was charged to the memory budget: what
/// [[OwnPropertyKeys]] answers, and the lists made from its answer. The code that holds it
/// credits the bytes when it drops it ([`Agent::drop_key_list`]), or takes them over into a
/// charge of its own.
pub(crate) struct KeyList {
    pub(crate) keys: Vec<Key>,
    pub(crate) bytes: u64,
}

impl core::ops::Deref for KeyList {
    type Target = [Key];

    fn deref(&self) -> &[Key] {
        &self.keys
    }
}

/// The index keys of an object's property list in ascending order: read in place from the list
/// (added ascending), or sorted into a buffer charged `bytes` (with the merge's second buffer).
enum ListedIndices {
    InPlace,
    Sorted { indices: Vec<u32>, bytes: u64 },
}

impl ListedIndices {
    /// The bytes still charged for the buffers.
    fn bytes(&self) -> u64 {
        match self {
            ListedIndices::InPlace => 0,
            ListedIndices::Sorted { bytes, .. } => *bytes,
        }
    }
}

/// Push to `keys` the index keys of `properties` from `from` on, ascending, read through
/// `listed`, then its String keys and its Symbol keys in creation order.
fn push_listed_keys(
    properties: &Properties,
    listed: &ListedIndices,
    from: u32,
    keys: &mut Vec<Key>,
) {
    match listed {
        ListedIndices::InPlace => keys.extend(
            properties
                .listed_indices()
                .filter(|&index| index >= from)
                .map(Key::Index),
        ),
        ListedIndices::Sorted { indices, .. } => keys.extend(
            indices
                .iter()
                .copied()
                .filter(|&index| index >= from)
                .map(Key::Index),
        ),
    }
    keys.extend(properties.names());
}

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
    /// [[PrivateElements]] (`#sec-privateelement-specification-type`; `docs/design/engine.md`,
    /// "Private names", item 2), `None` until the first is added (`object/private_names.rs`):
    /// every kind of object has it, since the adds act on the object itself, through no trap.
    /// The box and its list are charged before they grow and recorded in `grown`.
    pub(crate) private_elements: Option<alloc::boxed::Box<private_names::PrivateElements>>,
    grown: Grown,
}

/// What kind of object a record is, with the slots of that kind.
pub(crate) enum ObjectKind {
    /// An ordinary object (10.1), and the Boolean, Number, Symbol and Error objects and the
    /// Array and String Iterators, each with its slots in [`ordinary::OrdinaryData`].
    Ordinary(Ordinary),
    /// An Array exotic object (10.4.2).
    Array(Array),
    /// A String exotic object (10.4.3).
    String(StringObject),
    /// An arguments exotic object (10.4.4), and an unmapped arguments object.
    Arguments(ArgumentsObject),
    /// A built-in function object (10.3), host functions included.
    BuiltinFunction(BuiltinFunction),
    /// A bound function exotic object (10.4.1).
    BoundFunction(BoundFunction),
    /// An ECMAScript function object (10.2).
    EcmascriptFunction(EcmascriptFunction),
    /// A host object (`docs/design/host-interface.md`, decision 15).
    Host(HostObject),
    /// A RegExp object (22.2.7), `story:regexp-objects-exec-and-literals`.
    RegExp(RegExpObject),
    /// A generator object (27.5), `story:generator-functions-and-suspended-frames`.
    Generator(Generator),
    /// A Promise instance (27.5.6), `story:promise-constructor-then-and-jobs`.
    Promise(Promise),
    /// A Map or a Set instance (24.1.4, 24.2.5), with [[MapData]] or [[SetData]] in a box of
    /// its kind, `story:map-and-set` (`object/collection.rs`).
    Collection(alloc::boxed::Box<collection::Collection>),
    /// A Map or Set Iterator (24.1.5, 24.2.6), `story:map-and-set` (`object/collection.rs`).
    CollectionIterator(collection::CollectionIterator),
    /// The activation of an `async` function (`docs/design/engine.md`, decision 3, "`async`
    /// functions on suspended frames", item 1), `story:async-functions-and-await`.
    AsyncActivation(AsyncActivation),
    /// An Async-from-Sync Iterator object (27.1.5.3), with [[SyncIteratorRecord]] in a box of
    /// its kind, `story:for-await-and-async-from-sync-iterator` (`builtins/async_iteration.rs`).
    AsyncFromSyncIterator(crate::builtins::async_iteration::AsyncFromSyncIterator),
    /// An async generator object (27.6), with its frame and its request queue in boxes of its
    /// kind (`docs/design/engine.md`, decision 3, "Async generators on suspended frames"),
    /// `story:async-generators-and-their-request-queue`.
    AsyncGenerator(AsyncGenerator),
    /// The activation of a built-in async function (`docs/design/engine.md`, decision 3, "A
    /// built-in async function", item 1), `story:array-from-async` (`object/async_builtin.rs`).
    AsyncBuiltinActivation(AsyncBuiltinActivation),
    /// A Proxy exotic object (10.5), `story:proxy-objects-and-access-traps`
    /// (`object/proxy.rs`).
    Proxy(ProxyObject),
    /// A WeakMap or a WeakSet instance (24.3.4, 24.4.4), with [[WeakMapData]] or
    /// [[WeakSetData]] in a box of its kind, `story:weak-map-and-weak-set`
    /// (`object/weak_collection.rs`).
    WeakCollection(alloc::boxed::Box<weak_collection::WeakCollection>),
}

/// Which kind an object is, for the dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KindTag {
    Ordinary,
    ImmutablePrototype,
    Array,
    String,
    Arguments,
    BuiltinFunction,
    BoundFunction,
    EcmascriptFunction,
    Host,
    RegExp,
    Generator,
    Promise,
    Collection,
    CollectionIterator,
    AsyncActivation,
    AsyncFromSyncIterator,
    AsyncGenerator,
    AsyncBuiltinActivation,
    Proxy,
    WeakCollection,
}

impl ObjectRecord {
    /// A new extensible object with no property.
    pub(crate) fn new(prototype: Option<ObjectRef>, kind: ObjectKind) -> ObjectRecord {
        ObjectRecord {
            prototype,
            extensible: true,
            properties: Properties::default(),
            kind,
            private_elements: None,
            grown: Grown::default(),
        }
    }

    /// Kind site of the collector: [[Prototype]], the property keys and values, the private
    /// elements (each one's Private Name and its values), then the slots of the object's kind.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/mod.rs ObjectRecord::trace");
        if let Some(prototype) = self.prototype {
            mark.object(prototype);
        }
        self.properties.trace(mark);
        if let Some(elements) = &self.private_elements {
            elements.trace(mark);
        }
        match &self.kind {
            ObjectKind::Ordinary(ordinary) => ordinary.trace(mark),
            ObjectKind::Array(array) => array.trace(mark),
            ObjectKind::String(string) => string.trace(mark),
            ObjectKind::Arguments(arguments) => arguments.trace(mark),
            ObjectKind::BuiltinFunction(function) => function.trace(mark),
            ObjectKind::BoundFunction(function) => function.trace(mark),
            ObjectKind::EcmascriptFunction(function) => function.trace(mark),
            ObjectKind::Host(host) => host.trace(mark),
            ObjectKind::RegExp(regexp) => regexp.trace(mark),
            ObjectKind::Generator(generator) => generator.trace(mark),
            ObjectKind::Promise(promise) => promise.trace(mark),
            ObjectKind::Collection(collection) => collection.trace(mark),
            ObjectKind::CollectionIterator(iterator) => iterator.trace(mark),
            ObjectKind::AsyncActivation(activation) => activation.trace(mark),
            ObjectKind::AsyncFromSyncIterator(iterator) => iterator.trace(mark),
            ObjectKind::AsyncGenerator(generator) => generator.trace(mark),
            ObjectKind::AsyncBuiltinActivation(activation) => activation.trace(mark),
            ObjectKind::Proxy(proxy) => proxy.trace(mark),
            ObjectKind::WeakCollection(table) => table.trace(mark),
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
            ObjectKind::Arguments(_) => KindTag::Arguments,
            ObjectKind::BuiltinFunction(_) => KindTag::BuiltinFunction,
            ObjectKind::BoundFunction(_) => KindTag::BoundFunction,
            ObjectKind::EcmascriptFunction(_) => KindTag::EcmascriptFunction,
            ObjectKind::Host(_) => KindTag::Host,
            ObjectKind::RegExp(_) => KindTag::RegExp,
            ObjectKind::Generator(_) => KindTag::Generator,
            ObjectKind::Promise(_) => KindTag::Promise,
            ObjectKind::Collection(_) => KindTag::Collection,
            ObjectKind::CollectionIterator(_) => KindTag::CollectionIterator,
            ObjectKind::AsyncActivation(_) => KindTag::AsyncActivation,
            ObjectKind::AsyncFromSyncIterator(_) => KindTag::AsyncFromSyncIterator,
            ObjectKind::AsyncGenerator(_) => KindTag::AsyncGenerator,
            ObjectKind::AsyncBuiltinActivation(_) => KindTag::AsyncBuiltinActivation,
            ObjectKind::Proxy(_) => KindTag::Proxy,
            ObjectKind::WeakCollection(_) => KindTag::WeakCollection,
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

    /// Add a property `object` does not have, making room in its list first
    /// (`KeyedList::make_room`): the growth of the list and of its index map is charged before
    /// it is made and recorded in [`Grown`]; over the budget the run ends in
    /// `Terminated(MemoryBudget)`. A compaction of the list charges a step per entry it visits,
    /// before it runs.
    pub(crate) fn add_property(
        &mut self,
        object: ObjectRef,
        key: Key,
        property: Property,
    ) -> Completion<()> {
        let compaction = match self.heap.objects.get(object.handle) {
            Ok(record) => record.properties.compaction_visits(),
            Err(_) => return Err(self.fault()),
        };
        if compaction > 0 {
            self.charge_steps(compaction)?;
        }
        let queue = Rc::clone(&self.credit);
        let hasher = self.heap.hasher;
        let mut room = Room::default();
        let made = match self.heap.objects.get_mut(object.handle) {
            Ok(record) => {
                let made =
                    record
                        .properties
                        .make_room(&mut self.heap.accounting, &hasher, &mut room);
                if room.bytes > 0 {
                    record.grown.add(room.bytes, &queue);
                }
                made
            }
            Err(_) => return Err(self.fault()),
        };
        if made.is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let pushed = self
            .engine_object_mut(object)?
            .properties
            .push(key, property);
        if pushed.is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(())
    }

    /// Record `bytes` charged for storage `object` owns beside its list (the dense elements of
    /// an Array), to be credited when it is freed.
    pub(crate) fn record_grown(&mut self, object: ObjectRef, bytes: u64) -> Completion<()> {
        let queue = Rc::clone(&self.credit);
        self.engine_object_mut(object)?.grown.add(bytes, &queue);
        Ok(())
    }

    /// [[GetPrototypeOf]] (`#sec-ordinary-object-internal-methods-and-internal-slots-getprototypeof`).
    pub(crate) fn get_prototype_of(&mut self, object: ObjectRef) -> Completion<Option<ObjectRef>> {
        match self.kind_of(object)? {
            KindTag::Proxy => self.proxy_get_prototype_of(object),
            KindTag::Host => self.host_get_prototype_of(object),
            _ => self.ordinary_get_prototype_of(object),
        }
    }

    /// [[SetPrototypeOf]]: SetImmutablePrototype for `%Object.prototype%` (10.4.7.1,
    /// `#sec-immutable-prototype-exotic-objects-setprototypeof-v`), ordinary for the others.
    pub(crate) fn set_prototype_of(
        &mut self,
        object: ObjectRef,
        prototype: Option<ObjectRef>,
    ) -> Completion<bool> {
        match self.kind_of(object)? {
            KindTag::ImmutablePrototype => {
                // SetImmutablePrototype: "Let current be ? O.[[GetPrototypeOf]](). If
                // SameValue(V, current) is true, return true. Return false."
                Ok(self.get_prototype_of(object)? == prototype)
            }
            KindTag::Proxy => self.proxy_set_prototype_of(object, prototype),
            KindTag::Host => self.host_set_prototype_of(object, prototype),
            _ => self.ordinary_set_prototype_of(object, prototype),
        }
    }

    /// [[IsExtensible]].
    pub(crate) fn is_extensible(&mut self, object: ObjectRef) -> Completion<bool> {
        match self.kind_of(object)? {
            KindTag::Proxy => self.proxy_is_extensible(object),
            KindTag::Host => self.host_is_extensible(object),
            _ => self.ordinary_is_extensible(object),
        }
    }

    /// [[PreventExtensions]].
    pub(crate) fn prevent_extensions(&mut self, object: ObjectRef) -> Completion<bool> {
        match self.kind_of(object)? {
            KindTag::Proxy => self.proxy_prevent_extensions(object),
            KindTag::Host => self.host_prevent_extensions(object),
            _ => self.ordinary_prevent_extensions(object),
        }
    }

    /// [[GetOwnProperty]]: the String (10.4.3.1) and arguments (10.4.4.1) exotic methods,
    /// OrdinaryGetOwnProperty over both storages of an Array (`object/array.rs`), ordinary for
    /// the others.
    pub(crate) fn get_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
    ) -> Completion<Option<Property>> {
        match self.kind_of(object)? {
            KindTag::Array => self.array_get_own_property(object, key),
            KindTag::String => self.string_get_own_property_method(object, key),
            KindTag::Arguments => self.arguments_get_own_property(object, key),
            KindTag::Proxy => self.proxy_get_own_property(object, key),
            KindTag::Host => self.host_get_own_property(object, key),
            _ => self.ordinary_get_own_property(object, key),
        }
    }

    /// [[DefineOwnProperty]]: the Array (10.4.2.1), String (10.4.3.2) and arguments (10.4.4.2)
    /// exotic methods, ordinary for the others.
    pub(crate) fn define_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        match self.kind_of(object)? {
            KindTag::Array => self.array_define_own_property(object, key, descriptor),
            KindTag::String => self.string_define_own_property(object, key, descriptor),
            KindTag::Arguments => self.arguments_define_own_property(object, key, descriptor),
            KindTag::Proxy => self.proxy_define_own_property(object, key, descriptor),
            KindTag::Host => self.host_define_own_property(object, key, descriptor),
            _ => self.ordinary_define_own_property(object, key, descriptor),
        }
    }

    /// [[HasProperty]].
    pub(crate) fn has_property(&mut self, object: ObjectRef, key: Key) -> Completion<bool> {
        match self.kind_of(object)? {
            KindTag::Proxy => self.proxy_has_property(object, key),
            KindTag::Host => self.host_has_property(object, key),
            _ => self.ordinary_has_property(object, key),
        }
    }

    /// [[Get]]: ordinary for every kind; that of an arguments object (10.4.4.3) answers what
    /// OrdinaryGet does (`object/arguments.rs`).
    pub(crate) fn internal_get(
        &mut self,
        object: ObjectRef,
        key: Key,
        receiver: Value,
    ) -> Completion<Value> {
        match self.kind_of(object)? {
            KindTag::Proxy => self.proxy_get(object, key, receiver),
            KindTag::Host => self.host_get(object, key, receiver),
            _ => self.ordinary_get(object, key, receiver),
        }
    }

    /// [[Set]]: ordinary for every kind; that of an arguments object (10.4.4.4) does what
    /// OrdinarySet does (`object/arguments.rs`).
    pub(crate) fn internal_set(
        &mut self,
        object: ObjectRef,
        key: Key,
        value: Value,
        receiver: Value,
    ) -> Completion<bool> {
        match self.kind_of(object)? {
            KindTag::Proxy => self.proxy_set(object, key, value, receiver),
            KindTag::Host => self.host_set(object, key, value, receiver),
            _ => self.ordinary_set(object, key, value, receiver),
        }
    }

    /// [[Delete]]: the arguments exotic method (10.4.4.5), OrdinaryDelete over both storages of
    /// an Array, ordinary for the others.
    pub(crate) fn internal_delete(&mut self, object: ObjectRef, key: Key) -> Completion<bool> {
        match self.kind_of(object)? {
            KindTag::Arguments => self.arguments_delete(object, key),
            KindTag::Array => self.array_delete(object, key),
            KindTag::Proxy => self.proxy_delete(object, key),
            KindTag::Host => self.host_delete(object, key),
            _ => self.ordinary_delete(object, key),
        }
    }

    /// [[OwnPropertyKeys]]: the String exotic method (10.4.3.3) for String objects,
    /// OrdinaryOwnPropertyKeys over both storages of an Array, ordinary for the others. Each
    /// charges its steps and then the list's bytes before the list exists, and answers the
    /// keys with the bytes they were charged, which the caller credits when it drops them.
    pub(crate) fn own_property_keys(&mut self, object: ObjectRef) -> Completion<KeyList> {
        match self.kind_of(object)? {
            KindTag::String => self.string_own_property_keys(object),
            KindTag::Array => self.array_own_property_keys(object),
            KindTag::Proxy => self.proxy_own_property_keys(object),
            KindTag::Host => self.host_own_property_keys(object),
            _ => self.ordinary_own_property_keys(object),
        }
    }

    /// Charge `bytes` to the memory budget; over it the run ends in
    /// `Terminated(MemoryBudget)` and nothing is charged.
    pub(crate) fn charge_bytes(&mut self, bytes: u64) -> Completion<()> {
        if self.heap.accounting.charge(bytes).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(())
    }

    /// An empty list with room for `count` keys: its bytes are charged before the room is
    /// reserved; over the budget the run ends in `Terminated(MemoryBudget)` and nothing stays
    /// charged.
    pub(crate) fn new_key_list(&mut self, count: usize) -> Completion<KeyList> {
        let bytes = key_list_bytes(count);
        self.charge_bytes(bytes)?;
        let mut keys = Vec::new();
        #[expect(
            clippy::disallowed_methods,
            reason = "charged by `charge_bytes` before it is reserved, `key_list_bytes` of the keys"
        )]
        if keys.try_reserve_exact(count).is_err() {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(KeyList { keys, bytes })
    }

    /// Credit what `list` was charged; the list drops.
    pub(crate) fn drop_key_list(&mut self, list: KeyList) {
        self.heap.accounting.credit(list.bytes);
    }

    /// The list of `count` keys an [[OwnPropertyKeys]] answers, after its steps: made by
    /// [`Agent::new_key_list`], then filled by `fill`; when `fill` ends abruptly the list is
    /// credited and dropped.
    fn own_key_list(
        &mut self,
        count: usize,
        fill: impl FnOnce(&mut Self, &mut Vec<Key>) -> Completion<()>,
    ) -> Completion<KeyList> {
        let mut list = self.new_key_list(count)?;
        #[cfg(test)]
        self.heap.accounting.note_key_list(count);
        match fill(self, &mut list.keys) {
            Ok(()) => Ok(list),
            Err(abrupt) => {
                self.drop_key_list(list);
                Err(abrupt)
            }
        }
    }

    /// The index keys of `object`'s property list in ascending order. Read in place when they
    /// are ascending in the list: added ascending (`KeyedList` records it; no walk), or found
    /// so by a walk the caller's step per key covers. Otherwise the n index keys are sorted
    /// (`docs/design/engine.md`, "The step budget"): a buffer of them and the
    /// merge's second buffer, n `u32` each, are charged before they are reserved, and the
    /// bottom-up merge sort charges n steps before each pass, at most n comparisons, so the
    /// step budget and the interrupt are read every n comparisons. The second buffer is credited
    /// when the sort ends; the answer holds the charge of the first.
    fn listed_indices(&mut self, object: ObjectRef) -> Completion<ListedIndices> {
        let Some(count) = self
            .engine_object(object)?
            .properties
            .unsorted_index_count()
        else {
            return Ok(ListedIndices::InPlace);
        };
        let half = u64::try_from(count.saturating_mul(size_of::<u32>())).unwrap_or(u64::MAX);
        let bytes = half.saturating_mul(2);
        self.charge_bytes(bytes)?;
        let mut indices: Vec<u32> = Vec::new();
        let mut buffer: Vec<u32> = Vec::new();
        #[expect(
            clippy::disallowed_methods,
            reason = "the indices and the merge's buffer, both charged by `charge_bytes` before they are reserved, a `u32` per index each"
        )]
        if indices.try_reserve_exact(count).is_err() || buffer.try_reserve_exact(count).is_err() {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        match self.engine_object(object) {
            Ok(record) => indices.extend(record.properties.listed_indices().take(count)),
            Err(abrupt) => {
                self.heap.accounting.credit(bytes);
                return Err(abrupt);
            }
        }
        buffer.resize(indices.len(), 0);
        let mut compared = 0;
        let sorted = sort_indices(&mut indices, &mut buffer, &mut compared, |steps| {
            self.charge_steps(steps)
        });
        #[cfg(test)]
        if let Ok(record) = self.heap.objects.get(object.handle) {
            record.properties.count_compared(compared);
        }
        drop(buffer);
        if let Err(abrupt) = sorted {
            self.heap.accounting.credit(bytes);
            return Err(abrupt);
        }
        self.heap.accounting.credit(bytes.saturating_sub(half));
        Ok(ListedIndices::Sorted {
            indices,
            bytes: half,
        })
    }

    /// Push to `keys` the keys of `object`'s property list in the order of
    /// OrdinaryOwnPropertyKeys (`#sec-ordinaryownpropertykeys`), its index keys from `from` on:
    /// the index keys ascending ([`Agent::listed_indices`]), then the String keys, then the
    /// Symbol keys, each in creation order. `keys` has room for all of them.
    fn push_own_keys(
        &mut self,
        object: ObjectRef,
        from: u32,
        keys: &mut Vec<Key>,
    ) -> Completion<()> {
        let listed = self.listed_indices(object)?;
        let pushed = match self.engine_object(object) {
            Ok(record) => {
                push_listed_keys(&record.properties, &listed, from, keys);
                Ok(())
            }
            Err(abrupt) => Err(abrupt),
        };
        self.heap.accounting.credit(listed.bytes());
        pushed
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
            KindTag::Proxy => self.proxy_call(function, this, args),
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
            KindTag::Proxy => self.proxy_construct(function, args, new_target),
            _ => Err(self.fault()),
        }
    }

    /// Whether `object` has a [[Call]] internal method.
    pub(crate) fn has_call(&mut self, object: ObjectRef) -> Completion<bool> {
        Ok(match &self.engine_object(object)?.kind {
            ObjectKind::BuiltinFunction(_) => true,
            ObjectKind::BoundFunction(function) => function.has_call(),
            ObjectKind::EcmascriptFunction(function) => function.has_call(),
            ObjectKind::Proxy(proxy) => proxy.has_call(),
            _ => false,
        })
    }

    /// Whether `object` has a [[Construct]] internal method.
    pub(crate) fn has_construct(&mut self, object: ObjectRef) -> Completion<bool> {
        Ok(match &self.engine_object(object)?.kind {
            ObjectKind::BuiltinFunction(function) => function.constructor,
            ObjectKind::BoundFunction(function) => function.has_construct(),
            ObjectKind::EcmascriptFunction(function) => function.has_construct(),
            ObjectKind::Proxy(proxy) => proxy.has_construct(),
            _ => false,
        })
    }
}

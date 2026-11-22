//! Environment Records (ECMA-262 9.1, `#sec-environment-records`): declarative, object, global
//! and function Environment Records. Filled by `story:interpreter-and-script-entry-points`;
//! `story:with-arguments-and-dynamic-names` added the records of `with` statements and the
//! lookup by name.
//!
//! The compiler resolves almost every name at compile time (`swell-compiler`, `resolve.rs`): a
//! binding is a local slot of a frame, a slot of a declarative record a number of hops outward,
//! or a name of the global environment. So the records in the heap are declarative records of
//! numbered slots ([`EnvironmentRecord`]): a catch clause's
//! (`#sec-runtime-semantics-catchclauseevaluation`), a function's when a nested function
//! captures one of its bindings (`env/function.rs`), and the one of a named function
//! expression's own name; and the Object Environment Record of each `with` statement
//! (`#sec-with-statement-runtime-semantics-evaluation`), whose binding object is
//! [`EnvironmentRecord::binding_object`]. The chain of a record's [[OuterEnv]] ends at the
//! global environment of the realm of the code that made it, which is not a record of this arena
//! but the realm's global object and global `this` (`env/global.rs`); `None` stands for it. The
//! object Environment Record operations are `env/object.rs`.
//!
//! A reference with a `with` body between it and its binding is looked up by name at run time
//! (`docs/design/engine.md`, decision 3, "Names"): ResolveBinding (`#sec-resolvebinding`) walks
//! the records from the LexicalEnvironment outward ([`Agent::resolve_name`]), one step per record
//! it visits. A declarative record the compiler made for such a scope knows the names of its
//! slots ([`SlotNames`], a table of the compiled code); a record without names binds no name a
//! lookup asks for and is passed. That table stays a scan
//! (`story:property-index-and-dense-elements`): it belongs to the compiled code, which an
//! Environment Record shares and does not own, and its size is fixed by the source text; each
//! comparison past the small size of `object/property.rs` (8) is charged as a step, the first
//! ones being covered by the step the lookup pays for the record, so no lookup does work the
//! step budget does not see.
//!
//! Non-strict eval code adds its `var` and function bindings to the VariableEnvironment of its
//! caller at run time (`#sec-evaldeclarationinstantiation`, `dynamic_code.rs`), and strict eval
//! code to a declarative record of its own: a declarative record keeps such bindings by name
//! beside its slots ([`Added`]), mutable and deletable, and a lookup by name finds them, through
//! an index map above the small size as an object's properties are. A
//! reference that may reach one is looked up by name (decision 3, "Names", item 1).
//!
//! Each record's slots are charged to the memory budget when it is made and credited when the
//! collector frees it; the bindings eval code adds are charged as they are made and credited the
//! same way.
//!
//! Slice 6 adds the `let` and `const` bindings (`docs/design/engine.md`, decision 3, "Lexical
//! bindings"): a slot holds a value or the uninitialized state ([`SlotValue`]); a record of a
//! block, a head, an iteration, a function body or eval code names its slots, each marked `let`
//! or `const` ([`SlotKind`]), so a lookup by name throws as a slot access does: a
//! `ReferenceError` for an uninitialized binding and a `TypeError` for an assignment to a
//! `const` one. The global environment's [[DeclarativeRecord]] is a record of this arena whose
//! bindings are kept by name ([`RecordNames::Global`], `env/global.rs`).
//!
//! Kind site of the collector: [`EnvironmentRecord::trace`] (the outer record, the slots, the
//! bindings eval code added, the binding object and the script whose code names the slots).

pub(crate) mod declarative;
pub(crate) mod function;
pub(crate) mod global;
pub(crate) mod object;

use std::cell::Cell;
use std::rc::Rc;

use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion, Termination};
use crate::interp::frame::SlotValue;
use crate::object::property::{Key, KeyedList, Room, SMALL};
use crate::script::Compiled;
use crate::value::{ObjectRef, Value};

/// Which table of the compiled code names the slots of a record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NameTable {
    /// The function environment of function template `t`: its `captured` names.
    Function(u32),
    /// The environment `PushNamedEnv(index)` makes in the code of function template `template`,
    /// or of the script when it is `None`: `scopes[index]` of that code.
    Scope { template: Option<u32>, index: u32 },
    /// The one immutable binding of the own name of the named function expression of template
    /// `t`.
    SelfName(u32),
}

/// The names of a declarative record's slots: a table of the compiled code of a script, which
/// the script record holds the atoms of.
pub(crate) struct SlotNames {
    pub(crate) script: Handle,
    pub(crate) compiled: Rc<Compiled>,
    pub(crate) table: NameTable,
}

/// What kind of binding a named slot is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlotKind {
    /// A `var`, a parameter, `arguments` or a `catch` parameter.
    Mutable,
    /// A `let` binding, uninitialized until its declaration ran.
    Let,
    /// A `const` binding: immutable and strict (CreateImmutableBinding(N, true)), uninitialized
    /// until its declaration ran.
    Const,
    /// The own name of a named function expression: immutable, not strict.
    SelfName,
}

impl SlotNames {
    /// The slot that binds `key` and its kind, with the steps the scan costs: the table is
    /// scanned, and each comparison past the first [`SMALL`] is a step (the first ones are
    /// covered by the step the lookup pays for the record).
    fn find(&self, key: Key) -> (Option<(u32, SlotKind)>, u64) {
        let mut visited = 0usize;
        let found = self.scan(key, &mut visited);
        let steps = u64::try_from(visited.saturating_sub(SMALL)).unwrap_or(u64::MAX);
        (found, steps)
    }

    /// The scan of [`SlotNames::find`], counting the names it compares in `visited`.
    fn scan(&self, key: Key, visited: &mut usize) -> Option<(u32, SlotKind)> {
        let code = &self.compiled.code;
        let template = |t: u32| code.functions.get(usize::try_from(t).ok()?);
        let named = |name: u32| {
            usize::try_from(name)
                .ok()
                .and_then(|name| self.compiled.keys.get(name))
                == Some(&key)
        };
        let (position, kind) = match self.table {
            NameTable::Function(t) => {
                let position = template(t)?.captured.iter().position(|&name| {
                    *visited += 1;
                    named(name)
                })?;
                (position, SlotKind::Mutable)
            }
            NameTable::Scope { template: t, index } => {
                let scopes = match t {
                    Some(t) => &template(t)?.scopes,
                    None => &code.scopes,
                };
                let names = scopes.get(usize::try_from(index).ok()?)?;
                let (position, name) = names.iter().enumerate().find(|(_, name)| {
                    *visited += 1;
                    named(name.name())
                })?;
                let kind = if name.is_constant() {
                    SlotKind::Const
                } else if name.is_lexical() {
                    SlotKind::Let
                } else {
                    SlotKind::Mutable
                };
                (position, kind)
            }
            NameTable::SelfName(t) => {
                let own = self.compiled.name_keys.get(usize::try_from(t).ok()?)?;
                *visited = 1;
                return (*own == key).then_some((0, SlotKind::SelfName));
            }
        };
        Some((u32::try_from(position).ok()?, kind))
    }
}

/// A binding of the global environment's [[DeclarativeRecord]]: its state and whether it is
/// `const` (decision 3, "Lexical bindings", item 4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LexicalBinding {
    pub(crate) value: SlotValue,
    pub(crate) constant: bool,
}

/// The bindings of the global environment's [[DeclarativeRecord]], by name: GlobalDeclarationInstantiation
/// creates them (step 15), none is ever deleted. Their growth is charged before it is made, and
/// the drop queues the bytes for the agent to credit, as [`Added`] does.
#[derive(Default)]
pub(crate) struct GlobalBindings {
    pub(crate) bindings: KeyedList<LexicalBinding>,
    pub(crate) bytes: u64,
    pub(crate) queue: Option<Rc<Cell<u64>>>,
}

impl Drop for GlobalBindings {
    fn drop(&mut self) {
        if let Some(queue) = &self.queue {
            queue.set(queue.get().saturating_add(self.bytes));
        }
    }
}

/// How a declarative record names its bindings: the slots, through a table of compiled code;
/// or, for the global environment's [[DeclarativeRecord]], by name, each with its state.
pub(crate) enum RecordNames {
    Slots(SlotNames),
    Global(Box<GlobalBindings>),
}

/// The bindings eval code added to a declarative record at run time
/// (`#sec-evaldeclarationinstantiation`, steps 17 and 18: CreateMutableBinding(name, true) and
/// InitializeBinding), each mutable and deletable, in the order they were made: a
/// [`KeyedList`], found through an index map above the small size as an object's properties are
/// (`object/property.rs`), so that a script that adds bindings without end does not make each
/// lookup longer. Their growth is charged to the memory budget before it is made; the drop
/// queues the bytes for the agent to credit after the sweep that frees the record, as an
/// object's property map does.
#[derive(Default)]
pub(crate) struct Added {
    bindings: KeyedList<Value>,
    bytes: u64,
    queue: Option<Rc<Cell<u64>>>,
}

impl Added {
    /// The position of the binding of `key`.
    fn find(&self, key: Key) -> Option<usize> {
        self.bindings.position(key)
    }

    /// The entries lookups visited so far.
    #[cfg(test)]
    pub(crate) fn visited(&self) -> u64 {
        self.bindings.visited()
    }
}

impl Drop for Added {
    fn drop(&mut self) {
        if let Some(queue) = &self.queue {
            queue.set(queue.get().saturating_add(self.bytes));
        }
    }
}

/// An Environment Record with its [[OuterEnv]]: another record, or `None` for the global
/// environment of the realm. A declarative record (9.1.1.1) is its numbered slots, named when
/// a lookup by name may reach them, and the bindings eval code added to it; an Object
/// Environment Record of a `with` statement (9.1.1.2, [[IsWithEnvironment]] true) is its binding
/// object, with no slot.
pub(crate) struct EnvironmentRecord {
    pub(crate) outer: Option<Handle>,
    pub(crate) slots: Vec<SlotValue>,
    pub(crate) names: Option<RecordNames>,
    /// The bindings eval code added at run time.
    pub(crate) added: Added,
    /// [[BindingObject]] of an Object Environment Record.
    pub(crate) binding_object: Option<ObjectRef>,
}

/// Where a declarative record binds a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bound {
    /// A named slot, and its kind.
    Slot(u32, SlotKind),
    /// A binding eval code added, by index.
    Added(usize),
}

impl EnvironmentRecord {
    /// Kind site of the collector: the outer environment, the value of every binding, the
    /// binding object, and the script whose code names the slots.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("env/mod.rs EnvironmentRecord::trace");
        if let Some(outer) = self.outer {
            mark.push(Item::Environment(outer));
        }
        for value in self.slots.iter().filter_map(|slot| slot.value()) {
            mark.value(value);
        }
        for (key, value) in self.added.bindings.iter() {
            key.trace(mark);
            mark.value(*value);
        }
        if let Some(object) = self.binding_object {
            mark.object(object);
        }
        match &self.names {
            Some(RecordNames::Slots(names)) => mark.push(Item::Script(names.script)),
            Some(RecordNames::Global(global)) => {
                for (key, binding) in global.bindings.iter() {
                    key.trace(mark);
                    if let Some(value) = binding.value.value() {
                        mark.value(value);
                    }
                }
            }
            None => {}
        }
    }

    /// The names of the record's slots, when a table of compiled code names them.
    fn slot_names(&self) -> Option<&SlotNames> {
        match &self.names {
            Some(RecordNames::Slots(names)) => Some(names),
            _ => None,
        }
    }

    /// An Environment Record, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> EnvironmentRecord {
        EnvironmentRecord {
            outer: None,
            slots: Vec::new(),
            names: None,
            added: Added::default(),
            binding_object: None,
        }
    }
}

/// A Reference Record whose [[Base]] is an Environment Record, as ResolveBinding answers it
/// (`#sec-getidentifierreference`); its [[ReferencedName]] is the name the instruction carries,
/// whose slot in a declarative record is found again from the record's names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NameReference {
    /// [[Base]] is unresolvable.
    Unresolvable,
    /// The global Environment Record of the running code's realm binds the name.
    Global,
    /// The declarative record `env` binds the name.
    Declarative(Handle),
    /// The Object Environment Record of a `with` statement whose binding object is this.
    Object(ObjectRef),
}

impl<H: Host> Agent<H> {
    /// A new record of `slots` bindings, each `undefined`, whose [[OuterEnv]] is `outer` and
    /// whose slots `names` names, if any. Over the budget the run ends in
    /// `Terminated(MemoryBudget)`.
    pub(crate) fn new_environment(
        &mut self,
        outer: Option<Handle>,
        slots: u32,
        names: Option<SlotNames>,
    ) -> Completion<Handle> {
        let initial = SlotValue::Value(Value::Undefined);
        self.new_environment_of(outer, slots, names.map(RecordNames::Slots), initial)
    }

    /// A new record of `slots` bindings, each `initial`, whose [[OuterEnv]] is `outer`, named by
    /// `names`; charged as [`Agent::new_environment`] says.
    pub(crate) fn new_environment_of(
        &mut self,
        outer: Option<Handle>,
        slots: u32,
        names: Option<RecordNames>,
        initial: SlotValue,
    ) -> Completion<Handle> {
        let count = usize::try_from(slots).unwrap_or(usize::MAX);
        let owned = u64::try_from(count.saturating_mul(size_of::<SlotValue>())).unwrap_or(u64::MAX);
        if self.heap.accounting.check(owned).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let mut values = Vec::new();
        if values.try_reserve_exact(count).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        values.resize(count, initial);
        self.store_environment(
            EnvironmentRecord {
                outer,
                slots: values,
                names,
                added: Added::default(),
                binding_object: None,
            },
            owned,
        )
    }

    /// `record` in the `environments` arena, with `owned` bytes beside its own.
    fn store_environment(&mut self, record: EnvironmentRecord, owned: u64) -> Completion<Handle> {
        match crate::heap::store(
            &mut self.heap.accounting,
            &mut self.heap.environments,
            record,
            owned,
        ) {
            Ok(handle) => Ok(handle),
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    /// NewObjectEnvironment(`object`, true, `outer`) (`#sec-newobjectenvironment`): the record
    /// of a `with` statement.
    pub(crate) fn new_with_environment(
        &mut self,
        object: ObjectRef,
        outer: Option<Handle>,
    ) -> Completion<Handle> {
        self.store_environment(
            EnvironmentRecord {
                outer,
                slots: Vec::new(),
                names: None,
                added: Added::default(),
                binding_object: Some(object),
            },
            0,
        )
    }

    /// The record `env`; a stale one inside the engine faults the agent.
    pub(crate) fn environment(&mut self, env: Handle) -> Completion<&EnvironmentRecord> {
        if self.heap.environments.get(env).is_err() {
            return Err(self.fault());
        }
        self.heap
            .environments
            .get(env)
            .map_err(|_| Abrupt::Terminated(Termination::EngineFault))
    }

    /// The record `hops` outward from `env`: a record the compiler counted must exist.
    pub(crate) fn environment_at(&mut self, env: Option<Handle>, hops: u32) -> Completion<Handle> {
        let mut current = env;
        for _ in 0..hops {
            current = match current {
                Some(handle) => self.environment(handle)?.outer,
                None => None,
            };
        }
        match current {
            Some(handle) => Ok(handle),
            None => Err(self.fault()),
        }
    }

    /// GetBindingValue of slot `slot` of the record `hops` outward from `env`
    /// (`#sec-declarative-environment-records-getbindingvalue-n-s`) where the compiler knows the
    /// binding is initialized: an uninitialized one there is a broken invariant, which faults.
    pub(crate) fn get_slot(
        &mut self,
        env: Option<Handle>,
        hops: u32,
        slot: u32,
    ) -> Completion<Value> {
        match self.slot_state(env, hops, slot)? {
            SlotValue::Value(value) => Ok(value),
            SlotValue::Uninitialized => Err(self.fault()),
        }
    }

    /// What slot `slot` of the record `hops` outward from `env` holds.
    pub(crate) fn slot_state(
        &mut self,
        env: Option<Handle>,
        hops: u32,
        slot: u32,
    ) -> Completion<SlotValue> {
        let handle = self.environment_at(env, hops)?;
        let index = usize::try_from(slot).unwrap_or(usize::MAX);
        match self.environment(handle)?.slots.get(index) {
            Some(&value) => Ok(value),
            None => Err(self.fault()),
        }
    }

    /// The name of slot `slot` of the record `env`, for a message: its records of `let` and
    /// `const` bindings always name their slots.
    pub(crate) fn slot_name(&mut self, env: Handle, slot: u32) -> String {
        let name = self.heap.environments.get(env).ok().and_then(|record| {
            let names = record.slot_names()?;
            let code = &names.compiled.code;
            let NameTable::Scope { template, index } = names.table else {
                return None;
            };
            let scopes = match template {
                Some(t) => &code.functions.get(usize::try_from(t).ok()?)?.scopes,
                None => &code.scopes,
            };
            let name = scopes
                .get(usize::try_from(index).ok()?)?
                .get(usize::try_from(slot).ok()?)?;
            code.strings.get(usize::try_from(name.name()).ok()?)
        });
        name.map(|units| String::from_utf16_lossy(units))
            .unwrap_or_default()
    }

    /// SetMutableBinding of slot `slot` of the record `hops` outward from `env`
    /// (`#sec-declarative-environment-records-setmutablebinding-n-v-s`, step 4: a mutable
    /// binding's value changes; the immutable ones are `AssignImmutable`'s), or the
    /// InitializeBinding of a `let` or `const` binding by its declaration
    /// (`#sec-declarative-environment-records-initializebinding-n-v`).
    pub(crate) fn set_slot(
        &mut self,
        env: Option<Handle>,
        hops: u32,
        slot: u32,
        value: Value,
    ) -> Completion<()> {
        let handle = self.environment_at(env, hops)?;
        let index = usize::try_from(slot).unwrap_or(usize::MAX);
        let stored = self
            .heap
            .environments
            .get_mut(handle)
            .ok()
            .and_then(|record| record.slots.get_mut(index))
            .map(|binding| *binding = SlotValue::Value(value));
        match stored {
            Some(()) => Ok(()),
            None => Err(self.fault()),
        }
    }

    /// The [[OuterEnv]] of `env`.
    pub(crate) fn outer_environment(&mut self, env: Handle) -> Completion<Option<Handle>> {
        Ok(self.environment(env)?.outer)
    }

    /// ResolveBinding of `key` (`#sec-resolvebinding`, `#sec-getidentifierreference`) from the
    /// record `env` outward, the global environment of `realm` last: HasBinding of each record,
    /// one step per record visited (decision 3, "Names"). A declarative record binds the names
    /// of its named slots, an Object Environment Record what HasBinding of `env/object.rs`
    /// answers.
    pub(crate) fn resolve_name(
        &mut self,
        env: Option<Handle>,
        realm: Handle,
        key: Key,
    ) -> Completion<NameReference> {
        let mut current = env;
        loop {
            self.charge_steps(1)?;
            let Some(handle) = current else {
                return Ok(if self.global_has_binding(realm, key)? {
                    NameReference::Global
                } else {
                    NameReference::Unresolvable
                });
            };
            let (outer, object, found, steps) = {
                let record = self.environment(handle)?;
                let (named, steps) = record
                    .slot_names()
                    .map_or((None, 0), |names| names.find(key));
                (
                    record.outer,
                    record.binding_object,
                    named.is_some() || record.added.find(key).is_some(),
                    steps,
                )
            };
            if steps > 0 {
                self.charge_steps(steps)?;
            }
            if let Some(object) = object {
                if self.with_has_binding(object, key)? {
                    return Ok(NameReference::Object(object));
                }
            } else if found {
                return Ok(NameReference::Declarative(handle));
            }
            current = outer;
        }
    }

    /// Where the declarative record `env` binds `key`: a named slot or a binding eval code
    /// added; `None` when it binds no such name (an added binding a `delete` removed).
    pub(crate) fn bound(&mut self, env: Handle, key: Key) -> Completion<Option<Bound>> {
        let record = self.environment(env)?;
        let (named, steps) = record
            .slot_names()
            .map_or((None, 0), |names| names.find(key));
        let bound = match named {
            Some((slot, kind)) => Some(Bound::Slot(slot, kind)),
            None => record.added.find(key).map(Bound::Added),
        };
        if steps > 0 {
            self.charge_steps(steps)?;
        }
        Ok(bound)
    }

    /// HasBinding of the declarative record `env`
    /// (`#sec-declarative-environment-records-hasbinding-n`) for a name: a named slot or a
    /// binding eval code added.
    pub(crate) fn declarative_has_binding(&mut self, env: Handle, key: Key) -> Completion<bool> {
        Ok(self.bound(env, key)?.is_some())
    }

    /// CreateMutableBinding(`key`, true) and InitializeBinding(`key`, `value`) of the
    /// declarative record `env` (`#sec-declarative-environment-records-createmutablebinding-n-d`,
    /// `#sec-declarative-environment-records-initializebinding-n-v`), which binds no such name: a
    /// binding eval code adds at run time, deletable. Its growth is charged before it is made
    /// (decision 3, "Names", item 5); over the budget the run ends in
    /// `Terminated(MemoryBudget)`.
    pub(crate) fn add_binding(&mut self, env: Handle, key: Key, value: Value) -> Completion<()> {
        let queue = Rc::clone(&self.credit);
        let hasher = self.heap.hasher;
        let mut room = Room::default();
        let made = match self.heap.environments.get_mut(env) {
            Ok(record) => {
                let added = &mut record.added;
                let made = added
                    .bindings
                    .make_room(&mut self.heap.accounting, &hasher, &mut room);
                if room.bytes > 0 {
                    added.bytes = added.bytes.saturating_add(room.bytes);
                    if added.queue.is_none() {
                        added.queue = Some(queue);
                    }
                }
                made
            }
            Err(_) => return Err(self.fault()),
        };
        if made.is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        if room.visited > 0 {
            self.charge_steps(room.visited)?;
        }
        let pushed = match self.heap.environments.get_mut(env) {
            Ok(record) => record.added.bindings.push(key, value),
            Err(_) => return Err(self.fault()),
        };
        if pushed.is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(())
    }

    /// SetMutableBinding(`key`, `value`, false) of a binding the declarative record `env` has
    /// (`#sec-declarative-environment-records-setmutablebinding-n-v-s`): a named slot or a
    /// binding eval code added; an immutable one keeps its value.
    pub(crate) fn set_declarative_binding(
        &mut self,
        env: Handle,
        key: Key,
        value: Value,
    ) -> Completion<()> {
        match self.bound(env, key)? {
            Some(Bound::Slot(slot, SlotKind::Mutable | SlotKind::Let)) => {
                self.set_slot(Some(env), 0, slot, value)
            }
            Some(Bound::Slot(_, SlotKind::Const | SlotKind::SelfName)) => Ok(()),
            Some(Bound::Added(index)) => self.set_added(env, index, value),
            None => Err(self.fault()),
        }
    }

    /// The value of the binding eval code added to `env` at `index`.
    fn added_value(&mut self, env: Handle, index: usize) -> Completion<Value> {
        match self.environment(env)?.added.bindings.at(index) {
            Some(&value) => Ok(value),
            None => Err(self.fault()),
        }
    }

    /// Set the binding eval code added to `env` at `index`.
    fn set_added(&mut self, env: Handle, index: usize, value: Value) -> Completion<()> {
        let stored = self
            .heap
            .environments
            .get_mut(env)
            .ok()
            .and_then(|record| record.added.bindings.at_mut(index))
            .map(|binding| *binding = value);
        match stored {
            Some(()) => Ok(()),
            None => Err(self.fault()),
        }
    }

    /// GetValue (`#sec-getvalue`) of the resolved `reference` to `key`: GetBindingValue of its
    /// record; `None` for an unresolvable one, which the caller throws for, and for a binding of
    /// a declarative record that a `delete` removed since the lookup.
    pub(crate) fn name_value(
        &mut self,
        reference: NameReference,
        realm: Handle,
        key: Key,
        strict: bool,
    ) -> Completion<Option<Value>> {
        match reference {
            NameReference::Unresolvable => Ok(None),
            NameReference::Global => self.global_get_binding_value(realm, key, strict).map(Some),
            // Step 2 of GetBindingValue: an uninitialized binding is a `ReferenceError`.
            NameReference::Declarative(env) => match self.bound(env, key)? {
                Some(Bound::Slot(slot, _)) => match self.slot_state(Some(env), 0, slot)? {
                    SlotValue::Value(value) => Ok(Some(value)),
                    SlotValue::Uninitialized => {
                        let name = self.key_text(key);
                        Err(self.not_initialized(&name))
                    }
                },
                Some(Bound::Added(index)) => self.added_value(env, index).map(Some),
                None => Ok(None),
            },
            NameReference::Object(object) => {
                self.object_get_binding_value(object, key, strict).map(Some)
            }
        }
    }

    /// PutValue (`#sec-putvalue`) of `value` to the resolved `reference` to `key`, but the
    /// `ReferenceError` of an unresolvable one in strict code, which the caller throws:
    /// otherwise Set on the global object; SetMutableBinding of a resolved one.
    pub(crate) fn put_name_value(
        &mut self,
        reference: NameReference,
        realm: Handle,
        key: Key,
        value: Value,
        strict: bool,
    ) -> Completion<()> {
        match reference {
            NameReference::Unresolvable => {
                let global = self.global_binding_object(realm)?;
                self.set(global, key, value, false)?;
                Ok(())
            }
            NameReference::Global => self.global_set_mutable_binding(realm, key, value, strict),
            NameReference::Declarative(env) => match self.bound(env, key)? {
                Some(Bound::Slot(slot, SlotKind::Mutable)) => {
                    self.set_slot(Some(env), 0, slot, value)
                }
                // Steps 3 and 5: an uninitialized binding is a `ReferenceError`, a `const` one a
                // `TypeError` in any code.
                Some(Bound::Slot(slot, kind @ (SlotKind::Let | SlotKind::Const))) => {
                    let state = self.slot_state(Some(env), 0, slot)?;
                    let name = self.key_text(key);
                    match (state, kind) {
                        (SlotValue::Uninitialized, _) => Err(self.not_initialized(&name)),
                        (_, SlotKind::Const) => self.assign_const(&name),
                        _ => self.set_slot(Some(env), 0, slot, value),
                    }
                }
                Some(Bound::Slot(_, SlotKind::SelfName)) => {
                    let name = self.key_text(key);
                    self.assign_immutable(strict, &name)
                }
                Some(Bound::Added(index)) => self.set_added(env, index, value),
                // Step 1: a binding a `delete` removed since the lookup.
                None if strict => {
                    let name = self.key_text(key);
                    let message = format!("{name} is not defined");
                    Err(self.throw_error(crate::host::ErrorKind::ReferenceError, &message))
                }
                None => self.add_binding(env, key, value),
            },
            NameReference::Object(object) => {
                self.object_set_mutable_binding(object, key, value, strict)
            }
        }
    }

    /// The `delete` of the resolved `reference` to `key`
    /// (`#sec-delete-operator-runtime-semantics-evaluation`, step 6): `true` when it is
    /// unresolvable, otherwise DeleteBinding of its record. Of a declarative record
    /// (`#sec-declarative-environment-records-deletebinding-n`): a named slot is not deletable,
    /// a binding eval code added is (CreateMutableBinding(name, true)) and is removed, and a
    /// name the record no longer binds answers `true`.
    pub(crate) fn delete_name(
        &mut self,
        reference: NameReference,
        realm: Handle,
        key: Key,
    ) -> Completion<bool> {
        match reference {
            NameReference::Unresolvable => Ok(true),
            NameReference::Global => self.global_delete_binding(realm, key),
            NameReference::Declarative(env) => match self.bound(env, key)? {
                Some(Bound::Slot(..)) => Ok(false),
                Some(Bound::Added(_)) => {
                    let removed = self
                        .heap
                        .environments
                        .get_mut(env)
                        .ok()
                        .and_then(|record| record.added.bindings.remove(key));
                    match removed {
                        Some(_) => Ok(true),
                        None => Err(self.fault()),
                    }
                }
                None => Ok(true),
            },
            NameReference::Object(object) => self.object_delete_binding(object, key),
        }
    }

    /// The text of the String key `key`, for a message.
    pub(crate) fn key_text(&mut self, key: Key) -> String {
        match key {
            Key::Index(index) => index.to_string(),
            Key::String(string) => self
                .heap
                .strings
                .get(string.handle)
                .map(|s| s.to_rust_string_lossy())
                .unwrap_or_default(),
            Key::Symbol(_) => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EnvironmentRecord, GlobalBindings, NameReference, RecordNames, SlotNames};
    use crate::host::hooks::test_host::{agent_with_realm, enter};
    use crate::host::outcome::{Abrupt, Termination};
    use crate::object::property::{Key, SMALL};
    use crate::value::Value;

    fn number(i: u32) -> Value {
        Value::Number(f64::from(i))
    }

    /// Decision 3, "Lexical bindings", items 2 and 4: the names of the global
    /// [[DeclarativeRecord]] take no room in a record beside the slot names it had, so every
    /// record is charged what it was.
    #[test]
    fn the_names_of_a_record_take_the_room_they_took() {
        println!(
            "Option<SlotNames> {} bytes, Option<RecordNames> {} bytes, EnvironmentRecord {} bytes, \
             GlobalBindings {} bytes, RealmGlobalEnv {} bytes, a record's arena slot {} bytes",
            size_of::<Option<SlotNames>>(),
            size_of::<Option<RecordNames>>(),
            size_of::<EnvironmentRecord>(),
            size_of::<GlobalBindings>(),
            size_of::<super::global::RealmGlobalEnv>(),
            crate::heap::arena::slot_bytes::<EnvironmentRecord>()
        );
        assert_eq!(
            size_of::<Option<RecordNames>>(),
            size_of::<Option<SlotNames>>()
        );
    }

    /// EvalDeclarationInstantiation (`#sec-evaldeclarationinstantiation`, steps 17 and 18) and
    /// DeleteBinding (`#sec-declarative-environment-records-deletebinding-n`): 100 bindings
    /// added to a declarative record, past the small size, are each found visiting one entry
    /// of the map; a deleted one is found no more and the others still are. The growth of the
    /// bindings and of their map is charged before it is made: over the budget it is refused
    /// with nothing charged.
    #[test]
    fn added_bindings_are_found_through_the_map_and_charged() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let realm_handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let env = agent.new_environment(None, 0, None).unwrap();
        let reference = NameReference::Declarative(env);
        for i in 0..100 {
            assert_eq!(agent.add_binding(env, Key::Index(i), number(i)), Ok(()));
        }
        let visited = |agent: &mut crate::host::agent::Agent<_>| {
            agent.environment(env).unwrap().added.visited()
        };
        let before = visited(&mut agent);
        for i in 0..100 {
            assert_eq!(
                agent.name_value(reference, realm_handle, Key::Index(i), false),
                Ok(Some(number(i)))
            );
        }
        assert_eq!(visited(&mut agent) - before, 100);
        assert_eq!(
            agent.delete_name(reference, realm_handle, Key::Index(5)),
            Ok(true)
        );
        assert_eq!(agent.declarative_has_binding(env, Key::Index(5)), Ok(false));
        assert_eq!(
            agent.name_value(reference, realm_handle, Key::Index(6), false),
            Ok(Some(number(6)))
        );

        let small = agent.new_environment(None, 0, None).unwrap();
        for i in 0..SMALL as u32 {
            assert_eq!(agent.add_binding(small, Key::Index(i), number(i)), Ok(()));
        }
        let bytes = agent.heap.bytes();
        agent.heap.accounting.set_limit(bytes);
        assert_eq!(
            agent.add_binding(small, Key::Index(99), Value::Null),
            Err(Abrupt::Terminated(Termination::MemoryBudget))
        );
        assert_eq!(agent.heap.bytes(), bytes);
        assert_eq!(
            agent.declarative_has_binding(small, Key::Index(99)),
            Ok(false)
        );
    }
}

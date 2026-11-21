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
//! lookup asks for and is passed.
//!
//! Non-strict eval code adds its `var` and function bindings to the VariableEnvironment of its
//! caller at run time (`#sec-evaldeclarationinstantiation`, `dynamic_code.rs`), and strict eval
//! code to a declarative record of its own: a declarative record keeps such bindings by name
//! beside its slots ([`Added`]), mutable and deletable, and a lookup by name finds them. A
//! reference that may reach one is looked up by name (decision 3, "Names", item 1).
//!
//! Each record's slots are charged to the memory budget when it is made and credited when the
//! collector frees it; the bindings eval code adds are charged as they are made and credited the
//! same way.
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
use crate::object::property::Key;
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

impl SlotNames {
    /// The slot that binds `key`, and whether the binding is mutable.
    fn find(&self, key: Key) -> Option<(u32, bool)> {
        let code = &self.compiled.code;
        let template = |t: u32| code.functions.get(usize::try_from(t).ok()?);
        let (names, mutable): (&[u32], bool) = match self.table {
            NameTable::Function(t) => (&template(t)?.captured, true),
            NameTable::Scope { template: t, index } => {
                let scopes = match t {
                    Some(t) => &template(t)?.scopes,
                    None => &code.scopes,
                };
                (scopes.get(usize::try_from(index).ok()?)?, true)
            }
            NameTable::SelfName(t) => {
                let own = self.compiled.name_keys.get(usize::try_from(t).ok()?)?;
                return (*own == key).then_some((0, false));
            }
        };
        let position = names.iter().position(|&name| {
            usize::try_from(name)
                .ok()
                .and_then(|name| self.compiled.keys.get(name))
                == Some(&key)
        })?;
        Some((u32::try_from(position).ok()?, mutable))
    }
}

/// The bindings eval code added to a declarative record at run time
/// (`#sec-evaldeclarationinstantiation`, steps 17 and 18: CreateMutableBinding(name, true) and
/// InitializeBinding), each mutable and deletable, in the order they were made. Their growth is
/// charged to the memory budget as it is made; the drop queues the bytes for the agent to
/// credit after the sweep that frees the record, as an object's property map does.
#[derive(Default)]
pub(crate) struct Added {
    bindings: Vec<(Key, Value)>,
    bytes: u64,
    queue: Option<Rc<Cell<u64>>>,
}

impl Added {
    /// The index of the binding of `key`.
    fn find(&self, key: Key) -> Option<usize> {
        self.bindings.iter().position(|(name, _)| *name == key)
    }

    /// The bytes one more binding takes.
    fn entry_bytes() -> u64 {
        u64::try_from(size_of::<(Key, Value)>()).unwrap_or(u64::MAX)
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
    pub(crate) slots: Vec<Value>,
    pub(crate) names: Option<SlotNames>,
    /// The bindings eval code added at run time.
    pub(crate) added: Added,
    /// [[BindingObject]] of an Object Environment Record.
    pub(crate) binding_object: Option<ObjectRef>,
}

/// Where a declarative record binds a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bound {
    /// A named slot, and whether it is mutable.
    Slot(u32, bool),
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
        for &value in &self.slots {
            mark.value(value);
        }
        for (key, value) in &self.added.bindings {
            key.trace(mark);
            mark.value(*value);
        }
        if let Some(object) = self.binding_object {
            mark.object(object);
        }
        if let Some(names) = &self.names {
            mark.push(Item::Script(names.script));
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
        let count = usize::try_from(slots).unwrap_or(usize::MAX);
        let owned = u64::try_from(count.saturating_mul(size_of::<Value>())).unwrap_or(u64::MAX);
        if self.heap.accounting.check(owned).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let mut values = Vec::new();
        if values.try_reserve_exact(count).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        values.resize(count, Value::Undefined);
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
    /// (`#sec-declarative-environment-records-getbindingvalue-n-s`; no binding of slice 1 is
    /// ever uninitialized).
    pub(crate) fn get_slot(
        &mut self,
        env: Option<Handle>,
        hops: u32,
        slot: u32,
    ) -> Completion<Value> {
        let handle = self.environment_at(env, hops)?;
        let index = usize::try_from(slot).unwrap_or(usize::MAX);
        match self.environment(handle)?.slots.get(index) {
            Some(&value) => Ok(value),
            None => Err(self.fault()),
        }
    }

    /// SetMutableBinding of slot `slot` of the record `hops` outward from `env`
    /// (`#sec-declarative-environment-records-setmutablebinding-n-v-s`, step 4: a mutable
    /// binding's value changes; the immutable ones are `AssignImmutable`'s).
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
            .map(|binding| *binding = value);
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
            let (outer, object, found) = {
                let record = self.environment(handle)?;
                (
                    record.outer,
                    record.binding_object,
                    record
                        .names
                        .as_ref()
                        .and_then(|names| names.find(key))
                        .is_some()
                        || record.added.find(key).is_some(),
                )
            };
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
    fn bound(&mut self, env: Handle, key: Key) -> Completion<Option<Bound>> {
        let record = self.environment(env)?;
        if let Some((slot, mutable)) = record.names.as_ref().and_then(|names| names.find(key)) {
            return Ok(Some(Bound::Slot(slot, mutable)));
        }
        Ok(record.added.find(key).map(Bound::Added))
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
        let (full, capacity) = {
            let added = &self.environment(env)?.added.bindings;
            (added.len() == added.capacity(), added.capacity())
        };
        if full {
            let wanted = capacity.saturating_mul(2).max(4);
            let more = u64::try_from(wanted.saturating_sub(capacity)).unwrap_or(u64::MAX);
            let bytes = more.saturating_mul(Added::entry_bytes());
            if self.heap.accounting.charge(bytes).is_err() {
                return Err(self.terminate(Termination::MemoryBudget));
            }
            let queue = Rc::clone(&self.credit);
            let reserved = match self.heap.environments.get_mut(env) {
                Ok(record) => {
                    let added = &mut record.added;
                    let additional = wanted.saturating_sub(added.bindings.len());
                    let reserved = added.bindings.try_reserve_exact(additional).is_ok();
                    if reserved {
                        added.bytes = added.bytes.saturating_add(bytes);
                        if added.queue.is_none() {
                            added.queue = Some(queue);
                        }
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
        match self.heap.environments.get_mut(env) {
            Ok(record) => {
                record.added.bindings.push((key, value));
                Ok(())
            }
            Err(_) => Err(self.fault()),
        }
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
            Some(Bound::Slot(slot, true)) => self.set_slot(Some(env), 0, slot, value),
            Some(Bound::Slot(_, false)) => Ok(()),
            Some(Bound::Added(index)) => self.set_added(env, index, value),
            None => Err(self.fault()),
        }
    }

    /// The value of the binding eval code added to `env` at `index`.
    fn added_value(&mut self, env: Handle, index: usize) -> Completion<Value> {
        match self.environment(env)?.added.bindings.get(index) {
            Some(&(_, value)) => Ok(value),
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
            .and_then(|record| record.added.bindings.get_mut(index))
            .map(|binding| binding.1 = value);
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
            NameReference::Declarative(env) => match self.bound(env, key)? {
                Some(Bound::Slot(slot, _)) => self.get_slot(Some(env), 0, slot).map(Some),
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
                Some(Bound::Slot(slot, true)) => self.set_slot(Some(env), 0, slot, value),
                Some(Bound::Slot(_, false)) => {
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
                Some(Bound::Added(index)) => {
                    let removed = self
                        .heap
                        .environments
                        .get_mut(env)
                        .ok()
                        .filter(|record| index < record.added.bindings.len())
                        .map(|record| record.added.bindings.remove(index));
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
    fn key_text(&mut self, key: Key) -> String {
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

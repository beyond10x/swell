//! Environment Records (ECMA-262 9.1, `#sec-environment-records`): declarative, object, global
//! and function Environment Records. Filled by `story:interpreter-and-script-entry-points`.
//!
//! The compiler resolves every name at compile time (`swell-compiler`, `resolve.rs`): a binding
//! is a local slot of a frame, a slot of a declarative record a number of hops outward, or a
//! name of the global environment. So the records in the heap are declarative records of
//! numbered slots ([`EnvironmentRecord`]): a catch clause's
//! (`#sec-runtime-semantics-catchclauseevaluation`), a function's when a nested function
//! captures one of its bindings
//! (`env/function.rs`), and the one of a named function expression's own name. The chain of a
//! record's [[OuterEnv]] ends at the global environment of the realm of the code that made it,
//! which is not a record of this arena but the realm's global object and global `this`
//! (`env/global.rs`); `None` stands for it. The object Environment Record operations the global
//! environment uses are `env/object.rs`.
//!
//! Each record's slots are charged to the memory budget when it is made and credited when the
//! collector frees it.
//!
//! Kind site of the collector: [`EnvironmentRecord::trace`] (the outer record and the slots).

pub(crate) mod declarative;
pub(crate) mod function;
pub(crate) mod global;
pub(crate) mod object;

use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion, Termination};
use crate::value::Value;

/// A declarative Environment Record (9.1.1.1) whose bindings are numbered slots, with its
/// [[OuterEnv]]: another record, or `None` for the global environment of the realm.
pub(crate) struct EnvironmentRecord {
    pub(crate) outer: Option<Handle>,
    pub(crate) slots: Vec<Value>,
}

impl EnvironmentRecord {
    /// Kind site of the collector: the outer environment and the value of every binding.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("env/mod.rs EnvironmentRecord::trace");
        if let Some(outer) = self.outer {
            mark.push(Item::Environment(outer));
        }
        for &value in &self.slots {
            mark.value(value);
        }
    }

    /// An Environment Record, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> EnvironmentRecord {
        EnvironmentRecord {
            outer: None,
            slots: Vec::new(),
        }
    }
}

impl<H: Host> Agent<H> {
    /// A new record of `slots` bindings, each `undefined`, whose [[OuterEnv]] is `outer`. Over
    /// the budget the run ends in `Terminated(MemoryBudget)`.
    pub(crate) fn new_environment(
        &mut self,
        outer: Option<Handle>,
        slots: u32,
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
        let record = EnvironmentRecord {
            outer,
            slots: values,
        };
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
}

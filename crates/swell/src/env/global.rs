//! Global Environment Records (ECMA-262 9.1.1.4, `#sec-global-environment-records`), and the one
//! a realm holds as [[GlobalEnv]] ([`RealmGlobalEnv`]: its constructor and its trace). Filled by
//! `story:interpreter-and-script-entry-points`; the realm record of
//! `story:object-model-and-realms` holds the field and calls both.
//!
//! A Global Environment Record is its [[ObjectRecord]], its [[GlobalThisValue]] and its
//! [[DeclarativeRecord]]. The first two are the realm record's `global` and `global_this`
//! (`realm/mod.rs`, set by SetRealmGlobalObject, `#sec-setrealmglobalobject`); the
//! [[DeclarativeRecord]] holds the `let`, `const` and class bindings of the realm's scripts
//! (slice 6; a class declaration of a script binds here since `story:class-definition-evaluation`,
//! mutable as a `let` binding, and its evaluation initializes it): a declarative record of the `environments` arena whose bindings
//! are kept by name, each with its state (uninitialized until its declaration ran, the temporal
//! dead zone) and whether it is `const`, found through a keyed list and never deleted
//! (`docs/design/engine.md`, decision 3, "Lexical bindings", item 4). [`RealmGlobalEnv`] holds
//! its handle; the realm record cannot allocate it when it is built, so CreateRealm's caller
//! makes it ([`Agent::make_global_declarative_record`]) once the realm record is stored, as
//! SetRealmGlobalObject sets the global object. The global code's names reach the concrete
//! methods below by name (`GetGlobal` and the other global instructions of `swell-compiler`, and
//! a lookup by name that reaches the global environment); each asks the [[DeclarativeRecord]]
//! first ("If ! DclRec.HasBinding(N) is true"). GlobalDeclarationInstantiation (`script.rs`)
//! calls HasLexicalDeclaration and HasRestrictedGlobalProperty (steps 3 and 4) and creates the
//! bindings (step 15); the evaluation of a declaration initializes one (`InitGlobal`).
//!
//! The map lookup is no step of its own (item 6); a binding is charged when it is created and
//! credited with the realm.

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec::Vec;

use crate::env::{Added, EnvironmentRecord, GlobalBindings, LexicalBinding, RecordNames};
use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::{Completion, Termination};
use crate::interp::frame::SlotValue;
use crate::object::property::{Descriptor, Key, Room};
use crate::value::{ObjectRef, Value};

/// The global environment a realm holds: its [[ObjectRecord]] and [[GlobalThisValue]] are the
/// realm record's global object and global `this`; this holds its [[DeclarativeRecord]] (the
/// module's documentation).
pub(crate) struct RealmGlobalEnv {
    /// The [[DeclarativeRecord]], `None` until the caller of CreateRealm makes it.
    pub(crate) declarative: Option<Handle>,
}

impl RealmGlobalEnv {
    /// A global environment whose [[DeclarativeRecord]] is not made yet.
    pub(crate) fn new() -> RealmGlobalEnv {
        RealmGlobalEnv { declarative: None }
    }

    /// Site of the collector, called by the realm record's trace: the [[DeclarativeRecord]]. The
    /// global object and the global `this` are reported by the realm record itself.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("env/global.rs RealmGlobalEnv::trace");
        if let Some(env) = self.declarative {
            mark.push(Item::Environment(env));
        }
    }

    /// A global environment, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> RealmGlobalEnv {
        RealmGlobalEnv::new()
    }
}

impl<H: Host> Agent<H> {
    /// NewGlobalEnvironment (`#sec-newglobalenvironment`), step 2: the [[DeclarativeRecord]] of
    /// `realm`, an empty declarative record whose bindings are kept by name, charged with its
    /// list's header.
    pub(crate) fn make_global_declarative_record(&mut self, realm: Handle) -> Completion<()> {
        let owned = u64::try_from(size_of::<GlobalBindings>()).unwrap_or(u64::MAX);
        let record = EnvironmentRecord {
            outer: None,
            slots: Vec::new(),
            names: Some(RecordNames::Global(Box::default())),
            added: Added::default(),
            binding_object: None,
        };
        let env = self.store_environment(record, owned)?;
        match self.heap.realms.get_mut(realm) {
            Ok(record) => {
                record.global_env.declarative = Some(env);
                Ok(())
            }
            Err(_) => Err(self.fault()),
        }
    }

    /// The [[DeclarativeRecord]] of the global environment of `realm`.
    fn global_declarative(&mut self, realm: Handle) -> Completion<Handle> {
        match self.realm_record(realm)?.global_env.declarative {
            Some(env) => Ok(env),
            None => Err(self.fault()),
        }
    }

    /// The binding `name` of the [[DeclarativeRecord]] of `realm`, if it has one.
    pub(crate) fn global_lexical(
        &mut self,
        realm: Handle,
        name: Key,
    ) -> Completion<Option<LexicalBinding>> {
        let env = self.global_declarative(realm)?;
        let found = match &self.environment(env)?.names {
            Some(RecordNames::Global(global)) => Some(global.bindings.get(name).copied()),
            _ => None,
        };
        found.ok_or_else(|| self.fault())
    }

    /// Sets the binding `name`, which the [[DeclarativeRecord]] of `realm` has, to `value`.
    fn set_global_lexical(&mut self, realm: Handle, name: Key, value: Value) -> Completion<()> {
        let env = self.global_declarative(realm)?;
        let stored = match self.heap.environments.get_mut(env) {
            Ok(EnvironmentRecord {
                names: Some(RecordNames::Global(global)),
                ..
            }) => global
                .bindings
                .get_mut(name)
                .map(|binding| binding.value = SlotValue::Value(value)),
            _ => None,
        };
        stored.ok_or_else(|| self.fault())
    }

    /// HasLexicalDeclaration (`#sec-haslexicaldeclaration`): whether the [[DeclarativeRecord]]
    /// of `realm` binds `name`.
    pub(crate) fn has_lexical_declaration(&mut self, realm: Handle, name: Key) -> Completion<bool> {
        Ok(self.global_lexical(realm, name)?.is_some())
    }

    /// HasRestrictedGlobalProperty (`#sec-hasrestrictedglobalproperty`): whether the global
    /// object of `realm` has an own non-configurable property `name`.
    pub(crate) fn has_restricted_global_property(
        &mut self,
        realm: Handle,
        name: Key,
    ) -> Completion<bool> {
        let global = self.global_binding_object(realm)?;
        Ok(self
            .get_own_property(global, name)?
            .is_some_and(|property| !property.configurable))
    }

    /// CreateMutableBinding(`name`, false), or with `constant` CreateImmutableBinding(`name`,
    /// true), of the [[DeclarativeRecord]] of `realm` (GlobalDeclarationInstantiation, step 15):
    /// an uninitialized binding, which `name` is not yet. Its growth is charged before it is made;
    /// over the budget the run ends in `Terminated(MemoryBudget)`.
    pub(crate) fn create_global_lexical(
        &mut self,
        realm: Handle,
        name: Key,
        constant: bool,
    ) -> Completion<()> {
        let env = self.global_declarative(realm)?;
        let queue = Rc::clone(&self.credit);
        let hasher = self.heap.hasher;
        let mut room = Room::default();
        let made = match self.heap.environments.get_mut(env) {
            Ok(EnvironmentRecord {
                names: Some(RecordNames::Global(global)),
                ..
            }) => {
                let made = global
                    .bindings
                    .make_room(&mut self.heap.accounting, &hasher, &mut room);
                if room.bytes > 0 {
                    global.bytes = global.bytes.saturating_add(room.bytes);
                    if global.queue.is_none() {
                        global.queue = Some(queue);
                    }
                }
                made
            }
            _ => return Err(self.fault()),
        };
        if made.is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        if room.visited > 0 {
            self.charge_steps(room.visited)?;
        }
        let binding = LexicalBinding {
            value: SlotValue::Uninitialized,
            constant,
        };
        let pushed = match self.heap.environments.get_mut(env) {
            Ok(EnvironmentRecord {
                names: Some(RecordNames::Global(global)),
                ..
            }) => global.bindings.push(name, binding),
            _ => return Err(self.fault()),
        };
        if pushed.is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(())
    }

    /// InitializeBinding(`name`, `value`) of the [[DeclarativeRecord]] of `realm`
    /// (`#sec-declarative-environment-records-initializebinding-n-v`): the evaluation of a
    /// `let` or `const` declaration of a script, whose binding GlobalDeclarationInstantiation
    /// created.
    pub(crate) fn initialize_global_lexical(
        &mut self,
        realm: Handle,
        name: Key,
        value: Value,
    ) -> Completion<()> {
        self.set_global_lexical(realm, name, value)
    }

    /// The [[BindingObject]] of the [[ObjectRecord]] of the global environment of `realm`: its
    /// global object.
    pub(crate) fn global_binding_object(&mut self, realm: Handle) -> Completion<ObjectRef> {
        match self.realm_record(realm)?.global {
            Some(global) => Ok(global),
            None => Err(self.fault()),
        }
    }

    /// GetThisBinding (`#sec-global-environment-records-getthisbinding`): [[GlobalThisValue]].
    pub(crate) fn global_this_binding(&mut self, realm: Handle) -> Completion<Value> {
        match self.realm_record(realm)?.global_this {
            Some(this) => Ok(Value::Object(this)),
            None => Err(self.fault()),
        }
    }

    /// HasBinding (`#sec-global-environment-records-hasbinding-n`): the [[DeclarativeRecord]]'s
    /// bindings, then the [[ObjectRecord]]'s.
    pub(crate) fn global_has_binding(&mut self, realm: Handle, name: Key) -> Completion<bool> {
        if self.global_lexical(realm, name)?.is_some() {
            return Ok(true);
        }
        let global = self.global_binding_object(realm)?;
        self.object_has_binding(global, name)
    }

    /// GetBindingValue (`#sec-global-environment-records-getbindingvalue-n-s`): of a binding
    /// of the [[DeclarativeRecord]], a `ReferenceError` while it is uninitialized.
    pub(crate) fn global_get_binding_value(
        &mut self,
        realm: Handle,
        name: Key,
        strict: bool,
    ) -> Completion<Value> {
        if let Some(binding) = self.global_lexical(realm, name)? {
            return match binding.value {
                SlotValue::Value(value) => Ok(value),
                SlotValue::Uninitialized => {
                    let text = self.key_text(name);
                    Err(self.not_initialized(&text))
                }
                // Only a private record holds a Private Name: a logic error here.
                SlotValue::PrivateName(_) => Err(self.fault()),
            };
        }
        let global = self.global_binding_object(realm)?;
        self.object_get_binding_value(global, name, strict)
    }

    /// SetMutableBinding (`#sec-global-environment-records-setmutablebinding-n-v-s`): of a
    /// binding of the [[DeclarativeRecord]], a `ReferenceError` while it is uninitialized and a
    /// `TypeError` for a `const` one, in any code.
    pub(crate) fn global_set_mutable_binding(
        &mut self,
        realm: Handle,
        name: Key,
        value: Value,
        strict: bool,
    ) -> Completion<()> {
        if let Some(binding) = self.global_lexical(realm, name)? {
            return match (binding.value, binding.constant) {
                (SlotValue::Uninitialized, _) => {
                    let text = self.key_text(name);
                    Err(self.not_initialized(&text))
                }
                (_, true) => {
                    let text = self.key_text(name);
                    self.assign_const(&text)
                }
                (_, false) => self.set_global_lexical(realm, name, value),
            };
        }
        let global = self.global_binding_object(realm)?;
        self.object_set_mutable_binding(global, name, value, strict)
    }

    /// DeleteBinding (`#sec-global-environment-records-deletebinding-n`): a binding of the
    /// [[DeclarativeRecord]] is not deletable (`false`, nothing checked); an own property of
    /// the global object is deleted through the [[ObjectRecord]]; any other name answers true.
    pub(crate) fn global_delete_binding(&mut self, realm: Handle, name: Key) -> Completion<bool> {
        if self.global_lexical(realm, name)?.is_some() {
            return Ok(false);
        }
        let global = self.global_binding_object(realm)?;
        // HasOwnProperty (`#sec-hasownproperty`).
        if self.get_own_property(global, name)?.is_some() {
            return self.object_delete_binding(global, name);
        }
        Ok(true)
    }

    /// CanDeclareGlobalVar (`#sec-candeclareglobalvar`).
    pub(crate) fn can_declare_global_var(&mut self, realm: Handle, name: Key) -> Completion<bool> {
        let global = self.global_binding_object(realm)?;
        // HasOwnProperty (`#sec-hasownproperty`).
        if self.get_own_property(global, name)?.is_some() {
            return Ok(true);
        }
        self.is_extensible(global)
    }

    /// CanDeclareGlobalFunction (`#sec-candeclareglobalfunction`).
    pub(crate) fn can_declare_global_function(
        &mut self,
        realm: Handle,
        name: Key,
    ) -> Completion<bool> {
        let global = self.global_binding_object(realm)?;
        let Some(existing) = self.get_own_property(global, name)? else {
            return self.is_extensible(global);
        };
        if existing.configurable {
            return Ok(true);
        }
        Ok(matches!(
            existing.slot,
            crate::object::property::Slot::Data { writable: true, .. }
        ) && existing.enumerable)
    }

    /// CreateGlobalVarBinding (`#sec-createglobalvarbinding`).
    pub(crate) fn create_global_var_binding(
        &mut self,
        realm: Handle,
        name: Key,
        deletable: bool,
    ) -> Completion<()> {
        let global = self.global_binding_object(realm)?;
        // HasOwnProperty (`#sec-hasownproperty`).
        let has_property = self.get_own_property(global, name)?.is_some();
        let extensible = self.is_extensible(global)?;
        if !has_property && extensible {
            self.object_create_mutable_binding(global, name, deletable)?;
        }
        Ok(())
    }

    /// CreateGlobalFunctionBinding (`#sec-createglobalfunctionbinding`).
    pub(crate) fn create_global_function_binding(
        &mut self,
        realm: Handle,
        name: Key,
        value: Value,
        deletable: bool,
    ) -> Completion<()> {
        let global = self.global_binding_object(realm)?;
        let existing = self.get_own_property(global, name)?;
        let descriptor = match existing {
            None => Descriptor::data(value, true, true, deletable),
            Some(property) if property.configurable => {
                Descriptor::data(value, true, true, deletable)
            }
            Some(_) => Descriptor {
                value: Some(value),
                ..Descriptor::default()
            },
        };
        self.define_property_or_throw(global, name, descriptor)?;
        self.set(global, name, value, false)?;
        Ok(())
    }
}

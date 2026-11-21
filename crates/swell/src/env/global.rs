//! Global Environment Records (ECMA-262 9.1.1.4, `#sec-global-environment-records`), and the one
//! a realm holds as [[GlobalEnv]] ([`RealmGlobalEnv`]: its constructor and its trace). Filled by
//! `story:interpreter-and-script-entry-points`; the realm record of
//! `story:object-model-and-realms` holds the field and calls both.
//!
//! A Global Environment Record is its [[ObjectRecord]], its [[GlobalThisValue]] and its
//! [[DeclarativeRecord]]. In slice 1 the first two are the realm record's `global` and
//! `global_this` (`realm/mod.rs`, set by SetRealmGlobalObject, `#sec-setrealmglobalobject`), and
//! the [[DeclarativeRecord]] is always empty: only `let`, `const` and `class` declarations of
//! global code put a binding there, and they come with slice 6 (`bindings-and-iteration`) and
//! slice 8. So [`RealmGlobalEnv`] holds nothing yet, each concrete method below answers for an
//! empty [[DeclarativeRecord]] ("If ! declRecord.HasBinding(name) is true" is never true), and
//! the global code's names reach these methods by name (`GetGlobal` and the other global
//! instructions of `swell-compiler`).
//!
//! What slice 6 adds, and where:
//!
//! - here, a field of [`RealmGlobalEnv`] holding the handle of a declarative record of the
//!   `environments` arena for the global lexical bindings, with uninitialized bindings (the
//!   temporal dead zone) and their names, made by a constructor `RealmGlobalEnv::new` that
//!   NewGlobalEnvironment (`#sec-newglobalenvironment`) calls, and reported by
//!   [`RealmGlobalEnv::trace`] as an `Item::Environment`;
//! - in `realm/mod.rs`, `RealmRecord::new`, which builds `RealmGlobalEnv {}` today: the record
//!   cannot be allocated there, so CreateRealm's caller makes it once the realm record is stored,
//!   as SetRealmGlobalObject does for the global object;
//! - here, the [[DeclarativeRecord]] branch of each concrete method (HasBinding,
//!   GetBindingValue, SetMutableBinding, DeleteBinding), HasLexicalDeclaration and
//!   HasRestrictedGlobalProperty, which GlobalDeclarationInstantiation (`script.rs`) then calls
//!   for the script's LexicallyDeclaredNames, steps 3 and 4.

use crate::heap::arena::Handle;
use crate::heap::collect::Mark;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::object::property::{Descriptor, Key};
use crate::value::{ObjectRef, Value};

/// The global environment a realm holds: in slice 1 its [[ObjectRecord]] and [[GlobalThisValue]]
/// are the realm record's global object and global `this`, and its [[DeclarativeRecord]] is
/// empty (the module's documentation).
pub(crate) struct RealmGlobalEnv {}

impl RealmGlobalEnv {
    /// Site of the collector, called by the realm record's trace: the [[DeclarativeRecord]],
    /// which holds nothing before slice 6. The global object and the global `this` are reported
    /// by the realm record itself.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("env/global.rs RealmGlobalEnv::trace");
    }

    /// A global environment, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> RealmGlobalEnv {
        RealmGlobalEnv {}
    }
}

impl<H: Host> Agent<H> {
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

    /// HasBinding (`#sec-global-environment-records-hasbinding-n`).
    pub(crate) fn global_has_binding(&mut self, realm: Handle, name: Key) -> Completion<bool> {
        let global = self.global_binding_object(realm)?;
        self.object_has_binding(global, name)
    }

    /// GetBindingValue (`#sec-global-environment-records-getbindingvalue-n-s`).
    pub(crate) fn global_get_binding_value(
        &mut self,
        realm: Handle,
        name: Key,
        strict: bool,
    ) -> Completion<Value> {
        let global = self.global_binding_object(realm)?;
        self.object_get_binding_value(global, name, strict)
    }

    /// SetMutableBinding (`#sec-global-environment-records-setmutablebinding-n-v-s`).
    pub(crate) fn global_set_mutable_binding(
        &mut self,
        realm: Handle,
        name: Key,
        value: Value,
        strict: bool,
    ) -> Completion<()> {
        let global = self.global_binding_object(realm)?;
        self.object_set_mutable_binding(global, name, value, strict)
    }

    /// DeleteBinding (`#sec-global-environment-records-deletebinding-n`): an own property of
    /// the global object is deleted through the [[ObjectRecord]]; any other name answers true.
    pub(crate) fn global_delete_binding(&mut self, realm: Handle, name: Key) -> Completion<bool> {
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

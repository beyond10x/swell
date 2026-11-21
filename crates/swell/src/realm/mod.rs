//! Realms (ECMA-262 9.3, `#sec-code-realms`): the Realm Record (its intrinsics, global object,
//! global `this` value and global environment), CreateRealm (`#sec-createrealm`) and
//! InitializeHostDefinedRealm (`#sec-initializehostdefinedrealm`) with the setup closure of
//! `docs/design/host-interface.md`, decision 7, the current realm, and the well-known symbols
//! (`#sec-well-known-symbols`). Written by `story:object-model-and-realms`.
//!
//! The well-known symbols are agent values, "shared by all realms" (6.1.5.1): the first
//! `create_realm` makes them and every realm record holds them with its intrinsics. The agent
//! holds them weakly and makes them again only when none of them is alive: with no realm left
//! and no value holding one, no code and no host value can tell the new ones from the old, and a
//! dropped last realm gives back every byte it took (HOST-52).
//!
//! Two sites of the collector: the kind site [`RealmRecord::trace`] and the root site
//! [`trace_roots`] (the agent-wide values: none is held strongly, by the paragraph above).

pub(crate) mod global_object;
pub(crate) mod intrinsics;

use crate::env::global::RealmGlobalEnv;
use crate::heap::arena::Handle;
use crate::heap::collect::Mark;
use crate::host::agent::Agent;
use crate::host::host_object::{Brand, HostData, HostObject, Tracer};
use crate::host::outcome::{Abrupt, Completion, Outcome, Termination};
use crate::host::{GlobalObjects, GlobalSetup, HookCx, Host, HostDefined};
use crate::object::ordinary::OrdinaryData;
use crate::value::{ObjectRef, Realm, RealmRef, SymbolRef, Value};
use intrinsics::{Intrinsic, Intrinsics};

/// The well-known symbols of this part (`#sec-well-known-symbols`, Table 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WellKnown {
    pub(crate) has_instance: SymbolRef,
    pub(crate) to_primitive: SymbolRef,
    pub(crate) to_string_tag: SymbolRef,
    pub(crate) species: SymbolRef,
}

impl WellKnown {
    fn all(&self) -> [SymbolRef; 4] {
        [
            self.has_instance,
            self.to_primitive,
            self.to_string_tag,
            self.species,
        ]
    }
}

/// The `[[HostDefined]]` value of a realm, kept as the data of a [`HostObject`] so that its
/// `trace` is called the one way host data is.
struct RealmHostDefined(HostDefined);

impl HostData for RealmHostDefined {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        self.0.trace(tracer);
    }
}

/// A Realm Record (9.3, Table 24).
pub(crate) struct RealmRecord {
    /// [[Intrinsics]]; `None` only while CreateIntrinsics runs.
    pub(crate) intrinsics: Option<Intrinsics>,
    /// The well-known symbols the realm's code sees.
    pub(crate) symbols: Option<WellKnown>,
    /// [[GlobalObject]].
    pub(crate) global: Option<ObjectRef>,
    /// The global `this` value of [[GlobalEnv]] (`#sec-setrealmglobalobject`).
    pub(crate) global_this: Option<ObjectRef>,
    /// [[HostDefined]].
    host_defined: Option<HostObject>,
    /// [[GlobalEnv]].
    pub(crate) global_env: RealmGlobalEnv,
}

impl RealmRecord {
    /// A realm record with nothing in it yet (CreateRealm, step 1).
    fn new() -> RealmRecord {
        RealmRecord {
            intrinsics: None,
            symbols: None,
            global: None,
            global_this: None,
            host_defined: None,
            global_env: RealmGlobalEnv {},
        }
    }

    /// Kind site of the collector: every intrinsic, the well-known symbols, the global object,
    /// the global `this`, the host-defined value and the global environment.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("realm/mod.rs RealmRecord::trace");
        if let Some(intrinsics) = &self.intrinsics {
            intrinsics.trace(mark);
        }
        if let Some(symbols) = &self.symbols {
            for symbol in symbols.all() {
                mark.value(Value::Symbol(symbol));
            }
        }
        for object in [self.global, self.global_this].into_iter().flatten() {
            mark.object(object);
        }
        if let Some(host_defined) = &self.host_defined {
            host_defined.trace(mark);
        }
        self.global_env.trace(mark);
    }

    /// The host-defined value.
    pub(crate) fn host_defined(&self) -> Option<HostDefined> {
        let data: &dyn std::any::Any = self.host_defined.as_ref()?.data.as_deref()?;
        data.downcast_ref::<RealmHostDefined>()
            .map(|held| HostDefined::clone(&held.0))
    }

    /// A realm record, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> RealmRecord {
        RealmRecord {
            global_env: RealmGlobalEnv::for_test(),
            ..RealmRecord::new()
        }
    }
}

/// Root site of the collector: the agent-wide values. The agent holds the well-known symbols
/// weakly (the module's documentation), so this site reports nothing.
pub(crate) fn trace_roots<H: Host>(_agent: &Agent<H>, mark: &mut Mark) {
    mark.count("realm/mod.rs trace_roots");
}

impl<H: Host> Agent<H> {
    /// The realm record of `realm`; a stale one inside the engine faults the agent.
    pub(crate) fn realm_record(&mut self, realm: Handle) -> Completion<&RealmRecord> {
        if self.heap.realms.get(realm).is_err() {
            return Err(self.fault());
        }
        self.heap
            .realms
            .get(realm)
            .map_err(|_| Abrupt::Terminated(Termination::EngineFault))
    }

    /// The intrinsic `which` of `realm`.
    pub(crate) fn intrinsic(&mut self, realm: Handle, which: Intrinsic) -> Completion<ObjectRef> {
        match &self.realm_record(realm)?.intrinsics {
            Some(intrinsics) => Ok(intrinsics.get(which)),
            None => Err(self.fault()),
        }
    }

    /// The current realm (`#sec-execution-contexts`): the realm of the running builtin; while
    /// no code runs, the realm created last that is still alive (a choice of swell: the draft
    /// has no current realm then); `None` when the agent has no realm.
    pub(crate) fn current_realm(&mut self) -> Option<Handle> {
        if let Some(realm) = self
            .activations
            .last()
            .and_then(|activation| activation.realm)
        {
            return Some(realm);
        }
        while let Some(&realm) = self.realm_list.last() {
            if self.heap.realms.get(realm).is_ok() {
                return Some(realm);
            }
            self.realm_list.pop();
        }
        None
    }

    /// The intrinsic `which` of the current realm, `None` when the agent has no realm.
    pub(crate) fn current_intrinsic(&mut self, which: Intrinsic) -> Completion<Option<ObjectRef>> {
        match self.current_realm() {
            Some(realm) => self.intrinsic(realm, which).map(Some),
            None => Ok(None),
        }
    }

    /// The well-known symbols, when they are alive.
    pub(crate) fn well_known(&self) -> Option<WellKnown> {
        let symbols = self.symbols?;
        symbols
            .all()
            .iter()
            .all(|symbol| self.heap.symbols.get(symbol.handle).is_ok())
            .then_some(symbols)
    }

    /// The well-known symbols, made when none is alive.
    fn ensure_well_known(&mut self) -> Completion<WellKnown> {
        if let Some(symbols) = self.well_known() {
            return Ok(symbols);
        }
        let mut made = Vec::new();
        for description in [
            "Symbol.hasInstance",
            "Symbol.toPrimitive",
            "Symbol.toStringTag",
            "Symbol.species",
        ] {
            let description = self.new_string_str(description)?;
            match self.heap.new_symbol(Some(description)) {
                Ok(symbol) => made.push(symbol),
                Err(_) => return Err(self.terminate(Termination::MemoryBudget)),
            }
        }
        let &[has_instance, to_primitive, to_string_tag, species] = made.as_slice() else {
            return Err(self.fault());
        };
        let symbols = WellKnown {
            has_instance,
            to_primitive,
            to_string_tag,
            species,
        };
        self.symbols = Some(symbols);
        Ok(symbols)
    }

    /// InitializeHostDefinedRealm (`#sec-initializehostdefinedrealm`) with CreateRealm
    /// (`#sec-createrealm`): the intrinsics, then the setup closure, then SetRealmGlobalObject
    /// (`#sec-setrealmglobalobject`) and SetDefaultGlobalBindings. What the closure returns is
    /// checked before it is used (decision 6). No step is charged: no code runs.
    pub(crate) fn initialize_host_defined_realm(
        &mut self,
        setup: Option<GlobalSetup<'_, H>>,
        host_defined: Option<HostDefined>,
    ) -> Outcome<Realm> {
        match self.create_realm_steps(setup, host_defined) {
            Ok(realm) => {
                self.realm_list
                    .retain(|&kept| self.heap.realms.get(kept).is_ok());
                self.realm_list.push(realm);
                Outcome::Normal(Realm(self.heap.roots.add_realm(realm)))
            }
            Err(Abrupt::Throw(value)) => Outcome::Throw(self.heap.roots.add(value)),
            Err(Abrupt::Terminated(termination)) => Outcome::Terminated(termination),
        }
    }

    fn create_realm_steps(
        &mut self,
        setup: Option<GlobalSetup<'_, H>>,
        host_defined: Option<HostDefined>,
    ) -> Completion<Handle> {
        let symbols = self.ensure_well_known()?;
        let record = RealmRecord {
            symbols: Some(symbols),
            host_defined: host_defined.map(|value| HostObject {
                brand: Brand(0),
                data: Some(Box::new(RealmHostDefined(value))),
            }),
            ..RealmRecord::new()
        };
        let realm =
            match crate::heap::store(&mut self.heap.accounting, &mut self.heap.realms, record, 0) {
                Ok(realm) => realm,
                Err(_) => return Err(self.terminate(Termination::MemoryBudget)),
            };
        self.create_intrinsics(realm)?;

        let (global, this) = match setup {
            Some(setup) => {
                let start = self.scope.len();
                let answer = setup(&mut HookCx { agent: self }, RealmRef { handle: realm });
                self.scope.truncate(start);
                self.running()?;
                match answer {
                    Ok(GlobalObjects { global, this }) => {
                        let stale = self.heap.objects.get(global.handle).is_err()
                            || this.is_some_and(|this| self.heap.objects.get(this.handle).is_err());
                        if stale {
                            return Err(self.fault());
                        }
                        (global, this)
                    }
                    Err(Abrupt::Throw(value)) => {
                        if self.heap.check_value(value).is_err() {
                            return Err(self.fault());
                        }
                        return Err(Abrupt::Throw(value));
                    }
                    Err(Abrupt::Terminated(termination)) => {
                        return Err(self.terminate(termination));
                    }
                }
            }
            None => {
                let prototype = self.intrinsic(realm, Intrinsic::ObjectPrototype)?;
                (
                    self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?,
                    None,
                )
            }
        };
        if let Ok(record) = self.heap.realms.get_mut(realm) {
            record.global = Some(global);
            record.global_this = Some(this.unwrap_or(global));
        }
        self.set_default_global_bindings(realm)?;
        Ok(realm)
    }
}

#[cfg(test)]
mod tests {
    use crate::host::hooks::test_host::agent_with_realm;
    use crate::object::KindTag;
    use crate::realm::intrinsics::Intrinsic;

    /// HOST-51 (`#sec-initializehostdefinedrealm`, step 7: "Let global be
    /// OrdinaryObjectCreate(realm.[[Intrinsics]].[[%Object.prototype%]])"): without a setup
    /// closure the global object is an ordinary object whose [[Prototype]] is the realm's
    /// `%Object.prototype%`, and the global `this` is the global object.
    #[test]
    fn host_51_default_global_is_ordinary_with_object_prototype() {
        let (mut agent, realm) = agent_with_realm();
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let object_prototype = agent.intrinsic(handle, Intrinsic::ObjectPrototype).unwrap();
        let record = agent.realm_record(handle).unwrap();
        let (global, this) = (record.global.unwrap(), record.global_this.unwrap());
        assert_eq!(global, this);
        assert_eq!(agent.kind_of(global), Ok(KindTag::Ordinary));
        assert_eq!(agent.get_prototype_of(global), Ok(Some(object_prototype)));
        assert_eq!(agent.get_prototype_of(object_prototype), Ok(None));
    }

    /// With only a `Realm` root kept, a collection keeps `%Object.prototype%`, the global
    /// object's prototype, reachable through the realm record alone (the public part is
    /// `tests/realms.rs`, `intrinsic_reachable_only_through_the_realm_survives_collect`).
    #[test]
    fn object_prototype_survives_collect_through_the_realm() {
        let (mut agent, realm) = agent_with_realm();
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let before = agent.intrinsic(handle, Intrinsic::ObjectPrototype).unwrap();
        let global = agent.realm_record(handle).unwrap().global.unwrap();
        assert!(matches!(agent.collect(), Ok(crate::Outcome::Normal(_))));
        assert!(matches!(agent.collect(), Ok(crate::Outcome::Normal(_))));
        let after = agent.intrinsic(handle, Intrinsic::ObjectPrototype).unwrap();
        assert_eq!(after, before);
        assert!(agent.heap.objects.get(before.handle).is_ok());
        assert_eq!(agent.get_prototype_of(global), Ok(Some(before)));
        assert_eq!(agent.kind_of(before), Ok(KindTag::ImmutablePrototype));
    }
}

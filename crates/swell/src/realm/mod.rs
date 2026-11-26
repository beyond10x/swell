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

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::Cell;
use swell_num::random::Xoshiro256;

pub(crate) mod global_object;
pub(crate) mod intrinsics;

use crate::env::global::RealmGlobalEnv;
use crate::heap::arena::{Access, Handle};
use crate::heap::collect::Mark;
use crate::host::agent::Agent;
use crate::host::host_object::{Brand, HostData, HostObject, Tracer};
use crate::host::outcome::{Abrupt, Completion, Outcome, Termination};
use crate::host::{GlobalObjects, GlobalSetup, HookCx, Host, HostDefined};
use crate::object::ordinary::OrdinaryData;
use crate::value::{ObjectRef, Realm, RealmRef, SymbolRef, Value};
use intrinsics::{Intrinsic, Intrinsics};

/// The well-known symbols (`#sec-well-known-symbols`, Table 1): thirteen, the seven of slice 1,
/// the five `story:symbol-constructor-and-well-known-symbols` adds with the `Symbol`
/// constructor, whose properties they are (`builtins/symbol.rs`), and `%Symbol.asyncIterator%`,
/// the last, of slice 10 (`story:for-await-and-async-from-sync-iterator`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WellKnown {
    pub(crate) has_instance: SymbolRef,
    pub(crate) to_primitive: SymbolRef,
    pub(crate) to_string_tag: SymbolRef,
    pub(crate) species: SymbolRef,
    /// @@unscopables, which HasBinding of the Object Environment Record of a `with` statement
    /// reads (`#sec-object-environment-records-hasbinding-n`).
    pub(crate) unscopables: SymbolRef,
    /// @@isConcatSpreadable, which IsConcatSpreadable reads for `Array.prototype.concat`
    /// (`#sec-isconcatspreadable`). No script obtains it before `Symbol` (slice 6).
    pub(crate) is_concat_spreadable: SymbolRef,
    /// @@match, which IsRegExp reads for `includes`, `startsWith` and `endsWith` of
    /// `String.prototype` (`#sec-isregexp`). No script obtains it before `Symbol` (slice 6).
    pub(crate) r#match: SymbolRef,
    /// @@iterator (`#sec-symbol.iterator`); the iterator protocol that reads it is a later story
    /// of slice 6.
    pub(crate) iterator: SymbolRef,
    /// @@matchAll (`#sec-symbol.matchall`), a hook of `RegExp` (slice 5).
    pub(crate) match_all: SymbolRef,
    /// @@replace (`#sec-symbol.replace`), a hook of `RegExp` (slice 5).
    pub(crate) replace: SymbolRef,
    /// @@search (`#sec-symbol.search`), a hook of `RegExp` (slice 5).
    pub(crate) search: SymbolRef,
    /// @@split (`#sec-symbol.split`), a hook of `RegExp` (slice 5).
    pub(crate) split: SymbolRef,
    /// @@asyncIterator (`#sec-symbol.asynciterator`), which GetIterator with kind async reads
    /// (`#sec-getiterator`), the last, of slice 10.
    pub(crate) async_iterator: SymbolRef,
}

/// The [[Description]] of each well-known symbol, in the order of [`WellKnown::all`].
const DESCRIPTIONS: [&str; 13] = [
    "Symbol.hasInstance",
    "Symbol.toPrimitive",
    "Symbol.toStringTag",
    "Symbol.species",
    "Symbol.unscopables",
    "Symbol.isConcatSpreadable",
    "Symbol.match",
    "Symbol.iterator",
    "Symbol.matchAll",
    "Symbol.replace",
    "Symbol.search",
    "Symbol.split",
    "Symbol.asyncIterator",
];

impl WellKnown {
    /// Every symbol, in the order they are made; the tracer reads each.
    fn all(&self) -> [SymbolRef; 13] {
        [
            self.has_instance,
            self.to_primitive,
            self.to_string_tag,
            self.species,
            self.unscopables,
            self.is_concat_spreadable,
            self.r#match,
            self.iterator,
            self.match_all,
            self.replace,
            self.search,
            self.split,
            self.async_iterator,
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
    /// `create_realm` ended without making the realm (the setup closure threw or ended the
    /// run, or a budget ran out): the record stays until a collection frees it, and the host,
    /// which may still hold the `RealmRef` its setup closure was handed, is answered as for a
    /// freed realm ([`Agent::host_realm`]).
    pub(crate) unmade: bool,
    /// The state of the realm's `Math.random` generator (`swell_num::random::Xoshiro256`;
    /// `docs/design/engine.md`, decision 6, "`Math.random`"): derived from the agent's
    /// `random_seed` and the realm's creation index when the record is made, and moved only by
    /// a call of this realm's `Math.random` (`builtins/math.rs`).
    pub(crate) random: Cell<[u64; 4]>,
}

impl RealmRecord {
    /// A realm record with nothing in it yet (CreateRealm, step 1), whose generator starts at
    /// `random`.
    fn new(random: [u64; 4]) -> RealmRecord {
        RealmRecord {
            intrinsics: None,
            symbols: None,
            global: None,
            global_this: None,
            host_defined: None,
            global_env: RealmGlobalEnv::new(),
            unmade: false,
            random: Cell::new(random),
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
        let data: &dyn core::any::Any = self.host_defined.as_ref()?.data.as_deref()?;
        data.downcast_ref::<RealmHostDefined>()
            .map(|held| HostDefined::clone(&held.0))
    }

    /// A realm record, for the tests of the collector, whose generator starts at the fixed
    /// state (1, 2, 3, 4).
    #[cfg(test)]
    pub(crate) fn for_test() -> RealmRecord {
        RealmRecord {
            global_env: RealmGlobalEnv::for_test(),
            ..RealmRecord::new([1, 2, 3, 4])
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

    /// The current realm (`#sec-execution-contexts`, "The value of the Realm component of the
    /// running execution context"): the [[Realm]] of the running builtin; while no code runs,
    /// the realm the entry names (`docs/design/host-interface.md`, decision 7: `with_cx`,
    /// `Agent::call` and `Agent::construct` take it, `create_realm` sets the new one). Every
    /// engine operation runs inside an entry, so no current realm, or a dead one, is a broken
    /// invariant and faults the agent.
    pub(crate) fn current_realm(&mut self) -> Completion<Handle> {
        let realm = match self.activations.last() {
            Some(activation) => activation.realm,
            None => self.entry_realm,
        };
        match realm {
            Some(realm) if self.heap.realms.get(realm).is_ok() => Ok(realm),
            _ => Err(self.fault()),
        }
    }

    /// The intrinsic `which` of the current realm.
    pub(crate) fn current_intrinsic(&mut self, which: Intrinsic) -> Completion<ObjectRef> {
        let realm = self.current_realm()?;
        self.intrinsic(realm, which)
    }

    /// Run `f` with `realm` as the realm of the entry, the current realm while no builtin runs;
    /// the realm before is restored after.
    pub(crate) fn in_entry_realm<R>(&mut self, realm: Handle, f: impl FnOnce(&mut Self) -> R) -> R {
        let before = self.entry_realm.replace(realm);
        let result = f(self);
        self.entry_realm = before;
        result
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
        for description in DESCRIPTIONS {
            let description = self.new_string_str(description)?;
            match self.heap.new_symbol(Some(description)) {
                Ok(symbol) => made.push(symbol),
                Err(_) => return Err(self.terminate(Termination::MemoryBudget)),
            }
        }
        let &[
            has_instance,
            to_primitive,
            to_string_tag,
            species,
            unscopables,
            is_concat_spreadable,
            r#match,
            iterator,
            match_all,
            replace,
            search,
            split,
            async_iterator,
        ] = made.as_slice()
        else {
            return Err(self.fault());
        };
        let symbols = WellKnown {
            has_instance,
            to_primitive,
            to_string_tag,
            species,
            unscopables,
            is_concat_spreadable,
            r#match,
            iterator,
            match_all,
            replace,
            search,
            split,
            async_iterator,
        };
        self.symbols = Some(symbols);
        Ok(symbols)
    }

    /// InitializeHostDefinedRealm (`#sec-initializehostdefinedrealm`) with CreateRealm
    /// (`#sec-createrealm`): the intrinsics, then the setup closure, then SetRealmGlobalObject
    /// (`#sec-setrealmglobalobject`) and SetDefaultGlobalBindings. What the closure returns is
    /// checked before it is used (decision 6). No step is charged: no code runs. `index` is the
    /// realm's creation index, which `Agent::create_realm` counted: the record's `Math.random`
    /// state is derived from it and the agent's `random_seed`.
    pub(crate) fn initialize_host_defined_realm(
        &mut self,
        setup: Option<GlobalSetup<'_, H>>,
        host_defined: Option<HostDefined>,
        index: u64,
    ) -> Outcome<Realm> {
        match self.create_realm_steps(setup, host_defined, index) {
            Ok(realm) => Outcome::Normal(Realm(self.heap.roots.add_realm(realm))),
            Err(Abrupt::Throw(value)) => Outcome::Throw(self.heap.roots.add(value)),
            Err(Abrupt::Terminated(termination)) => Outcome::Terminated(termination),
        }
    }

    fn create_realm_steps(
        &mut self,
        setup: Option<GlobalSetup<'_, H>>,
        host_defined: Option<HostDefined>,
        index: u64,
    ) -> Completion<Handle> {
        let symbols = self.ensure_well_known()?;
        let random = Xoshiro256::from_seed(&self.random_seed, index).state();
        let record = RealmRecord {
            symbols: Some(symbols),
            host_defined: host_defined.map(|value| HostObject {
                brand: Brand(0),
                data: Some(Box::new(RealmHostDefined(value))),
                error_kind: None,
            }),
            ..RealmRecord::new(random)
        };
        let realm =
            match crate::heap::store(&mut self.heap.accounting, &mut self.heap.realms, record, 0) {
                Ok(realm) => realm,
                Err(_) => return Err(self.terminate(Termination::MemoryBudget)),
            };
        // "Set the Realm of newContext to realm. ... Push newContext onto the execution context
        // stack": the new realm is the current realm until the realm is made.
        let populated = self.in_entry_realm(realm, |agent| agent.populate_realm(realm, setup));
        if populated.is_err()
            && let Ok(record) = self.heap.realms.get_mut(realm)
        {
            record.unmade = true;
        }
        populated?;
        Ok(realm)
    }

    /// The realm record a host's `RealmRef` names: a realm `create_realm` did not make answers
    /// `Stale`, as a freed one does (HOST-30).
    pub(crate) fn host_realm(&self, realm: Handle) -> Result<&RealmRecord, Access> {
        match self.heap.realms.get(realm) {
            Ok(record) if record.unmade => Err(Access::Stale),
            answer => answer,
        }
    }

    /// CreateIntrinsics, the setup closure, SetRealmGlobalObject (with the global environment's
    /// [[DeclarativeRecord]]) and SetDefaultGlobalBindings of the realm record `realm`.
    fn populate_realm(
        &mut self,
        realm: Handle,
        setup: Option<GlobalSetup<'_, H>>,
    ) -> Completion<()> {
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
        // NewGlobalEnvironment (`#sec-newglobalenvironment`), step 2: the [[DeclarativeRecord]],
        // made once the realm record holds the global object (`env/global.rs`).
        self.make_global_declarative_record(realm)?;
        self.set_default_global_bindings(realm)
    }
}

#[cfg(test)]
mod tests {
    use crate::host::ErrorKind;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm};
    use crate::host::outcome::Outcome;
    use crate::object::KindTag;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Realm, Value};
    use alloc::string::String;

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

    /// `#sec-well-known-symbols`: %Symbol.isConcatSpreadable% and %Symbol.match%, which
    /// IsConcatSpreadable (`#sec-isconcatspreadable`) and IsRegExp (`#sec-isregexp`) read, are
    /// two Symbols with the descriptions of Table 1, distinct from each other and from the
    /// five others; "shared by all realms", so two realms of one agent hold the same ones; a
    /// realm keeps them alive across a collection, and they are the same after it.
    #[test]
    fn is_concat_spreadable_and_match_are_well_known_in_every_realm() {
        let (mut agent, a) = agent_with_realm();
        let b = crate::host::hooks::test_host::new_realm(&mut agent);
        let symbols_of = |agent: &mut Agent<NoHooks>, realm: &Realm| {
            let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
            agent.realm_record(handle).unwrap().symbols.unwrap()
        };
        let symbols = symbols_of(&mut agent, &a);
        assert_eq!(symbols_of(&mut agent, &b), symbols);
        assert_eq!(agent.well_known(), Some(symbols));
        let all = symbols.all();
        for (index, symbol) in all.iter().enumerate() {
            for other in all.iter().skip(index + 1) {
                assert_ne!(symbol, other);
            }
        }
        for (symbol, expected) in [
            (symbols.is_concat_spreadable, "Symbol.isConcatSpreadable"),
            (symbols.r#match, "Symbol.match"),
        ] {
            assert!(all.contains(&symbol));
            let description = agent
                .heap
                .symbols
                .get(symbol.handle)
                .unwrap()
                .description
                .unwrap();
            let units = agent.string_units_for_tests(description).unwrap();
            assert_eq!(String::from_utf16_lossy(&units), expected);
        }
        drop(b);
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        assert_eq!(symbols_of(&mut agent, &a), symbols);
        assert_eq!(agent.well_known(), Some(symbols));
    }

    /// What HOST-161 reads of realm `a`: ToObject of `true` in `with_cx` of `a`, and the
    /// TypeError of `agent.call` with `a` of the Number 1, have `a`'s prototypes.
    fn check_current_realm_is(
        agent: &mut Agent<NoHooks>,
        a: &Realm,
        boolean_prototype: ObjectRef,
        type_error_prototype: ObjectRef,
    ) {
        let (wrapper_prototype, one, undefined) = agent
            .with_cx(a, |cx| {
                let wrapper = cx.to_object(Value::Boolean(true)).unwrap().unwrap();
                (
                    cx.agent.get_prototype_of(wrapper),
                    cx.root(Value::Number(1.0)).unwrap(),
                    cx.root(Value::Undefined).unwrap(),
                )
            })
            .unwrap();
        assert_eq!(wrapper_prototype, Ok(Some(boolean_prototype)));
        let thrown = match agent.call(a, &one, &undefined, &[]) {
            Ok(Outcome::Throw(thrown)) => agent.heap.roots.get(&thrown).unwrap(),
            other => panic!("agent.call answered {other:?}"),
        };
        let Value::Object(error) = thrown else {
            panic!("thrown {thrown:?}");
        };
        assert_eq!(
            agent.get_prototype_of(error),
            Ok(Some(type_error_prototype))
        );
    }

    /// HOST-161 (`docs/design/host-interface.md`, decision 7, "The current realm is the running
    /// context's; outside a run, the host names it"): with realm A created and then realm B,
    /// `with_cx` of A and `agent.call` with A use A's intrinsics while B is alive, after the
    /// host dropped every root of B, and after B is collected. Run also with the feature
    /// `collect-every-safe-point`.
    #[test]
    fn host_161_the_entry_names_the_current_realm() {
        let (mut agent, a) = agent_with_realm();
        let b = match agent.create_realm(None, None) {
            Ok(Outcome::Normal(b)) => b,
            other => panic!("create_realm answered {other:?}"),
        };
        let a_handle = agent.heap.roots.get_realm(&a.0).unwrap();
        let b_handle = agent.heap.roots.get_realm(&b.0).unwrap();
        let boolean_prototype = agent
            .intrinsic(a_handle, Intrinsic::BooleanPrototype)
            .unwrap();
        let type_error_prototype = agent
            .intrinsic(a_handle, Intrinsic::ErrorPrototype(ErrorKind::TypeError))
            .unwrap();
        assert_ne!(
            agent.intrinsic(b_handle, Intrinsic::BooleanPrototype),
            Ok(boolean_prototype)
        );

        check_current_realm_is(&mut agent, &a, boolean_prototype, type_error_prototype);
        drop(b);
        check_current_realm_is(&mut agent, &a, boolean_prototype, type_error_prototype);
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        assert!(agent.heap.realms.get(b_handle).is_err());
        check_current_realm_is(&mut agent, &a, boolean_prototype, type_error_prototype);
    }
}

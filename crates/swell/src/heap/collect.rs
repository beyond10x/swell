//! The collector (`docs/design/engine.md`, decision 4, "Collector"; `docs/design/host-interface.md`,
//! decision 18): precise, stop-the-world mark and sweep over the arenas. It runs only when the
//! host asks (`collect`) and at safe points, when the bytes allocated since the last collection
//! pass a threshold, or at every safe point with the feature `collect-every-safe-point`
//! (decisions 4 and 13). It runs no ECMAScript code and calls no hook (HOST-129).
//!
//! The mark phase is a loop over explicit worklists, never a Rust recursion, fed by two kinds
//! of sites, each defined once in the file of what it traces and called from here in a fixed
//! order:
//!
//! - a root site per module that owns roots, `fn trace_roots(agent, mark: &mut Mark)`: the root
//!   table (`heap/roots.rs`), the GlobalSymbolRegistry (`heap/mod.rs`, `trace_registry`,
//!   written by `story:symbol-constructor-and-well-known-symbols`), the handles of the running
//!   `with_cx`, host function and hook scopes (`host/agent.rs`), the agent-wide values
//!   (`realm/mod.rs`), the slots of running builtins (`object/builtin_function.rs`) and the
//!   frame stack (`interp/mod.rs`);
//! - a trace function per kind of record, `fn trace(&self, mark: &mut Mark)`: strings
//!   (`heap/string.rs`), symbols (`heap/mod.rs`), objects (`object/mod.rs`, which calls the trace
//!   of the object's kind in `object/*.rs` and `host/host_object.rs`), realm records (`realm/mod.rs`, with the global
//!   environment of `env/global.rs`), environment records (`env/mod.rs`) and script records
//!   (`script.rs`).
//!
//! The workspace lives in the arenas and is charged with their capacity (`heap/arena.rs`): a
//! mark word per slot and room on a worklist per slot. A collection takes it out of the arenas,
//! marks an item when a site reports it and puts it on its arena's worklist only the first time,
//! so a worklist holds at most one entry per slot; it gives the workspace back cleared. A
//! collection allocates nothing, so one that starts at the memory limit finishes.
//!
//! A site whose owner comes in a later story reports nothing, since no such record can exist
//! before it. A handle a site reports that is stale or of another agent (a report for a slot
//! marked by another generation, or a record that is not there when it is traced) ends the
//! collection in `Terminated(EngineFault)`, before anything is swept. The sweep then frees every
//! record not marked, in slot order, drops what it owns (host data included) and credits the
//! bytes it owned; the slot stays charged with its arena's capacity.

use super::Heap;
use super::arena::{Access, Handle, Marked, Workspace};
use crate::host::agent::Agent;
use crate::host::{CollectReport, Host};
use crate::value::{ObjectRef, Value};

/// What a site reports: a handle into one of the arenas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Item {
    String(Handle),
    Symbol(Handle),
    Object(Handle),
    Realm(Handle),
    Environment(Handle),
    Script(Handle),
    /// A BigInt: no arena holds them before slice 12, so no live one exists.
    NoArena,
}

/// The mark phase's state: the workspace of each arena, taken out of the arenas for the
/// collection, and whether a site reported a stale handle. It is the [`crate::Tracer`] host
/// data reports through.
pub(crate) struct Mark {
    /// The agent's key, carried by every handle of its heap.
    key: u32,
    strings: Workspace,
    symbols: Workspace,
    objects: Workspace,
    realms: Workspace,
    environments: Workspace,
    scripts: Workspace,
    /// A site reported a stale handle, or one of another agent.
    stale: bool,
    /// The sites called, in order, in the counting build of the tests.
    #[cfg(test)]
    calls: Vec<&'static str>,
    /// The counting build: pushes a worklist had no room for.
    #[cfg(test)]
    allocations: u64,
    /// The counting build: the most entries the worklists held at once.
    #[cfg(test)]
    peak: usize,
}

impl Mark {
    /// Take the workspace out of the arenas of `heap` for a mark phase: moves it, allocates
    /// nothing.
    pub(crate) fn take(heap: &mut Heap) -> Mark {
        Mark {
            key: heap.key,
            strings: heap.strings.take_workspace(),
            symbols: heap.symbols.take_workspace(),
            objects: heap.objects.take_workspace(),
            realms: heap.realms.take_workspace(),
            environments: heap.environments.take_workspace(),
            scripts: heap.scripts.take_workspace(),
            stale: false,
            #[cfg(test)]
            calls: Vec::new(),
            #[cfg(test)]
            allocations: 0,
            #[cfg(test)]
            peak: 0,
        }
    }

    /// Give the workspace back to the arenas of `heap`, the worklists empty; the mark words
    /// stay for the sweep.
    fn give_back(self, heap: &mut Heap) {
        heap.strings.give_back_workspace(self.strings);
        heap.symbols.give_back_workspace(self.symbols);
        heap.objects.give_back_workspace(self.objects);
        heap.realms.give_back_workspace(self.realms);
        heap.environments.give_back_workspace(self.environments);
        heap.scripts.give_back_workspace(self.scripts);
    }

    /// Report `item`: mark it, and put it on its arena's worklist the first time. A report for
    /// a slot marked by another generation, of another agent or of no arena is a stale handle.
    pub(crate) fn push(&mut self, item: Item) {
        let (workspace, handle) = match item {
            Item::String(handle) => (&mut self.strings, handle),
            Item::Symbol(handle) => (&mut self.symbols, handle),
            Item::Object(handle) => (&mut self.objects, handle),
            Item::Realm(handle) => (&mut self.realms, handle),
            Item::Environment(handle) => (&mut self.environments, handle),
            Item::Script(handle) => (&mut self.scripts, handle),
            Item::NoArena => {
                self.stale = true;
                return;
            }
        };
        if handle.key != self.key {
            self.stale = true;
            return;
        }
        match workspace.mark(handle.index, handle.generation) {
            Marked::New | Marked::Before => {}
            Marked::Stale => self.stale = true,
            Marked::Full => {
                #[cfg(test)]
                {
                    self.allocations += 1;
                }
                self.stale = true;
            }
        }
        #[cfg(test)]
        {
            let held = self.strings.held()
                + self.symbols.held()
                + self.objects.held()
                + self.realms.held()
                + self.environments.held()
                + self.scripts.held();
            self.peak = self.peak.max(held);
        }
    }

    /// Report the handle `value` holds, if any.
    pub(crate) fn value(&mut self, value: Value) {
        match value {
            Value::Undefined | Value::Null | Value::Boolean(_) | Value::Number(_) => {}
            Value::String(string) => self.push(Item::String(string.handle)),
            Value::Object(object) => self.push(Item::Object(object.handle)),
            Value::Symbol(symbol) => self.push(Item::Symbol(symbol.handle)),
            Value::BigInt(_) => self.push(Item::NoArena),
        }
    }

    /// Report an object.
    pub(crate) fn object(&mut self, object: ObjectRef) {
        self.push(Item::Object(object.handle));
    }

    /// Every site calls this first with its name; the counting build of the tests records it.
    pub(crate) fn count(&mut self, site: &'static str) {
        #[cfg(test)]
        self.calls.push(site);
        #[cfg(not(test))]
        let _ = site;
    }

    /// The sites called so far, in order.
    #[cfg(test)]
    pub(crate) fn calls(&self) -> &[&'static str] {
        &self.calls
    }

    /// Whether `item` is marked.
    #[cfg(test)]
    pub(crate) fn is_marked(&self, item: Item) -> bool {
        let (workspace, handle) = match item {
            Item::String(handle) => (&self.strings, handle),
            Item::Symbol(handle) => (&self.symbols, handle),
            Item::Object(handle) => (&self.objects, handle),
            Item::Realm(handle) => (&self.realms, handle),
            Item::Environment(handle) => (&self.environments, handle),
            Item::Script(handle) => (&self.scripts, handle),
            Item::NoArena => return false,
        };
        workspace.is_marked(handle.index, handle.generation)
    }

    /// The next marked item to trace, as the handle it was marked by.
    fn pop(&mut self) -> Option<Item> {
        let key = self.key;
        let handle = |(index, generation): (u32, u32)| Handle {
            key,
            index,
            generation,
        };
        if let Some(slot) = self.objects.pop() {
            return Some(Item::Object(handle(slot)));
        }
        if let Some(slot) = self.strings.pop() {
            return Some(Item::String(handle(slot)));
        }
        if let Some(slot) = self.symbols.pop() {
            return Some(Item::Symbol(handle(slot)));
        }
        if let Some(slot) = self.environments.pop() {
            return Some(Item::Environment(handle(slot)));
        }
        if let Some(slot) = self.realms.pop() {
            return Some(Item::Realm(handle(slot)));
        }
        if let Some(slot) = self.scripts.pop() {
            return Some(Item::Script(handle(slot)));
        }
        None
    }
}

/// A site reported a stale handle, or one of another agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stale;

/// The mark phase: the root sites in their fixed order, then the worklists until they are
/// empty, each record read, checked and traced once by the trace function of its kind.
pub(crate) fn mark_phase<H: Host>(agent: &Agent<H>, mark: &mut Mark) -> Result<(), Stale> {
    super::roots::trace_roots(agent, mark);
    super::trace_registry(agent, mark);
    crate::host::agent::trace_roots(agent, mark);
    crate::realm::trace_roots(agent, mark);
    crate::object::builtin_function::trace_roots(agent, mark);
    crate::interp::trace_roots(agent, mark);

    let heap = &agent.heap;
    let stale = |_: Access| Stale;
    while let Some(item) = mark.pop() {
        if mark.stale {
            return Err(Stale);
        }
        match item {
            Item::String(handle) => heap.strings.get(handle).map_err(stale)?.trace(mark),
            Item::Symbol(handle) => heap.symbols.get(handle).map_err(stale)?.trace(mark),
            Item::Object(handle) => heap.objects.get(handle).map_err(stale)?.trace(mark),
            Item::Realm(handle) => heap.realms.get(handle).map_err(stale)?.trace(mark),
            Item::Environment(handle) => {
                heap.environments.get(handle).map_err(stale)?.trace(mark);
            }
            Item::Script(handle) => heap.scripts.get(handle).map_err(stale)?.trace(mark),
            Item::NoArena => return Err(Stale),
        }
    }
    if mark.stale { Err(Stale) } else { Ok(()) }
}

/// A full collection: release the dropped roots, mark, and sweep every arena and the atom
/// table. On a stale handle nothing is swept and the caller faults the agent. The collection
/// threshold is set by the caller, `Agent::collect_now`, once every credit of the collection
/// is made.
pub(crate) fn collect<H: Host>(agent: &mut Agent<H>) -> Result<CollectReport, Stale> {
    agent.heap.roots.drain();
    let heap_bytes_before = agent.heap.bytes();
    let mut mark = Mark::take(&mut agent.heap);
    let marked = mark_phase(agent, &mut mark);
    #[cfg(test)]
    let (allocations, peak) = (mark.allocations, mark.peak);
    mark.give_back(&mut agent.heap);

    let heap = &mut agent.heap;
    if let Err(stale) = marked {
        heap.clear_marks();
        return Err(stale);
    }
    #[cfg(test)]
    let growths = heap.free_list_growths();
    let mut credited = heap.objects.sweep();
    credited = credited.saturating_add(heap.strings.sweep());
    credited = credited.saturating_add(heap.symbols.sweep());
    credited = credited.saturating_add(heap.realms.sweep());
    credited = credited.saturating_add(heap.environments.sweep());
    credited = credited.saturating_add(heap.scripts.sweep());
    credited = credited.saturating_add(heap.atoms.sweep(&heap.strings));
    heap.accounting.credit(credited);
    #[cfg(test)]
    {
        heap.collect_allocations = allocations + (heap.free_list_growths() - growths);
        heap.collect_peak_work = peak;
    }
    Ok(CollectReport {
        heap_bytes_before,
        heap_bytes_after: heap.bytes(),
    })
}

#[cfg(test)]
mod tests {
    use super::{Item, Mark, mark_phase};
    use crate::env::EnvironmentRecord;
    use crate::heap::SymbolRecord;
    use crate::heap::string::HeapString;
    use crate::host::hooks::test_host;
    use crate::host::hooks::test_host::NoHooks;
    use crate::host::host_object::HostObject;
    use crate::object::arguments::ArgumentsObject;
    use crate::object::array::Array;
    use crate::object::bound_function::BoundFunction;
    use crate::object::builtin_function::BuiltinFunction;
    use crate::object::ecmascript_function::EcmascriptFunction;
    use crate::object::ordinary::Ordinary;
    use crate::object::property::{Key, Property};
    use crate::object::regexp::RegExpObject;
    use crate::object::string::StringObject;
    use crate::object::{ObjectKind, ObjectRecord};
    use crate::realm::RealmRecord;
    use crate::script::ScriptRecord;
    use crate::value::{StringRef, Value};
    use crate::{Agent, AgentState, Limits, Outcome, Parsed, Seeds, Source, Termination};

    /// The root sites, in the order the mark phase calls them.
    const ROOT_SITES: [&str; 6] = [
        "heap/roots.rs trace_roots",
        "heap/mod.rs trace_registry",
        "host/agent.rs trace_roots",
        "realm/mod.rs trace_roots",
        "object/builtin_function.rs trace_roots",
        "interp/mod.rs trace_roots",
    ];

    /// The trace function of each kind of the layout but the common object record, which is
    /// called once per object.
    const KIND_SITES: [&str; 15] = [
        "heap/string.rs HeapString::trace",
        "heap/mod.rs SymbolRecord::trace",
        "host/host_object.rs HostObject::trace",
        "object/ordinary.rs Ordinary::trace",
        "object/array.rs Array::trace",
        "object/string.rs StringObject::trace",
        "object/arguments.rs ArgumentsObject::trace",
        "object/builtin_function.rs BuiltinFunction::trace",
        "object/bound_function.rs BoundFunction::trace",
        "object/ecmascript_function.rs EcmascriptFunction::trace",
        "object/regexp.rs RegExpObject::trace",
        "realm/mod.rs RealmRecord::trace",
        "env/global.rs RealmGlobalEnv::trace",
        "env/mod.rs EnvironmentRecord::trace",
        "script.rs ScriptRecord::trace",
    ];

    const OBJECT_SITE: &str = "object/mod.rs ObjectRecord::trace";

    /// A counting build of the mark phase over a heap that holds one record of every kind
    /// calls each root site once, in order, each kind trace function once, and the common
    /// object record once per object. Its Array holds a dense element and a sparse one, each the
    /// only path to a String, so both storages are traced (the String site runs three times).
    #[test]
    fn mark_phase_calls_every_site_of_the_layout() {
        let limits = Limits {
            steps_per_entry: 1_000,
            memory_bytes: 1 << 20,
            frame_depth: 100,
            reentry_depth: 10,
            parser_nesting: 100,
            string_length: 1 << 20,
            bigint_bits: 1 << 10,
        };
        let seeds = Seeds {
            hash_key: [1; 16],
            random_seed: [2; 32],
        };
        let mut agent = Agent::new(NoHooks, limits, seeds).unwrap();

        let heap = &mut agent.heap;
        let string = heap.strings.allocate(HeapString::for_test(), 0).unwrap();
        // Reached only through the dense and the sparse element of the Array.
        let dense = heap.strings.allocate(HeapString::for_test(), 0).unwrap();
        let sparse = heap.strings.allocate(HeapString::for_test(), 0).unwrap();
        let symbol = heap.symbols.allocate(SymbolRecord::for_test(), 0).unwrap();
        let kinds = [
            ObjectKind::Ordinary(Ordinary::for_test()),
            ObjectKind::Array(Array::for_test_with(vec![
                None,
                Some(Value::String(StringRef { handle: dense })),
            ])),
            ObjectKind::String(StringObject::for_test(StringRef { handle: string })),
            ObjectKind::Arguments(ArgumentsObject::for_test()),
            ObjectKind::BuiltinFunction(BuiltinFunction::for_test()),
            ObjectKind::BoundFunction(BoundFunction::for_test()),
            ObjectKind::EcmascriptFunction(EcmascriptFunction::for_test()),
            ObjectKind::Host(HostObject::for_test()),
            ObjectKind::RegExp(RegExpObject::for_test(StringRef { handle: string })),
        ];
        let objects: Vec<_> = kinds
            .into_iter()
            .map(|kind| {
                heap.objects
                    .allocate(ObjectRecord::new(None, kind), 0)
                    .unwrap()
            })
            .collect();
        let array = heap.objects.get_mut(objects[1]).unwrap();
        let element = Property::data(
            Value::String(StringRef { handle: sparse }),
            false,
            true,
            true,
        );
        array.properties.push(Key::Index(5_000), element).unwrap();
        let realm = heap.realms.allocate(RealmRecord::for_test(), 0).unwrap();
        let environment = heap
            .environments
            .allocate(EnvironmentRecord::for_test(), 0)
            .unwrap();
        let script = heap.scripts.allocate(ScriptRecord::for_test(), 0).unwrap();

        let mut mark = Mark::take(&mut agent.heap);
        mark.push(Item::String(string));
        mark.push(Item::Symbol(symbol));
        for &object in &objects {
            mark.push(Item::Object(object));
        }
        mark.push(Item::Realm(realm));
        mark.push(Item::Environment(environment));
        mark.push(Item::Script(script));
        // Reported twice, traced once.
        mark.push(Item::String(string));

        mark_phase(&agent, &mut mark).unwrap();
        let calls = mark.calls().to_vec();

        assert_eq!(
            &calls[..ROOT_SITES.len()],
            &ROOT_SITES[..],
            "the root sites, in order"
        );
        let count = |site: &str| calls.iter().filter(|call| **call == site).count();
        let strings = "heap/string.rs HeapString::trace";
        for site in ROOT_SITES.iter().chain(&KIND_SITES) {
            let once = if *site == strings { 3 } else { 1 };
            assert_eq!(count(site), once, "{site}: {calls:#?}");
        }
        assert_eq!(count(OBJECT_SITE), objects.len());
        assert_eq!(
            calls.len(),
            ROOT_SITES.len() + KIND_SITES.len() + 2 + objects.len(),
            "{calls:#?}"
        );

        assert!(mark.is_marked(Item::String(string)));
        assert!(mark.is_marked(Item::String(dense)), "the dense element");
        assert!(mark.is_marked(Item::String(sparse)), "the sparse element");
        assert!(mark.is_marked(Item::Symbol(symbol)));
        for &object in &objects {
            assert!(mark.is_marked(Item::Object(object)));
        }
        assert!(mark.is_marked(Item::Realm(realm)));
        assert!(mark.is_marked(Item::Environment(environment)));
        assert!(mark.is_marked(Item::Script(script)));
    }

    /// An ordinary object record with no prototype.
    fn ordinary() -> ObjectRecord {
        ObjectRecord::new(None, ObjectKind::Ordinary(Ordinary::for_test()))
    }

    /// Evaluate `source` in `realm`; it must complete normally.
    fn evaluate(agent: &mut Agent<NoHooks>, realm: &crate::Realm, source: &str) {
        let Ok(Parsed::Parsed(script)) = agent.parse_script(realm, Source::Utf8(source), None)
        else {
            panic!("{source} did not parse");
        };
        assert!(
            matches!(agent.evaluate_script(&script), Ok(Outcome::Normal(_))),
            "{source}"
        );
    }

    /// The counting build counts, during a collection, every vector the collector makes and
    /// every push past capacity onto the mark words, the worklists and the free lists of the
    /// sweep (the atom table's sweep pushes nothing): 0, for two collections of a heap whose
    /// scripts left garbage of objects, Strings, environments and atoms.
    #[test]
    fn a_collection_allocates_nothing() {
        let (mut agent, realm) = test_host::agent_with_realm();
        evaluate(
            &mut agent,
            &realm,
            "var kept = { s: 'a' + 1, a: [1, 2, 3] }; \
             for (var i = 0; i < 20; i++) { \
               var o = (function (n) { var p = { s: 'x' + n }; p['k' + n] = [n]; return p; })(i); \
             } 1",
        );
        for round in 0..2 {
            assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
            assert_eq!(
                agent.heap.collect_allocations, 0,
                "collection {round}: vectors made or grown"
            );
        }
    }

    /// An Array of 100,000 elements that all name one object: the worklists hold at most one
    /// entry per slot at any time, so their peak is at most the number of slots.
    #[test]
    fn the_worklist_holds_at_most_one_entry_per_slot() {
        let mut limits = test_host::limits();
        limits.memory_bytes = 1 << 26;
        let mut agent = test_host::agent_with(limits);
        let target = agent.heap.new_object(ordinary(), 0).unwrap();
        let elements = vec![Some(Value::Object(target)); 100_000];
        let array = ObjectRecord::new(None, ObjectKind::Array(Array::for_test_with(elements)));
        let array = agent.heap.new_object(array, 0).unwrap();
        let _root = agent.heap.roots.add(Value::Object(array));
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        let slots = agent.heap.slot_total();
        let peak = agent.heap.collect_peak_work;
        assert!(
            peak <= slots,
            "the worklists held {peak} entries, {slots} slots"
        );
        assert!(agent.heap.objects.get(target.handle).is_ok());
    }

    /// A guard of the marking: a handle of a freed record whose slot holds a newer record that
    /// is already marked ends the collection in `Terminated(EngineFault)`, and nothing is swept.
    #[test]
    fn a_stale_handle_reported_for_a_marked_slot_ends_the_collection() {
        let mut agent = test_host::agent_with(test_host::limits());
        let old = agent.heap.new_object(ordinary(), 0).unwrap();
        assert!(agent.heap.objects.free(old.handle).is_some());
        let new = agent.heap.new_object(ordinary(), 0).unwrap();
        assert_eq!(new.handle.index, old.handle.index);
        let garbage = agent.heap.new_object(ordinary(), 0).unwrap();
        // The root table is traced first and marks the new record; the scope then reports
        // the old handle of the same slot.
        let _root = agent.heap.roots.add(Value::Object(new));
        agent.hand_out(Value::Object(old));
        assert!(matches!(
            agent.collect(),
            Ok(Outcome::Terminated(Termination::EngineFault))
        ));
        assert_eq!(agent.state(), AgentState::Faulted);
        assert!(agent.heap.objects.get(new.handle).is_ok());
        assert!(
            agent.heap.objects.get(garbage.handle).is_ok(),
            "nothing was swept"
        );
    }

    /// A guard of the workspace: with `memory_bytes` 1 MiB, objects are made until the heap
    /// refuses one (`OverBudget`, which every caller of the engine ends as
    /// `Terminated(MemoryBudget)`), every other one kept by a root; `collect()` then answers
    /// `Normal` and `heap_bytes()` reads at most 1 MiB.
    #[test]
    fn a_collection_at_the_memory_limit_finishes() {
        let mut limits = test_host::limits();
        limits.memory_bytes = 1 << 20;
        let mut agent = test_host::agent_with(limits);
        let mut roots = Vec::new();
        let mut made = 0usize;
        while let Ok(object) = agent.heap.new_object(ordinary(), 0) {
            if made.is_multiple_of(2) {
                roots.push(agent.heap.roots.add(Value::Object(object)));
            }
            made += 1;
        }
        assert!(made > 0);
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        let bytes = agent.heap_bytes().unwrap();
        assert!(bytes <= 1 << 20, "{bytes} after the collection");
        assert_eq!(roots.len(), made.div_ceil(2));
    }

    /// The lane `slow` sets `SWELL_FULL_COUNTS=1` for the full counts of a test.
    fn full_counts() -> bool {
        // HOST-2 (source_scan.rs): no run-time environment read in engine sources, tests included.
        matches!(option_env!("SWELL_FULL_COUNTS"), Some("1"))
    }

    /// The collection threshold follows the live reading, taken after every credit of the
    /// collection. Ordinary object records stored straight into the heap and kept by nothing
    /// leave, after two collections, their slots charged with the objects arena's capacity:
    /// the charged reading U is more than max(L, 1 MiB) + 8 MiB above the live reading L. With
    /// the budget M = U + max(L, 1 MiB) + 8 MiB, a script that makes garbage Strings of 2^19
    /// units, at most two of them live at once, past the room M - U answers `Normal(1)`: a
    /// safe point collects once max(L, 1 MiB) bytes were charged since the last collection,
    /// well inside that room. A threshold of the charged reading, max(U, 1 MiB), is larger
    /// than the room and could only fire after the budget refused a charge. Each garbage
    /// String is a 1,024-unit String repeated 512 times: `"x".repeat(1 << 19)` makes the same
    /// String by 2^19 appends of one unit, and 128 of those ran past the 60 s a test is given.
    ///
    /// The lane `slow` (`SWELL_FULL_COUNTS=1`) stores the story's 262,144 records, the gate
    /// 65,536, whose slots charge more than the room; both make as many Strings as fill the
    /// room M - U and two more: past the room's end is the first collection the live reading
    /// makes and the charged one would put after the budget's refusal. The lane made the
    /// story's 128 Strings, 64 MiB, before, and took 2.6 CPU seconds alone (measured on
    /// 2026-11-22). With the feature `collect-every-safe-point` every safe point collects,
    /// so there the test passes whichever reading the threshold takes: it stores the gate's
    /// 65,536 records and runs the loop twice, in the gate and in the lane `slow` alike, each
    /// collection sweeping every slot (with 262,144 records and 4 loops the lane took 3.8 CPU
    /// seconds alone, measured on 2026-11-22).
    #[test]
    fn the_collection_threshold_follows_the_live_reading() {
        let full = full_counts() && !cfg!(feature = "collect-every-safe-point");
        let mut limits = test_host::limits();
        limits.memory_bytes = 1 << 30;
        limits.steps_per_entry = u64::MAX;
        let (mut agent, realm) = test_host::agent_with_realm_and(limits);
        let records = if full { 262_144 } else { 65_536 };
        for _ in 0..records {
            agent.heap.new_object(ordinary(), 0).unwrap();
        }
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        let charged = agent.heap_bytes().unwrap();
        let live = agent.live_bytes().unwrap();
        let room = live.max(1 << 20) + (8 << 20);
        agent.heap.accounting.set_limit(charged + room);
        assert!(
            charged > room,
            "heap_bytes {charged}, live_bytes {live}: the empty slots are not charged"
        );
        let rounds = if cfg!(feature = "collect-every-safe-point") {
            2
        } else {
            room.div_ceil(1 << 19) + 2
        };
        let source = format!(
            "var s = \"x\".repeat(1 << 10); \
             for (var i = 0; i < {rounds}; i++) {{ var t = s.repeat(1 << 9); }} 1"
        );
        let Ok(Parsed::Parsed(script)) = agent.parse_script(&realm, Source::Utf8(&source), None)
        else {
            panic!("the script did not parse");
        };
        let outcome = agent.evaluate_script(&script).unwrap();
        let Outcome::Normal(root) = outcome else {
            panic!("the script answered {outcome:?}");
        };
        assert_eq!(agent.heap.roots.get(&root), Ok(Value::Number(1.0)));
    }
}

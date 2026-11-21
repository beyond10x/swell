//! The collector (`docs/design/engine.md`, decision 4, "Collector"; `docs/design/host-interface.md`,
//! decision 18): precise, stop-the-world mark and sweep over the arenas. It runs only when the
//! host asks (`collect`) and at safe points, when the bytes allocated since the last collection
//! pass a threshold, or at every safe point with the feature `collect-every-safe-point`
//! (decisions 4 and 13). It runs no ECMAScript code and calls no hook (HOST-129).
//!
//! The mark phase is a loop over an explicit worklist, never a Rust recursion, fed by two kinds
//! of sites, each defined once in the file of what it traces and called from here in a fixed
//! order:
//!
//! - a root site per module that owns roots, `fn trace_roots(agent, mark: &mut Mark)`: the root
//!   table (`heap/roots.rs`), the handles of the running `with_cx`, host function and hook
//!   scopes (`host/agent.rs`), the agent-wide values (`realm/mod.rs`), the slots of running
//!   builtins (`object/builtin_function.rs`) and the frame stack (`interp/mod.rs`);
//! - a trace function per kind of record, `fn trace(&self, mark: &mut Mark)`: strings
//!   (`heap/string.rs`), symbols (`heap/mod.rs`), objects (`object/mod.rs`, which calls the trace
//!   of the object's kind in `object/*.rs` and `host/host_object.rs`), realm records (`realm/mod.rs`, with the global
//!   environment of `env/global.rs`), environment records (`env/mod.rs`) and script records
//!   (`script.rs`).
//!
//! A site whose owner comes in a later story reports nothing, since no such record can exist
//! before it. A handle a site reports that is stale or of another agent ends the collection in
//! `Terminated(EngineFault)`, before anything is swept. The sweep then frees every record not
//! marked, in slot order, drops what it owns (host data included) and credits its bytes.

use super::Heap;
use super::arena::{Access, Handle};
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

/// The mark phase's worklist: what the sites report. It is the [`crate::Tracer`] host data
/// reports through.
pub(crate) struct Mark {
    work: Vec<Item>,
    /// The sites called, in order, in the counting build of the tests.
    #[cfg(test)]
    calls: Vec<&'static str>,
}

impl Mark {
    pub(crate) fn new() -> Mark {
        Mark {
            work: Vec::new(),
            #[cfg(test)]
            calls: Vec::new(),
        }
    }

    /// Report `item`.
    pub(crate) fn push(&mut self, item: Item) {
        self.work.push(item);
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

    fn pop(&mut self) -> Option<Item> {
        self.work.pop()
    }
}

/// A site reported a stale handle, or one of another agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stale;

/// The mark bits of every arena, one per slot.
pub(crate) struct Marks {
    strings: Vec<bool>,
    symbols: Vec<bool>,
    objects: Vec<bool>,
    realms: Vec<bool>,
    environments: Vec<bool>,
    scripts: Vec<bool>,
}

impl Marks {
    fn new(heap: &Heap) -> Marks {
        Marks {
            strings: vec![false; heap.strings.slot_count()],
            symbols: vec![false; heap.symbols.slot_count()],
            objects: vec![false; heap.objects.slot_count()],
            realms: vec![false; heap.realms.slot_count()],
            environments: vec![false; heap.environments.slot_count()],
            scripts: vec![false; heap.scripts.slot_count()],
        }
    }

    fn bits(&self, item: Item) -> Option<(&Vec<bool>, Handle)> {
        match item {
            Item::String(handle) => Some((&self.strings, handle)),
            Item::Symbol(handle) => Some((&self.symbols, handle)),
            Item::Object(handle) => Some((&self.objects, handle)),
            Item::Realm(handle) => Some((&self.realms, handle)),
            Item::Environment(handle) => Some((&self.environments, handle)),
            Item::Script(handle) => Some((&self.scripts, handle)),
            Item::NoArena => None,
        }
    }

    /// Mark `item`; answers whether it was not marked before.
    fn set(&mut self, item: Item) -> bool {
        let (bits, handle) = match item {
            Item::String(handle) => (&mut self.strings, handle),
            Item::Symbol(handle) => (&mut self.symbols, handle),
            Item::Object(handle) => (&mut self.objects, handle),
            Item::Realm(handle) => (&mut self.realms, handle),
            Item::Environment(handle) => (&mut self.environments, handle),
            Item::Script(handle) => (&mut self.scripts, handle),
            Item::NoArena => return false,
        };
        match usize::try_from(handle.index)
            .ok()
            .and_then(|i| bits.get_mut(i))
        {
            Some(bit) if !*bit => {
                *bit = true;
                true
            }
            _ => false,
        }
    }

    /// Whether `item` is marked.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "read by the tests of the mark phase")
    )]
    pub(crate) fn is_marked(&self, item: Item) -> bool {
        self.bits(item).is_some_and(|(bits, handle)| {
            usize::try_from(handle.index)
                .ok()
                .and_then(|i| bits.get(i))
                .copied()
                .unwrap_or(false)
        })
    }
}

/// The mark phase: the root sites in their fixed order, then the worklist until it is empty,
/// each record traced once by the trace function of its kind.
pub(crate) fn mark_phase<H: Host>(agent: &Agent<H>, mark: &mut Mark) -> Result<Marks, Stale> {
    super::roots::trace_roots(agent, mark);
    crate::host::agent::trace_roots(agent, mark);
    crate::realm::trace_roots(agent, mark);
    crate::object::builtin_function::trace_roots(agent, mark);
    crate::interp::trace_roots(agent, mark);

    let heap = &agent.heap;
    let mut marks = Marks::new(heap);
    let stale = |_: Access| Stale;
    while let Some(item) = mark.pop() {
        match item {
            Item::String(handle) => {
                let string = heap.strings.get(handle).map_err(stale)?;
                if marks.set(item) {
                    string.trace(mark);
                }
            }
            Item::Symbol(handle) => {
                let symbol = heap.symbols.get(handle).map_err(stale)?;
                if marks.set(item) {
                    symbol.trace(mark);
                }
            }
            Item::Object(handle) => {
                let object = heap.objects.get(handle).map_err(stale)?;
                if marks.set(item) {
                    object.trace(mark);
                }
            }
            Item::Realm(handle) => {
                let realm = heap.realms.get(handle).map_err(stale)?;
                if marks.set(item) {
                    realm.trace(mark);
                }
            }
            Item::Environment(handle) => {
                let environment = heap.environments.get(handle).map_err(stale)?;
                if marks.set(item) {
                    environment.trace(mark);
                }
            }
            Item::Script(handle) => {
                let script = heap.scripts.get(handle).map_err(stale)?;
                if marks.set(item) {
                    script.trace(mark);
                }
            }
            Item::NoArena => return Err(Stale),
        }
    }
    Ok(marks)
}

/// A full collection: release the dropped roots, mark, and sweep every arena and the atom
/// table. On a stale handle nothing is swept and the caller faults the agent.
pub(crate) fn collect<H: Host>(agent: &mut Agent<H>) -> Result<CollectReport, Stale> {
    agent.heap.roots.drain();
    let heap_bytes_before = agent.heap.bytes();
    let mut mark = Mark::new();
    let marks = mark_phase(agent, &mut mark)?;

    let heap = &mut agent.heap;
    let mut credited = heap.objects.sweep(&marks.objects);
    credited = credited.saturating_add(heap.strings.sweep(&marks.strings));
    credited = credited.saturating_add(heap.symbols.sweep(&marks.symbols));
    credited = credited.saturating_add(heap.realms.sweep(&marks.realms));
    credited = credited.saturating_add(heap.environments.sweep(&marks.environments));
    credited = credited.saturating_add(heap.scripts.sweep(&marks.scripts));
    credited = credited.saturating_add(heap.atoms.sweep(&heap.strings));
    heap.accounting.credit(credited);
    heap.collected();
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
    use crate::host::hooks::test_host::NoHooks;
    use crate::host::host_object::HostObject;
    use crate::object::arguments::ArgumentsObject;
    use crate::object::array::Array;
    use crate::object::bound_function::BoundFunction;
    use crate::object::builtin_function::BuiltinFunction;
    use crate::object::ecmascript_function::EcmascriptFunction;
    use crate::object::ordinary::Ordinary;
    use crate::object::string::StringObject;
    use crate::object::{ObjectKind, ObjectRecord};
    use crate::realm::RealmRecord;
    use crate::script::ScriptRecord;
    use crate::value::StringRef;
    use crate::{Agent, Limits, Seeds};

    /// The root sites, in the order the mark phase calls them.
    const ROOT_SITES: [&str; 5] = [
        "heap/roots.rs trace_roots",
        "host/agent.rs trace_roots",
        "realm/mod.rs trace_roots",
        "object/builtin_function.rs trace_roots",
        "interp/mod.rs trace_roots",
    ];

    /// The trace function of each kind of the layout but the common object record, which is
    /// called once per object.
    const KIND_SITES: [&str; 14] = [
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
        "realm/mod.rs RealmRecord::trace",
        "env/global.rs RealmGlobalEnv::trace",
        "env/mod.rs EnvironmentRecord::trace",
        "script.rs ScriptRecord::trace",
    ];

    const OBJECT_SITE: &str = "object/mod.rs ObjectRecord::trace";

    /// A counting build of the mark phase over a heap that holds one record of every kind
    /// calls each root site once, in order, each kind trace function once, and the common
    /// object record once per object.
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
        let symbol = heap.symbols.allocate(SymbolRecord::for_test(), 0).unwrap();
        let kinds = [
            ObjectKind::Ordinary(Ordinary::for_test()),
            ObjectKind::Array(Array::for_test()),
            ObjectKind::String(StringObject::for_test(StringRef { handle: string })),
            ObjectKind::Arguments(ArgumentsObject::for_test()),
            ObjectKind::BuiltinFunction(BuiltinFunction::for_test()),
            ObjectKind::BoundFunction(BoundFunction::for_test()),
            ObjectKind::EcmascriptFunction(EcmascriptFunction::for_test()),
            ObjectKind::Host(HostObject::for_test()),
        ];
        let objects: Vec<_> = kinds
            .into_iter()
            .map(|kind| {
                heap.objects
                    .allocate(ObjectRecord::new(None, kind), 0)
                    .unwrap()
            })
            .collect();
        let realm = heap.realms.allocate(RealmRecord::for_test(), 0).unwrap();
        let environment = heap
            .environments
            .allocate(EnvironmentRecord::for_test(), 0)
            .unwrap();
        let script = heap.scripts.allocate(ScriptRecord::for_test(), 0).unwrap();

        let mut mark = Mark::new();
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

        let marks = mark_phase(&agent, &mut mark).unwrap();
        let calls = mark.calls().to_vec();

        assert_eq!(
            &calls[..ROOT_SITES.len()],
            &ROOT_SITES[..],
            "the root sites, in order"
        );
        let count = |site: &str| calls.iter().filter(|call| **call == site).count();
        for site in ROOT_SITES.iter().chain(&KIND_SITES) {
            assert_eq!(count(site), 1, "{site}: {calls:#?}");
        }
        assert_eq!(count(OBJECT_SITE), objects.len());
        assert_eq!(
            calls.len(),
            ROOT_SITES.len() + KIND_SITES.len() + objects.len(),
            "{calls:#?}"
        );

        assert!(marks.is_marked(Item::String(string)));
        assert!(marks.is_marked(Item::Symbol(symbol)));
        for &object in &objects {
            assert!(marks.is_marked(Item::Object(object)));
        }
        assert!(marks.is_marked(Item::Realm(realm)));
        assert!(marks.is_marked(Item::Environment(environment)));
        assert!(marks.is_marked(Item::Script(script)));
    }
}

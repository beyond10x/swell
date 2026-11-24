//! String exotic objects (ECMA-262 10.4.3, `#sec-string-exotic-objects`): [[StringData]],
//! StringCreate, StringGetOwnProperty and the three internal methods that differ from the
//! ordinary ones, and the reads of a String's code units the engine uses. Written by
//! `story:object-model-and-realms`.

use super::property::{Descriptor, Key, Property};
use super::{KeyList, ObjectKind};
use crate::heap::accounting::ChargedVec;
use crate::heap::collect::Mark;
use crate::heap::string::HeapString;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{Host, Termination};
use crate::value::{ObjectRef, StringRef, Value};

/// A copy of a String's code units, charged by its capacity while it lives.
pub(crate) type Units = ChargedVec<u16>;

/// The slots of a String object.
pub(crate) struct StringObject {
    /// [[StringData]].
    pub(crate) data: StringRef,
}

impl StringObject {
    /// Kind site of the collector: [[StringData]].
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/string.rs StringObject::trace");
        mark.value(Value::String(self.data));
    }

    /// A String object of the empty String, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test(data: StringRef) -> StringObject {
        StringObject { data }
    }
}

impl<H: Host> Agent<H> {
    /// The record of `string`; a stale one inside the engine faults the agent.
    fn engine_string(&mut self, string: StringRef) -> Completion<&HeapString> {
        if self.heap.strings.get(string.handle).is_err() {
            return Err(self.fault());
        }
        self.heap
            .strings
            .get(string.handle)
            .map_err(|_| crate::host::Abrupt::Terminated(Termination::EngineFault))
    }

    /// The code units of `string`, for the test lines that read them; no charge.
    #[cfg(test)]
    pub(crate) fn string_units_for_tests(&mut self, string: StringRef) -> Completion<Vec<u16>> {
        let units = self.engine_string(string)?.to_utf16();
        copies::note(units.len());
        Ok(units)
    }

    /// The code units of `string`, a step per code unit charged before they are copied
    /// (`docs/design/engine.md`, "The step budget"), then their bytes held charged to the
    /// memory budget while the copy lives ([`Agent::string_units_in`]).
    pub(crate) fn charged_units(&mut self, string: StringRef) -> Completion<Units> {
        let length = self.string_length(string)?;
        self.charge_steps(u64::try_from(length).unwrap_or(u64::MAX))?;
        self.string_units_in(string, 0, length)
    }

    /// The code units of `string` from `from` to `to`, with no step: for a caller that charges
    /// them as it appends them. Their bytes are held charged to the memory budget before the
    /// copy is made and while it lives (`docs/design/engine.md`, "Memory budget"); over the
    /// budget the run ends in `Terminated(MemoryBudget)`. A range that is not inside the String
    /// is a broken invariant.
    pub(crate) fn string_units_in(
        &mut self,
        string: StringRef,
        from: usize,
        to: usize,
    ) -> Completion<Units> {
        if from > to || to > self.string_length(string)? {
            return Err(self.fault());
        }
        let mut units = self.new_charged_vec(to - from)?;
        self.extend_units(&mut units, string, from, to)?;
        #[cfg(test)]
        copies::note(units.len());
        Ok(units)
    }

    /// Append the code units of `string` from `from` to `to` to `units`, in room reserved
    /// before ([`Agent::reserve_vec`]), with no step. A range that is not inside the String, or
    /// no room, is a broken invariant.
    pub(crate) fn extend_units(
        &mut self,
        units: &mut Units,
        string: StringRef,
        from: usize,
        to: usize,
    ) -> Completion<()> {
        let appended = match self.heap.strings.get(string.handle) {
            Ok(HeapString::Latin1(bytes)) => bytes.get(from..to).is_some_and(|bytes| {
                units
                    .extend_counted(bytes.len(), bytes.iter().map(|&byte| u16::from(byte)))
                    .is_ok()
            }),
            Ok(HeapString::Utf16(own)) => own
                .get(from..to)
                .is_some_and(|own| units.extend_from_slice(own).is_ok()),
            Err(_) => false,
        };
        if !appended {
            return Err(self.fault());
        }
        Ok(())
    }

    /// The length of `string` in code units.
    pub(crate) fn string_length(&mut self, string: StringRef) -> Completion<usize> {
        Ok(match self.engine_string(string)? {
            HeapString::Latin1(bytes) => bytes.len(),
            HeapString::Utf16(units) => units.len(),
        })
    }

    /// The code unit of `string` at `index`, if there is one.
    pub(crate) fn code_unit_at(
        &mut self,
        string: StringRef,
        index: usize,
    ) -> Completion<Option<u16>> {
        Ok(match self.engine_string(string)? {
            HeapString::Latin1(bytes) => bytes.get(index).map(|&b| u16::from(b)),
            HeapString::Utf16(units) => units.get(index).copied(),
        })
    }

    /// A new String of `units`; over the budget the run ends in `Terminated(MemoryBudget)`.
    pub(crate) fn new_string_value(&mut self, units: &[u16]) -> Completion<StringRef> {
        match self.heap.new_string_from_units(units) {
            Ok(string) => Ok(string),
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    /// A new String of `text`; over the budget the run ends in `Terminated(MemoryBudget)`.
    pub(crate) fn new_string_str(&mut self, text: &str) -> Completion<StringRef> {
        match self.heap.new_string_from_str(text) {
            Ok(string) => Ok(string),
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    /// StringCreate (`#sec-stringcreate`).
    pub(crate) fn string_create(
        &mut self,
        value: StringRef,
        prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        let length = self.string_length(value)?;
        let object = self.allocate_object(
            Some(prototype),
            ObjectKind::String(StringObject { data: value }),
        )?;
        let key = self.name_key("length")?;
        // A length is below 2^53, so the conversion is exact.
        let length = length as f64;
        self.add_property(
            object,
            key,
            Property::data(Value::Number(length), false, false, false),
        )?;
        Ok(object)
    }

    /// [[StringData]] of a String object.
    fn string_data(&mut self, object: ObjectRef) -> Completion<StringRef> {
        match &self.engine_object(object)?.kind {
            ObjectKind::String(string) => Ok(string.data),
            _ => Err(self.fault()),
        }
    }

    /// StringGetOwnProperty (`#sec-stringgetownproperty`). Only an array index can name a code
    /// unit: CanonicalNumericIndexString answers an integral, non-negative value below the
    /// length for nothing else.
    fn string_get_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
    ) -> Completion<Option<Property>> {
        let Key::Index(index) = key else {
            return Ok(None);
        };
        let data = self.string_data(object)?;
        let Ok(index) = usize::try_from(index) else {
            return Ok(None);
        };
        let Some(unit) = self.code_unit_at(data, index)? else {
            return Ok(None);
        };
        let result = self.new_string_value(&[unit])?;
        Ok(Some(Property::data(
            Value::String(result),
            false,
            true,
            false,
        )))
    }

    /// [[GetOwnProperty]] of a String object
    /// (`#sec-string-exotic-objects-getownproperty-p`).
    pub(crate) fn string_get_own_property_method(
        &mut self,
        object: ObjectRef,
        key: Key,
    ) -> Completion<Option<Property>> {
        if let Some(property) = self.ordinary_get_own_property(object, key)? {
            return Ok(Some(property));
        }
        self.string_get_own_property(object, key)
    }

    /// [[DefineOwnProperty]] of a String object
    /// (`#sec-string-exotic-objects-defineownproperty-p-desc`).
    pub(crate) fn string_define_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        if let Some(current) = self.string_get_own_property(object, key)? {
            let extensible = self.is_extensible(object)?;
            // IsCompatiblePropertyDescriptor (`#sec-iscompatiblepropertydescriptor`).
            return self.validate_and_apply_property_descriptor(
                None,
                key,
                extensible,
                descriptor,
                Some(current),
            );
        }
        self.ordinary_define_own_property(object, key, descriptor)
    }

    /// [[OwnPropertyKeys]] of a String object
    /// (`#sec-string-exotic-objects-ownpropertykeys`): a step per key, counted from the
    /// String's length and the length of the property list before anything is allocated (the
    /// list holds no index below the length, which [[DefineOwnProperty]] answers from the
    /// String), then the list's bytes before it is reserved; index keys of the list not added
    /// ascending are sorted, n steps before each pass (`object/mod.rs`).
    pub(crate) fn string_own_property_keys(&mut self, object: ObjectRef) -> Completion<KeyList> {
        let data = self.string_data(object)?;
        let length = self.string_length(data)?;
        let listed = self.engine_object(object)?.properties.len();
        let count = length.saturating_add(listed);
        self.charge_steps(u64::try_from(count).unwrap_or(u64::MAX))?;
        // 5. For each integer i such that 0 ≤ i < length, in ascending order, append
        // ! ToString(𝔽(i)); 6 to 8, the keys of the list, its array indices from length on.
        let from = u32::try_from(length).unwrap_or(u32::MAX);
        self.own_key_list(count, |agent, keys| {
            keys.extend((0..from).map(Key::Index));
            agent.push_own_keys(object, from, keys)
        })
    }
}

/// The counting build: the longest copy [`Agent::string_units_in`] (or [`Agent::charged_units`],
/// `Agent::string_units_for_tests`) made on this thread since the last reset, a high-water mark
/// in code units. Each test runs in a process of its own under
/// cargo-nextest and resets it first; nothing else reads it.
///
/// It also keeps two tallies of the buffers held as a `ChargedVec` (and of the microtask
/// queue's charge): their capacity in bytes and their charged bytes, each added on a growth
/// and taken off on a drop. At every entry into script ([`entered`], called by the two
/// re-entry checks) it records the most capacity not covered by a charge. A buffer that is not
/// a `ChargedVec` is not seen.
#[cfg(test)]
pub(crate) mod copies {
    use std::cell::Cell;

    thread_local! {
        static LONGEST: Cell<usize> = const { Cell::new(0) };
        static CAPACITY: Cell<u64> = const { Cell::new(0) };
        static CHARGED: Cell<u64> = const { Cell::new(0) };
        static UNCOVERED: Cell<u64> = const { Cell::new(0) };
    }

    /// Count a copy of `units` code units.
    pub(crate) fn note(units: usize) {
        LONGEST.with(|longest| longest.set(longest.get().max(units)));
    }

    /// The longest copy since the last reset.
    pub(crate) fn longest() -> usize {
        LONGEST.with(Cell::get)
    }

    /// Start counting again from 0: the longest copy and the most uncovered at an entry.
    pub(crate) fn reset() {
        LONGEST.with(|longest| longest.set(0));
        UNCOVERED.with(|uncovered| uncovered.set(0));
    }

    /// An entry into script: record the capacity of the live buffers not covered by their
    /// charges, if it is the most so far.
    pub(crate) fn entered() {
        let capacity = CAPACITY.with(Cell::get);
        let charged = CHARGED.with(Cell::get);
        let uncovered = capacity.saturating_sub(charged);
        UNCOVERED.with(|most| most.set(most.get().max(uncovered)));
    }

    /// The most capacity not covered by a charge at any entry into script since the last
    /// reset, in bytes.
    pub(crate) fn uncovered_at_entry() -> u64 {
        UNCOVERED.with(Cell::get)
    }

    /// What one buffer adds to the two tallies: its capacity in bytes and its charged bytes,
    /// taken off when it drops.
    #[derive(Default)]
    pub(crate) struct Tally {
        capacity: u64,
        charged: u64,
    }

    impl Tally {
        /// The buffer now has `capacity` bytes of room and `charged` bytes charged.
        pub(crate) fn set(&mut self, capacity: u64, charged: u64) {
            CAPACITY.with(|total| {
                total.set(
                    total
                        .get()
                        .saturating_add(capacity)
                        .saturating_sub(self.capacity),
                );
            });
            CHARGED.with(|total| {
                total.set(
                    total
                        .get()
                        .saturating_add(charged)
                        .saturating_sub(self.charged),
                );
            });
            self.capacity = capacity;
            self.charged = charged;
        }
    }

    impl Drop for Tally {
        fn drop(&mut self) {
            self.set(0, 0);
        }
    }
}

/// The agent the tests of `story:copies-held-across-a-call-into-script-are-charged` share: the
/// story's limits, Strings and host functions the host makes, and the readings each test
/// asserts (`used()`, the most capacity not covered by a charge at an entry into script, the
/// steps of the last entry).
#[cfg(test)]
pub(crate) mod across {
    use super::copies;
    use crate::builtins::string::transform::tests::Answer;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm_and, limits};
    use crate::host::{Cx, HostFn};
    use crate::{Limits, Outcome, Parsed, PropertyKey, Realm, Root, Source, Value};

    /// The limits of the story's tests: 64 MiB, 1,000,000 steps, `reentry_depth` 256,
    /// `frame_depth` 10,000, `string_length` 2^24.
    pub(crate) fn story_limits() -> Limits {
        let mut limits = limits();
        limits.memory_bytes = 1 << 26;
        limits.steps_per_entry = 1_000_000;
        limits.reentry_depth = 256;
        limits.frame_depth = 10_000;
        limits.string_length = 1 << 24;
        limits
    }

    /// An agent and its realm.
    pub(crate) struct Across {
        pub(crate) agent: Agent<NoHooks>,
        pub(crate) realm: Realm,
    }

    impl Across {
        /// An agent with `limits`.
        pub(crate) fn new(limits: Limits) -> Across {
            let (agent, realm) = agent_with_realm_and(limits);
            Across { agent, realm }
        }

        /// An agent with [`story_limits`] and `steps` for each entry.
        pub(crate) fn with_steps(steps: u64) -> Across {
            let mut limits = story_limits();
            limits.steps_per_entry = steps;
            Across::new(limits)
        }

        /// Set the global `name` to `value`, made in a `Cx` of the realm.
        fn global(&mut self, name: &str, value: impl FnOnce(&mut Cx<'_, NoHooks>) -> Value) {
            let global = self.agent.global_object(&self.realm).unwrap();
            self.agent
                .with_cx(&self.realm, |cx| {
                    let made = value(cx);
                    let Value::Object(global) = cx.get(&global).unwrap() else {
                        panic!("the global object is no object");
                    };
                    let key = PropertyKey::String(cx.new_string(name).unwrap());
                    let set = cx.set(global, key, made, true).unwrap();
                    assert_eq!(set, Ok(true), "{name}");
                })
                .unwrap();
        }

        /// The global `name`, a String of `units` the host makes with `new_string_utf16`.
        pub(crate) fn string(&mut self, name: &str, units: &[u16]) {
            self.global(name, |cx| {
                Value::String(cx.new_string_utf16(units).unwrap())
            });
        }

        /// The global `name`, a host function of `steps`, a constructor when `constructor`.
        pub(crate) fn function(&mut self, name: &str, steps: HostFn<NoHooks>, constructor: bool) {
            let realm = &self.realm;
            let global = self.agent.global_object(realm).unwrap();
            self.agent
                .with_cx(realm, |cx| {
                    let realm = cx.realm_ref(realm).unwrap();
                    let function = cx
                        .new_host_function(realm, name, 0, steps, constructor, None)
                        .unwrap()
                        .unwrap();
                    let Value::Object(global) = cx.get(&global).unwrap() else {
                        panic!("the global object is no object");
                    };
                    let key = PropertyKey::String(cx.new_string(name).unwrap());
                    let set = cx.set(global, key, Value::Object(function), true).unwrap();
                    assert_eq!(set, Ok(true), "{name}");
                })
                .unwrap();
        }

        /// `source`, parsed.
        pub(crate) fn parse(&mut self, source: &str) -> crate::Script {
            match self
                .agent
                .parse_script(&self.realm, Source::Utf8(source), None)
            {
                Ok(Parsed::Parsed(script)) => script,
                other => panic!("{source} did not parse: {other:?}"),
            }
        }

        /// The answer of `script`, with the readings of `copies` reset before it.
        pub(crate) fn evaluate(&mut self, script: &crate::Script) -> Answer {
            copies::reset();
            let outcome = self.agent.evaluate_script(script).unwrap();
            self.describe(outcome)
        }

        /// The answer of `source`, parsed before the readings of `copies` are reset.
        pub(crate) fn run(&mut self, source: &str) -> Answer {
            let script = self.parse(source);
            self.evaluate(&script)
        }

        /// The bytes charged now.
        pub(crate) fn used(&self) -> u64 {
            self.agent.heap.accounting.used()
        }

        /// The budget set to leave `room` bytes free now.
        pub(crate) fn room(&mut self, room: u64) {
            let used = self.used();
            self.agent.heap.accounting.set_limit(used + room);
        }

        /// The steps of the last entry.
        pub(crate) fn steps(&self) -> u64 {
            self.agent.last_entry_steps().unwrap()
        }

        /// The answer of `outcome`.
        fn describe(&mut self, outcome: Outcome<Root>) -> Answer {
            let (root, thrown) = match outcome {
                Outcome::Normal(root) => (root, false),
                Outcome::Throw(root) => (root, true),
                Outcome::Terminated(termination) => return Answer::Terminated(termination),
            };
            self.agent
                .with_cx(&self.realm, |cx| {
                    let mut value = cx.get(&root).unwrap();
                    if let (true, Value::Object(error)) = (thrown, value) {
                        let key = PropertyKey::String(cx.new_string("name").unwrap());
                        value = cx.get_property(error, key).unwrap().unwrap();
                    }
                    let text = match value {
                        Value::String(string) => cx.to_rust_string_lossy(string).unwrap(),
                        other => format!("{other:?}"),
                    };
                    if thrown {
                        Answer::Throw(text)
                    } else {
                        Answer::Normal(text)
                    }
                })
                .unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Realm;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm_and, enter, limits};
    use crate::host::outcome::{Abrupt, Termination};
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Value};

    /// The code units of the long String.
    const UNITS: usize = 1_000_000;

    /// An agent with a budget of 256 MiB and a String object of [`UNITS`] Latin-1 code units,
    /// both kept.
    fn long_string_object() -> (Agent<NoHooks>, Realm, ObjectRef) {
        let mut limits = limits();
        limits.memory_bytes = 1 << 28;
        let (mut agent, realm) = agent_with_realm_and(limits);
        enter(&mut agent, &realm);
        let prototype = agent.current_intrinsic(Intrinsic::StringPrototype).unwrap();
        let data = agent.new_string_str(&"a".repeat(UNITS)).unwrap();
        agent.keep(Value::String(data));
        let object = agent.string_create(data, prototype).unwrap();
        agent.keep(Value::Object(object));
        (agent, realm, object)
    }

    /// `docs/design/engine.md`, "The step budget": [[OwnPropertyKeys]] of a String object
    /// (`#sec-string-exotic-objects-ownpropertykeys`) charges a step per key it will answer,
    /// counted from the String's length, before the list exists. With 1,000 steps left the run
    /// ends in `Terminated(StepBudget)` and no list was allocated.
    #[test]
    fn own_keys_of_a_long_string_end_at_the_step_budget_before_the_list_exists() {
        let (mut agent, _realm, object) = long_string_object();
        agent.limits.steps_per_entry = agent.steps + 1_000;
        let answer = agent.own_property_keys(object).map(|keys| keys.len());
        assert_eq!(answer, Err(Abrupt::Terminated(Termination::StepBudget)));
        assert_eq!(agent.heap.accounting.largest_key_list(), 0);
    }

    /// "Memory budget": the list of a String object's keys, `size_of::<Key>()` bytes per code
    /// unit, is charged before it is allocated. With 1 MiB left and steps enough for every key,
    /// the run ends in `Terminated(MemoryBudget)` and nothing stays charged.
    #[test]
    fn own_keys_of_a_long_string_are_charged_before_they_are_built() {
        let (mut agent, _realm, object) = long_string_object();
        agent.limits.steps_per_entry = 10_000_000;
        agent.steps = 0;
        let used = agent.heap.bytes();
        agent.heap.accounting.set_limit(used + (1 << 20));
        let answer = agent.own_property_keys(object).map(|keys| keys.len());
        assert_eq!(answer, Err(Abrupt::Terminated(Termination::MemoryBudget)));
        assert_eq!(agent.heap.bytes(), used);
    }

    /// `docs/design/engine.md`, "Memory budget", the lib twin of HOST-215: with 8 MiB of room
    /// and a String of 2^20 code units, `join` nested through ToString holds a builder of
    /// 2 MiB per level, charged, and ends in `Terminated(MemoryBudget)` within the budget with
    /// no capacity uncovered at any entry; with no charge and a separator of 65,536 code units
    /// nested to `reentry_depth` (64 here), the separator is read in place and the run throws
    /// the `RangeError` of `reentry_depth` with no capacity uncovered.
    #[test]
    fn no_buffer_is_uncovered_at_an_entry_across_a_call() {
        use super::across::{Across, story_limits};
        use super::copies;
        use crate::builtins::string::transform::tests::Answer;
        let mut limits = story_limits();
        limits.steps_per_entry = 10_000_000;
        let mut across = Across::new(limits);
        across.string("s", &vec![0x78; 1 << 20]);
        let script = across.parse(
            "var o = { toString: function () { return [s, o].join(\"\"); } }; [s, o].join(\"\")",
        );
        across.room(8 << 20);
        assert_eq!(
            across.evaluate(&script),
            Answer::Terminated(Termination::MemoryBudget)
        );
        assert!(across.agent.heap.accounting.high_water() <= 1 << 26);
        assert_eq!(copies::uncovered_at_entry(), 0);

        // A test thread's stack holds 64 re-entries of this path, not 256 (`min_stack_bytes`
        // asks for more), and the engine's code makes no thread (HOST-1).
        limits.reentry_depth = 64;
        let mut across = Across::new(limits);
        across.string("s", &vec![0x78; 65_536]);
        let answer = across
            .run("var o = { toString: function () { return [o, 1].join(s); } }; [o, 1].join(s)");
        assert_eq!(answer, Answer::Throw("RangeError".to_string()));
        assert_eq!(copies::uncovered_at_entry(), 0);
    }

    /// The scripts of the guard: each enters script with a buffer of its site alive (a
    /// `toString`, `valueOf`, getter, replacer, reviver, mapper or job), or switches a caller of
    /// the readers, on the global String `S`. A script whose name starts with `queue` is
    /// followed by a microtask checkpoint, whose answer and steps are the ones recorded. It runs
    /// last, so the room the queue keeps after its checkpoint is not alive at the other sites.
    const GUARD_SITES: &[(&str, &str)] = &[
        (
            "join separator",
            "var d = 0; var o = { toString: function () { return ++d < 2 ? [o, 1].join(S) : \
             \"e\"; } }; [o, 1].join(S).length",
        ),
        (
            "sort compare",
            "var o = { toString: function () { return S; } }; var p = { toString: function () \
             { return \"p\"; } }; [o, p].sort()[0] === o",
        ),
        ("Call(n)", "[S].map(function (x) { return x.length; })[0]"),
        (
            "New(n)",
            "new Promise(function (r) { r(S); }) instanceof Promise",
        ),
        (
            "replace copies",
            "S.replace(\"\", function () { return \"r\"; }).length",
        ),
        (
            "GetSubstitution",
            "var re = /a/; re.exec = function () { re.exec = function () { return null; }; \
             return { length: 1, 0: \"\", index: 0, groups: { get g() { return \"q\"; } } }; \
             }; re[Symbol.replace](S, \"$<g>\" + S).length",
        ),
        (
            "join builder",
            "var o = { toString: function () { return \"e\"; } }; [S, o].join(\"\").length",
        ),
        (
            "toLocaleString",
            "var o = { toLocaleString: function () { return \"e\"; } }; [S, \
             o].toLocaleString().length",
        ),
        (
            "JSON.stringify out",
            "var o = { toJSON: function () { return 1; } }; JSON.stringify([S, o]).length",
        ),
        (
            "String.raw",
            "var o = { toString: function () { return \"e\"; } }; String.raw({ raw: [S, S] }, \
             o).length",
        ),
        (
            "concat",
            "var o = { toString: function () { return \"e\"; } }; S.concat(o).length",
        ),
        (
            "fromCharCode",
            "var v = { valueOf: function () { return 98; } }; String.fromCharCode.apply(null, \
             S.split(\"\").map(function () { return 97; }).concat([v])).length",
        ),
        (
            "fromCodePoint",
            "var v = { valueOf: function () { return 98; } }; String.fromCodePoint.apply(null, \
             S.split(\"\").map(function () { return 97; }).concat([v])).length",
        ),
        (
            "replaceAll",
            "S.replaceAll(\"a\", function () { return \"r\"; }).length",
        ),
        (
            "@@replace",
            "/a/g[Symbol.replace](S, function () { return \"r\"; }).length",
        ),
        (
            "flatMap",
            "[S, [S]].flatMap(function (x) { return [x]; }).length",
        ),
        (
            "JSON property list",
            "var o = { get k() { return 1; } }; JSON.stringify(o, [S, \"k\"])",
        ),
        (
            "JSON frames",
            "var o = { toJSON: function () { return 1; } }; JSON.stringify([[S, o]], null, \
             4).length",
        ),
        (
            "JSON.parse reviver",
            "JSON.parse(\"[\" + JSON.stringify(S) + \"]\", function (k, v) { return v; }).length",
        ),
        (
            "apply",
            "(function () { return arguments.length; }).apply(null, [S, { toString: function () \
             { return S; } }])",
        ),
        (
            "Function",
            "Function(\"a\", { toString: function () { return \"b\"; } }, \"return \" + \
             S.length).length",
        ),
        ("concatenation", "(S + S).length"),
        (
            "Error toString",
            "var e = new Error(S); e.name = S; String(e).length",
        ),
        ("Number", "Number(S)"),
        ("padStart", "\"x\".padStart(S.length + 5, S + \"-\").length"),
        ("URI", "encodeURIComponent(S).length + decodeURI(S).length"),
        ("RegExp flags", "new RegExp(S, \"gi\").flags"),
        (
            "queue",
            "for (var i = 0; i <= S.length; i++) Promise.resolve(S).then(function (x) { return \
             x; });",
        ),
    ];

    /// The sites of [`GUARD_SITES`] whose buffer is alive across the entry and is a list the
    /// counting build sees, with a non-empty String.
    const GUARD_SEEN: &[&str] = &[
        "join separator",
        "sort compare",
        "Call(n)",
        "New(n)",
        "replace copies",
        "GetSubstitution",
        "join builder",
        "toLocaleString",
        "JSON.stringify out",
        "String.raw",
        "concat",
        "fromCharCode",
        "fromCodePoint",
        "replaceAll",
        "@@replace",
        "queue",
        "flatMap",
        "JSON property list",
        "JSON frames",
        "JSON.parse reviver",
    ];

    /// The answer and the steps of each site of [`GUARD_SITES`] on Strings of 0, 1, 33 and
    /// 300 code units, recorded on the base before the story, in that order.
    const GUARD_RECORDED: &[(&str, usize, &str, u64)] = &[
        ("join separator", 0, "Normal(\"Number(3.0)\")", 76),
        ("sort compare", 0, "Normal(\"Boolean(true)\")", 53),
        ("Call(n)", 0, "Normal(\"Number(0.0)\")", 23),
        ("New(n)", 0, "Normal(\"Boolean(true)\")", 22),
        ("replace copies", 0, "Normal(\"Number(1.0)\")", 17),
        ("GetSubstitution", 0, "Normal(\"Number(1.0)\")", 85),
        ("join builder", 0, "Normal(\"Number(1.0)\")", 32),
        ("toLocaleString", 0, "Normal(\"Number(2.0)\")", 36),
        ("JSON.stringify out", 0, "Normal(\"Number(6.0)\")", 38),
        ("String.raw", 0, "Normal(\"Number(1.0)\")", 34),
        ("concat", 0, "Normal(\"Number(1.0)\")", 26),
        ("fromCharCode", 0, "Normal(\"Number(1.0)\")", 61),
        ("fromCodePoint", 0, "Normal(\"Number(1.0)\")", 61),
        ("replaceAll", 0, "Normal(\"Number(0.0)\")", 14),
        ("@@replace", 0, "Normal(\"Number(0.0)\")", 40),
        ("flatMap", 0, "Normal(\"Number(2.0)\")", 35),
        ("JSON property list", 0, "Normal(\"{\\\"k\\\":1}\")", 42),
        ("JSON frames", 0, "Normal(\"Number(37.0)\")", 77),
        ("JSON.parse reviver", 0, "Normal(\"Number(1.0)\")", 44),
        ("apply", 0, "Normal(\"Number(2.0)\")", 26),
        ("Function", 0, "Normal(\"Number(2.0)\")", 66),
        ("concatenation", 0, "Normal(\"Number(0.0)\")", 8),
        ("Error toString", 0, "Normal(\"Number(0.0)\")", 29),
        ("Number", 0, "Normal(\"Number(0.0)\")", 9),
        ("padStart", 0, "Normal(\"Number(5.0)\")", 24),
        ("URI", 0, "Normal(\"Number(0.0)\")", 17),
        ("RegExp flags", 0, "Normal(\"gi\")", 28),
        (
            "queue",
            0,
            "Normal(CheckpointReport { promise_jobs: 1, host_microtasks: 0 })",
            5,
        ),
        ("join separator", 1, "Normal(\"Number(5.0)\")", 79),
        ("sort compare", 1, "Normal(\"Boolean(true)\")", 54),
        ("Call(n)", 1, "Normal(\"Number(1.0)\")", 23),
        ("New(n)", 1, "Normal(\"Boolean(true)\")", 22),
        ("replace copies", 1, "Normal(\"Number(2.0)\")", 19),
        ("GetSubstitution", 1, "Normal(\"Number(3.0)\")", 91),
        ("join builder", 1, "Normal(\"Number(2.0)\")", 33),
        ("toLocaleString", 1, "Normal(\"Number(3.0)\")", 37),
        ("JSON.stringify out", 1, "Normal(\"Number(7.0)\")", 39),
        ("String.raw", 1, "Normal(\"Number(3.0)\")", 36),
        ("concat", 1, "Normal(\"Number(2.0)\")", 27),
        ("fromCharCode", 1, "Normal(\"Number(2.0)\")", 71),
        ("fromCodePoint", 1, "Normal(\"Number(2.0)\")", 71),
        ("replaceAll", 1, "Normal(\"Number(1.0)\")", 21),
        ("@@replace", 1, "Normal(\"Number(1.0)\")", 52),
        ("flatMap", 1, "Normal(\"Number(2.0)\")", 35),
        ("JSON property list", 1, "Normal(\"{\\\"k\\\":1}\")", 42),
        ("JSON frames", 1, "Normal(\"Number(38.0)\")", 78),
        ("JSON.parse reviver", 1, "Normal(\"Number(1.0)\")", 48),
        ("apply", 1, "Normal(\"Number(2.0)\")", 26),
        ("Function", 1, "Normal(\"Number(2.0)\")", 66),
        ("concatenation", 1, "Normal(\"Number(2.0)\")", 10),
        ("Error toString", 1, "Normal(\"Number(4.0)\")", 33),
        ("Number", 1, "Normal(\"Number(NaN)\")", 10),
        ("padStart", 1, "Normal(\"Number(6.0)\")", 26),
        ("URI", 1, "Normal(\"Number(2.0)\")", 21),
        ("RegExp flags", 1, "Normal(\"gi\")", 29),
        (
            "queue",
            1,
            "Normal(CheckpointReport { promise_jobs: 2, host_microtasks: 0 })",
            10,
        ),
        ("join separator", 33, "Normal(\"Number(69.0)\")", 175),
        ("sort compare", 33, "Normal(\"Boolean(true)\")", 54),
        ("Call(n)", 33, "Normal(\"Number(33.0)\")", 23),
        ("New(n)", 33, "Normal(\"Boolean(true)\")", 22),
        ("replace copies", 33, "Normal(\"Number(34.0)\")", 83),
        ("GetSubstitution", 33, "Normal(\"Number(67.0)\")", 283),
        ("join builder", 33, "Normal(\"Number(34.0)\")", 65),
        ("toLocaleString", 33, "Normal(\"Number(35.0)\")", 69),
        ("JSON.stringify out", 33, "Normal(\"Number(39.0)\")", 71),
        ("String.raw", 33, "Normal(\"Number(67.0)\")", 100),
        ("concat", 33, "Normal(\"Number(34.0)\")", 59),
        ("fromCharCode", 33, "Normal(\"Number(34.0)\")", 391),
        ("fromCodePoint", 33, "Normal(\"Number(34.0)\")", 391),
        ("replaceAll", 33, "Normal(\"Number(33.0)\")", 245),
        ("@@replace", 33, "Normal(\"Number(33.0)\")", 436),
        ("flatMap", 33, "Normal(\"Number(2.0)\")", 35),
        ("JSON property list", 33, "Normal(\"{\\\"k\\\":1}\")", 43),
        ("JSON frames", 33, "Normal(\"Number(70.0)\")", 110),
        ("JSON.parse reviver", 33, "Normal(\"Number(1.0)\")", 176),
        ("apply", 33, "Normal(\"Number(2.0)\")", 26),
        ("Function", 33, "Normal(\"Number(2.0)\")", 68),
        ("concatenation", 33, "Normal(\"Number(66.0)\")", 74),
        ("Error toString", 33, "Normal(\"Number(68.0)\")", 97),
        ("Number", 33, "Normal(\"Number(NaN)\")", 42),
        ("padStart", 33, "Normal(\"Number(38.0)\")", 90),
        ("URI", 33, "Normal(\"Number(66.0)\")", 149),
        ("RegExp flags", 33, "Normal(\"gi\")", 61),
        (
            "queue",
            33,
            "Normal(CheckpointReport { promise_jobs: 34, host_microtasks: 0 })",
            170,
        ),
        ("join separator", 300, "Normal(\"Number(603.0)\")", 976),
        ("sort compare", 300, "Normal(\"Boolean(true)\")", 54),
        ("Call(n)", 300, "Normal(\"Number(300.0)\")", 23),
        ("New(n)", 300, "Normal(\"Boolean(true)\")", 22),
        ("replace copies", 300, "Normal(\"Number(301.0)\")", 617),
        ("GetSubstitution", 300, "Normal(\"Number(601.0)\")", 1885),
        ("join builder", 300, "Normal(\"Number(301.0)\")", 332),
        ("toLocaleString", 300, "Normal(\"Number(302.0)\")", 336),
        ("JSON.stringify out", 300, "Normal(\"Number(306.0)\")", 338),
        ("String.raw", 300, "Normal(\"Number(601.0)\")", 634),
        ("concat", 300, "Normal(\"Number(301.0)\")", 326),
        ("fromCharCode", 300, "Normal(\"Number(301.0)\")", 3061),
        ("fromCodePoint", 300, "Normal(\"Number(301.0)\")", 3061),
        ("replaceAll", 300, "Normal(\"Number(300.0)\")", 2114),
        ("@@replace", 300, "Normal(\"Number(300.0)\")", 3640),
        ("flatMap", 300, "Normal(\"Number(2.0)\")", 35),
        ("JSON property list", 300, "Normal(\"{\\\"k\\\":1}\")", 310),
        ("JSON frames", 300, "Normal(\"Number(337.0)\")", 377),
        ("JSON.parse reviver", 300, "Normal(\"Number(1.0)\")", 1244),
        ("apply", 300, "Normal(\"Number(2.0)\")", 26),
        ("Function", 300, "Normal(\"Number(2.0)\")", 70),
        ("concatenation", 300, "Normal(\"Number(600.0)\")", 608),
        ("Error toString", 300, "Normal(\"Number(602.0)\")", 631),
        ("Number", 300, "Normal(\"Number(NaN)\")", 309),
        ("padStart", 300, "Normal(\"Number(305.0)\")", 624),
        ("URI", 300, "Normal(\"Number(600.0)\")", 1217),
        ("RegExp flags", 300, "Normal(\"gi\")", 328),
        (
            "queue",
            300,
            "Normal(CheckpointReport { promise_jobs: 301, host_microtasks: 0 })",
            1505,
        ),
    ];

    /// The guard of `story:copies-held-across-a-call-into-script-are-charged`: no site gains or
    /// loses a step and no answer changes, on Strings of 0, 1, 33 and 300 code units; after the
    /// story no capacity is uncovered at an entry into script at any site.
    #[test]
    fn the_step_totals_of_the_sites_held_across_a_call_are_unchanged() {
        use super::across::Across;
        use super::copies;
        let mut readings = Vec::new();
        let mut uncovered = Vec::new();
        for length in [0_usize, 1, 33, 300] {
            let mut across = Across::with_steps(1_000_000);
            across.string("S", &vec![0x61; length]);
            for &(name, source) in GUARD_SITES {
                let mut answer = format!("{:?}", across.run(source));
                let mut seen = copies::uncovered_at_entry();
                if name.starts_with("queue") {
                    copies::reset();
                    let ran = across.agent.perform_microtask_checkpoint().unwrap();
                    answer = format!("{ran:?}");
                    seen = seen.max(copies::uncovered_at_entry());
                }
                readings.push((name, length, answer, across.steps()));
                if seen > 0 {
                    uncovered.push((name, length, seen, GUARD_SEEN.contains(&name)));
                }
            }
        }
        let recorded: Vec<(&str, usize, String, u64)> = GUARD_RECORDED
            .iter()
            .map(|&(name, length, answer, steps)| (name, length, answer.to_string(), steps))
            .collect();
        let table: String = readings
            .iter()
            .map(|(name, length, answer, steps)| {
                format!("    ({name:?}, {length}, {answer:?}, {steps}),\n")
            })
            .collect();
        assert!(readings == recorded, "the readings:\n{table}");
        assert_eq!(uncovered, Vec::new());
    }
}

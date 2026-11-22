---
format: aep.planning-md/3
id: story:for-in-key-list-charged-while-the-loop-holds-it
kind: story
status: implemented
title: The key list a for-in loop holds is charged before it exists and stays charged until the loop ends, on every way out
summary: EnumerateObjectProperties charges its list and its set of visited keys by capacity before each grows, credits the set and everything else it charged on every way out of the call, and answers the list with its charge; the for-in enumerator holds that charge in its frame and credits it when the loop ends normally, by break, return, throw or a termination.
relations:
- depends_on: story:own-keys-charged-before-the-list-exists
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T05:27:42Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T05:27:42Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:50Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Waits for: `story:own-keys-charged-before-the-list-exists` and the merges of
`let-const-and-block-scopes` and `template-literals`. Base: `wave/s3-1` with the gate unit
(`gate-test-steps-within-budget`) and the design text merged, and those three merged.
Preconditions: the design text, which the base holds: `engine.md`, "The step budget" (the key
list the `for`-`in` enumerator holds: charged before each growth, held by the enumerator,
credited when it ends on every exit) and "Memory budget" (a temporary structure is credited
when it drops); and the sentence on the set of visited keys in `engine.md` "The step budget"
(`:742-749` of the design text): the set of visited keys is credited when
EnumerateObjectProperties returns, and the enumerator holds the list alone.

## Context

Line numbers are of `wave/s3-1` at `c108c2d`. The two language units change
`crates/swell/src/interp/` before this story starts; the unit finds each site again by its name.

- `for_in_start` (`crates/swell/src/interp/dispatch.rs:1427-1449`) calls
  EnumerateObjectProperties (`crates/swell/src/ops/objects.rs:560-580`), keeps the list it
  answers in an `Enumerator` (`crates/swell/src/interp/frame.rs:72-76`), drops the object from
  the operand stack (`drop_entries(1)`, `dispatch.rs:1447`) and pushes the enumerator
  (`push_entry`, `:1448`), where it stays for the whole loop (the compiler leaves it there,
  `crates/compiler/src/statement.rs:185-230`).
- EnumerateObjectProperties grows its list `found` by `push` and its set `visited`, a
  `BTreeSet` of `key_order` tuples (`ops/objects.rs:564-565`, `:38-53`), with no charge. Its ways
  out are its answer (`:582`) and the `?` of `charge_steps` (`:568`: `StepBudget`,
  `Interrupted`), of `own_property_keys` (`:569`), `get_own_property` (`:573`) and
  `get_prototype_of` (`:580`). After `story:own-keys-charged-before-the-list-exists` the lists
  of own keys it consumes are charged and credited, and the list it answers and `visited` are
  still charged nowhere.
- So a script holds one uncharged list per live loop: a recursive function that runs
  `for (var k in new String(s))` in each of `frame_depth` frames keeps `frame_depth` × the
  length of `s` × `size_of::<Key>()` bytes the memory budget does not see, against one charged
  String.
- A frame already carries a charge that leaves with it: `Frame::charged`
  (`frame.rs:116-117`), credited by `pop_frame` and `drop_frames`
  (`crates/swell/src/interp/mod.rs:166-183`) on a return, an uncaught throw and a termination.
  Entries leave the operand stack without the frame by the `Pop` at the loop's exit, by
  `drop_entries` (`interp/mod.rs:256-265`) of a jump out of the loop, and by the truncation in
  `enter_handler` (`dispatch.rs:136-141`) when a handler catches a throw.
- `push_entry` (`interp/mod.rs:196-215`) ends in `Terminated(MemoryBudget)` only when the stack
  is full and its growth is refused (`:199-210`). In `for_in_start` it follows
  `drop_entries(1)`, so the stack has a free entry there and the growth is not taken.
- `ForInNext` (`dispatch.rs:1456-1481`) answers a String key as its atom (`key_value`), which
  allocates nothing.
- No host function in `crates/swell/tests/` can read the heap: `heap_bytes()`
  (`crates/swell/src/host/agent.rs:210-214`) takes `&Agent`, which the `&mut Cx` of a host
  function (`crates/swell/src/host/mod.rs:111`, `:128-130`) holds, and `Cx` has no reading.

## Change

Failing tests first (Acceptance). Then:

1. **The list and the set are charged before they grow, and credited on every way out of the
   call.** EnumerateObjectProperties grows `found` by its own doubling with
   `try_reserve_exact`, charging the new capacity's bytes (`size_of::<Key>()` per entry) before
   each growth, as `KeyedList::make_room` does (`crates/swell/src/object/property.rs:305-358`).
   `visited` becomes a `KeyedMap` (`crates/swell/src/heap/map.rs`), grown with
   `KeyedMap::reserve` after its new table's bytes (`KeyedMap::table_bytes_for`,
   `map.rs:95-108`) less the old are charged. On every way out of the call other than its
   answer (a growth over the budget, `StepBudget` or `Interrupted` from the step per object at
   `:568` or from the steps of an [[OwnPropertyKeys]], an abrupt completion of
   `own_property_keys`, `get_own_property` or `get_prototype_of`), everything the call charged
   and still holds, `found`, `visited` and the list of own keys being consumed, is credited
   before the completion is returned. On its answer it credits `visited` (the precondition's
   sentence) and answers `found` with its charge, in the type
   `story:own-keys-charged-before-the-list-exists` gives a charged key list.
2. **Steps before the work.** A step per object of the prototype chain before that object's keys
   are asked for (`ops/objects.rs:568`, kept), and the steps of each [[OwnPropertyKeys]] before
   its list exists (`story:own-keys-charged-before-the-list-exists`); the [[GetOwnProperty]]
   and the `visited` insertion of each key are covered by that key's step. Nothing is added.
3. **The enumerator holds the charge.** `Enumerator` records the bytes of its list. In
   `for_in_start` they join the frame's `charged` before `push_entry`, so on every way out
   after the list exists the frame holds them: `pop_frame` and `drop_frames` credit them on a
   return, an uncaught throw and a termination, and a `push_entry` that ends in
   `Terminated(MemoryBudget)` (`interp/mod.rs:196-215`) leaves them in the frame, which
   `drop_frames` credits as the termination unwinds it. No script reaches that answer at this
   site, since `drop_entries(1)` frees an entry just before (Context). One function removes
   entries from a frame's operand stack for `Pop`, `drop_entries` and `enter_handler`; it
   credits what an `Enumerator` among the removed entries holds and lowers `charged` by it, so a
   loop that ends normally, by `break` or `continue` out of it, or by a throw caught outside it
   credits its list at once. The empty enumerator of `undefined` and `null` holds nothing.
4. **Which story does what to the list EnumerateObjectProperties answers:**
   `story:own-keys-charged-before-the-list-exists` leaves it and `visited` uncharged and
   credits the lists of own keys consumed; this story charges both, credits `visited` and
   everything else on every way out of the call, and makes the enumerator hold and credit the
   list.
5. **Stays as it is.** The keys and their order (`#sec-enumerate-object-properties`); the step
   per key `ForInNext` passes over (`dispatch.rs:1456-1481`); the step counter of bytecode. A
   suspended frame (slice 9's generators) keeps its `charged`, the lists its loops hold
   included, and credits it when it is resumed to its end or when the collector frees it.

No requirement of `host-interface.md` changes; this story has no line in
`conformance/host/requirements.txt`.

Writes: `crates/swell/src/ops/objects.rs` (EnumerateObjectProperties and its tests, the
expected value of `enumerate_object_properties_skips_shadowed_and_symbols` included),
`crates/swell/src/interp/{dispatch,frame,mod}.rs` (`for_in_start`, `Enumerator`, the removal of
entries from an operand stack). Not written: `crates/swell/src/heap/map.rs`,
`crates/swell/src/object/` (the charged key list is story 2's type, used as it is; this story
adds nothing to it). Running units: `interp/dispatch.rs`, `interp/frame.rs` and `interp/mod.rs`
are written by `let-const-and-block-scopes` and `template-literals`; this story starts from the
tree after both have merged and after story 2, which writes `ops/objects.rs`.

## Acceptance

Unit tests in the lib test binary of `swell`, in the tests module of
`crates/swell/src/ops/objects.rs`, on agents of the library's test host
(`crates/swell/src/host/hooks.rs:365-440`). With n = 100,000, L = n × `size_of::<Key>()` and
V = `KeyedMap::table_bytes_for(n)`. A test reads the heap from script through a host function
`bytes`, made with the module's helper (`ops/objects.rs:602-611`) and defined on the realm's
global object, which answers `cx.agent.heap.bytes()` as a Number: the value `heap_bytes()`
answers (`heap/mod.rs:101-104`), read inside the running frame, which no host function of
`crates/swell/tests/` can do (Context).

- `for_in_enumerators_held_by_a_recursion_end_in_memory_budget`: `frame_depth` 64,
  `steps_per_entry` 100,000,000, above the steps of 61 loops: each charges at most 2n + 10
  steps for the String's own keys (story 2) and a step and the own-key steps per object of its
  prototype chain, and breaks after one key. A global `s` of n code units; `memory_bytes` =
  `heap_bytes()` after `s` exists + 3L + V + 64 KiB. One loop's peak is the String's own-key
  list and `found`, each at most 2^17 entries (the doubling's capacity for n, 1.32 L), and
  `visited` at V: at most 2.63 L + V, under the budget; a second loop beside a held list of
  1.32 L passes it. The script
  `function f(d) { for (var k in new String(s)) { if (d > 0) f(d - 1); break; } } f(60); 1`.
  Today `Normal(1)`: each frame's list is held uncharged. After `Terminated(MemoryBudget)`, and
  the next script `1` answers `Normal(1)`. The same limits with `f(0)` answer `Normal(1)`
  before and after.
- `every_exit_of_a_for_in_credits_its_key_list`: `steps_per_entry` 1,000,000 (the test host's
  limits, `host/hooks.rs:390-400`), above the at most 2 × 10,000 + 10 steps of the object's own
  keys and the 10,000 key steps of a whole loop, so only the case built to end in `StepBudget`
  ends there; `memory_bytes` 1 GiB. Over a global object `o` of 10,000 String keys, seven
  scripts whose loop ends normally, by `break`, by `continue` of an outer labelled loop, by
  `return` from a function, by a throw caught outside the loop, by an uncaught throw (the entry
  answers `Throw`) and by `Terminated(StepBudget)` inside the body (`while (true) {}`). In each
  the body stores `inside = bytes()`, which reads above `before = bytes()`, read just before
  the loop in the same frame, by at least 10,000 × `size_of::<Key>()`. In the first five the
  script stores `after = bytes()` right after the loop in the same frame (for `return`, right
  after the call, in the caller's frame), and `after` reads at most `before` + 4 KiB: the list
  was credited when the loop ended, not when the frame pops. In every case, after `collect()`,
  `heap_bytes()` reads the same after the second and the tenth run of the script. Today the body
  reads no such rise: the list of own keys was credited when EnumerateObjectProperties
  returned, and the held list is not charged.
  An eighth case ends inside EnumerateObjectProperties: on its own agent with
  `steps_per_entry` 40,000, `o` has the 10,000 keys and a prototype chain of 50,000 objects
  with no keys, made by the test with `ordinary_object_create` as
  `enumerate_object_properties_skips_shadowed_and_symbols` makes its two
  (`ops/objects.rs:691-696`); `for (var k in o) {}` charges at most 20,010 steps for `o`'s
  keys, then a step per object of the chain (`:568`), so it answers `Terminated(StepBudget)`
  while EnumerateObjectProperties walks the chain, after `found` and `visited` hold `o`'s
  keys; after `collect()` `heap_bytes()` reads the same after the second and the tenth run.
  Without the credit on that way out each run would leave `found` and `visited` charged.
  The exit at `push_entry` (Change 3) is not among the cases: no script reaches it.
- `enumerate_object_properties_skips_shadowed_and_symbols` (`ops/objects.rs:688-723`): its
  expected value is edited (`:716-719`). The call now answers story 2's charged key list, so
  the assertion compares the keys the list holds, read as story 2's type gives them, with
  `[Key::Index(0), length, a, c]`, and the test then credits the list. The keys and their
  order do not change.
- The tests of `crates/swell/tests/builtins_object_enumeration.rs` pass unchanged.

Commands, each one job of the build pool: `slot cargo nextest run --locked -p swell --profile
default --lib`, `slot cargo nextest run --locked -p swell --profile default --test <b>` for each
`<b>` of `builtins_object_enumeration` and `language`, the same with `--features
collect-every-safe-point`, `slot cargo clippy --workspace --all-targets --locked -- -D
warnings`, `slot task conform` and `slot task conform-engine` (the counts of the base), and at
the end, once, `slot task test262 TEST262_OUT=.out/test262-unit`, whose `runs.tsv` is compared
with that of `slot task test262 TEST262_OUT=.out/test262-base` run on the base: `awk -F'\t' '{
print $3, $4, $5 }'` over each, and `cmp` of the two outputs exits 0 (no pass lost in the
directories named here). Never `cargo test -p swell` whole, `slot task test-collect`, `slot task
test` or `slot task check`.

Directories: `test/language/statements/for-in` and `test/annexB/language/statements/for-in`.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change, except one named as a guard. A test is killed at 60 s and the
gate's test steps fail over 120 s; each test runs in a process of its own under cargo-nextest,
so no fixture and no counter is shared through a `static` between tests (a counting build
counts within one test's process). Each new test is timed, and one over 5 s is named in the
report with its time. A job of the build pool asks for one slot. Determinism: the same script
under the same budgets ends the same way at the same count on every run. Nothing is added to
the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack tests are unchanged. No
third-party crate, no unsafe code.

## Cost

Bound: the CPU seconds (user + system) of `slot cargo nextest run --locked -p swell --profile
default --test builtins_object_enumeration` (its `for`-`in` tests; this story adds none there),
built first with `--no-run` and measured with `/usr/bin/time -f '%U %S'` inside the pool job,
three runs on the base and three on the unit, the median of each, rise by at most 3 %. Why 3 %:
per loop the change adds one charge per doubling of `found` and of `visited` and one credit at
the end, against a [[GetOwnProperty]] and an insertion per key; `visited` trades a B-tree
insertion for a hash insertion of the same key. Past the bound the unit reports the three
numbers of each side and stops before merging.

## Report

The unit's report gives the new tests with their times, the Cost numbers, and this sentence for
the changelog: "The key list a `for`-`in` loop holds is now charged to `memory_bytes` before it
grows and while the loop runs, and credited when the loop ends, by `break`, `return`, a throw
or a termination included."

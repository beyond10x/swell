---
format: aep.planning-md/3
id: story:arena-capacity-and-collector-workspace-on-the-ledger
kind: story
status: implemented
title: An arena's slots are charged by capacity while the agent lives, the collector's workspace is part of that charge, and the host reads the live bytes beside the charged ones
summary: Slots are charged when an arena grows and stay charged; a sweep credits what a freed record owns; a mark word per slot and a worklist of at most one entry per slot live in each arena, so a collection allocates nothing and finishes at the heap limit; live_bytes() reads what the records hold, and the collection threshold follows it.
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T05:27:41Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T05:27:41Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:49Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Waits for: nothing; starts on the base at once. Base: `wave/s3-1` with the gate unit
(`gate-test-steps-within-budget`) and the design text merged. Preconditions: the design text of
decision D5 (taken: option A), which the base holds: `engine.md` "Collector" and "Memory
budget" (capacity charged at growth, the workspace, the threshold of the live reading, "Two
readings") and `host-interface.md` section 2 (the `memory_bytes` row and "Two readings of the
heap": `heap_bytes()` the charged reading, `live_bytes()` the live one); and the sentence on the
atom table in `engine.md` "Memory budget" and "Two readings" and in `host-interface.md` "Two
readings of the heap": the atom table's buckets are named in `heap_bytes()` and left out of
`live_bytes()` where empty. Stories that wait for this one:
`story:frontend-scratch-on-the-memory-ledger`.

## Context

Line numbers are of `wave/s3-1` at `c108c2d`.

- `store` (`crates/swell/src/heap/mod.rs:177-192`) charges `slot_bytes::<T>()`
  (`crates/swell/src/heap/arena.rs:41-44`) plus what the record owns, then `Arena::allocate`
  (`arena.rs:66-92`) reuses a freed slot (`:67-79`) or pushes a new one after `try_reserve(1)`
  (`:80-86`): the `slots` vector grows by the policy of the standard library, and its capacity
  beyond its length is charged to nothing.
- `free_index` (`arena.rs:130-144`) takes the record's whole charge, slot included (`:135`),
  which the sweep credits (`:151-167`, `crates/swell/src/heap/collect.rs:245-252`), while the
  slot stays in `slots` to be used again; the index is pushed on `free` (`arena.rs:139`), a
  vector that grows with no charge. After a collection `heap_bytes()` therefore falls by the
  slot bytes of every freed record although the arena keeps those slots, and an allocation that
  reuses one charges its slot bytes again.
- A collection allocates its own workspace, with no charge and no fallible reservation:
  `Marks::new` (`collect.rs:117-127`) makes a `vec![false; slot_count]` for each of the six
  arenas at every collection, and the worklist `Mark::work` (`:47-52`) grows by `push`
  (`:64-66`). An item is marked only when it is popped (`:192-232`), and every reference pushes
  its target (`Mark::value`, `:69-77`), so the worklist holds one entry per reference, not per
  record: an Array whose n elements name one object pushes n entries.
- The atom table (`crates/swell/src/heap/string.rs:168-200`) charges each entry its key and the
  size of one entry (`:137-140`); the table's buckets are charged nowhere, and `Atoms::sweep`
  (`:189-200`) drops entries with `retain`, which keeps the table's capacity.
- The collection threshold is the charged reading after the last collection
  (`Heap::collected`, `heap/mod.rs:112-116`: `trigger_bytes = used()`). `collect`
  (`collect.rs:238-258`) calls `Heap::collected` at `:253`; the bytes of the property maps the
  sweep freed are credited after it returns, by `collect_now` (`crates/swell/src/host/agent.rs:590-596`,
  the credit at `:592-593`), so the reading `Heap::collected` takes still holds them.
- An ordinary object owns no bytes beside its slot (`crates/swell/src/object/mod.rs:204`).
- `heap_bytes()` (`agent.rs:210-214`) is the one reading of the heap a host has.

## Change

Failing tests first (Acceptance). Then:

1. **Capacity, charged at growth.** An arena grows by its own doubling (at least 16 slots) with
   `try_reserve_exact`; before it grows, `store` charges (new capacity − old capacity) × the
   bytes of one slot, where one slot is `size_of::<Slot<T>>()`, one `u32` of the free list, one
   `u32` mark word and one `u32` of room on the worklist (point 3). Over the budget, or when the
   allocator refuses, nothing is charged or stored and the caller ends the run in
   `Terminated(MemoryBudget)`, as today. The free list is reserved to the arena's capacity at
   the same growth, so a free never allocates. These bytes stay charged while the agent lives;
   arenas do not shrink.
2. **A record is charged what it owns.** `store` charges the owned bytes; `Slot::bytes` holds
   them; the sweep credits them. A record that reuses a freed slot is charged its owned bytes
   only.
3. **The workspace lives in each arena.** An arena holds, beside `slots` and `free`, its mark
   words and its worklist room, grown with its capacity (point 1). The mark state of a slot is
   one `u32`: 0 unmarked, otherwise the generation marked plus one. `Mark::push` marks an item
   when it is reported and pushes its index on its arena's worklist only when it was not
   marked, so each worklist holds at most one entry per slot of its arena; a report for a marked
   slot with another generation, or of `Item::NoArena`, is a stale handle and ends the
   collection in `Terminated(EngineFault)` before the sweep, as today (`collect.rs:22-23`); the
   record is still read and checked when popped. `collect` takes the mark words and worklists
   out of the arenas for the mark phase (`std::mem::take`, no allocation) and gives them back
   cleared. A collection allocates nothing, so one that starts at the memory limit finishes.
4. **`store` keeps its signature** (`heap/mod.rs:177-182`: the accounting, one arena, the
   value, the owned bytes): the growth of point 1 is the growth of that one arena, so the
   callers `env/mod.rs:269`, `realm/mod.rs:292` and `script.rs:383` do not change. `Mark` keeps
   its name and its methods (`push`, `value`, `object`, `count`) and gains no lifetime, so none
   of the trace sites of the other modules (`object/*.rs`, `env/*.rs`, `realm/*.rs`,
   `interp/*.rs`, `script.rs`, `host/*.rs`) changes.
5. **The atom table is charged by capacity, like an arena** (the precondition's sentence).
   Before the table grows, `Atoms` charges `KeyedMap::table_bytes_for` of the new room less
   that of the old (`crates/swell/src/heap/map.rs:95-108`) and reserves it with
   `KeyedMap::reserve`; an entry is then charged its key's units only; the table's bytes stay
   charged while the agent lives. Not shrunk on sweep: shrinking a `HashMap` moves it to a new
   table, an allocation inside a collection, which point 3 rules out.
6. **The live reading.** Each arena counts its slots that hold a live record.
   `Agent::live_bytes()`, beside `heap_bytes()` (`agent.rs:210-214`), answers the charged
   reading less the room: for each arena the slots that hold no record (free or never used)
   with the workspace of every slot, and the atom table's buckets that hold no entry. Between
   entries that is what the live records own with their slots, property maps included. It
   answers `Err(Faulted)` on a faulted agent as `heap_bytes()` does, is at most `heap_bytes()`
   and falls by everything a sweep frees.
7. **The threshold follows the live reading, taken after every credit of the collection.**
   `collect` (`collect.rs:238-258`) no longer sets the threshold at `:253`; `collect_now`
   (`agent.rs:590-596`) credits the freed property maps (`:592-593`) and then calls
   `Heap::collected` (`heap/mod.rs:112-116`), which sets the threshold from the live reading
   (point 6), not from `used()`. So the threshold counts neither the freed property maps nor the
   room of the arenas, and garbage is collected before the charged reading reaches
   `memory_bytes`. `collect` keeps resetting nothing else; `collect_now` remains its one caller.
   Triggers still count bytes, never time.
8. **The requirements that read the heap**, each with the reading its test reads after the
   change: HOST-10 (`swell::limits_scripts::host_10_endless_allocation_ends_in_memory_budget`)
   the charged one, unchanged; HOST-28 (`swell::collect::host_28_an_object_held_only_by_a_root_survives_collect`)
   neither, unchanged; HOST-29 (`swell::collect::host_29_dropping_the_last_root_lowers_heap_bytes`)
   the live one; HOST-52 (`swell::realms::host_52_dropped_realm_returns_heap_bytes`) the live
   one; HOST-70 (`swell::termination::host_70_agent_stays_alive_after_each_budget`) the charged
   one, which sizes its budget, unchanged; HOST-71 (`swell::faults::host_71_injected_fault_at_each_call_faults_the_agent`)
   both on a faulted agent; HOST-102 (`swell::misuse::host_102_reads_change_nothing_and_call_no_hook`)
   both: the design text adds `live_bytes()` to the readings a read leaves unchanged;
   HOST-132 (`swell::collect::host_132_report_matches_heap_bytes`) the charged one, unchanged.
   The tests of HOST-29, HOST-52, HOST-71 and HOST-102 keep their names, so their lines of
   `conformance/host/requirements.txt` stay as they are. HOST-171 (section 2, the two readings,
   `[slice 6]`), which the design text adds, is this story's; its line is the one the design text
   added, `HOST-171 pending slice 6`.
9. **Stays as it is.** The order of the sites and "reported twice, traced once"
   (`mark_phase_calls_every_site_of_the_layout`, `collect.rs:316-418`); the generation check of
   every access; the step counter; HOST-70 (an agent that ended in `MemoryBudget` runs `1`).
10. Not charged, by the design's rule (`engine.md`, "Memory budget": what the host's own calls
   hold is the host's and bounded by its calls): the root table (`crates/swell/src/heap/roots.rs`)
   and `Agent::scope`. The room `Agent::kept` keeps is
   `story:own-keys-charged-before-the-list-exists`'s.

Writes: `crates/swell/src/heap/{arena,collect,mod,string}.rs` (with their tests),
`crates/swell/src/host/agent.rs` (`live_bytes` beside `heap_bytes`, lines 210-214, and
`collect_now`, lines 589-596, nothing else); the test files
`crates/swell/tests/collect.rs` (the tests of HOST-29 and HOST-171),
`crates/swell/tests/realms.rs` (the test of HOST-52), `crates/swell/tests/misuse.rs` (the test
of HOST-102 only), `crates/swell/tests/termination.rs` (one new test),
`crates/swell/tests/faults.rs` (`answers_faulted_everywhere` only),
`crates/swell/tests/budgets.rs` (`memory_budget_of_zero_refuses_every_allocation` only), and,
for their expected byte numbers only, `crates/swell/tests/{determinism,tracing}.rs` (files the
gate unit changed; the base holds its version), `crates/swell/src/host/{hooks,host_object}.rs`
and `crates/swell/src/builtins/string/mod.rs` (their tests' budgets); and
`conformance/host/requirements.txt` (the line of HOST-171). Not written: any trace site outside
`heap/`, `crates/swell/src/heap/{accounting,map,roots}.rs`. Running units: no file of
`let-const-and-block-scopes` or `template-literals`. Shared: `host/agent.rs` with
`story:interrupt-read-at-every-parse-entry` (its `parse_script`, other lines) and
`tests/collect.rs` and `tests/misuse.rs` with `story:values-read-from-a-root-live-for-the-scope`
(its HOST-170 test and `good_values`), both starting beside this one or after it; the unit
that lands second gets the wave branch
merged into its tree and resolves the overlap (`AGENTS.md`, "How the work is run").

## Acceptance

Unit tests in the lib test binary of `swell`, with counting builds of the tests (as
`Mark::count` records the sites):

- `arena.rs`: `the_ledger_covers_the_arenas_over_repeated_allocation_and_collection`: ten rounds
  of 10,000 objects allocated with none kept, each followed by a collection; after every
  allocation and every collection `accounting.used()` is at least the counting build's sum of
  the capacities of `slots`, `free`, the mark words and the worklists times their element
  sizes, plus the owned bytes of the live records, and that sum's high-water mark stays at most
  `memory_bytes`. Today the first collection makes `used()` fall below the sum: red.
- `collect.rs`: `a_collection_allocates_nothing`: the counting build counts, during `collect`,
  every vector the collector creates and every push past capacity onto the mark words, the
  worklists, the free lists of the sweep and the atom table (a check of `len() == capacity()`
  before each push); it reads 0. Today it reads at least 6, the bitmaps of `Marks::new`, and
  the growths of `Mark::work`.
  `the_worklist_holds_at_most_one_entry_per_slot`: an Array of 100,000 elements that all name
  one object: the worklists' peak sum is at most the number of slots (today at least 100,000).
  `a_stale_handle_reported_for_a_marked_slot_ends_the_collection` (a guard of the new marking,
  green before and after). `a_collection_at_the_memory_limit_finishes` (a guard, green before
  and after): `memory_bytes` 1 MiB, objects allocated until `Terminated(MemoryBudget)`, then
  `collect()` answers `Normal` and `heap_bytes()` at most 1 MiB.
  `the_collection_threshold_follows_the_live_reading` (point 7): an agent of the library's test
  host (`crates/swell/src/host/hooks.rs:365-440`) with `memory_bytes` 1 GiB and a realm; the
  test stores 262,144 ordinary object records straight into the heap (`Heap::new_object`,
  `heap/mod.rs:119-126`, owned bytes 0) and keeps none; two `collect()`s. Then U =
  `heap_bytes()` and L = `live_bytes()`; the test sets the limit to M = U + max(L, 1 MiB) +
  8 MiB (`Accounting::set_limit`, `heap/accounting.rs:55-59`) and asserts U > max(L, 1 MiB) +
  8 MiB: the at least 262,144 empty object slots stay charged, each at least 52 bytes with its
  workspace (point 1), 13 MiB in all, so a threshold of max(U, 1 MiB) is larger than the room
  M − U and could only fire after the budget refused a charge. The script
  `for (var i = 0; i < 128; i++) { var t = "x".repeat(1 << 19); } 1` makes only garbage, 64 MiB
  of it, at most two of its Strings (each at most 1 MiB of units with its slot) live at once;
  it answers `Normal(1)`: with the threshold at max(live reading, 1 MiB), at most max(L, 1 MiB)
  + 2 MiB is charged between collections, under M − U by 6 MiB. Left at `trigger_bytes =
  used()`, it answers `Terminated(MemoryBudget)`. Today the test does not compile
  (`live_bytes()` does not exist). With the feature `collect-every-safe-point` every safe
  point collects, so there it passes whichever reading the threshold takes; the run without
  the feature is the check of point 7.
- `string.rs`: `the_atom_table_is_charged_by_its_capacity`: 10,000 atoms interned, then a
  collection that frees them all: `used()` before and after the collection is at least
  `KeyedMap::table_bytes()` of the table (today the table's buckets are not in `used()`, and
  after the sweep `used()` falls by every entry while the table keeps its room).

Integration tests:

- `crates/swell/tests/collect.rs`, `host_171_live_bytes_reads_the_records_and_heap_bytes_the_budget`,
  as HOST-171 words it: with `memory_bytes` M = the realm's `heap_bytes()` + 4 MiB, after each
  entry and each `collect` of the test `live_bytes()` reads at most `heap_bytes()` and
  `heap_bytes()` at most M; the script `(function () { var a = []; while (true) a.push({}); })()`
  answers `Terminated(MemoryBudget)`; two `collect`s lower `live_bytes()` by more than they
  lower `heap_bytes()`; then k is taken from the readings with `live_bytes()` + 2 × k ≤ M <
  `heap_bytes()` + k (the test asserts such a k exists) and the script `"x".repeat(k).length`
  answers `Terminated(MemoryBudget)`. Today `live_bytes()` does not exist (the test does not
  compile), and after the drop `heap_bytes()` falls by the slots as well.
- `host_29_dropping_the_last_root_lowers_heap_bytes` reads `live_bytes()` where it read
  `heap_bytes()`; `host_52_dropped_realm_returns_heap_bytes` (`crates/swell/tests/realms.rs`)
  reads `live_bytes()` where it read `heap_bytes()`, and its equality holds (with
  `heap_bytes()` it fails under the change: the realm's arena capacity stays charged);
  `host_102_reads_change_nothing_and_call_no_hook` (`crates/swell/tests/misuse.rs`) reads
  `live_bytes()` beside `heap_bytes()` before and after the reads it makes.
- `crates/swell/tests/faults.rs`: `answers_faulted_everywhere` (`:30-68`) asserts
  `live_bytes()` answers `Err(HostError::Faulted)` beside its `heap_bytes()` line (`:34`), so
  `host_71_injected_fault_at_each_call_faults_the_agent` checks it after a fault at each of
  the three points. Today it does not compile.
- `crates/swell/tests/termination.rs`,
  `an_agent_that_ended_in_memory_budget_runs_again_after_a_collection` (a guard, green before
  and after): `memory_bytes` the realm's `heap_bytes()` + 64 KiB, as HOST-70 sizes it
  (`termination.rs:58-64`), and the support limits' `steps_per_entry` of 1,000,000
  (`crates/swell/tests/support/host.rs:63-75`): each iteration of the fill adds an element to
  `a`, whose capacity is charged as it doubles, so 64 KiB lasts at most 65,536 /
  `size_of::<Value>()` iterations, far within the steps. `var n = 0; function fill() { var a =
  []; while (true) { a.push({}); n++; } } fill();` answers `Terminated(MemoryBudget)`;
  `collect()` answers `Normal`; `n` answers the count of objects the fill made, which the test
  prints; `(function () { var a = []; for (var i = 0; i < 10; i++) a.push({}); return
  a.length; })()` answers `Normal(10)`. The second step reuses what the fill freed: its ten
  objects and its Array take slots of the objects arena that the fill's objects left free, at
  their owned bytes, 0 (`object/mod.rs:204`), and its call's environment the slot of `fill`'s
  call. The room figure: after the collection, M − `heap_bytes()` is at least what the
  collection credited of the fill, the element capacity of its `a` (at least 16 ×
  `size_of::<Value>()` once the fill made 11 objects) and its frame (`Frame::charged`, credited
  at the termination, `crates/swell/src/interp/mod.rs:177-183`), against the second step's
  16 × `size_of::<Value>()` of elements, its frame and its code; the test prints M −
  `heap_bytes()` and `live_bytes()` after the collection. When the guard is red after the
  change, the unit stops and reports those figures and `n`, and changes no budget:
  `engine.md` says a budget holds the arenas' spare capacity.
- The tests that size `memory_bytes` from `heap_bytes()` or a `realm_bytes()` helper, with the
  reading each uses after the change (the charged one, the only one they read) and whether its
  expectation holds. Three facts carry most of them: `Accounting::charge` refuses a charge that
  would pass the limit and charges nothing (`heap/accounting.rs:30-38`), so every "`heap_bytes()`
  at most the budget" holds; a realm made in the test's agent repeats the probe's charges,
  growths included, and leaves nothing for a collection (`budgets.rs:25-31` asserts it), so a
  budget of the probe's reading plus a room leaves that room; a refusal by a record's owned
  bytes beyond the room happens as today (point 2).
  - `budgets.rs`: `memory_budget_ends_with_cx_in_memory_budget` (`:38-71`, room 1,024): 2,000
    units own more than 1,024 bytes, refused; the next entry's `"small"` charges 5 owned bytes
    in a free or spare slot of the strings arena: holds. `memory_budget_charges_utf16_strings_and_host_data`
    (`:75-106`): 600 UTF-16 units own 1,200 bytes and `Big` 2,048, both over 1,024: holds.
    `memory_budget_of_zero_refuses_every_allocation` (`:108-122`, room 0) cannot hold as
    written: `new_string("")` owns 0 bytes, so in a spare slot it charges 0 and is made. The
    unit changes it to `new_string("x")`, which owns at least one byte (or grows the arena) and
    so is refused with room 0, and its comment to "every allocation that charges a byte"; its
    other assertions hold (the refusal charges nothing).
  - `limits_scripts.rs`, HOST-10 (`:43-71`, rooms 4,096, 65,536, 1 MiB beyond the realm read
    after a collection, `:27-32`): `a` keeps every object, so the loop ends by memory (steps
    `u64::MAX`, `:52`; each element is charged with `a`'s capacity); `heap_bytes()` at most the
    budget holds. `1;` after it charges its code's owned bytes in the scripts arena's spare slot
    (the loop's script grew that arena to 16 slots); the termination credited the loop's frame
    (`interp/mod.rs:177-183`), whose operand stack alone was charged 8 entries
    (`interp/mod.rs:196-215`): holds where that covers `1;`'s code, as on the base, where `1;`
    also paid its slot.
  - `property_index.rs`, `memory_budget_of_8_mib_ends_a_push_loop` (`:28-51`; 8 MiB, or the
    realm + 64 KiB with `collect-every-safe-point`, `:31-35`): the HOST-10 loop and `1 + 1`:
    as HOST-10.
  - `names.rs`, `memory_budget_ends_a_loop_that_keeps_arguments_objects_and_with_records`
    (`:103-131`, realm + 64 KiB): `kept` holds everything, each iteration adds an element, so
    each loop ends by memory; it asserts `heap_bytes()` at most the budget and `Alive`: holds.
  - `calls.rs`, `entries_check_their_realm_first` (`:607-649`): `heap_bytes()` equal before and after calls refused with
    `ForeignHandle` or `WrongKind`, which allocate nothing: holds.
  - `builtins_function_source.rs`, `the_source_text_is_charged_and_credited` (`:332-396`): its
    readings differ by the source text's 100,000 units, owned bytes charged at the parse and
    credited by the sweep (point 2): holds; with the realm + 150,000 the parse is refused by
    the text's charge: holds. `apply_of_a_length_of_2_53_ends_in_a_budget` (`:520-542`, the
    realm + 1 MiB): the argument list grows without end and is charged as it grows: holds.
  - `builtins_object_integrity.rs`, `define_properties_charges_its_lists_to_the_memory_budget`
    (`:646-695`, its setup `redefine_agent` `:606-638`): `per_unit` is the 1,024-unit probe's charge over 1,024 in whole bytes, the
    same before and after (a slot is under 1,024 bytes); `left` (`:677-678`) is room + 1,024 −
    the filler's charge beyond its units, at most `per_unit` off: 0 when the filler takes a
    free or spare slot of the strings arena, so `room ≤ left < room + 4,096` holds unless the
    filler's String is the one that doubles that arena. The redefinition's lists are owned
    charges over 16 KiB and under 1 MiB, and the two scripts' records take slots of the scripts
    arena that `REDEFINE_ALL`'s script grew: both outcomes hold.
  - `termination.rs`, HOST-70 (`:57-97`, realm + 64 KiB, `steps_per_entry` 10,000): the memory
    script `a = [a, a]` ends by memory within 10,000 steps when the objects arena's doubling is
    refused once the realm's spare slots are used, or when 64 KiB of element capacity, 65,536 /
    (2 × `size_of::<Value>()`) Arrays, is reached before; an Array in a spare slot is charged its
    two elements only, so this run makes more Arrays than on the base. `1;` after each
    termination as in HOST-10.
  - `builtins_array_mutation.rs`, `memory_budget_ends_a_sort_of_a_long_array_like`
    (`:573-597`, realm + 512 KiB): the sort's list of 100,000 values is an owned charge over
    512 KiB: holds; the next script's Array and String take slots the ended run's records
    freed, credited with the frame that held them.
  - `dynamic_function.rs` (`:221-243`) and `eval.rs` (`:561-585`), realm + 512 KiB: `s` of
    65,536 units fits; the code compiled from it is an owned charge past the room: holds; the
    next small function or `eval` as in the sort test.
  - `builtins_object_enumeration.rs`, `memory_budget_ends_the_keys_of_a_long_string`
    (`:398-424`, realm + 300,000): `Object.keys(long)` needs 50,000 Strings, so the strings
    arena must reach 65,536 slots, at least 3.4 MB charged at its doublings; `Object.assign`
    charges its 50,000 keys first: both refused, holds; `Object.keys("abc").join()` as in the
    sort test.
  - `builtins_array_iteration.rs`, `flat_of_an_array_that_holds_itself_ends_in_a_budget`
    (`:711-744`, realm + 1 MiB): each open level is an owned charge and the levels never end:
    holds.
  - `builtins_uri.rs`, `encode_past_the_memory_budget_terminates` (`:539-554`): the result of
    120,000 units is an owned charge past 64 KiB + 40,000 beyond the realm: holds.
  - `realms.rs`, HOST-12 (`:23-38`, budgets 0 and 1,000): a realm takes far more than 1,000
    bytes, refused before it is made: holds.
  Tests that read `heap_bytes()` only to compare it with another reading (`collect.rs` HOST-132,
  `misuse.rs`, `determinism.rs`, `property_index.rs:151-184`, `fuzz.rs:1825-1957`) or on a
  faulted agent (`interrupt.rs:106`, `host_objects.rs:200`, `eval.rs:290`) read the charged
  reading after the change and hold. No test module of `crates/swell/src` sizes `memory_bytes`
  from a reading. A listed test whose expectation does not hold where this list says it does,
  or a test outside this list that goes red, is reported with its numbers before the unit
  changes anything in it, and the unit stops. A test of the files listed for their byte
  numbers whose expected number moves is named in the report with the old and the new number
  and why.
- `conformance/host/requirements.txt`: the line the design text added, `HOST-171 pending slice
  6`, becomes `HOST-171 test swell::collect::host_171_live_bytes_reads_the_records_and_heap_bytes_the_budget`;
  `slot task host-requirements-check` then prints one pending line fewer than on the base.

Commands, each one job of the build pool: `slot cargo nextest run --locked -p swell --profile
default --lib`, `slot cargo nextest run --locked -p swell --profile default --test <b>` for each
`<b>` of `collect`, `realms`, `termination`, `limits_scripts`, `misuse`, `determinism`,
`tracing`, `budgets`, `property_index`, `names`, `calls`, `builtins_function_source`,
`builtins_object_integrity`, `builtins_array_mutation`, `dynamic_function`, `eval`,
`builtins_object_enumeration`, `builtins_array_iteration` and `builtins_uri`, the same with
`--features collect-every-safe-point`, `slot cargo nextest run --locked -p swell --profile
default --features fault-injection --test faults`, `slot cargo clippy --workspace
--all-targets --locked -- -D warnings`, `slot task conform` and `slot task conform-engine` (the
counts of the base), and at the end, once, `slot task test262 TEST262_OUT=.out/test262-unit`,
whose `runs.tsv` is compared with that of `slot task test262 TEST262_OUT=.out/test262-base` run
on the base: `awk -F'\t' '{ print $3, $4, $5 }'` over each, and `cmp` of the two outputs exits 0
(no pass lost in the directories named here). Never `cargo test -p swell` whole, `slot task
test-collect`, `slot task test` or `slot task check`.

Directories: all; the budget field of `test262 total` stays 0 (a case ending by `MemoryBudget`
sooner would show there). The cases that allocate most:
`test/staging/sm/JSON/parse-mega-huge-array.js`,
`test/built-ins/Array/prototype/sort/stability-2048-elements.js`.

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

Memory: each slot is charged for its capacity, a free-list entry, a mark word and a worklist
entry. The HOST-10 script (`var a = []; while (true) a.push({});`) at 64 MiB makes, after the
change, at least 0.4 and at most 1.0 times the `a.push({})` iterations it makes on the base
before `Terminated(MemoryBudget)`. Why 0.4: on the base an iteration is charged one object slot
S = `size_of::<Slot<ObjectRecord>>()` and the element it adds, 16 to 32 bytes of element
capacity; after, the doubling charges at most twice the slots in use, each with 12 bytes of
workspace, so the ratio is at least (S + 16) / (2S + 24 + 32), above 0.4 for S of 40 bytes or
more (a slot holds the generation, the charge and the record, whose property list alone is a
`Vec`). The report prints S, `size_of::<Slot<T>>()` for the other five arenas and both counts.

CPU bound: the CPU seconds (user + system) of `slot cargo nextest run --locked -p swell
--profile default --test tracing` (collections under a 4 MiB budget, 100,000 arrays of
garbage), built first with `--no-run` and measured with `/usr/bin/time -f '%U %S'` inside the
pool job, three runs on the base and three on the unit, the median of each, rise by at most 3 %.
Why 3 %: an allocation adds one comparison of length and capacity; the mark phase reads a mark
word per report instead of per pop and pushes no more than before; each collection clears the
mark words in place of allocating six bitmaps. Past the bound the unit reports the three
numbers of each side and stops before merging.

## Report

The unit's report gives the new tests with their times, the slot sizes and counts of Cost, the
figures the guard prints, the tests whose numbers moved, the CPU numbers, and this sentence for
the changelog: "`memory_bytes` and `heap_bytes()` now count an arena's slots by capacity,
charged when the arena grows and kept while the agent lives, with the collector's workspace and
the atom table's buckets; before, a slot was charged with each record and credited by the sweep
that freed it, and the collector's workspace was not charged. A collection now lowers
`heap_bytes()` by what the freed records owned; the new `live_bytes()` reads what the live
records hold, slots included, falls by everything a sweep frees, and sets the collection
threshold. A String of no code units in a spare slot is no longer refused by a budget with no
room left."

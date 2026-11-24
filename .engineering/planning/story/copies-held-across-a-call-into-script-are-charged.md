---
format: aep.planning-md/3
id: story:copies-held-across-a-call-into-script-are-charged
kind: story
status: active
title: A buffer whose size a script controls is charged while the engine calls into script, or is not made
summary: Copies of a String's units, String builders, lists of values, the argument copy of call and new, the kept stack's room and the queue of jobs are charged to memory_bytes by capacity before they are made and credited when dropped, or the code reads in place; HOST-215 and HostError::MemoryBudget.
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T12:36:28Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T12:36:28Z", actor: "human:operator", revision: 3}
---
Waits for: nothing; starts on `wave/s10-1` at `76aa4fc`. Consumer: the browser that embeds the
engine. Its condition for running page scripts is that no buffer of the engine stays alive without
a charge while the engine calls back into script. Closes the two rows "The `call` and `new`
instructions copy their argument list uncharged" and "`string_units` and `string_units_in` copy
without a memory charge" of `docs/plan/second-batch.md`, "Findings left open" (lines 284-285);
the coordinator edits that file on the merge. Adds HOST-215. The check that holds later code to
the rule is `story:buffer-growth-outside-the-charged-list-is-listed`, built after this one.

The rule this story makes true:

- Every buffer of the engine outside its arenas, whose size a script controls and that is alive
  while the engine calls into script, is charged to `memory_bytes` by its capacity before it is
  made or grows, and credited when it drops.
- Or it is not made, and the code reads in place.
- This covers:
  - copies of a String's code units, and the units a String builder grows;
  - lists of values, keys, captures and records;
  - the microtask queue;
  - the kept stack's room.
- A copy of n code units charges n steps before it is made, unless its caller charged those
  units first.
- Two exceptions, each named in the design text:
  - The URI functions copy first and charge each unit as they read it.
  - The `call` and `new` instructions charge no step for their copy of the arguments.

## Context

Line numbers are of `76aa4fc`, under `crates/swell/src/`. No file under `crates/` changed since
`57e49c4` (`git diff --stat 57e49c4 76aa4fc -- crates` prints nothing).

Counts on this base (`grep -rnF '<text>' crates/swell/src | wc -l`):

| text | lines |
|---|---|
| `string_units(` | 31 |
| `string_units_in(` | 6 |
| `to_utf16()` | 6 |
| `peek_values` | 3 |
| `charged_units(` | 10 |
| `read_units(` | 32 |
| `StringBuilder::new` | 34 (33 outside test modules) |
| `self.keep(` | 293 |
| `kept.try_reserve` | 5 |
| `accounting.check(` | 7 |

Explicit growth outside test modules, read by a scan that skips everything from `#[cfg(test)]
mod` on:

| call | lines |
|---|---|
| `.try_reserve(` | 18 |
| `.try_reserve_exact(` | 43 |
| `.to_vec()` | 5 |
| `with_capacity(` | 0 |
| `.collect()` | 16 |
| `Vec::new()` | 82 |

The readers of code units:

- `string_units` (`object/string.rs:48-53`) copies the whole String, with no step and no byte.
- `charged_units` (`:57-61`) charges a step per unit, and copies with no byte.
- `string_units_in` (`:65-82`) copies a range, with no step and no byte.
- `read_units` (`builtins/string/access.rs:185-205`) charges a step per unit, and copies with no
  byte.
- The test-only `copies` module (`object/string.rs:240-261`) records the longest copy since a
  reset.

The String builder:

- `StringBuilder` (`builtins/string/mod.rs:72-82`) is a plain `Vec<u16>`.
- `append_units` (`:199-220`) and `append_string` (`:231-266`) charge a step per unit, then only
  *check* that the whole result fits the room (`accounting.check`, `:213`, `:246`). The
  standard library then grows the buffer.
- Nothing is charged while a builder lives, so every builder alive at once checks against the
  same room.
- `finish_string` (`:269-271`) copies the units into a String, which is charged.

The ledger today:

- `Accounting::charge`, `check`, `credit` and `used` (`heap/accounting.rs:75`, `:116`, `:124`,
  `:135`).
- Three types whose drop adds bytes to an `Rc<Cell<u64>>` that the agent credits later: `Grown`
  (`object/mod.rs:140-162`), `CreditGuard` (`interp/suspend.rs:38-48`), and `HostBytes`
  (`host/host_bytes.rs:27-48`, credited after a sweep or at the next host charge).
- A list charged by capacity with the engine's doubling: `Charged<T>` and `push_charged`
  (`builtins/regexp/symbol.rs:64-79`, `:93-110`). Its owner credits `bytes` by hand.
- The kept stack's room: `KeptRoom { capacity, charged }`, `reserve_kept` and `end_kept_room`
  (`object/builtin_function.rs:338-341`, `:406-436`).
  - `reserve_kept` charges the growth before it reserves, and a refused charge ends the run in
    `Terminated(MemoryBudget)` (`:416-418`).
  - `end_kept_room` restores the capacity and credits `charged` by hand, so a holder that
    returns before calling it keeps the charge and the capacity.
  - `JSON.stringify` already holds one room and ends it on every path (`builtins/json/mod.rs:
    166-178`).
- Lists charged per element that grow by the standard doubling, so up to half their capacity is
  uncharged:
  - the open sources of `flat` (`builtins/array/iteration.rs:500-515`, `OPEN_BYTES` `:115`);
  - the property list of `JSON.stringify` (`builtins/json/mod.rs:269-277`), its frames and
    `indent` (`:610-623`);
  - the records of `JSON.parse` (`builtins/json/parse.rs:504-519`) and their children (`:582`),
    and its walker's frames (`:974-981`).
- The cycle set of `JSON.stringify`, `on_stack: BTreeSet<(u32, u32, u32)>`
  (`builtins/json/mod.rs:97`):
  - It is inserted at `:623` when a frame opens and removed at `:740` when it closes.
  - Each frame's charge includes `size_of::<(u32, u32, u32)>() * 4` (`:610-614`), so it holds
    at most one entry per open frame.
  - The walk is a loop over `frames` with no depth limit (`:12`). The open frames are bounded by
    `memory_bytes` through their charge.
  - A B-tree allocates its nodes as entries are inserted. No test here measures those bytes:
    an allocator that counts would need `unsafe`.
- The microtask queue (`host/jobs.rs:132-141`):
  - It doubles a `VecDeque` with `try_reserve_exact`, at least 4 entries, with no fallback to a
    smaller growth.
  - Its capacity is not charged (`:11-13`).
  - A promise job's entry is charged (`promise_job_bytes`, `:93-97`): a step, then the entry's
    bytes, then the reservation. A refused reservation credits the bytes and faults the agent
    (`:170-176`).
  - A host microtask's entry is not charged.
  - The capacity stays at its peak until a checkpoint empties the queue, then is shrunk to
    `SPARE_ENTRIES` = 64 (`:73-75`, `:246-248`).
  - `drop_microtasks` takes the whole buffer (`:199`).

Where the engine calls into script: every call goes through one of two re-entry checks:

- `enter_function` (`object/builtin_function.rs:673-681`), for builtin and host functions;
- `enter_code` (`interp/mod.rs:117`), for ECMAScript code from Rust, generator and async
  resumption included through `resume_frame` (`interp/suspend.rs:130`).

The `Call(n)` and `New(n)` instructions call an ECMAScript callee without leaving the loop:
`push_call_frame` (`interp/dispatch.rs:1419-1421`) and `push_construct_frame` (`:1496-1498`) make
its frame and return before its code runs.

The scan. For each call into script, every engine-side buffer alive at that moment was read:

- the callers of the four readers;
- the 33 builder sites;
- every growth call of the table above, and every `Vec`/`VecDeque` they grow;
- every growth of the kept stack.

| # | site | buffer | alive across | decision |
|---|---|---|---|---|
| 1 | `builtins/array/mod.rs:336-345` (`join`) | separator, `string_units` (`:341`) | `Get` `:370`, `ToString` `:373` | in place |
| 2 | `builtins/array/mutation.rs:789-802` (sort, no comparator) | ToString of an object operand, `string_units` (`:800`) | ToString of the other operand (`:759`) | in place |
| 3 | `interp/dispatch.rs:1408`, `:1410` (`Call(n)`) | `peek_values` of n+2 and `to_vec` of n | `internal_call` `:1422` | charged, one copy |
| 4 | `interp/dispatch.rs:1487`, `:1489` (`New(n)`) | the same, n+1 and n | `construct_value` `:1499` | charged, one copy |
| 5 | `builtins/string/replace.rs:306`, `:308` | the String's and searchString's units (`read_units`) | the replacer, `:343`, `:469` | charged |
| 6 | `builtins/string/replace.rs:520` (GetSubstitution) | the template's units (`read_units`) | `Get` `:699`, `ToString` `:704` | charged |
| 7 | `builtins/array/mod.rs:360` (`join`) | builder | as 1 | charged |
| 8 | `builtins/array/search.rs:530` (`toLocaleString`) | builder | Invoke `:543` | charged |
| 9 | `builtins/json/mod.rs:172` (`JSON.stringify`, `State::out`) | builder | `json_walk` `:229` | charged |
| 10 | `builtins/string/access.rs:336` (`String.raw`) | builder | `Get` `:343`, `ToString` `:345`, `:361` | charged |
| 11 | `builtins/string/access.rs:468` (`concat`) | builder | `ToString` `:473` | charged |
| 12 | `builtins/string/access.rs:260` (`fromCharCode`) | builder | `ToUint16` `:264` | charged |
| 13 | `builtins/string/access.rs:276` (`fromCodePoint`) | builder | `ToNumber` `:280` | charged |
| 14 | `builtins/string/replace.rs:338` (`replace`) | builder, empty across the replacer | `:343` | charged |
| 15 | `builtins/string/replace.rs:461` (`replaceAll`) | builder | `:469` | charged |
| 16 | `builtins/string/replace.rs:521` (GetSubstitution) | builder | `:699`, `:704` | charged |
| 17 | `builtins/regexp/symbol.rs:388` (`@@replace`) | builder | `:397` | charged |
| 18 | `host/jobs.rs:132-141`, `:246` | the queue's capacity | every job | charged by capacity |
| 19 | `builtins/array/iteration.rs:510` (`flat`, `flatMap`) | open sources, and their kept slots | `Get`, the mapper | `ChargedVec`, kept room |
| 20 | `builtins/json/mod.rs:273` | property list | `Get` `:249`, `ToString` `:253-255` | `ChargedVec` |
| 21 | `builtins/json/mod.rs:617-618` | frames, `indent` | toJSON, the replacer, getters | `ChargedVec` |
| 22 | `builtins/json/parse.rs:512`, `:582` | records, children | the reviver | `ChargedVec` |
| 23 | `builtins/json/parse.rs:979` | walker's frames | the reviver | `ChargedVec` |
| 24 | `builtins/function.rs:224` (`apply`) | kept stack, `try_reserve(1)` | `Get`, the call | kept room |
| 25 | `builtins/json/parse.rs:505` | kept stack, `try_reserve(2)` | the reviver | kept room |
| 26 | `builtins/json/mod.rs:276` | kept stack, `keep` per item, inside the room of `:174` | as 20 | kept room (already ended on every path) |
| 27 | `dynamic_code.rs:756` (`Function` and the other dynamic constructors) | kept stack, `keep` per parameter | ToString of each part (`:709-716`) | kept room |

The browser named sites 1 to 4 and `string_units`; it did not name sites 5 to 27.

Charged by hand at their exact length, alive across a call, and kept as they are. The design text
lists them:

- the activation's copy of the arguments (`object/builtin_function.rs:704-719`, the credit at
  `:731`; `interp/mod.rs:136-150`);
- the spread list (`interp/spread.rs:95-123`);
- the rest list (`interp/parameters.rs:52-70`);
- the `apply` list (`builtins/function.rs:216-235`);
- the bound arguments (`object/bound_function.rs:88-96`, `:235-240`);
- `param_strings` (`dynamic_code.rs:729-750`);
- the descriptors of `Object.freeze` and `Object.seal` (`builtins/object/integrity.rs:283-291`);
- every `KeyList` (`object/mod.rs:83-86`), and `SourceCopy` (`script.rs:345-368`);
- the text of `JSON.parse` (`json_text_units`, `builtins/json/parse.rs:263-271`);
- the positions of `replaceAll` (`builtins/string/replace.rs:429-441`);
- the buffers of `normalize` (`builtins/string/transform.rs:690-705`);
- the captures and group names of RegExpBuiltinExec (`builtins/regexp/exec.rs:111-121`,
  `:319-330`);
- the sort's list on its kept room (`builtins/array/mutation.rs:595-656`).

## Change

The order of the work has three phases. They are named by letter so they do not clash with the
numbered points below.

- **Phase A: stand-ins with the base's behaviour.** Everything the tests name is added in a form
  that keeps the base's behaviour, so the lib test binary compiles in phase B and every test is
  red by an assertion, never by a build error:
  - `Charge`, with `grow` and `release`, and `Accounting::hold`. In phase A, `hold` answers a
    `Charge` of 0 bytes and charges nothing.
  - `ChargedVec<T>` and `Agent::reserve_vec`. In phase A, `reserve_vec` grows with
    `try_reserve_exact` by the doubling of point 2 and charges nothing.
  - `ChargedVec::uncharged()` at sites 1 to 23, and at the four readers' answers.
  - `KeptRoom` holding a `Charge`, and `Agent::with_kept_room`. In phase A, `reserve_kept` and
    `end_kept_room` charge and credit as today, and `with_kept_room` runs its body and does not
    end the room. The holders call `end_kept_room` by hand, as today.
  - `HostError::MemoryBudget`, which nothing answers in phase A.
  - The test-only instrument of point 9, and the counter `INSTRUCTION_ARGUMENTS_COPIED` beside
    `CALL_ARGUMENTS_COPIED` (`object/ecmascript_function.rs:655-664`). The queue's `Charge`
    reports its capacity to the instrument already in phase A.
  - No step and no byte moves in phase A, and every existing test stays green.
- **Phase B: the tests.** Each test of `## Tests` is written, and the bracket after each
  assertion is what it reads on the phase-A tree. The guards are green there.
- **Phase C: the charges.** Points 1 to 8, with the stand-ins replaced. `ChargedVec::uncharged()`
  is gone at the end: `grep -rn 'uncharged()' crates/swell/src` prints nothing.

1. **One guard that gives its bytes back on drop.**
   - `heap/accounting.rs` gains `Charge`, built like `HostBytes`: the bytes it holds, and a clone
     of an `Rc<Cell<u64>>` of the accounting. Its drop adds its bytes to the cell.
   - `charge`, `credit` and `hold` fold the cell into `used` first. `check` and `used()` take
     `&self`, so they subtract the cell without folding it. A credit is exact at every reading.
   - `Accounting::hold(bytes) -> Result<Charge, OverBudget>` charges like `charge`. It does not
     add to `since_collection`, since the collector frees none of it, so the moments of
     collection stay as they are.
   - A collection that runs inside a call reads the held charges in `live_bytes()` when
     `Heap::collected` sets `trigger_bytes`. That is right: they are live.
   - `Charge::grow(bytes)` charges more into a held charge. `Charge::release(bytes)` gives back
     part of it at once, at most what it holds.
2. **`ChargedVec<T>`.**
   - `Charged<T>` moves from `builtins/regexp/symbol.rs:64-79` to `heap/accounting.rs`, as a
     `Vec<T>` with a `Charge` in place of `bytes`. It dereferences to `[T]`.
   - It has no method that grows it other than `reserve_vec`. Its `push` takes room already
     reserved and answers `Err(Full)` when there is none, which its callers turn into a fault of
     the agent (a broken invariant). So no later code regrows it past its charge.
   - `push_charged` (`:93-110`) becomes `Agent::reserve_vec(&mut ChargedVec<T>, more)`:
     - It wants twice the capacity, at least 16 and at least the need, and charges that growth
       with `Charge::grow` before `try_reserve_exact`.
     - When the doubled growth is refused, it charges the exact need instead. Only when that is
       refused too does it end in `Terminated(MemoryBudget)`, with nothing grown. So a buffer
       whose need fits the room never fails where the base's check passed.
     - A refused reservation releases the growth's charge and ends in
       `Terminated(MemoryBudget)`.
     - `Vec::capacity()` after `try_reserve_exact` is at least the request (the standard
       library's contract). The charge is made for the request, and the instrument compares it
       with `capacity()`, so a difference shows there.
   - The callers in `symbol.rs` use it, and their credits of `bytes` go.
   - The lists charged per element, sites 19 to 23, become `ChargedVec` grown by `reserve_vec`.
     Their per-element byte charges go: `OPEN_BYTES`'s list part, `json_charge` of the frame's
     own bytes, the item of the property list, and `json_parse_charge` of the record and child.
     No step is charged at any of these lines, so no step total moves.
   - The frame charge of `JSON.stringify` keeps its `size_of::<(u32, u32, u32)>() * 4` for
     `on_stack`, which stays a `BTreeSet`.
3. **The readers answer a charged copy.** `Units` is `ChargedVec<u16>`.
   - `charged_units` and `read_units` charge their steps, then hold the bytes, then copy.
   - `string_units_in` holds the bytes and copies, with no step.
   - `string_units` becomes `string_units_for_tests`, under `#[cfg(test)]` only, for the test
     lines that call it.
   - `Agent::extend_units(&mut ChargedVec<u16>, StringRef, range)` copies a range of a heap String
     into room that `reserve_vec` made, with no step.
   - Concatenation (`ops/operators.rs:133-136`) and `Error.prototype.toString`
     (`builtins/error.rs:408-414`) charge their steps as today. Then each makes one `reserve_vec`
     of n+m (n+2+m) units and copies both Strings in place with `extend_units`. There is one
     charged buffer, where the base had two or three uncharged copies.
   - StringToNumber (`ops/conversion.rs:184`), CanonicalNumericIndexString (`:436`) and the flags
     of `RegExp` (`builtins/regexp/mod.rs:523`, at most `MOST_FLAGS` = 8 units, `:190`) switch to
     `string_units_in(s, 0, length)` with their steps unchanged.
   - The URI functions (`builtins/global.rs:250`) switch the same way. They keep copying first and
     charging each unit in their loops (`:266-336`), as
     `story:string-builtins-charge-before-they-copy` decided. That is the first named exception.
   - `padStart`/`padEnd` read their filler in place: `string_pad`
     (`builtins/string/transform.rs:376-411`) takes the filler as a String with the count of
     units it uses, or a space, and appends its ranges with `append_string` (`:231`). That charges
     a step per unit, as `append_units` does today. `string_units_in` at `:365` goes.
   - `json_text_units` (`builtins/json/parse.rs:263-271`) drops its own byte charge, which
     `charged_units` now makes.
4. **The builder is charged while it lives.** `StringBuilder` holds a `ChargedVec<u16>`.
   - `append_units` and `append_string` keep their order: the `RangeError` past `string_length`,
     the step per unit, then the memory. The memory part is `reserve_vec` in place of `check`,
     with the doubling capped at `string_length` units.
   - `finish_string` takes the units out, drops the charge, then makes the String.
   - The places that grow `builder.units` by hand go through `reserve_vec`: `result_builder`
     (`builtins/string/transform.rs:231-249`), the doubling of `repeat` (`:288-316`) and
     `BuilderBuffer` (`:785-795`).
5. **In place, sites 1 and 2.**
   - `join` keeps the separator's String (`keep`) and appends it with `append_string`, or `","`
     when the separator is undefined. Each append charges `|sep|` steps, as today.
   - The sort's compare keeps the String that ToString of an object answers, and compares it in
     place with `cmp_string` / `cmp_units` (`heap/string.rs:197`).
   - The units of a Number, a Boolean or `null` (`units_of`, at most 25 units) stay as they are,
     and so does `charge_steps(compared)` (`mutation.rs:766`).
6. **One charged copy of the arguments, sites 3 and 4.**
   - `call_instruction` and `new_instruction` read `this` and the function with `peek_value`.
   - When the callee is a builtin or a host function, they copy the n arguments once, into a
     `ChargedVec<Value>` charged before it is made. It is dropped, and so credited, when the call
     returns, on every way out.
   - This copy charges bytes and no step. Each of the n values was pushed by an instruction that
     spent a step in the same code, so the copy is in proportion to steps already charged. That is
     the second named exception.
   - When the callee is ECMAScript code, the frame path copies the n arguments once into a plain
     `Vec`. `push_call_frame` / `push_construct_frame` move them into the frame's charged slots,
     and the `Vec` is dropped before the frame's code runs. That path charges nothing new, so no
     result near a limit moves on it. The base made two copies there; this path makes one.
   - `peek_values` (`interp/mod.rs:330-349`) goes.
   - `INSTRUCTION_ARGUMENTS_COPIED` counts the values the two instructions copy, on both paths.
7. **The microtask queue, site 18.**
   - The agent holds a `Charge` of the queue's capacity in bytes: capacity ×
     `size_of::<Entry<H>>()`.
   - The queue grows as today: a full buffer doubles, at least 4 entries, with no fallback to a
     smaller growth. A fallback would grow one entry at a time, and the refusal tests need one
     fixed size of growth.
   - **A promise job** (`enqueue_promise_job`, `:165-192`), in this order:
     1. its step;
     2. its entry's bytes (`charge_bytes(promise_job_bytes)`), as today;
     3. when the buffer is full, the capacity growth with `Charge::grow`;
     4. the reservation with `try_reserve_exact`.
     - A refused entry charge ends the run in `Terminated(MemoryBudget)` with nothing queued, as
       today.
     - A refused capacity charge credits the entry's bytes and ends the running entry in
       `Terminated(MemoryBudget)`, with nothing queued.
     - A refused reservation releases the capacity growth with `Charge::release`, credits the
       entry's bytes, and faults the agent, as today.
     - While queued, the slot is charged twice, in the capacity and in the entry's bytes. That
       way no recorded reading of a queued job moves.
   - **A host microtask** (`enqueue_microtask`, `:146-157`; `Agent::queue_microtask`,
     `Cx::queue_microtask`):
     - When the buffer is full, it charges the capacity growth with `Charge::grow`, then
       reserves.
     - A refused charge queues nothing, changes nothing else, and answers
       `Err(HostError::MemoryBudget)`.
     - A refused reservation releases the growth and answers `Err(HostError::Faulted)` with the
       agent faulted, as today.
   - **Giving room back.**
     - After `shrink_to(SPARE_ENTRIES)` (`:246-248`) the queue's `Charge` releases the bytes
       above `capacity()` × `size_of::<Entry<H>>()`.
     - `drop_microtasks` (`:198-208`) takes the buffer and releases all of its capacity.
   - **`HostError::MemoryBudget`**, in `host/outcome.rs`: "The memory budget refused the room the
     call needs; nothing changed." Its doc comments at `host/agent.rs:696-701` and
     `host/mod.rs:1027-1031` say so in place of "not charged to the memory budget". The module
     text at `host/jobs.rs:8-13` changes the same way.
8. **Kept rooms, sites 19 and 24 to 27.**
   - `KeptRoom` holds a `Charge` in place of `charged`. `reserve_kept` grows it, and
     `end_kept_room` restores the capacity; the credit comes from the drop.
   - `Agent::with_kept_room(|agent, room| …)` takes the room, runs the body and calls
     `end_kept_room` on every way out.
   - The holders that take it:
     - `flat`'s open sources;
     - `apply` (`create_list_from_array_like` and `function_apply`);
     - the parse of `JSON.parse` (ended where `:198` releases);
     - `JSON.stringify` (`:166-178`), whose hand-written end goes;
     - `create_dynamic_function` (`dynamic_code.rs:729-739`).
   - The per-element charge of the kept slot goes at `function.rs:216-222` (one of its two
     slots), at `json/mod.rs:271-272` and from `OPEN_BYTES`.
9. **The test-only instrument.**
   - In test builds, every `ChargedVec` and the queue's `Charge` add their capacity in bytes and
     their charged bytes to two thread-local tallies of `copies`. They do so on each growth, and
     take them off on drop.
   - `enter_function` and `enter_code` call `copies::entered()`. It records the most, at any
     entry into script, of the capacity not covered by a charge (the first tally less the
     second).
   - `copies::uncovered_at_entry()` reads it, and `copies::reset()` clears it with `LONGEST`.
   - A buffer that is not a `ChargedVec` is not seen. The kept stack is one: sites 24 to 27 are
     covered by tests 8 and 9, not by the instrument.
10. **Stays as it is.**
    - Every result, every `RangeError` and `SyntaxError` and its message.
    - Every step total: no site gains or loses a step.
    - `heap_bytes()` after an entry, except for the queue's retained capacity (point 7).
    - The moments of collection, determinism, `STACK_BYTES_PER_REENTRY` and `keep` itself.

Design text, `docs/design/engine.md`, "Memory budget", added after "The rule, for every slice"
(`:1569`):

> Added on 2026-11-24: a buffer of the engine outside its arenas whose size a script controls (a
> copy of a String's code units, the units a String builder grows, a list of values, keys,
> captures or records, the microtask queue, the kept stack's room) is charged by its capacity
> before it is made or grows, and credited when it drops. It is held as a `ChargedVec` grown only
> by `Agent::reserve_vec`, or as a `Charge` (`heap/accounting.rs`), whose drop gives the bytes back
> at once; the kept stack's room is a `KeptRoom` holding a `Charge`, ended on every way out by
> `Agent::with_kept_room`. A list charged by hand at its exact length is allowed only when this
> paragraph names it: the activation's copy of the arguments, the spread and rest lists, the
> `apply` list, the bound arguments, `param_strings` of `CreateDynamicFunction`, the descriptors
> of `Object.freeze` and `Object.seal`, `KeyList`, `SourceCopy`, the text of `JSON.parse`, the
> positions of `replaceAll`, the buffers of `normalize`, the captures and group names of
> RegExpBuiltinExec, and the list of `Agent::coerce_all`. Each is bounded by its count of
> elements, which steps or the arguments bound. One set is charged per entry and not by its
> nodes: the cycle set of `JSON.stringify` (`on_stack`, a `BTreeSet`), charged four keys' bytes
> in each open frame's charge and bounded by the open frames, which `memory_bytes` bounds through
> that charge; the walk has no depth limit. A buffer alive while the engine calls into script is
> never left uncharged: it is charged, or the code reads in place and makes none. Three are
> bounded by a limit and not charged: the list of activations, at most `reentry_depth` + 1
> entries (at 256, 257 × `size_of::<Activation>()` = <A> bytes); the spare capacity of the frame
> stack, at most `frame_depth` frames, each charged `frame_bytes()` (spare at most `frame_depth` ×
> `size_of::<Frame>()` = <F> bytes at `frame_depth` 10,000); and the growth of the kept stack by
> `keep` outside a room, a fixed count of values per running builtin (at `reentry_depth` 256, at
> most <K> × `size_of::<Value>()` = <V> bytes). A copy of n code units charges n steps before it
> is made, unless its caller charged those units first; the URI functions copy first and charge
> each unit as their loops read it; the copy of a call's n arguments by `Call(n)` and `New(n)`
> charges no step, since each was pushed by an instruction that spent one.

The unit measures `<A>`, `<F>` and `<V>` with `size_of` in a test. `<K>` is the most values one
builtin keeps outside a room, which the unit reads from the code. It writes the numbers into the
sentence and gives each number's source in its report.

`docs/design/engine.md`, "**Queued jobs.**" (`:1599-1637`). Every sentence the change makes
false is replaced:

- `:1601-1602`, "a host microtask costs the budget nothing", becomes "a host microtask costs its
  slot of the queue's capacity, and a refused growth answers `Err(HostError::MemoryBudget)` with
  nothing queued".
- `:1624-1629`, "A host microtask is not charged: its box is the host's allocation …", becomes "A
  host microtask's box is not charged: it is the host's allocation …". The rest of that sentence
  is unchanged.
- `:1629-1632`, "The queue's buffer grows by the engine's own doubling with `try_reserve_exact`;
  its spare capacity … is not charged, and a checkpoint … shrinks a buffer of more than 64
  entries to 64", becomes: "The queue's buffer grows by the engine's own doubling with
  `try_reserve_exact`, with no smaller fallback; its capacity is charged by a `Charge` before it
  grows, credited when a checkpoint that leaves the queue empty shrinks a buffer of more than 64
  entries to 64, and credited whole when the queue is dropped. While a promise job is queued its
  slot is charged twice, in the capacity and in its entry. A refused growth for a promise job
  credits the entry's bytes and ends the entry in `Terminated(MemoryBudget)` with nothing queued;
  for a host microtask `queue_microtask` answers `Err(HostError::MemoryBudget)` with nothing
  queued".
- `:1635-1637`, "Rejected: charging a host microtask (…); charging the queue's capacity as an
  arena's (…)", becomes: "Rejected: charging a host microtask's box (the closure is the host's
  allocation, which the engine cannot size); leaving the queue's capacity uncharged (2026-11-24:
  a script that queues jobs would hold a buffer it sized across every job it runs)."

`docs/design/host-interface.md`:

- Section 2, at `:170-173` after "(HOST-181, HOST-182).", add: "Added on 2026-11-24: a buffer the
  engine holds while it calls into script is charged to `memory_bytes` before it is made, or not
  made (HOST-215)."
- Section 2, after HOST-182 (`:275`), add the requirement. Every requirement of the file ends in
  a slice tag (`grep '^\*\*HOST-' docs/design/host-interface.md | grep -vc '\[slice [0-9]*\]$'`
  prints 0), and the budget requirements HOST-175 to HOST-182 read `[slice 6]`. The line:

  > **HOST-215** (swell:D2) With `memory_bytes` M = 67,108,864, `string_length` 2^24,
  > `reentry_depth` 256 and `steps_per_entry` 10,000,000, a global `s` defined by the host as a
  > String of 1,048,576 code units `x`, and a charge with `charge_host_bytes` of all of the room
  > M − `heap_bytes()` but 8,388,608 bytes, held until the entry ends, the script `var o = {
  > toString: function () { return [s, o].join(""); } }; [s, o].join("")` answers
  > `Terminated(MemoryBudget)`; `heap_bytes()` before the entry, after it and after a `collect()`
  > reads at most M each time, and once the host drops its charge and runs `collect()` the script
  > `1` answers `Normal(1)`. [slice 6]

- The enum block (`:583-591`): `MemoryBudget,` after `EvaluationInProgress,`.
- The error table (`:635-646`) gets a row: "`queue_microtask` when the memory budget refuses the
  queue's growth | none | `Err(MemoryBudget)`".
- Section 11, "What a queued job costs" (`:1220-1225`): "A host microtask is not charged: …"
  becomes "A host microtask's slot of the queue's capacity is charged, and a refused growth
  answers `Err(HostError::MemoryBudget)` with nothing queued; its closure is the host's own
  storage …".

`conformance/host/requirements.txt` gains `HOST-215 test
swell::budgets::host_215_a_buffer_held_across_a_call_into_script_is_charged`. The file's `test`
lines carry no slice tag (`sed -n 206,208p`).

Writes:

- `crates/swell/src/heap/accounting.rs`: `Charge` with `grow` and `release`, `ChargedVec`,
  `hold`, the folded cell, test 1.
- `crates/swell/src/object/string.rs`: the readers (`:48-82`), `extend_units`, the `copies`
  module (`:240-261`), test 13 and the guard.
- `crates/swell/src/builtins/string/access.rs`: `read_units` (`:185-205`), test 4.
- `crates/swell/src/builtins/string/mod.rs`: `StringBuilder` (`:72-82`, `:199-271`), and the
  test `builder_past_the_memory_budget_terminates` (`:756-774`).
- `crates/swell/src/builtins/string/transform.rs`: `:231-249`, `:288-316`, `:355-411`,
  `:785-795`.
- `crates/swell/src/builtins/array/mod.rs`: `:330-377`, test 2.
- `crates/swell/src/builtins/array/mutation.rs`: `:745-802`, test 3.
- `crates/swell/src/builtins/array/iteration.rs`: `:100-115`, `:495-560`.
- `crates/swell/src/builtins/string/replace.rs`: the types of `Matched` (`:885`) and of
  `Replacement::Template`, test 6.
- `crates/swell/src/builtins/regexp/symbol.rs`: `:64-110` and its callers.
- `crates/swell/src/interp/dispatch.rs`: `:1406-1424`, `:1485-1502`, test 7.
- `crates/swell/src/interp/mod.rs`: `peek_values` goes, and `copies::entered` in `enter_code`.
- `crates/swell/src/object/builtin_function.rs`: `KeptRoom`, `reserve_kept`, `end_kept_room`,
  `with_kept_room` (`:333-437`), `copies::entered` in `enter_function`, tests 8 and 9.
- `crates/swell/src/object/ecmascript_function.rs`: `INSTRUCTION_ARGUMENTS_COPIED` beside
  `CALL_ARGUMENTS_COPIED` (`:658-664`).
- `crates/swell/src/builtins/function.rs`: `:160-240`.
- `crates/swell/src/builtins/json/parse.rs`: `:190-271`, `:480-590`, `:960-985`.
- `crates/swell/src/builtins/json/mod.rs`: `:160-184`, `:236-282`, `:600-630`, test 5.
- `crates/swell/src/dynamic_code.rs`: `:722-758`.
- `crates/swell/src/host/jobs.rs`: `reserve_entry`, `enqueue_microtask`, `enqueue_promise_job`,
  `drop_microtasks`, `run_microtasks`, the module text (`:8-13`), tests 10 to 12.
- `crates/swell/src/host/agent.rs`: the field of the queue's `Charge`, and the doc of
  `queue_microtask` (`:696-705`).
- `crates/swell/src/host/mod.rs`: the doc of `Cx::queue_microtask` (`:1027-1045`).
- `crates/swell/src/host/outcome.rs`: `HostError::MemoryBudget`.
- `crates/swell/src/ops/operators.rs` and `crates/swell/src/builtins/error.rs`: one `reserve_vec`
  and `extend_units`.
- The callers of the readers whose types the compiler names, and nothing else in them:
  - `crates/swell/src/ops/conversion.rs`;
  - `crates/swell/src/builtins/global.rs`;
  - `crates/swell/src/builtins/regexp/{mod,exec,match_all,escape}.rs`;
  - `crates/swell/src/builtins/string/pattern.rs`;
  - `crates/swell/src/builtins/iterator.rs`;
  - `crates/swell/src/builtins/mod.rs`.
- `crates/swell/tests/budgets.rs`: HOST-215.
- `docs/design/engine.md`: "Memory budget", and "Queued jobs" (`:1599-1637`).
- `docs/design/host-interface.md`: section 2 (`:170-173`, after `:275`), the enum block
  (`:583-591`), the error table (`:635-646`), and section 11 (`:1220-1225`).
- `conformance/host/requirements.txt`.

Not written by the unit:

- `CHANGELOG.md`, `website/data/status.json`, `conformance/test262/expected.txt` and
  `conformance/test262/expected.digest`. The coordinator makes one full test262 run on the
  join, compares it run by run with the base, and writes those files on the merge.
- `docs/plan/second-batch.md`, `AGENTS.md`, and any `clippy.toml`.

## Tests

A bracket gives what the assertion reads on the phase-A tree. Strings are made by the host with
`new_string_utf16`. Unless a test says otherwise, it runs under `limits()` of the test host: 64
MiB, 1,000,000 steps, `reentry_depth` 256, `frame_depth` 10,000. "Room R" means `set_limit` (lib)
or a held `charge_host_bytes` (integration) leaves R bytes free before the entry.

Every new test has `across_a_call` in its name and runs in both collection modes. Each is sized
to stay under 1.5 CPU seconds in the default mode and 2 s with a collection at every safe point:

- at most 4 levels of nesting, except tests 13 and 14;
- at most 300 elements;
- memory limits of a few hundred KiB where the heap grows with the loop;
- step limits stated wherever 1,000,000 would not cover the test's own work.

Times given are bounds; the acceptance measures them.

Helpers that phase A adds besides the stand-ins:

- a lib-test host function `probe`, which stores `used()` in a test-local cell and answers
  `undefined`, made with `new_host_function` (`host/mod.rs:693-699`);
- a lib-test reader of `agent.kept.capacity()`.

1. `heap::accounting::tests::a_charge_held_across_a_call_is_given_back_on_drop`. 0.01 s.
   - `hold(100)` raises `used()` by 100 [0]. Its drop lowers it by 100 at once, and `check` sees
     the credit before any fold.
   - `Charge::release(40)` of a held 100 lowers `used()` by 40 [0].
   - A `ChargedVec` grown three times by `reserve_vec` reads capacity 16, 32, 64, with `used()` up
     by its capacity in bytes [unchanged].
   - With room for the need but not for the doubling, `reserve_vec` grows to the exact need and
     answers `Ok` [`Ok`, with the doubled capacity].
   - A refused `hold` charges nothing.
2. `builtins::array::tests::join_reads_its_separator_in_place_across_a_call`. 0.05 s / 0.2 s.
   - `s` has 65,536 units, under `steps_per_entry` 4,000,000. `var d = 0; var o = { toString:
     function () { return ++d < 4 ? [o, 1].join(s) : "e"; } }; [o, 1].join(s).length` answers
     the base's Number.
   - `copies::longest()` reads 0 [65,536], and `uncovered_at_entry()` reads 0 [at least 3 ×
     131,072].
   - `last_entry_steps()` equals the base's constant.
3. `builtins::array::mutation::tests::sort_reads_a_converted_string_in_place_across_a_call`. 0.05
   s / 0.2 s.
   - `[o, p].sort()`, where `o.toString` answers `s` (65,536 units) and `p.toString` sorts a new
     `[o, p]` until 4 levels deep, under `steps_per_entry` 4,000,000: the base's answer and steps.
   - `longest()` reads 0 [65,536], and `uncovered_at_entry()` reads 0 [at least 131,072].
4. `builtins::string::access::tests::a_builder_is_charged_across_a_call_of_to_string_and_value_of`.
   0.05 s / 0.2 s.
   - `s.concat(o)`, with `o.toString` calling `s.concat(o)` until 4 levels, `s` of 65,536 units,
     room 3 × 131,072, under `steps_per_entry` 4,000,000: `Terminated(MemoryBudget)`
     [`Normal`].
   - `String.fromCharCode(…64 × 65…, v)`, with `v.valueOf` calling the same until 4 levels:
     `uncovered_at_entry()` 0 [at least 128].
5. `builtins::json::tests::stringify_holds_charged_buffers_across_a_call_of_to_json`. 0.05 s /
   0.2 s.
   - `JSON.stringify([s, o])`, with `o.toJSON` calling it again until 4 levels, `s` of 65,536
     units, room 3 × 131,072, under `steps_per_entry` 4,000,000: `Terminated(MemoryBudget)`
     [`Normal`].
   - The same with a replacer list of 64 Strings and a `space` of 4: `uncovered_at_entry()` 0
     [more than 0: frames, indent, property list].
6. `builtins::string::replace::tests::replace_holds_charged_copies_across_a_call_of_its_replacer`.
   0.05 s / 0.2 s.
   - `s.replace("x", f)`, with `f` calling `s.replace("x", f)` until 4 levels, `s` of 65,536 units,
     under `steps_per_entry` 4,000,000: `high_water()` rises by at least 4 × 131,072 over the
     reading before [less].
   - With room 3 × 131,072: `Terminated(MemoryBudget)` [`Normal`].
   - `uncovered_at_entry()` 0 [more than 0], and steps equal to the base's.
7. `interp::dispatch::tests::call_and_new_hold_one_charged_copy_of_their_arguments_across_a_call`.
   0.05 s / 0.3 s.
   - A host function `h`, made a constructor, is run by `h(a1, …, a49, probe())` and by `new
     h(a1, …, a49, probe())`: 50 arguments, `probe` evaluated last.
   - The operand stack grows to 64 entries at the 33rd push, before `probe` runs. After `probe`
     returns it holds 52 entries for `Call` and 51 for `New`, so it does not grow again before
     `h` is entered.
   - Inside `h`, `used()` reads at least 2 × 50 × `size_of::<Value>()` over `probe`'s reading:
     the instruction's copy and the activation's [50 × `size_of::<Value>()`, the activation's
     only].
   - `INSTRUCTION_ARGUMENTS_COPIED` reads 50 per call [102 for `Call`, 101 for `New`].
   - With `h` calling back through `cx.call` into a function that calls `h` again, 4 levels:
     `uncovered_at_entry()` 0 [at least 102 × `size_of::<Value>()`].
   - An ECMAScript callee with 50 arguments: `INSTRUCTION_ARGUMENTS_COPIED` 50 [102], and `used()`
     inside it, read by `probe` as its first statement, equal to the base's.
   - Steps equal the base's.
8. `object::builtin_function::tests::holders_give_the_kept_stack_back_its_room_across_a_call`.
   0.1 s / 0.5 s in total.
   - Each holder runs in a fresh agent: first a warm-up call with 1 element, then the measured
     call. The test first asserts that the kept capacity after the warm-up is below the measured
     call's element count, so the measured call has to grow the stack.
   - The holders and their measured calls:
     - `f.apply(null, a)`, `a` of 300 elements;
     - `JSON.parse(t, r)` of a text of 300 numbers with a reviver;
     - `Function(…300 names…, "")`;
     - `[[[…]]].flat(Infinity)`, 300 deep.
   - `kept.capacity()` after the measured call equals its reading after the warm-up [at least
     300].
   - `used()` after the call equals `used()` before, on a normal end and on a `Throw` (an element
     getter that throws at element 150, or the 150th level for `flat`). This part is a guard:
     the base credits these per-element charges on every path, so it is green on phase A.
   - The guard part also covers `JSON.stringify({}, l)` with `l` of 300 distinct Strings (about
     45,000 steps for the list's duplicate check, `json/mod.rs:262-267`). The base already ends
     its room on every path (`:174-178`), so both its readings are green on phase A.
9. `object::builtin_function::tests::a_kept_room_past_the_budget_ends_across_a_call`. 0.01 s /
   0.05 s.
   - Inside `with_kept_room`, the body calls `reserve_kept(room, 200)`, which fits, then
     `reserve_kept(room, 100,000)` under a room of 64 × 1,024 bytes.
   - The answer is `Terminated(MemoryBudget)`.
   - After `with_kept_room` returns, `used()` and `kept.capacity()` equal their readings before
     it [`used()` + 200 × `size_of::<Value>()` or more, and a larger capacity: phase A's
     `with_kept_room` does not end the room].
10. `host::jobs::tests::the_queue_is_charged_by_its_capacity_across_a_call`. 0.02 s / 0.1 s.
    - The host queues 100 host microtasks: `used()` rises by `microtasks.capacity()` ×
      `size_of::<Entry<H>>()` [0].
    - A checkpoint whose microtasks call a script function (an entry into script with the queue
      alive): `uncovered_at_entry()` 0 [more than 0].
    - After the checkpoint `used()` reads the reading before + 64 × `size_of::<Entry<H>>()`
      [the reading before]. After 100 more and a `clear_microtasks`, it reads the reading
      before.
11. `host::jobs::tests::a_promise_job_past_the_queue_room_ends_across_a_call`. 0.02 s / 0.1 s.
    - A script `p = Promise.resolve(); function f() {}` runs, then `p.then(f)` three times, so
      the queue holds 3 entries in a capacity of 4.
    - The 4th `p.then(f)` does not grow the queue. The rise of `used()` over it is R.
    - The limit is set to `used()` + R + `size_of::<Entry<H>>()`, short of the 4 entries' growth
      the 5th needs.
    - The 5th `p.then(f)` answers `Terminated(MemoryBudget)` [`Normal`].
    - `used()` after reads `used()` before + R − `promise_job_bytes::<H>()` [before + R]. The
      objects `then` made before the enqueue stay charged; the entry's bytes are credited, and
      no capacity is charged.
    - `pending_microtasks()` reads 4 [5].
    - Both entries are scripts and no checkpoint runs between them: the host runs one only
      when it calls `perform_microtask_checkpoint`. The only callers of `Agent::checkpoint` are
      `host/agent.rs:749` and `host/mod.rs:1063`.
12. `host::jobs::tests::a_host_microtask_past_the_queue_room_is_refused_across_a_call`. 0.01 s /
    0.05 s.
    - The host queues 4 host microtasks (capacity 4), then the limit is set to `used()` +
      `size_of::<Entry<H>>()`.
    - The 5th `queue_microtask` answers `Err(HostError::MemoryBudget)` [`Ok(())`].
    - `pending_microtasks()` reads 4 [5], and `used()` is unchanged.
13. `object::string::tests::no_buffer_is_uncovered_at_an_entry_across_a_call`. This is the lib
    twin of HOST-215, under its limits: `memory_bytes` 64 MiB, `string_length` 2^24,
    `reentry_depth` 256, 10,000,000 steps. 0.1 s / 0.3 s.
    - HOST-215's script answers `Terminated(MemoryBudget)` [`Terminated(StepBudget)`; not run,
      measured by the unit], with `high_water()` at most `memory_bytes`, and
      `uncovered_at_entry()` 0 [at least 2,097,152].
    - With no host charge, and `s` of 65,536 units, `var o = { toString: function () { return
      [o, 1].join(s); } }; [o, 1].join(s)` answers `Throw` of a `RangeError` on the base and
      after (not run, measured by the unit). `uncovered_at_entry()` reads 0 [at least 255 ×
      131,072].
    - This half has no red through the public API, so it lives only here.
14. Integration test, `crates/swell/tests/budgets.rs`:
    `host_215_a_buffer_held_across_a_call_into_script_is_charged`. 0.1 s / 0.3 s.
    - It runs HOST-215 as written, and reads `heap_bytes()` before the entry, after it, and after
      the `collect()`.
    - On the phase-A tree the script answers `Terminated(StepBudget)` [red], since only-checked
      builders let it reach the step budget near the tenth level (9 × 1,048,576 steps of
      appends). Not run, measured by the unit.

The guard: `object::string::tests::the_step_totals_of_the_sites_held_across_a_call_are_unchanged`.
0.1 s / 0.5 s.

- Each site is entered with its buffer alive (a `toString`, `valueOf`, getter, replacer, reviver,
  mapper or job). It also covers the readers' callers that switch: concatenation, `Error`
  `toString`, `Number(s)`, `padStart`, `encodeURIComponent` and `decodeURI` of valid Strings, and
  `RegExp` with short flags.
- Each runs on Strings of 0, 1, 33 and 300 units. `last_entry_steps()` and the answer equal the
  constants the test records from the base: green on phase A and after.
- `uncovered_at_entry()` after each site, on the phase-A tree and after phase C:

  | sites | phase A | after |
  |---|---|---|
  | 1 to 13, 15 to 17 (with a non-empty String) | more than 0 | 0 |
  | 14 | 0: the builder is empty across the replacer; the copies of site 5 are covered by test 6 | 0 |
  | 18 | more than 0 (the queue reports in phase A) | 0 |
  | 19 to 23 | more than 0 | 0 |
  | 24 to 27 | not seen by the instrument; covered by tests 8 and 9 | not seen |
  | the switched callers | 0: nothing alive across a call | 0 |

Changed tests:

- `builtins::string::tests::builder_past_the_memory_budget_terminates`
  (`builtins/string/mod.rs:756-774`) asserts today that `used()` does not move while a builder of
  1,000 units lives. After the change it asserts `used()` + 2,000 while the builder lives, and
  `used()` again once it drops.
- No step pin moves. The search was `grep -rln 'last_entry_steps' crates/swell/tests
  crates/swell/src` (59 files) and `grep -rn 'last_entry_steps' crates/swell/tests crates/swell/src
  | grep 'assert_eq\|=='` (28 lines).
- Readings that may move are found with these searches over `crates/swell/tests crates/swell/src`
  (`grep -rnF '<text>' … | wc -l`):

  | text | lines | files |
  |---|---|---|
  | `memory_bytes = ` | 86 | 61 |
  | `set_limit(` | 36 | 12 |
  | `charge_host_bytes` | 33 | 7 |
  | `high_water()` | 16 | 3 |

  Under `crates/swell/tests` they name 45 test modules, `support` left out (`grep -rlE
  'memory_bytes = |set_limit\(|charge_host_bytes|high_water\(\)' crates/swell/tests | grep -v
  support | wc -l`). `tests/jobs.rs` is one of them: it sizes a room from `heap_bytes()` at
  `:735-745`. The 47 pins of `heap_bytes()`/`used()` come from `grep -rn
  'assert_eq!(.*heap_bytes\|heap_bytes().*==\|assert_eq!(.*used()' crates/swell/tests
  crates/swell/src`.
- What the unit does with a test whose reading moves:
  - A test whose room was sized for the uncharged state gets a wider room in the test, and the
    report gives its old and new number.
  - A pin of `heap_bytes()` that moves by exactly the queue's retained capacity (point 7) is
    written with that term, also named in the report.
  - A test whose result changes for another reason is a defect of the change. It is not edited.
- No document under `docs/`, `website/` or `conformance/` pins a count the change moves. The
  search was `grep -rln 'string_units\|peek_values\|StringBuilder' docs website/docs website/src
  conformance`, and its one hit is `docs/plan/second-batch.md:285`.

The new tests total at most 5 CPU seconds over both modes. A test over its bound is made smaller,
since 2 levels show each mechanism.

test262 cannot change, for two reasons:

- No step total changes.
- The runner's budgets are far above what a case's buffers add: `crates/test262/src/run.rs:69-74`
  sets 2^30 steps, 512 MiB, `reentry_depth` 1,000 and `string_length` 2^30.

The coordinator's run on the join checks it.

## Acceptance

Each command is one job of the build pool. A job over 10 minutes names its time, at most 30
minutes. A test is killed at 60 s, and the gate's test steps fail over 120 s.

- `slot cargo nextest run --locked -p swell --profile default --lib`: `Summary … N tests run: N
  passed`, with no `failed`. Then the same in collect mode: `slot cargo nextest run --locked -p
  swell --features collect-every-safe-point --config .cargo/collect-profile.toml --cargo-profile
  collect --profile default --lib`.
- The modules that can move, in both modes. Let `m=$(grep -rlE 'memory_bytes = |set_limit\(|
  charge_host_bytes|high_water\(\)' crates/swell/tests | grep -v support | sed
  's|.*/||; s|\.rs$||' | paste -sd'|')`; this is a read, run without `slot`. Then:
  - `slot cargo nextest run --locked -p swell --profile default --test main -E
    "test(/^($m)::/)"`;
  - the same with `--features collect-every-safe-point --config .cargo/collect-profile.toml
    --cargo-profile collect`;
  - each prints `Summary … N tests run: N passed`. This is the form of
    `own-keys-charged-before-the-list-exists`.
- The new tests: `slot cargo nextest run --locked -p swell --profile default -E
  'test(/across_a_call/)'` and the same in collect mode. Each runs 15 tests (lib tests 1 to 13,
  the guard, and HOST-215), then `15 passed`.
- In the last round, once, the whole of `swell` in both modes:
  - `SLOT_TIMEOUT=30m cargo nextest run --locked -p swell --profile default`;
  - `SLOT_TIMEOUT=30m cargo nextest run --locked -p swell --features collect-every-safe-point
    --config .cargo/collect-profile.toml --cargo-profile collect --profile default`;
  - `cargo` is the pool's shim here. Each prints a `Summary` with no `failed`.
- Times, in the form of `AGENTS.md` ("A test stays under 1.5 CPU seconds"). One job per binary
  and mode: `slot sh -c 'b=$(cargo test --locked -p swell --lib --no-run 2>&1 | sed -n
  "s/.*Executable .*(\(.*\))$/\1/p"); for t in <the names>; do sh -c "\"$b\" --exact $t
  --test-threads 1 >/dev/null; times"; done'`, the same for `--test main`, and both for the
  collect build.
  - Each test's user plus system is under 1.5 s in the default mode and under 2 s in the collect
    mode.
  - The sum over both modes is under 5 s.
  - The report gives each time.
- `slot cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0, no `warning:`
  line.
- Greps after the change:
  - `grep -rnw 'string_units' crates/swell/src | wc -l` reads 0 [26 on the base].
  - `grep -rnw 'peek_values' crates/swell/src | wc -l` reads 0 [3].
  - `grep -rn 'uncharged()' crates/swell/src` prints nothing.
- `slot --timeout 30m task host-requirements-check`, before and after, quoted whole in the
  report. On `76aa4fc` the file has 214 lines, 145 `test` and 69 `pending` (`wc -l`, `awk '{print
  $2}' conformance/host/requirements.txt | sort | uniq -c`). After: one line more and one `test`
  more, with `pending` unchanged. On this base that is `host-requirements: 215 requirement(s), 215
  line(s): 146 test, 69 pending, 0 omitted; …`.
- `task host-surface-check` compiles nothing, so it runs without `slot`. Run it before and after
  and quote both whole.
  - After, run it with `HOST_SURFACE_CHANGELOG=.out/CHANGELOG.md`: a copy of `CHANGELOG.md` in
    the ignored `.out/`, with `HOST-215` and `HostError::MemoryBudget` under `### Host
    interface` of `## [Unreleased]`.
  - Expected: `host-surface: <b+2> change(s) since 0.8.0, each named`, where b is the count
    before.
  - The report gives the two changelog lines it used, for the coordinator.
- The nested-join probe is run once on the phase-A tree and once after phase C.
  - It is a test `budgets::probe_nested_join_to_the_reentry_depth`, kept in the unit's scratch
    directory and copied into `crates/swell/tests/budgets.rs` only for the two runs, and never
    committed.
  - The test builds a separator of 2^24 units and nests `[o, 1].join(sep)` through `o.toString`
    to `reentry_depth` 256, under `memory_bytes` 64 MiB, `string_length` 2^24 and 10,000,000
    steps. It prints the completion and `heap_bytes()`, and asserts only that `heap_bytes()` is
    at most 64 MiB.
  - The binary is found and run in one job, in the `--no-run` form of `AGENTS.md`, "Commands and
    the gate", under a cap of the process's memory: `slot sh -c 'b=$(cargo test --locked -p swell
    --test main --no-run 2>&1 | sed -n "s/.*Executable .*(\(.*\))$/\1/p"); ulimit -v 4194304;
    "$b" --exact budgets::probe_nested_join_to_the_reentry_depth --test-threads 1 --nocapture'`.
  - The report gives, for each run, the completion: `Terminated(MemoryBudget)`,
    `Terminated(StepBudget)`, a throw with its kind, or a failed allocation of the process (exit
    status and last line). It also gives `heap_bytes()` at the end.
  - After phase C it passes when the run exits 0, prints one completion of that list, and prints
    `heap_bytes()` at most 64 MiB. The completion expected, not run, is `Throw` of a
    `RangeError`. On the phase-A tree the 32 MiB copy per level is expected to exhaust the 4 GiB
    cap, near level 128.

Never `slot task check`, `slot task test-collect`, `slot task test`, nor any test262 task.

Rules this unit keeps:

- Each test above is written in phase B and seen red by its bracketed assertion on the phase-A
  tree before phase C. The parts named as guards are green there.
- Each test runs in a process of its own under cargo-nextest, so the `copies` tallies are
  thread-local and reset by each test.
- A command that compiles or tests carries `slot`, also inside a script. A script that only
  measures or waits starts without it.
- No Python, no third-party crate, no `unsafe`, no `#[ignore]`.
- Determinism: the same script under the same budgets ends the same way at the same count on
  every run.

## Not in this story

Sites that are not alive across a call. Each is dropped before any call into script and bounded as
named; all are charged bytes by the new readers anyway, with their steps unchanged.

- The String iterator: 2 units (`builtins/iterator.rs:360`).
- The `space` of `JSON.stringify`: 10 units (`builtins/json/mod.rs:328`).
- The ranges of QuoteJSONString: 256 units, `QUOTE_CHUNK` (`builtins/json/mod.rs:516`).
- The form of `normalize`: 4 units (`builtins/string/transform.rs:610`).
- The parser stack and the String tokens of `JSON.parse` (`builtins/json/parse.rs:390`, `:435`,
  `:661`, `:720`, `:733`): the parse ends before any reviver, bounded by the charged text.
- The other 28 callers of `read_units` and the 8 of `charged_units`: steps first, bounded by
  their String.
- The 22 builder sites that hold nothing across a call.
- `new_environment_of` (`env/mod.rs:353`): stored at once.
- The strings that `Cx` reads hand to the host (`host/mod.rs:239`, `:328`): the host's.

Alive across a call, plain, and bounded:

- The units of a Number, a Boolean or `null` in the sort's compare
  (`builtins/array/mutation.rs:789-798`): at most 25 units, alive across ToString of the other
  operand, one per level, so at most 256 × 50 bytes at `reentry_depth` 256.
- `on_stack` of `JSON.stringify`: charged per entry in the frame's charge, as the design text
  states. Whether four keys' bytes cover the B-tree's nodes is not measured here.
- The three buffers bounded by a depth limit, named in the design text with their sizes: the
  activations, the frame stack's spare capacity, and `keep` outside a room. `keep` was read for
  its 9 calls inside loops (all charged or in a room after this story), not for all 293.

A buffer that is a plain `Vec` is not seen by the instrument. The check that names every
remaining growth call with the charge or limit that bounds it is
`story:buffer-growth-outside-the-charged-list-is-listed`, built after this story.

Moved out:

- `Math.max`, `Math.min` and `Math.hypot` belong to `story:math-constants-and-exact-functions`.
  It is built now and, in its tree (`builtins/math.rs:237-271`, uncommitted), its
  `Agent::coerce_all` charges 8 bytes per argument before the list is made and credits them after,
  whatever the answer. That list is exact (`try_reserve_exact(args.len())`) and is named in the
  design text.
- The hand-charged lists (`KeyList`, `SourceCopy`, the activation copies, the spread, rest, bound
  and `apply` lists, `param_strings`, the positions of `replaceAll`, the buffers of `normalize`,
  the captures) stay on their own charges. They charge their exact length and credit already.

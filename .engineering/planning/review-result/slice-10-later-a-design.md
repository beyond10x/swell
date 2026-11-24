---
format: aep.planning-md/3
id: review-result:slice-10-later-a-design
kind: review-result
status: active
title: 'Slice 10, later stories, set A, the design critic: the eight stories of orders 4 and 5'
relations:
- reviews: story:promise-statics-finally-and-aggregate-error
- reviews: story:promise-rejection-tracker-and-promise-reads
- reviews: story:async-methods-in-object-literals
- reviews: story:async-function-constructor
- reviews: story:for-await-and-async-from-sync-iterator
- reviews: story:async-generator-syntax-and-early-errors
- reviews: story:async-generators-and-their-request-queue
- reviews: story:ess-target-checkpoint-and-promise-hooks
revision: 1
---
needs-revision
.engineering/planning/story/promise-statics-finally-and-aggregate-error.md — with option A, `ErrorKind::ALL` grows to 8, which breaks `every_engine_error_records_its_kind`, and the Writes line omits `crates/swell/tests/errors.rs` (the test asserts the seven-entry `cases` array equals `ErrorKind::ALL`) — crates/swell/tests/errors.rs:548-556, .engineering/planning/story/promise-statics-finally-and-aggregate-error.md:78
.engineering/planning/story/promise-statics-finally-and-aggregate-error.md — the acceptance says `host-surface-check` prints the `ErrorKind::AggregateError` line "and no other", but the `ALL` line (`[ErrorKind; 7]`) moves too and the changelog text must name it — conformance/host/surface.txt:2, .engineering/planning/story/promise-statics-finally-and-aggregate-error.md:76,82
.engineering/planning/story/promise-statics-finally-and-aggregate-error.md — it adds calls to user code (iterator `next`, `resolve`, `then`) but no test reaches `reentry_depth` or `frame_depth` through them, and no script runs `Promise.all` over an iterator that never ends to show it ends at a budget — .engineering/planning/story/promise-statics-finally-and-aggregate-error.md:67,69
.engineering/planning/story/for-await-and-async-from-sync-iterator.md — the thirteenth well-known symbol breaks two existing tests the Writes line and acceptance leave out: `the_well_known_symbols_are_twelve_and_the_same_in_every_realm` (asserts `Symbol.asyncIterator === undefined`, NAMES of 12) and the Symbol-values list of 12 in `builtins_object_enumeration.rs`; add both files and both modules to Writes and acceptance, and add `builtins_symbol.rs` line 151 to the Context search — crates/swell/tests/builtins_symbol.rs:151-193, crates/swell/tests/builtins_object_enumeration.rs:633-646, .engineering/planning/story/for-await-and-async-from-sync-iterator.md:68,105,109
.engineering/planning/story/for-await-and-async-from-sync-iterator.md — it adds up to three instructions but states nothing on the run loop (the work in a function of its own, nothing added to the frame of `Agent::execute`, `STACK_BYTES_PER_REENTRY` and the stack tests unchanged), which `async-functions-and-await` states — .engineering/planning/story/for-await-and-async-from-sync-iterator.md:88,92, .engineering/planning/story/async-functions-and-await.md:152
.engineering/planning/story/for-await-and-async-from-sync-iterator.md — the async-from-sync iterator and its builtin closures get no test that shows their credit, and no test reaches `reentry_depth` or `frame_depth` through `%AsyncFromSyncIteratorPrototype%.next` calling the sync iterator's `next` — .engineering/planning/story/for-await-and-async-from-sync-iterator.md:92,94
.engineering/planning/story/async-generators-and-their-request-queue.md — it adds four instructions and states nothing on the run loop (the work in a function of its own, nothing added to the frame of `Agent::execute`, `STACK_BYTES_PER_REENTRY` and the stack tests unchanged) — .engineering/planning/story/async-generators-and-their-request-queue.md:101,105
.engineering/planning/story/async-generators-and-their-request-queue.md — no test reaches `reentry_depth` or `frame_depth` through the resumption by `next`, `return` and `throw` (the first wave has `a_resumption_re_enters_on_the_promised_stack`), and no script that queues requests without end is shown to end at a budget — .engineering/planning/story/async-generators-and-their-request-queue.md:105,107, .engineering/planning/story/async-functions-and-await.md:154
.engineering/planning/story/async-generators-and-their-request-queue.md — an `await` in an async generator must go through the `await` instruction and the two functions of `story:async-functions-and-await`, which hold an `AsyncActivation` and hand the promise of the call to the caller on the first suspension, so `interp/async_function.rs` and `object/async_function.rs` are written, yet the Writes line says "Not written" and leaves it to "the smallest change" — .engineering/planning/story/async-functions-and-await.md:148, .engineering/planning/story/async-generators-and-their-request-queue.md:103,116
.engineering/planning/story/async-generators-and-their-request-queue.md — 8 cases of its list call `Promise.all` (for example `request-queue-order.js` line 47 and `this-val-not-object.js` line 98), so acceptance (a) cannot pass until `story:promise-statics-finally-and-aggregate-error` lands, and no depends_on edge records it; add the edge or move the 8 cases to the close — docs/plan/slice-10/story-ag-functions.cases.txt, aep plan artifact graph
.engineering/planning/story/async-generators-and-their-request-queue.md — it records the held refusal in the function of YieldExpression that `story:yield-star-delegation` rewrites and edits the same `admitted` arm and the same `bytecode.rs`, `disasm.rs` and `dispatch.rs` tables, with no edge to it (`story:yield-star-in-async-generators` has one); add depends_on `story:yield-star-delegation` — .engineering/planning/story/async-generators-and-their-request-queue.md:99,116, aep plan artifact graph
.engineering/planning/story/promise-rejection-tracker-and-promise-reads.md — its acceptance prints `0` for `grep -c 'pending slice 10$'` only after `story:async-functions-and-await` moves HOST-194 and HOST-195, and the only edge is to `story:promise-constructor-then-and-jobs`; Context also says the count is `4` after that story, which that story gives as `6`; add depends_on `story:async-functions-and-await` and fix the sentence — .engineering/planning/story/promise-rejection-tracker-and-promise-reads.md:34,71, .engineering/planning/story/promise-constructor-then-and-jobs.md:113, .engineering/planning/story/async-functions-and-await.md:162
.engineering/planning/story/ess-target-checkpoint-and-promise-hooks.md — it says `story:job-queue-and-microtask-checkpoint` "names the gap of the lifecycle and leaves it to this story", but that story holds no such sentence (its only mention of the `Job` lifecycle is the declared `Enqueued` → `Ran`); drop the claim or add the sentence to the first-wave story — .engineering/planning/story/ess-target-checkpoint-and-promise-hooks.md:54, .engineering/planning/story/job-queue-and-microtask-checkpoint.md:76

What I read: the 8 stories of set A whole, the epic, the 4 first-wave stories, and the Writes lines of `generator-function-constructor`. I ran `aep plan artifact relations`, `graph` and `validate` (216 artifacts, valid). I read the design sections the brief names, `docs/plan/unit-invariants.md`, `AGENTS.md`, the test262 lists and checked their text against the pinned checkout, and the code at the places the Writes lines name. I walked about 70 declared edges, including those to the open slice 7 stories and the set B stories; there is no cycle and no serialising chain. I traced 14 acceptance reads to a producer: 11 have an edge and 3 do not (the tracker's HOST-194 and HOST-195 lines, the `Promise.all` cases in the async generator list, and the `yield*` hunks).

What I could not establish:
- Whether `ops/promise.rs` of the first wave leaves the operations the statics call as `pub(crate)`. The file is unbuilt, and the statics acceptance forbids writing it while the tracker story writes it in the same order.
- Pairs of one order across the sets: set B stories all have a path to `async-generators-and-their-request-queue`; I did not check them against `promise-statics-finally-and-aggregate-error`, `promise-rejection-tracker-and-promise-reads` or `ess-target-checkpoint-and-promise-hooks`, and found no shared file for them.
- The `(b)` acceptance lines of `async-generator-syntax-and-early-errors`, `for-await-and-async-from-sync-iterator` and `async-generators-and-their-request-queue` print `0` and state no base value. The acceptance critic owns that.
- Out of my lane: the hunk-level merge order of `NESTING_KINDS`, `BuiltinId`, `Intrinsic` and `builtins/mod.rs` between the stories of order 4. They are declared "in hunks of their own", and the parallel-safety critic owns whether they merge.

```findings
- file: .engineering/planning/story/promise-statics-finally-and-aggregate-error.md
  line: 78
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "with option A, ErrorKind::ALL grows to 8 and breaks every_engine_error_records_its_kind (crates/swell/tests/errors.rs:548-556, which asserts its seven-entry cases array equals ErrorKind::ALL), and the Writes line omits that test file"
- file: .engineering/planning/story/promise-statics-finally-and-aggregate-error.md
  line: 82
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says host-surface-check prints the ErrorKind::AggregateError line and no other, but the ALL line of conformance/host/surface.txt (line 2, [ErrorKind; 7]) moves too, and the changelog text of Requirements must name it"
- file: .engineering/planning/story/promise-statics-finally-and-aggregate-error.md
  line: 69
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story adds calls to user code (iterator next, resolve, then) but no test reaches reentry_depth or frame_depth through them, and no script runs Promise.all over an iterator that never ends to show it ends at a budget"
- file: .engineering/planning/story/for-await-and-async-from-sync-iterator.md
  line: 105
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the thirteenth well-known symbol breaks builtins_symbol.rs::the_well_known_symbols_are_twelve_and_the_same_in_every_realm (asserts Symbol.asyncIterator === undefined, line 192-193) and the 12-name Symbol-values list of builtins_object_enumeration.rs:633-646, and the Writes line, the Context search and the acceptance name neither file nor module"
- file: .engineering/planning/story/for-await-and-async-from-sync-iterator.md
  line: 88
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story adds up to three instructions and states nothing on the run loop (work in a function of its own, nothing added to the frame of Agent::execute, STACK_BYTES_PER_REENTRY and the stack tests unchanged or the measured change), as async-functions-and-await line 152 does"
- file: .engineering/planning/story/for-await-and-async-from-sync-iterator.md
  line: 94
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no test shows the credit of the async-from-sync iterator and its builtin closures, and no test reaches reentry_depth or frame_depth through AsyncFromSyncIteratorPrototype.next calling the sync iterator's next"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 101
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story adds four instructions and states nothing on the run loop (work in a function of its own, nothing added to the frame of Agent::execute, STACK_BYTES_PER_REENTRY and the stack tests unchanged or the measured change)"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 107
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no test reaches reentry_depth or frame_depth through the resumption by next, return and throw (the first wave has a_resumption_re_enters_on_the_promised_stack), and no script that queues requests without end is shown to end at a budget"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 116
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "an await in an async generator runs through the await instruction and the two functions that hold an AsyncActivation and hand the promise of the call to the caller (async-functions-and-await.md:148), so interp/async_function.rs and object/async_function.rs are written, yet the Writes line says Not written and leaves it to the smallest change"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 81
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "8 cases of its list call Promise.all (request-queue-order.js:47, this-val-not-object.js:98 and six more), so acceptance (a) cannot pass until promise-statics-finally-and-aggregate-error lands and no depends_on edge records it; add the edge or move the 8 cases to the close"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 116
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it records the held refusal in the function of YieldExpression that yield-star-delegation rewrites and edits the same admitted arm and the same bytecode.rs, disasm.rs and dispatch.rs tables, with no edge to that story; add depends_on story:yield-star-delegation as yield-star-in-async-generators has"
- file: .engineering/planning/story/promise-rejection-tracker-and-promise-reads.md
  line: 71
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance prints 0 for grep -c 'pending slice 10$' only after async-functions-and-await moves HOST-194 and HOST-195, and the only edge is to promise-constructor-then-and-jobs; add depends_on story:async-functions-and-await and correct Context line 34, which says 4 after promise-constructor-then-and-jobs where that story states 6"
- file: .engineering/planning/story/ess-target-checkpoint-and-promise-hooks.md
  line: 54
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it says job-queue-and-microtask-checkpoint names the gap of the swell.host.Job lifecycle and leaves it to this story, but that story holds no such sentence (job-queue-and-microtask-checkpoint.md:76 only cites the declared Enqueued to Ran); drop the claim or add the sentence there"
```

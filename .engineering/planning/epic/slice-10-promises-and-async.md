---
format: aep.planning-md/3
id: epic:slice-10-promises-and-async
kind: epic
status: active
title: 'Slice 10, promises-and-async: jobs, Promise, async functions'
summary: 'The job queue, Promise, async functions and generators, for await: 3,879 cases and 7,499 runs.'
relations:
- depends_on: epic:slice-07-generators
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T18:19:56Z", actor: "human:operator", revision: 4, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T18:19:57Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
---
## Context

Slice 10 of 19, `promises-and-async` (`docs/design/engine.md`, section 12). It is delivered after
slice 7 and before slices 4, 9 and 8 (section 12, "The order of delivery": 1, 2, 3, 6, 5, 7, 10,
4, 9, 8), and it needs slice 7: "10 needs 7's suspended frames", because `await` inside an
`async` function or an async generator saves the frame as `yield` does. That frame exists since
the second story of slice 7 (`crates/swell/src/interp/suspend.rs`, which names no generator). The
slice builds the job queue, `Promise`, `async` functions, async generators and `for await`.
Classes come after it, so async members of classes are counted in slice 8.

It is ordered by the first consumer: what remains of the two scripts of its test harness is one
`async` function with two `await` on promises of the host, and the host that embeds the engine
drains the queue after each script. Governing decisions (`docs/design/engine.md`, decision N): 3
("Suspended frames", with the items on an `async` function's activation), 4 ("Queued jobs"), 9
(jobs, the host's hooks, `perform_microtask_checkpoint`), 11 (the runner's `async` handling), 13;
and decision 11 of `docs/design/host-interface.md`.

## Outcome

- The promise job queue kept in the agent, because jobs hold engine values; the host is told of
  each job through `HostEnqueuePromiseJob` (so HTML can attach incumbent settings through
  `HostMakeJobCallback` and `HostCallJobCallback`) and decides when to drain it (decision 9).
- `perform_microtask_checkpoint`: one outermost entry that runs the entries of the queue until it
  is empty under one step budget, one step per entry taken off, at most `steps_per_entry` entries
  a call; a termination ends the checkpoint and what is left stays queued in its order; the host
  can empty the queue with one call (decision 9; `HOST-193`, `HOST-196` to `HOST-200`).
- Host microtasks, queued with a realm (`HOST-192`).
- `HostPromiseRejectionTracker` and the event `PromiseRejectionTracked` of the ESS model
  (decisions 9 and 10).
- `Promise` with its feature set (`Promise.allSettled`, `Promise.any`, `Promise.prototype.finally`,
  `promise-with-resolvers`, `promise-try`), `AggregateError`, `Array.fromAsync`.
- `async` functions and `async` arrow functions on a suspended frame held by the function's
  activation, an internal kind of object; `await` through the host's job-callback hooks as a
  `then` goes (`HOST-194`, `HOST-195`); async methods of object literals; async generators with
  the request queue on the object (27.6.3), `for await`; `AsyncFunction`,
  `AsyncGeneratorFunction`, `AsyncGeneratorPrototype`, `AsyncFromSyncIteratorPrototype`,
  `AsyncIteratorPrototype`, `Symbol.asyncIterator` (section 12, item 10; the `dir` and `feature`
  lines below).
- The runner's `async` handling (decision 11): after the script it drains the job queue; the case
  passes when `print` received `Test262:AsyncTestComplete`, and fails on
  `Test262:AsyncTestFailure:` or when the queue is empty without either. No timer is needed.
- Generic and timeout jobs are handed to the host, which runs each with `run_job`, one entry
  each, when it chooses (decision 9); slice 14 (`Atomics.waitAsync`) is the first to use them.
- The 31 requirements of the host interface pending for slice 10 on `3648bf6`, as
  `grep -c 'pending slice 10$' conformance/host/requirements.txt` counts them, the first
  `HOST-46` and the last `HOST-208`, each get a test (`HOST-46`, `HOST-77` to `HOST-84`,
  `HOST-100`, `HOST-101`, `HOST-121`, `HOST-148`, `HOST-149`, `HOST-192` to `HOST-208`).

### What the first reading of the epic left open, and where it is settled

- Whether `perform_microtask_checkpoint` is bounded as a whole: decision 9, "Jobs", written for
  this slice: one step budget for the whole call and a bound on the entries (`HOST-193`).
- Which slice first uses `HostEnqueueGenericJob` and `HostEnqueueTimeoutJob`: slice 14.
- Which features with no `swell-slices` line fall inside this slice: none
  (`docs/design/engine.md`, the paragraph on the features without a line).

## Counts

- Slice 10 (`promises-and-async`): 3,879 cases and 7,499 runs (section 12 table;
  `conformance/test262/expected.txt`, `test262-slice 10 3879 7499 0 0 3879 13 3866 0` on the last
  full run before generators ran: 13 cases pass). The count is by the order of delivery: 2,743
  cases of async class members are slice 8's.
- The flag `async` is carried by 5,624 cases of the inventory, 2,748 of them in slice 10; none
  passes before the runner drains the queue and judges the `print` lines.
- The failing cases by the story that makes them pass are under `docs/plan/slice-10/` (the scans
  with their commands in `scan.awk` and `scan-later.awk`, a case list per story, the lists of the
  close).
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/built-ins/Promise/ 10`
  - `dir test/built-ins/AsyncFunction/ 10`
  - `dir test/built-ins/AsyncGeneratorFunction/ 10`
  - `dir test/built-ins/AsyncGeneratorPrototype/ 10`
  - `dir test/built-ins/AsyncFromSyncIteratorPrototype/ 10`
  - `dir test/built-ins/AsyncIteratorPrototype/ 10`
  - `dir test/built-ins/Array/fromAsync/ 10`
  - `dir test/language/expressions/async-arrow-function/ 10`
  - `dir test/language/expressions/async-function/ 10`
  - `dir test/language/expressions/async-generator/ 10`
  - `dir test/language/expressions/await/ 10`
  - `dir test/language/statements/async-function/ 10`
  - `dir test/language/statements/async-generator/ 10`
  - `dir test/language/statements/for-await-of/ 10`
  - `feature Promise 10`
  - `feature Promise.allSettled 10`
  - `feature Promise.any 10`
  - `feature Promise.prototype.finally 10`
  - `feature promise-with-resolvers 10`
  - `feature promise-try 10`
  - `feature AggregateError 10`
  - `feature async-functions 10`
  - `feature async-iteration 10`
  - `feature Array.fromAsync 10`
  - `feature Symbol.asyncIterator 10`
  - `flag async 10`

A count is measured again on the pinned test262 checkout before a story's acceptance uses it
(`AGENTS.md`, "How the work is run").

## Out of scope

- Async members of classes: slice 8, delivered after this slice.
- Modules, `import`, dynamic `import`, top-level `await`: slice 11.
- `Atomics.waitAsync` and its timeout job: slice 14.
- `AsyncDisposableStack` and `await using`: slice 15.
- `FinalizationRegistry` cleanup jobs: slice 16.
- `await-dictionary`: stage 3, excluded (the `exclude` line).
- The HTML event loop, task queues and the choice of when to drain the queue: the host's
  (decision 9).
- `Temporal` (slice 20), ECMA-402 (slice 21), other stage 3 features, concurrent agents.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`: a suspended async frame is data, no
  host thread (`AGENTS.md`, "Rules"; decision 3).
- No panics, no `unwrap`, `expect` or unchecked indexing (`AGENTS.md`, "Rules").
- Every loop charged to the step budget, every allocation, queued promise jobs included, to the
  memory budget; a new limit gets a test that reaches it (`AGENTS.md`, "Rules").
- A step stands for a bounded amount of work on every path the slice adds: an entry taken off the
  queue, a reaction queued, an `await` and its resumption, a request of an async generator.
- Nothing is added to the frame of the run loop; a resumption from a job is one re-entry.
- Jobs come from the host interface: the engine reads no clock and starts no timer (`AGENTS.md`,
  "Rules", "No ambient authority"; decision 9).
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 10 reads `test262-slice 10 <c> <r> 0 0 <c> <c> 0 0` with the cases and runs the design's table states for the slice after its close, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. An `async` case whose queue is empty without `Test262:AsyncTestComplete` or `Test262:AsyncTestFailure:` is a failed case, not a pass and not a hang (decision 11).
6. An `async` function with two `await` on promises a host function made, called from a script, runs to its end when the host settles the promises and drains the queue (the shape of the first consumer's function), and a test of the engine holds it.
7. A checkpoint whose jobs queue jobs without end ends; a running job, the jobs behind it and a suspended activation are each ended or kept as the design states at `Terminated(StepBudget)`, `Terminated(MemoryBudget)` and `Terminated(Interrupted)`; `reentry_depth` and `frame_depth` are each reached through jobs by a test.

## Stories

Decomposed on 2026-11-23 into seventeen stories. The four of the first wave were read by the four
plan critics (`review-result:slice-10-round-1-*`: 13 findings, none a blocker, all applied); the
design text they build on was written beside them (decisions 3, 4 and 9 and nine requirements of
the host interface, `HOST-192` to `HOST-200`, four of them asked for by the first embedder).
In the order of their dependencies:

1. `job-queue-and-microtask-checkpoint`, `async-function-syntax-and-early-errors`
2. `promise-constructor-then-and-jobs`
3. `async-functions-and-await` (with it the first consumer's function runs)
4. `promise-rejection-tracker-and-promise-reads`, `async-methods-in-object-literals`,
   `async-function-constructor`, `for-await-and-async-from-sync-iterator`,
   `async-generator-syntax-and-early-errors`
5. `promise-statics-finally-and-aggregate-error`, `ess-target-checkpoint-and-promise-hooks`
6. `async-generators-and-their-request-queue`
7. `yield-star-in-async-generators`, `async-generator-methods-in-object-literals`,
   `async-generator-function-constructor`, `array-from-async`
8. `slice-10-conformance-close` (after `slice-07-conformance-close`)

---
format: aep.planning-md/3
id: epic:slice-10-promises-and-async
kind: epic
status: draft
title: 'Slice 10, promises-and-async: jobs, Promise, async functions'
summary: 'The job queue, Promise, async functions and generators, for await: 6,573 cases and 12,879 runs.'
relations:
- depends_on: epic:slice-09-collections-proxy-reflect
revision: 1
---
## Context

Slice 10 of 19, `promises-and-async` (`docs/design/engine.md`, section 12). It needs slice 7: section 12
says "10 needs 7's suspended frames", because `await` inside an async function or async generator
saves the frame as `yield` does. It also follows slices 6 and 8 in the design's order (async class
members are counted here). It builds the job queue, `Promise`, async functions and generators and
`for await`. Governing decisions (`docs/design/engine.md`, decision N): 3 (async frames, request
queue), 9 (jobs, hooks, `perform_microtask_checkpoint`), 11 (the runner's `async` handling), 13.

## Outcome

- The promise job queue kept in the agent, because jobs hold engine values; the host is told of
  each job through `HostEnqueuePromiseJob` (so HTML can attach incumbent settings through
  `HostMakeJobCallback` and `HostCallJobCallback`) and decides when to drain it (decision 9).
- `perform_microtask_checkpoint`: runs promise jobs until the queue is empty, each with the budget
  (decision 9).
- `HostPromiseRejectionTracker` and the event `PromiseRejectionTracked` of the ESS model
  (decisions 9 and 10).
- `Promise` with its feature set (`Promise.allSettled`, `Promise.any`, `Promise.prototype.finally`,
  `promise-with-resolvers`, `promise-try`), `AggregateError`, `Array.fromAsync`.
- Async functions, async arrow functions, async generators with the request queue on the object
  (27.6.3), `for await`, `await`; `AsyncFunction`, `AsyncGeneratorFunction`,
  `AsyncGeneratorPrototype`, `AsyncFromSyncIteratorPrototype`, `AsyncIteratorPrototype`,
  `Symbol.asyncIterator` (section 12, item 10; the `dir` and `feature` lines below).
- The runner's `async` handling (decision 11): after the script it drains the job queue; the case
  passes when `print` received `Test262:AsyncTestComplete`, and fails on
  `Test262:AsyncTestFailure:` or when the queue is empty without either. No timer is needed.
- Generic and timeout jobs are handed to the host, which runs them through `call` (decision 9).

### Open, to be settled by this epic's first story

- Whether `perform_microtask_checkpoint` is bounded as a whole. Decision 9 gives each job the
  budget; a job that enqueues another job without end is not named as bounded, and `AGENTS.md`
  requires every loop of the engine to be charged.
- Which slice uses `HostEnqueueGenericJob` and `HostEnqueueTimeoutJob` first (the design assigns
  `Atomics.waitAsync` timeouts to the runner's host and slice 14).
- Which of the 39 features with no `swell-slices` line fall inside this slice.

## Counts

- Slice 10 (`promises-and-async`): 6,573 cases, 12,879 runs (section 12 table).
- Directories the design names for it (paths under `test/`, cases): `statements/class` 1,348,
  `expressions/class` 1,341, `statements/for-await-of` 1,232, `Promise` 643. Inventory, in all
  slices: the flag `async` is carried by 5,624 cases, `async-iteration` by 4,971, `async-functions`
  by 705.
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

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- Modules, `import`, dynamic `import`, top-level `await`: slice 11.
- `Atomics.waitAsync` and its timeout job: slice 14.
- `AsyncDisposableStack` and `await using`: slice 15.
- `FinalizationRegistry` cleanup jobs: slice 16.
- `await-dictionary`: stage 3, excluded (the `exclude` line).
- The HTML event loop, task queues and the choice of when to drain the queue: surf's (decision 9).
- `Temporal` (slice 20), ECMA-402 (slice 21), other stage 3 features, concurrent agents.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`: a suspended async frame is data, no
  host thread (`AGENTS.md`, "Rules"; decision 3).
- No panics, no `unwrap`, `expect` or unchecked indexing (`AGENTS.md`, "Rules").
- Every loop charged to the step budget, every allocation, queued jobs included, to the memory
  budget; a new limit gets a test that reaches it (`AGENTS.md`, "Rules").
- Jobs come from the host interface: the engine reads no clock and starts no timer (`AGENTS.md`,
  "Rules", "No ambient authority"; decision 9).
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 10 reads 6,573 cases and 12,879 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. An `async` case whose queue is empty without `Test262:AsyncTestComplete` or `Test262:AsyncTestFailure:` is a failed case, not a pass and not a hang (decision 11).

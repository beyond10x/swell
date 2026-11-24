# Slice 10, wave 1, with the closes of slices 5 and 7 and the first stories of slice 4

The branch `wave/s10-1`, opened on the release commit `a350de3` of release `0.8.0` (the tag `0.8.0`
is on `0c18271`, whose last commits the wave took), 2026-11-23 21:20 CET to 2026-11-24 18:30 CET,
tip `f27c52e` before its closing gate; the fix that gate asked for (`0c8ea8f`) and the commits
of release `0.9.0` follow it ("How the wave was checked"). Up to `f27c52e`, 45 commits on the
first-parent line: 26 merges that carry 28 stories, each
implemented by a sub-agent in its own worktree while the coordinator verified, committed and
merged, and 19 commits of plan, conformance, documents and merges of the release branch. The
source of every line below is a commit message
(`git log --first-parent --format='%h %s' 0.8.0..f27c52e`), a file of the tree at `f27c52e` or the
coordinator's log of the wave. Before: [wave 3 of slice 5](slice-05-wave-3.md), whose release
`0.8.0` is the start of this branch.

The wave holds the work of four slices on one branch: the close of slice 5 (the flag `v` and the
close), the close of slice 7 (generator methods and the close), the whole of slice 10, promises
and `async` functions, and the plan, the design text and the first six stories of slice 4. It
also holds the gate changes after release `0.8.0` and one story of the memory budget,
`story:copies-held-across-a-call-into-script-are-charged`.

## What the wave delivered

- **Slice 5 is delivered.** The flag `v` runs: classes with union, intersection, subtraction and
  nesting, `\q{...}`, and the seven properties of strings from a sixteenth generated table; no
  construct of slice 5 is refused any more. The close: every run of slice 5 passes
  (`test262-slice 05 2724 5441 0 0 2724 2724 0 0`); 18 cases that need a part of a later slice
  move to slices 4, 8 and 9 (14, 2 and 2) and 18 cases of the pattern grammar of Annex B.1.2 come
  in from slice 18.
- **Slice 7 is delivered.** Generator methods of object literals, `{ *g() {} }`, parse and run.
  The close: every run of slice 7 passes (`test262-slice 07 1472 2801 0 0 1472 1472 0 0`); 40
  cases that need `class` (36) or `super` (4) move to slice 8 and 21 cases come in, 2 from slice
  18 and 19 from slice 19.
- **Slice 10 is delivered**, in seventeen stories. The microtask queue of the agent and its
  checkpoint; `Promise` with its constructor, `then`, `catch`, `resolve`, `reject`, the species
  getter and the promise job; `Promise.all`, `allSettled`, `any`, `race`, `try`, `withResolvers`,
  `Promise.prototype.finally` and `AggregateError`; `async` functions, `await` and `async`
  methods of object literals on suspended frames; `%AsyncFunction%`; the rejection tracker and
  the reads of a promise; `for await` with the Async-from-Sync Iterator; async generators with
  their request queue, their methods, their constructor `%AsyncGeneratorFunction%` and `yield*`;
  `Array.fromAsync`; the target of the executable specification for the checkpoint and the hooks
  of a promise; and the close. Every run of slice 10 passes
  (`test262-slice 10 3759 7258 0 0 3759 3759 0 0`); 141 cases that need a later slice move to
  slices 8, 9, 12 and 13 and 21 cases come in from slices 18 and 19.
- **Slice 4, planned and begun.** `epic:slice-04-numbers-and-dates` is accepted in seventeen
  stories in five orders (`docs/plan/waves.md`, "Slice 4, `numbers-and-dates`"; the case list and
  the guard list of each story are under `docs/plan/slice-04/`), with the design text of its
  first and later waves. Six stories are built and merged: in `swell-num` the calendar arithmetic
  and local time, the text of a date, the double-double arithmetic with `hypot`, the exponential
  and logarithmic functions with `cbrt` and `exponentiate`, and the trigonometric functions; and
  in `swell` the `Date` object with `Date.now`, `getTime`, `setTime` and `valueOf` and the read
  of the host's clock. 564 of the 1,612 cases of the slice pass
  (`test262-slice 04 1612 3184 0 0 1612 564 1048 0`).
- **The gate.** Two steps of `task check`, `libm-check` and `math-data-check`; the minimum of
  scenarios that pass on the engine in `conform-engine` moved from 14 to 18 and its maximum of
  unsupported ones from 44 to 42; and what the wave took from the release branch (below).
- **A buffer held across a call into script is charged.** A buffer whose size a script controls
  and that is alive while the engine calls into script is charged to `memory_bytes` before it is
  made and credited when it is dropped, or it is not made and the String is read in its own
  storage. `HostError::MemoryBudget` is new and `HOST-215` has a test.

**The consumer's gap.** The first consumer's next gap, one `async` function with two `await` on
promises the host made, runs to its end since `1be065f`
(`async_functions::the_consumer_shape_runs_to_its_end`). The host that embeds the engine drains
the queue with `perform_microtask_checkpoint` after each script.

## Units

| story | unit commit | merge | what it brought |
|---|---|---|---|
| `job-queue-and-microtask-checkpoint` | `f2f0cea` | `2757246` | the agent's microtask queue and its checkpoint, the host's calls for them, the runner's handling of the flag `async` |
| `generator-methods-in-object-literals` | `93fcb70` | `c72fbee` | `{ *g() {} }` parses as a GeneratorMethod; no file of the compiler or the engine changed |
| `async-function-syntax-and-early-errors` | `52a630e` | `29bad25` | `async` declarations, expressions and arrow functions parse with their early errors; `await` parses as a prefix expression where [Await] is set; the refusal of slice 10 is held to the end of the parse |
| `regexp-v-flag-class-sets-and-properties-of-strings` | `ab58cc7` | `df40dfb` | the flag `v`: class set operations, `\q{...}`, seven properties of strings |
| `promise-constructor-then-and-jobs` | `20e2142` | `529a79f` | `Promise`, its resolving functions, `then`, `catch`, `Promise.resolve`, `Promise.reject`, the species getter, the promise job, `Cx::new_promise_capability` |
| `async-functions-and-await` | `3a1806c` | `1be065f` | `async` functions run on suspended frames; `%AsyncFunction.prototype%` |
| `promise-rejection-tracker-and-promise-reads` | `be16110` | `fbc2251` | `host_promise_rejection_tracker` with `Reject` and `Handle`; the reads of a promise and the mark as handled |
| `slice-05-conformance-close` | `3dbef6f` | `fa35b9d` | every run of slice 5 passes; 18 cases moved out, 18 taken in |
| `async-methods-in-object-literals` | `26c5b17` | `8812383` | `{ async m() {} }`, also under a computed name; `async *` refused with a `SyntaxError` that names slice 10 |
| `async-function-constructor` | `704455e` | `b19e0ce` | `%AsyncFunction%` through the one path of the dynamic functions |
| `slice-07-conformance-close` | `4e5e8eb` | `c6d44d1` | every run of slice 7 passes; 40 cases moved out, 21 taken in |
| `async-generator-syntax-and-early-errors` | `2094b09` | `2a6eb7f` | `async function*` parses whole with its early errors; the refusal of slice 10 is held behind them |
| `promise-statics-finally-and-aggregate-error` | `04942c6` | `c523990` | the statics of `Promise`, `finally`, `AggregateError`; `ErrorKind` is `#[non_exhaustive]` and has `AggregateError` |
| `for-await-and-async-from-sync-iterator` | `65dd2d3` | `8d4ae8b` | `for await`, `%AsyncIteratorPrototype%`, `WellKnownSymbol::AsyncIterator`, the Async-from-Sync Iterator |
| `ess-target-checkpoint-and-promise-hooks` | `fc5b4b8` | `c1f0bdc` | the conformance target of the executable specification runs the checkpoint and awaits the promise hooks; two scenario files; no engine crate changed |
| `async-generators-and-their-request-queue` | `61bfd0c` | `3f210ad` | `async function*`, `%AsyncGeneratorPrototype%` with `next`, `return`, `throw`, the queue of requests and its drain; tests of `HOST-204` to `HOST-206` |
| `async-generator-function-constructor` | `4c1d8fd`, `7442fcd` | `aafa59f` | `%AsyncGeneratorFunction%` |
| `async-generator-methods-in-object-literals` | `3112355` | `2b5b095` | `async *m() {}` in an object literal |
| `yield-star-in-async-generators` | `a0b60c9` | `818329b` | `yield*` in an async generator, to an async iterator or through the Async-from-Sync Iterator |
| `array-from-async` | `78cde0e` | `57e49c4` | `Array.fromAsync` as a built-in async function; tests of `HOST-207` and `HOST-208` |
| `slice-10-conformance-close` | `a46fe88` | `b825e55` | every run of slice 10 passes; 141 cases moved out, 21 taken in |
| `swell-num-calendar-and-local-time` | `9bcc7b1` | `c297c51` | the fields of a time value, `MakeFullYear`, the reading of an offset string, the two conversions between local time and UTC in `crates/num/src/date.rs` |
| `date-object-clock-and-time-value` | `f0fb125` | `ad94d0e` | `%Date%` with `Date.now`, `%Date.prototype%` with `getTime`, `setTime` and `valueOf`, the host's clock, `time_clip` |
| `swell-num-math-double-double` | `101cf4a` | `1e7092d` | `crates/num/src/math/dd.rs`, `hypot`, `data/math/reference-hypot.txt`, `libm-check` and `math-data-check` |
| `swell-num-date-text` | `97e2270` | `1e7092d` | `crates/num/src/date_text.rs`: the parser of the three forms `Date.parse` reads and the writers of the five date strings |
| `swell-num-math-exp-and-log` | `ea41fc6` | `4a8b544` | `exp`, `expm1`, `sinh`, `cosh`, `tanh`, `log`, `log2`, `log10`, `log1p`, `asinh`, `acosh`, `atanh`, `cbrt`, `exponentiate` with their reference data |
| `swell-num-math-trigonometry` | `dd862e5` | `4a8b544` | `sin`, `cos`, `tan`, `atan`, `asin`, `acos`, `atan2` with their reference data |
| `copies-held-across-a-call-into-script-are-charged` | `5ae993d` | `f27c52e` | `Accounting::hold`, `ChargedVec`, the copy of `string_units_in`; the microtask queue grows through the budget |

Also on the branch:

- The full test262 runs written as commits of their own: `c3bd9aa` (the join of the job queue,
  generator methods and the syntax of `async` functions, in two parts) and `80a9aae` (the flag
  `v` and `Promise`, one pool job of 264 s).
- Taken from the release branch: `0ea3df9` (the documents of release `0.8.0`), `ad77a79` (the
  gate change of release `0.8.0`, `story:gate-test-steps-fit-four-cores`, unit `a182a02`),
  `0acd99d` (the dated heading of `0.8.0` in `CHANGELOG.md`) and `db95e1c`
  (`story:slow-lane-starts-by-hand`, so that the wave stays a descendant of the release tip
  `0c18271`).
- Plan commits: `cd04bc1` and `034c0a8` (a case of the Promise list needs `Map`; four cases of
  the `async` functions list need `new.target`), `db89faa` (the close of slice 7 states the rule
  its base needs), `e87917e` (the target story states what its unit measured), `b160d1b` (the
  twelve stories of release `0.8.0` move to `implemented`) and `75b857d` (the close of slice 10
  states what its base measured).
- Documents: `2a05088` and `ac4edfc` (rules and counts of `AGENTS.md`) and `3776116` (the three
  requirements of the clock and the time zones in `CHANGELOG.md`).
- Design text and the plan of slice 4: `9caacb6` (decision 6 of `docs/design/engine.md` for the
  first wave, decision 17 of `docs/design/host-interface.md`, `HOST-209` to `HOST-211`),
  `80dfd7b` (decision 6 for the later waves, `HOST-212` and `HOST-213`), `543b7d3` (four design
  sentences, `HOST-147` changed, `HOST-214` new) and `76aa4fc` (the plan: seventeen stories
  accepted, `aep plan artifact validate` prints `valid`).

## Numbers

Cases that pass in the full test262 run after each step that wrote its own lines (`of 44,812`,
and in the columns of a slice, of its cases: slice 5 of 2,724, slice 7 of 1,491 and 1,472 after
its close, slice 10 of 3,879 and 3,759 after its close, slice 4 of 1,612). A dash is a number the
merge commit does not state.

| after | cases that pass | slice 5 | slice 7 | slice 10 | slice 4 |
|---|---|---|---|---|---|
| release `0.8.0` (`a350de3`) | 20,229 | 2,523 | 1,168 | 14 | 261 |
| job queue, generator methods, `async` syntax (`c3bd9aa`) | 20,697 | 2,523 | 1,451 | 161 | |
| flag `v`, `Promise` (`80a9aae`) | 21,095 | 2,706 | 1,451 | 365 | |
| `async` functions (`1be065f`) | 21,325 | | | | |
| close of slice 5 (`fa35b9d`) | 21,325 | 2,724 | | | |
| `async` methods (`8812383`) | 21,374 | | | | |
| `%AsyncFunction%` (`b19e0ce`) | 21,383 | | | | |
| close of slice 7 (`c6d44d1`) | 21,383 | | 1,472 | 638 | |
| async generator syntax (`2a6eb7f`) | 21,527 | | | 781 | |
| statics of `Promise` (`c523990`) | 21,945 | | | 1,189 | |
| `for await` (`8d4ae8b`) | 22,426 | | | 1,665 | |
| async generators (`3f210ad`) | 23,553 | | | 2,785 | |
| `%AsyncGeneratorFunction%` (`aafa59f`) | 23,564 | | | 2,796 | |
| async generator methods (`2b5b095`) | 23,815 | | | 3,046 | |
| `yield*` in async generators (`818329b`) | 24,432 | | | 3,663 | |
| `Array.fromAsync` (`57e49c4`) | 24,508 | | | 3,738 | |
| close of slice 10 (`b825e55`) | 24,508 | | | 3,759 | |
| `Date` (`ad94d0e`) | 24,838 | | | | 564 |
| the tip (`f27c52e`) | 24,838 | | | | 564 |

The lines of `conformance/test262/expected.txt` at `f27c52e`:

```
test262 total 53801 8982 7 44812 24838 19972 2
test262-slice 04 1612 3184 0 0 1612 564 1048 0
test262-slice 05 2724 5441 0 0 2724 2724 0 0
test262-slice 07 1472 2801 0 0 1472 1472 0 0
test262-slice 10 3759 7258 0 0 3759 3759 0 0
```

- No case that passed in a full run fails in the next one: each merge that wrote a full run states
  "none lost" or "no run that passed before failing". For the `Promise` run, `check.sh` of
  `docs/plan/slice-10` over all 85,556 run lines of both runs prints no line `regressed`.
- Runs that meet a refusal: 30,642 at release `0.8.0`, 29,603 on the join of `c3bd9aa`.
- Cases moved with evidence: 1,774 at release `0.8.0`, 1,785 after the close of slice 5, 1,825
  after the close of slice 7.
- The close of slice 7 was made on a base that holds code of slice 10: 682 candidates, 21 come
  in, 661 stay where they are counted. The close of slice 10 moves 141 cases and 280 runs (the
  story had written 138 cases): 116 to slice 8 (`class` 90, `super` 20, `new.target` 6), 20 to slice
  12 (BigInt literals), 4 to slice 9 (`Proxy` 3, `Map` 1) and 1 to slice 13 (`ArrayBuffer`); 21
  cases come in, 1 from slice 18 and 20 from slice 19; 66 other cases of later slices that pass
  stay where they are counted.
- Every case of each story's list passes in every run: 186 for `Promise` (the list lost one case
  that needs `Map`), 203 for `async` functions (207 before the four that need `new.target`
  left), 154 for the syntax of async generators, 424 for the statics, 420 for `for await`, 1,124
  for async generators, 23 for `%AsyncGeneratorFunction%`, 250 for their methods, 595 for
  `yield*` and 73 for `Array.fromAsync`. The 164 cases of the `Date` list pass, none did before,
  and the 186 of its guard list still pass; the merge made 330 more cases pass, 166 of them
  outside the story's list and 91 of those under `test/built-ins/Date/prototype`.
- A `next` through n levels of `yield*` in an async generator costs 15 steps per level.
- The host requirements (`conformance/host/requirements.txt`, 215 lines at `f27c52e`):

| after | lines | with a test | pending |
|---|---|---|---|
| release `0.8.0` | 208 | 114 | 94 |
| `ac4edfc` | 211 | 140 | 71 |
| `80dfd7b` | 213 | 140 | 73 |
| `3f210ad` | 213 | 143 | 70 |
| `57e49c4` | 213 | 145 | 68 |
| `543b7d3` | 214 | 145 | 69 |
| `ad94d0e` | 214 | 147 | 67 |
| the tip (`f27c52e`) | 215 | 148 | 67 |

  Of the 67 pending at the tip, 12 are for slice 4, 6 for slice 9, 32 for slice 11, 1 for slice
  8, 2 each for slices 12 and 13, 6 for slice 14, 5 for slice 16 and 1 for slice 18; none for
  slice 10.
- The model of the host interface (`spec/`): 60 scenarios, 10 authored, 41 refusals and 53
  markers of what it cannot state; 18 scenarios pass on the engine and 42 name what they wait for
  (14 of 58, 8 authored and 51 markers before `c1f0bdc`).
- Tests of `swell` at each join the merge commits state (default mode, then with a collection
  at every safe point), then the six small crates and `swell-test262`:

| merge | `swell` | with a collection at every safe point | six small crates | `swell-test262` |
|---|---|---|---|---|
| `1be065f` | 1014 passed, 1 skipped | 1012 passed, 1 skipped | | |
| `fbc2251`, `fa35b9d` | 1021 passed, 1 skipped | 1019 passed, 1 skipped | | |
| `8812383` | 1025 passed, 1 skipped | 1023 passed, 1 skipped | | |
| `b19e0ce` | 1036 passed, 1 skipped | 1034 passed, 1 skipped | | |
| `c6d44d1` | 1036 passed, 1 skipped | 1034 passed, 1 skipped | 224 | 73 |
| `2a6eb7f` | 1036 passed, 1 skipped | 1034 passed, 1 skipped | 226 | 73 |
| `c523990` | 1057 passed, 1 skipped | 1055 passed, 1 skipped | 226 | 73 |
| `8d4ae8b` | 1072 passed, 1 skipped | 1070 passed, 1 skipped | 226 | 73 |
| `ad94d0e` | 1,153 passed | 1,151 passed | 243 | 73 |
| `1e7092d` | 1,153 passed | 1,151 passed | 275 | 73 |
| `4a8b544` | 1,153 passed | 1,151 passed | 294 | 73 |
| `f27c52e` | 1,169 passed | 1,167 passed | 294 | 73 |

- The cost of the new tests, in CPU seconds as the unit's commit states it: the 27 tests of
  `Promise` 9.84 with a collection at every safe point, the costliest 1.78; the 20 tests of the
  statics 0.5 in the default mode and 2.8 with a collection (the sum is 3.3, where the story
  expected under 3); the tests of `for await` under 0.11 each in the default mode, the costliest
  1.47 with a collection, about 4.3 together at opt-level 0 where the story expected 3; the 26
  tests of async generators 0.82 in the default mode and 7.98 with a collection; the 14 tests of
  `Date` about 2.1 over both modes; the 16 new tests of the charged buffers about 0.47 and 2.64.
- Frames at opt-level 0 after the charged buffers, unchanged: `run_builtin` 1,152,
  `builtin_call` 792, `call_value` 216, `execute` 392, `call_function` 1,384 and
  `run_regexp_builtin` 296 bytes; `regexp_replace_results` reads 840 bytes of frame, 1,032 before.
  The 37 tests `*_on_the_promised_stack` pass in both modes and `STACK_BYTES_PER_REENTRY` is as it
  was. The arm of `Date` in `run_builtin` is one call out of line that passes `this` and
  `new_target` by reference: by value the frame of `run_builtin` was 1,184 bytes.
- The charged buffers: 16 new tests (14 in the library, the sizes test and `HOST-215`); on the
  tree before the charges 15 of 15 failed at their assertion.

## Decisions

Taken by the coordinator, each with its reason where the source states one.

- **D89.** `wave/s10-1` was opened on the release commit `a350de3` while the checks of release
  `0.8.0` ran, and took the release's later commits (`0ea3df9`, `ad77a79`, `0acd99d`, `db95e1c`).
  Reason: the wave stays a descendant of the release tip `0c18271`.
- **D93.** The lane `slow` keeps the collect-mode stack modules at the test profile (opt-level 0);
  only the gate's collect steps use the profile `collect` (`.cargo/collect-profile.toml`).
- **D106, D107.** The gate bounds the test steps' CPU seconds by a table keyed by core count
  (`CPU_BOUNDS_BY_CORES` in `taskfiles/forge.yml`), not by a ratio; a core count without an entry
  prints and decides nothing, and on the forge the wall time decides. 64 cores get no entry: the
  rule of 1.25 is not met (changed code 116.1 to 200.6 CPU seconds, base 254.0 and 301.7 on the
  node). 16 cores get the entry 245 (195.0 × 1.25 = 243.8 is at most 245, and 245 × 1.25 = 306.3
  is at most 312.1).
- **The wave's gate runs once, at its close.** The merge that took the gate change ran no gate;
  each merge commit states the checks it ran on its tree (below).
- **D103.** The rejection-tracker story started on `cd04bc1`, before `async-functions-and-await`
  was merged. Reason: its edge to that story is a count of pending requirement lines only, and
  its acceptance reads that count on its own base (14, then 10).
- **D109.** A case of a later slice comes into slice 7 only when every failed run of it at the
  start of the slice ends on a part of slice 7, no `feature` or `flag` line names a later slice
  for it, and it uses no part of a later slice in its text or its harness files. Reason: the base
  of the close holds code of slice 10, so "passes on the base" no longer shows that slice 7 made
  a case pass. The close of slice 10 states what its base measured the same way.
- **D118, D120, D121.** The date story of slice 4 was cut in two, a clock story
  (`date-object-clock-and-time-value`, which owns `time_clip`) and a zone story; `MakeFullYear`
  went into the calendar story. The clock story has no edge to the math story (D120) and is in
  the first order; `math-functions` has no edge to `number-functions` (D121; 2 cases go to the
  close).
- **D122.** The story that would grow the model's scenarios leaves slice 4. Reason: `AGENTS.md`
  says the model is not grown to chase coverage.
- **D130, taken back.** It was first worded as no full run for a merge that changes only
  `crates/num`; it was withdrawn for this rule: every merge that changes a crate gets a full
  test262 run. Reason: `test262-check` compares a digest of the run's sources, `crates/num`
  included, so a change there moves `expected.digest` even where no script reaches it. Units
  that are ready together are merged in one run (`1e7092d` holds two units and one run,
  `4a8b544` two units and one run).
- **D131.** The lint that holds later code to the rule of the charged buffers is a story of its
  own, `story:buffer-growth-outside-the-charged-list-is-listed`, built last before the release.
  Reason: it would turn every later unit that reserves a buffer red.
- **D145.** Release `0.9.0` is cut from `f27c52e` with the fix of its closing gate; the open units
  of slice 4 and the lint story go into the release after that. Reason: slice 10 is closed and
  the charged buffers are merged.
- **D146.** The fix of the gate self-test that the closing gate found red is a task of the
  wave's own, `task:moves-selftest-plants-by-the-order-of-delivery`, accepted without the
  plan-critic panel. Reason: it changes one planted copy of one script under `tests/gate/`, and
  its acceptance is the script's own output and a weakened comparison shown red.
- **The close of slice 10 needed no join.** It was made on the wave's tip, so its full run is the
  run of the tree's sources (`b825e55`).

## What changed for a host

All of it is in `CHANGELOG.md`, "0.9.0", "Host interface".

- **Queue and checkpoint.** `Agent::queue_microtask`, `Agent::perform_microtask_checkpoint`,
  `Cx::queue_microtask`, `Cx::perform_microtask_checkpoint`, of the agent `pending_microtasks`
  and `clear_microtasks`, the type `Microtask` and the struct `CheckpointReport` with
  `promise_jobs` and `host_microtasks`.
- **Promises.** The struct `PromiseCapability` (`promise`, `resolve`, `reject`) and
  `new_promise_capability` of the context; the enum `PromiseState` (`Pending`, `Fulfilled`,
  `Rejected`), `Agent::promise_state`, `Agent::promise_is_handled`,
  `Agent::mark_promise_as_handled` and the three of the contexts. The engine calls
  `host_promise_rejection_tracker` with `Reject` and with `Handle`, and calls
  `host_prepare_to_run_job`, `host_call_job_callback` and `host_clean_up_after_job` around a
  promise job.
- **Breaking.** `ErrorKind` is `#[non_exhaustive]`: a `match` over it outside swell needs a
  wildcard arm. `ErrorKind::AggregateError` is new; `ErrorKind::ALL` keeps its seven entries.
  `HostError::MemoryBudget` is a new variant of an enum that is not `#[non_exhaustive]`: a
  `match` over `HostError` needs an arm for it or a wildcard arm. `queue_microtask` answers it
  when the memory budget refuses the growth of the queue, with nothing queued.
- **New names.** `WellKnownSymbol::AsyncIterator`, the thirteenth well-known symbol a host
  names.
- **Requirements that have a test.** `HOST-46`, `HOST-77` to `HOST-84`, `HOST-100`, `HOST-101`,
  `HOST-121`, `HOST-125`, `HOST-126`, `HOST-148`, `HOST-149`, `HOST-192` to `HOST-208` and
  `HOST-215`.
- **New requirements, pending.** `HOST-209` to `HOST-211` (the clock and the time zones: an
  offset outside ±86,399,999,999,999 ns is clamped; NaN after 31 hook calls when no instant is
  found; each hook call is charged one step), `HOST-212` and `HOST-213` (`Math.random`) and
  `HOST-214` (the key order of a `Map`, slice 9). `HOST-147` no longer names a `Map`.
- **The model of the interface** states the microtask checkpoint with its answers at the step
  budget and at an interrupt, and a job that will not run to its end (`Abandoned`).
- **The check of the names of the host interface** printed `host-surface: 37 change(s) since
  0.8.0, each named` in the gate on `f27c52e`: 35 lines added and two changed (`ErrorKind`,
  `HOST-147`), none removed. The release commit writes the snapshot of `0.9.0`
  (`conformance/host/surface.txt`, 549 lines).

## Findings left open

| finding | where it is recorded |
|---|---|
| The stack room per re-entry of a replacer is about 14 bytes: `regexp_symbol_methods::a_replacer_re_enters_on_the_promised_stack` has 13,984 bytes of room on a promised stack of 12,838,560 bytes at 1,000 re-entries, and the smallest stack that passes is 12,824,576. `STACK_BYTES_PER_REENTRY` is 14 KiB. A later arm of `run_builtin` takes `this` and `new_target` by reference and calls out of line, or the test overflows. No story holds the room. | `f0fb125`; `crates/swell/src/host/limits.rs` line 46; `crates/swell/tests/regexp_symbol_methods.rs` line 631 |
| The walk over every function of the realm, `builtins_symbol_values::every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_error`, is over its bound of 2.5 CPU seconds at its full count with a collection at every safe point: 2.71 in the unit of `Array.fromAsync` (2.33 and 2.34 in two other reads, 2.28 to 2.44 on its base) and 3.54 at a machine load of 65 in the report of the unit of `Date`, which reached 217 functions from 196 (2.89 and 2.50 on its base at load 28; these figures are not in the tree). The doc comment of `places_per_function` still says 1.8. | `78cde0e`, `f0fb125`, the coordinator's log; `crates/swell/tests/builtins_symbol_values.rs` lines 468 and 469 |
| Tests compare a budget or a reading with a room fitted to one size of the realm. Two tests of this wave changed at a join because the first round of a script grows an arena and `heap_bytes` keeps the capacity: `async_functions::an_activation_whose_job_was_cleared_is_never_resumed` (225,349 to 279,621 before the change) and `async_generators::an_async_generator_suspended_at_a_termination_resumes_at_a_later_checkpoint`. Others, such as `termination::an_agent_that_ended_in_memory_budget_runs_again_after_a_collection`, `generators::a_suspended_frame_is_charged_until_its_generator_is_freed` and `determinism::host_133_same_calls_same_hooks_same_outcomes`, set budgets from the realm's size when they were written. The story `tests-hold-at-any-size-of-the-realm` is being built; it is not yet in the store of this tree. | `e1fa948`, `82c505f`, the coordinator's log |
| `BINDINGS_REALM_HEAP_BYTES` is 506,112 and the realm grows with each story that adds functions to it: 217 functions with `Date` (196 before), and the reading of `HOST-187` was 437,533 with `Date` alone. A raise of the constant is a change of the host interface named in `CHANGELOG.md`. The stories of slice 4 that add functions name the readings of `HOST-186` and `HOST-187` that their unit runs. | `crates/swell/src/host/limits.rs` line 95; `docs/design/host-interface.md`, decision 14 and `HOST-187`; `f0fb125`; the coordinator's log |
| Two test files of `swell-num`, `crates/num/tests/math_exp_log.rs` and `crates/num/tests/math_trig.rs`, include `tests/support/exact.rs` by path with `#[allow(clippy::duplicate_mod)]`; a generated `mod support` in `tests/main.rs` would remove both allows. | the two files; the coordinator's log |
| The lint of `story:buffer-growth-outside-the-charged-list-is-listed` is planned, not built: a list of disallowed methods for the engine crate and an `expect` with a reason at each remaining growth call. It depends on `story:copies-held-across-a-call-into-script-are-charged`. | `.engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md`; `f27c52e` |
| The epics of slices 5, 7 and 10 still state the counts of their first measurement and are `active`: `epic:slice-05-regexp` 2,118 cases and 4,234 runs (2,724 and 5,441 in `expected.txt`), `epic:slice-07-generators` 1,491 cases and 2,841 runs (1,472 and 2,801) and `epic:slice-10-promises-and-async` 3,879 cases and 7,499 runs (3,759 and 7,258). | `.engineering/planning/epic/`; `conformance/test262/expected.txt` |
| The stories of this wave stay `active` until a release holds them; the twelve of release `0.8.0` moved to `implemented` after the forge ran `check` and the lane `slow` green on `0c18271`. | `b160d1b` |
| The new tests of several units cost more CPU seconds with a collection at every safe point than their story expected in sum: 26 tests 7.98 (expected under 3), the tests of `for await` about 4.3 (expected 3), 20 tests 3.3 (expected under 3); the 27 tests of `Promise` sum to 9.84. | `20e2142`, `3f210ad`, `65dd2d3`, `04942c6` |
| The gate ended red once on `c6d44d1` (exit 201 after 83 s, 133 lines "is not a test of the workspace") while two measuring jobs of the same tree ran beside it; it ended 0 when run again with none. That the second job caused the red run is a reading of the two runs, not a proof. | `2a05088`; `AGENTS.md`, "Commands and the gate" |
| The entry `16:245` of `CPU_BOUNDS_BY_CORES` is provisional: its pair under this tree is to be recorded by the first two `check-runner` jobs on a 16-core machine. The check that holds the table is a draft. | `story:gate-cpu-table-held-by-a-check`; `AGENTS.md`, "Commands and the gate" |
| Four cases of slice 5 that depend on the Unicode version have no section in `conformance/test262/sections.txt`; they pass. | `fa35b9d` |
| 42 of the model's 60 scenarios are unsupported on the engine. `faulted-agent-refuses-every-call` stays unsupported, and no scenario states `HOST-83`: `ess` 0.97.4 refuses an awaited step `HostEnqueuePromiseJob` (`ESS-AUTHOR-045`). | `e87917e`; `c1f0bdc` |
| The header comment of `crates/num/src/lib.rs` still lists `date` by its four older functions. | `9bcc7b1`; `crates/num/src/lib.rs` |
| Stories of slice 4 built and not merged at `f27c52e`: `math-constants-and-exact-functions`, `number-format`, `date-object-constructor-now-utc-and-getters`, `number-functions`, `math-functions`, `math-sum-precise`, `date-setters` and `date-text-formats`; not built: the story of `**` and `**=`, `math-random` and `slice-04-conformance-close`. | `docs/plan/waves.md`, "Slice 4, `numbers-and-dates`" |
| The self-test of `test262-moves-check` plants a `<from>` that comes strictly after its `<to>` in the order of delivery. With the task's comparison `<=` turned to `<` in a scratch copy the script still passes: a line whose two slices are equal is not planted. | `b3e4953`; `tests/gate/test262-moves-check-refuses-a-line-without-evidence.sh` |

## How the wave was checked

- **The units.** Each unit ran its story's tests in both collection modes, the tests of the crates
  it changed, clippy and fmt, and, where a script can reach its change, its story's case list on
  the test262 runner against the base.
  A unit whose base was older than the branch was joined with the branch, and the join was tested
  on the pool's build node before its merge.
- **The joins** (as the merge commits state them). `1be065f`, joined in `b26c94e`: `swell` 1014
  passed and 1 skipped, with a collection at every safe point 1012 passed and 1 skipped, the other
  crates, clippy and the checks of the tables, the names of the host interface and the host
  requirements, each exit 0. `fbc2251` (`f0e332c`), `fa35b9d` (`0be6d75`), `8812383` (`f437477`)
  and `b19e0ce`: the same checks with the counts of the table above, and the full run of test262
  with the wave's totals and no run that passed before failing. `c6d44d1` (`03ef654`), `2a6eb7f`
  (`0fe483a`), `c523990` (`721a08d`) and `8d4ae8b` (`e1fa948`) also: the six small crates, 224 to
  226 passed, and `swell-test262` 73 passed. `3f210ad` (`102c50d`), `aafa59f` (`7442fcd`),
  `2b5b095` (`93a743f`) and `818329b` (`34346a8`): the full test262 run of the join, none lost.
  `57e49c4` (`82c505f`, `1d478f1`): the full run of the join, none lost, and the model's scenarios
  on the engine, 18 of 60 passed, 0 failed, 42 unsupported. `c1f0bdc` (`29d003f`): fmt, clippy,
  `validate`, `suite` (60 scenarios, 10 authored, 41 refusals) and `conform-engine` (3 test
  binaries, 16 passed; both planted defects refused; `60 scenarios: 18 passed, 0 failed, 0 error,
  42 unsupported`), each exit 0, and `slot task conform` on the interpreter of `ess`:
  `60 scenarios: 60 passed, 0 failed, 0 error, 0 unsupported`.
- **The joins of slice 4 and the charged buffers.** `ad94d0e` (`f731d7e`): `swell` 1,153 passed
  and 1,151 with a collection, `swell-test262` 73, the other six crates 243, clippy. `1e7092d`
  (`a83e398`): the same with the six crates 275, one full run for both units, compared run by run
  with the run before it (`passed 24838 bound 24838 B 24838 F 0 R 0`). `4a8b544` (`1ad0f05`):
  the six crates 294, `swell-test262` 73, `swell` 1,153 and 1,151, clippy and
  `cargo fmt --all --check` exit 0, the full run unchanged. `c297c51`: the full run gives the same
  expected file and no run changed (`passed 24508 bound 24508 B 24508 F 0 R 0`). `f27c52e`
  (`2407acc`): the six crates 294, `swell-test262` 73, `swell` 1,169 and 1,167, clippy and
  `cargo fmt --all --check` exit 0, the full run unchanged (`test262 total 53801 8982 7 44812
  24838 19972 2`, no case regressed).
- **The merge checks.** Each merge commit names, on its tree, `host-requirements-check`,
  `site-data-check`, `test262-check`, `test262-moves-check` and the check of the names of the host
  interface, each exit 0; `4a8b544` and `1e7092d` add `libm-check` and `math-data-check`. The
  message of `9caacb6` said all five exited 0; the check of the names of the host interface
  failed on it (exit 201: `HOST-209` to `HOST-211` changed and "Unreleased" did not name them),
  the merge was committed before the result was read, and `3776116` named them.
- **The full test262 runs.** `80a9aae`: `task test262-expected`, one pool job of 264 s, written
  as `test262 total 53801 8982 7 44812 21095 23715 2`. `c3bd9aa`: made in two parts and counted
  from them. `task test262-check` and `task site-data-check` exit 0 on each merge that wrote a
  run. `expected.digest` moved with `crates/num` and with the charged buffers, and the run on
  the joined tree gave the same lines each time.
- **The releases.** Release `0.8.0` was published on `0c18271`; the forge ran `check` and the
  lane `slow` green there (`b160d1b`).
- **Not checked here**: the forge's runs of `check` and the lane `slow` on the commits of this
  wave. The gate on the tip is the closing gate below.

- **The closing gate.** `slot --timeout 30m task check-runner` on `f27c52e`, on the pool's build
  node: every step passed up to the last one, `gate-selftest`, which printed
  `gate-selftest: 65 passed, 1 failed`, and the run exited 201. The steps before it printed
  `17 test binaries: 1536 passed, 0 failed, 1 ignored`,
  `test-doc: 8 test suites: 19 passed, 0 failed, 0 ignored`,
  `test-collect: 2 test binaries: 1167 passed, 0 failed, 1 ignored`,
  `test-faults: 1 test binaries: 1 passed, 0 failed, 964 ignored`,
  `conform-engine: 3 test binaries: 16 passed, 0 failed, 0 ignored`,
  `60 scenarios: 18 passed, 0 failed, 0 error, 42 unsupported`,
  `host-requirements: 215 requirement(s), 215 line(s): 148 test, 67 pending, 0 omitted`,
  `test262-refused 23324` and `test262 total 53801 8982 7 44812 24838 19972 2`; the test steps
  before the self-test took 6.4, 3.7, 2.4, 0.2, 1.4, 0.1 and 2.5 s.
- **The red self-test and its fix.** `test262-moves-check-refuses-a-line-without-evidence` ended
  with `FAIL the planted <to> below <from> copy equals the unchanged lines`: its third planted
  copy set the `<from>` of a line to `<to> + 1`, and since the close of slice 10 the first line
  of `conformance/test262/moves.tsv` moves a case from slice 10 to slice 9. The script now
  plants by the order of delivery (`b3e4953`, merged in `0c8ea8f`; D146); on the unit's tree it
  printed `ok` and `slot task gate-selftest` printed `gate-selftest: 66 passed, 0 failed`.
- **The lane `slow`** on `f27c52e`, on the build node, exited 0: the full test262 run
  (`slow-test262: 287 lines equal conformance/test262/expected.txt byte for byte`),
  `slow-tests: 4 test binaries: 670 passed, 0 failed, 669 ignored`,
  `slow-tests collect-every-safe-point: 2 test binaries: 867 passed, 0 failed, 301 ignored`, and
  the four tests of each stack profile (`test-stack-profiles release` and `stack-debug`:
  `4 passed, 0 failed, 961 skipped` each).
- **The gate on the release commit.** `slot --timeout 30m task check-runner` on `5ee604a`, on
  the build node at load 36 on 64 cores, exited 0: `gate-selftest: 66 passed, 0 failed`,
  `gate-time test-steps 24.9 of 120, cpu 166.7`, `host-surface: 0 change(s) since 0.9.0, each
  named`, and the counts of the closing gate above for the tests, the scenarios, the host
  requirements and test262. Its lines are in `AGENTS.md`, "The gate's numbers".

---
format: aep.planning-md/3
id: review-result:slice-10-round-1-scope
kind: review-result
status: active
title: 'Slice 10, round 1, the scope critic: the four stories of the first wave'
relations:
- reviews: story:job-queue-and-microtask-checkpoint
- reviews: story:async-function-syntax-and-early-errors
- reviews: story:promise-constructor-then-and-jobs
- reviews: story:async-functions-and-await
revision: 1
---
needs-revision

story:async-functions-and-await — The story removes the held `async function` refusal, but its Writes line leaves out `crates/syntax/src/parser/mod.rs` and `crates/syntax/src/lib.rs`. The story before it rewrites the documentation of `held` (lines 80 to 84) and of `parse_script` (lines 148 to 150) to name that refusal, so both doc comments would still state a refusal that no longer exists. The story's own search (`grep -rn 'async function is not supported\|"async function"' ...`) cannot find them: they read "an `async` function", with backticks. The Writes line should take both files, and the search should also match the doc wording. — .engineering/planning/story/async-functions-and-await.md:155 (the earlier story's edits are at .engineering/planning/story/async-function-syntax-and-early-errors.md:73 and :85; the search is at async-functions-and-await.md:113)

**What I read:** 4 drafted stories against their parent. I ran `aep plan artifact show` on the epic (read first, its promises listed before any story) and on each of the four stories, plus `aep plan artifact graph` (no other item decomposes the epic) and `aep plan artifact relations`. For context I read `review-result:slice-07-round-1-scope`, `docs/plan/waves.md` lines 160 to 323, and engine.md lines 1822 to 1836 and 4035 to 4055. I also read the parts of the code each Writes line names: `host/{agent,mod,hooks,outcome}.rs`, `compile_fail.rs`, `heap/collect.rs`, the `ObjectKind`/`KindTag` sites, the `Instruction` matches, `Expr::Yield` sites, `realm/global_object.rs` with `tests/builtins_global_this.rs`, `crates/test262/src/run.rs` and `tests/{run,runner}.rs`, `conformance/ess/src/target.rs`, `conformance/test262/later-parts.tsv`, `taskfiles/{host-surface,host-requirements,test262}.yml`. And the Writes lines of the four open stories of slices 5 and 7 that share files with this wave, plus the consumer's `testharness.js`.

**Promises:** I extracted 32 from the epic and accounted for all 32:
- **19 traced to a story of this wave:**
  - The queue in the agent.
  - The host deciding when to drain it.
  - The checkpoint and its one budget, a departure that the story names as a design change.
  - The runner's `async` judgement.
  - Acceptance item 5.
  - Open questions 1 and 2, answered by `story:job-queue-and-microtask-checkpoint`: generic and timeout jobs go to slice 14, with a reason.
  - HostEnqueuePromiseJob, HostMakeJobCallback and HostCallJobCallback.
  - The base of `Promise`.
  - Async functions, async arrows and `await`.
  - The 5 constraints.
- **12 named out to later stories or slices:**
  - The rejection tracker and the ESS event.
  - The statics, `finally`, `AggregateError` and `Array.fromAsync`.
  - Async generators, `for await` and `AsyncFunction` with the other async intrinsics.
  - `Symbol.asyncIterator`.
  - Acceptance items 1 to 4.
- **1 answered by the design:** open question 3. engine.md:4045 settles it, as the slice-7 scope round found.

**Coverage:** no gap, nothing reaching beyond the parent, and no outcome claimed by two stories. Host microtasks, `pending_microtasks` and `Cx::new_promise_capability` trace to HOST-78, 79 and 81 and to host-interface.md:1364, which are pending for slice 10. Nothing is narrowed silently: each departure is named. Each story can be shown on its own.

**What I could not establish, or left out of my lane:**
- **The cut (design lane, does not set my verdict).** `story:promise-constructor-then-and-jobs` says it cannot be cut further because "the job without `then` has no reaction to run" (its Size bullet). That only half holds: NewPromiseResolveThenableJob runs a promise job without `Promise.prototype.then`. So the story could split into two parts that can each be shown:
  - the constructor, resolve/reject and the thenable job, with HOST-82 and HOST-83;
  - `then`/`catch` and the reactions, with HOST-78, 80, 84 and 148.

  The other three stories can each be shown alone, and none of them needs another to be shown.
- **Bounds rule (acceptance lane).** No test in the set reaches `reentry_depth` through jobs. There are tests that show `then` chains and `await` chains take no depth, but none reaches the bound. And no test of `story:job-queue-and-microtask-checkpoint` queues host microtasks without end; HOST-80 in the Promise story covers that only for promise jobs.
- **Stale epic (not this set's to fix).** The epic's line 16 ("async class members are counted here") and slice-08-classes.md:70 contradict engine.md:1830, which moves 2,743 cases from slice 10 to 8. The set follows the design. Both epics need the coordinator's rewrite.
- **Files shared with open stories (point 6 of the brief).** The shared files are named, apart from the new syntax fixtures they both add under `crates/syntax/tests/sources/`, which do not collide (different names). `story:regexp-v-flag-class-sets-and-properties-of-strings` is the one story the set does not name, and it shares only that fixture directory.
- I created no scratch and wrote nothing.

```findings
- file: .engineering/planning/story/async-functions-and-await.md
  line: 155
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The story removes the held `async function` refusal, but its Writes line leaves out `crates/syntax/src/parser/mod.rs` and `crates/syntax/src/lib.rs`. The story before it rewrites the documentation of `held` (lines 80 to 84) and of `parse_script` (lines 148 to 150) to name that refusal, so both doc comments would still state a refusal that no longer exists. The story's own search (`grep -rn 'async function is not supported\\|\"async function\"' ...`) cannot find them: they read \"an `async` function\", with backticks. The Writes line should take both files, and the search should also match the doc wording."
```

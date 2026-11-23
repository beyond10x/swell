---
format: aep.planning-md/3
id: review-result:slice-10-round-1-acceptance
kind: review-result
status: active
title: 'Slice 10, round 1, the acceptance critic: the four stories of the first wave'
relations:
- reviews: story:job-queue-and-microtask-checkpoint
- reviews: story:async-function-syntax-and-early-errors
- reviews: story:promise-constructor-then-and-jobs
- reviews: story:async-functions-and-await
revision: 1
---
needs-revision

story:async-functions-and-await — the refusal search finds only the quoted text, so it misses two doc comments that `story:async-function-syntax-and-early-errors` rewrites to name the held `async` function refusal: the comment on `held` in `crates/syntax/src/parser/mod.rs` and the doc of `parse_script` in `crates/syntax/src/lib.rs`; neither file is on this story's Writes line, so both stay wrong once the refusal is lifted — .engineering/planning/story/async-functions-and-await.md:113, :155; .engineering/planning/story/async-function-syntax-and-early-errors.md:73, :85
story:async-functions-and-await — `a_completed_activation_holds_no_frame` names no `heap_bytes()` reading, so no test shows the frame's credit when the call completes, one of the three credit paths in Bounds (this is slice 7's round-1 finding on `a_completed_generator_holds_no_frame` back again) — .engineering/planning/story/async-functions-and-await.md:145
story:async-functions-and-await — `a_suspended_activation_resumes_after_a_termination_of_another_job` names no termination, so it passes with just one, while a suspended activation has to be shown at `StepBudget`, `MemoryBudget` and `Interrupted` each (slice 7's round-1 finding back again) — .engineering/planning/story/async-functions-and-await.md:145
story:async-functions-and-await — Bounds says a resumption at `frame_depth` gives a `RangeError` that rejects nothing, the activation stays suspended and the throw goes to `host_clean_up_after_job`, but no test in Change reaches `frame_depth` at a resumption (for example a checkpoint run from a host function deep in the frame stack) — .engineering/planning/story/async-functions-and-await.md:143, :145
story:async-functions-and-await — `a_resumption_re_enters_on_the_promised_stack` is to use `reach_through`, but that helper evaluates one script and asserts a `RangeError` with `f` run `reentry_depth + 1` times; a resumption runs only inside a checkpoint and a nested checkpoint returns at once (HOST-81), so the helper cannot pass as written; the body has to state the test's own script and assertion, and `tests/support/stack.rs` is not on Writes — crates/swell/tests/support/stack.rs:222; .engineering/planning/story/async-functions-and-await.md:145
story:promise-constructor-then-and-jobs — its test262 part holds no case that passes on the base (its own check prints `B 0`), so "no run that passed on the base fails" checks nothing, while the story changes paths every passing run goes through (the builtin dispatch through `Behaviour`/`BuiltinId`, the intrinsics, the global object, `KIND_SITES`/`trace_roots`); the part needs a guard list of passing cases — .engineering/planning/story/promise-constructor-then-and-jobs.md:116
story:promise-constructor-then-and-jobs — Bounds charges a pending promise's reactions and credits them when the box is emptied at settlement, but no test reads `heap_bytes()` across that (`a_queued_job_is_charged_until_it_is_dequeued` measures queued jobs only) — .engineering/planning/story/promise-constructor-then-and-jobs.md:96, :98
story:promise-constructor-then-and-jobs — Bounds says a handler's call from a job is one re-entry counted against `reentry_depth`, but no test reaches `reentry_depth` through a promise job (for example a `then` handler run by `cx.perform_microtask_checkpoint` from a host function at the limit) — .engineering/planning/story/promise-constructor-then-and-jobs.md:96, :98
story:job-queue-and-microtask-checkpoint — Change says a microtask run by `Cx::perform_microtask_checkpoint` re-enters above the caller's frames and counts against `reentry_depth`, but no test in `jobs.rs` reaches `reentry_depth` or `frame_depth` through such a microtask — .engineering/planning/story/job-queue-and-microtask-checkpoint.md:89, :93

**What I read:** all 4 of the 4 ids, in full (`aep plan artifact show story:<id>`). I also read the slice 7 acceptance results (rounds 1 to 3), `docs/plan/slice-10/{part.sh,check.sh}`, the code each Writes line and acceptance command cites (`agent.rs`, `limits.rs`, `builtin_function.rs`, `object/mod.rs`, `collect.rs`, `intrinsics.rs`, `run.rs`, the `swell-test262` tests, `sources.rs`, `arrow_functions.rs`, `support/stack.rs`, `requirements.txt`, the host-surface task, the test modules in `tests/main.rs`), and recounted the parts against the decomposer's copy of the base run: 853/1,675/80, 646/1,286/950, 160 `$DONE`, 6,994 and 2 all match.

**What I could not establish:**
- Whether the part counts of stories 2 and 4 (379/699 and 586/1,037) match. I did not recount them.
- Whether the HOST-187 margin holds after the Promise intrinsics. Today's reading on the join is not in the tree.
- Out of my lane: the Context of `story:job-queue-and-microtask-checkpoint` counts async cases with `$5`, but in `meta.tsv` field 5 is the negative type and the flags are field 2. The command prints 0, not 5,624 (.engineering/planning/story/job-queue-and-microtask-checkpoint.md:63).
- Out of my lane: whether `story:async-functions-and-await` (1,600 lines) could be shown in two parts, an `async` body without `await` and then `await`, belongs to scope and design.

```findings
- file: .engineering/planning/story/async-functions-and-await.md
  line: 113
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the refusal search finds only the quoted text, so it misses two doc comments that story:async-function-syntax-and-early-errors rewrites to name the held async function refusal: the comment on held in crates/syntax/src/parser/mod.rs and the doc of parse_script in crates/syntax/src/lib.rs; neither file is on this story's Writes line, so both stay wrong once the refusal is lifted"
- file: .engineering/planning/story/async-functions-and-await.md
  line: 145
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "a_completed_activation_holds_no_frame names no heap_bytes() reading, so no test shows the frame's credit when the call completes, one of the three credit paths in Bounds (this is slice 7's round-1 finding on a_completed_generator_holds_no_frame back again)"
- file: .engineering/planning/story/async-functions-and-await.md
  line: 145
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "a_suspended_activation_resumes_after_a_termination_of_another_job names no termination, so it passes with just one, while a suspended activation has to be shown at StepBudget, MemoryBudget and Interrupted each (slice 7's round-1 finding back again)"
- file: .engineering/planning/story/async-functions-and-await.md
  line: 143
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Bounds says a resumption at frame_depth gives a RangeError that rejects nothing, the activation stays suspended and the throw goes to host_clean_up_after_job, but no test in Change reaches frame_depth at a resumption (for example a checkpoint run from a host function deep in the frame stack)"
- file: .engineering/planning/story/async-functions-and-await.md
  line: 145
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "a_resumption_re_enters_on_the_promised_stack is to use reach_through, but that helper evaluates one script and asserts a RangeError with f run reentry_depth + 1 times; a resumption runs only inside a checkpoint and a nested checkpoint returns at once (HOST-81), so the helper cannot pass as written; the body has to state the test's own script and assertion, and tests/support/stack.rs is not on Writes"
- file: .engineering/planning/story/promise-constructor-then-and-jobs.md
  line: 116
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its test262 part holds no case that passes on the base (its own check prints B 0), so 'no run that passed on the base fails' checks nothing, while the story changes paths every passing run goes through (the builtin dispatch through Behaviour/BuiltinId, the intrinsics, the global object, KIND_SITES/trace_roots); the part needs a guard list of passing cases"
- file: .engineering/planning/story/promise-constructor-then-and-jobs.md
  line: 98
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Bounds charges a pending promise's reactions and credits them when the box is emptied at settlement, but no test reads heap_bytes() across that (a_queued_job_is_charged_until_it_is_dequeued measures queued jobs only)"
- file: .engineering/planning/story/promise-constructor-then-and-jobs.md
  line: 96
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Bounds says a handler's call from a job is one re-entry counted against reentry_depth, but no test reaches reentry_depth through a promise job (for example a then handler run by cx.perform_microtask_checkpoint from a host function at the limit)"
- file: .engineering/planning/story/job-queue-and-microtask-checkpoint.md
  line: 89
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Change says a microtask run by Cx::perform_microtask_checkpoint re-enters above the caller's frames and counts against reentry_depth, but no test in jobs.rs reaches reentry_depth or frame_depth through such a microtask"
```

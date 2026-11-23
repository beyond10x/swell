---
format: aep.planning-md/3
id: review-result:slice-10-later-b-design
kind: review-result
status: active
title: 'Slice 10, later stories, set B, the design critic: the five stories of orders 6 and 7'
relations:
- reviews: story:yield-star-in-async-generators
- reviews: story:async-generator-methods-in-object-literals
- reviews: story:async-generator-function-constructor
- reviews: story:array-from-async
- reviews: story:slice-10-conformance-close
revision: 1
---
needs-revision
story:yield-star-in-async-generators — blocker: Bounds and the test `a_chain_of_delegating_async_generators_longer_than_frame_depth_runs_to_its_end` claim a chain of n delegating async generators takes no Rust frame per level, but the design grants that only when [[NextMethod]] is `%GeneratorPrototype%.next`. The inner `next` of an async generator is a builtin call that resumes the frame in one re-entry per `async-generators-and-their-request-queue` (line 105), so each level nests one re-entry and the chain ends at `reentry_depth` (default 10), well below `frame_depth` (100). The story must put an async fast path to the design author as a question with a default, or state one re-entry per level with a `RangeError` at `reentry_depth` and test that instead. "No sentence is missing" is wrong. — docs/design/engine.md:950, docs/design/engine.md:956, docs/design/engine.md:967, .engineering/planning/story/yield-star-in-async-generators.md:53, .engineering/planning/story/yield-star-in-async-generators.md:73, .engineering/planning/story/yield-star-in-async-generators.md:75, crates/swell/src/host/hooks.rs:394
story:yield-star-in-async-generators — warning: the compiler bullet names `Instruction::target` and `disasm.rs` for the new instructions but not `with_target`, which `story:yield-star-delegation` names for a jump. `with_target` ends in `other => other`, so a jump instruction without its arm is silently left unpatched; the story must name `with_target` in Change and Writes. — crates/compiler/src/bytecode.rs:748, .engineering/planning/story/yield-star-in-async-generators.md:69
story:yield-star-in-async-generators — warning: it adds instructions and writes `dispatch.rs` but never states the run-loop rule (each arm calls a function of its own, nothing added to the frame of `Agent::execute`, `STACK_BYTES_PER_REENTRY` and the stack tests unchanged, or the measured change). `STACK`, `Agent::execute` and `reentry_depth` occur nowhere in the story. — .engineering/planning/story/yield-star-in-async-generators.md:71
story:array-from-async — warning: the resumption of the activation is a new re-entry path from a job, yet no named test reaches `reentry_depth` or `frame_depth` through it. The story also does not say what a refused resumption leaves (RangeError from the function the job called, activation still suspended, as the design says for an `await`). Name that outcome and add the test. — .engineering/planning/story/array-from-async.md:57, .engineering/planning/story/array-from-async.md:59, docs/design/engine.md:1190
story:async-generator-function-constructor — warning: the search that proves the removed text is gone is the line-bound `` grep -rn '`async generator` (slice 10)' ``, but the sentence it targets is hard-wrapped at the backtick, so it prints nothing on today's code. A check that prints nothing on its base shows nothing. Give a search that spans the wrap, for example `async generator` and `slice 10` as two patterns, or a multi-line grep. — crates/syntax/src/lib.rs:215, .engineering/planning/story/async-generator-function-constructor.md:36

What I read: 5 stories of set B whole, the epic, the other set-A and first-wave stories as context (the request-queue story in full, the for-await, rejection-tracker and statics edges and bodies where they bear on B), and the review results of round 1.
- I ran `aep plan artifact relations`, `graph`, `validate` (it says valid) and `waves --kind story --status draft`.
- I read engine.md decisions 3 and 4 ("Suspended frames", `yield*`, "`async` functions on suspended frames", "Queued jobs", "A built-in function with slots") and `host/limits.rs`.
- I checked the code behind each Writes line (`bytecode.rs`, `early.rs`, `parser/expression.rs`, `lib.rs`, `array/mod.rs`, `object/mod.rs`, `heap/collect.rs`, `intrinsics.rs`, `dynamic_code.rs`).
- I checked the case lists (no overlap across the 13 story lists plus `close-cross`, sum 3,866), the `Promise.*` uses in the B lists, and the 23 HOST requirements against the stories.

Edges, reads and cycles:
- I walked about 60 `depends_on` edges, including those outside the set (slice 7 and slice 5 stories, the first wave and set A).
- I found no cycle.
- The close reaches all 16 other stories of the slice, directly or transitively, and slice 5's close through slice 7's close.
- I traced about 22 acceptance reads (test modules, refusal fixtures, case lists, base runs) to producers in the set or beside it. Every one is covered by a direct or transitive edge, so I found no unrecorded hidden dependency.

What I could not establish:
- Whether `yield-star-delegation`'s final instruction shape makes the async kind need a jump or only an operand: that story is not built yet.
- Whether the hard-wrapped docs of `held` and `parse_script` make the yield-star story's search miss a line after the earlier stories rewrite them.
- Whether `waves` placing `async-generator-function-constructor` and `async-generator-methods-in-object-literals` in wave 7 and the close in wave 8, against orders 6 and 7 in `docs/plan/waves.md`, comes from file collisions. That and the hunk-by-hunk pairs on `builtin_function.rs`, `tests/main.rs` and `sources.rs` belong to parallel safety, so I did not judge them.
- Out of my lane (acceptance): `array-from-async` names no test for a termination while it waits (state kept, a later checkpoint resumes it); `async-generator-function-constructor` names no budget or interrupt test of its own parse.

```findings
- file: .engineering/planning/story/yield-star-in-async-generators.md
  line: 73
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its Bounds and the test a_chain_of_delegating_async_generators_longer_than_frame_depth_runs_to_its_end claim a chain of n delegating async generators takes no Rust frame per level, but the design grants that only for %GeneratorPrototype%.next and the inner next of an async generator is a builtin call that resumes the frame in one re-entry, so each level nests a re-entry and the chain ends at reentry_depth far below frame_depth; the story must put the async fast path to the design author as a question with a default, or state one re-entry per level with a RangeError at reentry_depth and test that"
- file: .engineering/planning/story/yield-star-in-async-generators.md
  line: 69
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the compiler bullet names Instruction::target and disasm.rs for its new instructions but not with_target, which story:yield-star-delegation names for a jump and whose match ends in a wildcard that leaves a jump without its arm silently unpatched, so the story must name with_target in Change and Writes"
- file: .engineering/planning/story/yield-star-in-async-generators.md
  line: 71
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it adds instructions and writes dispatch.rs but never states the run-loop rule: each arm calls a function of its own, nothing is added to the frame of Agent::execute, STACK_BYTES_PER_REENTRY and the stack tests stay unchanged, or the measured change is stated"
- file: .engineering/planning/story/array-from-async.md
  line: 57
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the resumption of the activation is a new re-entry path from a job, yet no named test reaches reentry_depth or frame_depth through it and the story does not say what a refused resumption leaves (RangeError from the function the job called, activation still suspended, as the design says for an await)"
- file: .engineering/planning/story/async-generator-function-constructor.md
  line: 36
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the search that proves the removed text is gone is a line-bound grep for the backticked async generator (slice 10), but the target sentence in crates/syntax/src/lib.rs is hard-wrapped between the backtick and (slice 10), so it prints nothing on today's code and shows nothing; give a search that spans the wrap"
```

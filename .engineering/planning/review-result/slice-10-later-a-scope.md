---
format: aep.planning-md/3
id: review-result:slice-10-later-a-scope
kind: review-result
status: active
title: 'Slice 10, later stories, set A, the scope critic: the eight stories of orders 4 and 5'
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

- story:async-generators-and-their-request-queue — its list holds `test/built-ins/Object/seal/seal-asyncgeneratorfunction.js` and `test/built-ins/Function/prototype/toString/AsyncGenerator.js`, which call `%AsyncGeneratorFunction%` through `.constructor` and can pass only after `story:async-generator-function-constructor` (order 6); `scan-later.awk` misses them because it matches only the word `AsyncGeneratorFunction` — the two cases belong on the `ag-constructor` list — docs/plan/slice-10/story-ag-functions.cases.txt:51 and :54, `.engineering/planning/story/async-generators-and-their-request-queue.md:81`
- story:async-generators-and-their-request-queue — the property list of `%AsyncGeneratorPrototype%` omits `constructor` (`#sec-asyncgenerator-prototype-constructor`, value `%AsyncGeneratorFunction.prototype%`), and the constructor story claims only the `constructor` of `%AsyncGeneratorFunction.prototype%`, so nothing claims it, yet `built-ins/AsyncGeneratorPrototype/constructor.js` is on this story's list — `.engineering/planning/story/async-generators-and-their-request-queue.md:103`, docs/plan/slice-10/story-ag-functions.cases.txt:2
- story:async-generators-and-their-request-queue — 8 cases on its list call `Promise.all` (`AsyncGeneratorPrototype/next/request-queue-order.js`, `request-queue-promise-resolve-order.js`, and the `this-val-not-*` cases of `next`, `return` and `throw`), but the story has no edge to `story:promise-statics-finally-and-aggregate-error`; the frontmatter needs that `depends_on`, or the cases leave the list — `.engineering/planning/story/async-generators-and-their-request-queue.md:10`, docs/plan/slice-10/story-ag-functions.cases.txt:9
- story:promise-statics-finally-and-aggregate-error — with option A the `Writes:` line omits `crates/swell/tests/errors.rs`: `every_engine_error_records_its_kind` (lines 548 to 556) compares a 7-entry `cases` array with `ErrorKind::ALL`, which gets 8 entries, so the file does not compile; the unit has to change it, and the acceptance already runs `errors::` — `crates/swell/tests/errors.rs:556`, `.engineering/planning/story/promise-statics-finally-and-aggregate-error.md:78`
- story:promise-rejection-tracker-and-promise-reads — the three new reads on `Agent` are written into only two lists of `crates/swell/tests/misuse.rs`; the `Writes:` line omits the `reads()` helper of `host_102_reads_change_nothing_and_call_no_hook` and `crates/swell/tests/fuzz.rs` (the operations of `host_3`, which feed wrong-kind handles to every read), and the first wave writes both for its new entry points — `.engineering/planning/story/promise-rejection-tracker-and-promise-reads.md:67`, `.engineering/planning/story/job-queue-and-microtask-checkpoint.md:114`
- story:for-await-and-async-from-sync-iterator — it adds new instructions and arms in `interp/dispatch.rs` but states no run-loop rule: each instruction does its work in a function of its own, nothing is added to the frame of `Agent::execute`, and `STACK_BYTES_PER_REENTRY` and the stack tests stay unchanged (or the measured change is given); `async-functions-and-await` states it — `.engineering/planning/story/for-await-and-async-from-sync-iterator.md:90`, `.engineering/planning/story/async-functions-and-await.md:152`
- story:async-generators-and-their-request-queue — the same: it adds up to four instructions and their arms in `dispatch.rs` with no run-loop rule (function of its own, `Agent::execute` frame unchanged, `STACK_BYTES_PER_REENTRY` and the stack tests unchanged) — `.engineering/planning/story/async-generators-and-their-request-queue.md:101`
- story:async-generators-and-their-request-queue — no test reaches `reentry_depth` through a path the story adds (`next`, `return` and `throw` resume the frame as one re-entry), and no test shows that a script that queues requests without end ends; only `frame_depth` and the 1,000-request charge test are named — `.engineering/planning/story/async-generators-and-their-request-queue.md:107`
- story:promise-statics-finally-and-aggregate-error — no test reaches `reentry_depth` or `frame_depth` through the paths the story adds (the iterator's `next`, `resolve` and `then` called from the statics), and no test shows that `Promise.all` over an iterable that never ends stops at the step or memory budget with the list credited; the Bounds paragraph says only "takes no depth" — `.engineering/planning/story/promise-statics-finally-and-aggregate-error.md:67` and `:69`

What I read: the epic and the eight set A stories whole, the first wave's Requirements and Writes sections, and the stories `async-generator-function-constructor` and `generator-function-constructor` in part. Also `docs/plan/waves.md` "Slice 10", `docs/plan/slice-10/` (`scan-later.awk`, `check.sh`, `part.sh`, the case lists, `later.tsv` and `scan.tsv`), `conformance/host/requirements.txt`, and the code under `crates/swell/tests` and `crates/swell/src` for the Writes checks. Commands: `aep plan artifact show` and `aep plan artifact graph`, plus `git grep`, `grep` and `sed` over the pinned test262 checkout.

Scope result: I extracted 19 epic promises and traced all 19 to a story; the 8 set A stories claim 12 of them. No gap and no reach beyond the epic.
- The 23 host requirements are split exactly: first wave 19, tracker story 4.
- The key counts match the epic: the case lists sum to the 3,866 failing cases of `scan.tsv`, and no case is on two lists.
- Two lists differ from their keys: `from-async` (75 in `later.tsv`, 73 on its list; the other 2 are in `close-cross`) and `ag-cross` (58 in `later.tsv`, 60 lines in `close-cross`, which also holds those 2). Both belong to set B and the close, so I did not judge them.

What I could not establish:
- Whether the waves are run as barriers, in which case the missing `Promise.all` edge is only a recorded dependency. The edge is still absent from the frontmatter.
- The rest of the item 4 rules (time rule, memory credits, symbol sweep, HOST-186 and HOST-187) beyond the ones in findings. They are the acceptance and design critics' lane, and I saw them present in every set A story that adds a function to a realm.
- Item 7: side-by-side pairs, except that the Writes lines of order 4 and 5 name their shared files with separate hunks. That is the parallel-safety critic's lane.

```findings
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 81
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the list holds seal-asyncgeneratorfunction.js and Function/prototype/toString/AsyncGenerator.js, which call %AsyncGeneratorFunction% through .constructor and can pass only after story:async-generator-function-constructor (order 6); scan-later.awk matches only the word AsyncGeneratorFunction, so the two cases belong on the ag-constructor list"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 103
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the property list of %AsyncGeneratorPrototype% omits constructor (#sec-asyncgenerator-prototype-constructor, value %AsyncGeneratorFunction.prototype%), no other story claims it, and built-ins/AsyncGeneratorPrototype/constructor.js is on this story's list"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 10
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "8 cases of the list call Promise.all (AsyncGeneratorPrototype next/request-queue-order.js, request-queue-promise-resolve-order.js and the this-val-not-* cases of next, return and throw) but the story has no depends_on edge to story:promise-statics-finally-and-aggregate-error; add the edge or take the cases off the list"
- file: .engineering/planning/story/promise-statics-finally-and-aggregate-error.md
  line: 78
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "with option A the Writes line omits crates/swell/tests/errors.rs, whose every_engine_error_records_its_kind compares a 7-entry cases array with ErrorKind::ALL (lines 548 to 556) and no longer compiles once ALL has 8 entries"
- file: .engineering/planning/story/promise-rejection-tracker-and-promise-reads.md
  line: 67
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Writes line omits the reads() helper of host_102_reads_change_nothing_and_call_no_hook in misuse.rs and crates/swell/tests/fuzz.rs (host_3 operations over every read with wrong-kind handles), which the three new reads on Agent must be added to, as the first wave does for its entry points"
- file: .engineering/planning/story/for-await-and-async-from-sync-iterator.md
  line: 90
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story adds instructions and their arms in dispatch.rs but states no run-loop rule: each instruction works in a function of its own, nothing is added to the frame of Agent::execute, STACK_BYTES_PER_REENTRY and the stack tests stay unchanged or the measured change is given"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 101
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story adds up to four instructions and their arms in dispatch.rs but states no run-loop rule: each works in a function of its own, nothing is added to the frame of Agent::execute, STACK_BYTES_PER_REENTRY and the stack tests stay unchanged or the measured change is given"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 107
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no test reaches reentry_depth through a path the story adds (next, return and throw resume the frame as one re-entry) and none shows that a script that queues requests without end ends"
- file: .engineering/planning/story/promise-statics-finally-and-aggregate-error.md
  line: 69
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no test reaches reentry_depth or frame_depth through the paths the statics add, and none shows that Promise.all over an iterable that never ends stops at the step or memory budget with its list credited; the Bounds paragraph says only that the statics take no depth"
```

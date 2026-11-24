---
format: aep.planning-md/3
id: review-result:slice-10-later-a-acceptance
kind: review-result
status: active
title: 'Slice 10, later stories, set A, the acceptance critic: the eight stories of orders 4 and 5'
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

story:promise-statics-finally-and-aggregate-error — With option A the Writes line omits files the acceptance cannot pass without: `crates/swell/tests/errors.rs` (`every_engine_error_records_its_kind` asserts the 7 kinds equal `ErrorKind::ALL`, and the acceptance filter runs `errors::`), the `[ErrorIntrinsics; 7]` arrays in `realm/intrinsics.rs` and `builtins/error.rs`, and the `ErrorKind::name` arm (`host/mod.rs`, outside "lines 40 to 60"); it also does not say what `Cx::new_error(ErrorKind::AggregateError, …)` makes — .engineering/planning/story/promise-statics-finally-and-aggregate-error.md:78 (code: crates/swell/tests/errors.rs:556, crates/swell/src/realm/intrinsics.rs:158, crates/swell/src/host/mod.rs:71)
story:ess-target-checkpoint-and-promise-hooks — The four declarations of `spec/domains/host.yaml` are checked only by "`slot task validate` exits 0", which holds before the work as after, so the acceptance reads the same on both sides of the change — .engineering/planning/story/ess-target-checkpoint-and-promise-hooks.md:68 (Change, line 54)
story:ess-target-checkpoint-and-promise-hooks — The acceptance says the task's "planted-defect check names `a_target_whose_checkpoint_runs_no_job_fails_a_scenario` passed", but the task checks one named test only and the Writes line limits the taskfile change to the floor and `ESS_UNSUPPORTED_MAX` — taskfiles/conform-engine.yml:61 (story :64, :68)
story:ess-target-checkpoint-and-promise-hooks — Change raises the floor and lowers `ESS_UNSUPPORTED_MAX` "to the count the unit measures", but the acceptance checks only "more passed and fewer unsupported", so a floor left at 14 and a maximum left at 44 pass it — .engineering/planning/story/ess-target-checkpoint-and-promise-hooks.md:68 (Change, line 54)
story:async-generators-and-their-request-queue — No test reaches `reentry_depth` or `frame_depth` through the resumption by `next`, `return` and `throw` that this story adds; only `a_chain_of_yields_longer_than_frame_depth_runs_to_its_end` is named, while the generators have `a_resumption_at_frame_depth_throws_a_range_error_and_leaves_the_generator_suspended` and `a_generator_resuming_a_generator_reaches_reentry_depth` — .engineering/planning/story/async-generators-and-their-request-queue.md:107 (crates/swell/tests/generators.rs:654, :679)
story:async-generators-and-their-request-queue — 8 cases of its list use `Promise.all` (for example `test/built-ins/AsyncGeneratorPrototype/next/request-queue-order.js:47`), but its relations have no edge to `story:promise-statics-finally-and-aggregate-error`, so acceptance (a) "every run of the list passes" depends on a story it does not name — .engineering/planning/story/async-generators-and-their-request-queue.md:10
story:async-generators-and-their-request-queue — The sweep sentence "the unit reports whether it has other roots" is a condition the code answers: the sweep starts from the global object alone, so the story must say it adds roots or rows for the new prototypes, or that they stay unreached — .engineering/planning/story/async-generators-and-their-request-queue.md:107 (crates/swell/tests/builtins_symbol_values.rs:350)
story:async-generators-and-their-request-queue — Check (b) "prints `0`" does not say what it prints on its base, and the same command prints 0 on the property-escapes run the story names, so it cannot tell the transition — .engineering/planning/story/async-generators-and-their-request-queue.md:122
story:async-generator-syntax-and-early-errors — It says `story:async-generators-and-their-request-queue` "takes out" the `admitted` arm, but that story says the arm "stays and counts the new held refusal", so the two bodies give a unit opposite instructions — .engineering/planning/story/async-generator-syntax-and-early-errors.md:56 (the other story, Change, first bullet)
story:async-generator-syntax-and-early-errors — Check (b) "prints `0`" does not say what it prints on its base (the refusal at the `*` ends every run of the list there), so the count that moves is not stated — .engineering/planning/story/async-generator-syntax-and-early-errors.md:76
story:for-await-and-async-from-sync-iterator — The acceptance "the search of Context prints nothing", but the search also matches `docs/design/engine.md:1431` ("`Symbol.asyncIterator` comes with slice 10"), which the Context base list omits and no Writes line changes — .engineering/planning/story/for-await-and-async-from-sync-iterator.md:68 (docs/design/engine.md:1431)
story:for-await-and-async-from-sync-iterator — Check (b) "prints `0`" does not say what it prints on its base, and the same command prints 0 on the property-escapes run the story names (that text is "async function is not supported") — .engineering/planning/story/for-await-and-async-from-sync-iterator.md:111
story:for-await-and-async-from-sync-iterator — "the unit reports whether the sweep has roots beside the global object" is a condition the code answers (one root, the global object), so the story must say it adds roots or rows for `%AsyncIteratorPrototype%` and `%AsyncFromSyncIteratorPrototype%` — .engineering/planning/story/for-await-and-async-from-sync-iterator.md:94 (crates/swell/tests/builtins_symbol_values.rs:350)
story:async-function-constructor — The bounds are shown by one test, `async_function_constructor_charges_its_parse_and_reads_the_interrupt`, with no quantity or budget named, while `Function` has separate tests for step budget, memory budget, `reentry_depth` and parser nesting that the async kind does not repeat — .engineering/planning/story/async-function-constructor.md:61 (crates/swell/tests/dynamic_function.rs:148, :174, :221, :245)
story:async-function-constructor — "the unit adds `%AsyncFunction%` to its roots if the sweep has a list of intrinsics that are not globals" is a condition the code answers (the sweep has none), so the acceptance cannot show `%AsyncFunction%` is swept unless the story says it adds a root — .engineering/planning/story/async-function-constructor.md:61 (crates/swell/tests/builtins_symbol_values.rs:350)

What I read: 8 of 8 stories of set A (`aep plan artifact show` for each), the criteria in the critic procedure and rubric, `docs/plan/slice-10/{check.sh,part.sh}`, the case lists and `waves.md` "Slice 10". I checked the Writes lines and the named greps against the code and design on the branch, and the case lists against the pinned test262 checkout. The Writes line, bounds and sweep findings come from the brief's rules; none of the stories is a story-by-story approve.

What I could not establish:
- The `passed N bound N B F R` figures in the acceptances: no `runs.tsv` is in the worktree and I built and ran nothing. I checked only that each part's case total equals list plus guard where that is derivable.
- Whether `async-methods-in-object-literals` and `promise-rejection-tracker-and-promise-reads` hold no further acceptance defect. I found none in my lane, so they carry no finding. Both state their search, base counts and check, and tests in both collection modes.
- Out of my lane, for the design critic or the drafter:
  - `story:promise-rejection-tracker-and-promise-reads` cites `docs/design/host-interface.md` lines 1187, 1209, 1296, 1298, 1638, 1649 and 1680, none of which hold what it names on this branch. HOST-100 and HOST-101 are at 1415 and 1417, `PromiseState` at 1306, the reads at 1328 to 1330.
  - `story:promise-statics-finally-and-aggregate-error` cites line 1198 for "Reading error objects", which is the interrupt text.
  - File overlap with `yield-star-delegation` and `generator-function-constructor` was not judged.

```findings
- file: .engineering/planning/story/promise-statics-finally-and-aggregate-error.md
  line: 78
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "With option A the Writes line omits files the acceptance cannot pass without: crates/swell/tests/errors.rs (every_engine_error_records_its_kind asserts the 7 kinds equal ErrorKind::ALL; the acceptance filter runs errors::), the [ErrorIntrinsics; 7] arrays in realm/intrinsics.rs and builtins/error.rs, and the ErrorKind::name arm in host/mod.rs; it also does not say what Cx::new_error(ErrorKind::AggregateError) makes"
- file: .engineering/planning/story/ess-target-checkpoint-and-promise-hooks.md
  line: 68
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The four declarations of spec/domains/host.yaml are checked only by 'slot task validate exits 0', which holds before the work as after, so the acceptance reads the same on both sides of the change"
- file: taskfiles/conform-engine.yml
  line: 61
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The acceptance says the planted-defect check names a_target_whose_checkpoint_runs_no_job_fails_a_scenario passed, but the task checks one named test only and the Writes line limits the taskfile change to the floor and ESS_UNSUPPORTED_MAX"
- file: .engineering/planning/story/ess-target-checkpoint-and-promise-hooks.md
  line: 68
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Change raises the floor and lowers ESS_UNSUPPORTED_MAX to the measured counts, but the acceptance checks only 'more passed and fewer unsupported', so a floor left at 14 and a maximum left at 44 pass it"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 107
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "No test reaches reentry_depth or frame_depth through the resumption by next, return and throw that this story adds; only a_chain_of_yields_longer_than_frame_depth_runs_to_its_end is named, while generators have a_resumption_at_frame_depth_throws_a_range_error_and_leaves_the_generator_suspended and a_generator_resuming_a_generator_reaches_reentry_depth"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 10
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "8 cases of its list use Promise.all (test/built-ins/AsyncGeneratorPrototype/next/request-queue-order.js:47 among them) but its relations have no edge to story:promise-statics-finally-and-aggregate-error, so acceptance (a) 'every run of the list passes' depends on a story it does not name"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 107
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The sweep sentence 'the unit reports whether it has other roots' is a condition the code answers: the sweep starts from the global object alone (builtins_symbol_values.rs:350), so the story must say it adds roots or rows for the new prototypes or that they stay unreached"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 122
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Check (b) 'prints 0' does not say what it prints on its base, and the same command prints 0 on the property-escapes run the story names, so it cannot tell the transition"
- file: .engineering/planning/story/async-generator-syntax-and-early-errors.md
  line: 56
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "It says story:async-generators-and-their-request-queue takes out the admitted arm, but that story says the arm stays and counts the new held refusal, so the two bodies give a unit opposite instructions"
- file: .engineering/planning/story/async-generator-syntax-and-early-errors.md
  line: 76
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Check (b) 'prints 0' does not say what it prints on its base (the refusal at the * ends every run of the list there), so the count that moves is not stated"
- file: .engineering/planning/story/for-await-and-async-from-sync-iterator.md
  line: 68
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The acceptance says the search of Context prints nothing, but the search also matches docs/design/engine.md:1431 ('Symbol.asyncIterator comes with slice 10'), which the Context base list omits and no Writes line changes"
- file: .engineering/planning/story/for-await-and-async-from-sync-iterator.md
  line: 111
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Check (b) 'prints 0' does not say what it prints on its base, and the same command prints 0 on the property-escapes run the story names (that text is 'async function is not supported')"
- file: .engineering/planning/story/for-await-and-async-from-sync-iterator.md
  line: 94
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'the unit reports whether the sweep has roots beside the global object' is a condition the code answers (one root, the global object), so the story must say it adds roots or rows for %AsyncIteratorPrototype% and %AsyncFromSyncIteratorPrototype%"
- file: .engineering/planning/story/async-function-constructor.md
  line: 61
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The bounds are shown by one test, async_function_constructor_charges_its_parse_and_reads_the_interrupt, with no quantity or budget named, while Function has separate tests for step budget, memory budget, reentry_depth and parser nesting that the async kind does not repeat"
- file: .engineering/planning/story/async-function-constructor.md
  line: 61
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'the unit adds %AsyncFunction% to its roots if the sweep has a list of intrinsics that are not globals' is a condition the code answers (the sweep has none), so the acceptance cannot show %AsyncFunction% is swept unless the story says it adds a root"
```

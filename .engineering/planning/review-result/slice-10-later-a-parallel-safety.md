---
format: aep.planning-md/3
id: review-result:slice-10-later-a-parallel-safety
kind: review-result
status: active
title: 'Slice 10, later stories, set A, the parallel-safety critic: the eight stories of orders 4 and 5'
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
story:promise-rejection-tracker-and-promise-reads — its acceptance `grep -c 'pending slice 10$'` prints 0 only after lines 194 and 195, which story:async-functions-and-await moves, but its one edge is to promise-constructor-then-and-jobs (which leaves 6 pending, not the 4 Context states), so add `depends_on story:async-functions-and-await` and correct the count — .engineering/planning/story/promise-rejection-tracker-and-promise-reads.md:71, .engineering/planning/story/async-functions-and-await.md:162
story:promise-statics-finally-and-aggregate-error — with option A its Writes line omits `crates/swell/tests/errors.rs`, whose `every_engine_error_records_its_kind` asserts `cases.map(..) == ErrorKind::ALL` over 7 entries (the test no longer compiles with 8 variants) and whose `ERRORS: [&str; 7]` feeds two loops, while its acceptance runs `errors::` — crates/swell/tests/errors.rs:556, .engineering/planning/story/promise-statics-finally-and-aggregate-error.md:78
story:for-await-and-async-from-sync-iterator — its Writes line and acceptance omit `crates/swell/tests/builtins_symbol.rs` (asserts twelve names and `Symbol.asyncIterator === undefined`) and `crates/swell/tests/builtins_object_enumeration.rs` (expects the twelve `global.Symbol.*` paths), both of which fail once the thirteenth well-known symbol exists, and its search pattern does not find them — crates/swell/tests/builtins_symbol.rs:192, crates/swell/tests/builtins_object_enumeration.rs:624, .engineering/planning/story/for-await-and-async-from-sync-iterator.md:105
story:for-await-and-async-from-sync-iterator — it names `iterator_complete` being made `pub(crate)` by story:yield-star-delegation but does not say whether its check-of-an-iterator-result instruction (work in `interp/for_of.rs`) calls the private `iterator_complete` and `iterator_value` of `ops/iteration.rs`, so either it needs an edge to that story or it makes both `pub(crate)` itself in the same lines — crates/swell/src/ops/iteration.rs:129, .engineering/planning/story/for-await-and-async-from-sync-iterator.md:105
story:async-function-constructor — it says `crates/syntax/src/lib.rs` is shared with "no other story of its wave" and names no sharing of `crates/syntax/src/parser/mod.rs`, but story:async-generator-syntax-and-early-errors (and story:async-generators-and-their-request-queue) write the `parse_script` and `held` documentation in both files, with no edge to this story: name the pair with its hunks, or order them, or split the surface (cited, both Writes lines) — .engineering/planning/story/async-function-constructor.md:70, .engineering/planning/story/async-generator-syntax-and-early-errors.md:70
story:async-function-constructor — it adds `%AsyncFunction%` "to its roots if the sweep has a list of roots", but the sweep has one root (`vec![(global, ..)]`) and story:for-await-and-async-from-sync-iterator and story:async-generators-and-their-request-queue each plan to add roots to the same line of `functions_of_the_realm`, with none naming the others for `builtins_symbol_values.rs`: name one story that adds the root list the others append to, or order them (cited, three bodies) — crates/swell/tests/builtins_symbol_values.rs:350, .engineering/planning/story/async-function-constructor.md:61
story:async-generators-and-their-request-queue — it writes `object/builtin_function.rs`, `realm/intrinsics.rs` and `builtins/mod.rs` (new variants, intrinsics, a `mod` line) as story:promise-statics-finally-and-aggregate-error and story:async-function-constructor do, with no edge to either, yet its Writes line says shared with its wave: no file and names neither (cited, three Writes lines) — .engineering/planning/story/async-generators-and-their-request-queue.md:116, .engineering/planning/story/promise-statics-finally-and-aggregate-error.md:78
story:async-methods-in-object-literals — its conditional edits of `instantiate_function` in `object/ecmascript_function.rs` and of `compiler/src/expression.rs` fall on the same function and file that story:async-generators-and-their-request-queue writes (the async generator branch of `instantiate_function`, the `Expr::Yield` arm) with no edge between them, and this story's line says expression.rs has no other writer (cited, both Writes lines) — .engineering/planning/story/async-methods-in-object-literals.md:80, .engineering/planning/story/async-generators-and-their-request-queue.md:116
story:async-generators-and-their-request-queue — it records its held refusal of `yield*` inside the function of YieldExpression (`yield_delegate`, where the slice 7 refusal sits today) and adds an `Expr::Yield` arm, and story:yield-star-delegation rewrites both and lands first, yet it says "needs no open story of slice 7" and has no edge to it: add `depends_on story:yield-star-delegation` or state which base its hunks are written against — crates/syntax/src/parser/expression.rs:279, .engineering/planning/story/async-generators-and-their-request-queue.md:116
story:async-generators-and-their-request-queue — its acceptance (a) needs 8 cases of its list to pass that call `Promise.all`, which story:promise-statics-finally-and-aggregate-error creates, and no `depends_on` edge runs to it: add the edge or take the 8 cases off the list (`AsyncGeneratorPrototype/next/request-queue-order.js:47`, `request-queue-promise-resolve-order.js:54`, `{next,return,throw}/this-val-not-*.js`) — .engineering/planning/story/async-generators-and-their-request-queue.md:81, .cache/test262/2a1ade303f72d3de80565c921d685222405c32ce/test/built-ins/AsyncGeneratorPrototype/next/request-queue-order.js:47
story:async-generators-and-their-request-queue — its list holds 36 `for-await-of/async-func-dstr-*` cases whose text has `yield*` inside an `async function*` (`later.tsv` field 3 `yield-star`), which pass only with story:yield-star-in-async-generators, which depends on this story, so no edge can fix it: move the 36 cases to that story's list and correct the 1,162 and `F 1162 R 1162` — docs/plan/slice-10/later.tsv:1 (`awk -F'\t' '$2=="ag-functions" && $3 ~ /yield-star/' docs/plan/slice-10/later.tsv` prints 36)

**What you read:** 17 stories (my 8 whole, the 4 first-wave and 5 set B Writes and shared-file paragraphs), `aep plan artifact graph`, `docs/plan/waves.md` "Slice 10", `docs/plan/slice-10/{later.tsv,scan-later.awk}` and the case lists, `conformance/host/requirements.txt`, and the code at the files each Writes line names (`ErrorKind` users, `IterationKind` sites, `ObjectKind`/`Intrinsic`/`BuiltinId` sites, `instantiate_function`, `accessor_kind`, `parse_dynamic_function`, the symbol sweep, `builtins_symbol.rs`, `errors.rs`).

**What you could not establish:**
- Surfaces: 8 cited, 0 inferred, 0 unplaced.
- Acceptance reads traced to a producer in the set or the first wave: 18, of which 15 are recorded by a `depends_on` edge and 3 are not (finding 1, and findings 10 and 11, of which 11 cannot take an edge).
- Pairs of one order with a collision that is named in both bodies and in hunks apart, so not findings: story:promise-statics-finally-and-aggregate-error with story:promise-rejection-tracker-and-promise-reads (`host/mod.rs`); story:async-methods-in-object-literals, story:for-await-and-async-from-sync-iterator and story:async-generator-syntax-and-early-errors (`early.rs`, `sources.rs`, `parser/expression.rs`, `parser/statement.rs`); story:promise-statics-finally-and-aggregate-error, story:async-function-constructor and story:for-await-and-async-from-sync-iterator (`builtin_function.rs`, `intrinsics.rs`, `builtins/mod.rs`).
- story:ess-target-checkpoint-and-promise-hooks: no collision found.
- Set B: story:array-from-async shares `builtin_function.rs` and `builtins_symbol_values.rs` with story:promise-statics-finally-and-aggregate-error and story:async-function-constructor with no edge and does not name them.
- Out of my lane, which did not set the verdict: no story that adds an instruction (story:for-await-and-async-from-sync-iterator, story:async-generators-and-their-request-queue) states the run-loop rule (`Agent::execute` frame, `STACK_BYTES_PER_REENTRY`), and none says a test reaches `reentry_depth` through its new paths; under option A, `global_object.rs` line 42 already installs `AggregateError` through the `ErrorKind::ALL` loop, while the story names an explicit entry at lines 64 to 77.

```findings
- file: .engineering/planning/story/promise-rejection-tracker-and-promise-reads.md
  line: 71
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance grep -c 'pending slice 10$' prints 0 only after lines 194 and 195, which story:async-functions-and-await moves, but its one edge is to promise-constructor-then-and-jobs (which leaves 6 pending, not the 4 Context states), so add depends_on story:async-functions-and-await and correct the count"
- file: crates/swell/tests/errors.rs
  line: 556
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "with option A the Writes line of story:promise-statics-finally-and-aggregate-error omits crates/swell/tests/errors.rs, whose every_engine_error_records_its_kind asserts cases.map(..) == ErrorKind::ALL over 7 entries and whose ERRORS array of 7 feeds two loops, while its acceptance runs errors::"
- file: crates/swell/tests/builtins_symbol.rs
  line: 192
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:for-await-and-async-from-sync-iterator omits from its Writes line and acceptance builtins_symbol.rs (asserts twelve names and Symbol.asyncIterator === undefined) and builtins_object_enumeration.rs (expects the twelve global.Symbol.* paths at line 624), both of which fail once the thirteenth well-known symbol exists, and its search pattern does not find them"
- file: crates/swell/src/ops/iteration.rs
  line: 129
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:for-await-and-async-from-sync-iterator names iterator_complete being made pub(crate) by story:yield-star-delegation but does not say whether its check-of-an-iterator-result instruction in interp/for_of.rs calls the private iterator_complete and iterator_value, so either it needs an edge to that story or it makes both pub(crate) itself in the same lines"
- file: .engineering/planning/story/async-function-constructor.md
  line: 70
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its Writes line says crates/syntax/src/lib.rs is shared with no other story of its wave and names no sharing of parser/mod.rs, but story:async-generator-syntax-and-early-errors and story:async-generators-and-their-request-queue write the parse_script and held documentation in both files with no edge to it: name the pair with its hunks, or order them, or split the surface (cited, both Writes lines)"
- file: crates/swell/tests/builtins_symbol_values.rs
  line: 350
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:async-function-constructor, story:for-await-and-async-from-sync-iterator and story:async-generators-and-their-request-queue each plan to add roots to the one-root queue of functions_of_the_realm, none naming the others for this file: name one story that adds the root list the others append to, or order them (cited, three bodies)"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 116
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it writes object/builtin_function.rs, realm/intrinsics.rs and builtins/mod.rs as story:promise-statics-finally-and-aggregate-error and story:async-function-constructor do, with no edge to either, yet its Writes line says shared with its wave: no file and names neither (cited, three Writes lines)"
- file: .engineering/planning/story/async-methods-in-object-literals.md
  line: 80
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its conditional edits of instantiate_function in object/ecmascript_function.rs and of compiler/src/expression.rs fall on the same function and file that story:async-generators-and-their-request-queue writes with no edge between them, and this story says expression.rs has no other writer (cited, both Writes lines)"
- file: crates/syntax/src/parser/expression.rs
  line: 279
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:async-generators-and-their-request-queue records its held yield* refusal inside the function of YieldExpression and adds an Expr::Yield arm that story:yield-star-delegation rewrites and lands first, yet it says it needs no open story of slice 7 and has no edge to it: add depends_on story:yield-star-delegation or state the base its hunks are written against"
- file: .engineering/planning/story/async-generators-and-their-request-queue.md
  line: 81
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance (a) needs 8 cases of its list to pass that call Promise.all (AsyncGeneratorPrototype next/request-queue-order.js:47, request-queue-promise-resolve-order.js:54 and the this-val-not-* cases of next, return and throw), which story:promise-statics-finally-and-aggregate-error creates, and no depends_on edge runs to it: add the edge or take the 8 cases off the list"
- file: docs/plan/slice-10/later.tsv
  line: 1
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the list of story:async-generators-and-their-request-queue holds 36 for-await-of/async-func-dstr-* cases with yield* inside an async function*, which pass only with story:yield-star-in-async-generators, which depends on it, so no edge can fix it: move the 36 cases to that story's list and correct the 1,162 and F 1162 R 1162 (awk -F'\\t' '$2==\"ag-functions\" && $3 ~ /yield-star/' prints 36)"
```

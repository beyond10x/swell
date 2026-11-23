---
format: aep.planning-md/3
id: story:for-await-and-async-from-sync-iterator
kind: story
status: draft
title: for await, the async iterator protocol, %AsyncFromSyncIteratorPrototype%, %AsyncIteratorPrototype% and Symbol.asyncIterator
summary: for-await-of in async functions with AsyncIteratorClose, GetIterator with kind async, CreateAsyncFromSyncIterator and the thirteenth well-known symbol; 420 cases.
relations:
- decomposes: epic:slice-10-promises-and-async
- depends_on: story:async-functions-and-await
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/statement.rs
- confidence: inferred
  path: crates/compiler/tests/sources/for-await.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/for-await.js
- confidence: inferred
  path: crates/swell/src/builtins/async_iteration.rs
- confidence: cited
  path: crates/swell/src/builtins/mod.rs
- confidence: cited
  path: crates/swell/src/builtins/symbol.rs
- confidence: cited
  path: crates/swell/src/heap/collect.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/for_of.rs
- confidence: cited
  path: crates/swell/src/object/builtin_function.rs
- confidence: cited
  path: crates/swell/src/object/mod.rs
- confidence: cited
  path: crates/swell/src/ops/iteration.rs
- confidence: cited
  path: crates/swell/src/realm/intrinsics.rs
- confidence: cited
  path: crates/swell/src/realm/mod.rs
- confidence: cited
  path: crates/swell/tests/builtins_symbol_values.rs
- confidence: inferred
  path: crates/swell/tests/for_await.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/parser/statement.rs
- confidence: cited
  path: crates/syntax/tests/sources.rs
- confidence: cited
  path: crates/syntax/tests/sources/refused/for-await.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/for-await.js
revision: 26
---
## Context

Slice 10 brings jobs, `Promise`, `async` functions and generators and `for await` (`epic:slice-10-promises-and-async`; `docs/design/engine.md`, section 12, item 10). This story brings `for await` in an `async` function (`#sec-for-in-and-for-of-statements`, ForIn/OfHeadEvaluation with iterationKind async-iterate, ForIn/OfBodyEvaluation with iteratorKind async, AsyncIteratorClose), the async iterator protocol (GetIterator with kind async, `#sec-getiterator`; CreateAsyncFromSyncIterator and `%AsyncFromSyncIteratorPrototype%`, `#sec-%asyncfromsynciteratorprototype%-object`), `%AsyncIteratorPrototype%` (`#sec-asynciteratorprototype`) and `%Symbol.asyncIterator%`. The async generators of this slice need all of it (396 cases of their list use `for await`), and so does `Array.fromAsync`. It depends on `story:async-functions-and-await` (each `await` of the loop is that story's `await`, with its two functions and its job) and needs no open story of slice 7.

The refusal this story removes: `for await is not supported before slice 10`, which `story:async-function-syntax-and-early-errors` gave `for await` inside an `async` function (at the `await` of `for_head`, `crates/syntax/src/parser/statement.rs` line 701 on `397ffff`). The text leaves the code. The search for what expects the text or says that `for await` and `%Symbol.asyncIterator%` wait, `grep -rn 'for await is not supported\|"for await"\|for await` waits for slice 10\|comes with slice 10' crates conformance docs/design taskfiles tests website/docs`, prints on its base the line of `for_head`, line 1 of `crates/syntax/tests/sources/refused/for-await.js`, line 8 of `statement.rs` (the module documentation), line 33 of `crates/swell/src/realm/mod.rs` and line 16 of `crates/swell/src/builtins/symbol.rs`; after this story it prints nothing. The documentation of `for_head` (line 696: "`async of` stays refused with `async` (slice 10)") is reworded: `for (async of x)` is a `SyntaxError` by the lookahead of the grammar, and `for await (async of x)` is allowed.

The list of this story is `docs/plan/slice-10/story-for-await.cases.txt`: the 420 cases of the key `for-await` of `docs/plan/slice-10/later.tsv` (the command is in the head of `docs/plan/slice-10/scan-later.awk`: `for await`, `Symbol.asyncIterator` or the async iteration objects with no async generator, and the negative parse cases of the key `async-generators` with no async generator in their text). By directory: `language/statements/for-await-of` 413, `built-ins/AsyncFromSyncIteratorPrototype` 5, `built-ins/Symbol` 1 (`asyncIterator`), `language/expressions/optional-chaining` 1; 34 are negative parse cases and 385 have the flag `async`. They have 815 runs in the base run of the property-escapes join (the newest full run, made before generators and the flag `i` were merged and before the first wave of this slice; 18,935 of 44,812 cases pass): 813 fail on `async function is not supported before slice 10` (the text before the first wave), 2 on a `Test262Error` that reads `typeof Symbol.asyncIterator`. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on, by name (the text of the slice, merged at `843f178`): `docs/design/engine.md` decision 3, "`async` functions on suspended frames", items 3 to 5 (each `await` of the loop), "The microtask checkpoint is one entry"; decision 4, "Symbols" (line 1377: the well-known symbols are made with the agent, one per agent) and "A built-in function with slots" (the functions of AsyncFromSyncIteratorContinuation); "Depth bounds" (the parser's nesting counter). One sentence is missing and is put to the design author as a question of this story (the story follows the answer): `%Symbol.asyncIterator%` joins `WellKnown` (`crates/swell/src/realm/mod.rs` line 34; the field documentation of line 30 says it "comes with slice 10") as the thirteenth well-known symbol, which `engine.md` line 1122 and `feature Symbol.asyncIterator 10` (line 4127 on `397ffff`) presuppose but no sentence states; the default is that sentence.

## What later stories reuse

GetIterator with kind async and `%AsyncFromSyncIteratorPrototype%`, which `yield*` in an async generator (`story:yield-star-in-async-generators`) and `Array.fromAsync` (`story:array-from-async`) call; `%AsyncIteratorPrototype%`, the [[Prototype]] of `%AsyncGeneratorPrototype%` (`story:async-generators-and-their-request-queue`); the loop instructions, which `for await` in an async generator runs as they are.

## Domain relations

- Async-from-sync iterator → its sync iterator record: one to one, owned; the object holds [[SyncIteratorRecord]] and is never handed to a script but through the loop and `yield*` — inferable (inferred from ECMA-262 `#sec-createasyncfromsynciterator` and `#sec-properties-of-async-from-sync-iterator-instances`; no ess/1 document declares it: the host model `spec/domains/host.yaml` holds no engine value, and ESS does not restate the slots of ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base (`for await` is refused at parse; `Symbol.asyncIterator` is undefined).

- `swell-syntax`. `crates/syntax/src/ast.rs`: a variant `AsyncIterate` of `IterationKind` (line 503) with its arms in every match over it without a wildcard (the unit lists them with `git grep -n 'IterationKind::' crates`). `crates/syntax/src/parser/statement.rs`, `for_head` (line 698): `for await (` where [Await] is set reads a for-of head only (`for await (x in y)` and `for await (;;)` are grammar errors), with `async of` allowed after `for await (`; outside [Await] `for await` stays a grammar error; the module documentation (line 8) and the documentation of `for_head` reworded (Context). `crates/syntax/src/early.rs`: the early errors of a for-await-of head as of a for-of head (`#sec-for-in-and-for-of-statements-static-semantics-early-errors`). Fixtures: `refused/for-await.js` removed; `valid/for-await.js` (new: declarations, patterns, `async of`, a label, `break` and `continue`), `early-errors/for-await-*.js` (new, one per early error and grammar error above). `crates/syntax/tests/sources.rs`: the nesting kind `for_await` (`for await (x of y) for await (x of y) ...` in an `async` function) in `NESTING_KINDS` and `nested`; a level over `STACK_BYTES_PER_LEVEL` gets its path split into functions, never a new value of the constant.

- `swell-compiler`. `crates/compiler/src/statement.rs`, `for_of` (line 738): with `AsyncIterate`, GetIterator(v, async), and for each step the call of `next`, the `await` of `story:async-functions-and-await` on its result, then the check of an object and of `done` (`#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`, steps for iteratorKind async), and on every abrupt exit AsyncIteratorClose (`#sec-asynciteratorclose`: the call of `return`, the `await` of its result, the check of an object). `crates/compiler/src/bytecode.rs`: at most three new instructions (GetIterator with kind async; the check of an iterator result; the call of `return` for the close), each documented with its stack effect and its clause, with its arm in `Instruction::target` (no wildcard); `disasm.rs` their text. Fixtures `crates/compiler/tests/sources/for-await.{js,disasm}` (new).

- `swell`. `crates/swell/src/interp/for_of.rs` and `interp/dispatch.rs`: the arms of the new instructions. `crates/swell/src/ops/iteration.rs`: GetIterator with kind async (`get_iterator`, line 89: the method of `%Symbol.asyncIterator%`, else CreateAsyncFromSyncIterator of the sync iterator) and AsyncIteratorClose. `crates/swell/src/builtins/async_iteration.rs` (new; its `mod` line in `builtins/mod.rs`): `%AsyncIteratorPrototype%` with `[%Symbol.asyncIterator%]` returning `this`; `%AsyncFromSyncIteratorPrototype%` with `next`, `return` and `throw` (`#sec-%asyncfromsynciteratorprototype%.next` and the two after it) and AsyncFromSyncIteratorContinuation (`#sec-asyncfromsynciteratorcontinuation`: PromiseResolve of the value, the function that unwraps it and the function that closes the sync iterator on a rejection, built-in functions with slots, then PerformPromiseThen); the object kind `ObjectKind::AsyncFromSyncIterator` (`crates/swell/src/object/mod.rs`: its variant, tag and arms; its kind site; [[SyncIteratorRecord]] traced), and its site in `KIND_SITES` of `crates/swell/src/heap/collect.rs` (18 after `story:async-functions-and-await`, 19 after) and the record of every kind; the `BuiltinId` variants and the slots in `crates/swell/src/object/builtin_function.rs`; `Intrinsic::AsyncIteratorPrototype` and `Intrinsic::AsyncFromSyncIteratorPrototype` in `crates/swell/src/realm/intrinsics.rs` (the enum, `get`, `trace`, `create_intrinsics` and its list). `%Symbol.asyncIterator%`: a field `async_iterator` of `WellKnown` (`crates/swell/src/realm/mod.rs`), made with the others, its description `Symbol.asyncIterator`, and the property `Symbol.asyncIterator` of `%Symbol%` (`crates/swell/src/builtins/symbol.rs`); the two documentation lines of Context reworded.

- Bounds. Steps: each step of the loop is the call of `next` and one `await` (its step, its job, its resumption: 3 steps when the body runs again, item 4); the continuation of `%AsyncFromSyncIteratorPrototype%` makes two functions and one reaction within the step of its call. Depth: each iteration is resumed by a job of its own, so a loop of any length takes no depth. Memory: the async-from-sync iterator, the functions and the reactions are charged as objects and reactions are; a loop whose `next` never settles holds what an `await` that never ends holds. Termination: as for every `await` (item 5): a termination while the loop body runs leaves the function's promise pending for good and closes no iterator; a loop suspended at its `await` keeps everything and resumes at a later checkpoint.

- Tests, each in both collection modes, in `crates/swell/tests/for_await.rs` (new): `for_await_over_an_async_iterable_awaits_each_result`, `for_await_over_a_sync_iterable_awaits_each_value` (CreateAsyncFromSyncIterator: a promise value is unwrapped), `a_break_in_for_await_closes_the_iterator_and_awaits_its_return`, `a_throw_in_for_await_closes_the_iterator_and_keeps_the_throw`, `a_rejected_value_of_a_sync_iterator_closes_it` (the close of AsyncFromSyncIteratorContinuation), `a_for_await_longer_than_frame_depth_runs_to_its_end` (1,000 iterations, below the loop bound), `a_termination_in_the_body_of_for_await_closes_nothing_and_leaves_the_promise_pending` (for `StepBudget`, `MemoryBudget` and `Interrupted`), `symbol_async_iterator_is_one_well_known_symbol_per_agent` (the same symbol in two realms of one agent; `Symbol.for` gives another). The sweep of every function with a Symbol (`crates/swell/tests/builtins_symbol_values.rs`) reaches `Symbol.asyncIterator` from the global object; `%AsyncIteratorPrototype%` and `%AsyncFromSyncIteratorPrototype%` are not reachable from it, and the unit reports whether the sweep has roots beside the global object to which they are added. `object::generator::tests::a_generator_keeps_the_object_record_at_its_size` is run in both modes and passes as it is. `host_objects::host_186_a_host_function_charges_every_byte` and `host_objects::host_187_a_realm_of_300_host_functions_and_40_host_objects` are run in both modes and the reading of HOST-187 is reported (a symbol and two intrinsics more per realm). `crates/swell/tests/main.rs` generated again (`sh taskfiles/test-main.sh write`).

- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with a collection at every safe point and under 2.5 at its full count (`SWELL_FULL_COUNTS=1`); measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system). The unit times each test before it adds it and names each one over 1 CPU second with its time; no loop of a test runs more than 1,000 iterations; the tests of this story are expected to add under 3 CPU seconds to the gate in both modes together, and the unit reports the sum it measured (the test steps of the gate cost about 330 CPU seconds). A product several tests share is built once inside one test function or is a committed file, never in a `static`; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, the most); a script that measures or waits runs without the prefix. No Python in any tool, test or command; no `cd ... &&` with a write or a delete; intermediate files in the scratch directory.
- The parser part: each part of `every_nesting_kind_fits_the_stack_bound` is timed before and after at the gate's count and at the full count (`SWELL_FULL_COUNTS=1`) and both are reported.

- Size: about 1,300 lines (the syntax 150, the compiler 200, the interpreter and the operations 200, the builtins and the symbol 250, the tests and fixtures 500): one long session. Not cut further: `for await` over a sync iterable needs `%AsyncFromSyncIteratorPrototype%`, and the 5 cases of that object run through `for await`.

## Requirements

None of `conformance/host/requirements.txt`.

Writes: `crates/syntax/src/ast.rs` (`IterationKind`), `crates/syntax/src/parser/statement.rs` (`for_head` and the module documentation), `crates/syntax/src/early.rs` (the head of for-await-of), `crates/syntax/tests/sources.rs` (`NESTING_KINDS`, `nested`), `crates/syntax/tests/sources/refused/for-await.js` (removed), `crates/syntax/tests/sources/valid/for-await.js` and `early-errors/for-await-*.js` (new); `crates/compiler/src/{statement,bytecode,disasm}.rs`, `crates/compiler/tests/sources/for-await.{js,disasm}` (new); `crates/swell/src/interp/{for_of,dispatch}.rs`, `crates/swell/src/ops/iteration.rs`, `crates/swell/src/builtins/async_iteration.rs` (new), `crates/swell/src/builtins/{mod,symbol}.rs`, `crates/swell/src/object/{mod,builtin_function}.rs`, `crates/swell/src/heap/collect.rs` (`KIND_SITES` and the record of every kind), `crates/swell/src/realm/{mod,intrinsics}.rs`; `crates/swell/tests/for_await.rs` (new), `crates/swell/tests/builtins_symbol_values.rs` (rows or roots only as Change says), `crates/swell/tests/main.rs` (generated). Checked against the code on `397ffff` and the Writes lines of the first wave: the matches without a wildcard this story adds arms to are `IterationKind` (the sites `git grep` lists), `Instruction` (`bytecode.rs` `target`, `disasm.rs`, `dispatch.rs`), `ObjectKind` and `KindTag` (`object/mod.rs`, `heap/collect.rs`), `BuiltinId` and `Intrinsic`; `WellKnown` is built in one place (`realm/mod.rs`); the lists the gate reads are `KIND_SITES` (18, 19 after), `NESTING_KINDS`, `crates/swell/tests/main.rs` (generated) and the fixture directories read by `read_dir`. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/swell/src/interp/async_function.rs` and `crates/swell/src/object/{promise,async_function}.rs` (the `await` of `story:async-functions-and-await` used as it is), `crates/swell/src/ops/promise.rs`, `crates/swell/src/host/`, `crates/test262`, `conformance/`, `website/`. Shared with the stories of its wave, each in hunks of its own: `crates/syntax/src/early.rs` (`story:async-methods-in-object-literals`, the branch of a method; `story:async-generator-syntax-and-early-errors`, the early errors of an async generator; this story, the head of for-await-of), `crates/syntax/tests/sources.rs` (each story its nesting kind; the coordinator joins the hunks of `NESTING_KINDS`), `crates/swell/src/object/builtin_function.rs` and `crates/swell/src/realm/intrinsics.rs` (`story:promise-statics-finally-and-aggregate-error` and `story:async-function-constructor`, each its own variants and intrinsics), `crates/swell/src/builtins/mod.rs` (each story its `mod` line), `crates/swell/tests/main.rs` (generated). Shared with the open stories of slice 7, none ordered against it: `crates/compiler/src/{bytecode,disasm}.rs` and `crates/swell/src/interp/dispatch.rs` (`story:yield-star-delegation`, its instruction), `crates/swell/src/ops/iteration.rs` (`story:yield-star-delegation` makes `iterator_complete` `pub(crate)`; this story, `get_iterator` and AsyncIteratorClose), `crates/swell/src/object/builtin_function.rs` and `crates/swell/src/realm/intrinsics.rs` (`story:generator-function-constructor`).

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-syntax --test main` and the same with `SWELL_FULL_COUNTS=1`, `cargo nextest run --locked -p swell-compiler --test main`, `cargo nextest run --locked -p swell --test main -E 'test(/^for_await::/) | test(/^async_functions::/) | test(/^for_of::/) | test(/^builtins_iterators::/) | test(/^builtins_symbol_values::/) | test(/^host_objects::host_186_a_host_function_charges_every_byte$/) | test(/^host_objects::host_187/) | test(/^termination::/) | test(/^collect::/)'` and the same with `--features collect-every-safe-point`, and `cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; each exits 0. `slot task clippy` exits 0. The text: the search of Context prints nothing. Time: every test this story adds stays under 1.5 CPU seconds alone in the default mode and under 2 with `collect-every-safe-point`, and each part of `every_nesting_kind_fits_the_stack_bound` under 1.5 at the gate's count and 2.5 at the full count, as the unit measured them; the report lists each measured time and the sum, and a time over its bound fails this acceptance.

test262. The full run is not the unit's: the unit runs the runner on a part of the checkout. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-10/story-for-await.cases.txt" > .out/story-for-await.cases.txt` (`WAVE` the wave branch the unit merges into), and the guard `docs/plan/slice-10/guard-async-token.cases.txt` (the 241 cases with a passing run in that base run whose text has the word `async` or `await`), `cat .out/story-for-await.cases.txt docs/plan/slice-10/guard-async-token.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-10/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt test/language/statements/for-of` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; the list, the guard and every case of for-of, whose head and loop this story shares: 1,406 cases, 2,682 runs over the base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-10/check.sh .out/story-for-await.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the base run of the property-escapes join (the newest full run, made before generators and the flag `i` were merged and before the first wave of this slice; 18,935 of 44,812 cases pass) it exits 1, prints 815 lines `failing` and `passed 816 bound 816 B 816 F 420 R 420`): every run of the list passes, no run that passed on the base fails, and the part has at least B + F − R passing cases; (b) `awk -F'\t' '$6 ~ /for await is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0`. On the join, the coordinator's full run, made in parts, passes (a) over the whole run, and (b) over the whole run prints `0`.

## Out of scope

Async generators and `yield*` in them (`story:async-generators-and-their-request-queue`, `story:yield-star-in-async-generators`), whose cases with `for await` are on their lists; `Array.fromAsync` (`story:array-from-async`); `for await` in a module at the top level (slice 11); the moves of cases and the slice lines of the design (the conformance close of this slice).

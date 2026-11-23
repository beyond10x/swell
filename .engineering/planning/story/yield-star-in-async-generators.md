---
format: aep.planning-md/3
id: story:yield-star-in-async-generators
kind: story
status: draft
title: yield* in an async generator delegates through the async iterator protocol
summary: 'The async kind of the delegation loop of slice 7: GetIterator with kind async, the await of each inner result, throw and return passed on; the last held refusal of slice 10 leaves; 559 cases.'
relations:
- decomposes: epic:slice-10-promises-and-async
- depends_on: story:async-generators-and-their-request-queue
- depends_on: story:yield-star-delegation
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: inferred
  path: crates/compiler/tests/sources/async-generator-yield-star.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/async-generator-yield-star.js
- confidence: cited
  path: crates/swell/src/interp/async_generator.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/yield_star.rs
- confidence: cited
  path: crates/swell/tests/async_generators.rs
- confidence: cited
  path: crates/syntax/src/lib.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: cited
  path: crates/syntax/tests/sources.rs
- confidence: cited
  path: crates/syntax/tests/sources/refused/async-generator-yield-star.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/async-generator-yield-star.js
revision: 16
---
## Context

Slice 10 brings jobs, `Promise`, `async` functions and generators and `for await` (`epic:slice-10-promises-and-async`; `docs/design/engine.md`, section 12, item 10). This story brings `yield*` in an async generator (`#sec-generator-function-definitions-runtime-semantics-evaluation`, YieldExpression : `yield*` AssignmentExpression with generatorKind async: GetIterator(value, async), the `await` of each inner result, AsyncGeneratorYield of its value, the received `throw` and `return` passed on with their `await`s, AsyncIteratorClose when the inner iterator has no `throw`). It depends on `story:async-generators-and-their-request-queue` (the async generator, its `yield` and the held refusal it gives `yield*`), on `story:for-await-and-async-from-sync-iterator` through it (GetIterator with kind async and `%AsyncFromSyncIteratorPrototype%`, whose 33 cases of `return` and `throw` are on this list), and on the open story of slice 7 `story:yield-star-delegation`, whose delegation loop (`crates/swell/src/interp/yield_star.rs`, its instruction and the function of YieldExpression of `crates/syntax/src/parser/expression.rs`) this story extends to the async kind.

The refusal this story removes: `yield* in an async generator is not supported before slice 10`, held by `story:async-generators-and-their-request-queue`. The text leaves the code. The search, `grep -rn 'yield\* in an async generator' crates conformance docs/design taskfiles tests website/docs`, prints on its base the record of the held refusal in `crates/syntax/src/parser/expression.rs`, the documentation of `held` (`crates/syntax/src/parser/mod.rs`) and of `parse_script` (`crates/syntax/src/lib.rs`), line 1 of `crates/syntax/tests/sources/refused/async-generator-yield-star.js` and the test `the_refusal_of_yield_star_in_an_async_generator_is_counted` of `crates/swell/tests/async_generators.rs`; after this story it prints nothing. With it the last held refusal of slice 10 leaves the parser; if no held refusal is left in the code (the one of `yield*` of slice 7 gone with `story:yield-star-delegation`), the field `held`, its answer in `analyse_parsed` and the arm of `admitted` in `crates/syntax/tests/sources.rs` stay only as far as clippy with `-D warnings` allows, and the unit reports what it removed.

The list of this story is `docs/plan/slice-10/story-ag-yield-star.cases.txt`: the 559 cases of the key `ag-yield-star` of `docs/plan/slice-10/later.tsv` (an async generator with `yield*` and no `async *` method; the 58 cases with both are the key `ag-cross`, which the conformance close checks; the command is in the head of `docs/plan/slice-10/scan-later.awk`). By directory: `language/statements/for-await-of` 342, `language/expressions/async-generator` 119, `language/statements/async-generator` 65, `built-ins/AsyncFromSyncIteratorPrototype` 33; 558 have the flag `async`. They have 1,118 runs in the base run of the property-escapes join (the newest full run, made before generators and the flag `i` were merged and before the first wave of this slice; 18,935 of 44,812 cases pass), all failing on `async function is not supported before slice 10` (the text before the first wave). A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on, by name (the text of the slice, merged at `843f178`): `docs/design/engine.md` decision 3, "Suspended frames", items 4 to 6, and the paragraph on `yield*` that `story:yield-star-delegation` builds (a chain of n delegating generators holds n frames and takes no Rust frame per level); "`async` functions on suspended frames", items 3 to 5 (each `await` of the loop); the answers to the three questions of `story:async-generators-and-their-request-queue`. No sentence is missing beyond those.

## What later stories reuse

The async kind of the delegation loop, which `story:async-generator-methods-in-object-literals` and `story:async-generator-function-constructor` do not need, and which the 60 cases of `docs/plan/slice-10/close-cross.cases.txt` (58 with an `async *` method, 2 of `Array.fromAsync`) read once the stories of their other parts land.

## Domain relations

- Delegating async generator → its inner iterator: one to one while it delegates; the inner iterator is held by the delegating frame and closed or completed before the frame goes on — inferable (inferred from ECMA-262 `#sec-generator-function-definitions-runtime-semantics-evaluation` and `crates/swell/src/interp/yield_star.rs`, the loop of `story:yield-star-delegation`; no ess/1 document declares it: the host model `spec/domains/host.yaml` holds no engine value, and ESS does not restate the slots of ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base (a `yield*` in an async generator ends on the held refusal).

- `swell-syntax`, the refusal only: the record of the held refusal removed from the function of YieldExpression (`crates/syntax/src/parser/expression.rs`), the documentation of `held` and of `parse_script` no longer naming it; fixtures `refused/async-generator-yield-star.js` removed and `valid/async-generator-yield-star.js` (new).

- `swell-compiler`: the arm of `Expr::Yield` with `delegate` in an async generator (`crates/compiler/src/expression.rs`) compiles to the async kind of the delegation instruction of `story:yield-star-delegation` (an operand or a new instruction, at most two, each documented with its stack effect and its clause, with its arm in `Instruction::target` and `disasm.rs`). Fixture `crates/compiler/tests/sources/async-generator-yield-star.{js,disasm}` (new).

- `swell`: `crates/swell/src/interp/yield_star.rs`: the async kind of the loop: GetIterator(value, async) of `crates/swell/src/ops/iteration.rs`; each call of `next`, `throw` or `return` of the inner iterator followed by the `await` of `story:async-functions-and-await` on its result (the holder of the async generator, question 1 of `story:async-generators-and-their-request-queue`), then AsyncGeneratorYield of IteratorValue when not done (without a second `await` of the value: `#sec-asyncgeneratoryield`, step for `yield*`); a received `return` awaited before it is passed on; AsyncIteratorClose of `story:for-await-and-async-from-sync-iterator` when the inner iterator has no `throw`. `crates/swell/src/interp/async_generator.rs`: the `yield` of a delegation, if the loop needs it apart from the plain `yield`.

- Bounds. Steps: each turn of the loop is the call of the inner method and one `await` (3 steps when the loop resumes, item 4), plus the `yield` of the value; the loop keeps the bound of `story:yield-star-delegation`: a chain of n delegating async generators holds n frames, each suspended, and takes no Rust frame per level. Memory: nothing beyond the `await`s and the frames. Termination: as for every `await` and every async generator (question 3 of `story:async-generators-and-their-request-queue`).

- Tests, each in both collection modes, in `crates/swell/tests/async_generators.rs` (the module of `story:async-generators-and-their-request-queue`): `yield_star_in_an_async_generator_delegates_to_an_async_iterator`, `yield_star_in_an_async_generator_delegates_to_a_sync_iterator_through_async_from_sync`, `yield_star_passes_throw_and_return_on_and_awaits_them`, `yield_star_closes_an_inner_iterator_without_throw`, `a_chain_of_delegating_async_generators_longer_than_frame_depth_runs_to_its_end` (n above `frame_depth`, at most 1,000), `a_termination_inside_a_delegating_async_generator_leaves_its_request_pending` (for `StepBudget`, `MemoryBudget` and `Interrupted`); `the_refusal_of_yield_star_in_an_async_generator_is_counted` removed. `crates/swell/tests/main.rs` does not change (no new module).

- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with a collection at every safe point and under 2.5 at its full count (`SWELL_FULL_COUNTS=1`); measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system). The unit times each test before it adds it and names each one over 1 CPU second with its time; no loop of a test runs more than 1,000 iterations; the tests of this story are expected to add under 3 CPU seconds to the gate in both modes together, and the unit reports the sum it measured (the test steps of the gate cost about 330 CPU seconds). A product several tests share is built once inside one test function or is a committed file, never in a `static`; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, the most); a script that measures or waits runs without the prefix. No Python in any tool, test or command; no `cd ... &&` with a write or a delete; intermediate files in the scratch directory.
- Size: about 700 lines (the loop 250, the compiler 60, the tests and fixtures 390): one session.

## Requirements

None of `conformance/host/requirements.txt`.

Writes: `crates/syntax/src/parser/{expression,mod}.rs` (the function of YieldExpression; the documentation of `held`), `crates/syntax/src/lib.rs` (the documentation of `parse_script`), `crates/syntax/tests/sources/refused/async-generator-yield-star.js` (removed), `crates/syntax/tests/sources/valid/async-generator-yield-star.js` (new), and `crates/syntax/tests/sources.rs` and `crates/syntax/src/parser/mod.rs` further only as Context says; `crates/compiler/src/{expression,bytecode,disasm}.rs`, `crates/compiler/tests/sources/async-generator-yield-star.{js,disasm}` (new); `crates/swell/src/interp/{yield_star,async_generator,dispatch}.rs`; `crates/swell/tests/async_generators.rs`. Checked against the code on `397ffff` and the Writes lines of `story:yield-star-delegation` and `story:async-generators-and-their-request-queue`: the match without a wildcard this story may add arms to is `Instruction` (`bytecode.rs` `target`, `disasm.rs`, `dispatch.rs`); no `Expr`, `ObjectKind`, `BuiltinId` or `Intrinsic`. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/swell/src/ops/iteration.rs` (GetIterator with kind async and AsyncIteratorClose used as `story:for-await-and-async-from-sync-iterator` leaves them), `crates/swell/src/interp/{suspend,async_function}.rs`, `crates/swell/src/object/`, `crates/swell/src/heap/collect.rs`, `crates/test262`, `conformance/`, `website/`. Shared with the stories of its wave, each in hunks of its own: `crates/syntax/src/parser/expression.rs` (`story:async-generator-methods-in-object-literals`, `accessor_kind`; this story, the function of YieldExpression), `crates/syntax/src/parser/mod.rs` and `crates/syntax/src/lib.rs` (`story:async-generator-function-constructor`, the dynamic entry; this story, the documentation of `held` and `parse_script`), `crates/compiler/src/{bytecode,disasm}.rs` and `crates/swell/src/interp/dispatch.rs` (no other story of its wave adds an instruction), `crates/swell/tests/async_generators.rs` (`story:async-generator-methods-in-object-literals` and `story:async-generator-function-constructor` put their tests in modules of their own). Shared with the open stories of slice 7: the files of `story:yield-star-delegation`, which this story follows by its edge.

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test main -E 'test(/^async_generators::/) | test(/^yield_star::/) | test(/^generators::/) | test(/^for_await::/) | test(/^termination::/) | test(/^eval_stack::/)'` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell-syntax --test main`; `cargo nextest run --locked -p swell-compiler --test main`; each exits 0. `slot task clippy` exits 0. The text: the search of Context prints nothing. Time: every test this story adds stays under 1.5 CPU seconds alone in the default mode and under 2 with `collect-every-safe-point`, as the unit measured them (Change, "Time"); the report lists each measured time and the sum, and a time over its bound fails this acceptance.

test262. The full run is not the unit's: the unit runs the runner on a part of the checkout. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-10/story-ag-yield-star.cases.txt" > .out/story-ag-yield-star.cases.txt` (`WAVE` the wave branch the unit merges into), and the guard `docs/plan/slice-10/guard-async-token.cases.txt` (the 241 cases with a passing run in that base run whose text has the word `async` or `await`), `cat .out/story-ag-yield-star.cases.txt docs/plan/slice-10/guard-async-token.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-10/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt test/language/expressions/yield test/built-ins/GeneratorPrototype` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; the list, the guard and the `yield*` and generator cases of slice 7, whose loop this story extends: 924 cases, 1,797 runs over the base run; on the join they pass and count in B), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-10/check.sh .out/story-ag-yield-star.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the base run of the property-escapes join (the newest full run, made before generators and the flag `i` were merged and before the first wave of this slice; 18,935 of 44,812 cases pass), made before generators were merged, it exits 1, prints 1,118 lines `failing` and `passed 244 bound 244 B 244 F 559 R 559`): every run of the list passes, no run that passed on the base fails, and the part has at least B + F − R passing cases; (b) `awk -F'\t' '$6 ~ /yield\* in an async generator is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0`. On the join, the coordinator's full run, made in parts, passes (a) over the whole run, and (b) over the whole run prints `0`.

## Out of scope

`yield*` in a generator (`story:yield-star-delegation`); `async *` methods (`story:async-generator-methods-in-object-literals`); the 60 cases of `docs/plan/slice-10/close-cross.cases.txt` (the conformance close checks them after every story of their parts); the moves of cases and the slice lines of the design (the conformance close of this slice).

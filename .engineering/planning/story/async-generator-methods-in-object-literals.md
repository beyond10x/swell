---
format: aep.planning-md/3
id: story:async-generator-methods-in-object-literals
kind: story
status: active
title: Async generator methods of object literals and their early errors
summary: async *m() {} parses, compiles and runs as an async generator with a home object; 250 cases, 38 of them negative.
relations:
- decomposes: epic:slice-10-promises-and-async
- depends_on: story:async-generators-and-their-request-queue
- depends_on: story:async-methods-in-object-literals
- depends_on: story:generator-methods-in-object-literals
scope:
- confidence: inferred
  path: crates/compiler/tests/sources/async-generator-method.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/async-generator-method.js
- confidence: cited
  path: crates/swell/src/interp/object_literal.rs
- confidence: cited
  path: crates/swell/src/object/ecmascript_function.rs
- confidence: inferred
  path: crates/swell/tests/async_generator_methods.rs
- confidence: cited
  path: crates/swell/tests/async_methods.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/tests/sources.rs
- confidence: cited
  path: crates/syntax/tests/sources/refused/async-generator-method.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/async-generator-method.js
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T19:58:15Z", actor: "human:operator", revision: 15, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T19:58:15Z", actor: "human:operator", revision: 16, executor: "agent:coordinator"}
---
## Context

Slice 10 brings jobs, `Promise`, `async` functions and generators and `for await` (`epic:slice-10-promises-and-async`; `docs/design/engine.md`, section 12, item 10). This story parses, compiles and runs `async *` methods of object literals (`#sec-async-generator-function-definitions`, AsyncGeneratorMethod; `#sec-runtime-semantics-definemethodproperty`). It depends on `story:async-generators-and-their-request-queue` (the async generator, which a method takes as it is), on `story:async-methods-in-object-literals` (the branch of an `async` method in `accessor_kind`, where it left the refusal of `async *`) and on the open story of slice 7 `story:generator-methods-in-object-literals` (the generator method in the same function and in the branch of a method of `crates/syntax/src/early.rs`).

The refusal this story removes: `async generator method is not supported before slice 10`, which `story:async-methods-in-object-literals` gave `async *m() {}` at the `*`, at parse. The text leaves the code. The search, `grep -rn 'async generator method is not supported\|"async generator method"' crates conformance docs/design taskfiles tests website/docs`, prints on its base the line of `accessor_kind` (`crates/syntax/src/parser/expression.rs`), line 1 of `crates/syntax/tests/sources/refused/async-generator-method.js` and the test `the_refusal_of_an_async_generator_method_is_counted` of `crates/swell/tests/async_methods.rs`; after this story it prints nothing.

The list of this story is `docs/plan/slice-10/story-ag-methods.cases.txt`: the 250 cases of the key `ag-methods` of `docs/plan/slice-10/later.tsv` (an `async *` method of an object literal and no `yield*`, the negative parse cases with one among them; the command is in the head of `docs/plan/slice-10/scan-later.awk`). By directory: `language/expressions/object` 232, `language/eval-code/direct` 12, and 6 in directories of fewer cases (`language/arguments-object`, `built-ins/Function/prototype/toString`); 38 are negative parse cases and 143 have the flag `async`. They have 478 runs in the base run of the property-escapes join (the newest full run, made before generators and the flag `i` were merged and before the first wave of this slice; 18,935 of 44,812 cases pass), all failing on `async function is not supported before slice 10` (the text before the first wave). A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on, by name: decision 3 of `docs/design/engine.md`, "Suspended frames", and "`async` functions on suspended frames" (the text of the slice, merged at `843f178`), and the async generator of `story:async-generators-and-their-request-queue` (an `async *` method is an async generator with a [[HomeObject]]), given by "Async generators on suspended frames" (`docs/design/engine.md` line 1230, completed at `3648bf6`); "Depth bounds" (the parser's nesting counter, `STACK_BYTES_PER_LEVEL`). No sentence is missing from the design.

## What later stories reuse

Nothing of this slice; slice 8 adds `async *` methods of classes beside it.

## Domain relations

- `async *` method → its object literal: many to one, a method property whose function has the object as [[HomeObject]], `%AsyncGeneratorFunction.prototype%` as [[Prototype]] and a `prototype` of its own — inferable (inferred from ECMA-262 `#sec-runtime-semantics-definemethodproperty` and `#sec-asyncgenerator-definitions-propertydefinitionevaluation`; no ess/1 document declares it: the host model `spec/domains/host.yaml` holds no engine value, and ESS does not restate the grammar of ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base (the method is refused at parse).

- `crates/syntax/src/parser/expression.rs`, `accessor_kind`: `async *` then a property name is an async generator method (`FunctionKind::Method`, `generator` and `asynchronous` set), its parameters and body read with [+Yield, +Await], its span from the `async`. `crates/syntax/src/early.rs`, the branch of a method: the early errors of `#sec-async-generator-function-definitions-static-semantics-early-errors` for AsyncGeneratorMethod. `crates/syntax/tests/sources.rs`: the nesting kind `async_generator_method` in `NESTING_KINDS` and `nested`, one more than on its base; a level over `STACK_BYTES_PER_LEVEL` gets its path split into functions, never a new value of the constant. Fixtures: `refused/async-generator-method.js` removed; `valid/async-generator-method.js` and `early-errors/async-generator-method-*.js` (new).

- `swell-compiler` and `swell`: an `async *` method compiles to an async generator template; if the path of a method (`crates/swell/src/interp/object_literal.rs`, `define_computed_function`; `crates/swell/src/object/ecmascript_function.rs`, `instantiate_function`) does not reach the async generator branch of `story:async-generators-and-their-request-queue`, the unit makes it reach it there and reports the line. Fixture `crates/compiler/tests/sources/async-generator-method.{js,disasm}` (new).

- Bounds. No instruction and no running path is added: an `async *` method is the async generator of `story:async-generators-and-their-request-queue` with a [[HomeObject]], so its steps, its depth checks (a resumption by `next`, `return` or `throw` checked before the request is queued, item 4), the charge of its suspended frame and its queue (items 1 and 2) and its terminations (item 5) are that story's, and nothing is added to the frame of `Agent::execute` (`crates/swell/src/interp/dispatch.rs` line 244); `STACK_BYTES_PER_REENTRY` and the stack tests do not change. The parse adds a nesting kind, bounded by `STACK_BYTES_PER_LEVEL` as above.

- Tests, each in both collection modes, in `crates/swell/tests/async_generator_methods.rs` (new): `an_async_generator_method_yields_promises_of_iterator_results`, `an_async_generator_method_ends_at_each_budget_and_at_an_interrupt` (a method whose body loops without end after an `await`: with `steps_per_entry` N the checkpoint answers `Terminated(StepBudget)` and `last_entry_steps()` reads at most N; a body that grows an array without end answers `Terminated(MemoryBudget)`; with the interrupt handle set while the body runs, `Terminated(Interrupted)`; after each the promise of the `next` reads `Pending`), `an_async_generator_method_has_its_object_as_home_object`, `an_async_generator_method_has_a_prototype_and_no_construct`, `a_computed_async_generator_method_gets_its_name`. `crates/swell/tests/async_methods.rs`: `the_refusal_of_an_async_generator_method_is_counted` removed. `crates/swell/tests/main.rs` generated again (`sh taskfiles/test-main.sh write`). No function of the realm is added, so neither the sweep with a Symbol nor HOST-187 is touched.

- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with a collection at every safe point and under 2.5 at its full count (`SWELL_FULL_COUNTS=1`); measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system). The unit times each test before it adds it and names each one over 1 CPU second with its time; no loop of a test runs more than 1,000 iterations; the tests of this story are expected to add under 3 CPU seconds to the gate in both modes together, and the unit reports the sum it measured (the test steps of the gate cost about 330 CPU seconds). A product several tests share is built once inside one test function or is a committed file, never in a `static`; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, the most); a script that measures or waits runs without the prefix. No Python in any tool, test or command; no `cd ... &&` with a write or a delete; intermediate files in the scratch directory.
- The parser part: each part of `every_nesting_kind_fits_the_stack_bound` is timed before and after at the gate's count and at the full count (`SWELL_FULL_COUNTS=1`) and both are reported.

- Size: about 450 lines (the parser and early errors 120, the compile path 30, the tests and fixtures 300): one session.

## Requirements

None of `conformance/host/requirements.txt`.

Writes: `crates/syntax/src/parser/expression.rs` (`accessor_kind`), `crates/syntax/src/early.rs` (the branch of a method), `crates/syntax/tests/sources.rs` (`NESTING_KINDS`, `nested`), `crates/syntax/tests/sources/refused/async-generator-method.js` (removed), `crates/syntax/tests/sources/valid/async-generator-method.js` and `early-errors/async-generator-method-*.js` (new); `crates/compiler/tests/sources/async-generator-method.{js,disasm}` (new), and if Change finds it needed `crates/swell/src/interp/object_literal.rs` and `crates/swell/src/object/ecmascript_function.rs` (one branch each, reported); `crates/swell/tests/async_generator_methods.rs` (new), `crates/swell/tests/async_methods.rs` (one test out), `crates/swell/tests/main.rs` (generated). Checked against the code on `397ffff` and the Writes lines of the stories before it: no enum variant is added, so no match changes; the fixture directories are read by `read_dir`; `NESTING_KINDS` is the list of the gate this story extends. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/{lib,ast}.rs`, `crates/syntax/src/parser/{mod,function,statement}.rs`, `crates/swell/src/interp/async_generator.rs` and `crates/swell/src/builtins/async_generator.rs` (used as they are), `crates/test262`, `conformance/`, `website/`. Shared with the stories of its wave, each in hunks of its own: `crates/syntax/src/parser/expression.rs` (`story:yield-star-in-async-generators`, the function of YieldExpression; this story, `accessor_kind`), `crates/syntax/tests/sources.rs` (`story:yield-star-in-async-generators` may take out the arm of `admitted`; this story, its nesting kind), `crates/swell/src/object/ecmascript_function.rs` (if written: no other story of its wave writes it), `crates/swell/tests/main.rs` (generated). Shared with the open stories of slice 7: `crates/syntax/src/parser/expression.rs` and `crates/syntax/src/early.rs` with `story:generator-methods-in-object-literals`, which this story follows by its edge.

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-syntax --test main` and the same with `SWELL_FULL_COUNTS=1`, `cargo nextest run --locked -p swell-compiler --test main`, and `cargo nextest run --locked -p swell --test main -E 'test(/^async_generator_methods::/) | test(/^async_methods::/) | test(/^async_generators::/) | test(/^generator_methods::/) | test(/^eval::/) | test(/^language::/) | test(/^termination::/) | test(/^interrupt::/)'` and the same with `--features collect-every-safe-point`; each exits 0. `slot task clippy` exits 0. The text: the search of Context prints nothing. Time: every test this story adds stays under 1.5 CPU seconds alone in the default mode and under 2 with `collect-every-safe-point`, and each part of `every_nesting_kind_fits_the_stack_bound` under 1.5 at the gate's count and 2.5 at the full count, as the unit measured them; the report lists each measured time, and a time over its bound fails this acceptance.

test262. The full run is not the unit's: the unit runs the runner on a part of the checkout. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-10/story-ag-methods.cases.txt" > .out/story-ag-methods.cases.txt` (`WAVE` the wave branch the unit merges into), and the guard `docs/plan/slice-10/guard-async-token.cases.txt` (the 241 cases with a passing run in that base run whose text has the word `async` or `await`), `cat .out/story-ag-methods.cases.txt docs/plan/slice-10/guard-async-token.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-10/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt test/language/expressions/object/method-definition` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; the list, the guard and every case of method definitions: 736 cases, 1,365 runs over the base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-10/check.sh .out/story-ag-methods.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the base run of the property-escapes join (the newest full run, made before generators and the flag `i` were merged and before the first wave of this slice; 18,935 of 44,812 cases pass) it exits 1, prints 478 lines `failing` and `passed 288 bound 288 B 288 F 250 R 250`): every run of the list passes, no run that passed on the base fails, and the part has at least B + F − R passing cases; (b) `awk -F'\t' '$6 ~ /async generator method is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0`. Over the property-escapes base run the same command prints `0` too, since that text came with `story:async-methods-in-object-literals`; so the unit runs it over `.out/test262-base/runs.tsv`, the base run of the join it starts from, before its first change and reports that count (expected at most 478, the runs of the list), and the check that cannot hold before the work is (a), whose count of passing cases must reach B + F − R. On the join, the coordinator's full run, made in parts, passes (a) over the whole run, and (b) over the whole run prints `0`.

## Out of scope

The 58 cases of the key `ag-cross` (an `async *` method with `yield*`: `docs/plan/slice-10/close-cross.cases.txt`, which the conformance close checks after this story and `story:yield-star-in-async-generators`); `async *` methods of classes (slice 8); the moves of cases and the slice lines of the design (the conformance close of this slice).

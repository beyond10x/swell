---
format: aep.planning-md/3
id: story:async-methods-in-object-literals
kind: story
status: draft
title: Async methods of object literals and their early errors
summary: async m() {} parses, compiles and runs as an async function with a home object; async *m() gets a refusal of its own; 52 cases.
relations:
- decomposes: epic:slice-10-promises-and-async
- depends_on: story:async-functions-and-await
- depends_on: story:generator-methods-in-object-literals
scope:
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: inferred
  path: crates/compiler/tests/sources/async-method.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/async-method.js
- confidence: cited
  path: crates/swell/src/interp/object_literal.rs
- confidence: cited
  path: crates/swell/src/object/ecmascript_function.rs
- confidence: cited
  path: crates/swell/tests/async_functions.rs
- confidence: inferred
  path: crates/swell/tests/async_methods.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/tests/sources.rs
- confidence: inferred
  path: crates/syntax/tests/sources/refused/async-generator-method.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/async-method.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/async-method.js
revision: 16
---
## Context

Slice 10 brings jobs, `Promise`, `async` functions and generators and `for await` (`epic:slice-10-promises-and-async`; `docs/design/engine.md`, section 12, item 10). After the first wave an `async` method of an object literal is still refused at parse in `accessor_kind` (`crates/syntax/src/parser/expression.rs` line 1473 on `397ffff`) with `async method is not supported before slice 10`, the text `story:async-functions-and-await` gave it, for `async m() {}` and `async *m() {}` alike. This story parses, compiles and runs `async` methods of object literals (`#sec-async-function-definitions`, AsyncMethod; `#sec-runtime-semantics-definemethodproperty`). It depends on `story:async-functions-and-await` (the activation, `await` and the instantiation of an `async` template, which a method takes as it is) and on the open story of slice 7 `story:generator-methods-in-object-literals`, which writes `accessor_kind` and the branch of a method in `crates/syntax/src/early.rs` (its Writes line), the lines this story changes next to; the edge orders the two.

The refusal this story removes: `async method is not supported before slice 10`. The text leaves the code: `async *m() {}` gets a refusal of its own text, `async generator method is not supported before slice 10`, at the `*`, at parse (its place in `accessor_kind`; `story:async-generator-methods-in-object-literals` lifts it). The search for what expects the text, `grep -rn 'async method is not supported\|"async method"' crates conformance docs/design taskfiles tests website/docs`, prints on its base (the join after the first wave) the line of `accessor_kind`, line 1 of `crates/syntax/tests/sources/refused/async-method.js` and the test `the_refusal_of_an_async_method_is_counted` of `crates/swell/tests/async_functions.rs`; after this story it prints nothing.

The list of this story is `docs/plan/slice-10/story-async-methods.cases.txt`: the 52 cases of the key `async-methods` of `docs/plan/slice-10/later.tsv` (the command is in the head of `docs/plan/slice-10/scan-later.awk`). By directory: `language/expressions/object` 33, `language/eval-code/direct` 12, `harness` 6 (`asyncHelpers-throwsAsync-*`, whose fixtures use an `async` method), `built-ins/Function/prototype/toString` 1 (`async-method-object.js`, which reads a span that starts at `async`); 11 are negative parse cases and 39 have the flag `async`. They have 84 runs in the base run of the property-escapes join (the newest full run, made before generators and the flag `i` were merged and before the first wave of this slice; 18,935 of 44,812 cases pass), all failing on `async function is not supported before slice 10` (the text before the first wave). A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on, by name: decision 3 of `docs/design/engine.md`, "Suspended frames", and "`async` functions on suspended frames", items 1 to 5 (`docs/design/engine.md` line 1101, merged at `843f178` and changed at `7abd7e0`; an `async` method is an `async` function with a [[HomeObject]]); "Depth bounds" (the parser's nesting counter, `STACK_BYTES_PER_LEVEL`). No sentence is missing from the design.

## What later stories reuse

The branch of an `async` method in `accessor_kind` and in the early errors of a method, which `story:async-generator-methods-in-object-literals` extends to `async *`.

## Domain relations

- `async` method → its object literal: many to one, a method property whose function has the object as [[HomeObject]] and `%AsyncFunction.prototype%` as [[Prototype]] — inferable (inferred from ECMA-262 `#sec-runtime-semantics-definemethodproperty` and `#sec-makemethod`; no ess/1 document declares it: the host model `spec/domains/host.yaml` holds no engine value, and ESS does not restate the grammar of ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base (the method is refused at parse).

- `crates/syntax/src/parser/expression.rs`, `accessor_kind`: `async` then a property name with no line terminator between is an `async` method (`FunctionKind::Method`, `asynchronous: true`), its parameters and body read with [+Await] and [~Yield], its span from the `async`; `async *` refuses at the `*` with `async generator method`. `crates/syntax/src/early.rs`, the branch of a method: the early errors of `#sec-async-function-definitions-static-semantics-early-errors` for AsyncMethod (`await` in the parameters; a Use Strict Directive with a parameter list that is not simple; a parameter the body declares lexically; duplicate parameters), and `super` calls refused in a method as for every method. `crates/syntax/tests/sources.rs`: the nesting kind `async_method` (`({ async m() { return { async m() { ... } }; } })`) in `NESTING_KINDS` and `nested`, one more than on its base; a level over `STACK_BYTES_PER_LEVEL` gets its path split into functions (`#[inline(never)]`), never a new value of the constant. Fixtures: `refused/async-method.js` removed; `valid/async-method.js` (new: a method, a computed name, a method with defaults, `await` in its body), `refused/async-generator-method.js` (new: the new text), `early-errors/async-method-*.js` (new, one per early error above).

- `swell-compiler` and `swell`: an `async` method compiles to an `async` template as a function does (the start, the activation slot, the return of `story:async-functions-and-await`); if the path of a method (`crates/compiler/src/expression.rs`, the object literal; `crates/swell/src/interp/object_literal.rs`, `define_computed_function`, line 69; `crates/swell/src/object/ecmascript_function.rs`, `instantiate_function`, line 497) does not reach the `async` branch of `instantiate_function` (`%AsyncFunction.prototype%`, no [[Construct]], no `prototype`), the unit makes it reach it there and reports the line. Fixture `crates/compiler/tests/sources/async-method.{js,disasm}` (new). No instruction and no new path into script is added: the method is called as every function is, so the run loop, `STACK_BYTES_PER_REENTRY` and the depth tests of `story:async-functions-and-await` hold for it as they are.

- Tests, each in both collection modes, in `crates/swell/tests/async_methods.rs` (new): `an_async_method_returns_a_promise_and_awaits`, `an_async_method_has_its_object_as_home_object` (`super.x` inside it reads the prototype of the object), `an_async_method_has_no_prototype_and_no_construct`, `a_computed_async_method_gets_its_name`, `the_refusal_of_an_async_generator_method_is_counted` (through `eval`: a `SyntaxError` with the exact text, `refusals()` moved by 1). `crates/swell/tests/async_functions.rs`: `the_refusal_of_an_async_method_is_counted` removed (its case now runs). `crates/swell/tests/main.rs` generated again (`sh taskfiles/test-main.sh write`). No function of the realm is added, so neither the sweep with a Symbol nor HOST-187 is touched.

- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with a collection at every safe point and under 2.5 at its full count (`SWELL_FULL_COUNTS=1`); measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system). The unit times each test before it adds it and names each one over 1 CPU second with its time; no loop of a test runs more than 1,000 iterations; the tests of this story are expected to add under 3 CPU seconds to the gate in both modes together, and the unit reports the sum it measured (the test steps of the gate cost about 330 CPU seconds). A product several tests share is built once inside one test function or is a committed file, never in a `static`; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, the most); a script that measures or waits runs without the prefix. No Python in any tool, test or command; no `cd ... &&` with a write or a delete; intermediate files in the scratch directory.
- The parser part: each part of `every_nesting_kind_fits_the_stack_bound` is timed before and after at the gate's count and at the full count (`SWELL_FULL_COUNTS=1`) and both are reported.

- Size: about 500 lines (the parser and early errors 150, the compile path 50, the tests and fixtures 300): one session.

## Requirements

None of `conformance/host/requirements.txt` (no line is pending for slice 10 that this story moves).

Writes: `crates/syntax/src/parser/expression.rs` (`accessor_kind`), `crates/syntax/src/early.rs` (the branch of a method), `crates/syntax/tests/sources.rs` (`NESTING_KINDS`, `nested`), `crates/syntax/tests/sources/refused/async-method.js` (removed), `crates/syntax/tests/sources/valid/async-method.js`, `refused/async-generator-method.js` and `early-errors/async-method-*.js` (new); `crates/compiler/tests/sources/async-method.{js,disasm}` (new), and if Change finds it needed `crates/compiler/src/expression.rs` (the object literal), `crates/swell/src/interp/object_literal.rs` and `crates/swell/src/object/ecmascript_function.rs` (one branch each, reported); `crates/swell/tests/async_methods.rs` (new), `crates/swell/tests/async_functions.rs` (one test out), `crates/swell/tests/main.rs` (generated). Checked against the code on `397ffff` and the Writes lines of the first wave and of `story:generator-methods-in-object-literals`: no enum variant is added (`FunctionKind::Method` with `asynchronous` of the first wave), so no match changes; the fixture directories are read by `read_dir`; `NESTING_KINDS` is the list of the gate this story extends. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/{lib,ast}.rs`, `crates/syntax/src/parser/{mod,function,statement}.rs`, `crates/swell/src/interp/async_function.rs` and `crates/swell/src/object/promise.rs` (used as they are), `crates/test262`, `conformance/`, `website/`. Shared with the stories of its wave, each in hunks of its own: `crates/syntax/src/parser/expression.rs` (`story:async-generator-syntax-and-early-errors`, `primary_kind` for `async function*`; this story, `accessor_kind`), `crates/syntax/src/early.rs` (`story:async-generator-syntax-and-early-errors`, the early errors of an async generator; `story:for-await-and-async-from-sync-iterator`, those of `for await`; this story, the branch of a method), `crates/syntax/tests/sources.rs` (each story its nesting kind; the coordinator joins the hunks of `NESTING_KINDS`), `crates/swell/tests/main.rs` (generated); of its wave, `story:for-await-and-async-from-sync-iterator` writes `crates/compiler/src/statement.rs`, not `expression.rs`. Shared with `story:async-generators-and-their-request-queue` (order 5), with no path of edges between them, in hunks apart, named in both stories: if this story writes them, the branch of an `async` method in `instantiate_function` (`crates/swell/src/object/ecmascript_function.rs`) and the object literal of `crates/compiler/src/expression.rs`; that story, the branch of an async generator template in `instantiate_function` and the arm of `Expr::Yield` in `expression.rs`. Shared with the open stories of slice 7: `crates/syntax/src/parser/expression.rs` and `crates/syntax/src/early.rs` with `story:generator-methods-in-object-literals`, which this story follows by its edge.

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-syntax --test main` and the same with `SWELL_FULL_COUNTS=1`, `cargo nextest run --locked -p swell-compiler --test main`, and `cargo nextest run --locked -p swell --test main -E 'test(/^async_methods::/) | test(/^async_functions::/) | test(/^generator_methods::/) | test(/^eval::/) | test(/^language::/)'` and the same with `--features collect-every-safe-point`; each exits 0. `slot task clippy` exits 0. The text: `grep -rn 'async method is not supported\|"async method"' crates conformance docs/design taskfiles tests website/docs` prints nothing (Context: three lines on its base). Time: every test this story adds stays under 1.5 CPU seconds alone in the default mode and under 2 with `collect-every-safe-point`, and each part of `every_nesting_kind_fits_the_stack_bound` under 1.5 at the gate's count and 2.5 at the full count, as the unit measured them; the report lists each measured time, and a time over its bound fails this acceptance.

test262. The full run is not the unit's: the unit runs the runner on a part of the checkout. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-10/story-async-methods.cases.txt" > .out/story-async-methods.cases.txt` (`WAVE` the wave branch the unit merges into), and the guard `docs/plan/slice-10/guard-async-token.cases.txt` (the 241 cases with a passing run in that base run whose text has the word `async` or `await`: they must keep passing when the parse of a method changes), `cat .out/story-async-methods.cases.txt docs/plan/slice-10/guard-async-token.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-10/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt test/language/expressions/object/method-definition` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; the list, the guard and every case of method definitions, generator methods among them: 555 cases, 1,003 runs over the base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-10/check.sh .out/story-async-methods.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the base run of the property-escapes join (the newest full run, made before generators and the flag `i` were merged and before the first wave of this slice; 18,935 of 44,812 cases pass) it exits 1, prints 84 lines `failing` and `passed 288 bound 288 B 288 F 52 R 52`): every run of the list passes, no run that passed on the base fails, and the part has at least B + F − R passing cases; (b) `awk -F'\t' '$6 ~ /async method is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0`. Over the property-escapes base run the same command prints `0` too, since the text came with the first wave; so the unit runs it over `.out/test262-base/runs.tsv`, the base run of the join it starts from, before its first change and reports that count (expected about 84, the runs of the list), and the check that cannot hold before the work is (a), whose count of passing cases must reach B + F − R. On the join, the coordinator's full run, made in parts, passes (a) over the whole run, and (b) over the whole run prints `0` (its count on the join after the first wave is what that base prints; the runs of `async *` methods end on `async generator method is not supported before slice 10`).

## Out of scope

`async *` methods (`story:async-generator-methods-in-object-literals`); `async` methods of classes (slice 8); generator methods (`story:generator-methods-in-object-literals`); the moves of cases and the slice lines of the design (the conformance close of this slice).

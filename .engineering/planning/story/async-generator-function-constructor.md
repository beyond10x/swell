---
format: aep.planning-md/3
id: story:async-generator-function-constructor
kind: story
status: draft
title: '%AsyncGeneratorFunction% makes async generators from source text through CreateDynamicFunction and the host hook'
summary: The kind async generator of CreateDynamicFunction, %AsyncGeneratorFunction% and the constructor of its prototype; 21 cases.
relations:
- decomposes: epic:slice-10-promises-and-async
- depends_on: story:async-generators-and-their-request-queue
- depends_on: story:async-function-constructor
- depends_on: story:generator-function-constructor
scope:
- confidence: cited
  path: crates/swell/src/builtins/async_generator.rs
- confidence: cited
  path: crates/swell/src/dynamic_code.rs
- confidence: cited
  path: crates/swell/src/object/builtin_function.rs
- confidence: cited
  path: crates/swell/src/realm/intrinsics.rs
- confidence: cited
  path: crates/swell/tests/dynamic_function.rs
- confidence: cited
  path: crates/syntax/src/lib.rs
- confidence: cited
  path: crates/syntax/src/parser/function.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
revision: 9
---
## Context

Slice 10 brings jobs, `Promise`, `async` functions and generators and `for await` (`epic:slice-10-promises-and-async`; `docs/design/engine.md`, section 12, item 10). This story brings `%AsyncGeneratorFunction%` (`#sec-asyncgeneratorfunction-constructor`): CreateDynamicFunction of kind `async generator` through the one parse entry and the one engine entry that `story:eval-and-function-constructor` made and `story:generator-function-constructor` and `story:async-function-constructor` extend, with HostEnsureCanCompileStrings and every limit, and the `constructor` of `%AsyncGeneratorFunction.prototype%`. It is not a global. It depends on `story:async-generators-and-their-request-queue` (`%AsyncGeneratorFunction.prototype%` and the async generator template), on `story:async-function-constructor` (the kind `async` beside which this one goes) and on the open story of slice 7 `story:generator-function-constructor` (the kind `generator`, `%GeneratorFunction%`, whose case on this list reads both).

The text this story makes false and removes: the documentation of `parse_dynamic_function` (`crates/syntax/src/lib.rs`, line 214 on `397ffff`), which after `story:async-function-constructor` names only `async generator` (slice 10) as a kind with no entry. No refusal text of the parser names the constructor: on its base a script that reaches `%AsyncGeneratorFunction%` gets a `TypeError` (the `constructor` of its prototype is undefined). The search `grep -rn '`async generator` (slice 10)' crates/syntax/src crates/swell/src` prints on its base that line of `lib.rs`; after this story it prints nothing.

The list of this story is `docs/plan/slice-10/story-ag-constructor.cases.txt`: the 21 cases of the key `ag-constructor` of `docs/plan/slice-10/later.tsv` (the command is in the head of `docs/plan/slice-10/scan-later.awk`): `built-ins/AsyncGeneratorFunction` 20 and `language/comments/hashbang` 1 (a hashbang in the body text of the constructor); one of them reads `%GeneratorFunction%` too. They have 42 runs in the base run of the property-escapes join (the newest full run, made before generators and the flag `i` were merged and before the first wave of this slice; 18,935 of 44,812 cases pass), all failing on `async function is not supported before slice 10` (the text before the first wave). A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on, by name: decision 3 of `docs/design/engine.md` (the limits of a parse and a compile at run time), `docs/design/host-interface.md` HostEnsureCanCompileStrings and the counted refusals; the form of the constructors of `story:generator-function-constructor` and `story:async-function-constructor`. No sentence is missing from the design.

## What later stories reuse

Nothing of this slice: the four kinds of CreateDynamicFunction are complete with it.

## Domain relations

- `%AsyncGeneratorFunction%` → `%AsyncGeneratorFunction.prototype%`: one to one, each the `prototype` and the `constructor` of the other, per realm — inferable (inferred from ECMA-262 `#sec-asyncgeneratorfunction-prototype` and `#sec-asyncgeneratorfunction-prototype-constructor`; no ess/1 document declares it: the host model `spec/domains/host.yaml` holds no engine value, and ESS does not restate the intrinsics of ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base (`%AsyncGeneratorFunction%` does not exist).

- `swell-syntax`: the parse entry of a dynamic async generator beside the entries of a dynamic generator and a dynamic `async` function (`crates/syntax/src/lib.rs`; `crates/syntax/src/parser/mod.rs`, `parse_dynamic_function`; `crates/syntax/src/parser/function.rs`, `dynamic_function_text`): the parameter text alone as `FormalParameters[+Yield, +Await]`, the body alone as `AsyncGeneratorBody`, then `async function* anonymous(<P>\n) {\n<body>\n}` as an `AsyncGeneratorExpression` with its early errors; the documentation of `parse_dynamic_function` names no kind as waiting.

- `swell`: `crates/swell/src/dynamic_code.rs`, the kind `async generator` of `create_dynamic_function` and `dynamic_function_steps` (fallbackProto `%AsyncGeneratorFunction.prototype%`, the source text, a `prototype` property whose [[Prototype]] is `%AsyncGeneratorPrototype%`, the instantiation of an async generator template of `story:async-generators-and-their-request-queue`); `crates/swell/src/builtins/async_generator.rs`: `%AsyncGeneratorFunction%` (`#sec-asyncgeneratorfunction`: [[Prototype]] `%Function%`, `length` 1, `name` "AsyncGeneratorFunction", `prototype` `%AsyncGeneratorFunction.prototype%`, not writable) and the `constructor` of `%AsyncGeneratorFunction.prototype%`; its `BuiltinId` and its arm in the dispatch (`crates/swell/src/object/builtin_function.rs`); `Intrinsic::AsyncGeneratorFunction` (`crates/swell/src/realm/intrinsics.rs`: the enum, `get`, `trace`, `create_intrinsics` and its list).

- Bounds, as for `Function`: the parse and the compile charge before they grow and read the interrupt; the hook is called before the parse; a refusal through it is counted. No new kind, no new instruction.

- Tests, each in both collection modes, in `crates/swell/tests/dynamic_function.rs`: `async_generator_function_constructor_makes_an_async_generator_from_source_text`, `async_generator_function_constructor_parses_yield_and_await_in_its_body_only`, `async_generator_function_constructor_is_refused_by_the_hook_of_compile_strings`, `async_generator_function_constructor_source_text_starts_with_async_function_star_anonymous`, `async_generator_function_constructor_is_not_a_global_and_is_the_constructor_of_its_prototype`. `host_objects::host_186_a_host_function_charges_every_byte` and `host_objects::host_187_a_realm_of_300_host_functions_and_40_host_objects` are run in both modes and the reading of HOST-187 is reported. The sweep of every function with a Symbol does not reach `%AsyncGeneratorFunction%` from the global object; the unit reports whether it has other roots, as `story:async-function-constructor` found. `crates/swell/tests/main.rs` does not change.

- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with a collection at every safe point and under 2.5 at its full count (`SWELL_FULL_COUNTS=1`); measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system). The unit times each test before it adds it and names each one over 1 CPU second with its time; no loop of a test runs more than 1,000 iterations; the tests of this story are expected to add under 3 CPU seconds to the gate in both modes together, and the unit reports the sum it measured (the test steps of the gate cost about 330 CPU seconds). A product several tests share is built once inside one test function or is a committed file, never in a `static`; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, the most); a script that measures or waits runs without the prefix. No Python in any tool, test or command; no `cd ... &&` with a write or a delete; intermediate files in the scratch directory.
- Size: about 350 lines (the parse entry 80, the kind and the constructor 120, the tests 150): one session.

## Requirements

None of `conformance/host/requirements.txt`.

Writes: `crates/syntax/src/lib.rs` (the entry and the documentation of `parse_dynamic_function`), `crates/syntax/src/parser/{mod,function}.rs` (`parse_dynamic_function`, `dynamic_function_text`); `crates/swell/src/dynamic_code.rs`, `crates/swell/src/builtins/async_generator.rs`, `crates/swell/src/object/builtin_function.rs`, `crates/swell/src/realm/intrinsics.rs`; `crates/swell/tests/dynamic_function.rs` (new tests). Checked against the code on `397ffff` and the Writes lines of the stories before it: the matches without a wildcard this story adds arms to are `BuiltinId` and `Intrinsic`, and the kind of CreateDynamicFunction as the two constructor stories leave it; no `ObjectKind`, no `Instruction`. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/swell/src/host/{hooks,limits}.rs`, `conformance/host/surface.txt`, `crates/swell/tests/host_objects.rs`, `crates/syntax/tests/sources.rs`, `crates/compiler`, `crates/test262`, `conformance/`, `website/`. Shared with the stories of its wave, each in hunks of its own: `crates/syntax/src/parser/mod.rs` and `crates/syntax/src/lib.rs` (`story:yield-star-in-async-generators`, the documentation of `held` and of `parse_script`; this story, the dynamic entry), `crates/swell/src/object/builtin_function.rs` (`story:array-from-async`, the `BuiltinId` of `Array.fromAsync` and the functions of its steps; this story, its own variant). Shared with the open stories of slice 7: the same files with `story:generator-function-constructor`, which this story follows by its edge.

## Acceptance

The tests named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test main -E 'test(/^dynamic_function::/) | test(/^async_generators::/) | test(/^eval::/) | test(/^hooks::/) | test(/^frontend_ledger::/) | test(/^host_objects::host_186_a_host_function_charges_every_byte$/) | test(/^host_objects::host_187/)'` and the same with `--features collect-every-safe-point`, and `cargo nextest run --locked -p swell-syntax --test main`; each exits 0. `slot task clippy` exits 0. The text: the search of Context prints nothing. Time: every test this story adds stays under 1.5 CPU seconds alone in the default mode and under 2 with `collect-every-safe-point`, as the unit measured them (Change, "Time"); the report lists each measured time and the sum, and a time over its bound fails this acceptance.

test262. The full run is not the unit's: the unit runs the runner on a part of the checkout. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-10/story-ag-constructor.cases.txt" > .out/story-ag-constructor.cases.txt` (`WAVE` the wave branch the unit merges into), and the guard `docs/plan/slice-10/guard-async-token.cases.txt` (the 241 cases with a passing run in that base run whose text has the word `async` or `await`), `cat .out/story-ag-constructor.cases.txt docs/plan/slice-10/guard-async-token.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-10/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt test/built-ins/GeneratorFunction test/built-ins/AsyncFunction test/built-ins/AsyncGeneratorFunction` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; the list, the guard and the cases of the other dynamic constructors, whose entry this story shares: 305 cases, 562 runs over the base run; on the join the cases of `%GeneratorFunction%` and `%AsyncFunction%` pass and count in B), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-10/check.sh .out/story-ag-constructor.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the base run of the property-escapes join (the newest full run, made before generators and the flag `i` were merged and before the first wave of this slice; 18,935 of 44,812 cases pass) it exits 1, prints 42 lines `failing` and `passed 241 bound 241 B 241 F 21 R 21`): every run of the list passes, no run that passed on the base fails, and the part has at least B + F − R passing cases. On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

`%GeneratorFunction%` and `%AsyncFunction%` (`story:generator-function-constructor`, `story:async-function-constructor`); the moves of cases and the slice lines of the design (the conformance close of this slice).

---
format: aep.planning-md/3
id: story:async-function-constructor
kind: story
status: draft
title: '%AsyncFunction% makes async functions from source text through CreateDynamicFunction and the host hook'
summary: The kind async of CreateDynamicFunction beside the generator kind, %AsyncFunction% and the constructor of %AsyncFunction.prototype%; 21 cases.
relations:
- decomposes: epic:slice-10-promises-and-async
- depends_on: story:async-functions-and-await
- depends_on: story:generator-function-constructor
scope:
- confidence: cited
  path: crates/swell/src/builtins/async_function.rs
- confidence: cited
  path: crates/swell/src/dynamic_code.rs
- confidence: cited
  path: crates/swell/src/object/builtin_function.rs
- confidence: cited
  path: crates/swell/src/realm/intrinsics.rs
- confidence: cited
  path: crates/swell/tests/builtins_symbol_values.rs
- confidence: cited
  path: crates/swell/tests/dynamic_function.rs
- confidence: cited
  path: crates/syntax/src/lib.rs
- confidence: cited
  path: crates/syntax/src/parser/function.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
revision: 10
---
## Context

Slice 10 brings jobs, `Promise`, `async` functions and generators and `for await` (`epic:slice-10-promises-and-async`; `docs/design/engine.md`, section 12, item 10). This story brings `%AsyncFunction%` (`#sec-async-function-constructor`): CreateDynamicFunction of kind `async` through the one parse entry and the one engine entry of the `Function` constructor (`crates/swell/src/dynamic_code.rs`, `function_constructor` line 641, `create_dynamic_function` line 671, `dynamic_function_steps` line 690 on `397ffff`), with the host hook HostEnsureCanCompileStrings and every limit. It is not a global: a script reaches it as `Object.getPrototypeOf(async function () {}).constructor`. It depends on `story:async-functions-and-await` (`%AsyncFunction.prototype%` and the `async` template) and on the open story of slice 7 `story:generator-function-constructor`, which brings the kind `generator` the same way: the parse entry of a dynamic generator beside `parse_dynamic_function` (`crates/syntax/src/lib.rs` lines 196 to 245), `dynamic_function_text` (`crates/syntax/src/parser/function.rs`) and the kind in `dynamic_code.rs`; this story adds the kind `async` in the form that story leaves, so the edge orders the two.

The text this story makes false and removes: the documentation of `parse_dynamic_function` (`crates/syntax/src/lib.rs` line 214 on `397ffff`: "The kinds `generator` (slice 7), `async` (slice 10) and `async generator` (slice 10) come with their constructors and have no entry before then"), which after `story:generator-function-constructor` still names `async` (slice 10). No refusal text of the parser names the constructor: on its base a script that calls `%AsyncFunction%` gets a `TypeError` (`constructor` of `%AsyncFunction.prototype%` is undefined after `story:async-functions-and-await`). The search `grep -rn '`async` (slice 10)' crates/syntax/src crates/swell/src` prints on `397ffff` that line of `lib.rs` and line 697 of `crates/syntax/src/parser/statement.rs` (`for_head`, which `story:for-await-and-async-from-sync-iterator` rewords); after this story it prints neither line of `lib.rs`, and the documentation names only `async generator` (slice 10), which `story:async-generator-function-constructor` removes.

The list of this story is `docs/plan/slice-10/story-function-constructor.cases.txt`: the 21 cases of the key `function-constructor` of `docs/plan/slice-10/later.tsv` (the command is in the head of `docs/plan/slice-10/scan-later.awk`). By directory: `built-ins/AsyncFunction` 15, `harness` 3, `built-ins/Object` 2, `built-ins/Function` 1. They have 42 runs in the base run of the property-escapes join (the newest full run, made before generators and the flag `i` were merged and before the first wave of this slice; 18,935 of 44,812 cases pass), all failing on `async function is not supported before slice 10` (the text before the first wave). A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on, by name: decision 3 of `docs/design/engine.md` (the limits of a parse and a compile at run time, as for `eval` and `Function`), `docs/design/host-interface.md` HostEnsureCanCompileStrings and the counted refusals; the form `story:eval-and-function-constructor` fixed and `story:generator-function-constructor` extends. No sentence is missing from the design.

## What later stories reuse

The kind `async` of CreateDynamicFunction, beside which `story:async-generator-function-constructor` adds the kind `async generator`; `%AsyncFunction%`, the `constructor` of `%AsyncFunction.prototype%` and the [[Prototype]] of `%AsyncGeneratorFunction%`.

## Domain relations

- `%AsyncFunction%` → `%AsyncFunction.prototype%`: one to one, each the `prototype` and the `constructor` of the other, per realm — inferable (inferred from ECMA-262 `#sec-async-function-constructor-prototype` and `#sec-async-function-prototype-properties-constructor`; no ess/1 document declares it: the host model `spec/domains/host.yaml` holds no engine value, and ESS does not restate the intrinsics of ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base (`%AsyncFunction%` does not exist).

- `swell-syntax`: the parse entry of a dynamic `async` function beside the entry of a dynamic generator (`crates/syntax/src/lib.rs`, in the form `story:generator-function-constructor` leaves; `crates/syntax/src/parser/mod.rs`, `parse_dynamic_function`; `crates/syntax/src/parser/function.rs`, `dynamic_function_text`): the parameter text alone as `FormalParameters[~Yield, +Await]`, the body alone as `AsyncFunctionBody`, then `async function anonymous(<P>\n) {\n<body>\n}` as an `AsyncFunctionExpression` with its early errors (`#sec-createdynamicfunction`, steps for kind `async`); the documentation of `parse_dynamic_function` no longer names `async` (slice 10).

- `swell`: `crates/swell/src/dynamic_code.rs`, the kind `async` of `create_dynamic_function` and `dynamic_function_steps` (fallbackProto `%AsyncFunction.prototype%`, the source text, no `prototype` property, the instantiation of an `async` template of `story:async-functions-and-await`); `crates/swell/src/builtins/async_function.rs`: `%AsyncFunction%` (`#sec-asyncfunction`: [[Prototype]] `%Function%`, `length` 1, `name` "AsyncFunction", `prototype` `%AsyncFunction.prototype%`, not writable) and the `constructor` of `%AsyncFunction.prototype%`; its `BuiltinId` and its arm in the dispatch (`crates/swell/src/object/builtin_function.rs`); `Intrinsic::AsyncFunction` (`crates/swell/src/realm/intrinsics.rs`: the enum, `get`, `trace`, `create_intrinsics` and its list).

- Bounds, as for `Function`: the parse and the compile charge before they grow and read the interrupt; the hook is called before the parse; a refusal through it is counted. No new kind, no new instruction.

- Tests, each in both collection modes, in `crates/swell/tests/dynamic_function.rs` (the module of the constructors, where `story:generator-function-constructor` puts its tests): `async_function_constructor_makes_an_async_function_from_source_text`, `async_function_constructor_parses_await_in_its_body_only`, `async_function_constructor_is_refused_by_the_hook_of_compile_strings`, `async_function_constructor_source_text_starts_with_async_function_anonymous`, `async_function_constructor_is_not_a_global_and_is_the_constructor_of_its_prototype`, `async_function_constructor_charges_its_parse_and_reads_the_interrupt`. The sweep of every function with a Symbol (`crates/swell/tests/builtins_symbol_values.rs`) walks from the global object only, so it does not reach `%AsyncFunction%`; the unit adds `%AsyncFunction%` to its roots if the sweep has a list of intrinsics that are not globals, and reports which. `host_objects::host_186_a_host_function_charges_every_byte` and `host_objects::host_187_a_realm_of_300_host_functions_and_40_host_objects` are run in both modes and the reading of HOST-187 is reported. `crates/swell/tests/main.rs` does not change (no new module).

- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with a collection at every safe point and under 2.5 at its full count (`SWELL_FULL_COUNTS=1`); measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system). The unit times each test before it adds it and names each one over 1 CPU second with its time; no loop of a test runs more than 1,000 iterations; the tests of this story are expected to add under 3 CPU seconds to the gate in both modes together, and the unit reports the sum it measured (the test steps of the gate cost about 330 CPU seconds). A product several tests share is built once inside one test function or is a committed file, never in a `static`; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, the most); a script that measures or waits runs without the prefix. No Python in any tool, test or command; no `cd ... &&` with a write or a delete; intermediate files in the scratch directory.
- Size: about 450 lines (the parse entry 100, the kind and the constructor 150, the tests 200): one session.

## Requirements

None of `conformance/host/requirements.txt`.

Writes: `crates/syntax/src/lib.rs` (the entry beside the generator entry and the documentation of `parse_dynamic_function`), `crates/syntax/src/parser/{mod,function}.rs` (`parse_dynamic_function`, `dynamic_function_text`); `crates/swell/src/dynamic_code.rs`, `crates/swell/src/builtins/async_function.rs`, `crates/swell/src/object/builtin_function.rs`, `crates/swell/src/realm/intrinsics.rs`; `crates/swell/tests/dynamic_function.rs` (new tests), `crates/swell/tests/builtins_symbol_values.rs` (only as Change says). Checked against the code on `397ffff` and the Writes line of `story:generator-function-constructor`: the matches without a wildcard this story adds arms to are `BuiltinId` (the dispatch of `builtin_function.rs`) and `Intrinsic` (`intrinsics.rs` `get` and `trace`), both on the line, and the kind of CreateDynamicFunction as that story leaves it; no `ObjectKind`, no `Instruction`. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/swell/src/host/{hooks,limits}.rs`, `conformance/host/surface.txt`, `crates/swell/tests/host_objects.rs`, `crates/syntax/tests/sources.rs` (the new entry is shown through the engine's tests), `crates/compiler`, `crates/test262`, `conformance/`, `website/`. Shared with the stories of its wave, each in hunks of its own: `crates/swell/src/object/builtin_function.rs` and `crates/swell/src/realm/intrinsics.rs` (`story:promise-statics-finally-and-aggregate-error` and `story:for-await-and-async-from-sync-iterator`, each its own variants and intrinsics), `crates/syntax/src/parser/function.rs` (`story:async-generator-syntax-and-early-errors`, the head of `async function*`; this story, `dynamic_function_text`), `crates/syntax/src/lib.rs` (no other story of its wave). Shared with the open stories of slice 7: the same files with `story:generator-function-constructor`, which this story follows by its edge.

## Acceptance

The tests named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test main -E 'test(/^dynamic_function::/) | test(/^async_functions::/) | test(/^eval::/) | test(/^hooks::/) | test(/^frontend_ledger::/) | test(/^builtins_symbol_values::/) | test(/^host_objects::host_186_a_host_function_charges_every_byte$/) | test(/^host_objects::host_187/)'` and the same with `--features collect-every-safe-point`, and `cargo nextest run --locked -p swell-syntax --test main`; each exits 0. `slot task clippy` exits 0. The text: `grep -n '`async` (slice 10)' crates/syntax/src/lib.rs` prints nothing. Time: every test this story adds stays under 1.5 CPU seconds alone in the default mode and under 2 with `collect-every-safe-point`, as the unit measured them (Change, "Time"); the report lists each measured time and the sum, and a time over its bound fails this acceptance.

test262. The full run is not the unit's: the unit runs the runner on a part of the checkout. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-10/story-function-constructor.cases.txt" > .out/story-function-constructor.cases.txt` (`WAVE` the wave branch the unit merges into), `sh docs/plan/slice-10/part.sh "$CHECKOUT" .out/test262-part .out/story-function-constructor.cases.txt test/built-ins/Function test/built-ins/AsyncFunction` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; the list and every case of `Function` and `AsyncFunction`, whose constructors share the entry this story extends, the guard: 532 cases, 939 runs over the base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-10/check.sh .out/story-function-constructor.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the base run of the property-escapes join (the newest full run, made before generators and the flag `i` were merged and before the first wave of this slice; 18,935 of 44,812 cases pass) it exits 1, prints 42 lines `failing` and `passed 409 bound 409 B 409 F 21 R 21`): every run of the list passes, no run that passed on the base fails, and the part has at least B + F − R passing cases. On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

`%AsyncGeneratorFunction%` and the kind `async generator` (`story:async-generator-function-constructor`); `%GeneratorFunction%` (`story:generator-function-constructor`); the moves of cases and the slice lines of the design (the conformance close of this slice).

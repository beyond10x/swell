---
format: aep.planning-md/3
id: story:generator-methods-in-object-literals
kind: story
status: implemented
title: Generator methods of object literals and their early errors
summary: The parse and early errors of GeneratorMethod in an object literal; removes the refusal generator method is not supported before slice 7; no compiler or engine file; 280 cases (525 runs), 34 of them negative parse cases.
relations:
- decomposes: epic:slice-07-generators
- depends_on: story:generator-functions-and-suspended-frames
- depends_on: story:yield-star-delegation
scope:
- confidence: inferred
  path: crates/compiler/tests/sources/generator-method.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/generator-method.js
- confidence: cited
  path: crates/swell/tests/generator_methods.rs
- confidence: cited
  path: crates/swell/tests/generators.rs
- confidence: inferred
  path: crates/swell/tests/language/generator-methods.js
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/tests/sources.rs
- confidence: inferred
  path: crates/syntax/tests/sources/early-errors/generator-method-yield-in-parameters.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/generator-method.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/generator-method.js
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T11:07:28Z", actor: "human:operator", revision: 9, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T11:07:28Z", actor: "human:operator", revision: 10, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-26T05:55:46Z", actor: "human:operator", revision: 11, decided_on: {"recorded":{"test_result":1}}}
---
## Context

Slice 7, fourth wave (`epic:slice-07-generators`). A generator method of an object literal (`{ *g() { yield 1; } }`, MethodDefinition : GeneratorMethod, `#sec-generator-function-definitions`, `#sec-runtime-semantics-methoddefinitionevaluation`) with its early errors. It depends on `story:generator-functions-and-suspended-frames` and comes after `story:yield-star-delegation` (Writes says why), and the second story's `instantiate_function` makes every template with `generator` set a generator function whatever its kind, and the first story's `function_rest` parses a function's parameters and body with the [Yield] its `generator` field gives; so this story writes the parser, the early errors and tests, and no file of the compiler or the engine. The consumer of the slice needs slice 10, not this story.

On its base (the join after the second story) `accessor_kind` (`crates/syntax/src/parser/expression.rs` line 1365 on `4971b9b`) refuses a `*` in an object literal with `generator method is not supported before slice 7`. The refusal this story removes is that text. The search for what expects it, `grep -rn 'generator method is not supported\|"generator method"' crates conformance docs/design taskfiles tests website/docs`, prints on its base `accessor_kind`, line 1 of `crates/syntax/tests/sources/refused/generator-method.js` and the test `the_refusal_of_a_generator_method_is_counted` of `crates/swell/tests/generators.rs`; after it, nothing. No fixture of the runner holds a generator method, so no line of `crates/test262` moves. Generator methods of classes stay with slice 8 (`class` is refused there).

The list is `docs/plan/slice-07/story-generator-methods.cases.txt`: the 280 cases of the key `methods` of `docs/plan/slice-07/scan.tsv` (the head of `scan.awk` gives the command), 525 runs, all failing on `4971b9b` on `generator function is not supported before slice 7`. By directory: `language/expressions/object` 261 (`method-definition/` most of them), `language/eval-code/direct` 12 (the `gen-meth-` cases), `language/arguments-object` 5, `language/computed-property-names/object` 1, `built-ins/Function/prototype/toString` 1 (`generator-method.js`). 34 are negative parse cases, the early errors of Change. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on: decision 3 of `docs/design/engine.md`, "Depth bounds" and the stack bound of the parser (`STACK_BYTES_PER_LEVEL`, `crates/syntax/src/lib.rs` line 80); "Suspended frames" through the second story, unchanged.

## What slice 10 reuses

The parse of a method whose kind sets the [Yield] of its parameters and body: the form of an async method and an async generator method (slice 10).

## Domain relations

- Generator method → its object literal: many to one; the method is a property of the object, enumerable, and its [[HomeObject]] is the object — inferable (inferred from `crates/swell/src/interp/object_literal.rs:64`, DefineMethod with MakeMethod, and ECMA-262 `#sec-runtime-semantics-methoddefinitionevaluation`; no ess/1 document declares it: the host model `spec/domains/host.yaml` holds no engine value, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base (the refusal).

- `crates/syntax/src/parser/expression.rs`, `accessor_kind` (line 1365) and the functions of a property after it (`value_property_rest`, line 1450, and the method head at line 1511): a `*` begins a GeneratorMethod, `* ClassElementName ( UniqueFormalParameters[+Yield, ~Await] ) { GeneratorBody }`; the function is begun as `FunctionKind::Method` with `generator: true` and its parameters and body are parsed by `function_rest`; the property name is read with the [Yield] of the context, so `[yield]` inside a generator is a YieldExpression. The refusal goes.
- `crates/syntax/src/early.rs`, the walk of functions at line 1013 (the branch of `FunctionKind::Method`): the early errors of GeneratorMethod (`#sec-generator-function-definitions-static-semantics-early-errors`: UniqueFormalParameters Contains YieldExpression; a body with a Use Strict Directive and a parameter list that is not simple; a parameter name the body declares lexically; duplicate parameters in every mode, as UniqueFormalParameters asks; HasDirectSuper needs `super`, refused before slice 8), with the checks of the first story for a function whose `generator` is set.
- `crates/syntax/tests/sources.rs`: the nesting kind `generator_method` (generator methods each holding an object literal with the next), added to `NESTING_KINDS` (53 after the first story, 54 after) and to `nested`; a kind whose level takes more than `STACK_BYTES_PER_LEVEL` gets its path split into functions, never a new value of the constant, so `lib.rs` line 80 stays; the unit reports its bytes per level. Fixtures: `refused/generator-method.js` removed, `valid/generator-method.js` and `early-errors/generator-method-*.js` (new, one per early error above).
- `crates/compiler/tests/sources/generator-method.js` and `.disasm` (new): the template of a generator method, its kind and its start.
- Tests, each in both collection modes, in `crates/swell/tests/generator_methods.rs` (new): `a_generator_method_is_a_generator_function_with_its_prototype_object` ([[Prototype]] `%GeneratorFunction.prototype%`, a `prototype` property writable, not enumerable, not configurable, whose [[Prototype]] is `%GeneratorPrototype%`; `new o.g()` throws a `TypeError`), `a_generator_method_runs_as_a_generator` (`next`, `return`, `throw`), `a_generator_method_with_a_computed_name_gets_its_name`, `yield_in_a_computed_name_inside_a_generator_belongs_to_the_outer_generator`, `a_generator_method_is_an_enumerable_property`. `crates/swell/tests/generators.rs`: the test `the_refusal_of_a_generator_method_is_counted` goes. `crates/swell/tests/main.rs` generated again (`sh taskfiles/test-main.sh write`). Language script `crates/swell/tests/language/generator-methods.js` (new). No new function of the realm, so neither the sweep with a Symbol nor HOST-187 is touched.

- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with a collection at every safe point and under 2.5 at its full count (`SWELL_FULL_COUNTS=1`) in either mode; a language script under 0.5 CPU seconds per run; measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system). The unit times each test before it adds it and names each one over 1 CPU second with its time; no loop of a test runs more than 1,000 iterations; a story whose tests add more than 3 CPU seconds to the gate in both modes together says so and gives the gate's counts (its test steps cost about 330 CPU seconds). A product several tests share is built once inside one test function or is a committed file, never in a `static`; no test makes a network request; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m`, the most); a script that measures or waits runs without the prefix. No Python in any tool, test or command; no `cd ... &&` with a write or a delete; intermediate files in the scratch directory.

- Size: about 450 lines (the parser 100, the early errors 80, the tests and fixtures 270): one session.

## Requirements

None of `conformance/host/requirements.txt`.

Writes: `crates/syntax/src/parser/expression.rs` (`accessor_kind` and the functions of a property, lines 1340 to 1520), `crates/syntax/src/early.rs` (the branch of a method), `crates/syntax/tests/sources.rs` (`NESTING_KINDS`, `nested`), `crates/syntax/tests/sources/refused/generator-method.js` (removed), `crates/syntax/tests/sources/valid/generator-method.js` and `early-errors/generator-method-*.js` (new); `crates/compiler/tests/sources/generator-method.{js,disasm}` (new); `crates/swell/tests/generator_methods.rs` (new), `crates/swell/tests/generators.rs` (one test out), `crates/swell/tests/main.rs` (generated), `crates/swell/tests/language/generator-methods.js` (new). Checked against the code: no enum variant is added (`FunctionKind::Method` with `generator` from the first story), so no match changes; the fixture directories are read by `read_dir`. Not written: `crates/syntax/src/{lib,ast}.rs`, `parser/{mod,function,statement}.rs` (the place of the held refusal, a field of the parser state and of `Parsed` and its branch in `analyse_parsed`, stays, holding none, past slice 7 for slice 10: no story of slice 7 removes it, and the form `story:generator-syntax-and-early-errors` fixes leaves nothing in it unused), every source file of `crates/compiler` and `crates/swell` (if the unit finds one needed, it stops and reports), `crates/test262`, `conformance/`, `website/`. The only story of its wave. It comes after `story:yield-star-delegation`, which writes other functions of `crates/syntax/src/parser/expression.rs` (the function of YieldExpression) and of `crates/syntax/tests/sources.rs` (`admitted`, line 603), and `crates/swell/tests/{generators,main}.rs`: a unit works on files no other unit of its wave writes (`AGENTS.md`, "How the work is run"), so the two are not run at once; this story starts from the join after it.

## Acceptance

The tests named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test main -E 'test(/^generator_methods::/) | test(/^generators::/) | test(/^language::/) | test(/^objects::/)'` and the same with `--features collect-every-safe-point`, `cargo nextest run --locked -p swell-syntax --test main` and the same with `SWELL_FULL_COUNTS=1`, and `cargo nextest run --locked -p swell-compiler --test main` exit 0; `slot task clippy` exits 0. Time: every test this story adds stays under 1.5 CPU seconds alone in the default mode and under 2 with `collect-every-safe-point`, each part of `every_nesting_kind_fits_the_stack_bound` under 1.5 at the gate's count and 2.5 at the full count, and its language script under 0.5 per run, as the unit measured them; the report lists each time, and a time over its bound fails this acceptance. `slot --timeout 30m task test262` is run once after the change (in the background) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint; on its run lines, with the list as committed on the wave branch when the wave closes copied by `git show "$WAVE:docs/plan/slice-07/story-generator-methods.cases.txt" > .out/story-generator-methods.cases.txt` and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv` and `.out/test262-base/lines.txt`: (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-generator-methods.cases.txt .out/test262/runs.tsv` exits 0 (on its base it exits 1 and prints the failing runs of the F cases; 525 runs on `4971b9b`); (b) `awk -F'\t' '$6 ~ /generator method is not supported/' .out/test262/runs.tsv | wc -l` prints `0` (over `.out/test262-base/runs.tsv`, the join after `story:yield-star-delegation`, it prints at least the 525 runs of the list, which all end on that text there; on `4971b9b` it prints `0` too, since the text first exists after the second story); (c) `awk '$1 == "test262-slice" && $2 == "07" { print $8 }' .out/test262/lines.txt` (field 8, passed) prints at least B + F − R, and `awk '$1 == "test262" && $2 == "language/expressions/object" { print $7 }' .out/test262/lines.txt` (field 7, passed) at least B1 + F1 − R1, B and B1 what the same commands print over `.out/test262-base/lines.txt`, F the cases of the list with a failing run in the base run (`awk -F'\t' 'NR == FNR { w[$1] = 1; next } ($3 in w) && $5 != "Passed" { f[$3] = 1 } END { print length(f) }' .out/story-generator-methods.cases.txt .out/test262-base/runs.tsv`; 280 on `4971b9b`), F1 the same with `&& $3 ~ /^test\/language\/expressions\/object\//` added to its condition (261 on `4971b9b`), R and R1 the cases among the runs (a) prints; (d) `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing.

## Out of scope

`yield*` (`story:yield-star-delegation`) and the two cases that need it in a generator method (`docs/plan/slice-07/close-cross.cases.txt`, checked by the conformance close); generator methods of classes and `super` (slice 8; the 40 cases of `close-later-8.cases.txt`); async methods (slice 10); `%GeneratorFunction%` (`story:generator-function-constructor`).

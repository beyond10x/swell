---
format: aep.planning-md/3
id: story:public-class-fields
kind: story
status: active
title: Public instance and static fields, their initializers run as frames of the run loop
summary: 515 cases (1,029 runs); lifts class field.
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:class-definition-evaluation
- depends_on: story:class-element-syntax-and-early-errors
scope:
- confidence: inferred
  path: crates/compiler/src/class.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: inferred
  path: crates/compiler/tests/sources/class-fields.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/class-fields.js
- confidence: inferred
  path: crates/swell/tests/class_fields.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: inferred
  path: crates/syntax/src/parser/class.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: inferred
  path: crates/syntax/src/scope.rs
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T08:23:47Z", actor: "human:operator", revision: 10, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-26T08:23:47Z", actor: "human:operator", revision: 11, executor: "agent:coordinator"}
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story runs public fields, instance and static, as compiled code (item 4 of "Classes"): ClassFieldDefinitionEvaluation (a computed key evaluated once at the class's definition, in source order with the other elements, converted by `ToPropertyKey` and kept in a hidden binding of the class scope, item 2), the instance fields as pass 2 of the instance initializer (`initializer_fields` of `story:class-definition-evaluation`: each value evaluated or `undefined`, an anonymous function named by the field's name, defined by `DefineField` or `DefineComputedField`, CreateDataPropertyOrThrow, so a non-extensible receiver or a non-configurable property throws a `TypeError`), and each run of static fields as its own `FieldInitializer` function, called with `this` F in the step-4 sequence (`<F>; MakeMethod(f); Call(0); Pop`). It fills one arm of the step-4 sequence that `story:class-definition-evaluation` owns (`static_calls`, `crates/compiler/src/class.rs`): the arm of a run of static fields, by writing the function that arm calls, `static_field_run`, which answers the run's template; it writes only that function: the sequence places the call, and this story places no call and changes no order. The initializer runs with the instance as `this`, `new.target` undefined and SuperProperty through its home object (the prototype, or F for a static run). No Rust loop and no list of fields exist (item 4, decision 7 of slice 8); `ecmascript_function.rs` is not edited. It lifts the hold `class field is not supported before slice 8` of `story:class-element-syntax-and-early-errors`.

Edges: `story:class-definition-evaluation` (the functions `compile_field`, `initializer_fields`, `field_key_bindings`, `InitInstance`, the step-4 sequence) and `story:class-element-syntax-and-early-errors` (the fields' tree, their run functions and context). Order 3, built beside `story:derived-classes-and-super-call` and `story:private-names-and-private-methods`. `story:class-static-blocks` comes after it by an edge (it fills the other arm of the same step-4 sequence; `static-init-sequence.js` on its list needs both).

An initializer runs as a frame of the run loop pushed by `InitInstance` or by `Call(0)`, not as a re-entry, so the step budget, the frame depth and an interrupt reach it as they reach a call.

The list is `docs/plan/slice-08/story-public-fields.cases.txt`: the 515 cases of the key `public-fields` of `docs/plan/slice-08/scan.tsv` (a class with public fields and no heritage, SuperCall, private name or static block; no negative case): `language/statements/class` 264 and `language/expressions/class` 251 (the generated `*-field-*`, `*-computed-names*`, `*-literal-names*`, `fields-*`, `static-field-*` and `cpn-class-*-fields-*` cases). 2 of them joined this list in revision 1 (cases of the order of the instance elements whose parts this story's closure already holds); 2 left it in revision 2 for `story:private-fields-and-private-in` (and are on `story:class-static-blocks`' list since revision 3) (`language/{statements,expressions}/class/elements/super-access-from-arrow-func-on-field.js`: `super.x` in an arrow in a field initializer, whose refusal `story:derived-classes-and-super-call`, built beside this story, lifts; `docs/plan/slice-08/scan.awk`, the part `A`). Their 1,029 runs fail on the plan's base run (the full run of 2026-11-26 on `149e12f1`, equal to `conformance/test262/expected.txt` on `62c7cf59`; slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`), all on `class is not supported before slice 8`. A case of the list that ends on a part outside this story goes to `docs/plan/slice-08/close-later.tsv` by the coordinator's commit, as `story:new-target-meta-property` states.

Design it builds on, by name: `docs/design/engine.md` section 3, "Classes", items 2 (step 1, the hidden bindings of computed keys; step 3; step 4) and 4 (form (b), one function per run; `DefineField` and `DefineComputedField`; the static runs). No sentence of the design is missing.

## What later stories reuse

Pass 2 of the instance initializer and the static run functions (`story:private-fields-and-private-in` adds private fields to both through the private add; `story:class-static-blocks` places its blocks' calls in the same step-4 sequence); the hidden bindings of computed field keys; the `FieldInitializer` template that `story:eval-in-class-field-initializers` reads as [[ClassFieldInitializerName]].

## Domain relations

- A class evaluation → its fields (in source order, each with one key fixed at the class's definition); a field initializer function → the home object of its placement (the prototype or F) — inferable (inferred from ECMA-262 `#sec-classfielddefinition-record-specification-type` and `#sec-definefield`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- Syntax: the hold `class field` is lifted: the `hold` call in `field` (`parser/class.rs`) and the line `("class field", 8),` of `HOLDS` (`parser/mod.rs`, the list `story:class-syntax-and-early-errors` adds) are deleted. No fixture moves: `crates/syntax/tests/sources/held/class-field.js` (header `held "class", "class field"`) parses in both modes from this story on, which `sources::held_sources_answer_their_first_construct_still_held` reads from `HOLDS`, and `compile::held_sources_compile_once_nothing_in_them_is_held` compiles it.
- The stack bound at the depth of the parser's bound (`fits_the_stack_bound`, `crates/syntax/tests/sources.rs`). This story lifts the last hold of the nesting kind `class { x = class { x = ... } }` (it holds `class`, lifted by `story:class-definition-evaluation`, then `class field`) of `story:class-element-syntax-and-early-errors` (`HOLDS` then has no line that kind's source holds), so from this story on that source parses to the end at the bound's depth, `admitted` no longer answers it by the held refusal, and `analyse_parsed` (`crates/syntax/src/lib.rs`) runs the scope analysis (`scope::analyse`, `crates/syntax/src/scope.rs`) on it at that depth for the first time; `fits_the_stack_bound` now runs the analysis for this kind through every entry. This story owns what follows: a failure of that test for this kind (a stack overflow, a panic or a wrong answer of the analysis) is fixed here, in `scope.rs`: the analysis refuses at the depth the parser's bound gives, with the nesting error, as the parser does, and has no recursion without a bound (its walk keeps its pending work on the heap today, `scope.rs`, the module documentation: "The walk does not recurse"); the report names the kind and the depth reached. No other story fixes it, and no test is relaxed for it.
- Compiler (`crates/compiler/src/class.rs`): `compile_field` (the `CompileError::Tree` of `story:class-definition-evaluation` replaced): at step 3 a computed key is evaluated, converted by `ToPropertyKey` and stored in its hidden binding; an instance field joins pass 2 of the initializer, `initializer_fields` (the value or `undefined`, `SetFunctionName` for an anonymous function or class definition, then `DefineField(k)` for a literal key or `DefineComputedField` with the key read from its binding); a run of consecutive static fields becomes one `FieldInitializer` function whose code defines the run's fields on `this` the same way; `static_field_run` answers that function, and the step-4 sequence of `story:class-definition-evaluation` calls it where the run stands in source order. `crates/compiler/src/resolve.rs`, `field_key_bindings`: one hidden binding per computed field key in the class scope, a record slot because the initializers reach it. No instruction is added. A compiler fixture pair `crates/compiler/tests/sources/class-fields.{js,disasm}` (new).
- Tests, each in both collection modes, in `crates/swell/tests/class_fields.rs` (new): `fields_with_and_without_initializers_in_order`; `computed_keys_are_evaluated_once_at_definition_with_their_side_effects`; `this_and_new_target_in_an_initializer`; `super_property_in_an_initializer`; `an_anonymous_function_or_class_takes_the_field_name`; `a_field_over_a_prototype_getter_is_an_own_data_property`; `define_field_on_a_frozen_constructor_throws_a_type_error` (`static a = Object.freeze(this); static b = 1;`); `static_fields_and_their_this`; `static_fields_run_after_every_method_is_defined`; `an_initializer_loop_ends_at_the_step_budget` (`Termination::StepBudget`); `an_initializer_loop_ends_at_an_interrupt` (`Termination::Interrupted`); `constructions_of_a_class_with_100_fields_are_charged_and_freed` (10,000 constructions, 1,000 under `collect-every-safe-point`; `Termination::MemoryBudget` under a small budget; with enough, `heap_bytes()` after a `collect()` within the reading before); `new_in_an_initializer_stops_at_frame_depth` (a `RangeError`): that is 13 tests under `class_fields::`, none on the base (the file is new). `crates/swell/tests/main.rs` generated again.
- Sizes: no layout changes (no field of a record, a frame or a function is added; the record tests pass as they are).
- Bounds, as `AGENTS.md` and the `AGENTS.md` of the folder that holds the repository ("Tests stay fast", "A job in the pool ends within 10 minutes", "No Python for permanent tools") state them: a test is terminated at 60 s and counts as failed; the gate fails when the wall time of its test steps passes 120 s; a test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` (at the reduced count named above) and under 2.5 at `SWELL_FULL_COUNTS=1`, measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` (user plus system); the unit times each test before it adds it and names each one over 1 CPU second with its time. A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures, probes or waits runs without the prefix. No `unsafe`, no third-party crate, no panic, `unwrap`, `expect` or unchecked indexing; no Python in any tool, test or command; intermediate files in the scratch directory or `.out/`.
- Size: about 550 lines (compiler 200, resolve 60, tests 290): one session.

## Requirements

- No line of `conformance/host/requirements.txt` moves here: `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `1` on `61f2a8e2` and after this story; only `story:host-ensure-can-add-private-element` moves those lines.
- The run loop is not touched (no instruction is added); `STACK_BYTES_PER_REENTRY` is unchanged, and the stack tests pass on seven eighths of `min_stack_bytes()`.

Writes: `crates/syntax/src/parser/{class,mod}.rs` (the hold call, its line of `HOLDS`), `crates/syntax/src/scope.rs` (only for a failure of `fits_the_stack_bound` at its nesting kind, as Change says); `crates/compiler/src/class.rs` (`compile_field`, `initializer_fields`, and `static_field_run`, the function that the arm of a static-field run of `static_calls` calls; `static_calls` itself is not written), `crates/compiler/src/resolve.rs` (`field_key_bindings`), `crates/compiler/tests/sources/class-fields.{js,disasm}` (new); `crates/swell/tests/class_fields.rs` (new), `crates/swell/tests/main.rs` (generated). A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/{lexer,early}.rs`, `crates/compiler/src/{bytecode,disasm,expression}.rs`, `crates/swell/src/` (the engine runs the existing instructions), `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared with the stories of order 3, built at the same time with no edge between them, each at its own named site: `parser/class.rs` (here the `hold` call in `field`; `story:derived-classes-and-super-call` the `hold("class heritage", ..)` call; `story:private-names-and-private-methods` the call in `private_method`); `HOLDS` in `parser/mod.rs` (each deletes its own line: here `("class field", 8),`, there `("class heritage", 8),` and `("private method", 8),`; the lines are adjacent, so the second to merge gets a conflict there, fixed by "keep both deletions"); `compiler/src/class.rs` (here `compile_field` and `initializer_fields`; there `class_heritage`, `default_derived_constructor`, `compile_private_method`, `initializer_private_methods`); `compiler/src/resolve.rs` (here `field_key_bindings`; there `function`, the binding `this`, and `private_record`); `tests/main.rs` (generated again on the join). Ordered after this story by edges: `story:class-static-blocks` (it fills the other arm of the same step-4 sequence, `compile_static_block`), `story:eval-in-class-field-initializers`, `story:private-fields-and-private-in` (the private add in `initializer_fields` and in the static runs).

## Acceptance

The tests named in Change pass in both collection modes, run by the binaries of this change and not as a whole step. `slot cargo nextest run --locked --no-tests=fail -p swell --test main -E 'test(/^class_fields::/)'` runs 13 tests and passes (on the base the filter matches none and the command fails); then `slot cargo nextest run --locked -p swell --test main -E 'test(/^class_fields::/) | test(/^classes::/) | test(/^builtins_object_integrity::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/) | test(/^collect::/) | test(/^termination::/) | test(/^interrupt::/)'` and the same with `--features collect-every-safe-point`; `slot cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; `slot cargo nextest run --locked -p swell-syntax --test main`; `slot cargo nextest run --locked -p swell-compiler --test main`; `slot cargo nextest run --locked -p swell-test262 --test main`; each exits 0, in this story's tree and after `story:derived-classes-and-super-call` or `story:private-names-and-private-methods` is merged first: the wave branch is merged into the unit, the conflicts this story names are fixed as it names them (the adjacent lines of `HOLDS`: keep both deletions; `tests/main.rs`: generated again), and the same commands run again and exit 0. `ls crates/compiler/tests/sources/class-fields.* | wc -l` prints `2` (0 on the base). `slot task clippy`, `slot task forbid-unsafe-check` and `sh taskfiles/test-main.sh check` exit 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing (`BASE` the join the unit starts from); `git diff --quiet "$BASE" -- crates/swell/src` exits 0. The text: `grep -rn '"class field", 8' crates | wc -l` prints `0` (2 on its base: the hold call and the line of `HOLDS`; the header of `held/class-field.js` does not match). `ls crates/syntax/tests/sources/held | wc -l` prints the same number as on its base (no fixture moves). `git diff --quiet "$BASE" -- conformance/host/requirements.txt` exits 0 and `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `1`. Time: every test this story adds stays within the bounds of Change, as the unit measured them; the report lists each measured time and the sum, and a time over its bound fails this acceptance.

test262. The full run is not the unit's. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-public-fields.cases.txt" > .out/story-public-fields.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-objects.cases.txt` (the 523 passing cases of `Object.freeze`, `seal`, `preventExtensions`, the `is*` tests, the key and entry functions, `assign`, `getOwnPropertyDescriptors` and `JSON.stringify`), `cat .out/story-public-fields.cases.txt docs/plan/slice-08/guard-objects.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 1,038 cases, 2,069 runs over the plan's base run), then, in the background, `slot --timeout 30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt` (one pool job, as `story:new-target-meta-property` states), and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-public-fields.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1, prints 1,029 lines `failing` and `passed 523 bound 523 B 523 F 515 R 515`; it prints `missing` for a list case the part lacks and exits 1 on an empty list); (b) `awk -F'\t' '$6 ~ /class (field )?is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0` (1,029 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

Private fields (`story:private-fields-and-private-in`); fields of a derived class run after `super()` and their order with private methods (the cases where the parts meet, on the lists of `story:class-static-blocks` and `story:private-fields-and-private-in`); eval in an initializer (`story:eval-in-class-field-initializers`); static blocks (`story:class-static-blocks`); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).

## As delivered

The unit `fields` of wave big-2 delivered the story on `impl/fields` (one correction after the
adversary's pass), merged into `wave/big-2`. Where the delivery differs from the text above:

- A direct eval in a field initializer is refused. The story leaves eval in an initializer to
  `story:eval-in-class-field-initializers`; with the hold on fields lifted, eval ran there
  without PerformEval's rule, and `eval("arguments")` threw a ReferenceError
  (`review-result:adversary-fields-pass-1`). Until that story, a direct eval whose function is
  a field initializer, an arrow in one too, throws the SyntaxError `eval in a class field
  initializer is not supported before slice 8` (`crates/swell/src/dynamic_code.rs`,
  `eval_context`); an indirect eval and a direct eval in a method run. The test
  `class_fields::a_direct_eval_in_an_initializer_is_refused` holds it; that story deletes the
  refusal and the test.
- The tests are 17, not 13. Added after the adversary's pass:
  `a_direct_eval_in_an_initializer_is_refused`,
  `a_computed_key_made_at_run_time_lives_until_the_constructions` (red in the collect build
  when the collector skips the hidden key bindings),
  `a_field_error_is_reported_at_the_field_and_at_its_key` (line 3, column 3 for DefineField;
  line 3, column 4 for the key's ToPropertyKey) and `a_literal_proto_field_is_an_own_property`.
- The counts of `constructions_of_a_class_with_100_fields_are_charged_and_freed`. The story
  names 10,000 constructions, and 1,000 with a collection at every safe point. The test runs,
  per round, 500 in the gate and 3,000 with `SWELL_FULL_COUNTS=1` in the default build (0.44
  and 2.05 CPU s alone; 10,000 took 6.48, over the 2.5 s of a test there), and 4 and 8 in the
  collect build (1.44 and 2.10 CPU s; one construction costs about 84 ms there). The collect
  build skips the memory-budget part and holds 4 instances, not 100. A byte charged and never
  credited per DefineField, and a DefineField that defines nothing, are red at the gate counts
  in both builds.
- The hidden binding of a computed field key is named `#key <n>`, `<n>` the number of the
  key's expression in the tree: a name with a space, which no identifier and no private name
  has.
- Files changed outside the Writes line: `crates/compiler/src/function.rs` (the call of
  `instance_initializer`, and `expression_value` made `pub(crate)`),
  `crates/syntax/tests/sources.rs` (three tests that named the hold),
  `crates/compiler/src/resolve.rs` (`class_scope`, not only `field_key_bindings`) and
  `crates/swell/src/dynamic_code.rs` (the refusal). `crates/syntax/src/scope.rs`, an inferred
  scope line, needed no change.
- The part run of the story's list: 515 cases and the guard, 2,069 runs, all passed
  (`passed 1038 bound 1037 B 522 F 515 R 0`).

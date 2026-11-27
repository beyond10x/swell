---
format: aep.planning-md/3
id: story:private-names-in-eval-code
kind: story
status: implemented
title: 'Private names in direct eval code: the private environment of the caller'
summary: 107 cases (203 runs).
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:private-fields-and-private-in
- depends_on: story:eval-in-class-field-initializers
scope:
- confidence: cited
  path: crates/compiler/src/lib.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/swell/src/dynamic_code.rs
- confidence: cited
  path: crates/swell/src/env/mod.rs
- confidence: inferred
  path: crates/swell/tests/eval_private_names.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/lib.rs
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T08:23:48Z", actor: "human:operator", revision: 9, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-26T08:23:49Z", actor: "human:operator", revision: 10, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-27T21:14:36Z", actor: "human:operator", revision: 12, decided_on: {"recorded":{"test_result":1,"review_outcome":3,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story is private names in direct eval code: PerformEval's private environment (`#sec-performeval`: "Let privateIdentifiers be a new empty List. Let pointer be privateEnv. Repeat, while pointer is not null ... If AllPrivateIdentifiersValid of body with argument privateIdentifiers is false, throw a SyntaxError exception"), and the evaluation of `#x`, `o.#x`, `#x in o` in eval code against the Private Names of the classes around the eval call (the eval's running execution context takes the caller's PrivateEnvironment, `#sec-performeval`, step 16), in methods, constructors, field initializers, static blocks and the arrows in them, each evaluation of a class seeing its own names. Indirect eval and `Function` code have no private environment (a private name in them is the SyntaxError of AllPrivateIdentifiersValid with no names, which `story:class-element-syntax-and-early-errors` already applies).

Edges: `story:private-fields-and-private-in` (every private element kind, `PrivateIn` and `AddPrivateField`; with it, through its edges, `story:private-names-and-private-methods`'s private record, `GetPrivate` and `SetPrivate`) and `story:eval-in-class-field-initializers` (eval in an initializer, and the last writer of the eval context before this story). Order 6, built beside `story:host-ensure-can-add-private-element`, which shares only the generated `tests/main.rs` with it (see "Shared files").

The list is `docs/plan/slice-08/story-eval-private.cases.txt`: the 107 cases of the key `eval-private` of `docs/plan/slice-08/scan.tsv` (a class with a private name and a call of `eval`; no negative case). By directory: `language/statements/class/elements` 59, `language/expressions/class/elements` 41 (`privatename-*-eval-*`, `private-*-direct-eval*`, `*-indirect-eval-*`, `private-field-as-*-eval*`), `language/expressions/class/private-*-multiple-evaluations-of-class-{direct-eval,eval}.js` 7. Their 203 runs fail on the plan's base run (the full run of 2026-11-26 on `149e12f1`, equal to `conformance/test262/expected.txt` on `62c7cf59`; slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`): 196 on `class is not supported before slice 8` and 7 as counted refusals. A case of the list that ends on a part outside this story goes to `docs/plan/slice-08/close-later.tsv` by the coordinator's commit, as `story:new-target-meta-property` states.

Design it builds on, by name: `docs/design/engine.md`, section 3, "Private names", item 4 (resolution in eval code: PerformEval walks the caller's LexicalEnvironment chain once, a step per record, and lists each slot whose name starts with `#` as its text, its hops and its slot, an inner name hiding an outer one of the same text; the parse gets the texts, the compile entry of eval code the positions; the list holds no handle and is dropped after the compile; rejected: a lookup by text at run time, and the private environment of the function an arrow's `this` comes from), item 3 (the private record is a record of the environment chain) and item 5 (the instructions eval code uses, the same as other code). No sentence of the design is missing.

## What later stories reuse

Nothing of slice 8. The walk is read by any later slice that adds a kind of code with a class context (none is planned).

## Domain relations

- Direct eval code → the private names of its caller's environment chain (texts and positions, one list per eval) — inferable (inferred from ECMA-262 `#sec-performeval` and `#sec-privateenvironment-records`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- Syntax: the eval context of `parse_eval_with` (`crates/syntax/src/lib.rs`, the struct `story:new-target-meta-property` introduced) gains a field `private_names`, the texts of the caller's private names, after the field `in_class_field_initializer`; `all_private_identifiers_valid` of `story:class-element-syntax-and-early-errors` (`crates/syntax/src/early.rs`) takes them as the names around eval code in place of none; the documentation of `parse_eval` names AllPrivateIdentifiersValid beside the other rules.
- Compiler: `compile_eval_with` (`crates/compiler/src/lib.rs`, which takes the flag of `story:derived-classes-and-super-call`) takes the positions (text, hops, slot), and the resolve of eval code (`crates/compiler/src/resolve.rs`, beside `eval_bindings`) resolves a private name against them with the hops counted as the eval code's own records plus the caller's hops; the instructions are `GetPrivate`, `SetPrivate`, `PrivateIn` and the call of a private method, as other code compiles them. No instruction is added (no `bytecode.rs`, no `disasm.rs`, no arm in `interp/dispatch.rs`). A compiler unit test in the test module of `lib.rs`, `tests::eval_private_names_count_the_callers_hops`, holds the hops (resolve.rs has no test module).
- Engine: `crates/swell/src/env/mod.rs` gets `Agent::eval_private_names`, after `Agent::outer_environment`: the walk of item 4 from the caller's LexicalEnvironment, a step per record charged through `Agent::charge_steps`, each slot whose name starts with `#` listed once (the innermost of one text; the private record's slots carry their names exactly when a direct `eval` stands inside the class body, by the sentence of `story:private-names-and-private-methods` on `mark_named` and the rule of `env/mod.rs` for a record a lookup by name may reach, and this walk starts only from such an `eval`), the list charged to the memory budget while it lives and credited after the compile. `crates/swell/src/dynamic_code.rs`: `eval_code_of` calls it for a direct eval only and passes the texts to `parse_eval_code` and the positions to `compile_eval_with`; an indirect eval passes none.
- Tests, each in both collection modes, in `crates/swell/tests/eval_private_names.rs` (new): `eval_reads_a_private_field_in_a_method`; `eval_reads_a_private_field_in_a_constructor`; `eval_reads_a_private_field_in_a_field_initializer`; `eval_reads_a_private_field_in_a_static_block`; `an_arrow_in_a_method_passes_the_names_to_its_eval`; `an_arrow_in_an_inner_computed_key_sees_the_inner_names` (the case the rejected design missed); `eval_of_private_in`; `eval_writes_a_private_field_and_calls_a_private_method_and_accessor`; `an_inner_class_eval_reads_an_outer_classes_name`; `an_inner_name_hides_an_outer_name_of_the_same_text`; `two_evaluations_of_one_class_each_read_their_own_name_through_eval`; `indirect_eval_and_function_code_have_no_private_names` (SyntaxError); `an_undeclared_private_name_in_eval_is_a_syntax_error`; `a_collection_between_the_walk_and_the_run_keeps_the_names`; `nested_eval_with_private_names_is_bounded` (the `RangeError` of `eval_stack` or `frame_depth`, no panic; at most 1,000 levels, 100 under `collect-every-safe-point`); `an_eval_loop_reading_a_private_field_ends_at_the_step_budget` (10,000 evals, 1,000 under `collect-every-safe-point`, `steps_per_entry` small, `Termination::StepBudget`; `heap_bytes()` back to its value after a collection, so the list leaks nothing): that is 16 tests under `eval_private_names::`, none on the base (the file is new). `crates/swell/tests/main.rs` generated again.
- Sizes: no layout changes (the list is a local of the eval's start, not a field of a frame, a function or a record).
- Bounds, as `AGENTS.md` and the `AGENTS.md` of the folder that holds the repository ("Tests stay fast", "A job in the pool ends within 10 minutes", "No Python for permanent tools") state them: a test is terminated at 60 s and counts as failed; the gate fails when the wall time of its test steps passes 120 s; a test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` (at the reduced counts named above) and under 2.5 at `SWELL_FULL_COUNTS=1`, measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` (user plus system); the unit times each test before it adds it and names each one over 1 CPU second with its time. A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures, probes or waits runs without the prefix. No `unsafe`, no third-party crate, no panic, `unwrap`, `expect` or unchecked indexing; no Python in any tool, test or command; intermediate files in the scratch directory or `.out/`.
- Size: about 450 lines (syntax 40, compiler 60, engine 100, tests 250): one session.

## Requirements

- No line of `conformance/host/requirements.txt` moves here: `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `1` on `61f2a8e2` and on this story's base; after this story it prints what `story:host-ensure-can-add-private-element` leaves (`0` once that story is merged, `1` before), and this story changes no line of the file. The eval context and `compile_eval_with` gain a field and a parameter: changes of public items of `swell-syntax` and `swell-compiler`. The unit names them in one line of text it hands to the coordinator for `## [Unreleased]` of `CHANGELOG.md` (the wave's coordinator writes that file); no acceptance command reads it.
- The run loop is not touched; the walk does its work in a function of its own outside `Agent::execute`; `STACK_BYTES_PER_REENTRY` is unchanged, and the stack tests pass on seven eighths of `min_stack_bytes()`.

Writes: `crates/syntax/src/{lib,early}.rs`, `crates/compiler/src/{lib,resolve}.rs`, `crates/swell/src/env/mod.rs` (`Agent::eval_private_names` only), `crates/swell/src/dynamic_code.rs`, `crates/swell/tests/eval_private_names.rs` (new), `crates/swell/tests/main.rs` (generated). A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `CHANGELOG.md` (the coordinator's), `crates/syntax/src/parser/`, `crates/compiler/src/{bytecode,disasm}.rs`, `crates/swell/src/interp/`, `crates/swell/src/object/`, `crates/swell/src/host/`, `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared files: with `story:host-ensure-can-add-private-element`, built at the same time with no edge between them: `crates/swell/tests/main.rs` (generated again on the join); no other file. Written before it by stories it depends on, at other named sites: the eval context in `lib.rs` (`story:eval-in-class-field-initializers` last, its rule; here the field after `in_class_field_initializer`), `early.rs` (`story:class-element-syntax-and-early-errors`'s `all_private_identifiers_valid`, here only its eval input; `story:eval-in-class-field-initializers`' `contains_arguments`, not touched here), `compiler/src/{lib,resolve}.rs` (`story:derived-classes-and-super-call` the flag of `compile_eval_with`; here the positions), `env/mod.rs` (`story:private-names-and-private-methods` the trace of `SlotValue::PrivateName`; here a new function after `outer_environment`), `dynamic_code.rs` (`story:new-target-meta-property`, `story:class-definition-evaluation` and `story:eval-in-class-field-initializers` the flags in `eval_code_of`, `story:derived-classes-and-super-call` the `constructed` of the frame literal in `eval_frame`; here the call of the walk in the same function, after the flags).

## Acceptance

The tests named in Change pass in both collection modes, run by the binaries of this change and not as a whole step. `slot cargo nextest run --locked --no-tests=fail -p swell --test main -E 'test(/^eval_private_names::/)'` runs 16 tests and passes (on the base the filter matches none and the command fails); `slot cargo nextest run --locked --no-tests=fail -p swell-compiler --lib -E 'test(/^tests::eval_private_names_count_the_callers_hops$/)'` runs 1 test (none on the base: the command fails); then `slot cargo nextest run --locked -p swell --test main -E 'test(/^eval_private_names::/) | test(/^private_fields::/) | test(/^private_methods::/) | test(/^class_field_eval::/) | test(/^class_static_blocks::/) | test(/^eval::/) | test(/^eval_stack::/) | test(/^dynamic_function::/) | test(/^arrow_functions::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^collect::/) | test(/^termination::/)'` and the same with `--features collect-every-safe-point`; `slot cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; `slot cargo nextest run --locked -p swell-syntax --test main`; `slot cargo nextest run --locked -p swell-compiler --test main`; `slot cargo nextest run --locked -p swell-test262 --test main`; each exits 0, in this story's tree and after `story:host-ensure-can-add-private-element` is merged first: the wave branch is merged into the unit, the one conflict this story names is fixed as it names it (`tests/main.rs`: generated again; the two stories share no other file), and the same commands run again and exit 0. `slot task clippy`, `slot task forbid-unsafe-check` and `sh taskfiles/test-main.sh check` exit 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing (`BASE` the join the unit starts from). The text: `grep -c 'AllPrivateIdentifiersValid' crates/syntax/src/lib.rs` prints at least `1` (0 on `61f2a8e2`); `grep -c 'eval_private_names' crates/swell/src/dynamic_code.rs` prints at least `1` (0 on `61f2a8e2`). `git diff --quiet "$BASE" -- conformance/host/requirements.txt` exits 0. Time: every test this story adds stays within the bounds of Change, as the unit measured them; the report lists each measured time and the sum, and a time over its bound fails this acceptance.

test262. The full run is not the unit's. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-eval-private.cases.txt" > .out/story-eval-private.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-functions.cases.txt` (2,546 passing cases of functions, arrows, methods, calls, `new`, eval code and `Function`), `cat .out/story-eval-private.cases.txt docs/plan/slice-08/guard-functions.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 2,653 cases, 4,475 runs over the plan's base run), then, in the background, `slot --timeout 30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt` (one pool job), and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-eval-private.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1, prints 203 lines `failing` and `passed 2546 bound 2546 B 2546 F 107 R 107`; it prints `missing` for a list case the part lacks and exits 1 on an empty list); (b) `awk -F'\t' '$6 ~ /class is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0` (196 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

The hook of HOST-119 (`story:host-ensure-can-add-private-element`); private names in `Function` code (none, per `#sec-createdynamicfunction`); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).

## As delivered

The unit `evalprivate` of wave big-2 delivered the story on `impl/evalprivate` (two
corrections after the adversary's two passes), merged into `wave/big-2`. Where the delivery
differs from the text above:

- A direct eval walks the caller's records only when its String holds a `#` code unit
  (`Agent::source_holds_hash`). The first build walked for every direct eval: a step per
  record and per named slot, 992 steps in a non-strict function of 1,000 `var`s with no
  class around it. The adversary's first pass found it; the coordinator decided the scan.
  A String without `#` costs what it cost before the story; one with a `#` anywhere, also
  in a string literal or a comment, walks.
- The clause is `#sec-evaldeclarationinstantiation`, steps 4 to 7 of the pinned draft, not
  PerformEval: the story's text and the first build cited PerformEval. The engine checks
  AllPrivateIdentifiersValid at the parse, before the var checks of step 3 that the run
  makes; no eval text tells the orders apart but by the message.
- No function `all_private_identifiers_valid` exists in `early.rs`: the rule lives in
  `Checker::private_reference` and the checker's private map, which `early::check` fills
  from `EvalContext::private_names` before it walks the body.
- The public surface: `swell_syntax::EvalContext` gains a lifetime and the field
  `private_names`; `parse_eval` and `parse_eval_with` take `impl Into<EvalContext<'a>>`;
  `swell_compiler::compile_eval_with` gains `private_names: &[(&str, u32, u32)]`. No
  instruction was added. `SlotNames::text` is factored from `slot_name`, and
  `chain_names_slot` is removed (`env/mod.rs`).
- The shadow rule stands at three sites (the walk, the resolver's map, the parse's map),
  the first two each with a test of its own.
- 35 tests under `eval_private_names::`, one library test of the walk
  (`dynamic_code::tests::the_walk_lists_a_text_once_at_its_innermost_record`) and one
  compiler test. Six of the tests and the two sweeps over the walk's records came with the
  adversary's second pass. Five sweeps of the step budget cover every budget from the
  walk's first record to the answer, each named after what stops in its range (block
  records, the class record, the compares, the parse's charge, the run); they read their
  bounds from the run and step over the budgets of one charge. Three planted leaks are each
  red in the sweep of that name. The sweeps return at once in the collect build.
- Measured steps of one direct eval of `'#'` less an indirect one at the same place: 0
  names 1 / 8 / 64 at 1 / 8 / 64 blocks; 1 name 2 / 9 / 65; 100 names 94 / 101 / 157. A
  text without `#`: 3 steps less than an indirect one, as before the story.
- The clause numbers of PerformEval in the engine's comments and tests (22 sites, most of
  them older than the story) are corrected from the pinned text: the record of strict eval
  code is made in step 17 and becomes the variableEnv in step 19.
- A library test of the syntax crate that asserted a class with private fields is held
  back, red since `story:private-fields-and-private-in` lifted the last hold, states what is
  true now; it is this unit's commit because this unit found it.
- test262: the story's list of 2,653 cases passes on the first build (`passed 2653 bound
  2653 B 2641 F 12 R 0`); the 125 cases that hold an eval and a private name pass in all 225
  runs on correction 1. No test262 run on the merged head: the conformance close runs all.

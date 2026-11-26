---
format: aep.planning-md/3
id: story:class-static-blocks
kind: story
status: draft
title: Static blocks, run in the order of the static elements
summary: 33 cases (66 runs); lifts class static block.
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:class-definition-evaluation
- depends_on: story:class-element-syntax-and-early-errors
- depends_on: story:public-class-fields
scope:
- confidence: inferred
  path: crates/compiler/src/class.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: inferred
  path: crates/compiler/tests/sources/class-static-block.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/class-static-block.js
- confidence: inferred
  path: crates/swell/tests/class_static_blocks.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: inferred
  path: crates/syntax/src/parser/class.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: inferred
  path: crates/syntax/tests/sources/refused/class-static-block.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/class-static-block.js
revision: 5
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story runs static blocks as compiled calls (item 4 of "Classes"): each static block is its own `StaticBlock` function (the body function of ClassStaticBlockDefinitionEvaluation, without the field flag, with a `var` scope of its own, an arguments object only when a direct `eval` in it can observe one), made as a method with home object F and called with `this` F at its place in the step-4 sequence of `story:class-definition-evaluation` (`<F>; MakeMethod(f); Call(0); Pop`), between the calls of the static field runs of `story:public-class-fields` in source order; `new.target` undefined, SuperProperty through F, `await` reserved; an abrupt completion ends the class's definition, the class's own name left uninitialized. No list of static elements and no walk exist (decision 7 of slice 8): `static-init-sequence.js` holds by construction. It lifts the hold `class static block is not supported before slice 8` of `story:class-element-syntax-and-early-errors`.

Edges: `story:class-definition-evaluation` (`compile_static_block`, `MakeMethod`, the step-4 sequence), `story:class-element-syntax-and-early-errors` (the block's tree, its function and context) and `story:public-class-fields`, which is first of the two that share the order of the emitted static calls: that story places the static runs' calls in the sequence and this story inserts the blocks' calls between them, and two cases of this list (`language/statements/class/static-init-sequence.js`: field, block, field, block; `static-init-abrupt.js`) need both. Order 4, built beside `story:eval-in-class-field-initializers`, which writes none of its files.

The list is `docs/plan/slice-08/story-static-blocks.cases.txt`: the 35 cases of the key `static-blocks` of `docs/plan/slice-08/scan.tsv` (a class with a static block and no heritage, private name or SuperCall; fields allowed since this revision, so the two cases named above moved here from the private-fields list; no negative case): `language/statements/class` 15 (`static-init-*`: the block's statements, `this`, scope, the order with static fields, an abrupt block), the `static-init-await-binding-valid.js` cases of `language/statements/{variable,try,let,function,const}` and `language/expressions/{object,generators,function}` 19 (`await` as a binding name inside a function in a static block), and `language/expressions/class` 1. Their 70 runs fail on the plan's base run (the full run of 2026-11-26 on `149e12f1`, equal to `conformance/test262/expected.txt` on `62c7cf59`; slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`), all on `class is not supported before slice 8`. Static blocks that read private names are on `story:private-fields-and-private-in`'s list. A case of the list that ends on a part outside this story goes to `docs/plan/slice-08/close-later.tsv` by the coordinator's commit, as `story:new-target-meta-property` states.

Design it builds on, by name: `docs/design/engine.md` section 3, "Classes", items 2 (step 4) and 4 (the static elements; a static block's function and its arguments object; form (c) rejected). No sentence of the design is missing.

## What later stories reuse

The static block's function and its call in the step-4 sequence (`story:private-fields-and-private-in` runs static blocks that read private names; `story:eval-in-class-field-initializers` reads that a `StaticBlock` template has no field flag).

## Domain relations

- A static block → the class constructor that is its home object and its `this` (one) — inferable (inferred from ECMA-262 `#sec-runtime-semantics-classstaticblockdefinitionevaluation`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- Syntax: the hold `class static block` is lifted (`parser/class.rs`, the `hold` call in `static_block`) and its line deleted from the documentation of `held` (`parser/mod.rs`); `crates/syntax/tests/sources/refused/class-static-block.js` moves to `valid/`.
- Compiler: `crates/compiler/src/class.rs`, `compile_static_block` (the `CompileError::Tree` of `story:class-definition-evaluation` replaced): the block's `StaticBlock` function compiled as a function body, its call placed in the step-4 sequence where the block stands in source order, among the runs' calls of `story:public-class-fields`. `crates/compiler/src/resolve.rs`, a function `static_block_scope` called from `function` for the kind `StaticBlock`: the block's `var` scope, its lexical declarations and functions, and the arguments object made only when the block's code contains a direct `eval` ("Names", item 4). No instruction is added. A compiler fixture pair `crates/compiler/tests/sources/class-static-block.{js,disasm}` (new).
- Tests, each in both collection modes, in `crates/swell/tests/class_static_blocks.rs` (new): `a_block_runs_with_this_the_constructor`; `a_blocks_var_is_not_visible_outside`; `super_property_in_a_block`; `new_target_in_a_block_is_undefined`; `blocks_and_static_fields_run_in_source_order`; `a_throw_from_a_block_leaves_the_class_binding_uninitialized`; `await_as_a_binding_name_in_a_nested_function`; `a_direct_eval_of_arguments_in_a_block_sees_its_own_arguments_object`; `a_block_loop_ends_at_the_step_budget` (`Termination::StepBudget`); `a_block_loop_ends_at_an_interrupt` (`Termination::Interrupted`); `a_recursion_of_classes_with_blocks_stops_at_frame_depth` (a `RangeError`; the depth of the recursion 1,000, 100 under `collect-every-safe-point`): that is 11 tests under `class_static_blocks::`, none on the base (the file is new). `crates/swell/tests/main.rs` generated again.
- Sizes: no layout changes (the record tests pass as they are).
- Bounds, as `AGENTS.md` and the `AGENTS.md` of the folder that holds the repository ("Tests stay fast", "A job in the pool ends within 10 minutes", "No Python for permanent tools") state them: a test is terminated at 60 s and counts as failed; the gate fails when the wall time of its test steps passes 120 s; a test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` (at the reduced count named above) and under 2.5 at `SWELL_FULL_COUNTS=1`, measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` (user plus system); the unit times each test before it adds it and names each one over 1 CPU second with its time. A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures, probes or waits runs without the prefix. No `unsafe`, no third-party crate, no panic, `unwrap`, `expect` or unchecked indexing; no Python in any tool, test or command; intermediate files in the scratch directory or `.out/`.
- Size: about 350 lines (compiler 110, resolve 50, tests 190): one session, the smallest of the slice; kept apart because it lifts its own hold and shares no function with the field story.

## Requirements

- No line of `conformance/host/requirements.txt` moves here: `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `1` on `61f2a8e2` and after this story; only `story:host-ensure-can-add-private-element` moves those lines.
- The run loop is not touched (no instruction is added); the block runs as a frame of the loop, never through a re-entry; `STACK_BYTES_PER_REENTRY` is unchanged, and the stack tests pass on seven eighths of `min_stack_bytes()`.

Writes: `crates/syntax/src/parser/{class,mod}.rs` (the hold call, its line of `held`), `crates/syntax/tests/sources/{refused,valid}/class-static-block.js` (moved); `crates/compiler/src/{class,resolve}.rs`, `crates/compiler/tests/sources/class-static-block.{js,disasm}` (new); `crates/swell/tests/class_static_blocks.rs` (new), `crates/swell/tests/main.rs` (generated). A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/{lexer,early}.rs`, `crates/compiler/src/{bytecode,disasm,expression}.rs`, `crates/swell/src/`, `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared files: with `story:public-class-fields`, ordered before this story by an edge (`compiler/src/class.rs`: there `compile_field` and `initializer_fields`, here `compile_static_block`; `compiler/src/resolve.rs`: there `field_key_bindings`, here `static_block_scope`); with `story:derived-classes-and-super-call` and `story:private-names-and-private-methods` (order 3; no edge to this story, which the orders put after them: `parser/class.rs`, each its own `hold` call; the documentation of `held`, each its own line, adjacent: a conflict there keeps every deletion; `resolve.rs`: there `function`'s binding `this` and `private_record`, here `static_block_scope` and its one call line in `function`, after the lines of `story:derived-classes-and-super-call`); `story:eval-in-class-field-initializers`, beside it in order 4, writes none of these files; `tests/main.rs` generated again on the join.

## Acceptance

The tests named in Change pass in both collection modes, run by the binaries of this change and not as a whole step. `slot cargo nextest run --locked --no-tests=fail -p swell --test main -E 'test(/^class_static_blocks::/)'` runs 11 tests and passes (on the base the filter matches none and the command fails); then `slot cargo nextest run --locked -p swell --test main -E 'test(/^class_static_blocks::/) | test(/^class_fields::/) | test(/^classes::/) | test(/^eval::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/) | test(/^collect::/) | test(/^termination::/) | test(/^interrupt::/)'` and the same with `--features collect-every-safe-point`; `slot cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; `slot cargo nextest run --locked -p swell-syntax --test main`; `slot cargo nextest run --locked -p swell-compiler --test main`; `slot cargo nextest run --locked -p swell-test262 --test main`; each exits 0. `ls crates/compiler/tests/sources/class-static-block.* | wc -l` prints `2` (0 on the base). `slot task clippy`, `slot task forbid-unsafe-check` and `sh taskfiles/test-main.sh check` exit 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing (`BASE` the join the unit starts from); `git diff --quiet "$BASE" -- crates/swell/src` exits 0. The text: `grep -rn 'class static block is not supported\|hold("class static block", 8' crates | wc -l` prints `0` (2 on its base: the hold call and the header of `refused/class-static-block.js`). `git diff --quiet "$BASE" -- conformance/host/requirements.txt` exits 0 and `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `1`. Time: every test this story adds stays within the bounds of Change, as the unit measured them; the report lists each measured time and the sum, and a time over its bound fails this acceptance.

test262. The full run is not the unit's. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-static-blocks.cases.txt" > .out/story-static-blocks.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-objects.cases.txt` (the 523 passing cases of the integrity, key and entry functions of `Object` and `JSON.stringify`), `cat .out/story-static-blocks.cases.txt docs/plan/slice-08/guard-objects.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 558 cases, 1,110 runs over the plan's base run), then, in the background, `slot --timeout 30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt` (one pool job, as `story:new-target-meta-property` states), and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-static-blocks.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1, prints 70 lines `failing` and `passed 523 bound 523 B 523 F 35 R 35`; it prints `missing` for a list case the part lacks and exits 1 on an empty list); (b) `awk -F'\t' '$6 ~ /class (static block )?is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0` (70 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

Static blocks that read private names (`story:private-fields-and-private-in`); static fields (`story:public-class-fields`); eval code's flags in a block (`story:eval-in-class-field-initializers`); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).

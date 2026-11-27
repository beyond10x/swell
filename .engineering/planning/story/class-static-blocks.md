---
format: aep.planning-md/3
id: story:class-static-blocks
kind: story
status: implemented
title: Static blocks in the order of the static elements, and the cases where the parts of order 3 meet
summary: '484 cases (968 runs): 35 of static blocks and 449 where the derived, field and private-method parts meet; lifts class static block.'
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:class-definition-evaluation
- depends_on: story:class-element-syntax-and-early-errors
- depends_on: story:public-class-fields
- depends_on: story:derived-classes-and-super-call
- depends_on: story:private-names-and-private-methods
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
  path: crates/syntax/src/scope.rs
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T08:23:48Z", actor: "human:operator", revision: 12, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-26T08:23:48Z", actor: "human:operator", revision: 13, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-27T08:38:02Z", actor: "human:operator", revision: 15, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story runs static blocks as compiled calls (item 4 of "Classes"): each static block is its own `StaticBlock` function (the body function of ClassStaticBlockDefinitionEvaluation, without the field flag, with a `var` scope of its own, an arguments object only when a direct `eval` in it can observe one), made as a method with home object F and called with `this` F at its place in the step-4 sequence of `story:class-definition-evaluation` (`<F>; MakeMethod(f); Call(0); Pop`), in source order with the static field runs of `story:public-class-fields`. It fills one arm of the step-4 sequence that `story:class-definition-evaluation` owns (`static_calls`, `crates/compiler/src/class.rs`): the arm of a static block, by writing the function that arm calls, `compile_static_block`, which answers the block's template; it writes only that function: the sequence places the call, and this story places no call and changes no order; `new.target` undefined, SuperProperty through F, `await` reserved; an abrupt completion ends the class's definition, the class's own name left uninitialized. No list of static elements and no walk exist (decision 7 of slice 8): `static-init-sequence.js` holds by construction. It lifts the hold `class static block is not supported before slice 8` of `story:class-element-syntax-and-early-errors`. Since revision 3 it is also the story where the parts of order 3 meet without a private field (fields of a derived class run after `super()`, a field initializer that calls a private method, private methods in a derived class): its closure, through its edges, holds the derived, field and private-method parts, and it is the first story to hold them together.

Edges: `story:class-definition-evaluation` (`compile_static_block`, `MakeMethod`, the step-4 sequence), `story:class-element-syntax-and-early-errors` (the block's tree, its function and context) and `story:public-class-fields`, which fills the other arm of the same step-4 sequence, and two cases of this list (`language/statements/class/static-init-sequence.js`: field, block, field, block; `static-init-abrupt.js`) need both. `story:derived-classes-and-super-call` and `story:private-names-and-private-methods` (order 3): their parts are read by the 449 cases of this list where the parts of order 3 meet, and this story writes three files after them, so the edges order the merges: `parser/class.rs` (their `hold` calls and this one), `HOLDS` in `parser/mod.rs` (their lines and this one) and `compiler/src/resolve.rs` `function` (derived's binding `this` and this story's call line), and the call line in `function` is placed after derived's lines, which exist only once that story is merged. Order 4, built beside `story:eval-in-class-field-initializers`, which writes none of its files.

The list is `docs/plan/slice-08/story-static-blocks.cases.txt`: the 484 cases of the key `static-blocks` of `docs/plan/slice-08/scan.tsv`, no negative case. 35 are static-block cases without a private name or a meeting of order-3 parts: `language/statements/class` 15 (`static-init-*`: the block's statements, `this`, scope, the order with static fields, an abrupt block; `static-init-sequence.js` and `static-init-abrupt.js` since revision 1), the `static-init-await-binding-valid.js` cases of `language/statements/{variable,try,let,function,const}` and `language/expressions/{object,generators,function}` 19 (`await` as a binding name inside a function in a static block), and `language/expressions/class` 1. 449 are the cases where the parts of order 3 meet without a private field, which moved here from `story:private-fields-and-private-in` in revision 3: by their parts in `scan.tsv`, 415 `B D PM` (a public field beside a private method or accessor, nearly all in `language/{statements,expressions}/class/elements/`), 15 `B C PM`, 9 `B C D PM`, 3 `B C SP PM`, 3 `B C D`, 2 `B SP D A` (`language/{statements,expressions}/class/elements/super-access-from-arrow-func-on-field.js`, which went to the private-fields list in revision 2), 1 `B C D A` and 1 `B C SP E` (`language/statements/class/static-init-super-property.js`). The rule that puts them here is the scan's one rule (`docs/plan/slice-08/scan.awk`, `cl["static-blocks"]`): a case goes to the first story, in the order of the edges, whose closure (its own parts and those of every story it depends on, directly or not) holds every part the case needs; with the edges of revision 2 to `story:derived-classes-and-super-call` and `story:private-names-and-private-methods` this story's closure is `B NT SP C D PM E A`, no story of order 3 holds two of `C`, `D` and `PM`, and only `story:private-fields-and-private-in` holds `PF`. By directory the list is `language/statements/class` 249, `language/expressions/class` 216, `language/expressions/object` 8, `language/statements/variable` 3, `language/expressions/generators` 2, `language/expressions/function` 2, `language/statements/{try,let,function,const}` 1 each. Their 968 runs fail on the plan's base run (the full run of 2026-11-26 on `149e12f1`, equal to `conformance/test262/expected.txt` on `62c7cf59`; slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`), all on `class is not supported before slice 8`. Cases that also need a private field are on `story:private-fields-and-private-in`'s list. A case of the list that ends on a part outside this story goes to `docs/plan/slice-08/close-later.tsv` by the coordinator's commit, as `story:new-target-meta-property` states.

Design it builds on, by name: `docs/design/engine.md` section 3, "Classes", items 2 (step 4) and 4 (the static elements; a static block's function and its arguments object; form (c) rejected). No sentence of the design is missing.

## What later stories reuse

The static block's function and its call in the step-4 sequence (`story:private-fields-and-private-in` runs static blocks that read private names; `story:eval-in-class-field-initializers` reads that a `StaticBlock` template has no field flag).

## Domain relations

- A static block → the class constructor that is its home object and its `this` (one) — inferable (inferred from ECMA-262 `#sec-runtime-semantics-classstaticblockdefinitionevaluation`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- Syntax: the hold `class static block` is lifted: the `hold` call in `static_block` (`parser/class.rs`) and the line `("class static block", 8),` of `HOLDS` (`parser/mod.rs`) are deleted. No fixture moves: `crates/syntax/tests/sources/held/class-static-block.js` (header `held "class", "class static block"`) parses in both modes from this story on, as `sources::held_sources_answer_their_first_construct_still_held` reads from `HOLDS`, and `compile::held_sources_compile_once_nothing_in_them_is_held` compiles it. The line before it, `("private name in", 8),`, is deleted by `story:private-fields-and-private-in`, which comes after this story by an edge.
- The stack bound at the depth of the parser's bound (`fits_the_stack_bound`, `crates/syntax/tests/sources.rs`). This story lifts the last hold of the nesting kind `class { static { class { static { ... } } } }` (it holds `class`, lifted by `story:class-definition-evaluation`, then `class static block`) of `story:class-element-syntax-and-early-errors` (`HOLDS` then has no line that kind's source holds), so from this story on that source parses to the end at the bound's depth, `admitted` no longer answers it by the held refusal, and `analyse_parsed` (`crates/syntax/src/lib.rs`) runs the scope analysis (`scope::analyse`, `crates/syntax/src/scope.rs`) on it at that depth for the first time; `fits_the_stack_bound` now runs the analysis for this kind through every entry. This story owns what follows: a failure of that test for this kind (a stack overflow, a panic or a wrong answer of the analysis) is fixed here, in `scope.rs`: the analysis refuses at the depth the parser's bound gives, with the nesting error, as the parser does, and has no recursion without a bound (its walk keeps its pending work on the heap today, `scope.rs`, the module documentation: "The walk does not recurse"); the report names the kind and the depth reached. No other story fixes it, and no test is relaxed for it.
- Compiler: `crates/compiler/src/class.rs`, `compile_static_block` (the `CompileError::Tree` of `story:class-definition-evaluation` replaced): the block's `StaticBlock` function compiled as a function body; `compile_static_block` answers that function, and the step-4 sequence of `story:class-definition-evaluation` calls it where the block stands in source order. `crates/compiler/src/resolve.rs`, a function `static_block_scope` called from `function` for the kind `StaticBlock`: the block's `var` scope, its lexical declarations and functions, and the arguments object made only when the block's code contains a direct `eval` ("Names", item 4). No instruction is added. A compiler fixture pair `crates/compiler/tests/sources/class-static-block.{js,disasm}` (new).
- The cases where the parts of order 3 meet (449 of the list; taken over in revision 3 from `story:private-fields-and-private-in`, with the tests of them below): the unit runs them on its part and fixes, in the function that owns it, what the parts did not settle where they meet: the private method adds before the first field (pass 1 before pass 2 of the instance initializer, `story:class-definition-evaluation`'s emitter); the initializer of the class whose constructor runs `super()` (item 7, step 5; for `super()` in an arrow, the box's function); an initializer's throw after `BindThis` ends the SuperCall with `this` bound; a derived constructor that returns an object before `super()` adds nothing to it. A fix outside this story's Writes is the smallest change in the owning function (`compiler/src/class.rs`'s emitter, `interp/class.rs`) and is named in the report.
- Tests, each in both collection modes, in `crates/swell/tests/class_static_blocks.rs` (new): `a_block_runs_with_this_the_constructor`; `a_blocks_var_is_not_visible_outside`; `super_property_in_a_block`; `new_target_in_a_block_is_undefined`; `blocks_and_static_fields_run_in_source_order`; `a_throw_from_a_block_leaves_the_class_binding_uninitialized`; `await_as_a_binding_name_in_a_nested_function`; `a_direct_eval_of_arguments_in_a_block_sees_its_own_arguments_object`; `a_block_loop_ends_at_the_step_budget` (`Termination::StepBudget`); `a_block_loop_ends_at_an_interrupt` (`Termination::Interrupted`); `a_recursion_of_classes_with_blocks_stops_at_frame_depth` (a `RangeError`; the depth of the recursion 1,000, 100 under `collect-every-safe-point`); and, for the cases where the parts of order 3 meet (moved with them from `story:private-fields-and-private-in`), `a_field_initializer_sees_the_private_methods`; `fields_run_after_super_for_the_class_whose_constructor_called_it`; `fields_run_after_super_in_an_arrow_called_later`; `an_initializer_that_throws_after_super_leaves_this_bound`; `a_derived_constructor_returning_an_object_before_super_adds_nothing`; `super_property_in_an_initializer_of_a_derived_class`; `a_class_with_100_private_methods_and_100_fields_constructed_1000_times` (1,000 constructions, 100 under `collect-every-safe-point`; within the step budget with a small `steps_per_entry`, `Termination::StepBudget`, and within a memory budget); `a_recursion_of_new_through_super_in_an_initializer_stops_at_frame_depth` (a `RangeError`): that is 19 tests under `class_static_blocks::`, none on the base (the file is new). `crates/swell/tests/main.rs` generated again.
- Sizes: no layout changes (the record tests pass as they are).
- Bounds, as `AGENTS.md` and the `AGENTS.md` of the folder that holds the repository ("Tests stay fast", "A job in the pool ends within 10 minutes", "No Python for permanent tools") state them: a test is terminated at 60 s and counts as failed; the gate fails when the wall time of its test steps passes 120 s; a test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` (at the reduced count named above) and under 2.5 at `SWELL_FULL_COUNTS=1`, measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` (user plus system); the unit times each test before it adds it and names each one over 1 CPU second with its time. A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures, probes or waits runs without the prefix. No `unsafe`, no third-party crate, no panic, `unwrap`, `expect` or unchecked indexing; no Python in any tool, test or command; intermediate files in the scratch directory or `.out/`.
- Size: about 650 lines (compiler 110, resolve 50, the meeting cases' fixes about 60, tests 430): one session; kept apart from the field story because it lifts its own hold and shares no function with it, and the meeting cases are here because this is the first story whose closure holds their parts.

## Requirements

- No line of `conformance/host/requirements.txt` moves here: `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `1` on `61f2a8e2` and after this story; only `story:host-ensure-can-add-private-element` moves those lines.
- The run loop is not touched (no instruction is added); the block runs as a frame of the loop, never through a re-entry; `STACK_BYTES_PER_REENTRY` is unchanged, and the stack tests pass on seven eighths of `min_stack_bytes()`.

Writes: `crates/syntax/src/parser/{class,mod}.rs` (the hold call, its line of `HOLDS`), `crates/syntax/src/scope.rs` (only for a failure of `fits_the_stack_bound` at its nesting kind, as Change says); `crates/compiler/src/class.rs` (`compile_static_block`, the function that the arm of a static block of `static_calls` calls; `static_calls` itself is not written), `crates/compiler/src/resolve.rs` (`static_block_scope` and its call line in `function`), `crates/compiler/tests/sources/class-static-block.{js,disasm}` (new); `crates/swell/tests/class_static_blocks.rs` (new), `crates/swell/tests/main.rs` (generated). A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/{lexer,early}.rs`, `crates/compiler/src/{bytecode,disasm,expression}.rs`, `crates/swell/src/` but for the smallest fix of a meeting case in its owning function (Change), `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared files: with `story:public-class-fields`, ordered before this story by an edge (`compiler/src/class.rs`: there `compile_field` and `initializer_fields`, here `compile_static_block`; `compiler/src/resolve.rs`: there `field_key_bindings`, here `static_block_scope`); with `story:derived-classes-and-super-call` and `story:private-names-and-private-methods` (order 3; ordered before this story by an edge each, for these shared files: `parser/class.rs`, each its own `hold` call; `HOLDS`, each its own line; `resolve.rs`: there `function`'s binding `this` and `private_record`, here `static_block_scope` and its one call line in `function`, after the lines of `story:derived-classes-and-super-call`); `story:eval-in-class-field-initializers`, beside it in order 4, writes none of these files; `tests/main.rs` generated again on the join.

## Acceptance

The tests named in Change pass in both collection modes, run by the binaries of this change and not as a whole step. `slot cargo nextest run --locked --no-tests=fail -p swell --test main -E 'test(/^class_static_blocks::/)'` runs 19 tests and passes (on the base the filter matches none and the command fails); then `slot cargo nextest run --locked -p swell --test main -E 'test(/^class_static_blocks::/) | test(/^class_fields::/) | test(/^derived_classes::/) | test(/^private_methods::/) | test(/^classes::/) | test(/^eval::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/) | test(/^collect::/) | test(/^termination::/) | test(/^interrupt::/)'` and the same with `--features collect-every-safe-point`; `slot cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; `slot cargo nextest run --locked -p swell-syntax --test main`; `slot cargo nextest run --locked -p swell-compiler --test main`; `slot cargo nextest run --locked -p swell-test262 --test main`; each exits 0, in this story's tree and after `story:eval-in-class-field-initializers` is merged first: the wave branch is merged into the unit, the one conflict this story names is fixed as it names it (`tests/main.rs`: generated again; the two stories share no other file), and the same commands run again and exit 0. `ls crates/compiler/tests/sources/class-static-block.* | wc -l` prints `2` (0 on the base). `slot task clippy`, `slot task forbid-unsafe-check` and `sh taskfiles/test-main.sh check` exit 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing (`BASE` the join the unit starts from); `git diff --stat "$BASE" -- crates/swell/src` prints nothing, or only the files of the fixes of meeting cases the report names. The text: `grep -rn '"class static block", 8' crates | wc -l` prints `0` (2 on its base: the hold call and the line of `HOLDS`; the header of `held/class-static-block.js` does not match). `ls crates/syntax/tests/sources/held | wc -l` prints the same number as on its base (no fixture moves). `git diff --quiet "$BASE" -- conformance/host/requirements.txt` exits 0 and `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `1`. Time: every test this story adds stays within the bounds of Change, as the unit measured them; the report lists each measured time and the sum, and a time over its bound fails this acceptance.

test262. The full run is not the unit's. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-static-blocks.cases.txt" > .out/story-static-blocks.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-objects.cases.txt` (the 523 passing cases of the integrity, key and entry functions of `Object` and `JSON.stringify`), `cat .out/story-static-blocks.cases.txt docs/plan/slice-08/guard-objects.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 1,007 cases, 2,008 runs over the plan's base run), then, in the background, `slot --timeout 30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt` (one pool job, as `story:new-target-meta-property` states), and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-static-blocks.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1, prints 968 lines `failing` and `passed 523 bound 523 B 523 F 484 R 484`; it prints `missing` for a list case the part lacks and exits 1 on an empty list); (b) `awk -F'\t' '$6 ~ /class (static block )?is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0` (968 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

Static blocks that read private names and every case that needs a private field (`story:private-fields-and-private-in`); static fields (`story:public-class-fields`); eval code's flags in a block (`story:eval-in-class-field-initializers`); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).

## As delivered

The unit `staticblocks` of wave big-2 delivered the story on `impl/staticblocks` (one
correction after the adversary's pass), merged into `wave/big-2`. Where the delivery differs
from the text above:

- `static_block_scope` (`crates/compiler/src/resolve.rs`) is not written and `resolve.rs` is
  not changed. The resolver's `function` walks a `StaticBlock` like any function: its `var`
  names in the function scope, the top level of its body a lexical scope, and the binding
  `arguments` only when a direct `eval` in the block's code can observe one (the scope
  analysis sets it). The tests, the compiler fixture, the part run and the adversary's probes
  of the scope pass without it.
- No file under `crates/swell/src` changed and no instruction is added:
  `Compiler::compile_static_block` answers the block's own function, and `static_calls`,
  which existed, emits `Dup; MakeMethod(f); Call(0); Pop` where the block stands.
- The meeting cases (fields of a derived class, a field initializer that calls a private
  method, private methods in a derived class) needed no fix: on the unit's base every case
  of the list without a static block passed. The list read 448 of 484 passed on the base
  (the 36 cases with a static block failed on the hold) and 484 of 484 on the unit's head.
- 20 tests under `class_static_blocks::`, not 19:
  `a_method_between_static_fields_ends_their_run` was added after the adversary's pass. It
  holds that a method or an accessor between static fields ends their run in `static_calls`;
  with the reset planted out it answers `"a,block 1 undefined,c|1,,3,g"` for
  `"a,b,block 1 2,c|1,2,4,g"`. The acceptance's filter runs 20 tests.
- `a_class_with_100_private_methods_and_100_fields_constructed_1000_times` runs fewer
  constructions than the story names: 500 in the gate's default build and 1,000 under
  `SWELL_FULL_COUNTS=1`; 1 in a collect build and 2 at the full count (the story's 100 took
  53.0 CPU s there). Every count holds the step charge of the private find: with the charge
  planted out the test is red at each (`570331 steps for 500 constructions`, `1471 steps for
  1 constructions`). Its memory part runs in the default build only, in a room of 1 MiB.
- A direct `eval` in a block: `arguments` in the eval code is no SyntaxError. PerformEval's
  inClassFieldInitializer is false for a block's function, and EvaluateClassStaticBlockBody
  runs FunctionDeclarationInstantiation, so `eval("arguments")` is the block's own arguments
  object, of length 0. `super()` in the eval code is a SyntaxError, `new.target` is
  `undefined`, `super.x` reads through F, a `var` stays in the eval's own record. A private
  name the class declares meets the refusal of `story:private-names-and-private-methods`
  until `story:private-names-in-eval-code`, whose change turns that line of
  `a_direct_eval_of_arguments_in_a_block_sees_its_own_arguments_object` into a run.
- The stack-bound check of the syntax crate admits 155 levels of a class nested in a static
  block at limit 2,500 and 467 at 7,500, through Script, Eval and FunctionBody;
  `crates/syntax/src/scope.rs` needed no change.
- No fixture moved: `crates/syntax/tests/sources/held/` holds 25 files; its two sources with
  a static block parse in both modes and compile, which the two tests that walk the
  directory require of a source none of whose constructs is held.
- Outside the story's writes: `crates/syntax/tests/sources.rs`, where
  `class_elements_scale_linearly` expects its shape "static blocks" to parse.
- The reviews: `review-result:adversary-staticblocks-pass-1` (green: three notes; four
  planted defects red in both builds; 71 cases of test262 beyond the list, 132 of 138 runs
  passed, 60 on the base, none lost) and the coordinator's read of the correction
  (`review-result:staticblocks-correction-1-coordinator-read`).

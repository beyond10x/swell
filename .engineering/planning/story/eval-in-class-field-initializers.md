---
format: aep.planning-md/3
id: story:eval-in-class-field-initializers
kind: story
status: draft
title: 'Eval in field initializers: [[ClassFieldInitializerName]] and PerformEval''s inClassFieldInitializer'
summary: 84 cases (164 runs).
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:public-class-fields
- depends_on: story:derived-classes-and-super-call
scope:
- confidence: cited
  path: crates/swell/src/dynamic_code.rs
- confidence: inferred
  path: crates/swell/tests/class_field_eval.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/lib.rs
revision: 5
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story is direct and indirect eval inside a field initializer: PerformEval's `inClassFieldInitializer` (`#sec-performeval`: "If inClassFieldInitializer is true and ContainsArguments of body is true, throw a SyntaxError exception"), and the other three flags as they read for an initializer (`inFunction` and `inMethod` true, `inDerivedConstructor` false: `new.target` is undefined in the eval, a SuperProperty reaches the initializer's home object, a SuperCall is a SyntaxError), the eval's `this` the instance (or F in a static field), its `var` declarations in a record of that eval alone (class code is strict, so a strict eval's `var` names stay in its own variable environment, `#sec-performeval`, strictEval).

[[ClassFieldInitializerName]] is the field flag of the template: a template of kind `FieldInitializer` (item 5 of "`this` and `arguments` in an arrow function", item 3 of "Classes"), which `story:new-target-meta-property` already reads to derive the flag from the eval frame's function (the caller's for a direct eval; for an arrow in an initializer, the function of its box). So this story adds the rule that reads the flag, at the eval's parse, and the tests that prove the four flags for initializers; it puts nothing on a frame or in `interp/class.rs`, and `crates/swell/src/dynamic_code.rs` is written only if the unit finds the derivation of `story:new-target-meta-property` wrong for a case, which it then names.

Edges: `story:public-class-fields` (the initializers and their `FieldInitializer` templates) and `story:derived-classes-and-super-call` (the eval rules of `inMethod` and `inDerivedConstructor`, beside which this rule goes, and the derived classes of its SuperCall test). Order 4, built beside `story:class-static-blocks`, which writes none of its files.

The list is `docs/plan/slice-08/story-eval-fields.cases.txt`: the 84 cases of the key `eval-fields` of `docs/plan/slice-08/scan.tsv` (a class with a public field whose initializer calls `eval`, and no private name; no negative case): `language/statements/class/elements` 42 and `language/expressions/class/elements` 42 (`*-direct-eval-err-contains-arguments`, `*-direct-eval-err-contains-supercall*`, `*-direct-eval-contains-superproperty-*`, `*-direct-eval-err-contains-newtarget`, their `indirect-eval` and `nested-*` variants, and `arrow-body-*-eval-*`). Their 164 runs fail on the plan's base run (the full run of 2026-11-26 on `149e12f1`, equal to `conformance/test262/expected.txt` on `62c7cf59`; slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`), all on `class is not supported before slice 8`. The same cases with private names are on `story:private-names-in-eval-code`'s list. A case of the list that ends on a part outside this story goes to `docs/plan/slice-08/close-later.tsv` by the coordinator's commit, as `story:new-target-meta-property` states.

Design it builds on, by name: `docs/design/engine.md` decision 3, "`this` and `arguments` in an arrow function", item 5 (the four flags; the flag on the template; rejected: the flag on the frame); section 3, "Classes", item 4 (a `FieldInitializer` template has no arguments object, and eval code there refuses `arguments`). No sentence of the design is missing.

## What later stories reuse

The rule of `inClassFieldInitializer` (`story:private-names-in-eval-code` evaluates private names in the same eval code).

## Domain relations

- A field initializer function → its field flag (one bit of its template) — inferable (inferred from ECMA-262 `#sec-runtime-semantics-classfielddefinitionevaluation` and `#sec-performeval`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- Syntax: `crates/syntax/src/lib.rs`, `parse_eval_with`: when `in_class_field_initializer` is set, ContainsArguments of the eval code (`arguments` anywhere outside a function that is not an arrow) is the SyntaxError, with the rule in `crates/syntax/src/early.rs` (a function `contains_arguments`, beside the eval rules of `story:derived-classes-and-super-call`); the documentation of `parse_eval` names the rule (ContainsArguments) beside the other three.
- Tests, each in both collection modes, in `crates/swell/tests/class_field_eval.rs` (new): `eval_of_arguments_in_an_initializer_is_a_syntax_error`; `indirect_eval_of_arguments_reads_the_global_binding`; `arguments_in_a_function_inside_the_eval_is_allowed`; `eval_new_target_is_undefined`; `eval_super_property_reads_the_home_objects_prototype`; `eval_super_call_is_a_syntax_error_also_in_a_derived_class`; `eval_this_is_the_instance`; `an_evals_var_stays_in_that_eval` (the test of the earlier draft that held a `var` visible to a later eval was wrong and is replaced: `x = eval("var v = 1; v")` gives 1, and a later `eval("typeof v")` in the same initializer and in the next field's both give `"undefined"`, since a strict eval's `var` stays in its own record); `eval_in_a_static_field_sees_the_constructor_as_this_and_refuses_arguments`; `an_arrow_in_an_initializer_passes_the_flags_to_its_eval` (`x = () => eval("arguments")`: SyntaxError; `() => eval("super.x")`: the home object's prototype); `an_eval_loop_in_an_initializer_ends_at_the_step_budget` (`Termination::StepBudget`); `an_eval_loop_in_an_initializer_ends_at_an_interrupt` (`Termination::Interrupted`); `nested_eval_in_an_initializer_is_bounded` (an eval that evals itself until the `RangeError` of `eval_stack` or `frame_depth`, no panic; at most 1,000 levels, 100 under `collect-every-safe-point`): that is 13 tests under `class_field_eval::`, none on the base (the file is new). `crates/swell/tests/main.rs` generated again.
- Sizes: no layout changes (no field of a frame, a function or a record; the flag is the template's kind).
- Bounds, as `AGENTS.md` and the `AGENTS.md` of the folder that holds the repository ("Tests stay fast", "A job in the pool ends within 10 minutes", "No Python for permanent tools") state them: a test is terminated at 60 s and counts as failed; the gate fails when the wall time of its test steps passes 120 s; a test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` (at the reduced count named above) and under 2.5 at `SWELL_FULL_COUNTS=1`, measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` (user plus system); the unit times each test before it adds it and names each one over 1 CPU second with its time. A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures, probes or waits runs without the prefix. No `unsafe`, no third-party crate, no panic, `unwrap`, `expect` or unchecked indexing; no Python in any tool, test or command; intermediate files in the scratch directory or `.out/`.
- Size: about 300 lines (syntax 50, tests 250): one session.

## Requirements

- No line of `conformance/host/requirements.txt` moves here: `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `1` on `61f2a8e2` and after this story; only `story:host-ensure-can-add-private-element` moves those lines.
- The run loop is not touched; eval stays the re-entry it is (`eval_stack::`, `reentry_depth`); `STACK_BYTES_PER_REENTRY` is unchanged, and the stack tests pass on seven eighths of `min_stack_bytes()`.

Writes: `crates/syntax/src/{lib,early}.rs`, `crates/swell/tests/class_field_eval.rs` (new), `crates/swell/tests/main.rs` (generated); `crates/swell/src/dynamic_code.rs` only as Context says. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/parser/`, `crates/compiler/`, `crates/swell/src/interp/`, `crates/swell/src/object/`, `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared files: `lib.rs` and `early.rs` of `swell-syntax` are written before it by `story:derived-classes-and-super-call` (an edge; here a rule and `contains_arguments` beside its rules) and after it by `story:private-names-in-eval-code` (an edge; there the private names of the eval context). `story:class-static-blocks`, beside it in order 4, writes none of these files; `tests/main.rs` generated again on the join.

## Acceptance

The tests named in Change pass in both collection modes, run by the binaries of this change and not as a whole step. `slot cargo nextest run --locked --no-tests=fail -p swell --test main -E 'test(/^class_field_eval::/)'` runs 13 tests and passes (on the base the filter matches none and the command fails); then `slot cargo nextest run --locked -p swell --test main -E 'test(/^class_field_eval::/) | test(/^class_fields::/) | test(/^derived_classes::/) | test(/^eval::/) | test(/^eval_stack::/) | test(/^arrow_functions::/) | test(/^new_target::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^collect::/) | test(/^termination::/) | test(/^interrupt::/)'` and the same with `--features collect-every-safe-point`; `slot cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; `slot cargo nextest run --locked -p swell-syntax --test main`; `slot cargo nextest run --locked -p swell-test262 --test main`; each exits 0. `slot task clippy`, `slot task forbid-unsafe-check` and `sh taskfiles/test-main.sh check` exit 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing (`BASE` the join the unit starts from). The text: `grep -c 'ContainsArguments' crates/syntax/src/lib.rs` prints at least `1` (0 on the base: the documentation of `parse_eval` names no such rule). `git diff --quiet "$BASE" -- conformance/host/requirements.txt` exits 0 and `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `1`. Time: every test this story adds stays within the bounds of Change, as the unit measured them; the report lists each measured time and the sum, and a time over its bound fails this acceptance.

test262. The full run is not the unit's. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-eval-fields.cases.txt" > .out/story-eval-fields.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-functions.cases.txt` (2,546 passing cases of functions, arrows, methods, calls, `new`, eval code and `Function`), `cat .out/story-eval-fields.cases.txt docs/plan/slice-08/guard-functions.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 2,630 cases, 4,436 runs over the plan's base run), then, in the background, `slot --timeout 30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt` (one pool job, as `story:new-target-meta-property` states), and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-eval-fields.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1, prints 164 lines `failing` and `passed 2546 bound 2546 B 2546 F 84 R 84`; it prints `missing` for a list case the part lacks and exits 1 on an empty list); (b) `awk -F'\t' '$6 ~ /class is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0` (164 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

Private names in eval code (`story:private-names-in-eval-code`); eval in static blocks beyond what `story:class-static-blocks` tests; the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).

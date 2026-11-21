---
format: aep.planning-md/3
id: story:interpreter-and-script-entry-points
kind: story
status: active
title: 'The interpreter runs scripts: environments, functions, parse_script and evaluate_script with every budget'
summary: Environments, ECMAScript functions, the frame stack interpreter, ScriptEvaluation and GlobalDeclarationInstantiation, strict runtime semantics, steps, depths, interrupt, termination, min_stack_bytes; 28 HOST lines to test, HOST-54 after its split.
relations:
- decomposes: epic:slice-01-harness
- depends_on: story:object-model-and-realms
- depends_on: story:bytecode-compiler
scope:
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: crates/swell/Cargo.toml
- confidence: cited
  path: crates/swell/src/heap/collect.rs
- confidence: cited
  path: crates/swell/src/heap/roots.rs
- confidence: cited
  path: crates/swell/src/host
- confidence: cited
  path: crates/swell/src/interp
- confidence: cited
  path: crates/swell/src/object/mod.rs
- confidence: cited
  path: crates/swell/src/ops/conversion.rs
- confidence: cited
  path: crates/swell/src/ops/objects.rs
- confidence: cited
  path: crates/swell/src/ops/operators.rs
- confidence: cited
  path: crates/swell/src/ops/testing.rs
- confidence: cited
  path: crates/swell/src/script.rs
- confidence: cited
  path: crates/swell/src/value.rs
- confidence: cited
  path: crates/swell/tests
- confidence: cited
  path: crates/swell/tests/language
- confidence: cited
  path: crates/swell/tests/language.rs
- confidence: cited
  path: crates/swell/tests/support/script.rs
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T04:49:59Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T04:49:59Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
---
## Context

This is where swell first runs a script: environments, ECMAScript function objects, the
interpreter of decision 3 over the bytecode of `story:bytecode-compiler`, and the script entry
points of `docs/design/host-interface.md` section 8 (`parse_script`, `evaluate_script`, the same
on `Cx`), on the object model and realms of `story:object-model-and-realms`. The bounds of the
loops and recursions this story creates come with it, each with the test that reaches it (the
epic, acceptance items 5 to 8, 10 and 13; decision 3): the step budget charged one unit per
instruction and checked at every `LoopJump`, call, return and re-entry, the interrupt checked at
the same places, `frame_depth` on the engine frame stack, `reentry_depth` and the Rust stack a
re-entry needs (`Limits::min_stack_bytes`), `parser_nesting` through `parse_script`.

Termination semantics (decision 3, interface section 10): at `Terminated` no `catch` or `finally`
block runs, the frames are dropped, the agent stays `Alive`, and a host function that catches
`Abrupt::Terminated` cannot end it (`HOST-72`).

Strict mode, runtime half (taken into slice 1, see `story:syntax-es5-and-strict`): `this` is not
coerced in strict functions (`sec-ordinarycallbindthis`), assignment to an unresolvable
reference throws a `ReferenceError` (`sec-putvalue`), a failed `[[Set]]` or `[[Delete]]` throws a
`TypeError` (`sec-putvalue`, `sec-delete-operator`). A regular expression literal throws its
`SyntaxError` of the current realm when evaluated (the design, "A part before its slice";
`story:bytecode-compiler` emits the throw).

`object/ecmascript_function.rs` holds the kind data and the `[[Call]]` and `[[Construct]]`
signatures that `story:object-model-and-realms` wrote for the dispatch arm of this kind; this
story replaces the struct, the bodies and the trace function in that file and does not edit
`object/mod.rs`. The other handles this story creates are reported to the collector through the
sites `story:heap-values-and-host-surface` created empty for it (Change lists them); the frame
stack becomes a root and the global environment is attached to the realm without an edit of
`heap/collect.rs` or `realm/mod.rs`.

`Limits::min_stack_bytes` is first measured here on re-entry through host functions;
`story:host-interface-hardening` measures it again once the builtins that re-enter exist and
corrects the constants if needed.

Requirements of slice 1 whose test needs the builtins of `story:harness-builtins` as well as
scripts are named by `story:host-interface-hardening`, not here: `HOST-10` (`push`), `HOST-16`
(`map`), `HOST-49` (`Array`), `HOST-53` (`Object.prototype.hasOwnProperty.call`).

## Requirements

Lines moved from `pending slice 1` to `test`:

- `budgets.rs`: `HOST-6` `swell::budgets::host_6_endless_loop_ends_in_step_budget`, `HOST-7`
  `swell::budgets::host_7_next_entry_gets_the_whole_budget`, `HOST-8`
  `swell::budgets::host_8_nested_entry_spends_the_outer_budget`, `HOST-9`
  `swell::budgets::host_9_zero_steps_ends_before_the_first`, `HOST-13`
  `swell::budgets::host_13_frame_depth_counts_the_script_context` (the three scripts of the
  requirement: the recursion throws a `RangeError` of the realm, `n` then answers
  `Normal(D - 1)`, and `try { f(); } catch (e) {}` answers `Normal`), `HOST-14`
  `swell::budgets::host_14_zero_frame_depth_throws_range_error`, `HOST-20`
  `swell::budgets::host_20_doubling_a_string_past_string_length_throws`, `HOST-22`
  `swell::budgets::host_22_zero_string_length_throws_on_concatenation`.
- `scripts.rs`: `HOST-18` `swell::scripts::host_18_100000_brackets_are_one_parse_error`,
  `HOST-19` `swell::scripts::host_19_zero_parser_nesting_refuses_any_source`, `HOST-137`
  `swell::scripts::host_137_nesting_error_names_parser_nesting_and_10_levels_parse` (the error
  object has the realm `%SyntaxError.prototype%` as prototype), `HOST-35`
  `swell::scripts::host_35_utf8_and_utf16_sources_agree`, `HOST-36`
  `swell::scripts::host_36_parse_error_has_the_realm_syntax_error_prototype`, `HOST-37`
  `swell::scripts::host_37_throw_and_normal_completion_values`, `HOST-50`
  `swell::scripts::host_50_setup_this_is_script_this`, `HOST-54`
  `swell::scripts::host_54_var_script_evaluates_twice`.
- `termination.rs`: `HOST-69` `swell::termination::host_69_finally_does_not_run_after_step_budget`
  (then the script `f` answers `Normal(undefined)`), `HOST-70`
  `swell::termination::host_70_agent_stays_alive_after_each_budget`, `HOST-72`
  `swell::termination::host_72_a_host_function_cannot_end_a_termination`.
- `interrupt.rs`: `HOST-90` `swell::interrupt::host_90_interrupt_from_another_thread_ends_the_loop`,
  `HOST-91` `swell::interrupt::host_91_interrupted_entry_clears_the_flag`, `HOST-92`
  `swell::interrupt::host_92_interrupt_before_an_entry_ends_it_at_zero_steps`, `HOST-93`
  `swell::interrupt::host_93_clear_before_the_entry_lets_it_run`, `HOST-96`
  `swell::interrupt::host_96_interrupt_set_in_a_host_function_ends_the_run_after_it`.
- `calls.rs`: `HOST-103` `swell::calls::host_103_host_function_receives_this_and_arguments`,
  `HOST-104` `swell::calls::host_104_new_target_and_non_constructors`, `HOST-105`
  `swell::calls::host_105_host_throw_reaches_the_script_catch`, `HOST-107`
  `swell::calls::host_107_cx_evaluate_script_returns_its_completion_value`.

## Additions after wave 2 of slice 1 (2026-11-21)

The design changed after this story was accepted (`docs/design/host-interface.md`, decision 5,
"A method that takes a handle returns `Result<_, HostError>`"; see
`docs/plan/slice-01-wave-2.md`). Where this story's text gives another result type, the
design governs.

- `Cx::evaluate_script` returns `Result<Completion<Value>, HostError>`, and `Cx::parse_script`
  wraps its scoped result in `Result<_, HostError>`. Each checks its handle before anything
  else, before the sticky termination of decision 10 too: a stale handle given while a run is
  terminated answers `Err(StaleHandle)` and the run stays terminated.
- Host function results reach this story only through the `[[Call]]` and `[[Construct]]` of
  `story:object-model-and-realms`, which check the handles a host function returns.
- `HOST-137`, the case that parses: its source is `0` inside 10 nested parentheses followed by
  `;`. Automatic semicolon insertion is slice 2's, and `swell-syntax` refuses a source that
  needs it (found by the unit of `story:syntax-es5-and-strict`). The requirement asks for the
  nesting, not for the missing semicolon.
- `Limits::min_stack_bytes()` (this story) includes the parser's stated need,
  `swell_syntax::ParseOptions { max_nesting: parser_nesting, .. }.stack_bytes()` (32,768 bytes
  plus 224 per level; `docs/plan/slice-01-wave-3.md`), plus the stack the re-entries take,
  which this story measures in a build without incremental compilation and without a compiler
  cache (frames there are up to 20 % larger than in the pool's default build; a test that
  passes only with the smaller frames aborted the first run of the gate on the forge). A test
  of a stack bound runs its subject in a child process on a thread of the stated size, so that
  an overflow fails one test and does not abort the test binary.
- The bytecode this story runs is the contract documented in `crates/compiler/src/bytecode.rs`
  (`story:bytecode-compiler`): the completion value register, initially `undefined`; `Closure`
  with an environment for a function expression's own name; the `items` and `envs` a handler
  restores, and that an empty range covers nothing; the tag values of a completion that leaves
  a `finally` block; `ForInStart` giving an empty enumerator for `null` and `undefined`; a
  function environment only when `captured` is not empty. `CompileError` has two variants:
  `TooMany` for a bound of the format and `Tree` for a tree `parse_script` cannot produce; both
  become a defined answer of the entry point, never a panic.
- The counts of Acceptance are measured on the base of the wave: with `HOST-160` the list has
  161 lines (`HOST-161` was added on the same day).

- The current realm (2026-11-21, `docs/design/host-interface.md` decision 7, `HOST-161`): `with_cx`,
  `Agent::call` and `Agent::construct` take a `&Realm`; this story's tests pass one.
  `evaluate_script` and `Cx::evaluate_script` make the script's realm the current one for the
  run (`ecma262:sec-runtime-semantics-scriptevaluation`).

- `Writes:` is extended (2026-11-21, from what the unit of `story:harness-builtins` found on the
  same base): in any file under `crates/swell/src/`, an `expect(dead_code)` mark whose reason
  names this story may be removed once the item is used, and nothing else in that file changes;
  `crates/swell/src/object/mod.rs` (the mark on the ECMAScript function kind);
  `crates/swell/src/ops/objects.rs` (GetFunctionRealm reading an ECMAScript function's
  `[[Realm]]`, `is_constructor`, `instanceof_operator`, and their marks). The other story of
  the wave changes its own hunks in the same two files; the unit that lands second takes the
  other one's in.
- `Writes:` is extended once more (2026-11-21, found by this story's unit before it wrote
  anything): `crates/swell/src/value.rs` (the field of `Script` becomes visible in the crate)
  and `crates/swell/src/heap/roots.rs` (a root slot that holds a script record, with its
  trace). `RealmGlobalEnv` stays empty in slice 1: the declarative part of the global
  environment is empty until slice 6, which edits `realm/mod.rs`. Every call from Rust into
  an ECMAScript function or a script pushes an activation, so that `reentry_depth` and the
  current realm hold without a change to `object/builtin_function.rs`.

## Change

Failing tests first: the tests of Requirements, the language scripts and the tests named below
are written before the code; they fail to build or fail.

- `env/declarative.rs`, `env/object.rs`, `env/global.rs`, `env/function.rs`, `env/mod.rs` (9.1,
  `sec-the-environment-record-type-hierarchy`, `sec-global-environment-records`).
- `object/ecmascript_function.rs` (10.2: `OrdinaryFunctionCreate`, `MakeConstructor`,
  `[[Call]]`, `[[Construct]]`, `OrdinaryCallBindThis`, `FunctionDeclarationInstantiation`
  without an arguments object, which the parser refuses until slice 2).
- `interp/frame.rs` (frame: instruction index, operand stack, local slots, environment, handler
  table; the frame stack), `interp/dispatch.rs` (one arm per instruction of `swell-compiler`, the
  regular expression throw among them), `interp/budget.rs` (charging one step per instruction
  through the step counter of `story:object-model-and-realms`, the checks at `LoopJump`, call,
  return and re-entry, `frame_depth`), `interp/mod.rs` (run a frame stack to completion,
  re-entry from Rust, termination unwinding that runs no handler). Collections at safe points:
  instruction boundaries and re-entry points, as decision 4 says.
- `script.rs` (16.1: ParseScript with UTF-8 and UTF-16 sources, the parse errors as objects of
  the realm `%SyntaxError%`, the bytecode and its strings charged to the memory budget;
  ScriptEvaluation; `sec-globaldeclarationinstantiation`).
- `host/agent.rs`, `host/mod.rs`: `parse_script`, `evaluate_script`, `script_host_defined`,
  `last_entry_steps`, `Cx::parse_script`, `Cx::evaluate_script`. `host/limits.rs`:
  `Limits::min_stack_bytes`, computed from per-level Rust stack costs measured by
  `min_stack_bytes_is_enough` and written as constants with that test named beside them.
- `crates/swell/Cargo.toml`: dependencies on `swell-syntax` and `swell-compiler`. The feature
  `collect-every-safe-point` is defined by `story:heap-values-and-host-surface`; this story adds
  the instruction boundaries to the safe points it collects at.
- The trace and root sites the table of `story:heap-values-and-host-surface` gives to this story,
  each in a file this story already writes: the trace of the ECMAScript function kind
  (environment, realm, script, home object) in `object/ecmascript_function.rs`; the trace of
  environment records in `env/mod.rs`; `RealmGlobalEnv` in `env/global.rs` (its constructor
  creates the global environment record of a realm, `sec-newglobalenvironment`, and its trace
  reports it; the realm record of `story:object-model-and-realms` already holds the field and
  calls both, so this story does not write `realm/mod.rs`); the trace of script records in
  `script.rs`; the root site of the frame stack in `interp/mod.rs` (each frame operand stack,
  locals, environment, function and realm). It writes neither `heap/collect.rs` nor
  `object/mod.rs`.
- `crates/swell/tests/support/script.rs`: `run(limits, source) -> Outcome` with a fresh agent and
  realm, and `run_both(source)` that runs a source as written and with a `"use strict";`
  prologue.
- `crates/swell/tests/language/*.js`: one small script per statement, expression, function and
  strict-mode rule of the slice 1 grammar and one for the regular expression literal throw, each
  starting with `// <anchor>: <expected>`, the clause anchor and the expected outcome
  (`normal <value>`, `throw <ErrorName>`), and an optional `// strict-only` or `// sloppy-only`;
  `crates/swell/tests/language.rs`: `language_scripts_answer_their_expected_outcome` runs each
  in the modes it allows, and fails when the directory holds no `.js` file or a file has no
  expectation line (`AGENTS.md`, "A check that finds nothing to read fails").
- Tests besides Requirements: `budgets.rs`
  `catching_the_depth_error_and_recursing_again_ends_in_step_budget` (the epic, item 7),
  `budgets.rs` `min_stack_bytes_is_enough` (a thread with exactly `min_stack_bytes()` for
  limits of `frame_depth` 10,000, `reentry_depth` 1,000 and `parser_nesting` 1,000 runs a script
  that reaches `reentry_depth` through host functions and parses 1,000 nested brackets at the
  deepest, and ends without a stack overflow), `termination.rs`
  `same_script_twice_gives_the_same_steps_and_value` (the epic, item 13, held here; the
  determinism tests of `story:host-interface-hardening` cite it).
- `crates/swell/tests/tracing.rs`, run also under `collect-every-safe-point` (each fails when the
  trace function or root site it names reports nothing):
  `closure_environment_survives_collect` (an object reachable only through the environment a
  returned closure captured is read after collections; the trace of `env/mod.rs` and of
  `object/ecmascript_function.rs`), `operand_stack_and_local_values_survive_collect` (an object
  held only on the operand stack, as the pending left operand of a call whose argument allocates,
  and one held only in a local slot are read after the collections at the safe points between;
  the root site of `interp/mod.rs`), `global_environment_survives_collect` (with only the
  `Realm` root kept, a collection, then a second script reads a global `var` binding and a
  global function the first declared; `env/global.rs` and the realm record),
  `script_record_survives_collect` (a parsed `Script` evaluated after a collection; `script.rs`).

`Cargo.lock`: the unit regenerates it in its own tree (the two new path dependencies), because
its build needs it, and runs its acceptance with `--locked` against that lock; the coordinator
does not take the unit lock into the wave branch but regenerates it there after the merges of
the wave.

Writes: `crates/swell/Cargo.toml`, `crates/swell/src/env/{mod,declarative,object,global,
function}.rs`, `crates/swell/src/object/ecmascript_function.rs`,
`crates/swell/src/interp/{mod,frame,dispatch,budget}.rs`, `crates/swell/src/script.rs`,
`crates/swell/src/host/{agent,mod,limits}.rs`, `crates/swell/tests/support/script.rs`,
`crates/swell/tests/language.rs`, `crates/swell/tests/language/`,
`crates/swell/tests/{budgets,scripts,termination,interrupt,calls,tracing}.rs`,
`conformance/host/requirements.txt` (the 28 lines), `Cargo.lock` (regenerated in the unit tree;
the wave branch lock is regenerated by the coordinator).

## Acceptance

`cargo test --locked -p swell` and `cargo test --locked -p swell --features
collect-every-safe-point --test language --test scripts --test budgets --test tracing` exit 0 and run the tests
of Requirements and the tests named in Change, and `slot task -d . -t
taskfiles/host-requirements.yml host-requirements-check` exits 0 and prints
`host-requirements: 161 requirement(s), 161 line(s): 55 test, 106 pending, 0 omitted; 50
UNMAPPED marker(s), each naming a HOST-<n> or ESS limit` and `host-requirements: pending by
slice: 1:15 2:5 3:5 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1`.

## Out of scope

The builtins beyond those of `story:object-model-and-realms` (`story:harness-builtins`); fuzzing,
misuse across entry points, fault injection, re-entry through builtins
(`story:host-interface-hardening`); `eval`, `arguments`, `with`, labels and automatic semicolon
insertion (slice 2); `globalThis` (slice 3); jobs and modules (10, 11); the test262 run path
(`story:test262-run-path`). The root `Cargo.toml` and `Taskfile.yml` (the coordinator).

## Domain relations

- Script to Realm, many-to-one, each script references the realm it was parsed in: inferable
  (`spec/domains/host.yaml`, the Script entity, relation `realm`, `references`, `one`, via
  `realm_id`; Table 35 of ECMA-262, `[[Realm]]`). `parse_script` records it, `evaluate_script`
  runs in it.
- Realm to global `this`, one per realm: inferable (`docs/design/host-interface.md`, section 7;
  `HOST-50`).

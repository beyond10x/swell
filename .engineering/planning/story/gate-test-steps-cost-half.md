---
format: aep.planning-md/3
id: story:gate-test-steps-cost-half
kind: story
status: active
title: The gate's test steps cost less CPU time, and what halving them needs
summary: Measured per step, per test in both modes and per planted script; the tests whose count is not their property get gate counts, five identical runs of one fixture tree become one test, the planted scripts run as a rolling pool with the Go runtime's collector off, and the one change that halves the cost, an optimized build of the collect mode, is measured and left to a decision.
relations:
- decomposes: epic:foundation
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T04:00:43Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T04:00:43Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---

## Context

A defect under the organization's rule "Tests stay fast": the test step of a gate ends within
2 minutes of wall time, and a step over the budget is fixed before the next feature; a longer
timeout, a shard or waiting longer is not a fix.

The gate's test steps (`test`, `test-doc`, `test-collect`, `test-faults`, `conform-engine`,
`test262-check`, `test262-moves-check`, `gate-selftest`) cost about 300 CPU seconds, and the
machine they run on is shared: at a load of 100 a process gets a fraction of a core. Four runs of
the gate on the branch of slice 6, 2026-11-22 and 23, every check passed in each:

| tree | load at the end (start) | test steps, wall | CPU |
|---|---|---|---|
| `9373a70` | 36.7 | 89.4 s | 281.2 s |
| `79b98b5` | 43.7 | 139.5 s | 301.3 s |
| `1e46de5` | 54.3 (153) | 263.9 s | 303.2 s |
| `1e46de5` | 77.1 (113) | 327.8 s | 323.6 s |

On `4e71892`, 2026-11-23, `slot --timeout 30m task check-runner` at a load of 75.7 at the start
and 80.1 at the end: `gate-time test-steps 294.5 of 120, cpu 300.7`, `gate-selftest` 155.5 s of
it, every other check passed.

Measured on `4e71892` (each step as `task <step>` in check-runner's environment, user and system
seconds of its process tree; each test its binary run directly with `--exact <name>
--test-threads 1`; each planted script alone), at loads of 35 to 105 on 18 cores:

- per step: `test` 98.1, `test-collect` 172.9 (run beside the per-test timing), `gate-selftest`
  42.3, `test-doc` 6.1, `test262-moves-check` 3.5, `conform-engine` 2.4, `test-faults` 0.4,
  `test262-check` 0.3;
- the 977 tests of the default mode 94.0 in sum (the 40 costliest 32.4, the median 0.022, the
  least 0.001 with the start of the process); the 796 with a collection at every safe point
  154.5 (the 40 costliest 54.5, the median 0.066). In that mode the cost of a test is its
  instructions times the live heap, which every safe point marks: the 14 `language` parts 19.6,
  13.0 of it the 14 `try-completion-*.js` scripts; the 17 URI planes 9.4; tests of a step per
  index or key 1.1 to 1.8 each where the default mode takes 0.02 to 0.04;
- the 65 planted scripts 43.7 in sum, none over 2.2: most of it is go-task starting, several
  times a script (0.15 CPU seconds a start, 0.10 with the Go runtime's collector off);
- the swell-test262 tests `run::eval_script_*`, five tests that each count the same fixture tree
  again, 0.95 each;
- the build of the collect mode's binaries at `opt-level = 1`, in a profile of their own: all 796
  tests pass and cost 19.3 CPU seconds in sum (154.5 unoptimized); the build took 311 s at a load
  of about 60, about 4 minutes more than now.

Halving the cost by the forms the earlier rounds used (a gate count with its full count in the
lane `slow`, the gate's count in the collect mode where the size is not what that mode shows, one
process for many identical ones) is not reachable: the collect mode costs 155 of the 300 seconds,
and its cost is spread over hundreds of tests whose scripts are their claim.

## Change

1. `gate-selftest` runs the scripts as a rolling pool, `xargs -r -P GATE_SELFTEST_JOBS` (4), where
   it started four and waited for the slowest of them before the next four; the order of the
   results and every check of them stay. The scripts run with `GOGC=off GOMEMLIMIT=1GiB`: 36.9
   CPU seconds for the 65 scripts against 43.0, run one after another.
2. The five tests of the eval-script tree (`eval_script_declares_in_the_global_scope`,
   `eval_script_throws_the_first_syntax_error`, `eval_script_answers_the_completion_value`,
   `a_refusal_inside_eval_script_fails_the_run`,
   `the_nesting_limit_inside_eval_script_fails_the_run`) become one test that counts the tree once
   and makes every assertion of the five on that count; a failure names the case by its path.
3. Gate counts, the full count in the lane `slow` (`SWELL_FULL_COUNTS=1`), each named in the
   test's documentation with its measurement and with the assertion that fails on the defect at
   the smaller count:
   - the frontend's ledger over a large source, 1,000 in the gate in either mode (10,000 at the
     full count): `block_functions`, `computed_names_and_object_spread`, `for_of`,
     `array_patterns`, `tagged_templates` (`many_*_are_charged_as_they_are_read`). Each asserts
     that the ledger equals what the lists hold by capacity, which a list grown uncharged breaks
     at any size; 1,000 grows every list past its first capacity many times over;
   - `lexical_declarations::the_per_iteration_record_is_charged`: 500 iterations in the default
     mode's gate (4,000 at the full count); its budget is a reading of the two loops at the same
     count and half their difference, and the assertion of more than 32 bytes a record refuses an
     uncharged record at any count;
   - the walks and searches over 2^32 - 1 and 2^53 - 1 indices end in a step budget of 1,000 in
     the default mode's gate (10,000 at the full count; the collect mode keeps 200 and 1,000):
     each still starts every walk and ends it in `Terminated(StepBudget)`, which a walk that
     charges no step never reaches.
4. The collect mode's counts where the size is not what that mode shows, in the gate and in the
   lane alike, the default mode unchanged: the URI round trips take every sixteenth String of
   1,024 code points of a plane (the default mode keeps every scalar value);
   `each_search_charges_a_step_per_index` 10 and 20 elements (100 and 200),
   `freeze_charges_a_step_per_key` 2 and 20 keys (10 and 100), `apply_charges_a_step_per_element`
   10 and 20 (100 and 200), `eval_added_bindings_are_found_past_the_small_size` 20 bindings (100;
   the small size is 8), `a_site_gives_the_same_template_object_on_each_evaluation` 10 calls
   (100), `each_walk_charges_a_step_per_index` 10 (100). Each difference of steps is exact at any
   count, so a missing charge fails as before.

No test is ignored, no assertion and no budget form is changed, no test leaves the gate, and no
timeout, part count or engine source changes. The count of the step `test` goes from 977 to 973
by the five tests merged into one.

Not changed: `option_env!("SWELL_FULL_COUNTS")` in `crates/swell/src/heap/collect.rs`,
`script.rs` and `ops/objects.rs`. A count read at run time needs `std::env` in an engine crate,
which HOST-2's source scan refuses there, tests included; the cut is not small.

Writes: Taskfile.yml, crates/swell/tests/array_patterns.rs, crates/swell/tests/block_functions.rs,
crates/swell/tests/builtins_array_iteration.rs, crates/swell/tests/builtins_array_search.rs,
crates/swell/tests/builtins_function_source.rs, crates/swell/tests/builtins_object_integrity.rs,
crates/swell/tests/builtins_uri.rs, crates/swell/tests/computed_names_and_object_spread.rs,
crates/swell/tests/for_of.rs, crates/swell/tests/lexical_declarations.rs,
crates/swell/tests/property_index.rs, crates/swell/tests/tagged_templates.rs,
crates/test262/tests/run.rs

## Acceptance

- `slot --timeout 30m task check-runner`, twice: every check passes but the time line where the
  load is above the core count; the counts are those of `4e71892` but `test` (977 to 973, the
  merged tests); the CPU figure of `gate-time test-steps` is read beside the load. Measured
  2026-11-23: 306.3 at a load of 99.8 at the start and 156.4 at the end, 300.3 at 153.0 and 95.2,
  against 300.7 at 75.7 and 80.1 on `4e71892`. On this machine the figure rises with the load by
  more than the savings, so the savings are shown by the tests each alone, below.
- The changed tests, each alone: in sum 8.5 CPU seconds in the default mode's gate (18.6 before),
  4.1 in the collect mode's gate (20.1 before); at the full count (`SWELL_FULL_COUNTS=1`) 14.5 and
  9.8, none over 1.6; the lane's command over them in both modes, all pass.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all --check` and
  `sh taskfiles/test-main.sh check` exit 0.

## Out of scope

- Building the collect mode at `opt-level = 1` in a profile of its own (`[profile.collect]` in
  `.cargo/config.toml`, `--cargo-profile collect` in `build-collect`, `test-collect` and the lane's
  collect half, `--profile collect` for test-doc's collect half): it takes the collect mode from
  about 139 to 19 CPU seconds and the steps to about 150, and adds about 4 minutes to the build
  phase; its stack tests then run on optimized frames, so it is decided on its own, as
  `story:collect-mode-tests-far-from-the-kill` left it.
- `test262-moves-check` (3.5 CPU seconds): its task file is a source of the digest of the expected
  file, so a change of it waits for the full test262 run.
- The `language` scripts, the fuzz parts, the stack derivations and the tests that are over their
  bound and stay so (`determinism::host_133_same_calls_same_hooks_same_outcomes`).
- `AGENTS.md`, `CHANGELOG.md` and the forge's workflows.

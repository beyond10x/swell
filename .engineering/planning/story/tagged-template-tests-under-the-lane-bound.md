---
format: aep.planning-md/3
id: story:tagged-template-tests-under-the-lane-bound
kind: story
status: active
title: Two tagged-template tests stay under the lane's bound in the collect mode
summary: The template-object charge test and the memory-budget test of tagged_templates take their gate count of 100 also in the lane's collect half, where 10,000 evaluations and 1,000 sites took 32 to 43 CPU seconds alone and were killed at 60 s; the default mode keeps its counts.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T01:32:29Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T01:32:29Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---

## Context

The lane `slow` (`task slow`, step `slow-tests`) runs the tests that read `SWELL_FULL_COUNTS=1`
at their full counts in the default mode and with the feature `collect-every-safe-point`. The
rule (`AGENTS.md`, "Commands and the gate"): a test stays under 2.5 CPU seconds alone at its
full count in either mode, under 1.5 at the gate's count in the default mode and under 2 in the
collect mode.

In `crates/swell/tests/tagged_templates.rs`, `gate_count(full, collect)` answers `full` also in
the collect mode when `SWELL_FULL_COUNTS=1`, where every instruction collects. Measured on
2026-11-23, the test binary run directly with `--exact <name> --test-threads 1`, user + system
(two runs on the wave branch, the second at loads of 18 to 38):

| test | collect, full count | collect, gate count | default, full count |
|---|---|---|---|
| `a_template_object_is_charged_once_per_site` (10,000 evaluations; 100 in the collect gate) | 32.4 to 34.3 | 0.31 to 0.35 | 0.29 to 0.46 |
| `many_sites_end_at_the_memory_budget` (1,000 sites; 100 in the collect gate) | 30.9 to 43.4 | 0.94 to 1.43 | 0.36 to 0.59 |

In the lane's collect half at a load of 155 both were killed at 60 s. Every other test of the
file is under 1.4 CPU seconds in all four cells;
`many_tagged_templates_are_charged_as_they_are_read` (parse and compile only) takes 0.36 to
0.68 at its full count in the collect mode.

## Change

The two tests take their count from `mode_count(default, collect)`: 10,000 evaluations and
1,000 sites in the default mode, 100 and 100 with the feature `collect-every-safe-point`, in the
gate and in the lane alike. Each count is stated in the test's documentation with the
measurement and what the smaller count still shows:

- the charge test: one template object is 672 bytes (measured); an evaluation that kept an
  allocation of that size would hold 99 × 672 = 66,528 bytes over one evaluation at 100, 99
  times the assertion's bound (100 evaluations hold 4 bytes over one); garbage is freed at the
  next instruction in that mode at any count.
- the budget test: the budget stays the reading of none (191,253 bytes) plus a room of half what
  the sites add (100 sites add 145,936 bytes, room 72,968); the script of 100 sites ends in
  `Terminated(MemoryBudget)` and the script of none completes under it.

After the change, at loads of 73 to 129: the charge test 0.43 to 0.46 CPU seconds in the
default mode and 0.49 to 0.51 in the collect mode, the budget test 0.57 to 0.69 and 1.39 to
1.44, gate and full count alike.

`gate_count` stays for `many_tagged_templates_are_charged_as_they_are_read`, so the file still
reads `SWELL_FULL_COUNTS` and stays in the lane's filter. No assertion, room or budget rule
changes.

Writes: crates/swell/tests/tagged_templates.rs

## Acceptance

- Every test of `tagged_templates`, its binary run directly with `--exact <name>
  --test-threads 1` under the shell's `times` (user + system), in the default and the collect
  mode, with and without `SWELL_FULL_COUNTS=1`: at most 2.5 CPU seconds at the full count, at
  most 1.5 in the default gate and 2 in the collect gate.
- `SWELL_FULL_COUNTS=1 cargo nextest run --locked -p swell --profile default --test main -E
  'test(/^tagged_templates::/)'` and the same with `--features collect-every-safe-point`, and
  both without `SWELL_FULL_COUNTS`: 15 passed each, none timed out.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0.
- `cargo fmt --all --check`: exit 0.

## Out of scope

- The engine's cost in the collect mode (every safe point marks the whole heap).
- The collect-gate count of `many_tagged_templates_are_charged_as_they_are_read` (1,000, under
  0.1 CPU seconds; its full count 10,000 takes 0.36 to 0.67 in that mode).
- `AGENTS.md`, the task files and `.config/nextest.toml`.

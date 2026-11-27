---
format: aep.planning-md/3
id: story:full-count-tests-fit-the-time-bound-in-the-collect-build
kind: story
status: draft
title: The full counts of six tests fit the time bound in the collect build
summary: Six tests take 2.96 to 9.73 CPU s alone with SWELL_FULL_COUNTS=1 and a collection at every safe point, over the bound of 2.5 s. Backlog, from correction 1 of unit lanetests of wave big-2.
revision: 1
---
## Context

Found by correction 1 of the unit `lanetests` of wave big-2
(`review-result:lanetests-correction-1-coordinator-read`), in tests older than the wave that
the unit did not change. A test stays under 2.5 CPU seconds alone at its full count
(`SWELL_FULL_COUNTS=1`), also with a collection at every safe point. Six tests of
`crates/swell/tests/` take more there, measured alone on one thread in the lane's collect build
(test profile, opt-level 0, feature `collect-every-safe-point`, `SWELL_FULL_COUNTS=1`) on
2026-11-27 on the build node at a load of 2 to 3:

- `builtins_map_and_set::a_set_of_10000_keys_is_charged_and_credited`: 5.25 s;
- `builtins_map_and_set::finished_loops_leave_no_cursor`: 3.41 s;
- `builtins_map_and_set::abandoned_iterators_release_their_cursors_at_collect`: 2.96 s;
- `termination::host_70_agent_stays_alive_after_each_budget`: 3.56 s;
- `determinism::host_25_key_order_does_not_depend_on_hash_key`: 8.79 s;
- `determinism::host_214_two_processes_print_the_same_map_order`: 8.78 to 9.73 s (two
  processes).

The finding is older than the wave and is filed here, in the backlog. No wave takes it before
the planned waves are delivered.

## Change

Each of the six takes a count in the collect build at which it runs under 2.5 CPU seconds
alone with `SWELL_FULL_COUNTS=1` and under 2 s without it, or is split into tests that do. The
default build keeps its counts. Each smaller count is shown red in the collect build with the
defect the test exists for planted in a copy.

## Acceptance

- Each of the six, alone on one thread in the lane's collect build with `SWELL_FULL_COUNTS=1`,
  takes under 2.5 CPU seconds (user plus system), three runs, the figures in the report.
- For each changed count: the planted defect and the red line of the test at that count.
- `slot --timeout 30m task slow-tests` exits 0, and the sum of its test-seconds is not higher
  than before the change.

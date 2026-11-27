---
format: aep.planning-md/3
id: review-result:lanesix-plan-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of the story of the gate''s test steps after the plan critics: ready for its unit'
relations:
- reviews: story:gate-test-steps-hold-the-budget-on-the-slower-runner
revision: 1
---
Coordinator's read of `story:gate-test-steps-hold-the-budget-on-the-slower-runner` after one
read by two plan critics (acceptance, design), 2026-11-27. Verdict: ready for its unit with
its second body.

The operator took the story into wave big-2 as lane unit 6
(`approval-record:wave-big-2-lane-unit-6`) after the forge failed `check` on the wave's first
pull request with the test steps at 147.6 s of 120 on a runner that took 1.94 times the CPU
seconds of the run before it. The coordinator wrote the first body from the two run logs.
Both critics read it on `wave/big-2` at `cae5f1a` and asked for a revision. What each finding
became:

The acceptance critic, fourteen findings:

- The bound on the forge had a branch that the code before the unit passes on a runner 1.5
  times slower. Taken in the critic's form: the test steps read at most 29.5 times the
  yardstick's seconds (62 / 2.1) and at most 120, on the run's first attempt, checked by the
  coordinator on the pull request.
- The runner line was not named, and nothing checked "printed once" or the planted script.
  Taken: the line is `gate-runner: ...`, from the task `slow-runner` with a label, its `echo`
  once, a new script under `tests/gate/`.
- The lists of the measurement had no bullet. Taken.
- The CPU readings on the build node: no pairing, no measure named, no rule for the load.
  Taken with a change: base, head, base, head; a pair counts when its loads differ by at
  most 5; two counting pairs at 0.80 or under; at most two retakes. Three pairs were cut to
  two: each is a whole gate on a node that is loaded.
- The entry of the table of CPU bounds: the first body said "between the highest and the
  lowest reading", and `AGENTS.md` asks for the geometric mean with a margin of 1.25 on each
  side, which needs a cut to 64 %. Decided: the acceptance stays at 80 %, which is what the
  budget needs; the entry follows `AGENTS.md`, and where a margin fails no entry is added and
  the report says so. The local gate then does not see the forge's wall time; that is said
  in the story and is the coordinator's to file after the unit.
- The build node's core count is 64 in one paragraph of `AGENTS.md` and 32 in its lines of
  the gate's numbers. Taken: the unit reports `nproc` of a pool job; the paragraph is the
  coordinator's to correct.
- "Unchanged but for a line" for the bounds and the two scripts. Taken in the critic's form.
- "The count of assertions" has no tool. Taken: the `passed` and `ignored` counts per step,
  each difference named test by test with its cases.
- What a cut may not be had no check. Taken: the steps of `check` between the two marks, the
  workflow file unchanged.
- "Each names its time" had no measure. Taken: the `gate-slowest` lines.
- The lane's bullet joined two statements and read a time the task does not print. Taken
  with a change: the unit shows equal counts of `task slow-tests` on base and head; the
  time is read by the coordinator from the lane's `slow-time slow-tests` line on the forge
  (at most 175 s; 168.8 and 167.6 before).
- The `desc` of `slow-runner` would go stale. Taken.
- The rule "Tests stay fast" is not in the tree's `AGENTS.md`. Taken: the story names the
  organization's rule in words and no file outside the repository.

The design critic, six groups:

- Files this unit and the open units would both change. Taken: a cut stays inside a module
  (the generated `tests/main.rs` does not change), a test named in the host requirements or
  in `STACK_TESTS` keeps its path, the files of the two open units are listed and left
  alone, a cut under `crates/*/src` is named for the digest of the test262 sources, and in
  `Taskfile.yml` the unit changes `check` and `slow-runner` only. `story:slice-08-conformance-close`
  gets an edge `depends_on` to this story.
- The tree has a mechanism for a smaller count in the gate (`SWELL_FULL_COUNTS`), and a
  count that reads it runs full in the lane `slow`, which has no room. Decided: a sweep is
  changed by a stride or by stepping over budgets of one charge, reads no such variable and
  keeps its range; a sweep that would give up range is not changed.
- The entry of the table: as above.
- "A test that runs in both steps runs in one of them" would narrow what the collect step
  covers (a missing root, a second frame size). Decided: cut from the story; no module
  leaves the collect step, and the story lists it as out of scope and the operator's.
- One unit or two: one, with its order written: the runner line first, the lists on the
  base, the entry last and conditional.
- Base and head: the commit of the wave the branch is cut from and the unit's last commit
  before it merges a later wave, neither with the two open units.

Not taken: nothing. Open after the unit: a change that makes the local gate fail where the
forge's wall time would, when no entry of the table can be set.

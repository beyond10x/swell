---
format: aep.planning-md/3
id: story:gate-cpu-table-held-by-a-check
kind: story
status: draft
title: 'The gate''s table of CPU seconds is held by a check: each entry, its measured pair and the path from the table to the clock'
summary: A committed file of measured pairs and an awk check recompute the rule of 1.25 for every entry of CPU_BOUNDS_BY_CORES; a planted case holds the path of the table from check-runner to gate-time-sum; the pair of 16:245 is recorded on the merged tree; one variable names the collect profile.
relations:
- decomposes: epic:foundation
- depends_on: story:gate-test-steps-fit-four-cores
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: taskfiles/collect-mode.yml
- confidence: cited
  path: taskfiles/forge.yml
- confidence: cited
  path: taskfiles/gate-time.yml
- confidence: cited
  path: tests/gate
revision: 6
---
## Context

`story:gate-test-steps-fit-four-cores` gave check-runner a table of CPU seconds per kind of
machine, `CPU_BOUNDS_BY_CORES` of `taskfiles/forge.yml`, keyed by the core count the gate
prints. An entry is added or changed only together with a measured pair (the steps before the
change over the entry, the change under it, each a whole check-runner job) and a factor of 1.25
on both sides. That story names three things no program holds:

- the value of an entry, its pair and the rule: an entry changed to a number far above its pair
  leaves every planted script green;
- the path of the table from `taskfiles/forge.yml` to `gate-time-sum`: a slip in the line that
  passes `GATE_CPU_BOUNDS` drops the table, and the gate then prints its CPU seconds without a
  bound and decides nothing by them;
- the name of the collect profile, written in `taskfiles/collect-mode.yml` and `Taskfile.yml`
  where a path or `--profile` needs it, beside the one flag `COLLECT_CONFIG`.

The entry `16:245` is provisional until a pair under the merged tree is recorded on a 16-core
machine.

## Change

- A committed file of pairs, one line per entry: the core count, the entry, the lowest CPU
  seconds of the steps before the change, the highest of the change, and the commit each was
  measured on. A check of the gate, in awk, recomputes the rule for every entry of the table
  (the highest change times 1.25 at most the entry; the entry times 1.25 at most the lowest
  base) and fails on an entry without a line and on a line without an entry.
- The first two check-runner jobs on a 16-core machine after the merge of
  `story:gate-test-steps-fit-four-cores` give the pair of `16:245` under the merged tree; the
  entry is confirmed or moved by the rule in the commit that records them.
- A planted case reads the value `check` receives from check-runner (`gate-vars` in a dry run)
  and fails when it is not the table of `taskfiles/forge.yml`.
- One variable for the name of the collect profile, used by every place that writes it.

Failing cases first: each planted script is red before its check exists.

## Acceptance

1. With an entry of the table raised above its recorded base, the gate's check prints the entry
   and the rule it breaks, and exits non-zero; with the table as committed it exits 0.
2. With the line that passes `GATE_CPU_BOUNDS` removed from check-runner, the planted case
   fails and names the variable.
3. The pairs file holds the pair of `16:245` measured on the merged tree, two whole
   check-runner jobs, with their `gate-cpu` lines in the commit that records them.
4. `gate-selftest` grows by at most 0.5 CPU seconds, measured with the shell's `times` over
   three runs of the new scripts.

A test is terminated at 60 s and the gate's test steps fail over 120 s; the new checks run
no build and no test.

## Out of scope

An entry for 4 or 64 cores: the first has two speeds of one core count, the second no value
that keeps the factor on both sides.

---
format: aep.planning-md/3
id: review-result:realmwalk-correction-1-coordinator-read
kind: review-result
status: active
title: Coordinator's read of correction 1 of the unit realmwalk
relations:
- reviews: story:realm-walk-in-parts-under-the-cpu-bound
revision: 1
---
Coordinator's read of correction 1 of the unit realmwalk (impl/realmwalk, 542f9fe to
fa63ea9), 2026-11-27. Verdict: no defect.

Correction 1 is one commit (`d8fe96e`: `Taskfile.yml`, `taskfiles/realm-places.yml` and
`crates/swell/tests/builtins_symbol_values.rs`, 352 lines added, 152 removed; no engine code)
and the merge `fa63ea9` of `wave/big-2`. It answers
`review-result:adversary-realmwalk-pass-1`, whose pass was green with one warning and twelve
notes. Read: the whole diff of the two task files, the diff of the test file without its
comments, and the report's plant lines. A second adversary pass is not made: no engine code
changed.

- Finding 1 (warning: `TEST_RUSTFLAGS` and `TEST_NEXTEST_ARGS` were not in the gate's list of
  variables, so a value from the command line changed what the step `test` ran, unseen).
  Both names are in `GATE_VARIABLES`. A new internal task `test-vars-check` is the first
  command of `build`, `build-tests`, `test`, `test-doc` and the writer: it fails the step
  when either value is empty or differs from its line in `taskfiles/realm-places.yml`. The
  unit planted 12 values on the command line and 8 exported with the file moved away: each
  exit 201 with the line that names the step. With the file present an exported value does
  not reach the steps (go-task takes the file's), which `gate-vars` shows.
- Findings 2, 3 and 5 (a part cut to its first place, a part that calls less, an empty
  part). `walk_part` takes from the file what the part owes before it walks (its places,
  their calls, the `Always` rows of `WALK_EXPECTED` among them) and compares after; an empty
  part and a part under half the mean are refused. Each planted change is red with the
  count it found against the count the file gives.
- Finding 6 (nine parts). `MOST_PARTS` = 8, checked by the cover test; nine parts are red
  with the line that names the story.
- Finding 7 (the text of the two comment lines). One definition, `REALM_PLACES_HEADER`,
  written by the test's writing mode and compared by the equality test; the task no longer
  echoes the lines.
- Finding 8 (a writer that writes a place its parser cannot read). Before it writes, the
  test reads every place back through `parse_place` and `resolve_place` and fails naming
  the place; two planted intrinsics (a name with a dot, a name with a space) are red, and
  the fixture stayed equal to the head.
- Finding 12 (literal copies of the flags in other steps). `build` and `test-doc` read
  `TEST_RUSTFLAGS` now. The lane's steps and the steps with a manifest or a target of their
  own keep their literal; the unit lists each line, and `AGENTS.md` names them.
- The adversary's three cases are in the file: the partition against an FNV computed apart
  (`[181, 163]`), the limit of parts in the cover test, the comment lines in the equality
  test.

Not changed, with the reason: finding 4 (the line-by-line comparison is the guard against an
alias; the mutant removes that comparison itself); finding 11 (the parts run in a realm
each, which the story states). Findings 9 and 10 are older limits of the walk (a builtin
that answers where a TypeError is due; 10 functions of hidden intrinsics the walk does not
reach, counted by a probe) and finding 13 is an older limit of the gate (an exported
`RUSTFLAGS` wins over a step's `env:`): each is filed as a draft for the backlog.

The whole gate ran on the unit's head (`task check`), as the story's acceptance 9 asks. Every
step to `conform-engine` passed (19 test binaries, 2305 passed; `test-collect` 1818;
`test-faults` 1; `conform-engine` 16; the step `test` took 11.6 s). The step
`test262-check` was red: the sources digest of the expected file differs from the
committed one. That is the state of `wave/big-2` and not the unit's: 21 files under
`crates/*/src` changed on the wave branch since the expected file was written, the unit
changes no file the digest covers, and the wave's close writes both files again. The two
steps after it ran apart on the same head: `test262-moves-check` exit 0, `gate-selftest`
99 passed, 0 failed. The lane's `slow-tests` was run by the unit before the correction,
not after it; the lane runs on the merged branch before the wave's pull request.

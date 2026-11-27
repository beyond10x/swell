---
format: aep.planning-md/3
id: story:lane-tasks-say-why-they-fail-without-support-files
kind: story
status: draft
title: The lane's tasks say why they fail when the stack rule has no support file
summary: 'slow-tests and test-realm-sizes end without a line of their own when crates/swell/tests/support/ holds no .rs file: the loop''s body ends in a false test. Backlog, from pass 2 of the adversary on unit lanesix of wave big-2.'
revision: 1
---
## Context

Found by pass 2 of the adversary on the unit `lanesix` of wave big-2
(`review-result:adversary-lanesix-pass-2`, finding 4; read in
`review-result:lanesix-correction-2-coordinator-read`). It is older than the unit.

Two tasks of the lane `slow` in `Taskfile.yml`, `slow-tests` and `test-realm-sizes`, gather
the files the stack rule reads with a loop whose body ends in a test:

```
for f in crates/swell/tests/support/*.rs; do [ -f "$f" ] && set -- "$@" "$f"; done
```

When the glob matches no file, the last command of the loop is false, and the shell that
go-task runs its commands in ends the command there with exit status 1 and no line. The
workstation's `sh -e` does not end there, so the form passes a reading by hand. The unit
`lanesix` had copied the form into the gate's task `test-split`; there `task test` exited
201 and printed only `task: Failed to run task "test": task: Failed to run task
"test-split": exit status 1`. Correction 2 of the unit gave `test-split` an `if ... fi`
and a line of its own, with the planted script
`tests/gate/test-says-why-it-fails-when-the-stack-rule-has-no-support-file.sh`. The two
tasks of the lane were not the unit's to change and keep the form.

The form holds today because `crates/swell/tests/support/` holds `.rs` files. A red lane
with no line of its own would be read as a failure of the runner.

The finding is older than the wave and is filed here, in the backlog. No wave takes this
story before the planned waves are delivered.

## Change

To be written when the story is planned. The direction: both tasks gather the files with
`if ... fi` and fail with one line that names the step and the missing files, as
`test-split` does; every other loop of `Taskfile.yml` and `taskfiles/` whose body ends in
a test that can be false is read and corrected; a check of the gate (in the form of
`kill-check`) refuses the form in a task's command.

## Acceptance

To be written when the story is planned: with no file under `crates/swell/tests/support/`
(planted in a copy of the tree, nothing built) `task slow-tests` and `task
test-realm-sizes` each fail with a line that starts with the step's name; a planted script
under `tests/gate/` holds each; the check refuses a planted loop of the old form.

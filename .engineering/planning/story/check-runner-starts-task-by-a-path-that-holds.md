---
format: aep.planning-md/3
id: story:check-runner-starts-task-by-a-path-that-holds
kind: story
status: draft
title: check-runner starts task by a path that holds from the Taskfile's directory
summary: check-runner starts TASK_EXE in the directory of Taskfile.yml, so a relative path of task from another directory is not found. Backlog, from the corrections of unit lanesix of wave big-2.
revision: 1
---
## Context

Found by pass 1 of the adversary on the unit `lanesix` of wave big-2
(`review-result:adversary-lanesix-pass-1`, finding 2) in the unit's task `test-sum`, and
by correction 1 of the unit at an older place of the same kind
(`review-result:lanesix-correction-2-coordinator-read`). The older place is this story's.

The task `check-runner` of `taskfiles/forge.yml` starts the gate as a second process:

```
env <the runner's environment> {{shellQuote .TASK_EXE}} check GATE_CPU_BOUNDS=...
```

`TASK_EXE` is the path of the executable as the caller typed it, and go-task runs the
command in the directory of `Taskfile.yml`. Called by a relative path from another
directory (`./bin/task -d <tree> check-runner`), the second process is looked for under
the tree and is not found: `stat <tree>/bin/task: no such file or directory`. Calls
through `PATH` (the pool's `slot task check-runner`, the forge's workflow) work, so no
run of today shows it.

The unit's task `test-sum` had the same defect and was corrected: a path that names a
directory and is not absolute is taken from `USER_WORKING_DIR`
(`tests/gate/test-sums-its-runs-when-task-is-called-by-a-relative-path.sh` holds it).

The finding is older than the wave and is filed here, in the backlog. No wave takes this
story before the planned waves are delivered.

## Change

To be written when the story is planned. The direction: one place makes the path of the
executable absolute (a variable beside `TASK_EXE`, or an internal task), and `check-runner`,
`test-sum` and every other command of `Taskfile.yml` and `taskfiles/` that starts
`TASK_EXE` use it.

## Acceptance

To be written when the story is planned: `./rel/task -d <tree> check-runner -- --dry` from
another directory prints the same lines as `task check-runner -- --dry` in the tree; a
planted script under `tests/gate/` holds it for a relative path with and without a leading
`./`; `git grep -n TASK_EXE` shows no start of the executable outside the one place.

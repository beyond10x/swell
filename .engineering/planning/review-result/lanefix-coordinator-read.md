---
format: aep.planning-md/3
id: review-result:lanefix-coordinator-read
kind: review-result
status: active
title: 'The coordinator''s read of the unit lanefix: the lane''s kills under the forge''s sh'
relations:
- reviews: story:realm-size-lane-within-its-share
revision: 1
---
Coordinator's read of the unit lanefix of wave big-2 (impl/lanefix, a114833 to e3c97c4),
2026-11-27. It corrects `story:realm-size-lane-within-its-share` after the story was
implemented. Verdict: a defect of the delivered lane, fixed; one part of the fix is not
verified where the unit ran.

The defect. The forge's run of `task check` on the pull request's head `d16fe575` (run
45967542103) ended red after three green runs of the same gate on the build node:
`gate-selftest: 103 passed, 1 failed`, the planted script
`slow-collect-builds-beside-the-lane-and-ends-with-it` with "task slow ended without
slow-collect-stop ending the build", and the job's cleanup ended orphan processes `cargo`
and `sleep`. The forge's `sh` is dash; the build node's is bash. Every one of the lane's 17
kills of a process group stood inside a `sh -c` text or a script run with `sh`, written as
the signal, two dashes and the group. dash's builtin `kill` takes the word after the signal
as the first process and reads the two dashes as the process number "-" ("Illegal number"),
so it signals nothing; bash skips the two dashes. On the forge `slow-collect-stop` left at
its first `kill -0`, the watcher of `slow-collect-start` failed to end its group, and the
test's wait for the group read a failed `kill -0` as "gone".

Read: the diff of `a114833..e3c97c4` (5 files, 230 lines added, 19 removed) and the unit's
report.

- The 17 calls (5 in `Taskfile.yml`, 8 and 4 in the two slow-collect scripts) are written
  with the group right after the signal, the form both shells read. `grep -rn 'kill .*-- '`
  over `Taskfile.yml`, `taskfiles/`, `tests/gate/*.sh` and `.github/` finds no call left but
  the text of the new check and its planted script.
- `kill-check` (`taskfiles/shell.yml`, in `task check` after `python-check`) refuses a
  `kill` whose signal operand is followed by two dashes in `Taskfile.yml`, `taskfiles/`,
  `tests/gate/` and `.github/workflows/`, names file and line, and fails on a tree with no
  file to read. `kill -s <signal>` with two dashes, a `kill` with no signal before them and
  `pkill` pass. Its planted script plants the form in each of the four places and reads the
  four lines; with one old line put back in `slow-collect-stop` the check printed
  `Taskfile.yml:316: kill -TERM -- "-$pid" 2>/dev/null` and exited 1.
- The two test scripts ask through `group_alive`, which reads the answer of `kill -0`: "No
  such process" is a group that is gone, an empty or non-numeric group and any other error
  fail the test with their own message. The failure of the forge's run would now stop the
  test at its first wait, with dash's message in the line.
- The lane prints one line `slow-runner: <n> cores, <cpu>, <memory>, sh is <sh>` after its
  clock starts (the coordinator's addition, after the forge ran the lane on one code in
  882.6 s and 1,142.0 s); case 3a of the test reads the line, red with the call removed.
- Nothing else that bash reads and dash does not was found in the lane's text and the two
  scripts. `.github/workflows/release-build.yml` uses a bash substring, in steps that set
  `shell: bash`.

The unit's runs on `e3c97c4`, build node: `task check-runner` exit 0 with `gate-selftest:
105 passed, 0 failed`, `kill-check: no kill with two dashes after its signal operand in 141
file(s)` and `gate-time test-steps 35.3 of 120`; each of the three scripts alone, `ok`.

Not verified: no dash is installed on the workstation, and the unit did not probe the build
node for one. Nothing ran under dash. That dash reads `kill -TERM "-$pid"` as a group comes
from dash's source and from the forge's log; the forge's run of `task check` on the head
that holds this unit is the check, and the pull request is not handed over before it is
green.

The local gate now refuses the form that failed (the rule of the organization: when the
forge goes red after a green local gate, the fix includes the change that makes the local
gate catch the same failure). It does not run the gate's shell under dash; a second
construct that only dash refuses would again be seen first on the forge.

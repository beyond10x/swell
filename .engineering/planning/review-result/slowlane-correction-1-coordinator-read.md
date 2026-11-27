---
format: aep.planning-md/3
id: review-result:slowlane-correction-1-coordinator-read
kind: review-result
status: active
title: Coordinator's read of correction 1 of the unit slowlane
relations:
- reviews: story:realm-size-lane-within-its-share
revision: 1
---
Coordinator's read of correction 1 of the unit slowlane (impl/slowlane, 355c789b to 949e906),
2026-11-26. Verdict: no defect.

The correction answers review-result:adversary-slowlane-pass-1 (two red gate selftests, three
findings). It is two commits, 7 files, 364 lines added, 11 removed. Finding 3 (the collect
mode at the sizes between the ends) is not answered here: the coordinator decided that the
collect mode runs at every size again, and the second unit of the story (impl/lanebuilds)
makes that change with the cost of the one test that made it expensive.

Commit 527ef00:

- Finding 1 (`Taskfile.yml`, task `test-realm-sizes`): the tests the task does not run no
  longer reach nextest as one `-E` argument (75,099 bytes for 996 tests; one argument holds
  at most 131,072 bytes). The task writes `.out/realm-sizes-nextest.toml`, a copy of
  `.config/nextest.toml` with one line `default-filter = 'not (binary_id(swell::main) &
  (test(=...) | ...))'` under `[profile.default]`, and each run passes it with
  `--config-file` and keeps `--profile default -E binary_id(swell::main)`. Read: the awk
  program prints every line of the configuration and adds the filter after the line
  `[profile.default]` (or makes that table when there is no configuration file); the task
  then counts the `default-filter` lines and the `test(=` terms of the generated file and
  fails before any run unless there is one line that names every test printed "other"; an
  empty list and a file it cannot write fail with a line that names the file. A test name is
  a Rust path, so it holds no quote that would end the TOML string. The correction tried
  `--tool-config-file` first on nextest 0.9.143: its `default-filter` is ignored there, so
  the file is a whole configuration. `cargo nextest list` with the old argument and with the
  file gives the same 312 tests in both modes, and the nine runs of the task give the same
  counts on 355c789b and on the new head (312 passed and 997 skipped in each; 311 and one
  failed at 128 for the proxy test another unit corrects). The adversary's selftest is green
  as written and has three more steps: 20,000 tests (the longest argument the stub cargo
  saw: 177 bytes; the environment 5,705 bytes at 900, 2,000 and 20,000 tests), a
  configuration it cannot write, an empty list.
- Finding 2 (`taskfiles/gate-time.yml`, task `slow-time-sum`): the sum of the timed parts
  stays, and the task reads the lane's clock too (`.out/gate-time/start`, which
  `gate-time-start` writes as the first step of `task slow`), prints `slow-time clock <s> of
  900, ...` and fails a lane whose clock is over the bound, naming the clock's seconds, the
  sum and the seconds in no part's line. It fails as well without a clock, with a clock
  that holds no time, and with a clock another task process started (the PID in
  `.out/gate-time/cpu`), so the lines and the clock an earlier run left do not pass a lane.
  Read: a part run on its own never runs `slow-time-sum`, so no clock fails it; the lane's
  sum on a host now needs `task slow` in one task process, which is how the forge runs it.
  The adversary's selftest is green as written; the unit's two selftests of the bound plant
  a clock started now (two lines each).

Commit 949e906: the selftest `slow-time-refuses-a-clock-of-another-run.sh`. Its step 0 runs a
planted lane (`gate-time-start`, one timed part, `slow-time-sum`) in one task process, which
passes and prints its clock; `slow-time-sum` called after it on its own is refused and names
the task process of that lane. Steps 1 to 4: a clock started now, a clock of the task process
0, no clock, a clock that holds "x". In copies with one refusal removed each (the clock's
bound; the clock it cannot take; a PID check that refuses every clock) the selftest of that
refusal prints FAIL.

On the new head the implementor ran `task gate-selftest` (91 passed, 2 failed: the two
`site-data-check` selftests, red on the base for a stale data file), `task test-realm-sizes`
(1 run red, the proxy test at 128, red on the base; the runs sum to 88.4 s beside 88.0 s
before), and the four parts of the lane in one job and one task process on the build node:
`slow-time lane 699.3 of 900, longest part stack-profiles 293.4 of 600`, `slow-time clock
699.4 of 900`, exit 0 (slow-test262 271.9, slow-tests 52.4, test-realm-sizes 81.6,
stack-profiles 293.4).

Left open, named by the correction: `gate-time-sum` (the gate's 120 s budget) adds up only
its own time lines in the same way; no step of `check` is outside them today. The summary of
`task slow` (`Taskfile.yml` 173 to 178) does not name the clock yet; the second unit of the
story or the coordinator writes it. Text for `AGENTS.md` and `CHANGELOG.md` is in the
correction's notes.

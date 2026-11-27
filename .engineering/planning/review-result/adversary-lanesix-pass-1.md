---
format: aep.planning-md/3
id: review-result:adversary-lanesix-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit lanesix
relations:
- reviews: story:gate-test-steps-hold-the-budget-on-the-slower-runner
revision: 1
---
Adversary pass 1 on the unit lanesix of wave big-2, impl/lanesix at 808f926b, 2026-11-27.
Red: the split holds (every test runs once, the profile's three checks give the expected results), and three defects have a red planted script each: a caller's value of TEST_OPT1_ARGS or NO_STACK_TESTS is accepted, test-sum fails when task is called by a relative path, and nothing keeps the profile's test in the run at opt-level 1; five notes.

unit: lanesix
verdict: red
cases: executed 107→110 (`gate-selftest`), red 3
origin: introduced 8, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/lanesix/adversary-1/
needs-coordinator: no

**Result:** the split itself holds: every test runs once and the profile's three checks give the expected results. Three defects break it, each with a red planted script. Five more are notes. Goal story moved: none (adversary pass).

## Findings

1. **A value of `TEST_OPT1_ARGS` or `NO_STACK_TESTS` given on the command line is accepted** (`Taskfile.yml:962`). `test-vars-check` holds only `TEST_RUSTFLAGS` and `TEST_NEXTEST_ARGS`.
   - Measured: `task test TEST_OPT1_ARGS='--locked -p swell --test main --profile default'` exits 0. The stub logs `run profile test guard 1`, so the run "at opt-level 1" runs at opt-level 0, and the step still prints "at opt-level 1".
   - Measured: `task test NO_STACK_TESTS=SWELL_NO_GUARD` exits 0, and the run at opt-level 1 runs without the guard.
   - `gate-vars` only prints both values. A value exported in the environment is ignored, because the variables at the top of `Taskfile.yml` win.
   - Reached by: the caller's command line (`task check NAME=...`). Introduced.
2. **The step goes red when `task` is called by a relative path from another directory** (`Taskfile.yml:1271`). The step's line is `test: no line of counts in .out/test-opt1.report` after `stat .../run/rel/task: no such file or directory`, although every test passed.
   - `"{{.TASK_EXE}}"` is the path as the caller typed it, but the new process starts in the directory of `Taskfile.yml`. Before the split, `nextest-report` was called as a task and needed no path.
   - Reached by: `./bin/task -d swell check`. Calls through `PATH` (`slot task check`, CI) work. Introduced.
3. **Nothing keeps the profile's test in the run at opt-level 1** (`Taskfile.yml:1247`, `realm_padding.rs:116`). With the test made "stack" (its text names `stack_with_room(`), `task test` exits 0 and the test sits in the list of the run at opt-level 0.
   - There, nothing holds the operator's condition (debug assertions and overflow checks on in the run at opt-level 1).
   - The same happens when the test moves into a nested `mod`: the rule prints `realm_padding::<test>`, the binary holds `realm_padding::inner::<test>`, and the exact filter misses. It also happens when a macro writes the test.
   - The answer to item 3 of the brief: a move into a file whose other tests are "stack" does not move it, since the rule reads each function. Introduced.
4. **The profile's test sees only its own crate** (`realm_padding.rs:116`; reasoned, not built). `cfg!(debug_assertions)` and the checked `i32` addition belong to the test crate and the `swell` package.
   - A per-package override for the other workspace crates the binary links goes unseen: `[profile.collect.package.swell-syntax] debug-assertions = false` (or `swell-compiler`, `swell-regexp`, `swell-unicode`).
   - Such a line in `.cargo/config.toml` or `Cargo.toml` is not refused; `build-collect` refuses lines only in `.cargo/collect-profile.toml`. Introduced.
5. **Mutant m1 survives the unit's script** (`Taskfile.yml:1271`). Mutant: `|| { cat .out/test-opt1.report; ok=0; }` becomes `|| cat .out/test-opt1.report`. A failing test in the run at opt-level 1 then leaves the step green: the awk in `test-sum` never looks at `failed`.
   - The code is correct. Case text: stub `STUB_OPT1=fail` prints `2 tests run: 1 passed, 1 failed` and exits 100; expect `task test` non-zero and `2 test binaries: 3 passed, 1 failed, 1 ignored`. On the real code this gives exit 201 and that line.
6. **Mutant m2 survives the unit's script** (`Taskfile.yml:1280`). Mutant: `ignored[2] - passed[1] - failed[1]` becomes `ignored[2] - passed[1]`. The script has no failing run, so it cannot see the difference.
   - The code is correct. Case text: the same `STUB_OPT1=fail` stub; expect `1 ignored`. The mutant prints 2.
7. **Mutant m3 survives the unit's script** (`Taskfile.yml:1230`). Mutant: the `compile-check` of `.out/test-list-opt1.txt.log` is deleted.
   - The code is correct. Case text: the stub's list at opt-level 1 writes `Compiling swell v0.1.0` to stderr; expect a red step with `test: compiled 1 crate(s) inside the timed step ... (.out/test-list-opt1.txt.log)`. That is what the real code prints.
8. **The summed line undercounts when the run at opt-level 0 ends by a signal** (`Taskfile.yml:1280`). Stub `kill -9` gives `2 test binaries: 2 passed, 0 failed, 1 ignored`, but 3 tests did not run.
   - The step is still red, and it names `read no "Summary [" line in .out/test.log`. Introduced.

## Checked, no finding
- **Item 1, every test runs once.**
  - The filter is an exact `test(=...)` match inside `binary_id(swell::main)`, so a name that is a prefix or suffix of another cannot match it. A test of another binary with the same name stays in the run at opt-level 0.
  - `faults::host_71_injected_fault_at_each_call_faults_the_agent` is the one name the rule prints "other" that the binary does not hold (1648 "other", 1647 in the list).
  - Plants read by the rule only (not built):
    - In a new file: "other" and "stack" are printed correctly.
    - In a nested `mod`: printed under a name the binary does not hold, so the test lands at opt-level 0.
    - Written by a macro, or a `#[test] fn` on one line: not printed, so it lands at opt-level 0.
    - Every misread lands at opt-level 0, where the test runs once and the step stays green.
  - A list that fails, prints nothing, or prints nothing for one run makes the step red (stub cases).
- **Item 2, red runs.**
  - Failure in either run or in both: the second run still runs, the exit is 201, both Summary lines are printed, and `i` is 1.
  - A signal, a nextest timeout, or a `Compiling` line in either run log or in a list log: red, naming the right log.
  - The slowest test of the first run is ranked first among the ten.
  - After the step, `target/nextest/default/junit.xml` holds the run at opt-level 0, and no later task reads it.
  - `check` stops at a red `test` before its `gate-time` line, as on the base.
- **Item 3, the profile.** The acceptance's three runs are quoted under "Commands". The overflow happens at run time (`Ok(-2147483648)` when checks are off), so I did not plant `wrapping_add`. An exported override or `RUSTFLAGS` change of the whole `collect` profile is caught by the test while it is in the run.
- **Item 4, the guard.**
  - A stack test in the run at opt-level 1 panics at `host.rs:265:9` with the reworded line. The lane reads the same constant, and no other file holds the old text.
  - An exported `SWELL_NO_STACK_TESTS` reaches the run at opt-level 0 too (stub: `guard 1`), as on the base.
  - 64 tests the rule prints "other" assert a depth, a re-entry limit, a parser nesting or a `size_of`. All of them count depth with counters (`frame_depth`, `reentry_depth`, `parser_nesting`), and type layout does not depend on the opt-level. At opt-level 1 frames are smaller, so a test that passes at opt-level 0 cannot overflow there.
  - No test outside `support/` calls `stack_size(`.
  - Every entry of the story's "Not changed" list has an empty diff from `f93ab563` to `808f926b`.
- **Item 5, the build before the clock.** A second `--no-run`, with the guard variable and `--config-file`, compiled nothing (quoted under "Commands"). `TEST_OPT1_ARGS` is in `GATE_VARIABLES`; its override is finding 1.
- **Item 6, the measurement.** I recomputed every reading from the unit's logs: 0.758, 0.782 and 0.53; load differences 20.64, 1.79 and 3.21. The order of the runs was kept. Item 4 adds no entry, and both of its margins fail (394.9 > 315; 393.75 > 314.1).
  - The retaken head 1 has `test-collect` at 52.1 s of wall time with no change to that step, so its 136.7 s is the node's load, not the split.
- **Item 7, the runner line.** The `echo` exists once (`Taskfile.yml:281`). The cores come from `getconf` and then `/proc/cpuinfo`, not from `nproc`. On arm, with no `model name`, the line reads `cpu not read` and does not fail.
- **Item 8, shell.** `kill-check` passes, and all three new scripts run the same under dash. Task commands run in go-task's own shell. Test names flow only through awk and a TOML literal. Files are written only under `.out/` and `target/`.

## Cases added (uncommitted, in the worktree)
- `tests/gate/test-refuses-a-caller-value-of-its-run-at-opt-level-1.sh`: finding 1; 0.73 s run alone.
- `tests/gate/test-sums-its-runs-when-task-is-called-by-a-relative-path.sh`: finding 2; 0.20 s run alone.
- `tests/gate/test-keeps-the-profile-test-in-its-run-at-opt-level-1.sh`: finding 3; 0.75 s run alone.

## Commands and results
- The three scripts above, plus the unit's `test-runs-each-test-once-at-one-opt-level.sh` (one `slot sh -c` job, timed inside it): exits 1, 1, 1 and 0 (`ok`, 0.54 s). The same three under dash: exit 1 each.
- `slot task kill-check`: `kill-check: no kill with two dashes after its signal operand in 148 file(s)`.
- `slot task gate-selftest`: exit 201, `gate-selftest: 107 passed, 3 failed` (the three cases above).
- `cargo nextest run` with `TEST_OPT1_ARGS` and `RUSTFLAGS='-D warnings'`, one pool job at a time:
  - `--no-run`: Compiling 1 (swell; the node's mirror was stale), `Finished` in 5m 07s.
  - The same again with `SWELL_NO_STACK_TESTS=1` and `--config-file`: Compiling 0, `Finished` in 0.01s.
  - `-E 'test(=realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on)'`: exit 0, `1 test run: 1 passed, 1770 skipped`.
  - The same with `--config 'profile.collect.debug-assertions=false'`: exit 100, `debug assertions are off in the profile this test binary was built in`.
  - The same with `--config 'profile.collect.overflow-checks=false'`: exit 100, `an i32 addition that overflows did not panic: overflow checks are off in the profile this test binary was built in (it gave Ok(-2147483648))`.
  - `SWELL_NO_STACK_TESTS=1 ... -E 'test(=budgets::min_stack_bytes_is_enough)'`: exit 100, `panicked at crates/swell/tests/support/host.rs:265:9: this test runs on a bounded stack, but SWELL_NO_STACK_TESTS is set: it runs at opt-level 1, in the gate's run of swell::main (task test) or in the lane's run of the collect mode (task slow-tests), ...`.
  - `--no-run` once more at the end: Compiling 0.
- No pool (they build nothing), all with a stub cargo or awk only:
  - `explore.sh`: 15 scenarios of the step.
  - `mutants.sh`: mutants m1, m2 and m3 against the unit's script; each prints `ok`.
  - `rule-plants.sh`: the rule on a new file, a nested `mod` and a macro.
  - `depth.sh`: the 64 depth tests the rule prints "other".
  - `vars.sh`: `gate-vars` and `test-vars-check` with overrides; `test-vars-check` exits 0.

## Not attacked
- **Whole gates:** none run. My three red cases make `gate-selftest` red; I recomputed the unit's six readings instead.
- **Real builds of planted tests:** none. The nested `mod`, the macro and the new file were read by the rule only.
- **Package override (finding 4):** not built.
- **Also not run:** the lane `slow-tests`, the forge's acceptance, and `test262`.

```findings
- file: Taskfile.yml
  line: 962
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: A command-line value of TEST_OPT1_ARGS (dropping the profile collect) or NO_STACK_TESTS (naming another variable) is accepted, so task test runs its opt-level 1 run at opt-level 0 or without the guard and exits 0, while test-vars-check refuses the same for TEST_RUSTFLAGS and TEST_NEXTEST_ARGS.
- file: Taskfile.yml
  line: 1271
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: test-sum starts nextest-report through the TASK_EXE path in the Taskfile directory, so a task called by a relative path from another directory makes the step red with no line of counts although every test passed.
- file: Taskfile.yml
  line: 1247
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Nothing holds that realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on is in the run at opt-level 1, so when the rule stops printing it other (stack_with_room in its text, a nested mod, a macro) it runs at opt-level 0 and the step stays green with the operator's condition unchecked.
- file: crates/swell/tests/realm_padding.rs
  line: 116
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The profile test reads debug_assertions and overflow checks of its own crate only, so a per-package override that turns them off for swell-syntax, swell-compiler, swell-regexp or swell-unicode in the profile collect is not seen.
- file: Taskfile.yml
  line: 1271
  category: mutant
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Dropping ok=0 after the opt-level 1 nextest-report call leaves a failing test of that run green and survives the unit's planted script, which has no failing run.
- file: Taskfile.yml
  line: 1280
  category: mutant
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Dropping failed[1] from the ignored count survives the unit's planted script, which has no failing run to tell the two apart.
- file: Taskfile.yml
  line: 1230
  category: mutant
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Deleting the compile-check of the opt-level 1 list log survives the unit's planted script, which never plants a Compiling line in a list.
- file: Taskfile.yml
  line: 1280
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: When the opt-level 0 run ends by a signal the summed line reads 1 ignored although 3 tests did not run, while the step is still red and names the missing Summary line.
```

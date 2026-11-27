---
format: aep.planning-md/3
id: review-result:adversary-lanesix-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the unit lanesix
relations:
- reviews: story:gate-test-steps-hold-the-budget-on-the-slower-runner
revision: 1
---
Adversary pass 2 on the unit lanesix of wave big-2, on correction 1 (impl/lanesix, 808f926b to c63040c1), 2026-11-27.
Red: the three caller-value checks, the path prefix of test-sum and the check that keeps the profile's test in the run at opt-level 1 held every input; test-opt1-profile-check passes a file that the cargo configuration includes, an escaped key and an inline table over several lines; test-split fails without a line of its own when the stack rule has no support file; three more mutants survived the five scripts and are caught by added cases; two notes of judgement.

unit: lanesix
verdict: red
cases: executed 111→113 (`gate-selftest`), red 2
origin: introduced 8, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/lanesix/adversary-2/
needs-coordinator: no

**Result:** red. Most of the correction holds. The three caller-value checks, the path prefix in `test-sum` and the check that keeps the profile's test in the run at opt-level 1 all held every input I tried. Two defects are left:
- `test-opt1-profile-check` lets through three forms that cargo 1.99.1 reads as a profile of one package.
- `test-split` fails without printing a line of its own when `crates/swell/tests/support/` holds no `.rs` file.

My new mutants m4, m5 and m6 each survived the five scripts; my added cases now catch all three. Goal story moved: none (adversary pass).

## Findings
1. **An included cargo config file is not read** (`Taskfile.yml:1055`, warning). Cargo 1.99.1 reads `include = ["more.toml"]` in `.cargo/config.toml` on stable.
   - Measured: with `.cargo/more.toml` holding `[profile.test.package.no-such-crate]` and `panic = "abort"`, `cargo nextest list` exits 101: ``config profile `test` is not valid (defined in `.../.cargo/more.toml`)``.
   - The same include with `[profile.test.package.swell-syntax] debug-assertions = false` passes the check: build-tests calls the stub cargo 3 times.
   - The unit's stated limit covers only files outside the tree; this file is inside it. Origin: introduced, commit `4ad70476`.
2. **An escaped key and a multi-line inline table pass the check** (`Taskfile.yml:1072`, note). The awk reads one line at a time and does not decode escapes.
   - `[profile.test."p\U00000061ckage".swell-syntax]` (or `\u0061`) passes. `cargo metadata` on the same key says ``failed to parse manifest ... `panic` may not be specified in a `package` profile``, so cargo reads it as `package`.
   - `[profile]` then `test = {` / `package = { swell-syntax = {...} }` / `}` in `.cargo/config.toml` passes too. Cargo 1.99.1 accepts multi-line inline tables: a four-line `probe = {...}` gave the same `panic` error. Origin: introduced, `4ad70476`.
3. **Profiles the run never uses are refused** (`Taskfile.yml:1066`, note, judgement).
   - `[profile.release.package.swell-syntax]` and `[profile.bench.build-override]` are refused with "which the test ... does not see". The run's chain is collect → test → dev.
   - A `"""` string under `[workspace.metadata.notes]` that holds the text `[profile.collect]` is refused too (`Cargo.toml:23`).
   - Today's files hold none of these. Origin: introduced.
4. **`test-split` fails silently** (`Taskfile.yml:1317`, note). With no `crates/swell/tests/support/*.rs`, `task test` exits 201 and prints only `task: Failed to run task "test": task: Failed to run task "test-split": exit status 1`. No test ran.
   - The same loop form is in the lane at `Taskfile.yml:486` and `:690` (commits `f9ddc9e9` and `dc6c9075`, before the base).
   - The line at 1317 is the unit's (`1910825c`). It is not the correction's: the correction's report only noted it. Origin: introduced.
5. **Mutant m4 survived the five scripts** (`Taskfile.yml:1408`). Change: `*/*) exe=` becomes `./*) exe=`.
   - My case 3 in the relative-path script, `rel/task -d run test`, catches it: `FAIL rel/task -d run test exited non-zero ... stat .../rel/task`. Origin: introduced.
6. **Mutant m5 survived the five scripts** (`Taskfile.yml:1086`). Change: the line `else if (value ~ /[{,](package|build-override)[.=]/) ...` is deleted.
   - My case 8 in the profile script, `[profile]` plus `test = { package = { swell-syntax = {...} } }`, catches it: `FAIL task build-tests did not name .cargo/config.toml:14`. Origin: introduced.
7. **Mutant m6 survived the five scripts** (`Taskfile.yml:1366`). Change: `$0 == "swell::main " keep` becomes `index($0, keep)`.
   - My case 5 in the profile-test script catches it: the test renamed in its Rust file only, to `..._on_in_collect`. Line: `FAIL task test exited 0 with the profile's test renamed in its Rust file only`. Origin: introduced.
8. **The "Internal." tasks can be called by name** (`Taskfile.yml:1280`, note). None of them has `internal: true`.
   - `task test-split TEST_OPT1_PROFILE_TEST=deep::on_any_stack` exits 0 and prints `test: swell::main 3 at opt-level 1, 1 at opt-level 0`. The same happens with `TEST_OPT1_ARGS` set without the profile collect.
   - No test runs, and `test` runs `test-split` again, so no gate result carries this. Origin: introduced.

## Checked, no finding
- **Item 1, the caller-value check.**
  - These are refused, and the line names step, variable, value and file: a trailing space, `A'$HOME\x` (printed as typed), an empty `TEST_OPT1_PROFILE_TEST`.
  - A value exported in the environment is ignored: the stub logs `run profile collect guard 1`.
  - A second `TEST_OPT1_ARGS:` line lower in `vars:`: go-task takes the lower one. `sed` prints both lines, so the value is refused and both appear in the line.
  - Mutant m7 (dropping `[ -z "$2" ] ||`) behaves exactly like the code (an equivalent mutant). An empty value never equals the file's line. With the line deleted, go-task fails first: `template: :10:33: executing "" at <.TEST_OPT1_ARGS>: invalid value; expected string`.
  - No change of the correction touches `slow-tests`, `slow-collect-start`, `build-realm-sizes` or `test-realm-sizes`.
- **Item 2, the path.** Nine forms pass with the summed line: `rel/task`, `../rel/task`, `./a b/task`, `gotask` through PATH, `--taskfile`, a call from `run/crates/swell` (go-task walks up), an exported `USER_WORKING_DIR`, and a symlinked directory.
  - `USER_WORKING_DIR` given on the command line goes red with `stat /nonexistent/./rel/task`. That is the caller overriding the variable.
  - `nextest-report` reads only the variables it is passed.
  - `forge.yml:715` dates from `a182a020` (2026-11-24), which is an ancestor of `f93ab563`, and it is unchanged.
- **Item 3, the profile's test.**
  - A real `cargo nextest list` leaves an ignored test out. `faults::host_71_injected_fault_at_each_call_faults_the_agent` is printed "other" in `.out/test-stack.txt` and named in `.out/test-opt1-nextest.toml`, but is in none of the three lists (2532 lines in `.out/test-list-all.txt`). So `#[ignore]` makes the step red (my case 6), and so do a rename or a deletion.
  - The match is on the whole line, so a test of another binary cannot match.
  - No expected line of the unit's script got weaker. The stub's opt-level 1 run still prints `2 tests run` while its list holds 3. The summed-line check is arithmetic on the stub's numbers and is unchanged.
- **Item 4, the profile check.** These are refused: quoted keys, `"*"`, spaces inside a header, a dotted key under `[profile.test]` and at the top of the file, one-line inline tables, a header with a trailing comment, `.cargo/config`, `CARGO_PROFILE_COLLECT_*`, `*_BUILD_OVERRIDE_*`, and `CARGO_PROFILE_TEST_PACKAGE_swell-syntax_*` (cargo has no environment form for a package profile).
  - Not refused, and correctly so: `[workspace.metadata.profile]`, `[workspace.package]`, and a lower-case variable name (cargo does not read lower case).
  - A member crate's own `Cargo.toml`: cargo ignores profiles there and warns. Reasoned, not run.
- **Item 5.** m1: `FAIL task test exited 0 with a failed test in the run at opt-level 1`. m2: `FAIL task test did not print the summed line with the failed test, 3 passed, 1 failed, 1 ignored ...`. m3: `FAIL task test exited 0 with a Compiling line in the log of the list at opt-level 1`.
- **Items 6 and 7.**
  - `slot task kill-check`: `kill-check: no kill with two dashes after its signal operand in 149 file(s)`.
  - Clean head: `gate-selftest: 111 passed, 0 failed`.
  - The five scripts alone under dash: all `ok`, at 1.06, 0.69, 0.75, 0.90 and 0.88 s. The node's `sh` is bash.
  - `git diff --stat f93ab563..c63040c1` is empty for: `taskfiles/gate-time.yml`, `taskfiles/nextest.yml`, the four scripts the story names, `realm-room-tests.awk`, `collect-mode.yml`, `stack-profiles.yml`, `test262.yml`, `.cargo`, `Cargo.toml`, `.github` and `conformance`.

## Cases added (uncommitted, in the worktree)
Both new scripts are red, and each run alone ends within 2 s under sh and dash:
- `tests/gate/build-tests-refuses-a-package-profile-in-every-form-cargo-reads.sh`: findings 1 and 2; 0.62 s; red.
- `tests/gate/test-says-why-it-fails-when-the-stack-rule-has-no-support-file.sh`: finding 4; 0.12 s; red.

Cases added to existing scripts, green on the code:
- `test-sums-its-runs-when-task-is-called-by-a-relative-path.sh`, case 3: catches m4; script 1.11 s.
- `build-tests-refuses-a-profile-its-profile-test-does-not-see.sh`, case 8: catches m5; script 1.07 s.
- `test-keeps-the-profile-test-in-its-run-at-opt-level-1.sh`, case 5 (catches m6) and case 6 (`#[ignore]`); script 1.39 s.

## Commands and results
- `slot sh` probes 1 to 3 (temporary scripts, deleted afterwards): the outputs quoted above.
- Cargo runs, each with the planted file saved, planted and put back:
  - `cargo metadata` with the `\U00000061` key: exit 101, `` `panic` may not be specified in a `package` profile``.
  - The multi-line `probe = {...}`: the same.
  - `SLOT_TIMEOUT=30m cargo nextest list` with the opt-level 1 arguments and `include = ["more.toml"]`: exit 101, ``config profile `test` is not valid``. The string form `include = "more.toml"` gives `expected a list of strings or a list of tables`.
- `mutants.sh m1..m7` on the head's scripts: m1, m2 and m3 are caught; m4 to m7 give `ok` in all five. `mutants.sh m4 m5 m6` with my cases: each is caught. Each run ended with `Taskfile.yml put back`.
- `slot task gate-selftest` with my cases: exit 201, `gate-selftest: 111 passed, 2 failed`. `git status --short` lists only my five case files.

## Not attacked
- `TASK_X_*` experiments, `.taskrc`, and go-task's `--set` (it has no such flag).
- The first pass's `vars.sh`: it works in a scratch copy, so I did not run it. I relied on the diff, which has no hunk in the lane's tasks.
- A real build with a package profile set; I used cargo's validation errors instead.
- Whole gates, `slow`, `test262`.

```findings
- file: Taskfile.yml
  line: 1055
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: test-opt1-profile-check does not read a file that .cargo/config.toml includes, and cargo 1.99.1 reads include = ["more.toml"] on stable, so [profile.test.package.swell-syntax] in .cargo/more.toml passes build-tests unseen by the profile test.
- file: Taskfile.yml
  line: 1072
  category: boundary
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The line-wise awk of test-opt1-profile-check passes a quoted key with a unicode escape ("p\U00000061ckage") and an inline table over several lines, both of which cargo 1.99.1 reads as a profile of one package.
- file: Taskfile.yml
  line: 1066
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: test-opt1-profile-check refuses package and build-override profiles of release and bench, which the run at opt-level 1 never uses, and refuses profile text inside a multi-line string.
- file: Taskfile.yml
  line: 1317
  category: boundary
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: With no crates/swell/tests/support/*.rs the false [ -f ] at the end of the loop ends test-split in go-task's shell, so task test exits 201 with no line of its own.
- file: Taskfile.yml
  line: 1408
  category: mutant
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Narrowing the path prefix case to ./* survives the five scripts; a case calling rel/task without a leading ./ catches it.
- file: Taskfile.yml
  line: 1086
  category: mutant
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Deleting the inline-table branch for package under [profile] survives the five scripts; a case with test = { package = {...} } under [profile] catches it.
- file: Taskfile.yml
  line: 1366
  category: mutant
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Replacing the whole-line match of the profile test with index($0, keep) survives the five scripts; a case with the test renamed to a longer name in its Rust file only catches it.
- file: Taskfile.yml
  line: 1280
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The tasks described as Internal have no internal true, so task test-split with a caller value of TEST_OPT1_PROFILE_TEST or TEST_OPT1_ARGS exits 0 and prints its green line, though no test runs and task test reruns it.
```

---
format: aep.planning-md/3
id: review-result:adversary-laneeight-step-2-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on step 2 of the unit laneeight: red, 7 findings, 2 failing cases'
relations:
- reviews: story:slow-lane-holds-its-bound-on-the-slower-runner
revision: 1
---
Adversary pass 1 on step 2 of the unit laneeight of wave big-2, on impl/laneeight at fddb4a52 (the step's commits 7f9392c0..fddb4a52), 2026-11-28.
The adversary's own report follows as it was given.

unit: laneeight, step 2 (impl/laneeight, head fddb4a52, base 7f9392c0), worktree as handed plus two untracked cases
verdict: red
cases: executed 117→119, red 2
origin: introduced 7, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/laneeight/adversary-s2-1/ (work files only, listed in part 6)
needs-coordinator: yes. The fix for F1 needs `gate-vars` (GATE_VARIABLES), which is outside the story's scope of nine tasks.

**1. What I touched**
- `git --no-pager diff --stat`: empty. No tracked file changed.
- `git status --short`: only my two cases, both untracked:
  - `?? tests/gate/test-realm-sizes-counts-only-the-parts-it-ran.sh`
  - `?? tests/gate/test-split-takes-no-caller-value-of-a-split.sh`
- `Taskfile.yml` was changed in the tree once, for the mutants. I put it back from the saved copy, and `cmp` plus `git diff --quiet` showed no difference to HEAD.

**2. Cases added. Both are red now. Each was run alone, before the suite.**

- **`tests/gate/test-split-takes-no-caller-value-of-a-split.sh`** (3.29 s alone; builds nothing).
  - What it checks: a caller's SPLIT_ values must not leave a test out of both parts while the step passes.
  - Harness first: `task test` and `task slow-tests` without caller values each run `deep::on_any_stack` once.
  - Red output, verbatim:
    ```
    test-split-takes-no-caller-value-of-a-split: FAIL a caller's SPLIT_ value left a test out of every run, and the step passed:
      1. task test SPLIT_RUN=test SPLIT_ARGS='--workspace --locked --profile default' SPLIT_EXCLUDE=.out/skip.txt exited 0 and never ran swell::main deep::on_any_stack: 4 test binaries: 6 passed, 0 failed, 1 ignored (...cli.out)
      2. SPLIT_RUN, SPLIT_ARGS and SPLIT_EXCLUDE in the environment of task test: exit 0, swell::main deep::on_any_stack never ran (...env.out)
      3. SPLIT_EXCLUDE in the environment of task slow-tests: exit 0, swell::main deep::on_any_stack ran in neither part of its default mode:slow-tests: 2 tests at opt-level 1, 1 at opt-level 0 (...lane.out)
    exit 1, seconds 3.294827987
    ```

- **`tests/gate/test-realm-sizes-counts-only-the-parts-it-ran.sh`** (0.92 s alone; builds nothing).
  - What it checks: a second run of `test-realm-sizes` in the same tree, whose part at opt-level 0 is empty, must count 0 at opt-level 0.
  - Red output, verbatim:
    ```
    test-realm-sizes-counts-only-the-parts-it-ran: FAIL task test-realm-sizes ran no part at opt-level 0 and printed "realm-sizes 4 default: 2 tests at opt-level 1, 1 at opt-level 0", the count of a part an earlier run left in .../run/.out/realm-sizes-4-default-opt0.log.count (...second.out)
    exit 1, seconds .917689472
    ```
  - My first version of this case had no test outside the room set besides the profile test. That run is the evidence for F3: `error: the stub's filter cannot read the filterset: an unexpected token )` / `realm-sizes: cargo nextest list --message-format json-pretty --locked -p swell --profile default --test main failed, exit 2`.

**3. Suite, run after the cases existed**
- `slot task gate-selftest`: `gate-selftest: 117 passed, 2 failed`, exit 201.
- The two that failed are exactly my cases (second run, filtered for "fail"): `test-realm-sizes-counts-only-the-parts-it-ran: failed, exit 1`, `test-split-takes-no-caller-value-of-a-split: failed, exit 1`.
- `slot task kill-check`: `kill-check: no kill with two dashes after its signal operand in 159 file(s)`.

**4. Findings** (all cover fddb4a52)

- **F1. Callers can pass SPLIT_ values to `test-split`.** `Taskfile.yml:1611` (`SPLIT_RUN: '{{.SPLIT_RUN | default ""}}'`), with the gate's call at `:1513`, which passes no vars.
  - Measured: with SPLIT_RUN, SPLIT_ARGS and SPLIT_EXCLUDE given on the command line or in the environment, `task test` exits 0 and the excluded test runs in neither part. `test-sum` reads it as one more "ignored" test.
  - Also measured: SPLIT_EXCLUDE in the environment reaches the lane's nested call from `slow-tests`, which passes no SPLIT_EXCLUDE.
  - `gate-vars`' own summary says "A new gate variable is added to GATE_VARIABLES in the change that adds it", and the change did not add them.
  - What reaches it: any caller of `task check`, `task test` or `task slow` who sets them. No documented workflow does. It is the same kind of caller value that `test-opt1-vars-check` refuses for TEST_OPT1_ARGS.
  - Origin: introduced (the base has no SPLIT_ variable).
  - Fix I would name: the gate's call passes the SPLIT_ variables empty; the lane's two calls pass SPLIT_EXCLUDE and SPLIT_OPT1_TARGETS explicitly; `test-opt1-vars-check` refuses SPLIT_ values from a caller; GATE_VARIABLES lists them.
- **F2. Stale count files in `test-realm-sizes`.** `Taskfile.yml:957-958`. The line "realm-sizes <k> default" reads `.count` files, which are removed only for the modes that run (`:908`). When a part is empty, the count from an earlier run is printed (red case above).
  - What reaches it: nothing found. The part at opt-level 0 has 50 tests in the repository, and I built the empty state myself.
  - Origin: introduced.
- **F3. Empty exclusion list gives an invalid filter.** `Taskfile.yml:1664`. When SPLIT_EXCLUDE holds no "other" line, the form `all` writes `not (binary_id(=swell::main) & ())`, and the realm list then fails with exit 2.
  - Shown with the stub's parser, not with real nextest.
  - What reaches it: nothing found. It needs every test of `swell::main` outside the room set to be the profile test.
  - Origin: introduced.
- **F4. The wait bound no longer sits under the part's bound.** `Taskfile.yml:506`. The summary says "(400, under the part's 600)", but `slow-tests` now waits up to 400 s for each of three builds, up to 1,200 s in one part.
  - When the waits add up past 600 s, the first message is the part's `slow-time` bound, after the part has ended. That message is true; the summary's bracket is not.
  - Read from the text, not run.
  - Origin: introduced.
- **F5–F7. Three mutants of the unit's lines that the unit's seven lane scripts do not catch.** All three were applied in one tree and all seven scripts printed `ok`. The code itself is correct on each point, so each is a note giving the case that would catch it:
  - **F5.** `:914`, the guard `{{.NO_STACK_TESTS}}=1` removed from the realm run at opt-level 1. A case should assert a stub line `^run profile=collect .* guard=\[1\] .* config=realm-sizes-opt1-nextest.toml`.
  - **F6.** `:585`, `SPLIT_FULL_COUNTS=1` removed from `slow-tests`' call of `test-split`. A case should assert `full=[1]` on every `^list ` line of `slow-tests`. On the forge, the real lists would compile and fail; no script catches it.
  - **F7.** `:893`, `SPLIT_BUILT_BY` removed from the realm call, so the realm lists are no longer checked for Compiling. A case should run `STUB_COMPILE_LIST=collect task test-realm-sizes REALM_SIZES=4` and require a failure naming "realm-sizes: the list compiled".

**5. Attacked and not broken**
- **Complement of the two parts:** the opt-level-0 filter is `not (<opt-level-1 filter>)`, and the same exclusion is added to both. Each list uses the same arguments, configuration and `-E` as its run, and every term names its binary. The check covers "in neither" and "in both".
- **45 against 44:** the probe counted 44 tests stopped by the guard plus 1 stack test that passed under it (`limits_scripts::a_stack_test_fails_by_itself_where_no_stack_tests_is_set`). That is 45, which is 34 stack tests plus 11 written by a macro (probe report lines 42–47).
- **313 against 305:** I ran the room and stack rules on f962c324 and on fddb4a52. Room tests go 323→331, all 8 new ones in `builtins_array_from_async`; the stack lines stay at 73 on both. So the difference is tests added after the probe's base, as the unit guessed.
- **Guard:** it is set in every moved run.
- **Builds and runs:** by the text, each build and the runs that read it have the same SWELL_FULL_COUNTS, RUSTFLAGS and profile. `slow-collect-a-lane-reads-only-its-own-build.sh` compares build and run lines. `collect-profile.toml` inherits `test`, so debug assertions and overflow checks stay on.
- **The gate's own call of `test-split`:** base against head, in a planted tree with a third TEST_OPT0_TESTS pair and an added stack test. Stdout and 8 files under `.out/` are byte for byte the same (`test: swell::main 2 at opt-level 1, 2 at opt-level 0`). The probe script was removed afterwards.
- **Scope:** only the nine tasks changed, and the top variables did not. `gate-time.yml` and the two `slow-time-refuses-*` scripts are unchanged; the bounds stay at 900 and 600.
- **Shell:** no `local`, `[[`, `echo -e` or process substitution in the added lines.
- **The reported `continue` defect:**
  - The fix covers the loop it was reported in.
  - The only other new `continue` (`:1690`) is the last command in its block, so it is safe.
  - The unit's advice ("avoid `continue` inside `case`") is too narrow. On task 3.54, `continue` inside an `if` or a `{ }` block does not stop the rest of that block either. It only skips what follows the block. Probe output: `plain continue in if: still ran b`, `brace group: ran b`.

Not attacked:
- builds against real cargo with a planted difference between build and run;
- two lanes at once in one tree;
- a whole lane run;
- re-running the unit's plants of the old defects (`red-before.txt`).

**6. Paths written outside the worktree**
All under <scratch>/wave2/laneeight/adversary-s2-1/:
- scripts: rules.sh, tasks.sh, mutate.sh;
- copies and diffs: taskfile.diff, Taskfile.yml.saved, Taskfile.yml.mutant;
- probe/Taskfile.yml;
- extracted trees: src-f962c324/, src-fddb4a52/;
- rule lists: room-f962c324.txt, room-fddb4a52.txt, stack-f962c324.txt, stack-fddb4a52.txt;
- per-task texts: t-7f9392c0/, t-fddb4a52/.

```findings
- file: Taskfile.yml
  line: 1611
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: A caller's SPLIT_RUN, SPLIT_ARGS and SPLIT_EXCLUDE on the command line or in the environment make the gate's step test and the lane's slow-tests leave a test out of both parts and pass, and GATE_VARIABLES does not list them.
- file: Taskfile.yml
  line: 958
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: The realm count line reads a .count file that an earlier run left when a part is empty and does not run, so it prints a stale count at opt-level 0.
- file: Taskfile.yml
  line: 1664
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: A SPLIT_EXCLUDE file with no other line makes test-split write the filter not (binary_id(=swell::main) & ()), which the list cannot read.
- file: Taskfile.yml
  line: 506
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The summary puts the wait of 400 s under the part's 600 s, but slow-tests now waits up to 400 s for each of three builds, so the part's bound and not the wait's message speaks first.
- file: Taskfile.yml
  line: 914
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Removing the guard NO_STACK_TESTS from the realm run at opt-level 1 leaves all seven lane scripts green.
- file: Taskfile.yml
  line: 585
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Removing SPLIT_FULL_COUNTS=1 from the call of test-split in slow-tests leaves all seven lane scripts green.
- file: Taskfile.yml
  line: 893
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Removing SPLIT_BUILT_BY from the realm call of test-split, so that its lists are not checked for a Compiling line, leaves all seven lane scripts green.
```

---
format: aep.planning-md/3
id: review-result:adversary-laneeight-step-2-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on step 2 of the unit laneeight: green, one note on a refusal no script held'
relations:
- reviews: story:slow-lane-holds-its-bound-on-the-slower-runner
revision: 1
---
Adversary pass 2 on step 2 of the unit laneeight of wave big-2, on impl/laneeight at 2e278984 (correction 1, the diff fddb4a52..2e278984), 2026-11-28.
The adversary's own report follows as it was given.

unit: laneeight, step 2, correction (impl/laneeight, head 2e278984); tree clean afterwards
verdict: green
cases: executed 119→119, red 0
origin: introduced 1, pre-existing 0, undecided 0
wrote-outside-worktree: 7 paths in <scratch>/wave2/laneeight/adversary-s2-2/ (listed in part 6)
needs-coordinator: no

The correction holds: none of the five attack lines broke it. The one finding is a note: the new SPLIT_ refusal lines can be deleted and the whole gate self-test stays green.

**1. What I touched**
- `git --no-pager diff --stat` and `git status --short` are both empty at the end.
- I added no case this pass.
- Two probe files were in the tree only while they ran and are now deleted: `tests/gate/probe-pass-two.sh` and `tests/gate/fixtures/probe-all.toml`.
- I mutated `Taskfile.yml` four times; each time I put back the saved copy and `git diff --quiet` showed no difference to HEAD.

**2. Cases:** none added. Every point I attacked holds, and the one gap I found is not a defect in the code, so it is a note (F8 below) with the case that would catch it.

**3. Commands and their result lines** (one pool job at a time)

- **Probe, `slot sh tests/gate/probe-pass-two.sh`** (stub cargo, builds nothing)
  - **A. The gate's step `test`, fddb4a52 against the head, no caller value:**
    - `base exit 0`, `head exit 0`, `stdout same (12 lines)`.
    - Every file under `.out/` is the same bytes.
    - The stub logs are the same once the planted tree's name in the config paths is evened out: `stub logs same but for the tree's name`.
  - **B. A caller's `SPLIT_RUN=x`:** `build-tests`, `test`, `slow-tests` and `test-realm-sizes`, each from the command line and from the environment:
    - every one gives `exit 201, refused 1, stub calls 0`, so nothing was built, listed or run;
    - empty values (`SPLIT_RUN= SPLIT_EXCLUDE=`): `exit 0`;
    - `SPLIT_OTHER=1` in the environment: `exit 0` (`test-split` reads no such name);
    - `gate-vars` with `SPLIT_EXCLUDE=f`: `exit 0`, and its line shows `SPLIT_EXCLUDE=f`, as its summary says.
  - **C. F4's shared wait bound:**
    - A stale `.out/slow-tests-waited` holding 999 is ignored: `exit 0`, and the lines `slow-build opt1 2.0 s ... waited 2 s; lib ... waited 1 s; collect ... waited 2 s` each still give the build's own seconds.
    - First build hangs: `slow-tests: the build opt1 that the lane started wrote no status within 2 s of waiting for the lane's builds in sum, 2 s waited, 2 s of them for the build opt1`.
    - Third build hangs: `... the build collect ... within 3 s ... in sum, 3 s waited, 1 s of them for the build collect`.
    - The unit's own sum case covers the second wait.

- **F3's `all()` with real nextest**
  - `cargo nextest list --locked -p swell-num --profile default --config-file tests/gate/fixtures/probe-all.toml -E 'binary_id(swell-num::main)' --message-format oneline` → `exit 0`, 153 tests.
  - The same list without that config file: `same 153 lines as without the default filter`.

- **Item 4, the three new assertions, each mutant applied by pattern** (`mutate2.sh`):
  - F6, `SPLIT_FULL_COUNTS=1` removed from `slow-tests`' call → `slow-split-refuses-a-test-in-neither-part-or-in-both: FAIL a list of slow-tests ran without SWELL_FULL_COUNTS=1`.
  - F5, guard removed from the realm run at opt-level 1, and F7, `SPLIT_BUILT_BY` removed from the realm call, applied together:
    - `slow-split-...: FAIL task test-realm-sizes ran its part at opt-level 1 without the guard SWELL_NO_STACK_TESTS=1`
    - `slow-refuses-a-moved-run-that-compiles: FAIL task test-realm-sizes exited 0 with a Compiling line in the list of its part at opt-level 1`
  - Without the mutants all three scripts pass (the unit's `gate-selftest: 119 passed, 0 failed`, and my last self-test run below).

- **Item 5, the four fixes taken out** (one mutant per fix; each script holds a different fix):
  - F1 taken out = the seven refusal lines, the gate's empty call vars and `slow-tests`' empty SPLIT_OPT1_TARGETS and SPLIT_EXCLUDE: `test-split-takes-no-caller-value-of-a-split: FAIL ...` on all three reaches, as in pass 1.
  - F2 taken out: `test-realm-sizes-counts-only-the-parts-it-ran: FAIL ... printed "realm-sizes 4 default: 2 tests at opt-level 1, 1 at opt-level 0"`.
  - F4 taken out (`used=0`): `slow-collect-a-lane-reads-only-its-own-build: FAIL the waits did not share one bound of 4 s: opt1 waited 4 s, lib 4 s, 4 s in sum`.
  - F3 taken out (the old filter line): `slow-split-...: FAIL task test-realm-sizes exited non-zero with no test outside the room set but the profile test`.

- **Only the seven refusal lines removed** (`Taskfile.yml:1270-1276`), `slot task gate-selftest` → `gate-selftest: 119 passed, 0 failed` (this is F8).

**4. Finding**

- **F8. The refusal lines are held by no script.** `Taskfile.yml:1270` (the seven `given SPLIT_*` lines of `test-opt1-vars-check`).
  - Measured: with those lines deleted, all 119 self-test scripts pass.
  - Why it stays green: the other two layers of the fix cover the same ground. The gate calls `test-split` with every SPLIT_ value empty, and the lane passes all seven on the command line of its nested call, which overrides the environment.
  - So the refusal, its message, and its summary's promise that a part run alone refuses too, are untested.
  - What reaches it: nothing harmful found. Each remaining layer alone kept a caller's value out of every split.
  - The case that would catch it: `task test SPLIT_RUN=x` and `SPLIT_RUN=x task slow-tests` in a planted tree must exit non-zero, print `<step>: SPLIT_RUN=x is given by the caller`, and leave the stub log empty. Probe B passes exactly this on the head.
  - Origin: introduced.

**5. Attacked and not broken**
- **F1:**
  - The refusal is a list of seven names, but `test-split` reads no other SPLIT_ name.
  - An empty value is accepted and does nothing.
  - The lane's own values do not trip the check: `test-split` does not call it, and the lane's run in probe C exited 0.
  - `task slow` refuses right after `gate-vars`, before anything else runs (`Taskfile.yml:241`).
  - `task check` refuses only at `build-tests` (`:1429`), after `nostd-build` and `build` have run. That is the same point where the existing TEST_OPT1_ARGS refusal fires, and nothing wrong comes of it.
  - `task test-split` alone writes only files that the next `test` writes again.
  - `build-realm-sizes` does not call the split.
- **F2:** the count line is printed only for the sizes of the current call, and the counts of those sizes are removed first, so no line of another size can appear.
- **F3:** real nextest reads `all()`.
- **F4:** a stale sum file is removed at the task's start; a build already done counts 0 s of waiting; the message is right in the first, second and third wait; each `slow-build` line still gives the build's own seconds.
- **The unit says my pass-1 F6 mutant changed nothing.** That is not so: after the first `sed` missed, I applied `sed -i '585s/ SPLIT_FULL_COUNTS=1 / /'` and checked 3 changed lines against the saved copy before that job ran. The result stands either way: the unit's new assertion now catches that mutant.

Not attacked: `task slow` and `task check` with a caller value (not run, as the brief says; read from the text only), and two lanes at once in one tree.

**6. Paths written outside the worktree**
<scratch>/wave2/laneeight/adversary-s2-2/:
- `tf.diff`
- `all-list.txt`
- `all-list.err`
- `f3-old-line.txt`
- `mutate2.sh`
- `Taskfile.yml.saved`
- `Taskfile.yml.mutant`

```findings
- file: Taskfile.yml
  line: 1270
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Removing the seven SPLIT_ refusal lines of test-opt1-vars-check leaves gate-selftest at 119 passed, so no script holds the refusal or its message, though the gate's empty call values and the lane's explicit values still keep a caller's value out of every split.
```

---
format: aep.planning-md/3
id: review-result:adversary-lanebuilds-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit lanebuilds: red, one warning and four notes'
relations:
- reviews: story:realm-size-lane-within-its-share
revision: 1
---
Adversary pass 1 on `impl/lanebuilds` at `d97404f` (wave big-2, the lane `slow` within its bound, second unit), run on 2026-11-26 and 2026-11-27.
The report below is the adversary's last message; paths of the workstation are replaced.

unit: lanebuilds
verdict: red
cases: executed 94→97, red 1
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/lanebuilds/adversary-1/
needs-coordinator: yes

**The verdict is red because of one introduced warning (F1), and my new case for it fails on the head.** No gate build loses a module: the module selection does what it says. The other four findings are minor.

## Findings

**F1 (warning, introduced): misspelled feature names in swell's tests are no longer refused.**
- Where: `taskfiles/test-main.sh:85` writes `#![expect(unexpected_cfgs, …)]` at the top of `crates/swell/tests/main.rs` (lines 2-5). That turns the lint off for all 104 modules, not just for `test_modules`.
- What happens: a test file with `cfg!(feature = "collect-every-safe-points")` (one letter too many) now builds under `-D warnings` and quietly takes its other count. swell's tests have 122 such feature checks.
- Proof: my case fails on the head (`Finished dev`). With `main.rs` in the base form (`TEST_MAIN_SELECT=` empty) the same build is refused with ``error: unexpected `cfg` condition value: `collect-every-safe-points` ``.
- What reaches it: every gate build of `swell::main`.
- Possible fix: declare the name in `crates/swell/Cargo.toml` (`[lints.rust] unexpected_cfgs` with `check-cfg = ['cfg(test_modules, values(any()))']`) and drop the expect line.

**F2 (note, introduced): `test-stack-profiles` runs whatever binary the metadata file names, even if it was built from other sources.**
- Where: `taskfiles/stack-profiles.yml:234-236`. Nothing ties the file to the current sources. Before this change the task built the binary itself.
- Proof (job a1): after a build with `STACK_MODULES=nope`, the task ran that 0-test binary (`main-6de2…`). Older binaries stay side by side in `target/<profile>/deps` (`main-6eff…`, `main-d1c8…`, `main-6de2…`).
- What reaches it: running the task alone after a source change. The lane itself always builds first. Story `test-builds-carry-no-debug-information` step 3 runs build and test as two separate commands.

**F3 (note, introduced): one refusal of `stack-modules-check` is held by no step of the unit's selftest.**
- The refusal is "STACK_TESTS names no test" (`stack-profiles.yml:49`).
- With that line removed, the unit's selftest still prints `ok`; my case prints `FAIL stack-modules-check accepted tests`.
- Later in the lane, `test-stack-profiles:229` would still refuse an empty list.

**F4 (note, introduced): a `RUSTFLAGS` set by the caller can build a test binary with almost no tests.**
- go-task 3.54 lets an exported `RUSTFLAGS` override a task's own `env:`. Measured with a scratch Taskfile: it printed `RUSTFLAGS=[--cfg test_modules]`.
- So `RUSTFLAGS='--cfg test_modules' task check` builds `swell::main` with only `support`, and without `-D warnings`.
- What would catch it (not run end to end): in the gate, `host-requirements-check` (the conformance files name tests from 46 of the 103 modules); in the lane, `test-realm-sizes` and the collect run of `slow-tests` ("no tests to run"). `taskfiles/test-main.sh check` does not see it.
- What reaches it: only a variable the caller exports.

**F5 (note, introduced): a new environment variable changes what the gate's layout check expects, and the gate does not show it.**
- `taskfiles/test-main.sh:54` reads `TEST_MAIN_SELECT`. It is not in `GATE_VARIABLES`, against `Taskfile.yml:492` ("A new gate variable is added to GATE_VARIABLES in the change that adds it").
- An exported `TEST_MAIN_SELECT=` makes `test-main` write, and `check` accept, a `main.rs` that the forge refuses. That failure is loud, not silent.

## Results per point, with no further finding
1. **No lost module.**
   - The small and full binaries get different file names (stack-debug `main-6eff…` with 9 tests, `main-d1c8…` with 1,309). After the small build, cargo rebuilt the full one (1 `Compiling`).
   - `swell::main` has 1,309 tests on the head (gate dev list). The base has the same 104 `mod` lines, and `builtins_json_parse.rs` adds no test, so the base also has 1,309.
   - `build-stack-profiles` leaves nothing in the environment, and no later part of the lane reads its files.
   - All three planted `main.rs` forms are refused by `check` (case green): a cfg line naming another module, one without `not(test_modules)`, and a module with no line.
2. **A value that is not a module.** `test_modules` is declared nowhere (that is F1). `STACK_MODULES=nope` is refused by `stack-modules-check` (no file). With the check fooled through `STACK_TESTS_DIR`, both 0-test binaries were built, and `test-stack-profiles` refused them (`error: no tests to run`, nextest exit 4).
3. **Planted frame growth in `Agent::execute`:**

   | added | binary | release | stack-debug |
   |---|---|---|---|
   | 4 KiB | small | 4 passed | 0 of 4 (stack overflow, SIGABRT) |
   | 4 KiB | full | 4 passed | 0 of 4 |
   | 16 KiB | small | 0 of 4 | not run: the task stops at the first red profile |
   | 16 KiB | full | 0 of 4 | 0 of 4 |

   - The small and full binaries never disagreed at these sizes, so the unit's 3,072-byte gap does not show here.
   - `evaluate_script` is 824 bytes in the small binary and 296 in the full one because of inlining. The engine is generic over the host, so each binary compiles its own copy. In the small binary a callee with a single caller gets inlined into `evaluate_script`, and its locals join that frame.
   - A host's own binary, with few call sites, can get the larger form.
4. **`test-stack-profiles` refusals.** A missing file is refused (`:234`). A test count other than the list's, or a skipped test, is refused (`want`). The debug-section check reads the binary path from the file and calls `no-debug-sections.sh` as before. The task builds nothing. The stale file is F2.
5. **`stack-modules-check` edge cases** (from reading the code, not run).
   - A duplicate test name passes the check, then `test-stack-profiles` refuses with a misleading count.
   - A file `tests/<m>/mod.rs`, a `pub fn`, or a test written by a macro: refused as "in no file".
   - A test name that is a prefix of another: the check's pattern handles it. nextest's `test()` filter matches substrings, so the extra test would make the run count wrong: a loud refusal, as before this change.
6. **The JSON test at 200 levels.** It walks four ways out: (i) a valid text with a reviver and (ii) an unclosed text, both at 200/2,000 levels; (iii, iv) the budget cases at `DEPTH`, unchanged. Nothing in `parse.rs` turns on only above 200: `STEP_CHUNK` (4,096) is reached at neither size, and the kept stack starts empty.
   - My planted defect: delete `parse.rs:824` (`self.heap.accounting.credit(frame.charged);`).
   - It is red at 200: `valid with reviver: call 2: … left: 584575 right: 549375`. It is also red at 2,000.
7. **Workflow path lists.** Every file the lane reads is in both lists. The diff under `.github` is the 6 added lines and nothing else.
8. **Numbers.** 271.5 + 75.9 + 139.2 + 17.3 = 503.9 s. 174.5 × 1,916 / 1,617 = 206.8 s, with 821 + 1,095 = 1,916 (`lane4.log:9-10`). 3,220 × 0.4 / 4 = 322 s; `for-coordinator.md:172` adds 14 nextest starts to reach about 350. All seven collect runs selected 312 tests (311 passed + 1 failed at k = 128), and 312 + 997 skipped = 1,309.

## Cases added (uncommitted)
- `tests/gate/test-main-check-refuses-a-changed-module-cfg.sh`: ok, 193 ms
- `tests/gate/stack-modules-check-refuses-a-blank-list.sh`: ok, 281 ms
- `tests/gate/build-refuses-a-misspelled-feature-in-a-test-module.sh`: **FAIL**, 201 ms (F1)

## Commands (all from the worktree; scripts and logs in `adversary-1/`)
- `task` with a scratch Taskfile and an exported `RUSTFLAGS`, no slot: `RUSTFLAGS=[--cfg test_modules] CARGO_ENCODED_RUSTFLAGS=[]`
- `slot --timeout 25m sh -c "$(cat a1.sh)"`: `exit 0`; `a1 gate dev list: swell::main testcases 1309`
- `slot --timeout 30m sh -c "SIZE=4096 FULL_RELEASE=1; …plant.sh"`: `exit 0`; `plant full release: Summary … 4 tests run: 4 passed, 1305 skipped`
- `slot --timeout 30m sh -c "SIZE=16384 FULL_RELEASE=1; …plant.sh"`: `exit 0`; `plant full release: Summary … 4 tests run: 0 passed, 4 failed, 1305 skipped`
- `slot --timeout 15m sh -c "$(cat a3.sh)"`: `exit 0`; `a3 planted collect SWELL_FULL_COUNTS=0: exit 101 in 0 s`
- `slot --timeout 10m sh -c "$(cat a4.sh)"`: `exit 0`; `a4 mutant case stack-modules-check-refuses-a-blank-list: exit 1: … FAIL stack-modules-check accepted tests`

All copies were made and removed inside the pool jobs, on the build node: `.out/adv-plant/target` (twice), `.out/adv-json/target` and `.out/adv-mut` (whole).

## Not attacked
- A bisection of the least stack each test needs, small against full binary (I only checked pass or fail at 4 and 16 KiB).
- An end-to-end gate run with `RUSTFLAGS` set (F4).
- The forge itself.
- The filter lines of `test-realm-sizes`, `gate-time.yml`, and the 79 slow collect tests (other passes have them).

```findings
- file: taskfiles/test-main.sh
  line: 85
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The crate-wide expect of unexpected_cfgs in swell's tests/main.rs lets a misspelled feature such as collect-every-safe-points build under -D warnings, which the base form refused.
- file: taskfiles/stack-profiles.yml
  line: 236
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: test-stack-profiles runs whatever binary the binaries-metadata file names, with nothing tying it to the current sources, so a run alone after a source change tests an old binary.
- file: taskfiles/stack-profiles.yml
  line: 49
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The refusal of a blank STACK_TESTS is held by no step of the unit's selftest; a copy with it removed still prints ok there.
- file: crates/swell/tests/main.rs
  line: 6
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: go-task lets an exported RUSTFLAGS win over a task's env, so RUSTFLAGS with --cfg test_modules builds swell::main with support only, and in the gate only host-requirements-check would notice.
- file: taskfiles/test-main.sh
  line: 54
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: TEST_MAIN_SELECT changes what the gate's layout check expects but is not in GATE_VARIABLES, against the rule in Taskfile.yml line 492.
```

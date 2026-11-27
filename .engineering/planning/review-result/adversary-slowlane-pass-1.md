---
format: aep.planning-md/3
id: review-result:adversary-slowlane-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit slowlane
relations:
- reviews: story:realm-size-lane-within-its-share
revision: 1
---
Adversary pass 1 on `impl/slowlane` at `355c789b` (wave big-2, the lane `slow` within its bound), run on 2026-11-26.
The report below is the adversary's last message; paths of the workstation are replaced.

unit: slowlane (story realm-size-lane-within-its-share), tree impl/slowlane at 355c789b plus 2 untracked selftests
verdict: NEEDS-CHANGE (warning); no blocker
cases: executed gate-selftest 90→92, red 4 (my 2, and 2 site-data-check that were already red on the base)
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory
needs-coordinator: the operator decides whether the collect mode runs at the 5 middle sizes (finding 3)

**1. Diff of the tree**
`git --no-pager diff --stat` is empty: I changed no tracked file. `git status --short` shows only my two new files, both test files:
```
?? tests/gate/slow-time-refuses-a-lane-whose-untimed-part-ran-over-its-bound.sh
?? tests/gate/test-realm-sizes-hands-nextest-a-long-list-of-tests.sh
```

**2. Cases added (uncommitted, both red now)**
- `tests/gate/test-realm-sizes-hands-nextest-a-long-list-of-tests.sh` plants a tree with the real `Taskfile.yml` and `taskfiles/`, and a stub `cargo`. It then runs `task test-realm-sizes REALM_SIZES=4`:
  - With 900 "other" tests it passes; the stub saw a 68,428-byte `-E` argument.
  - With 2,000 "other" tests it must pass too. It fails. Red when run alone:
    `FAIL task test-realm-sizes with 2000 tests that read no room exited non-zero: task: Failed to run task "test-realm-sizes": fork/exec .../bin/cargo: argument list too long`
- `tests/gate/slow-time-refuses-a-lane-whose-untimed-part-ran-over-its-bound.sh` plants a lane clock started 1,000 s ago and four timed parts of 25 s each. It expects `slow-time-sum` to refuse the lane. Red when run alone:
  `FAIL slow-time-sum exited 0 for a lane that started 1000 s ago, over its bound of 900 s, with 100.0 s in its timed parts (... slow-time lane 100.0 of 900, longest part slow-test262 25.0 of 600)`

**3. Suite run, after the cases existed**
`slot --timeout 30m task gate-selftest` ended with `gate-selftest: 88 passed, 4 failed`, exit 201. The 4 red:
- my 2 cases;
- `site-data-check-refuses-a-number-typed-into-engines-json` and `site-data-check-refuses-a-stale-status-file`, red on the base because `status.json` is stale.

`gate-selftest-refuses-a-script-that-writes-a-tracked-file` passed, so none of the scripts writes a tracked file.

**4. Findings (they cover 355c789b)**
1. **Test list too long for one argument.** `Taskfile.yml:379` turns every "other" test into one `-E` filter; `:398` and `:401` pass it as a single argument.
   - Measured: today the filter is 75,099 bytes for 996 tests. Linux refuses any single argument over 131,072 bytes. That limit is hit at about 1,738 "other" tests (mean name 65.4 characters).
   - When it is hit, go-task aborts the whole task with E2BIG. The task does not print its `run(s) red` line.
   - What reaches it: test growth. `swell::main` went from 816 tests (HEAD~100, 2026-11-24) to 1,259 (HEAD).
   - Origin introduced: the base passed no filter. Warning, NEEDS-CHANGE.
2. **An untimed part escapes the lane bound.** `taskfiles/gate-time.yml:286` adds up only the `slow-time` lines; it never reads `.out/gate-time/start`.
   - A part added to `slow` after `stack-profiles` with no `gate-time` line is in no sum.
   - A part added earlier without its own line is counted under the next part's name.
   - What reaches it: nothing found. All four parts have their `gate-time` line today.
   - Introduced. Note, INFEASIBLE.
3. **Collect mode dropped at 11, 17, 38, 64 and 128.** The unit's text (`Taskfile.yml:327`) names what it no longer runs but gives cost as the only reason. Story point 3 also asks why no defect of this kind hides there.
   - History says such defects do appear there. Of the eight size-red tests listed in story tests-hold-at-any-size-of-the-realm, seven were seen red in the collect mode only. The fix `5078361a` was red at 200 only, in the collect mode only for `termination`. The proxy test is red at 128 only.
   - So these defects show up at single sizes, and the collect mode now checks 2 sizes instead of 7.
   - The cost mostly comes from one test: `builtins_json_parse::parse_gives_back_its_charges_on_every_way_out` takes 21.5 s of the 22.7 s run at k=4.
   - Measured now: no test is red only in the dropped runs (see point 5).
   - Introduced. Warning, CONFIRMED. Whether to run them is the operator's call.

**5. What I attacked and could not break**
- **List rule against a full run.** I ran all 1,308 tests of `swell::main` at k = 4, 11, 17, 38, 64, 128 and 200 in both modes on 355c789b, one pool job per mode.
  - The only red test was `proxy::revoke_is_idempotent_and_drops_target_and_handler`, at 128 in both modes. The rule prints it as "room", and it was already red on the base.
  - The list drops no test that goes red by size.
- **Blind spots of the rule.** The `CollectReport` fields (`heap_bytes_after` and the like), nested `fn` inside a test, macros, calls into another test module, `ownKeys` or `getOwnPropertySymbols` on the global: a widened copy of the rule moves no test that was red into the list.
- **Nested modules.** Their tests print under a wrong name, and that name does not match any test. So they run at every size, which is the safe side.
- **Filter form.** `test(=…)` is an exact match, so a name that is the prefix of another, or holds a regex character, causes no wrong match. The counts agree: 312 run plus 997 skipped is 1,309.
- **Library unit tests.** Nothing under `crates/swell/src` reads `SWELL_TEST_REALM_FUNCTIONS`. Their 1 MiB budgets fail in the gate itself at the real realm size. The lane never padded them, before or now.
- **`slow-time-sum` at the edges.** 600.0 and 900.0 pass; 600.1 and 900.1 are refused. A raised `SLOW_*_BUDGET` is printed by `gate-vars`, as `GATE_TIME_BUDGET` is.
- **The unit's two selftests.** They read the real defaults. Against a copy whose refusal is removed, both print FAIL (`mutant.log`). A changed default is caught by neither, by design.
- **What the lane lost.** `test-collect` in the gate still runs the opt-level 1 collect profile at k=0, so no test is left that runs nowhere.

Not attacked, as the brief says: `taskfiles/stack-profiles.yml`, the modes of `test-realm-sizes`, and the workflow's path lists.

Commands and their result lines:
- default-mode run, 7 sizes: `k=128 default exit 100 … 1307 passed, 1 failed`; every other k `1308 passed`; `slot exit 0`
- collect-mode run, 7 sizes: `k=128 collect exit 100 … 1307 passed, 1 failed`; every other k `1308 passed`, k=200 in 93.3 s; `slot exit 0`
- my two cases, run alone: both `exit 1`, the output quoted in part 2
- the mutant run: both unit selftests `FAIL … exited 0`, exit 1
- `slot --timeout 30m task gate-selftest`: `88 passed, 4 failed`, exit 201

No test I added runs longer than 1 s.

**6. Written outside the worktree**
`<scratch>/wave2/slowlane/adversary-1/`, holding:
- scripts: `sizes.sh`, `cases.sh`, `mutant.sh`, `wide.awk`, `nested.awk`
- logs and lists: `sizes-default.log`, `sizes-collect.log`, `cases.log`, `cases2.log`, `mutant.log`, `gate-selftest.log`, `list.txt`, `wide.txt`, `nested.txt`
- copies of my two selftests

Inside the tree, the pool jobs wrote under the ignored `.out/`: `adv-*`, `adv-mutant/` and `gate-tests/`.

```findings
- file: Taskfile.yml
  line: 379
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: test-realm-sizes passes all 996 other tests to nextest as one 75,099-byte -E argument, and at about 1,738 such tests it passes Linux's 131,072-byte limit for one argument, so go-task aborts with "argument list too long" (tests/gate/test-realm-sizes-hands-nextest-a-long-list-of-tests.sh).
- file: taskfiles/gate-time.yml
  line: 286
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: slow-time-sum adds up only the slow-time lines and never reads the lane's clock in .out/gate-time/start, so a part added to task slow without its own gate-time line escapes the 900 s bound (tests/gate/slow-time-refuses-a-lane-whose-untimed-part-ran-over-its-bound.sh); no such part exists today.
- file: Taskfile.yml
  line: 327
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The collect mode now runs only at the two end sizes, yet seven of the eight known size-red tests and the termination test of 5078361a were red in the collect mode only and at single sizes, and the unit's only reason is a cost that is mostly one test (builtins_json_parse, 21.5 s of the 22.7 s run).
```

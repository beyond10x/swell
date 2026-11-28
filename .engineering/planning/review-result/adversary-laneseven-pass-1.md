---
format: aep.planning-md/3
id: review-result:adversary-laneseven-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit laneseven (wave big-2, lane unit 7): red, one case, five findings'
relations:
- reviews: story:gate-test-steps-hold-the-budget-on-the-slower-runner
revision: 1
---
Adversary pass 1 on the unit laneseven of wave big-2, on impl/laneseven at c47fc9e2 (base 9a8ad008), 2026-11-28.
The adversary's own report follows as it was given.

unit: laneseven, worktree `swell-impl-laneseven` at `c47fc9e2` (base `9a8ad008`)
verdict: red
cases: executed 113→114, red 1
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/laneseven/adversary-1/
needs-coordinator: yes (finding 1: either the story's equation or the step's lists has to change, and only the coordinator can decide which)

**1. `git --no-pager diff --stat`:** empty. `git status --short` is also empty. I took the planted case back out of the tree and kept a copy in my work directory.

**2. The case I added**

`tests/gate/test-lists-and-summed-line-count-the-same-tests.sh`, copy at `adversary-1/test-lists-and-summed-line-count-the-same-tests.sh`. It uses a stub cargo and builds nothing. It runs in 1.18 s alone. Its four sub-cases:
- **Sub-case 1, red.** The step must print 5 tests at opt-level 1 and 3 at opt-level 0, and 6 binaries (one of them has no tests). It must count both ignored tests once (one in `swell-num::main`, one in `swell-syntax::main`). Both `swell plain::outside_the_stack` and `swell::main plain::outside_the_stack` must be in the opt-level 1 run. And n plus m must equal p plus f plus i. Only that last assertion is red.
- **Sub-case 2, green.** With the pair `swell plain::outside_the_stack`, the library's test goes to opt-level 0 and the test of the same name in `swell::main` stays at opt-level 1.
- **Sub-case 3, green.** A JSON list that prints nothing fails with "cannot read the JSON", and no test runs.
- **Sub-case 4, green.** An opt-level 1 list that prints nothing fails, naming `swell-num::main num::adds is in neither run`, and no test runs.

Red output, the case run alone (`slot --fetch … sh tests/gate/test-lists-and-summed-line-count-the-same-tests.sh`, exit 1):
```
test-lists-and-summed-line-count-the-same-tests: FAIL 1: n plus m of "test: 5 tests at opt-level 1, 3 at opt-level 0" is 8, not p plus f plus i of the summed line (8 + 0 + 2 = 10), as the story's acceptance reads; the two lists leave the ignored tests out, the summed line counts them (.out/gate-tests/test-lists-and-summed-line-count-the-same-tests/split.out)
```

**3. Suite run, after the case existed**

`slot task gate-selftest` → `gate-selftest: 113 passed, 1 failed` (the one is my case), exit 201. `slot task kill-check` → `kill-check: no kill with two dashes after its signal operand in 152 file(s)`.

**4. Findings**

1. **Acceptance (`Taskfile.yml:1475` against `:1518`).**
   - Measured: the story's acceptance says n plus m equals p plus f plus i (2533). The head's gate prints 2187 + 345 = 2532 next to `19 test binaries: 2532 passed, 0 failed, 1 ignored`. My case shows the same gap: 8 against 10.
   - What reaches it: every gate run. The unit disclosed this.
   - Two ways out: the coordinator amends the equation, or the step lists all three runs with `--run-ignored all`. The second way would also bring the ignored tests under the each-test-once check. Origin: introduced.
2. **Mutant (`Taskfile.yml:1399`).**
   - Measured: I dropped `& not binary_id(=swell::main)` from the filter in a copy of the Taskfile. Every changed or new planted script still passes (mutant M4, exit 0 for each of the three it was run against). The stubs place `swell::main` tests by their names only.
   - With real nextest, the split check would also pass: all 106 stack tests of `swell::main` would move to opt-level 1 and still be in exactly one run. Only the guard catches it, at run time, when those tests panic.
   - What reaches it: an edit of `put()`; nothing in normal use. Origin: introduced.
3. **Boundary (`Taskfile.yml:1468`).**
   - Measured: a pair naming a test of a binary that did not move is accepted and does nothing. So is a pair naming a `swell::main` test that the rule prints "stack". The unit's own `test-runs-each-test-once-at-one-opt-level.sh` plants `swell-syntax::main sources::nests` and expects exit 0.
   - The story defines the variable as naming "tests of a moved binary", and the code does not enforce that.
   - What reaches it: an edit of the `TEST_OPT0_TESTS` line. Origin: introduced.
4. **Contract drift (`Taskfile.yml:1144`).** The summary of `build-tests` says it refuses a caller's value of `TEST_OPT1_ARGS`, `NO_STACK_TESTS` or `TEST_OPT1_PROFILE_TEST`. Since the unit, `test-opt1-vars-check` refuses `TEST_OPT0_TESTS` too, and the summary does not say so. What reaches it: a reader of `task --summary build-tests`. Origin: introduced.
5. **Acceptance (`report.md`).** Item 5 asks the report to list each changed and new script with its seconds run alone. `report.md` has none. The numbers exist in `head1.out` (0.13 to 1.54 s). What reaches it: the coordinator's check of the report. Origin: introduced.

**5. Attacked and not broken**
- **The JSON awk:** run on the real nextest 0.9.143 JSON, its output equals the oneline list (2532 lines), and the count it writes is `19 2533`.
- **The two filters:** the opt-level 0 filter is `not (…)` of the same expression, so it is the complement by construction. The written lines are 129,146 and 129,152 bytes.
- **Pairs:**
  - Right test with the wrong binary: refused by the "does not hold" check.
  - A pair naming a `swell::main` test printed "other": refused, "not in the run at opt-level 0".
  - An odd number of words, or a word with other characters: refused by the line check.
  - A test name that is a prefix of another's: no effect, because the filter matches `test(=` exactly.
  - The same pair twice: the counts stay consistent.
- **The cut, recomputed from `cut/*.tsv`:** 540 tests in each run, 37.16 and 6.01 CPU seconds, ratio 0.162 (bound 0.40). Every binary's sum matches the report. The moved tests equal the head's opt-level 1 list less the base's (1647 tests), exactly.
- **The build before the clock (probe job):**
  - A second `--no-run` of each run, and the JSON lists: 0 `Compiling` lines.
  - The test binary of the opt-level 1 run has `target/collect/swell-test262` compiled in, and the opt-level 0 one has `target/debug/swell-test262`.
  - The build seconds are confirmed from the logs: 824 s base, 814 s head. 6 and 7 crates are compiled at opt-level 1.
- **The profile check:** `CARGO_PROFILE_COLLECT_PACKAGE_SWELL_NUM_DEBUG_ASSERTIONS=false` and `CARGO_PROFILE_TEST_PACKAGE_SWELL_TEST262_OVERFLOW_CHECKS=false` are each refused with their line (exit 201).
- **`SWELL_NO_STACK_TESTS`:** nothing reads it in `crates/test262`, `crates/num` or `crates/swell/src`. No workspace member turns on a feature of `swell`, so building the packages together changes nothing.
- **The two rules:**
  - Stack rule: "other" goes from 1648 to 1665 and "stack" stays at 73.
  - Room rule: "other" goes from 1398 to 1415 and "room" stays at 323.
  - Only the 17 plane lines are added and none are removed. The 17 names in the base and head lists are the same.
  - The planes' step budget is a fixed limit per call (`ROUND_TRIP_STEPS`), not a reading of the realm's room.
- **Taskfile mutants caught by the unit's scripts:**
  - M1 (no `held TEST_OPT0_TESTS`)
  - M2 (no fix for a relative task path)
  - M3 (no `pair` in the exit)
  - M5 (i not less f)
  - M6 (ignored tests printed into the list)

**Not attacked**
- I planted no engine frame-size defect, because that rebuilds at both levels. By reading: the two pairs, the parser's tests and the `swell::main` stack tests still run unoptimized frames. `object::string::tests::no_buffer_is_uncovered_at_an_entry_across_a_call` now runs only at opt-level 1, and I did not establish whether a stack test at opt-level 0 reaches the re-entry limit through `Array.prototype.join`.
- The forge's job time and its 4-core wall time.
- The whole gate (`check-runner`).
- `test-binaries-check`: `main.rs` is unchanged.

**6. Paths written outside the worktree:** the directory `<scratch>/wave2/laneseven/adversary-1/`. It holds the case, `probe.sh`, `mutants.sh`, `timed.sh`, `rule-diff.sh`, their outputs, `probe/`, `base-uri/` and the derived lists. Inside the tree I also wrote `.out/adv1/` and `.out/gate-tests/<case>`, and removed both.

```findings
- file: Taskfile.yml
  line: 1475
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The new line's n plus m counts the lists without the ignored tests (2532) while the story's acceptance requires it to equal p plus f plus i of the summed line (2533), so the story or the lists must change.
- file: Taskfile.yml
  line: 1399
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Dropping the term that keeps swell::main out of the package part of the filter passes every planted script and the each-test-once check, and only the stack guard catches it at run time.
- file: Taskfile.yml
  line: 1468
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: A TEST_OPT0_TESTS pair naming a test of a binary that did not move, or a swell::main stack test, is accepted as a no-op although the story defines the variable as naming tests of a moved binary.
- file: Taskfile.yml
  line: 1144
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The summary of build-tests still names three variables of test-opt1-vars-check and leaves out TEST_OPT0_TESTS, which the check now refuses too.
- file: <scratch>/wave2/laneseven/report.md
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The unit's report does not list the changed and new planted scripts with their seconds run alone, which the acceptance of item 5 asks for.
```

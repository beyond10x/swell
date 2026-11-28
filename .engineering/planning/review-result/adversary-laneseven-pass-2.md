---
format: aep.planning-md/3
id: review-result:adversary-laneseven-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the unit laneseven: green, one note'
relations:
- reviews: story:gate-test-steps-hold-the-budget-on-the-slower-runner
revision: 1
---
Adversary pass 2 on the unit laneseven of wave big-2, on impl/laneseven at 971ac76a (correction 1, the diff c47fc9e2..971ac76a), 2026-11-28.
The adversary's own report follows as it was given.

unit: laneseven, correction 1, worktree `swell-impl-laneseven` at `971ac76a` (diff `c47fc9e2..971ac76a`)
verdict: green
cases: executed 114→114, red 0
origin: introduced 1, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/laneseven/adversary-2/
needs-coordinator: no

The correction holds against every input I tried. One mutant still gets past every check that runs before the tests (finding 1). It is a code edit, not something an input can reach, so I added no failing case.

**1. `git --no-pager diff --stat`:** empty. `git status --short` is empty too. I added no case to the tree this pass. Every mutant ran in a mirror under `.out/adv2/` or on the build node's copy of the tree, and `.out/adv2/` is removed.

**2. Cases:** none added. The taken case `tests/gate/test-lists-and-summed-line-count-the-same-tests.sh` was compared with my copy in `adversary-1/`:
- The only changes are the comment, a check that the planted JSON's test-count is 10, and the corrected equation. The equation is now n + m = p + f, and p + f + i = 10.
- Both equation checks are already pinned by the exact-line greps for `5 … 3` and `6 test binaries: 8 passed, 0 failed, 2 ignored`. They are redundant: they add a second red line but cannot fail on their own.
- Sub-cases 2 to 4 are unchanged, and the script passes on the head (M0).

**3. Suite run**
- `slot task gate-selftest` → `gate-selftest: 114 passed, 0 failed`, exit 0.
- `slot task kill-check` → `kill-check: no kill with two dashes after its signal operand in 152 file(s)`.

**4. Findings**

1. **Mutant (`Taskfile.yml:1436`, `:1441`, `:1442`).** The new string check of the filter line counts exact spellings only.
   - **M9 (two-line change):** a second package part `| package(swell)`, written with nextest's glob matcher, at the end of `put()`. It keeps `package(=` at 3, `binary_id(swell::main) & (` at 1 and the exclusion at 1.
   - **Real nextest lists (node job):** `task test-split` exits 0 and prints `test: swell::main 1770 at opt-level 1, 0 at opt-level 0` and `test: 2293 tests at opt-level 1, 239 at opt-level 0`. 73 tests the rule prints "stack" are in the opt-level 1 run.
   - **What would catch it:** only the guard at run time, when those tests panic. M9 also passes all six planted scripts.
   - **Fix (named, not applied):** count every `package(` and every `binary_id(` in the line, whatever the matcher.
   - **What reaches it:** an edit of `put()`, nothing in normal use. Origin: introduced.

**5. Attacked, not broken**
- **Item 1, filters that hold both strings once and are still wrong:**
  - M8 moves the exclusion around the whole expression. It passes all six planted scripts. With real lists it is refused before any test runs, because the profile test is no longer in the opt-level 1 run (`… is not in the run at opt-level 1`, exit 201).
  - An opt-level 0 filter built from another expression cannot happen: the same `put()` writes both lines.
  - M4 (exclusion dropped) is now refused by all six scripts.
  - No legitimate future name is refused: Rust test names cannot contain `(`, `&` or a space, and the pair check rejects parentheses.
- **Item 2, pairs:**
  - Packages are matched exactly. `swell-syntax::main` and `swell-regexp::main` are never read as binaries of `swell`.
  - A pair of the library `swell` is accepted and takes effect (sub-case 2 of the taken case passes).
  - A pair of a moved binary that the workspace list does not hold is refused after the lists (`test-runs-each-test-of-a-moved-binary-once.sh`, case 4).
  - The forms `--package=x`, `-p=x` and `-px` are not read. With `--package=swell-num` (M12), both `test-runs-each-test-of-a-moved-binary-once.sh` and the taken case go red, so the selftest catches it.
  - A caller's value is still refused first, before the pairs are read (`test-refuses-a-caller-value-of-its-run-at-opt-level-1.sh` passes on M0).
- **Item 3:** the changed plant in `test-runs-each-test-once-at-one-opt-level.sh` still catches M4, M5 and M6. M1, M2 and M3 are each caught by their own scripts.
- **Item 4:** yes, such an input exists. M11 removes the package-count check (M7) and adds the single fault `| package(=swell)`. It passes all six planted scripts. With real lists it passes the split just as M9 does, which is the same fault in the glob spelling. With the check present (M10) all six scripts are red. So that check is the only guard before the tests run, and only for the `=` spelling.
- **Item 5, dash:** the node has dash (`/usr/local/bin/dash`). `dash -n` exits 0 on the three changed scripts. The new lines use no `local`, arrays, `[[`, `echo -e` or process substitution.

**Commands and result lines**
- `slot sh .out/adv2/mutants2.sh`, exit 0, 13 mutants (full log in `adversary-2/mutants2.out`):

| Mutant | What it changes | Result |
|---|---|---|
| M0 | none | 6/6 scripts ok |
| M1, M2, M3 | as in pass 1 | red |
| M4 | exclusion dropped | 6/6 red |
| M5 | i not less f | caught by `test-runs-each-test-once-at-one-opt-level.sh` |
| M6 | ignored tests printed into the list | caught by `test-runs-each-test-once-at-one-opt-level.sh` and the taken case |
| M7 | package-count check removed | 6/6 ok |
| M8 | exclusion around the whole expression | 6/6 ok |
| M9 | second part `package(swell)` | 6/6 ok |
| M10 | second part `package(=swell)` | 6/6 red |
| M11 | M10 plus M7 | 6/6 ok |
| M12 | `--package=swell-num` | 2/2 red |

- `slot --timeout 30m --fetch .out/adv2/real sh .out/adv2/real2.sh`, exit 0:
  - M0: exit 0.
  - M8: exit 201, the profile test is not in the opt-level 1 run.
  - M9: exit 0, `swell::main 1770 at opt-level 1, 0 at opt-level 0`; 73 "stack" tests at opt-level 1.
  - `Taskfile.yml put back, git diff empty`.

**Not attacked**
- No whole gate run.
- No run of `task test` under M9 to show the guard's panics; I read the effect from the lists.
- No new failing case: every finding needs a code mutant, not an input.

**6. Paths written outside the worktree:** `<scratch>/wave2/laneseven/adversary-2/`. It holds `mutants2.sh`, `mutants2.out`, `dry.sh`, `dry/`, `real2.sh`, `real2.client`, `real/` and `selftest.out`.

```findings
- file: Taskfile.yml
  line: 1436
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The string checks of the written filter count only the spellings package(= and the swell::main exclusion, so a second package part written with the glob matcher passes them, every planted script and the real split with all 106 tests of swell::main at opt-level 1, and only the stack guard catches it at run time.
```

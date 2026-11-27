---
format: aep.planning-md/3
id: review-result:adversary-lanetests-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit lanetests: red, one warning and seven notes'
relations:
- reviews: story:realm-size-lane-within-its-share
revision: 1
---
Adversary pass 1 on `impl/lanetests` at `d693bdc` (wave big-2, the lane `slow` within its bound, third unit), run on 2026-11-27.
The report below is the adversary's last message; paths of the workstation are replaced.

unit: lanetests
verdict: red
cases: executed 68→68, red 0
origin: introduced 2, pre-existing 6, undecided 0
wrote-outside-worktree: <scratch>/wave2/lanetests/adversary-1/ (scripts, logs, transcripts, plant files, and a scratch clone whose `target` I deleted)
needs-coordinator: yes

**Verdict: red, because the old collect-mode counts now run in no lane.** No defect I planted was caught at an old count and missed at a new one. But `SWELL_FULL_COUNTS=1` no longer brings back the base's collect counts, so `slow-tests` runs the smaller counts too. The coordinator decides whether that is accepted or fixed.

## Findings

1. **The old collect counts run in no lane (introduced, warning).**
   - 12 of the 13 changed tests pick their new count from `cfg!(collect)` alone. In termination, the pins (`termination.rs:259`) also depend on that flag alone.
   - So `task slow-tests` runs the new counts. The base's collect counts run nowhere: host_133 2,000/256/50/48, for_await 10,000, async 100/64 KiB, property_index 50, tagged 100, new_target 3,000/60 and 40, the five map_and_set counts, and termination's fill without pins.
   - `for-coordinator.md` says "the default mode and SWELL_FULL_COUNTS=1 keep theirs". That is not what the code does.
   - What reaches it: `task slow-tests`.
2. **A reason given in the report is wrong (pre-existing test, note).** The report says `random_operations_match_the_model_list_of_the_clauses` (`builtins_map_and_set.rs:762`) "runs in slow-tests only". It is not ignored and no filter excludes it. It ran in the plain default run, 2.076 s alone, which is over the 1.5 s bound. In the collect build it took 2.140 s.
3. **churn is over the default bound (pre-existing, note).** `churn_without_live_entries_stays_bounded` (`builtins_map_and_set.rs:1187`) took 1.555 s alone in the default build, over 1.5 s. It was 2.4 to 2.6 s in the default runs of test-realm-sizes. The unit did not change its default count.
4. **host_206 does not test the queue's charge (pre-existing, note).** I set the queue's charge to zero (`async_generator.rs:492`, `* 0`). host_206 (`async_generators.rs:579`) stayed green at 40 and at 100 requests. Only settling went red: `489265 -> 489233` at 40, `751409 -> 751377` at 100. So the smaller count loses nothing there; the test never held it. It does go red at both counts with the property-map credit removed.
5. **host_133 checks no hooks (pre-existing, note).** All three transcripts have 0 hook lines: default, base collect and head collect.
   - The outcome kinds are the same at every corpus index between base collect and head collect: normal, three kinds of throw, an error that is a thrown value, StepBudget twice, MemoryBudget, `errors 1`, print, callBack normal and callBack throw.
   - What the collect run no longer does:
     - `SQUARES` runs 219 steps instead of 1,794.
     - The doubling string stops at 64 code units instead of 128 (196 steps instead of 598).
     - The memory loop runs 119 steps instead of 561.
     - The step budget is 300 instead of 2,000.
6. **A script with another extension runs nowhere (pre-existing, boundary, note).** `language_scripts()` (`language.rs:86`) reads only `*.js` files directly in `tests/language/`.
   - I planted a script with a wrong expected value as `zz-planted-wrong.js`. It went red in `part_18`.
   - The same script as `.mjs` ran nowhere and nothing went red.
   - A `.js` file cannot be left in no part, because each part takes every 47th file by position.
7. **The new.target split is 29 + 25, not 27 + 27 as reported (introduced, note).** All 54 cases are there and the prelude text is the same (`new_target.rs:665`).
8. **super_property could be split by form (pre-existing, note).** It takes 2.416 s alone in the collect build. It runs 4 forms one after another (`super_property.rs:1130`). Depth is what it checks, but splitting by form would keep the depth.

## What I checked

**Language parts (`b722d89`):**
- 286 scripts became 299: 13 originals each became `-break` and `-continue` halves.
- In each, 13 places split 7 + 6. Each half has no place of the other way out. The prelude is the same text, and each has 2 `check` calls.
- A script can't fall into no part, as finding 6 says.

**awk lists, base against head:** room 257 / other 996 became room 257 / 1046. The only new lines are the 47 language parts and 3 tests from the splits, all "other". No test moved from room to other.

**Language parts at other realm sizes:** at 4, 128 and 200, in both modes, 48 passed every time. No script reads the heap or global keys, and the 64 MiB budget is far away, so the awk's verdict is right for them.

**test-realm-sizes on the head (`d693bdc`):** the only red is `proxy::revoke_is_idempotent_and_drops_target_and_handler` at 128, in both modes.

**The surface split:** the three expected strings joined give the original string exactly.

**Second plants**, all in the collect build, first at the new count, then with the base's test files:

| Plant | New count | Base count |
|---|---|---|
| Interpreter step check `>=` changed to `>` (`budget.rs:28`) | for_await red `for_await.rs:690` "2001"; loops_that_add red `:1355` "501 steps of 500" | for_await red at 10,000 |
| The unit's off-by-one in `charge_steps` | for_await green | for_await green at 10,000 too, so not lost |
| The unit's nocredit plant | termination green | termination green at the old count too, so not lost; it was never red |
| Iterator's trace of its Map removed (`collection.rs:136`) | the_collector_keeps red `:546:42` Faulted at junk 50 | red at 200 |
| Property-map credit removed | red: host_206, settling, memory_budget_twice, arrow_boxes, budgets_around_new | the same 5 red |
| Freed record credited at most 4 KiB (`arena.rs`) | all green | all green |

**Times alone, one thread:**
- Collect build, the most among the changed tests: memory_budget_twice 1.808 s, churn 1.678 s, host_133 1.652 s. Over 2 s: super_property 2.416 s and random_operations 2.140 s.
- Default build: random_operations 2.076 s and churn 1.555 s are over 1.5 s.

**Cases added:** none. The worktree is clean.

## Commands and result lines

| Command | Result line |
|---|---|
| `SLOT_TIMEOUT=20m cargo nextest run ... --features collect-every-safe-point --test-threads 1` (changed tests and language parts) | Summary [58.223s] 68 tests run: 68 passed |
| the same in the default build | Summary [11.261s] 68 tests run: 68 passed |
| `SWELL_TEST_REALM_FUNCTIONS=4/128/200`, language parts, both modes (6 runs) | 48 tests run: 48 passed, each run |
| `SWELL_DETERMINISM_CHILD=1 ... host_133 --no-capture`: head collect, head default, base collect | exit 0 each; 57/58/57 transcript lines |
| plant runs in the clone, `p0` to `p6`, head and base test files | `p0` exit 0; `p1`–`p5` exit 100; `p6` exit 0 |
| language plant, default build | 48 tests run: 47 passed, 1 failed (`part_18`) |
| `slot --timeout 30m task test-realm-sizes` | realm-sizes: 2 run(s) red (exit 201) |

## Not attacked

- No second plant of mine aimed at host_133, property_index, tagged_templates or churn turned them red. The general plants were green on them, and I made no plant specific to each.
- I did not measure CPU seconds (only wall time with one test thread). I did not check times under `SWELL_FULL_COUNTS=1`.
- I did not judge the D2 test-second figures.

The deleted target directory: `<scratch>/wave2/lanetests/adversary-1/clone/target`

```findings
- file: crates/swell/tests/termination.rs
  line: 259
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: No changed count reads SWELL_FULL_COUNTS (the 12 new counts and termination's pins depend on cfg!(collect) only), so the base's collect counts run in no lane, against for-coordinator.md's claim that SWELL_FULL_COUNTS=1 keeps them.
- file: crates/swell/tests/builtins_map_and_set.rs
  line: 762
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: random_operations_match_the_model_list_of_the_clauses is not slow-tests only as the report says; it runs in the plain default run at 2.076 s alone (over 1.5 s) and 2.140 s in the collect build.
- file: crates/swell/tests/builtins_map_and_set.rs
  line: 1187
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: churn_without_live_entries_stays_bounded took 1.555 s alone in the default build, over the 1.5 s bound, with its default count unchanged.
- file: crates/swell/tests/async_generators.rs
  line: 579
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: With the queue charge set to zero in async_generator.rs:492, host_206 stays green at 40 and at 100 requests, so it never held the queue's charge and the smaller count loses nothing.
- file: crates/swell/tests/determinism.rs
  line: 804
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: The host_133 transcripts hold 0 hook lines in the default, base-collect and head-collect runs, so same hooks holds nothing; the outcome kinds are the same at every index before and after the smaller counts.
- file: crates/swell/tests/language.rs
  line: 86
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: A script with a wrong expected value named .mjs in tests/language runs in no part and nothing goes red, while the same script as .js goes red in part_18.
- file: crates/swell/tests/new_target.rs
  line: 665
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The new.target parse cases are split 29 and 25, not 27 and 27 as the report says; all 54 are present with the same prelude.
- file: crates/swell/tests/super_property.rs
  line: 1144
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: super_re_enters_the_interpreter_on_the_promised_stack takes 2.416 s in the collect build and runs four forms one after another; a split by form would keep its depth, so depth alone does not justify the overrun.
```

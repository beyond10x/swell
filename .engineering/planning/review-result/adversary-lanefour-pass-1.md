---
format: aep.planning-md/3
id: review-result:adversary-lanefour-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit lanefour
relations:
- reviews: story:realm-size-lane-within-its-share
revision: 1
---
Adversary pass 1 on impl/lanefour at f9ddc9e (wave big-2, unit lanefour).
Verdict red: 97 cases executed, 1 red; 2 warnings, 3 notes.

unit: lanefour
verdict: red
cases: executed 91→97, red 1
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/lanefour/adversary-1/ (7 clones: mut, lane, realm, roots, ab, gate, sel; their `target/` removed; scripts and logs)
needs-coordinator: yes. Decide whether to take the cases. The red gate script would turn `gate-selftest` red if it is committed as it is.

Nothing in either commit gives a wrong match, a lost root, or a test that runs in neither build. The memo answered the same as a search without it in every order and form I tried. The collect-mode split is exact. The verdict is red because of my one failing gate case (F3), and because the unit's selftest misses a defect that would silently drop every stack test (F5). Neither is a blocker.

## Findings

**F1** (`80b6693`, mutant, note): `crates/regexp/src/program.rs:195`. If a class under `i`, or one with `\s` or `\S`, keeps its answer for one code point too many (`(…, c, c.saturating_add(1))`), all 91 of the unit's and the two crates' tests still pass. Only my `adv_a_class_answers_as_a_search_of_the_character_alone_in_every_order` fails (exit 100). Reaches: every `/i` class through the matcher. The code itself is correct.

**F2** (`80b6693`, mutant, note): `crates/unicode/src/property.rs:215`. If the answer above U+10FFFF for `Any`, `ASCII` and `Assigned` starts at U+10FFFF instead of U+110000, the unit's test still passes: for such code points it checks only that the range contains the code point asked. My `adv_contains_around_kept_as_a_memo_answers_as_contains_in_three_orders` fails on it. Reaches: the public `Property::contains_around`; the matcher never asks above U+10FFFF.

**F3** (`f9ddc9e`, boundary, warning): `taskfiles/realm-room-tests.awk:99`. `-v rule=stack` prints "other" for a stack test in three forms:
- it calls a helper of another test module by its path;
- it calls a top-level helper whose name an earlier nested fn of the same file also has (the first definition wins);
- it reaches the helper through a function pointer.

Such a test would run at opt-level 1 only. The failing case is `tests/gate/slow-tests-stack-rule-finds-a-stack-test-in-every-form.sh` (exit 1, three FAIL lines). No test in the tree has these forms today: with every same-name definition counted, the rule gives the same 67.

**F4** (`f9ddc9e`, contract-drift, note): `Taskfile.yml:285`. The task prints "they run at opt-level 0, the 1389 others at opt-level 1". In fact 933 of them run at opt-level 1. The other 456 are in modules outside the lane's collect filter and run in neither build, as before this change.

**F5** (`f9ddc9e`, mutant, warning): `tests/gate/slow-tests-runs-the-stack-tests-at-opt-level-0.sh:101`. I planted three defects in the task's filter (in `Taskfile.yml`); the selftest caught one:

| Planted defect | Effect | Selftest |
|---|---|---|
| stack names dropped from the opt-level 1 filter (line 301) | stack list empty | red |
| `--ignore-default-filter` on the opt-level 1 run (line 325) | stack tests run in both builds | green |
| `filter="$lib"` for the opt-level 0 run (line 314) | every stack test runs in neither build | green |

It checks each run's default-filter names but not its `-E` filter. Reaches: the lane, which stays green with the third defect, because the library's 273 tests meet `MIN_PASSED`.

## Cases added (uncommitted)
- `crates/regexp/tests/property_escapes.rs`, all passing:
  - `adv_a_class_answers_as_a_search_of_the_character_alone_in_every_order` 0.236 s
  - `adv_a_class_in_a_lookbehind_answers_as_a_search_of_the_character_alone` 0.305 s
  - `adv_a_class_asked_again_by_backtracking_answers_as_a_search_of_the_character_alone` 0.126 s
  - `adv_two_classes_asked_in_turn_on_neighbouring_code_points_answer_for_themselves` 0.097 s
- `crates/unicode/tests/property.rs`: `adv_contains_around_kept_as_a_memo_answers_as_contains_in_three_orders` 0.541 s, passing.
  - 20 properties, each asked in three orders: ascending, descending and a linear congruential shuffle.
  - Sample: every code point c with c % 61 == 0, plus one before, at and one after both ends of every range of the property.
  - Code points not asked are covered only through the ranges the memo keeps for them, each checked against the property's own range list.
- `tests/gate/slow-tests-stack-rule-finds-a-stack-test-in-every-form.sh`: red, 0.014 s.

Times are nextest wall times of single-threaded tests, all under 1.5 s.

## Commands and results
- `cargo nextest run --locked -p swell-unicode -p swell-regexp` (worktree): `96 tests run: 96 passed`.
- Item 8, `sh adversary-1/mutants.sh` (clone `mut`), one plant per run:
  - MA (F1) and MB (F2): only my cases are red.
  - The memo not reset between two classes (`known.class == class` removed): red in 5 tests, including the unit's `a_class_answers_each_character…`.
  - Either end of the gap between a class's own ranges off by one: red in 3 existing matcher tests and in mine (the unit's own test misses both).
  - Three plants in `swell-unicode`'s range lookups: the unit's `contains_around_answers_as_contains_over_its_whole_range` is red.
  - "The memo keyed without the negation" cannot occur as written: the key is the class number, and `[x]` and `[^x]` get different numbers. MA is my closest variant.
- `slot --timeout 30m --fetch .out/adv sh adv-l1.sh` (clean clone `lane`):
  - `task slow-tests` exit 0 after 285 s; `371 passed` at opt-level 0 and `964 passed` at opt-level 1.
  - `cargo nextest list` of both builds and of the old collect run: union = old = 1304, intersection = 31.
  - The 31 in both builds are the tests macros write: 11 `builtins_array_iteration`, 17 `builtins_uri`, 3 `proxy`.
  - All 67 stack tests are in the opt-level 0 list and none in the opt-level 1 list. The opt-level 0 list is 273 library tests + 98 `swell::main`.
- `adv-l2.sh` (clone `lane`), three runs of `task slow-tests`, each exit 201:
  - a failing test in the opt-level 1 share: `FAIL … builtins_map_and_set::finished_loops_leave_no_cursor`;
  - a failing test in the opt-level 0 share: `FAIL … limits_scripts::min_stack_bytes_is_enough_through_builtins`, `370 passed, 1 failed`;
  - the profile collect broken: `profile collect inherits from no-such-profile` and `opt-level 1: read no "Starting …" line`.
- `adv-l5.sh` (clean clone `realm`): `task test-realm-sizes` exit 201 after 291 s.
  - `build-realm-sizes: Finished test … 22.10s` and `Finished collect … 3m 05s`.
  - Both planted tests fail at all 7 sizes; the task prints `realm-sizes: 14 run(s) red`.
  - A part run alone prints no `slow-time` line; only `task slow` does, by design.
- Bounds grep (`git diff 158888d f9ddc9e -- Taskfile.yml taskfiles .github`): the hits are prose and the new count line; no bound, timeout or count was raised.
- Workflow: `.cargo/collect-profile.toml` is in both path lists (pull_request and push), so a pull request that changes only that file runs the lane. Read only, not run on the forge.
- Item 4, `adv-roots.sh` (clone `roots`, two missing roots planted: `Tracer::value` and the kept values of running builtins):
  - the same 26 tests fail at opt-level 0 and at opt-level 1;
  - the profile collect compiles with `-C debug-assertions=on -C opt-level=1`, and `cfg debug_assertions` is set;
  - overflow checks stay on (inherited from `test`; cargo passes no flag because on is already the default with debug assertions);
  - no test reads `cfg!(debug_assertions)`.
- Item 2, `adv-ab.sh` (clone `ab`): the runner built at `f9ddc9e` and with the four sources of `158888d` put back, run over 20 other generated property-escape files.
  - `40 Passed` for each build;
  - `runs-*.tsv`, stdout and the 20 `test262-steps` lines are equal between the two.
- `slot --timeout 30m task check-runner` (clone `gate`, with the unit's `expected.digest.new`): exit 0.
  - `gate-selftest: 100 passed, 0 failed`
  - `gate-time test-steps 53.6 of 120, cpu 219.8, load 4.80 on 16 cores`
  - `gate-cpu test-steps 219.8 of 245 on 16 cores`
- `adv-3c.sh` (clone `sel`): the three planted filter defects of F5; the unit's selftest is clean `ok`, red on the first, green on the other two.
- The unit's new selftest alone: exit 0, wall 0.335 s. The unit's two Rust tests: 0.135 s and 0.117 s. I found three new tests, not four.

## Not attacked, or only read
- Read only, not run:
  - two matches interleaving on one RegExp, or one RegExp used from two realms: the machine is made per `search`/`match_at` call (`matcher.rs:153,171,195`) and `Known` is a plain `Copy` field;
  - no `Cell`/`RefCell` was added, `Send`/`Sync` are unchanged, and nothing is allocated or charged;
  - classes under `v` with set operations do not use the memo (`SetClass`); only a bare `\p{L}` under `v` was tested.
- The engine's determinism tests were compared only in their green runs in both collect builds (L1), not against `158888d`.
- Item 4 used two planted missing roots instead of five fixes from history: `git log` shows no commit that fixes a missing root found by the collect mode, and `5078361a` fixes a test.
- I did not run anything on the forge, or `task slow` as a whole.
- Seen while running L1: the library's tests in the opt-level 0 part take 13.9 s and 11.3 s under `SWELL_FULL_COUNTS=1`. That was there before this change, and I did not attack it.

```findings
- file: crates/regexp/src/program.rs
  line: 195
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: A class under i or with \s or \S that keeps its answer for one code point too many passes every test of the unit and of both crates' suites; only adv_a_class_answers_as_a_search_of_the_character_alone_in_every_order fails on it.
- file: crates/unicode/src/property.rs
  line: 215
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: contains_around above U+10FFFF may give a range that reaches U+10FFFF for Any, ASCII and Assigned and the unit's test still passes, since it checks only that the range holds the code point asked; adv_contains_around_kept_as_a_memo_answers_as_contains_in_three_orders fails on it.
- file: taskfiles/realm-room-tests.awk
  line: 99
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: rule=stack prints other for a stack test that reaches stack_with_room through a helper of another test module, through a top-level helper whose name an earlier nested fn shares, or through a function pointer, so such a test would run at opt-level 1 only; no test of the tree has these forms today.
- file: Taskfile.yml
  line: 285
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The printed line says the 1389 other tests run at opt-level 1, but 933 run there and 456 run in neither collect build because their modules lie outside the lane's collect filter.
- file: tests/gate/slow-tests-runs-the-stack-tests-at-opt-level-0.sh
  line: 101
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The selftest stays green when the opt-level 0 run takes the library's tests alone (every stack test in neither build) or the opt-level 1 run ignores its default filter, because it checks the default-filter names but not the -E filter of either run.
```

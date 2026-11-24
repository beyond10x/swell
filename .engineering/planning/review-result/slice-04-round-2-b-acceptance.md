---
format: aep.planning-md/3
id: review-result:slice-04-round-2-b-acceptance
kind: review-result
status: active
title: 'Slice 4, round 2, the acceptance critic: the twelve later stories'
relations:
- reviews: story:swell-num-date-text
- reviews: story:math-sum-precise
- reviews: story:number-functions
- reviews: story:swell-num-math-exp-and-log
- reviews: story:swell-num-math-trigonometry
- reviews: story:date-setters
- reviews: story:math-functions
- reviews: story:date-text-formats
- reviews: story:exponentiation-operator
- reviews: story:math-random
- reviews: story:slice-04-conformance-close
- reviews: story:spec-scenarios-for-the-numbers-and-dates-requirements
revision: 1
---
needs-revision (panel B)

I applied the brief's rule: a story is `needs-revision` only on a blocker. Warnings do not hold a story back.

- `swell-num-date-text`: approve
- `math-sum-precise`: needs-revision
- `number-functions`: approve
- `swell-num-math-exp-and-log`: approve
- `swell-num-math-trigonometry`: approve
- `date-setters`: approve
- `math-functions`: approve
- `date-text-formats`: needs-revision
- `exponentiation-operator`: approve
- `math-random`: approve
- `slice-04-conformance-close`: approve
- `spec-scenarios-for-the-numbers-and-dates-requirements`: approve

Findings. File lines are lines of the story files.

1. **math-sum-precise, line 59 (blocker).** The acceptance needs `slot task libm-check` to exit 0, but that task is created by `swell-num-math-double-double`. The story has no edge to it, and it sits in wave 2 beside it. On the wave worktree, `Taskfile.yml` and `taskfiles/` define no `libm-check`. Add `depends_on: story:swell-num-math-double-double`, or replace the task with the story's own grep for forbidden methods.
2. **date-text-formats, line 66 (blocker).** The acceptance says the pending count prints 5, with 6 before, and `4:5` against `4:6`. Those are the counts before `math-functions` merges. In the wave order (`math-functions` in wave 4, this story in wave 5) the count before is lower, and two higher again on `wave/s10-1` (HOST-212 and HOST-213). Say "one less than before", as `math-functions` does. The same change is needed at line 58.
3. **math-sum-precise, line 63 (warning).** The text says "23 cases, 46 runs", but the command copies the whole `test/built-ins/Math`, which is 327 cases. 23 is the 10 files of `Math/sumPrecise` plus the 13 guard cases. Change the directory argument to `test/built-ins/Math/sumPrecise`, or state 327.
4. **math-random, line 75 (warning).** The text says "20 cases, 40 runs" and `B 13`. The command copies `Math`, `Object/getOwnPropertyDescriptor` and `property-accessors` whole, which is 327 + 310 + 21 cases. 20 is `Math/random` (5) plus the guard (13) plus 2 list cases. Either use `test/built-ins/Math/random` with the 2 outside cases listed, or restate the counts, including `B`.
5. **exponentiation-operator, line 82 (warning).** The text says "1027 cases" and `B 944`, but the listed directories hold about 5,900 cases (`Array/prototype` alone is 2,812, `object` 1,170). 1027 is 76 + 944 + 7, so only the `exponentiation` directory is meant. Restate the counts, or cut the directory arguments to the one directory. The printed `passed 944 bound 944 B 944` cannot appear.
6. **exponentiation-operator, line 45 (warning).** The list holds 22 `Array/prototype` cases, not 20, and `reverse` (1) is missing from the named ones. The named parts sum to 74, not 76.
7. **exponentiation-operator, line 43 (note).** The parser lines 391 and 465 are of the older base. On the wave worktree they are 412 and 486. The other quoted lines (`ast.rs:105`, `bytecode.rs:295`, `dispatch.rs:334`, `operators.rs:21`) match.
8. **date-text-formats, line 32 (warning).** "Date/prototype 99" is wrong. The list file has 90 there, and 99 is `prototype` + `parse` + `UTC`. As written the directories sum to 174, not 165.
9. **date-text-formats, line 36 (warning).** The tests assert `Date.parse(d.toLocaleString())` equals `d`. The design on the wave worktree (`engine.md` about line 2349) says the `toLocale*` output is NaN to `Date.parse`. The acceptance has no start gate that shows the design sentence is written. Add the `grep -n` that shows it.
10. **number-functions, line 39 (warning).** The `parseInt` test table "follows the written sentence", but no sentence exists yet. The story's Writes line says the design author writes it, outside the unit. Add a start gate: a `grep` that prints the sentence in `engine.md` decision 6.
11. **swell-num-math-exp-and-log and swell-num-math-trigonometry, line 32 of each (warning).** The one-file-per-story decision is not in the design either. The design still names `data/math/reference.txt` (`engine.md` about line 1989). Add the same start gate.
12. **swell-num-math-exp-and-log, line 62 (note).** It hard-codes `math-data-check: 1 file(s)`. This is false if the trigonometry story merges first. The trigonometry story says "one more than before", which is right.
13. **math-random, line 63 (warning).** The pending count is "lower by 5 (by 6 if HOST-147 moves)". This is the open point 1, and the acceptance should not hold two outcomes. State 5, and let HOST-147 move only if `grep -n "HOST-147" docs/design/host-interface.md` shows the `Map` half split off.
14. **math-random, lines 55 and 73 (warning).** `host_147` is allowed 5 s, but the Time clause bounds every new test at 1.5 CPU seconds. Say which one governs.
15. **math-functions, lines 32, 51, 53 and 54 (warning).** The story says "21 functions", but the names it lists are 22, and it writes 22 variants. The HOST-135 test says "every one of the 21". The tests say "the 22". Pick 22.
16. **math-functions, line 53 (warning).** It says the HOST-135 test runs "on every release platform in the forge's matrix". `.github/workflows/` has no matrix, only `ubuntu-latest`. The acceptance names only local runs, so the platform claim is not checked. Add the platform job, or say HOST-135 is held on the forge's one platform and this host.
17. **date-setters, line 59 (warning).** The text says "guard empty" and `B 0`, which holds only at `c6d44d1`. On the join the clock and zone stories have already lifted about 362 `Date` cases. The same applies to the printed `passed … bound … B …` line in `number-functions` (72), `math-functions` (72), `date-text-formats` (70) and `math-sum-precise` (63). Only the `failing` line count and the exit status are stable.
18. **swell-num-date-text, line 53 (warning).** The Time clause asks for "under 2 with `collect-every-safe-point`" and "each language script under 0.5 s". `swell-num` has no such feature and runs no script. The same holds for `swell-num-math-exp-and-log` (line 63) and `swell-num-math-trigonometry` (line 63). Say "not applicable in `swell-num`".
19. **spec-scenarios, line 67 (warning).** `grep -n "HOST-209"` over the three files is said to print "one line in each". `host-interface.md` already prints 3 lines (1979, 2040, 2143). Use `grep -c "HOST-209 to HOST-213"`.
20. **spec-scenarios, lines 55 and 67 (warning).** The text says "at least 14 passed" and an unsupported maximum of 44. The wave worktree's `conform-engine.yml` has 18 and 42. The floor must be 18 plus the new scenarios.
21. **spec-scenarios, line 53 (note).** "Prints nothing or a rewritten sentence" does not tell before from after. Name the sentence the unit writes.
22. **Filter repeats (note).** The `-E` filter repeats the `host_186` through `tracing::` alternatives twice in `math-sum-precise` (59), `math-functions` (68), `exponentiation-operator` (78) and `math-random` (71).

Open points. Point 1 is finding 13. Point 2 is finding 9. Point 3 is finding 10. Point 4 is finding 11. Point 6 (`slice-04-conformance-close` depends on `slice-10-conformance-close`) is acceptable. The acceptance has a before and after (`all 0` to at least `all 3`), and every check is a command.

Other line numbers of the older base, with no finding: in `math-sum-precise`, `get_iterator` is at 91 (not 89), `iterator_step_value` at 216 (not 173) and `iterator_close` at 241 (not 198). In `math-functions`, the 12,500-call loop test is at line 942 (not 746).

What I read: the brief, the critic procedure and rubric, round-1's acceptance result, all 12 panel B stories in full, and the `docs/plan/slice-04` lists and the two scripts, `part.sh` and `check.sh`. I also read the wave worktree (taskfiles, requirements, design, workflows, tests) and counted test262 files in the pinned checkout. I did not run `aep plan artifact show`; I read the story files directly. No build, test or network.

Not established: the exact printed `B` values on a join base (I did not run `check.sh`), the 543 count of `number-functions` (my count is 553, which may be the runner's exclusions), and whether the Math timings hold. Panel A stories I read only for edges (`date_clock`, `date_zone`, the `swell-num-math-double-double` gates). The shared-file hunks of `math.rs` and `lib.rs` belong to the parallel-safety critic.

```findings
- file: .engineering/planning/story/math-sum-precise.md
  line: 59
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the acceptance needs `slot task libm-check` to exit 0, but that task is made by swell-num-math-double-double, which the story has no edge to, so the task does not exist on its base"
- file: .engineering/planning/story/date-text-formats.md
  line: 66
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the acceptance says the pending count prints 5 (6 before) and `4:5` against `4:6`, which are the counts before math-functions merges, so it is false in the wave order and should read one less than before"
- file: .engineering/planning/story/math-sum-precise.md
  line: 63
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the text says 23 cases and 46 runs but the part.sh command copies the whole built-ins/Math (327 cases), so the directory argument or the count is wrong"
- file: .engineering/planning/story/math-random.md
  line: 75
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the text says 20 cases, 40 runs and B 13 but the part.sh command copies Math, Object/getOwnPropertyDescriptor and property-accessors whole (658 cases), so the printed line cannot appear"
- file: .engineering/planning/story/exponentiation-operator.md
  line: 82
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the text says 1027 cases and B 944 but the listed directories hold about 5,900 cases, so the printed check line cannot appear"
- file: .engineering/planning/story/exponentiation-operator.md
  line: 45
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the list has 22 Array/prototype cases not 20 and omits reverse, so the named parts sum to 74 not 76"
- file: .engineering/planning/story/exponentiation-operator.md
  line: 43
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the parser lines 391 and 465 are of the older base and are 412 and 486 on the wave worktree"
- file: .engineering/planning/story/date-text-formats.md
  line: 32
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Date/prototype is given as 99 but the list file has 90 (99 includes parse and UTC), so the directory counts sum to 174 not 165"
- file: .engineering/planning/story/date-text-formats.md
  line: 36
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the tests assert Date.parse(d.toLocaleString()) equals d while the written design says it is NaN, and the acceptance has no start gate showing the design sentence is written"
- file: .engineering/planning/story/number-functions.md
  line: 39
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the parseInt test table depends on a design sentence that does not exist yet and the acceptance has no command that shows it is written"
- file: .engineering/planning/story/swell-num-math-exp-and-log.md
  line: 32
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the one-file-per-story data decision is not in the design (which names data/math/reference.txt) and the acceptance has no start gate that shows it is written"
- file: .engineering/planning/story/swell-num-math-trigonometry.md
  line: 32
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the one-file-per-story data decision is not in the design (which names data/math/reference.txt) and the acceptance has no start gate that shows it is written"
- file: .engineering/planning/story/swell-num-math-exp-and-log.md
  line: 62
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the acceptance hard-codes `math-data-check: 1 file(s)`, which is false if the trigonometry story merges first, where the trigonometry story says one more than before"
- file: .engineering/planning/story/math-random.md
  line: 63
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the pending count is lower by 5 or by 6 depending on a design split, so the acceptance holds two outcomes and should state 5 with a command that shows the split"
- file: .engineering/planning/story/math-random.md
  line: 55
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "host_147 is allowed 5 s while the Time clause bounds every new test at 1.5 CPU seconds, so the two bounds conflict"
- file: .engineering/planning/story/math-functions.md
  line: 32
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the story says 21 functions but lists and writes 22, and the HOST-135 test says 21 where the other tests say 22"
- file: .engineering/planning/story/math-functions.md
  line: 53
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the test is said to run on every release platform in the forge's matrix but the workflows have only ubuntu-latest and the acceptance names only local runs, so HOST-135 is not checked across platforms"
- file: .engineering/planning/story/date-setters.md
  line: 59
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the empty guard and `B 0` are of c6d44d1 and are false on the join after the clock and zone stories, and the same printed passed/bound/B line is of the old base in number-functions, math-functions, date-text-formats and math-sum-precise"
- file: .engineering/planning/story/swell-num-date-text.md
  line: 53
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the Time clause asks for measurements with collect-every-safe-point and per language script, which swell-num does not have, so it cannot be met (same in swell-num-math-exp-and-log line 63 and swell-num-math-trigonometry line 63)"
- file: .engineering/planning/story/spec-scenarios-for-the-numbers-and-dates-requirements.md
  line: 67
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "grep -n HOST-209 is said to print one line in each of three files but host-interface.md already prints 3 lines, so use grep -c on the list string"
- file: .engineering/planning/story/spec-scenarios-for-the-numbers-and-dates-requirements.md
  line: 55
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the floor of 14 passed and the unsupported maximum of 44 are of the older base while conform-engine.yml has 18 and 42"
- file: .engineering/planning/story/spec-scenarios-for-the-numbers-and-dates-requirements.md
  line: 53
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the check that the grep prints one line before and either nothing or a rewritten sentence after does not tell before from after"
- file: .engineering/planning/story/math-sum-precise.md
  line: 59
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the -E filter repeats its host_186 through tracing alternatives twice (same in math-functions line 68, exponentiation-operator line 78, math-random line 71)"
```

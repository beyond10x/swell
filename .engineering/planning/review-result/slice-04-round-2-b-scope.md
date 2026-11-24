---
format: aep.planning-md/3
id: review-result:slice-04-round-2-b-scope
kind: review-result
status: active
title: 'Slice 4, round 2, the scope critic: the twelve later stories'
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

Per story:
- `story:swell-num-date-text` — approve
- `story:math-sum-precise` — approve
- `story:number-functions` — needs-revision
- `story:swell-num-math-exp-and-log` — approve
- `story:swell-num-math-trigonometry` — approve
- `story:date-setters` — approve
- `story:math-functions` — needs-revision
- `story:date-text-formats` — needs-revision
- `story:exponentiation-operator` — needs-revision
- `story:math-random` — approve
- `story:slice-04-conformance-close` — approve
- `story:spec-scenarios-for-the-numbers-and-dates-requirements` — needs-revision

Findings:
1. **spec-scenarios, blocker.** The epic asks for no ESS scenario, no change to the spec model and no change to the ESS target. The story adds scenarios under `spec/scenarios/`. It also edits `spec/domains/host.yaml`, `host-interface-model.md`, `conformance/ess/src/target.rs` and the floor in `taskfiles/conform-engine.yml`.
   - The swell `AGENTS.md` (in the wave tree, "Rules", lines 134-141) says "ESS is not extended for swell, and the model is not grown to chase coverage". It also says no limit of ESS is filed and no agent works around a refusal to get one more construct expressed.
   - The story's Change line 55 adds steps to the target. Its Out of scope calls a limit "a request to the ESS maintainers".
   - It names all 14 requirements, but the same `AGENTS.md` says every requirement of a slice is named by exactly one of its stories. Nine of the 14 are moved by other stories (HOST-26, 27, 125 to 128, 144, 209 to 213); HOST-135 and HOST-147 are named for the move or for staying pending by `math-functions` and `math-random`. Its scenario files also need an outside policy admission before any commit, so it cannot finish on its own.
   - Fix: remove it from slice 4, or cut it to bookkeeping with no target or ESS change, and drop the "request to the ESS maintainers" sentence. The zone story's Out of scope (line 75) points at it and must change with it.
2. **math-functions, warning.** The summary (line 7), "What later stories reuse" (line 42) and the HOST-135 bullet (line 53) say 21 functions. The list, the variants and the scan have 22. Make every mention 22.
3. **date-text-formats, warning.** Context line 32 says `Date/prototype` 99. The list has 90, which gives 165 with the rest; the stated breakdown sums to 174. Correct it to 90.
4. **date-text-formats, warning.** The tests (line 52) assert `Date.parse(d.toLocaleString())` equals `d`. Design `engine.md` lines 2349-2350 say `Date.parse` of `toLocale*` output is NaN "unless a later decision" says otherwise. That decision is not written (open point 2).
   - The recommendation stays within the epic: it prints the `toString` text and so adds no locale behaviour.
   - Before acceptance, either the design author writes the sentence, or the story drops the round-trip assertion for the `toLocale*` methods and keeps only length, name and descriptor.
5. **exponentiation-operator, warning.** Context line 45 says `built-ins/Array/prototype` 20. The list has 22 (it includes one `reverse` case). The breakdown sums to 74, not 76. Correct it.
6. **number-functions, warning.** `Writes` (line 64) names `docs/design/engine.md`, but the unit does not write it. The `parseInt` precondition (line 39) is still not in the design.
   - Remove the file from `Writes` and state the sentence as a prerequisite owned by the design author.
   - Open point 3 is acceptable as drafted. The method is deterministic on every platform and inside the draft's allowance.
7. **Epic, warning.** The Outcome (line 21) names no operator `**` and no `Math.sumPrecise`. `exponentiation-operator` and `math-sum-precise` trace only to design item 4 and the `dir test/built-ins/Math/ 4` line. The Outcome also names no `Number` predicates (`isFinite`, `isInteger`, `isNaN`, `isSafeInteger`) and no `Date` setters. The coordinator's rewrite of the epic should add them.

Open points:
- **1, HOST-147.** Acceptable as drafted. `math-random` writes the two-process test and leaves the line pending, and the close handles both cases (0 pending, or 1 named). The slice does not close unless the design splits the requirement, so ask for the split before the close starts.
- **3, `parseInt` radix.** See finding 6.
- **4, reference data split.** The design still names one file, `data/math/reference.txt` (`engine.md` lines 1989 and 2003). The design must change before either `swell-num-math` story starts.
- **6, close depends on slice 10 close.** Correct. It is `active`, it writes the same five files, and delivery order puts 10 before 4.

Not findings:
- **Line numbers.** The wave tree differs from the base the stories cite (`c6d44d1`), and the stories anchor by name. Examples:
  - `iterator_step_value` is at 216 in the wave tree, not 173.
  - `iterator_close` is at 241, not 198.
  - The parser refusals are at 412 and 486, not 391 and 465.
  - The `Instruction` stack-effect list is at 790, not 723.
  - The `a_loop_of_builtin_calls...` test is at 942, not 746.
  - `create_global_functions` is at 146, not 144.
- **Matching.** `number.rs` is 241 lines, `STACK_BYTES_PER_REENTRY` is at `limits.rs:46`, `charge_steps` is at `agent.rs:971`, and `random_seed` is at `agent.rs:172`.
- **The 14 pending requirements.** Each is moved by one story, except HOST-147, which stays pending.
- **Case coverage.** The 11 story lists, with no overlap, equal exactly the 1,348 cases of `scan.tsv` whose later-slice column is `-`. The 3 others (two for slice 9, one for slice 12) are named in the close and are its first `moves.tsv` lines. No case has the key `other`.
- **Outcomes the base already meets.** `Number::toString` for radix 2 to 36, the parsing grammars and the `{:e}` check already exist on the base: `literal.rs`, `to_string.rs`, and `tests/to_string.rs:6`.
- **`exp2` and `powi`.** `exp-and-log` states that `exp2` and `powi` are not language functions, so that narrowing is recorded.

What I read:
- The epic.
- The 12 panel B stories, whole.
- The head of `number-format`.
- The close's dependency list.
- `scan.tsv` and `scan.awk`, all 11 case lists and the guard counts.
- The 14 requirement lines and their texts.
- The `engine.md` passages on `Math.random`, `Math.sumPrecise` and `Date.parse`.
- `AGENTS.md` in the wave tree.
- The round-1 scope result (head only).
- `aep plan artifact graph` and `waves`.

From the epic I extracted 17 promises: 8 outcomes plus 9 acceptance items. All 17 trace to a story or to code already on the base. Those cases came from `comm` and `awk` over the lists and `scan.tsv`.

What I could not establish:
- Whether the `Number.prototype.toLocaleString` text and the other panel A points (open point 5) are right. They are outside my panel.
- Out of my lane (acceptance):
  - `exp-and-log` expects `math-data-check` to print `1 file(s)`, while `trigonometry` expects "one more than before". Both sit in wave 3, so the first expectation depends on merge order.
  - The reference data lines come from an uncommitted `bc` script.
  - The close's "261 passing cases are the guards" does not match the joined guard files, which include cases outside slice 4 (944 under `language/expressions`).
- Out of my lane (parallel safety): `math-random` depends on `math-functions` only to order edits to `math.rs`, which puts it in wave 6.

```findings
- file: .engineering/planning/story/spec-scenarios-for-the-numbers-and-dates-requirements.md
  line: 55
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the story adds ESS scenarios and extends the ESS target, the spec model and the conform floor, which no sentence of the epic asks for and which AGENTS.md forbids (ESS is not extended for swell; each requirement is named by exactly one story), so it must leave slice 4 or shrink to bookkeeping."
- file: .engineering/planning/story/math-functions.md
  line: 7
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the summary, line 42 and line 53 say 21 functions while the variants, the list and the scan have 22, so state 22 everywhere."
- file: .engineering/planning/story/date-text-formats.md
  line: 32
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Context gives Date/prototype as 99 cases where the list has 90, and the stated breakdown sums to 174 instead of 165, so correct the count to 90."
- file: .engineering/planning/story/date-text-formats.md
  line: 52
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the tests assert Date.parse(d.toLocaleString()) equals d while engine.md lines 2349-2350 say that output parses to NaN unless a later decision says otherwise, so the decision must be written in the design before the story starts or the assertion must go."
- file: .engineering/planning/story/exponentiation-operator.md
  line: 45
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Context gives built-ins/Array/prototype as 20 cases where the list has 22 (one reverse case is not named) and the breakdown sums to 74 instead of 76, so correct the count and add the reverse case."
- file: .engineering/planning/story/number-functions.md
  line: 64
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Writes names docs/design/engine.md although the unit does not write it and the parseInt precondition is not yet in the design, so remove the file from Writes and state the sentence as a prerequisite of the design author."
- file: .engineering/planning/epic/slice-04-numbers-and-dates.md
  line: 21
  category: scope
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: "the Outcome names no operator ** and **=, no Math.sumPrecise, no Number predicates and no Date setters although stories claim them, so the coordinator's rewrite of the epic must add outcomes for them."
```

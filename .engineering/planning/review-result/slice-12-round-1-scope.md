---
format: aep.planning-md/3
id: review-result:slice-12-round-1-scope
kind: review-result
status: active
title: 'Slice 12, round 1, the scope critic: needs-revision, 3 findings (1 blocker)'
relations:
- reviews: story:swell-num-bigint-core
- reviews: story:swell-num-bigint-powers-shifts-and-bits
- reviews: story:bigint-values-and-literals
- reviews: story:bigint-comparison-and-loose-equality
- reviews: story:bigint-arithmetic-and-bitwise-operators
- reviews: story:bigint-constructor-and-prototype
- reviews: story:bigint-json-number-and-joined-cases
- reviews: story:slice-12-conformance-close
revision: 1
---
Plan critic, scope, round 1 on the eight draft stories of
`epic:slice-12-bigint` (`plan/slice-12` at `e4a59dde`), 2026-11-28.
Verdict: needs-revision, 3 findings, 1 of them a blocker; revision 1 answers them. The critic's answer, as returned:

needs-revision

1. **Finding 1** (`story:slice-12-conformance-close`, with `story:swell-num-bigint-core`), **blocking**. The deliberate omission for the BigInt limit is promised and no story writes it.
   - Place: `.engineering/planning/story/slice-12-conformance-close.md:61` (Writes line), `.engineering/planning/story/swell-num-bigint-core.md:39`, `.engineering/planning/epic/slice-12-bigint.md:69`.
   - The epic's constraint reads: "A deliberate omission is recorded where the gate reads it".
   - The core story's Context (question 3) says the limit is "listed as an omission of the sections it touches in `conformance/test262/sections.txt` by `story:slice-12-conformance-close`".
   - The close's Writes line says "Not written: `conformance/test262/sections.txt` ... no line is expected to change". No Change bullet or acceptance check of the close writes an `omit` line.
   - The core story also forbids writing `conformance/` ("Not written: ... `conformance/`").
   - The outcome is already known. `docs/design/host-interface.md:215-217` says `grep -i 'implementation limit'` finds nothing and the BigInt `RangeError` is "to be listed as omissions of the sections they touch".
   - Precedent: `conformance/test262/sections.txt:95` has `omit sec-applystringornumericbinaryoperator string-length-limit decision-3`, and line 29 has `omit sec-array-objects ...`. `sec-bigint-objects` has only `section` and `path` lines. No `omit` line mentions BigInt, so nothing in the base covers it.
   - What settles it: the close's Change, Writes and Acceptance carry the `omit` lines. For example, `omit sec-bigint-objects bigint-limit decision-6` and the same for `sec-applystringornumericbinaryoperator`, with a check that they exist. Drop `sections.txt` from the close's "Not written". Reword the core story's sentence to match.

2. **Finding 2** (`story:slice-12-conformance-close`, coordinator list at line 58), **note** (severity warning). This is the same mistake as slice 9, round 1: an epic exclusion contradicts a story, and the coordinator's rewrite list does not name it.
   - Place: `.engineering/planning/epic/slice-12-bigint.md:58` ("`Number` and `Math`: slice 4"), `.engineering/planning/story/bigint-json-number-and-joined-cases.md:17,61`, `.engineering/planning/story/slice-12-conformance-close.md:58`.
   - The joint story writes `crates/swell/src/builtins/number.rs` for step 1.b. The case `built-ins/Number/bigint-conversion.js` is on its list, so the epic's own acceptance 1 requires that change.
   - The close's coordinator list names the epic's counts and "32-bit limbs" only. Nothing rewrites line 58, and the joint story's Out of scope does not mention it.
   - What settles it: add the epic's line 58 to the close's coordinator list, worded for example "`Number` and `Math` except the BigInt arm of `Number()`". Add one sentence to the joint story's Context saying so.

3. **Finding 3** (`story:bigint-values-and-literals` and `story:bigint-json-number-and-joined-cases`), **note**. Out of my lane; I list it here so the coordinator can route it. Epic acceptance 6, claimed by `story:bigint-arithmetic-and-bitwise-operators`, is `2n ** (2n ** 24n)` ending in `Terminated(MemoryBudget)`.
   - Place: `.engineering/planning/story/bigint-arithmetic-and-bitwise-operators.md`, Change, the `bigint_operation` steps (3) and the test `a_result_within_bigint_bits_but_over_the_memory_budget_ends_in_memory_budget`.
   - The body charges the steps from `pow_cost` before the memory. The test names no `steps_per_entry`, and the step cost of that power is of the order of 10^9. A small test budget would end in `StepBudget` first.
   - What settles it: the acceptance critic reads this.

**Read.**
- The epic, then all 8 stories in full.
- `waves-section.md`, the decomposer's report, `scan.awk`, `lists.sh`, `check.sh`.
- `waves.md` "What later slices take over", `sections.txt`, `moves.tsv`.
- The slice 8 and slice 9 scope reviews.
- Counts, all re-run read-only on the base `runs.tsv`:
  - `sh docs/plan/slice-12/lists.sh <base runs>` exits 0 and prints `slice 381 lists 381 incoming 0`, with the lists 46+100+42+43+146+4 = 381 and no case twice.
  - Per-story part runs recounted by awk: values 291+477 = 768, compare 84+2,289 = 2,373, arith 92+2,289 = 2,381, builtin 199+595 = 794, joint 86+595 = 681. These match the stories.
  - Guard cases are all outside slice 12 and pass in every run.
  - Failing runs 213+84+92+199+86+8 = 682, equal to the slice's failing runs in the file.
  - `check.sh` prints the failing-line counts the stories quote.
  - Directory counts re-run on the lists: `built-ins/BigInt` 75+2 = 77, `language/literals/bigint` 59, and the joint story's 33 operator cases = 31 + 2.
  - Case features: no later-slice feature among the 381.
  - The failure messages on each list match the story's text.
- Promises extracted from the epic, `host-interface.md` and `waves.md`: 25. 22 are traced to one story each. 2 are named out by the drafter to the coordinator (the epic's counts and limb size; the `AGENTS.md` and `README.md` numbers). 1 is not claimed (finding 1).

**Found right.**
- Every story carries the time bounds, no `unsafe`, no outside crate, no Python, and one test binary per crate.
- The three open questions have an owner (the core story).
- HOST-23, HOST-24 and HOST-219 each have one owner.
- Epic acceptance 5 and 6 are claimed once, by the arithmetic story.
- The waves.md JSON stack entry belongs to the joint story.
- The refusal text at `lexer.rs:570` names slice 12, which matches the design.
- Nothing is claimed twice: `string_to_bigint` is the core's in `swell-num` and the values story's in `ops`, with no overlap.
- The five guard lists are the part-run inputs, and none is a story list.

**Not established or out of lane.**
- The base `runs.tsv` has 85,556 lines for slices 1 to 19, not the 85,794 the stories quote. 85,794 is the design table's total: slice 14 has 954 runs in the file against 1,192 in the table. The percentages (0.9 %, 2.8 %, 0.8 %) hold either way. The cause is not mine to say.
- `string_to_bigint` in `ops/conversion.rs` has no caller in the values story. With `-D warnings` and `slot task clippy` in its acceptance it may trip `dead_code`. Design and acceptance critics should check this.
- I did not run `aep plan artifact validate`; the decomposer reports `valid`.

**D1 to D5, my view.**
- **D1:** A, compile-time value. It is traceable to "literals" and charged through the compile `Meter`.
- **D2:** A, derive the threshold from the counted limb operations. It fills the epic's open question without a CPU measurement.
- **D3:** A, precharge from the upper-bound `<op>_cost`. It is the epic's "known in advance … before computing".
- **D4:** A. A host reader of BigInt digits and a new HOST requirement are not in the epic, so B would be reach beyond it.
- **D5:** A, literals unlimited. The design table (`host-interface.md:181`) says "computed or converted at run time". The design critic should confirm that "only `0n` results" at 0 is not meant to cover literals.

```findings
- file: .engineering/planning/story/slice-12-conformance-close.md
  line: 61
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the epic's constraint 'a deliberate omission is recorded where the gate reads it' is claimed by no story: the core story hands the BigInt-limit omission lines of conformance/test262/sections.txt to the close, and the close lists sections.txt under 'Not written' and has no Change or acceptance for an omit line, so the close must write and check them (for example omit sec-bigint-objects bigint-limit decision-6 and the same for sec-applystringornumericbinaryoperator, as sections.txt:95 does for string-length-limit)"
- file: .engineering/planning/story/slice-12-conformance-close.md
  line: 58
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic's Out of scope line 'Number and Math: slice 4' (epic line 58) contradicts story:bigint-json-number-and-joined-cases, which edits builtins/number.rs for the case built-ins/Number/bigint-conversion.js, and the close's coordinator list names only the epic's counts and limb size, so add that epic line, reworded to except the BigInt arm of Number(), to the list and say so in the joint story's Context"
```

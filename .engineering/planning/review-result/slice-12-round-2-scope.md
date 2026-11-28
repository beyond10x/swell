---
format: aep.planning-md/3
id: review-result:slice-12-round-2-scope
kind: review-result
status: active
title: 'Slice 12, round 2, the scope critic: approve, 3 closing points'
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
Plan critic, scope, round 2 on the eight draft stories of
`epic:slice-12-bigint` (`plan/slice-12` at `82dde80a`), 2026-11-28.
Verdict: approve, 3 closing points; revision 2 answers them. The critic's answer, as returned:

approve

**Round-1 findings of the scope review**

1. `settled`. The omission lines are now written and checked by the close.
   - Writes: `.engineering/planning/story/slice-12-conformance-close.md` has `conformance/test262/sections.txt` in scope (line 27-28), in Change (line 59) and in Writes (line 64).
   - Acceptance: line 68 greps both lines (`omit sec-bigint-objects bigint-bits-limit decision-3` and `omit sec-applystringornumericbinaryoperator bigint-bits-limit decision-3`). Each greps `1`, and `0` on the base.
   - Core story: `.engineering/planning/story/swell-num-bigint-core.md:41` names the same two lines, and its Out of scope (line 83) hands them to the close.
   - Base file: on `f962c324`, `sections.txt:14` is the `path sec-bigint-objects` line and `sections.txt:95` is the `string-length-limit` line.
   - Reader: `crates/test262/src/sections.rs:147-170` accepts `omit <anchor> <reason> <decision>` with four words, so both lines parse.
   - Count: the `test262-section` counts of the two sections (`expected.txt:252` and `:285`) go from 0 to 1 and from 1 to 2, as the close says.
   - Condition: the lines depend on core's finding about an allowance. The close says the coordinator removes them if an allowance is found. That is a named condition, not a gap.
2. `settled`. The epic's line "`Number` and `Math`: slice 4" is handled in two places.
   - Close: the coordinator list at `slice-12-conformance-close.md:61` rewords it to "`Number` and `Math`: slice 4, except the BigInt arm of `Number()`".
   - Joint story: `bigint-json-number-and-joined-cases.md:30` carries the sentence in its Context.
3. `settled`. Out of my lane, but I checked it in the file.
   - Order: `bigint-arithmetic-and-bitwise-operators.md:47` quotes `Agent::concatenate` as limit, then `charge_steps`, then `reserve_vec`. In `crates/swell/src/ops/operators.rs` the order is `string_length` at line 133, `charge_steps` at 139 and `reserve_vec` at 145.
   - Test: line 49 names `steps_per_entry` 2^40 and shows the numbers. The result needs at least 262,145 limbs, which is 2,097,160 bytes, against 1,048,576 left. The step bound of about 1.83e11 is under 2^38, so `Terminated(MemoryBudget)` follows.

**New findings**

None that would change what was drafted. Three small points sit in the closing line below.

**Checked and found right**
- **Promises:** I read the epic whole, with `host-interface.md` and `waves.md`, and extracted 25 promises. 23 trace to one story each. 2 are named out to the coordinator: the epic's counts and limb size, and the `AGENTS.md` and `README.md` numbers.
- **Lists and counts, re-run:**
  - `lists.sh` on the base `runs.tsv` prints `slice 381 lists 381 incoming 0`, exit 0. The lists are 46, 100, 42, 43 and 146, plus 4 for the close.
  - `built-ins/BigInt` is 77 cases and `language/literals/bigint` is 59.
  - There are 760 runs of slice 12, of which 682 fail.
  - The base run file has 85,556 lines. The quoted shares 0.90, 2.77, 2.78, 0.93 and 0.80 % hold against it.
  - 338 failing cases on the five lists plus 4 on the close list is 342.
  - The 64 moved-in cases (22, 20, 16, 3, 2 and 1) sum to 64.
- **Source ownership:** All 16 `git grep 'before slice 12' -- crates` lines on the base have an owner among the stories. The stories write nothing outside the epic's reach; the edits to `interrupt.rs` and `dynamic_function.rs` follow from removing the refusal.
- **Store:** `aep plan artifact validate` ends `valid`. The `waves` command puts the three stories of order 3 in one wave, and none of the printed collisions is inside order 3.

**Closing line.** Three points are not findings:
- `bigint-comparison-and-loose-equality.md:39` says the close's omission lines record the `RangeError` of StringToBigInt in `<` and `==`. `sections.txt` has no section path for `less-than` or `equals`, so only the sections of the two lines can carry it. This is a loose sentence, not a gap.
- The omission lines cite `decision-3`, copied from the `string-length-limit` lines. The epic ties the BigInt limit to decisions 6 and 9. The reader does not check the token.
- `crates/num/src/literal.rs:66` still says "BigInt is slice 12's", and no story owns it. The close's check reads only `before slice 12` and `refused("BigInt literal"`.

```findings
[]
```

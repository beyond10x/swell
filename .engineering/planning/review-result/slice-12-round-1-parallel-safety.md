---
format: aep.planning-md/3
id: review-result:slice-12-round-1-parallel-safety
kind: review-result
status: active
title: 'Slice 12, round 1, the parallel-safety critic: needs-revision, 6 findings (2 blockers)'
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
Plan critic, parallel-safety, round 1 on the eight draft stories of
`epic:slice-12-bigint` (`plan/slice-12` at `e4a59dde`), 2026-11-28.
Verdict: needs-revision, 6 findings, 2 of them blockers; revision 1 answers them. The critic's answer, as returned:

needs-revision

**Findings**

1. `story:bigint-constructor-and-prototype` — blocking — Its test `as_int_n_and_as_uint_n` runs `asIntN(64, 2n ** 63n)` and `asUintN(64, -1n)` against `2n ** 64n - 1n`. That is the binary `**` and `-` that `story:bigint-arithmetic-and-bitwise-operators` writes. The two are order-3 siblings with no edge, and the body says "no edge between them". The implementor cannot pass this test until the arith story has landed (reader beside producer; cited).
   - Place: `.engineering/planning/story/bigint-constructor-and-prototype.md:57` (the test) and `:66` (Writes, "no edge between them"). Producer: `bigint-arithmetic-and-bitwise-operators.md:47`.
   - Evidence: `grep -o "2n \*\* 6[34]n[^,;)]*"` on the constructor story prints `2n ** 63n` and `2n ** 64n - 1n`. `bigint-comparison-and-loose-equality.md:37` shows the precedent: "literals and Numbers only, no arithmetic".
   - Settles it: add `depends_on: story:bigint-arithmetic-and-bitwise-operators` to the constructor story, or rewrite those expectations with literals only (`-(2n ** 63n)` as `-9223372036854775808n`).

2. `story:bigint-arithmetic-and-bitwise-operators` — note — Inferred, not cited. The test `add_sub_mul_agree_with_numbers_for_small_values` runs "a loop of 1,000 pairs in the script" and names no way to make a BigInt from the loop counter.
   - Place: `bigint-arithmetic-and-bitwise-operators.md:49`.
   - Evidence: `grep -c 'BigInt(' ` on the story finds none. The only source of a BigInt from a Number is `BigInt(i)`, which is `story:bigint-constructor-and-prototype`'s, and no edge runs that way.
   - Settles it: one sentence saying the pairs come from generated source text (literals); do not use `BigInt(i)`. This also keeps finding 1's edge from becoming a cycle.

3. `set` (compare, arith, values, constructor) — blocking — Four acceptances require "unchanged against `$BASE`" on files that a sibling of the same order writes. The same bodies require the unit to merge the wave branch into its head before reporting. After that merge the check reads non-empty and fails. The bodies do not say whether the diff is taken before or after the merge.
   - Places:
     - compare, `bigint-comparison-and-loose-equality.md:50`: `git diff --quiet "$BASE" -- conformance crates/num crates/swell/src/ops/conversion.rs`. Constructor writes `conversion.rs` (`bigint-constructor-and-prototype.md:66`).
     - arith, `bigint-arithmetic-and-bitwise-operators.md:62`: `... crates/num crates/swell/src/ops/testing.rs`. Compare writes `testing.rs`.
     - constructor, `bigint-constructor-and-prototype.md:70`: `... crates/swell/src/ops/testing.rs crates/swell/src/ops/operators.rs`. Compare and arith write them.
     - values, `bigint-values-and-literals.md:107`: `git diff --quiet "$BASE" -- ... crates/num`. Powers writes `crates/num` in the same order, and `:96` also merges the wave branch.
   - Settles it: define the check on the unit's own commits, for example `git diff --quiet "$(git merge-base HEAD "$BASE")" HEAD -- <paths>`, or take the diff before the merge. Say which in each body.

4. `set` (compare, arith, constructor) — note — The three order-3 stories say they are "built at the same time", and each lists `crates/swell/tests/main.rs` in scope. `aep plan artifact waves` therefore serialises them. It puts compare in wave 11, arith in 19 and constructor in 20, so the report's "7 waves for 8 stories" is the tool's reading, not the bodies'. The decomposer disclosed this, so a person decides.
   - There is also a real textual conflict. After `bigint_values` lands, `bigint_arithmetic` and `bigint_comparison` both insert immediately before the `bigint_values` line of the generated file. They share one anchor, which slice 9's critic did not find for its stories (round 1: "each inserts at a different point"). The coordinator regenerates the file on the join, but each unit's own merge still conflicts.
   - Evidence: `aep plan artifact waves`, collision lines for `crates/swell/tests/main.rs`; `crates/swell/tests/main.rs` (modules in byte order).
   - Places: `bigint-comparison-and-loose-equality.md:46`, `bigint-arithmetic-and-bitwise-operators.md:58`, `bigint-constructor-and-prototype.md:66`.
   - Settles it, either way: drop `crates/swell/tests/main.rs` from the three scopes and say a unit that conflicts there takes either side and runs `sh taskfiles/test-main.sh write`, or keep the scope and delete "built at the same time".

5. `story:slice-12-conformance-close` — note — Cross-set. Its Writes line says the coordinator adds the edge to the slice-11 close "when that story exists". `story:slice-11-conformance-close` now exists on `plan/slice-11` (head `c3021d72`). Its scope shares `conformance/test262/{expected.txt,expected.digest,later-parts.tsv,moves.tsv}` and `docs/design/engine.md` with this story.
   - Place: `slice-12-conformance-close.md:61`.
   - Evidence: the intersection of the scope paths of both worktrees' stories.
   - Settles it: record `depends_on: story:slice-11-conformance-close` when the two branches merge; the validator refuses it before then.

6. `story:bigint-values-and-literals` — note — Cross-set. Its scope and the slice-11 stories (ten of the thirteen) share these files: `lexer.rs`, `ast.rs`, `parser/mod.rs`, `bytecode.rs`, `disasm.rs`, `expression.rs`, `resolve.rs`, `dispatch.rs`, `heap/collect.rs`, `heap/mod.rs`, `value.rs`, `requirements.txt` and `status.json`. The arith story shares `dispatch.rs`, `requirements.txt` and `status.json` with them. The epic orders 12 after 11, but neither body names a story-level edge or the slice-11 hunks. Every anchor in the body is a line number on `f962c324`, for example `lexer.rs:570`, `Number(u32)` at `bytecode.rs:72`, and `Item::NoArena` at `collect.rs:66`. They will have moved once slice 11 is in the base.
   - Place: `bigint-values-and-literals.md:103`.
   - Settles it: either an edge to `story:slice-11-conformance-close` for the first story that writes these files, or a sentence saying anchors are found by name and the line numbers belong to `f962c324`.

**What I checked and found right**
- Placement: 8 of 8 stories have their surface cited in the body (scope entries marked inferred are only new files), none unplaced.
- Acceptance reads traced to a producer in the set: 21, all recorded by `depends_on`, 4 of them through a path (core via values, powers and compare). Two more reads have no edge (findings 1 and 2), and one is outside the set (finding 5).
- Every `docs/plan/slice-12/` count reproduces on `runs.tsv`:
  - `lists.sh` prints `slice 381 lists 381 incoming 0`, exit 0.
  - Part sizes: 392/768, 1275/2373, 1279/2381, 398/794 and 341/681 cases and runs.
  - The values `check.sh` figures are `passed 285 bound 285 F 107 R 107`, and 213 failing runs.
  - The 16 `before slice 12` lines are each owned by one story.
  - `docs/design/engine.md` is written only by core (decision 6) and the close (section 12).
  - `Cargo.toml` already has `swell-num` in `swell` and `compiler`.
- Every collision with a store story outside the set is with an `implemented` or `archived` story (117 stories checked).
- All 8 bodies carry the 60 s, 120 s and 10/30 minute bounds, "no Python" and the scripts-off-slot rule.

Out of my lane, for the other critics: the part runs (`cargo run --release ... count`) are called one pool job, but none was timed (acceptance). The "85,794 runs" figure is the design total; the base run file has 85,556 (acceptance).

**Decomposer's open questions**
- D1: A. Compile-time conversion changes the same compiler files under either option, so it does not change collisions.
- D2: A. A derived count is deterministic. A CPU-measured threshold would put a timing into a test.
- D3: A. One `Cost` form fixed by core is the single interface the four parallel stories share.
- D4: A. It keeps `conformance/host/surface.txt` out of every story.
- D5: A. No view from my lane.

```findings
- file: .engineering/planning/story/bigint-constructor-and-prototype.md
  line: 57
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the test as_int_n_and_as_uint_n runs 2n ** 63n and 2n ** 64n - 1n, operators that story:bigint-arithmetic-and-bitwise-operators writes (bigint-arithmetic-and-bitwise-operators.md:47), and no depends_on edge orders the two; add the edge or rewrite the expectations with literals only (cited)"
- file: .engineering/planning/story/bigint-arithmetic-and-bitwise-operators.md
  line: 49
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "inferred surface: the test add_sub_mul_agree_with_numbers_for_small_values loops over 1,000 pairs and names no way to make a BigInt from the counter, and the only one is BigInt(i) of story:bigint-constructor-and-prototype, with no edge; say the pairs are generated literals"
- file: .engineering/planning/story/bigint-comparison-and-loose-equality.md
  line: 50
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance requires git diff --quiet against BASE on ops/conversion.rs while the unit merges the wave branch whose sibling story:bigint-constructor-and-prototype writes it, so the check fails after the merge; the same holds for arith (:62, testing.rs), constructor (:70, testing.rs and operators.rs) and values (:107, crates/num written by powers); define the check on the unit's own commits"
- file: .engineering/planning/story/bigint-comparison-and-loose-equality.md
  line: 46
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the three order-3 stories claim to be built at the same time but list crates/swell/tests/main.rs in scope, so aep plan artifact waves puts them in waves 11, 19 and 20, and arith and compare insert at one anchor (before bigint_values) so their merge conflicts; drop the file from the scopes with a take-either-side-and-regenerate rule, or drop the claim"
- file: .engineering/planning/story/slice-12-conformance-close.md
  line: 61
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:slice-11-conformance-close now exists on plan/slice-11 (c3021d72) and writes the same five files; record depends_on to it when the branches merge (cross-set)"
- file: .engineering/planning/story/bigint-values-and-literals.md
  line: 103
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "cross-set: ten slice-11 stories share lexer.rs, ast.rs, parser/mod.rs, bytecode.rs, expression.rs, resolve.rs, dispatch.rs, heap/collect.rs, value.rs, requirements.txt and status.json with this story, with no story-level edge, and every anchor is a line number of f962c324; add the edge or say anchors are found by name"
```

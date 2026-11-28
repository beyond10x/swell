---
format: aep.planning-md/3
id: review-result:slice-12-round-2-parallel-safety
kind: review-result
status: active
title: 'Slice 12, round 2, the parallel-safety critic: needs-revision, 1 finding (1 blocker)'
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
Plan critic, parallel-safety, round 2 on the eight draft stories of
`epic:slice-12-bigint` (`plan/slice-12` at `82dde80a`), 2026-11-28.
Verdict: needs-revision, 1 blocker (`crates/compiler/src/lib.rs` missing from the scope of the values story) and three points outside its role; revision 2 answers them. The critic's answer, as returned:

needs-revision

Round-1 findings of this role:

1. `settled`. `bigint-constructor-and-prototype.md:57` writes the `as_int_n_and_as_uint_n` expectations as literals and says "the binary operators are `story:bigint-arithmetic-and-bitwise-operators`'s". I found no operator, `BigInt(` or builtin of another story in the tests of core, powers, values, compare or arith. The coordinator's ruling is followed.
2. `settled`. `bigint-arithmetic-and-bitwise-operators.md:49` generates the 1,000 pairs as literals in source text and says "no `BigInt(i)`".
3. `settled`. All seven bodies use one command form: `git log --no-merges --first-parent --format= --name-only "$BASE"..HEAD -- <paths>` prints no names. The places are core `:79`, powers `:61`, values `:108`, compare `:51`, arith `:63`, constructor `:71` and json `:60`.
   - I ran the form on `53f6a3f8..362b69b1`. A path brought only by merges (`AGENTS.md`) prints 0 lines, where `git diff --quiet` exits 1 on it. The unit's own paths print.
4. `settled`. `crates/swell/tests/main.rs` and `crates/num/tests/main.rs` are out of every scope, and each body has the take-either-side-and-regenerate sentence. `aep plan artifact waves --status draft` gives wave 3 = compare, arith and constructor. Its 9 collisions are all between stories joined by a `depends_on` edge, none inside order 3.
5. `settled as ruled`. `slice-12-conformance-close.md:64` keeps the sentence that the coordinator adds the edge to the slice-11 close.
6. `settled as ruled`. Every body says its line numbers are those of `f962c324` and that anchors are found by name, with no edge to slice 11. The report's list of files shared with slice 11 is wrong for values (new finding 1).

New findings:

1. `story:bigint-values-and-literals` — `crates/compiler/src/lib.rs` is written but not in scope or Writes (cited in the code, not in the body) — blocking.
   - Place: `bigint-values-and-literals.md:89` (`Code::bigints`, "each value once") and `:104` (Writes), plus the scope list.
   - What is wrong: `Code::bigints` cannot be added without editing `crates/compiler/src/lib.rs`.
     - `Constants` (`lib.rs:326-332`) holds `numbers` and `number_index`, which is how each value is made once.
     - `Constants::number` (`:361`) and the emitter's `number` (`:753`) are the model for a `bigint` method, which the `Expr::Number` arm in `expression.rs` would call.
     - `Code { … numbers: constants.numbers … }` (`:313`) builds the record.
     - The test helper `code_walk` (`:1457`, inside `mod tests`, used at `:1515`) sums every vector of a `Code`, so a charged `bigints` field with no term there turns that test red.
   - Collision: three slice-11 stories list `crates/compiler/src/lib.rs` in scope (`module-code-compiles`, `top-level-await`, `import-call-and-import-meta-syntax`, draft, head `c3021d72`). The report's shared-file list for values (14 files) omits it; I recomputed the same 14 and the scope has no 15th.
   - Evidence: `git grep -n 'compiler/src/lib.rs' .engineering/planning/story/bigint-values-and-literals.md` prints 0 lines; the slice-11 scopes were compared with `awk` over their `scope:` lists.
   - What would settle it: add `crates/compiler/src/lib.rs` to the scope and to Writes, naming `Constants` (pool and index), the `Code` literal and `code_walk`. Say in the report that values shares 15 files with slice 11, so the coordinator's join edges cover it.

Checked and right:
- **Placement:** 8 of 8 surfaces cited, 0 inferred, 0 unplaced (inferred entries are only new files); every name in the new stories' scopes is free on the base.
- **Acceptance reads:** 20 traced to a producer in the set, all recorded by `depends_on` (17 direct, 3 through a path: arith, compare and constructor to core). No reader lacks an edge.
- **Collisions outside the set:** all 108 counterparts of the tool's collisions are `implemented`.
- **Other checks:**
  - `sections.txt` is written only by the close in slice 12, and not by slice 11's close.
  - `expected.digest` is a digest of `crates/*/src` and others, so only the close writes it.
  - Test counts reproduce: 16, 11, 10 + 1, 6, 13 + 2, 11 and 6.
  - All 16 `before slice 12` lines in `crates` are owned by a story.
  - The three order-3 stories share no file.

Out of your lane (for the other critics, not weighed in the verdict):
- **Empty `$BASE` (acceptance):** an empty `$BASE` makes `"$BASE"..HEAD` read as `..HEAD`, an empty range, so the "names are none" check passes with nothing checked. Each body should require `BASE` to be set.
- **Core's grep acceptance (acceptance and design):** core forbids every `vec!` and `.push(` in `bigint.rs` and `bigint/` and allows one `try_reserve_exact`, which is `limbs_with_capacity` returning `Vec<u64>`. `to_radix` returns `Vec<u8>`, and no body says how that buffer is made.
- **json step 2 (acceptance):** `bigint-json-number-and-joined-cases.md:44` says `get_v` makes no wrapper for a BigInt, but `get_v` (`ops/objects.rs:70`) calls `to_object`, which makes one.

```findings
- file: .engineering/planning/story/bigint-values-and-literals.md
  line: 89
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "cited: Code::bigints needs crates/compiler/src/lib.rs (Constants pool and index at :326-332, the Code literal at :313, Constants::number :361 and the emitter's number :753, and code_walk in mod tests :1457), which is in neither scope nor Writes; module-code-compiles, top-level-await and import-call-and-import-meta-syntax of slice 11 write the same file, and the report's list of 14 shared files omits it; add the file to scope and Writes naming those places, and count 15 shared files"
```

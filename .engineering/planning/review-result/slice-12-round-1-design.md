---
format: aep.planning-md/3
id: review-result:slice-12-round-1-design
kind: review-result
status: active
title: 'Slice 12, round 1, the design critic: needs-revision, 8 findings (5 blockers)'
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
Plan critic, design, round 1 on the eight draft stories of
`epic:slice-12-bigint` (`plan/slice-12` at `e4a59dde`), 2026-11-28.
Verdict: needs-revision, 8 findings, 5 of them blockers; revision 1 answers them. The critic's answer, as returned:

needs-revision

Findings

1. `story:bigint-constructor-and-prototype` — blocking — Hidden dependency. The test `as_int_n_and_as_uint_n` runs `asIntN(64, 2n ** 63n)`, `-(2n ** 63n)` and `2n ** 64n - 1n`. The binary operators on BigInts are made by `story:bigint-arithmetic-and-bitwise-operators`, and no `depends_on` edge runs from the constructor to it; the two are built at the same time in order 3. On the constructor's branch those expressions end in `Terminated(EngineFault)`, so its acceptance cannot be met there.
   - Evidence: `.engineering/planning/story/bigint-constructor-and-prototype.md:57`, the test list ("`asIntN(64, 2n ** 63n)` is `-(2n ** 63n)`, `asUintN(64, -1n)` is `2n ** 64n - 1n`"). `aep plan artifact graph` shows the constructor depends only on `bigint-values-and-literals` and `swell-num-bigint-powers-shifts-and-bits`.
   - Settle by writing the expectations as literals (`9223372036854775808n`, `-9223372036854775808n`, `18446744073709551615n`), which keeps order 3 parallel. The alternative is a `depends_on` edge from the constructor to arith.

2. `story:swell-num-bigint-powers-shifts-and-bits` — blocking — Two halves of one cost rule contradict each other. Powers says "an exponent longer than 64 bits has a cost of `u64::MAX`, so the engine refuses it by its size". The same story answers a base of 0, 1 or −1 without a loop whatever the exponent, and gives `pow_bits` only for other bases.
   - Arith's `bigint_operation` charges `pow_cost` before the work. Arith's test `exponentiation_rules` requires `(-1n) ** (2n ** 64n)` to be `1n` with few steps. The exponent `2n ** 64n` has 65 bits, so `pow_cost` is `u64::MAX` and the entry ends in `Terminated(StepBudget)`. `pow_bits` for a base of ±1 is also unstated.
   - Evidence: `story/swell-num-bigint-powers-shifts-and-bits.md:41` and `story/bigint-arithmetic-and-bitwise-operators.md:49`.
   - Settle by having powers state `pow_bits` and `pow_cost` for bases 0, 1 and −1 as constant (the exponent's length does not enter), and by adding one powers test that calls them with a 65-bit exponent.

3. `story:bigint-arithmetic-and-bitwise-operators` — blocking — The charge order in `bigint_operation` cannot reach `Terminated(MemoryBudget)` in the epic's acceptance 6 expression. Step (3) charges the steps from `pow_cost` before the memory temporary. The test `a_result_within_bigint_bits_but_over_the_memory_budget…` runs `2n ** (2n ** 24n)`, a 262,144-limb result; its Karatsuba count is far above any default step budget, and it names no `steps_per_entry`. The host default I read is 1,000,000 (`crates/swell/src/host/hooks.rs:412`).
   - So the entry ends in `Terminated(StepBudget)` before the memory charge is reached.
   - Evidence: `story/bigint-arithmetic-and-bitwise-operators.md:47`, step (3) ("the steps before the work … the result's limbs and the scratch as one temporary before the work"), and `:49`, the test.
   - Settle either by charging memory before steps in `bigint_operation`, or by having the test name a `steps_per_entry` above the `pow_cost` it reads. Say which in the body, since the order of the two charges is a design choice.

4. `story:bigint-values-and-literals` — blocking — A split puts a function in one story and its only callers in later ones, and `slot task clippy` (named in this story's acceptance) runs `cargo clippy --workspace --all-targets -- -D warnings`. The story adds `string_to_bigint`, "for later stories", and `bigint_check_bits`, whose callers named are in arith, the constructor and compare. A `pub(crate)` function with no caller in a non-test build fails `dead_code`. The repository's precedent is `#[expect(dead_code, reason = "...")]` (`crates/swell/src/ops/conversion.rs:434`).
   - Evidence: `Taskfile.yml:1229`; `story/bigint-values-and-literals.md:91` (`string_to_bigint` for later stories); `story/bigint-comparison-and-loose-equality.md:46` (compare does not write `ops/conversion.rs`).
   - An `expect` goes red when the function gains a caller, and compare, which calls it first, is told not to touch `ops/conversion.rs`.
   - Settle by moving the first caller into values. The alternative is to name the `expect(dead_code)` lines, which story removes each, and allow that story the one-line change in a file it does not otherwise write, naming the files in both bodies.

5. `story:slice-12-conformance-close` — blocking — A duty that no story holds. `story:swell-num-bigint-core` says that if ECMA-262 words no allowance, the `RangeError` limit is a deviation "listed as an omission of the sections it touches in `conformance/test262/sections.txt` by `story:slice-12-conformance-close`". The close says it does not write `sections.txt`: "no line is expected to change". `docs/design/host-interface.md:215` says the draft words no allowance, so core's condition holds.
   - `sections.txt` already carries one omission per limit (`omit sec-applystringornumericbinaryoperator string-length-limit decision-3`, line 95).
   - Evidence: `story/swell-num-bigint-core.md:39` and `story/slice-12-conformance-close.md:61`.
   - Settle by adding `conformance/test262/sections.txt` to the close's scope and Writes, with a check on the new `omit … bigint-limit decision-6` lines. Alternatively core writes the lines itself.

6. `set` — note — Trade-off, not a request to remove the edges. The generated `crates/swell/tests/main.rs` is declared by every story that adds a test module. `aep plan artifact waves` therefore separates the three order-3 stories into three waves, giving 7 waves for 8 stories against 5 orders in the bodies. The decomposer reports this.
   - Option A, as drafted: accept the 7 waves.
   - Option B: drop `crates/swell/tests/main.rs` from each `scope` and keep the sentence "the coordinator generates it on the join", which puts compare, arith and the constructor in one wave.
   - Evidence: `aep plan artifact waves | grep ^collision` pairs the three on `crates/swell/tests/main.rs`.

7. `story:swell-num-bigint-core` — note — The body cites "the deterministic generator of `crates/num/tests/support/`" for its 2,000 pairs. `crates/num/tests/support/` holds only `exact.rs` (a `Big`/`Rat` reference type) and no generator. Other test files inline their own LCG (for example `parse.rs:74`).
   - `crates/num/tests/support/` is also not in core's scope or Writes.
   - Settle by saying core writes the generator and listing its file in scope, or by saying each test inlines one. Powers' 2,000-pair test depends on the same helper.

8. `story:slice-12-conformance-close` — note — The close has no edge to `story:slice-11-conformance-close`. That story exists on `plan/slice-11` (file `swell-plan-slice-11/.engineering/planning/story/slice-11-conformance-close.md`) but not on this branch, and the close reads runs "after the close of slice 11". The decomposer says the coordinator adds it. It has to be added when the two branches meet, before the close is ready.

What I checked and found right:
- **Counts and equations.** `lists.sh` over the base run prints `slice 381 lists 381 incoming 0`. I recomputed the part sizes myself: values 392 cases / 768 runs, compare 1,275 / 2,373, arith 1,279 / 2,381, builtin 398 / 794, joint 341 / 681. The slice has 760 runs and 682 failing, and the slice-11 list has 8 failing runs. The 16 lines of `before slice 12` split 9 / 3 / 1 / 3 across values, arith, constructor and joint, and the pending-HOST lines are 3 on the base. `aep plan artifact validate` ends `valid`.
- **Graph shape.** I walked 31 declared edges, including those to `slice-11`, `slice-13` and `slice-08` outside the set. There is no cycle, and the graph is not a chain (core, then values and powers, then three parallel stories, then joint, then close). Every other produced-state read I traced to a producer is recorded by an edge, by the transitive edge, or by the same-story edge. The one exception is finding 1; the other cross-story tests (compare, arith, joint) read only what an earlier edge produces.
- **Time bounds and rules.** Every story carries the 60 s, 120 s and 10-to-30-minute bounds, no `unsafe`, no outside crate, no Python, and one test binary per crate.

What I could not establish:
- Whether a case in a values, compare or arith list reads a BigInt held in a variable, for example through a property read that needs ToObject. The scan works on tokens, as the decomposer says, and I sampled only the builtin list. That belongs to `plan-critic-acceptance` and `plan-critic-scope`.
- The scan-derived counts that sum with the "85,794 runs" denominators match the design table, but the base run file has 85,556 lines. Slice 14 has 954 runs against 1,192 in the table. It does not touch slice 12, but whoever runs the full count should know.

Decomposer questions

- **D1, literal value:** A.
- **D2, Karatsuba threshold:** A, provided the counted limb operations include Karatsuba's splits and additions, since that count is also what is charged.
- **D3, charging:** A, with the memory and step order of finding 3 fixed.
- **D4, host surface:** A.
- **D5, limit check:** A. It agrees with the host-interface table, "computed or converted at run time", and with HOST-24's `0n` literal.

```findings
- file: .engineering/planning/story/bigint-constructor-and-prototype.md
  line: 57
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the test as_int_n_and_as_uint_n runs 2n ** 63n and 2n ** 64n - 1n, which story:bigint-arithmetic-and-bitwise-operators produces, and no depends_on edge runs from this story to it; write the expectations as literals or add the edge"
- file: .engineering/planning/story/swell-num-bigint-powers-shifts-and-bits.md
  line: 41
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "pow_cost is u64::MAX for any exponent over 64 bits while pow answers a base of 0, 1 or -1 without a loop, so arith's (-1n) ** (2n ** 64n) with few steps cannot pass; state pow_bits and pow_cost for those bases as independent of the exponent length"
- file: .engineering/planning/story/bigint-arithmetic-and-bitwise-operators.md
  line: 47
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "bigint_operation charges steps before memory, so 2n ** (2n ** 24n) of the epic's acceptance 6 ends in Terminated(StepBudget) under any default budget; charge memory first or name a steps_per_entry above pow_cost in the test"
- file: .engineering/planning/story/bigint-values-and-literals.md
  line: 91
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "string_to_bigint and bigint_check_bits have no caller in this story, so slot task clippy (-D warnings, Taskfile.yml:1229) fails on dead_code; give them a caller here or name the expect(dead_code) lines and the story that removes each, including the file that story may touch"
- file: .engineering/planning/story/slice-12-conformance-close.md
  line: 61
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "swell-num-bigint-core assigns the omission lines for the BigInt RangeError limit in sections.txt to this story, which lists sections.txt as not written; add the file and a check on the omit lines to this story"
- file: .engineering/planning/story/bigint-arithmetic-and-bitwise-operators.md
  line: 46
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "all swell stories declare the generated crates/swell/tests/main.rs, so waves derives 7 waves for 8 stories; either accept that or drop the generated file from each scope and keep the coordinator regeneration on the join"
- file: .engineering/planning/story/swell-num-bigint-core.md
  line: 62
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "cites a deterministic generator in crates/num/tests/support/ that does not exist (only exact.rs is there) and does not list that directory in scope; say core writes it or that each test inlines one"
- file: .engineering/planning/story/slice-12-conformance-close.md
  line: 61
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no depends_on edge to story:slice-11-conformance-close, which exists on plan/slice-11 and not here; add it when the branches meet"
```

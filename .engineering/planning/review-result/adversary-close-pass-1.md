---
format: aep.planning-md/3
id: review-result:adversary-close-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit close: red, 4 findings, 2 red cases'
relations:
- reviews: story:slice-04-conformance-close
revision: 1
---
Adversary, pass 1 on the unit close of wave big-1 (impl/slice-04-close at e2ae99d5), 2026-11-26.
Verdict: red. 4 findings (1 blocker, 1 warning, 2 notes), all introduced; 3 cases added, 2 red.

unit: close (story:slice-04-conformance-close), worktree swell-impl-slice-04-close at head e2ae99d5 with my uncommitted test additions
verdict: red
cases: executed 1→4, red 2
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: 3 paths (listed in part 6)
needs-coordinator: yes. The start run S at a46fe885 is needed to settle "failed at the start" case by case (finding 2).

**1. Diff stat**

```
 crates/swell/tests/consumer_slice_04.rs | 83 +++++++++++++++++++++++++++++++++
 1 file changed, 83 insertions(+)
```

Only a test file changed. Nothing is committed.

**2. Cases added** (appended to `crates/swell/tests/consumer_slice_04.rs`; the unit's test is unchanged)

- `adversary_probe_a_case_that_needs_math_pow_throws_without_it`: a control. It deletes `Math.pow` and runs line 13 of `staging/sm/Number/20.1.2.6-MAX_SAFE_INTEGER.js`. It expects a throw in both modes. **Green**, so the probe can tell a case that needs slice 4 from one that does not.
- `adversary_probe_regress_596805_2_fails_without_the_parts_of_slice_4_it_uses`: **red**. It deletes `Math.pow` and `Math.tan` and runs the case text. On a46fe885 `Math` had only `max` (`git show a46fe885:crates/swell/src/builtins/math.rs`, line 26). Output of the run of the probes alone:
  `the case completes without Math.pow and Math.tan (sloppy Normal("undefined"), strict Normal("undefined")): it passed at the start of the slice and needs nothing of slice 4` (consumer_slice_04.rs:124, nextest exit 100)
- `adversary_probe_the_nine_references_tell_a_ceil_that_does_not_round`: **red**. It replaces `Math.ceil` with the identity and runs the consumer script. The output is unchanged:
  `left: "\"1700000000000,4,3,2,0,3.5,0.5,3,4\"" right: "\"1700000000000,4,3,2,0,3.5,0.5,3,4\""` (consumer_slice_04.rs:147)

Log: `adversary-1/probes-alone.log`.

**3. Suite run, after the cases existed**

`SLOT_TIMEOUT=30m cargo nextest run --locked -p swell --test main -E 'test(/^consumer_slice_04::/)'` ended with `4 tests run: 2 passed, 2 failed`, exit 100. The unit's own test passes. Log: `adversary-1/suite.log`.

**4. Findings** (all on e2ae99d5)

- **F1: one of the 72 incoming cases does not belong in slice 4.**
  - Measured: `docs/plan/slice-04/close-into-4.tsv:72` and `docs/design/engine.md:5708` take `staging/sm/regress/regress-596805-2.js` into slice 4. Its only slice-4 parts (`Math.pow`, `Math.tan`) sit inside `try { … } catch(e) {}` and it asserts nothing. The probe completes normally in both modes without them, so it passed at a46fe885.
  - Rule broken: slices 7 and 10 took a case in only if it failed at the start on a part of that slice (waves.md:334-337, engine.md:3686-3691).
  - Fix: drop the line. Slice 4 becomes 1,680 cases / 3,318 runs, slice 19 becomes 1,364 / 2,552; the table, the output block, expected.txt and for-coordinator.md change with it.
  - Reaches it: the block line, which the runner and slices.awk read. Origin: introduced.
- **F2: "failed at the start" was inferred by count, not checked per case, and F1 shows the count is not exact.**
  - Measured: engine.md:3745-3751 says the 66 are "these 66" because slice 19 went from 604 passed (a46fe885 expected.txt) to 670. If regress-596805-2 passed at the start, the remaining 1,363 cases went from at most 603 to 604 passed. So at least one other slice-19 case without a slice-4 token started passing since a46fe885, or a regression hides one.
  - What committed files can show: a46fe885's expected.txt gives totals only (`staging/sm` 632 passed, slice 19 604), never one case. Slice 10's close used the start run's message for each case (engine.md:3686-3691).
  - Needs the run S at a46fe885 (coordinator). Origin: introduced.
- **F3: three of the consumer's nine references cannot fail if slice 4 is taken out.**
  - Measured: `Math.ceil(300 / 100)` (consumer_slice_04.rs:30) passes with an identity `ceil`; probe red at :147.
  - `Math.max(2 - 5, 0)` and `Math.max(3, 4)` (:27, :31) use `max`, which existed at a46fe885 (math.rs:26).
  - The story prescribes these argument values, so this is a note.
  - Reaches it: the story's acceptance filter `test(/^consumer_slice_04::/)`. Origin: introduced.
- **F4: one of the unit's three claims about the acceptance commands (for-coordinator.md:140-143) is stated wrongly.**
  - The unit says the joint check "over the base run as second and third argument" exits 0. Measured: `sh docs/plan/slice-04/check.sh docs/plan/slice-04/close-joint.cases.txt .out/test262-base/runs.tsv .out/test262-base/runs.tsv` prints `check.sh: <runs before> and <runs after> are the same file` and exits 2.
  - It exits 0 only with the copy `.out/base-runs-copy.tsv` (checks.log:45, "joint over B B-copy"). The substance holds; the command as written does not. Origin: introduced.

**5. Attacked and not broken**

- The other 71 incoming cases: each reaches its slice-4 part outside a swallowing `try`. All 72 are counted in `04` and pass in all 142 runs. None has a feature or flag line, and none uses a later-slice token (only matches inside comments: "using"). Fully read: 9 incoming cases and the 7 left out; the other 63 by every line holding a slice-4 token.
- The 7 cases left in slice 19: each passes without its part (an expected `TypeError`, or `Math.PI` read as undefined on both sides). The unit is right on all seven.
- The three moves: the base-run messages are `ReferenceError: Proxy is not defined` and `BigInt literal is not supported before slice 12`. later-parts.tsv gives `Proxy 9` (line 318) and `0n 12` (line 329). Keeping `from` 3 matches the closes of slices 5, 6 and 10 (commits 3dbef6f6, 6c74351c, a46fe885).
- The block: no file has two rule lines, every `dir <file>` line is a case in the runs, the output block equals the run's slice lines, and the sums are 44,931 / 85,794 and 53,801 / 103,360.
- Regressions: none from the base run to after, none from main to after (awk over the runs files). Cases and runs are 44,812 / 85,556 in main, base and after.
- limits.md and section 12: the story's stale-text grep finds nothing missing from section 7 of for-coordinator.md.
- The other two of the three commands the unit called impossible (the sum check, `all 1`): the unit is right.

**6. Paths written outside the worktree**

- <scratch>/wave/slice-04-close/adversary-1/block.txt
- <scratch>/wave/slice-04-close/adversary-1/probes-alone.log
- <scratch>/wave/slice-04-close/adversary-1/suite.log

```findings
- file: docs/plan/slice-04/close-into-4.tsv
  line: 72
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: regress-596805-2.js is taken into slice 4 although it completes without Math.pow and Math.tan, so it passed at the start and does not come in by the rule the closes of slices 7 and 10 applied.
- file: docs/design/engine.md
  line: 3745
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The 66 cases are said to have failed at a46fe885 only by a difference of counts, which F1 shows is not exact; the start run is needed to read each case's first failure as the slice 10 close did.
- file: crates/swell/tests/consumer_slice_04.rs
  line: 30
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The consumer test stays green with an identity Math.ceil, and its two Math.max references use a function that existed before slice 4, so three of nine references cannot fail if slice 4 is removed.
- file: <scratch>/wave/slice-04-close/for-coordinator.md
  line: 140
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: check.sh given the base run as both second and third argument exits 2 with "are the same file"; the stated exit 0 holds only with a copy of the base run.
```

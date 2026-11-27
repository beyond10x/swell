---
format: aep.planning-md/3
id: review-result:adversary-close-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the unit close: the incoming list holds case by case; 5 notes, 2 red cases on the consumer test'
relations:
- reviews: story:slice-04-conformance-close
revision: 1
---
Adversary, pass 2 (the last) on the unit close of wave big-1 (impl/slice-04-close at 76195b35), 2026-11-26.
Verdict: red on 2 cases of the consumer test. 5 findings, all notes, all introduced.

unit: close (story:slice-04-conformance-close), `impl/slice-04-close` at `76195b35`, plus my uncommitted cases
verdict: red (2 red cases; every finding is a note, no blocker)
cases: executed 3→6, red 2
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: 41 paths, all under `<scratch>/wave/slice-04-close/adversary-2/` (part 6)
needs-coordinator: yes. F5 is about the story's own text (line 86), and only you can decide whether the consumer test keeps the consumer's real calls.

**1. Diff stat**

```
 crates/swell/tests/consumer_slice_04.rs | 52 +++++++++++++++++++++++++++++++++
```

Only a test file changed. I planted the mutants in `math.rs` while no job ran and took them back: `git diff -- crates/swell/src | wc -l` prints 0. Nothing is committed or pushed. Four pool jobs ran; the first was misdirected (part 3).

**2. Cases added** (appended to `crates/swell/tests/consumer_slice_04.rs`)

- `adversary_the_references_tell_each_math_function_from_its_rounding_neighbours`: **red**. It swaps each Math function the script calls for a neighbour that rounds another way. All five swaps leave the nine texts as the standard gives them (:157):
  `the nine texts are those of the standard with: Math.floor answering as Math.trunc; Math.ceil answering as Math.round; Math.round answering as Math.ceil; Math.trunc answering as Math.ceil; Math.trunc answering as Math.round`
- `adversary_the_trunc_reference_sees_the_sign_of_its_zero`: **red**. A `Math.trunc` that answers +0 for -0.5 still gives the standard's texts (:172):
  `assertion left != right failed: Math.trunc answering +0 for -0.5 gives the nine texts of the standard`
- `adversary_probe_the_nine_texts_in_one_line`: green on the unit's code. It exists so the mutant run shows every reference that changed, not only the first.

Run of these cases alone: `3 tests run: 1 passed, 2 failed`, exit 100.

**3. Pool jobs, in order**

- Job 1 was misdirected: `slot task -t taskfiles/test262.yml test262-check` ran from `taskfiles/`, printed `test262-sources: no Cargo.lock in .` and exited 201. It left an empty `taskfiles/.out/test262`, which I removed with `rmdir`.
- Job 2: `slot sh -c 'task test262-check; task test262-moves-check'`. Last lines: `22 slice lines equal the design`, `sources digest 590d2ec3… equals conformance/test262/expected.digest`, `test262-moves: 1964 case(s) moved … slices 1 to 19: 44931 85794`. Both exited 0.
- Job 3: the cases alone, then the module, then the module with `collect-every-safe-point`. Both module runs: `6 tests run: 4 passed, 2 failed`, exit 100. The unit's 3 tests pass in both modes. Every test takes under 0.6 s.
- Job 4, against the mutants, with `--no-fail-fast`: `6 tests run: 4 passed, 2 failed`, exit 100.

The four mutants of job 4, in `math.rs`:
- `Min` returns the highest value.
- `Ceil` calls `fmath::floor`.
- `Round` calls `fmath::trunc`.
- `Trunc` answers +0 for any zero.

The probe printed `left: "1700000000000,4,3,4,0,3.5,0.5,2,3"` against `right: "…,3,2,0,3.5,0.5,3,4"`:
- Three references failed on their part: ref 4 (`Math.min`), ref 8 (`Math.ceil`) and ref 9 (`Math.round`).
- Ref 5 stayed "0", so the mutant that loses the sign of `trunc`'s zero survives.
- The unit's main test fails at ref 4.
- The unit's two new tests stay green under all four mutants. They only check that the script depends on each part (by deleting it or replacing it with the identity). They do not check the engine's functions.

Which part each reference fails without:

| reference | part |
|---|---|
| 1 and 2 (the two `Date.now()` reads) | `Date.now` |
| 3 | `Math.floor` |
| 4 | `Math.min` |
| 5 | `Math.trunc` |
| 6 and 7 | `Math.abs` |
| 8 | `Math.ceil` |
| 9 | `Math.round` |

**4. Findings** (on `76195b35`)

- **F1: the consumer references cannot tell neighbouring Math functions apart, or the sign of a zero.**
  - Red at `consumer_slice_04.rs:157` and `:172`; the engine mutant on `trunc` survived.
  - The docs at `consumer_slice_04.rs:26-27` and `:117-120` say `Math.trunc(-0.5)` "gives -0". The joined text cannot show that sign.
  - Reached by the story's filter `test(/^consumer_slice_04::/)`. Introduced (the correction chose these arguments).
- **F2: the close text says every message names a part of slice 4; 41 of the 72 name none.**
  - `engine.md:3748-3749`. The 41 are: `the value is not a function` 30, `Cannot use a property of undefined` 9, `Expected a RangeError but got a TypeError` 1, `counted refusal: 26 refusal(s)` 1.
  - The part comes from reading the case's source. That reading checks out: 84 of 84 named parts occur in their case's source, and the only new global objects from `a46fe885` to the head are `Date`, `parseInt` and `parseFloat`. Introduced.
- **F3: `engine.md:3766` says "93 of them the `not-a-constructor` cases"; S shows 92 of the 98.**
  - The 93 was kept from the old sentence about 99 cases. Of the 96 such cases in slice 9 now, 4 already passed at S. Introduced.
- **F4: `for-coordinator.md:112` (the changelog entry) says "each of its five functions of `Math`".**
  - The identity test loops over six: floor, min, trunc, abs, ceil and round. Introduced.
- **F5: `for-coordinator.md` §11 (:212) gives "old lines of the story's nine references", but the story holds no such lines.**
  - The sentence that is now false is story line 86: "the seven Math calls … with the argument values its test gives them".
  - `builtins_math.rs:352,358,360` still uses `(2 - 5, 0)`, `300 / 100` and `(3, 4)`.
  - References 5 and 9 now call `Math.trunc` and `Math.round`, which the consumer's harness does not call. Introduced, as the correction ordered.

**5. Attacked and not broken**

- **S is the run of `a46fe885`:**
  - Its `expected.txt` and `lines.txt` are byte-equal (`cmp` exit 0) to `git show a46fe885:conformance/test262/expected.txt`, and its digest equals the one committed there.
  - The tree's copy of its runs file is byte-equal to the start tree's.
  - S has the same 85,556 path and mode pairs as A and B, with no duplicates.
  - `conformance/test262/PIN` is unchanged since `a46fe885`, so S used the same test262 revision.
- **Incoming list, case by case** (`incoming.awk`): all 72 lines hold.
  - Every run of each case failed in S on the cause the file states. Two `**` refusals differ from it only in the line:column.
  - Every case is counted in slice 4 in A, passes in all 142 runs, and was in slice 19 in B.
  - None has a `features` entry or a flag line of a later slice, and `tokens.awk` finds no later-slice token in them (it does find one in a known case).
- **Other direction:** 98 cases, all slice 9, failed at S and pass in every run in A. All 98 carry `feature Reflect.construct 9` (causes: Date 47, isConstructor 44, parseInt 6, parseFloat 1). No other later slice has such a case.
- **Slice 19 from S to B:** 604 → 676 passed; 72 went from failed to passed, all 72 on the list; 0 the other way; no case changed slice.
  - Exactly 8 slice-19 cases hold a slice-4 token and passed at S; they are the 8 the text names.
- **Numbers against A:**
  - `expected.txt` equals the run's lines (`cmp` exit 0). Slice 4 is `1681 3320`, all passed; slice 19 is `1363 2550`; the sums are `53801 103360`.
  - The section-12 table and output block are right. The documented check (`slices.awk` taken from `engine.md`) reproduces the output block exactly. The `.cache/inventory/slices.awk` in the tree is an older copy that gives other numbers; it is ignored and not the unit's.
  - No run that passed in B or in S fails in A; outcome and cause are identical between B and A.
  - `moves.tsv` and `limits.md` agree with the run.
- **Section 8, run as written:** the base run against its copy prints 0 `failing` lines and exits 0; the S run against a copy prints 6 and exits 1. It rewrites the ignored `.out/base-runs-copy.tsv` with the same bytes.
- **Scripts of the story's acceptance:**
  - `check.sh` on all 22 story and guard lists plus the joint list: exit 0, 0 `failing`, 0 `regressed` each.
  - Checks (1) to (4) hold. The sum command prints `53801 103360` against `44812 85556`, and the moves command prints `12 1 / all 1`, as the unit reported. `pending slice 4` is 0.
  - `check.sh`, `scan.awk` and `part.sh` read ignored files (`.out/*/runs.tsv`, `.cache/inventory/meta.tsv`, the checkout). No gate task or workflow calls them.

**6. Paths written outside the worktree**

All files are in `<scratch>/wave/slice-04-close/adversary-2/`:

- Scripts: `s1.sh`, `incoming.awk`, `parts.sh`, `tokens.awk`, `other.sh`, `s19.awk`, `sentences.sh`, `scripts.sh`, `howcounts.sh`, `mutants.sh`
- Outputs and intermediate files: `expected-a46fe885.txt`, `S.keys`, `A.keys`, `B.keys`, `incoming.out`, `tokens.out`, `sanity.tsv`, `other.paths`, `other-features.txt`, `later-features.tsv`, `block.txt`, `s19.out`, `s19A.paths`, `s19A.tok`, `s19A.tokpaths`, `s19A.tokstatus`, `start-runs-copy.tsv`, `check.out`, `math.rs.orig`
- Logs: `test262-check.log`, `test262-check-2.log`, `sec8-base.log`, `sec8-start.log`, `job3.log`, `job4-mutants.log`
- `inv/`: `meta.tsv`, `slices.awk`, `rules.txt`, `measured.txt`, `stated.txt`

```findings
- file: crates/swell/tests/consumer_slice_04.rs
  line: 117
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The nine texts stay those of the standard when floor answers as trunc, ceil and round answer as each other, trunc answers as ceil or round, or trunc loses the sign of -0, so the references tell each function only from the identity, not from a wrong neighbour.
- file: docs/design/engine.md
  line: 3748
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The close says each of the 72 failed with a message that names a part of slice 4, but 41 messages in S name no part; the part comes from reading the case source.
- file: docs/design/engine.md
  line: 3766
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 92 of the 98 slice-9 cases that failed at S and pass now are not-a-constructor cases, not 93 as the text says; the number was kept from the earlier sentence about 99 cases.
- file: <scratch>/wave/slice-04-close/for-coordinator.md
  line: 112
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The changelog entry says the identity test replaces five functions of Math, but it loops over six (floor, min, trunc, abs, ceil, round).
- file: <scratch>/wave/slice-04-close/for-coordinator.md
  line: 212
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Section 11 gives old and new script calls as lines of the story, but the story holds no such lines; the sentence that is now false is story line 86 (the Math calls with the argument values of the builtins_math test), and references 5 and 9 now call trunc and round, which the consumer's harness does not call.
```

# Slice 2, wave 4 of 4: the close. Every case of slice 2 passes

One wave of the [wave plan](waves.md), 2026-11-21. Three units, each implemented by a
sub-agent on Opus in its own worktree; the coordinating session wrote no code, and added two
lines to the root `Taskfile.yml` as the story asks. Every commit is the organization bot's.

## Units

| unit | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `array-tostring-early` (no story; its brief was its specification) | `Array.prototype.toString`, complete (`sec-array.prototype.tostring`): Get `join`, call it, or fall back to the intrinsic `%Object.prototype.toString%`, which the realm now holds as a root. One item in the design under "Slice 2 takes these parts early" | 341 tests in the workspace; `test-collect: 38 test suites: 238 passed`; 20 cases more pass | `ab81a8e` |
| `try-completion-value` (no story; its brief was its specification) | The completion value of a `try` statement whose `finally` block is left by `break` or `continue` is `undefined`, as `UpdateEmpty` says; it was the value the `try` or `catch` block had stored. One line in the compiler. 81 forms per mode were wrong when read through `eval` | 330 tests; one case more passes | `1918a9f` |
| `slice-02-conformance-close` | 1,008 failing cases of slice 2 move to the slice they need, each with its evidence in `conformance/test262/moves.tsv` (1,032 lines: 24 more are cases of slice 3 that the new line `feature exponentiation 4` raises). `conformance/test262/later-parts.tsv`: 319 tokens of later-slice constructs and builtins. The task `test262-moves-check` and its planted script. Section 12 of the design: 933 rule lines more, the paragraph of groups, the table and the output block | 341 tests; `gate-selftest: 32 passed`; `test262-slice 02 4259 7543 0 0 4259 4259 0 0` | `f02045f` |

Each unit's claim was checked by the coordinator before its commit by running the tests of the
workspace, clippy, `test-collect` and `slot task test262` again in the unit's tree, and by
comparing its run lines with the base run by run. For the close, the list of moved cases was
checked against the coordinator's own base run lines as well: every moved case of slice 2 fails
on the base, and no moved case passes in either mode.

**What works now.** Slice 2 is delivered: all 4,259 cases that are slice 2's pass in all 7,543
runs. `[] + ""` is `""`. `slot task test262` counts 7,742 of the 44,812 run cases as passed
(7,721 before the wave).

**How the slice was closed.** After wave 3, 1,010 cases of slice 2 failed. The close unit
classified each one in a first phase and wrote no design text before the result was known:

- 1,008 need a later slice. For 791 the parser's refusal names the slice; the others were read:
  a builtin of a later slice the case calls (`Object.preventExtensions`, `.apply`,
  `String.fromCharCode`, `parseInt`, `Math`, `Date`, `RegExp`), or a construct met inside
  `eval` or `Function` (a counted refusal). By target slice: 3: 78; 4: 101 (41 by the feature
  `exponentiation`, 60 by `parseInt`, `Math`, `Date`, `toFixed`); 5: 136 (identifiers outside
  ASCII 84, regular expressions 43, `replace` and `split` 9); 6: 487 (`let` 180, `const` 65,
  arrow functions 53, functions in blocks 46, default parameters 45, spread and rest 36,
  computed names 30, and smaller groups); 7: 41; 8: 85; 9: 2; 10: 74; 11: 4.
- 2 were slice 2's own and were not moved: `expressions/concatenation/S9.8_A5_T2.js`
  (`Array.prototype.toString`, decided at the boundary of wave 3) and
  `statements/try/completion-values.js` (a defect of the compiler's completion values, in the
  tree since slice 1 and visible since `eval` returns a completion value). Each got a unit of
  its own; the close unit ran its second phase on a tree that held both.

**What keeps the list honest.** A line of `moves.tsv` names the case, the slice it leaves, the
slice it goes to, and its evidence: a token of `later-parts.tsv` with that slice that is found
in the case or a harness file it includes (comments removed, a token that begins or ends with
a word character only at a word boundary, so that `let` is not found in `delete`), or a feature
of the case that a `feature` line of the design names. `test262-moves-check` runs in the gate
on committed files and the pinned checkout only, and fails when a line has no such evidence,
moves a case into slice 2 or out of slices 1 to 19, or disagrees with the slice the design's
block gives the case. Its planted script changes one evidence, one target and plants a `let`
inside a word, and each must be refused. What needs a run (every moved case fails, none on a
part of slice 2) was checked once here, by the unit and by the coordinator, and is not a check
of the gate.

**The counts.** Slice 2 has 4,259 cases and 7,543 runs (5,267 and 9,217 before). The slices
that received cases: 3 has 7,635 (7,581), 4 has 1,323 (1,198), 5 has 2,254 (2,118), 6 has 4,204
(3,717), 7 has 1,473 (1,432), 8 has 6,273 (6,188), 9 has 1,752 (1,750), 10 has 6,647 (6,573),
11 has 1,281 (1,277). The 22 lines sum to 53,801 cases and 103,360 runs as before, slices 1 to
19 to 44,931 and 85,794.

## What went wrong, and what was decided

- **The builtin needed a file outside its list.** The clause of `Array.prototype.toString`
  names the intrinsic `%Object.prototype.toString%`, which the realm did not hold. The unit
  stopped with a patch instead of reading the property at call time (a wrong answer once a
  script replaces it); its list was extended by `realm/intrinsics.rs`.
- **Decisions.** The 19 cases `let-{array,block,identifier}-with-newline.js` move to slice 6:
  their subject is the lookahead rule of the `let` declaration. The harness case
  `asyncHelpers-asyncTest-func-throws-sync.js` moves from slice 2 to slice 3 (`globalThis` in
  its harness file); wave 3 had left it in slice 2. 159 tokens of the scan that no line uses
  stay in `later-parts.tsv` as the record of the scan.
- **A number of the story.** It estimated about 1,000 moves and about 4,269 cases staying,
  before the code and before 22 harness cases came to slice 2; measured: 1,008 and 4,259.
- **go-task.** A variable that is both exported and passed on the command line keeps the
  exported value in a task's `env:` and the command line's value in its templates (task
  3.54.0; shown with a nine-line task file by the close unit). The task `test262` reads its
  paths through `env:`. Recorded in the wave plan for the next change of that task file.

## The gate on the wave branch

`slot task check` with the variables of the runner (`CI=true GITHUB_ACTIONS=true
CARGO_TERM_COLOR=always CARGO_INCREMENTAL=0 RUSTC_WRAPPER=""`), on the wave branch with the
three units merged and `test262-moves-check` in `check`, exit 0:

```
gate: variables in effect: SPEC_REFUSALS_MAX=41 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log ESS_SUITE=.out/suite.json ESS_UNSUPPORTED_MAX=44
python-check: no Python file among 1009 file(s)
forbid-unsafe-check: 8 crate root(s), each with #![forbid(unsafe_code)]
Cargo.lock: 7 package(s), none from outside this repository
test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked
data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present
unicode-tables: 1 table(s) equal the generator output
swell v1 — 2 file(s), 8 scenario(s), valid
58 scenario(s) (8 authored), 41 refusal(s), written to .out/suite.json
  58 scenarios: 58 passed, 0 failed, 0 error, 0 unsupported
host-requirements: 164 requirement(s), 164 line(s): 80 test, 84 pending, 0 omitted; 51 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 3:3 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1
62 test suites: 341 passed, 0 failed, 1 ignored
test-collect: 38 test suites: 238 passed, 0 failed, 1 ignored
test-faults: 1 test suites: 1 passed, 0 failed, 0 ignored
conform-engine: planted defect refused: test a_target_whose_interrupt_does_nothing_fails_a_scenario ... ok
  58 scenarios: 14 passed, 0 failed, 0 error, 44 unsupported
test262-refused 46493
test262 total 53801 8982 7 44812 7742 37070 0
test262: 221 directory lines sum to the total
test262-slice 01 31 62 0 0 31 31 0 0
test262-slice 02 4259 7543 0 0 4259 4259 0 0
test262: 22 slice lines equal the design
test262-errors 0
test262: 286 lines equal conformance/test262/expected.txt
test262-moves: 1032 case(s) moved, 3:78 4:125 5:136 6:487 7:41 8:85 9:2 10:74 11:4; slices 1 to 19: 44931 85794
gate-selftest: 32 passed, 0 failed
```

## Where each finding went

| finding | where it went |
|---|---|
| `staging/sm/object/15.2.3.4-01.js` (slice 19) now ends on the missing `JSON.parse` | slice 3 |
| An exported variable beats the command line in a task's `env:` | the wave plan, "What later slices take over" |
| 20 cases of slice 2 moved to slice 3 only for `Function.prototype.apply`, 22 for `Object.preventExtensions`, 37 to slice 4 for `parseInt` (33 of them `with` cases) | slices 3 and 4; they come back as passing cases of those slices |

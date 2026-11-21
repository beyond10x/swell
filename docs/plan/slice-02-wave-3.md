# Slice 2, wave 3 of 4: `eval`, the global `Function`, the count of refusals

One wave of the [wave plan](waves.md), 2026-11-21. One unit, implemented by a sub-agent on
Opus in its own worktree; the coordinating session wrote no code. Its base is `main` after
wave 2 and [the run loop's frame](slice-02-run-loop-frame.md). Every commit is the
organization bot's.

## Unit

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `eval-and-function-constructor` | `eval`, direct and indirect, and the global `Function`, through one entry (`crates/swell/src/dynamic_code.rs`): PerformEval with EvalDeclarationInstantiation, CreateDynamicFunction. A non-strict direct `eval` uses the caller's environments and can add deletable bindings to them; a strict one gets one new declarative environment. The two host hooks, `host_ensure_can_compile_strings` and `host_get_code_for_eval`. `Agent::refusals()`: the engine counts every refusal it raises, at parse or run time, caught by the script or not, and the runner fails a run in which the count moved. Ten host requirements have their test: HOST-116, 117, 118, 139, 140, 141, 152, 162, 163, 164 | 330 tests in the workspace; `test-collect: 38 test suites: 227 passed`; 1,459 cases more pass | `b6d6477` |

The unit's claim was checked by the coordinator before its commit, in the unit's tree: the
tests of the workspace (the stack tests among them, in the forge's build), clippy,
`test-collect`, `test-faults`, the host requirements, the tasks `suite` and `conform-engine`,
and `slot task test262` (exit 0 each), and its run lines were compared with the base run by
run.

**What works now.** A script may call `eval` and `Function`: `eval("var x = 1")` adds `x` to
the caller's scope in non-strict code, `(0, eval)(...)` runs in the global scope,
`Function("a", "b", "return a + b")` makes a function whose scope is the global environment.
A host is asked before a string is compiled and may refuse or replace it. On the wave branch
`slot task test262` counts 7,721 of the 44,812 run cases as passed (6,262 before the wave),
and slice 2 passes 4,257 of its 5,267 cases (3,743 of 5,245 before; see the next paragraph for
the 22 cases more).

**Slice 2 has 22 cases more.** The 22 `test/harness` cases that wave 5 of slice 1 placed in
slice 3 because they need `Function` (the 14 `propertyhelper-*` cases, seven `verifyProperty-*`
cases and `fnGlobalObject`) are placed in slice 2, where `Function` now is; the design's rule
lines and its table follow in the same change, as the design's decision asks (5,267 cases and
9,217 runs for slice 2; 7,581 and 14,994 for slice 3). One case stays in slice 3
(`asyncHelpers-asyncTest-without-async-flag`: `Object.hasOwn`, `globalThis`).

**Runs, one by one.** Against the base (the run lines of wave 2's close): 2,693 runs pass
newly. 14 runs that passed on the base fail now, every one of them because a refusal is now
counted: the script caught the `SyntaxError` of a construct the parser does not have yet
(`let`, `const`, `class`, `super` inside `eval`) and the run passed on the base without having
tested anything. They are 7 cases of slices 6 and 8 (`eval-code/direct/` and `indirect/`:
`lex-env-no-init-const`, `-let`, `-cls`, and `direct/super-prop-expr-no-home-no-eval`), each in
both modes. That is what the count is for (the design, decision 11): slices 6 and 8 pass 4 and
3 cases fewer, and get them back when they parse those constructs. 2,286 runs carry the
detail `counted refusal:`.

**Refused runs went up, by the new rule.** `test262-refused` reads 46,493 (44,212 before).
`propertyHelper.js` now runs (5,100 runs ended in it on the base with "Function is not
defined"), and a run now also counts as refused when the engine's count moved during it, not
only when its own parse was refused. No run ends with "eval is not defined" or "Function is
not defined".

## What went wrong, and what was decided

- **A unit asked for the Python version again**, the fourth, with the rule in its brief and in
  its start message. It changed no file. The rule stays as it is written; the count is kept
  here.
- **A lookup that is not charged.** The bindings a non-strict direct `eval` adds to a record
  are kept in a list and found by a scan, as an object's properties are; the comparisons of
  that scan are not charged as steps. The design gives property lists an index map above a
  small size, which is not built yet. Both scans go to the slice that builds it
  ([wave plan](waves.md), "What later slices take over").
- **Decisions the unit took, kept.** The hook `host_get_code_for_eval` gets the original
  object as its body argument, as PerformEval says. GetPrototypeFromConstructor runs after
  the parse, in the order of CreateDynamicFunction. EvalDeclarationInstantiation runs with the
  frame of the eval code pushed; when it throws, the frame is dropped and the realm restored.
  Indirect `eval` and `Function` get a script record without host data; a direct `eval` copies
  its caller's. The walk over catch environments of Annex B.3.4 and B.3.2.3 are left to slice
  18. The runner reads the count of refusals after every parse and after the negative check,
  just before the first evaluation.
- **A number of the story.** The story writes the detail of `$262.evalScript` with a
  `SyntaxError: ` prefix that the runner does not print; the test uses the message as the
  runner prints it.

## The gate on the wave branch

`slot task check` with the variables of the runner (`CI=true GITHUB_ACTIONS=true
CARGO_TERM_COLOR=always CARGO_INCREMENTAL=0 RUSTC_WRAPPER=""`), on the wave branch with the
unit merged, exit 0:

```
gate: variables in effect: SPEC_REFUSALS_MAX=41 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log ESS_SUITE=.out/suite.json ESS_UNSUPPORTED_MAX=44
python-check: no Python file among 995 file(s)
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
62 test suites: 330 passed, 0 failed, 1 ignored
test-collect: 38 test suites: 227 passed, 0 failed, 1 ignored
test-faults: 1 test suites: 1 passed, 0 failed, 0 ignored
conform-engine: planted defect refused: test a_target_whose_interrupt_does_nothing_fails_a_scenario ... ok
  58 scenarios: 14 passed, 0 failed, 0 error, 44 unsupported
test262-refused 46493
test262 total 53801 8982 7 44812 7721 37091 0
test262: 221 directory lines sum to the total
test262-slice 01 31 62 0 0 31 31 0 0
test262-slice 02 5267 9217 0 0 5267 4257 1010 0
test262: 22 slice lines equal the design
test262-errors 0
test262: 286 lines equal conformance/test262/expected.txt
gate-selftest: 31 passed, 0 failed
```

## Where each finding went

| finding | where it went |
|---|---|
| The scan over the bindings `eval` adds is not charged, as the scan over an object's properties | the wave plan, "What later slices take over": the index map of the design |
| 7 cases of slices 6 and 8 passed only because a refusal went unseen | no action: they fail until their slice parses `let`, `const`, `class` and `super` in eval code |
| `Array.prototype.toString` is missing: 11 cases fail on it in 22 runs, 1 of slice 2 (`test/language/expressions/concatenation/S9.8_A5_T2.js`), 9 of slice 3, 1 of slice 19; `Array.prototype.join` exists | decided at this wave's boundary; wave 4 carries it out |
| The stack: the two new stack tests (`eval` and `Function` at the deepest re-entry with the parser at full nesting) pass on seven eighths of the promise; the constant is unchanged | nothing to do |

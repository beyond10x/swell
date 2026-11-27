---
format: aep.planning-md/3
id: review-result:evalprivate-correction-2-coordinator-read
kind: review-result
status: active
title: The coordinator's read of correction 2 of the unit evalprivate
relations:
- reviews: story:private-names-in-eval-code
revision: 1
---
Coordinator's read of correction 2 of the unit evalprivate (impl/evalprivate, 5b6335d to
0cbe31e), 2026-11-27, after the adversary's two passes: the correction changes no line of the
engine outside doc comments and test modules. Verdict: no defect.

The adversary's pass 1 was red on `82682c7` (`review-result:adversary-evalprivate-pass-1`):
every direct eval read every named slot of its caller's records. Correction 1 (`5b6335d`)
scans the eval String for a `#` code unit first and walks the records only when it holds
one. Pass 2 on `5b6335d` was green (`review-result:adversary-evalprivate-pass-2`), with one
warning and five notes. Read: the diff of `898ea75`, `36cd59b` and `0cbe31e`, the merge
`8ca1934` of `wave/big-2` at `eced569`, and the unit's two reports of the correction.

- The diff under `crates/*/src` between `5b6335d` and `36cd59b`, outside test modules, holds
  comment lines only (checked with `git diff -U0` and a filter for lines that are not
  comments: none). `0cbe31e` changes the test module of `crates/syntax/src/parser/class.rs`.
- The adversary's six cases are committed with their assertions (`898ea75`): the answers of
  eval texts without a `#` equal those of a walk, the scan in every form of a String, a
  String made by the host, an interrupt and the step budget around the scan, and two sweeps
  of the step budget across the walk's records.
- The warning: the three sweeps of correction 1 began after the walk's records, so a leak
  on a stop at a record passed them. Now five tests cover the budgets from the walk's first
  record to the answer without a gap (4799 to 5083 on the merged head), each named after
  what stops in its range: the inner block records, the outer block records with the
  method's and the class's record, the compares of the class record, the parse's charge,
  the run of the eval code. The bounds are read from the run (the step count, the cost of
  the walk as the difference of four evals, the parse's charge found from the end), and
  the test asserts that the ranges join. Three leaks planted by the adversary's means are
  each red in the test of that name and in no other: at budget 4831 (a record of the
  walk), 4865 (the compares), 4958 (the parse's charge). Budgets inside one charge stop at
  the same place; a sweep steps over them after two budgets stopped in the same charge.
- The notes on three small changes of the scan: each is red in the adversary's case
  `the_scan_finds_a_hash_in_every_form_of_a_string`, now committed.
- The notes on clause numbers: the unit read the pinned text and corrected 22 sites in the
  engine's comments and tests in one pass (the record of strict eval code is made in step
  17 of PerformEval and becomes the variableEnv in step 19; the private names of eval code
  are steps 4 to 7 of EvalDeclarationInstantiation), and lists the sites that were right.
  One site is in a file of the plan (`docs/plan/slice-08/scan.awk`) and is corrected by the
  coordinator after the merge.
- A library test of the syntax crate was red on `wave/big-2` since the unit privatefields
  was merged: `parser::class::tests::the_elements_of_a_class_are_functions_of_the_tree`
  asserted that its class is held back, and that unit lifted the last hold. The wave's
  checks before a merge ran the crate's integration tests only. `0cbe31e` states what is
  true now (no construct of the class is held back; the bytes of the tree equal the
  meter's) and shows the assertion red with a hold planted for `#x in`. The coordinator's
  checks before a merge now run the syntax and the compiler crate whole.
- The merge `8ca1934`: one conflict, in a doc comment of `crates/syntax/src/lib.rs`, both
  sides kept. No step count of the unit's table moved with the merged private fields and
  the hook.

The coordinator's runs on `0cbe31e`, build node, before the merge: `swell-syntax` 91 passed
(library and main; the same with full counts); `swell-compiler` 25 passed; `swell` main 1763
passed, 1 skipped (default build) and 1735 passed, 1 skipped (collect build); `swell` lib 289
passed; `task test-stack-profiles` 7 passed in each of the two profiles; `task clippy` exit
0; `cargo fmt --all -- --check` exit 0. The unit's runs: `task nostd-build` 6 libraries;
each of the five sweeps alone between 0.51 and 1.07 CPU s in the default build, and at once
in the collect build; the module `eval_private_names` alone on one thread 5.4 CPU s
(default) and 11.4 CPU s (collect).

Not covered: no test262 run on this head (pass 2 ran the 125 cases that hold an eval and a
private name on `5b6335d`: 125 passed in 225 runs; the conformance close runs all); a
third adversary pass did not run (the budget is two); the sweeps return at once in the
collect build.

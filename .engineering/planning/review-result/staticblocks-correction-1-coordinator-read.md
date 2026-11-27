---
format: aep.planning-md/3
id: review-result:staticblocks-correction-1-coordinator-read
kind: review-result
status: active
title: Coordinator's read of correction 1 of the unit staticblocks
relations:
- reviews: story:class-static-blocks
revision: 1
---
Coordinator's read of correction 1 of the unit staticblocks (impl/staticblocks, d61a063 to
5f8a7d3), 2026-11-27. Verdict: no defect.

The adversary's pass 1 was green (`review-result:adversary-staticblocks-pass-1`: no failing
case, three notes, four planted defects each red in both builds, one change of the code that
no test caught). Read: the whole diff of the unit in `crates/compiler/src` and
`crates/syntax/src` (`compile_static_block` answers the block's own function; the hold call
of `static_block_open` and its line of `HOLDS` are gone; no file under `crates/swell/src`
changed), the diff of `5f8a7d3`, and the unit's reports.

Correction 1 (`03b9599`, the merge of `wave/big-2` at `ed8a017`, no conflict; `5f8a7d3`):

- The change no test caught: in `static_calls` the arm of an element that is neither a static
  field nor a block ends the run of static fields; without it the fields after a method are
  never defined. `class_static_blocks::a_method_between_static_fields_ends_their_run` holds
  it in both modes (a static field, a method, a static field, a block that reads both, a
  static accessor, a static field). Planted in the unit's tree it reads
  `"a,block 1 undefined,c|1,,3,g"` for `"a,b,block 1 2,c|1,2,4,g"`; the file was put back.
- Note 1 (a line printed by `fits_the_stack_bound` that the gate never shows): removed.
- Note 2 (the test `a_class_with_100_private_methods_and_100_fields_constructed_1000_times`
  runs 500 constructions in the gate's default build and 1 in a collect build): no change. The
  name is the story's; the full count runs under `SWELL_FULL_COUNTS=1` in the default build,
  and the adversary built the plant the unit had only reasoned (the step charge of the
  private find removed): red at every count, 1 included.
- Note 3 (the story names `static_block_scope` in `resolve.rs`, which was not written): no
  change of the code. The resolver's `function` gives a block its `var` scope, its body
  scope and, with a direct `eval` in its code, the binding `arguments`; the adversary's
  probes of the scope (46 sources that are a SyntaxError, 24 that run, a block inside
  `with`, a direct `eval` that reads the block's bindings) found no wrong binding. "As
  delivered" of the story says so.
- The unit's reading of a direct `eval` in a block (`arguments` in the eval code is no
  SyntaxError; it is the block's own arguments object of length 0) is the pinned draft's:
  EvaluateClassStaticBlockBody runs FunctionDeclarationInstantiation, and PerformEval's
  inClassFieldInitializer is false for a function without [[ClassFieldInitializerName]].
  The adversary read both clauses and agrees.

The adversary's part of test262 beyond the unit's list (71 cases that name the feature
`class-static-block` or hold a static block): 132 of 138 runs pass on the head, 60 on the
base, none lost; the 3 cases that fail need a private field, `using` or explicit resource
management.

The coordinator's runs on `5f8a7d3`, build node, before the merge: `swell-syntax` main 80
passed, and 80 with `SWELL_FULL_COUNTS=1`; `swell-compiler` main 17 passed; `swell` main 1677
passed, 1 skipped (default build) and 1650 passed, 1 skipped (collect build); `swell` lib 283
passed; `task test-stack-profiles` 7 passed in each of the two profiles; `task clippy` exit
0; `cargo fmt --all -- --check` exit 0.

Not covered: `task test-realm-sizes`, `task slow-tests` and the unit's test262 part ran on
`d61a063` and not again on `5f8a7d3`, which adds one test and removes one printed line; one
line of a doc comment in `crates/syntax/src/parser/class.rs` is longer than 100 columns
(`cargo fmt` does not wrap comments); the adversary's planted builds ran in the workstation's
pool, from a clone outside the managed worktrees.

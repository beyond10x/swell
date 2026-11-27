---
format: aep.planning-md/3
id: review-result:adversary-staticblocks-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit staticblocks
relations:
- reviews: story:class-static-blocks
revision: 1
---
Adversary pass 1 on the unit staticblocks of wave big-2, impl/staticblocks at d61a063, 2026-11-27.
Green: no failing case, three notes. The adversary's own report follows.

unit: staticblocks (`story:class-static-blocks`), worktree `swell-impl-staticblocks` at `d61a063`, tree clean
verdict: green
cases: executed 19→39 (default; 19→36 collect), red 0
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/staticblocks/adversary-1
needs-coordinator: no

I could not break the unit. No case is red, so I added no case to the tree. The three findings below are notes only.

## 1. `git --no-pager diff --stat`
Empty. The worktree equals `d61a063`: `git diff --quiet` printed "tree-clean". My 21 probe tests are saved in `adversary-1/class_static_blocks.rs.probes`. I took them out of the tree because all are green, and two of them run over the time bound (4.77 s and 1.99 CPU s).

## 2. Cases written
I added 21 probes to `crates/swell/tests/class_static_blocks.rs`, named `adversary_*`. All of them are green now.

The first run alone (`run-1-default.log`) printed `17 tests run: 14 passed, 3 failed`. All three failures were my own errors, not the engine's:
- **Computed keys:** the engine logged `key s2` before `field s1`. That is correct: every computed key, including a later static one, is evaluated before any static element runs.
- **`() => await` in a block:** the engine runs it. That is correct by the pinned draft: an arrow's ConciseBody is `[~Await]`, `Contains` does not look into arrow functions for `await`, and test262 `static-init-await-binding-valid.js` says the same.
- **The budget test's own logic:** at step budgets 92 to 99 the run is terminated after the class is published, and the later script hits the same step budget.

## 3. Suite runs, after the probes existed
- `cargo nextest run --locked -p swell --test main --no-capture -E 'test(/^class_static_blocks::/)'` printed `39 tests run: 39 passed, 1612 skipped`, exit 0.
- The same with `--features collect-every-safe-point` printed `36 tests run: 36 passed, 1585 skipped`, exit 0.

## 4. Measurements by brief item
- **Item 1 (order):** green in both builds. Covered: computed keys before static elements, `this` in a block, an arrow and a plain function, the inner and outer binding, a throw from a block, the name of an anonymous class, blocks nested 3 deep, `super` read, call and assignment in a derived class, static private methods called from a block, a block in a derived constructor before `super()`, and a block that runs `new this()`.
- **Item 2 (early errors):** 46 bad sources are a `SyntaxError` and 24 good sources run, through direct eval, indirect eval and a `Function` body, in sloppy and strict mode.
- **Item 3 (scope):** green, including a block inside `with` and after a sloppy `eval`, and a block whose direct `eval` reads its own bindings.
- **Item 4 (direct eval):** the unit's reading is right by the draft. EvaluateClassStaticBlockBody step 2 runs FunctionDeclarationInstantiation, and PerformEval's inClassFieldInitializer is false because the block function has no [[ClassFieldInitializerName]].
  - `eval("arguments")` is the same object across evals, of length 0. Its `callee` throws a `TypeError` and a write to it stays.
  - An arrow's `eval` sees the block's object, also when the arrow runs after the class is defined.
  - `eval("var await = 3; await")` answers 3.
  - An undeclared private name is a plain `SyntaxError`, not the refusal.
  - Without a direct eval, no script can tell whether the arguments object exists.
- **Item 5 (budgets and bounds):**
  - **Step budget**, one step at a time over 100 steps: every run ended whole or terminated. Every terminated run that left no class had a heap equal to a run stopped at step 1 (395,482 B).
  - **Memory budget**, 64 bytes at a time: bound 695,450 B. 64 terminations inside the class all left one heap reading (510,191 B), and no class was reachable.
  - **Frame depth 1,000:** `f` reached 499 levels, `h` 499, `k` 333.
  - **Native stack:** 10,000 frames of block recursion ran on a 256 KiB thread. That is under 52 bytes per level, against `STACK_BYTES_PER_REENTRY` of 14,336.
  - **Many blocks** (wall time inside the test): 1 block 4.8 ms, 1,000 blocks 58 ms, 4,000 blocks 221 ms (3.8 times, linear).
  - **70,000 blocks:** `Terminated(MemoryBudget)` at 64 MiB in 0.56 s; at 1 GiB the run completes (`"70000"`) in 3.9 s.
  - **Nesting:** depths 1 to 400 each ran whole or got the nesting error, on seven eighths of the promised stack.
  - **`slot --timeout 30m task test-stack-profiles` passes:** `release: 1 test binaries: 4 passed, 0 failed, 5 skipped` and `stack-debug: 1 test binaries: 4 passed, 0 failed, 5 skipped`, exit 0.
- **Item 6 (plants), built in the clone `adversary-1/tree`.** Each plant is caught by the unit's 19 tests, with the same tests red in both builds:

  | Plant | Change | Red among the 19 |
  |---|---|---|
  | P1 | a block called with `this` undefined (`Nip 1; Undefined; Bury 1`) | 6: `a_block_runs_with_this_the_constructor`, `super_property_in_a_block`, `blocks_and_static_fields_run_in_source_order`, `a_block_loop_ends_at_the_step_budget`, `a_block_loop_ends_at_an_interrupt`, `a_recursion_of_classes_with_blocks_stops_at_frame_depth` |
  | P2 | all blocks after the last static field | 2: `blocks_and_static_fields_run_in_source_order`, `a_throw_from_a_block_leaves_the_class_binding_uninitialized` |
  | P3 | `Closure` in place of `MakeMethod`, so no home object | 1: `super_property_in_a_block` |

- **Item 7 (the lowered count):** I built P4, the step charge of `private_element_find` removed. `a_class_with_100_…` is red at every count, so the lowered counts still catch it:
  - default, 500: `570331 steps for 500 constructions`
  - default full count, 1,000: `1140331 steps for 1000 constructions`
  - collect, 1: `1471 steps for 1 constructions`
  - collect full count, 2: `2611 steps for 2 constructions`

  The test holds no static block, so a static block changes nothing in it. It belongs to the private-methods meeting cases the story took over.
- **Item 8 (held fixtures):** not a vacuous pass. A held source with no hold left must parse in both modes (`sources.rs:286`) and compile (`compile.rs:195`). `class_elements_scale_linearly` now also runs the scope analysis on its static-block shape, because that shape now parses without the refusal.
- **Item 9 (test262), 71 cases of my own:** front matter naming `class-static-block`, or a `static {` in the file, or `static-init-*`.
  - Base: 60 of 138 runs passed. Head: 132 of 138.
  - `check.sh` printed `passed 66 bound 66 B 30 F 39 R 3`, with no `regressed` line.
  - The 3 failing cases (6 runs) are outside this unit: `static-init-scope-private.js` (the private field hold), `statements/using/static-init-await-binding-valid.js` (`using`, slice 15) and `staging/explicit-resource-management/call-dispose-methods.js` (slice 19).
  - 2 decorator cases are refused as stage 3, which is why `check.sh` lists them as `missing`.
- **Item 10 (times), the unit's 19 tests, CPU seconds alone:**

  | Build | Sum | Largest |
  |---|---|---|
  | default | 1.170 | 0.834 |
  | default, full count | 1.834 | 1.499 |
  | collect | 6.584 | 1.340 |
  | collect, full count | 7.306 | 2.068 |

  Every time is within its bound.
- **Item 11 (mutants the 19 tests and the 484 cases would not catch).** The code has neither defect, so these are notes and no case was added:
  - **M1, parser `static_block_open`:** no `mem::take` of `self.functions` admits `return` in a block of a class inside a function. It is caught only by `crates/syntax/tests/sources.rs:2583` and by test262 `static-init-invalid-return.js` in a full run; the 484 list has no negative case.
  - **M2, `static_calls`:** the `_ =>` arm without `in_run = false` drops `static b` in `class C { static a = 1; m() {} static b = 2; }`. The test that catches it: `both_answer("class C { static a = 1; m() {} static b = 2; } [C.a, C.b].join();", "1,2")`. This was reasoned from the code, not built, and I did not run the 484 cases against it.
  - **Third mutant:** I did not find one. The only other candidate, the block arm without `in_run = false`, is caught by `blocks_and_static_fields_run_in_source_order` (reasoned).

## 5. Findings (all notes, covering `d61a063`)
- **`println` never seen.** `crates/syntax/tests/sources.rs:1275`: nextest hides a passing test's output, so the depth this line prints never shows in the gate. Nothing reaches it. Introduced.
- **Test name.** `crates/swell/tests/class_static_blocks.rs:543`: the name says 1,000 constructions, but the gate runs 500 (default) and 1 (collect). P4 shows the defect is still caught at every count, so only the name misleads. Introduced.
- **Story text.** `.engineering/planning/story/class-static-blocks.md:63`: the story requires `static_block_scope` in `resolve.rs`, which was not written. The behaviour holds without it; the unit disclosed this. Introduced.

## 6. Deviations of mine
- **Plants ran on the workstation.** The clone is outside the worktree directory, so slot ran its 9 jobs in this host's pool, not on the build node. The unit's statement about such copies was right. The jobs left a 1.4 GB `target/` in my work directory.
- **Two jobs overlapped.** I started `test-stack-profiles` and restored the test file while the test262 job was still copying its files back to the tree. Afterwards the tree is clean, and the stack job synced `d61a063 + 0 changed files`.
- **No lease.** I took no worktree lease.

## 7. Paths written outside the worktree
- Everything under `<scratch>/wave2/staticblocks/adversary-1/`: the clone `tree/` with its `target/` (1.4 GB), the plant copies `class.rs.{orig,p1,p2,p3}` and `private_names.rs.{orig,p4}`, the test-file copies `class_static_blocks.rs.{orig,adv,probes}`, `head.rs`, `mid.rs`, `tail.rs`, the scripts `plants.sh`, `measure.sh`, `part.sh`, `part.cases.txt`, and every log there.
- In the worktree's ignored `.out/`: `adv-part/` and `adv-part.cases.txt`. The `test-stack-profiles` and `build-stack-profiles` outputs there were written again.

```findings
- file: crates/syntax/tests/sources.rs
  line: 1275
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The depth that fits_the_stack_bound prints never reaches the gate output because nextest hides a passing test's output, so nothing reads it.
- file: crates/swell/tests/class_static_blocks.rs
  line: 543
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The test name states 1000 constructions while the gate runs 500 in the default build and 1 in the collect build; the planted missing step charge is still red at every count, so only the name misleads.
- file: .engineering/planning/story/class-static-blocks.md
  line: 63
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The story requires static_block_scope in resolve.rs and lists it under Writes, but the unit wrote no such function and the behaviour holds without it.
```

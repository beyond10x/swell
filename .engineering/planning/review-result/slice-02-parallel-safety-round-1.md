---
format: aep.planning-md/3
id: review-result:slice-02-parallel-safety-round-1
kind: review-result
status: active
title: parallel-safety critic, slice 2 stories, round 1
relations:
- reviews: story:test262-run-causes
- reviews: story:syntax-language-core
- reviews: story:number-boolean-and-global-functions-early
- reviews: story:with-arguments-and-dynamic-names
- reviews: story:eval-and-function-constructor
- reviews: story:slice-02-conformance-close
revision: 1
---
needs-revision
story:with-arguments-and-dynamic-names — its `Writes:` line leaves out `crates/syntax/tests/sources.rs`, but lifting the `with` refusal breaks the assertion there that `"use\x20strict"; with (o) {}` is refused naming slice 2. That same file also holds `eval_entry_is_strict_after_a_strict_caller`, which story:syntax-language-core says switches to `with` "once `with` is lifted", and `every_nesting_kind_fits_the_stack_bound`, which gets no `with` nesting kind (surface cited) — crates/syntax/tests/sources.rs:535, .engineering/planning/story/with-arguments-and-dynamic-names.md:54, .engineering/planning/story/syntax-language-core.md:50
story:test262-run-causes — `TEST262_RUNS` defaults to `.out/test262/runs.tsv` rather than a path under `TEST262_OUT`. So the gate's planted test262 runs overwrite the real file with an empty one: they set only `TEST262_OUT` with `TEST262_RUN=none`, and `check` runs them right after `test262`. The `wc -l` prints `0` checks of the three later stories then pass with nothing in the file whenever the file is read after `slot task check` (surface cited) — .engineering/planning/story/test262-run-causes.md:24, tests/gate/test262-refuses-a-count-that-differs-from-expected.sh:31, Taskfile.yml:58
story:with-arguments-and-dynamic-names — it and story:number-boolean-and-global-functions-early both write `crates/swell/src/realm/intrinsics.rs` (cited, both `Writes:` lines). No `depends_on` edge joins them and neither body names the other; they sit in different waves only because this story depends on story:syntax-language-core. Fix with an edge that records the shared file, or a split of the surface — .engineering/planning/story/with-arguments-and-dynamic-names.md:54, .engineering/planning/story/number-boolean-and-global-functions-early.md:33
story:with-arguments-and-dynamic-names — its `Writes:` line leaves out `crates/swell/src/script.rs`. `Instruction` is `Copy`, so the slot names of catch, function and named-function-expression records need a new table in `Code` or `FunctionTemplate`, and `code_bytes` in `script.rs` charges every vector of the compiled code to the memory budget (surface inferred) — crates/swell/src/script.rs:125, crates/compiler/src/bytecode.rs:55, .engineering/planning/story/with-arguments-and-dynamic-names.md:36

Read: the 6 stories, the graph and `waves`, and the code each story builds on. In wave 1 the three stories share no file except the coordinator's `expected.txt`. Not established: no `scope` is recorded yet, so `aep plan artifact waves` prints all six as `unassessed`; both wave 1 `Writes:` lines name the directory `crates/swell/tests/language/` rather than its files, and a scope copied from them would report a collision.

```findings
- file: .engineering/planning/story/with-arguments-and-dynamic-names.md
  line: 54
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Writes leaves out crates/syntax/tests/sources.rs, whose assertions change when the with refusal is lifted"
- file: .engineering/planning/story/test262-run-causes.md
  line: 24
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "TEST262_RUNS defaults outside TEST262_OUT, so the gate's planted runs overwrite the real file with an empty one"
- file: .engineering/planning/story/with-arguments-and-dynamic-names.md
  line: 54
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "shares crates/swell/src/realm/intrinsics.rs with story:number-boolean-and-global-functions-early without an edge"
- file: .engineering/planning/story/with-arguments-and-dynamic-names.md
  line: 36
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Writes leaves out crates/swell/src/script.rs, whose code_bytes charges the new table"
```

---
format: aep.planning-md/3
id: review-result:slice-07-round-1-acceptance
kind: review-result
status: active
title: 'Slice 7, round 1, the acceptance critic: the first story'
relations:
- reviews: story:generator-functions-and-suspended-frames
revision: 1
---
needs-revision
story:generator-functions-and-suspended-frames — the `Writes:` line leaves out `crates/test262/tests/expected.rs`, which hard-codes the printed total `test262 total 22 5 1 16 3 13 0` of the fixture tree in two diffs (lines 108-109 and 153-154); those totals move once the two `yield` fixtures parse, so the unit stops on a file outside its line before `swell-test262` exits 0 — crates/test262/tests/expected.rs:108
story:generator-functions-and-suspended-frames — check (c) reads the "seventh field" of `test262-slice 07`, but in that line field 7 is the run count (1491 on the base, already above 6 + 1,093), so (c) passes before the work as well as after; passed is field 8 there and field 7 only in the directory lines, and (c) gives no awk command naming the field — conformance/test262/expected.txt:229
story:generator-functions-and-suspended-frames — the body says the Symbol sweep "is run over the three new functions", but `functions_of_the_realm` walks only from the global object, and no global reaches `%GeneratorPrototype%` in this story, so `next`, `return` and `throw` are never called with a Symbol and the claim reads the same before and after; a test of the story's own has to make those calls — crates/swell/tests/builtins_symbol_values.rs:332
story:generator-functions-and-suspended-frames — the acceptance filter runs only `host_objects::host_187`, while `host_186_a_host_function_charges_every_byte` asserts `HOST_OBJECT_LIVE_BYTES` (256), which `ObjectKind::Generator` holding `Option<SuspendedFrame>` inline (a whole `Frame`) would raise for every object record; the body neither states that the frame is boxed nor runs `host_186` — crates/swell/tests/host_objects.rs:1029
story:generator-functions-and-suspended-frames — `cargo nextest run --locked -p swell --lib` runs only in the default mode, though the story changes `mark_phase_calls_every_site_of_the_layout`, a lib test that `test-collect` runs with `collect-every-safe-point`; the acceptance needs `--lib` in both collection modes — .engineering/planning/story/generator-functions-and-suspended-frames.md:155
story:generator-functions-and-suspended-frames — `a_generator_suspended_when_its_entry_terminates_resumes_in_a_later_entry` names no termination, so it passes with one, while the Bounds paragraph needs a suspended generator ended at `StepBudget`, `MemoryBudget` and `Interrupted` each — .engineering/planning/story/generator-functions-and-suspended-frames.md:141
story:generator-functions-and-suspended-frames — `a_completed_generator_holds_no_frame` and `return_and_throw_complete_a_suspended_start_generator_without_running_it` name no `heap_bytes()` reading, so no test shows the frame's credit at completion or at an abrupt completion of a suspended-start generator, two of the four credit paths the Bounds paragraph states — .engineering/planning/story/generator-functions-and-suspended-frames.md:139
story:generator-functions-and-suspended-frames — the CPU bounds (1.5, 2 and 2.5 CPU seconds a test, 0.5 a language script) and the report of the measured times are in Change only; the acceptance has no such condition, so a new test of 1.8 CPU seconds in the default mode still passes it — .engineering/planning/story/generator-functions-and-suspended-frames.md:155
story:generator-functions-and-suspended-frames — the Size paragraph's reason for not splitting ("none of its parts could be shown alone") does not hold for the parser: with its early errors, fixtures and nesting kinds, and the refusal moved behind the early-error pass, the parser alone is checked by the `swell-syntax` tests and the 115 negative parse cases of the list (counted from `.cache/inventory/meta.tsv`) — .engineering/planning/story/generator-functions-and-suspended-frames.md:145

Read: 1 of 1 artifact (`aep plan artifact show story:generator-functions-and-suspended-frames`), plus the epic, the acceptance of 3 accepted stories, `aep plan artifact kinds` and `lifecycle story`, `AGENTS.md` (Rules, Commands and the gate, How the work is run), `unit-invariants.md`, the records of waves 1 and 2 of slice 5, `docs/plan/slice-07/`, and the code at every match site the `Writes:` line names, with `git grep` and the story's search command run on the base.

Could not establish:
- No run of `4971b9b` exists in any worktree, so these figures were not checked: 2,081 / 2,076 runs, 4,872 refusal runs, 18,238 passes. The 1,093 cases, the 115 negative parse cases, the directory counts and the search output were checked and match.
- Whether HOST-187's reading stays under 506,112 with the new intrinsics. If it goes over, `host/limits.rs` and `conformance/host/surface.txt` change, and neither is on the `Writes:` line.
- Not counted as findings:
  - Check (b) does not select by the list. It needs no selection, because the story removes the message from the code, so 0 over all runs follows from the change.
  - The new stack test runs only in the dev profile without debug information, not in `stack-debug`. That is the convention of earlier builtin stories, and the layout of the stack tests is the design critic's lane.
  - The split question also belongs to the design and scope critics.

```findings
- file: crates/test262/tests/expected.rs
  line: 108
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the `Writes:` line leaves out `crates/test262/tests/expected.rs`, which hard-codes the printed total `test262 total 22 5 1 16 3 13 0` of the fixture tree in two diffs (lines 108-109 and 153-154); those totals move once the two `yield` fixtures parse, so the unit stops on a file outside its line before `swell-test262` exits 0"
- file: conformance/test262/expected.txt
  line: 229
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "check (c) reads the \"seventh field\" of `test262-slice 07`, but in that line field 7 is the run count (1491 on the base, already above 6 + 1,093), so (c) passes before the work as well as after; passed is field 8 there and field 7 only in the directory lines, and (c) gives no awk command naming the field"
- file: crates/swell/tests/builtins_symbol_values.rs
  line: 332
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body says the Symbol sweep \"is run over the three new functions\", but `functions_of_the_realm` walks only from the global object, and no global reaches `%GeneratorPrototype%` in this story, so `next`, `return` and `throw` are never called with a Symbol and the claim reads the same before and after; a test of the story's own has to make those calls"
- file: crates/swell/tests/host_objects.rs
  line: 1029
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance filter runs only `host_objects::host_187`, while `host_186_a_host_function_charges_every_byte` asserts `HOST_OBJECT_LIVE_BYTES` (256), which `ObjectKind::Generator` holding `Option<SuspendedFrame>` inline (a whole `Frame`) would raise for every object record; the body neither states that the frame is boxed nor runs `host_186`"
- file: .engineering/planning/story/generator-functions-and-suspended-frames.md
  line: 155
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "`cargo nextest run --locked -p swell --lib` runs only in the default mode, though the story changes `mark_phase_calls_every_site_of_the_layout`, a lib test that `test-collect` runs with `collect-every-safe-point`; the acceptance needs `--lib` in both collection modes"
- file: .engineering/planning/story/generator-functions-and-suspended-frames.md
  line: 141
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "`a_generator_suspended_when_its_entry_terminates_resumes_in_a_later_entry` names no termination, so it passes with one, while the Bounds paragraph needs a suspended generator ended at `StepBudget`, `MemoryBudget` and `Interrupted` each"
- file: .engineering/planning/story/generator-functions-and-suspended-frames.md
  line: 139
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "`a_completed_generator_holds_no_frame` and `return_and_throw_complete_a_suspended_start_generator_without_running_it` name no `heap_bytes()` reading, so no test shows the frame's credit at completion or at an abrupt completion of a suspended-start generator, two of the four credit paths the Bounds paragraph states"
- file: .engineering/planning/story/generator-functions-and-suspended-frames.md
  line: 155
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the CPU bounds (1.5, 2 and 2.5 CPU seconds a test, 0.5 a language script) and the report of the measured times are in Change only; the acceptance has no such condition, so a new test of 1.8 CPU seconds in the default mode still passes it"
- file: .engineering/planning/story/generator-functions-and-suspended-frames.md
  line: 145
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Size paragraph's reason for not splitting (\"none of its parts could be shown alone\") does not hold for the parser: with its early errors, fixtures and nesting kinds, and the refusal moved behind the early-error pass, the parser alone is checked by the `swell-syntax` tests and the 115 negative parse cases of the list (counted from `.cache/inventory/meta.tsv`)"
```

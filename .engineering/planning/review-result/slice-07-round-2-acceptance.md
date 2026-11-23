---
format: aep.planning-md/3
id: review-result:slice-07-round-2-acceptance
kind: review-result
status: active
title: 'Slice 7, round 2, the acceptance critic: the six stories'
relations:
- reviews: story:generator-syntax-and-early-errors
- reviews: story:generator-functions-and-suspended-frames
- reviews: story:generator-function-constructor
- reviews: story:yield-star-delegation
- reviews: story:generator-methods-in-object-literals
- reviews: story:slice-07-conformance-close
revision: 1
---
needs-revision
story:generator-function-constructor — check (b) asks for B + 21 − R and B1 + 18 − R1, but 2 of the 21 cases need only what `story:generator-functions-and-suspended-frames` builds and already pass on this story's base: `GeneratorFunction/prototype/Symbol.toStringTag.js` and `Object/prototype/toString/symbol-tag-generators-builtin.js` (the scan put them here only because their text contains "GeneratorFunction"). Correct work therefore misses the slice bound by 2 and the directory bound by 1, and Context's "on its base its cases end on a `TypeError` or a wrong value" is wrong for these two. Replace 21 and 18 with F and F1, the cases of the list with a failing run in `.out/test262-base/runs.tsv` (`awk -F'\t' 'NR == FNR { w[$1] = 1; next } ($3 in w) && $5 != "Passed" { f[$3] = 1 } END { print length(f) }' <list> .out/test262-base/runs.tsv`, and the same restricted to `test/built-ins/GeneratorFunction/` for F1) — .engineering/planning/story/generator-function-constructor.md:68
story:generator-function-constructor — the acceptance runs `swell-syntax` only at the gate's count, though the story changes `parse_dynamic_function`. The stack-bound check goes through that entry, and its larger fixed part only shows past the bound at the full count ("3,464 through the body of `parse_dynamic_function`"). Add `cargo nextest run --locked -p swell-syntax --test main` with `SWELL_FULL_COUNTS=1` and the 2.5 CPU-second bound at the full count — crates/syntax/tests/sources.rs:720
story:yield-star-delegation — check (c) asks for B + 49 − R and B1 + 41 − R1, but the story's own Context says the first story may already pass 3 negative parse cases of the list. Those cases are `statements/generators/yield-star-after-newline.js`, `expressions/generators/yield-star-after-newline.js` and `expressions/yield/star-in-iteration-stmt.js`, each a grammar error that now comes before the held refusal. Once they are in B, correct work misses the bounds by 3 and 1. Use F and F1 (the list's cases with a failing run on the base) instead of 49 and 41 — .engineering/planning/story/yield-star-delegation.md:90
story:yield-star-delegation — check (b) (`/yield\* is not supported/` … `wc -l` prints `0`) gives no value on its base. On `4971b9b`, the base the other checks quote, it prints `0` as well because that text does not exist there yet, so the check reads the same before and after. State what it prints over `.out/test262-base/runs.tsv`: at least the list's failing runs — .engineering/planning/story/yield-star-delegation.md:90
story:generator-methods-in-object-literals — check (b) (`/generator method is not supported/` … prints `0`) gives no value on its base. On `4971b9b` it prints `0` as well because that text first exists after the second story, so the check reads the same before and after. State what it prints over `.out/test262-base/runs.tsv` (at least 525 runs on the list) — .engineering/planning/story/generator-methods-in-object-literals.md:79
story:slice-07-conformance-close — the second half of check (4) ("no case of a slice delivered after 7 that passes in every run lacks a line unless a `feature` or `flag` line of a later slice holds it") has no command, so only the unit's report shows it. Add one that prints each such case, to be compared with the report's list: `awk -F'\t' 'NR == FNR { w[$1] = 1; next } { s[$3] = $2; if ($5 != "Passed") f[$3] = 1 } END { for (k in s) if (!(k in w) && !(k in f) && index(",10,04,09,08,11,12,13,14,15,16,17,18,19,", "," s[k] ",")) print k }' docs/plan/slice-07/close-into-7.tsv .out/test262/runs.tsv` — .engineering/planning/story/slice-07-conformance-close.md:62
story:slice-07-conformance-close — check (1) prints `0` and does not say what it prints on the base (2,825 failing runs of slice 7 on `4971b9b`, the number the second story quotes) — .engineering/planning/story/slice-07-conformance-close.md:62
story:generator-syntax-and-early-errors — the story mentions neither the Symbol sweep nor HOST-187, which every story must carry. Add after the "Time" bullet: "No new function of the realm, so neither the sweep with a Symbol (`crates/swell/tests/builtins_symbol_values.rs`) nor HOST-187 (`host_objects::host_187_...`) is touched." — .engineering/planning/story/generator-syntax-and-early-errors.md:81
story:generator-syntax-and-early-errors — the story does not say how the held refusal is recorded. If it is a new function, struct or enum that only the record uses, it loses its last use when `story:yield-star-delegation` removes the last call site. Clippy runs with `-D warnings` and that story's `Writes:` line holds neither `parser/mod.rs` nor `lib.rs`, so its clippy step would fail. Add to the `parser/mod.rs` bullet: "The record is made at its call site, the first one kept, as a value of an existing type (an `Option<SyntaxError>` built by `SyntaxError::refused`, `crates/syntax/src/error.rs` line 84), with no new function, struct or enum that only the record uses, so that a later story that removes the last call site leaves no item `dead_code` names." — Taskfile.yml:509

Story 2, round 1 findings of this role:
- closed — `expected.rs` (and `sections.rs`) are now on the `Writes:` line: `crates/test262/tests/{runner,expected,sections}.rs` — generator-functions-and-suspended-frames.md:150
- closed — check (c) now reads field 8 of the slice line with a named awk (`{ print $8 }`) and field 7 of the directory lines — :154
- closed — the Symbol claim is replaced by the story's own test `next_return_and_throw_take_a_symbol`, and `builtins_symbol_values.rs` does not change — :138
- closed — the frame is boxed (`SuspendedFrame`, "in a `Box`"), `host_186` is in the filter, `a_generator_keeps_the_object_record_at_its_size` checks the record size, and `git diff --quiet "$BASE" -- …limits.rs …surface.txt` is in the acceptance — :132, :154
- closed — `--lib` runs in both collection modes — :154
- closed — there are now three tests of a suspended generator, one each for StepBudget, MemoryBudget and Interrupted — :138
- closed — `a_completed_generator_holds_no_frame` and `return_and_throw_complete_a_suspended_start_generator_without_running_it` name `heap_bytes()` readings of at least 3,200 — :138
- closed — the CPU bounds and the measured report are in the acceptance ("a time over its bound fails this acceptance"); the 2.5 bound at the full count is missing, but the story adds no test that reads `SWELL_FULL_COUNTS` — :154
- closed — the parser part is cut out into `story:generator-syntax-and-early-errors` — :144

Read: 6 of 6 stories in full (`aep plan artifact show` for each), `review-result:slice-07-round-1-acceptance`, `aep plan artifact kinds` and `lifecycle story`. I checked the following on the tree at `a732ac4`:
- the 20 fixture lines, by the story's own `git grep`;
- the refusal search command;
- `refuse` in `parser/mod.rs`;
- `sources.rs`: `ENTRIES` and `deep_limit`;
- `taskfiles/test262.yml` and `host-requirements.yml`;
- the clippy task;
- the list files against `meta.tsv` and `scan.tsv`;
- the sources of the 3 negative parse cases of the `yield*` list and 5 cases of the constructor list;
- section 12 of `docs/design/engine.md`;
- the acceptance of `story:slice-05-conformance-close`;
- the slice lines of `expected.txt`;
- one existing `runs.tsv` (another worktree), for the detail format.

Could not establish:
- No run of any join exists. That the first story passes the 3 `yield*` cases and the second story passes the 2 constructor cases comes from reading their sources, not from a run; F and F1 fix the bound either way.
- The held-refusal record: whether the running unit builds a helper is not visible yet. By the texts no story ever removes the held-refusal place: the methods story and the close write neither `lib.rs` nor `parser/mod.rs`, and the first story keeps the place on purpose ("What slice 10 reuses"). So it stays longer than one story. That is fine for the acceptance only if no item loses its last use, as the last finding asks.
- Out of my lane, the order row (parallel-safety): `yield-star-delegation` writes the YieldExpression function in `expression.rs` and `admitted` in `sources.rs`, and the methods story writes `accessor_kind`..line 1520 and `NESTING_KINDS`/`nested`. Those hunks are apart, but both stories also write `crates/swell/tests/generators.rs` (one test out each) and the generated `main.rs`, so the row is needed anyway.
- Out of my lane, the close (scope): its wording follows section 12, rules 1 to 3, and the form of slice 5's close. But check (4), second half, takes in every passing case of a later slice, and on `4971b9b` about 1,288 such cases already pass (slice 19: 581, 4: 261, 18: 187, 9: 149 and others). The story estimates only the 1,054 that end on the generator refusal; slice 5's close took 18.
- Out of my lane, slice 5 overlaps: stories 1 and 3 share `crates/syntax/src/lib.rs` with `story:identifiers-and-group-names-outside-ascii` (they write lines 146–149, 196–245 and 262–280; it writes lines 18–21).

```findings
- file: .engineering/planning/story/generator-function-constructor.md
  line: 68
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "check (b) asks for B + 21 − R and B1 + 18 − R1, but 2 of the 21 cases need only what story:generator-functions-and-suspended-frames builds and already pass on this story's base (GeneratorFunction/prototype/Symbol.toStringTag.js and Object/prototype/toString/symbol-tag-generators-builtin.js), so correct work misses the slice bound by 2 and the directory bound by 1, and Context's claim that every case ends on a TypeError or a wrong value is wrong for these two; replace 21 and 18 with F and F1, the cases of the list with a failing run in .out/test262-base/runs.tsv"
- file: crates/syntax/tests/sources.rs
  line: 720
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance runs swell-syntax only at the gate's count, though the story changes parse_dynamic_function; the stack-bound check goes through that entry, and its larger fixed part only shows past the bound at the full count; add the run with SWELL_FULL_COUNTS=1 and the 2.5 CPU-second bound at the full count"
- file: .engineering/planning/story/yield-star-delegation.md
  line: 90
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "check (c) asks for B + 49 − R and B1 + 41 − R1, but the story's own Context says the first story may already pass 3 negative parse cases of the list (statements/generators/yield-star-after-newline.js, expressions/generators/yield-star-after-newline.js, expressions/yield/star-in-iteration-stmt.js); once they are in B, correct work misses the bounds by 3 and 1; use F and F1, the list's cases with a failing run on the base, instead of 49 and 41"
- file: .engineering/planning/story/yield-star-delegation.md
  line: 90
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "check (b) gives no value on its base, and on 4971b9b, the base the other checks quote, it prints 0 as well because the text yield* is not supported does not exist there yet, so the check reads the same before and after; state what it prints over .out/test262-base/runs.tsv"
- file: .engineering/planning/story/generator-methods-in-object-literals.md
  line: 79
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "check (b) gives no value on its base, and on 4971b9b it prints 0 as well because the text generator method is not supported first exists after the second story, so the check reads the same before and after; state what it prints over .out/test262-base/runs.tsv (at least 525 runs on the list)"
- file: .engineering/planning/story/slice-07-conformance-close.md
  line: 62
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the second half of check (4), that no case of a slice delivered after 7 that passes in every run lacks a line unless a feature or flag line of a later slice holds it, has no command, so only the unit's report shows it; add a command that prints each such case for comparison with the report's list"
- file: .engineering/planning/story/slice-07-conformance-close.md
  line: 62
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "check (1) prints 0 and does not say what it prints on the base (2,825 failing runs of slice 7 on 4971b9b)"
- file: .engineering/planning/story/generator-syntax-and-early-errors.md
  line: 81
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story mentions neither the Symbol sweep nor HOST-187, which every story must carry; add: No new function of the realm, so neither the sweep with a Symbol (crates/swell/tests/builtins_symbol_values.rs) nor HOST-187 (host_objects::host_187_...) is touched."
- file: Taskfile.yml
  line: 509
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story does not say how the held refusal is recorded; a new function, struct or enum that only the record uses loses its last use when story:yield-star-delegation removes the last call site, and that story's clippy step (-D warnings) would fail while its Writes line holds neither parser/mod.rs nor lib.rs; add to the parser/mod.rs bullet that the record is made at its call site as a value of an existing type (an Option<SyntaxError> built by SyntaxError::refused), with no new function, struct or enum that only the record uses"
```

---
format: aep.planning-md/3
id: review-result:slice-07-round-2-scope
kind: review-result
status: active
title: 'Slice 7, round 2, the scope critic: the six stories'
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

epic:slice-07-generators — No item claims "The slice's cases also pass in the test mode that collects at every safe point (decisions 4 and 13), so that a frame saved in a generator object is shown to be traced", and no item says this promise was dropped or replaced. The test262 runner has no collect mode: `crates/test262/Cargo.toml` depends on `swell` without that feature, and `taskfiles/test262.yml` has no collect step. The stories run only their own tests in both modes, and `story:slice-07-conformance-close` never mentions the mode. The close should take this promise. It can either run the slice's runs once with `swell/collect-every-safe-point`, or state that the tests and language scripts of the earlier stories replace it and list that sentence for the coordinator to rewrite, as `story:slice-05-conformance-close` does with replaced epic sentences. — .engineering/planning/epic/slice-07-generators.md:84 (`grep -c -i collect .engineering/planning/story/slice-07-conformance-close.md` prints 0)

Story 2, my finding from round 1:
- closed: the boxed frame and `host_186`. Line 132 says "`SuspendedFrame`, the `Frame` … in a `Box`". Line 134 adds `object::generator::tests::a_generator_keeps_the_object_record_at_its_size` and says the unit stops if the record grows. Line 154 runs `host_objects::host_186_a_host_function_charges_every_byte$` and `git diff --quiet "$BASE" -- crates/swell/src/host/limits.rs conformance/host/surface.txt`. The round-1 blocker (`expected.rs` and `sections.rs`) belongs to another role; both files are now on the `Writes:` line.

**What I read.** The epic first: I listed 21 promises before reading any story, and 20 of them trace. Covered by stories: `yield`, `yield*`, declarations, expressions, methods, `%GeneratorFunction%`, `%GeneratorPrototype%`, tracing, the step budget, the memory budget, open question 1, the 4 constraints, and acceptance 1, 2 and 4. Acceptance 3 is partly with the coordinator: the close writes `expected.txt` and names `AGENTS.md` and `README.md` as the coordinator's (close line 55), which is an honest omission. Open question 2 is answered by `docs/design/engine.md:4225`.
- The six stories, each read in full, plus `review-result:slice-07-round-1-scope` and the graph. No other item decomposes the epic.
- The five slice-5 `Writes:` lines and `docs/design/engine.md` section 12.
- The code at `4971b9b`: the matches over `Expr`, `Instruction`, `Flow`, `ObjectKind`, `BuiltinId` and `Intrinsic`; the struct literals of `Function` and `FunctionTemplate`; the readers that use `read_dir`; the 20 recorded runner lines; the fixtures and tests that use `function*` or `yield`.
- The list files: 115 + 978 + 49 + 280 + 21 + 40 + 2 = 1,485 cases, with no overlap (`uniq -d` prints 0). That is every failing case of slice 7.
- Nothing else is wrong in coverage: no item reaches beyond the epic, no outcome is claimed twice, and nothing is narrowed silently. The 2 `cross` cases belong to close check (6).

**Other questions of the brief:**
- **`Writes:` lines against the code.** None misses a file. One small error that changes no file: in story 2, `has_call` and `has_construct` (`object/mod.rs` around lines 626 and 636 at `4971b9b`) end in `_ => false`, so they are not matches "without a wildcard".
- **Story 4 after `yield-star-delegation`.** The hunks are apart:
  - `parser/expression.rs`: the YieldExpression function against `accessor_kind` and lines 1340 to 1520.
  - `tests/sources.rs`: `admitted` (line 603) against `NESTING_KINDS` (527) and `nested` (455).

  But both stories also write `crates/swell/tests/generators.rs` (each removes one test) and the generated `crates/swell/tests/main.rs`. So the order is needed under the repository's rule that no two units of a wave write the same file.
- **Files shared with slice 5:**
  - Stories 1 and 3a share `crates/syntax/src/lib.rs` with `identifiers-and-group-names-outside-ascii`. That story writes lines 18 to 21, story 1 writes lines 146 to 149 and 262 to 280, story 3a writes lines 196 to 245 and line 212.
  - The same three add or remove differently named fixtures under `crates/syntax/tests/sources/`.
  - `slice-07-conformance-close` and `slice-05-conformance-close` both write `docs/design/engine.md` section 12, `moves.tsv`, `later-parts.tsv` and `expected.txt`. No edge orders the slice-7 close after the slice-5 close, yet its text builds on "its block as the close of slice 5 leaves them". That is the parallel-safety critic's to judge, so it does not set my verdict.
  - The other three slice-5 stories share no file with slice 7.
- **Dead code after `yield*`.** By the text, no story removes the place of the held refusal:
  - `yield-star-delegation` line 71 keeps it.
  - Story 4 lists `lib.rs` and `parser/mod.rs` as files it does not write.
  - The close writes no source.

  Keeping it is within the epic, because story 1 line 58 names it as something slice 10 reuses.

  A risk outside my lane: clippy runs with `-D warnings` (`Taskfile.yml:509`). If story 1 records the held refusal through a helper function or a type of its own, nothing will construct it after `yield*` lands. Clippy would then warn about dead code, and the `yield*` unit could not finish without writing `lib.rs` or `parser/mod.rs`. Text to add to story 1's `parser/mod.rs` bullet, since its unit is already running: "the held refusal is an `Option` field set at the site of the `*` from types other code also uses, with no function or type of its own, so that it compiles without a warning once no site sets it".
- **The close against section 12.** Its wording matches rules 1 to 4, "The order of delivery", and check (4) of `story:slice-05-conformance-close`. No finding.

**What I could not establish, or left to other lanes:**
- The acceptance checks (b) of `yield-star-delegation` and `generator-methods-in-object-literals` expect `0` but give no value on the base. Round 1's point 3 asks for that; this is the acceptance critic's.
- `yield-star-delegation` does not say outright that `STACK_BYTES_PER_REENTRY` stays unchanged, though `host/limits.rs` is off its line. This is the design critic's.
- My scratch directory `.out/critic-scope/` is removed, and the tree is clean.

```findings
- file: .engineering/planning/epic/slice-07-generators.md
  line: 84
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "No item claims \"The slice's cases also pass in the test mode that collects at every safe point (decisions 4 and 13), so that a frame saved in a generator object is shown to be traced\", and no item says this promise was dropped or replaced. The test262 runner has no collect mode: `crates/test262/Cargo.toml` depends on `swell` without that feature, and `taskfiles/test262.yml` has no collect step. The stories run only their own tests in both modes, and `story:slice-07-conformance-close` never mentions the mode. The close should take this promise. It can either run the slice's runs once with `swell/collect-every-safe-point`, or state that the tests and language scripts of the earlier stories replace it and list that sentence for the coordinator to rewrite, as `story:slice-05-conformance-close` does with replaced epic sentences."
```

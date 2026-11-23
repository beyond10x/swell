---
format: aep.planning-md/3
id: review-result:slice-07-round-1-parallel-safety
kind: review-result
status: active
title: 'Slice 7, round 1, the parallel-safety critic: the first story'
relations:
- reviews: story:generator-functions-and-suspended-frames
revision: 1
---
needs-revision
story:generator-functions-and-suspended-frames — the `Writes:` line leaves out `crates/test262/tests/expected.rs` (lines 108-109 and 153-154 hold `test262 total 22/23 5 1 16 3 13 0` and `23 5 1 17 4 13 0`) and `crates/test262/tests/sections.rs` (line 127 `test262-section sec-fixture-language incomplete 4 2 0`, line 4 "only two of `sec-fixture-language` pass"), and its hunk for `runner.rs` and the two fixture files names only the yield lines and the totals, although the slice 07 line (`runner.rs:109`, fixtures line 18) and the section line (fixtures line 27) move as well; once `function* g() { yield 1; }` parses, both yield cases pass, because the fixture harness files hold only a comment, so `cargo nextest run -p swell-test262 --test main` cannot exit 0 within the line — .engineering/planning/story/generator-functions-and-suspended-frames.md:151, crates/test262/tests/expected.rs:108, crates/test262/tests/sections.rs:127, crates/test262/tests/fixtures/tree/harness/assert.js:1
story:generator-functions-and-suspended-frames — the line limits `builtins_symbol_values.rs` to "rows only where the sweep needs one", but the sweep starts from the global object alone, and no global property reaches `%GeneratorFunction.prototype%` or `%GeneratorPrototype%`; so running it "over the three new functions" means adding a root in `functions_of_the_realm`, and the hunk has to say so — .engineering/planning/story/generator-functions-and-suspended-frames.md:141, crates/swell/tests/builtins_symbol_values.rs:350
story:generator-functions-and-suspended-frames — the body never says whether `[[GeneratorContext]]` (`Option<SuspendedFrame>`, a whole `Frame`) is held in a box; held inline in `ObjectKind`, it makes every object slot bigger, then `HOST-187` (56,291 bytes left under 506,112) and `BINDINGS_REALM_HEAP_BYTES` in `crates/swell/src/host/limits.rs` and `conformance/host/surface.txt` move, and neither file is on the line (inferred: no build; the estimate is about 230 bytes for `Frame` by the size of its fields, against 204 bytes per slot today) — .engineering/planning/story/generator-functions-and-suspended-frames.md:135, crates/swell/tests/builtins_json_parse.rs:376
story:generator-functions-and-suspended-frames — the story adds `generator_expression` and `yield` to the nesting kinds that `every_nesting_kind_fits_the_stack_bound` holds to `STACK_BYTES_PER_LEVEL` (224, `crates/syntax/src/lib.rs:80`) and lists `lib.rs` as not written, yet it does not say that a level over that bound gets its path split rather than a new constant; a change of the constant lands in `lib.rs`, which `story:identifiers-and-group-names-outside-ascii` writes (line 20) with no edge between the two (inferred: no measurement) — .engineering/planning/story/generator-functions-and-suspended-frames.md:151, crates/syntax/tests/sources.rs:788

What I read: 7 artifacts (the draft and the 6 stories of `epic:slice-05-regexp`) with `aep plan artifact show` each, the story file, `AGENTS.md` ("How the work is run"), `docs/plan/slice-05-wave-{1,2}.md`, and `git grep`/`sed` over `crates/{syntax,compiler,swell,test262}` for every enum, match, list and recorded line the story changes. I placed 7 surfaces from citations, 0 by inference, 0 not at all. I traced 2 acceptance reads to a producer in the set: `site-data-check` and the `task test262` comparison both read `conformance/test262/expected.txt`, which `story:slice-05-conformance-close` writes. No `depends_on` edge records either, and neither is a defect: each check reads the unit's own tree, and the coordinator rewrites both files after each join.

Checked and clean: the exhaustive matches the line names (`Expr` at `ast.rs` 1022/1260, `early.rs:980`, `resolve.rs:1280`, compiler `expression.rs:650`; `Instruction::target`, `execute`, `disasm`; `ObjectKind`/`KindTag` in `object/mod.rs` and `collect.rs`; `BuiltinId`; `Intrinsic`, with the struct built only in `intrinsics.rs`); `Function` and `FunctionTemplate` are each built in one place, both on the line; `Flow` is matched only in `run`; the language scripts and compiler fixtures are read by `read_dir`. Against slice 5, no source file is shared: `regexp-match-all-and-escape` is already in the base, and the others write `crates/{unicode,unicode-gen,regexp}`, `crates/syntax/src/{lexer,lib}.rs`, the regexp tests and other fixture files.

Could not establish:
- The real sizes of `Frame` and `ObjectRecord` (no build was allowed).
- Whether the section judgement becomes `complete 4 4 0` or only changes its counts.
- Whether the design author's edit of decision 3 in `docs/design/engine.md` collides with `story:slice-05-conformance-close`'s edit of section 12 of the same file (that author is outside the set).
- Out of my lane (design critic): whether the story should be split.

```findings
- file: .engineering/planning/story/generator-functions-and-suspended-frames.md
  line: 151
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the `Writes:` line leaves out `crates/test262/tests/expected.rs` (lines 108-109 and 153-154 hold `test262 total 22/23 5 1 16 3 13 0` and `23 5 1 17 4 13 0`) and `crates/test262/tests/sections.rs` (line 127 `test262-section sec-fixture-language incomplete 4 2 0`, line 4 \"only two of `sec-fixture-language` pass\"), and its hunk for `runner.rs` and the two fixture files names only the yield lines and the totals, although the slice 07 line (`runner.rs:109`, fixtures line 18) and the section line (fixtures line 27) move as well; once `function* g() { yield 1; }` parses, both yield cases pass, because the fixture harness files hold only a comment, so `cargo nextest run -p swell-test262 --test main` cannot exit 0 within the line"
- file: .engineering/planning/story/generator-functions-and-suspended-frames.md
  line: 141
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the line limits `builtins_symbol_values.rs` to \"rows only where the sweep needs one\", but the sweep starts from the global object alone, and no global property reaches `%GeneratorFunction.prototype%` or `%GeneratorPrototype%`; so running it \"over the three new functions\" means adding a root in `functions_of_the_realm`, and the hunk has to say so"
- file: .engineering/planning/story/generator-functions-and-suspended-frames.md
  line: 135
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body never says whether `[[GeneratorContext]]` (`Option<SuspendedFrame>`, a whole `Frame`) is held in a box; held inline in `ObjectKind`, it makes every object slot bigger, then `HOST-187` (56,291 bytes left under 506,112) and `BINDINGS_REALM_HEAP_BYTES` in `crates/swell/src/host/limits.rs` and `conformance/host/surface.txt` move, and neither file is on the line (inferred: no build; the estimate is about 230 bytes for `Frame` by the size of its fields, against 204 bytes per slot today)"
- file: .engineering/planning/story/generator-functions-and-suspended-frames.md
  line: 151
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story adds `generator_expression` and `yield` to the nesting kinds that `every_nesting_kind_fits_the_stack_bound` holds to `STACK_BYTES_PER_LEVEL` (224, `crates/syntax/src/lib.rs:80`) and lists `lib.rs` as not written, yet it does not say that a level over that bound gets its path split rather than a new constant; a change of the constant lands in `lib.rs`, which `story:identifiers-and-group-names-outside-ascii` writes (line 20) with no edge between the two (inferred: no measurement)"
```

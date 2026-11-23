---
format: aep.planning-md/3
id: review-result:slice-07-round-2-design
kind: review-result
status: active
title: 'Slice 7, round 2, the design critic: the six stories'
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

story:slice-07-conformance-close — its rules come from section 12 "as the close of slice 5 leaves them" and it assumes slice 5 is already delivered, both of which are produced by story:slice-05-conformance-close, but no depends_on edge runs to that story; both write the block, the table rows, the "Its output" block of section 12, `moves.tsv`, `later-parts.tsv` and `expected.txt`, and if this story runs first the 18 annexB grammar cases that slice 5's close takes into slice 5 already pass, so this close would take them into slice 7 — .engineering/planning/story/slice-07-conformance-close.md:36, .engineering/planning/story/slice-07-conformance-close.md:32, .engineering/planning/story/slice-05-conformance-close.md:35, .engineering/planning/story/slice-05-conformance-close.md:58, `aep plan artifact graph`

story:generator-syntax-and-early-errors — the body does not say how the parser records the held refusal; a recording function in `parser/mod.rs` next to `refuse`/`refuse_at` loses its last caller in story:yield-star-delegation, whose line does not hold `parser/mod.rs`, so that story fails `slot task clippy` (`-D warnings`); at line 71, after "or none.", add: "The parser keeps it in a field of its state, set where the construct is read (the first one kept), with no function of its own in `parser/mod.rs`: `story:yield-star-delegation` removes the last place that sets it and does not write `parser/mod.rs`." — .engineering/planning/story/generator-syntax-and-early-errors.md:71, crates/syntax/src/parser/mod.rs:320, Taskfile.yml:509, .engineering/planning/story/yield-star-delegation.md:86

story:generator-syntax-and-early-errors — it names story:generator-functions-and-suspended-frames as the story that takes the `admitted` arm out, but that story leaves `tests/sources.rs` alone and story:yield-star-delegation removes the arm; at line 79 replace "(`story:generator-functions-and-suspended-frames` takes the arm out again)" with "(`story:yield-star-delegation` takes the arm out, with the last refusal of slice 7 held back to the end of the parse)" — .engineering/planning/story/generator-syntax-and-early-errors.md:79, .engineering/planning/story/generator-functions-and-suspended-frames.md:128, .engineering/planning/story/yield-star-delegation.md:71

story:generator-methods-in-object-literals — after story:yield-star-delegation nothing sets the held refusal, and no story in the set removes it; this story is the first one after both wave-3 stories and runs alone in its wave, so its `Writes:` line should add `crates/syntax/src/parser/mod.rs` (the field of `Parser` and of `Parsed`) and `crates/syntax/src/lib.rs` (the branch of `analyse_parsed`, lines 262 to 280), and story:yield-star-delegation should name it as the story that removes them — .engineering/planning/story/generator-methods-in-object-literals.md:75, .engineering/planning/story/yield-star-delegation.md:71

Story 2, round-1 findings of the design critic:
- **Finding 1 (cut out the parser part): closed.** story:generator-syntax-and-early-errors exists and carries the edge. The line that closes it is generator-functions-and-suspended-frames.md:102: "it depends on `story:generator-syntax-and-early-errors`, which gives it the tree … the early errors and the place of the held refusal".
- **Finding 2 (frame stored inline in `ObjectKind`): closed.** The lines that close it:
  - generator-functions-and-suspended-frames.md:132: "`SuspendedFrame`, the `Frame` … in a `Box`" with the `Grown`-form guard.
  - :134: `a_generator_keeps_the_object_record_at_its_size`, and the unit stops if the record grows.
  - :154: `host_186_a_host_function_charges_every_byte` is run, and `git diff --quiet "$BASE" -- crates/swell/src/host/limits.rs conformance/host/surface.txt`.

**What I read**
- **Artifacts:** 6 stories, the round-1 review result `review-result:slice-07-round-1-design` and the `Writes:` lines of the 5 open slice-5 stories, with `aep plan artifact show`, `relations`, `graph` and `validate` (validate says valid).
- **Code checked:** `crates/syntax/src/{lib.rs,parser/mod.rs,parser/expression.rs}`, `crates/syntax/tests/sources.rs`, `crates/swell/src/{object/mod.rs,object/ecmascript_function.rs,interp/mod.rs,interp/dispatch.rs}`, design decision 3 "Suspended frames" and section 12, `Taskfile.yml` clippy, and the runner's fixture tree.
- **Edges:** I walked 29, outside the set too (the epic edges and the 13 edges of the open slice-5 stories).
  - There is no cycle.
  - The chain 1 → 2 → {3a, 3b} → 4 → 5 has a reason written beside every edge: code read, or a shared file.
- **Acceptance reads:** I traced 15.
  - 14 have a producer in the set (the test modules and refusal texts from story 2, the `admitted` arm and `NESTING_KINDS` from story 1, `close-cross` from 3b and 4); every one is recorded by an edge, directly or through another story.
  - 1 has a producer outside the set (slice-05-conformance-close) and no edge: finding 1.
- **Answers to the brief:**
  - **Row 4 after 3b:**
    - The hunks are apart in `parser/expression.rs`: 3b changes the YieldExpression at the `assignment` level (line 219), story 4 changes lines 1340 to 1520.
    - They are apart in `tests/sources.rs`: `admitted` at 603, against `nested` at 455 and `NESTING_KINDS` at 527.
    - They are not apart in `crates/swell/tests/generators.rs` (story 2 writes the two refusal tests side by side) or in the generated `tests/main.rs`.
    - So the row is needed under `AGENTS.md` "How the work is run" (lines 495 to 496). The other option is story 2 putting the two refusal tests in separate modules. The edge says why, so it is not a finding.
  - **Files shared with the open slice-5 stories:**
    - `crates/syntax/src/lib.rs`: identifiers-and-group-names-outside-ascii writes the module documentation, lines 18 to 21. Story 1 writes 146 to 149 and 262 to 280, story 3a writes 196 to 245 and 212, and story 4 would write 262 to 280 after finding 4. These hunks are apart.
    - `docs/design/engine.md` section 12, `moves.tsv`, `later-parts.tsv` and `expected.txt`: slice-05-conformance-close and slice-07-conformance-close write the same hunks (finding 1).
    - Under `crates/syntax/tests/sources/` they share directories only, never a file name. There is no other shared file.
  - **Dead code:**
    - A field of `Parsed` that `analyse_parsed` reads and that is only set to none raises no lint, so it may stay across wave 3, as long as no recording function is left without a caller (finding 2).
    - No story removes it by its text today (finding 4).
  - **The close's wording:** it matches section 12 "A case that comes into a slice from one delivered later", rules 1 to 3 (engine.md lines 1834 to 1856), and "The order of delivery". That includes the feature/flag exception and "a delivered slice takes no case". It also matches how slice-05-conformance-close treats its 2 generator cases in annexB.

**What I could not establish**
- Whether story 1's running unit has already written a recording helper in `parser/mod.rs`. Its worktree was not read.
- **Out of my lane (parallel safety):** 3a and 3b both run `generators::` while 3b removes one test from it. That is harmless but not stated.
- **Out of my lane (acceptance):** the close's check (4), "no passing case of a later slice lacks a line", gives no command.

No scratch files were made.

```findings
- file: .engineering/planning/story/slice-07-conformance-close.md
  line: 36
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its rules come from section 12 'as the close of slice 5 leaves them' and it assumes slice 5 is already delivered, both of which are produced by story:slice-05-conformance-close, but no depends_on edge runs to that story; both write the block, the table rows, the 'Its output' block of section 12, moves.tsv, later-parts.tsv and expected.txt, and if this story runs first the 18 annexB grammar cases that slice 5's close takes into slice 5 already pass, so this close would take them into slice 7"
- file: .engineering/planning/story/generator-syntax-and-early-errors.md
  line: 71
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body does not say how the parser records the held refusal; a recording function in parser/mod.rs next to refuse/refuse_at loses its last caller in story:yield-star-delegation, whose line does not hold parser/mod.rs, so that story fails slot task clippy (-D warnings); at line 71, after 'or none.', add: 'The parser keeps it in a field of its state, set where the construct is read (the first one kept), with no function of its own in parser/mod.rs: story:yield-star-delegation removes the last place that sets it and does not write parser/mod.rs.'"
- file: .engineering/planning/story/generator-syntax-and-early-errors.md
  line: 79
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it names story:generator-functions-and-suspended-frames as the story that takes the admitted arm out, but that story leaves tests/sources.rs alone and story:yield-star-delegation removes the arm; at line 79 replace '(story:generator-functions-and-suspended-frames takes the arm out again)' with '(story:yield-star-delegation takes the arm out, with the last refusal of slice 7 held back to the end of the parse)'"
- file: .engineering/planning/story/generator-methods-in-object-literals.md
  line: 75
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "after story:yield-star-delegation nothing sets the held refusal, and no story in the set removes it; this story is the first one after both wave-3 stories and runs alone in its wave, so its Writes line should add crates/syntax/src/parser/mod.rs (the field of Parser and of Parsed) and crates/syntax/src/lib.rs (the branch of analyse_parsed, lines 262 to 280), and story:yield-star-delegation should name it as the story that removes them"
```

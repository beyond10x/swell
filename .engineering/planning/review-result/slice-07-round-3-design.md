---
format: aep.planning-md/3
id: review-result:slice-07-round-3-design
kind: review-result
status: active
title: 'Slice 7, third reading, the design critic: the close, the held refusal and the caller of yield*'
relations:
- reviews: story:slice-07-conformance-close
- reviews: story:generator-syntax-and-early-errors
- reviews: story:generator-functions-and-suspended-frames
- reviews: story:yield-star-delegation
revision: 1
---
needs-revision

1. **Blocker on `story:slice-07-conformance-close`: closed.** The edge `depends_on story:slice-05-conformance-close` is at `.engineering/planning/story/slice-07-conformance-close.md:13`. The `Writes:` line at `:59` names all four shared files with their hunks (section 12's rule lines, table rows and the block after "Its output on this document:", plus `moves.tsv`, `later-parts.tsv` and `expected.txt`). It also names the 18 Annex B cases this close would wrongly take into slice 7 if it ran first.
2. **Findings 2 and 3 on `story:generator-syntax-and-early-errors`: both closed.**
   - Finding 2 (form of the held refusal): `generator-syntax-and-early-errors.md:71`, "Its form: a value of an existing type, an `Option<SyntaxError>` built by `SyntaxError::refused` … set by a field assignment … with no function, struct or enum that only the record uses". `yield-star-delegation.md:71` repeats it.
   - Finding 3 (who removes the `admitted` arm): `generator-syntax-and-early-errors.md:79` now says "`story:yield-star-delegation` takes the arm out, with the last refusal of slice 7 held back to the end of the parse".
3. **Kept held refusal: nothing fails `clippy -D warnings`.**
   - After `yield*` removes the last site, the field is set only to none when it is created, then read into `Parsed` and in `analyse_parsed`. Neither rustc nor clippy warns about a field that is read and only ever set to none.
   - The crate adds only `unwrap_used`, `expect_used`, `panic` and `indexing_slicing` (`crates/syntax/Cargo.toml:16`), and no item is left without a caller.
   - The branch of `analyse_parsed` can no longer be taken and no test reaches it once 3b removes `refused/yield-star.js` and the `admitted` arm. That follows from your decision to keep it. Nothing fails, so this is no finding.
4. **The delegation step does need files off its line.** Making GeneratorResume and GeneratorResumeAbrupt `pub(crate)` covers the resume side only if they stop before running the frame in a loop of their own. Story 2 does not say so. If they include the run, calling them from `yield*` adds one re-entry per level.
   - The yield side is not covered at all. "The yield of the inner result" must set the delegating generator to suspended-yield and suspend with the inner result as it is. That needs a `pub(crate)` function of `interp/generator.rs`, or the state field of `object/generator.rs`.
   - Neither story names such a function, and 3b's line holds neither file.

story:generator-functions-and-suspended-frames — the body does not say that GeneratorResume and GeneratorResumeAbrupt stop before the run, and it gives no `pub(crate)` GeneratorYield that suspends with a given value. So `story:yield-star-delegation` either resumes with one re-entry per level, against item 6, or writes `interp/generator.rs` and `builtins/generator.rs` off its line; at line 135, after "which `story:generator-function-constructor` writes in the same wave.", add: "GeneratorResume and GeneratorResumeAbrupt stop before the run: for a suspended generator each answers the generator and the completion to resume it with, and `next`, `return` and `throw` then call the two functions of `suspend.rs`. GeneratorYield (the state suspended-yield and the suspension with a value given as it is) is a `pub(crate)` function of `interp/generator.rs`, which the yield instruction calls after it makes its result object. So the caller of `yield*` takes the branches and the yield without a loop of its own and without writing either file." — .engineering/planning/story/generator-functions-and-suspended-frames.md:135, .engineering/planning/story/yield-star-delegation.md:72, .engineering/planning/story/yield-star-delegation.md:73, .engineering/planning/story/yield-star-delegation.md:86, docs/design/engine.md:951

**What I read:** the diff `a732ac4..82790ef` of the six stories, `aep plan artifact show story:slice-07-conformance-close`, `crates/syntax/Cargo.toml`, and the visibility of `get_method`, `iterator_close`, `iterator_complete`, `Behaviour`, `BuiltinId`, `Entry::Iterator` and the module lines of `builtins/mod.rs`. Only `iterator_complete` is private, and it is on 3b's line.

**What I could not establish:** whether story 2's running unit (now `active`) has already shaped GeneratorResume or the yield function. Its worktree was not read. I made no scratch files.

```findings
- file: .engineering/planning/story/generator-functions-and-suspended-frames.md
  line: 135
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the body does not say that GeneratorResume and GeneratorResumeAbrupt stop before the run, and it gives no pub(crate) GeneratorYield that suspends with a given value. So story:yield-star-delegation either resumes with one re-entry per level, against item 6, or writes interp/generator.rs and builtins/generator.rs off its line; at line 135, after 'which story:generator-function-constructor writes in the same wave.', add: 'GeneratorResume and GeneratorResumeAbrupt stop before the run: for a suspended generator each answers the generator and the completion to resume it with, and next, return and throw then call the two functions of suspend.rs. GeneratorYield (the state suspended-yield and the suspension with a value given as it is) is a pub(crate) function of interp/generator.rs, which the yield instruction calls after it makes its result object. So the caller of yield* takes the branches and the yield without a loop of its own and without writing either file.'"
```

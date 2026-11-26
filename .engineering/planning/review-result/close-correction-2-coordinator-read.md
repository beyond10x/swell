---
format: aep.planning-md/3
id: review-result:close-correction-2-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 2 of the unit close: the consumer test and two sentences, one comment corrected'
relations:
- reviews: story:slice-04-conformance-close
revision: 1
---
Coordinator's read of correction 2 of the unit close of wave big-1
(`impl/slice-04-close`, `76195b35..a1ca60ed`, two commits, two files:
`crates/swell/tests/consumer_slice_04.rs` and `docs/design/engine.md`), 2026-11-26. The
adversary's budget of two passes is used; pass 2 left 5 notes and 2 red cases, none on the
close's lists or numbers, so this read stands in for a third pass.

## What was read

`git diff --stat 76195b35 a1ca60ed`: two files, no line of `crates/swell/src`, no file under
`conformance/` or `docs/plan/`. The test file whole (260 lines); the two changed places of
section 12 of the design.

## The five notes of pass 2

- F1 (the nine references cannot tell a function of `Math` from its rounding neighbour or
  see the sign of a zero) and F5 (two references called functions the consumer does not
  call, so the story's line on the argument values did not hold): the consumer's script is
  again the consumer's calls with the consumer's argument values (`Math.max(2 - 5, 0)`,
  `Math.ceil(300 / 100)`, `Math.max(3, 4)`), as at `e2ae99d5` and as
  `builtins_math::consumer_calls_of_the_harness_answer_as_the_standard_says` has them. The
  story's line holds again without a change of the story. A second script of 22 calls in the
  same file covers the five parts of slice 4 the consumer calls (`Date.now`, `Math.floor`,
  `Math.min`, `Math.abs`, `Math.ceil`), and five tests run it: against the texts of the
  standard; with each part deleted; with each of the four functions of `Math` as the
  identity; with `floor` and `ceil` each answering as each of the other three rounding
  functions; with each of the four answering a zero of the other sign. The unit planted four
  mutants in `builtins/math.rs`, one at a time, and took each back: `min` answering the
  highest and `ceil` calling `floor` turn tests of this file red; `round` calling `trunc`
  and `trunc` answering +0 turn `builtins_math` and three test262 cases red, which is where
  the functions the consumer does not call are held.
- The adversary's two red cases were not kept unchanged: three of their five swaps and the
  `trunc` sign case are on functions the consumer does not call. The swaps on `floor` and
  `ceil` are kept and widened to every neighbour.
- F2 (the close text said every message names a part of slice 4): the text now says that
  the message of 31 of the 72 names the part and the message of 41 names none, with the four
  messages of the 41 and their counts (30, 9, 1, 1), and that for those the part is read
  from the case's source.
- F3: 92, not 93, of the 98 cases of slice 9 are the `not-a-constructor` cases.
- F4: the changelog entry in the unit's text for the coordinator names the five parts and
  the six tests as delivered.

## What the read found

- One sentence of the module's doc comment at `11609c8a` was not true: it said three of the
  nine references cannot fail when slice 4 is taken out, among them `Math.ceil(300 / 100)`.
  The start commit `a46fe885` defines only `max` on `Math`, so that call throws without the
  slice. The unit corrected the comment at `a1ca60ed` (8 lines in, 7 out, no code): the two
  calls of `Math.max` cannot fail; the call of `ceil` fails when `ceil` is absent and cannot
  fail when `ceil` answers as the identity or as another rounding function.
- The second paragraph of the same comment says "each with arguments that tell it from the
  identity, from each rounding neighbour" of all five parts. The tests are narrower, and
  their own doc comments say so: the identity for the four functions of `Math`, the
  neighbours for `floor` and `ceil`. `Date.now` is held by its absence and by a clock that
  does not stand on a whole millisecond. No change asked.
- The story names one test for this file and a bound of 1.5 CPU seconds; the file has six.
  The unit measured each: the slowest is 0.068 s in the default mode and 1.553 s with a
  collection at every safe point. The coordinator states the delivered count in the story at
  the close.
- No `#[allow`, no `#[ignore`, no name with `adversary_`.

## Verdict

The correction closes F1 to F5. The claim is run again by the coordinator on `a1ca60ed`
before the merge and recorded as verification.

```findings
[]
```

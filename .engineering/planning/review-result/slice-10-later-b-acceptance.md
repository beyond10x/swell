---
format: aep.planning-md/3
id: review-result:slice-10-later-b-acceptance
kind: review-result
status: active
title: 'Slice 10, later stories, set B, the acceptance critic: the five stories of orders 6 and 7'
relations:
- reviews: story:yield-star-in-async-generators
- reviews: story:async-generator-methods-in-object-literals
- reviews: story:async-generator-function-constructor
- reviews: story:array-from-async
- reviews: story:slice-10-conformance-close
revision: 1
---
needs-revision
story:yield-star-in-async-generators — check (b) `... | wc -l` prints `0` with no value over the base run: the refusal text did not exist in the property-escapes base run, so it prints `0` before and after the work; the acceptance must state what it prints over the join's base run, as the accepted first-wave stories do — .engineering/planning/story/yield-star-in-async-generators.md:90
story:yield-star-in-async-generators — the test262 part holds only `test/language/expressions/yield` and `test/built-ins/GeneratorPrototype`, but the story changes the sync delegation loop and `Instruction` and omits the sync `yield*` cases of `test/language/statements/for-of/yield-star*.js`, `test/annexB/language/expressions/yield/star-*.js`, `test/language/{statements,expressions}/generators/yield-star-before-newline.js` and `test/staging/sm/generators/*`; name them as directories or lines of the part — .engineering/planning/story/yield-star-in-async-generators.md:90
story:yield-star-in-async-generators — Bounds and Tests name a test only for `frame_depth` (a chain above it); no test reaches `reentry_depth` through the loop's calls of the inner `next`, `throw` and `return`, which the time and bounds rule requires — .engineering/planning/story/yield-star-in-async-generators.md:75
story:async-generator-methods-in-object-literals — check (b) `awk ... | wc -l` prints `0` with no value over the base run; state what it prints over the join's base run so it can tell before from after — .engineering/planning/story/async-generator-methods-in-object-literals.md:83
story:async-generator-methods-in-object-literals — Change has no Bounds or run-loop line: it does not say that the story adds no instruction and no running path (so termination, depth and the charge of a suspended frame are those of `story:async-generators-and-their-request-queue`) or give a test that an `async *` method ends at each budget and at an interrupt — .engineering/planning/story/async-generator-methods-in-object-literals.md:58
story:async-generator-function-constructor — Bounds says "the parse and the compile charge before they grow and read the interrupt", but Tests names no test of it, unlike `async_function_constructor_charges_its_parse_and_reads_the_interrupt` of the sibling story; add `async_generator_function_constructor_charges_its_parse_and_reads_the_interrupt` — .engineering/planning/story/async-generator-function-constructor.md:60
story:async-generator-function-constructor — the test262 part leaves out `test/built-ins/Function` (509 cases), which the story's change to `create_dynamic_function`, `parse_dynamic_function` and `dynamic_function_text` also runs through; `story:async-function-constructor` includes it — .engineering/planning/story/async-generator-function-constructor.md:75
story:array-from-async — the list holds `built-ins/Array/fromAsync/async-iterable-input-does-not-await-input.js`, which uses `async next() {}` in an object literal, and the story has no edge or mention of `story:async-methods-in-object-literals`, so acceptance (a) cannot pass without a story it does not depend on — .engineering/planning/story/array-from-async.md:74
story:array-from-async — Bounds says a termination while the activation waits keeps its state and a later checkpoint resumes it, but the only termination test is `a_termination_in_the_steps_of_from_async_leaves_its_promise_pending`; no test covers the waiting half — .engineering/planning/story/array-from-async.md:57
story:slice-10-conformance-close — "prints the same two numbers as on the base runs" names no file or command for the base side (`.out/test262-base/` holds only `runs.tsv`); name the command that gives the base numbers — .engineering/planning/story/slice-10-conformance-close.md:67
story:slice-10-conformance-close — the paragraph "Slice 10's cases that need a later slice" (groups by target slice with counts summing to the lines added to `moves.tsv`) and the sentence under the table have no check in the acceptance; add a command that compares the paragraph's counts with `awk -F'\t' '$2 == "10"' conformance/test262/moves.tsv | wc -l` — .engineering/planning/story/slice-10-conformance-close.md:58

**What you read:** 5 stories (`story:yield-star-in-async-generators`, `story:async-generator-methods-in-object-literals`, `story:async-generator-function-constructor`, `story:array-from-async`, `story:slice-10-conformance-close`), whole, with `aep plan artifact show`. I also read `part.sh`, `check.sh`, `scan-later.awk`, the list files, `moves.tsv`, `expected.txt` and `test262.yml` (the `test262-moves-check` text). I recomputed the part sizes and runs (924/1,797, 736/1,365, 305/562, 551/1,065), the base `B` values (244, 288, 241, 454), the list runs (1,118, 478, 42, 144) and the close's 138 cases and 274 runs against `base-copy.tsv`; all match. I also checked the edges of the set A stories and the `Writes:` lines against the code.

**What I could not establish:**
- The base run of the join does not exist yet, so I could not tell what check (b) prints there.
- Whether `story:from-async` lists other cases that need `async` methods: my scan matched `async name(` once, in `async-iterable-input-does-not-await-input.js`.
- The Context line "4 on a `TypeError` of a call of undefined, 2 on a `Test262Error`" in `story:array-from-async` sums to 142, not 144. The base run has 4 `TypeError`s and 4 `Test262Error`s. This is a context figure, not an acceptance finding.
- Out of my lane, and not counted in the verdict: `story:array-from-async` and `story:promise-statics-finally-and-aggregate-error` both write `builtin_function.rs`, `object/mod.rs` or `intrinsics.rs` hunks with no edge between them (orders 4 and 6, so not simultaneous). `story:async-generator-methods-in-object-literals` and `story:array-from-async` both regenerate `tests/main.rs` in order 6.

```findings
- file: .engineering/planning/story/yield-star-in-async-generators.md
  line: 90
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "check (b) prints `0` with no value over the base run: the refusal text did not exist in the property-escapes base run, so it prints 0 before and after the work; state what it prints over the join's base run"
- file: .engineering/planning/story/yield-star-in-async-generators.md
  line: 90
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the test262 part holds only test/language/expressions/yield and test/built-ins/GeneratorPrototype, but the story changes the sync delegation loop and omits the sync yield* cases of test/language/statements/for-of/yield-star*.js, test/annexB/language/expressions/yield/star-*.js, the generators yield-star-before-newline.js cases and test/staging/sm/generators/*; name them in the part"
- file: .engineering/planning/story/yield-star-in-async-generators.md
  line: 75
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Bounds and Tests name a test only for frame_depth; no test reaches reentry_depth through the loop's calls of the inner next, throw and return"
- file: .engineering/planning/story/async-generator-methods-in-object-literals.md
  line: 83
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "check (b) prints `0` with no value over the base run; state what it prints over the join's base run so it can tell before from after"
- file: .engineering/planning/story/async-generator-methods-in-object-literals.md
  line: 58
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Change has no Bounds or run-loop line: it does not say the story adds no instruction and no running path, so termination, depth and the charge of a suspended frame are those of the async generator story, and no test shows an async * method ending at each budget and at an interrupt"
- file: .engineering/planning/story/async-generator-function-constructor.md
  line: 60
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Bounds says the parse and the compile charge before they grow and read the interrupt, but Tests names no test of it, unlike async_function_constructor_charges_its_parse_and_reads_the_interrupt of the sibling story"
- file: .engineering/planning/story/async-generator-function-constructor.md
  line: 75
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the test262 part leaves out test/built-ins/Function (509 cases), which the change to create_dynamic_function, parse_dynamic_function and dynamic_function_text also runs through; story:async-function-constructor includes it"
- file: .engineering/planning/story/array-from-async.md
  line: 74
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the list holds built-ins/Array/fromAsync/async-iterable-input-does-not-await-input.js, which uses async next() in an object literal, and the story has no edge to story:async-methods-in-object-literals, so acceptance (a) cannot pass without a story it does not depend on"
- file: .engineering/planning/story/array-from-async.md
  line: 57
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Bounds says a termination while the activation waits keeps its state and a later checkpoint resumes it, but the only termination test covers the steps; no test covers the waiting half"
- file: .engineering/planning/story/slice-10-conformance-close.md
  line: 67
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"prints the same two numbers as on the base runs\" names no file or command for the base side; name the command that gives the base numbers"
- file: .engineering/planning/story/slice-10-conformance-close.md
  line: 58
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the paragraph \"Slice 10's cases that need a later slice\" and the sentence under the table have no check in the acceptance; add a command that compares the paragraph's counts with the lines of moves.tsv whose from is 10"
```

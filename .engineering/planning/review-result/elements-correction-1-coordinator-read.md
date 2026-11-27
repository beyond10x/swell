---
format: aep.planning-md/3
id: review-result:elements-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit elements: no defect'
relations:
- reviews: story:class-element-syntax-and-early-errors
revision: 1
---
Coordinator's read of correction 1 of the unit elements (impl/elements, bd71f105 to 5364e45e),
2026-11-26. Verdict: no defect.

The correction answers review-result:adversary-elements-pass-1 (one red case of 14, five
findings). It is two commits.

Commit ae0397e9 (7 files, 1,261 lines added, 32 removed; `tests/sources.rs` 1,035 of them):

- Finding 1, as the coordinator decided: a YieldExpression and an AwaitExpression in the code
  of a field initializer stay a SyntaxError, a stated difference from the pinned grammar
  `Initializer[+In, ?Yield, ?Await]`. The parser no longer clears [Yield] and [Await] there. It
  keeps both and carries a flag `field_code` (`parser/mod.rs:139`), set by
  `element_context_open(true)` (`parser/class.rs:556`), cleared by `save_context`
  (`parser/function.rs:170`) for every function and by `element_context_open(false)` for a
  static block, and put back by `element_context_close` and `finish_function` from the list
  `outer_field_code`, whose growth is charged first and which `finish_parse` releases. Read:
  each of the two places that set the flag pushes the old value, and each of the two that end
  such code pops it (`static_block_close` calls `element_context_close`), so the flag after a
  nested static block or function is the one before it. `yield_expression`
  (`parser/expression.rs:350`) and `prefix_operators` (`:771`) refuse at the token with
  `yield expression in a class field initializer` and `await expression in a class field
  initializer`. The adversary's red case is turned around:
  `a_field_initializer_refuses_yield_and_await_expressions` holds 14 refused sources with
  message, line and column through the three parse entries and 13 more for their verdict (the
  computed key `[await 1] = 0` stays accepted). It was red on the unit's head with 42
  mismatches.
- Finding 2: a static block makes an arguments object when a direct `eval` in it, or in an
  arrow function in it, can observe one (`scope.rs:243`: `StaticBlock` is out of the
  exclusion), as `docs/design/engine.md` says. The test
  `scope::tests::a_static_block_has_an_arguments_object_only_for_a_direct_eval` (7 cases) was
  red before the change in two of them.
- Finding 4: `parser::class::tests::a_static_private_method_makes_no_instance_elements`; with
  the guard replaced by `if true` in a copy it is red in two of its three sources.
- Finding 5: the checker carries `static_block_await` (`early.rs:445`), set by a static block,
  kept through a field initializer and an arrow function that is not `async`, cleared by every
  other function; `await_identifier` reads it. `class C { static { class D { x = await; } } }`
  now answers `await may not be an identifier in a class static block`. An `async` arrow
  function inside a static block now answers the async function's message, since its own
  [Await] reserves the word. Test `await_as_an_identifier_names_the_code_that_reserves_it`,
  12 sources, 21 mismatches on the unit's head.
- The adversary's 13 green cases under the unit's names; one needed a type alias for clippy.
- Finding 3 (no test in this tree sees the five element holds, because the hold of `class`
  comes first) stays open for the join with story:class-definition-evaluation: the correction
  measured, in a copy with `hold("class")` taken out, the five sources whose first answer is
  each element's hold. The coordinator adds that test at the join.

Commit 5364e45e: the merge of `wave/big-2` at 7718153e (the unit randomcases, the changelog,
plan commits). No conflict; no count a test holds moved.

On the merged head the implementor ran: swell-syntax main 78 passed (default and full
counts), its library tests 11, swell-compiler main 17, swell main `1311 passed, 1 skipped`
in the default mode, the story's 800 cases of test262 with the guard `passed 3355 bound 3355
B 2555 F 800 R 0`, clippy and fmt clean, `test-main: 9 crate(s)`. The stack-bound part 8 at
the full counts, alone: fastest of five runs 1.567 s before and 1.611 s after.

Text for the design (the stated difference), for the story's "As delivered" and for the
changelog is in the correction's notes; the coordinator writes it at the wave's close.

---
format: aep.planning-md/3
id: review-result:newtarget-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit newtarget: tests, fixtures, one doc comment and the merge of the wave branch, no defect'
relations:
- reviews: story:new-target-meta-property
revision: 1
---
Coordinator's read of correction 1 of the unit newtarget (impl/newtarget, 90960dae to
f73a936c), 2026-11-26. Verdict: no defect; the correction adds tests, fixtures and one doc
comment, and merges the wave branch with nine resolved conflicts.

The correction answers review-result:adversary-newtarget-pass-1. It is two commits.

Commit 68c32a34 (3 files, 609 added lines, 1 removed):

- `crates/swell/tests/new_target.rs`: the adversary's 7 cases under the unit's names; the
  module runs 21 tests where it ran 14. Its measurement test (`#[ignore]`) is not taken.
- `crates/syntax/tests/sources/early-errors/new-target-after-a-function.js`: `(function () {});
  new.target;` answers `new.target outside a function`.
- `crates/swell/src/object/ecmascript_function.rs`: the doc comment at `#[inline(never)]` of
  `realm_of_function` gives what the attribute is for, with the adversary's numbers: 12,400
  bytes of stack per re-entry on the replacer path with it and 12,783 without, against
  `STACK_BYTES_PER_REENTRY` 14,336; without it
  `regexp_symbol_methods::a_replacer_re_enters_on_the_promised_stack` is 96,608 bytes short.
  No line of code changed.
- Measured by the implementor in a copy of the tree, each change planted alone: with the two
  traces of the arrow's box removed, `an_arrows_box_alone_keeps_its_new_target_and_function` is
  red (`Terminated(EngineFault)`); with the frame's NewTarget dropped on resume,
  `async_code_and_the_host_keep_new_target` is red; with the restore dropped in `function_end`,
  `new_target_parses_only_inside_a_function` and two tests of the syntax crate are red (the new
  fixture parses). Without a planted change 21 and 56 pass.

Commit f73a936c, the merge of `wave/big-2` at 66882ac0 (the units classsyntax and super), read
in the combined diff:

- `parser/mod.rs`: the fields `new_target` and `outer_new_target` stand beside
  `super_property` and `outer_super_properties`; `Parser::new` sets all four and the end of the
  parse releases both lists. `parse` keeps the path of the unit super (`Parser::new`, then
  `parse_script_with(parser, false)`). `parse_eval` sets `strict`, `new_target` from
  `context.in_function` and `super_property` to `SuperProperty::Eval`, then calls
  `parse_script_with(parser, context.strict_caller)`.
- `parser/function.rs`: `open_context` takes the function's kind, sets `new_target` and sets
  `super_property` from `super_property_of(kind)`; `save_context` pushes both flags and
  `function_end` restores both.
- `parser/expression.rs`: the module doc.
- The test `sources::a_script_is_not_parsed_as_eval_code` holds the path of `parse`: with
  `parse` routed through `parse_eval` in a copy, it and two more tests are red (`super.x;`
  answers the eval refusal where a script answers `super property outside a method`).
- Six fixtures state what a class source with `new.target` answers while `class` is held: four
  `held/class-new-target-in-*.js` (constructor, an arrow in a constructor, method with getter
  and setter, static method) answer the hold of `class` alone; `new.target` in a computed key
  and in the heritage of a class at the top of a script answer `new.target outside a function`.
  `new.target` is admitted in a constructor, a field initializer and a static block by the
  specification; fields and static blocks are refused at their first token today, so the
  fixtures for them come with the stories that parse them.

On the merged head the implementor ran: swell-syntax 61 passed (also at the full counts),
swell-compiler 17, swell main `1226 passed, 1 skipped` in both modes, the library's tests 250
and 246, the story's part of test262 with both guards `test262 total 4792 0 0 4792 4792 0 0`
and `test262-slice 08 37 74 0 0 37 37 0 0`, the stack tests in the profiles release and
stack-debug (4 passed each), clippy and fmt clean.

Finding 5 of the pass (four re-entry tests overflow the promised stack in the profile
stack-debug, five on the wave's base) is older than the wave: filed as the draft
`story:stack-tests-hold-in-the-stack-debug-profile`.

Not checked by the correction: no change was planted for the four held fixtures or the two
class early-error fixtures. Not read: the 7 cases line by line; the read took their names.

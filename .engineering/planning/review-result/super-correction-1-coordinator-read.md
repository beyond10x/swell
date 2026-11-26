---
format: aep.planning-md/3
id: review-result:super-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit super: tests, fixtures and the merge of the wave branch, no defect'
relations:
- reviews: story:super-property-and-super-syntax
revision: 1
---
Coordinator's read of correction 1 of the unit super (impl/super, 6a89ce2d to 96e070c4),
2026-11-26. Verdict: no defect; the correction adds tests and fixtures, and merges the wave
branch with three resolved conflicts and one function extended.

The correction answers review-result:adversary-super-pass-1. It is two commits.

Commit ba13cabb, tests only (5 files, 845 added lines, none under `src/`):

- `crates/swell/tests/super_property.rs`: the adversary's 31 cases under the unit's names; the
  module runs 45 tests where it ran 14. The one red case of the pass is now
  `a_method_in_an_arrow_parameter_inside_a_method_is_refused_until_derived_classes`: it states
  the refusal the engine answers today (`super in an arrow function is not supported before
  slice 8`), which is older than the unit (the wave's base refuses the same source), and its
  doc comment names `story:derived-classes-and-super-call`, which lifts the refusal, and the
  answer the source must then give.
- Three early-error fixtures and one valid fixture of the syntax crate, renamed from the
  adversary's.
- Measured by the implementor in a copy of the tree: with the key converted before the null
  check in `get_super_index` or in `super_key`, `null_base_throws_before_the_key_is_converted`
  is red (44 of 45 passed); with `delete_super` reading the base first,
  `delete_and_typeof_of_a_super_reference` is red. Without a planted change all 45 pass.

Commit 96e070c4, the merge of `wave/big-2` (at 8dad2f03, which holds the unit classsyntax),
read hunk by hunk in the combined diff:

- `crates/syntax/src/early.rs`: the duplicate-parameter rule of a method covers
  `FunctionKind::Method | FunctionKind::ClassConstructor` (the class unit's side), under the
  unit's comment on HasDirectSuper. Both sides kept.
- `crates/syntax/src/parser/expression.rs`: `Keyword::Class` goes to `class_expression_node`
  (the class unit's side) and `Keyword::Super` to `super_admitted` (the unit's side); the
  module doc names both.
- `crates/syntax/src/parser/function.rs`, `super_property_of`: the three kinds the class unit
  added, `ClassConstructor`, `FieldInitializer` and `StaticBlock`, are `SuperProperty::Method`.
  A constructor is a MethodDefinition; the early errors of a field's Initializer and of a
  ClassStaticBlockStatementList forbid a SuperCall and admit a SuperProperty. No wildcard arm.
  A SuperCall is decided apart, in `super_admitted`, and stays an early error everywhere until
  `story:derived-classes-and-super-call`.
- Eight fixtures state what a class source with `super` answers while `class` is held: seven
  `held/class-super-property-in-*.js` (method, getter, setter, static method, generator method,
  async method, constructor) answer the hold of `class` alone; with `ClassConstructor` moved to
  `Outside` in a copy the held test is red on the constructor fixture.
  `early-errors/super-call-in-derived-class-constructor.js` states today's answer to a SuperCall
  in a derived class's constructor and names the story that changes it.

Finding 2 of the pass (`super_base` answers `Termination::EngineFault` for a frame with no
function or no home object, where the story's text says "the `TypeError` of a missing base"):
the code stays. No script reaches the state (`no_script_reaches_a_frame_without_home_object`),
and the engine answers a broken invariant of its own with `EngineFault` elsewhere
(`generator_start`, `async_generator_start`). The story's "As delivered" section states it at
the wave's close.

Counts the stories' texts take at the close: `early-errors/super-*.js` 7 to 11, `held/` 5 to
12, `super_property::` 14 to 45 tests.

Not read: the 807 lines of the adversary's cases line by line; the read took their names, the
renamed red case and the helper `rounds`.

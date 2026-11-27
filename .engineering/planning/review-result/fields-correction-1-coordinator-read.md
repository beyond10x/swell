---
format: aep.planning-md/3
id: review-result:fields-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit fields: no defect'
relations:
- reviews: story:public-class-fields
revision: 1
---
Coordinator's read of correction 1 of the unit fields (impl/fields, 6a77e86 to 52307e7),
2026-11-27. Verdict: no defect.

Correction 1 (3986086, two files, 122 lines added, 6 removed, and the merge of `wave/big-2`
at 4c0c86a, without a conflict) answers review-result:adversary-fields-pass-1. Read: the whole
diff of 3986086.

- Finding 1 (a direct eval of `arguments` in a field initializer threw a ReferenceError where
  PerformEval requires a SyntaxError). The rule, PerformEval's inClassFieldInitializer with
  ContainsArguments, belongs to `story:eval-in-class-field-initializers`, the next order of the
  slice. Until that story, the engine refuses: `eval_context`
  (`crates/swell/src/dynamic_code.rs`) reads the template kind of the function the eval frame
  takes and throws the SyntaxError `eval in a class field initializer is not supported before
  slice 8` when it is `FieldInitializer`. An indirect eval passes no function and is not
  touched. The test `a_direct_eval_in_an_initializer_is_refused` holds seven refused forms
  (instance and static fields, `arguments`, an arrow naming it, `1`, an arrow in an
  initializer that calls eval) and four forms that run (an indirect eval in an instance and in
  a static initializer, a direct eval in a method and in a static method). The implementor
  showed it red with the change of `dynamic_code.rs` undone. The 515 cases of the unit's list
  and its guard give the same 2,069 results before and after, byte for byte; no run carries the
  refusal's message.
- Finding 2 (no test held a computed key made at run time alive between the class's
  definition and a construction). `a_computed_key_made_at_run_time_lives_until_the_constructions`
  is the adversary's script. With the adversary's plant A (the collector skips the hidden key
  bindings) it is red in the collect build with `Terminated(EngineFault)`; without the plant it
  is green in both modes.
- Finding 3 (error sites not held). `a_field_error_is_reported_at_the_field_and_at_its_key`
  asserts line 3, column 3 for the DefineField a non-extensible instance refuses and line 3,
  column 4 for a key whose ToPropertyKey throws. Each mutant the finding names turns it red
  (`DefineField` at the function's span: (2, 3); `ToPropertyKey` at the field's span: (3, 3)).
- Finding 4 (a literal `__proto__` field not held). `a_literal_proto_field_is_an_own_property`
  holds an instance and a static field; the mutant that compiles the field as the object
  literal's prototype set turns it red.
- The time bound. `constructions_of_a_class_with_100_fields_are_charged_and_freed` took 6.48
  CPU s alone in the default build with `SWELL_FULL_COUNTS=1`, over the 2.5 s of a test there.
  Its full count is 3,000 constructions per round in place of the story's 10,000 (2.05 CPU s);
  the number of fields per construction (100), the gate count (500) and the counts of the
  collect build are unchanged. The unit's plants (a byte charged and never credited per
  DefineField; a DefineField that defines nothing) are red at 500 already, so the lower full
  count loses no defect of the kind the test exists for.

What the read did not cover again: the unit's first change (f264bd3), which the adversary's
pass covered; the test262 run of the list on the merged head (the coordinator's verification
and the wave's gate run it).

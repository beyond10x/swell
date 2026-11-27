---
format: aep.planning-md/3
id: review-result:classes-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of corrections 1 and 2 of the unit classes: no defect'
relations:
- reviews: story:class-definition-evaluation
revision: 1
---
Coordinator's read of corrections 1 and 2 of the unit classes (impl/classes, 60103e33 to
4d2d8345), 2026-11-27. Verdict: no defect.

Correction 1 (3e013e7, 19 files, 732 lines added, 85 removed, and the merge of `wave/big-2`
at e249bfd) answers review-result:adversary-classes-pass-1: class code is strict on every
instruction, and an anonymous class under a computed key of an object literal is named before
its elements run. The adversary's second pass ran on that correction and is recorded as
review-result:adversary-classes-pass-2: green, with two notes. The coordinator did not read
correction 1's code line by line again; the second pass did (62 ways into and out of class
code and 21 naming places answer as the standard says in both modes; a planted removal of an
early error of class elements turns four tests red, so the merge's resolutions hold).

Correction 2 (4d2d8345, one file, `crates/swell/tests/classes.rs`, 324 lines added, 43
removed; no file under `crates/*/src`) answers the two notes of the second pass. Read: the
whole diff.

- The pass's four cases and their helper are committed under names that say what they hold:
  `every_way_into_and_out_of_class_code_keeps_its_strictness` (38 results),
  `every_named_evaluation_names_an_anonymous_class_as_the_standard_does` (21),
  `a_step_budget_in_a_key_leaves_no_strictness_behind` and
  `strict_semantics_hold_in_keys_of_every_kind` (24), with `every_entry_is_true`. Compared with
  the adversary's file line by line, doc comments left out: the nine lines that differ are the
  five names and their four uses; no assertion, count or source line changed.
- The two mutants the unit's 24 tests missed are held. Made in a copy by the implementor and
  run against the 29 tests in both modes, each turns one test red and no other:
  `exit_return` writing `SetStrict(false)` in strict code (`crates/compiler/src/lib.rs:893`)
  fails `every_way_into_and_out_of_class_code_keeps_its_strictness` at result 32; the resolver
  naming a class under a `__proto__` key (`resolve.rs:1285`) fails
  `every_named_evaluation_names_an_anonymous_class_as_the_standard_does` at results 2 and 3.
- `every_strict_operation_in_a_computed_key_is_strict` took 1.743 CPU s alone in the lane's
  collect build, of a bound of 2 s. It is split by kind of operation into
  `every_strict_write_in_a_computed_key_throws` (four operations) and
  `a_direct_eval_in_a_computed_key_is_strict_eval_code` (two), both built by
  `strict_operations_script` over the enum `StrictOperation`. Read: the six operations of the
  old body are the six arms of the match, text for text; the six positions and the two
  contexts are unchanged, so the two tests hold 48 and 24 results, the 72 of the one. Each
  script still ends in `r.length === n` and the answer of every entry. Times alone in the
  lane's collect build, by the implementor: 0.981 s and 0.658 s.

Checks of the implementor on 4d2d8345, on the build node: the 29 tests of `classes::` in both
modes; the whole `swell::main` in both modes, `1351 tests run: 1351 passed, 1 skipped` each;
each `classes::` test alone on one thread in the default build (highest 0.353 s, bound 1.5)
and in the lane's collect build (highest 1.356 s, bound 2),
`a_class_made_in_a_loop_is_charged_and_freed` both times; clippy and the format check.

Open at the time of this read, outside the unit: the stack test
`limits_scripts::min_stack_bytes_is_enough_through_builtins` is red in the profile
`stack-debug` on `wave/big-2` since the merge of `impl/weak`, without this unit too; the unit
`stackjoin` corrects it, and this unit merges after it.

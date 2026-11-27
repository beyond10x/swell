---
format: aep.planning-md/3
id: review-result:hook-correction-1-coordinator-read
kind: review-result
status: active
title: The coordinator's read of correction 1 of the unit hook
relations:
- reviews: story:host-ensure-can-add-private-element
revision: 1
---
Coordinator's read of correction 1 of the unit hook (impl/hook, c04824c to 716b3cf),
2026-11-27, in place of a second adversary pass: the correction changes no line of the
engine outside doc comments and test modules. Verdict: no defect.

The adversary's pass 1 was red (`review-result:adversary-hook-pass-1`): one failing case and
five findings on what the tests hold. Read: the diff of `a454207` and `167005b` (3 files, 657
lines added, 34 removed: doc comments in `crates/swell/src/host/hooks.rs` and
`crates/swell/src/object/private_names.rs`, one library test in the test module of
`private_names.rs`, and `crates/swell/tests/private_element_hook.rs`), the merge `716b3cf` of
`wave/big-2` at `38edf82`, and the unit's three reports.

- Finding 1, the failing case: a hook that allows an object and then refuses it between the
  two halves of an accessor leaves the getter added and the setter not. The coordinator
  decided it in the text and not in the compiler: `#sec-hostensurecanaddprivateelement`
  requires of the host that "Any two calls of this abstract operation with the same argument
  must return the same kind of Completion Record", so a host that answers one object two
  ways is outside the clause. The hook's documentation states the duty, that the engine does
  not check it, and what such a host gets: the elements added before the refusal, in source
  order, each half of an accessor where it stands, nothing taken back and no fault. The case
  was written again to hold that
  (`a_throw_between_the_halves_of_an_accessor_keeps_the_getter_added_before_it`: two hook
  calls, the method between the halves not added, the accessor with its getter only, a write
  that throws a `TypeError`, the agent alive, and a later construction on a fresh object with
  both halves). Its assertion changed on the coordinator's order; the adversary's wish, the
  whole accessor before the refused element, is not built, because the compiler emits each
  half where it stands in the source and the clause does not ask a host that keeps its duty
  for more. The requirement paragraphs of HOST-119 and HOST-223 get the same sentences.
- Findings 2 to 5: the adversary's eight cases and its library test are kept with their
  assertions, without the prefix in their names. Four changes of the code that the unit's
  first tests did not catch are each red now: the hook's call without the check that the
  agent still runs (`a_termination_the_hook_swallowed_ends_the_run_and_adds_nothing`), the
  scope of the hook's values not cut back (`what_a_hook_makes_is_not_kept_across_many_adds`),
  no join when the setter stands first
  (`an_accessor_pair_in_either_order_is_one_element_with_both_halves`), and a safe point
  inside a method of the hook's context (the library test
  `values_held_only_by_the_adds_survive_a_hook_that_calls_hookcx`, collect build).
- Finding 6: a `Throw` value that is not checked was caught in the default build only. The
  two tests of HOST-223 and the test of a string handle now run each class a second time
  with the throw uncaught, where no handler holds the value and only the check of the hook's
  answer faults the agent; with the check planted out three tests are red in the collect
  build.
- `167005b` is a lint fix after the first clippy run (two `is_multiple_of`, one type alias);
  no assertion changes in it.
- The merge `716b3cf`: one conflict, in the test module of `private_names.rs`, where this
  unit and the correction of the unit privatefields each added a library test at the same
  place; both are kept whole.

The coordinator's runs on `716b3cf`, build node, before the merge: `swell` main 1728 passed,
1 skipped (default build) and 1700 passed, 1 skipped (collect build); `swell` lib 287 passed;
`task test-stack-profiles` 7 passed in each of the two profiles; `task clippy` exit 0; `cargo
fmt --all -- --check` exit 0. The unit's runs on the same head: its filter with the private
fields and private methods tests, 85 passed (default) and 81 passed (collect);
`host-requirements-check` exit 0 with no requirement pending for slice 8;
`host-surface-check` names `HOST-223` alone, which the changelog entry of the commit after
the merge names.

Not covered: no case of test262 reaches the hook, so no part of test262 ran; a second
adversary pass did not run; the behaviour for a host that breaks its duty is held by one
test and by the text, and the engine does not detect such a host.

---
format: aep.planning-md/3
id: review-result:laneeight-step-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of step 1 of the unit laneeight after two adversary passes: ready, step 2 is built on it'
relations:
- reviews: story:slow-lane-holds-its-bound-on-the-slower-runner
revision: 1
---
Coordinator's read of step 1 of `story:slow-lane-holds-its-bound-on-the-slower-runner`
(the lane's eighth unit of wave big-2) after two adversary passes and two corrections,
2026-11-28. Verdict: step 1 is ready; step 2 is built on its head `7f9392c0`. The read
stands in place of a third pass.

What step 1 is. The 19 types of `crates/swell/tests/` that implement `Host` are three: the
scripted type `Scripted` (`support/scripted.rs`: a table `Hooks` of function pointers, the
test's state behind `Any` with a typed accessor, copies of the 18 provided defaults), and
`TestHost` and `ProvidedHost`, unchanged but for their marker lines. The 17 other types
are folded, none kept. The guard is the module `host_types.rs`.

The readings, on the build node, each build cold (the unit's report, confirmed by the
adversary from the logs and cargo's timing pages):

- The unit `main` in the collect build: 54.91 s (load 4.81 at the start), against 311 to
  358 s on the base. Bound 150 s.
- The unit `main` in the build with `TEST_OPT1_ARGS`: 72.17 s (load 10.83), against
  328.7 s. Bound 150 s.
- The inference of the story (about 100 s for three types) was on the safe side.

The gate, `task check-runner` on `3dba5760` (the unit's first head and the wave branch's
documents): exit 0; `test: swell::main 1677 at opt-level 1, 106 at opt-level 0` (1664 and
106 on the base, 13 guard tests more); `19 test binaries: 2545 passed, 0 failed, 1
ignored`; `gate-selftest: 114 passed, 0 failed`; `gate-time test-steps 52.2 of 120, cpu
303.4, load 28.98 on 32 cores`. The unit's own first gate stopped at `host-surface-check`
for a changelog entry of `HOST-224` that the coordinator had left out with the design
text; the entry is on the wave branch.

Pass 1 (on `3dba5760`): needs-change, 8 findings, 4 failing cases. The folds stood: the
hook sets equal the base's, a token comparison leaves formatting only, the 18 copies equal
the trait's defaults but for the documented `Err` arm, state is not shared, the test names
differ by the guard's only. What was found and what became of it:

- A macro that takes the trait as an argument writes an unmarked type. Taken.
- A second type named `TestHost` in another module passes as `provided-defaults`. Taken,
  with the reading "three types": the marker is accepted only in `tests/support/host.rs`,
  for the two names, once each.
- An `own-constant` with the trait's default value passes. Taken, with the reading "a
  value no other type has", the default included.
- `scripted.rs` named `std::any`, against the emulated host's sentence that an adapter
  calls no system API. Taken: `core::any`, held by the adversary's case.
- Six rules of the guard without a text case. Taken: three got a case that fails alone
  under the rule's mutant; three were removed as decided by no source form.
- No test pinned the panics of the accessor and of a required entry. Taken: two tests.
- Line numbers drift after a line that ends in `::`. Not taken: no formatted source has
  the form, and the helper is `source_scan.rs`'s.
- `math_random::a_call_makes_no_hook_call_and_costs_one_step` asserts absence only. Older
  than the unit: filed as `story:math-random-hook-count-reads-a-hook-that-runs`, a draft.

Pass 2 (on `c773ed4a`, the first correction): needs-change, 4 findings, 4 failing cases,
all in the guard. A variable as a later segment of the trait path and a header that is a
repetition still passed; the empty list with a comment or behind a cast still passed; the
panic of `state_mut` was not pinned; and the stated reason for removing one rule was
disproved by a form no macro of the tests has (a note: a false refusal, with a message
that tells its author what to write). The second correction, `7f9392c0`, takes the first
three and writes the note into the module's comment.

Checked by the coordinator on `7f9392c0`:

- `git diff --stat a510cf4b` outside `crates/swell/tests/` names three documents of the
  wave branch's merge and nothing else; no file under `crates/*/src/`.
- The diff of the second correction (`host_types.rs`, `scripted_host.rs`): the value of
  an `own-constant` is read with comments removed and must be a list literal with an
  element; the trait path is split at `::` and each segment but `$crate` is tested; a
  header with a repetition and no `for` is refused.
- The unit's report of the correction: 308 tests of the guard, `scripted_host`, `hooks`
  and the 17 folded modules pass; 12 mutants, each failing its own cases; clippy exit 0;
  1,805 names in `swell::main`, none removed since the base.

Where the delivery differs from the story's text, and stays: the guard is a module of
tests, one over the tree and one per text case, where the story writes "one test"; the
marker `provided-defaults` is bound to the support file; an `own-constant` must be a list
literal. The story's body takes these at the unit's close. The whole gate runs once more
on the head that carries both steps.

```findings
[]
```

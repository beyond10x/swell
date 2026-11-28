---
format: aep.planning-md/3
id: review-result:laneeight-plan-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of the lane story after two rounds of the plan critics: ready for the lane''s eighth unit'
relations:
- reviews: story:slow-lane-holds-its-bound-on-the-slower-runner
revision: 1
---
Coordinator's read of `story:slow-lane-holds-its-bound-on-the-slower-runner` after two
rounds of two plan critics (acceptance, design), 2026-11-28. Verdict: ready for the lane's
eighth unit with its body of revision 6. The read stands in place of a third round.

The story is the lane's eighth unit of wave big-2
(`approval-record:wave-big-2-lane-unit-8`). Its body rests on five forge runs of the lane
and on two probes on the build node: the build of the test binary of `crates/swell/tests`
is 95 % of the lane's background build, and that binary compiles the engine once for each
of its 19 host types.

Round 1, on revision 3. The acceptance critic: needs-revision, 13 findings. The design
critic: needs-revision, 10 findings. Revision 5 took them. What changed the story's shape:

- "At most two host types" was wrong: `TestHost` overrides the required hooks and three
  time-zone hooks only, and 84 files run the trait's provided defaults through it. Taken:
  three types (the scripted type, `TestHost`, `ProvidedHost`).
- A guard that holds a list and a count is a file every later story edits. Taken: a
  marker comment on the line before each `impl`, with a closed set of reasons.
- Step 2 was optional. Taken: required. Step 1 changes one part of the lane, and the other
  three sum to 518.5 s on the slower runner, where the lane is 223 s over.
- The figures: the 976 tests that can move take 249.21 s and 40.05 s; 18 files hold the 19
  types; the share of the property-escape cases is 76.8 %; the checks of the values at
  opt-level 1 are `test-opt1-vars-check` and `test-opt1-profile-check`.
- The bound of the build of `main`: 150 s in both builds at opt-level 1, from the
  inference for three types, and the forge's fallback as 900 and 600 divided by 1.419.

Round 2, on revision 5. The acceptance critic: needs-revision, 5 findings, and 11 of its
13 of round 1 settled. The design critic: needs-revision, 3 findings (1 major, 1 minor,
1 note), 9 of its 10 of round 1 settled, and its answer to the question put to it: the
panic in the scripted copy of `host_call_job_callback` is sound, since none of the nine
types that leave that hook to the trait has a test that reaches the `Err` arm. Revision 6
takes all eight:

- The profile test `realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on`
  is in `swell::main` and cannot be in the library run. Taken: the runs of `swell::main`
  hold it; the library run takes its profile and flags from the same two definitions.
- The library part printed no line when it moved. Taken: each moved run prints its counts,
  the library part with 0 at opt-level 0; the counts at opt-level 0 are named for each run.
- The guard bounded the reason `scripted` only. Taken: `provided-defaults` is refused on
  any type but the two named; `own-constant` must define the constant with a value text no
  other such `impl` has.
- Three acceptance lines joined several outcomes. Taken: split, and two more that the
  coordinator found on the same read.
- The two heads of the first pull request differ in `Taskfile.yml` and under
  `conformance/`. Taken: "differ in no crate source".
- The library reads `SWELL_FULL_COUNTS` when it is compiled, so the gate's build at
  opt-level 1 is not the lane's. Taken: three builds of the lane with the variable set and
  `COLLECT_RUSTFLAGS`; a moved run that compiles counts as failed.
- Nothing ties the copies of the provided defaults to the trait. Taken as accepted, said
  in the story and to be said in the type's doc comment; the defaults stay tested on
  `TestHost`.
- A bound on `Host` must not be refused. Taken: said, with a text case that passes.

Checked by the coordinator on `e2250d2b`, each by a command:

- 19 lines `impl Host for` under `crates/swell/tests/`, none with a type parameter; of the
  17 files with a type of their own, 9 do not name `host_call_job_callback` and 8 do.
- `option_env!("SWELL_FULL_COUNTS")` in three files of `crates/swell/src/`.
- `scan::rust_files` and `code_of` in `crates/swell/tests/source_scan.rs`; the macros
  `bad!`, `pool!`, `scoped_op!` in `fuzz.rs` and `scoped_answers!` in `misuse.rs`.
- The gate's line as the forge printed it on `f962c324` (run 46091909500):
  `test: swell::main 1664 at opt-level 1, 106 at opt-level 0`. The same log reads the
  gate's own build of the tests at opt-level 1 at 5 min 51 s, so step 1 also shortens the
  job `check`.
- The gate cases the scope names exist under `tests/gate/`.

Not taken: nothing. Open, and said in the story: the seconds test262 takes on its own on
the slower runner are not read, so the margin after both steps is an estimate until the
forge's run; what two further background builds cost the test262 workers on 4 cores is
read in that run. Not read again by a critic: each finding of round 2 is taken in the
critic's own smallest change, and the unit's adversary reads the result.

```findings
[]
```

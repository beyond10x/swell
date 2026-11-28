---
format: aep.planning-md/3
id: story:slow-lane-holds-its-bound-on-the-slower-runner
kind: story
status: draft
title: The lane slow holds its bound on the forge's slower runner
summary: The forge ran the lane slow on one head in 1,123.4 s on an AMD EPYC 7763 and 843.8 s on an AMD EPYC 9V74, of its 900. The test binary of swell compiles the engine once per test host type, 19 times, and that build bounds the lane's longest part. The test binary instantiates the engine once, the lane's runs that can move run at opt-level 1, and the lane holds 900 s and 600 s per part on the 7763 with the bounds unchanged. Lane unit 8 of wave big-2.
revision: 3
---
## Context

The lane `slow` is bounded on the forge at 900 s, with 600 s for each part
(`story:realm-size-lane-within-its-share`). Filed from the first pull request of wave
big-2, where the forge ran the lane on the same code in 882.6 s, 1,142.0 s and 876.4 s;
taken into the wave as its lane unit 8 on 2026-11-28
(`approval-record:wave-big-2-lane-unit-8`) after the lane failed on the third pull request.
This body is the second: it replaces the first with what the forge's runs and two probes
of 2026-11-28 measured.

### The lane by runner model

The lane prints its runner from `8d6cfebc` on (`slow-runner: 4 cores, <model>, ...`). The
forge's runs since, all on 4 cores:

| run | head | runner | lane | slow-test262 | build collect | slow-tests | test-realm-sizes | stack-profiles |
|---|---|---|---|---|---|---|---|---|
| 45972624800 | `8d6cfebc` | AMD EPYC 9V74 | 876.4 | 504.4 | 375.1 | 167.6 | 183.3 | 21.1 |
| 46069633020 | `5ab9f8e5` | AMD EPYC 9V45 | 673.0 | 359.8 | not read | 145.3 | 149.6 | 18.3 |
| 46077580621 | `4bb28e6f` (`main`) | AMD EPYC 7763 | 1,124.2 | 595.1 | not read | 228.1 | 275.5 | 25.5 |
| 46091909506, attempt 1 | `f962c324` | AMD EPYC 7763 | 1,123.4 | 604.9 | 604.4 | 231.2 | 260.0 | 27.3 |
| 46091909506, attempt 2 | `f962c324` | AMD EPYC 9V74 | 843.8 | 468.2 | 467.8 | 171.0 | 183.2 | 21.4 |

The slower runner is the AMD EPYC 7763. The two attempts of run 46091909506 ran the same
code: the 7763 took 1.331 times the seconds of the 9V74 for the lane (1,123.4 against
843.8). On the 7763 the lane is 223 s over its bound and the part `slow-test262` 4.9 s over
its own.

On both attempts the part `slow-test262` ended with the background build of the profile
collect (604.9 against 604.4, 468.2 against 467.8): the part waits for that build, and the
build is what bounds it. At `8d6cfebc` on the same 9V74 the build took 375.1 s and the
part 504.4 s.

### Where the build's seconds are

Measured on the build node (32 cores, Intel Xeon Platinum 8375C), every build cold:

- The collect build (`cargo nextest run --no-run` with `COLLECT_MAIN`) took 266 to 271 s at
  `8d6cfebc` and 319 to 327 s at `f962c324` at low load: 1.20 times. The unit `main`, the
  test binary of `crates/swell/tests`, is 95 % of it (259 to 358 s over six readings); the
  library `swell` takes 3.9 to 5.8 s. The same binary without the feature (`TEST_OPT1_ARGS`,
  the gate's build at opt-level 1) took 328.7 s for `main`. At opt-level 0 `main` takes
  22.8 s.
- `cargo check` of `main` takes 5.5 s at both commits. More than 95 % of the unit is
  monomorphization and LLVM, and the unit's CPU seconds are 1.1 to 1.2 times its wall
  seconds: it compiles on about one core.
- The engine is generic over its host (`impl<H: Host> Agent<H>`, 104 files). The test
  binary defines 19 types that implement `Host` (17 at `8d6cfebc`), and each instantiates
  the engine again inside `main`: about 350,000 lines of unoptimized LLVM IR, about 3,000
  functions and about 1.1 MB of machine code per host type. At `f962c324`, 6,782,548 of the
  8,018,579 IR lines of `main` (84.6 %) and 22.63 MB of its 25.84 MB of machine code
  (87.6 %) carry a host type in their symbol. One of the 16 codegen units of `main` holds
  21.69 MB of that code (83.9 %); 99.8 % of that unit is host instantiations.
- The bisect from `8d6cfebc` to `f962c324`, with the IR lines as the test: the merges
  `43284b76` (adds `HookHost`, `crates/swell/tests/private_element_hook.rs`) and `79ca8d38`
  (adds `HashHost`, `crates/swell/tests/eval_private_names.rs`) bring 372,338 and 382,006 of
  the 804,064 lines of growth; the two new host types are 87.2 % of it. The build read
  271.3 s before them and 319.5 to 325.0 s after.

Inferred, not measured: with one host type in place of 19, the large codegen unit shrinks
from 21.7 MB to 2 to 3 MB and `main` builds at opt-level 1 in under 100 s on the build
node, in both builds at that level (the lane's and the gate's).

### Where the tests' seconds are

Measured on the same node in CPU seconds (the forge's seconds on 4 cores are estimates from
these shares):

- `test-realm-sizes`, default mode at opt-level 0: 305 tests take 77.28 s at size 4 and
  87.23 s at size 200; at opt-level 1 the same tests take 11.38 s and 13.25 s. 50 further
  tests are guarded (stack tests) and stay at opt-level 0 (25.62 s and 29.82 s).
- `slow-tests`, default mode at opt-level 0: the 976 tests that can move take 329.70 s
  there and 40.45 s at opt-level 1. 44 guarded tests and the 71 tests of `swell-syntax`
  are not built at opt-level 1.
- `slow-tests`, collect mode at opt-level 0: the library part runs 287 tests in 75.49 s;
  62 of them, in the three modules that read `SWELL_FULL_COUNTS` (`heap::collect`,
  `script`, `ops::objects`), take 16.9 s. Two library tests take 18.98 s and 15.30 s at
  opt-level 0 and 2.14 s and 1.74 s at opt-level 1.
- `slow-test262`: `test/built-ins/RegExp/property-escapes/generated/` is 1,012 of the
  run's 1,317 CPU seconds (469 cases, 938 runs). That is the engine's time
  (`story:property-escape-cases-spend-less-in-the-interpreter-loop` took its first share)
  and no part of this story.

No test failed at opt-level 1 that passes at opt-level 0.

## Change

The lane `slow` ends within its bounds on the AMD EPYC 7763, with the bounds as they are
(900 s and 600 s). A bound is not raised, a part is not dropped from the lane, the lane is
not split into several jobs, and no test leaves the lane without a place that still runs
it. The engine's source (`crates/*/src/`) is not changed.

Two steps, each a unit of its own in the order below, each measured on the build node
before and after (one cold build, one run).

**Step 1, the cause: the test binary instantiates the engine once.** The host types of
`crates/swell/tests/` are folded into one host type whose hooks are held as values (a table
of function pointers and the test's state behind `Any`, or the form the unit finds
smaller): a test that today writes `impl Host for X` gives its hooks to that one type. A
second type stays only where one type cannot do it, each with its reason in the guard's
list below: `ProvidedHost` of `crates/swell/tests/support/host.rs` implements the five
required hooks only, so that the provided defaults of the trait are what runs, and a type
that overrode every hook would not run them. The tests' assertions are unchanged: the same
test names, the same step totals, the same transcripts of hook calls.

A guard in the gate holds the count: a test of the workspace reads the sources of
`crates/swell/tests/` and fails when a type implements `Host` that its list does not name,
with the line of the `impl` in its message. The list is in the test and names each type
with one reason.

**Step 2, the lane's opt-level-0 runs that can move.** With `main` cheap to build at
opt-level 1, the lane builds it without the feature too (`TEST_OPT1_ARGS`, the gate's
definition, in the background after the collect build) and runs at opt-level 1:

- the default mode of `test-realm-sizes` at its seven sizes, for the tests that are not
  guarded;
- the default mode of `slow-tests`, for the 976 tests that can move;
- the library part of the collect mode of `slow-tests`, with `SWELL_FULL_COUNTS=1`.

The guarded stack tests and the tests of `swell-syntax` stay at opt-level 0 in the lane,
as today. The definitions stay single: the lane takes `TEST_OPT1_ARGS`, `COLLECT_MAIN` and
`NO_STACK_TESTS` from the top of `Taskfile.yml`, and `task test-vars-check` covers every
new use.

Step 2 is built on step 1's head after the coordinator has read step 1's forge run. If
that run already holds the bounds on the 7763 with 100 s to spare, step 2 is still built:
the lane had 23.6 s and 56.2 s to spare on the 9V74 in the two runs above.

## Acceptance

- The guard: with a planted `impl Host for` of a type the list does not name, in a file of
  `crates/swell/tests/`, the guard's test fails and names the file and the line; without
  the plant it passes. `grep -rn -E 'impl (swell::)?Host for' crates/swell/tests/` prints
  as many lines as the list has entries, at most 2.
- The unit `main` of the collect build (`cargo nextest run --no-run` with `COLLECT_MAIN`,
  cold, on the build node, read from `cargo --timings`) takes at most 120 s after step 1,
  against 311 to 358 s at `f962c324`. The report names both readings and the node's load
  at their start.
- The set of test names of `swell::main` printed by `cargo nextest list` is the same before
  and after step 1, but for the guard's test, and `task check` passes with
  `conformance/test262/expected.txt` and every recorded step total unchanged.
- A run of the workflow `slow` on the forge on the unit's head whose `slow-runner` line
  names the AMD EPYC 7763 prints `slow-time lane <s> of 900` with s at or under 900 and no
  part over 600. Or, when the forge gives no 7763 in three runs of the head, a run on the
  AMD EPYC 9V74 with s at or under 676 and no part over 450 (900 and 600 divided by
  1.331). The pull request names the run and its `slow-runner` line.
- The bounds in `taskfiles/gate-time.yml` are unchanged, and
  `tests/gate/slow-time-refuses-a-part-over-its-bound.sh` and
  `tests/gate/slow-time-refuses-the-lane-over-its-bound.sh` pass unchanged.
- Every test the lane ran before the unit runs after it, in the lane: the report compares
  the lists of test names of each of the lane's runs, before and after, per build, and
  names for each test that changed its opt-level the run that holds it now.
- The gate's test steps stay within 120 s and no test runs over 60 s; the forge's job
  `check` on the unit's head is green, and the pull request names its `gate-time` line of
  the test steps beside the 84.8 s of run 46091909500.
- `git diff --stat` of the unit names no file under `crates/*/src/`.

## Out of scope

- The engine's time on the property-escape cases, 73 % of the test262 run.
- Making the engine's code that does not depend on the host non-generic, or spreading its
  methods over codegen units: a change of the engine's architecture, the operator's.
- `collect-every-safe-point` as a switch at run time.
- The forge's choice of runner, and a larger or a paid runner: the operator's.
- Splitting the lane into several forge jobs: it changes what the bound of 900 s measures
  and is the operator's decision.

## Scope

Step 1: `crates/swell/tests/` (the 19 files with a host type, `support/host.rs`, the
guard's module and the generated `main.rs`). Step 2: the tasks `slow`, `slow-tests`,
`test-realm-sizes` and `test-vars-check` of `Taskfile.yml`, `taskfiles/gate-time.yml` (no
bound), `tests/gate/` (the lane's cases), `AGENTS.md` (the lane's paragraph, the
coordinator's). No file of `crates/*/src/`, `conformance/` or `docs/design/`.

---
format: aep.planning-md/3
id: story:slow-lane-holds-its-bound-on-the-slower-runner
kind: story
status: draft
title: The lane slow holds its bound on the forge's slower runner
summary: 'The forge ran the lane slow on one head in 1,123.4 s on an AMD EPYC 7763 and 843.8 s on an AMD EPYC 9V74, of its 900. The test binary of swell compiles the engine once per test host type, 19 times, and the lane''s longest part does not end before that build. Step 1: three host types in the test binary, held by a guard. Step 2: the lane''s runs that can move run at opt-level 1. The lane then holds 900 s and 600 s per part on the 7763 with the bounds unchanged. Lane unit 8 of wave big-2.'
revision: 6
---
## Context

The lane `slow` is bounded on the forge at 900 s, with 600 s for each part
(`story:realm-size-lane-within-its-share`). Filed from the first pull request of wave
big-2, where the forge ran the lane in 882.6 s and 876.4 s on two heads that differ in no
crate source (`b9adbe35`, `8d6cfebc`); taken into the wave as its lane unit 8 on 2026-11-28
(`approval-record:wave-big-2-lane-unit-8`) after the lane failed on the third pull request.
This body rests on the forge's runs, on two probes of 2026-11-28 and on two rounds of the
plan critics.

### The lane by runner model

The lane prints its runner from `8d6cfebc` on (`slow-runner: 4 cores, <model>, ...`). The
forge's runs since, all on 4 cores, read from each run's log (`slow-time` and `slow-build`
lines):

| run | head | runner | lane | slow-test262 | build collect | slow-tests | test-realm-sizes | stack-profiles |
|---|---|---|---|---|---|---|---|---|
| 45972624800 | `8d6cfebc` | AMD EPYC 9V74 | 876.4 | 504.4 | 375.1 | 167.6 | 183.3 | 21.1 |
| 46069633020 | `5ab9f8e5` | AMD EPYC 9V45 | 673.0 | 359.8 | not read | 145.3 | 149.6 | 18.3 |
| 46077580621 | `4bb28e6f` (`main`) | AMD EPYC 7763 | 1,124.2 | 595.1 | not read | 228.1 | 275.5 | 25.5 |
| 46091909506, attempt 1 | `f962c324` | AMD EPYC 7763 | 1,123.4 | 604.9 | 604.4 | 231.2 | 260.0 | 27.3 |
| 46091909506, attempt 2 | `f962c324` | AMD EPYC 9V74 | 843.8 | 468.2 | 467.8 | 171.0 | 183.2 | 21.4 |

The slower runner is the AMD EPYC 7763. The two attempts of run 46091909506 ran the same
code: the 7763 took 1.331 times the seconds of the 9V74 for the lane (1,123.4 against
843.8), and by part 1.292 (`slow-test262`), 1.352 (`slow-tests`), 1.419
(`test-realm-sizes`) and 1.276 (`stack-profiles`). That is one pair of readings. On the
7763 the lane is 223 s over its bound and the part `slow-test262` 4.9 s over its own.

On both attempts the part `slow-test262` ended within 0.5 s of the background build of the
profile collect (604.9 against 604.4, 468.2 against 467.8), and `slow-tests` then printed
`slow-build collect 467.8 s, exit 0, waited 0 s`: the part did not end before the build. At
`8d6cfebc` on the 9V74 the build ended first (375.1 s) and the part read 504.4 s. Not read:
the seconds test262 takes on its own on either model at `f962c324`; the lane prints no
line for them. They are at most the part's seconds (604.9 on the 7763), and after step 1
below they are what the part reads.

### Where the build's seconds are

Measured on the build node (32 cores, Intel Xeon Platinum 8375C), every build cold (the
build probe's report):

- The collect build (`cargo nextest run --no-run` with `COLLECT_MAIN`) took 266 to 271 s at
  `8d6cfebc` and 319 to 327 s at `f962c324` at low load: 1.20 times. The unit `main`, the
  test binary of `crates/swell/tests`, is 95 % of it (259 to 358 s over six readings); the
  library `swell` takes 3.9 to 5.8 s. The same binary without the feature (`TEST_OPT1_ARGS`,
  the gate's build at opt-level 1) took 328.7 s for `main`. At opt-level 0 `main` takes
  22.8 s (the tests probe's report).
- `cargo check` of `main` takes 5.5 s at both commits. More than 95 % of the unit is
  monomorphization and LLVM, and the build's CPU seconds are 1.1 to 1.2 times its wall
  seconds: it runs on about one core.
- The engine is generic over its host (`impl<H: Host> Agent<H>`, 104 files). The test
  binary defines 19 types that implement `Host`, in 18 files (17 types at `8d6cfebc`), and
  each instantiates the engine again inside `main`: about 350,000 lines of unoptimized LLVM
  IR, about 3,000 functions and about 1.1 MB of machine code per host type (`TestHost`, the
  largest: 444,790 lines, 1.79 MB). At `f962c324`, 6,782,548 of the 8,018,579 IR lines of
  `main` (84.6 %) and 22.63 MB of its 25.84 MB of machine code (87.6 %) carry a host type
  in their symbol. One of the 16 codegen units of `main` holds 21.69 MB of that code
  (83.9 %); 99.8 % of that unit is host instantiations.
- The bisect from `8d6cfebc` to `f962c324`, with the IR lines as the test: the merges
  `43284b76` (adds `HookHost`, `crates/swell/tests/private_element_hook.rs`) and `79ca8d38`
  (adds `HashHost`, `crates/swell/tests/eval_private_names.rs`) bring 372,338 and 382,006 of
  the 804,064 lines of growth; the two new host types are 87.2 % of it. The build read
  271.3 s before them and 319.5 to 325.0 s after.

Inferred, not measured: with three host types in place of 19 (step 1), the large codegen
unit shrinks from 21.7 MB to about 5 MB, and `main` builds at opt-level 1 in about 100 s on
the build node if its seconds follow its bytes, in both builds at that level (the lane's
and the gate's).

### Where the tests' seconds are

Measured on the same node in CPU seconds (the tests probe's report; the forge's seconds on
4 cores are estimates from these shares):

- `test-realm-sizes`, default mode at opt-level 0: 305 tests take 77.28 s at size 4 and
  87.23 s at size 200; at opt-level 1 the same tests take 11.38 s and 13.25 s. 50 further
  tests are stack tests, stopped by the guard at opt-level 1, and stay at opt-level 0
  (25.62 s and 29.82 s).
- `slow-tests`, default mode at opt-level 0: the 976 tests that can move take 249.21 s
  there and 40.05 s at opt-level 1. 44 stack tests are built at opt-level 1 and stopped
  there by the guard; the 71 tests of `swell-syntax` are not built at opt-level 1.
- `slow-tests`, collect mode at opt-level 0: the library part runs 287 tests in 75.49 s;
  62 of them, in the three modules that read `SWELL_FULL_COUNTS` (`heap::collect`,
  `script`, `ops::objects`), take 16.9 s. Two library tests take 18.98 s and 15.30 s at
  opt-level 0 and 2.14 s and 1.74 s at opt-level 1.
- `slow-test262`: `test/built-ins/RegExp/property-escapes/generated/` is 1,012 of the
  run's 1,317 CPU seconds (76.8 %; 469 cases, 938 runs). That is the engine's time
  (`story:property-escape-cases-spend-less-in-the-interpreter-loop` took its first share)
  and no part of this story.

No test failed at opt-level 1 that passes at opt-level 0, but the stack tests the guard
stops. The one assertion of the tests that turns on the profile is
`realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on`, which the
profile collect satisfies; the readings of a clock in the tests are printed and not
asserted.

## Change

The lane `slow` ends within its bounds on the AMD EPYC 7763, with the bounds as they are
(900 s and 600 s). A bound is not raised, a part is not dropped from the lane, the lane is
not split into several jobs, and no test leaves the lane. The engine's source
(`crates/*/src/`) is not changed.

Two steps, each a unit of its own in the order below, each measured on the build node
before and after. Both are required: step 1 changes the part `slow-test262` only, which
cannot fall below the seconds of test262 itself, and the lane is 223 s over on the 7763,
where its other three parts sum to 518.5 s. The tests probe puts the gain of step 2 at
about 220 s on a 4-core runner (an estimate from the node's shares).

### Step 1, the cause: three host types in the test binary

The 19 host types of `crates/swell/tests/` become three:

- A new scripted type in `crates/swell/tests/support/`: its hooks are values (a table of
  function pointers over `&mut HookCx<'_, Self>`, or `&mut Cx<'_, Self>` where the trait
  has it, and the test's own state behind `Any` with a typed accessor). The 17 types that
  are neither `TestHost` nor `ProvidedHost` are folded into it: a test that writes
  `impl Host for X` today gives its hooks and its state to the scripted type. The scripted
  type overrides a hook when one of the 17 does. For a test that left such a hook to the
  trait, the table's entry is a copy of the provided default, written once beside the type.
  The copy of `host_call_job_callback` calls `cx.call` as the default does; its `Err` arm,
  which the default answers with the agent's fault (not reachable from a test crate),
  panics with a message that names the broken invariant. Nine of the 17 leave that hook to
  the trait, and none of their tests reaches that arm. Accepted and said in the type's doc
  comment: nothing ties a copy to the trait's default but this sentence, so a change of a
  provided default changes its copy in the same change; the defaults themselves stay tested
  on `TestHost`.
- `TestHost` (`support/host.rs`) stays as it is: it overrides the required hooks and the
  three time-zone hooks only, so the 84 files that use it run the provided defaults of the
  trait, and the tests of `crates/swell/tests/hooks.rs` that call a default on it
  (`host_124_*`, the constant at line 169) stay unchanged.
- `ProvidedHost` (`support/host.rs`) stays as it is: the five required hooks only.

The tests' names and assertions are unchanged. What changes is how a test reaches its
host's state (the typed accessor in place of a field of its own type), in the test files
and in the macros that name one (`bad!`, `pool!` and `scoped_op!` of `fuzz.rs`,
`scoped_answers!` of `misuse.rs`).

A guard in the gate holds this. One test of `swell::main`, in a module of its own, reads
the sources of `crates/swell/tests/` with the helpers `source_scan.rs` already has
(`scan::rust_files`, `code_of`, which blanks comments and strings) and fails, naming file
and line, on:

- an `impl` of `Host` (any path to the trait, any generics, the `impl`, the trait and `for`
  on one line or wrapped over several) that does not carry, on the line before it, the
  marker comment `// host-type: <reason>` (read from the source as written) with a reason
  of the closed set `scripted`, `provided-defaults`, `own-constant`;
- a second `impl` marked `scripted`;
- an `impl` marked `provided-defaults` for a type other than `TestHost` and `ProvidedHost`;
- an `impl` marked `own-constant` whose body does not define `SUPPORTED_IMPORT_ATTRIBUTES`,
  or defines it with the text of the value another such `impl` has;
- an `impl` of `Host` with a type parameter (one engine copy per argument);
- `Host` renamed in a `use`;
- `Host` named inside the body of a `macro_rules!`.

It does not refuse `Host` as a bound (`impl<H: swell::Host> Trait for X<H>`, `fn f<H:
Host>`).

`own-constant` is for a type whose `SUPPORTED_IMPORT_ATTRIBUTES` no other type has: the
constant is per type, and the stories of slice 11 that test a load with an attribute need
one. No such type exists at the end of this unit; each one a later story adds is one more
copy of the engine in `main`, at most one per value of the constant. The guard holds no
count and no list a story edits, so two stories built at the same time write no common
file for it. Its refusals are proven by text cases inside the test, as `source_scan.rs`
proves its own: a plain `impl` without the marker, a generic one, a path `::swell::Host`,
a wrapped one, a renamed `use`, a `macro_rules!` body, an unknown reason, two `scripted`,
a third `provided-defaults`, an `own-constant` without the constant, and two `own-constant`
with one value; and a bound, which passes.

### Step 2, the lane's opt-level-0 runs that can move

With `main` cheap to build at opt-level 1, the lane builds it without the feature too and
runs at opt-level 1:

- the default mode of `test-realm-sizes` at its seven sizes, for the tests that are not
  stack tests;
- the default mode of `slow-tests`, for the tests that can move (976 on the base);
- the library part of the collect mode of `slow-tests`, with the full counts.

The stack tests and the tests of `swell-syntax` stay at opt-level 0 in the lane, as today.
How:

- The split of a run into its opt-level-1 list and its opt-level-0 list is the gate's:
  `test-split` is given the run's filter as a parameter and writes the two nextest
  configurations for it, and it fails, naming the test, when a test of the run is in none
  or in both. The lane calls it for each run it splits (the default mode of `slow-tests`,
  and the default mode of `test-realm-sizes`, whose list is the tests that read the realm's
  room without the stack tests). It is a check of every run of the lane, not a report made
  once.
- Each opt-level-1 run of `swell::main` in the lane holds
  `realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on` in its list.
  The library run has no such test: its build takes the profile and the flags from
  `COLLECT_MAIN` and `COLLECT_RUSTFLAGS`, the single definitions from which the collect
  run of `swell::main`, which holds the test, is built.
- The lane calls `test-opt1-vars-check` and `test-opt1-profile-check`, which hold
  `TEST_OPT1_ARGS`, `NO_STACK_TESTS`, `TEST_OPT1_PROFILE_TEST` and `TEST_OPT0_TESTS`
  against a value given on the command line; the sentence of `test-opt1-vars-check` that
  the lane's tasks do not call it goes.
- The builds. The library reads the full counts when it is compiled
  (`crates/swell/src/heap/collect.rs`, `option_env!`), so the gate's build at opt-level 1
  is not the lane's: a test built without `SWELL_FULL_COUNTS=1` would run at the gate's
  counts and pass. The lane makes three builds at opt-level 1, one after the other in the
  background, each with `SWELL_FULL_COUNTS=1` and `COLLECT_RUSTFLAGS` (so that no crate is
  compiled twice for a flag), each with its own status file and its own `slow-build` line:
  the collect build of `main` (today's), the build with `TEST_OPT1_ARGS` for the default
  modes, and the library tests with the feature for the collect mode. `slow-collect-start`
  and `slow-collect-stop` carry the three. `build-realm-sizes` builds what
  `test-realm-sizes` runs, since that task counts a `Compiling` line in a run as a failure.
- Each run of the lane that moved prints, as the gate's step `test` does, a line with its
  counts: `<run>: <n> tests at opt-level 1, <m> at opt-level 0`; the library part of the
  collect mode prints it with 0 at opt-level 0.
- A run cannot use a build made without the variable: each run sets `SWELL_FULL_COUNTS=1`
  as today, cargo compiles the library again when the build it finds was made without it,
  and each moved run counts a `Compiling` line in its output as a failure, as
  `test-realm-sizes` does today.

Accepted and said here: a library test that bounds its own stack without the helpers of
`crates/swell/tests/support/` would not be stopped by the guard in the library run at
opt-level 1; the tests probe found none. Not measured: what the two further builds cost
the four workers of test262 on a 4-core runner while they run beside it; the step's forge
run reads it in the part `slow-test262`.

Step 2 is built on step 1's head.

## Acceptance

Step 1:

- The guard's test passes on the sources of the unit's head.
- Each text case of a refusal named above is refused with its file and line in the
  message, and the text case of a bound passes.
- `crates/swell/tests/` holds three types that implement `Host`: the scripted type,
  `TestHost` and `ProvidedHost`, each with its marker.
- The unit `main` of the collect build (`cargo nextest run --no-run` with `COLLECT_MAIN`,
  cold, on the build node, read from `cargo --timings`) takes at most 150 s, against 311 to
  358 s at `f962c324`. The report names both readings and the node's load at their start.
  The bound is set from the inference above for three types.
- The unit `main` of the build with `TEST_OPT1_ARGS`, read the same way, takes at most
  150 s, against 328.7 s.
- The test names of `swell::main` printed by `cargo nextest list` are the same before and
  after, but for the guard's tests.
- `task check` passes.
- `conformance/test262/expected.txt` is unchanged.
- The line `test: swell::main <a> at opt-level 1, <b> at opt-level 0` of `task check`
  reads a sum that is higher than on the base by the number of the guard's tests.
- `git diff --stat` of the step names no file under `crates/*/src/`.

Step 2:

- Each moved run of the lane prints its line of counts. On the unit's head the counts at
  opt-level 0 are: in each default run of `test-realm-sizes` its stack tests (50 on the
  base); in the default mode of `slow-tests` its stack tests (44 on the base) and the 71
  tests of `swell-syntax`; in the library part of the collect mode 0. Every other test of
  a run is at opt-level 1.
- A moved run whose output holds a `Compiling` line fails (a case in `tests/gate/`).
- With a test planted that is in neither list of a split run, and with one planted in
  both, the lane's call of `test-split` fails and names the test (two cases in
  `tests/gate/`).
- For each run of the lane, the set of test names it ran before the unit equals the set it
  runs after, the two opt-levels taken together; the lists are those of `cargo nextest
  list` with the run's filter on the base and on the head, and the comparison is in the
  unit's report.
- `task test-opt1-vars-check` passes.
- `task test-opt1-profile-check` passes.
- The lane fails when `TEST_OPT1_ARGS` is given another value on the command line (a case
  in `tests/gate/`).
- `taskfiles/gate-time.yml`, `tests/gate/slow-time-refuses-a-part-over-its-bound.sh` and
  `tests/gate/slow-time-refuses-the-lane-over-its-bound.sh` are unchanged (`git diff
  --stat` of the step names none of them).
- The two scripts `slow-time-refuses-*` named above pass.
- The gate's test steps stay within 120 s (the line of the test steps of `task check`).
- No test of the gate runs over 60 s.

The forge, on the head of the pull request that carries both steps:

- The job `slow` is green on a runner whose `slow-runner` line names the AMD EPYC 7763,
  with `slow-time clock` at or under 900 and the longest part at or under 600. The pull
  request names the run, its attempt and its `slow-runner` line.
- The runs that count are the run of the pull request's head and at most two further
  attempts of it; an attempt counts as a run. When none of the three draws a 7763, a green
  run on the AMD EPYC 9V74 with `slow-time clock` at or under 634 and the longest part at
  or under 422 (900 and 600 divided by 1.419, the largest ratio of a part above) closes
  the story as an estimate, said so in the pull request with the three `slow-runner`
  lines, and the first run of the lane on a 7763 after it that is over a bound opens the
  story again. A run on another model is no reading.
- The job `check` is green; the pull request names its `gate-runner` line and its line of
  the test steps, read against the bound of 120 s.

## Out of scope

- The engine's time on the property-escape cases, three quarters of the test262 run.
- Making the engine's code that does not depend on the host non-generic, or spreading its
  methods over codegen units: a change of the engine's architecture, the operator's.
- The test binary in a package of its own at opt-level 0 against the engine at opt-level 1
  (measured by the tests probe: it builds in 32 s and runs 2.5 times slower than opt-level
  1): it renames the binary `swell::main` in about 140 lines of the repository and moves
  the directory `source_scan.rs` reads; not taken.
- `collect-every-safe-point` as a switch at run time.
- The forge's choice of runner, and a larger or a paid runner: the operator's.
- Splitting the lane into several forge jobs: it changes what the bound of 900 s measures
  and is the operator's decision.

## Scope

Step 1: `crates/swell/tests/`: the 18 files that hold a host type, the new file of the
scripted type under `support/`, the guard's module, and the generated `main.rs`. Step 2:
the tasks `slow`, `slow-collect-start`, `slow-collect-stop`, `slow-tests`,
`build-realm-sizes`, `test-realm-sizes`, `test-split`, `test-opt1-vars-check` and
`test-opt1-profile-check` of `Taskfile.yml`; `tests/gate/` (the cases `slow-collect-*.sh`,
`slow-tests-runs-the-stack-tests-at-opt-level-0.sh`,
`test-realm-sizes-hands-nextest-a-long-list-of-tests.sh` and the new ones);
`AGENTS.md` (the lane's paragraphs, the coordinator's). `taskfiles/gate-time.yml` is not
written. No file of `crates/*/src/`, `conformance/` or `docs/design/`.

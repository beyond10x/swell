---
format: aep.planning-md/3
id: story:gate-test-steps-within-budget
kind: story
status: active
title: The gate's test steps end within two minutes
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T21:08:02Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T21:08:03Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
## Context

A defect of the gate, not a feature. A rule for every repository of the organization, set on
2026-11-21 ("Tests stay fast", the `AGENTS.md` of the folder that holds this repository):

- the test step of a repository's gate ends within 2 minutes of wall time;
- no single test runs longer than 60 s; the runner kills such a test and counts it as failed
  (cargo-nextest `slow-timeout` with `terminate-after`), and the step prints its ten slowest
  tests;
- an expensive product is made once, never per test: a committed generated file with a digest
  of its sources that a fast check compares, or one shared fixture built once;
- full regeneration from sources runs in a separate lane named `slow`, only when the sources
  changed, and compares bytes with the committed file;
- the test step makes no model call, no call to a real API and no network request;
- a step over the budget gets a story and is fixed before the next feature. Raising a timeout,
  adding shards or waiting longer is not a fix.

Measured on `29f674c` (release 0.2.0), warm build, the shared machine, each step alone:

- `test` 84 s: 62 suites, their run times sum to 83.2 s; `crates/swell/tests/sources.rs`
  29.63 s, `tests/to_string.rs` 27.24 s, `tests/fuzz.rs` 14.47 s, `tests/tracing.rs` 4.75 s;
- `test-collect` 110 s (every test of `swell` again with the collector run at every safe
  point);
- `test262` 90 s (85,794 runs; it grows with every slice, since a case that fails early today
  runs to its end once its builtin exists);
- `host-requirements-check` 7 s;
- not measured: `conform` (it calls `ess verify conform synthesize` once per gate run),
  `conform-engine`, `test-faults`, `test262-moves-check`, `gate-selftest` (32 scripts).

The three measured steps alone take 284 s. The whole gate in runner form took about 11 minutes
on the release commit, the build included. Slice 3 starts after this story.

## Change

"The test steps" below are all steps `check` runs after `clippy`.

1. **Measure first and keep the output.** The wall time of each test step alone (once each;
   the whole gate is not started before the steps are under the budget), and the time of each
   test of `test` and `test-collect`. For each of the twenty slowest tests, say what it spends
   its time on.
2. **The time of each step is printed.** `check` prints one line per test step,
   `gate-time <step> <seconds>`, and a last line `gate-time test-steps <sum> of 120`. **The
   gate fails when that sum is over 120**, locally and on the forge (the rule, as extended on
   2026-11-21: "the step itself fails when it runs over 120 s"), with a line that names the
   sum and the three slowest steps. The seconds of a step are the time it ran: time a command
   waited in the pool's queue for a slot is not the repository's and is left out where `slot`
   reports it; say in the report how you separate the two, and what the number means on the
   forge, where there is no pool. The budget, 120, is a variable of the gate with that
   default, so that a planted script can show the failure with a small budget; it is a gate
   variable, and nothing raises it. The wave record and the gate block of `AGENTS.md` carry
   the line. A planted script shows the refusal: a gate run whose test steps exceed the
   budget fails.
3. **`test`, `test-collect` and `test-faults` run through cargo-nextest** (0.9.143 is on the
   machine; it is a tool of the gate and ships with nothing). `.config/nextest.toml`: a test
   is terminated at 60 s and counts as failed. Each of the three steps prints its ten slowest
   tests as `gate-slowest <step> <seconds> <binary> <test>`. Doc tests, which nextest does not
   run, keep a step of their own that is timed like the others. The summed line each step
   prints today keeps its meaning (suites or binaries, passed, failed, ignored); state the old
   and the new form side by side in the report, since the documents quote it.
   nextest starts a process per test: a `OnceLock` in a test binary is then built once per
   test, not once. A product several tests need is therefore a committed generated file with
   its digest check, or the tests that share it are one test function that builds it once.
4. **No test costs more than it must.** Where a test builds something expensive that other
   tests build too (a parse of a large source, a realm with every builtin, a compiled set),
   it is built once by one of the two forms of point 3. No test is deleted and no assertion is
   removed or weakened. An iteration count (`tests/fuzz.rs`) is lowered only when the full
   count runs in the lane `slow`, and the report states both counts. After the change no test
   runs over 5 s; one that must is named in the report with its time and the reason.
5. **The full test262 run leaves `check` and becomes the lane `slow`.**
   `conformance/test262/expected.txt` is the committed generated file. A digest of its sources
   is committed beside it (you name the file): every tracked file that can change a run line
   (the crates, the root manifest and lock, the toolchain file if there is one, the inputs
   under `conformance/test262/`, the `swell-slices` block of `docs/design/engine.md`, the task
   file of the runner). `check` runs a fast check in the place of `test262`: it computes the
   digest again, fails with the names of both when they differ, and prints the lines the gate
   prints today (`test262-refused`, `test262 total`, the `test262-slice` lines) from the
   committed file. `task slow` runs the full run from the sources and compares bytes with the
   committed file, as `task test262` does today, and checks the digest. The run that writes
   `expected.txt` writes the digest, and nothing else does. The variables that point the
   comparison at another file today (`TEST262_EXPECTED` and the others of
   `taskfiles/test262.yml`) work for the digest the same way and are gate variables, so that a
   unit of a wave of several units still leaves both files to the coordinator.
6. **`conform` and `conform-engine`.** Measure them. If `ess verify conform synthesize` takes
   more than 5 s, its suite becomes a committed generated file with a digest of `spec/`, read
   by the gate, and the synthesis moves to the lane `slow`. No file under `spec/scenarios/`
   is edited, and the seed of the scenarios is quoted in no document.
7. **`gate-selftest`.** Measure each of the 32 scripts. A script that builds or runs tests
   to show a refusal is changed to show it on the smallest input that still triggers the
   refusal. Each script still refuses its planted defect.
8. **Planted defects for the new checks**, each a script under `tests/gate/` that runs in
   under 5 s and is written before the check it tests: a source of `expected.txt` changed
   without the digest is refused; a line of `expected.txt` changed with the digest left alone
   is refused by `task slow`'s comparison (show it on a reduced run, not on the full one);
   a `.config/nextest.toml` that lets a test run past 60 s is refused; a test step that does
   not run through nextest is refused.
9. **The forge.** `.github/workflows/check.yml` installs cargo-nextest pinned by version and
   SHA-256, in the form the sibling repository `conductor` uses in its `check.yml` (read it
   there; copy the form, and the version and digest when they are those of 0.9.143; make no
   network request yourself). A new workflow `slow.yml`, named `Slow lane`, runs `task slow`
   on a push to `main` and on a pull request when a source of the digest, `expected.txt` or
   the digest file changed, with the same pinned actions and tool versions as `check.yml`.
   Neither can be run from the unit; list for the coordinator what is new for the runner.

## Constraints

- Nothing third-party in what ships; no `unsafe`; the stack tests, `STACK_BYTES_PER_REENTRY`
  and the frame of the run loop are not touched (`AGENTS.md`, "Commands and the gate").
- The local build equals the forge's (`.cargo/config.toml`): no profile, feature or flag that
  only one side sets.
- No count of the gate goes down: tests passed, scenarios passed, requirements with a test,
  test262 lines, planted defects.
- `AGENTS.md`, `README.md`, `CHANGELOG.md` and `docs/` are the coordinator's: list every place
  that quotes a line whose form changed.

## Acceptance

On the unit's branch, on a warm build:

1. `slot task check` exits 0 and its last `gate-time` line reads `gate-time test-steps <sum>
   of 120` with a sum of at most 120; the report states the pool's load during that run.
2. No `gate-slowest` line of that run shows a test over 5 s, except the tests the report
   names with their reason, and none over 60 s.
3. The sum of passed tests of `test` (doc tests included), `test-collect` and `test-faults`
   is at least what `29f674c` printed (341, 238 and 1), and no test is ignored that was not.
4. `check` prints the same `test262-refused`, `test262 total` and `test262-slice` lines as on
   `29f674c`, read from the committed file, and one line that the digest is equal.
5. `slot task slow` exits 0: the full run equals `expected.txt` byte for byte (286 lines).
6. `gate-selftest` passes with at least 36 scripts (the 32 of today and the four of point 8).
7. The four planted scripts of point 8 each ran red before their check existed; the report
   quotes the red line of each.

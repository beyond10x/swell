---
format: aep.planning-md/3
id: story:stack-requirements-hold-in-the-release-profile
kind: story
status: implemented
title: The gate runs the stack tests of HOST-17, HOST-152, HOST-163 and the builtins in the release profile and in the dev profile with debug information off
summary: A build step builds the two stack test binaries in the release profile and in a profile stack-nodebug (dev with debug off); a timed test step runs their four tests in both, and readelf shows no debug section in either. Pass or fail bounds on the bytes per re-entry and the added wall time.
relations:
- depends_on: story:gate-test-steps-within-budget
- depends_on: story:local-gate-equals-forge
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:48:30Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T06:48:30Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:51Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Waits for: `story:gate-test-steps-within-budget` (cargo-nextest, `.config/nextest.toml` with a
test killed at 60 s, the `gate-time` lines and their sum of 120) and `story:local-gate-equals-forge`
(the pinned compiler in `rust-toolchain.toml`, read by the local gate and the forge). Batch H1.
No new requirement: HOST-17, HOST-152 and HOST-163 hold in two more profiles. Consumer: the
browser, requirement 6 of its list ("the release profile and debug information off, each shown by
a test in that profile"). Decision D6 (taken: option A, the release-profile stack tests run in
the gate).

## Context

Line numbers are of `wave/budget-1` at `ed56b05`, and of the gate unit's tree where named.

- The stack tests: HOST-17 `host_17_deepest_reentry_parses_at_full_nesting`
  (`crates/swell/tests/limits_scripts.rs:110`), `min_stack_bytes_is_enough_through_builtins`
  (`:137`), the worst path, `toJSON`, at 11,100 bytes per re-entry (`AGENTS.md:168`), HOST-152 and
  HOST-163 (`crates/swell/tests/eval_stack.rs:172`, `:200`). Each runs its subject in a child
  process on a thread of seven eighths of `min_stack_bytes()` (`crates/swell/tests/support/stack.rs:79`,
  `on_bounded_stack`; `crates/swell/tests/support/host.rs:90`, `stack_with_room`).
- `STACK_BYTES_PER_REENTRY` is 14,336 bytes (`crates/swell/src/host/limits.rs:46`), the largest
  measured need plus a fifth, measured in the forge's build, dev with full debug information, by
  bisection of the smallest thread stack in a child process at depths 200 and 1,000
  (`docs/plan/slice-02-run-loop-frame.md:22-24`).
- The gate builds the dev profile with full debug information: `.cargo/config.toml` holds only
  `[profile.dev] debug = true`, in this tree and in the gate unit's; the manifest's profiles set
  only `overflow-checks = true` (`Cargo.toml:10-20`); the gate's `build` task sets only
  `RUSTFLAGS: -D warnings` (gate unit's `Taskfile.yml:407-415`). No step of the gate builds
  without debug information, and none builds the release profile: `cargo run --release` of
  `taskfiles/test262.yml:72` runs in the lane `slow` and by hand, not in `check`.
  `story:local-gate-equals-forge` does not write `.cargo/config.toml`.
- No test runs the stack tests in the release profile, where inlining joins frames and a frame on
  the re-entry path can grow or shrink, nor in a dev build without debug information.
- An embedder's own profile applies to swell when swell is its dependency; swell's profiles do
  not. A third profile of a host (link-time optimisation, one codegen unit, `opt-level = "s"`) is
  the host's to test with the same tests.

**Times, measured and estimated.** Measured: the gate unit tree's last `.out/build.log`, a dev
build of the workspace after a change of `swell`, "Finished `dev` profile ... in 5.73s". Estimated
from the source sizes (`swell` 27,605 lines, `swell-syntax` 5,860, `swell-compiler` 3,922) and
not measured: a release build of `swell` and the two test binaries, cold 60 to 120 s, after a
change of `swell` 30 to 60 s, with nothing changed under 5 s; the same in `stack-nodebug`, cold
about 30 s, after a change of `swell` about 10 s. The four tests run in about 5 s per profile
(each a child process on a bounded stack, no bisection). The unit measures every one of these and the report
gives the numbers beside these estimates.

## Change

Failing tests first is not the form here (the tests exist); the planted script below is the red
run. Then (decision D6, option A):

1. **Two profiles** in `.cargo/config.toml`: `[profile.release]` gains `debug = false` and
   `strip = "debuginfo"`, so a user's configuration cannot change the release build of the gate;
   `[profile.stack-nodebug]` with `inherits = "dev"`, `debug = false`, `strip = "debuginfo"`, a
   dev build without debug information, built in `target/stack-nodebug/`.
2. **The decision on the dev profile with debug information off**, named: it is run, in
   `stack-nodebug`. Reason: the gate's dev build has full debug information (Context), so no
   build of the gate shows that profile, and whether a frame differs without it is a question a
   test answers, not this text.
3. **A task file** `taskfiles/stack-profiles.yml` with two tasks.
   `build-stack-profiles`, a build step after `build` and before `clippy`: `cargo nextest run
   --locked -p swell --cargo-profile <p> --test limits_scripts --test eval_stack --no-run` for
   `<p>` of `release` and `stack-nodebug`, with no `RUSTFLAGS` of its own (so the release build of
   `swell` is the one `taskfiles/test262.yml:72` reuses). `test-stack-profiles`, a test step
   after `test-faults` with its `gate-time` line: in each profile the four tests by name, `cargo
   nextest run --locked -p swell --cargo-profile <p> --test limits_scripts --test eval_stack -E
   'test(host_17_deepest_reentry_parses_at_full_nesting) | test(min_stack_bytes_is_enough_through_builtins)
   | test(host_152_deepest_reentry_evals_at_full_nesting) |
   test(host_163_deepest_reentry_calls_function_with_parameters_and_body_at_full_nesting)'`,
   printing its line of counts; then the check of point 4 on the two binaries of each profile.
4. **The check**, `taskfiles/no-debug-sections.sh <binary>...`: `readelf -S` of each; it fails
   when one has a section whose name starts with `.debug_`, printing `stack-profiles: <binary>
   has debug sections`. The task and the planted script call this one file.
5. **The planted script** `tests/gate/stack-profiles-refuses-a-binary-with-debug-sections.sh`: it copies the
   machine's `true` binary into `.out/gate-tests/` (no build), adds a `.debug_info` section with
   `objcopy --add-section`, runs the check on it, and passes when the check fails with the line
   of point 4, printing `stack-profiles-refuses-a-binary-with-debug-sections: ok`; and runs the check on the
   unchanged copy, which passes. Under 5 s.
6. **Measure**, in both profiles, the bytes per re-entry on the worst path by the method of
   `docs/plan/slice-02-run-loop-frame.md:22-24`. `STACK_BYTES_PER_REENTRY` is not changed by this
   story.

Design preconditions (the design author writes them before the unit starts):
`docs/design/host-interface.md`, "What `min_stack_bytes()` covers" (`:217`), gains: "It holds in
the dev profile with full debug information, in the dev profile without it, and in the release
profile without it, each shown by the stack tests in the gate; a host that builds swell with a
profile of its own runs the same tests in it." `AGENTS.md` ("Commands and the gate") gains the
two tasks; the coordinator writes it from the unit's report.

Writes: `.cargo/config.toml`, `taskfiles/stack-profiles.yml` (new), `taskfiles/no-debug-sections.sh`
(new), `tests/gate/stack-profiles-refuses-a-binary-with-debug-sections.sh` (new). Not written: any file under
`crates/`, the root `Taskfile.yml` (the coordinator's, from the report's lines).

## Acceptance

On the unit's branch. The tasks of `taskfiles/stack-profiles.yml` set `dir: '{{.TASKFILE_DIR}}/..'`,
so they run the same included or alone; the unit runs them alone (`slot task --taskfile
taskfiles/stack-profiles.yml <task>`), and shows the coordinator's lines valid on a copy,
`.out/Taskfile.yml`, of the root `Taskfile.yml` with the report's lines and its include paths
prefixed `../`: `task --taskfile .out/Taskfile.yml --list` names both tasks (the coordinator's
file is not written).

1. `build-stack-profiles` exits 0. Its wall time with nothing changed is at most 10 s,
   and after a one-line change of a comment in `crates/swell/src/lib.rs` at most 60 s; cold, the
   time is reported. Past either bound the unit stops before merging and reports the numbers with
   option B of D6 (the lane `slow`).
2. `test-stack-profiles` exits 0: eight tests passed (four per profile), each under
   60 s, the step at most 20 s; its `gate-time test-stack-profiles` line is printed; `readelf -S`
   of the four binaries lists no `.debug_` section (the report quotes the count, 0, and the dev
   binary's beside it).
3. `slot task gate-selftest` passes with the new planted script; the script was seen red (its
   line quoted) before `taskfiles/no-debug-sections.sh` existed.
4. The bytes per re-entry on the worst path are at most 14,336 in each profile; past that the
   unit stops before merging and reports the numbers (the constant is raised only by a measured
   decision, `AGENTS.md`, "Commands and the gate").
5. The step's time added to the sum of the last `gate-time test-steps <sum> of 120` line on the
   base (the gate unit's report gives it) is at most 120; past that the unit stops before merging
   and reports both numbers.

Commands, each one job of the build pool (one slot): the two tasks above, `slot task
gate-selftest`, `slot cargo clippy --workspace --all-targets --locked -- -D warnings`. No test262
run: no builtin changes. No `host-requirements-check`: no requirement line changes. Never the
whole gate (`slot task check`), `slot task test`, `slot task test-collect` or `cargo test -p
swell` whole.

Rules this unit keeps. A test is killed at 60 s and the gate's test steps fail over 120 s: the new
test step adds at most 20 s, and the two builds are a build step, outside the test steps. Each
test runs in a process of its own under cargo-nextest, and its subject in a child process of its
own. The planted script runs under 5 s. A job of the build pool asks for one slot; one over 10 minutes is killed, so `host-requirements-check` on a cold tree and the full test262 run are named `slot timeout 1800 <command>`. Nothing is
added to the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack tests are
unchanged. No third-party crate, no unsafe code. No network: `readelf` and `objcopy` are the
machine's. Every build compiles through the host's shared compile cache (no `RUSTC_WRAPPER`
set).

## Cost

Two builds of `swell` and its two stack test binaries per gate run: about 10 s and 60 s after a
change (estimates, Context), measured by the unit; 20 s at most of the 120 s of test steps.

## Report

The build times warm, after a change and cold, the test step's time, the `readelf` counts, the
bytes per re-entry in each profile, and the coordinator's lines:

- `Taskfile.yml` `includes:`: `stack-profiles: {taskfile: taskfiles/stack-profiles.yml, flatten:
  true, optional: true}`, written in the block form of the others.
- In `check`: `- task: build-stack-profiles` after `- task: build`; `- task: test-stack-profiles`
  and `- task: gate-time` with `vars: {STEP: test-stack-profiles}` after the `gate-time` of
  `test-faults`.
- `CHANGELOG.md`, "Changed": "The gate now shows `Limits::min_stack_bytes()` in the release
  profile and in the dev profile, each with debug information off, as well as in the dev profile
  with it (`HOST-17`, `HOST-152`, `HOST-163`)."

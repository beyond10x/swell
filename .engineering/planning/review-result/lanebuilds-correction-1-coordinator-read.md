---
format: aep.planning-md/3
id: review-result:lanebuilds-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit lanebuilds: no defect'
relations:
- reviews: story:realm-size-lane-within-its-share
revision: 1
---
Coordinator's read of correction 1 of the unit lanebuilds (impl/lanebuilds, d97404f to d80db027),
2026-11-27. Verdict: no defect.

The correction answers review-result:adversary-lanebuilds-pass-1 (one red case of three, one
warning and four notes, all introduced by the unit). It is three commits over 11 files, 437
lines added and 34 removed; no line of `crates/*/src`, no lock file, no outside crate.

Commit 40df7d3, the lint (F1):

- `crates/swell/Cargo.toml` gets `[lints.rust] unexpected_cfgs = { level = "warn", check-cfg =
  ["cfg(test_modules, values(any()))"] }`. The workspace has no `[workspace.lints]`, so this is
  the one place. The level is the lint's default; the gate's `-D warnings` makes it an error.
- `taskfiles/test-main.sh` writes no `expect` of the lint; `crates/swell/tests/main.rs` loses
  those four lines and nothing else.
- The adversary's case `build-refuses-a-misspelled-feature-in-a-test-module.sh` is green and
  holds two sides more: `test_module = "first"` (the cfg's name misspelled) is refused by the
  build, and the tree with both names right builds under `-D warnings`.

Commit 7de3c89, the stack tasks (F2, F3, point 6):

- `test-stack-profiles` runs `build-stack-profiles` as its first step with the same
  `STACK_MODULES` and `STACK_TESTS`; `slow` calls `test-stack-profiles` alone and its time line
  `stack-profiles` covers build and run as before. Measured by the correction on the node: a
  build of an unchanged tree takes 0.23 s for the two profiles. Read: cargo's fingerprints
  decide what is fresh, so no comparison of file times is needed, and the two-command form of
  story test-builds-carry-no-debug-information step 3 still passes (the second build compiles
  nothing).
- `test-stack-profiles-runs-no-stale-binary.sh` (new) holds it on a planted workspace: a
  source changed after the build is built again and its test fails; against the task file of
  d97404f the case is red ("ran the binary built before depth() changed").
- `stack-modules-check` refuses a test that `STACK_TESTS` names twice, with the count; one more
  step in the unit's selftest, red against the task file of d97404f.
- The adversary's case for a blank `STACK_TESTS` is committed unchanged.

Commit d80db02, the variables (F4, F5):

- `gate-vars`, the first step of `check` and of `slow`, fails when `RUSTFLAGS`,
  `CARGO_ENCODED_RUSTFLAGS`, `CARGO_BUILD_RUSTFLAGS` or a `CARGO_TARGET_<triple>_RUSTFLAGS` of
  the environment names `test_modules`, with one line per variable. Read: the awk reads `env`
  line by line, takes the name before the first `=`, and prints the separator of the encoded
  form as a blank; `build-stack-profiles` passes the cfg after `cargo rustc ... --`, so the
  lane's own build is not refused (the coordinator's run of `test-stack-profiles` below went
  through `gate-vars`' rule unharmed: the task does not run `gate-vars`, and the lane does
  before any build). Not covered, and said here: a `rustflags` key in a cargo configuration
  file. The repository's `.cargo/config.toml` is tracked, and a change of it shows in a diff.
- `taskfiles/test-main.sh` takes the packages as arguments after the mode (default `swell`)
  and reads no variable, so the gate's layout check and the forge's cannot differ by the
  environment. The adversary's case passes its package as the argument and holds one more
  step: `check` without the argument refuses the planted file with `TEST_MAIN_SELECT=planted`
  exported.

One line of `Taskfile.yml`'s summary of `slow` is longer than the file's width after the
change; the coordinator wraps it with the wording of the summaries the units left for it.
A comment of `taskfiles/stack-profiles.yml` names "correction 1 ... F2"; it stays, the record
is in the store under that name.

The coordinator's verification of d80db027 on the build node, 2026-11-27 00:29 CET: the whole
`swell::main` 1,308 passed and 1 skipped in the default mode and with
collect-every-safe-point (the skipped test is
`faults::host_71_injected_fault_at_each_call_faults_the_agent`, which is `#[ignore]`d
without the feature `fault-injection`, as on the base); the library tests 251 passed;
`sh taskfiles/test-main.sh check` exit 0; `task gate-selftest` 97 passed and 2 failed, the two
`site-data-check` cases that are red on the wave's base until `website/data/status.json` is
written again at the close; `task test-stack-profiles` exit 0 with 4 tests passed in each
profile (0 debug sections in the release binary, 8 in stack-debug); clippy and fmt exit 0; no
changed file.

Not measured: the seconds the two selftests that build a planted crate add to the gate's
selftest step on a cold target (warm on the node: 0.3 s and 1.7 s). The gate's own time sum
holds its budget at the wave's close. The forge projection of the lane stays 930 to 1,060 s
until the third unit (lanetests) is merged and the forge run is measured.

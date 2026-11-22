---
format: aep.planning-md/3
id: story:test-builds-carry-no-debug-information
kind: story
status: implemented
title: The dev and test builds carry no debug information, and the gate shows it with readelf on every test executable after the build
summary: .cargo/config.toml sets the dev profile's debug to false with strip = "debuginfo"; test-binaries-check, a step of check right after build, runs readelf -S on every test executable and fails on a .debug_ section; the stack tests keep a run with full debug information in a profile stack-debug, which replaces stack-nodebug.
relations:
- depends_on: story:stack-requirements-hold-in-the-release-profile
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T08:02:58Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T08:02:59Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:52Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Order, the same in both stories and the plan: `story:stack-requirements-hold-in-the-release-profile`
(accepted), then this story, then `story:one-integration-test-binary-per-crate`.

Waits for: `story:stack-requirements-hold-in-the-release-profile`: its `taskfiles/no-debug-sections.sh`,
its `taskfiles/stack-profiles.yml`, and its measurement of the bytes per re-entry in
`stack-nodebug` (the dev profile with debug off, the level this story makes the default), shown at
most 14,336 before the default changes. Through it, `story:local-gate-equals-forge`, which adds
`taskfiles/forge.yml` (its `RUNNER_FORM`, `:6`, and the `check-runner` summary this story edits).
Followed by `story:one-integration-test-binary-per-crate`, which extends `taskfiles/test-binaries.yml`.
Consumer: the organization's rule "Tests stay fast" ("no debug info in test builds ... prove it
with `readelf -S` on one test binary (no `.debug_*` sections)"). Goal story moved: none of the
browser's list; the rule's own story.

## Context

Line numbers are of `wave/s3-1` at `e077d7d`; the unit reads every count again at its base by the
command given.

- `.cargo/config.toml:1-2` is `[profile.dev]` / `debug = true`. The manifest's profiles set only
  `overflow-checks` (`Cargo.toml:10-20`). The test profile inherits dev, so every test executable
  of `cargo nextest run` and `cargo test --no-run` carries full DWARF.
- Measured on 2026-11-22 for the test binary `budgets`: 25,729,792 bytes with `debug = true`
  (19,981,428 of them `.debug_*`), 14,556,496 with `line-tables-only`, 5,448,224 with `false`.
  The number of test executables at the base: `jq -r 'select(.reason == "compiler-artifact" and
  .profile.test and .executable != null) | .executable' .out/test-binaries/build.json | wc -l`
  after the check's first run (step 1).
- Why it says `true` today: the forge builds the dev profile with cargo's default, full debug
  information, and a developer's own cargo home sets `line-tables-only`; the frames differ, and a
  stack test passed locally and aborted on the forge twice (`AGENTS.md:156-161`: 29,855 against
  31,314 bytes per re-entry, line tables only against full). That was the debug level, not another
  change. The repository's file wins over both, so `false` there is the same on both sides
  (`taskfiles/forge.yml:244-248` names this difference).
- `STACK_BYTES_PER_REENTRY` is 14 KiB (`crates/swell/src/host/limits.rs:46`), measured in the
  full-debug build (`docs/plan/slice-02-run-loop-frame.md:22-24`). Full debug has the largest
  frames of the levels measured so far; a lower level shrinks them, so the promise still holds,
  but the gate's dev run then catches a growing frame later than the 1,560 bytes
  `crates/swell/tests/support/host.rs:78-89` states. An embedder's own dev build is cargo's
  default, full debug: the promise must keep being shown there.
- The accepted stack story adds `[profile.stack-nodebug]` (dev, `debug = false`, `strip =
  "debuginfo"`) and runs the stack tests in it and in `release`. After this story the dev profile
  is that profile, so `stack-nodebug` would build the same thing twice.
- Backtraces: `line-tables-only` keeps a file and line per frame; `false` keeps function names
  (the symbol table stays with `strip = "debuginfo"`) and loses the lines. A failing assertion's
  own message keeps `tests/<file>.rs:<line>:<col>`: it comes from `Location`, compiled in, not
  from DWARF. No test reads a backtrace (`grep -rn backtrace crates/*/tests` finds none).
- The tests whose frames the level reaches: every test that runs its subject on a bounded stack
  in a child process, found by `grep -lE 'on_bounded_stack|on_small_stack|current_exe'
  crates/*/tests/*.rs` (the helper of `crates/swell/tests/support/stack.rs:79` and its callers,
  the copies of the helper in single files, and `crates/syntax/tests/sources.rs`).
- `.cargo/config.toml` is a source of the test262 digest (`taskfiles/test262.yml:383`). The run is
  a release build (`taskfiles/test262.yml:76`), so no run line can change. The lane `slow` watches
  the file (`.github/workflows/slow.yml:9,24`).

**Recommendation: `false`, with `strip = "debuginfo"` written out.** The rule says "no debug
info" and its proof is "no `.debug_*` sections". `line-tables-only` writes `.debug_line`,
`.debug_info` and `.debug_abbrev`, so it fails the proof by construction, and it is 2.7 times the
bytes of `false`. The strip is explicit so that the precompiled standard library's line tables
leave the binary whatever cargo's implicit default is.

## Change

The failing check first.

1. **The check.** A new task file `taskfiles/test-binaries.yml` with the task `test-binaries-check`
   (`dir: '{{.TASKFILE_DIR}}/..'`, so it runs the same included or alone). It reads the test
   executables from `GATE_TEST_BUILD_JSON`, a file of cargo's JSON messages. When that is not
   given, it runs `cargo test --locked --workspace --no-run --message-format=json` with
   `RUSTFLAGS: -D warnings` (the build step's flags, so it reuses the build step's artifacts and
   builds nothing after `build`) into `.out/test-binaries/build.json`. Executables are the
   messages with `reason == "compiler-artifact"`, `.profile.test` and `.executable != null` (jq).
   It runs `taskfiles/no-debug-sections.sh` on all of them and fails naming each one with a
   `.debug_` section. When it passes it prints `test-binaries-check: <n> test executables, none
   with a .debug_ section`. `nextest-check` accepts the cargo line because it has `--no-run`
   (`taskfiles/nextest.yml:125`).
2. **The planted script**
   `tests/gate/test-binaries-check-refuses-a-test-binary-with-debug-sections.sh` copies the
   machine's `true` into its directory, adds `.debug_info` with `objcopy --add-section`, writes one
   JSON message naming it as a test executable, runs the task with `GATE_TEST_BUILD_JSON` on that
   file, and passes when the task fails naming the copy. The same run with the unchanged copy
   passes. No build, under 5 s. It is seen red before step 1 exists.
3. **The level.** `.cargo/config.toml`: `[profile.dev]` `debug = false`, `strip = "debuginfo"`.
4. **The full-debug run of the stack tests stays.** In `.cargo/config.toml`,
   `[profile.stack-nodebug]` is replaced by `[profile.stack-debug]` (`inherits = "dev"`, `debug =
   true`, `strip = "none"`). In `taskfiles/stack-profiles.yml` every `stack-nodebug` becomes
   `stack-debug`, and its `readelf` check applies to the release binaries only (a stack-debug
   binary has debug sections by design; the task prints their count beside 0 for release). The
   promise is still shown in three builds: dev without debug (the gate's test step), dev with full
   debug (`stack-debug`), and release.
5. **Texts that state the old level.** `crates/swell/tests/support/host.rs:86-89`, the doc comment
   of `stack_with_room`: the build here is the forge's because both now read `false`, and the
   full-debug margin is shown in `stack-debug`. `taskfiles/forge.yml:244-248`, the bullet on debug
   information: `false`, and the stack tests' full-debug run in `stack-debug`.
6. **The digest.** `slot timeout 1800 task test262-expected`, in the background, writes
   `conformance/test262/expected.digest` again; `conformance/test262/expected.txt` shows no diff.

Coordinator's lines, from the report, written in the merge of this story:
- **Root `Taskfile.yml`:**
  - under `includes:`, `test-binaries: {taskfile: taskfiles/test-binaries.yml, flatten: true,
    optional: true}` in block form;
  - in `check`, `- task: test-binaries-check` right after `- task: build`;
  - `GATE_TEST_BUILD_JSON` in the `GATE_VARIABLES` of `gate-vars`.
- **`AGENTS.md`, "Commands and the gate":** the sentence of lines 156-161 becomes "The
  repository's `.cargo/config.toml` builds the dev and test profiles without debug information
  (`debug = false`, `strip = "debuginfo"`), on the forge and here alike; `test-binaries-check`
  shows it with `readelf -S` on every test executable. A stack bound is measured in the profile
  `stack-debug` (dev with full debug information, the largest frames, an embedder's default)."
- **`docs/design/host-interface.md`, "What `min_stack_bytes()` covers":** the sentence the stack
  story adds names `stack-debug` for the full-debug build.

A developer who wants a debugger for one command sets `CARGO_PROFILE_DEV_DEBUG=true` (an
environment variable wins over a config file) and pays one rebuild.

Shared file: `crates/swell/tests/support/host.rs`. The accepted
`story:a-kept-handle-does-not-keep-its-object-alive` adds a field of the test host there (its
`Writes:` line). This story edits only the doc comment of `stack_with_room` (`:78-89`). A branch
that meets a conflict there keeps both hunks: the other story's field and this story's comment.

Writes: `.cargo/config.toml`, `taskfiles/test-binaries.yml` (new),
`tests/gate/test-binaries-check-refuses-a-test-binary-with-debug-sections.sh` (new),
`taskfiles/stack-profiles.yml`, `crates/swell/tests/support/host.rs` (the doc comment of
`stack_with_room` only), `taskfiles/forge.yml` (the summary of `check-runner` only),
`conformance/test262/expected.digest`.
Not written:
- the root `Taskfile.yml`, `AGENTS.md` and the design document (the coordinator's, from the
  report);
- any file under `crates/*/src/`;
- `taskfiles/no-debug-sections.sh` (the stack story's; called, not changed).

## Acceptance

On the unit's branch. The task file is run alone (`task --taskfile taskfiles/test-binaries.yml
test-binaries-check`). The coordinator's lines are shown valid on a copy `.out/Taskfile.yml` of
the root file with its include paths prefixed `../`. A bound is measured by the unit itself,
before it reports, in its own pool job run under the runner's variables (`env CI=true
GITHUB_ACTIONS=true CARGO_TERM_COLOR=always CARGO_INCREMENTAL=0`, the `RUNNER_FORM` of
`taskfiles/forge.yml:6`). A missed bound: the unit stops before merging and reports the numbers.

1. **Red first.** Before step 3, `slot timeout 1800 task --taskfile taskfiles/test-binaries.yml
   test-binaries-check` exits 1 and names every test executable; the report quotes the count the
   command of Context prints at the base. The planted script was seen red before step 1 (its line
   quoted).
2. **Green, with its time and bytes.**
   - After step 3, the same command exits 0 with its line.
   - Its second run, with nothing changed, takes at most 10 s of wall time (zsh `time`, runner's
     variables).
   - `readelf -S` of the `budgets` test executable lists 0 `.debug_` sections.
   - The report gives that executable's bytes beside 25,729,792 and 5,448,224, and the bytes of
     all test executables summed (`du -cb`) before and after.
3. **The stack profiles.** `slot timeout 1800 task --taskfile taskfiles/stack-profiles.yml
   build-stack-profiles` and then `slot timeout 1800 task --taskfile taskfiles/stack-profiles.yml
   test-stack-profiles`, under the runner's variables, exit 0.
   - The build is within the stack story's bounds: at most 10 s with nothing changed, at most 60 s
     after a one-line comment change in `crates/swell/src/lib.rs`.
   - The tests: eight passed, each under 60 s, the step at most 20 s.
   - `readelf`: release binaries 0 `.debug_` sections, `stack-debug` binaries more than 0.
   - The bytes per re-entry on the worst path (`toJSON`) in `stack-debug`, by the method of
     `docs/plan/slice-02-run-loop-frame.md:22-24`, are at most 14,336; past that the unit stops
     and reports.
4. **The tests the debug level can reach pass in the dev profile, in both collection modes.**
   Every file the command of Context lists, by its own binary:
   - `slot timeout 1800 cargo nextest run --locked -p swell $(grep -lE
     'on_bounded_stack|on_small_stack|current_exe' crates/swell/tests/*.rs | sed -E
     's|.*/([^/]*)\.rs$|--test \1|')`, and the same with `--features collect-every-safe-point`;
   - `slot timeout 1800 cargo nextest run --locked -p swell-syntax --test sources`.

   Each exits 0, and each run takes at most 120 s of wall time under the runner's variables (the
   first build after the profile change is outside that figure: it is reported separately).
5. **The gate calls the check after the build.** `task --taskfile .out/Taskfile.yml check --dry
   --verbose` prints `task: "test-binaries-check" started` as the next task after `task: "build"
   started`, and `task --taskfile .out/Taskfile.yml gate-vars` names `GATE_TEST_BUILD_JSON` when
   it is set.
6. **The test262 files.** `conformance/test262/expected.txt` is unchanged after `task
   test262-expected` (`git diff --exit-code` on it exits 0), and `slot timeout 1800 task
   --taskfile taskfiles/test262.yml test262-check` exits 0 with the new digest.
7. **Planted scripts and clippy.** `slot timeout 1800 task gate-selftest` passes with the new
   script, and `slot timeout 1800 task clippy` exits 0.

Rules this unit keeps:
- **Test time.** A test is killed at 60 s (`.config/nextest.toml:2`) and the gate's test steps
  fail over 120 s. This story adds no test step: the check runs before `gate-time-start`, in at
  most 10 s.
- **Pool jobs.** A job of the build pool asks for one slot and is killed after 10 minutes unless
  it names its time, so every pool command above names it (`slot timeout 1800`, at most 3600).
  The first build after the profile change rebuilds every crate.
- **No whole-workspace test run.** The check's `cargo test --no-run` builds and runs nothing new.
- **Code.** No third-party crate, no unsafe code, nothing added to the frame of `Agent::execute`;
  `STACK_BYTES_PER_REENTRY` and the stack tests are unchanged.
- **Tools.** `readelf` and `objcopy` are the machine's; no network.

## Cost

One full rebuild per worktree when it lands (a profile change), then smaller and faster links:
about a fifth of the bytes per test executable (5.4 MB against 25.7 MB, measured for one). Gate:
the check adds at most 10 s, outside the test steps. `stack-debug` replaces `stack-nodebug`, so
the stack story's build step keeps its cost. For a branch merged after it, the only shared file
is `support/host.rs` (above); `expected.digest` is recomputed at each merge, as for any source
change.

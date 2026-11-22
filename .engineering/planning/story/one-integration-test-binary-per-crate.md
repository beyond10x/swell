---
format: aep.planning-md/3
id: story:one-integration-test-binary-per-crate
kind: story
status: active
title: Each crate has one integration-test binary, tests/main.rs, generated from the test files where they are now
summary: autotests = false and one [[test]] main per crate; tests/main.rs is generated from the directory and checked against it; mod support becomes use crate::support; a test that starts its binary again passes a typed full path; the gate names tests with a nextest filter, and the test list keeps today's names, so no line of requirements.txt changes.
relations:
- depends_on: story:test-builds-carry-no-debug-information
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T08:02:59Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T08:02:59Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
Order, the same in both stories and the plan: `story:stack-requirements-hold-in-the-release-profile`
(accepted), then `story:test-builds-carry-no-debug-information`, then this story.

Waits for:
- `story:test-builds-carry-no-debug-information`: it creates `taskfiles/test-binaries.yml`, which
  this story extends, and makes the measurements below those of the final debug level.
- Through it, `story:stack-requirements-hold-in-the-release-profile`: its
  `taskfiles/stack-profiles.yml` names `--test limits_scripts --test eval_stack`, and this story
  translates that.

Built as a unit of its own between two waves: after the units building when this story starts
have merged, and before the next wave opens. Only the coordinator's merges follow it, and the
coordinator commits the two lines of `docs/plan/unit-invariants.md` (below) in its merge.
Consumer: the organization's rule "Tests stay fast" ("one integration-test binary per crate
(`tests/main.rs` with modules)"). Goal story moved: none of the browser's list; the rule's own
story.

## Context

Line numbers are of `wave/s3-1` at `e077d7d`. Every count is read again at the unit's base by the
command given; none is written here as a number of a branch.

- **Test executables, test files, shared modules.**
  - Integration-test files that are binaries today: `ls crates/*/tests/*.rs`. Swell's share:
    `ls crates/swell/tests/*.rs | wc -l`.
  - Test executables, unit-test harnesses of libraries and binaries included: `wc -l <
    .out/host-requirements/targets.tsv` after `task host-requirements-test-list`.
  - Not binaries: `crates/swell/tests/support/` (`mod.rs`, `host.rs`, `script.rs`, `stack.rs`),
    and the fixture directories `crates/swell/tests/language/`, `probes/`,
    `crates/compiler/tests/sources/`, `crates/syntax/tests/sources/`,
    `crates/test262/tests/fixtures/`.
  - No manifest declares a `[[test]]`, `autotests` or `harness` (`crates/*/Cargo.toml`).
  - `conformance/ess` is a workspace of its own with one test file and is out of scope.
- **`mod support;` and inner attributes.**
  - The swell test files that declare `mod support;`: `grep -l '^mod support;'
    crates/swell/tests/*.rs`. In `crates/swell/tests/faults.rs:14-15` the declaration is under
    `#[cfg(feature = "fault-injection")]`; `source_scan.rs` has none.
  - `support/mod.rs:6-8` says "Each test file declares `mod support;`" and holds
    `#![allow(dead_code)]`.
  - No test file uses `crate::`, `#[path]`, `include!` or a crate-level inner attribute. The only
    inner attribute is `#![cfg(test)]`, which is valid on a module. No other crate's test files
    share a module.
- **Tests that start their own binary again** run the child with `<name> --exact` and the bare
  test name:
  - **The shared helper:** `crates/swell/tests/support/stack.rs:79,91-93` (`on_bounded_stack`,
    its callers found by `grep -n 'on_bounded_stack(' crates/swell/tests/*.rs`; one in a macro,
    `builtins_array_iteration.rs:981`, `stringify!($name)`).
  - **Own copies of the helper:** `budgets.rs:458-472`, `eval_stack.rs:38-52`,
    `builtins_function.rs:327-341`, `builtins_json.rs:510-524`, `fuzz.rs:2198-2214`, and
    `crates/syntax/tests/sources.rs:351-369` (called at `:386` and in the macro at `:610`).
  - **A name written out:** `determinism.rs:443-451`.

  In one binary a test's name is `<module>::<name>`. `--exact <name>` then matches nothing, the
  child runs no test and exits 0 without its "done" line, and the parent fails, loudly but only
  when it runs.
- **Environment variables keep working.** Tests that read `CARGO_BIN_EXE_swell-test262`,
  `CARGO_BIN_EXE_swell-unicode-gen`, `CARGO_MANIFEST_DIR` or `CARGO_TARGET_TMPDIR`
  (`crates/test262/tests/*.rs`, `crates/unicode-gen/tests/check.rs:22,41`,
  `crates/compiler/tests/compile.rs:66`, `crates/swell/tests/language.rs:87`) are unaffected:
  cargo sets these for every integration test of the package.
- **Where the gate names test binaries:**
  - `taskfiles/fault-injection.yml:26` (`--test faults`);
  - the root `Taskfile.yml:150-170`: `slow-tests` builds `binary_id(<pkg>::<stem>)` from each
    `crates/*/tests/<stem>.rs` that reads `SWELL_FULL_COUNTS`;
  - `taskfiles/host-requirements.yml:356-374`: the test list writes `<package>::<target>::<test
    path>`, the target being the file's stem;
  - `tests/gate/test-faults-refuses-a-crate-without-the-feature.sh:46-47,67-75`: it keeps the
    real `[package]`, drops targets and writes `tests/faults.rs`;
  - the stack story's `taskfiles/stack-profiles.yml` (`--test limits_scripts --test eval_stack`).

  `.config/nextest.toml` has no per-binary override. `crates/swell/tests/faults.rs:10,138` names
  `--test faults` in text.
- **The requirement lines.** `conformance/host/requirements.txt` names tests as
  `swell::<target>::...`, the target being a test file's stem, `lib` or `doc` (`grep ' test '
  conformance/host/requirements.txt`). The line form admits any number of `::` segments
  (`taskfiles/host-requirements.yml:177`).
- **Accepted stories not yet built** name their tests as `--test <binary>` and many add a file
  `crates/swell/tests/<stem>.rs` (`grep -l -- '--test ' .engineering/planning/story/*.md` in the
  store).

**The form: (A), the files stay where they are.**
- **Manifest:** `autotests = false` in `[package]`, and one `[[test]]` with `name = "main"` and
  `path = "tests/main.rs"`.
- **`tests/main.rs` is generated, never written by hand.**
  - A script, `taskfiles/test-main.sh`, writes it from the directory: a first line saying the file
    is generated, by `taskfiles/test-main.sh` (task `test-main`), and is not edited; then `mod
    <name>;` for every `tests/*.rs` but `main.rs`, and for `support` where `tests/support/mod.rs`
    exists, sorted bytewise (`LC_ALL=C sort`).
  - A crate root resolves `mod budgets;` to `tests/budgets.rs`, so no `#[path]` is needed.
  - Inside a file, `mod support;` would look for `tests/<stem>/support.rs`, so it becomes `use
    crate::support;`. A child module's `super::support::...` still resolves through it.
- **Why (B) lost.** (B) moves the files under `tests/main/`:
  - it renames every test file, and the in-flight branches and accepted stories' `Writes:` lines
    cite those paths (a branch that edits one meets a rename on merge);
  - it moves the fixture paths that tests and `taskfiles/forge.yml:259-263` name;
  - it leaves autodiscovery on, so a file a later unit adds under `tests/` becomes a second binary
    without anyone noticing.

  (A) keeps every path. Its own failure mode is a file not declared and so not compiled; it is
  closed by generating `main.rs` and checking it.

**The test list keeps today's names.** For a target named `main`, the test list writes
`<package>::<test path>`, so the module is the first segment: `swell::budgets::<test>` stays
`swell::budgets::<test>`. No line of `requirements.txt` changes, and no planted host-requirements
script changes (they read fixture lists). A module named `lib`, `doc` or like a binary target
would collide with those names, so the layout check refuses such names.

**How a test is named afterwards.**
- One file: `--test <stem>` of package `<pkg>` becomes `-p <pkg> --test main -E
  'test(/^<stem>::/)'`.
- Several files: `-E 'test(/^a::/) | test(/^b::/)'`. Anchored, because a bare `scripts::` also
  matches `limits_scripts::` and `builtins_scripts::`.
- The collect mode: the same with `--features collect-every-safe-point`.
- In nextest's output and in JUnit, the binary is `<pkg>::main` and the test `<stem>::<name>`.

## Change

The failing check first.

1. **The generator and the layout check.** `taskfiles/test-main.sh write` writes every crate's
   `tests/main.rs` under `GATE_CRATES` (default `crates`). `taskfiles/test-main.sh check`
   generates the same in memory and fails on any of these, each named with its file:
   - a `tests/main.rs` that differs from the generated one;
   - a crate with `tests/*.rs` whose `[package]` lacks `autotests = false`;
   - a crate that lacks exactly one `[[test]]` named `main`;
   - a module named `lib`, `doc` or a binary target of the package with `-` written `_`.

   It is shell and awk: it builds nothing and runs off the pool. Tasks in
   `taskfiles/test-binaries.yml`: `test-main` (write), and `test-binaries-check`, which first runs
   the layout check.
2. **The count.** `test-binaries-check` also fails, from the JSON it already reads, when a package
   has more than one executable of kind `test`, naming the package and its targets. It prints
   `test-binaries-check: <n> test executables, <k> integration-test binaries in <k> packages, none
   with a .debug_ section`.
3. **Planted scripts.** Each runs without a build, under 5 s, and is seen red before steps 1 and 2:
   - `tests/gate/test-binaries-check-refuses-a-second-test-binary.sh`: a JSON file with two `test`
     executables of one package;
   - `tests/gate/test-binaries-check-refuses-a-main-rs-that-is-not-generated.sh`: a planted
     `GATE_CRATES` tree with, in turn, a file missing from `main.rs`, a manifest without
     `autotests = false`, and a module named `lib`; each refused with its own line, and the
     generated tree accepted.
4. **Manifests and generated files.** The manifests of every crate with test files get the form
   above. `taskfiles/test-main.sh write` writes their `tests/main.rs`.
5. **`mod support;`.** In each swell file `mod support;` becomes `use crate::support;`; the `cfg`
   line above it in `faults.rs` stays. A file where the import is then unused (the compiler says
   so under `-D warnings`) drops it. `support/mod.rs:6-7` says instead: "Each test file is a module
   of `tests/main.rs` and reaches this one with `use crate::support;`."
6. **A bare name cannot be passed.** `support/stack.rs` gains a type `TestPath`, made only by the
   macro `test_path!("<name>")`, which builds it from the caller's `module_path!()` with the
   crate's segment cut off (`<module>::<name>`, with any inline module between).
   - `on_bounded_stack`, the own copies of `budgets`, `eval_stack`, `builtins_function`,
     `builtins_json` and `fuzz`, and the child of `determinism` take a `TestPath` and pass it after
     `--exact`. A call with a string literal no longer compiles ("expected `TestPath`, found
     `&str`"), and the doc comment of `TestPath` says what to write: `test_path!("<name>")`.
   - `crates/syntax/tests/sources.rs` gets the same type and macro of its own, for its helper and
     its two call sites.
7. **The gate's tasks.**
   - **`taskfiles/fault-injection.yml:26`:** `--test main -E 'test(/^faults::/)'`. Its summary,
     and the ignore text and doc line of `faults.rs:10,138`, say the same.
   - **Root `Taskfile.yml`, `slow-tests`:** each term becomes `binary_id(<pkg>::main) &
     test(/^<stem>::/)`, joined with `|`.
   - **`taskfiles/host-requirements.yml`:** the target `main` is written as nothing, and its
     summary says so. An empty TSV field collapses in `read`, a tab being white space to `IFS`:
     write a placeholder and drop it in `awk`.
   - **`taskfiles/stack-profiles.yml`:** `--test main` in both commands; the four `test(...)`
     terms are unchanged, since each still matches one test.
8. **The planted fault script** writes `tests/main.rs` with the generator and keeps the real
   manifest's `[[test]]` section beside `[package]` and `[lints]`. Its expected lines stay.
9. **The stack profiles, when they do not fit (decision D3, option a).** If item 7 of Acceptance
   misses one of its bounds, the unit moves the release and `stack-debug` runs of HOST-17, HOST-152
   and HOST-163 to the lane `slow`:
   - **Root `Taskfile.yml`:** `check` loses `- task: build-stack-profiles`, `- task:
     test-stack-profiles` and the `gate-time` of the latter; `slow` gains both, the second with
     `vars: {STEP: test-stack-profiles, LABEL: slow-time}`.
   - **`taskfiles/stack-profiles.yml`:** its summaries say the tasks run in the lane `slow`.
   - **Coordinator's line in the merge, `docs/releasing.md`:** "A tag is cut only on a commit whose
     lane `slow` run is green; that run shows `Limits::min_stack_bytes()` in the release profile
     (HOST-17, HOST-152, HOST-163)."

     The lane runs on every pull request and push to `main` that changes `crates/**`
     (`.github/workflows/slow.yml:6,21`), so the consumer's requirement (the stack promise shown in
     the release profile before a tag) still holds.
10. **The digest.** The manifests are sources (`taskfiles/test262.yml:383`): `slot timeout 1800
    task test262-expected` runs in the background, and `expected.txt` shows no diff.

**Shared files, named:**
- **`crates/syntax/tests/sources.rs`.** This story edits the helper and its call sites
  (`:351-369`, `:386`, `:610`). `story:arrow-functions`, `story:block-function-declarations` and
  `story:for-of-statement` write the same file (`nesting_combinations`, `stack_bound_part`). On a
  conflict there a branch keeps its own lines and the full-path form of every helper call
  (`test_path!(...)`); a call it adds with a bare name does not compile.
- **`crates/swell/tests/support/mod.rs`:** the sentence of step 5.
- **`crates/swell/tests/support/stack.rs`:** `TestPath`.
- **Every crate's `tests/main.rs`:** generated. On a merge conflict there, run
  `taskfiles/test-main.sh write`; never resolve it by hand.

**The rule for every later unit**, the two lines the coordinator commits to
`docs/plan/unit-invariants.md` in the merge of this story, before the next wave opens:

```
- A test file is a module of its crate's one test binary: after adding, removing or renaming a `crates/<c>/tests/<stem>.rs`, and on a merge conflict in a `tests/main.rs`, run `sh taskfiles/test-main.sh write` and then `sh taskfiles/test-main.sh check` (exit 0, off the pool) before you report; `tests/main.rs` is never edited by hand, and a file reaches the shared helpers with `use crate::support;`, never `mod support;`.
- A test is named by a filter, not a binary: a story's `--test <stem>` is `--test main -E 'test(/^<stem>::/)'` (several: `-E 'test(/^a::/) | test(/^b::/)'`), the same with `--features collect-every-safe-point`; a test that starts its own binary again passes `test_path!("<name>")`, never a bare name.
```

**The sentence for each brief** of a unit that adds or names tests: "This unit runs after
`story:one-integration-test-binary-per-crate`: read every `--test <stem>` of the story as `--test
main -E 'test(/^<stem>::/)'`, add `crates/<c>/tests/main.rs` (generated by `sh
taskfiles/test-main.sh write`) to the `Writes:` line for each crate where you add a test file,
and run `sh taskfiles/test-main.sh check` before you report."

Writes:
- **Manifests and generated files:** the `Cargo.toml` and `tests/main.rs` (new, generated) of
  every crate with test files (`ls -d crates/*/tests`).
- **Test files:**
  - the swell test files that declare `mod support;` (`grep -l '^mod support;'
    crates/swell/tests/*.rs`), on that line;
  - the helper copies and their call lines in the files Context names, and `faults.rs:10,138`;
  - `crates/swell/tests/support/stack.rs` and `crates/swell/tests/support/mod.rs` (the sentence);
  - `crates/syntax/tests/sources.rs` (the helper and its call sites).
- **Task files and scripts:** `taskfiles/test-main.sh` (new), `taskfiles/test-binaries.yml`,
  `taskfiles/fault-injection.yml`, `taskfiles/host-requirements.yml`,
  `taskfiles/stack-profiles.yml`.
- **Root `Taskfile.yml`**, assigned to this unit since it runs alone and no other unit writes the
  file meanwhile: task `slow-tests`, and with D3 (a) the tasks `check` and `slow`; the layout
  check runs inside `test-binaries-check`, so `check` gains no line.
- **Planted scripts:** `tests/gate/test-faults-refuses-a-crate-without-the-feature.sh`, and the two
  planted scripts of step 3 (new).
- **`conformance/test262/expected.digest`.**

Not written:
- `conformance/host/requirements.txt` (unchanged is the proof);
- any file under `crates/*/src/`;
- `docs/plan/` records of past waves, and `.engineering/`;
- `AGENTS.md`, `docs/plan/unit-invariants.md` and `docs/releasing.md` (the coordinator's, from the
  report).

## Acceptance

A bound is measured by the unit itself, before it reports, in its own pool job under the runner's
variables (`env CI=true GITHUB_ACTIONS=true CARGO_TERM_COLOR=always CARGO_INCREMENTAL=0`, the
`RUNNER_FORM` of `taskfiles/forge.yml:6`), on the base commit first and again after. A missed
bound: the unit stops before merging and reports the numbers.

1. **The base.** Before any change:
   - `slot timeout 1800 task host-requirements-test-list`, then copy
     `.out/host-requirements/tests.txt` to `.out/tests-before.txt`.
   - The count of test executables is `wc -l < .out/host-requirements/targets.tsv`; their bytes are
     `cut -f3 .out/host-requirements/targets.tsv | xargs du -cb | tail -n 1`.
   - Per package `<pkg>` with test files: `slot timeout 1800 env <runner's variables> cargo nextest
     run --locked -p <pkg> -E 'kind(test)'`, and for swell the same with `--features
     collect-every-safe-point`. Their passed and ignored counts are the base counts.
   - `slot timeout 1800 env <runner's variables> task test-collect` and `task test-faults`: their
     wall times are the base times.
   - Rebuild times, each `slot timeout 1800` with `RUSTFLAGS='-D warnings'` and zsh's `time`
     (wall, user, system): after `touch crates/swell/tests/support/mod.rs`, `cargo test --locked
     -p swell --no-run`; after `touch crates/swell/src/lib.rs`, `cargo test --locked --workspace
     --no-run`.
2. **Red first.** `test-binaries-check` on the base exits 1. It names each package with more than
   one integration-test binary, with the count `ls crates/<c>/tests/*.rs | wc -l` gives, and each
   crate without a generated `tests/main.rs`. The two planted scripts were seen red before steps 1
   and 2.
3. **Green.**
   - `sh taskfiles/test-main.sh check` exits 0 off the pool.
   - `slot timeout 1800 task --taskfile taskfiles/test-binaries.yml test-binaries-check` exits 0
     with its line: one integration-test binary per crate with tests.
   - Its second run, with nothing changed, takes at most 10 s.
   - The executables, their bytes and the rebuild times of item 1 are reported beside the base's.
4. **No test lost or renamed.** `slot timeout 1800 task host-requirements-test-list`, then `cmp
   .out/tests-before.txt .out/host-requirements/tests.txt` exits 0. `slot timeout 1800 task
   host-requirements-check` prints the same counts as at the base.
5. **Each package's tests pass**, one pool job each, never the whole workspace at once.
   - The commands: `slot timeout 1800 env <runner's variables> cargo nextest run --locked -p
     <pkg> --test main`, and for swell the same with `--features collect-every-safe-point`.
   - Each exits 0 with the passed and ignored counts of item 1's base command for that package and
     mode, and takes at most 120 s of wall time (the cold build reported apart).
   - `slot timeout 1800 env <runner's variables> task test-faults` prints `test-faults: 1 test
     binaries: 1 passed, 0 failed, 0 ignored`.
   - `task test-faults` and `task test-collect` each take at most their base time plus 10 s.
6. **The slow-tests filter.** `cargo nextest list --locked --workspace --message-format oneline -E
   "$(cat .out/slow-tests.filter)"` (the filter `slow-tests` writes, its first command run alone,
   `slot timeout 1800`) lists the same tests as the base's filter, a test `<pkg>::<stem> <t>` of
   the base being `<pkg>::main <stem>::<t>` after. `slow-tests` itself is not run by the unit: it
   is the lane `slow`.
7. **The stack profiles.** `slot timeout 1800 env <runner's variables> task --taskfile
   taskfiles/stack-profiles.yml build-stack-profiles`, then `test-stack-profiles` likewise.
   - The bounds, which are the stack story's: build at most 10 s with nothing changed and at most
     60 s after a one-line comment change in `crates/swell/src/lib.rs`; test step at most 20 s;
     eight tests passed.
   - These three numbers decide D3. All within: the tasks stay in `check`. One missed: step 9 is applied. The
     report shows
     `task --taskfile .out/Taskfile.yml check --dry --verbose` without them and `task --taskfile
     .out/Taskfile.yml slow --dry --verbose` with them, on a copy of the root file with its include
     paths prefixed `../`.
8. **The test262 files, planted scripts and clippy.**
   - `conformance/test262/expected.txt` is unchanged after `task test262-expected`, and `slot
     timeout 1800 task test262-check` exits 0.
   - `slot timeout 1800 task gate-selftest` and `slot timeout 1800 task clippy` exit 0.
9. **The report** gives the `<b> test binaries` line of each run of item 5 beside the base's. The
   coordinator's gate after the merge prints `gate-time test-steps <s> of 120, cpu <c>` beside the
   last gate before; it confirms the unit's numbers and decides nothing.

Rules this unit keeps:
- **Test time.** A test is killed at 60 s (`.config/nextest.toml:2`) and the gate's test steps fail
  over 120 s; no test is added or lengthened.
- **Pool jobs.** A job of the build pool asks for one slot and is killed after 10 minutes unless it
  names its time, so every pool command above names it (`slot timeout 1800`, at most 3600). The
  generator and the layout check build nothing and run off the pool.
- **The one whole-workspace command** is `cargo test --no-run` (item 1 and inside
  `host-requirements-test-list`), which runs no test, with its time named.
- **Code.** No third-party crate, no unsafe code, nothing added to the frame of `Agent::execute`;
  `STACK_BYTES_PER_REENTRY` and every stack test's bound are unchanged.

## Cost

- **Size.** The swell test files touched by one line each, the helpers, the manifests and seven
  generated files: a unit of a few hours, mostly pool time.
- **Gate.**
  - The test step links one integration-test binary per crate instead of one per file, and
    `test-collect` links one instead of swell's whole count (inside a timed step, the larger
    saving).
  - `test-faults` compiles the whole swell test crate with the feature instead of one file: the one
    step that may get slower, bounded in item 5.
  - The stack profiles build the whole test crate, bounded in item 7, with D3 (a) behind it.
- **Rebuilds.** A change of any test file recompiles its crate's whole test crate, incrementally
  where incremental compilation is on.
- **For a branch merged after this story,** the mechanical steps:
  - run `sh taskfiles/test-main.sh write` (a new file, or a conflict in `main.rs`);
  - write `use crate::support;` for `mod support;`;
  - wrap the name of each self-spawning call in `test_path!(...)` (the compiler names every one);
  - read `--test x --test y` in its commands as `--test main -E 'test(/^x::/) | test(/^y::/)'`;
  - on a conflict in `crates/syntax/tests/sources.rs`, keep the full-path helper calls;
  - then `cargo check --locked --workspace --all-targets` and `sh taskfiles/test-main.sh check` on
    the merged commit.

#!/bin/sh
# Planted override of the lane slow's runs at opt-level 1 (Taskfile.yml, tasks slow,
# test-opt1-vars-check and test-opt1-profile-check; story
# slow-lane-holds-its-bound-on-the-slower-runner, step 2): the runs of slow-tests and
# test-realm-sizes that moved to opt-level 1 take their arguments from TEST_OPT1_ARGS (the
# profile collect), the guard of their test processes from NO_STACK_TESTS and the test that must
# be in them from TEST_OPT1_PROFILE_TEST, as the step test's run does. go-task lets a value given
# on the command line win over the file's, which would run them at opt-level 0, without the
# guard or without the test that holds their profile, under the same lines; a profile setting
# the test does not see changes their profile the same way. The lane calls
# test-opt1-vars-check and test-opt1-profile-check with STEP slow right after gate-vars, before
# gate-time-start, slow-runner and the background builds.
# No test runs and nothing is built: the script plants a tree of its own
# (.out/gate-tests/<name>/run-<case>) with the repository's Taskfile.yml and taskfiles/, a test262
# pin whose URL does not exist (so that slow-test262 fails at once), a crate swell with a stack
# test, another test and the test TEST_OPT1_PROFILE_TEST names, and the stub cargo of
# tests/gate/fixtures/cargo-stub.sh first on PATH, which logs each build.
# 0. The harness: task slow without a caller's value passes both checks; it prints one line
#    "slow-runner: ..." and fails later, on the pin.
# 1. task slow TEST_OPT1_ARGS='--locked -p swell --profile default' (the runs at opt-level 1
#    without the profile collect): exits non-zero with "slow: TEST_OPT1_ARGS=--locked -p swell
#    --profile default is not the value Taskfile.yml defines", prints no slow-runner line, the
#    stub built nothing and .out/slow-collect does not exist.
# 2. task slow NO_STACK_TESTS=SWELL_NO_GUARD: the same with "slow: NO_STACK_TESTS=SWELL_NO_GUARD
#    is not the value Taskfile.yml defines".
# 3. task slow TEST_OPT1_PROFILE_TEST=deep::runs_on_any_stack: the same with "slow:
#    TEST_OPT1_PROFILE_TEST=deep::runs_on_any_stack is not the value Taskfile.yml defines".
# 4. CARGO_PROFILE_COLLECT_OPT_LEVEL=0 in the environment of task slow: the same with "slow:
#    CARGO_PROFILE_COLLECT_OPT_LEVEL is set in the environment".
# 5. A table [profile.collect] in Cargo.toml: the same with "slow: Cargo.toml:3:
#    [profile.collect]: the profile collect is defined in .cargo/collect-profile.toml only".
# Run from the repository root: sh tests/gate/slow-refuses-a-caller-value-of-its-runs-at-opt-level-1.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS SWELL_NO_GUARD SLOW_COLLECT STUB_BUILD STUB_BUILD_MATCH STUB_COMPILE STUB_COMPILE_MATCH STUB_COMPILE_LIST STUB_DROP STUB_DOUBLE STUB_FAIL
for v in $(env | sed -n 's/^\(CARGO_PROFILE_[A-Z0-9_]*\)=.*/\1/p'); do unset "$v"; done
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=slow-refuses-a-caller-value-of-its-runs-at-opt-level-1
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/bin" || { echo "$name: FAIL cannot make $dir"; exit 1; }
root=$(pwd)

fail() {
  echo "$name: FAIL $*"
  exit 1
}

[ -f Taskfile.yml ] && [ -f taskfiles/realm-room-tests.awk ] || fail "run it from the repository root"
cp tests/gate/fixtures/cargo-stub.sh "$dir/bin/cargo" && cp tests/gate/fixtures/nextest-filter.awk "$dir/bin/" && chmod +x "$dir/bin/cargo" ||
  fail "cannot copy the stub cargo of tests/gate/fixtures"

plant() {
  run="$dir/run-$1"
  mkdir -p "$run/crates/swell/tests/support" "$run/crates/swell/src" "$run/.out" "$run/.config" "$run/stub" || fail "cannot make $run"
  cp "$root/Taskfile.yml" "$run/Taskfile.yml" || fail "cannot copy Taskfile.yml"
  cp -R "$root/taskfiles" "$run/taskfiles" || fail "cannot copy taskfiles"
  cp "$root/.config/nextest.toml" "$run/.config/" || fail "cannot copy .config/nextest.toml"
  mkdir -p "$run/spec" "$run/conformance/test262" || fail "cannot make $run/spec"
  cp "$root/spec/ess-inputs.yaml" "$run/spec/" || fail "cannot copy spec/ess-inputs.yaml"
  awk '$1 == "url" { print "url file:///nonexistent/test262"; next } { print }' "$root/conformance/test262/PIN" > "$run/conformance/test262/PIN" ||
    fail "cannot write the planted PIN"
  printf '[package]\nname = "swell"\n' > "$run/crates/swell/Cargo.toml"
  printf '// a unit test reads SWELL_FULL_COUNTS\n' > "$run/crates/swell/src/lib.rs"
  printf 'mod deep;\nmod realm_padding;\nmod support;\n' > "$run/crates/swell/tests/main.rs"
  printf 'pub fn stack_with_room(bytes: usize) -> usize {\n    bytes / 8 * 7\n}\n' > "$run/crates/swell/tests/support/host.rs"
  printf '#[test]\nfn runs_on_the_promised_stack() {\n    let _ = stack_with_room(1 << 20);\n}\n\n#[test]\nfn runs_on_any_stack() {\n    assert_eq!(1 + 1, 2);\n}\n' > "$run/crates/swell/tests/deep.rs"
  printf '#[test]\nfn the_profile_keeps_debug_assertions_and_overflow_checks_on() {\n    assert!(cfg!(debug_assertions));\n}\n' > "$run/crates/swell/tests/realm_padding.rs"
  printf '%s\n' \
    'swell::main deep::runs_on_the_promised_stack' \
    'swell::main deep::runs_on_any_stack' \
    'swell::main realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on' > "$run/stub-tests"
  : > "$run/stub/log"
}

# go <case> <task arguments...>: run task slow in the planted tree of <case>.
go() {
  case_=$1
  shift
  STUB_DIR="$root/$dir/run-$case_/stub" STUB_LOG="$root/$dir/run-$case_/stub/log" STUB_TESTS="$root/$dir/run-$case_/stub-tests" \
    PATH="$root/$dir/bin:$PATH" task -d "$dir/run-$case_" slow "$@" > "$dir/$case_.out" 2>&1
}

# refused <case> <line>: task slow of <case> exited non-zero (checked by the caller), printed
# <line>, and started nothing after the checks.
refused() {
  grep -qF "$2" "$dir/$1.out" || fail "task slow did not print \"$2\" ($dir/$1.out)"
  ! grep -q '^slow-runner: ' "$dir/$1.out" || fail "task slow went on to slow-runner after the refusal ($dir/$1.out)"
  ! grep -q '^build ' "$dir/run-$1/stub/log" || fail "task slow built after the refusal ($dir/run-$1/stub/log)"
  [ ! -e "$dir/run-$1/.out/slow-collect" ] || fail "task slow started the background builds after the refusal ($dir/run-$1/.out/slow-collect)"
}

# 0. The harness.
plant plain
if go plain; then fail "task slow exited 0 with a test262 pin that cannot be fetched ($dir/plain.out)"; fi
[ "$(grep -c '^slow-runner: ' "$dir/plain.out")" = 1 ] || fail "the harness: task slow did not pass its checks and print one slow-runner line ($dir/plain.out)"
! grep -q 'is not the value\|is set in the environment' "$dir/plain.out" || fail "the harness: task slow refused a value without a caller's value ($dir/plain.out)"

# 1. TEST_OPT1_ARGS from the command line, without the profile collect.
plant args
if go args TEST_OPT1_ARGS='--locked -p swell --profile default'; then fail "task slow exited 0 with TEST_OPT1_ARGS given on the command line ($dir/args.out)"; fi
refused args 'slow: TEST_OPT1_ARGS=--locked -p swell --profile default is not the value Taskfile.yml defines'

# 2. NO_STACK_TESTS from the command line, naming another variable.
plant guard
if go guard NO_STACK_TESTS=SWELL_NO_GUARD; then fail "task slow exited 0 with NO_STACK_TESTS given on the command line ($dir/guard.out)"; fi
refused guard 'slow: NO_STACK_TESTS=SWELL_NO_GUARD is not the value Taskfile.yml defines'

# 3. TEST_OPT1_PROFILE_TEST from the command line.
plant keep
if go keep TEST_OPT1_PROFILE_TEST=deep::runs_on_any_stack; then fail "task slow exited 0 with TEST_OPT1_PROFILE_TEST given on the command line ($dir/keep.out)"; fi
refused keep 'slow: TEST_OPT1_PROFILE_TEST=deep::runs_on_any_stack is not the value Taskfile.yml defines'

# 4. The profile collect changed through the environment.
plant env
if CARGO_PROFILE_COLLECT_OPT_LEVEL=0 go env; then fail "task slow exited 0 with CARGO_PROFILE_COLLECT_OPT_LEVEL=0 in its environment ($dir/env.out)"; fi
refused env 'slow: CARGO_PROFILE_COLLECT_OPT_LEVEL is set in the environment'

# 5. The profile collect defined in Cargo.toml.
plant table
printf '[workspace]\n\n[profile.collect]\nopt-level = 0\n' > "$dir/run-table/Cargo.toml"
if go table; then fail "task slow exited 0 with a table [profile.collect] in Cargo.toml ($dir/table.out)"; fi
refused table 'slow: Cargo.toml:3: [profile.collect]: the profile collect is defined in .cargo/collect-profile.toml only'

echo "$name: ok"

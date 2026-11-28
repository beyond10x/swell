#!/bin/sh
# Planted defect of the lane slow's runs at opt-level 1 (Taskfile.yml, tasks slow-tests and
# test-realm-sizes; story slow-lane-holds-its-bound-on-the-slower-runner, step 2): each run that
# moved to opt-level 1 reads a build made before it (the lane's background builds of
# slow-collect-start, slow-tests' own builds run alone, build-realm-sizes), with
# SWELL_FULL_COUNTS=1 where the library's unit tests are compiled, so a cargo Compiling line in
# its output means it read another build: cargo built it again inside the run, maybe without the
# variable. Such a run fails, naming the log.
# No test runs: the script plants a tree of its own (.out/gate-tests/<name>/run-<case>) with the
# repository's Taskfile.yml and taskfiles/, a crate swell whose library and tests/deep.rs read
# SWELL_FULL_COUNTS (deep.rs: a stack test and another), whose tests/realm_padding.rs holds the
# test TEST_OPT1_PROFILE_TEST names and a test that reads the room on a bounded stack, and the
# stub cargo of tests/gate/fixtures/cargo-stub.sh first on PATH; STUB_COMPILE=<profile> makes a
# run in that profile whose arguments hold the word STUB_COMPILE_MATCH print a Compiling line
# (swell-num: only the runs at opt-level 1 of the default modes name that package).
# 1. The default mode of slow-tests at opt-level 1 compiles: task slow-tests fails with
#    "slow-tests opt-level 1: compiled 1 crate(s), which ... builds before the run: swell
#    (.out/slow-tests-opt1.log)" and runs nothing of the collect mode.
# 2. Its part at opt-level 0 compiles (the run of --workspace in the test profile): the same with
#    "slow-tests opt-level 0: compiled 1 crate(s)".
# 3. The library's run of the collect mode compiles (--lib): task slow-tests fails with
#    "slow-tests collect-every-safe-point library: compiled 1 crate(s)".
# 4. The default mode of test-realm-sizes at opt-level 1 compiles (--test main in the profile
#    collect without the feature): task test-realm-sizes REALM_SIZES=4 fails with "realm-sizes
#    4 default at opt-level 1: compiled 1 crate(s), which the task build-realm-sizes builds
#    before the runs" and "realm-sizes: 1 run(s) red".
# 5. The list of slow-tests' part at opt-level 1 compiles (STUB_COMPILE_LIST=collect): task
#    slow-tests fails with "slow-tests: the list compiled 1 crate(s), which ... builds before
#    it: swell (.out/slow-tests-list-opt1.txt.log)" and runs no test.
# Run from the repository root: sh tests/gate/slow-refuses-a-moved-run-that-compiles.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS SLOW_COLLECT REALM_SIZES REALM_COLLECT STUB_BUILD STUB_BUILD_MATCH STUB_COMPILE STUB_COMPILE_MATCH STUB_COMPILE_LIST STUB_DROP STUB_DOUBLE STUB_FAIL
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=slow-refuses-a-moved-run-that-compiles
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
  mkdir -p "$run/crates/swell/tests/support" "$run/crates/swell/src" "$run/.out" "$run/.config" || fail "cannot make $run"
  cp "$root/Taskfile.yml" "$run/Taskfile.yml" || fail "cannot copy Taskfile.yml"
  cp -R "$root/taskfiles" "$run/taskfiles" || fail "cannot copy taskfiles"
  cp "$root/.config/nextest.toml" "$run/.config/" || fail "cannot copy .config/nextest.toml"
  # Files the task files read when they load (taskfiles/spec.yml, taskfiles/test262*.yml).
  mkdir -p "$run/spec" "$run/conformance/test262" || fail "cannot make $run/spec"
  cp "$root/spec/ess-inputs.yaml" "$run/spec/" || fail "cannot copy spec/ess-inputs.yaml"
  cp "$root/conformance/test262/PIN" "$run/conformance/test262/" || fail "cannot copy conformance/test262/PIN"
  printf '[package]\nname = "swell"\n' > "$run/crates/swell/Cargo.toml"
  printf '// a unit test reads SWELL_FULL_COUNTS\n' > "$run/crates/swell/src/lib.rs"
  printf 'mod deep;\nmod realm_padding;\nmod support;\n' > "$run/crates/swell/tests/main.rs"
  printf 'pub fn stack_with_room(bytes: usize) -> usize {\n    bytes / 8 * 7\n}\n' > "$run/crates/swell/tests/support/host.rs"
  printf '%s\n' \
    'fn count() -> u32 {' \
    '    if std::env::var("SWELL_FULL_COUNTS").is_ok() { 1000 } else { 10 }' \
    '}' \
    '' \
    '#[test]' \
    'fn runs_on_the_promised_stack() {' \
    '    let _ = stack_with_room(1 << 20);' \
    '}' \
    '' \
    '#[test]' \
    'fn runs_on_any_stack() {' \
    '    assert!(count() > 0);' \
    '}' > "$run/crates/swell/tests/deep.rs"
  printf '%s\n' \
    '#[test]' \
    'fn the_profile_keeps_debug_assertions_and_overflow_checks_on() {' \
    '    assert!(cfg!(debug_assertions));' \
    '}' \
    '' \
    '#[test]' \
    'fn reads_the_room_on_a_bounded_stack() {' \
    '    let _ = stack_with_room(1 << 20);' \
    '    let _ = agent.heap_bytes();' \
    '}' > "$run/crates/swell/tests/realm_padding.rs"
  printf '%s\n' \
    'swell unit::reads_full_counts' \
    'swell::main deep::runs_on_the_promised_stack' \
    'swell::main deep::runs_on_any_stack' \
    'swell::main realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on' \
    'swell::main realm_padding::reads_the_room_on_a_bounded_stack' > "$run/stub-tests"
}

# go <case> <STUB_COMPILE> <STUB_COMPILE_MATCH> <task arguments...>: run task in the planted tree of <case>.
go() {
  case_=$1 compile=$2 match=$3
  shift 3
  plant "$case_"
  : > "$dir/stub-$case_.log"
  STUB_COMPILE=$compile STUB_COMPILE_MATCH=$match STUB_TESTS="$root/$dir/run-$case_/stub-tests" STUB_LOG="$root/$dir/stub-$case_.log" \
    PATH="$root/$dir/bin:$PATH" task -d "$dir/run-$case_" "$@" > "$dir/$case_.out" 2>&1
}

# 0. Nothing compiles: both tasks pass.
go clean '' '' slow-tests || fail "task slow-tests exited non-zero with no Compiling line ($dir/clean.out)"
go clean '' '' test-realm-sizes REALM_SIZES=4 || fail "task test-realm-sizes exited non-zero with no Compiling line ($dir/clean.out)"

# 1. The default mode at opt-level 1 compiles.
if go opt1 collect swell-num slow-tests; then fail "task slow-tests exited 0 with a Compiling line in its run at opt-level 1 ($dir/opt1.out)"; fi
grep -q '^slow-tests opt-level 1: compiled 1 crate(s), which .* builds before the run: swell (\.out/slow-tests-opt1\.log)$' "$dir/opt1.out" ||
  fail "task slow-tests did not name the Compiling line of .out/slow-tests-opt1.log ($dir/opt1.out)"
! grep -q '^run .*features=collect-every-safe-point' "$dir/stub-opt1.log" || fail "task slow-tests ran the collect mode after a run at opt-level 1 that compiled ($dir/stub-opt1.log)"

# 2. The default mode at opt-level 0 compiles.
if go opt0 test --workspace slow-tests; then fail "task slow-tests exited 0 with a Compiling line in its run at opt-level 0 ($dir/opt0.out)"; fi
grep -q '^slow-tests opt-level 0: compiled 1 crate(s), which .* builds before the run: swell (\.out/slow-tests\.log)$' "$dir/opt0.out" ||
  fail "task slow-tests did not name the Compiling line of .out/slow-tests.log ($dir/opt0.out)"

# 3. The library's run of the collect mode compiles.
if go lib collect --lib slow-tests; then fail "task slow-tests exited 0 with a Compiling line in the library's run ($dir/lib.out)"; fi
grep -q '^slow-tests collect-every-safe-point library: compiled 1 crate(s), which .* builds before the run: swell (\.out/slow-tests-collect-lib\.log)$' "$dir/lib.out" ||
  fail "task slow-tests did not name the Compiling line of .out/slow-tests-collect-lib.log ($dir/lib.out)"

# 4. The default mode of test-realm-sizes at opt-level 1 compiles.
if go realm collect swell-num test-realm-sizes REALM_SIZES=4 REALM_COLLECT=all; then fail "task test-realm-sizes exited 0 with a Compiling line in its run at opt-level 1 ($dir/realm.out)"; fi
grep -qxF 'realm-sizes 4 default at opt-level 1: compiled 1 crate(s), which the task build-realm-sizes builds before the runs' "$dir/realm.out" ||
  fail "task test-realm-sizes did not name the Compiling line of its run at opt-level 1 ($dir/realm.out)"
grep -qxF 'realm-sizes: 1 run(s) red' "$dir/realm.out" || fail "task test-realm-sizes did not count one red run ($dir/realm.out)"

# 5. The list of the part at opt-level 1 compiles: task test-split fails before any run.
if STUB_COMPILE_LIST=collect go list '' '' slow-tests; then fail "task slow-tests exited 0 with a Compiling line in the list of its part at opt-level 1 ($dir/list.out)"; fi
grep -q '^slow-tests: the list compiled 1 crate(s), which .* builds before it: swell (\.out/slow-tests-list-opt1\.txt\.log)$' "$dir/list.out" ||
  fail "task slow-tests did not name the Compiling line of .out/slow-tests-list-opt1.txt.log ($dir/list.out)"
! grep -q '^run ' "$dir/stub-list.log" || fail "task slow-tests ran tests after a list that compiled ($dir/stub-list.log)"

echo "$name: ok"

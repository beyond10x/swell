#!/bin/sh
# Planted caller values of the task test-split (Taskfile.yml; story
# slow-lane-holds-its-bound-on-the-slower-runner, step 2): test-split reads SPLIT_RUN,
# SPLIT_ARGS, SPLIT_EXCLUDE and the other SPLIT_ variables, which only the lane's tasks
# slow-tests and test-realm-sizes are meant to pass. GATE_VARIABLES does not list them and no
# check refuses them, and task takes a variable from the command line or the environment, so a
# caller of the gate's step test (task check, task test) or of the lane can hand test-split a
# list of tests to leave out of both parts. Each such test runs in neither part while the
# step's check of "every test runs once" reads a list that leaves it out too.
# No test runs and nothing is built: the script plants a tree of its own
# (.out/gate-tests/<name>/run) with the repository's Taskfile.yml and taskfiles/, a crate
# swell whose library and tests/deep.rs read SWELL_FULL_COUNTS (deep.rs: a stack test and
# another), the test TEST_OPT1_PROFILE_TEST names, a crate swell-syntax with one test, the
# tests TEST_OPT0_TESTS names, and the stub cargo of tests/gate/fixtures/cargo-stub.sh first
# on PATH, which answers each list and run from its filters and logs each test it ran.
# 0. task test as the gate calls it, and task slow-tests as the lane calls it: each exits 0 and
#    ran swell::main deep::on_any_stack once in the two parts of its split run.
# 1. task test SPLIT_RUN=test SPLIT_ARGS=<TEST_NEXTEST_ARGS> SPLIT_EXCLUDE=.out/skip.txt
#    (skip.txt: "other deep::on_any_stack"): must exit non-zero or run that test.
# 2. The same three variables from the environment of task test: the same.
# 3. SPLIT_EXCLUDE=.out/skip.txt in the environment of task slow-tests (the lane's part, whose
#    nested call of test-split passes no SPLIT_EXCLUDE): must exit non-zero or run that test.
# Run from the repository root: sh tests/gate/test-split-takes-no-caller-value-of-a-split.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS SLOW_COLLECT STUB_BUILD STUB_BUILD_MATCH STUB_COMPILE STUB_DROP STUB_DOUBLE STUB_FAIL
unset SPLIT_RUN SPLIT_ARGS SPLIT_EXCLUDE SPLIT_FILTER SPLIT_OPT1_TARGETS SPLIT_FULL_COUNTS SPLIT_BUILT_BY
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-split-takes-no-caller-value-of-a-split
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
  mkdir -p "$run/crates/swell/tests/support" "$run/crates/swell/src" "$run/crates/syntax/tests" "$run/.out" "$run/.config" || fail "cannot make $run"
  cp "$root/Taskfile.yml" "$run/Taskfile.yml" || fail "cannot copy Taskfile.yml"
  cp -R "$root/taskfiles" "$run/taskfiles" || fail "cannot copy taskfiles"
  cp "$root/.config/nextest.toml" "$run/.config/" || fail "cannot copy .config/nextest.toml"
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
    'fn on_the_promised_stack() {' \
    '    let _ = stack_with_room(1 << 20);' \
    '}' \
    '' \
    '#[test]' \
    'fn on_any_stack() {' \
    '    assert!(count() > 0);' \
    '}' > "$run/crates/swell/tests/deep.rs"
  printf '#[test]\nfn the_profile_keeps_debug_assertions_and_overflow_checks_on() {\n    assert!(cfg!(debug_assertions));\n}\n' > "$run/crates/swell/tests/realm_padding.rs"
  printf '[package]\nname = "swell-syntax"\n' > "$run/crates/syntax/Cargo.toml"
  printf '#[test]\nfn parses() {\n    assert!(true);\n}\n' > "$run/crates/syntax/tests/nest.rs"
  {
    printf '%s\n' \
      'swell unit::reads_full_counts' \
      'swell::main deep::on_the_promised_stack' \
      'swell::main deep::on_any_stack' \
      'swell::main realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on' \
      'swell-syntax::main nest::parses'
    sed -n 's/^  TEST_OPT0_TESTS: //p' "$root/Taskfile.yml" | tr ' ' '\n' | paste -d ' ' - -
  } > "$run/stub-tests"
  printf 'other deep::on_any_stack\n' > "$run/.out/skip.txt"
}

# go <case> <task arguments...>: run task in the planted tree of <case>; the caller's
# environment reaches task.
go() {
  case_=$1
  shift
  : > "$dir/stub-$case_.log"
  STUB_TESTS="$root/$dir/run-$case_/stub-tests" STUB_LOG="$root/$dir/stub-$case_.log" PATH="$root/$dir/bin:$PATH" task -d "$dir/run-$case_" "$@" > "$dir/$case_.out" 2>&1
}

# ran <case> [<run>]: how many times the stub ran swell::main deep::on_any_stack in the two
# parts of the split run <run> (default test: .out/<run>-opt1-nextest.toml and -opt0-).
ran() {
  awk -v a="config=${2:-test}-opt1-nextest.toml " -v b="config=${2:-test}-opt0-nextest.toml " '
    /^run / { on = (index($0, a) || index($0, b)) }
    on && $0 == "ran " $2 " swell::main deep::on_any_stack" { n++ }
    END { print n + 0 }
  ' "$dir/stub-$1.log"
}

args=$(sed -n 's/^  TEST_NEXTEST_ARGS: //p' taskfiles/realm-places.yml)
[ -n "$args" ] || fail "taskfiles/realm-places.yml defines no TEST_NEXTEST_ARGS"

# 0. The gate's call.
plant plain
go plain test || fail "task test exited non-zero without a caller's value ($dir/plain.out)"
[ "$(ran plain)" = 1 ] || fail "the harness: task test ran deep::on_any_stack $(ran plain) time(s), not once ($dir/stub-plain.log)"
plant laneplain
go laneplain slow-tests || fail "task slow-tests exited non-zero without a caller's value ($dir/laneplain.out)"
[ "$(ran laneplain slow-tests)" = 1 ] || fail "the harness: task slow-tests ran deep::on_any_stack $(ran laneplain slow-tests) time(s) in its default mode, not once ($dir/stub-laneplain.log)"

bad=""
# 1. SPLIT_ variables on the command line of the gate's step test.
plant cli
if go cli test SPLIT_RUN=test SPLIT_ARGS="$args" SPLIT_EXCLUDE=.out/skip.txt && [ "$(ran cli)" = 0 ]; then
  bad="$bad
  1. task test SPLIT_RUN=test SPLIT_ARGS='$args' SPLIT_EXCLUDE=.out/skip.txt exited 0 and never ran swell::main deep::on_any_stack: $(grep '^[0-9]* test binaries' "$dir/cli.out") ($dir/cli.out)"
fi

# 2. The same from the environment.
plant env
if SPLIT_RUN=test SPLIT_ARGS="$args" SPLIT_EXCLUDE=.out/skip.txt go env test && [ "$(ran env)" = 0 ]; then
  bad="$bad
  2. SPLIT_RUN, SPLIT_ARGS and SPLIT_EXCLUDE in the environment of task test: exit 0, swell::main deep::on_any_stack never ran ($dir/env.out)"
fi

# 3. SPLIT_EXCLUDE in the environment of the lane's part slow-tests.
plant lane
if SPLIT_EXCLUDE=.out/skip.txt go lane slow-tests && [ "$(ran lane slow-tests)" = 0 ]; then
  bad="$bad
  3. SPLIT_EXCLUDE in the environment of task slow-tests: exit 0, swell::main deep::on_any_stack ran in neither part of its default mode:$(grep '^slow-tests: [0-9]* tests at' "$dir/lane.out") ($dir/lane.out)"
fi

[ -z "$bad" ] || fail "a caller's SPLIT_ value left a test out of every run, and the step passed:$bad"
echo "$name: ok"

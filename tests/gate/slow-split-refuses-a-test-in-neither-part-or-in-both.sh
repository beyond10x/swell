#!/bin/sh
# Planted defects of the split of the lane slow's runs at opt-level 1 (Taskfile.yml, tasks
# test-split, slow-tests and test-realm-sizes; story
# slow-lane-holds-its-bound-on-the-slower-runner, step 2): slow-tests splits its default mode and
# test-realm-sizes its default mode into a part at opt-level 1 and one at opt-level 0, each
# through task test-split with the run's filter, which takes cargo nextest list of the run and of
# each part and fails, naming the test, when a test of the run is in not exactly one part.
# No test runs: the script plants a tree of its own (.out/gate-tests/<name>/run-<case>) with the
# repository's Taskfile.yml and taskfiles/, a crate swell whose library and tests/deep.rs read
# SWELL_FULL_COUNTS (deep.rs: a stack test and another), whose tests/realm_padding.rs holds the
# test TEST_OPT1_PROFILE_TEST names, a test that reads the room and one that reads the room on a
# bounded stack, a crate swell-syntax whose tests/nest.rs reads SWELL_FULL_COUNTS, and the stub
# cargo of tests/gate/fixtures/cargo-stub.sh first on PATH, which answers each list and run from
# its filters over those tests; STUB_DROP leaves a test out of each list in the profile collect,
# STUB_DOUBLE adds one to it.
# 0. The split as it is: task slow-tests exits 0 and prints "slow-tests: swell::main 2 at
#    opt-level 1, 1 at opt-level 0" and "slow-tests: 3 tests at opt-level 1, 2 at opt-level 0"
#    (the library's test, the other test of deep and the profile test at opt-level 1, the stack
#    test and the test of swell-syntax at opt-level 0); task test-realm-sizes REALM_SIZES=4
#    exits 0 and prints "realm-sizes: 2 tests at opt-level 1, 1 at opt-level 0" (the profile
#    test and the room test, and the room test on a bounded stack) and "realm-sizes 4 default:
#    2 tests at opt-level 1, 1 at opt-level 0". Every list of slow-tests ran with
#    SWELL_FULL_COUNTS=1, and the part at opt-level 1 of test-realm-sizes with the guard
#    SWELL_NO_STACK_TESTS=1.
# 1. A test in neither part: the list of slow-tests at opt-level 1 drops deep::runs_on_any_stack.
#    task slow-tests fails, naming it "in neither run", and the stub ran no test.
# 2. A test in both parts: the list of test-realm-sizes at opt-level 1 adds the room test on a
#    bounded stack. task test-realm-sizes fails, naming it "in both runs", and the stub ran no
#    test.
# 3. (run after 0) An exclusion list with no test of kind "other": task test-realm-sizes
#    REALM_SIZES=4 exits 0, runs 2 tests at opt-level 1 and 1 at opt-level 0, and
#    .out/realm-sizes-all-nextest.toml holds "default-filter = 'all()'", not a filter with an
#    empty term.
# Run from the repository root: sh tests/gate/slow-split-refuses-a-test-in-neither-part-or-in-both.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS SLOW_COLLECT REALM_SIZES REALM_COLLECT STUB_BUILD STUB_BUILD_MATCH STUB_COMPILE STUB_DROP STUB_DOUBLE STUB_FAIL
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=slow-split-refuses-a-test-in-neither-part-or-in-both
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
    'fn reads_the_room() {' \
    '    let _ = agent.heap_bytes();' \
    '}' \
    '' \
    '#[test]' \
    'fn reads_the_room_on_a_bounded_stack() {' \
    '    let _ = stack_with_room(1 << 20);' \
    '    let _ = agent.heap_bytes();' \
    '}' > "$run/crates/swell/tests/realm_padding.rs"
  printf '[package]\nname = "swell-syntax"\n' > "$run/crates/syntax/Cargo.toml"
  printf '#[test]\nfn parses_deep() {\n    let _ = std::env::var("SWELL_FULL_COUNTS");\n}\n' > "$run/crates/syntax/tests/nest.rs"
  printf '%s\n' \
    'swell unit::reads_full_counts' \
    'swell::main deep::runs_on_the_promised_stack' \
    'swell::main deep::runs_on_any_stack' \
    'swell::main realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on' \
    'swell::main realm_padding::reads_the_room' \
    'swell::main realm_padding::reads_the_room_on_a_bounded_stack' \
    'swell-syntax::main nest::parses_deep' > "$run/stub-tests"
}

# go <case> <task arguments...>: run task in the planted tree of <case>.
go() {
  case_=$1
  shift
  : > "$dir/stub-$case_.log"
  STUB_TESTS="$root/$dir/run-$case_/stub-tests" STUB_LOG="$root/$dir/stub-$case_.log" PATH="$root/$dir/bin:$PATH" task -d "$dir/run-$case_" "$@" > "$dir/$case_.out" 2>&1
}

# 0. The split as it is.
plant ok
go ok slow-tests || fail "task slow-tests exited non-zero on a split that runs each test once ($dir/ok.out)"
for line in 'slow-tests: swell::main 2 at opt-level 1, 1 at opt-level 0' 'slow-tests: 3 tests at opt-level 1, 2 at opt-level 0'; do
  grep -qxF "$line" "$dir/ok.out" || fail "task slow-tests did not print \"$line\" ($dir/ok.out)"
done
# Every list of slow-tests runs with SWELL_FULL_COUNTS=1 (SPLIT_FULL_COUNTS=1 on its call of
# test-split): the library's unit tests read it when they are compiled, so a list without it
# would not find the build and compile.
[ "$(grep -c '^list ' "$dir/stub-ok.log")" -ge 3 ] || fail "the stub saw fewer than three lists of slow-tests ($dir/stub-ok.log)"
! grep '^list ' "$dir/stub-ok.log" | grep -vq ' full=\[1\] ' || fail "a list of slow-tests ran without SWELL_FULL_COUNTS=1 ($dir/stub-ok.log)"
go ok test-realm-sizes REALM_SIZES=4 || fail "task test-realm-sizes exited non-zero on a split that runs each test once ($dir/ok.out)"
for line in 'realm-sizes: swell::main 2 at opt-level 1, 1 at opt-level 0' 'realm-sizes: 2 tests at opt-level 1, 1 at opt-level 0' 'realm-sizes 4 default: 2 tests at opt-level 1, 1 at opt-level 0'; do
  grep -qxF "$line" "$dir/ok.out" || fail "task test-realm-sizes did not print \"$line\" ($dir/ok.out)"
done
# The part at opt-level 1 of test-realm-sizes runs with the guard NO_STACK_TESTS.
grep -q '^run profile=collect .* guard=\[1\] .* config=realm-sizes-opt1-nextest\.toml ' "$dir/stub-ok.log" ||
  fail "task test-realm-sizes ran its part at opt-level 1 without the guard SWELL_NO_STACK_TESTS=1 ($dir/stub-ok.log)"

# 3. An exclusion list with no test of kind "other": every test of swell::main reads the
#    realm's room but the profile test, so .out/realm-sizes-default-other.txt names none, and
#    test-split adds no term: .out/realm-sizes-all-nextest.toml reads all().
plant noother
printf '%s\n' \
  '#[test]' \
  'fn the_profile_keeps_debug_assertions_and_overflow_checks_on() {' \
  '    assert!(cfg!(debug_assertions));' \
  '}' \
  '' \
  '#[test]' \
  'fn reads_the_room() {' \
  '    let _ = agent.heap_bytes();' \
  '}' \
  '' \
  '#[test]' \
  'fn reads_the_room_on_a_bounded_stack() {' \
  '    let _ = stack_with_room(1 << 20);' \
  '    let _ = agent.heap_bytes();' \
  '}' > "$dir/run-noother/crates/swell/tests/realm_padding.rs"
printf 'mod realm_padding;\nmod support;\n' > "$dir/run-noother/crates/swell/tests/main.rs"
rm -f "$dir/run-noother/crates/swell/tests/deep.rs"
printf '%s\n' \
  'swell::main realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on' \
  'swell::main realm_padding::reads_the_room' \
  'swell::main realm_padding::reads_the_room_on_a_bounded_stack' > "$dir/run-noother/stub-tests"
go noother test-realm-sizes REALM_SIZES=4 || fail "task test-realm-sizes exited non-zero with no test outside the room set but the profile test ($dir/noother.out)"
grep -qxF 'realm-sizes 4 default: 2 tests at opt-level 1, 1 at opt-level 0' "$dir/noother.out" ||
  fail "task test-realm-sizes did not run 2 tests at opt-level 1 and 1 at opt-level 0 with an empty exclusion ($dir/noother.out)"
grep -qxF "default-filter = 'all()'" "$dir/run-noother/.out/realm-sizes-all-nextest.toml" ||
  fail "test-split did not write the filter all() for an exclusion list with no test of kind other ($dir/run-noother/.out/realm-sizes-all-nextest.toml)"

# 1. A test in neither part of slow-tests's default mode.
plant neither
if STUB_DROP='swell::main deep::runs_on_any_stack' go neither slow-tests; then
  fail "task slow-tests exited 0 with a test in neither part ($dir/neither.out)"
fi
grep -qxF 'slow-tests: swell::main deep::runs_on_any_stack is in neither run' "$dir/neither.out" ||
  fail "task slow-tests did not name the test in neither part ($dir/neither.out)"
! grep -q '^run ' "$dir/stub-neither.log" || fail "task slow-tests ran tests although a test is in neither part ($dir/stub-neither.log)"

# 2. A test in both parts of test-realm-sizes's default mode.
plant both
if STUB_DOUBLE='swell::main realm_padding::reads_the_room_on_a_bounded_stack' go both test-realm-sizes REALM_SIZES=4; then
  fail "task test-realm-sizes exited 0 with a test in both parts ($dir/both.out)"
fi
grep -qxF 'realm-sizes: swell::main realm_padding::reads_the_room_on_a_bounded_stack is in both runs' "$dir/both.out" ||
  fail "task test-realm-sizes did not name the test in both parts ($dir/both.out)"
! grep -q '^run ' "$dir/stub-both.log" || fail "task test-realm-sizes ran tests although a test is in both parts ($dir/stub-both.log)"

echo "$name: ok"

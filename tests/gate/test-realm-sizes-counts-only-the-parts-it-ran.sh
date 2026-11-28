#!/bin/sh
# Planted second run of the task test-realm-sizes in one tree (Taskfile.yml; story
# slow-lane-holds-its-bound-on-the-slower-runner, step 2): after the two parts of its default
# mode at a size k the task prints "realm-sizes <k> default: <n> tests at opt-level 1, <m> at
# opt-level 0" from the Summary lines of the parts it ran, and a part whose list is empty does
# not run. The counts must be those of this run: a part that did not run counts 0, whatever an
# earlier run of the task left under .out/.
# No test runs and nothing is built: the script plants a tree of its own
# (.out/gate-tests/<name>/run) with the repository's Taskfile.yml and taskfiles/, a crate swell
# whose tests/realm_padding.rs holds the test TEST_OPT1_PROFILE_TEST names, a test that reads
# the room and one that reads the room on a bounded stack, whose tests/plain.rs holds a test
# that reads no room, and the stub cargo of
# tests/gate/fixtures/cargo-stub.sh first on PATH.
# 1. task test-realm-sizes REALM_SIZES=4 REALM_COLLECT=ends: exits 0 and prints "realm-sizes 4
#    default: 2 tests at opt-level 1, 1 at opt-level 0".
# 2. The test on a bounded stack is removed from the tree and the task runs again in the same
#    tree: it exits 0, runs no part at opt-level 0 and prints "realm-sizes 4 default: 2 tests at
#    opt-level 1, 0 at opt-level 0".
# Run from the repository root: sh tests/gate/test-realm-sizes-counts-only-the-parts-it-ran.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS REALM_SIZES REALM_COLLECT STUB_BUILD STUB_BUILD_MATCH STUB_COMPILE STUB_DROP STUB_DOUBLE STUB_FAIL
unset SPLIT_RUN SPLIT_ARGS SPLIT_EXCLUDE SPLIT_FILTER SPLIT_OPT1_TARGETS SPLIT_FULL_COUNTS SPLIT_BUILT_BY
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-realm-sizes-counts-only-the-parts-it-ran
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

run=$dir/run
mkdir -p "$run/crates/swell/tests/support" "$run/crates/swell/src" "$run/.out" "$run/.config" || fail "cannot make $run"
cp "$root/Taskfile.yml" "$run/Taskfile.yml" || fail "cannot copy Taskfile.yml"
cp -R "$root/taskfiles" "$run/taskfiles" || fail "cannot copy taskfiles"
cp "$root/.config/nextest.toml" "$run/.config/" || fail "cannot copy .config/nextest.toml"
mkdir -p "$run/spec" "$run/conformance/test262" || fail "cannot make $run/spec"
cp "$root/spec/ess-inputs.yaml" "$run/spec/" || fail "cannot copy spec/ess-inputs.yaml"
cp "$root/conformance/test262/PIN" "$run/conformance/test262/" || fail "cannot copy conformance/test262/PIN"
printf '[package]\nname = "swell"\n' > "$run/crates/swell/Cargo.toml"
printf '// the library\n' > "$run/crates/swell/src/lib.rs"
printf 'mod plain;\nmod realm_padding;\nmod support;\n' > "$run/crates/swell/tests/main.rs"
printf '#[test]\nfn adds() {\n    assert_eq!(1 + 1, 2);\n}\n' > "$run/crates/swell/tests/plain.rs"
printf 'pub fn stack_with_room(bytes: usize) -> usize {\n    bytes / 8 * 7\n}\n' > "$run/crates/swell/tests/support/host.rs"

# padding <with the test on a bounded stack: 1 or 0>: write realm_padding.rs and the stub's tests.
padding() {
  {
    printf '%s\n' \
      '#[test]' \
      'fn the_profile_keeps_debug_assertions_and_overflow_checks_on() {' \
      '    assert!(cfg!(debug_assertions));' \
      '}' \
      '' \
      '#[test]' \
      'fn reads_the_room() {' \
      '    let _ = agent.heap_bytes();' \
      '}'
    [ "$1" = 0 ] || printf '%s\n' \
      '' \
      '#[test]' \
      'fn reads_the_room_on_a_bounded_stack() {' \
      '    let _ = stack_with_room(1 << 20);' \
      '    let _ = agent.heap_bytes();' \
      '}'
  } > "$run/crates/swell/tests/realm_padding.rs"
  {
    printf '%s\n' \
      'swell::main plain::adds' \
      'swell::main realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on' \
      'swell::main realm_padding::reads_the_room'
    [ "$1" = 0 ] || echo 'swell::main realm_padding::reads_the_room_on_a_bounded_stack'
  } > "$run/stub-tests"
}

go() {
  : > "$dir/stub-$1.log"
  STUB_TESTS="$root/$run/stub-tests" STUB_LOG="$root/$dir/stub-$1.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test-realm-sizes REALM_SIZES=4 REALM_COLLECT=ends > "$dir/$1.out" 2>&1
}

# 1. Both parts hold a test.
padding 1
go first || fail "task test-realm-sizes exited non-zero with a test in each part ($dir/first.out)"
grep -qxF 'realm-sizes 4 default: 2 tests at opt-level 1, 1 at opt-level 0' "$dir/first.out" ||
  fail "the harness: task test-realm-sizes did not count 2 tests at opt-level 1 and 1 at opt-level 0 ($dir/first.out)"

# 2. The part at opt-level 0 is empty in the second run of the same tree.
padding 0
go second || fail "task test-realm-sizes exited non-zero with no test at opt-level 0 ($dir/second.out)"
! grep -q '^run profile=test ' "$dir/stub-second.log" || fail "the harness: the stub ran a part at opt-level 0 with no test in it ($dir/stub-second.log)"
grep -qxF 'realm-sizes 4 default: 2 tests at opt-level 1, 0 at opt-level 0' "$dir/second.out" ||
  fail "task test-realm-sizes ran no part at opt-level 0 and printed \"$(grep '^realm-sizes 4 default: ' "$dir/second.out")\", the count of a part an earlier run left in $run/.out/realm-sizes-4-default-opt0.log.count ($dir/second.out)"
echo "$name: ok"

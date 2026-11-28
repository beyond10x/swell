#!/bin/sh
# Planted defect of the task slow-tests (Taskfile.yml; stories realm-size-lane-within-its-share
# and slow-lane-holds-its-bound-on-the-slower-runner, step 2): the tests of swell::main that run
# on a bounded stack (taskfiles/realm-room-tests.awk -v rule=stack prints them "stack") run in
# the test profile at opt-level 0, in the default mode and in the collect mode; every other test
# of swell's packages that can move runs at opt-level 1: in the default mode as the gate's step
# test splits it (task test-split), in the collect mode the library's unit tests and the tests
# printed "other". A stack test proves the promised stack with frames whose size depends on the
# optimization level, so a run that put it at opt-level 1 only would prove less, and say nothing.
# No test runs: the script plants a tree of its own (.out/gate-tests/<name>/run-<case>) with the
# repository's Taskfile.yml and taskfiles/, a crate swell whose library reads
# SWELL_FULL_COUNTS, whose tests/deep.rs reads it and holds three tests (one of them under
# #[cfg(not(feature = "collect-every-safe-point"))]), whose tests/plain.rs holds one test and
# reads neither SWELL_FULL_COUNTS nor stack_with_room and whose tests/realm_padding.rs holds the
# test TEST_OPT1_PROFILE_TEST names, and the stub cargo of tests/gate/fixtures/cargo-stub.sh,
# which answers each list and run from its filters over the planted tests and logs each run with
# its profile, feature, SWELL_FULL_COUNTS, SWELL_NO_STACK_TESTS, configuration and tests.
# 1. deep::runs_on_the_promised_stack calls stack_with_room( from a function nested in it (its
#    body passed by value, as the stack tests of swell do), deep::runs_on_any_stack does not:
#    task slow-tests exits 0 and prints "slow-tests: swell::main 3 at opt-level 1, 1 at
#    opt-level 0", "slow-tests: 4 tests at opt-level 1, 1 at opt-level 0", "1 of the 2 tests
#    ... run at opt-level 0, the 1 others at opt-level 1 with the unit tests of the library; 1
#    test(s) of those modules are not in the collect build; 2 test(s) ... in other modules run
#    in neither" and "slow-tests collect-every-safe-point library: 1 tests at opt-level 1, 0 at
#    opt-level 0"; its five runs are, in order: the default mode at opt-level 1 (the profile
#    collect, the library's test, the two tests of deep printed "other" and the profile test),
#    at opt-level 0 (the test profile, the stack test), the library's unit tests in the profile
#    collect with the feature, the stack test in the test profile with the feature, and the test
#    of deep printed "other" in the profile collect with the feature (a test the rule does not
#    print, one a macro writes, may run on a bounded stack, so it runs at opt-level 0 only);
#    each with SWELL_FULL_COUNTS=1, the runs at opt-level 1 with SWELL_NO_STACK_TESTS=1 (the
#    guard that fails a stack test the rule missed) and no other run with it.
# 2. The planted defect: no test reaches stack_with_room (the module names it in a function no
#    test calls), so the rule prints no test "stack" and every test of the module would run at
#    opt-level 1 only: the task fails, saying it finds no test that runs on a bounded stack,
#    and the stub ran no run of the collect mode; the default mode ran its part at opt-level 1
#    alone (its part at opt-level 0 has no test).
# Run from the repository root: sh tests/gate/slow-tests-runs-the-stack-tests-at-opt-level-0.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=slow-tests-runs-the-stack-tests-at-opt-level-0
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
profile=realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on

# runs <log>: one line per run of the stub, "<profile> <feature> full=<SWELL_FULL_COUNTS>
# guard=<SWELL_NO_STACK_TESTS> config=<file>: <binary>/<test> ...".
runs() {
  awk '
    function field(k,   v) { if (match($0, " " k "=[^ ]*")) { v = substr($0, RSTART + length(k) + 2, RLENGTH - length(k) - 2); gsub(/[][]/, "", v); return v } return "" }
    /^run / { if (r != "") print r; r = field("profile") " " field("features") " full=" field("full") " guard=" field("guard") " config=" field("config") ":"; next }
    /^ran / { r = r " " $3 "/" $4 }
    END { if (r != "") print r }
  ' "$1"
}

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
  printf 'mod deep;\nmod plain;\nmod realm_padding;\nmod support;\n' > "$run/crates/swell/tests/main.rs"
  printf 'pub fn stack_with_room(bytes: usize) -> usize {\n    bytes / 8 * 7\n}\n' > "$run/crates/swell/tests/support/host.rs"
  printf '#[test]\nfn outside_the_filter() {\n    assert_eq!(1 + 1, 2);\n}\n' > "$run/crates/swell/tests/plain.rs"
  printf '#[test]\nfn the_profile_keeps_debug_assertions_and_overflow_checks_on() {\n    assert!(cfg!(debug_assertions));\n}\n' > "$run/crates/swell/tests/realm_padding.rs"
  printf '%s\n' \
    'swell unit::reads_full_counts' \
    'swell::main deep::runs_on_the_promised_stack' \
    'swell::main deep::runs_on_any_stack' \
    'swell::main deep::runs_in_the_default_mode_only default-only' \
    'swell::main plain::outside_the_filter' \
    "swell::main $profile" > "$run/stub-tests"
}

# go <case> <task arguments...>: run task in the planted tree of <case>.
go() {
  case_=$1
  shift
  : > "$dir/stub-$case_.log"
  STUB_TESTS="$root/$dir/run-$case_/stub-tests" STUB_LOG="$root/$dir/stub-$case_.log" PATH="$root/$dir/bin:$PATH" task -d "$dir/run-$case_" "$@" > "$dir/$case_.out" 2>&1
}

# 1. The stack test calls stack_with_room( in a function nested in it.
plant split
printf '%s\n' \
  'fn count() -> u32 {' \
  '    if std::env::var("SWELL_FULL_COUNTS").is_ok() { 1000 } else { 10 }' \
  '}' \
  '' \
  '#[test]' \
  'fn runs_on_the_promised_stack() {' \
  '    fn body() {' \
  '        let _ = stack_with_room(1 << 20);' \
  '    }' \
  '    std::thread::spawn(body).join().unwrap();' \
  '}' \
  '' \
  '#[test]' \
  'fn runs_on_any_stack() {' \
  '    assert!(count() > 0);' \
  '}' \
  '' \
  '/// Not in the collect build.' \
  '#[cfg(not(feature = "collect-every-safe-point"))]' \
  '#[test]' \
  'fn runs_in_the_default_mode_only() {' \
  '    assert!(count() > 0);' \
  '}' > "$dir/run-split/crates/swell/tests/deep.rs"
go split slow-tests || fail "task slow-tests exited non-zero on a module with one stack test and one other ($dir/split.out)"
for line in \
  'slow-tests: swell::main 3 at opt-level 1, 1 at opt-level 0' \
  'slow-tests: 4 tests at opt-level 1, 1 at opt-level 0' \
  "slow-tests collect-every-safe-point: 1 of the 2 tests of swell::main in the collect filter's modules that the collect build holds run on a bounded stack (taskfiles/realm-room-tests.awk -v rule=stack) and run at opt-level 0, the 1 others at opt-level 1 with the unit tests of the library; 1 test(s) of those modules are not in the collect build; 2 test(s) of swell::main in other modules run in neither" \
  'slow-tests collect-every-safe-point library: 1 tests at opt-level 1, 0 at opt-level 0'; do
  grep -qxF "$line" "$dir/split.out" || fail "slow-tests did not print \"$line\" ($dir/split.out)"
done
runs "$dir/stub-split.log" > "$dir/split.runs"
printf '%s\n' \
  "collect none full=1 guard=1 config=slow-tests-opt1-nextest.toml: swell/unit::reads_full_counts swell::main/deep::runs_on_any_stack swell::main/deep::runs_in_the_default_mode_only swell::main/$profile" \
  'test none full=1 guard=unset config=slow-tests-opt0-nextest.toml: swell::main/deep::runs_on_the_promised_stack' \
  'collect collect-every-safe-point full=1 guard=1 config=none: swell/unit::reads_full_counts' \
  'test collect-every-safe-point full=1 guard=unset config=slow-tests-collect-not-other-nextest.toml: swell::main/deep::runs_on_the_promised_stack' \
  'collect collect-every-safe-point full=1 guard=1 config=slow-tests-collect-only-other-nextest.toml: swell::main/deep::runs_on_any_stack' > "$dir/split.want"
cmp -s "$dir/split.want" "$dir/split.runs" || fail "the runs of slow-tests are not the five runs of $dir/split.want: $(tr '\n' ';' < "$dir/split.runs") ($dir/stub-split.log)"
for part in 'slow-tests opt-level 1: 2 test binaries: 4 passed' 'slow-tests opt-level 0: 1 test binaries: 1 passed' 'slow-tests collect-every-safe-point library: 1 test binaries: 1 passed' 'slow-tests collect-every-safe-point opt-level 1: 1 test binaries: 1 passed'; do
  grep -q "^$part" "$dir/split.out" || fail "slow-tests printed no count \"$part\" ($dir/split.out)"
done

# 2. The planted defect: no test reaches stack_with_room.
plant none
printf '%s\n' \
  'fn count() -> u32 {' \
  '    if std::env::var("SWELL_FULL_COUNTS").is_ok() { 1000 } else { 10 }' \
  '}' \
  '' \
  'fn unused() -> usize {' \
  '    stack_with_room(1 << 20)' \
  '}' \
  '' \
  '#[test]' \
  'fn runs_on_the_promised_stack() {' \
  '    assert!(count() > 1);' \
  '}' \
  '' \
  '#[test]' \
  'fn runs_on_any_stack() {' \
  '    assert!(count() > 0);' \
  '}' > "$dir/run-none/crates/swell/tests/deep.rs"
grep -v ' default-only$' "$dir/run-none/stub-tests" > "$dir/run-none/stub-tests.new" && mv "$dir/run-none/stub-tests.new" "$dir/run-none/stub-tests" ||
  fail "cannot write the planted tests of run-none"
if go none slow-tests; then
  fail "task slow-tests exited 0 when no test of swell::main runs on a bounded stack ($dir/none.out)"
fi
grep -qF 'slow-tests: taskfiles/realm-room-tests.awk -v rule=stack finds no test of swell::main that runs on a bounded stack' "$dir/none.out" ||
  fail "slow-tests did not say it finds no test on a bounded stack ($dir/none.out)"
runs "$dir/stub-none.log" > "$dir/none.runs"
! grep -q ' collect-every-safe-point ' "$dir/none.runs" || fail "the stub ran a run of the collect mode although no test runs on a bounded stack ($dir/none.runs)"
[ "$(grep -c . "$dir/none.runs")" = 1 ] && grep -q '^collect none full=1 guard=1 ' "$dir/none.runs" ||
  fail "the default mode did not run its part at opt-level 1 alone, with no test at opt-level 0 ($dir/none.runs)"

echo "$name: ok"

#!/bin/sh
# Planted defect of the split of the step test (Taskfile.yml, tasks test, test-split and
# test-sum; story gate-test-steps-hold-the-budget-on-the-slower-runner, item 3): the step runs
# the tests of swell::main that taskfiles/realm-room-tests.awk -v rule=stack prints "other" at
# opt-level 1 (TEST_OPT1_ARGS, the profile collect without the feature, SWELL_NO_STACK_TESTS=1)
# and every other test of the workspace at opt-level 0 (TEST_NEXTEST_ARGS), and every test runs
# once: from cargo nextest list it takes the tests of each run and those of the workspace
# without a filter, and fails, naming the test, when one is in not exactly one run.
# No test runs and nothing is built: the script plants a tree of its own
# (.out/gate-tests/<name>/run) with the repository's Taskfile.yml and taskfiles/, a crate swell
# whose tests/deep.rs holds a test that calls stack_with_room( and one that does not and whose
# tests/plain.rs holds one test, and a stub cargo first on PATH. The stub answers cargo nextest
# list from the default-filter line of the --config-file it is given (the names after "test(="
# are the run at opt-level 1, a line "not (...)" the run at opt-level 0, no file the workspace),
# over four tests of two binaries (swell::main and swell-num::main); it answers cargo nextest
# run with a Starting and a Summary line and a JUnit file of its own per run, and logs each
# run's profile, target, guard and default filter.
# 1. The split as it is: task test exits 0 and prints "test: swell::main 2 at opt-level 1, 1 at
#    opt-level 0", one line "2 test binaries: 4 passed, 0 failed, 1 ignored" for both runs (the
#    ignored one is the test neither run ran, not the sum of what each filter skipped), and
#    the slowest tests of both runs (the JUnit file of the first run is kept before the second
#    writes its own); the run at opt-level 1 is --test main in the profile collect with
#    SWELL_NO_STACK_TESTS=1 and the two tests printed other, the run at opt-level 0 is the
#    workspace without the variable and without those two.
# 2. The planted defect: the stub's list of the run at opt-level 1 drops
#    plain::outside_the_stack and adds deep::on_the_promised_stack, so one test is in neither
#    run and one is in both: task test fails, naming each, and the stub ran no test.
# Run from the repository root: sh tests/gate/test-runs-each-test-once-at-one-opt-level.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS STUB_DEFECT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-runs-each-test-once-at-one-opt-level
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/bin" || { echo "$name: FAIL cannot make $dir"; exit 1; }
root=$(pwd)

fail() {
  echo "$name: FAIL $*"
  exit 1
}

[ -f Taskfile.yml ] && [ -f taskfiles/realm-room-tests.awk ] || fail "run it from the repository root"

printf '%s\n' '#!/bin/sh' \
  'profile=test; prev=""; cfg=""' \
  'for a in "$@"; do' \
  '  [ "$prev" = "--cargo-profile" ] && profile=$a' \
  '  [ "$prev" = "--config-file" ] && cfg=$a' \
  '  prev=$a' \
  'done' \
  'case " $* " in *" --test main "*) target=main ;; *) target=workspace ;; esac' \
  'kind=none; names=""' \
  'if [ -n "$cfg" ]; then' \
  '  if grep -q "^default-filter = .not (" "$cfg"; then kind=not; else kind=only; fi' \
  '  names=$(grep "^default-filter = " "$cfg" | grep -o "test(=[^)]*)" | sed "s/^test(=//; s/)\$//" | tr "\n" " ")' \
  'fi' \
  'all="swell::main deep::on_any_stack|swell::main deep::on_the_promised_stack|swell::main plain::outside_the_stack|swell-num::main num::adds"' \
  'if [ "$2" = list ]; then' \
  '  printf "%s\n" "$all" | tr "|" "\n" | while read -r bin test; do' \
  '    case " $names " in *" $test "*) named=yes ;; *) named=no ;; esac' \
  '    [ "$bin" = swell::main ] || named=no' \
  '    case $kind in' \
  '      none) echo "$bin $test" ;;' \
  '      only) if [ "$named" = yes ]; then [ "${STUB_DEFECT:-}" = 1 ] && [ "$test" = plain::outside_the_stack ] || echo "$bin $test"; fi' \
  '            [ "${STUB_DEFECT:-}" = 1 ] && [ "$test" = deep::on_the_promised_stack ] && echo "$bin $test" ;;' \
  '      not) [ "$named" = yes ] || echo "$bin $test" ;;' \
  '    esac' \
  '  done' \
  '  exit 0' \
  'fi' \
  'printf "run profile %s target %s guard %s config-%s %s\n" "$profile" "$target" "${SWELL_NO_STACK_TESTS:-unset}" "$kind" "$names" >> "$STUB_LOG"' \
  'mkdir -p target/nextest/default' \
  'if [ "$profile" = collect ]; then' \
  '  printf "<testsuites><testsuite><testcase name=\"deep::on_any_stack\" classname=\"swell::main\" time=\"0.300\"/></testsuite></testsuites>\n" > target/nextest/default/junit.xml' \
  '  echo "    Starting 2 tests across 1 binary (1 test skipped)"' \
  '  echo "     Summary [   0.300s] 2 tests run: 2 passed, 1 skipped"' \
  'else' \
  '  printf "<testsuites><testsuite><testcase name=\"deep::on_the_promised_stack\" classname=\"swell::main\" time=\"0.200\"/><testcase name=\"num::adds\" classname=\"swell-num::main\" time=\"0.100\"/></testsuite></testsuites>\n" > target/nextest/default/junit.xml' \
  '  echo "    Starting 2 tests across 2 binaries (3 tests skipped)"' \
  '  echo "     Summary [   0.200s] 2 tests run: 2 passed, 3 skipped"' \
  'fi' \
  'exit 0' > "$dir/bin/cargo" || fail "cannot write the stub cargo"
chmod +x "$dir/bin/cargo"

run="$dir/run"
mkdir -p "$run/crates/swell/tests/support" "$run/crates/swell/src" "$run/.out" "$run/.config" || fail "cannot make $run"
cp "$root/Taskfile.yml" "$run/Taskfile.yml" || fail "cannot copy Taskfile.yml"
cp -R "$root/taskfiles" "$run/taskfiles" || fail "cannot copy taskfiles"
cp "$root/.config/nextest.toml" "$run/.config/" || fail "cannot copy .config/nextest.toml"
# Files the task files read when they load (taskfiles/spec.yml, taskfiles/test262*.yml).
mkdir -p "$run/spec" "$run/conformance/test262" || fail "cannot make $run/spec"
cp "$root/spec/ess-inputs.yaml" "$run/spec/" || fail "cannot copy spec/ess-inputs.yaml"
cp "$root/conformance/test262/PIN" "$run/conformance/test262/" || fail "cannot copy conformance/test262/PIN"
printf '[package]\nname = "swell"\n' > "$run/crates/swell/Cargo.toml"
printf 'mod deep;\nmod plain;\nmod support;\n' > "$run/crates/swell/tests/main.rs"
printf 'pub fn stack_with_room(bytes: usize) -> usize {\n    bytes / 8 * 7\n}\n' > "$run/crates/swell/tests/support/host.rs"
printf '%s\n' \
  '#[test]' \
  'fn on_the_promised_stack() {' \
  '    let _ = stack_with_room(1 << 20);' \
  '}' \
  '' \
  '#[test]' \
  'fn on_any_stack() {' \
  '    assert_eq!(1 + 1, 2);' \
  '}' > "$run/crates/swell/tests/deep.rs"
printf '#[test]\nfn outside_the_stack() {\n    assert_eq!(2 + 2, 4);\n}\n' > "$run/crates/swell/tests/plain.rs"

# 1. The split as it is.
: > "$dir/stub-split.log"
STUB_LOG="$root/$dir/stub-split.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/split.out" 2>&1 ||
  fail "task test exited non-zero on a split that runs each test once ($dir/split.out)"
grep -qxF 'test: swell::main 2 at opt-level 1, 1 at opt-level 0' "$dir/split.out" ||
  fail "task test did not print the tests of swell::main at each opt-level, 2 and 1 ($dir/split.out)"
grep -qxF '2 test binaries: 4 passed, 0 failed, 1 ignored' "$dir/split.out" ||
  fail "task test did not print one line of both runs, 2 binaries (swell::main once), 4 passed, 1 ignored (the test neither run ran) ($dir/split.out)"
[ "$(grep -c 'test binaries:' "$dir/split.out")" = 1 ] ||
  fail "task test printed not one line of counts ($dir/split.out)"
for slow in 'gate-slowest test 0.30 swell::main deep::on_any_stack' 'gate-slowest test 0.20 swell::main deep::on_the_promised_stack' 'gate-slowest test 0.10 swell-num::main num::adds'; do
  grep -qxF "$slow" "$dir/split.out" || fail "task test did not print \"$slow\": the slowest tests are taken over both runs ($dir/split.out)"
done
high=$(grep '^run profile collect ' "$dir/stub-split.log")
low=$(grep '^run profile test ' "$dir/stub-split.log")
[ "$(grep -c '^run ' "$dir/stub-split.log")" = 2 ] || fail "the stub saw not two runs ($dir/stub-split.log)"
case "$high" in "run profile collect target main guard 1 config-only deep::on_any_stack plain::outside_the_stack ") ;; *) fail "the run at opt-level 1 is not --test main in the profile collect with SWELL_NO_STACK_TESTS=1 and the two tests printed other: $high" ;; esac
case "$low" in "run profile test target workspace guard unset config-not deep::on_any_stack plain::outside_the_stack ") ;; *) fail "the run at opt-level 0 is not the workspace without SWELL_NO_STACK_TESTS and without the tests printed other: $low" ;; esac

# 2. The planted defect: one test in neither run, one in both.
: > "$dir/stub-defect.log"
STUB_DEFECT=1 STUB_LOG="$root/$dir/stub-defect.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/defect.out" 2>&1 &&
  fail "task test exited 0 with a test in neither run and one in both ($dir/defect.out)"
grep -qF 'test: swell::main plain::outside_the_stack is in neither run' "$dir/defect.out" ||
  fail "task test did not name the test in neither run ($dir/defect.out)"
grep -qF 'test: swell::main deep::on_the_promised_stack is in both runs' "$dir/defect.out" ||
  fail "task test did not name the test in both runs ($dir/defect.out)"
[ ! -s "$dir/stub-defect.log" ] || fail "task test ran tests although a test is in not exactly one run ($dir/stub-defect.log)"

echo "$name: ok"

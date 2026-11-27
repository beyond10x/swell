#!/bin/sh
# Planted override of the step test's run at opt-level 1 (Taskfile.yml, tasks test and
# test-vars-check; story gate-test-steps-hold-the-budget-on-the-slower-runner, item 3): the run
# takes its arguments from TEST_OPT1_ARGS (the profile collect, opt-level 1) and its guard from
# NO_STACK_TESTS (SWELL_NO_STACK_TESTS=1 for its test processes). The step refuses a value of
# TEST_RUSTFLAGS or TEST_NEXTEST_ARGS given on the command line (task test-vars-check); a value
# of TEST_OPT1_ARGS that drops the profile, or of NO_STACK_TESTS that names another variable,
# changes what the gate proves in the same way and must be refused too, before any test runs.
# No test runs and nothing is built: the script plants a tree of its own
# (.out/gate-tests/<name>/run) with the repository's Taskfile.yml and taskfiles/, a crate swell
# with one stack test and three others (one of them the test TEST_OPT1_PROFILE_TEST names, which
# the step's run at opt-level 1 must hold), and a stub cargo first on PATH that answers cargo nextest
# list from the default-filter line of its --config-file and logs each cargo nextest run with
# its cargo profile and the value of SWELL_NO_STACK_TESTS.
# 1. task test TEST_OPT1_ARGS='--locked -p swell --test main --profile default' (the run at
#    opt-level 1 without the profile collect): exits non-zero, and the stub ran no test.
# 2. task test NO_STACK_TESTS=SWELL_NO_GUARD (the run at opt-level 1 without the guard): exits
#    non-zero, and the stub ran no test.
# 3. The lines of 1 and 2 name the step, the variable, its value and Taskfile.yml (task
#    test-opt1-vars-check).
# 4. task test TEST_OPT1_PROFILE_TEST=deep::on_any_stack: exits non-zero, names it, no test ran.
# 5. task build-tests with the value of 1: exits non-zero, names it, and the stub built nothing.
# Run from the repository root: sh tests/gate/test-refuses-a-caller-value-of-its-run-at-opt-level-1.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS SWELL_NO_GUARD
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-refuses-a-caller-value-of-its-run-at-opt-level-1
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
  'kind=none; names=""' \
  'if [ -n "$cfg" ]; then' \
  '  if grep -q "^default-filter = .not (" "$cfg"; then kind=not; else kind=only; fi' \
  '  names=$(grep "^default-filter = " "$cfg" | grep -o "test(=[^)]*)" | sed "s/^test(=//; s/)\$//" | tr "\n" " ")' \
  'fi' \
  'all="swell::main deep::on_any_stack|swell::main deep::on_the_promised_stack|swell::main plain::outside_the_stack|swell::main realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on"' \
  'if [ "$2" = list ]; then' \
  '  printf "%s\n" "$all" | tr "|" "\n" | while read -r bin test; do' \
  '    case " $names " in *" $test "*) named=yes ;; *) named=no ;; esac' \
  '    case $kind in' \
  '      none) echo "$bin $test" ;;' \
  '      only) [ "$named" = no ] || echo "$bin $test" ;;' \
  '      not) [ "$named" = yes ] || echo "$bin $test" ;;' \
  '    esac' \
  '  done' \
  '  exit 0' \
  'fi' \
  'printf "run profile %s guard %s\n" "$profile" "${SWELL_NO_STACK_TESTS:-unset}" >> "$STUB_LOG"' \
  'mkdir -p target/nextest/default' \
  'printf "<testsuites><testsuite><testcase name=\"deep::on_any_stack\" classname=\"swell::main\" time=\"0.100\"/></testsuite></testsuites>\n" > target/nextest/default/junit.xml' \
  'if [ "$kind" = only ]; then' \
  '  echo "    Starting 2 tests across 1 binary (1 test skipped)"' \
  '  echo "     Summary [   0.100s] 2 tests run: 2 passed, 1 skipped"' \
  'else' \
  '  echo "    Starting 1 test across 1 binary (2 tests skipped)"' \
  '  echo "     Summary [   0.100s] 1 test run: 1 passed, 2 skipped"' \
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
printf 'mod deep;\nmod plain;\nmod realm_padding;\nmod support;\n' > "$run/crates/swell/tests/main.rs"
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
printf '#[test]\nfn the_profile_keeps_debug_assertions_and_overflow_checks_on() {\n    assert!(cfg!(debug_assertions));\n}\n' > "$run/crates/swell/tests/realm_padding.rs"

# 0. The harness: without a caller's value the step passes, the run at opt-level 1 in the
#    profile collect with the guard.
: > "$dir/stub-plain.log"
STUB_LOG="$root/$dir/stub-plain.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/plain.out" 2>&1 ||
  fail "task test exited non-zero without a caller's value ($dir/plain.out)"
grep -qxF 'run profile collect guard 1' "$dir/stub-plain.log" ||
  fail "the harness: the run at opt-level 1 is not in the profile collect with the guard ($dir/stub-plain.log)"

# 1. TEST_OPT1_ARGS from the command line, without the profile collect.
: > "$dir/stub-args.log"
STUB_LOG="$root/$dir/stub-args.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test TEST_OPT1_ARGS='--locked -p swell --test main --profile default' > "$dir/args.out" 2>&1 &&
  fail "task test exited 0 with TEST_OPT1_ARGS given on the command line without the profile collect; the run at opt-level 1 ran at opt-level 0: $(grep '^run ' "$dir/stub-args.log" | tr '\n' ';') ($dir/args.out)"
[ ! -s "$dir/stub-args.log" ] || fail "task test ran tests with TEST_OPT1_ARGS given on the command line ($dir/stub-args.log)"

# 2. NO_STACK_TESTS from the command line, naming another variable.
: > "$dir/stub-guard.log"
STUB_LOG="$root/$dir/stub-guard.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test NO_STACK_TESTS=SWELL_NO_GUARD > "$dir/guard.out" 2>&1 &&
  fail "task test exited 0 with NO_STACK_TESTS=SWELL_NO_GUARD given on the command line; the run at opt-level 1 ran without its guard: $(grep '^run ' "$dir/stub-guard.log" | tr '\n' ';') ($dir/guard.out)"
[ ! -s "$dir/stub-guard.log" ] || fail "task test ran tests with NO_STACK_TESTS given on the command line ($dir/stub-guard.log)"

# 3. The refusal's lines name the step, the variable, the value and the file.
grep -qF 'test: TEST_OPT1_ARGS=--locked -p swell --test main --profile default is not the value Taskfile.yml defines' "$dir/args.out" ||
  fail "task test did not name the step, TEST_OPT1_ARGS, its value and Taskfile.yml ($dir/args.out)"
grep -qF 'test: NO_STACK_TESTS=SWELL_NO_GUARD is not the value Taskfile.yml defines' "$dir/guard.out" ||
  fail "task test did not name the step, NO_STACK_TESTS, its value and Taskfile.yml ($dir/guard.out)"

# 4. TEST_OPT1_PROFILE_TEST from the command line, naming a test the run holds anyway.
: > "$dir/stub-keep.log"
STUB_LOG="$root/$dir/stub-keep.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test TEST_OPT1_PROFILE_TEST=deep::on_any_stack > "$dir/keep.out" 2>&1 &&
  fail "task test exited 0 with TEST_OPT1_PROFILE_TEST given on the command line ($dir/keep.out)"
[ ! -s "$dir/stub-keep.log" ] || fail "task test ran tests with TEST_OPT1_PROFILE_TEST given on the command line ($dir/stub-keep.log)"
grep -qF 'test: TEST_OPT1_PROFILE_TEST=deep::on_any_stack is not the value Taskfile.yml defines' "$dir/keep.out" ||
  fail "task test did not name the step, TEST_OPT1_PROFILE_TEST, its value and Taskfile.yml ($dir/keep.out)"

# 5. The step build-tests refuses the same before it builds anything.
: > "$dir/stub-build.log"
STUB_LOG="$root/$dir/stub-build.log" PATH="$root/$dir/bin:$PATH" task -d "$run" build-tests TEST_OPT1_ARGS='--locked -p swell --test main --profile default' > "$dir/build.out" 2>&1 &&
  fail "task build-tests exited 0 with TEST_OPT1_ARGS given on the command line ($dir/build.out)"
[ ! -s "$dir/stub-build.log" ] || fail "task build-tests built with TEST_OPT1_ARGS given on the command line ($dir/stub-build.log)"
grep -qF 'build-tests: TEST_OPT1_ARGS=--locked -p swell --test main --profile default is not the value Taskfile.yml defines' "$dir/build.out" ||
  fail "task build-tests did not name the step, TEST_OPT1_ARGS, its value and Taskfile.yml ($dir/build.out)"

echo "$name: ok"

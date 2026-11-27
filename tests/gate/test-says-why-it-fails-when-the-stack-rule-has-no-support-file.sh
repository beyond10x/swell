#!/bin/sh
# Planted tree whose crates/swell/tests/support/ holds no .rs file (Taskfile.yml, task
# test-split; story gate-test-steps-hold-the-budget-on-the-slower-runner, item 3): the first
# command of test-split gathers the rule's files from crates/swell/tests/support/*.rs. A loop
# whose body ends in a false [ -f "$f" ] && ... ends the command in go-task's shell with exit 1
# and no line of its own: the step test failed with only
# 'task: Failed to run task "test": task: Failed to run task "test-split": exit status 1'.
# No test runs and nothing is built: the harness of
# tests/gate/test-refuses-a-caller-value-of-its-run-at-opt-level-1.sh (a tree of its own, a
# stub cargo first on PATH) without crates/swell/tests/support/host.rs.
# 1. task test exits non-zero, prints a line "test: ..." of its own that says why, and the stub
#    ran no test.
# Run from the repository root: sh tests/gate/test-says-why-it-fails-when-the-stack-rule-has-no-support-file.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS SWELL_NO_GUARD
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-says-why-it-fails-when-the-stack-rule-has-no-support-file
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


rm -f "$run/crates/swell/tests/support/host.rs"
: > "$dir/stub-empty.log"
STUB_LOG="$root/$dir/stub-empty.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/empty.out" 2>&1 &&
  fail "task test exited 0 with no file in crates/swell/tests/support/ ($dir/empty.out)"
grep -q '^test: ' "$dir/empty.out" ||
  fail "task test failed without a line of its own: $(head -n 2 "$dir/empty.out" | tr '\n' ';') ($dir/empty.out)"
[ ! -s "$dir/stub-empty.log" ] || fail "task test ran tests with no file in crates/swell/tests/support/ ($dir/stub-empty.log)"
echo "$name: ok"

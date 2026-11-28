#!/bin/sh
# Planted move of the profile's test out of the step test's run at opt-level 1 (Taskfile.yml,
# tasks test and test-split; story gate-test-steps-hold-the-budget-on-the-slower-runner, item
# 3): the cut to opt-level 1 holds on the condition that debug assertions and overflow checks
# stay on in that run, and one test holds both,
# realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on. It proves the
# condition only in the run it is in. When the stack rule stops printing it "other" (its text
# names stack_with_room(, it moves into a nested mod, a macro writes it) it runs at opt-level 0,
# where it passes whatever the profile collect holds, and the step must fail, naming it.
# No test runs and nothing is built: the harness of
# tests/gate/test-refuses-a-caller-value-of-its-run-at-opt-level-1.sh (a tree of its own, a
# stub cargo first on PATH) with a module realm_padding that holds the profile's test.
# 1. The test as the rule prints it "other": task test exits 0 and its list of the run at
#    opt-level 1 holds the test.
# 2. The test calls stack_with_room(, so the rule prints it "stack": task test exits non-zero
#    and names the test.
# 3. A macro writes the test (the rule does not print it): non-zero, the test named.
# 4. The test in a nested mod (the binary's name is not the one the rule prints): non-zero, the
#    test named.
# 5. The test renamed in its Rust file only, to a name that holds the old one: non-zero, the old
#    name named.
# 6. The test marked #[ignore] (cargo nextest list leaves it out): non-zero, the test named.
# Run from the repository root: sh tests/gate/test-keeps-the-profile-test-in-its-run-at-opt-level-1.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS SWELL_NO_GUARD
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-keeps-the-profile-test-in-its-run-at-opt-level-1
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
  'profile=test; prev=""; cfg=""; fmt=""' \
  'for a in "$@"; do' \
  '  [ "$prev" = "--cargo-profile" ] && profile=$a' \
  '  [ "$prev" = "--config-file" ] && cfg=$a' \
  '  [ "$prev" = "--message-format" ] && fmt=$a' \
  '  prev=$a' \
  'done' \
  'kind=none; names=""; pkgs=""; pairs=""' \
  'if [ -n "$cfg" ]; then' \
  '  if grep -q "^default-filter = .not (" "$cfg"; then kind=not; else kind=only; fi' \
  '  line=$(grep "^default-filter = " "$cfg")' \
  '  names=$(printf "%s\n" "$line" | sed "s/binary_id(=[^)]*) & test(=[^)]*)//g" | grep -o "test(=[^)]*)" | sed "s/^test(=//; s/)\$//" | tr "\n" " ")' \
  '  pkgs=$(printf "%s\n" "$line" | grep -o "package(=[^)]*)" | sed "s/^package(=//; s/)\$//" | tr "\n" " ")' \
  '  pairs=$(printf "%s\n" "$line" | grep -o "binary_id(=[^)]*) & test(=[^)]*)" | sed "s/^binary_id(=//; s/) & test(=/ /; s/)\$//" | tr "\n" "|")' \
  'fi' \
  'all="swell::main deep::on_any_stack|swell::main deep::on_the_promised_stack|swell::main plain::outside_the_stack"' \
  'extra=$(sed -n "s/^  TEST_OPT0_TESTS: //p" Taskfile.yml | tr " " "\n" | paste -d " " - - | tr "\n" "|")' \
  'if [ "$2" = list ] && [ "$fmt" = json-pretty ]; then' \
  '  printf "{\n  \"test-count\": %s,\n  \"rust-suites\": {\n" "$(printf "%s\n" "$all|$extra" | tr "|" "\n" | grep -c " ")"' \
  '  printf "%s\n" "$all|$extra" | tr "|" "\n" | while read -r bin test; do' \
  '    [ -n "$bin" ] || continue' \
  '    [ "$bin" = "${last:-}" ] || printf "    \"%s\": {\n      \"binary-id\": \"%s\",\n      \"testcases\": {\n" "$bin" "$bin"' \
  '    last=$bin' \
  '    printf "        \"%s\": {\n          \"kind\": \"test\",\n          \"ignored\": false,\n          \"filter-match\": {\n            \"status\": \"matches\"\n          }\n        },\n" "$test"' \
  '  done' \
  '  exit 0' \
  'fi' \
  'if [ "$2" = list ]; then' \
  '  printf "%s\n" "$all|$extra" | tr "|" "\n" | while read -r bin test; do' \
  '    [ -n "$bin" ] || continue' \
  '    if [ "$bin" = swell::main ]; then' \
  '      case " $names " in *" $test "*) named=yes ;; *) named=no ;; esac' \
  '    else' \
  '      case " $pkgs " in *" ${bin%%::*} "*) named=yes ;; *) named=no ;; esac' \
  '      case "|$pairs" in *"|$bin $test|"*) named=no ;; esac' \
  '    fi' \
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


profile_test=realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on
sed "s/|swell::main plain::outside_the_stack\"/|swell::main plain::outside_the_stack|swell::main $profile_test\"/" "$dir/bin/cargo" > "$dir/bin/cargo.new" &&
  mv "$dir/bin/cargo.new" "$dir/bin/cargo" && chmod +x "$dir/bin/cargo" || fail "cannot add the profile's test to the stub"
grep -qF "$profile_test" "$dir/bin/cargo" || fail "the stub does not list the profile's test"
printf 'mod deep;\nmod plain;\nmod realm_padding;\nmod support;\n' > "$run/crates/swell/tests/main.rs"

# 1. The profile's test as the rule prints it "other".
printf '%s\n' '#[test]' 'fn the_profile_keeps_debug_assertions_and_overflow_checks_on() {' '    assert!(cfg!(debug_assertions));' '}' > "$run/crates/swell/tests/realm_padding.rs"
: > "$dir/stub-other.log"
STUB_LOG="$root/$dir/stub-other.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/other.out" 2>&1 ||
  fail "task test exited non-zero with the profile's test printed other ($dir/other.out)"
grep -qxF "swell::main $profile_test" "$run/.out/test-list-opt1.txt" ||
  fail "the harness: the list of the run at opt-level 1 does not hold the profile's test ($run/.out/test-list-opt1.txt)"

# 2. The profile's test printed "stack": it would run at opt-level 0.
printf '%s\n' '#[test]' 'fn the_profile_keeps_debug_assertions_and_overflow_checks_on() {' '    let _ = stack_with_room(1 << 20);' '    assert!(cfg!(debug_assertions));' '}' > "$run/crates/swell/tests/realm_padding.rs"
: > "$dir/stub-stack.log"
STUB_LOG="$root/$dir/stub-stack.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/stack.out" 2>&1 &&
  fail "task test exited 0 with the profile's test out of the run at opt-level 1 (it is in $(grep -c "$profile_test" "$run/.out/test-list-opt0.txt") line(s) of the list of the run at opt-level 0); nothing holds debug assertions and overflow checks in the run at opt-level 1 ($dir/stack.out)"
grep -qF "$profile_test" "$dir/stack.out" ||
  fail "task test did not name the profile's test out of the run at opt-level 1 ($dir/stack.out)"
[ ! -s "$dir/stub-stack.log" ] || fail "task test ran tests with the profile's test out of the run at opt-level 1 ($dir/stub-stack.log)"

# 3. A macro writes the profile's test: the rule does not print it, the binary holds it.
printf '%s\n' 'macro_rules! profile_test {' '    ($name:ident) => {' '        #[test]' '        fn $name() {' '            assert!(cfg!(debug_assertions));' '        }' '    };' '}' 'profile_test!(the_profile_keeps_debug_assertions_and_overflow_checks_on);' > "$run/crates/swell/tests/realm_padding.rs"
: > "$dir/stub-macro.log"
STUB_LOG="$root/$dir/stub-macro.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/macro.out" 2>&1 &&
  fail "task test exited 0 with the profile's test written by a macro, out of the run at opt-level 1 ($dir/macro.out)"
grep -qF "test: swell::main $profile_test is not in the run at opt-level 1" "$dir/macro.out" ||
  fail "task test did not name the profile's test written by a macro ($dir/macro.out)"

# 4. The profile's test in a nested mod: the rule prints realm_padding::<test>, the binary
#    holds realm_padding::inner::<test>.
sed "s/|swell::main $profile_test\"/|swell::main realm_padding::inner::the_profile_keeps_debug_assertions_and_overflow_checks_on\"/" "$dir/bin/cargo" > "$dir/bin/cargo.new" &&
  mv "$dir/bin/cargo.new" "$dir/bin/cargo" && chmod +x "$dir/bin/cargo" || fail "cannot nest the profile's test in the stub"
grep -qF 'realm_padding::inner::' "$dir/bin/cargo" || fail "the stub does not list the nested profile's test"
printf '%s\n' 'mod inner {' '    #[test]' '    fn the_profile_keeps_debug_assertions_and_overflow_checks_on() {' '        assert!(cfg!(debug_assertions));' '    }' '}' > "$run/crates/swell/tests/realm_padding.rs"
: > "$dir/stub-nested.log"
STUB_LOG="$root/$dir/stub-nested.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/nested.out" 2>&1 &&
  fail "task test exited 0 with the profile's test in a nested mod, out of the run at opt-level 1 ($dir/nested.out)"
grep -qF "test: swell::main $profile_test is not in the run at opt-level 1" "$dir/nested.out" ||
  fail "task test did not name the profile's test in a nested mod ($dir/nested.out)"

# 5. The profile's test renamed in its Rust file only (TEST_OPT1_PROFILE_TEST still names the old
#    name): the run at opt-level 1 holds realm_padding::<old name>_in_collect, whose name holds
#    the old one; the step fails naming the old name.
sed "s/|swell::main realm_padding::inner::the_profile_keeps_debug_assertions_and_overflow_checks_on\"/|swell::main ${profile_test}_in_collect\"/" "$dir/bin/cargo" > "$dir/bin/cargo.new" &&
  mv "$dir/bin/cargo.new" "$dir/bin/cargo" && chmod +x "$dir/bin/cargo" || fail "cannot rename the profile's test in the stub"
grep -qF "${profile_test}_in_collect" "$dir/bin/cargo" || fail "the stub does not list the renamed profile's test"
printf '%s\n' '#[test]' 'fn the_profile_keeps_debug_assertions_and_overflow_checks_on_in_collect() {' '    assert!(cfg!(debug_assertions));' '}' > "$run/crates/swell/tests/realm_padding.rs"
: > "$dir/stub-renamed.log"
STUB_LOG="$root/$dir/stub-renamed.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/renamed.out" 2>&1 &&
  fail "task test exited 0 with the profile's test renamed in its Rust file only ($dir/renamed.out)"
grep -qF "test: swell::main $profile_test is not in the run at opt-level 1" "$dir/renamed.out" ||
  fail "task test did not name the profile's test renamed in its Rust file only ($dir/renamed.out)"
[ ! -s "$dir/stub-renamed.log" ] || fail "task test ran tests with the profile's test renamed in its Rust file only ($dir/stub-renamed.log)"

# 6. The profile's test marked #[ignore]: cargo nextest list leaves an ignored test out of its
#    list (faults::host_71_injected_fault_at_each_call_faults_the_agent, ignored without the
#    feature fault-injection, is in no list of the step), so the stub lists it in no run.
sed "s/|swell::main ${profile_test}_in_collect\"/\"/" "$dir/bin/cargo" > "$dir/bin/cargo.new" &&
  mv "$dir/bin/cargo.new" "$dir/bin/cargo" && chmod +x "$dir/bin/cargo" || fail "cannot drop the profile's test from the stub"
grep -qF "$profile_test" "$dir/bin/cargo" && fail "the stub still lists the profile's test"
printf '%s\n' '#[test]' '#[ignore]' 'fn the_profile_keeps_debug_assertions_and_overflow_checks_on() {' '    assert!(cfg!(debug_assertions));' '}' > "$run/crates/swell/tests/realm_padding.rs"
: > "$dir/stub-ignored.log"
STUB_LOG="$root/$dir/stub-ignored.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/ignored.out" 2>&1 &&
  fail "task test exited 0 with the profile's test ignored ($dir/ignored.out)"
grep -qF "test: swell::main $profile_test is not in the run at opt-level 1" "$dir/ignored.out" ||
  fail "task test did not name the ignored profile's test ($dir/ignored.out)"

echo "$name: ok"

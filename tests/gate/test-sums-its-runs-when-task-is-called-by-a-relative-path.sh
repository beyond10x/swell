#!/bin/sh
# Planted call of the gate by a relative path (Taskfile.yml, task test-sum; story
# gate-test-steps-hold-the-budget-on-the-slower-runner, item 3): test-sum runs the task
# nextest-report on each run of the step test as "{{.TASK_EXE}}" nextest-report, a new process
# started in the directory of Taskfile.yml. TASK_EXE is the task executable as the caller named
# it; a caller that names it by a path relative to another directory (./bin/task -d swell
# check) gets a step that cannot count its runs, although every test passed. Before the split
# the step called nextest-report as a task, which needs no path.
# No test runs and nothing is built: the harness of
# tests/gate/test-refuses-a-caller-value-of-its-run-at-opt-level-1.sh (a tree of its own, a
# stub cargo first on PATH, the test TEST_OPT1_PROFILE_TEST names among its tests), and a link
# to the task executable at <dir>/rel/task.
# 1. From <dir>, ./rel/task -d run test: exits 0 and prints "<b> test binaries: 3 passed, 0
#    failed, <i> ignored" (b and i come from the stub's list of the workspace, whose pairs of
#    TEST_OPT0_TESTS are those of the repository's Taskfile.yml).
# 2. The same with the link named by its absolute path.
# 3. From <dir>, rel/task -d run test (no leading ./): the same.
# Run from the repository root: sh tests/gate/test-sums-its-runs-when-task-is-called-by-a-relative-path.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS SWELL_NO_GUARD
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-sums-its-runs-when-task-is-called-by-a-relative-path
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
  'all="swell::main deep::on_any_stack|swell::main deep::on_the_promised_stack|swell::main plain::outside_the_stack|swell::main realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on"' \
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


exe=$(command -v task) || fail "no task on PATH"
mkdir -p "$dir/rel" && ln -sf "$exe" "$dir/rel/task" || fail "cannot link $dir/rel/task"

# 1. The task executable named by a path relative to the caller's directory.
: > "$dir/stub-rel.log"
(cd "$dir" && STUB_LOG="$root/$dir/stub-rel.log" PATH="$root/$dir/bin:$PATH" ./rel/task -d run test) > "$dir/rel.out" 2>&1 ||
  fail "./rel/task -d run test exited non-zero although every test passed: $(grep -v '^test: swell::main' "$dir/rel.out" | head -n 2 | tr '\n' ';') ($dir/rel.out)"
grep -qxE '[0-9]+ test binaries: 3 passed, 0 failed, [0-9]+ ignored' "$dir/rel.out" ||
  fail "./rel/task -d run test did not print the summed line of both runs ($dir/rel.out)"

# 2. The task executable named by its absolute path, from the same directory (a bare name is
#    looked up in PATH: every other planted script of the step calls "task -d").
: > "$dir/stub-abs.log"
(cd "$dir" && STUB_LOG="$root/$dir/stub-abs.log" PATH="$root/$dir/bin:$PATH" "$root/$dir/rel/task" -d run test) > "$dir/abs.out" 2>&1 ||
  fail "<dir>/rel/task by its absolute path -d run test exited non-zero although every test passed ($dir/abs.out)"
grep -qxE '[0-9]+ test binaries: 3 passed, 0 failed, [0-9]+ ignored' "$dir/abs.out" ||
  fail "<dir>/rel/task by its absolute path -d run test did not print the summed line of both runs ($dir/abs.out)"

# 3. The task executable named by a relative path without a leading ./ (rel/task): a path that
#    names a directory is the caller's, whatever its first part.
: > "$dir/stub-nodot.log"
(cd "$dir" && STUB_LOG="$root/$dir/stub-nodot.log" PATH="$root/$dir/bin:$PATH" rel/task -d run test) > "$dir/nodot.out" 2>&1 ||
  fail "rel/task -d run test exited non-zero although every test passed: $(grep -v '^test: swell::main' "$dir/nodot.out" | head -n 2 | tr '\n' ';') ($dir/nodot.out)"
grep -qxE '[0-9]+ test binaries: 3 passed, 0 failed, [0-9]+ ignored' "$dir/nodot.out" ||
  fail "rel/task -d run test did not print the summed line of both runs ($dir/nodot.out)"

echo "$name: ok"

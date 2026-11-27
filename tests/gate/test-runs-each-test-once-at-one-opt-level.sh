#!/bin/sh
# Planted defect of the split of the step test (Taskfile.yml, tasks test, test-split and
# test-sum; story gate-test-steps-hold-the-budget-on-the-slower-runner, items 3 and 5): the
# step runs the tests of swell::main that taskfiles/realm-room-tests.awk -v rule=stack prints
# "other" at opt-level 1 (TEST_OPT1_ARGS, the profile collect without the feature,
# SWELL_NO_STACK_TESTS=1) with the other binaries of its packages, and every other test of
# the workspace at opt-level 0 (TEST_NEXTEST_ARGS), and every test runs once: from cargo
# nextest list it takes the tests of each run and those of the workspace without a filter, and
# fails, naming the test, when one is in not exactly one run.
# No test runs and nothing is built: the script plants a tree of its own
# (.out/gate-tests/<name>/run) with the repository's Taskfile.yml, whose TEST_OPT0_TESTS line it
# sets to the pair "swell-syntax::main sources::nests", and taskfiles/, a crate swell whose
# tests/deep.rs holds a test that calls stack_with_room( and one that does not and whose
# tests/plain.rs holds one test, whose tests/realm_padding.rs holds the test
# TEST_OPT1_PROFILE_TEST names (the run at opt-level 1 must hold it), and a stub cargo first on
# PATH. The stub answers cargo nextest list from the default-filter line of the --config-file it
# is given (the names after "test(=" of swell::main, the packages after "package(=", the pairs
# "binary_id(=<binary>) & test(=<test>)" left out of them; a line "not (...)" is the run at
# opt-level 0, no file the workspace, in JSON) over five tests of two binaries (swell::main
# and swell-syntax::main, the second from the pair) and one test of swell::main that is
# ignored (in the JSON of the workspace only, as cargo nextest list leaves an ignored test
# out); it answers cargo nextest run with a Starting and a Summary line over the tests of its
# list and a JUnit file of its own per run, and logs each run's profile, target, guard and the
# names of swell::main in its default filter.
# 1. The split as it is: task test exits 0 and prints "test: swell::main 3 at opt-level 1, 1 at
#    opt-level 0", "test: 3 tests at opt-level 1, 2 at opt-level 0", one line "2 test
#    binaries: 5 passed, 0 failed, 1 ignored" for both runs (b and i from the list of the
#    workspace: its binaries, and its tests, the ignored one among them, less those passed and
#    failed), and the slowest tests of both runs (the JUnit file of the first run is kept
#    before the second writes its own); the run at opt-level 1 is the packages of
#    TEST_OPT1_ARGS in the profile collect with SWELL_NO_STACK_TESTS=1 and the three tests
#    printed other, the run at opt-level 0 is the workspace without the variable and without
#    those three.
# 2. The planted defect: the stub's list of the run at opt-level 1 drops
#    plain::outside_the_stack and adds deep::on_the_promised_stack, so one test is in neither
#    run and one is in both: task test fails, naming each, and the stub ran no test.
# 3. A failing test in the run at opt-level 1 (STUB_OPT1=fail): task test fails, both runs
#    ran, and the summed line reads "2 test binaries: 4 passed, 1 failed, 1 ignored".
# 4. A Compiling line in the log of the list at opt-level 1 (STUB_LIST_COMPILE=1): task test
#    fails naming .out/test-list-opt1.txt.log, and the stub ran no test.
# Run from the repository root: sh tests/gate/test-runs-each-test-once-at-one-opt-level.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS STUB_DEFECT STUB_OPT1 STUB_LIST_COMPILE
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
  'profile=test; prev=""; cfg=""; fmt=""' \
  'for a in "$@"; do' \
  '  [ "$prev" = "--cargo-profile" ] && profile=$a' \
  '  [ "$prev" = "--config-file" ] && cfg=$a' \
  '  [ "$prev" = "--message-format" ] && fmt=$a' \
  '  prev=$a' \
  'done' \
  'case " $* " in *" --workspace "*) target=workspace ;; *) target=packages ;; esac' \
  'kind=none; names=""; pkgs=""; pairs=""' \
  'if [ -n "$cfg" ]; then' \
  '  if grep -q "^default-filter = .not (" "$cfg"; then kind=not; else kind=only; fi' \
  '  line=$(grep "^default-filter = " "$cfg")' \
  '  names=$(printf "%s\n" "$line" | sed "s/binary_id(=[^)]*) & test(=[^)]*)//g" | grep -o "test(=[^)]*)" | sed "s/^test(=//; s/)\$//" | tr "\n" " ")' \
  '  pkgs=$(printf "%s\n" "$line" | grep -o "package(=[^)]*)" | sed "s/^package(=//; s/)\$//" | tr "\n" " ")' \
  '  pairs=$(printf "%s\n" "$line" | grep -o "binary_id(=[^)]*) & test(=[^)]*)" | sed "s/^binary_id(=//; s/) & test(=/ /; s/)\$//" | tr "\n" "|")' \
  'fi' \
  'all="swell::main deep::on_any_stack|swell::main deep::on_the_promised_stack|swell::main plain::outside_the_stack|swell::main realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on"' \
  'ign="swell::main deep::ignored_without_a_feature"' \
  'extra=$(sed -n "s/^  TEST_OPT0_TESTS: //p" Taskfile.yml | tr " " "\n" | paste -d " " - - | tr "\n" "|")' \
  'if [ "$2" = list ] && [ "$fmt" = json-pretty ]; then' \
  '  printf "{\n  \"test-count\": %s,\n  \"rust-suites\": {\n" "$(printf "%s\n" "$ign|$all|$extra" | tr "|" "\n" | grep -c " ")"' \
  '  printf "%s\n" "$ign|$all|$extra" | tr "|" "\n" | while read -r bin test; do' \
  '    [ -n "$bin" ] || continue' \
  '    [ "$bin" = "${last:-}" ] || printf "    \"%s\": {\n      \"binary-id\": \"%s\",\n      \"testcases\": {\n" "$bin" "$bin"' \
  '    last=$bin' \
  '    if [ "$bin $test" = "$ign" ]; then ig=true; st=mismatch; else ig=false; st=matches; fi' \
  '    printf "        \"%s\": {\n          \"kind\": \"test\",\n          \"ignored\": %s,\n          \"filter-match\": {\n            \"status\": \"%s\"\n          }\n        },\n" "$test" "$ig" "$st"' \
  '  done' \
  '  exit 0' \
  'fi' \
  'if [ "$2" = list ]; then' \
  '  if [ "${STUB_LIST_COMPILE:-}" = 1 ] && [ "$profile" = collect ]; then echo "   Compiling swell v0.1.0 (/src/crates/swell)" >&2; fi' \
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
  '      only) if [ "$named" = yes ]; then [ "${STUB_DEFECT:-}" = 1 ] && [ "$test" = plain::outside_the_stack ] || echo "$bin $test"; fi' \
  '            [ "${STUB_DEFECT:-}" = 1 ] && [ "$test" = deep::on_the_promised_stack ] && echo "$bin $test" ;;' \
  '      not) [ "$named" = yes ] || echo "$bin $test" ;;' \
  '    esac' \
  '  done' \
  '  exit 0' \
  'fi' \
  'printf "run profile %s target %s guard %s config-%s %s\n" "$profile" "$target" "${SWELL_NO_STACK_TESTS:-unset}" "$kind" "$names" >> "$STUB_LOG"' \
  'n=$("$0" nextest list --message-format oneline --config-file "$cfg" 2>/dev/null | grep -c " ")' \
  'mkdir -p target/nextest/default' \
  'if [ "$profile" = collect ]; then' \
  '  printf "<testsuites><testsuite><testcase name=\"deep::on_any_stack\" classname=\"swell::main\" time=\"0.300\"/></testsuite></testsuites>\n" > target/nextest/default/junit.xml' \
  '  echo "    Starting $n tests across 1 binary"' \
  '  if [ "${STUB_OPT1:-}" = fail ]; then echo "     Summary [   0.300s] $n tests run: $((n - 1)) passed, 1 failed, 0 skipped"; exit 100; fi' \
  '  echo "     Summary [   0.300s] $n tests run: $n passed, 0 skipped"' \
  'else' \
  '  printf "<testsuites><testsuite><testcase name=\"deep::on_the_promised_stack\" classname=\"swell::main\" time=\"0.200\"/><testcase name=\"sources::nests\" classname=\"swell-syntax::main\" time=\"0.100\"/></testsuite></testsuites>\n" > target/nextest/default/junit.xml' \
  '  echo "    Starting $n tests across 2 binaries"' \
  '  echo "     Summary [   0.200s] $n tests run: $n passed, 0 skipped"' \
  'fi' \
  'exit 0' > "$dir/bin/cargo" || fail "cannot write the stub cargo"
chmod +x "$dir/bin/cargo"

run="$dir/run"
mkdir -p "$run/crates/swell/tests/support" "$run/crates/swell/src" "$run/.out" "$run/.config" || fail "cannot make $run"
pair='swell-syntax::main sources::nests'
grep -q '^  TEST_OPT0_TESTS: ' Taskfile.yml || fail "Taskfile.yml has no line \"  TEST_OPT0_TESTS: <pairs>\" to plant the pair $pair in"
sed "s/^  TEST_OPT0_TESTS: .*\$/  TEST_OPT0_TESTS: $pair/" Taskfile.yml > "$run/Taskfile.yml" || fail "cannot copy Taskfile.yml"
grep -qxF "  TEST_OPT0_TESTS: $pair" "$run/Taskfile.yml" || fail "the planted Taskfile.yml does not hold the pair $pair"
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

# 1. The split as it is.
: > "$dir/stub-split.log"
STUB_LOG="$root/$dir/stub-split.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/split.out" 2>&1 ||
  fail "task test exited non-zero on a split that runs each test once ($dir/split.out)"
grep -qxF 'test: swell::main 3 at opt-level 1, 1 at opt-level 0' "$dir/split.out" ||
  fail "task test did not print the tests of swell::main at each opt-level, 3 and 1 ($dir/split.out)"
grep -qxF 'test: 3 tests at opt-level 1, 2 at opt-level 0' "$dir/split.out" ||
  fail "task test did not print the tests of the workspace at each opt-level, 3 and 2 ($dir/split.out)"
grep -qxF '2 test binaries: 5 passed, 0 failed, 1 ignored' "$dir/split.out" ||
  fail "task test did not print one line of both runs, 2 binaries (the workspace's, swell::main once), 5 passed, 1 ignored (the test neither run ran) ($dir/split.out)"
[ "$(grep -c 'test binaries:' "$dir/split.out")" = 1 ] ||
  fail "task test printed not one line of counts ($dir/split.out)"
for slow in 'gate-slowest test 0.30 swell::main deep::on_any_stack' 'gate-slowest test 0.20 swell::main deep::on_the_promised_stack' 'gate-slowest test 0.10 swell-syntax::main sources::nests'; do
  grep -qxF "$slow" "$dir/split.out" || fail "task test did not print \"$slow\": the slowest tests are taken over both runs ($dir/split.out)"
done
high=$(grep '^run profile collect ' "$dir/stub-split.log")
low=$(grep '^run profile test ' "$dir/stub-split.log")
[ "$(grep -c '^run ' "$dir/stub-split.log")" = 2 ] || fail "the stub saw not two runs ($dir/stub-split.log)"
case "$high" in "run profile collect target packages guard 1 config-only deep::on_any_stack plain::outside_the_stack realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on ") ;; *) fail "the run at opt-level 1 is not the packages of TEST_OPT1_ARGS in the profile collect with SWELL_NO_STACK_TESTS=1 and the three tests printed other: $high" ;; esac
case "$low" in "run profile test target workspace guard unset config-not deep::on_any_stack plain::outside_the_stack realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on ") ;; *) fail "the run at opt-level 0 is not the workspace without SWELL_NO_STACK_TESTS and without the tests printed other: $low" ;; esac

# 2. The planted defect: one test in neither run, one in both.
: > "$dir/stub-defect.log"
STUB_DEFECT=1 STUB_LOG="$root/$dir/stub-defect.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/defect.out" 2>&1 &&
  fail "task test exited 0 with a test in neither run and one in both ($dir/defect.out)"
grep -qF 'test: swell::main plain::outside_the_stack is in neither run' "$dir/defect.out" ||
  fail "task test did not name the test in neither run ($dir/defect.out)"
grep -qF 'test: swell::main deep::on_the_promised_stack is in both runs' "$dir/defect.out" ||
  fail "task test did not name the test in both runs ($dir/defect.out)"
[ ! -s "$dir/stub-defect.log" ] || fail "task test ran tests although a test is in not exactly one run ($dir/stub-defect.log)"

# 3. A test of the run at opt-level 1 fails: the stub's run there prints "3 tests run: 2
#    passed, 1 failed" and exits 100.
: > "$dir/stub-fail.log"
STUB_OPT1=fail STUB_LOG="$root/$dir/stub-fail.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/fail.out" 2>&1 &&
  fail "task test exited 0 with a failed test in the run at opt-level 1 ($dir/fail.out)"
grep -qxF '2 test binaries: 4 passed, 1 failed, 1 ignored' "$dir/fail.out" ||
  fail "task test did not print the summed line with the failed test, 4 passed, 1 failed, 1 ignored (the failed one is not ignored) ($dir/fail.out)"
[ "$(grep -c '^run ' "$dir/stub-fail.log")" = 2 ] || fail "the run at opt-level 0 did not run after a failed run at opt-level 1 ($dir/stub-fail.log)"

# 4. The list of the run at opt-level 1 compiles: its log has a cargo Compiling line.
: > "$dir/stub-compile.log"
STUB_LIST_COMPILE=1 STUB_LOG="$root/$dir/stub-compile.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/compile.out" 2>&1 &&
  fail "task test exited 0 with a Compiling line in the log of the list at opt-level 1 ($dir/compile.out)"
grep -qF 'test: compiled 1 crate(s) inside the timed step, which the task build-tests builds before gate-time-start: swell (.out/test-list-opt1.txt.log)' "$dir/compile.out" ||
  fail "task test did not name the Compiling line of .out/test-list-opt1.txt.log ($dir/compile.out)"
[ ! -s "$dir/stub-compile.log" ] || fail "task test ran tests although its list at opt-level 1 compiled ($dir/stub-compile.log)"

echo "$name: ok"

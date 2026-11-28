#!/bin/sh
# Planted defects of the split of the step test over the binaries it moves to opt-level 1
# (Taskfile.yml, tasks test, test-split and test-opt1-vars-check; story
# gate-test-steps-hold-the-budget-on-the-slower-runner, item 5): the run at opt-level 1
# (TEST_OPT1_ARGS, the packages swell, swell-num and swell-test262 in the profile collect)
# holds the tests of swell::main the stack rule prints "other" and every test of the other
# binaries of those packages but for the pairs "<binary> <test>" of TEST_OPT0_TESTS, which stay
# in the run at opt-level 0 with every test of the other packages. Every test runs once, and a
# pair of TEST_OPT0_TESTS names, once, a test of a moved binary that the workspace holds.
# No test runs and nothing is built: the script plants a tree of its own
# (.out/gate-tests/<name>/run) with the repository's Taskfile.yml, whose TEST_OPT0_TESTS line
# it sets to a pair of its own, and taskfiles/, a crate swell whose tests/deep.rs holds a test
# that calls stack_with_room( and whose tests/plain.rs and tests/realm_padding.rs hold one test
# each (the second the test TEST_OPT1_PROFILE_TEST names), and a stub cargo first on PATH. The
# stub answers cargo nextest list from the default-filter line of the --config-file it is
# given (the names after "test(=" of swell::main, the packages after "package(=", the pairs
# "binary_id(=<binary>) & test(=<test>)" left out of them; a line "not (...)" is the run at
# opt-level 0, no file the workspace, in JSON) over nine tests of six binaries (swell::main, swell,
# swell-num::main, swell-num, swell-test262::main and swell-syntax::main), and logs each run.
# 1. The split as it is: task test exits 0 and prints "test: swell::main 2 at opt-level 1, 1 at
#    opt-level 0" and "test: 6 tests at opt-level 1, 3 at opt-level 0"; the list of the run at
#    opt-level 1 holds the tests of swell, swell-num, swell-num::main and the test of
#    swell-test262::main that TEST_OPT0_TESTS does not name, the list of the run at opt-level 0
#    the stack test, the pair and the test of swell-syntax::main.
# 2. The stub's list of the run at opt-level 1 drops swell-num::main num::adds: task test fails,
#    naming it in neither run, and the stub ran no test.
# 3. The stub's list of the run at opt-level 0 adds swell-test262::main run::prints_a_line,
#    which the run at opt-level 1 holds: task test fails, naming it in both runs, no test ran.
# 4. TEST_OPT0_TESTS names a pair the workspace's list does not hold
#    (swell-test262::main run::no_such_test): task test fails, naming the pair, no test ran.
# 5. TEST_OPT0_TESTS names, beside the pair of case 1, a test of a binary whose package
#    TEST_OPT1_ARGS does not name (swell-syntax::main sources::nests) and one of swell::main
#    (deep::on_the_promised_stack, at opt-level 0 by the stack rule): task test fails before it
#    lists the workspace, naming each as not of a moved binary (and not the pair of case 1),
#    no test ran.
# 6. TEST_OPT0_TESTS names the pair of case 1 twice: task test fails before it lists the
#    workspace, naming the pair a second time, no test ran.
# Run from the repository root: sh tests/gate/test-runs-each-test-of-a-moved-binary-once.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS STUB_DEFECT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-runs-each-test-of-a-moved-binary-once
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
  'all="swell::main deep::on_the_promised_stack|swell::main plain::outside_the_stack|swell::main realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on|swell heap::tests::a_unit|swell-num num::tests::rounds|swell-num::main num::adds|swell-test262::main run::at_the_nesting_limit|swell-test262::main run::prints_a_line|swell-syntax::main sources::nests"' \
  'if [ "$2" = list ] && [ "$fmt" = json-pretty ]; then' \
  '  printf "{\n  \"test-count\": %s,\n  \"rust-suites\": {\n" "$(printf "%s\n" "$all" | tr "|" "\n" | grep -c " ")"' \
  '  printf "%s\n" "$all" | tr "|" "\n" | while read -r bin test; do' \
  '    [ "$bin" = "${last:-}" ] || printf "    \"%s\": {\n      \"binary-id\": \"%s\",\n      \"testcases\": {\n" "$bin" "$bin"' \
  '    last=$bin' \
  '    printf "        \"%s\": {\n          \"kind\": \"test\",\n          \"ignored\": false,\n          \"filter-match\": {\n            \"status\": \"matches\"\n          }\n        },\n" "$test"' \
  '  done' \
  '  exit 0' \
  'fi' \
  'if [ "$2" = list ]; then' \
  '  printf "%s\n" "$all" | tr "|" "\n" | while read -r bin test; do' \
  '    if [ "$bin" = swell::main ]; then' \
  '      case " $names " in *" $test "*) one=yes ;; *) one=no ;; esac' \
  '    else' \
  '      case " $pkgs " in *" ${bin%%::*} "*) one=yes ;; *) one=no ;; esac' \
  '      case "|$pairs" in *"|$bin $test|"*) one=no ;; esac' \
  '    fi' \
  '    case $kind in' \
  '      none) echo "$bin $test" ;;' \
  '      only) if [ "$one" = yes ]; then [ "${STUB_DEFECT:-}" = neither ] && [ "$test" = num::adds ] || echo "$bin $test"; fi ;;' \
  '      not) if [ "$one" = no ]; then echo "$bin $test"; fi' \
  '           [ "${STUB_DEFECT:-}" = both ] && [ "$test" = run::prints_a_line ] && echo "$bin $test" ;;' \
  '    esac' \
  '  done' \
  '  exit 0' \
  'fi' \
  'printf "run profile %s guard %s config-%s\n" "$profile" "${SWELL_NO_STACK_TESTS:-unset}" "$kind" >> "$STUB_LOG"' \
  'mkdir -p target/nextest/default' \
  'printf "<testsuites><testsuite><testcase name=\"num::adds\" classname=\"swell-num::main\" time=\"0.100\"/></testsuite></testsuites>\n" > target/nextest/default/junit.xml' \
  'if [ "$kind" = only ]; then' \
  '  echo "    Starting 6 tests across 5 binaries (1 test and 1 binary skipped)"' \
  '  echo "     Summary [   0.100s] 6 tests run: 6 passed, 1 skipped"' \
  'else' \
  '  echo "    Starting 3 tests across 3 binaries (3 tests and 3 binaries skipped)"' \
  '  echo "     Summary [   0.100s] 3 tests run: 3 passed, 3 skipped"' \
  'fi' \
  'exit 0' > "$dir/bin/cargo" || fail "cannot write the stub cargo"
chmod +x "$dir/bin/cargo"

run="$dir/run"
mkdir -p "$run/crates/swell/tests/support" "$run/crates/swell/src" "$run/.out" "$run/.config" || fail "cannot make $run"
pair='swell-test262::main run::at_the_nesting_limit'
grep -q '^  TEST_OPT0_TESTS: ' Taskfile.yml || fail "Taskfile.yml has no line \"  TEST_OPT0_TESTS: <pairs>\" to plant a pair in"
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
printf '#[test]\nfn on_the_promised_stack() {\n    let _ = stack_with_room(1 << 20);\n}\n' > "$run/crates/swell/tests/deep.rs"
printf '#[test]\nfn outside_the_stack() {\n    assert_eq!(2 + 2, 4);\n}\n' > "$run/crates/swell/tests/plain.rs"
printf '#[test]\nfn the_profile_keeps_debug_assertions_and_overflow_checks_on() {\n    assert!(cfg!(debug_assertions));\n}\n' > "$run/crates/swell/tests/realm_padding.rs"

# 1. The split as it is.
: > "$dir/stub-split.log"
STUB_LOG="$root/$dir/stub-split.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/split.out" 2>&1 ||
  fail "task test exited non-zero on a split that runs each test once ($dir/split.out)"
grep -qxF 'test: swell::main 2 at opt-level 1, 1 at opt-level 0' "$dir/split.out" ||
  fail "task test did not print the tests of swell::main at each opt-level, 2 and 1 ($dir/split.out)"
grep -qxF 'test: 6 tests at opt-level 1, 3 at opt-level 0' "$dir/split.out" ||
  fail "task test did not print the tests of the workspace at each opt-level, 6 and 3 ($dir/split.out)"
for t in 'swell heap::tests::a_unit' 'swell-num num::tests::rounds' 'swell-num::main num::adds' 'swell-test262::main run::prints_a_line' 'swell::main plain::outside_the_stack'; do
  grep -qxF "$t" "$run/.out/test-list-opt1.txt" || fail "the run at opt-level 1 does not hold $t ($run/.out/test-list-opt1.txt)"
done
for t in "$pair" 'swell::main deep::on_the_promised_stack' 'swell-syntax::main sources::nests'; do
  grep -qxF "$t" "$run/.out/test-list-opt0.txt" || fail "the run at opt-level 0 does not hold $t ($run/.out/test-list-opt0.txt)"
done
grep -qxF 'run profile collect guard 1 config-only' "$dir/stub-split.log" ||
  fail "the run at opt-level 1 is not in the profile collect with SWELL_NO_STACK_TESTS=1 ($dir/stub-split.log)"
grep -qxF 'run profile test guard unset config-not' "$dir/stub-split.log" ||
  fail "the run at opt-level 0 is not in the test profile without SWELL_NO_STACK_TESTS ($dir/stub-split.log)"

# 2. A test of a moved binary in neither run.
: > "$dir/stub-neither.log"
STUB_DEFECT=neither STUB_LOG="$root/$dir/stub-neither.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/neither.out" 2>&1 &&
  fail "task test exited 0 with swell-num::main num::adds in neither run ($dir/neither.out)"
grep -qF 'test: swell-num::main num::adds is in neither run' "$dir/neither.out" ||
  fail "task test did not name the test of a moved binary in neither run ($dir/neither.out)"
[ ! -s "$dir/stub-neither.log" ] || fail "task test ran tests although a test is in neither run ($dir/stub-neither.log)"

# 3. A test of a moved binary in both runs.
: > "$dir/stub-both.log"
STUB_DEFECT=both STUB_LOG="$root/$dir/stub-both.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/both.out" 2>&1 &&
  fail "task test exited 0 with swell-test262::main run::prints_a_line in both runs ($dir/both.out)"
grep -qF 'test: swell-test262::main run::prints_a_line is in both runs' "$dir/both.out" ||
  fail "task test did not name the test of a moved binary in both runs ($dir/both.out)"
[ ! -s "$dir/stub-both.log" ] || fail "task test ran tests although a test is in both runs ($dir/stub-both.log)"

# 4. A pair of TEST_OPT0_TESTS that the workspace's list does not hold.
sed "s/^  TEST_OPT0_TESTS: .*\$/  TEST_OPT0_TESTS: $pair swell-test262::main run::no_such_test/" "$run/Taskfile.yml" > "$run/Taskfile.yml.new" &&
  mv "$run/Taskfile.yml.new" "$run/Taskfile.yml" || fail "cannot plant the pair run::no_such_test"
: > "$dir/stub-pair.log"
STUB_LOG="$root/$dir/stub-pair.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/pair.out" 2>&1 &&
  fail "task test exited 0 with TEST_OPT0_TESTS naming a test the workspace does not hold ($dir/pair.out)"
grep -qF 'swell-test262::main run::no_such_test' "$dir/pair.out" ||
  fail "task test did not name the pair of TEST_OPT0_TESTS that the workspace does not hold ($dir/pair.out)"
[ ! -s "$dir/stub-pair.log" ] || fail "task test ran tests although TEST_OPT0_TESTS names a test the workspace does not hold ($dir/stub-pair.log)"

# 5. Pairs whose binary is not a moved binary: one of a package TEST_OPT1_ARGS does not name,
#    one of swell::main (a test the stack rule prints "stack", at opt-level 0 without the pair).
sed "s/^  TEST_OPT0_TESTS: .*\$/  TEST_OPT0_TESTS: $pair swell-syntax::main sources::nests swell::main deep::on_the_promised_stack/" "$run/Taskfile.yml" > "$run/Taskfile.yml.new" &&
  mv "$run/Taskfile.yml.new" "$run/Taskfile.yml" || fail "cannot plant the pairs of binaries that do not move"
rm -f "$run/.out/test-list-all.json"
: > "$dir/stub-unmoved.log"
STUB_LOG="$root/$dir/stub-unmoved.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/unmoved.out" 2>&1 &&
  fail "task test exited 0 with TEST_OPT0_TESTS naming tests of binaries that do not move ($dir/unmoved.out)"
for t in 'swell-syntax::main sources::nests' 'swell::main deep::on_the_promised_stack'; do
  grep -qF "test: TEST_OPT0_TESTS names $t, whose binary is not a moved binary" "$dir/unmoved.out" ||
    fail "task test did not refuse the pair $t, whose binary is not a moved binary ($dir/unmoved.out)"
done
grep -qF "names $pair, whose" "$dir/unmoved.out" && fail "task test refused the pair $pair of a moved binary ($dir/unmoved.out)"
[ ! -e "$run/.out/test-list-all.json" ] || fail "task test listed the workspace although TEST_OPT0_TESTS names tests of binaries that do not move ($run/.out/test-list-all.json)"
[ ! -s "$dir/stub-unmoved.log" ] || fail "task test ran tests although TEST_OPT0_TESTS names tests of binaries that do not move ($dir/stub-unmoved.log)"

# 6. A pair named twice: the second changes nothing.
sed "s/^  TEST_OPT0_TESTS: .*\$/  TEST_OPT0_TESTS: $pair $pair/" "$run/Taskfile.yml" > "$run/Taskfile.yml.new" &&
  mv "$run/Taskfile.yml.new" "$run/Taskfile.yml" || fail "cannot plant the pair twice"
rm -f "$run/.out/test-list-all.json"
: > "$dir/stub-twice.log"
STUB_LOG="$root/$dir/stub-twice.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/twice.out" 2>&1 &&
  fail "task test exited 0 with TEST_OPT0_TESTS naming $pair twice ($dir/twice.out)"
grep -qF "test: TEST_OPT0_TESTS names $pair a second time: the pair changes nothing" "$dir/twice.out" ||
  fail "task test did not refuse the pair named a second time ($dir/twice.out)"
[ ! -e "$run/.out/test-list-all.json" ] || fail "task test listed the workspace although TEST_OPT0_TESTS names a pair twice ($run/.out/test-list-all.json)"
[ ! -s "$dir/stub-twice.log" ] || fail "task test ran tests although TEST_OPT0_TESTS names a pair twice ($dir/stub-twice.log)"

echo "$name: ok"

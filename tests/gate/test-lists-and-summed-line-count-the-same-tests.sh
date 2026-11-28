#!/bin/sh
# Planted cases of the split of the step test and of its summed line (Taskfile.yml, tasks
# test, test-split and test-sum; story gate-test-steps-hold-the-budget-on-the-slower-runner,
# item 5): the lists of the two runs hold the tests that run, so in the line "test: <n> tests at
# opt-level 1, <m> at opt-level 0" n plus m is p plus f of the summed line, and p plus f plus i
# is the count of tests of the list of the workspace without a filter (its JSON's test-count,
# the ignored tests among them).
# No test runs and nothing is built: the script plants a tree of its own
# (.out/gate-tests/<name>/run) with the repository's Taskfile.yml, whose TEST_OPT0_TESTS line
# it sets to a pair of its own, and taskfiles/, a crate swell whose tests/deep.rs holds a test
# that calls stack_with_room( and whose tests/plain.rs and tests/realm_padding.rs hold one test
# each, and a stub cargo first on PATH. The stub answers cargo nextest list from the
# default-filter line of the --config-file it is given, as the step's filter reads, over eight
# tests of five binaries, and in JSON over those, two ignored tests (one of the moved binary
# swell-num::main, one of swell-syntax::main) and a binary without tests (swell-unicode): ten
# tests of six binaries. The library swell and swell::main each hold a test
# plain::outside_the_stack.
# 1. The split as it is (pair swell-test262::main run::at_the_nesting_limit): task test exits 0,
#    prints "test: 5 tests at opt-level 1, 3 at opt-level 0" and "6 test binaries: 8 passed, 0
#    failed, 2 ignored" (b counts the binary without tests, i each ignored test once), both
#    tests plain::outside_the_stack are in the run at opt-level 1, n plus m is p plus f, and p
#    plus f plus i is the test-count of the JSON list (10).
# 2. TEST_OPT0_TESTS names the library's test of the shared name (swell
#    plain::outside_the_stack): it runs at opt-level 0 and the one of swell::main at opt-level 1.
# 3. The JSON list of the workspace exits 0 and prints nothing: task test fails, saying it
#    cannot read the JSON, and runs no test.
# 4. The list of the run at opt-level 1 exits 0 and prints nothing: task test fails, naming a
#    moved test in neither run, and runs no test.
# Run from the repository root: sh tests/gate/test-lists-and-summed-line-count-the-same-tests.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS STUB_JSON_EMPTY STUB_OPT1_EMPTY
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-lists-and-summed-line-count-the-same-tests
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/bin" || { echo "$name: FAIL cannot make $dir"; exit 1; }
root=$(pwd)
bad=0

fail() {
  echo "$name: FAIL $*"
  exit 1
}

red() {
  echo "$name: FAIL $*"
  bad=1
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
  'if [ "$2" = list ] && [ "$fmt" = json-pretty ]; then' \
  '  [ "${STUB_JSON_EMPTY:-}" = 1 ] && exit 0' \
  '  cat "$STUB_JSON"' \
  '  exit 0' \
  'fi' \
  'if [ "$2" = list ]; then' \
  '  [ "${STUB_OPT1_EMPTY:-}" = 1 ] && [ "$kind" = only ] && exit 0' \
  '  while read -r bin test; do' \
  '    [ -n "$bin" ] || continue' \
  '    if [ "$bin" = swell::main ]; then' \
  '      case " $names " in *" $test "*) one=yes ;; *) one=no ;; esac' \
  '    else' \
  '      case " $pkgs " in *" ${bin%%::*} "*) one=yes ;; *) one=no ;; esac' \
  '      case "|$pairs" in *"|$bin $test|"*) one=no ;; esac' \
  '    fi' \
  '    case $kind in' \
  '      none) echo "$bin $test" ;;' \
  '      only) if [ "$one" = yes ]; then echo "$bin $test"; fi ;;' \
  '      not) if [ "$one" = no ]; then echo "$bin $test"; fi ;;' \
  '    esac' \
  '  done < "$STUB_ALL"' \
  '  exit 0' \
  'fi' \
  'printf "run profile %s config-%s\n" "$profile" "$kind" >> "$STUB_LOG"' \
  'n=$("$0" nextest list --message-format oneline --config-file "$cfg" 2>/dev/null | grep -c " ")' \
  'mkdir -p target/nextest/default' \
  'printf "<testsuites><testsuite><testcase name=\"run::%s\" classname=\"stub\" time=\"0.100\"/></testsuite></testsuites>\n" "$kind" > target/nextest/default/junit.xml' \
  'echo "    Starting $n tests across 2 binaries"' \
  'echo "     Summary [   0.100s] $n tests run: $n passed, 0 skipped"' \
  'exit 0' > "$dir/bin/cargo" || fail "cannot write the stub cargo"
chmod +x "$dir/bin/cargo"

printf '%s\n' \
  'swell::main deep::on_the_promised_stack' \
  'swell::main plain::outside_the_stack' \
  'swell::main realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on' \
  'swell plain::outside_the_stack' \
  'swell-num::main num::adds' \
  'swell-test262::main run::at_the_nesting_limit' \
  'swell-test262::main run::prints_a_line' \
  'swell-syntax::main sources::nests' > "$dir/all.txt" || fail "cannot write $dir/all.txt"

suite() {
  printf '    "%s": {\n      "package-name": "%s",\n      "binary-id": "%s",\n      "kind": "test",\n      "status": "listed",\n      "testcases": {\n' "$1" "${1%%::*}" "$1"
}
case_() {
  if [ "$2" = true ]; then
    printf '        "%s": {\n          "kind": "test",\n          "ignored": true,\n          "filter-match": {\n            "status": "mismatch",\n            "reason": "ignored"\n          }\n        },\n' "$1"
  else
    printf '        "%s": {\n          "kind": "test",\n          "ignored": false,\n          "filter-match": {\n            "status": "matches"\n          }\n        },\n' "$1"
  fi
}
end_suite() {
  printf '      }\n    },\n'
}
{
  printf '{\n  "rust-build-meta": {\n    "non-test-binaries": {}\n  },\n  "test-count": 10,\n  "rust-suites": {\n'
  suite swell
  case_ plain::outside_the_stack false
  end_suite
  suite swell::main
  case_ deep::on_the_promised_stack false
  case_ plain::outside_the_stack false
  case_ realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on false
  end_suite
  suite swell-num::main
  case_ num::adds false
  case_ num::ignored_in_a_moved_binary true
  end_suite
  suite swell-syntax::main
  case_ sources::ignored_in_a_binary_that_stays true
  case_ sources::nests false
  end_suite
  suite swell-test262::main
  case_ run::at_the_nesting_limit false
  case_ run::prints_a_line false
  end_suite
  printf '    "swell-unicode": {\n      "package-name": "swell-unicode",\n      "binary-id": "swell-unicode",\n      "kind": "lib",\n      "status": "listed",\n      "testcases": {}\n    }\n  }\n}\n'
} > "$dir/all.json" || fail "cannot write $dir/all.json"
total=$(sed -n 's/^  "test-count": \([0-9]*\),$/\1/p' "$dir/all.json")
[ "$total" = 10 ] || fail "the planted JSON list does not hold a test-count of 10 ($dir/all.json)"

run="$dir/run"
mkdir -p "$run/crates/swell/tests/support" "$run/crates/swell/src" "$run/.out" "$run/.config" || fail "cannot make $run"
grep -q '^  TEST_OPT0_TESTS: ' Taskfile.yml || fail "Taskfile.yml has no line \"  TEST_OPT0_TESTS: <pairs>\" to plant a pair in"
cp -R "$root/taskfiles" "$run/taskfiles" || fail "cannot copy taskfiles"
cp "$root/.config/nextest.toml" "$run/.config/" || fail "cannot copy .config/nextest.toml"
mkdir -p "$run/spec" "$run/conformance/test262" || fail "cannot make $run/spec"
cp "$root/spec/ess-inputs.yaml" "$run/spec/" || fail "cannot copy spec/ess-inputs.yaml"
cp "$root/conformance/test262/PIN" "$run/conformance/test262/" || fail "cannot copy conformance/test262/PIN"
printf '[package]\nname = "swell"\n' > "$run/crates/swell/Cargo.toml"
printf 'mod deep;\nmod plain;\nmod realm_padding;\nmod support;\n' > "$run/crates/swell/tests/main.rs"
printf 'pub fn stack_with_room(bytes: usize) -> usize {\n    bytes / 8 * 7\n}\n' > "$run/crates/swell/tests/support/host.rs"
printf '#[test]\nfn on_the_promised_stack() {\n    let _ = stack_with_room(1 << 20);\n}\n' > "$run/crates/swell/tests/deep.rs"
printf '#[test]\nfn outside_the_stack() {\n    assert_eq!(2 + 2, 4);\n}\n' > "$run/crates/swell/tests/plain.rs"
printf '#[test]\nfn the_profile_keeps_debug_assertions_and_overflow_checks_on() {\n    assert!(cfg!(debug_assertions));\n}\n' > "$run/crates/swell/tests/realm_padding.rs"

plant_pair() {
  sed "s/^  TEST_OPT0_TESTS: .*\$/  TEST_OPT0_TESTS: $1/" "$root/Taskfile.yml" > "$run/Taskfile.yml" || fail "cannot copy Taskfile.yml"
  grep -qxF "  TEST_OPT0_TESTS: $1" "$run/Taskfile.yml" || fail "the planted Taskfile.yml does not hold the pair $1"
}

step() {
  label=$1
  shift
  : > "$dir/stub-$label.log"
  env "$@" STUB_JSON="$root/$dir/all.json" STUB_ALL="$root/$dir/all.txt" STUB_LOG="$root/$dir/stub-$label.log" PATH="$root/$dir/bin:$PATH" task -d "$run" test > "$dir/$label.out" 2>&1
}

# 1. The split as it is.
plant_pair 'swell-test262::main run::at_the_nesting_limit'
if step split STUB_NONE=1; then
  grep -qxF 'test: 5 tests at opt-level 1, 3 at opt-level 0' "$dir/split.out" ||
    red "1: task test did not print 5 tests at opt-level 1 and 3 at opt-level 0 ($dir/split.out)"
  grep -qxF '6 test binaries: 8 passed, 0 failed, 2 ignored' "$dir/split.out" ||
    red "1: task test did not print 6 binaries (the one without tests among them), 8 passed and the 2 ignored tests ($dir/split.out)"
  for t in 'swell plain::outside_the_stack' 'swell::main plain::outside_the_stack'; do
    grep -qxF "$t" "$run/.out/test-list-opt1.txt" || red "1: the run at opt-level 1 does not hold $t ($run/.out/test-list-opt1.txt)"
  done
  nm=$(sed -n 's/^test: \([0-9]*\) tests at opt-level 1, \([0-9]*\) at opt-level 0$/\1 \2/p' "$dir/split.out")
  pfi=$(sed -n 's/^[0-9]* test binaries: \([0-9]*\) passed, \([0-9]*\) failed, \([0-9]*\) ignored$/\1 \2 \3/p' "$dir/split.out")
  set -- $nm $pfi
  if [ "$#" -ne 5 ]; then
    red "1: task test did not print both lines to compare ($dir/split.out)"
  else
    [ $(($1 + $2)) -eq $(($3 + $4)) ] ||
      red "1: n plus m of \"test: $1 tests at opt-level 1, $2 at opt-level 0\" is $(($1 + $2)), not p plus f of the summed line ($3 + $4 = $(($3 + $4))): the lists hold the tests that run ($dir/split.out)"
    [ $(($3 + $4 + $5)) -eq "$total" ] ||
      red "1: p plus f plus i of the summed line ($3 + $4 + $5 = $(($3 + $4 + $5))) is not the count of tests of the list without a filter, $total ($dir/all.json, $dir/split.out)"
  fi
else
  red "1: task test exited non-zero on a split that runs each test once ($dir/split.out)"
fi

# 2. The library's test of the shared name stays at opt-level 0, the one of swell::main moves.
plant_pair 'swell plain::outside_the_stack'
if step shared STUB_NONE=1; then
  grep -qxF 'swell plain::outside_the_stack' "$run/.out/test-list-opt0.txt" ||
    red "2: the run at opt-level 0 does not hold the pair swell plain::outside_the_stack ($run/.out/test-list-opt0.txt)"
  grep -qxF 'swell::main plain::outside_the_stack' "$run/.out/test-list-opt1.txt" ||
    red "2: the run at opt-level 1 does not hold swell::main plain::outside_the_stack ($run/.out/test-list-opt1.txt)"
else
  red "2: task test exited non-zero with the pair swell plain::outside_the_stack ($dir/shared.out)"
fi

# 3. The JSON list of the workspace prints nothing.
plant_pair 'swell-test262::main run::at_the_nesting_limit'
if step json STUB_JSON_EMPTY=1; then
  red "3: task test exited 0 with an empty JSON list of the workspace ($dir/json.out)"
else
  grep -qF 'test: cannot read the JSON of cargo nextest list' "$dir/json.out" ||
    red "3: task test did not say it cannot read the JSON ($dir/json.out)"
  [ ! -s "$dir/stub-json.log" ] || red "3: task test ran tests although the JSON list was empty ($dir/stub-json.log)"
fi

# 4. The list of the run at opt-level 1 prints nothing.
if step opt1 STUB_OPT1_EMPTY=1; then
  red "4: task test exited 0 with an empty list of the run at opt-level 1 ($dir/opt1.out)"
else
  grep -qF 'test: swell-num::main num::adds is in neither run' "$dir/opt1.out" ||
    red "4: task test did not name a moved test in neither run ($dir/opt1.out)"
  [ ! -s "$dir/stub-opt1.log" ] || red "4: task test ran tests although the list at opt-level 1 was empty ($dir/stub-opt1.log)"
fi

[ "$bad" = 0 ] || exit 1
echo "$name: ok"

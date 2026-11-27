#!/bin/sh
# Planted defect of the task slow-tests (Taskfile.yml; story realm-size-lane-within-its-share):
# the collect mode of slow-tests runs in two parts, the tests of swell::main that run on a
# bounded stack (taskfiles/realm-room-tests.awk -v rule=stack prints them "stack") and the
# library's unit tests in the test profile at opt-level 0, the others in the profile collect at
# opt-level 1. A stack test proves the promised stack with frames whose size depends on the
# optimization level, so a run that put it at opt-level 1 only would prove less, and say nothing.
# No test runs: the script plants a tree of its own (.out/gate-tests/<name>/run-<case>) with the
# repository's Taskfile.yml and taskfiles/, a crate swell whose tests/deep.rs reads
# SWELL_FULL_COUNTS and holds two tests, and a stub cargo first on PATH that logs each run's
# profile, its -E filter and the names of the default-filter line of its --config-file, and
# answers with a Starting and a Summary line and a JUnit file.
# 1. deep::runs_on_the_promised_stack calls stack_with_room( from a function nested in it (its
#    body passed by value, as the stack tests of swell do), deep::runs_on_any_stack does not:
#    task slow-tests exits 0, prints "1 of 2 tests of swell::main run on a bounded stack",
#    the collect run at opt-level 0 leaves out deep::runs_on_any_stack alone, and the run with
#    --cargo-profile collect and --test main leaves out deep::runs_on_the_promised_stack alone.
# 2. The planted defect: no test reaches stack_with_room (the module names it in a function no
#    test calls), so the rule prints no test "stack" and every test of the module would run at
#    opt-level 1 only: the task fails, saying it finds no test that runs on a bounded stack,
#    and the stub ran no collect run.
# Run from the repository root: sh tests/gate/slow-tests-runs-the-stack-tests-at-opt-level-0.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS
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

printf '%s\n' '#!/bin/sh' \
  'profile=test; prev=""; filter=""; cfg=""' \
  'for a in "$@"; do' \
  '  [ "$prev" = "--cargo-profile" ] && profile=$a' \
  '  [ "$prev" = "-E" ] && filter=$a' \
  '  [ "$prev" = "--config-file" ] && cfg=$a' \
  '  prev=$a' \
  'done' \
  'case " $* " in *" --test main "*) target=main ;; *) target=all ;; esac' \
  'case " $* " in *" collect-every-safe-point "*) mode=collect ;; *) mode=default ;; esac' \
  'names=none; [ -n "$cfg" ] && names=$(grep "^default-filter = " "$cfg" | grep -o "test(=[^)]*)" | tr "\n" " ")' \
  'printf "run %s profile %s target %s filter %s config-not %s\n" "$mode" "$profile" "$target" "$filter" "$names" >> "$STUB_LOG"' \
  'mkdir -p target/nextest/default' \
  'printf "<testsuites><testsuite><testcase name=\"deep::t\" classname=\"swell::main\" time=\"0.1\"/></testsuite></testsuites>\n" > target/nextest/default/junit.xml' \
  'echo "    Starting 2 tests across 1 binary"' \
  'echo "     Summary [   0.100s] 2 tests run: 2 passed, 0 skipped"' \
  'exit 0' > "$dir/bin/cargo" || fail "cannot write the stub cargo"
chmod +x "$dir/bin/cargo"

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
  printf '// nothing\n' > "$run/crates/swell/src/lib.rs"
  printf 'mod deep;\nmod support;\n' > "$run/crates/swell/tests/main.rs"
  printf 'pub fn stack_with_room(bytes: usize) -> usize {\n    bytes / 8 * 7\n}\n' > "$run/crates/swell/tests/support/host.rs"
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
  '}' > "$dir/run-split/crates/swell/tests/deep.rs"
: > "$dir/stub-split.log"
STUB_LOG="$root/$dir/stub-split.log" PATH="$root/$dir/bin:$PATH" task -d "$dir/run-split" slow-tests > "$dir/split.out" 2>&1 ||
  fail "task slow-tests exited non-zero on a module with one stack test and one other ($dir/split.out)"
grep -qF 'slow-tests collect-every-safe-point: 1 of 2 tests of swell::main run on a bounded stack' "$dir/split.out" ||
  fail "slow-tests did not count 1 of 2 tests on a bounded stack ($dir/split.out)"
low=$(grep '^run collect profile test ' "$dir/stub-split.log")
high=$(grep '^run collect profile collect ' "$dir/stub-split.log")
[ "$(printf '%s\n' "$low" | grep -c .)" = 1 ] || fail "the stub saw not one collect run in the test profile ($dir/stub-split.log)"
[ "$(printf '%s\n' "$high" | grep -c .)" = 1 ] || fail "the stub saw not one collect run in the profile collect ($dir/stub-split.log)"
case "$low" in *"config-not test(=deep::runs_on_any_stack) "*) ;; *) fail "the collect run at opt-level 0 does not leave out deep::runs_on_any_stack alone: $low" ;; esac
case "$high" in *"target main "*"config-not test(=deep::runs_on_the_promised_stack) "*) ;; *) fail "the collect run at opt-level 1 does not run --test main and leave out deep::runs_on_the_promised_stack alone: $high" ;; esac
grep -q '^slow-tests collect-every-safe-point opt-level 1: 1 test binaries: 2 passed' "$dir/split.out" ||
  fail "slow-tests printed no count of its run at opt-level 1 ($dir/split.out)"

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
: > "$dir/stub-none.log"
if STUB_LOG="$root/$dir/stub-none.log" PATH="$root/$dir/bin:$PATH" task -d "$dir/run-none" slow-tests > "$dir/none.out" 2>&1; then
  fail "task slow-tests exited 0 when no test of swell::main runs on a bounded stack ($dir/none.out)"
fi
grep -qF 'slow-tests: taskfiles/realm-room-tests.awk -v rule=stack finds no test of swell::main that runs on a bounded stack' "$dir/none.out" ||
  fail "slow-tests did not say it finds no test on a bounded stack ($dir/none.out)"
! grep -q '^run collect ' "$dir/stub-none.log" || fail "the stub ran a collect run although no test runs on a bounded stack ($dir/stub-none.log)"

echo "$name: ok"

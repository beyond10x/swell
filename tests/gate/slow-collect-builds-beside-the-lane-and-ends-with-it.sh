#!/bin/sh
# Planted defects of the background build of the lane slow (Taskfile.yml, tasks slow,
# slow-collect-start, slow-collect-stop and slow-tests; story realm-size-lane-within-its-share):
# the lane starts the build of the profile collect right after gate-time-start, beside
# slow-test262, and slow-tests waits for it (SLOW_COLLECT=lane).
# No test runs: the script plants a tree of its own (.out/gate-tests/<name>/run-<case>) with the
# repository's Taskfile.yml and taskfiles/, a crate swell whose tests/deep.rs reads
# SWELL_FULL_COUNTS and holds a stack test and another, and a stub cargo first on PATH. The stub
# logs each call ("build" for --no-run in the profile collect, "run <profile>" otherwise) and
# answers a run with a Starting and a Summary line and a JUnit file; the build does what STUB_BUILD
# says: ok (a Finished line after a second), fail (exit 3) or hang (writes its process number to
# build.pid and sleeps 120 s).
# 1. ok: task slow-collect-start slow-tests SLOW_COLLECT=lane, one task process as in the lane,
#    exits 0, prints "slow-build collect <s> s, exit 0", and the stub's run in the profile
#    collect comes after the build.
# 2. fail: the same exits non-zero, prints the build's log and "slow-tests: the build of the
#    profile collect that the lane started failed, exit 3", and the stub ran nothing in the
#    profile collect: a lane whose build failed must not pass.
# 3. hang: (a) task slow, with a test262 pin whose URL does not exist so that slow-test262 fails
#    at once, exits non-zero, prints "slow-collect-stop: ended the build", and the build's
#    process group (.out/slow-collect/pid) is gone when task returns; (b) task
#    slow-collect-start alone (its task process ends at once, as an interrupted lane does):
#    the build's process group is gone within 5 s.
# 4. task slow-tests alone, with no .out/slow-collect: it exits 0 and the stub ran the profile
#    collect itself (no --no-run build beside it): a part run alone builds what it needs.
# Run from the repository root: sh tests/gate/slow-collect-builds-beside-the-lane-and-ends-with-it.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SLOW_COLLECT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=slow-collect-builds-beside-the-lane-and-ends-with-it
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
  'profile=test; prev=""; norun=0' \
  'for a in "$@"; do [ "$prev" = "--cargo-profile" ] && profile=$a; [ "$a" = "--no-run" ] && norun=1; prev=$a; done' \
  'if [ "$norun" = 1 ] && [ "$profile" = collect ]; then' \
  '  echo "build start" >> "$STUB_DIR/calls"' \
  '  case "$STUB_BUILD" in' \
  '    fail) echo "error: planted build failure"; exit 3 ;;' \
  '    hang) echo $$ > "$STUB_DIR/build.pid"; sleep 120; exit 0 ;;' \
  '  esac' \
  '  sleep 1; echo "build end" >> "$STUB_DIR/calls"' \
  '  echo "    Finished \`collect\` profile [optimized] target(s) in 1.00s"; exit 0' \
  'fi' \
  'if [ "$norun" = 1 ]; then echo "    Finished \`test\` profile [unoptimized] target(s) in 0.00s"; exit 0; fi' \
  'echo "run $profile" >> "$STUB_DIR/calls"' \
  'mkdir -p target/nextest/default' \
  'printf "<testsuites><testsuite><testcase name=\"deep::t\" classname=\"swell::main\" time=\"0.1\"/></testsuite></testsuites>\n" > target/nextest/default/junit.xml' \
  'echo "    Starting 2 tests across 1 binary"' \
  'echo "     Summary [   0.100s] 2 tests run: 2 passed, 0 skipped"' \
  'exit 0' > "$dir/bin/cargo" || fail "cannot write the stub cargo"
chmod +x "$dir/bin/cargo"

plant() {
  run="$dir/run-$1"
  mkdir -p "$run/crates/swell/tests/support" "$run/crates/swell/src" "$run/.out" "$run/.config" "$run/stub" || fail "cannot make $run"
  cp "$root/Taskfile.yml" "$run/Taskfile.yml" || fail "cannot copy Taskfile.yml"
  cp -R "$root/taskfiles" "$run/taskfiles" || fail "cannot copy taskfiles"
  cp "$root/.config/nextest.toml" "$run/.config/" || fail "cannot copy .config/nextest.toml"
  mkdir -p "$run/spec" "$run/conformance/test262" || fail "cannot make $run/spec"
  cp "$root/spec/ess-inputs.yaml" "$run/spec/" || fail "cannot copy spec/ess-inputs.yaml"
  awk '$1 == "url" { print "url file:///nonexistent/test262"; next } { print }' "$root/conformance/test262/PIN" > "$run/conformance/test262/PIN" ||
    fail "cannot write the planted PIN"
  printf '[package]\nname = "swell"\n' > "$run/crates/swell/Cargo.toml"
  printf '// nothing\n' > "$run/crates/swell/src/lib.rs"
  printf 'mod deep;\nmod support;\n' > "$run/crates/swell/tests/main.rs"
  printf 'pub fn stack_with_room(bytes: usize) -> usize {\n    bytes / 8 * 7\n}\n' > "$run/crates/swell/tests/support/host.rs"
  printf '%s\n' \
    'fn count() -> u32 {' \
    '    if std::env::var("SWELL_FULL_COUNTS").is_ok() { 1000 } else { 10 }' \
    '}' \
    '' \
    '#[test]' \
    'fn runs_on_the_promised_stack() {' \
    '    let _ = stack_with_room(1 << 20);' \
    '}' \
    '' \
    '#[test]' \
    'fn runs_on_any_stack() {' \
    '    assert!(count() > 0);' \
    '}' > "$run/crates/swell/tests/deep.rs"
  : > "$run/stub/calls"
}

# go <case> <STUB_BUILD> <task arguments...>: run task in the planted tree of <case>.
go() {
  case_=$1 build=$2
  shift 2
  STUB_DIR="$root/$dir/run-$case_/stub" STUB_BUILD=$build PATH="$root/$dir/bin:$PATH" task -d "$dir/run-$case_" "$@" > "$dir/$case_.out" 2>&1
}

# group_gone_within <pgid> <tenths>: no process of the group <pgid> is left within <tenths> tenths of a second.
group_gone_within() {
  n=0
  while kill -0 -- "-$1" 2>/dev/null; do
    [ "$n" -ge "$2" ] && return 1
    sleep 0.1
    n=$((n + 1))
  done
  return 0
}

# 1. The build passes: the lane's order holds.
plant ok
go ok ok slow-collect-start slow-tests SLOW_COLLECT=lane || fail "the lane's slow-collect-start and slow-tests exited non-zero on a build that passed ($dir/ok.out)"
grep -q '^slow-build collect [0-9.]* s, exit 0, waited ' "$dir/ok.out" || fail "slow-tests printed no line slow-build collect ... exit 0 ($dir/ok.out)"
awk '$0 == "build end" { b = NR } $0 == "run collect" { r = NR } END { exit !(b && r && b < r) }' "$dir/run-ok/stub/calls" ||
  fail "the run in the profile collect did not come after the background build ($dir/run-ok/stub/calls)"

# 2. The build fails: the lane must fail.
plant fail
if go fail fail slow-collect-start slow-tests SLOW_COLLECT=lane; then
  fail "the lane's slow-tests exited 0 when the background build of the profile collect failed ($dir/fail.out)"
fi
grep -qF 'slow-tests: the build of the profile collect that the lane started failed, exit 3 (.out/slow-collect/log)' "$dir/fail.out" ||
  fail "slow-tests did not name the failed build ($dir/fail.out)"
grep -qF 'error: planted build failure' "$dir/fail.out" || fail "slow-tests did not print the build's log ($dir/fail.out)"
! grep -q '^run collect$' "$dir/run-fail/stub/calls" || fail "the stub ran the profile collect after the build failed ($dir/run-fail/stub/calls)"

# 3a. The lane fails in its first part while the build runs: no build outlives task slow.
plant hang
if go hang hang slow; then
  fail "task slow exited 0 with a test262 pin that cannot be fetched ($dir/hang.out)"
fi
pg=$(cat "$dir/run-hang/.out/slow-collect/pid" 2>/dev/null) || fail "the lane started no background build ($dir/hang.out)"
group_gone_within "$pg" 0 || { kill -KILL -- "-$pg" 2>/dev/null; fail "the build (process group $pg) still ran when task slow ended ($dir/hang.out)"; }
grep -qF 'slow-collect-stop: ended the build of the profile collect' "$dir/hang.out" ||
  fail "task slow ended without slow-collect-stop ending the build ($dir/hang.out)"

# 3b. The task process that started the build is gone: the watcher ends the build.
plant alone
go alone hang slow-collect-start || fail "task slow-collect-start exited non-zero ($dir/alone.out)"
pg=$(cat "$dir/run-alone/.out/slow-collect/pid" 2>/dev/null) || fail "slow-collect-start started no build ($dir/alone.out)"
group_gone_within "$pg" 50 || { kill -KILL -- "-$pg" 2>/dev/null; fail "the build (process group $pg) still ran 5 s after the task process that started it ended ($dir/alone.out)"; }

# 4. slow-tests alone builds the profile collect itself.
plant part
go part hang slow-tests || fail "task slow-tests run alone exited non-zero with no background build ($dir/part.out)"
grep -q '^run collect$' "$dir/run-part/stub/calls" || fail "task slow-tests run alone did not run the profile collect ($dir/run-part/stub/calls)"
! grep -q '^build ' "$dir/run-part/stub/calls" || fail "task slow-tests run alone started a build beside it ($dir/run-part/stub/calls)"

echo "$name: ok"

#!/bin/sh
# Planted defects of the background builds of the lane slow (Taskfile.yml, tasks slow,
# slow-collect-start, slow-collect-stop and slow-tests; stories realm-size-lane-within-its-share
# and slow-lane-holds-its-bound-on-the-slower-runner, step 2): the lane starts three builds at
# opt-level 1 right after gate-time-start, one after the other beside slow-test262 (opt1 with
# TEST_OPT1_ARGS, lib with the library's unit tests in the collect mode, collect with
# COLLECT_MAIN), and slow-tests waits for each where it first needs it (SLOW_COLLECT=lane).
# No test runs: the script plants a tree of its own (.out/gate-tests/<name>/run-<case>) with the
# repository's Taskfile.yml and taskfiles/, a crate swell whose library and tests/deep.rs read
# SWELL_FULL_COUNTS, deep.rs holding a stack test and another, tests/realm_padding.rs holding
# the test TEST_OPT1_PROFILE_TEST names, and the stub cargo of tests/gate/fixtures/cargo-stub.sh
# first on PATH, which logs each build ("build start" and "build end") and each run; a build in
# the profile collect sleeps a second (STUB_BUILD_SECS), or fails (exit 3) or hangs (writes its
# process number to build.pid and sleeps 120 s) as STUB_BUILD says.
# 1. ok: task slow-collect-start slow-tests SLOW_COLLECT=lane, one task process as in the lane,
#    exits 0, prints "slow-build <build> <s> s, exit 0" for opt1, lib and collect, the stub saw
#    those three builds in the profile collect and no other (slow-tests builds none of them in
#    the lane), and each run in the profile collect comes after the end of the build it reads.
# 2. fail: the same exits non-zero, prints the build's log and "slow-tests: the build opt1 that
#    the lane started failed, exit 3", and the stub ran nothing in the profile collect: a lane
#    whose build failed must not pass.
# 3. hang: (a) task slow, with a test262 pin whose URL does not exist so that slow-test262 fails
#    at once, exits non-zero, prints "slow-collect-stop: ended the builds", and the builds'
#    process group (.out/slow-collect/pid) is gone when task returns, and the output holds
#    exactly one line "slow-runner: <n> cores, ..." (task slow-runner); (b) task
#    slow-collect-start alone (its task process ends at once, as an interrupted lane does):
#    the builds' process group is gone within 5 s.
# 4. task slow-tests alone, with no .out/slow-collect: it exits 0, builds opt1 and lib itself,
#    each before the run that reads it, and starts no build of COLLECT_MAIN (its run builds
#    what it needs): a part run alone builds what it needs.
# 5. .out/slow-collect/pid names a process group of setsid sleep 60 (not a watcher of this
#    tree's lane) and no file done: task slow-collect-stop leaves the group alone and says so.
# 6. PATH without setsid: task slow-collect-start fails with "setsid is not on PATH".
# Run from the repository root: sh tests/gate/slow-collect-builds-beside-the-lane-and-ends-with-it.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SLOW_COLLECT STUB_BUILD STUB_BUILD_MATCH STUB_COMPILE STUB_DROP STUB_DOUBLE STUB_FAIL
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
cp tests/gate/fixtures/cargo-stub.sh "$dir/bin/cargo" && cp tests/gate/fixtures/nextest-filter.awk "$dir/bin/" && chmod +x "$dir/bin/cargo" ||
  fail "cannot copy the stub cargo of tests/gate/fixtures"

plant() {
  run="$dir/run-$1"
  mkdir -p "$run/crates/swell/tests/support" "$run/crates/swell/src" "$run/.out" "$run/.config" || fail "cannot make $run"
  cp "$root/Taskfile.yml" "$run/Taskfile.yml" || fail "cannot copy Taskfile.yml"
  cp -R "$root/taskfiles" "$run/taskfiles" || fail "cannot copy taskfiles"
  cp "$root/.config/nextest.toml" "$run/.config/" || fail "cannot copy .config/nextest.toml"
  mkdir -p "$run/spec" "$run/conformance/test262" || fail "cannot make $run/spec"
  cp "$root/spec/ess-inputs.yaml" "$run/spec/" || fail "cannot copy spec/ess-inputs.yaml"
  awk '$1 == "url" { print "url file:///nonexistent/test262"; next } { print }' "$root/conformance/test262/PIN" > "$run/conformance/test262/PIN" ||
    fail "cannot write the planted PIN"
  printf '[package]\nname = "swell"\n' > "$run/crates/swell/Cargo.toml"
  printf '// a unit test reads SWELL_FULL_COUNTS\n' > "$run/crates/swell/src/lib.rs"
  printf 'mod deep;\nmod realm_padding;\nmod support;\n' > "$run/crates/swell/tests/main.rs"
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
  printf '#[test]\nfn the_profile_keeps_debug_assertions_and_overflow_checks_on() {\n    assert!(cfg!(debug_assertions));\n}\n' > "$run/crates/swell/tests/realm_padding.rs"
  printf '%s\n' \
    'swell unit::reads_full_counts' \
    'swell::main deep::runs_on_the_promised_stack' \
    'swell::main deep::runs_on_any_stack' \
    'swell::main realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on' > "$run/stub-tests"
  mkdir -p "$run/stub"
  : > "$run/stub/log"
}

# go <case> <STUB_BUILD> <task arguments...>: run task in the planted tree of <case>.
go() {
  case_=$1 build=$2
  shift 2
  STUB_DIR="$root/$dir/run-$case_/stub" STUB_LOG="$root/$dir/run-$case_/stub/log" STUB_TESTS="$root/$dir/run-$case_/stub-tests" STUB_BUILD=$build STUB_BUILD_SECS=1 \
    PATH="$root/$dir/bin:$PATH" task -d "$dir/run-$case_" "$@" > "$dir/$case_.out" 2>&1
}

# group_alive <pgid>: 0 when a process of the group <pgid> runs, 1 when kill -0 answers "No such
# process". Any other answer fails the test: a group that is not a number (an empty pid file),
# or a kill that refused its operand (dash's kill read "--" after the signal as a process
# number, "Illegal number", and the group looked gone at once), is not a group that is gone.
group_alive() {
  case "$1" in ''|*[!0-9]*|0|1) fail "process group [$1] is not the number of a group; kill -0 cannot have asked for it" ;; esac
  err=$(LC_ALL=C; kill -0 "-$1" 2>&1) && return 0
  case "$err" in *"No such process"*) return 1 ;; esac
  fail "kill -0 -$1 failed for another reason than a group that is gone: $err"
}

# group_gone_within <pgid> <tenths>: no process of the group <pgid> is left within <tenths> tenths of a second.
group_gone_within() {
  n=0
  while group_alive "$1"; do
    [ "$n" -ge "$2" ] && return 1
    sleep 0.1
    n=$((n + 1))
  done
  return 0
}

# after <log> <build pattern> <run pattern>: the first "build end" line that matches the first
# pattern comes before the first "run" line that matches the second.
after() {
  awk -v b="$2" -v r="$3" '/^build end / && $0 ~ b && !e { e = NR } /^run / && $0 ~ r && !u { u = NR } END { exit !(e && u && e < u) }' "$1"
}
opt1_build='profile=collect features=none .* -p swell-num '
lib_build='profile=collect features=collect-every-safe-point .* --lib'
main_build='profile=collect features=collect-every-safe-point .* --test main'

# 1. The builds pass: the lane's order holds.
plant ok
go ok ok slow-collect-start slow-tests SLOW_COLLECT=lane || fail "the lane's slow-collect-start and slow-tests exited non-zero on builds that passed ($dir/ok.out)"
for b in opt1 lib collect; do
  grep -q "^slow-build $b [0-9.]* s, exit 0, waited " "$dir/ok.out" || fail "slow-tests printed no line slow-build $b ... exit 0 ($dir/ok.out)"
done
log=$dir/run-ok/stub/log
[ "$(grep -c '^build start profile=collect ' "$log")" = 3 ] || fail "the stub saw not the three builds of the lane in the profile collect: slow-tests built in the foreground what the lane built beside it ($log)"
after "$log" "$opt1_build" 'profile=collect features=none ' || fail "the run of the default mode at opt-level 1 did not come after the build opt1 ($log)"
after "$log" "$lib_build" 'profile=collect features=collect-every-safe-point .* --lib' || fail "the run of the library's unit tests did not come after the build lib ($log)"
after "$log" "$main_build" 'profile=collect features=collect-every-safe-point .* --test main' || fail "the run of the collect mode at opt-level 1 did not come after the build collect ($log)"

# 2. The builds fail: the lane must fail.
plant fail
if go fail fail slow-collect-start slow-tests SLOW_COLLECT=lane; then
  fail "the lane's slow-tests exited 0 when the background build opt1 failed ($dir/fail.out)"
fi
grep -qF 'slow-tests: the build opt1 that the lane started failed, exit 3 (.out/slow-collect/opt1.log)' "$dir/fail.out" ||
  fail "slow-tests did not name the failed build ($dir/fail.out)"
grep -qF 'error: planted build failure' "$dir/fail.out" || fail "slow-tests did not print the build's log ($dir/fail.out)"
! grep -q '^run profile=collect ' "$dir/run-fail/stub/log" || fail "the stub ran the profile collect after the build failed ($dir/run-fail/stub/log)"

# 3a. The lane fails in its first part while the builds run: no build outlives task slow.
plant hang
if go hang hang slow; then
  fail "task slow exited 0 with a test262 pin that cannot be fetched ($dir/hang.out)"
fi
pg=$(cat "$dir/run-hang/.out/slow-collect/pid" 2>/dev/null) || fail "the lane started no background build ($dir/hang.out)"
group_gone_within "$pg" 0 || { kill -KILL "-$pg" 2>/dev/null; fail "the builds (process group $pg) still ran when task slow ended ($dir/hang.out)"; }
grep -qF 'slow-collect-stop: ended the builds at opt-level 1' "$dir/hang.out" ||
  fail "task slow ended without slow-collect-stop ending the builds ($dir/hang.out)"
[ "$(grep -c '^slow-runner: ' "$dir/hang.out")" = 1 ] && grep -q '^slow-runner: [0-9][0-9]* cores, ' "$dir/hang.out" ||
  fail "task slow did not print exactly one line \"slow-runner: <n> cores, ...\" that names the machine it runs on ($dir/hang.out)"

# 3b. The task process that started the builds is gone: the watcher ends them.
plant alone
go alone hang slow-collect-start || fail "task slow-collect-start exited non-zero ($dir/alone.out)"
pg=$(cat "$dir/run-alone/.out/slow-collect/pid" 2>/dev/null) || fail "slow-collect-start started no build ($dir/alone.out)"
group_gone_within "$pg" 50 || { kill -KILL "-$pg" 2>/dev/null; fail "the builds (process group $pg) still ran 5 s after the task process that started them ended ($dir/alone.out)"; }

# 4. slow-tests alone builds what it runs itself.
plant part
go part ok slow-tests || fail "task slow-tests run alone exited non-zero with no background build ($dir/part.out)"
log=$dir/run-part/stub/log
[ ! -e "$dir/run-part/.out/slow-collect" ] || fail "task slow-tests run alone made .out/slow-collect"
after "$log" "$opt1_build" 'profile=collect features=none ' || fail "task slow-tests run alone did not build opt1 before its run ($log)"
after "$log" "$lib_build" 'profile=collect features=collect-every-safe-point .* --lib' || fail "task slow-tests run alone did not build lib before its run ($log)"
! grep -q "^build start $main_build" "$log" || fail "task slow-tests run alone started a build of COLLECT_MAIN ($log)"
grep -q '^run profile=collect features=collect-every-safe-point .* --test main' "$log" || fail "task slow-tests run alone did not run the collect mode at opt-level 1 ($log)"

# 5. The pid file names a process group this tree's lane did not start (a number the system gave
#    to another process since): slow-collect-stop leaves it alone.
plant foreign
mkdir -p "$dir/run-foreign/.out/slow-collect"
other=$(sh -c 'setsid sleep 60 < /dev/null > /dev/null 2>&1 & echo $!')
n=0
while ! group_alive "$other" && [ "$n" -lt 20 ]; do sleep 0.1; n=$((n + 1)); done
group_alive "$other" || fail "setsid sleep 60 made no process group of its own"
echo "$other" > "$dir/run-foreign/.out/slow-collect/pid"
go foreign ok slow-collect-stop || { kill -KILL "-$other" 2>/dev/null; fail "task slow-collect-stop exited non-zero on a pid file of another process ($dir/foreign.out)"; }
if ! group_alive "$other"; then
  fail "slow-collect-stop ended process group $other, which this tree's lane did not start ($dir/foreign.out)"
fi
kill -KILL "-$other" 2>/dev/null
grep -qF "slow-collect-stop: process $other of .out/slow-collect/pid is not the build this tree's lane started" "$dir/foreign.out" ||
  fail "slow-collect-stop did not say that it left process $other alone ($dir/foreign.out)"

# 6. No setsid on PATH: the lane's line names setsid. PATH holds the stub cargo and the tools
#    the task files run, but not setsid.
plant nosetsid
mkdir -p "$dir/nosetsid-bin"
cp "$dir/bin/cargo" "$dir/bin/nextest-filter.awk" "$dir/nosetsid-bin/"
for tool in sh rm mkdir mv cat date sleep awk grep tr sed git; do
  p=$(command -v "$tool") && ln -s "$p" "$dir/nosetsid-bin/$tool"
done
taskbin=$(command -v task)
STUB_DIR="$root/$dir/run-nosetsid/stub" STUB_BUILD=ok PATH="$root/$dir/nosetsid-bin" "$taskbin" -d "$dir/run-nosetsid" slow-collect-start > "$dir/nosetsid.out" 2>&1 &&
  fail "task slow-collect-start exited 0 without setsid on PATH ($dir/nosetsid.out)"
grep -qF 'slow-collect-start: setsid is not on PATH' "$dir/nosetsid.out" ||
  fail "slow-collect-start without setsid on PATH did not name setsid ($dir/nosetsid.out)"

echo "$name: ok"

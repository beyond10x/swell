#!/bin/sh
# Planted defects of the background builds of the lane slow that
# slow-collect-builds-beside-the-lane-and-ends-with-it.sh does not plant (Taskfile.yml, tasks
# slow-collect-start and slow-tests; stories realm-size-lane-within-its-share and
# slow-lane-holds-its-bound-on-the-slower-runner, step 2).
# No test runs: the script plants a tree of its own (.out/gate-tests/<name>/run-<case>) with the
# repository's Taskfile.yml and taskfiles/, a crate swell whose library and tests/deep.rs read
# SWELL_FULL_COUNTS, deep.rs holding a stack test and another, tests/realm_padding.rs holding
# the test TEST_OPT1_PROFILE_TEST names, and the stub cargo of tests/gate/fixtures/cargo-stub.sh
# first on PATH, which logs each build and run with RUSTFLAGS, CARGO_INCREMENTAL,
# SWELL_FULL_COUNTS and its arguments; a build in the profile collect fails (exit 3) or hangs
# (sleeps 120 s) as STUB_BUILD says.
# 1. stale: an earlier lane left .out/slow-collect with a pid, the status 0 of each build and
#    the file done; the new lane's builds fail. task slow-collect-start slow-tests
#    SLOW_COLLECT=lane exits non-zero, names "the build opt1 that the lane started failed, exit
#    3", and runs nothing in the profile collect: a lane reads its own builds' status, never one
#    an earlier lane left.
# 2. bound: SLOW_COLLECT_WAIT=1 and builds that hang. The same exits non-zero within 8 s,
#    prints "wrote no status within 1 s of waiting", and the builds' process group is gone
#    within 3 s after: the wait is bounded by SLOW_COLLECT_WAIT.
# 3. same: each of the three background builds (opt1, lib, collect) and the run at opt-level 1
#    that reads it have the same RUSTFLAGS, CARGO_INCREMENTAL and SWELL_FULL_COUNTS, and the
#    same arguments but --no-run on one side and --config-file <file> -E <filter> on the other;
#    otherwise cargo builds again in the foreground and the build beside slow-test262 is lost,
#    with every run green. SWELL_FULL_COUNTS counts: the library's unit tests read it when they
#    are compiled.
# Run from the repository root: sh tests/gate/slow-collect-a-lane-reads-only-its-own-build.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SLOW_COLLECT SLOW_COLLECT_WAIT STUB_BUILD STUB_BUILD_MATCH STUB_COMPILE STUB_DROP STUB_DOUBLE STUB_FAIL
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=slow-collect-a-lane-reads-only-its-own-build
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/bin" || { echo "$name: FAIL cannot make $dir"; exit 1; }
root=$(pwd)
began=$(date +%s.%N)
bad=0

fail() {
  echo "$name: FAIL $*"
  bad=1
}

[ -f Taskfile.yml ] && [ -f taskfiles/realm-room-tests.awk ] || { echo "$name: FAIL run it from the repository root"; exit 1; }
cp tests/gate/fixtures/cargo-stub.sh "$dir/bin/cargo" && cp tests/gate/fixtures/nextest-filter.awk "$dir/bin/" && chmod +x "$dir/bin/cargo" ||
  { echo "$name: FAIL cannot copy the stub cargo of tests/gate/fixtures"; exit 1; }

plant() {
  run="$dir/run-$1"
  mkdir -p "$run/crates/swell/tests/support" "$run/crates/swell/src" "$run/.out" "$run/.config" "$run/stub" "$run/spec" || return 1
  cp "$root/Taskfile.yml" "$run/Taskfile.yml" && cp -R "$root/taskfiles" "$run/taskfiles" &&
    cp "$root/.config/nextest.toml" "$run/.config/" && cp "$root/spec/ess-inputs.yaml" "$run/spec/" || return 1
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
  : > "$run/stub/log"
}

# go <case> <STUB_BUILD> <task arguments...>: run task in the planted tree of <case>.
go() {
  case_=$1 build=$2
  shift 2
  STUB_DIR="$root/$dir/run-$case_/stub" STUB_LOG="$root/$dir/run-$case_/stub/log" STUB_TESTS="$root/$dir/run-$case_/stub-tests" STUB_BUILD=$build \
    PATH="$root/$dir/bin:$PATH" task -d "$dir/run-$case_" "$@" > "$dir/$case_.out" 2>&1
}

# group_alive <pgid>: 0 when a process of the group <pgid> runs, 1 when kill -0 answers "No such
# process". Any other answer ends the test as failed: a group that is not a number (no pid file),
# or a kill that refused its operand (dash's kill read "--" after the signal as a process
# number, "Illegal number", and the group looked gone at once), is not a group that is gone.
group_alive() {
  case "$1" in ''|*[!0-9]*|0|1) echo "$name: FAIL process group [$1] is not the number of a group; kill -0 cannot have asked for it"; exit 1 ;; esac
  err=$(LC_ALL=C; kill -0 "-$1" 2>&1) && return 0
  case "$err" in *"No such process"*) return 1 ;; esac
  echo "$name: FAIL kill -0 -$1 failed for another reason than a group that is gone: $err"
  exit 1
}

# 2 starts first and runs beside 1 and 3: the builds hang; the wait ends at SLOW_COLLECT_WAIT.
plant bound || { echo "$name: FAIL cannot plant run-bound"; exit 1; }
go bound hang slow-collect-start slow-tests SLOW_COLLECT=lane SLOW_COLLECT_WAIT=1 &
tp=$!
bound_began=$(date +%s.%N)

# 1. An earlier lane's status 0 is left; this lane's builds fail.
plant stale || { echo "$name: FAIL cannot plant run-stale"; exit 1; }
mkdir -p "$dir/run-stale/.out/slow-collect"
echo 1 > "$dir/run-stale/.out/slow-collect/pid"
for b in opt1 lib collect; do
  echo 0 > "$dir/run-stale/.out/slow-collect/$b.status"
  echo 100.0 > "$dir/run-stale/.out/slow-collect/$b.start"
  echo 200.0 > "$dir/run-stale/.out/slow-collect/$b.end"
done
: > "$dir/run-stale/.out/slow-collect/done"
if go stale fail slow-collect-start slow-tests SLOW_COLLECT=lane; then
  fail "the lane exited 0 on a failed build, with the status 0 of an earlier lane left in .out/slow-collect ($dir/stale.out)"
fi
grep -qF 'the build opt1 that the lane started failed, exit 3' "$dir/stale.out" ||
  fail "the lane did not name its own failed build (exit 3); it read the status an earlier lane left ($dir/stale.out)"
! grep -q '^run profile=collect ' "$dir/run-stale/stub/log" || fail "the lane ran the profile collect on the status of an earlier lane ($dir/run-stale/stub/log)"

# 3. Each build and the run that reads it are the same build for cargo.
plant same || { echo "$name: FAIL cannot plant run-same"; exit 1; }
go same ok slow-collect-start slow-tests SLOW_COLLECT=lane || fail "the lane exited non-zero on builds that passed ($dir/same.out)"
# key <line>: the profile, feature, RUSTFLAGS, CARGO_INCREMENTAL, SWELL_FULL_COUNTS and the
# arguments of a stub line, without --no-run, --config-file <file> and -E <filter> (last).
key() {
  printf '%s\n' "$1" | awk '{
    k = ""
    n = split("profile features full rustflags incremental", f, " ")
    for (i = 1; i <= n; i++) if (match($0, " " f[i] "=(\\[[^]]*\\]|[^ ]*)")) k = k substr($0, RSTART, RLENGTH)
    a = $0; sub(/^.* args: /, "", a); sub(/ -E .*$/, "", a); sub(/ --config-file [^ ]*/, "", a); sub(/^--no-run /, "", a); sub(/ --no-run /, " ", a)
    gsub(/  +/, " ", a)
    print k " args: " a
  }'
}
log=$dir/run-same/stub/log
for pair in "opt1|features=none .* -p swell-num " "lib|features=collect-every-safe-point .* --lib" "collect|features=collect-every-safe-point .* --test main"; do
  b=${pair%%|*} pat=${pair#*|}
  bl=$(grep "^build start profile=collect " "$log" | grep -e "$pat" )
  rl=$(grep "^run profile=collect " "$log" | grep -e "$pat")
  [ "$(printf '%s\n' "$bl" | grep -c .)" = 1 ] && [ "$(printf '%s\n' "$rl" | grep -c .)" = 1 ] ||
    { fail "the stub saw not one background build $b and one run that reads it ($log)"; continue; }
  kb=$(key "$bl")
  kr=$(key "$rl")
  [ -n "$kb" ] && [ "$kb" = "$kr" ] || fail "the background build $b and the run at opt-level 1 that reads it differ for cargo, so cargo builds again in the foreground: build [$kb], run [$kr]"
done

# 2, read: at most 8 s after its start.
while kill -0 "$tp" 2>/dev/null &&
  [ "$(awk -v a="$bound_began" -v b="$(date +%s.%N)" 'BEGIN { print (b - a < 8) }')" = 1 ]; do sleep 0.1; done
pg=$(cat "$dir/run-bound/.out/slow-collect/pid" 2>/dev/null)
if kill -0 "$tp" 2>/dev/null; then
  kill -KILL "$tp" 2>/dev/null
  [ -n "$pg" ] && kill -KILL "-$pg" 2>/dev/null
  fail "slow-tests still waited 8 s after its start with SLOW_COLLECT_WAIT=1 and builds that hang ($dir/bound.out)"
else
  if wait "$tp"; then fail "the lane exited 0 when the build wrote no status within SLOW_COLLECT_WAIT ($dir/bound.out)"; fi
  grep -qF 'wrote no status within 1 s of waiting' "$dir/bound.out" || fail "slow-tests did not say that the build wrote no status within 1 s ($dir/bound.out)"
  n=0
  while group_alive "$pg" && [ "$n" -lt 30 ]; do sleep 0.1; n=$((n + 1)); done
  if group_alive "$pg"; then
    kill -KILL "-$pg" 2>/dev/null
    fail "the builds (process group $pg) still ran 3 s after the lane ended on the wait's bound ($dir/bound.out)"
  fi
fi

secs=$(awk -v a="$began" -v b="$(date +%s.%N)" 'BEGIN { printf "%.1f", b - a }')
echo "$name: $secs s"
[ "$bad" = 0 ] || exit 1
echo "$name: ok"

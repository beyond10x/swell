#!/bin/sh
# Planted defects of the background build of the lane slow that
# slow-collect-builds-beside-the-lane-and-ends-with-it.sh does not plant (Taskfile.yml, tasks
# slow-collect-start and slow-tests; story realm-size-lane-within-its-share).
# No test runs: the script plants a tree of its own (.out/gate-tests/<name>/run-<case>) with the
# repository's Taskfile.yml and taskfiles/, a crate swell whose tests/deep.rs reads
# SWELL_FULL_COUNTS and holds a stack test and another, and a stub cargo first on PATH. The stub
# logs each call of the profile collect with RUSTFLAGS, CARGO_INCREMENTAL and its arguments
# ("build ..." for --no-run, "run ..." otherwise); the build does what STUB_BUILD says: ok, fail
# (exit 3) or hang (sleeps 120 s).
# 1. stale: an earlier lane left .out/slow-collect with a pid and the status 0; the new lane's
#    build fails. task slow-collect-start slow-tests SLOW_COLLECT=lane exits non-zero, names
#    "failed, exit 3", and runs nothing in the profile collect: a lane reads its own build's
#    status, never one an earlier lane left.
# 2. bound: SLOW_COLLECT_WAIT=1 and a build that hangs. The same exits non-zero within 8 s,
#    prints "wrote no status within 1 s of waiting", and the build's process group is gone
#    within 3 s after: the wait is bounded by SLOW_COLLECT_WAIT.
# 3. same: the background build and the run at opt-level 1 that reads it have the same
#    RUSTFLAGS and CARGO_INCREMENTAL, and the same arguments but --no-run on one side and
#    --config-file <file> -E <filter> on the other; otherwise cargo builds the profile collect
#    again in the foreground and the build beside slow-test262 is lost, with every run green.
# Run from the repository root: sh tests/gate/slow-collect-a-lane-reads-only-its-own-build.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SLOW_COLLECT SLOW_COLLECT_WAIT
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

printf '%s\n' '#!/bin/sh' \
  'profile=test; prev=""; norun=0' \
  'for a in "$@"; do [ "$prev" = "--cargo-profile" ] && profile=$a; [ "$a" = "--no-run" ] && norun=1; prev=$a; done' \
  'if [ "$profile" = collect ]; then' \
  '  if [ "$norun" = 1 ]; then w=build; else w=run; fi' \
  '  echo "$w RUSTFLAGS=[$RUSTFLAGS] CARGO_INCREMENTAL=[$CARGO_INCREMENTAL] args: $*" >> "$STUB_DIR/flags"' \
  'fi' \
  'if [ "$norun" = 1 ] && [ "$profile" = collect ]; then' \
  '  case "$STUB_BUILD" in' \
  '    fail) echo "error: planted build failure"; exit 3 ;;' \
  '    hang) sleep 120; exit 0 ;;' \
  '  esac' \
  '  echo "    Finished \`collect\` profile [optimized] target(s) in 0.10s"; exit 0' \
  'fi' \
  'if [ "$norun" = 1 ]; then exit 0; fi' \
  'echo "run $profile" >> "$STUB_DIR/calls"' \
  'mkdir -p target/nextest/default' \
  'printf "<testsuites><testsuite><testcase name=\"deep::t\" classname=\"swell::main\" time=\"0.1\"/></testsuite></testsuites>\n" > target/nextest/default/junit.xml' \
  'echo "    Starting 2 tests across 1 binary"' \
  'echo "     Summary [   0.100s] 2 tests run: 2 passed, 0 skipped"' \
  'exit 0' > "$dir/bin/cargo" || { echo "$name: FAIL cannot write the stub cargo"; exit 1; }
chmod +x "$dir/bin/cargo"

plant() {
  run="$dir/run-$1"
  mkdir -p "$run/crates/swell/tests/support" "$run/crates/swell/src" "$run/.out" "$run/.config" "$run/stub" "$run/spec" || return 1
  cp "$root/Taskfile.yml" "$run/Taskfile.yml" && cp -R "$root/taskfiles" "$run/taskfiles" &&
    cp "$root/.config/nextest.toml" "$run/.config/" && cp "$root/spec/ess-inputs.yaml" "$run/spec/" || return 1
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
  : > "$run/stub/flags"
}

# go <case> <STUB_BUILD> <task arguments...>: run task in the planted tree of <case>.
go() {
  case_=$1 build=$2
  shift 2
  STUB_DIR="$root/$dir/run-$case_/stub" STUB_BUILD=$build PATH="$root/$dir/bin:$PATH" task -d "$dir/run-$case_" "$@" > "$dir/$case_.out" 2>&1
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

# 2 starts first and runs beside 1 and 3: the build hangs; the wait ends at SLOW_COLLECT_WAIT.
plant bound || { echo "$name: FAIL cannot plant run-bound"; exit 1; }
go bound hang slow-collect-start slow-tests SLOW_COLLECT=lane SLOW_COLLECT_WAIT=1 &
tp=$!
bound_began=$(date +%s.%N)

# 1. An earlier lane's status 0 is left; this lane's build fails.
plant stale || { echo "$name: FAIL cannot plant run-stale"; exit 1; }
mkdir -p "$dir/run-stale/.out/slow-collect"
echo 1 > "$dir/run-stale/.out/slow-collect/pid"
echo 0 > "$dir/run-stale/.out/slow-collect/status"
echo 100.0 > "$dir/run-stale/.out/slow-collect/start"
echo 200.0 > "$dir/run-stale/.out/slow-collect/end"
if go stale fail slow-collect-start slow-tests SLOW_COLLECT=lane; then
  fail "the lane exited 0 on a failed build, with the status 0 of an earlier lane left in .out/slow-collect ($dir/stale.out)"
fi
grep -qF 'the build of the profile collect that the lane started failed, exit 3' "$dir/stale.out" ||
  fail "the lane did not name its own failed build (exit 3); it read the status an earlier lane left ($dir/stale.out)"
! grep -q '^run collect$' "$dir/run-stale/stub/calls" || fail "the lane ran the profile collect on the status of an earlier lane ($dir/run-stale/stub/calls)"

# 3. The build and the run that reads it are the same build for cargo.
plant same || { echo "$name: FAIL cannot plant run-same"; exit 1; }
go same ok slow-collect-start slow-tests SLOW_COLLECT=lane || fail "the lane exited non-zero on a build that passed ($dir/same.out)"
b=$(grep '^build ' "$dir/run-same/stub/flags" | sed 's/^build //; s/ --no-run / /')
r=$(grep '^run ' "$dir/run-same/stub/flags" | sed 's/^run //; s/ --config-file [^ ]* / /; s/ -E .*$//')
[ "$(grep -c '^build ' "$dir/run-same/stub/flags")" = 1 ] && [ "$(grep -c '^run ' "$dir/run-same/stub/flags")" = 1 ] ||
  fail "the stub saw not one background build and one run of the profile collect ($dir/run-same/stub/flags)"
b=$(printf '%s' "$b" | tr -s ' ')
r=$(printf '%s' "$r" | tr -s ' ')
[ -n "$b" ] && [ "$b" = "$r" ] || fail "the background build and the run at opt-level 1 differ for cargo, so cargo builds again in the foreground: build [$b], run [$r]"

# 2, read: at most 8 s after its start.
while kill -0 "$tp" 2>/dev/null &&
  [ "$(awk -v a="$bound_began" -v b="$(date +%s.%N)" 'BEGIN { print (b - a < 8) }')" = 1 ]; do sleep 0.1; done
pg=$(cat "$dir/run-bound/.out/slow-collect/pid" 2>/dev/null)
if kill -0 "$tp" 2>/dev/null; then
  kill -KILL "$tp" 2>/dev/null
  [ -n "$pg" ] && kill -KILL "-$pg" 2>/dev/null
  fail "slow-tests still waited 8 s after its start with SLOW_COLLECT_WAIT=1 and a build that hangs ($dir/bound.out)"
else
  if wait "$tp"; then fail "the lane exited 0 when the build wrote no status within SLOW_COLLECT_WAIT ($dir/bound.out)"; fi
  grep -qF 'wrote no status within 1 s of waiting' "$dir/bound.out" || fail "slow-tests did not say that the build wrote no status within 1 s ($dir/bound.out)"
  n=0
  while group_alive "$pg" && [ "$n" -lt 30 ]; do sleep 0.1; n=$((n + 1)); done
  if group_alive "$pg"; then
    kill -KILL "-$pg" 2>/dev/null
    fail "the build (process group $pg) still ran 3 s after the lane ended on the wait's bound ($dir/bound.out)"
  fi
fi

secs=$(awk -v a="$began" -v b="$(date +%s.%N)" 'BEGIN { printf "%.1f", b - a }')
echo "$name: $secs s"
[ "$bad" = 0 ] || exit 1
echo "$name: ok"

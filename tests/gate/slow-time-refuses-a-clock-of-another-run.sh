#!/bin/sh
# Planted defect of the bounds of the lane slow (taskfiles/gate-time.yml, task slow-time-sum;
# story realm-size-lane-within-its-share, point 4): slow-time-sum reads the lane's clock,
# .out/gate-time/start, beside the slow-time lines, and a clock it cannot take as the one of the
# run of task slow it ends must not pass a lane: one that another task process started (a clock
# and lines left by an earlier run, read by slow-time-sum called on its own after the parts ran
# one by one), no clock at all, and a clock that holds no time.
# No part runs: as slow-time-refuses-the-lane-over-its-bound.sh, in a directory of its own whose
# Taskfile.yml includes taskfiles/gate-time.yml, with four timed parts of 1 s each.
# 0. A task lane of the planted Taskfile.yml runs gate-time-start, gate-time and slow-time-sum in
#    one task process, as task slow does: it exits 0 and prints its clock. Then slow-time-sum on
#    its own, in another task process, reads the clock and the line that lane left: it exits
#    non-zero and names the task process of that lane.
# 1. The clock started now and no .out/gate-time/cpu (as a planted run has): slow-time-sum
#    exits 0 and prints "slow-time clock <s> of <lane bound>, ...". This shows the planted tree
#    works.
# 2. The same with .out/gate-time/cpu naming the task process 0, which no task process is: it
#    exits non-zero and prints "slow-time: the lane's clock .out/gate-time/start was started by
#    the task process 0, not by this one (<pid>) ...".
# 3. No file .out/gate-time/start: it exits non-zero and prints "slow-time: no lane clock ...".
# 4. A file .out/gate-time/start that holds "x": it exits non-zero and prints "slow-time: the
#    lane clock .out/gate-time/start holds "x", not a time in seconds".
# Run from the repository root: sh tests/gate/slow-time-refuses-a-clock-of-another-run.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SLOW_LANE_BUDGET SLOW_PART_BUDGET
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=slow-time-refuses-a-clock-of-another-run
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/run/.out/gate-time" || { echo "$name: FAIL cannot make $dir"; exit 1; }

fail() {
  echo "$name: FAIL $*"
  exit 1
}

[ -f taskfiles/gate-time.yml ] || fail "no taskfiles/gate-time.yml"
lane=$(awk -F '"' '/SLOW_LANE_BUDGET:.*default/ { print $2; exit }' taskfiles/gate-time.yml)
[ -n "$lane" ] || fail "no default of SLOW_LANE_BUDGET in taskfiles/gate-time.yml"
printf "version: '3'\n\nincludes:\n  gate:\n    taskfile: %s\n    flatten: true\n" "$(pwd)/taskfiles/gate-time.yml" > "$dir/run/Taskfile.yml" ||
  fail "cannot write the planted Taskfile.yml"
# A lane in one task process, as task slow runs: its clock, one timed part, its sum.
printf '%s\n' '' 'tasks:' '  lane:' '    cmds:' '      - task: gate-time-start' '      - task: gate-time' \
  '        vars: {STEP: slow-test262, LABEL: slow-time}' '      - task: slow-time-sum' >> "$dir/run/Taskfile.yml" ||
  fail "cannot write the planted lane"
t() { task -d "$dir/run" "$@"; }
g=$dir/run/.out/gate-time

# 0. The lane in one task process passes and prints its clock; slow-time-sum called after it on
#    its own, in another task process, refuses the clock and lines that lane left.
t lane > "$dir/lane.out" 2>&1 || fail "a lane run in one task process was refused ($dir/lane.out)"
grep -qE "^slow-time clock [0-9]+\\.[0-9] of $lane, " "$dir/lane.out" || fail "the lane printed no line \"slow-time clock <s> of $lane, ...\" ($dir/lane.out)"
if t slow-time-sum > "$dir/later.out" 2>&1; then
  fail "slow-time-sum on its own exited 0 with the clock and lines an earlier lane left ($dir/later.out)"
fi
grep -q "^slow-time: the lane's clock .out/gate-time/start was started by the task process [0-9]*, not by this one (" "$dir/later.out" ||
  fail "slow-time-sum on its own did not name the task process of the earlier lane ($dir/later.out)"
rm -f "$g/cpu" "$g/mark" "$g/cpu-mark" "$g/cpu-steps"

printf 'slow-time slow-test262 1.0\nslow-time slow-tests 1.0\nslow-time test-realm-sizes 1.0\nslow-time stack-profiles 1.0\n' > "$g/steps" ||
  fail "cannot write the planted slow-time lines"

date +%s > "$g/start" || fail "cannot write the lane's clock"
t slow-time-sum > "$dir/own.out" 2>&1 || fail "slow-time-sum refused a lane of 4.0 s whose clock started now ($dir/own.out)"
grep -qE "^slow-time clock [0-9]+\\.[0-9] of $lane, " "$dir/own.out" || fail "slow-time-sum printed no line \"slow-time clock <s> of $lane, ...\" ($dir/own.out)"

echo '0 0' > "$g/cpu"
if t slow-time-sum > "$dir/other.out" 2>&1; then
  fail "slow-time-sum exited 0 with a clock that the task process 0 started ($dir/other.out)"
fi
grep -q "^slow-time: the lane's clock .out/gate-time/start was started by the task process 0, not by this one (" "$dir/other.out" ||
  fail "slow-time-sum did not name the task process that started the clock ($dir/other.out)"
rm -f "$g/cpu"

rm -f "$g/start"
if t slow-time-sum > "$dir/none.out" 2>&1; then
  fail "slow-time-sum exited 0 without a lane clock ($dir/none.out)"
fi
grep -q '^slow-time: no lane clock .out/gate-time/start' "$dir/none.out" || fail "slow-time-sum did not say that there is no clock ($dir/none.out)"

echo x > "$g/start"
if t slow-time-sum > "$dir/bad.out" 2>&1; then
  fail "slow-time-sum exited 0 with a clock that holds no time ($dir/bad.out)"
fi
grep -qF 'slow-time: the lane clock .out/gate-time/start holds "x", not a time in seconds' "$dir/bad.out" ||
  fail "slow-time-sum did not say that the clock holds no time ($dir/bad.out)"

echo "$name: ok"

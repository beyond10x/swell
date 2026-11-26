#!/bin/sh
# Planted defect of the bounds of the lane slow (taskfiles/gate-time.yml, task slow-time-sum;
# story realm-size-lane-within-its-share, point 4: "task slow fails when the lane ran over 900
# s"): slow-time-sum sums the "slow-time <part> <seconds>" lines and nothing else, so a part that
# task slow runs with no gate-time line after it (one added after stack-profiles, before
# slow-time-sum) is in no sum, and the lane passes however long that part ran. The lane's clock
# is there: gate-time-start writes .out/gate-time/start when the lane starts.
# No part runs: as slow-time-refuses-the-lane-over-its-bound.sh, in a directory of its own whose
# Taskfile.yml includes taskfiles/gate-time.yml. Four timed parts of 25 s each, and the lane's
# start file dated the lane bound plus 100 s before now: slow-time-sum must exit non-zero and
# print a line "slow-time: the lane ran ...".
# Run from the repository root: sh tests/gate/slow-time-refuses-a-lane-whose-untimed-part-ran-over-its-bound.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SLOW_LANE_BUDGET SLOW_PART_BUDGET
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=slow-time-refuses-a-lane-whose-untimed-part-ran-over-its-bound
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

now=$(date +%s)
start=$(awk -v now="$now" -v lane="$lane" 'BEGIN { printf "%d.000000000", now - lane - 100 }')
echo "$start" > "$dir/run/.out/gate-time/start"
mark=$(awk -v now="$now" 'BEGIN { printf "%d.000000000", now - 1 }')
echo "$mark" > "$dir/run/.out/gate-time/mark"
printf 'slow-time slow-test262 25.0\nslow-time slow-tests 25.0\nslow-time test-realm-sizes 25.0\nslow-time stack-profiles 25.0\n' > "$dir/run/.out/gate-time/steps"

if task -d "$dir/run" slow-time-sum > "$dir/over.out" 2>&1; then
  fail "slow-time-sum exited 0 for a lane that started $((lane + 100)) s ago, over its bound of $lane s, with 100.0 s in its timed parts ($dir/over.out: $(head -n 1 "$dir/over.out"))"
fi
grep -q '^slow-time: the lane ran ' "$dir/over.out" || fail "slow-time-sum did not refuse the lane by its clock ($dir/over.out)"

echo "$name: ok"

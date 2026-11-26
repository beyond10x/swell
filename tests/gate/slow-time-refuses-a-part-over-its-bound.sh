#!/bin/sh
# Planted defect of the bounds of the lane slow (taskfiles/gate-time.yml, task slow-time-sum;
# story realm-size-lane-within-its-share, point 4): a part of the lane that ran over
# SLOW_PART_BUDGET is refused, naming the part and its seconds, also when the lane as a whole is
# within SLOW_LANE_BUDGET.
# No part runs: the script plants the "slow-time <part> <seconds>" lines that gate-time writes
# to .out/gate-time/steps, in a directory of its own (.out/gate-tests/<name>/run) whose
# Taskfile.yml includes taskfiles/gate-time.yml (a task of an included file runs in the directory
# of the Taskfile that includes it). The bounds are read from the defaults of slow-time-sum, the
# one place that holds them; none is copied here.
# 1. Four parts, the first at the part bound and the lane within its bound: slow-time-sum exits 0
#    and prints "slow-time lane <sum> of <lane bound>, longest part slow-test262 <s> of <part
#    bound>".
# 2. slow-test262 at 0.1 s and slow-tests 0.1 s over the part bound, the lane within its bound: it
#    exits non-zero and prints "slow-time: the part slow-tests ran <s> s, over the bound of <part
#    bound> s for a part", and no line about the lane.
# Run from the repository root: sh tests/gate/slow-time-refuses-a-part-over-its-bound.sh
# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SLOW_LANE_BUDGET SLOW_PART_BUDGET
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=slow-time-refuses-a-part-over-its-bound
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/run/.out/gate-time" || { echo "$name: FAIL cannot make $dir"; exit 1; }

fail() {
  echo "$name: FAIL $*"
  exit 1
}

[ -f taskfiles/gate-time.yml ] || fail "no taskfiles/gate-time.yml"
lane=$(awk -F '"' '/SLOW_LANE_BUDGET:.*default/ { print $2; exit }' taskfiles/gate-time.yml)
part=$(awk -F '"' '/SLOW_PART_BUDGET:.*default/ { print $2; exit }' taskfiles/gate-time.yml)
[ -n "$lane" ] || fail "no default of SLOW_LANE_BUDGET in taskfiles/gate-time.yml"
[ -n "$part" ] || fail "no default of SLOW_PART_BUDGET in taskfiles/gate-time.yml"
awk -v lane="$lane" -v part="$part" 'BEGIN { exit !(part + 0.4 <= lane + 0) }' ||
  fail "the part bound $part leaves no room under the lane bound $lane for this script"
printf "version: '3'\n\nincludes:\n  gate:\n    taskfile: %s\n    flatten: true\n" "$(pwd)/taskfiles/gate-time.yml" > "$dir/run/Taskfile.yml" ||
  fail "cannot write the planted Taskfile.yml"
t() { task -d "$dir/run" "$@"; }

# Four parts, 0.1 s each but one: with "within" slow-test262 is at the part bound; with "over"
# slow-tests is 0.1 s over it. The lane is the part bound plus 0.3 or 0.4 s, within its own.
plant() {
  awk -v part="$part" -v how="$1" 'BEGIN {
    if (how == "within") printf "slow-time slow-test262 %.1f\n", part; else print "slow-time slow-test262 0.1"
    if (how == "over") printf "slow-time slow-tests %.1f\n", part + 0.1; else print "slow-time slow-tests 0.1"
    print "slow-time test-realm-sizes 0.1"
    print "slow-time stack-profiles 0.1"
  }' > "$dir/run/.out/gate-time/steps"
}

plant within
t slow-time-sum > "$dir/within.out" 2>&1 ||
  fail "slow-time-sum refused a lane whose longest part is at the bound of $part s ($dir/within.out)"
want=$(awk -v lane="$lane" -v part="$part" 'BEGIN { printf "slow-time lane %.1f of %s, longest part slow-test262 %.1f of %s", part + 0.3, lane, part, part }')
grep -qxF "$want" "$dir/within.out" || fail "slow-time-sum printed no line \"$want\" ($dir/within.out)"

plant over
if t slow-time-sum > "$dir/over.out" 2>&1; then
  fail "slow-time-sum exited 0 with the part slow-tests 0.1 s over the bound of $part s ($dir/over.out)"
fi
want=$(awk -v part="$part" 'BEGIN { printf "slow-time: the part slow-tests ran %.1f s, over the bound of %s s for a part", part + 0.1, part }')
grep -qxF "$want" "$dir/over.out" || fail "slow-time-sum did not print \"$want\" ($dir/over.out)"
if grep -q 'the lane ran' "$dir/over.out"; then
  fail "slow-time-sum refused the lane, which is within its bound of $lane s ($dir/over.out)"
fi

echo "$name: ok"

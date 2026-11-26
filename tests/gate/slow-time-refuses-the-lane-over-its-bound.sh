#!/bin/sh
# Planted defect of the bounds of the lane slow (taskfiles/gate-time.yml, task slow-time-sum;
# story realm-size-lane-within-its-share, point 4): a lane whose parts ran longer than
# SLOW_LANE_BUDGET together is refused, naming the sum and the three slowest parts, also when
# each part is within SLOW_PART_BUDGET.
# No part runs: the script plants the "slow-time <part> <seconds>" lines that gate-time writes
# to .out/gate-time/steps, in a directory of its own (.out/gate-tests/<name>/run) whose
# Taskfile.yml includes taskfiles/gate-time.yml (a task of an included file runs in the directory
# of the Taskfile that includes it). The bounds are read from the defaults of slow-time-sum, the
# one place that holds them; none is copied here.
# 1. Four parts whose seconds add up to the lane bound, each at most the part bound:
#    slow-time-sum exits 0 and prints "slow-time lane <lane bound> of <lane bound>, longest part
#    <part> <s> of <part bound>".
# 2. The same with stack-profiles 0.1 s longer, so the lane is 0.1 s over its bound and each
#    part still within its own: it exits non-zero and prints "slow-time: the lane ran <sum> s,
#    over its bound of <lane bound> s; the slowest: <part> <s>, <part> <s>, <part> <s>", and no
#    line about a part.
# Run from the repository root: sh tests/gate/slow-time-refuses-the-lane-over-its-bound.sh
# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SLOW_LANE_BUDGET SLOW_PART_BUDGET
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=slow-time-refuses-the-lane-over-its-bound
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
# Four parts of a quarter of the lane bound each, the last 0.1 s longer with "over": each must
# stay within the part bound.
awk -v lane="$lane" -v part="$part" 'BEGIN { exit !(lane / 4 + 0.1 <= part + 0) }' ||
  fail "a quarter of the lane bound $lane is over the part bound $part; four parts cannot pass the lane bound here"
printf "version: '3'\n\nincludes:\n  gate:\n    taskfile: %s\n    flatten: true\n" "$(pwd)/taskfiles/gate-time.yml" > "$dir/run/Taskfile.yml" ||
  fail "cannot write the planted Taskfile.yml"
t() { task -d "$dir/run" "$@"; }

plant() {
  awk -v lane="$lane" -v how="$1" 'BEGIN {
    q = sprintf("%.1f", lane / 4) + 0
    printf "slow-time slow-test262 %.1f\n", q
    printf "slow-time slow-tests %.1f\n", q
    printf "slow-time test-realm-sizes %.1f\n", q
    printf "slow-time stack-profiles %.1f\n", lane - 3 * q + (how == "over" ? 0.1 : 0)
  }' > "$dir/run/.out/gate-time/steps"
}

plant within
t slow-time-sum > "$dir/within.out" 2>&1 ||
  fail "slow-time-sum refused a lane at its bound of $lane s ($dir/within.out)"
grep -qE "^slow-time lane $(awk -v lane="$lane" 'BEGIN { printf "%.1f", lane }' | sed 's/\./\\./') of $lane, longest part [a-z0-9-]+ [0-9]+\\.[0-9] of $part\$" "$dir/within.out" ||
  fail "slow-time-sum printed no line \"slow-time lane <sum> of $lane, longest part <part> <s> of $part\" ($dir/within.out)"

plant over
if t slow-time-sum > "$dir/over.out" 2>&1; then
  fail "slow-time-sum exited 0 with the lane 0.1 s over its bound of $lane s ($dir/over.out)"
fi
sum=$(awk -v lane="$lane" 'BEGIN { printf "%.1f", lane + 0.1 }' | sed 's/\./\\./')
grep -qE "^slow-time: the lane ran $sum s, over its bound of $lane s; the slowest: [a-z0-9-]+ [0-9]+\\.[0-9], [a-z0-9-]+ [0-9]+\\.[0-9], [a-z0-9-]+ [0-9]+\\.[0-9]\$" "$dir/over.out" ||
  fail "slow-time-sum did not name the lane's $sum s and its three slowest parts ($dir/over.out)"
if grep -q 'slow-time: the part' "$dir/over.out"; then
  fail "slow-time-sum refused a part, and each is within its bound of $part s ($dir/over.out)"
fi

echo "$name: ok"

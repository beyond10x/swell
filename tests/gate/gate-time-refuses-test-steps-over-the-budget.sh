#!/bin/sh
# Planted defect of the time budget of the gate's test steps (Taskfile.yml, tasks gate-time-start,
# gate-time and gate-time-sum; story gate-test-steps-within-budget, point 2): a gate run whose test
# steps take longer than GATE_TIME_BUDGET fails, naming the sum and the three slowest steps.
# The clock runs as in check, in a directory of its own (task -d .out/gate-tests/<name>/run with
# the real Taskfile.yml): gate-time-start, then four steps, each a gate-time line, the first after
# a pause of one second. With the default budget (120) gate-time-sum passes and prints
# "gate-time test-steps <sum> of 120" (then ", cpu <s>" when gate-time-start ran in the same task
# process, and ", load <l> on <n> cores" where the machine tells);
# with GATE_TIME_BUDGET=0.2 it exits non-zero and prints
# "gate-time test-steps <sum> of 0.2" and "gate-time: the test steps ran <sum> s, over the budget of
# 0.2 s; the slowest: <step> <s>, <step> <s>, <step> <s>", the first of them the step that paused.
# Last, gate-time-start, gate-time and gate-time-sum in one task process print ", cpu <s>".
# Run from the repository root: sh tests/gate/gate-time-refuses-test-steps-over-the-budget.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=gate-time-refuses-test-steps-over-the-budget
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/run"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

taskfile=$(pwd)/Taskfile.yml
t() { task -d "$dir/run" -t "$taskfile" "$@"; }

{
  t gate-time-start &&
    sleep 1 &&
    t gate-time STEP=paused && t gate-time STEP=second && t gate-time STEP=third && t gate-time STEP=fourth
} > "$dir/steps.out" 2>&1 || fail "the clock tasks exited non-zero ($dir/steps.out)"
[ "$(grep -c '^gate-time ' "$dir/steps.out")" -eq 4 ] || fail "four steps printed no four gate-time lines ($dir/steps.out)"
grep -qE '^gate-time paused [0-9]+\.[0-9]$' "$dir/steps.out" || fail "no gate-time line for the paused step ($dir/steps.out)"

t gate-time-sum > "$dir/within.out" 2>&1 || fail "gate-time-sum refused four short steps under the default budget ($dir/within.out)"
grep -qE '^gate-time test-steps [0-9]+\.[0-9] of 120(, cpu [0-9]+\.[0-9])?(, load [0-9.]+ on [0-9]+ cores)?$' "$dir/within.out" ||
  fail "gate-time-sum printed no line \"gate-time test-steps <sum> of 120\" ($dir/within.out)"

if t gate-time-sum GATE_TIME_BUDGET=0.2 > "$dir/over.out" 2>&1; then
  fail "gate-time-sum exited 0 with test steps over a budget of 0.2 s ($dir/over.out)"
fi
grep -qE '^gate-time test-steps [0-9]+\.[0-9] of 0\.2(, cpu [0-9]+\.[0-9])?(, load [0-9.]+ on [0-9]+ cores)?$' "$dir/over.out" ||
  fail "gate-time-sum printed no line \"gate-time test-steps <sum> of 0.2\" ($dir/over.out)"
grep -qE '^gate-time: the test steps ran [0-9]+\.[0-9] s, over the budget of 0\.2 s; the slowest: paused [0-9]+\.[0-9], [a-z]+ [0-9]+\.[0-9], [a-z]+ [0-9]+\.[0-9]$' "$dir/over.out" ||
  fail "gate-time-sum did not name the sum and the three slowest steps, the paused one first ($dir/over.out)"

# In one task process, as in check, the sum line names the CPU seconds where /proc tells them;
# they do not make it fail.
t gate-time-start gate-time gate-time-sum STEP=one > "$dir/one.out" 2>&1 ||
  fail "the clock tasks in one task process exited non-zero ($dir/one.out)"
if [ -r /proc/self/stat ]; then
  grep -qE '^gate-time test-steps [0-9]+\.[0-9] of 120, cpu [0-9]+\.[0-9](, load [0-9.]+ on [0-9]+ cores)?$' "$dir/one.out" ||
    fail "gate-time-sum in the task process of gate-time-start printed no \", cpu <s>\" ($dir/one.out)"
fi

echo "$name: ok"

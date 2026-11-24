#!/bin/sh
# Planted defect of the time budget of the gate's test steps (taskfiles/gate-time.yml, tasks
# gate-time-start, gate-time and gate-time-sum; story gate-test-steps-within-budget, point 2): a
# gate run whose test steps take longer than GATE_TIME_BUDGET fails, naming the sum and the three
# slowest steps.
# The clock runs as in check, in a directory of its own (.out/gate-tests/<name>/run) whose
# Taskfile.yml includes taskfiles/gate-time.yml and adds the task paused: gate-time-start, then
# four steps, each a gate-time line, the first after a pause of one second. (A task of an included
# file runs in the directory of the Taskfile that includes it, so the real Taskfile.yml with
# task -d would run the clock in the repository root, on the gate's own .out/gate-time.) Its
# target/debug/deps holds one file older than gate-time-start, as a built tree has.
# 1. "task paused gate-time-sum", one task process as in check: four gate-time lines, and with
#    the default budget (120) gate-time-sum passes and prints "gate-time test-steps <sum> of 120,
#    cpu <s>" (the CPU seconds where /proc tells them; they do not make it fail without a bound),
#    then ", load <l> on <n> cores" where the machine tells; and, without a CPU bound as on the
#    forge, "gate-cpu test-steps <cpu>: paused <cpu>, second <cpu>, third <cpu>, fourth <cpu>".
# 2. gate-time-sum alone with GATE_TIME_BUDGET=0.2, GATE_TIME_CORES=7 and
#    GATE_CPU_BOUNDS=7:1000, in a task process of its own, on the same steps: it exits non-zero
#    and prints "gate-time test-steps <sum> of 0.2" (no CPU seconds: the clock started in
#    another task process) and "gate-time: the test steps ran <sum> s, over the budget of 0.2 s;
#    the slowest: <step> <s>, <step> <s>, <step> <s>", the paused step first; and, for the CPU
#    bound (story gate-test-steps-fit-four-cores), that the CPU seconds the bound of 1000 for 7
#    cores needs could not be read.
#    tests/gate/gate-time-refuses-test-steps-over-the-cpu-bound-or-building.sh holds the bound
#    where the CPU seconds are read.
# Run from the repository root: sh tests/gate/gate-time-refuses-test-steps-over-the-budget.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=gate-time-refuses-test-steps-over-the-budget
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/run/target/debug/deps" || { echo "$name: FAIL cannot make $dir"; exit 1; }

fail() {
  echo "$name: FAIL $*"
  exit 1
}

[ -f taskfiles/gate-time.yml ] || fail "no taskfiles/gate-time.yml"
: > "$dir/run/target/debug/deps/built-before-the-clock" || fail "cannot plant a file of the build phase"
printf "version: '3'\n\nincludes:\n  gate:\n    taskfile: %s\n    flatten: true\n\ntasks:\n%s\n" "$(pwd)/taskfiles/gate-time.yml" '  paused:
    silent: true
    cmds:
      - task: gate-time-start
      - sleep 1
      - task: gate-time
        vars: {STEP: paused}
      - task: gate-time
        vars: {STEP: second}
      - task: gate-time
        vars: {STEP: third}
      - task: gate-time
        vars: {STEP: fourth}' > "$dir/run/Taskfile.yml" || fail "cannot write the planted Taskfile.yml"
t() { task -d "$dir/run" "$@"; }

t paused gate-time-sum > "$dir/within.out" 2>&1 ||
  fail "the clock tasks with gate-time-sum refused four short steps under the default budget ($dir/within.out)"
[ "$(grep -cE '^gate-time [a-z]+ [0-9]+\.[0-9]$' "$dir/within.out")" -eq 4 ] || fail "four steps printed no four gate-time lines ($dir/within.out)"
grep -qE '^gate-time paused [0-9]+\.[0-9]$' "$dir/within.out" || fail "no gate-time line for the paused step ($dir/within.out)"
if [ -r /proc/self/stat ]; then
  grep -qE '^gate-time test-steps [0-9]+\.[0-9] of 120, cpu [0-9]+\.[0-9](, load [0-9.]+ on [0-9]+ cores)?$' "$dir/within.out" ||
    fail "gate-time-sum in the task process of gate-time-start printed no line \"gate-time test-steps <sum> of 120, cpu <s>\" ($dir/within.out)"
  grep -qE '^gate-cpu test-steps [0-9]+\.[0-9]: paused [0-9]+\.[0-9], second [0-9]+\.[0-9], third [0-9]+\.[0-9], fourth [0-9]+\.[0-9]$' "$dir/within.out" ||
    fail "gate-time-sum without a bound printed no line \"gate-cpu test-steps <cpu>: paused <cpu>, ...\" ($dir/within.out)"
else
  grep -qE '^gate-time test-steps [0-9]+\.[0-9] of 120(, load [0-9.]+ on [0-9]+ cores)?$' "$dir/within.out" ||
    fail "gate-time-sum printed no line \"gate-time test-steps <sum> of 120\" ($dir/within.out)"
fi

if t gate-time-sum GATE_TIME_BUDGET=0.2 GATE_TIME_CORES=7 GATE_CPU_BOUNDS=7:1000 > "$dir/over.out" 2>&1; then
  fail "gate-time-sum exited 0 with test steps over a budget of 0.2 s ($dir/over.out)"
fi
grep -qE '^gate-time test-steps [0-9]+\.[0-9] of 0\.2(, load [0-9.]+ on [0-9]+ cores)?$' "$dir/over.out" ||
  fail "gate-time-sum printed no line \"gate-time test-steps <sum> of 0.2\" without CPU seconds ($dir/over.out)"
grep -qE '^gate-time: the test steps ran [0-9]+\.[0-9] s, over the budget of 0\.2 s; the slowest: paused [0-9]+\.[0-9], [a-z]+ [0-9]+\.[0-9], [a-z]+ [0-9]+\.[0-9]$' "$dir/over.out" ||
  fail "gate-time-sum did not name the sum and the three slowest steps, the paused one first ($dir/over.out)"
grep -qE '^gate-cpu: the CPU seconds of the test steps could not be read .*; the bound of 1000 for 7 cores needs them$' "$dir/over.out" ||
  fail "gate-time-sum under a CPU bound did not say it could not read the CPU seconds in a task process of its own ($dir/over.out)"

echo "$name: ok"

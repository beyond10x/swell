#!/bin/sh
# Planted defect of the runner line of the gate (Taskfile.yml, tasks check and slow-runner;
# story gate-test-steps-hold-the-budget-on-the-slower-runner, item 1): check prints, after
# gate-time-start and before its first test step, one line "gate-runner: <n> cores, <model
# name>, <MemTotal> kB, sh is <sh>" that names the machine its test steps run on, so that two
# runs of the gate on the same code whose seconds differ can be read against their machines.
# The line comes from the task slow-runner with LABEL gate-runner; the lane slow calls the same
# task without a label and keeps its line "slow-runner: ...".
# No test runs and nothing is built.
# 1. task --dry --verbose check in the repository root (go-task prints each task it starts and
#    runs none): slow-runner starts once, after gate-time-start and before test, and the
#    commands it prints name the label gate-runner.
# 2. The same on a planted copy of Taskfile.yml whose check has no call of slow-runner: the
#    order check of point 1 refuses it, naming the missing line.
# 3. task slow-runner LABEL=gate-runner prints exactly one line, "gate-runner: <n> cores,
#    <cpu>, <kB> kB, sh is <sh>" (or the words the task prints for what it cannot read), and
#    task slow-runner alone prints the same with "slow-runner: ".
# Run from the repository root: sh tests/gate/check-prints-the-runner-line-before-its-first-test-step.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR LABEL
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=check-prints-the-runner-line-before-its-first-test-step
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/run" || { echo "$name: FAIL cannot make $dir"; exit 1; }

fail() {
  echo "$name: FAIL $*"
  exit 1
}

[ -f Taskfile.yml ] && [ -d taskfiles ] || fail "run it from the repository root"

# order <dry-run output>: exit 0 when slow-runner starts once, after gate-time-start and before
# test, and its commands name gate-runner; else print why and exit 1.
order() {
  LC_ALL=C awk '
    /^task: "[^"]*" started/ {
      n++
      t = $0; sub(/^task: "/, "", t); sub(/".*$/, "", t)
      if (t == "gate-time-start" && !start) start = n
      if (t == "test" && !first) first = n
      if (t == "slow-runner") { runner++; at = n; inrunner = 1; next }
      inrunner = 0
      next
    }
    inrunner && /gate-runner/ { labelled = 1 }
    END {
      if (!start) { print "no task gate-time-start started"; exit 1 }
      if (!first) { print "no task test started"; exit 1 }
      if (runner != 1) { print "slow-runner started " runner + 0 " times, not once"; exit 1 }
      if (!(start < at && at < first)) { print "slow-runner did not start between gate-time-start and test"; exit 1 }
      if (!labelled) { print "slow-runner printed no command that names gate-runner"; exit 1 }
    }
  ' "$1"
}

# 1. The tree's check.
task --dry --verbose check > "$dir/dry.out" 2>&1 || fail "task --dry --verbose check exited non-zero ($dir/dry.out)"
why=$(order "$dir/dry.out") || fail "check does not print the runner line before its first test step: $why ($dir/dry.out)"

# 2. A planted check without the call.
cp Taskfile.yml "$dir/run/Taskfile.yml" || fail "cannot copy Taskfile.yml"
cp -R taskfiles "$dir/run/taskfiles" || fail "cannot copy taskfiles"
mkdir -p "$dir/run/spec" "$dir/run/conformance/test262" || fail "cannot make $dir/run/spec"
cp spec/ess-inputs.yaml "$dir/run/spec/" || fail "cannot copy spec/ess-inputs.yaml"
cp conformance/test262/PIN "$dir/run/conformance/test262/" || fail "cannot copy conformance/test262/PIN"
LC_ALL=C awk '
  /^  [a-z0-9-]+:$/ { incheck = ($0 == "  check:") }
  incheck && $0 == "      - task: slow-runner" { skip = 1; dropped++; next }
  skip && /^        vars:/ { skip = 0; next }
  { skip = 0; print }
  END { exit dropped != 1 }
' Taskfile.yml > "$dir/run/Taskfile.yml" || fail "the check of Taskfile.yml holds no line \"      - task: slow-runner\" to take out"
task -d "$dir/run" --dry --verbose check > "$dir/planted.out" 2>&1 || fail "task --dry --verbose check exited non-zero on the planted copy ($dir/planted.out)"
if why=$(order "$dir/planted.out"); then
  fail "the order check passed a check without the runner line ($dir/planted.out)"
fi
[ "$why" = "slow-runner started 0 times, not once" ] || fail "the order check refused the planted check for another reason: $why ($dir/planted.out)"

# 3. The line itself, with the label and without.
form='([0-9]+ cores|cores not read), .+, ([0-9]+ kB|memory not read), sh is .+'
task slow-runner LABEL=gate-runner > "$dir/gate.out" 2>&1 || fail "task slow-runner LABEL=gate-runner exited non-zero ($dir/gate.out)"
[ "$(awk 'END { print NR }' "$dir/gate.out")" = 1 ] && grep -Eqx "gate-runner: $form" "$dir/gate.out" ||
  fail "task slow-runner LABEL=gate-runner did not print exactly one line \"gate-runner: <n> cores, <cpu>, <kB> kB, sh is <sh>\" ($dir/gate.out)"
task slow-runner > "$dir/lane.out" 2>&1 || fail "task slow-runner exited non-zero ($dir/lane.out)"
[ "$(awk 'END { print NR }' "$dir/lane.out")" = 1 ] && grep -Eqx "slow-runner: $form" "$dir/lane.out" ||
  fail "task slow-runner without a label did not print exactly one line \"slow-runner: <n> cores, ...\" ($dir/lane.out)"

echo "$name: ok"

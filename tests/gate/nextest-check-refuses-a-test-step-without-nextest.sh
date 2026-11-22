#!/bin/sh
# Planted defect of the task nextest-check (taskfiles/nextest.yml, story
# gate-test-steps-within-budget, point 8): a test step that does not run through cargo-nextest is
# refused. The task reads GATE_ROOT/Taskfile.yml and GATE_ROOT/taskfiles/*.yml; here GATE_ROOT is
# a copy under .out/gate-tests/<name>/. The unchanged copy passes first; then
#   1. the step test-faults with "cargo nextest run" written as "cargo test", and
#   2. a task of the root Taskfile.yml that runs "cargo test -p swell"
# each make it exit non-zero and name the file and line of the cargo test command; the first
# also names the step test-faults, which then runs no cargo nextest run.
# Run from the repository root: sh tests/gate/nextest-check-refuses-a-test-step-without-nextest.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=nextest-check-refuses-a-test-step-without-nextest
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/root/.config" "$dir/root/taskfiles"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

root=$dir/root
cp .config/nextest.toml "$root/.config/nextest.toml" || fail "cannot copy .config/nextest.toml"
cp Taskfile.yml "$root/Taskfile.yml" || fail "cannot copy Taskfile.yml"
cp taskfiles/*.yml "$root/taskfiles/" || fail "cannot copy taskfiles"
task -d . nextest-check GATE_ROOT="$root" > "$dir/base.out" 2>&1 ||
  fail "task nextest-check on the unchanged copy exited non-zero ($dir/base.out)"
grep -q '^nextest-check: ' "$dir/base.out" || fail "task nextest-check printed no line ($dir/base.out)"

faults=$root/taskfiles/fault-injection.yml
grep -q 'cargo nextest run' taskfiles/fault-injection.yml || fail "taskfiles/fault-injection.yml runs no cargo nextest run"
sed 's/cargo nextest run/cargo test/' taskfiles/fault-injection.yml > "$faults" || fail "cannot plant the step"
if task -d . nextest-check GATE_ROOT="$root" > "$dir/step.out" 2>&1; then
  fail "task nextest-check exited 0 with test-faults running cargo test ($dir/step.out)"
fi
grep -qE "^nextest-check: $faults:[0-9]+: cargo test runs tests outside cargo-nextest" "$dir/step.out" ||
  fail "task nextest-check did not name the line of $faults ($dir/step.out)"
grep -qF "nextest-check: the step test-faults runs no cargo nextest run" "$dir/step.out" ||
  fail "task nextest-check did not name the step test-faults ($dir/step.out)"
cp taskfiles/fault-injection.yml "$faults" || fail "cannot copy taskfiles/fault-injection.yml again"

{
  cat Taskfile.yml
  printf '\n  test-planted:\n    cmds:\n      - cargo test --locked -p swell\n'
} > "$root/Taskfile.yml" || fail "cannot plant the task"
if task -d . nextest-check GATE_ROOT="$root" > "$dir/task.out" 2>&1; then
  fail "task nextest-check exited 0 with a task running cargo test ($dir/task.out)"
fi
grep -qE "^nextest-check: $root/Taskfile.yml:[0-9]+: cargo test runs tests outside cargo-nextest" "$dir/task.out" ||
  fail "task nextest-check did not name the line of $root/Taskfile.yml ($dir/task.out)"

echo "$name: ok"

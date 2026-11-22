#!/bin/sh
# Planted defect of the task runner-env-check (taskfiles/forge.yml, story
# local-gate-equals-forge, point 2): check-runner sets every variable the forge's runner has
# for task check, and no other. The task reads GATE_ROOT/.github/workflows/check.yml; here
# GATE_ROOT is a copy under .out/gate-tests/<name>/. The unchanged copy passes first; then
#   1. a variable in the workflow's env that check-runner does not set,
#   2. a variable in the env of the step that runs task check that check-runner does not set,
#   3. check-runner without CARGO_INCREMENTAL (GATE_RUNNER_ENV on the command line),
#   4. check-runner with another value of CARGO_INCREMENTAL, and
#   5. check-runner with an empty RUSTC_WRAPPER, which neither the workflow nor the runner sets
# each make it exit non-zero and name the variable.
# Run from the repository root: sh tests/gate/runner-env-check-refuses-a-variable-the-task-does-not-set.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT GATE_RUNNER_ENV
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=runner-env-check-refuses-a-variable-the-task-does-not-set
dir=.out/gate-tests/$name
rm -rf "$dir"
fail() {
  echo "$name: FAIL $*"
  exit 1
}
mkdir -p "$dir/root/.github/workflows" || fail "cannot make $dir/root"

root=$dir/root
workflow=$root/.github/workflows/check.yml
cp .github/workflows/check.yml "$workflow" || fail "cannot copy check.yml"
task -d . runner-env-check GATE_ROOT="$root" > "$dir/base.out" 2>&1 ||
  fail "task runner-env-check on the unchanged copy exited non-zero ($dir/base.out)"
grep -q '^runner-env-check: check-runner sets CI=true GITHUB_ACTIONS=true ' "$dir/base.out" ||
  fail "task runner-env-check printed no line naming the runner's variables ($dir/base.out)"

awk '{ print } /^env:$/ { print "  RUST_BACKTRACE: \"1\"" }' .github/workflows/check.yml > "$workflow" ||
  fail "cannot plant the variable of the workflow"
grep -qx '  RUST_BACKTRACE: "1"' "$workflow" || fail "check.yml has no env block to plant a variable in"
if task -d . runner-env-check GATE_ROOT="$root" > "$dir/workflow.out" 2>&1; then
  fail "task runner-env-check exited 0 with RUST_BACKTRACE set by the workflow alone ($dir/workflow.out)"
fi
grep -qE "^runner-env-check: $workflow:[0-9]+: the workflow sets RUST_BACKTRACE=1 for task check, check-runner does not" "$dir/workflow.out" ||
  fail "task runner-env-check did not name RUST_BACKTRACE ($dir/workflow.out)"

awk '
  { print }
  /^ *- name: task check$/ { match($0, /^ */); pad = substr($0, 1, RLENGTH) "  "; print pad "env:"; print pad "  RUST_MIN_STACK: \"8388608\"" }
' .github/workflows/check.yml > "$workflow" || fail "cannot plant the variable of the step"
grep -q 'RUST_MIN_STACK' "$workflow" || fail "check.yml has no step named task check to plant a variable in"
if task -d . runner-env-check GATE_ROOT="$root" > "$dir/step.out" 2>&1; then
  fail "task runner-env-check exited 0 with RUST_MIN_STACK set by the step alone ($dir/step.out)"
fi
grep -qE "^runner-env-check: $workflow:[0-9]+: the workflow sets RUST_MIN_STACK=8388608 for task check, check-runner does not" "$dir/step.out" ||
  fail "task runner-env-check did not name RUST_MIN_STACK ($dir/step.out)"
cp .github/workflows/check.yml "$workflow" || fail "cannot copy check.yml again"

if task -d . runner-env-check GATE_ROOT="$root" GATE_RUNNER_ENV="CI=true GITHUB_ACTIONS=true CARGO_TERM_COLOR=always" > "$dir/missing.out" 2>&1; then
  fail "task runner-env-check exited 0 with check-runner not setting CARGO_INCREMENTAL ($dir/missing.out)"
fi
grep -qE "^runner-env-check: $workflow:[0-9]+: the workflow sets CARGO_INCREMENTAL=0 for task check, check-runner does not" "$dir/missing.out" ||
  fail "task runner-env-check did not name CARGO_INCREMENTAL ($dir/missing.out)"

if task -d . runner-env-check GATE_ROOT="$root" GATE_RUNNER_ENV="CI=true GITHUB_ACTIONS=true CARGO_TERM_COLOR=always CARGO_INCREMENTAL=1" > "$dir/value.out" 2>&1; then
  fail "task runner-env-check exited 0 with check-runner setting CARGO_INCREMENTAL=1 ($dir/value.out)"
fi
grep -qE "^runner-env-check: $workflow:[0-9]+: the workflow sets CARGO_INCREMENTAL=0, check-runner sets CARGO_INCREMENTAL=1" "$dir/value.out" ||
  fail "task runner-env-check did not name both values of CARGO_INCREMENTAL ($dir/value.out)"

if task -d . runner-env-check GATE_ROOT="$root" GATE_RUNNER_ENV="CI=true GITHUB_ACTIONS=true CARGO_TERM_COLOR=always CARGO_INCREMENTAL=0 RUSTC_WRAPPER=" > "$dir/extra.out" 2>&1; then
  fail "task runner-env-check exited 0 with check-runner setting RUSTC_WRAPPER ($dir/extra.out)"
fi
grep -qF "runner-env-check: check-runner sets RUSTC_WRAPPER=, which neither $workflow nor the runner sets for task check" "$dir/extra.out" ||
  fail "task runner-env-check did not name RUSTC_WRAPPER ($dir/extra.out)"

echo "$name: ok"

#!/bin/sh
# Planted defect of the task nextest-check (taskfiles/nextest.yml, story
# gate-test-steps-within-budget, point 8): a .config/nextest.toml that lets a test run past 60 s
# is refused. The task reads GATE_ROOT/.config/nextest.toml; here GATE_ROOT is a copy of the
# task files and the configuration under .out/gate-tests/<name>/. The unchanged copy passes
# first; then each of these makes it exit non-zero with "nextest-check: <file>:<line>: ...":
#   1. the period of the default profile's slow-timeout raised to 120s;
#   2. its terminate-after removed, so that a slow test is reported and never terminated;
#   3. an override whose slow-timeout terminates after 3 periods of 30s (90 s);
#   4. the default profile's slow-timeout written as a table of its own, period 61s;
#   5. no slow-timeout in the default profile at all.
# Run from the repository root: sh tests/gate/nextest-check-refuses-a-test-past-60-seconds.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=nextest-check-refuses-a-test-past-60-seconds
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/root/.config" "$dir/root/taskfiles"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

config=$dir/root/.config/nextest.toml
[ -f .config/nextest.toml ] || fail "no .config/nextest.toml"
cp Taskfile.yml "$dir/root/Taskfile.yml" || fail "cannot copy Taskfile.yml"
cp taskfiles/*.yml "$dir/root/taskfiles/" || fail "cannot copy taskfiles"
grep -qx 'slow-timeout = { period = "60s", terminate-after = 1 }' .config/nextest.toml ||
  fail ".config/nextest.toml has no line slow-timeout = { period = \"60s\", terminate-after = 1 }"

cp .config/nextest.toml "$config" || fail "cannot copy .config/nextest.toml"
task -d . nextest-check GATE_ROOT="$dir/root" > "$dir/base.out" 2>&1 ||
  fail "task nextest-check on the unchanged copy exited non-zero ($dir/base.out)"
grep -q '^nextest-check: ' "$dir/base.out" || fail "task nextest-check printed no line ($dir/base.out)"

refused() {
  # $1: the planted case, $2: what the refusal names
  if task -d . nextest-check GATE_ROOT="$dir/root" > "$dir/$1.out" 2>&1; then
    fail "task nextest-check exited 0 with $1 ($dir/$1.out)"
  fi
  grep -qF "nextest-check: $config:" "$dir/$1.out" ||
    fail "task nextest-check did not name the line of $config for $1 ($dir/$1.out)"
  grep -qF "$2" "$dir/$1.out" || fail "task nextest-check did not say \"$2\" for $1 ($dir/$1.out)"
}

sed 's/^slow-timeout = { period = "60s", terminate-after = 1 }$/slow-timeout = { period = "120s", terminate-after = 1 }/' \
  .config/nextest.toml > "$config" || fail "cannot plant the period"
refused period "120 s"

sed 's/^slow-timeout = { period = "60s", terminate-after = 1 }$/slow-timeout = { period = "60s" }/' \
  .config/nextest.toml > "$config" || fail "cannot plant the missing terminate-after"
refused unterminated "no terminate-after"

{
  cat .config/nextest.toml
  printf '\n[[profile.default.overrides]]\nfilter = "all()"\nslow-timeout = { period = "30s", terminate-after = 3 }\n'
} > "$config" || fail "cannot plant the override"
refused override "90 s"

{
  grep -v '^slow-timeout = ' .config/nextest.toml
  printf '\n[profile.default.slow-timeout]\nperiod = "61s"\nterminate-after = 1\n'
} > "$config" || fail "cannot plant the table"
refused table "61 s"

grep -v '^slow-timeout = ' .config/nextest.toml > "$config" || fail "cannot plant the missing slow-timeout"
if task -d . nextest-check GATE_ROOT="$dir/root" > "$dir/none.out" 2>&1; then
  fail "task nextest-check exited 0 with no slow-timeout in the default profile ($dir/none.out)"
fi
grep -qF "nextest-check: $config: the profile default sets no slow-timeout" "$dir/none.out" ||
  fail "task nextest-check did not say the default profile sets no slow-timeout ($dir/none.out)"

echo "$name: ok"

#!/bin/sh
# Planted defect of the lane slow (taskfiles/test262.yml, task slow-test262, which task slow runs;
# story gate-test-steps-within-budget, point 8): a line of the expected file changed with the
# digest left alone is refused by the comparison of the full run with the expected file.
# The run is reduced: TEST262_RUN=none counts every case of a small checkout of its own
# (tests/gate/fixtures/test262-checkout.sh) and runs none, as
# test262-refuses-a-count-that-differs-from-expected.sh does. The task test262-expected writes
# the expected file and its digest for that mode under .out/gate-tests/<name>/; slow-test262
# passes on them first. Then one test262-section line of the expected file has a count raised by
# one, the digest file unchanged: the fast check test262-check cannot see it (the sources did
# not change, and section lines are no sum), and slow-test262 must exit non-zero, print the line
# with - and the printed one with +, and "test262: 1 line(s) differ from <file>".
# Run from the repository root: sh tests/gate/slow-refuses-a-changed-expected-line.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=slow-refuses-a-changed-expected-line
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

export TEST262_RUN=none
. tests/gate/fixtures/test262-checkout.sh
test262_checkout "$dir"
task -d . test262-expected $checkout_vars TEST262_DESIGN="$checkout_design" TEST262_EXPECTED="$dir/expected.txt" TEST262_DIGEST="$dir/expected.digest" \
  TEST262_OUT="$dir/write" > "$dir/write.out" 2>&1 ||
  fail "task test262-expected exited non-zero ($dir/write.out)"
[ -s "$dir/expected.txt" ] || fail "task test262-expected wrote no $dir/expected.txt ($dir/write.out)"
grep -q '^test262-sources [0-9a-f]\{64\}$' "$dir/expected.digest" ||
  fail "task test262-expected wrote no digest line to $dir/expected.digest ($dir/write.out)"

task -d . slow-test262 $checkout_vars TEST262_DESIGN="$checkout_design" TEST262_EXPECTED="$dir/expected.txt" TEST262_DIGEST="$dir/expected.digest" \
  TEST262_OUT="$dir/base" > "$dir/base.out" 2>&1 ||
  fail "task slow-test262 on the written files exited non-zero ($dir/base.out)"
grep -qF "byte for byte" "$dir/base.out" ||
  fail "task slow-test262 on the written files printed no byte comparison ($dir/base.out)"

line=$(grep -m 1 '^test262-section ' "$dir/expected.txt") || fail "$dir/expected.txt has no test262-section line"
changed=$(printf '%s\n' "$line" | awk '{ $4 = $4 + 1; print }')
awk -v line="$line" -v changed="$changed" '$0 == line && !done { $0 = changed; done = 1 } { print }' \
  "$dir/expected.txt" > "$dir/planted-expected.txt" || fail "cannot write the planted expected file"
cmp -s "$dir/expected.txt" "$dir/planted-expected.txt" && fail "the planted expected file equals the written one"

task -d . test262-check $checkout_vars TEST262_DESIGN="$checkout_design" TEST262_EXPECTED="$dir/planted-expected.txt" TEST262_DIGEST="$dir/expected.digest" \
  TEST262_OUT="$dir/fast" > "$dir/fast.out" 2>&1 ||
  fail "task test262-check refused the planted line; the fast check is not what this script shows ($dir/fast.out)"

if task -d . slow-test262 $checkout_vars TEST262_DESIGN="$checkout_design" TEST262_EXPECTED="$dir/planted-expected.txt" TEST262_DIGEST="$dir/expected.digest" \
  TEST262_OUT="$dir/planted" > "$dir/planted.out" 2>&1; then
  fail "task slow-test262 exited 0 with a changed line in the expected file ($dir/planted.out)"
fi
grep -qxF -- "-$changed" "$dir/planted.out" || fail "task slow-test262 did not print -$changed ($dir/planted.out)"
grep -qxF -- "+$line" "$dir/planted.out" || fail "task slow-test262 did not print +$line ($dir/planted.out)"
grep -qxF "test262: 1 line(s) differ from $dir/planted-expected.txt" "$dir/planted.out" ||
  fail "task slow-test262 did not say 1 line differs from the expected file ($dir/planted.out)"

echo "$name: ok"

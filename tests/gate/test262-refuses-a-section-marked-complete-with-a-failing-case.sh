#!/bin/sh
# Planted defect of taskfiles/test262.yml: a copy of the expected file that marks
# the section sec-proxy-objects complete while its cases fail (the engine passes none). The
# runner judges it incomplete, so `task test262` must exit non-zero and print the planted line
# with - and the printed one with +. The unchanged copy passes first.
# The defect is on the counting side, so every run of the task counts a small checkout of its own
# with TEST262_RUN=none (no case is run; every run case is counted failed, so the section is
# incomplete in this mode as it is with the cases run), and the expected file is that mode's,
# made here from the task's own lines.
# The checkout is made by tests/gate/fixtures/test262-checkout.sh (a case under each path of
# sections.txt, one naming generators), with a copy of the design whose slice lines are its own.
# Run from the repository root: sh tests/gate/test262-refuses-a-section-marked-complete-with-a-failing-case.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test262-refuses-a-section-marked-complete-with-a-failing-case
dir=.out/gate-tests/$name
anchor=sec-proxy-objects
rm -rf "$dir"
mkdir -p "$dir"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

export TEST262_RUN=none
. tests/gate/fixtures/test262-checkout.sh
test262_checkout "$dir"
cp "$dir/checkout-lines.txt" "$dir/expected.txt" || fail "cannot copy $dir/checkout-lines.txt"
task -d . test262 $checkout_vars TEST262_DESIGN="$checkout_design" TEST262_EXPECTED="$dir/expected.txt" TEST262_OUT="$dir/base" > "$dir/base.out" 2>&1 ||
  fail "task test262 on the unchanged expected file exited non-zero ($dir/base.out)"
grep -qF "lines equal $dir/expected.txt" "$dir/base.out" ||
  fail "task test262 on the unchanged expected file printed no \"lines equal\" line ($dir/base.out)"

line=$(grep "^test262-section $anchor incomplete " "$dir/expected.txt") ||
  fail "expected.txt has no incomplete line of $anchor"
planted=$(printf '%s\n' "$line" | awk '{ $3 = "complete"; print }')
awk -v line="$line" -v planted="$planted" '$0 == line { $0 = planted } { print }' \
  "$dir/expected.txt" > "$dir/planted-expected.txt"
cmp -s "$dir/expected.txt" "$dir/planted-expected.txt" && fail "the planted expected file equals expected.txt"

if task -d . test262 $checkout_vars TEST262_DESIGN="$checkout_design" TEST262_EXPECTED="$dir/planted-expected.txt" TEST262_OUT="$dir/planted" \
  > "$dir/planted.out" 2>&1; then
  fail "task test262 exited 0 with $anchor marked complete ($dir/planted.out)"
fi
grep -qx -- "-$planted" "$dir/planted.out" || fail "task test262 did not print -$planted ($dir/planted.out)"
grep -qx -- "+$line" "$dir/planted.out" || fail "task test262 did not print +$line ($dir/planted.out)"
grep -qxF "test262: 1 line(s) differ from $dir/planted-expected.txt" "$dir/planted.out" ||
  fail "task test262 did not say 1 line differs from the expected file ($dir/planted.out)"

echo "$name: ok"

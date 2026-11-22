#!/bin/sh
# Planted defect of taskfiles/test262.yml: a copy of docs/design/engine.md whose first line after
# "Its output on this document:" has its case count raised by one. The runner's slice lines then
# differ from the design: `task test262` must exit non-zero and say so, printing the line on both
# sides. The unchanged copy passes first.
# The defect is on the counting side (fields 2 to 4 of a slice line do not depend on any run), so
# every run of the task counts a small checkout of its own with TEST262_RUN=none (no case is run;
# every run case is counted failed), against that mode's expected file, made here from the
# task's own lines.
# The checkout is made by tests/gate/fixtures/test262-checkout.sh (a case under each path of
# sections.txt, one naming generators), with a copy of the design whose slice lines are its own.
# Run from the repository root: sh tests/gate/test262-refuses-a-slice-line-that-differs-from-the-design.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test262-refuses-a-slice-line-that-differs-from-the-design
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
cp "$dir/checkout-lines.txt" "$dir/expected.txt" || fail "cannot copy $dir/checkout-lines.txt"
export TEST262_EXPECTED="$dir/expected.txt"
cp "$checkout_design" "$dir/engine.md" || fail "cannot copy $checkout_design"
task -d . test262 $checkout_vars TEST262_DESIGN="$dir/engine.md" TEST262_OUT="$dir/base" > "$dir/base.out" 2>&1 ||
  fail "task test262 on the unchanged design exited non-zero ($dir/base.out)"
grep -q '^test262: [0-9]* slice lines equal the design$' "$dir/base.out" ||
  fail "task test262 on the unchanged design did not say the slice lines equal it ($dir/base.out)"

awk '
  $0 == "Its output on this document:" { f = 1 }
  f && !done && NF == 3 && $2 ~ /^[0-9]+$/ { old = $0; $2 = $2 + 1; done = 1; print old > "/dev/stderr" }
  { print }
' "$dir/engine.md" > "$dir/planted-engine.md" 2> "$dir/planted-line.txt"
old=$(cat "$dir/planted-line.txt")
[ -n "$old" ] || fail "no slice line after \"Its output on this document:\" in the design"
cmp -s "$dir/engine.md" "$dir/planted-engine.md" && fail "the planted design equals the design"

if task -d . test262 $checkout_vars TEST262_DESIGN="$dir/planted-engine.md" TEST262_OUT="$dir/planted" > "$dir/planted.out" 2>&1; then
  fail "task test262 exited 0 with a changed slice line in the design ($dir/planted.out)"
fi
grep -qF "test262: the slice lines differ from $dir/planted-engine.md" "$dir/planted.out" ||
  fail "task test262 did not say the slice lines differ from the design ($dir/planted.out)"
grep -qx "> $old" "$dir/planted.out" ||
  fail "task test262 did not print the runner's line \"$old\" ($dir/planted.out)"

echo "$name: ok"

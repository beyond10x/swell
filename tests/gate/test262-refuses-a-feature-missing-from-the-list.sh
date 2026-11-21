#!/bin/sh
# Planted defect of taskfiles/test262.yml: conformance/test262/features.txt without the line of a
# feature that cases of the pinned checkout name. `task test262` must exit non-zero, name the
# feature as unlisted and as declared by test262 but not listed. The unchanged copy passes first.
# Run from the repository root: sh tests/gate/test262-refuses-a-feature-missing-from-the-list.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test262-refuses-a-feature-missing-from-the-list
dir=.out/gate-tests/$name
feature=generators
rm -rf "$dir"
mkdir -p "$dir"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

cp conformance/test262/features.txt "$dir/features.txt" || fail "cannot copy features.txt"
task -d . test262 TEST262_FEATURES="$dir/features.txt" TEST262_OUT="$dir/base" > "$dir/base.out" 2>&1 ||
  fail "task test262 on the unchanged features.txt exited non-zero ($dir/base.out)"
grep -q '^test262: [0-9]* lines equal ' "$dir/base.out" ||
  fail "task test262 on the unchanged features.txt printed no \"lines equal\" line ($dir/base.out)"

grep -qx "run $feature" "$dir/features.txt" || fail "features.txt has no line \"run $feature\""
grep -vx "run $feature" "$dir/features.txt" > "$dir/planted-features.txt"
if task -d . test262 TEST262_FEATURES="$dir/planted-features.txt" TEST262_OUT="$dir/planted" > "$dir/planted.out" 2>&1; then
  fail "task test262 exited 0 with $feature missing from features.txt ($dir/planted.out)"
fi
grep -q "^test262-unlisted-feature $feature [1-9][0-9]*$" "$dir/planted.out" ||
  fail "task test262 did not name $feature as an unlisted feature ($dir/planted.out)"
grep -qx "features.txt: $feature declared by test262, not listed" "$dir/planted.out" ||
  fail "task test262 did not name $feature as declared but not listed ($dir/planted.out)"

echo "$name: ok"

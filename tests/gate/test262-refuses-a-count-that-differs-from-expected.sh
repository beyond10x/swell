#!/bin/sh
# Planted defects of taskfiles/test262.yml (epic:foundation, acceptance item 3): a changed count in
# either the expected file or the printed lines fails the gate.
# 1. A copy of conformance/test262/expected.txt whose total line has its case count raised by one:
#    `task test262` must exit non-zero and print that line with - and the printed one with +,
#    then "test262: 1 line(s) differ from <file>".
# 2. A copy of conformance/test262/features.txt that excludes a feature it runs, so that the
#    printed counts change, against the unchanged expected file: `task test262` must exit non-zero
#    and print "line(s) differ from".
# The unchanged copies pass first.
# Run from the repository root: sh tests/gate/test262-refuses-a-count-that-differs-from-expected.sh
name=test262-refuses-a-count-that-differs-from-expected
dir=.out/gate-tests/$name
feature=generators
rm -rf "$dir"
mkdir -p "$dir"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

cp conformance/test262/expected.txt "$dir/expected.txt" || fail "cannot copy expected.txt"
cp conformance/test262/features.txt "$dir/features.txt" || fail "cannot copy features.txt"
task -d . test262 TEST262_EXPECTED="$dir/expected.txt" TEST262_FEATURES="$dir/features.txt" \
  TEST262_OUT="$dir/base" > "$dir/base.out" 2>&1 ||
  fail "task test262 on the unchanged files exited non-zero ($dir/base.out)"
grep -qF "lines equal $dir/expected.txt" "$dir/base.out" ||
  fail "task test262 on the unchanged files printed no \"lines equal\" line ($dir/base.out)"

total=$(grep '^test262 total ' "$dir/expected.txt") || fail "expected.txt has no total line"
changed=$(printf '%s\n' "$total" | awk '{ $3 = $3 + 1; print }')
awk -v total="$total" -v changed="$changed" '$0 == total { $0 = changed } { print }' \
  "$dir/expected.txt" > "$dir/planted-expected.txt"
cmp -s "$dir/expected.txt" "$dir/planted-expected.txt" && fail "the planted expected file equals expected.txt"
if task -d . test262 TEST262_EXPECTED="$dir/planted-expected.txt" TEST262_OUT="$dir/planted-file" \
  > "$dir/planted-file.out" 2>&1; then
  fail "task test262 exited 0 with a changed count in the expected file ($dir/planted-file.out)"
fi
grep -qx -- "-$changed" "$dir/planted-file.out" ||
  fail "task test262 did not print -$changed ($dir/planted-file.out)"
grep -qx -- "+$total" "$dir/planted-file.out" ||
  fail "task test262 did not print +$total ($dir/planted-file.out)"
grep -qxF "test262: 1 line(s) differ from $dir/planted-expected.txt" "$dir/planted-file.out" ||
  fail "task test262 did not say 1 line differs from the expected file ($dir/planted-file.out)"

grep -qx "run $feature" "$dir/features.txt" || fail "features.txt has no line \"run $feature\""
awk -v feature="$feature" '$0 == "run " feature { $0 = "exclude " feature " planted decision-1" } { print }' \
  "$dir/features.txt" > "$dir/planted-features.txt"
if task -d . test262 TEST262_FEATURES="$dir/planted-features.txt" TEST262_OUT="$dir/planted-output" \
  > "$dir/planted-output.out" 2>&1; then
  fail "task test262 exited 0 with changed printed counts ($dir/planted-output.out)"
fi
grep -q '^test262: [1-9][0-9]* line(s) differ from conformance/test262/expected.txt$' "$dir/planted-output.out" ||
  fail "task test262 did not say the printed lines differ from expected.txt ($dir/planted-output.out)"
grep -q '^+test262 total ' "$dir/planted-output.out" ||
  fail "task test262 did not print the changed total line with + ($dir/planted-output.out)"

echo "$name: ok"

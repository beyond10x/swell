#!/bin/sh
# Planted defect of taskfiles/test262.yml: a copy of conformance/test262/expected.txt that marks
# the section sec-proxy-objects complete while its cases fail (the engine passes none). The
# runner judges it incomplete, so `task test262` must exit non-zero and print the planted line
# with - and the printed one with +. The unchanged copy passes first.
# Run from the repository root: sh tests/gate/test262-refuses-a-section-marked-complete-with-a-failing-case.sh
name=test262-refuses-a-section-marked-complete-with-a-failing-case
dir=.out/gate-tests/$name
anchor=sec-proxy-objects
rm -rf "$dir"
mkdir -p "$dir"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

cp conformance/test262/expected.txt "$dir/expected.txt" || fail "cannot copy expected.txt"
task -d . test262 TEST262_EXPECTED="$dir/expected.txt" TEST262_OUT="$dir/base" > "$dir/base.out" 2>&1 ||
  fail "task test262 on the unchanged expected file exited non-zero ($dir/base.out)"
grep -qF "lines equal $dir/expected.txt" "$dir/base.out" ||
  fail "task test262 on the unchanged expected file printed no \"lines equal\" line ($dir/base.out)"

line=$(grep "^test262-section $anchor incomplete " "$dir/expected.txt") ||
  fail "expected.txt has no incomplete line of $anchor"
planted=$(printf '%s\n' "$line" | awk '{ $3 = "complete"; print }')
awk -v line="$line" -v planted="$planted" '$0 == line { $0 = planted } { print }' \
  "$dir/expected.txt" > "$dir/planted-expected.txt"
cmp -s "$dir/expected.txt" "$dir/planted-expected.txt" && fail "the planted expected file equals expected.txt"

if task -d . test262 TEST262_EXPECTED="$dir/planted-expected.txt" TEST262_OUT="$dir/planted" \
  > "$dir/planted.out" 2>&1; then
  fail "task test262 exited 0 with $anchor marked complete ($dir/planted.out)"
fi
grep -qx -- "-$planted" "$dir/planted.out" || fail "task test262 did not print -$planted ($dir/planted.out)"
grep -qx -- "+$line" "$dir/planted.out" || fail "task test262 did not print +$line ($dir/planted.out)"
grep -qxF "test262: 1 line(s) differ from $dir/planted-expected.txt" "$dir/planted.out" ||
  fail "task test262 did not say 1 line differs from the expected file ($dir/planted.out)"

echo "$name: ok"

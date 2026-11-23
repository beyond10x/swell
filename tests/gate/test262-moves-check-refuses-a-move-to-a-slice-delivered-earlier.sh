#!/bin/sh
# Planted defect of taskfiles/test262.yml, task test262-moves-check: a line of the list of moved
# cases whose <to> is a slice delivered before its <from>. The slices are not delivered in the
# order of their numbers (slice 6 before 5), and the line "order delivery ..." of the
# swell-slices block names the order; the task must compare a line's two slices by their place
# in it, not by their numbers. On a planted checkout of two cases and a copy of the design whose
# block gives their files slices 6 and 5:
#   1. a line that moves a case from 5 to 6 (6 is delivered first) must be refused, naming the
#      path and saying its <to> is not delivered after its <from>;
#   2. a line that moves a case from 6 to 5 (5 is delivered after 6) must be admitted;
#   3. the same admitted line against a copy whose block has no order line must be refused,
#      saying the block names no order.
# Run from the repository root:
#   sh tests/gate/test262-moves-check-refuses-a-move-to-a-slice-delivered-earlier.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test262-moves-check-refuses-a-move-to-a-slice-delivered-earlier
dir=.out/gate-tests/$name
design=${TEST262_DESIGN:-docs/design/engine.md}
unset TEST262_DESIGN TEST262_CACHE TEST262_COMMIT TEST262_MOVES TEST262_LATER_PARTS
rm -rf "$dir"
mkdir -p "$dir"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

[ -s "$design" ] || fail "no $design"
grep -q '^order delivery ' "$design" || fail "$design has no line \"order delivery ...\""

commit=planted
fake=$dir/checkout
planted_dir=test/built-ins/planted-moves-order
later=$planted_dir/let-declaration.js
earlier=$planted_dir/regexp-call.js
mkdir -p "$fake/$commit/$planted_dir" "$fake/$commit/harness" || fail "cannot make $fake"
printf '/*---\ndescription: a let declaration (slice 6)\n---*/\nlet x = 1;\n' > "$fake/$commit/$later" ||
  fail "cannot write $later"
printf '/*---\ndescription: a call of RegExp (slice 5)\n---*/\nvar r = RegExp("a");\n' > "$fake/$commit/$earlier" ||
  fail "cannot write $earlier"
awk -v a="dir $later 6" -v b="dir $earlier 5" '{ print } $0 == "```swell-slices" { print a; print b }' \
  "$design" > "$dir/engine.md" || fail "cannot copy the design"
awk '$0 == "```swell-slices" { f = 1; next } f && $0 == "```" { exit } f && $1 == "order" { print $3 }' "$dir/engine.md" |
  awk -F, '{ for (i = 1; i <= NF; i++) at[$i] = i } END { exit !(at[6] && at[5] && at[6] < at[5]) }' ||
  fail "the order of $design does not deliver slice 6 before slice 5"
awk '!($0 ~ /^order delivery /)' "$dir/engine.md" > "$dir/engine-no-order.md" || fail "cannot copy the design"
cmp -s "$dir/engine.md" "$dir/engine-no-order.md" && fail "the copy without the order line equals the design"

check() {
  task -d . test262-moves-check TEST262_CACHE="$fake" TEST262_COMMIT="$commit" TEST262_DESIGN="$1" \
    TEST262_MOVES="$2" > "$3" 2>&1
}

printf '%s\t5\t6\tpart\tlet\n' "$later" > "$dir/five-to-six.tsv"
if check "$dir/engine.md" "$dir/five-to-six.tsv" "$dir/five-to-six.out"; then
  fail "task test262-moves-check exited 0 with a line that moves $later from 5 to 6, a slice delivered before 5 ($dir/five-to-six.out)"
fi
grep -qF ": $later: to 6 is not greater than from 5 in the order of delivery" "$dir/five-to-six.out" ||
  fail "task test262-moves-check did not say slice 6 of $later is not delivered after slice 5 ($dir/five-to-six.out)"

printf '%s\t6\t5\tpart\tRegExp\n' "$earlier" > "$dir/six-to-five.tsv"
check "$dir/engine.md" "$dir/six-to-five.tsv" "$dir/six-to-five.out" ||
  fail "task test262-moves-check refused a line that moves $earlier from 6 to 5, a slice delivered after 6 ($dir/six-to-five.out)"
grep -q '^test262-moves: 1 case(s) moved, 5:1; ' "$dir/six-to-five.out" ||
  fail "task test262-moves-check did not count the line of $earlier in slice 5 ($dir/six-to-five.out)"

if check "$dir/engine-no-order.md" "$dir/six-to-five.tsv" "$dir/no-order.out"; then
  fail "task test262-moves-check exited 0 with a design whose block names no order ($dir/no-order.out)"
fi
grep -q '^test262-moves: no line "order delivery ' "$dir/no-order.out" ||
  fail "task test262-moves-check did not say the block names no order ($dir/no-order.out)"

echo "$name: ok"

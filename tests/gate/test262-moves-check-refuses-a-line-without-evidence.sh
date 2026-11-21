#!/bin/sh
# Planted defect of taskfiles/test262.yml, task test262-moves-check: a copy of
# conformance/test262/moves.tsv in which one line of kind "part" names as its evidence a token of
# conformance/test262/later-parts.tsv of the same slice that the case does not hold, and a copy
# in which one line moves its case into slice 2. `task test262-moves-check` must exit non-zero on
# each and name the path of the planted line. The unchanged copy passes first.
# The planted evidence is the first token of later-parts.tsv with the line's target slice that
# `grep -F` finds nowhere in the case, taken from the first "part" line whose case includes no
# harness file (so the case file is all the task may read for it).
# Run from the repository root: sh tests/gate/test262-moves-check-refuses-a-line-without-evidence.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test262-moves-check-refuses-a-line-without-evidence
dir=.out/gate-tests/$name
moves=conformance/test262/moves.tsv
parts=conformance/test262/later-parts.tsv
design=${TEST262_DESIGN:-docs/design/engine.md}
cache=${TEST262_CACHE:-.cache/test262}
unset TEST262_DESIGN TEST262_CACHE TEST262_COMMIT TEST262_MOVES TEST262_LATER_PARTS
rm -rf "$dir"
mkdir -p "$dir"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

commit=$(awk '$1 == "commit" { print $2 }' conformance/test262/PIN)
[ -n "$commit" ] || fail "no commit line in conformance/test262/PIN"
checkout=$cache/$commit
[ -d "$checkout/test" ] || fail "no pinned checkout at $checkout"
[ -s "$moves" ] || fail "no $moves"
[ -s "$parts" ] || fail "no $parts"

cp "$moves" "$dir/moves.tsv" || fail "cannot copy $moves"
task -d . test262-moves-check TEST262_CACHE="$cache" TEST262_DESIGN="$design" TEST262_MOVES="$dir/moves.tsv" > "$dir/base.out" 2>&1 ||
  fail "task test262-moves-check on the unchanged copy exited non-zero ($dir/base.out)"
grep -q '^test262-moves: [0-9]* case(s) moved, ' "$dir/base.out" ||
  fail "task test262-moves-check on the unchanged copy printed no summary line ($dir/base.out)"

tab=$(printf '\t')
line=0
planted=""
while IFS="$tab" read -r path from to kind evidence; do
  line=$((line + 1))
  [ "$kind" = part ] || continue
  grep -q '^includes:' "$checkout/$path" && continue
  tok=$(awk -F'\t' -v to="$to" '$2 + 0 == to + 0 && index($1, " ") == 0 { print $1 }' "$parts" | while IFS= read -r t; do
    if ! grep -qF -e "$t" "$checkout/$path"; then printf '%s\n' "$t"; break; fi
  done)
  if [ -n "$tok" ]; then
    planted=$path
    break
  fi
done < "$moves"
[ -n "$planted" ] || fail "no part line of $moves whose case lacks a token of its slice"
awk -F'\t' -v OFS='\t' -v n="$line" -v tok="$tok" 'NR == n { $5 = tok } { print }' "$moves" > "$dir/planted-evidence.tsv"
cmp -s "$moves" "$dir/planted-evidence.tsv" && fail "the planted evidence copy equals $moves"
if task -d . test262-moves-check TEST262_CACHE="$cache" TEST262_DESIGN="$design" TEST262_MOVES="$dir/planted-evidence.tsv" > "$dir/planted-evidence.out" 2>&1; then
  fail "task test262-moves-check exited 0 with evidence \"$tok\" that $planted does not hold ($dir/planted-evidence.out)"
fi
grep -qF ": $planted: " "$dir/planted-evidence.out" ||
  fail "task test262-moves-check did not name $planted ($dir/planted-evidence.out)"

first=$(awk -F'\t' 'NR == 1 { print $1 }' "$moves")
awk -F'\t' -v OFS='\t' 'NR == 1 { $3 = 2 } { print }' "$moves" > "$dir/planted-to-2.tsv"
cmp -s "$moves" "$dir/planted-to-2.tsv" && fail "the planted slice 2 copy equals $moves"
if task -d . test262-moves-check TEST262_CACHE="$cache" TEST262_DESIGN="$design" TEST262_MOVES="$dir/planted-to-2.tsv" > "$dir/planted-to-2.out" 2>&1; then
  fail "task test262-moves-check exited 0 with a line that moves $first into slice 2 ($dir/planted-to-2.out)"
fi
grep -qF ": $first: " "$dir/planted-to-2.out" ||
  fail "task test262-moves-check did not name $first ($dir/planted-to-2.out)"

# A token that grep -F finds only inside another word and in a comment is no evidence: a planted
# checkout under $dir holds two cases, one with "let" only in "letter", "delete" and a comment,
# one with a let declaration; a copy of the design gives their directory slice 6.
fake=$dir/checkout
planted_dir=test/language/planted-moves-check
mkdir -p "$fake/$commit/$planted_dir" "$fake/$commit/harness" || fail "cannot make $fake"
word=$planted_dir/let-inside-a-word-and-a-comment.js
decl=$planted_dir/let-declaration.js
printf '/*---\ndescription: let appears inside words and in a comment only\n---*/\n// let x = 1;\nvar letter = 1;\ndelete letter;\n' > "$fake/$commit/$word"
printf '/*---\ndescription: a let declaration\n---*/\nlet x = 1;\n' > "$fake/$commit/$decl"
grep -qF -e let "$fake/$commit/$word" || fail "grep -F does not find let in the planted case"
awk -v line="dir $planted_dir/ 6" '{ print } $0 == "```swell-slices" { print line }' \
  "$design" > "$dir/engine.md" || fail "cannot copy the design"
printf '%s\t2\t6\tpart\tlet\n' "$decl" > "$dir/planted-decl.tsv"
task -d . test262-moves-check TEST262_CACHE="$fake" TEST262_DESIGN="$dir/engine.md" \
  TEST262_MOVES="$dir/planted-decl.tsv" > "$dir/planted-decl.out" 2>&1 ||
  fail "task test262-moves-check refused a let declaration as evidence for let ($dir/planted-decl.out)"
printf '%s\t2\t6\tpart\tlet\n' "$word" > "$dir/planted-word.tsv"
if task -d . test262-moves-check TEST262_CACHE="$fake" TEST262_DESIGN="$dir/engine.md" \
  TEST262_MOVES="$dir/planted-word.tsv" > "$dir/planted-word.out" 2>&1; then
  fail "task test262-moves-check took let inside a word or a comment as evidence ($dir/planted-word.out)"
fi
grep -qF ": $word: " "$dir/planted-word.out" ||
  fail "task test262-moves-check did not name $word ($dir/planted-word.out)"

echo "$name: ok"

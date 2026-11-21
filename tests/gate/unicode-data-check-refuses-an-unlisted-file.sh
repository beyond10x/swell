#!/bin/sh
# Planted defect: a copy of data/unicode/17.0.0 with one file added that SOURCE does not list.
# The task unicode-data-check must refuse it with exactly one fault line, the "no SOURCE line"
# line of that file. The unchanged copy must pass first, so the refusal comes from the defect.
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=unicode-data-check-refuses-an-unlisted-file
work=.out/gate-tests/$name
data=data/unicode/17.0.0
file=ucd/Unlisted.txt
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$work" && mkdir -p "$work" || fail "cannot make $work"
[ -f taskfiles/unicode-data.yml ] || fail "no task: taskfiles/unicode-data.yml does not exist"
[ -d "$data" ] || fail "no $data"
cp -R "$data" "$work/copy" || fail "cannot copy $data"

task -d . -t taskfiles/unicode-data.yml unicode-data-check UNICODE_DATA_DIR="$work/copy" >"$work/clean.log" 2>&1 \
  || fail "the unchanged copy is refused: $(cat "$work/clean.log")"

[ ! -e "$work/copy/$file" ] || fail "$file already exists in the copy"
printf 'not part of the pinned data\n' >"$work/copy/$file" || fail "cannot add $file"

if task -d . -t taskfiles/unicode-data.yml unicode-data-check UNICODE_DATA_DIR="$work/copy" >"$work/planted.log" 2>&1; then
  fail "the task exited 0 on a copy with $file added"
fi
faults=$(grep -v '^task: ' "$work/planted.log")
[ "$faults" = "$file: no SOURCE line" ] || fail "expected exactly '$file: no SOURCE line', got: $faults"
echo "$name: ok"

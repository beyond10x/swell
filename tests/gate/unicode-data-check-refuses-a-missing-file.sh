#!/bin/sh
# Planted defect: a copy of data/unicode/17.0.0 without ucd/Scripts.txt and without its
# SOURCE line. The task unicode-data-check must refuse it with exactly one fault line,
# "ucd/Scripts.txt: required, missing". The unchanged copy must pass first, so the refusal
# comes from the planted defect and from nothing else.
name=unicode-data-check-refuses-a-missing-file
work=.out/gate-tests/$name
data=data/unicode/17.0.0
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$work" && mkdir -p "$work" || fail "cannot make $work"
[ -f taskfiles/unicode-data.yml ] || fail "no task: taskfiles/unicode-data.yml does not exist"
[ -d "$data" ] || fail "no $data"
cp -R "$data" "$work/copy" || fail "cannot copy $data"

task -d . -t taskfiles/unicode-data.yml unicode-data-check UNICODE_DATA_DIR="$work/copy" >"$work/clean.log" 2>&1 \
  || fail "the unchanged copy is refused: $(cat "$work/clean.log")"

rm "$work/copy/ucd/Scripts.txt" || fail "cannot remove ucd/Scripts.txt"
grep -v '^[^ ]* ucd/Scripts\.txt ' "$work/copy/SOURCE" >"$work/SOURCE" || fail "cannot filter SOURCE"
mv "$work/SOURCE" "$work/copy/SOURCE" || fail "cannot replace SOURCE"

if task -d . -t taskfiles/unicode-data.yml unicode-data-check UNICODE_DATA_DIR="$work/copy" >"$work/planted.log" 2>&1; then
  fail "the task exited 0 on a copy without ucd/Scripts.txt"
fi
faults=$(grep -v '^task: ' "$work/planted.log")
[ "$faults" = "ucd/Scripts.txt: required, missing" ] \
  || fail "expected exactly 'ucd/Scripts.txt: required, missing', got: $faults"
echo "$name: ok"

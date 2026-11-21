#!/bin/sh
# Planted defect: a copy of data/unicode/17.0.0 with one byte of ucd/CaseFolding.txt changed.
# The task unicode-data-check must refuse it with exactly one fault line, the sha256 line of
# that file. The unchanged copy must pass first, so the refusal comes from the planted defect.
name=unicode-data-check-refuses-a-changed-file
work=.out/gate-tests/$name
data=data/unicode/17.0.0
file=ucd/CaseFolding.txt
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$work" && mkdir -p "$work" || fail "cannot make $work"
[ -f taskfiles/unicode-data.yml ] || fail "no task: taskfiles/unicode-data.yml does not exist"
[ -d "$data" ] || fail "no $data"
cp -R "$data" "$work/copy" || fail "cannot copy $data"

task -d . -t taskfiles/unicode-data.yml unicode-data-check UNICODE_DATA_DIR="$work/copy" >"$work/clean.log" 2>&1 \
  || fail "the unchanged copy is refused: $(cat "$work/clean.log")"

first=$(head -c 1 "$work/copy/$file")
byte=X
[ "$first" != X ] || byte=Y
printf '%s' "$byte" | dd of="$work/copy/$file" bs=1 count=1 conv=notrunc 2>/dev/null || fail "cannot change $file"
cmp -s "$data/$file" "$work/copy/$file" && fail "$file is unchanged in the copy"
[ "$(wc -c <"$data/$file")" = "$(wc -c <"$work/copy/$file")" ] || fail "$file changed length"

actual=$(sha256sum "$work/copy/$file" | cut -d ' ' -f 1)
expected=$(awk -v p="$file" '$2 == p { print $1 }' "$work/copy/SOURCE")
[ -n "$expected" ] || fail "no SOURCE line for $file"

if task -d . -t taskfiles/unicode-data.yml unicode-data-check UNICODE_DATA_DIR="$work/copy" >"$work/planted.log" 2>&1; then
  fail "the task exited 0 on a copy with $file changed"
fi
faults=$(grep -v '^task: ' "$work/planted.log")
want="$file: sha256 $actual, SOURCE gives $expected"
[ "$faults" = "$want" ] || fail "expected exactly '$want', got: $faults"
echo "$name: ok"

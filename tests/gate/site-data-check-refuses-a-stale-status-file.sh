#!/bin/sh
# Planted defect of taskfiles/site.yml, task site-data-check: the site's numbers
# (website/data/status.json) no longer match the committed gate files they are generated from.
# `task site-data-check` regenerates the file from those inputs and must exit non-zero, naming
# the first line that differs, on each of: a copy of status.json with one count changed by hand
# (a number typed into the site), a copy of conformance/test262/expected.txt whose total line
# passes one case more (an input moved and the site was not regenerated), and a copy of
# status.json with its last line removed. The unchanged copy passes first, and the task needs no
# Node and no network.
# Run from the repository root: sh tests/gate/site-data-check-refuses-a-stale-status-file.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=site-data-check-refuses-a-stale-status-file
dir=.out/gate-tests/$name
status=website/data/status.json
expected=conformance/test262/expected.txt
unset SITE_DATA_FILE SITE_DATA_OUT SITE_DATA_EXPECTED SITE_DATA_DELIVERED SITE_DATA_REQUIREMENTS \
  SITE_DATA_MOVES SITE_DATA_DESIGN SITE_DATA_MANIFEST SITE_DATA_LOCK SITE_DATA_CRATES
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
[ -s "$status" ] || fail "no $status to copy"
[ -s "$expected" ] || fail "no $expected to copy"

# The task file is named, so the script runs whether or not the root Taskfile.yml includes it.
check() {
  task -d . -t taskfiles/site.yml site-data-check SITE_DATA_OUT="$dir/out" "$@"
}

cp "$status" "$dir/status.json" || fail "cannot copy $status"
check SITE_DATA_FILE="$dir/status.json" > "$dir/unplanted.out" 2>&1 ||
  fail "site-data-check refused the unchanged copy of $status ($dir/unplanted.out)"
grep -q '^site-data-check: .* equals its regeneration from ' "$dir/unplanted.out" ||
  fail "site-data-check printed no summary line for the unchanged copy ($dir/unplanted.out)"

# A number typed by hand: the first "passed" count, one higher.
line=$(grep -n '"passed": [0-9]' "$dir/status.json" | head -n 1 | cut -d: -f1)
[ -n "$line" ] || fail "no \"passed\" count in $status"
awk -v n="$line" 'NR == n { match($0, /[0-9]+/); $0 = substr($0, 1, RSTART - 1) (substr($0, RSTART, RLENGTH) + 1) substr($0, RSTART + RLENGTH) } { print }' \
  "$dir/status.json" > "$dir/typed.json" || fail "cannot plant the typed number"
cmp -s "$dir/status.json" "$dir/typed.json" && fail "the planted copy equals $status"
if check SITE_DATA_FILE="$dir/typed.json" > "$dir/typed.out" 2>&1; then
  fail "site-data-check accepted a count changed by hand at line $line ($dir/typed.out)"
fi
grep -q "at line $line:" "$dir/typed.out" ||
  fail "site-data-check did not name line $line, the planted one ($dir/typed.out)"

# An input that moved: the total line of the expected counts passes one more case.
awk '$1 == "test262" && $2 == "total" { $7 = $7 + 1 } { print }' "$expected" > "$dir/expected.txt" ||
  fail "cannot plant the moved input"
cmp -s "$expected" "$dir/expected.txt" && fail "the planted expected counts equal $expected"
if check SITE_DATA_FILE="$status" SITE_DATA_EXPECTED="$dir/expected.txt" > "$dir/moved.out" 2>&1; then
  fail "site-data-check accepted $status against expected counts that moved ($dir/moved.out)"
fi
grep -q 'differs from its regeneration at line [0-9]*:' "$dir/moved.out" ||
  fail "site-data-check did not name the differing line for the moved input ($dir/moved.out)"

# A file cut short: the last line removed.
sed '$d' "$status" > "$dir/short.json" || fail "cannot plant the short copy"
if check SITE_DATA_FILE="$dir/short.json" > "$dir/short.out" 2>&1; then
  fail "site-data-check accepted a copy of $status without its last line ($dir/short.out)"
fi
grep -q 'differs from its regeneration at line [0-9]*:' "$dir/short.out" ||
  fail "site-data-check did not name the differing line for the short copy ($dir/short.out)"

echo "$name: ok"

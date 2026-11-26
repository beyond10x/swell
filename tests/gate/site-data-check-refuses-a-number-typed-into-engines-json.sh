#!/bin/sh
# Planted defect of taskfiles/site.yml, task site-data-check: the compare page's data
# (website/data/engines.json) holds a number nobody generated. `task site-data-check`
# regenerates the file from the catalog, the published tables and swell's committed files and
# must exit non-zero, naming the planted line, on a copy of engines.json whose first "value"
# count is one higher, and on a copy without its last line. The unchanged copy passes first.
# Run from the repository root: sh tests/gate/site-data-check-refuses-a-number-typed-into-engines-json.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=site-data-check-refuses-a-number-typed-into-engines-json
dir=.out/gate-tests/$name
engines=website/data/engines.json
unset SITE_DATA_FILE SITE_DATA_OUT SITE_DATA_EXPECTED SITE_DATA_DELIVERED SITE_DATA_REQUIREMENTS \
  SITE_DATA_MOVES SITE_DATA_DESIGN SITE_DATA_MANIFEST SITE_DATA_LOCK SITE_DATA_CRATES \
  SITE_DATA_ENGINES SITE_DATA_CATALOG SITE_DATA_PUBLISHED SITE_DATA_RESULTS SITE_DATA_PIN \
  SITE_DATA_LIMITS SITE_DATA_INTERRUPT
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
[ -s "$engines" ] || fail "no $engines to copy"

check() {
  task -d . -t taskfiles/site.yml site-data-check SITE_DATA_OUT="$dir/out" "$@"
}

cp "$engines" "$dir/engines.json" || fail "cannot copy $engines"
check SITE_DATA_ENGINES="$dir/engines.json" > "$dir/unplanted.out" 2>&1 ||
  fail "site-data-check refused the unchanged copy of $engines ($dir/unplanted.out)"
grep -q "^site-data-check: $dir/engines.json equals its regeneration from " "$dir/unplanted.out" ||
  fail "site-data-check printed no summary line for the unchanged copy ($dir/unplanted.out)"

# A number typed by hand: the first "value" count, one higher.
line=$(grep -n '"value": [0-9]' "$dir/engines.json" | head -n 1 | cut -d: -f1)
[ -n "$line" ] || fail "no \"value\" count in $engines"
awk -v n="$line" 'NR == n { match($0, /[0-9]+/); $0 = substr($0, 1, RSTART - 1) (substr($0, RSTART, RLENGTH) + 1) substr($0, RSTART + RLENGTH) } { print }' \
  "$dir/engines.json" > "$dir/typed.json" || fail "cannot plant the typed number"
cmp -s "$dir/engines.json" "$dir/typed.json" && fail "the planted copy equals $engines"
if check SITE_DATA_ENGINES="$dir/typed.json" > "$dir/typed.out" 2>&1; then
  fail "site-data-check accepted a count changed by hand at line $line ($dir/typed.out)"
fi
grep -q "^site-data-check: $dir/typed.json differs from its regeneration at line $line:" "$dir/typed.out" ||
  fail "site-data-check did not name line $line, the planted one ($dir/typed.out)"

# A file cut short: the last line removed.
sed '$d' "$engines" > "$dir/short.json" || fail "cannot plant the short copy"
if check SITE_DATA_ENGINES="$dir/short.json" > "$dir/short.out" 2>&1; then
  fail "site-data-check accepted a copy of $engines without its last line ($dir/short.out)"
fi
grep -q 'differs from its regeneration at line [0-9]*:' "$dir/short.out" ||
  fail "site-data-check did not name the differing line for the short copy ($dir/short.out)"

echo "$name: ok"

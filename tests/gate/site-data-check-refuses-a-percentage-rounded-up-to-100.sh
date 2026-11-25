#!/bin/sh
# Planted defect of taskfiles/site.yml, task site-data: a percentage of passed cases rounded up to
# 100.0 while a case fails. Slice 3 was delivered with 7,010 of its 7,011 cases (design, section 12,
# "Slice 3's cases that need a later slice"); "%.1f" makes that 100.0, and the site would say that
# every case passes. The task floors to one decimal. From a copy of
# conformance/test262/expected.txt whose total line and slice 01 line pass 7,010 of 7,011 cases
# and whose slice 02 line passes 31 of 31, site-data must write "99.9" for the total and slice 01,
# never "100.0", and "100.0" for slice 02. It writes no tracked file: status.json, engines.json and
# the work directory of site-data all go under its own directory.
# Run from the repository root: sh tests/gate/site-data-check-refuses-a-percentage-rounded-up-to-100.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=site-data-check-refuses-a-percentage-rounded-up-to-100
dir=.out/gate-tests/$name
expected=conformance/test262/expected.txt
unset SITE_DATA_FILE SITE_DATA_OUT SITE_DATA_EXPECTED SITE_DATA_DELIVERED SITE_DATA_REQUIREMENTS \
  SITE_DATA_MOVES SITE_DATA_DESIGN SITE_DATA_MANIFEST SITE_DATA_LOCK SITE_DATA_CRATES \
  SITE_DATA_ENGINES SITE_DATA_CATALOG SITE_DATA_PUBLISHED SITE_DATA_RESULTS SITE_DATA_PIN \
  SITE_DATA_LIMITS SITE_DATA_INTERRUPT
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
[ -s "$expected" ] || fail "no $expected to copy"

# Fields: exist excluded not_applicable run passed failed budget (total);
# cases runs excluded not_applicable run passed failed budget (slice).
awk '
  $1 == "test262" && $2 == "total" { $0 = "test262 total 7011 0 0 7011 7010 1 0" }
  $1 == "test262-slice" && $2 == "01" { $0 = "test262-slice 01 7011 7011 0 0 7011 7010 1 0" }
  $1 == "test262-slice" && $2 == "02" { $0 = "test262-slice 02 31 31 0 0 31 31 0 0" }
  { print }
' "$expected" > "$dir/expected.txt" || fail "cannot plant the counts"
grep -q '^test262-slice 01 7011 ' "$dir/expected.txt" || fail "no slice 01 line in $expected to plant"
grep -q '^test262-slice 02 31 ' "$dir/expected.txt" || fail "no slice 02 line in $expected to plant"

# Every file site-data writes goes under $dir: the status file, the compare page's data and the
# work directory. Left at its default, SITE_DATA_ENGINES is the tracked website/data/engines.json,
# which this case would overwrite with the planted counts.
task -d . -t taskfiles/site.yml site-data SITE_DATA_OUT="$dir/out" SITE_DATA_FILE="$dir/status.json" \
  SITE_DATA_ENGINES="$dir/engines.json" SITE_DATA_EXPECTED="$dir/expected.txt" \
  > "$dir/site-data.out" 2>&1 || fail "site-data refused the planted counts ($dir/site-data.out)"

# The passed_percent of the test262 total, then of each slice by its id.
percent() {
  awk -v want="$1" '
    want == "total" && /^  "test262": \{/ { inside = 1 }
    want != "total" && $0 ~ "^      \"slice\": \"" want "\"," { inside = 1 }
    inside && /"passed_percent": / { v = $0; sub(/^.*"passed_percent": "/, "", v); sub(/".*$/, "", v); print v; exit }
  ' "$dir/status.json"
}
total=$(percent total)
one=$(percent 01)
two=$(percent 02)
[ "$total" = 99.9 ] || fail "7,010 of 7,011 in the total reads \"$total\", not \"99.9\" ($dir/status.json)"
[ "$one" = 99.9 ] || fail "7,010 of 7,011 in slice 01 reads \"$one\", not \"99.9\" ($dir/status.json)"
[ "$two" = 100.0 ] || fail "31 of 31 in slice 02 reads \"$two\", not \"100.0\" ($dir/status.json)"

echo "$name: ok"

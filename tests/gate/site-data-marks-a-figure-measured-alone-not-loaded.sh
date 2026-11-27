#!/bin/sh
# Planted input of taskfiles/site.yml, task site-data: a result file of format swell-bench/1
# with a measured row marked loaded: false (no other pool job on the build node during its job,
# the load under the bound). site-data must write that figure's cell of engines.json with
# loaded: false and without loaded_reason, so the page says the figure was measured alone.
# Run from the repository root: sh tests/gate/site-data-marks-a-figure-measured-alone-not-loaded.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=site-data-marks-a-figure-measured-alone-not-loaded
dir=.out/gate-tests/$name
unset SITE_DATA_FILE SITE_DATA_OUT SITE_DATA_EXPECTED SITE_DATA_DELIVERED SITE_DATA_REQUIREMENTS \
  SITE_DATA_MOVES SITE_DATA_DESIGN SITE_DATA_MANIFEST SITE_DATA_LOCK SITE_DATA_CRATES \
  SITE_DATA_ENGINES SITE_DATA_CATALOG SITE_DATA_PUBLISHED SITE_DATA_RESULTS SITE_DATA_PIN \
  SITE_DATA_LIMITS SITE_DATA_INTERRUPT
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/results" || fail "cannot make $dir"

alone='{"engine":"quickjs","version":"1","mode":"interpreter","flags":[],"kpi":"K3","measurement":"empty-script","unit":"µs","runs":3,"values":[900,1000,1100],"median":1000,"status":"measured","reason":"","loaded":false}'
printf '{"format":"swell-bench/1","date":"2026-11-25","commit":"0123456789abcdef","host":{"arch":"x86_64","cpus":4,"runner":"build-node"},"rows":[%s]}\n' \
  "$alone" > "$dir/results/run.json" || fail "cannot plant the result file"

task -d . -t taskfiles/site.yml site-data SITE_DATA_OUT="$dir/out" SITE_DATA_FILE="$dir/status.json" \
  SITE_DATA_ENGINES="$dir/engines.json" SITE_DATA_RESULTS="$dir/results" > "$dir/site-data.out" 2>&1 ||
  fail "site-data refused the planted result file ($dir/site-data.out)"

cell=$(jq -c '.engines[] | select(.id == "quickjs") | .values["K3.empty-script"]' "$dir/engines.json")
echo "$cell" | jq -e '.status == "measured" and .value == 1000 and .loaded == false and (has("loaded_reason") | not)' >/dev/null ||
  fail "the figure of a row measured alone is not loaded: false without loaded_reason: $cell"

echo "$name: ok"

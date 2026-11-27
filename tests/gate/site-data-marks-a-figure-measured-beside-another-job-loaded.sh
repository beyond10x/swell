#!/bin/sh
# Planted input of taskfiles/site.yml, task site-data: a result file of format swell-bench/1
# with a measured row marked loaded: true, loaded_reason "neighbour" (another pool job ran on
# the build node during its job). site-data must write that figure's cell of engines.json with
# loaded: true and loaded_reason "neighbour", so the compare page can mark it; a figure measured
# beside another job must not read like one measured alone. A not-measured row of the same file
# keeps its reason as the row wrote it, byte for byte (the page prints it as text).
# Run from the repository root: sh tests/gate/site-data-marks-a-figure-measured-beside-another-job-loaded.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=site-data-marks-a-figure-measured-beside-another-job-loaded
dir=.out/gate-tests/$name
unset SITE_DATA_FILE SITE_DATA_OUT SITE_DATA_EXPECTED SITE_DATA_DELIVERED SITE_DATA_REQUIREMENTS \
  SITE_DATA_MOVES SITE_DATA_DESIGN SITE_DATA_MANIFEST SITE_DATA_LOCK SITE_DATA_CRATES \
  SITE_DATA_ENGINES SITE_DATA_CATALOG SITE_DATA_PUBLISHED SITE_DATA_RESULTS SITE_DATA_PIN \
  SITE_DATA_LIMITS SITE_DATA_INTERRUPT
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/results" || fail "cannot make $dir"

loaded='{"engine":"quickjs","version":"1","mode":"interpreter","flags":[],"kpi":"K2","measurement":"bench-v8/Richards","unit":"ms","runs":3,"values":[39,40,41],"median":40,"status":"measured","reason":"","loaded":true,"loaded_reason":"neighbour"}'
failed='{"engine":"boa","version":"1","mode":"interpreter","flags":[],"kpi":"K2","measurement":"bench-v8/Richards","unit":"ms","runs":3,"values":[],"median":null,"status":"not-measured","reason":"failed: exit 1, signal -, timed out 0: <b>x</b> & \"y\"","loaded":true,"loaded_reason":"neighbour"}'
printf '{"format":"swell-bench/1","date":"2026-11-25","commit":"0123456789abcdef","host":{"arch":"x86_64","cpus":4,"runner":"build-node"},"rows":[%s,%s]}\n' \
  "$loaded" "$failed" > "$dir/results/run.json" || fail "cannot plant the result file"

task -d . -t taskfiles/site.yml site-data SITE_DATA_OUT="$dir/out" SITE_DATA_FILE="$dir/status.json" \
  SITE_DATA_ENGINES="$dir/engines.json" SITE_DATA_RESULTS="$dir/results" > "$dir/site-data.out" 2>&1 ||
  fail "site-data refused the planted result file ($dir/site-data.out)"

cell=$(jq -c '.engines[] | select(.id == "quickjs") | .values["K2.bench-v8/Richards"]' "$dir/engines.json")
echo "$cell" | jq -e '.status == "measured" and .value == 40 and .loaded == true and .loaded_reason == "neighbour"' >/dev/null ||
  fail "the figure of a row measured beside another job is not loaded: true with loaded_reason neighbour: $cell"
reason=$(jq -r '.engines[] | select(.id == "boa") | .values["K2.bench-v8/Richards"].reason' "$dir/engines.json")
[ "$reason" = 'failed: exit 1, signal -, timed out 0: <b>x</b> & "y"' ] ||
  fail "the reason of a not-measured row is not the row's own text: $reason"

echo "$name: ok"

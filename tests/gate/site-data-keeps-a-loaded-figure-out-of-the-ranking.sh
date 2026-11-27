#!/bin/sh
# Planted input of taskfiles/site.yml, task site-data: a result file of format swell-bench/1
# with one benchmark measured for two engines, QuickJS beside another pool job (loaded: true)
# and Boa alone (loaded: false), and a second benchmark measured alone for both. A figure
# measured beside another job does not rank: site-data must write ranks: false on the loaded
# figure and ranks: true on the figures measured alone. The compare page gives no figure of a
# comparison the mark "best of the picked" when one of its figures has ranks: false
# (bestOf in website/src/data/engines.ts).
# Run from the repository root: sh tests/gate/site-data-keeps-a-loaded-figure-out-of-the-ranking.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=site-data-keeps-a-loaded-figure-out-of-the-ranking
dir=.out/gate-tests/$name
unset SITE_DATA_FILE SITE_DATA_OUT SITE_DATA_EXPECTED SITE_DATA_DELIVERED SITE_DATA_REQUIREMENTS \
  SITE_DATA_MOVES SITE_DATA_DESIGN SITE_DATA_MANIFEST SITE_DATA_LOCK SITE_DATA_CRATES \
  SITE_DATA_ENGINES SITE_DATA_CATALOG SITE_DATA_PUBLISHED SITE_DATA_RESULTS SITE_DATA_PIN \
  SITE_DATA_LIMITS SITE_DATA_INTERRUPT
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/results" || fail "cannot make $dir"

row() {
  # engine benchmark median loaded-fields
  printf '{"engine":"%s","version":"1","mode":"interpreter","flags":[],"kpi":"K2","measurement":"bench-v8/%s","unit":"ms","runs":1,"values":[%s],"median":%s,"status":"measured","reason":"",%s}' \
    "$1" "$2" "$3" "$3" "$4"
}
rows="$(row quickjs Richards 40 '"loaded":true,"loaded_reason":"neighbour"'),$(row boa Richards 90 '"loaded":false')"
rows="$rows,$(row quickjs Splay 30 '"loaded":false'),$(row boa Splay 70 '"loaded":false')"
printf '{"format":"swell-bench/1","date":"2026-11-25","commit":"0123456789abcdef","host":{"arch":"x86_64","cpus":4,"runner":"build-node"},"rows":[%s]}\n' \
  "$rows" > "$dir/results/run.json" || fail "cannot plant the result file"

task -d . -t taskfiles/site.yml site-data SITE_DATA_OUT="$dir/out" SITE_DATA_FILE="$dir/status.json" \
  SITE_DATA_ENGINES="$dir/engines.json" SITE_DATA_RESULTS="$dir/results" > "$dir/site-data.out" 2>&1 ||
  fail "site-data refused the planted result file ($dir/site-data.out)"

cell() {
  jq -c --arg c "$1" --arg r "$2" '.engines[] | select(.id == $c) | .values[$r]' "$dir/engines.json"
}
hot=$(cell quickjs K2.bench-v8/Richards)
echo "$hot" | jq -e '.loaded == true and .ranks == false' >/dev/null ||
  fail "a figure measured beside another job is not ranks: false: $hot"
for id in boa:K2.bench-v8/Richards quickjs:K2.bench-v8/Splay boa:K2.bench-v8/Splay; do
  c=$(cell "${id%%:*}" "${id#*:}")
  echo "$c" | jq -e '.loaded == false and .ranks == true' >/dev/null ||
    fail "a figure measured alone ($id) is not ranks: true: $c"
done

echo "$name: ok"

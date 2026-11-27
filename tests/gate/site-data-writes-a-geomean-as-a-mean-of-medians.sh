#!/bin/sh
# Planted input of taskfiles/site.yml, task site-data: a result file of format swell-bench/1
# whose geomean row (aggregate "geometric mean of the benchmark medians", of Richards and
# DeltaBlue, values the two medians 40 and 90) is measured. A geomean is not a median of runs:
# site-data must write its cell without runs, min and max (the page would print them as "median
# of 3 runs, 40 to 90", a range of runs that does not exist), with an aggregate text and a source
# method taken from the row's own aggregate and of, and keep runs, min and max on the cell of a
# benchmark row.
# Run from the repository root: sh tests/gate/site-data-writes-a-geomean-as-a-mean-of-medians.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=site-data-writes-a-geomean-as-a-mean-of-medians
dir=.out/gate-tests/$name
unset SITE_DATA_FILE SITE_DATA_OUT SITE_DATA_EXPECTED SITE_DATA_DELIVERED SITE_DATA_REQUIREMENTS \
  SITE_DATA_MOVES SITE_DATA_DESIGN SITE_DATA_MANIFEST SITE_DATA_LOCK SITE_DATA_CRATES \
  SITE_DATA_ENGINES SITE_DATA_CATALOG SITE_DATA_PUBLISHED SITE_DATA_RESULTS SITE_DATA_PIN \
  SITE_DATA_LIMITS SITE_DATA_INTERRUPT
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/results" || fail "cannot make $dir"

row() {
  # benchmark median values extra-fields
  printf '{"engine":"boa","config":"boa","version":"1","mode":"interpreter","flags":[],"kpi":"K2","measurement":"are-we-fast-yet/%s","unit":"ms","runs":3,"values":[%s],"median":%s,"status":"measured","reason":"","suite":"are-we-fast-yet",%s}' \
    "$1" "$3" "$2" "$4"
}
richards=$(row Richards 40 '39,40,41' '"loaded":false')
deltablue=$(row DeltaBlue 90 '89,90,91' '"loaded":false')
geomean=$(row geomean 60 '40,90' '"aggregate":"geometric mean of the benchmark medians","of":["Richards","DeltaBlue"],"loaded":false')
printf '{"format":"swell-bench/1","date":"2026-11-25","commit":"0123456789abcdef","host":{"arch":"x86_64","cpus":4,"runner":"build-node"},"rows":[%s,%s,%s]}\n' \
  "$richards" "$deltablue" "$geomean" > "$dir/results/run.json" || fail "cannot plant the result file"

task -d . -t taskfiles/site.yml site-data SITE_DATA_OUT="$dir/out" SITE_DATA_FILE="$dir/status.json" \
  SITE_DATA_ENGINES="$dir/engines.json" SITE_DATA_RESULTS="$dir/results" > "$dir/site-data.out" 2>&1 ||
  fail "site-data refused the planted result file ($dir/site-data.out)"

text='geometric mean of the benchmark medians, 2 benchmarks, each the median of 3 runs: Richards, DeltaBlue'
geo=$(jq -c '.engines[] | select(.id == "boa") | .values["K2.are-we-fast-yet/geomean"]' "$dir/engines.json")
echo "$geo" | jq -e --arg t "$text" '.status == "measured" and .value == 60 and (has("runs") or has("min") or has("max") | not)
    and .aggregate == $t and .source.method == "are-we-fast-yet/geomean: " + $t' >/dev/null ||
  fail "the geomean cell is not a mean of medians without runs, min and max, with the text \"$text\": $geo"
one=$(jq -c '.engines[] | select(.id == "boa") | .values["K2.are-we-fast-yet/Richards"]' "$dir/engines.json")
echo "$one" | jq -e '.runs == 3 and .min == 39 and .max == 41 and (has("aggregate") | not)' >/dev/null ||
  fail "the benchmark cell beside it lost its runs, min and max: $one"

echo "$name: ok"

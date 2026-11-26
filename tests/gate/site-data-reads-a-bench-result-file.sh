#!/bin/sh
# Planted inputs of taskfiles/site.yml, task site-data: result files of format swell-bench/1
# under bench/results. With no result file the rows K2 to K5 read "not measured", "no run yet".
# From a planted result file with a measured swell row, a measured QuickJS row and a refused
# Boa row of one benchmark, site-data must write that benchmark as a row of K2 with swell's
# median, runs, min and max and the file as source, the reason of the refused row, and "not in
# a run yet" for an engine the file does not name. It must refuse a file of another format and
# a row whose engine names no configuration of bench/engines.jsonl.
# Run from the repository root: sh tests/gate/site-data-reads-a-bench-result-file.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=site-data-reads-a-bench-result-file
dir=.out/gate-tests/$name
unset SITE_DATA_FILE SITE_DATA_OUT SITE_DATA_EXPECTED SITE_DATA_DELIVERED SITE_DATA_REQUIREMENTS \
  SITE_DATA_MOVES SITE_DATA_DESIGN SITE_DATA_MANIFEST SITE_DATA_LOCK SITE_DATA_CRATES \
  SITE_DATA_ENGINES SITE_DATA_CATALOG SITE_DATA_PUBLISHED SITE_DATA_RESULTS SITE_DATA_PIN \
  SITE_DATA_LIMITS SITE_DATA_INTERRUPT
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/none" "$dir/one" "$dir/format" "$dir/stranger" || fail "cannot make $dir"

write() {
  # $1 the results directory to read, $2 the engines.json to write.
  task -d . -t taskfiles/site.yml site-data SITE_DATA_OUT="$dir/out" SITE_DATA_FILE="$dir/status.json" \
    SITE_DATA_ENGINES="$2" SITE_DATA_RESULTS="$1"
}
cell() {
  # $1 engines.json, $2 configuration id, $3 row id: the cell as one line of JSON.
  jq -c --arg c "$2" --arg r "$3" '.engines[] | select(.id == $c) | .values[$r]' "$1"
}

write "$dir/none" "$dir/none.json" > "$dir/none.out" 2>&1 || fail "site-data refused an empty results directory ($dir/none.out)"
[ "$(cell "$dir/none.json" swell K2)" = '{"status":"not-measured","reason":"no run yet"}' ] ||
  fail "with no result file swell's K2 is not \"not measured\", \"no run yet\" ($dir/none.json)"

row() {
  # engine mode kpi measurement status median values reason
  printf '{"engine":"%s","version":"1","mode":"%s","flags":[],"kpi":"%s","measurement":"%s","unit":"ms","runs":3,"values":[%s],"median":%s,"status":"%s","reason":"%s"}' \
    "$1" "$2" "$3" "$4" "$7" "$6" "$5" "$8"
}
result() {
  # $1 format, $2 rows
  printf '{"format":"%s","date":"2026-11-25","commit":"0123456789abcdef","host":{"arch":"x86_64","cpus":4,"runner":"build-node"},"rows":[%s]}\n' "$1" "$2"
}
rows="$(row swell interpreter K2 bench-v8-v7/Richards measured 120 '110,120,130' ''),$(row quickjs interpreter K2 bench-v8-v7/Richards measured 40 '39,40,41' ''),$(row boa interpreter K2 bench-v8-v7/Richards refused 0 '' 'SyntaxError: planted')"
result swell-bench/1 "$rows" > "$dir/one/run.json" || fail "cannot plant the result file"
write "$dir/one" "$dir/one.json" > "$dir/one.out" 2>&1 || fail "site-data refused the planted result file ($dir/one.out)"
jq -e '.rows | map(.id) | index("K2.bench-v8-v7/Richards") != null' "$dir/one.json" >/dev/null ||
  fail "no row K2.bench-v8-v7/Richards in $dir/one.json"
jq -e '.rows | map(.id) | index("K2") == null' "$dir/one.json" >/dev/null ||
  fail "the placeholder row K2 stayed beside a measured K2 row ($dir/one.json)"
swell=$(cell "$dir/one.json" swell K2.bench-v8-v7/Richards)
echo "$swell" | jq -e '.status == "measured" and .value == 120 and .runs == 3 and .min == 110 and .max == 130 and (.source.url | endswith("/one/run.json"))' >/dev/null ||
  fail "swell's measured cell is not median 120 of 3 runs, 110 to 130, from the file: $swell"
boa=$(cell "$dir/one.json" boa K2.bench-v8-v7/Richards)
[ "$boa" = '{"status":"not-measured","reason":"SyntaxError: planted"}' ] || fail "the refused row reads $boa"
v8=$(cell "$dir/one.json" v8-jitless K2.bench-v8-v7/Richards)
[ "$v8" = '{"status":"not-measured","reason":"not in a run yet"}' ] || fail "an engine the file does not name reads $v8"

result swell-bench/0 "$rows" > "$dir/format/run.json" || fail "cannot plant the other format"
if write "$dir/format" "$dir/format.json" > "$dir/format.out" 2>&1; then
  fail "site-data accepted a result file of format swell-bench/0 ($dir/format.out)"
fi
grep -q 'format is swell-bench/0, not swell-bench/1' "$dir/format.out" || fail "site-data did not name the format ($dir/format.out)"

result swell-bench/1 "$(row no-such-engine interpreter K3 empty measured 5 '5' '')" > "$dir/stranger/run.json" ||
  fail "cannot plant the unknown engine"
if write "$dir/stranger" "$dir/stranger.json" > "$dir/stranger.out" 2>&1; then
  fail "site-data accepted a row of an engine the catalog does not have ($dir/stranger.out)"
fi
grep -q 'engine no-such-engine, mode interpreter) names no configuration' "$dir/stranger.out" ||
  grep -q 'engine no-such-engine, mode interpreter names no configuration' "$dir/stranger.out" ||
  fail "site-data did not name the unknown engine ($dir/stranger.out)"

echo "$name: ok"

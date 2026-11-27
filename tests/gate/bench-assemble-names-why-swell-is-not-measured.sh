#!/bin/sh
# Planted job log of bench/run.sh: one job's rows (a measured K3 row of quickjs) between
# "== rows begin" and "== rows end", and its "== host" line. `sh bench/run.sh assemble` must
# write a result file that holds to swell-bench/1 (task bench-schema-check), carries the planted
# row, and adds swell's six rows as not-measured with the reason "not in this run; swell gets its
# configuration with story swell-in-the-benchmarks". No network, no engine run, no pool job.
# Run from the repository root: sh tests/gate/bench-assemble-names-why-swell-is-not-measured.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=bench-assemble-names-why-swell-is-not-measured
dir=.out/gate-tests/$name
unset BENCH_RESULTS BENCH_SCHEMA
fail() { echo "$name: FAIL $*"; exit 1; }
reason='not in this run; swell gets its configuration with story swell-in-the-benchmarks'

rm -rf "$dir" && mkdir -p "$dir/run/logs" "$dir/result" || fail "cannot make $dir"
{
  echo 'job output before the rows'
  echo '== host {"arch":"x86_64","cpus":4,"cpu_model":"planted","kernel":"planted"}'
  echo '== rows begin'
  echo '{"engine":"quickjs","config":"quickjs","version":"1","mode":"interpreter","flags":[],"kpi":"K3","measurement":"empty-script","unit":"µs","runs":3,"values":[900,1000,1100],"median":1000,"min":900,"max":1100,"status":"measured","reason":"","image_digest":"sha256:0000000000000000000000000000000000000000000000000000000000000000"}'
  echo '== rows end'
} > "$dir/run/logs/quickjs.start.log" || fail "cannot plant the job log"

sh bench/run.sh assemble "$dir/run" "$dir/result/run.json" > "$dir/assemble.out" 2>&1 ||
  fail "assemble refused the planted job log ($dir/assemble.out)"
task -d . -t taskfiles/bench.yml bench-schema-check BENCH_RESULTS="$dir/result" > "$dir/schema.out" 2>&1 ||
  fail "the assembled file does not hold to swell-bench/1 ($dir/schema.out)"
jq -e '[.rows[] | select(.engine == "quickjs" and .kpi == "K3" and .median == 1000)] | length == 1' \
  "$dir/result/run.json" > /dev/null || fail "the planted row is not in $dir/result/run.json"
jq -e --arg r "$reason" '[.rows[] | select(.engine == "swell")] | length == 6 and all(.status == "not-measured" and .reason == $r)' \
  "$dir/result/run.json" > /dev/null ||
  fail "swell's rows are not six not-measured rows with the reason \"$reason\": $(jq -c '[.rows[] | select(.engine == "swell") | .reason] | unique' "$dir/result/run.json")"

echo "$name: ok"

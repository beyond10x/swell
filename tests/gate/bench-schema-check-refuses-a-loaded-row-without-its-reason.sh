#!/bin/sh
# Planted defects of the field loaded_reason, task bench-schema-check: a row with loaded: false
# and a loaded_reason, a row with loaded: true and none, a loaded_reason that is not one of
# neighbour, load, unwatched, and a list of one reason. `task bench-schema-check` must pass on a
# planted valid result file (rows loaded: false without a reason, loaded: true with "neighbour",
# loaded: true with ["neighbour","load"], and a row without loaded) and exit non-zero on each
# defect, naming the row. No network, no engine run.
# Run from the repository root: sh tests/gate/bench-schema-check-refuses-a-loaded-row-without-its-reason.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=bench-schema-check-refuses-a-loaded-row-without-its-reason
dir=.out/gate-tests/$name
unset BENCH_RESULTS BENCH_SCHEMA
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/valid" || fail "cannot make $dir"

check() {
  task -d . -t taskfiles/bench.yml bench-schema-check "$@"
}

hex40=0123456789abcdef0123456789abcdef01234567
row='"engine":"quickjs","version":"1","mode":"interpreter","flags":[],"kpi":"K2","measurement":"bench-v8/Richards","unit":"ms","runs":3,"values":[39,40,41],"median":40,"min":39,"max":41,"status":"measured","reason":""'
printf '%s\n' '{"format":"swell-bench/1","date":"2026-11-27","commit":"'"$hex40"'",' \
  '"host":{"arch":"x86_64","cpus":4,"runner":"planted"},"rows":[' \
  "{$row,\"loaded\":false}," \
  "{$row,\"loaded\":true,\"loaded_reason\":\"neighbour\"}," \
  "{$row,\"loaded\":true,\"loaded_reason\":[\"neighbour\",\"load\"]}," \
  "{$row}" \
  ']}' > "$dir/valid/result.json" || fail "cannot plant the valid result file"
check BENCH_RESULTS="$dir/valid" > "$dir/valid.out" 2>&1 ||
  fail "bench-schema-check refused the planted valid result file ($dir/valid.out): $(head -n 1 "$dir/valid.out")"

# plant NAME JQ EXPECTED: the valid file changed by JQ must be refused with a line matching EXPECTED.
plant() {
  mkdir -p "$dir/$1" && jq "$2" "$dir/valid/result.json" > "$dir/$1/result.json" ||
    fail "cannot plant $1"
  if check BENCH_RESULTS="$dir/$1" > "$dir/$1.out" 2>&1; then
    fail "bench-schema-check accepted $1 ($dir/$1.out)"
  fi
  grep -q "$3" "$dir/$1.out" || fail "bench-schema-check did not name the row of $1 ($dir/$1.out): $(head -n 1 "$dir/$1.out")"
}
plant false-with-reason '.rows[0].loaded_reason = "load"' 'result.json: \$\.rows\[0\]\.loaded: is false, wanted true$'
plant true-without-reason 'del(.rows[1].loaded_reason)' 'result.json: \$\.rows\[1\]: missing loaded_reason$'
plant unknown-reason '.rows[1].loaded_reason = "busy"' 'result.json: \$\.rows\[1\]\.loaded_reason: is "busy", not one of '
plant list-of-one '.rows[2].loaded_reason = ["load"]' 'result.json: \$\.rows\[2\]\.loaded_reason: has 1 item(s), fewer than 2$'

echo "$name: ok"

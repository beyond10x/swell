#!/bin/sh
# Planted node record of `sh bench/run.sh all` (tests/gate/fixtures/bench-node.sh: a stub of the
# pool's client and BENCH_NODE_STATE a command that prints a planted `slot state`). One
# configuration, three jobs: bench-v8 has another pool job beside it on the node; start has
# one beside it and a load over the bound; are-we-fast-yet is alone. `all` must write
# logs/<config>.<suite>.neighbours.json beside each job's log, with the job's id, start and end
# and each neighbour's id, repository, start, end and command (a path under /home/<name>/ cut
# to ~/), and the result file must mark every row of bench-v8 loaded: true with loaded_reason
# "neighbour", every row of start loaded: true with ["neighbour","load"], and every row of
# are-we-fast-yet loaded: false without loaded_reason; its metadata .loaded counts 3 jobs, 2
# with a neighbour, 4 rows loaded; and it holds to swell-bench/1 (task bench-schema-check).
# No engine, no pool job, no network.
# Run from the repository root: sh tests/gate/bench-all-marks-every-row-of-a-job-with-a-neighbour-loaded.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=bench-all-marks-every-row-of-a-job-with-a-neighbour-loaded
dir=.out/gate-tests/$name
unset BENCH_RESULTS BENCH_SCHEMA BENCH_NODE_STATE BENCH_ALLOW_UNWATCHED BENCH_IDLE_WAIT_S BENCH_IDLE_POLL_S
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
. tests/gate/fixtures/bench-node.sh
bench_node_plant "$dir"

(cd "$dir/tree" && BENCH_NODE_STATE=$node_state BENCH_PLANT_NEIGHBOUR="bench-v8 start" \
  BENCH_PLANT_LOAD=start PATH="$plant/bin:$PATH" sh bench/run.sh all) > "$dir/all.out" 2>&1 ||
  fail "all failed on the planted node ($dir/all.out): $(tail -n 1 "$dir/all.out")"
logs=$(ls -d "$dir"/tree/.out/bench/run-*/logs 2>/dev/null | head -n 1)
[ -n "$logs" ] || fail "all wrote no logs directory ($dir/all.out)"

nb=$logs/planted.bench-v8.neighbours.json
[ -f "$nb" ] || fail "no neighbours file beside the job's log: $nb"
jq -e '.job.id == 1000001 and .job.started_ms == 1000010 and .job.ended_ms == 1000020
  and (.neighbours | length) == 1
  and .neighbours[0].id == 1000002 and .neighbours[0].repo == "planted-neighbour"
  and .neighbours[0].started_ms == 1000015 and .neighbours[0].ended_ms == 1000030
  and (.neighbours[0].command | endswith("/planted/Cargo.toml"))' "$nb" > /dev/null ||
  fail "the neighbours file does not name the job and its one neighbour: $(jq -c . "$nb")"
case $HOME in
  /home/?*/*|/home/?*)
    jq -e '.neighbours[0].command == "cargo test --manifest-path ~/planted/Cargo.toml"' "$nb" > /dev/null ||
      fail "the neighbour's command keeps the home directory: $(jq -c '.neighbours[0].command' "$nb")" ;;
esac
grep -q '/home/' "$nb" && fail "the neighbours file names a path under /home/: $nb"
jq -e '(.neighbours | length) == 0' "$logs/planted.are-we-fast-yet.neighbours.json" > /dev/null 2>&1 ||
  fail "the job alone has no neighbours file that lists no neighbour: $logs/planted.are-we-fast-yet.neighbours.json"

result=$(ls "$dir"/tree/bench/results/*.json 2>/dev/null | head -n 1)
[ -n "$result" ] || fail "all wrote no result file ($dir/all.out)"
jq -e '[.rows[] | select(.engine == "planted" and (.measurement | startswith("bench-v8/")))]
  | length == 2 and all(.loaded == true and .loaded_reason == "neighbour")' "$result" > /dev/null ||
  fail "the rows of the job with a neighbour are not loaded with the reason neighbour: $(jq -c '[.rows[] | select(.engine == "planted" and (.measurement | startswith("bench-v8/"))) | [.loaded, .loaded_reason]]' "$result")"
jq -e '[.rows[] | select(.engine == "planted" and (.measurement | startswith("start/")))]
  | length == 2 and all(.loaded == true and .loaded_reason == ["neighbour", "load"])' "$result" > /dev/null ||
  fail "the rows of the job with a neighbour and a load are not loaded with both reasons: $(jq -c '[.rows[] | select(.engine == "planted" and (.measurement | startswith("start/"))) | [.loaded, .loaded_reason]]' "$result")"
jq -e '[.rows[] | select(.engine == "planted" and (.measurement | startswith("are-we-fast-yet/")))]
  | length == 2 and all(.loaded == false and (has("loaded_reason") | not))' "$result" > /dev/null ||
  fail "the rows of the job alone are not loaded: false without a reason: $(jq -c '[.rows[] | select(.engine == "planted" and (.measurement | startswith("are-we-fast-yet/"))) | [.loaded, .loaded_reason]]' "$result")"
jq -e '.loaded.jobs == 3 and .loaded.jobs_with_neighbour == 2 and .loaded.rows_loaded == 4
  and (.loaded.rule | type) == "string" and (.loaded.rule | length) > 0' "$result" > /dev/null ||
  fail "the result's metadata does not count 3 jobs, 2 with a neighbour, 4 rows loaded: $(jq -c '.loaded' "$result")"

mkdir -p "$dir/results" && cp "$result" "$dir/results/" || fail "cannot copy the result file"
task -d . -t taskfiles/bench.yml bench-schema-check BENCH_RESULTS="$dir/results" > "$dir/schema.out" 2>&1 ||
  fail "the result file does not hold to swell-bench/1 ($dir/schema.out): $(head -n 1 "$dir/schema.out")"

echo "$name: ok"

#!/bin/sh
# Planted run of `sh bench/run.sh all` without BENCH_NODE_STATE (tests/gate/fixtures/bench-node.sh:
# a stub of the pool's client that records each call). `all` must refuse to start, exit
# non-zero, name BENCH_NODE_STATE in its last line, and start no pool job (not even prepare).
# With BENCH_ALLOW_UNWATCHED=1 it must run, and every row of its three jobs must be loaded: true
# with loaded_reason "unwatched"; the result file holds to swell-bench/1. No engine, no pool
# job, no network.
# Run from the repository root: sh tests/gate/bench-all-refuses-a-run-that-cannot-watch-the-node.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=bench-all-refuses-a-run-that-cannot-watch-the-node
dir=.out/gate-tests/$name
unset BENCH_RESULTS BENCH_SCHEMA BENCH_NODE_STATE BENCH_ALLOW_UNWATCHED BENCH_IDLE_WAIT_S BENCH_IDLE_POLL_S
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
. tests/gate/fixtures/bench-node.sh
bench_node_plant "$dir"

if (cd "$dir/tree" && PATH="$plant/bin:$PATH" sh bench/run.sh all) > "$dir/refused.out" 2>&1; then
  fail "all started without BENCH_NODE_STATE ($dir/refused.out)"
fi
tail -n 1 "$dir/refused.out" | grep -q 'BENCH_NODE_STATE' ||
  fail "all did not name BENCH_NODE_STATE when it refused ($dir/refused.out): $(tail -n 1 "$dir/refused.out")"
[ ! -d "$dir/tree/.out/bench" ] || fail "all started work before it refused: $dir/tree/.out/bench exists"

(cd "$dir/tree" && BENCH_ALLOW_UNWATCHED=1 PATH="$plant/bin:$PATH" sh bench/run.sh all) \
  > "$dir/unwatched.out" 2>&1 ||
  fail "all with BENCH_ALLOW_UNWATCHED=1 failed ($dir/unwatched.out): $(tail -n 1 "$dir/unwatched.out")"
result=$(ls "$dir"/tree/bench/results/*.json 2>/dev/null | head -n 1)
[ -n "$result" ] || fail "all with BENCH_ALLOW_UNWATCHED=1 wrote no result file ($dir/unwatched.out)"
jq -e '[.rows[] | select(.engine == "planted")]
  | length == 6 and all(.loaded == true and .loaded_reason == "unwatched")' "$result" > /dev/null ||
  fail "the rows of an unwatched run are not all loaded with the reason unwatched: $(jq -c '[.rows[] | select(.engine == "planted") | [.loaded, .loaded_reason]]' "$result")"
jq -e '.loaded.jobs == 3 and .loaded.rows_loaded == 6' "$result" > /dev/null ||
  fail "the result's metadata does not count 3 jobs and 6 rows loaded: $(jq -c '.loaded' "$result")"

mkdir -p "$dir/results" && cp "$result" "$dir/results/" || fail "cannot copy the result file"
task -d . -t taskfiles/bench.yml bench-schema-check BENCH_RESULTS="$dir/results" > "$dir/schema.out" 2>&1 ||
  fail "the result file does not hold to swell-bench/1 ($dir/schema.out): $(head -n 1 "$dir/schema.out")"

echo "$name: ok"

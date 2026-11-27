#!/bin/sh
# Planted node that never gets idle (tests/gate/fixtures/bench-node.sh, with one active job of
# another repository that does not end) and a wait bound of 1 s (BENCH_IDLE_WAIT_S=1; the poll
# stays at its default of 10 s). `sh bench/run.sh all` must, before each of its three jobs, read
# the node's record more than once, say "the node is not idle after 1 s; starting job <config>
# <suite> beside it", start the job all the same, and mark every row of each job loaded: true
# with loaded_reason "neighbour" (the active job overlaps each). The run ends within 10 s: a
# poll never sleeps past the bound. No engine, no pool job, no network.
# Run from the repository root: sh tests/gate/bench-all-starts-a-job-after-the-wait-when-the-node-stays-busy.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=bench-all-starts-a-job-after-the-wait-when-the-node-stays-busy
dir=.out/gate-tests/$name
unset BENCH_RESULTS BENCH_SCHEMA BENCH_NODE_STATE BENCH_ALLOW_UNWATCHED BENCH_IDLE_WAIT_S BENCH_IDLE_POLL_S
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
. tests/gate/fixtures/bench-node.sh
bench_node_plant "$dir"
printf '%s\n' '{"now_ms":1000000,"active":[{"id":999,"started_ms":999000,"ended_ms":0,"repo":"planted-busy","argv":["cargo","build"]}],"done":[]}' \
  > "$dir/state.json" || fail "cannot plant the busy node"

t0=$(date +%s)
(cd "$dir/tree" && BENCH_NODE_STATE=$node_state BENCH_IDLE_WAIT_S=1 PATH="$plant/bin:$PATH" \
  sh bench/run.sh all) > "$dir/all.out" 2>&1 ||
  fail "all failed on the busy node ($dir/all.out): $(tail -n 1 "$dir/all.out")"
took=$(( $(date +%s) - t0 ))
[ "$took" -lt 10 ] || fail "all took $took s on the busy node with a bound of 1 s ($dir/all.out)"

for suite in bench-v8 are-we-fast-yet start; do
  grep -q "the node is not idle after 1 s; starting job planted $suite beside it" "$dir/all.out" ||
    fail "all did not say that it starts planted $suite after the wait ($dir/all.out)"
done
reads=$(wc -l < "$dir/reads")
[ "$reads" -ge 9 ] ||
  fail "all read the node's record $reads time(s); a wait of three jobs past the bound reads it at least 9 times"
logs=$(ls -d "$dir"/tree/.out/bench/run-*/logs 2>/dev/null | head -n 1)
for suite in bench-v8 are-we-fast-yet start; do
  grep -q '^== rows end$' "$logs/planted.$suite.log" 2>/dev/null ||
    fail "the job planted $suite did not run after the wait ($logs)"
done

result=$(ls "$dir"/tree/bench/results/*.json 2>/dev/null | head -n 1)
[ -n "$result" ] || fail "all wrote no result file ($dir/all.out)"
jq -e '[.rows[] | select(.engine == "planted")]
  | length == 6 and all(.loaded == true and .loaded_reason == "neighbour")' "$result" > /dev/null ||
  fail "the rows measured beside the busy job are not loaded with the reason neighbour: $(jq -c '[.rows[] | select(.engine == "planted") | [.loaded, .loaded_reason]]' "$result")"

echo "$name: ok"

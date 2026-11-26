#!/bin/sh
# Planted defect of taskfiles/bench.yml, task bench-schema-check: a result file of bench/results/
# whose first row has no version, one whose first row has no runs, and a directory without a
# result file. `task bench-schema-check` must pass on copies of the committed result files and
# exit non-zero on each planted case, naming the missing field. No network, no engine run.
# Run from the repository root: sh tests/gate/bench-schema-check-refuses-a-row-without-version-or-runs.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=bench-schema-check-refuses-a-row-without-version-or-runs
dir=.out/gate-tests/$name
unset BENCH_RESULTS BENCH_SCHEMA
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/committed" "$dir/no-version" "$dir/no-runs" "$dir/empty" ||
  fail "cannot make $dir"
first=$(find bench/results -maxdepth 1 -type f -name '*.json' | LC_ALL=C sort | head -n 1)
[ -n "$first" ] || fail "no committed result file in bench/results to copy"

# The task file is named, so the script runs whether or not the root Taskfile.yml includes it.
check() {
  task -d . -t taskfiles/bench.yml bench-schema-check "$@"
}

cp bench/results/*.json "$dir/committed/" || fail "cannot copy bench/results"
check BENCH_RESULTS="$dir/committed" > "$dir/committed.out" 2>&1 ||
  fail "bench-schema-check refused the committed result files ($dir/committed.out)"
grep -q '^bench-schema-check: [0-9]* result file(s) hold to ' "$dir/committed.out" ||
  fail "bench-schema-check printed no summary line for the committed files ($dir/committed.out)"

jq 'del(.rows[0].version)' "$first" > "$dir/no-version/result.json" || fail "cannot plant the row without version"
if check BENCH_RESULTS="$dir/no-version" > "$dir/no-version.out" 2>&1; then
  fail "bench-schema-check accepted a row without version ($dir/no-version.out)"
fi
grep -q 'result.json: \$\.rows\[0\]: missing version$' "$dir/no-version.out" ||
  fail "bench-schema-check did not name the missing version ($dir/no-version.out)"

jq 'del(.rows[0].runs)' "$first" > "$dir/no-runs/result.json" || fail "cannot plant the row without runs"
if check BENCH_RESULTS="$dir/no-runs" > "$dir/no-runs.out" 2>&1; then
  fail "bench-schema-check accepted a row without runs ($dir/no-runs.out)"
fi
grep -q 'result.json: \$\.rows\[0\]: missing runs$' "$dir/no-runs.out" ||
  fail "bench-schema-check did not name the missing runs ($dir/no-runs.out)"

if check BENCH_RESULTS="$dir/empty" > "$dir/empty.out" 2>&1; then
  fail "bench-schema-check passed a directory without a result file ($dir/empty.out)"
fi
grep -q '^bench-schema-check: no result file in ' "$dir/empty.out" ||
  fail "bench-schema-check did not say that the directory is empty ($dir/empty.out)"

echo "$name: ok"

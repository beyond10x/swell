#!/bin/sh
# Planted change for the classifier of changed files (taskfiles/classify-changes.sh): a refresh
# of the compare page's inputs, the engine catalog (bench/engines.jsonl) and the published
# tables (bench/published/*), with the regenerated website/data/engines.json, is site-only:
# the forge runs python-check and site-data-check, which reads them, and neither builds nor
# tests the code. The same files with the generator of engines.json (taskfiles/site-engines.sh)
# are code. The classifier reads this repository's path list (.github/planning-paths.txt) and
# the paths from a file (-f).
# Run from the repository root: sh tests/gate/classify-changes-finds-site-only-in-the-engine-catalog-and-published-data.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".
# The same output on any machine: no colour, and no "::error" line that go-task adds under
# GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=classify-changes-finds-site-only-in-the-engine-catalog-and-published-data
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }
rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"

printf '%s\n' bench/engines.jsonl bench/published/zoo-test262-4a1e962.tsv bench/published/SHA256SUMS \
  bench/published/LICENSE-javascript-zoo-data website/data/engines.json > "$dir/refresh.txt" || fail "cannot write the paths"
sh taskfiles/classify-changes.sh -l .github/planning-paths.txt -f "$dir/refresh.txt" > "$dir/refresh.out" 2>&1 ||
  fail "the classifier exited non-zero ($dir/refresh.out)"
grep -qxF "classify-changes: site-only: 5 changed file(s), 5 of them of the site, the rest planning or prose" "$dir/refresh.out" ||
  fail "the classifier did not print site-only for the catalog and the published tables ($dir/refresh.out)"

cp "$dir/refresh.txt" "$dir/generator.txt" && printf '%s\n' taskfiles/site-engines.sh >> "$dir/generator.txt" ||
  fail "cannot write the paths"
sh taskfiles/classify-changes.sh -l .github/planning-paths.txt -f "$dir/generator.txt" > "$dir/generator.out" 2>&1 ||
  fail "the classifier exited non-zero ($dir/generator.out)"
grep -qF "classify-changes: code: 6 changed file(s), 1 of them code, the first taskfiles/site-engines.sh" "$dir/generator.out" ||
  fail "the classifier did not print code for the generator ($dir/generator.out)"

echo "$name: ok"

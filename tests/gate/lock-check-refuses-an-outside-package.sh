#!/bin/sh
# The task lock-check refuses a Cargo.lock holding a package from a registry, and names it.
# Run from the repository root: sh tests/gate/lock-check-refuses-an-outside-package.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=lock-check-refuses-an-outside-package
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
[ -f Cargo.lock ] || fail "no Cargo.lock at the root to copy"

cp Cargo.lock "$dir/Cargo.lock" || fail "cannot copy Cargo.lock"
task -d . lock-check GATE_LOCK="$dir/Cargo.lock" > "$dir/unplanted.log" 2>&1 ||
  fail "lock-check refused the unplanted copy of Cargo.lock (log: $dir/unplanted.log)"

source='registry+https://github.com/rust-lang/crates.io-index'
printf '\n[[package]]\nname = "planted-outside"\nversion = "1.2.3"\nsource = "%s"\n' "$source" >> "$dir/Cargo.lock"
if task -d . lock-check GATE_LOCK="$dir/Cargo.lock" > "$dir/planted.log" 2>&1; then
  fail "lock-check accepted a package from $source (log: $dir/planted.log)"
fi
grep -qxF "Cargo.lock: planted-outside 1.2.3 from $source" "$dir/planted.log" ||
  fail "lock-check failed without naming the planted package (log: $dir/planted.log)"

echo "$name: ok"

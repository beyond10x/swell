#!/bin/sh
# The task forbid-unsafe-check refuses a crate root without #![forbid(unsafe_code)], naming that
# root and no other.
# Run from the repository root: sh tests/gate/forbid-unsafe-check-refuses-a-crate-without-it.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=forbid-unsafe-check-refuses-a-crate-without-it
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"

crates=$dir/crates
mkdir -p "$crates/with/src" "$crates/without/src" || fail "cannot plant $crates"
printf '#![forbid(unsafe_code)]\n//! A planted crate with the line.\n' > "$crates/with/src/lib.rs"
printf '//! A planted crate without the line.\n\nfn main() {}\n' > "$crates/without/src/main.rs"

if task -d . forbid-unsafe-check GATE_CRATES="$crates" > "$dir/task.log" 2>&1; then
  fail "forbid-unsafe-check accepted $crates/without/src/main.rs (log: $dir/task.log)"
fi
grep -qxF "$crates/without/src/main.rs: no #![forbid(unsafe_code)]" "$dir/task.log" ||
  fail "forbid-unsafe-check failed without naming $crates/without/src/main.rs (log: $dir/task.log)"
if grep -qF "$crates/with/" "$dir/task.log"; then
  fail "forbid-unsafe-check named the crate that has the line (log: $dir/task.log)"
fi

echo "$name: ok"

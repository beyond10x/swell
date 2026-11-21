#!/bin/sh
# The command of the task build refuses a compiler warning (an unused variable); without
# RUSTFLAGS="-D warnings" the same crate builds, so the refusal comes from the denial.
# Run from the repository root: sh tests/gate/build-refuses-a-compiler-warning.sh
# Works only under .out/gate-tests/<name>/, which is also its cargo target directory's parent;
# prints "<name>: ok" or "<name>: FAIL <why>".
#
# The planted workspace has one member, crates/swell. Its root manifest is the real one's
# [workspace.package], [workspace.lints] and [profile] sections under its own [workspace]; its
# crates/swell/Cargo.toml is the real one's [package] and [lints] sections, without dependencies
# or targets, so a new path dependency or a new crate of the workspace does not reach it. Its
# Cargo.lock is made for it (cargo generate-lockfile --offline, no dependency to fetch); the
# build runs with --locked as the task's does, which then checks that the build leaves it as made.

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=build-refuses-a-compiler-warning
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/crates/swell/src" || fail "cannot make $dir"
for file in Cargo.toml clippy.toml crates/swell/Cargo.toml; do
  [ -f "$file" ] || fail "no $file to copy"
done
cp clippy.toml "$dir/clippy.toml" || fail "cannot copy clippy.toml"
{
  printf '[workspace]\nmembers = ["crates/swell"]\nresolver = "3"\n\n'
  awk '/^\[/ { keep = ($0 == "[workspace.package]" || $0 ~ /^\[workspace\.lints[].]/ || $0 ~ /^\[profile[].]/) } keep' Cargo.toml
} > "$dir/Cargo.toml" || fail "cannot write the planted Cargo.toml"
awk '/^\[/ { keep = ($0 == "[package]" || $0 ~ /^\[lints[].]/) } keep' crates/swell/Cargo.toml > "$dir/crates/swell/Cargo.toml" ||
  fail "cannot write the planted crates/swell/Cargo.toml"
grep -qx '\[package\]' "$dir/crates/swell/Cargo.toml" || fail "crates/swell/Cargo.toml has no [package] section"
cat > "$dir/crates/swell/src/lib.rs" <<'EOF' || fail "cannot write the planted lib.rs"
#![forbid(unsafe_code)]
//! A planted engine crate with an unused variable.

/// Binds a value and never reads it.
pub fn nothing() {
    let unread = 1;
}
EOF
(cd "$dir" && cargo generate-lockfile --offline) > "$dir/lock.log" 2>&1 ||
  fail "cannot make a Cargo.lock for the planted workspace (log: $dir/lock.log)"

if (cd "$dir" && RUSTFLAGS="-D warnings" cargo build --workspace --all-targets --locked) > "$dir/denied.log" 2>&1; then
  fail "the build accepted an unused variable (log: $dir/denied.log)"
fi
grep -qF 'unused variable' "$dir/denied.log" ||
  fail "the build failed without naming an unused variable (log: $dir/denied.log)"

(cd "$dir" && unset RUSTFLAGS && cargo build --workspace --all-targets --locked) > "$dir/allowed.log" 2>&1 ||
  fail "without -D warnings the planted crate does not build either (log: $dir/allowed.log)"

echo "$name: ok"

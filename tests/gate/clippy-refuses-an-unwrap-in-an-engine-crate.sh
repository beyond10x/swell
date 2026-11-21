#!/bin/sh
# The command of the task clippy refuses an unwrap() outside a test in the engine crate, through
# the lints of crates/swell/Cargo.toml: with those lints removed the same crate passes.
# Run from the repository root: sh tests/gate/clippy-refuses-an-unwrap-in-an-engine-crate.sh
# Works only under .out/gate-tests/<name>/, which is also its cargo target directory's parent;
# prints "<name>: ok" or "<name>: FAIL <why>".
#
# The planted workspace has one member, crates/swell. Its root manifest is the real one's
# [workspace.package], [workspace.lints] and [profile] sections under its own [workspace]; its
# crates/swell/Cargo.toml is the real one's [package] and [lints] sections, without dependencies
# or targets, so a new path dependency or a new crate of the workspace does not reach it while
# the lints are the real ones. Its Cargo.lock is made for it (cargo generate-lockfile --offline,
# no dependency to fetch); clippy runs with --locked as the task's does, which then checks that
# the run leaves it as made.

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=clippy-refuses-an-unwrap-in-an-engine-crate
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
grep -qx '\[lints\.clippy\]' "$dir/crates/swell/Cargo.toml" || fail "crates/swell/Cargo.toml has no [lints.clippy] section"
cat > "$dir/crates/swell/src/lib.rs" <<'EOF' || fail "cannot write the planted lib.rs"
#![forbid(unsafe_code)]
//! A planted engine crate that unwraps outside a test.

/// Returns the value, panicking when there is none.
pub fn first(value: Option<u8>) -> u8 {
    value.unwrap()
}
EOF
(cd "$dir" && cargo generate-lockfile --offline) > "$dir/lock.log" 2>&1 ||
  fail "cannot make a Cargo.lock for the planted workspace (log: $dir/lock.log)"

if (cd "$dir" && cargo clippy --workspace --all-targets --locked -- -D warnings) > "$dir/with-lints.log" 2>&1; then
  fail "clippy accepted an unwrap() in the engine crate (log: $dir/with-lints.log)"
fi
grep -qF 'requested on the command line with `-D clippy::unwrap-used`' "$dir/with-lints.log" ||
  fail "clippy failed without naming clippy::unwrap-used as requested by the lints (log: $dir/with-lints.log)"

awk '/^\[/ { skip = ($0 == "[lints.clippy]") } !skip' "$dir/crates/swell/Cargo.toml" > "$dir/without-lints.toml" &&
  mv "$dir/without-lints.toml" "$dir/crates/swell/Cargo.toml" ||
  fail "cannot remove the lints from the planted crates/swell/Cargo.toml"
(cd "$dir" && cargo clippy --workspace --all-targets --locked -- -D warnings) > "$dir/without-lints.log" 2>&1 ||
  fail "without the lints of crates/swell/Cargo.toml the planted crate is refused too (log: $dir/without-lints.log)"

echo "$name: ok"

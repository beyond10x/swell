#!/bin/sh
# The task test-collect refuses a crates/swell whose manifest has no feature
# collect-every-safe-point, so that it cannot pass by running nothing in that mode; the same
# crate with the feature passes it first, so the refusal comes from the missing feature.
# Run from the repository root: sh tests/gate/test-collect-refuses-a-crate-without-the-feature.sh
# Works only under .out/gate-tests/<name>/, which is also its cargo target directory's parent;
# prints "<name>: ok" or "<name>: FAIL <why>".
#
# The planted workspace has one member, crates/swell, as build-refuses-a-compiler-warning.sh
# builds it: its root manifest is the real one's [workspace.package], [workspace.lints] and
# [profile] sections under its own [workspace]; its crates/swell/Cargo.toml is the real one's
# [package] and [lints] sections, without dependencies, targets or features. Its Cargo.lock is
# made for it (cargo generate-lockfile --offline). Its lib.rs has one passing test. The task
# runs in the planted workspace (task -d), with the real taskfiles/collect-mode.yml.

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-collect-refuses-a-crate-without-the-feature
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

taskfile=$(pwd)/taskfiles/collect-mode.yml
[ -f "$taskfile" ] || fail "no taskfiles/collect-mode.yml"
rm -rf "$dir" && mkdir -p "$dir/crates/swell/src" || fail "cannot make $dir"
for file in Cargo.toml clippy.toml crates/swell/Cargo.toml; do
  [ -f "$file" ] || fail "no $file to copy"
done
cp clippy.toml "$dir/clippy.toml" || fail "cannot copy clippy.toml"
{
  printf '[workspace]\nmembers = ["crates/swell"]\nresolver = "3"\n\n'
  awk '/^\[/ { keep = ($0 == "[workspace.package]" || $0 ~ /^\[workspace\.lints[].]/ || $0 ~ /^\[profile[].]/) } keep' Cargo.toml
} > "$dir/Cargo.toml" || fail "cannot write the planted Cargo.toml"
manifest=$dir/crates/swell/Cargo.toml
awk '/^\[/ { keep = ($0 == "[package]" || $0 ~ /^\[lints[].]/) } keep' crates/swell/Cargo.toml > "$manifest.without" ||
  fail "cannot write the planted crates/swell/Cargo.toml"
grep -qx '\[package\]' "$manifest.without" || fail "crates/swell/Cargo.toml has no [package] section"
if grep -q 'collect-every-safe-point' "$manifest.without"; then
  fail "the planted manifest names the feature already ($manifest.without)"
fi
{
  cat "$manifest.without"
  printf '\n[features]\ncollect-every-safe-point = []\n'
} > "$manifest.with" || fail "cannot write the planted manifest with the feature"
cat > "$dir/crates/swell/src/lib.rs" <<'EOF' || fail "cannot write the planted lib.rs"
#![forbid(unsafe_code)]
//! A planted engine crate with one passing test.

/// Answers one.
pub fn one() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    #[test]
    fn one_is_one() {
        assert_eq!(super::one(), 1);
    }
}
EOF
cp "$manifest.with" "$manifest" || fail "cannot plant the manifest with the feature"
(cd "$dir" && cargo generate-lockfile --offline) > "$dir/lock.log" 2>&1 ||
  fail "cannot make a Cargo.lock for the planted workspace (log: $dir/lock.log)"

# With the feature the planted crate passes the task, which prints its counts.
task -d "$dir" -t "$taskfile" test-collect > "$dir/with.log" 2>&1 ||
  fail "the task refused the planted crate with the feature (log: $dir/with.log)"
grep -qxF 'test-collect: 2 test suites: 1 passed, 0 failed, 0 ignored' "$dir/with.log" ||
  fail "the task did not print the counts of the planted crate (log: $dir/with.log)"

# Without it the task fails, naming the feature.
cp "$manifest.without" "$manifest" || fail "cannot plant the manifest without the feature"
if task -d "$dir" -t "$taskfile" test-collect > "$dir/without.log" 2>&1; then
  fail "the task accepted a crates/swell without the feature collect-every-safe-point (log: $dir/without.log)"
fi
grep -qF 'collect-every-safe-point' "$dir/without.log" ||
  fail "the task failed without naming the feature (log: $dir/without.log)"
grep -qxF 'test-collect: 0 test suites: 0 passed, 0 failed, 0 ignored' "$dir/without.log" ||
  fail "the task did not print that no test ran (log: $dir/without.log)"

echo "$name: ok"

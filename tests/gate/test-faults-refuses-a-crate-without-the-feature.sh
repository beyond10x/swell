#!/bin/sh
# The task test-faults refuses a crates/swell whose manifest has no feature fault-injection, so
# that it cannot pass by running nothing with the switch; the same crate with the feature passes
# it first, so the refusal comes from the missing feature. With the feature and its one test
# ignored, the task refuses too: no test passed.
# Run from the repository root: sh tests/gate/test-faults-refuses-a-crate-without-the-feature.sh
# Works only under .out/gate-tests/<name>/, which is also its cargo target directory's parent;
# prints "<name>: ok" or "<name>: FAIL <why>".
#
# The planted workspace has one member, crates/swell, as test-collect-refuses-a-crate-without-
# the-feature.sh builds it: its root manifest is the real one's [workspace.package],
# [workspace.lints] and [profile] sections under its own [workspace]; its crates/swell/Cargo.toml
# is the real one's [package] and [lints] sections, without dependencies, targets or features.
# Its Cargo.lock is made for it (cargo generate-lockfile --offline). Its lib.rs is one function,
# and its tests/faults.rs one passing test. The task runs in the planted workspace (task -d),
# with the real taskfiles/fault-injection.yml and taskfiles/nextest.yml and the real
# .config/nextest.toml.

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-faults-refuses-a-crate-without-the-feature
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

real=$(pwd)/taskfiles/fault-injection.yml
nextest=$(pwd)/taskfiles/nextest.yml
[ -f "$real" ] || fail "no taskfiles/fault-injection.yml"
[ -f "$nextest" ] || fail "no taskfiles/nextest.yml"
[ -f .config/nextest.toml ] || fail "no .config/nextest.toml"
rm -rf "$dir" && mkdir -p "$dir/crates/swell/src" "$dir/crates/swell/tests" || fail "cannot make $dir"
# The task runs through cargo-nextest with the real configuration, and prints its counts with
# the task nextest-report of taskfiles/nextest.yml: the planted Taskfile.yml includes both files.
mkdir -p "$dir/.config" && cp .config/nextest.toml "$dir/.config/nextest.toml" || fail "cannot copy .config/nextest.toml"
taskfile=$(pwd)/$dir/Taskfile.yml
printf "version: '3'\n\nincludes:\n  real:\n    taskfile: %s\n    flatten: true\n  nextest:\n    taskfile: %s\n    flatten: true\n" \
  "$real" "$nextest" > "$taskfile" || fail "cannot write the planted Taskfile.yml"
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
if grep -q 'fault-injection' "$manifest.without"; then
  fail "the planted manifest names the feature already ($manifest.without)"
fi
{
  cat "$manifest.without"
  printf '\n[features]\nfault-injection = []\n'
} > "$manifest.with" || fail "cannot write the planted manifest with the feature"
cat > "$dir/crates/swell/src/lib.rs" <<'EOF' || fail "cannot write the planted lib.rs"
#![forbid(unsafe_code)]
//! A planted engine crate.

/// Answers one.
pub fn one() -> u32 {
    1
}
EOF
faults=$dir/crates/swell/tests/faults.rs
cat > "$faults.passing" <<'EOF' || fail "cannot write the planted tests/faults.rs"
//! A planted fault test that passes.

#[test]
fn injected() {
    assert_eq!(swell::one(), 1);
}
EOF
cat > "$faults.ignored" <<'EOF' || fail "cannot write the planted ignored tests/faults.rs"
//! A planted fault test that is ignored.

#[test]
#[ignore = "planted"]
fn injected() {
    assert_eq!(swell::one(), 1);
}
EOF
cp "$manifest.with" "$manifest" || fail "cannot plant the manifest with the feature"
cp "$faults.passing" "$faults" || fail "cannot plant the passing test"
(cd "$dir" && cargo generate-lockfile --offline) > "$dir/lock.log" 2>&1 ||
  fail "cannot make a Cargo.lock for the planted workspace (log: $dir/lock.log)"

# With the feature the planted crate passes the task, which prints its counts.
task -d "$dir" -t "$taskfile" test-faults > "$dir/with.log" 2>&1 ||
  fail "the task refused the planted crate with the feature (log: $dir/with.log)"
grep -qxF 'test-faults: 1 test binaries: 1 passed, 0 failed, 0 ignored' "$dir/with.log" ||
  fail "the task did not print the counts of the planted crate (log: $dir/with.log)"

# With the feature and the test ignored the task fails: no test passed.
cp "$faults.ignored" "$faults" || fail "cannot plant the ignored test"
if task -d "$dir" -t "$taskfile" test-faults > "$dir/ignored.log" 2>&1; then
  fail "the task accepted a run in which no test passed (log: $dir/ignored.log)"
fi
grep -qxF 'test-faults: 1 test binaries: 0 passed, 0 failed, 1 ignored' "$dir/ignored.log" ||
  fail "the task did not print the counts of the ignored run (log: $dir/ignored.log)"

# Without the feature the task fails, naming the feature.
cp "$faults.passing" "$faults" || fail "cannot plant the passing test again"
cp "$manifest.without" "$manifest" || fail "cannot plant the manifest without the feature"
if task -d "$dir" -t "$taskfile" test-faults > "$dir/without.log" 2>&1; then
  fail "the task accepted a crates/swell without the feature fault-injection (log: $dir/without.log)"
fi
grep -qF 'fault-injection' "$dir/without.log" ||
  fail "the task failed without naming the feature (log: $dir/without.log)"
grep -qxF 'test-faults: 0 test binaries: 0 passed, 0 failed, 0 ignored' "$dir/without.log" ||
  fail "the task did not print that no test ran (log: $dir/without.log)"

echo "$name: ok"

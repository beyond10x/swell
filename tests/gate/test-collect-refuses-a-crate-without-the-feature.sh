#!/bin/sh
# The gate refuses a crates/swell whose manifest has no feature collect-every-safe-point, so
# that it cannot pass by running nothing in that mode; the same crate with the feature passes
# first, so the refusal comes from the missing feature. The gate builds the step's binaries in
# its build phase (task build-collect, before gate-time-start), so the refusal comes there:
# build-collect fails, naming the feature and "build-collect: cargo nextest run --no-run failed,
# exit <n>". The step test-collect run alone on that crate fails too, with "test-collect: read
# no "Starting <n> tests across <b> binaries" line ...: nothing to count" and no count line.
# With the feature, build-collect also refuses a .cargo/collect-profile.toml with a key beyond
# inherits and opt-level, naming the line, before it builds.
# Run from the repository root: sh tests/gate/test-collect-refuses-a-crate-without-the-feature.sh
# Works only under .out/gate-tests/<name>/, which is also its cargo target directory's parent;
# prints "<name>: ok" or "<name>: FAIL <why>".
#
# The planted workspace has one member, crates/swell, as build-refuses-a-compiler-warning.sh
# builds it: its root manifest is the real one's [workspace.package], [workspace.lints] and
# [profile] sections under its own [workspace]; its crates/swell/Cargo.toml is the real one's
# [package] and [lints] sections, without dependencies, targets or features. Its Cargo.lock is
# made for it (cargo generate-lockfile --offline). Its lib.rs has one passing test. The task
# runs in the planted workspace (task -d), with the real taskfiles/collect-mode.yml and
# taskfiles/nextest.yml, the real .config/nextest.toml and the real .cargo/collect-profile.toml
# (the profile collect the step builds and runs in).

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-collect-refuses-a-crate-without-the-feature
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

real=$(pwd)/taskfiles/collect-mode.yml
nextest=$(pwd)/taskfiles/nextest.yml
[ -f "$real" ] || fail "no taskfiles/collect-mode.yml"
[ -f "$nextest" ] || fail "no taskfiles/nextest.yml"
[ -f .config/nextest.toml ] || fail "no .config/nextest.toml"
rm -rf "$dir" && mkdir -p "$dir/crates/swell/src" || fail "cannot make $dir"
# The task runs through cargo-nextest with the real configuration, and prints its counts with
# the task nextest-report of taskfiles/nextest.yml: the planted Taskfile.yml includes both files.
mkdir -p "$dir/.config" && cp .config/nextest.toml "$dir/.config/nextest.toml" || fail "cannot copy .config/nextest.toml"
mkdir -p "$dir/.cargo" && cp .cargo/collect-profile.toml "$dir/.cargo/collect-profile.toml" || fail "cannot copy .cargo/collect-profile.toml, the profile collect of the collect mode"
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

# With the feature the planted crate passes the build task of the gate's build phase,
# build-collect, and then the step, which compiles nothing and prints its counts.
task -d "$dir" -t "$taskfile" build-collect > "$dir/build-with.log" 2>&1 ||
  fail "the task build-collect refused the planted crate with the feature (log: $dir/build-with.log)"
grep -q '^build-collect: Finished `collect` profile' "$dir/build-with.log" ||
  fail "the task build-collect did not print cargo's Finished line of the profile collect (log: $dir/build-with.log)"
grep -qE '^build-collect: [1-9][0-9]* test executables in [^ ]*collect/deps, none with a \.debug_ section$' "$dir/build-with.log" ||
  fail "the task build-collect did not check the test executables of <target>/collect/deps for debug sections (log: $dir/build-with.log)"

# A profile collect with a key beyond inherits and opt-level is refused before anything builds.
cp "$dir/.cargo/collect-profile.toml" "$dir/collect-profile.toml.real" || fail "cannot keep the real profile"
printf 'debug-assertions = false\n' >> "$dir/.cargo/collect-profile.toml" || fail "cannot plant a fourth key"
if task -d "$dir" -t "$taskfile" build-collect > "$dir/build-profile.log" 2>&1; then
  fail "the task build-collect accepted a profile collect with debug-assertions = false (log: $dir/build-profile.log)"
fi
grep -qxF 'build-collect: .cargo/collect-profile.toml holds "debug-assertions = false" as its line 4; the profile collect is [profile.collect], inherits = "test", opt-level = 1 and nothing else' "$dir/build-profile.log" ||
  fail "the task build-collect did not name the planted line of the profile (log: $dir/build-profile.log)"
cp "$dir/collect-profile.toml.real" "$dir/.cargo/collect-profile.toml" || fail "cannot put the real profile back"
task -d "$dir" -t "$taskfile" test-collect > "$dir/with.log" 2>&1 ||
  fail "the task refused the planted crate with the feature (log: $dir/with.log)"
grep -qxF 'test-collect: 1 test binaries: 1 passed, 0 failed, 0 ignored' "$dir/with.log" ||
  fail "the task did not print the counts of the planted crate (log: $dir/with.log)"

# Without it the gate refuses the crate in its build phase: build-collect fails, naming the
# feature. The step run alone on that crate (no build phase before it) fails too, and does not
# pass as a run of no tests: nextest builds nothing, and the report says it read no
# "Starting" line, prints no count and fails.
cp "$manifest.without" "$manifest" || fail "cannot plant the manifest without the feature"
if task -d "$dir" -t "$taskfile" build-collect > "$dir/build-without.log" 2>&1; then
  fail "the task build-collect accepted a crates/swell without the feature collect-every-safe-point (log: $dir/build-without.log)"
fi
grep -qF 'collect-every-safe-point' "$dir/build-without.log" ||
  fail "the task build-collect failed without naming the feature (log: $dir/build-without.log)"
grep -qE '^build-collect: cargo nextest run --no-run failed, exit [1-9][0-9]*$' "$dir/build-without.log" ||
  fail "the task build-collect did not say that its build failed (log: $dir/build-without.log)"
if task -d "$dir" -t "$taskfile" test-collect > "$dir/without.log" 2>&1; then
  fail "the task accepted a crates/swell without the feature collect-every-safe-point (log: $dir/without.log)"
fi
grep -qF 'collect-every-safe-point' "$dir/without.log" ||
  fail "the task failed without naming the feature (log: $dir/without.log)"
grep -qF 'test-collect: read no "Starting <n> tests across <b> binaries" line with a binary in .out/test-collect.log (' "$dir/without.log" ||
  fail "the task did not say that it read no Starting line (log: $dir/without.log)"
if grep -q 'test binaries:' "$dir/without.log"; then
  fail "the task printed a count for a run that built nothing (log: $dir/without.log)"
fi

echo "$name: ok"

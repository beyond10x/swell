#!/bin/sh
# Planted defect of the task toolchain-check (taskfiles/forge.yml, story
# local-gate-equals-forge, point 1): the compiler is pinned in rust-toolchain.toml alone. The
# task reads GATE_ROOT/rust-toolchain.toml and GATE_ROOT/.github/workflows/*.yml; here GATE_ROOT
# is a copy under .out/gate-tests/<name>/. The unchanged copy passes first; then
#   1. check.yml with its toolchain input written as the version "1.99.1",
#   2. check.yml with the action taken by a version instead of its commit hash, and
#   3. rust-toolchain.toml pinning another channel than the rustc of the gate
# each make it exit non-zero: the first two name the file and line, the third names both
# versions in one line. rustc runs in the repository root, never in the copy, so the planted
# channel is not installed and nothing is fetched.
# Run from the repository root: sh tests/gate/toolchain-check-refuses-a-workflow-that-names-a-compiler.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=toolchain-check-refuses-a-workflow-that-names-a-compiler
dir=.out/gate-tests/$name
rm -rf "$dir"
fail() {
  echo "$name: FAIL $*"
  exit 1
}
mkdir -p "$dir/root/.github/workflows" || fail "cannot make $dir/root"

root=$dir/root
cp rust-toolchain.toml "$root/rust-toolchain.toml" || fail "cannot copy rust-toolchain.toml"
cp .github/workflows/*.yml "$root/.github/workflows/" || fail "cannot copy the workflows"
task -d . toolchain-check GATE_ROOT="$root" > "$dir/base.out" 2>&1 ||
  fail "task toolchain-check on the unchanged copy exited non-zero ($dir/base.out)"
grep -q '^toolchain-check: rustc ' "$dir/base.out" || fail "task toolchain-check printed no line ($dir/base.out)"

workflow=$root/.github/workflows/check.yml
sed 's/^\( *\)toolchain: .*$/\1toolchain: "1.99.1"/' .github/workflows/check.yml > "$workflow" ||
  fail "cannot plant the version"
grep -q 'toolchain: "1.99.1"' "$workflow" || fail "check.yml has no toolchain input to plant a version in"
if task -d . toolchain-check GATE_ROOT="$root" > "$dir/version.out" 2>&1; then
  fail "task toolchain-check exited 0 with check.yml naming 1.99.1 ($dir/version.out)"
fi
grep -qE "^toolchain-check: $workflow:[0-9]+: names a compiler" "$dir/version.out" ||
  fail "task toolchain-check did not name the line of $workflow ($dir/version.out)"

sed 's|dtolnay/rust-toolchain@[0-9a-f]*|dtolnay/rust-toolchain@1.99.1|' .github/workflows/check.yml > "$workflow" ||
  fail "cannot plant the action's version"
grep -q 'dtolnay/rust-toolchain@1.99.1' "$workflow" || fail "check.yml takes no dtolnay/rust-toolchain to plant a version in"
if task -d . toolchain-check GATE_ROOT="$root" > "$dir/action.out" 2>&1; then
  fail "task toolchain-check exited 0 with the action taken at 1.99.1 ($dir/action.out)"
fi
grep -qE "^toolchain-check: $workflow:[0-9]+: names a compiler" "$dir/action.out" ||
  fail "task toolchain-check did not name the line of $workflow ($dir/action.out)"
cp .github/workflows/check.yml "$workflow" || fail "cannot copy check.yml again"

gate=$(rustc --version | awk '{ print $2 }')
[ -n "$gate" ] || fail "rustc --version printed no version"
sed 's/^channel *=.*$/channel = "1.0.0"/' rust-toolchain.toml > "$root/rust-toolchain.toml" ||
  fail "cannot plant the channel"
if task -d . toolchain-check GATE_ROOT="$root" > "$dir/channel.out" 2>&1; then
  fail "task toolchain-check exited 0 with rust-toolchain.toml pinning 1.0.0 ($dir/channel.out)"
fi
grep -qxF "toolchain-check: rustc in the gate is $gate, $root/rust-toolchain.toml pins 1.0.0" "$dir/channel.out" ||
  fail "task toolchain-check did not name both versions in one line ($dir/channel.out)"

echo "$name: ok"

#!/bin/sh
# Planted defect of the task toolchain-check (taskfiles/forge.yml; story
# core-builds-without-std): the toolchain step of .github/workflows/check.yml installs every
# component and target rust-toolchain.toml names. The task reads GATE_ROOT/rust-toolchain.toml
# and GATE_ROOT/.github/workflows/*.yml; here GATE_ROOT is a copy under .out/gate-tests/<name>/.
# The unchanged copy passes first; then
#   1. check.yml with "components: clippy" (llvm-tools taken out), and
#   2. check.yml without its targets input
# each make it exit non-zero, naming the file, the line of the step and what it lacks; and
#   3. rust-toolchain.toml with a component more (rust-src) makes the unchanged check.yml lack it.
# rustc runs in the repository root, never in the copy, so nothing is installed or fetched.
# Run from the repository root: sh tests/gate/toolchain-check-refuses-a-workflow-without-a-component-of-the-toolchain-file.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=toolchain-check-refuses-a-workflow-without-a-component-of-the-toolchain-file
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/root/.github/workflows" || fail "cannot make $dir/root"
root=$dir/root
workflow=$root/.github/workflows/check.yml
cp rust-toolchain.toml "$root/rust-toolchain.toml" || fail "cannot copy rust-toolchain.toml"
cp .github/workflows/*.yml "$root/.github/workflows/" || fail "cannot copy the workflows"
task -d . toolchain-check GATE_ROOT="$root" > "$dir/base.out" 2>&1 ||
  fail "task toolchain-check on the unchanged copy exited non-zero ($dir/base.out)"
grep -q "^toolchain-check: rustc .*; $workflow installs the 2 component(s) and 1 target(s) $root/rust-toolchain.toml names$" "$dir/base.out" ||
  fail "task toolchain-check printed no line with the counts ($dir/base.out)"

sed 's/^\( *\)components: .*$/\1components: clippy/' .github/workflows/check.yml > "$workflow" ||
  fail "cannot plant the components"
grep -qx ' *components: clippy' "$workflow" || fail "check.yml has no components input to plant in"
if task -d . toolchain-check GATE_ROOT="$root" > "$dir/component.out" 2>&1; then
  fail "task toolchain-check exited 0 with check.yml installing clippy alone ($dir/component.out)"
fi
grep -qE "^toolchain-check: $workflow:[0-9]+: the toolchain step lacks the component llvm-tools, which rust-toolchain.toml names" "$dir/component.out" ||
  fail "task toolchain-check did not name llvm-tools and the line of the step ($dir/component.out)"

grep -v '^ *targets: ' .github/workflows/check.yml > "$workflow" || fail "cannot plant the missing targets"
if task -d . toolchain-check GATE_ROOT="$root" > "$dir/target.out" 2>&1; then
  fail "task toolchain-check exited 0 with check.yml installing no target ($dir/target.out)"
fi
grep -qE "^toolchain-check: $workflow:[0-9]+: the toolchain step lacks the target x86_64-unknown-none, which rust-toolchain.toml names" "$dir/target.out" ||
  fail "task toolchain-check did not name x86_64-unknown-none and the line of the step ($dir/target.out)"

cp .github/workflows/check.yml "$workflow" || fail "cannot copy check.yml again"
sed 's/^components = \[/components = ["rust-src", /' rust-toolchain.toml > "$root/rust-toolchain.toml" ||
  fail "cannot plant the component in rust-toolchain.toml"
if task -d . toolchain-check GATE_ROOT="$root" > "$dir/file.out" 2>&1; then
  fail "task toolchain-check exited 0 with rust-toolchain.toml naming rust-src ($dir/file.out)"
fi
grep -qE "^toolchain-check: $workflow:[0-9]+: the toolchain step lacks the component rust-src, which rust-toolchain.toml names" "$dir/file.out" ||
  fail "task toolchain-check did not name rust-src ($dir/file.out)"

echo "$name: ok"

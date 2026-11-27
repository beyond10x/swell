#!/bin/sh
# Planted defect of the task toolchain-check (taskfiles/forge.yml; story
# core-builds-without-std): rust-toolchain.toml may write its arrays with TOML literal strings
# ('clippy'). Here GATE_ROOT is a copy under .out/gate-tests/<name>/ whose rust-toolchain.toml
# names its components and targets that way:
#   1. with the unchanged check.yml it passes and counts 2 components and 1 target;
#   2. with check.yml installing clippy alone it exits non-zero and names llvm-tools;
#   3. with components = [clippy] (no string the check reads) it exits non-zero and names the
#      key.
# rustc runs in the repository root, never in the copy, so nothing is installed or fetched.
# Run from the repository root: sh tests/gate/toolchain-check-reads-a-single-quoted-array-of-the-toolchain-file.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=toolchain-check-reads-a-single-quoted-array-of-the-toolchain-file
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/root/.github/workflows" || fail "cannot make $dir/root"
root=$dir/root
file=$root/rust-toolchain.toml
workflow=$root/.github/workflows/check.yml
cp .github/workflows/*.yml "$root/.github/workflows/" || fail "cannot copy the workflows"
channel=$(awk -F '"' '/^[ \t]*channel[ \t]*=/ { print $2; exit }' rust-toolchain.toml)
{
  printf '[toolchain]\nchannel = "%s"\n' "$channel"
  printf "components = ['clippy', 'llvm-tools'] # the gate's two\n"
  printf "targets = ['x86_64-unknown-none']\n"
} > "$file" || fail "cannot write the single-quoted rust-toolchain.toml"

task -d . toolchain-check GATE_ROOT="$root" > "$dir/base.out" 2>&1 ||
  fail "task toolchain-check on single-quoted arrays and the unchanged check.yml exited non-zero ($dir/base.out)"
grep -q "^toolchain-check: rustc .*; $workflow installs the 2 component(s) and 1 target(s) $file names$" "$dir/base.out" ||
  fail "task toolchain-check did not count 2 components and 1 target ($dir/base.out)"

sed 's/^\( *\)components: .*$/\1components: clippy/' .github/workflows/check.yml > "$workflow" ||
  fail "cannot plant the components"
grep -qx ' *components: clippy' "$workflow" || fail "check.yml has no components input to plant in"
if task -d . toolchain-check GATE_ROOT="$root" > "$dir/component.out" 2>&1; then
  fail "task toolchain-check exited 0 with check.yml installing clippy alone ($dir/component.out)"
fi
grep -qE "^toolchain-check: $workflow:[0-9]+: the toolchain step lacks the component llvm-tools, which rust-toolchain.toml names" "$dir/component.out" ||
  fail "task toolchain-check did not name llvm-tools ($dir/component.out)"

cp .github/workflows/check.yml "$workflow" || fail "cannot copy check.yml again"
sed "s/^components = .*$/components = [clippy]/" "$file" > "$dir/unread.toml" && mv "$dir/unread.toml" "$file" ||
  fail "cannot plant the unread components"
if task -d . toolchain-check GATE_ROOT="$root" > "$dir/unread.out" 2>&1; then
  fail "task toolchain-check exited 0 with components = [clippy] ($dir/unread.out)"
fi
grep -qx "toolchain-check: $file: the key components names no value the check reads .*" "$dir/unread.out" ||
  fail "task toolchain-check did not name the key components ($dir/unread.out)"

echo "$name: ok"

#!/bin/sh
# Adversary pass 2 on unit gate: toolchain-check (taskfiles/forge.yml) reads the components and
# targets of rust-toolchain.toml from the line of the key alone. The same file in two other
# valid TOML forms must still make a check.yml that installs clippy alone and no target fail:
#   1. the arrays over several lines (components = [ / "clippy", / "llvm-tools", / ]), and
#   2. the arrays with TOML literal strings ('clippy', 'llvm-tools').
# GATE_ROOT is a copy under .out/gate-tests/<name>/; rustc runs in the repository root.
# Run from the repository root:
#   sh tests/gate/toolchain-check-reads-a-components-array-over-several-lines.sh
# Prints "<name>: ok" or "<name>: FAIL <why>".

unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=toolchain-check-reads-a-components-array-over-several-lines
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/root/.github/workflows" || fail "cannot make $dir/root"
root=$dir/root
workflow=$root/.github/workflows/check.yml
cp .github/workflows/*.yml "$root/.github/workflows/" || fail "cannot copy the workflows"
sed -e 's/^\( *\)components: .*$/\1components: clippy/' -e '/^ *targets: /d' \
  .github/workflows/check.yml > "$workflow" || fail "cannot plant check.yml"
grep -qx ' *components: clippy' "$workflow" || fail "check.yml has no components input to plant in"

channel=$(awk -F '"' '/^[ \t]*channel[ \t]*=/ { print $2; exit }' rust-toolchain.toml)
{
  printf '[toolchain]\nchannel = "%s"\n' "$channel"
  printf 'components = [\n    "clippy",\n    "llvm-tools",\n]\n'
  printf 'targets = [\n    "x86_64-unknown-none",\n]\n'
} > "$root/rust-toolchain.toml" || fail "cannot write the multi-line rust-toolchain.toml"
if task -d . toolchain-check GATE_ROOT="$root" > "$dir/multi-line.out" 2>&1; then
  fail "toolchain-check exited 0 with rust-toolchain.toml naming llvm-tools and x86_64-unknown-none over several lines and check.yml installing neither ($dir/multi-line.out: $(tail -n 1 "$dir/multi-line.out"))"
fi

{
  printf '[toolchain]\nchannel = "%s"\n' "$channel"
  printf "components = ['clippy', 'llvm-tools']\n"
  printf "targets = ['x86_64-unknown-none']\n"
} > "$root/rust-toolchain.toml" || fail "cannot write the literal-string rust-toolchain.toml"
if task -d . toolchain-check GATE_ROOT="$root" > "$dir/literal.out" 2>&1; then
  fail "toolchain-check exited 0 with rust-toolchain.toml naming llvm-tools in literal strings and check.yml lacking it ($dir/literal.out: $(tail -n 1 "$dir/literal.out"))"
fi

echo "$name: ok"

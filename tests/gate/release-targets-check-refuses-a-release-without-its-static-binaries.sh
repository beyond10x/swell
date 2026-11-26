#!/bin/sh
# Planted defect of the task release-targets-check (taskfiles/forge.yml, story
# releases-ship-static-binaries): a release ships a static musl Linux binary beside each glibc
# one and a Windows binary with its C runtime linked in, and docs/releasing.md and
# website/docs/embedding.md name the archives the workflow builds and no other. The task reads
# GATE_ROOT/.github/workflows/release-build.yml and the two documents; here GATE_ROOT is a copy
# under .out/gate-tests/<name>/. The unchanged copy passes first; then
#   1. the workflow without its x86_64-unknown-linux-musl entry,
#   2. the workflow without target-feature=+crt-static for windows-msvc,
#   3. docs/releasing.md without the row of aarch64-unknown-linux-musl, and
#   4. website/docs/embedding.md naming an archive of a target the workflow does not build
# each make it exit non-zero with the line that names the defect.
# Run from the repository root: sh tests/gate/release-targets-check-refuses-a-release-without-its-static-binaries.sh
# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=release-targets-check-refuses-a-release-without-its-static-binaries
dir=.out/gate-tests/$name
rm -rf "$dir"
fail() {
  echo "$name: FAIL $*"
  exit 1
}
root=$dir/root
mkdir -p "$root/.github/workflows" "$root/docs" "$root/website/docs" || fail "cannot make $root"
wf=$root/.github/workflows/release-build.yml
rel=$root/docs/releasing.md
emb=$root/website/docs/embedding.md
copy() {
  cp .github/workflows/release-build.yml "$wf" &&
    cp docs/releasing.md "$rel" &&
    cp website/docs/embedding.md "$emb" || fail "cannot copy the workflow and the documents"
}
copy
task -d . release-targets-check GATE_ROOT="$root" > "$dir/base.out" 2>&1 ||
  fail "task release-targets-check on the unchanged copy exited non-zero ($dir/base.out)"
grep -q '^release-targets-check: [0-9]* target(s) in ' "$dir/base.out" ||
  fail "task release-targets-check printed no line ($dir/base.out)"

# refused <case> <expected line>: the task exits non-zero on the planted copy and prints the line.
refused() {
  if task -d . release-targets-check GATE_ROOT="$root" > "$dir/$1.out" 2>&1; then
    fail "task release-targets-check exited 0 with $1 planted ($dir/$1.out)"
  fi
  grep -qxF -- "$2" "$dir/$1.out" || fail "task release-targets-check did not print \"$2\" ($dir/$1.out)"
  copy
}

awk '/^[ \t]*- target: x86_64-unknown-linux-musl[ \t]*$/ { skip = 3 } skip > 0 { skip--; next } { print }' \
  .github/workflows/release-build.yml > "$wf" || fail "cannot plant the missing musl target"
grep -q 'x86_64-unknown-linux-musl' .github/workflows/release-build.yml ||
  fail "the workflow builds no x86_64-unknown-linux-musl to remove"
refused musl "release-targets-check: $wf: builds x86_64-unknown-linux-gnu and not x86_64-unknown-linux-musl; a static musl binary goes beside each glibc one"

sed 's/ -C target-feature=+crt-static//' .github/workflows/release-build.yml > "$wf" ||
  fail "cannot plant the dynamic C runtime"
refused crt "release-targets-check: $wf: builds x86_64-pc-windows-msvc without -C target-feature=+crt-static on a line that names windows-msvc"

grep -v 'swell-<tag>-aarch64-unknown-linux-musl\.tar\.gz' docs/releasing.md > "$rel" ||
  fail "cannot plant the missing row"
refused row "release-targets-check: $rel: names no archive swell-<version>-aarch64-unknown-linux-musl.tar.gz, which $wf builds"

{ cat website/docs/embedding.md && echo 'Also `swell-<version>-riscv64gc-unknown-linux-gnu.tar.gz`.'; } > "$emb" ||
  fail "cannot plant the extra archive"
line=$(wc -l < "$emb" | tr -d ' ')
refused extra "release-targets-check: $emb:$line: names the archive swell-<version>-riscv64gc-unknown-linux-gnu.tar.gz, which $wf does not build"

echo "$name: ok"

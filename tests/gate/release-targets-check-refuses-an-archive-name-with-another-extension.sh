#!/bin/sh
# Planted defect of the task release-targets-check (taskfiles/forge.yml, story
# releases-ship-static-binaries): docs/releasing.md and website/docs/embedding.md name the
# archives the workflow builds "and no other". Here website/docs/embedding.md tells the reader
# to unpack an archive that no release has: the musl archive under the extension .tar.xz
# (case 1) and .tgz (case 2). Each must make the task exit non-zero.
# Run from the repository root: sh tests/gate/release-targets-check-refuses-an-archive-name-with-another-extension.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=release-targets-check-refuses-an-archive-name-with-another-extension
dir=.out/gate-tests/$name
rm -rf "$dir"
fail() {
  echo "$name: FAIL $*"
  exit 1
}
root=$dir/root
mkdir -p "$root/.github/workflows" "$root/docs" "$root/website/docs" || fail "cannot make $root"
emb=$root/website/docs/embedding.md
cp .github/workflows/release-build.yml "$root/.github/workflows/release-build.yml" &&
  cp docs/releasing.md "$root/docs/releasing.md" || fail "cannot copy the workflow and docs/releasing.md"
grep -q '^tar -xzf swell-<version>-x86_64-unknown-linux-musl\.tar\.gz$' website/docs/embedding.md ||
  fail "website/docs/embedding.md has no tar line to plant on"
bad=0
for ext in tar.xz tgz; do
  sed "s/^tar -xzf swell-<version>-x86_64-unknown-linux-musl\\.tar\\.gz\$/tar -xf swell-<version>-x86_64-unknown-linux-musl.$ext/" \
    website/docs/embedding.md > "$emb" || fail "$ext: cannot plant"
  grep -q "swell-<version>-x86_64-unknown-linux-musl\\.$ext" "$emb" || fail "$ext: the plant changed nothing"
  if task -d . release-targets-check GATE_ROOT="$root" > "$dir/$ext.out" 2>&1; then
    echo "$name: $ext: task release-targets-check exited 0 on $emb naming swell-<version>-x86_64-unknown-linux-musl.$ext ($dir/$ext.out)"
    bad=1
  fi
done
[ "$bad" = 0 ] || exit 1
echo "$name: ok"

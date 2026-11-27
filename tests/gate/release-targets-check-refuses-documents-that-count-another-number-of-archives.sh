#!/bin/sh
# Planted defect of the task release-targets-check (taskfiles/forge.yml, story
# releases-ship-static-binaries): a release has seven archives and SHA256SUMS, and
# docs/releasing.md says "seven targets", "seven archives" and "eight" assets. Here a target is
# removed from the matrix and from the table of both documents at once, and the counts of
# docs/releasing.md stay: aarch64-apple-darwin (case 1), and the pair x86_64-unknown-linux-gnu
# and x86_64-unknown-linux-musl (case 2). The documents then count archives the workflow does
# not build; each case must make the task exit non-zero.
# Run from the repository root: sh tests/gate/release-targets-check-refuses-documents-that-count-another-number-of-archives.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=release-targets-check-refuses-documents-that-count-another-number-of-archives
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
grep -q 'seven archives' docs/releasing.md || fail "docs/releasing.md does not say seven archives"
bad=0

# plant <case> <target>...: the copy without each target's matrix entry and document rows.
plant() {
  case=$1
  shift
  cp .github/workflows/release-build.yml "$wf" &&
    cp docs/releasing.md "$rel" &&
    cp website/docs/embedding.md "$emb" || fail "cannot copy the workflow and the documents"
  for t in "$@"; do
    awk -v t="$t" '$0 ~ "^[ \t]*- target: " t "[ \t]*$" { skip = 3 } skip > 0 { skip--; next } { print }' \
      "$wf" > "$wf.new" && mv "$wf.new" "$wf" || fail "$case: cannot remove $t from the matrix"
    grep -v "swell-<[a-z]*>-$t\\." "$rel" > "$rel.new" && mv "$rel.new" "$rel" || fail "$case: cannot remove $t from $rel"
    grep -v "swell-<[a-z]*>-$t\\." "$emb" > "$emb.new" && mv "$emb.new" "$emb" || fail "$case: cannot remove $t from $emb"
    grep -q -- "- target: $t" "$wf" && fail "$case: $t is still in the matrix"
  done
  if task -d . release-targets-check GATE_ROOT="$root" > "$dir/$case.out" 2>&1; then
    echo "$name: $case: task release-targets-check exited 0 on a matrix of $(grep -c -- '- target:' "$wf") targets while $rel says seven archives ($dir/$case.out)"
    bad=1
  fi
}

plant darwin aarch64-apple-darwin
plant linux-pair x86_64-unknown-linux-gnu x86_64-unknown-linux-musl

[ "$bad" = 0 ] || exit 1
echo "$name: ok"

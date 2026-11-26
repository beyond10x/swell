#!/bin/sh
# False red of the task release-targets-check (taskfiles/forge.yml, story
# releases-ship-static-binaries): each edit below leaves a correct workflow and correct
# documents, and the task must still exit 0, or it holds every later pull request.
#   1. the Build step logs the runner: echo "... $RUNNER_OS" (the runner sets RUNNER_OS);
#   2. the Build step prints the compiler before it builds: rustc -vV;
#   3. the Build step names the target as ${{ matrix.target }} instead of $TARGET (the
#      runner substitutes it before bash runs);
#   4. docs/releasing.md says what website/docs/embedding.md already says: "Releases up to
#      0.12.0 have five archives".
# Run from the repository root: sh tests/gate/release-targets-check-passes-innocent-edits-of-the-workflow-and-the-documents.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=release-targets-check-passes-innocent-edits-of-the-workflow-and-the-documents
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
bad=0

# passes <case>: the task exits 0 on the edited copy.
passes() {
  if ! task -d . release-targets-check GATE_ROOT="$root" > "$dir/$1.out" 2>&1; then
    echo "$name: $1: task release-targets-check exited non-zero on an innocent edit: $(grep -m 1 '^release-targets-check: ' "$dir/$1.out")"
    bad=1
  fi
  copy
}

# The Build step is the run block with "cargo build"; its first line is set -euo pipefail.
build_after_set() {
  awk -v add="$1" '/^      - name: Build swell-cli$/ { inbuild = 1 }
    { print }
    inbuild && /^          set -euo pipefail$/ { print "          " add; inbuild = 0 }' \
    .github/workflows/release-build.yml > "$wf" || fail "cannot plant \"$1\""
  grep -qF -- "$1" "$wf" || fail "the plant \"$1\" changed nothing"
}

build_after_set 'echo "release-build: building $TARGET on $RUNNER_OS"'
passes runner-os

build_after_set 'rustc -vV'
passes rustc-version

sed 's/^          cargo build --release --locked -p swell-cli --target "\$TARGET"$/          cargo build --release --locked -p swell-cli --target "${{ matrix.target }}"/' \
  .github/workflows/release-build.yml > "$wf" || fail "cannot plant the expression"
grep -qF 'target "${{ matrix.target }}"' "$wf" || fail "the expression plant changed nothing"
passes expression

{ cat docs/releasing.md && printf '\n%s\n' 'Releases up to 0.12.0 have five archives, without the two musl ones.'; } > "$rel" ||
  fail "cannot plant the history sentence"
passes history

[ "$bad" = 0 ] || exit 1
echo "$name: ok"

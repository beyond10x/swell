#!/bin/sh
# Planted change for the classifier of changed files (taskfiles/classify-changes.sh, story
# local-gate-equals-forge, point 3): a Markdown file a gate step reads as data is code. In a
# git repository under .out/gate-tests/<name>/, a base tree holds the path list of this
# repository (.github/planning-paths.txt), the design, the host interface's requirements, a
# page of the documentation site and a plan file. Against it:
#   1. a change of docs/design/engine.md alone is code (the runner reads its swell-slices block
#      and count table), with site=false;
#   2. a change of docs/design/host-interface.md alone is code (host-requirements-check reads
#      its requirements);
#      with site=false.
# (A page under website/ alone is site-only: classify-changes-finds-site-only-in-a-site-page.)
# No commit is made: the revisions are trees.
# Run from the repository root: sh tests/gate/classify-changes-finds-code-in-the-design.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=classify-changes-finds-code-in-the-design
dir=.out/gate-tests/$name
rm -rf "$dir"
fail() {
  echo "$name: FAIL $*"
  exit 1
}
classifier=$(pwd)/taskfiles/classify-changes.sh
repo=$dir/repo
mkdir -p "$repo" && git init -q "$repo" || fail "cannot make the git repository $repo"

put() {
  mkdir -p "$repo/$(dirname "$1")" && printf '%s\n' "$2" > "$repo/$1" &&
    git -C "$repo" update-index --add -- "$1" || fail "cannot write $1"
}
mkdir -p "$repo/.github" && cp .github/planning-paths.txt "$repo/.github/planning-paths.txt" &&
  git -C "$repo" update-index --add -- .github/planning-paths.txt || fail "cannot copy the path list"
put docs/design/engine.md '# Design'
put docs/design/host-interface.md '# Host interface'
put website/docs/index.md '# swell'
put docs/plan/waves.md '# Waves'
base=$(git -C "$repo" write-tree) || fail "cannot write the base tree"

# classify <label> <file> <site>: change <file> alone against the base, expect code
classify() {
  git -C "$repo" read-tree "$base" || fail "cannot read the base tree"
  put "$2" 'changed'
  head=$(git -C "$repo" write-tree) || fail "cannot write the head tree"
  : > "$dir/$1.github-output"
  GITHUB_OUTPUT=$dir/$1.github-output sh "$classifier" -C "$repo" "$base" "$head" > "$dir/$1.out" 2>&1 ||
    fail "the classifier exited non-zero ($dir/$1.out)"
  grep -qxF "classify-changes: code: 1 changed file(s), 1 of them code, the first $2" "$dir/$1.out" ||
    fail "the classifier did not print code for $2 ($dir/$1.out)"
  printf 'kind=code\ncount=1\nsite=%s\n' "$3" | cmp -s - "$dir/$1.github-output" ||
    fail "the classifier did not write kind=code, count=1, site=$3 ($dir/$1.github-output)"
}

classify design docs/design/engine.md false
classify host docs/design/host-interface.md false

echo "$name: ok"

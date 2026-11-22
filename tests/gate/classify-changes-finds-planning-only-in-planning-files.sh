#!/bin/sh
# Planted change for the classifier of changed files (taskfiles/classify-changes.sh, story
# local-gate-equals-forge, point 3), which the forge's workflow runs before anything else. In a
# git repository under .out/gate-tests/<name>/, a base tree holds the path list of this
# repository (.github/planning-paths.txt), a crate file, the design and a plan file; the head
# tree changes one .engineering/ file and adds one docs/plan/*.md file. Between the two the
# classifier prints "planning-only" with the count 2 and writes kind=planning-only, count=2
# and site=false to GITHUB_OUTPUT. The same head against no base (a first push: the zero
# commit of the push event) and against a base the repository does not hold counts as code.
# No commit is made: the two revisions are trees (git write-tree).
# Run from the repository root: sh tests/gate/classify-changes-finds-planning-only-in-planning-files.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=classify-changes-finds-planning-only-in-planning-files
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
put crates/swell/src/lib.rs '#![forbid(unsafe_code)]'
put docs/design/engine.md '# Design'
put docs/plan/waves.md '# Waves'
put .engineering/planning/story-a.md 'status: draft'
base=$(git -C "$repo" write-tree) || fail "cannot write the base tree"

put .engineering/planning/story-a.md 'status: active'
put docs/plan/wave-9.md '# Wave 9'
head=$(git -C "$repo" write-tree) || fail "cannot write the head tree"

: > "$dir/github-output"
GITHUB_OUTPUT=$dir/github-output sh "$classifier" -C "$repo" "$base" "$head" > "$dir/planning.out" 2>&1 ||
  fail "the classifier exited non-zero ($dir/planning.out)"
grep -qxF "classify-changes: planning-only: 2 changed file(s), each a planning or prose file" "$dir/planning.out" ||
  fail "the classifier did not print planning-only for 2 file(s) ($dir/planning.out)"
printf 'kind=planning-only\ncount=2\nsite=false\n' | cmp -s - "$dir/github-output" ||
  fail "the classifier did not write kind, count and site to GITHUB_OUTPUT ($dir/github-output)"

sh "$classifier" -C "$repo" 0000000000000000000000000000000000000000 "$head" > "$dir/first.out" 2>&1 ||
  fail "the classifier exited non-zero on a first push ($dir/first.out)"
grep -qxF "classify-changes: code: no base revision (a first push); every file counts as code" "$dir/first.out" ||
  fail "the classifier did not count a first push as code ($dir/first.out)"

missing=1111111111111111111111111111111111111111
sh "$classifier" -C "$repo" "$missing" "$head" > "$dir/unreadable.out" 2>&1 ||
  fail "the classifier exited non-zero on an unreadable base ($dir/unreadable.out)"
grep -qxF "classify-changes: code: the base $missing cannot be read; every file counts as code" "$dir/unreadable.out" ||
  fail "the classifier did not count an unreadable base as code ($dir/unreadable.out)"

echo "$name: ok"

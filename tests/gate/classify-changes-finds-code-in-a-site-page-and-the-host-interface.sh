#!/bin/sh
# Planted change for the classifier of changed files (taskfiles/classify-changes.sh): a change
# of docs/design/host-interface.md and one page of the documentation site (website/docs/) is
# code, with the requirements named: host-requirements-check reads that Markdown file as data,
# so it is code, not prose. In a git repository under .out/gate-tests/<name>/, a base tree
# holds the path list of this repository (.github/planning-paths.txt), a crate file, the host
# interface's requirements, a page of the site and a planning file; the head changes the
# requirements and the page. The classifier prints code and writes kind=code, count=2 and
# site=true to GITHUB_OUTPUT. No commit is made: the revisions are trees.
# Run from the repository root: sh tests/gate/classify-changes-finds-code-in-a-site-page-and-the-host-interface.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=classify-changes-finds-code-in-a-site-page-and-the-host-interface
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
put docs/design/host-interface.md '# Host interface'
put website/docs/index.md '# swell'
put .engineering/planning/story-a.md 'status: draft'
base=$(git -C "$repo" write-tree) || fail "cannot write the base tree"

put docs/design/host-interface.md '# Host interface, changed'
put website/docs/index.md '# swell, changed'
head=$(git -C "$repo" write-tree) || fail "cannot write the head tree"

: > "$dir/github-output"
GITHUB_OUTPUT=$dir/github-output sh "$classifier" -C "$repo" "$base" "$head" > "$dir/code.out" 2>&1 ||
  fail "the classifier exited non-zero ($dir/code.out)"
grep -qxF "classify-changes: code: 2 changed file(s), 1 of them code, the first docs/design/host-interface.md" "$dir/code.out" ||
  fail "the classifier did not print code for the requirements ($dir/code.out)"
printf 'kind=code\ncount=2\nsite=true\n' | cmp -s - "$dir/github-output" ||
  fail "the classifier did not write kind=code, count=2, site=true ($dir/github-output)"

echo "$name: ok"

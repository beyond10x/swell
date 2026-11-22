#!/bin/sh
# Planted change for the classifier of changed files (taskfiles/classify-changes.sh, story
# local-gate-equals-forge, point 3). In a git repository under .out/gate-tests/<name>/, a base
# tree holds the path list of this repository (.github/planning-paths.txt), a crate file, the
# design and a plan file. Against it:
#   1. the head that changes one .engineering/ file, adds one docs/plan/*.md file and changes
#      the crate file is code, with the crate file named;
#   2. a head that renames the crate file to a docs/plan/*.md file is code: the deleted crate
#      file counts, not only the new name;
#   3. a head that adds "prose crates/*" to the path list and changes the crate file is code:
#      the list is read from the base, and a change of the list is code itself.
# Each writes kind=code to GITHUB_OUTPUT. No commit is made: the revisions are trees.
# Run from the repository root: sh tests/gate/classify-changes-finds-code-in-a-crate-file.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=classify-changes-finds-code-in-a-crate-file
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
list() {
  mkdir -p "$repo/.github" && cp .github/planning-paths.txt "$repo/.github/planning-paths.txt" &&
    git -C "$repo" update-index --add -- .github/planning-paths.txt || fail "cannot copy the path list"
}
base_tree() {
  rm -rf "$repo/crates" "$repo/docs" "$repo/.engineering" "$repo/.github"
  git -C "$repo" read-tree --empty || fail "cannot empty the index"
  list
  put crates/swell/src/lib.rs '#![forbid(unsafe_code)]'
  put docs/design/engine.md '# Design'
  put docs/plan/waves.md '# Waves'
  put .engineering/planning/story-a.md 'status: draft'
}
# classify <label> <base> <head> <expected line>
classify() {
  : > "$dir/$1.github-output"
  GITHUB_OUTPUT=$dir/$1.github-output sh "$classifier" -C "$repo" "$2" "$3" > "$dir/$1.out" 2>&1 ||
    fail "the classifier exited non-zero ($dir/$1.out)"
  grep -qxF "$4" "$dir/$1.out" || fail "the classifier did not print \"$4\" ($dir/$1.out)"
  grep -qx 'kind=code' "$dir/$1.github-output" || fail "the classifier did not write kind=code ($dir/$1.github-output)"
}

base_tree
base=$(git -C "$repo" write-tree) || fail "cannot write the base tree"
put .engineering/planning/story-a.md 'status: active'
put docs/plan/wave-9.md '# Wave 9'
put crates/swell/src/lib.rs '#![forbid(unsafe_code)] // changed'
head=$(git -C "$repo" write-tree) || fail "cannot write the head tree"
classify crate "$base" "$head" \
  "classify-changes: code: 3 changed file(s), 1 of them code, the first crates/swell/src/lib.rs"

base_tree
git -C "$repo" update-index --force-remove -- crates/swell/src/lib.rs || fail "cannot remove the crate file"
put docs/plan/lib.md '#![forbid(unsafe_code)]'
head=$(git -C "$repo" write-tree) || fail "cannot write the renaming tree"
classify rename "$base" "$head" \
  "classify-changes: code: 2 changed file(s), 1 of them code, the first crates/swell/src/lib.rs"

base_tree
printf 'prose crates/*\n' >> "$repo/.github/planning-paths.txt" &&
  git -C "$repo" update-index --add -- .github/planning-paths.txt || fail "cannot plant the list"
put crates/swell/src/lib.rs '#![forbid(unsafe_code)] // changed'
head=$(git -C "$repo" write-tree) || fail "cannot write the tree with the changed list"
classify list "$base" "$head" \
  "classify-changes: code: 2 changed file(s), 2 of them code, the first .github/planning-paths.txt"

echo "$name: ok"

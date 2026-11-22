#!/bin/sh
# Planted change for the classifier of changed files (taskfiles/classify-changes.sh):
# CHANGELOG.md is a Markdown file a gate step reads as data (host-surface-check reads the names
# under "### Host interface" of [Unreleased]), so a change of it alone is code, while another
# Markdown file alone stays planning-only. The classifier reads this repository's path list
# (.github/planning-paths.txt) and a file of changed paths under .out/gate-tests/<name>/:
#   1. docs/plan/waves.md alone is planning-only (the *.md line holds for it);
#   2. CHANGELOG.md alone is code.
# Run from the repository root: sh tests/gate/planning-only-classifier-counts-the-changelog-as-code.sh
# The same output on any machine: no colour, and no "::error" line that go-task adds under
# GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=planning-only-classifier-counts-the-changelog-as-code
work=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$work" && mkdir -p "$work" || fail "cannot make $work"

# classify <label> <path> <expected line>
classify() {
  printf '%s\n' "$2" >"$work/$1.paths" || fail "cannot write $work/$1.paths"
  sh taskfiles/classify-changes.sh -l .github/planning-paths.txt -f "$work/$1.paths" >"$work/$1.out" 2>&1 ||
    fail "the classifier exited non-zero for $2: $(cat "$work/$1.out")"
  [ "$(cat "$work/$1.out")" = "$3" ] || fail "for $2 the classifier printed: $(cat "$work/$1.out")"
}

classify plan docs/plan/waves.md \
  'classify-changes: planning-only: 1 changed file(s), each a planning or prose file'
classify changelog CHANGELOG.md \
  'classify-changes: code: 1 changed file(s), 1 of them code, the first CHANGELOG.md'

echo "$name: ok"

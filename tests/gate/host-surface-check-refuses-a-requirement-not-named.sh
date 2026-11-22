#!/bin/sh
# Planted defect for host-surface-check (taskfiles/host-surface.yml): a new HOST-<n>
# requirement the changelog does not name. A name is found only as the whole token in
# backticks: HOST-999 does not name HOST-9999. On a copy of the tree's host interface under
# .out/gate-tests/<name>/ (Cargo.toml, crates/swell/src/value.rs, crates/swell/src/host/*.rs,
# docs/design/host-interface.md) a snapshot is taken with host-surface-snapshot, and the copy
# passes host-surface-check unchanged. Then the document gets a line **HOST-9999**, and:
#   1. a changelog that names only `HOST-999` under "### Host interface" of [Unreleased] is
#      refused with the one line of the name HOST-9999;
#   2. one that names `HOST-9999` there passes, with one change.
# Run from the repository root: sh tests/gate/host-surface-check-refuses-a-requirement-not-named.sh
# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=host-surface-check-refuses-a-requirement-not-named
work=.out/gate-tests/$name
tree=$work/tree
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$work" && mkdir -p "$work" || fail "cannot make $work"
[ -f taskfiles/host-surface.yml ] || fail "no task: taskfiles/host-surface.yml does not exist"
mkdir -p "$tree/crates/swell/src/host" "$tree/docs/design" &&
  cp Cargo.toml "$tree/Cargo.toml" &&
  cp crates/swell/src/value.rs "$tree/crates/swell/src/value.rs" &&
  cp crates/swell/src/host/*.rs "$tree/crates/swell/src/host/" &&
  cp docs/design/host-interface.md "$tree/docs/design/host-interface.md" ||
  fail "cannot copy the host interface to $tree"

# surface <task> <log> [<VAR>=<value> ...]: a task of taskfiles/host-surface.yml on the copy.
surface() {
  t=$1
  log=$2
  shift 2
  task -d . -t taskfiles/host-surface.yml "$t" HOST_SURFACE_ROOT="$tree" \
    HOST_SURFACE_SNAPSHOT="$work/surface.txt" HOST_SURFACE_OUT="$work/out" "$@" >"$log" 2>&1
}
# changelog <file> <heading> <name>: [Unreleased] with <name> in backticks under <heading>, or
# empty when <heading> is empty.
changelog() {
  {
    printf '%s\n' '# Changelog' '' '## [Unreleased]' ''
    if [ -n "$2" ]; then printf '%s\n' "$2" '' "- \`$3\`: planted." ''; fi
    printf '%s\n' '## [0.2.0] - 2026-11-21' '' '### Host interface' '' '- None.'
  } >"$1"
}

surface host-surface-snapshot "$work/snapshot.log" || fail "host-surface-snapshot failed: $(cat "$work/snapshot.log")"
version=$(sed -n '1s/^# swell \([^ ]*\)$/\1/p' "$work/surface.txt")
[ -n "$version" ] || fail "the snapshot does not start with \"# swell <version>\": $(head -n 1 "$work/surface.txt")"
changelog "$work/none.md" '' ''
surface host-surface-check "$work/unchanged.log" HOST_SURFACE_CHANGELOG="$work/none.md" ||
  fail "the unchanged copy is refused: $(cat "$work/unchanged.log")"
[ "$(cat "$work/unchanged.log")" = "host-surface: 0 change(s) since $version, each named" ] ||
  fail "the unchanged copy printed: $(cat "$work/unchanged.log")"

doc=$tree/docs/design/host-interface.md
grep -q '^\*\*HOST-9999\*\*' "$doc" && fail "$doc has HOST-9999 already"
printf '%s\n' '' '**HOST-9999** (swell:D1) A requirement planted by a test of the gate. [slice 1]' >>"$doc" ||
  fail "cannot add the requirement to $doc"

changelog "$work/near.md" '### Host interface' 'HOST-999'
if surface host-surface-check "$work/near.log" HOST_SURFACE_CHANGELOG="$work/near.md"; then
  fail "the check exited 0 with the new requirement HOST-9999 not named"
fi
lines=$(grep ' changed since ' "$work/near.log")
case $lines in
  "host-surface: HOST-9999 changed since $version and [Unreleased] does not name it (name form: docs/releasing.md)"*) ;;
  *) fail "expected one line for HOST-9999, got: $(cat "$work/near.log")" ;;
esac
[ "$(printf '%s\n' "$lines" | wc -l)" -eq 1 ] || fail "expected one name, got: $lines"

changelog "$work/named.md" '### Host interface' 'HOST-9999'
surface host-surface-check "$work/named.log" HOST_SURFACE_CHANGELOG="$work/named.md" ||
  fail "the check refused the requirement named \`HOST-9999\`: $(cat "$work/named.log")"
[ "$(cat "$work/named.log")" = "host-surface: 1 change(s) since $version, each named" ] ||
  fail "the named requirement printed: $(cat "$work/named.log")"

echo "$name: ok"

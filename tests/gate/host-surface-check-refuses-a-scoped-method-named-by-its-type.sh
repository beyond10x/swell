#!/bin/sh
# Planted defect for host-surface-check (taskfiles/host-surface.yml): a changed method of
# scoped_methods!, named with one of the two types that have it. Cx and HookCx both get the
# macro's methods, so the name form (docs/releasing.md) names such a method once, bare. On a
# copy of the tree's host interface under .out/gate-tests/<name>/ (Cargo.toml,
# crates/swell/src/value.rs, crates/swell/src/host/*.rs, docs/design/host-interface.md) a
# snapshot is taken with host-surface-snapshot, and the copy passes host-surface-check
# unchanged. Then the scoped get gets a type parameter, and:
#   1. a changelog that names it `Cx::get` under "### Host interface" of [Unreleased] is
#      refused with the one line of the name get;
#   2. one that names it `get` there passes, with one change.
# Run from the repository root: sh tests/gate/host-surface-check-refuses-a-scoped-method-named-by-its-type.sh
# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=host-surface-check-refuses-a-scoped-method-named-by-its-type
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
# edit <file> <sed program> <pattern the result holds once>
edit() {
  sed "$2" "$1" >"$1.new" && mv "$1.new" "$1" || fail "cannot edit $1"
  [ "$(grep -c -F -- "$3" "$1")" = 1 ] || fail "$1 does not hold $3 once after the edit"
}

surface host-surface-snapshot "$work/snapshot.log" || fail "host-surface-snapshot failed: $(cat "$work/snapshot.log")"
version=$(sed -n '1s/^# swell \([^ ]*\)$/\1/p' "$work/surface.txt")
[ -n "$version" ] || fail "the snapshot does not start with \"# swell <version>\": $(head -n 1 "$work/surface.txt")"
changelog "$work/none.md" '' ''
surface host-surface-check "$work/unchanged.log" HOST_SURFACE_CHANGELOG="$work/none.md" ||
  fail "the unchanged copy is refused: $(cat "$work/unchanged.log")"
[ "$(cat "$work/unchanged.log")" = "host-surface: 0 change(s) since $version, each named" ] ||
  fail "the unchanged copy printed: $(cat "$work/unchanged.log")"

edit "$tree/crates/swell/src/host/mod.rs" 's/pub fn get(/pub fn get<T>(/' 'pub fn get<T>('

changelog "$work/typed.md" '### Host interface' 'Cx::get'
if surface host-surface-check "$work/typed.log" HOST_SURFACE_CHANGELOG="$work/typed.md"; then
  fail "the check exited 0 with the scoped method named \`Cx::get\`"
fi
lines=$(grep ' changed since ' "$work/typed.log")
case $lines in
  "host-surface: get changed since $version and [Unreleased] does not name it (name form: docs/releasing.md)"*) ;;
  *) fail "expected one line for get, got: $(cat "$work/typed.log")" ;;
esac
[ "$(printf '%s\n' "$lines" | wc -l)" -eq 1 ] || fail "expected one name, got: $lines"

changelog "$work/bare.md" '### Host interface' 'get'
surface host-surface-check "$work/bare.log" HOST_SURFACE_CHANGELOG="$work/bare.md" ||
  fail "the check refused the scoped method named \`get\`: $(cat "$work/bare.log")"
[ "$(cat "$work/bare.log")" = "host-surface: 1 change(s) since $version, each named" ] ||
  fail "the bare name printed: $(cat "$work/bare.log")"

echo "$name: ok"

#!/bin/sh
# Planted defect for host-surface-check (taskfiles/host-surface.yml): a removed variant named
# bare. The name form (docs/releasing.md) names a variant as Type::name. On a copy of the
# tree's host interface under .out/gate-tests/<name>/ (Cargo.toml, crates/swell/src/value.rs,
# crates/swell/src/host/*.rs, docs/design/host-interface.md) a snapshot is taken with
# host-surface-snapshot, and the copy passes host-surface-check unchanged. Then the variant
# EvaluationInProgress of HostError is removed, and:
#   1. a changelog that names it `EvaluationInProgress` under "### Host interface" of
#      [Unreleased] is refused with the one line of the name HostError::EvaluationInProgress;
#   2. one that names it `HostError::EvaluationInProgress` there passes, with one change.
# Run from the repository root: sh tests/gate/host-surface-check-refuses-a-variant-named-bare.sh
# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=host-surface-check-refuses-a-variant-named-bare
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

outcome=$tree/crates/swell/src/host/outcome.rs
[ "$(grep -c '^ *EvaluationInProgress,$' "$outcome")" = 1 ] || fail "$outcome has no line EvaluationInProgress,"
grep -v '^ *EvaluationInProgress,$' "$outcome" >"$outcome.new" && mv "$outcome.new" "$outcome" ||
  fail "cannot remove the variant from $outcome"

changelog "$work/bare.md" '### Host interface' 'EvaluationInProgress'
if surface host-surface-check "$work/bare.log" HOST_SURFACE_CHANGELOG="$work/bare.md"; then
  fail "the check exited 0 with the variant named \`EvaluationInProgress\`"
fi
lines=$(grep ' changed since ' "$work/bare.log")
case $lines in
  "host-surface: HostError::EvaluationInProgress changed since $version and [Unreleased] does not name it (name form: docs/releasing.md)"*) ;;
  *) fail "expected one line for HostError::EvaluationInProgress, got: $(cat "$work/bare.log")" ;;
esac
[ "$(printf '%s\n' "$lines" | wc -l)" -eq 1 ] || fail "expected one name, got: $lines"

changelog "$work/typed.md" '### Host interface' 'HostError::EvaluationInProgress'
surface host-surface-check "$work/typed.log" HOST_SURFACE_CHANGELOG="$work/typed.md" ||
  fail "the check refused the variant named \`HostError::EvaluationInProgress\`: $(cat "$work/typed.log")"
[ "$(cat "$work/typed.log")" = "host-surface: 1 change(s) since $version, each named" ] ||
  fail "the typed name printed: $(cat "$work/typed.log")"

echo "$name: ok"

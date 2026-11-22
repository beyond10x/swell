#!/bin/sh
# Planted change for host-surface-check (taskfiles/host-surface.yml): a doc comment changes no
# line of the scan, so the check passes with nothing named. On a copy of the tree's host
# interface under .out/gate-tests/<name>/ (Cargo.toml, crates/swell/src/value.rs,
# crates/swell/src/host/*.rs, docs/design/host-interface.md) a snapshot is taken with
# host-surface-snapshot. Then the doc comment of Agent::host ("/// The host value.") and the
# module comment of agent.rs ("//! The agent") get more words, and with an empty [Unreleased]:
#   1. the copy differs from the tree in those two lines;
#   2. host-surface-check exits 0 and prints no change.
# Run from the repository root: sh tests/gate/host-surface-check-passes-a-changed-doc-comment.sh
# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=host-surface-check-passes-a-changed-doc-comment
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

surface host-surface-snapshot "$work/snapshot.log" || fail "host-surface-snapshot failed: $(cat "$work/snapshot.log")"
version=$(sed -n '1s/^# swell \([^ ]*\)$/\1/p' "$work/surface.txt")
[ -n "$version" ] || fail "the snapshot does not start with \"# swell <version>\": $(head -n 1 "$work/surface.txt")"

agent=$tree/crates/swell/src/host/agent.rs
sed -e 's|^    /// The host value\.$|    /// The host value, as the host gave it to Agent::new.|' \
  -e '1s|^//! The agent |//! The agent, one engine instance, |' "$agent" >"$agent.new" &&
  mv "$agent.new" "$agent" || fail "cannot edit $agent"
[ "$(diff crates/swell/src/host/agent.rs "$agent" | grep -c '^>')" = 2 ] ||
  fail "the copy of agent.rs does not differ from the tree in two lines"

printf '%s\n' '# Changelog' '' '## [Unreleased]' '' '## [0.2.0] - 2026-11-21' >"$work/none.md"
surface host-surface-check "$work/doc.log" HOST_SURFACE_CHANGELOG="$work/none.md" ||
  fail "the check refused a changed doc comment: $(cat "$work/doc.log")"
[ "$(cat "$work/doc.log")" = "host-surface: 0 change(s) since $version, each named" ] ||
  fail "a changed doc comment printed: $(cat "$work/doc.log")"

echo "$name: ok"

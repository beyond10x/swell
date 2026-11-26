#!/bin/sh
# Planted defect of the task release-targets-check (taskfiles/forge.yml, story
# releases-ship-static-binaries): the step "Check that the C runtime is linked in" keeps its
# name, its if:, its env TARGET and the words readelf -d, (NEEDED) and docker run, but can no
# longer fail a musl job: the step gets continue-on-error: true (case 1), or every line of its
# musl branch is commented out (case 2), as a session does to switch a check off for a while.
# Each must make the task exit non-zero.
# Run from the repository root: sh tests/gate/release-targets-check-refuses-a-static-check-that-cannot-fail.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=release-targets-check-refuses-a-static-check-that-cannot-fail
dir=.out/gate-tests/$name
rm -rf "$dir"
fail() {
  echo "$name: FAIL $*"
  exit 1
}
root=$dir/root
mkdir -p "$root/.github/workflows" "$root/docs" "$root/website/docs" || fail "cannot make $root"
wf=$root/.github/workflows/release-build.yml
cp docs/releasing.md "$root/docs/releasing.md" &&
  cp website/docs/embedding.md "$root/website/docs/embedding.md" || fail "cannot copy the documents"
bad=0

awk '{ print } /^      - name: Check that the C runtime is linked in$/ { print "        continue-on-error: true" }' \
  .github/workflows/release-build.yml > "$wf" || fail "cannot plant case 1"
grep -q '^        continue-on-error: true$' "$wf" || fail "case 1: the plant changed nothing"
if task -d . release-targets-check GATE_ROOT="$root" > "$dir/continue.out" 2>&1; then
  echo "$name: case 1: task release-targets-check exited 0 on a static check step with continue-on-error: true ($dir/continue.out)"
  bad=1
fi

awk '/^ *\*-linux-musl\)$/ { print; inside = 1; next }
     inside && /^ *;;$/ { inside = 0 }
     inside { match($0, /^ */); print substr($0, 1, RLENGTH) "# " substr($0, RLENGTH + 1); next }
     { print }' .github/workflows/release-build.yml > "$wf" || fail "cannot plant case 2"
grep -q '^ *readelf -d' "$wf" && fail "case 2: readelf -d still runs in the planted copy"
if task -d . release-targets-check GATE_ROOT="$root" > "$dir/commented.out" 2>&1; then
  echo "$name: case 2: task release-targets-check exited 0 on a musl branch whose every line is a comment ($dir/commented.out)"
  bad=1
fi

[ "$bad" = 0 ] || exit 1
echo "$name: ok"

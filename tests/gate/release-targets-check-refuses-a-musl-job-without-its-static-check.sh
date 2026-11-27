#!/bin/sh
# Planted defect of the task release-targets-check (taskfiles/forge.yml, story
# releases-ship-static-binaries): the story's change is that the smoke test of a musl job also
# checks that the binary has no dynamic section entry NEEDED, and the Windows job that its
# import table names no VCRUNTIME. Here the workflow keeps its seven targets and the flag
# target-feature=+crt-static, and loses the step "Check that the C runtime is linked in"
# (case 1), or keeps it with an if: that no target of the matrix meets (case 2). Each makes
# the task exit non-zero.
# Run from the repository root: sh tests/gate/release-targets-check-refuses-a-musl-job-without-its-static-check.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=release-targets-check-refuses-a-musl-job-without-its-static-check
dir=.out/gate-tests/$name
rm -rf "$dir"
fail() {
  echo "$name: FAIL $*"
  exit 1
}
root=$dir/root
mkdir -p "$root/.github/workflows" "$root/docs" "$root/website/docs" || fail "cannot make $root"
wf=$root/.github/workflows/release-build.yml
cp .github/workflows/release-build.yml "$wf" &&
  cp docs/releasing.md "$root/docs/releasing.md" &&
  cp website/docs/embedding.md "$root/website/docs/embedding.md" || fail "cannot copy the workflow and the documents"
bad=0

awk '/^      - name: Check that the C runtime is linked in$/ { skip = 1 }
     /^      - uses: actions\/upload-artifact/ { skip = 0 }
     !skip { print }' .github/workflows/release-build.yml > "$wf" || fail "cannot plant case 1"
grep -q 'readelf' "$wf" && fail "case 1: the planted copy still runs readelf"
if task -d . release-targets-check GATE_ROOT="$root" > "$dir/no-step.out" 2>&1; then
  echo "$name: case 1: task release-targets-check exited 0 on a workflow without the step that runs readelf and dumpbin ($dir/no-step.out)"
  bad=1
fi

sed "s/^        if: endsWith(matrix.target, '-linux-musl') || endsWith(matrix.target, '-windows-msvc')\$/        if: endsWith(matrix.target, '-linux-muslx')/" \
  .github/workflows/release-build.yml > "$wf" || fail "cannot plant case 2"
cmp -s "$wf" .github/workflows/release-build.yml && fail "case 2: the plant changed nothing"
if task -d . release-targets-check GATE_ROOT="$root" > "$dir/dead-if.out" 2>&1; then
  echo "$name: case 2: task release-targets-check exited 0 on a static check step that no target runs ($dir/dead-if.out)"
  bad=1
fi

[ "$bad" = 0 ] || exit 1
echo "$name: ok"

#!/bin/sh
# Planted defect of the task release-targets-check (taskfiles/forge.yml, story
# releases-ship-static-binaries): the task runs the workflow's Build step in bash with cargo
# stubbed, and says it builds nothing and works in .out/release-targets-check.<random>, which
# it removes. Every other command of the Build step runs for real on the machine of the gate,
# with its PATH: a line that installs a package, fetches a file or writes a file is carried
# out there. Here the Build step gets one line that writes a marker file into the working
# directory, the way "sudo apt-get install -y musl-tools", "rustup target add" or "curl" would
# act. The task must not leave the marker.
# Run from the repository root: sh tests/gate/release-targets-check-runs-nothing-of-the-build-step-but-the-stub.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=release-targets-check-runs-nothing-of-the-build-step-but-the-stub
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
marker=$dir/marker
awk -v marker="$marker" '/^      - name: Build swell-cli$/ { inbuild = 1 }
  { print }
  inbuild && /^          set -euo pipefail$/ { print "          printf \"%s\\n\" \"$TARGET\" >> " marker; inbuild = 0 }' \
  .github/workflows/release-build.yml > "$wf" || fail "cannot plant the line"
grep -qF ">> $marker" "$wf" || fail "the plant changed nothing"
task -d . release-targets-check GATE_ROOT="$root" > "$dir/run.out" 2>&1
status=$?
if [ -e "$marker" ]; then
  echo "$name: task release-targets-check (exit $status) ran a line of the Build step other than cargo: $marker holds $(tr '\n' ' ' < "$marker")"
  exit 1
fi
echo "$name: ok"

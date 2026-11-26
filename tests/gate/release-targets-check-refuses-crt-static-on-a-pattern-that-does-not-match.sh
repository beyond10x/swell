#!/bin/sh
# Planted defect of the task release-targets-check (taskfiles/forge.yml, story
# releases-ship-static-binaries): the Windows build takes -C target-feature=+crt-static. Here
# the line keeps both words, windows-msvc and target-feature=+crt-static, but its case pattern
# loses "*-" and so no target of the matrix meets it: x86_64-pc-windows-msvc builds with the
# dynamic C runtime. The task must exit non-zero.
# Run from the repository root: sh tests/gate/release-targets-check-refuses-crt-static-on-a-pattern-that-does-not-match.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GITHUB_OUTPUT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=release-targets-check-refuses-crt-static-on-a-pattern-that-does-not-match
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
sed 's/^\( *\)\*-windows-msvc | \*-linux-musl) flags="\$flags -C target-feature=+crt-static" ;;$/\1windows-msvc | *-linux-musl) flags="$flags -C target-feature=+crt-static" ;;/' \
  .github/workflows/release-build.yml > "$wf" || fail "cannot plant the pattern"
cmp -s "$wf" .github/workflows/release-build.yml && fail "the plant changed nothing"
# The planted pattern does not apply to the Windows target: the shell of the Build step says so.
applies=no
case x86_64-pc-windows-msvc in windows-msvc | *-linux-musl) applies=yes ;; esac
[ "$applies" = no ] || fail "the planted pattern still meets x86_64-pc-windows-msvc"
if task -d . release-targets-check GATE_ROOT="$root" > "$dir/pattern.out" 2>&1; then
  echo "$name: task release-targets-check exited 0 on a workflow whose +crt-static meets no Windows target ($dir/pattern.out)"
  exit 1
fi
echo "$name: ok"

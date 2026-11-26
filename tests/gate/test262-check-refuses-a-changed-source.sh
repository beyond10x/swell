#!/bin/sh
# Planted defect of the task test262-check (taskfiles/test262.yml, story
# gate-test-steps-within-budget, point 8): a source of conformance/test262/expected.txt changed
# without the digest beside it is refused.
# The digest file is made here from the task test262-digest on copies of the design, the feature
# list and the sections file, so that the script does not depend on the committed digest. With
# the unchanged copies test262-check passes and prints that the digest is equal; then
#   1. a copy of the design with one rule line more in its swell-slices block, and
#   2. a copy of features.txt with one feature excluded
# each make it exit non-zero and print "test262-check: the sources digest <new> differs from
# <old> in <digest file>". Then, on a small tree of its own (GATE_ROOT), 3. a changed file under
# crates/swell/tests/ leaves the digest equal and test262-check passes, and 4. a changed file
# under crates/swell/src/ makes it fail the same way: the digest covers the sources, not the
# tests.
# Run from the repository root: sh tests/gate/test262-check-refuses-a-changed-source.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test262-check-refuses-a-changed-source
dir=.out/gate-tests/$name
feature=generators
rm -rf "$dir"
mkdir -p "$dir"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

cp docs/design/engine.md "$dir/engine.md" || fail "cannot copy the design"
cp conformance/test262/features.txt "$dir/features.txt" || fail "cannot copy features.txt"
cp conformance/test262/sections.txt "$dir/sections.txt" || fail "cannot copy sections.txt"
copies="TEST262_DESIGN=$dir/engine.md TEST262_FEATURES=$dir/features.txt TEST262_SECTIONS=$dir/sections.txt"

# shellcheck disable=SC2086
task -d . test262-digest $copies > "$dir/digest.out" 2>&1 ||
  fail "task test262-digest exited non-zero ($dir/digest.out)"
digest=$(awk '$1 == "test262-sources" { print $2 }' "$dir/digest.out")
[ -n "$digest" ] || fail "task test262-digest printed no test262-sources line ($dir/digest.out)"
printf 'test262-sources %s\ntest262-refused 0\n' "$digest" > "$dir/expected.digest"

# shellcheck disable=SC2086
task -d . test262-check $copies TEST262_DIGEST="$dir/expected.digest" > "$dir/base.out" 2>&1 ||
  fail "task test262-check on the unchanged copies exited non-zero ($dir/base.out)"
grep -qF "digest $digest equals $dir/expected.digest" "$dir/base.out" ||
  fail "task test262-check on the unchanged copies printed no equal digest line ($dir/base.out)"

awk '{ print } $0 == "```swell-slices" { print "dir test/planted/ 3" }' docs/design/engine.md > "$dir/engine.md" ||
  fail "cannot write the planted design"
cmp -s docs/design/engine.md "$dir/engine.md" && fail "the planted design equals the design"
# shellcheck disable=SC2086
if task -d . test262-check $copies TEST262_DIGEST="$dir/expected.digest" > "$dir/design.out" 2>&1; then
  fail "task test262-check exited 0 with a changed swell-slices block ($dir/design.out)"
fi
grep -qE "^test262-check: the sources digest [0-9a-f]{64} differs from $digest in $dir/expected.digest" "$dir/design.out" ||
  fail "task test262-check did not name both digests for the changed design ($dir/design.out)"
cp docs/design/engine.md "$dir/engine.md" || fail "cannot copy the design again"

grep -qx "run $feature" "$dir/features.txt" || fail "features.txt has no line \"run $feature\""
awk -v feature="$feature" '$0 == "run " feature { $0 = "exclude " feature " planted decision-1" } { print }' \
  conformance/test262/features.txt > "$dir/features.txt" || fail "cannot write the planted features.txt"
# shellcheck disable=SC2086
if task -d . test262-check $copies TEST262_DIGEST="$dir/expected.digest" > "$dir/features.out" 2>&1; then
  fail "task test262-check exited 0 with a changed features.txt ($dir/features.out)"
fi
grep -qE "^test262-check: the sources digest [0-9a-f]{64} differs from $digest in $dir/expected.digest" "$dir/features.out" ||
  fail "task test262-check did not name both digests for the changed features.txt ($dir/features.out)"
cp conformance/test262/features.txt "$dir/features.txt" || fail "cannot copy features.txt again"

# A tree of its own (GATE_ROOT, a git repository whose files are untracked, which the digest
# lists as well): a changed test of a crate leaves the digest equal, a changed source does not.
root=$dir/root
mkdir -p "$root/crates/swell/src" "$root/crates/swell/tests" || fail "cannot make $root"
git -C "$root" init -q || fail "git init failed in $root"
printf '[workspace]\nmembers = ["crates/swell"]\n' > "$root/Cargo.toml"
printf '[package]\nname = "swell"\n' > "$root/crates/swell/Cargo.toml"
printf 'version = 4\n\n[[package]]\nname = "swell"\nversion = "0.1.0"\n\n[[package]]\nname = "swell-test262"\nversion = "0.1.0"\ndependencies = [\n "swell",\n]\n' > "$root/Cargo.lock"
printf 'pub fn answer() -> u32 {\n    42\n}\n' > "$root/crates/swell/src/lib.rs"
printf '#[test]\nfn answers() {}\n' > "$root/crates/swell/tests/answer.rs"
rooted="GATE_ROOT=$root $copies"
# shellcheck disable=SC2086
task -d . test262-digest $rooted > "$dir/root-digest.out" 2>&1 ||
  fail "task test262-digest on $root exited non-zero ($dir/root-digest.out)"
rdigest=$(awk '$1 == "test262-sources" { print $2 }' "$dir/root-digest.out")
[ -n "$rdigest" ] || fail "task test262-digest printed no test262-sources line for $root ($dir/root-digest.out)"
printf 'test262-sources %s\ntest262-refused 0\n' "$rdigest" > "$dir/root.digest"
printf '// changed\n' >> "$root/crates/swell/tests/answer.rs"
# shellcheck disable=SC2086
task -d . test262-check $rooted TEST262_DIGEST="$dir/root.digest" > "$dir/test-file.out" 2>&1 ||
  fail "task test262-check exited non-zero with a changed file under crates/swell/tests/ ($dir/test-file.out)"
grep -qF "digest $rdigest equals $dir/root.digest" "$dir/test-file.out" ||
  fail "task test262-check printed no equal digest line after a changed test ($dir/test-file.out)"
printf '// changed\n' >> "$root/crates/swell/src/lib.rs"
# shellcheck disable=SC2086
if task -d . test262-check $rooted TEST262_DIGEST="$dir/root.digest" > "$dir/src-file.out" 2>&1; then
  fail "task test262-check exited 0 with a changed file under crates/swell/src/ ($dir/src-file.out)"
fi
grep -qE "^test262-check: the sources digest [0-9a-f]{64} differs from $rdigest in $dir/root.digest" "$dir/src-file.out" ||
  fail "task test262-check did not name both digests for the changed source ($dir/src-file.out)"

echo "$name: ok"

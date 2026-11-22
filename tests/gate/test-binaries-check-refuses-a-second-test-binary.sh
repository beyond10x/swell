#!/bin/sh
# Planted defect of the task test-binaries-check (taskfiles/test-binaries.yml; story
# one-integration-test-binary-per-crate, point 2): a package with two integration-test binaries
# is refused and named. Nothing is built: the script copies the machine's true binary, which
# carries no debug section, and writes files of cargo's JSON messages that name the copies, which
# the task reads from GATE_TEST_BUILD_JSON in place of a build. The layout check the task runs
# first reads a planted crate, GATE_CRATES=<dir>/crates, whose tests/main.rs is written by
# taskfiles/test-main.sh, so that it passes and the refusal comes from the count.
#   1. One integration-test executable (target kind test) of the package planted and one unit-test
#      executable of its library: the task passes and prints "test-binaries-check: 2 test
#      executables, 1 integration-test binaries in 1 packages, none with a .debug_ section".
#   2. Two integration-test executables of the package planted, first and second: the task fails
#      and prints "test-binaries-check: package planted has 2 integration-test binaries: first,
#      second".
# Run from the repository root:
#   sh tests/gate/test-binaries-check-refuses-a-second-test-binary.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-binaries-check-refuses-a-second-test-binary
dir=.out/gate-tests/$name
taskfile=taskfiles/test-binaries.yml
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/crates/planted/src" "$dir/crates/planted/tests" || fail "cannot make $dir"
[ -f "$taskfile" ] || fail "$taskfile is missing"

# The planted crate: one library, one test file, the manifest of one test binary.
crate=$dir/crates/planted
printf '[package]\nname = "planted"\nversion = "0.0.0"\nedition = "2024"\nautotests = false\n\n[[test]]\nname = "main"\npath = "tests/main.rs"\n' \
  > "$crate/Cargo.toml" || fail "cannot write $crate/Cargo.toml"
printf '#![forbid(unsafe_code)]\n' > "$crate/src/lib.rs" || fail "cannot write $crate/src/lib.rs"
printf '#[test]\nfn one() {}\n' > "$crate/tests/first.rs" || fail "cannot write $crate/tests/first.rs"
# Without the generator the layout check of the task fails too; the count is what this script
# shows, so the crate is generated when the generator is there.
if [ -f taskfiles/test-main.sh ]; then
  GATE_CRATES=$dir/crates sh taskfiles/test-main.sh write > "$dir/write.out" 2>&1 ||
    fail "taskfiles/test-main.sh write failed on the planted crate ($dir/write.out)"
fi

true_bin=""
for candidate in /usr/bin/true /bin/true; do
  if [ -f "$candidate" ]; then true_bin=$candidate; break; fi
done
[ -n "$true_bin" ] || fail "no true binary in /usr/bin or /bin to copy"
for copy in first second unit; do
  cp "$true_bin" "$dir/$copy" || fail "cannot copy $true_bin to $dir/$copy"
done

# One message of cargo's JSON output for a test executable $1 of the target $2 of kind $3.
message() {
  printf '{"reason":"compiler-artifact","package_id":"path+file:///planted#planted@0.0.0","target":{"kind":["%s"],"name":"%s"},"profile":{"opt_level":"0","debuginfo":0,"test":true},"filenames":[],"executable":"%s","fresh":true}\n' \
    "$3" "$2" "$1"
}

{
  message "$dir/first" first test
  message "$dir/unit" planted lib
  printf '{"reason":"build-finished","success":true}\n'
} > "$dir/one.json" || fail "cannot write $dir/one.json"
task --taskfile "$taskfile" test-binaries-check GATE_CRATES="$dir/crates" GATE_TEST_BUILD_JSON="$dir/one.json" > "$dir/one.out" 2>&1 ||
  fail "test-binaries-check refused a package with one integration-test binary ($dir/one.out)"
grep -qxF "test-binaries-check: 2 test executables, 1 integration-test binaries in 1 packages, none with a .debug_ section" "$dir/one.out" ||
  fail "test-binaries-check did not count 2 test executables and 1 integration-test binary in 1 package ($dir/one.out)"

{
  message "$dir/first" first test
  message "$dir/second" second test
  printf '{"reason":"build-finished","success":true}\n'
} > "$dir/two.json" || fail "cannot write $dir/two.json"
if task --taskfile "$taskfile" test-binaries-check GATE_CRATES="$dir/crates" GATE_TEST_BUILD_JSON="$dir/two.json" > "$dir/two.out" 2>&1; then
  fail "test-binaries-check accepted a package with two integration-test binaries ($dir/two.out)"
fi
grep -qxF "test-binaries-check: package planted has 2 integration-test binaries: first, second" "$dir/two.out" ||
  fail "test-binaries-check did not name the package planted and its two integration-test binaries ($dir/two.out)"

echo "$name: ok"

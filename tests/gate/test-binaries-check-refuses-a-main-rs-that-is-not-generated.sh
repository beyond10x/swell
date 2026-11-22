#!/bin/sh
# Planted defect of the task test-binaries-check (taskfiles/test-binaries.yml) and of its layout
# check, taskfiles/test-main.sh check (story one-integration-test-binary-per-crate, point 1): a
# crate whose tests are not one generated integration-test binary is refused, naming the file.
# Nothing is built: the task reads GATE_CRATES=<dir>/crates, a planted crate with a library, a
# binary target and two test files, and GATE_TEST_BUILD_JSON, a file of cargo's JSON messages
# naming a copy of the machine's true binary as its one integration-test executable.
#   1. The planted crate as taskfiles/test-main.sh write generates it: the task passes and prints
#      "test-binaries-check: 1 test executables, 1 integration-test binaries in 1 packages, none
#      with a .debug_ section".
#   2. A third test file that tests/main.rs does not declare: refused with "test-main:
#      <crate>/tests/main.rs differs from its generation; run sh taskfiles/test-main.sh write".
#   3. The generated tree with a manifest whose [package] lacks autotests = false: refused with
#      "test-main: <crate>/Cargo.toml: [package] lacks autotests = false; ...".
#   4. The generated tree with a test file lib.rs, generated: refused with "test-main:
#      <crate>/tests/lib.rs: a module named lib, ...".
#   5. The generated tree with a test file named as the binary target, planted_tool.rs for the
#      binary planted-tool, generated: refused with "test-main: <crate>/tests/planted_tool.rs: a
#      module named planted_tool, ...".
#   In 2 to 5 the task also prints "test-binaries-check: the test layout differs from its
#   generation (taskfiles/test-main.sh check, above)", and the generated tree is restored after
#   each.
# Run from the repository root:
#   sh tests/gate/test-binaries-check-refuses-a-main-rs-that-is-not-generated.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-binaries-check-refuses-a-main-rs-that-is-not-generated
dir=.out/gate-tests/$name
taskfile=taskfiles/test-binaries.yml
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/crates/planted/src/bin" "$dir/crates/planted/tests" || fail "cannot make $dir"
[ -f "$taskfile" ] || fail "$taskfile is missing"
[ -f taskfiles/test-main.sh ] || fail "taskfiles/test-main.sh is missing"

crate=$dir/crates/planted
manifest=$crate/Cargo.toml
printf '[package]\nname = "planted"\nversion = "0.0.0"\nedition = "2024"\nautotests = false\n\n[[test]]\nname = "main"\npath = "tests/main.rs"\n' \
  > "$manifest" || fail "cannot write $manifest"
printf '#![forbid(unsafe_code)]\n' > "$crate/src/lib.rs" || fail "cannot write $crate/src/lib.rs"
printf '#![forbid(unsafe_code)]\nfn main() {}\n' > "$crate/src/bin/planted-tool.rs" || fail "cannot write the planted binary"
printf '#[test]\nfn one() {}\n' > "$crate/tests/first.rs" || fail "cannot write $crate/tests/first.rs"
printf '#[test]\nfn two() {}\n' > "$crate/tests/second.rs" || fail "cannot write $crate/tests/second.rs"
generate() {
  GATE_CRATES=$dir/crates sh taskfiles/test-main.sh write > "$dir/write.out" 2>&1 ||
    fail "taskfiles/test-main.sh write failed on the planted crate ($dir/write.out)"
}
generate
[ -f "$crate/tests/main.rs" ] || fail "taskfiles/test-main.sh write wrote no $crate/tests/main.rs"
cp "$manifest" "$dir/Cargo.toml.generated" || fail "cannot keep the generated manifest"
cp "$crate/tests/main.rs" "$dir/main.rs.generated" || fail "cannot keep the generated tests/main.rs"

true_bin=""
for candidate in /usr/bin/true /bin/true; do
  if [ -f "$candidate" ]; then true_bin=$candidate; break; fi
done
[ -n "$true_bin" ] || fail "no true binary in /usr/bin or /bin to copy"
cp "$true_bin" "$dir/main" || fail "cannot copy $true_bin to $dir/main"
{
  printf '{"reason":"compiler-artifact","package_id":"path+file:///planted#planted@0.0.0","target":{"kind":["test"],"name":"main"},"profile":{"opt_level":"0","debuginfo":0,"test":true},"filenames":[],"executable":"%s","fresh":true}\n' "$dir/main"
  printf '{"reason":"build-finished","success":true}\n'
} > "$dir/build.json" || fail "cannot write $dir/build.json"

check() {
  task --taskfile "$taskfile" test-binaries-check GATE_CRATES="$dir/crates" GATE_TEST_BUILD_JSON="$dir/build.json" > "$dir/$1.out" 2>&1
}
restore() {
  rm -f "$crate/tests/third.rs" "$crate/tests/lib.rs" "$crate/tests/planted_tool.rs"
  cp "$dir/Cargo.toml.generated" "$manifest" && cp "$dir/main.rs.generated" "$crate/tests/main.rs" ||
    fail "cannot restore the generated tree"
}
refused() {
  if check "$1"; then fail "test-binaries-check accepted $2 ($dir/$1.out)"; fi
  grep -qF "$3" "$dir/$1.out" || fail "test-binaries-check did not print \"$3\" for $2 ($dir/$1.out)"
  grep -qxF "test-binaries-check: the test layout differs from its generation (taskfiles/test-main.sh check, above)" "$dir/$1.out" ||
    fail "test-binaries-check did not say that the layout check failed for $2 ($dir/$1.out)"
}

check generated || fail "test-binaries-check refused the generated tree ($dir/generated.out)"
grep -qxF "test-binaries-check: 1 test executables, 1 integration-test binaries in 1 packages, none with a .debug_ section" "$dir/generated.out" ||
  fail "test-binaries-check did not print its line for the generated tree ($dir/generated.out)"

printf '#[test]\nfn three() {}\n' > "$crate/tests/third.rs" || fail "cannot write $crate/tests/third.rs"
refused missing "a test file that tests/main.rs does not declare" \
  "test-main: $crate/tests/main.rs differs from its generation; run sh taskfiles/test-main.sh write"
restore

grep -v '^autotests' "$dir/Cargo.toml.generated" > "$manifest" || fail "cannot write the manifest without autotests = false"
refused autotests "a manifest without autotests = false" \
  "test-main: $manifest: [package] lacks autotests = false"
restore

printf '#[test]\nfn unit() {}\n' > "$crate/tests/lib.rs" || fail "cannot write $crate/tests/lib.rs"
generate
refused lib "a module named lib" "test-main: $crate/tests/lib.rs: a module named lib,"
restore

printf '#[test]\nfn tool() {}\n' > "$crate/tests/planted_tool.rs" || fail "cannot write $crate/tests/planted_tool.rs"
generate
refused binary "a module named as the binary target planted-tool" \
  "test-main: $crate/tests/planted_tool.rs: a module named planted_tool,"
restore

check restored || fail "test-binaries-check refused the restored generated tree ($dir/restored.out)"

echo "$name: ok"

#!/bin/sh
# Planted defect of the task test-binaries-check (taskfiles/test-binaries.yml; story
# test-builds-carry-no-debug-information, points 1 and 2): a test executable with a section
# whose name starts with .debug_ is refused and named. Nothing is built: the script copies the
# machine's true binary, which carries no debug section, adds a section .debug_info to a second
# copy with objcopy --add-section, and writes files of cargo's JSON messages that name the
# copies, which the task reads from GATE_TEST_BUILD_JSON in place of a build.
#   1. The clean copy as the one test executable, the planted copy as an executable that is no
#      test (.profile.test false) and a library without an executable: the task passes and
#      counts 1 test executable, 1 integration-test binary in 1 package.
#   2. The clean copy and the planted copy as test executables, the planted one a library's
#      unit-test executable so that the package keeps one integration-test binary: the task fails, names the
#      planted copy ("test-binaries-check: <binary> has a .debug_ section") and not the clean one.
#   3. No test executable among the messages: the task fails (a check that reads nothing fails).
# Run from the repository root:
#   sh tests/gate/test-binaries-check-refuses-a-test-binary-with-debug-sections.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-binaries-check-refuses-a-test-binary-with-debug-sections
dir=.out/gate-tests/$name
taskfile=taskfiles/test-binaries.yml
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
[ -f "$taskfile" ] || fail "$taskfile is missing"

true_bin=""
for candidate in /usr/bin/true /bin/true; do
  if [ -f "$candidate" ]; then true_bin=$candidate; break; fi
done
[ -n "$true_bin" ] || fail "no true binary in /usr/bin or /bin to copy"

clean=$dir/clean
planted=$dir/planted
cp "$true_bin" "$clean" || fail "cannot copy $true_bin to $clean"
chmod u+w "$clean" || fail "cannot make $clean writable"
if LC_ALL=C readelf -S -W "$clean" | grep -q ' \.debug_'; then
  fail "$true_bin carries debug sections already; the clean copy needs a binary without them"
fi
printf 'planted\n' > "$dir/section.bin" || fail "cannot write $dir/section.bin"
objcopy --add-section .debug_info="$dir/section.bin" "$clean" "$planted" ||
  fail "objcopy did not add a section .debug_info to a copy of $true_bin"
LC_ALL=C readelf -S -W "$planted" | grep -q ' \.debug_info ' ||
  fail "the planted copy $planted shows no section .debug_info"

# One message of cargo's JSON output: reason compiler-artifact, .profile.test $2, executable $1
# (the word null for none).
message() {
  if [ "$1" = null ]; then executable=null; else executable="\"$1\""; fi
  printf '{"reason":"compiler-artifact","package_id":"path+file:///planted#planted@0.0.0","target":{"kind":["%s"],"name":"planted"},"profile":{"opt_level":"0","debuginfo":0,"test":%s},"filenames":[],"executable":%s,"fresh":true}\n' \
    "$3" "$2" "$executable"
}

{
  message "$clean" true test
  message "$planted" false bin
  message null false lib
  printf '{"reason":"build-finished","success":true}\n'
} > "$dir/clean.json" || fail "cannot write $dir/clean.json"
task --taskfile "$taskfile" test-binaries-check GATE_TEST_BUILD_JSON="$dir/clean.json" > "$dir/clean.out" 2>&1 ||
  fail "test-binaries-check refused a clean test executable ($dir/clean.out)"
grep -qxF "test-binaries-check: 1 test executables, 1 integration-test binaries in 1 packages, none with a .debug_ section" "$dir/clean.out" ||
  fail "test-binaries-check did not count 1 test executable and 1 integration-test binary in 1 package, none with a .debug_ section ($dir/clean.out)"

{
  message "$clean" true test
  message "$planted" true lib
  printf '{"reason":"build-finished","success":true}\n'
} > "$dir/planted.json" || fail "cannot write $dir/planted.json"
if task --taskfile "$taskfile" test-binaries-check GATE_TEST_BUILD_JSON="$dir/planted.json" > "$dir/planted.out" 2>&1; then
  fail "test-binaries-check accepted $planted, which has a section .debug_info ($dir/planted.out)"
fi
grep -qxF "test-binaries-check: $planted has a .debug_ section" "$dir/planted.out" ||
  fail "test-binaries-check did not print \"test-binaries-check: $planted has a .debug_ section\" ($dir/planted.out)"
if grep -qF "test-binaries-check: $clean has a .debug_ section" "$dir/planted.out"; then
  fail "test-binaries-check named the clean copy $clean ($dir/planted.out)"
fi

{
  message "$planted" false bin
  message null false lib
  printf '{"reason":"build-finished","success":true}\n'
} > "$dir/none.json" || fail "cannot write $dir/none.json"
if task --taskfile "$taskfile" test-binaries-check GATE_TEST_BUILD_JSON="$dir/none.json" > "$dir/none.out" 2>&1; then
  fail "test-binaries-check passed with no test executable among the messages ($dir/none.out)"
fi

echo "$name: ok"

#!/bin/sh
# Planted defect of the task stack-modules-check (taskfiles/stack-profiles.yml; adversary pass 1
# on unit lanebuilds): a list given on the command line that names nothing (blanks only; an
# empty value takes the default list) is refused, so a check that reads no test does not pass.
# Nothing is built: the task reads STACK_TESTS_DIR=<dir>/tests, planted files with one test each.
#   1. STACK_TESTS of blanks: refused with "stack-modules: STACK_TESTS names no test".
#   2. STACK_MODULES of blanks: refused with "stack-modules: STACK_MODULES names no module".
#   3. Both lists given: passes with "stack-modules: 1 stack tests in the modules first of
#      STACK_MODULES".
# Run from the repository root: sh tests/gate/stack-modules-check-refuses-a-blank-list.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=stack-modules-check-refuses-a-blank-list
dir=.out/gate-tests/$name
taskfile=taskfiles/stack-profiles.yml
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/tests" || fail "cannot make $dir"
[ -f "$taskfile" ] || fail "$taskfile is missing"
printf '#[test]\nfn deep_one() {}\n' > "$dir/tests/first.rs" || fail "cannot write $dir/tests/first.rs"

check() {
  out=$1
  shift
  task --taskfile "$taskfile" stack-modules-check STACK_TESTS_DIR="$dir/tests" "$@" > "$dir/$out.out" 2>&1
}
refused() {
  out=$1
  line=$2
  shift 2
  if check "$out" "$@"; then fail "stack-modules-check accepted $out ($dir/$out.out)"; fi
  grep -qxF "$line" "$dir/$out.out" || fail "stack-modules-check did not print \"$line\" ($dir/$out.out)"
}

refused tests "stack-modules: STACK_TESTS names no test" STACK_MODULES='first' STACK_TESTS='  '
refused modules "stack-modules: STACK_MODULES names no module" STACK_MODULES='  ' STACK_TESTS='deep_one'
check both STACK_MODULES='first' STACK_TESTS='deep_one' || fail "stack-modules-check refused one test in its module ($dir/both.out)"
grep -qxF "stack-modules: 1 stack tests in the modules first of STACK_MODULES" "$dir/both.out" ||
  fail "stack-modules-check did not print its line ($dir/both.out)"

echo "$name: ok"

#!/bin/sh
# Planted defect of the task stack-modules-check (taskfiles/stack-profiles.yml; story
# realm-size-lane-within-its-share): the stack builds of the lane slow compile only the modules
# of swell's test binary that STACK_MODULES names, so a stack test of STACK_TESTS in another
# module would not be compiled, and the task refuses it, naming the test and its module. Nothing
# is built: the task reads STACK_TESTS_DIR=<dir>/tests, planted files with one test each.
#   1. STACK_MODULES "first second", STACK_TESTS "deep_one deep_two": passes, printing
#      "stack-modules: 2 stack tests in the modules first second of STACK_MODULES".
#   2. STACK_MODULES "first": refused with "stack-modules: the stack test deep_two is in the
#      module second (<dir>/tests/second.rs), which STACK_MODULES does not compile".
#   3. STACK_TESTS with a test no file holds: refused with "stack-modules: the stack test
#      deep_none is in no file <dir>/tests/<module>.rs".
#   4. A test in two files: refused with "stack-modules: the stack test deep_one is in 2 files:".
#   5. STACK_MODULES naming a module without a file: refused with "stack-modules: STACK_MODULES
#      names third, but <dir>/tests/third.rs does not exist".
#   6. The tree's own lists and crates/swell/tests: passes.
# Run from the repository root:
#   sh tests/gate/stack-modules-check-refuses-a-test-outside-the-list.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=stack-modules-check-refuses-a-test-outside-the-list
dir=.out/gate-tests/$name
taskfile=taskfiles/stack-profiles.yml
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/tests" || fail "cannot make $dir"
[ -f "$taskfile" ] || fail "$taskfile is missing"

printf '#[test]\nfn deep_one() {}\n' > "$dir/tests/first.rs" || fail "cannot write $dir/tests/first.rs"
printf '#[test]\nfn deep_two() {}\n' > "$dir/tests/second.rs" || fail "cannot write $dir/tests/second.rs"
printf 'mod first;\nmod second;\nfn deep_one() {}\n' > "$dir/tests/main.rs" || fail "cannot write $dir/tests/main.rs"

check() {
  out=$1
  shift
  task --taskfile "$taskfile" stack-modules-check STACK_TESTS_DIR="$dir/tests" "$@" > "$dir/$out.out" 2>&1
}
refused() {
  out=$1
  what=$2
  line=$3
  shift 3
  if check "$out" "$@"; then fail "stack-modules-check accepted $what ($dir/$out.out)"; fi
  grep -qF "$line" "$dir/$out.out" || fail "stack-modules-check did not print \"$line\" for $what ($dir/$out.out)"
}

check both STACK_MODULES='first second' STACK_TESTS='deep_one deep_two' ||
  fail "stack-modules-check refused two tests in the two modules of the list ($dir/both.out)"
grep -qxF "stack-modules: 2 stack tests in the modules first second of STACK_MODULES" "$dir/both.out" ||
  fail "stack-modules-check did not print its line for the planted lists ($dir/both.out)"

refused outside "a test in a module the list does not name" \
  "stack-modules: the stack test deep_two is in the module second ($dir/tests/second.rs), which STACK_MODULES does not compile" \
  STACK_MODULES='first' STACK_TESTS='deep_one deep_two'

refused none "a test that no file holds" \
  "stack-modules: the stack test deep_none is in no file $dir/tests/<module>.rs" \
  STACK_MODULES='first second' STACK_TESTS='deep_one deep_none'

printf '#[test]\nfn deep_one() {}\n' >> "$dir/tests/second.rs" || fail "cannot add a second deep_one"
refused twice "a test in two files" \
  "stack-modules: the stack test deep_one is in 2 files:" \
  STACK_MODULES='first second' STACK_TESTS='deep_one'
printf '#[test]\nfn deep_two() {}\n' > "$dir/tests/second.rs" || fail "cannot restore $dir/tests/second.rs"

refused missing "a module without a file" \
  "stack-modules: STACK_MODULES names third, but $dir/tests/third.rs does not exist" \
  STACK_MODULES='first second third' STACK_TESTS='deep_one deep_two'

task --taskfile "$taskfile" stack-modules-check > "$dir/tree.out" 2>&1 ||
  fail "stack-modules-check refused the tree's own lists ($dir/tree.out)"

echo "$name: ok"

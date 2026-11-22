#!/bin/sh
# Planted defect of the task nextest-report (taskfiles/nextest.yml): a cargo-nextest log written
# with CARGO_TERM_COLOR=always, as the forge's runner and task check-runner set it, is read as
# a plain one. Before, the escape sequences around nextest's words hid its "Starting" and
# "Summary" lines from the task, which printed "0 test binaries: 0 passed, 0 failed, 0 ignored"
# for a run of 638 tests (the gate's run on the wave branch of slice 3). The lines below are
# nextest's own, with their escape sequences, from that run; the passing log is the same lines
# with the counts of three tests.
#   1. the failing run: the task prints "68 test binaries: 637 passed, 1 failed, 1 ignored" and
#      fails;
#   2. a passing run: it prints "1 test binaries: 3 passed, 0 failed, 0 ignored", the slowest
#      tests of its JUnit file, and exits 0;
#   3. a log with no "Starting" line: it prints no count, says what it could not read, and fails.
# The coloured logs, and the reports of the failing runs, which print their log, are *.txt
# files: gate-selftest searches a script's *.log and *.out files for escape bytes, and these
# hold them on purpose.
# Run from the repository root: sh tests/gate/nextest-report-reads-a-coloured-log.sh
# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=nextest-report-reads-a-coloured-log
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

e=$(printf '\033')
pass1="${e}[32;1m        PASS${e}[0m [   0.091s] (  1/638) ${e}[35;1mswell${e}[0m ${e}[36mbuiltins::array::search::tests${e}[0m${e}[36m::${e}[0m${e}[34;1marray_prototype_unscopables_lists_the_sixteen_names${e}[0m"
pass2="${e}[32;1m        PASS${e}[0m [   0.092s] (  2/638) ${e}[35;1mswell${e}[0m ${e}[36mbuiltins::array::iteration::tests${e}[0m${e}[36m::${e}[0m${e}[34;1ma_target_that_cannot_take_an_element_throws${e}[0m"
pass3="${e}[32;1m        PASS${e}[0m [   0.096s] (  3/638) ${e}[35;1mswell${e}[0m ${e}[36mbuiltins::array::mutation::tests${e}[0m${e}[36m::${e}[0m${e}[34;1msplice_uses_species${e}[0m"
failed="${e}[31;1m        FAIL${e}[0m [   1.366s] (626/638) ${e}[35;1mswell-test262::runner${e}[0m ${e}[34;1man_unknown_rule_kind_fails_the_run${e}[0m"

{
  echo "${e}[32;1m    Starting${e}[0m ${e}[1m638${e}[0m tests across ${e}[1m68${e}[0m binaries (${e}[1m1${e}[0m test ${e}[33;1mskipped${e}[0m)"
  echo "$pass1" && echo "$pass2" && echo "$pass3" && echo "$failed"
  echo "${e}[31;1m     Summary${e}[0m [ 235.902s] ${e}[1m638${e}[0m tests run: ${e}[1m637${e}[0m ${e}[32;1mpassed${e}[0m, ${e}[1m1${e}[0m ${e}[31;1mfailed${e}[0m, ${e}[1m1${e}[0m ${e}[33;1mskipped${e}[0m"
  echo "$failed"
} > "$dir/failing.txt" || fail "cannot write the failing log"
{
  echo "${e}[32;1m    Starting${e}[0m ${e}[1m3${e}[0m tests across ${e}[1m1${e}[0m binary"
  echo "$pass1" && echo "$pass2" && echo "$pass3"
  echo "${e}[32;1m     Summary${e}[0m [   0.279s] ${e}[1m3${e}[0m tests run: ${e}[1m3${e}[0m ${e}[32;1mpassed${e}[0m, ${e}[1m0${e}[0m ${e}[33;1mskipped${e}[0m"
} > "$dir/passing.txt" || fail "cannot write the passing log"
{
  echo "$pass1" && echo "$pass2"
} > "$dir/unread.txt" || fail "cannot write the log without a Starting line"
grep -q "$e" "$dir/failing.txt" || fail "the failing log holds no escape byte; this check proves nothing"
{
  echo '<testsuites><testsuite name="swell">'
  echo '<testcase name="builtins::array::search::tests::array_prototype_unscopables_lists_the_sixteen_names" classname="swell" time="0.091"/>'
  echo '<testcase name="builtins::array::iteration::tests::a_target_that_cannot_take_an_element_throws" classname="swell" time="0.092"/>'
  echo '<testcase name="builtins::array::mutation::tests::splice_uses_species" classname="swell" time="0.096"/>'
  echo '</testsuite></testsuites>'
} > "$dir/junit.xml" || fail "cannot write the JUnit file"
echo 1 > "$dir/failing.status"
echo 0 > "$dir/passing.status"

report() {
  # $1: the log's name
  task -d . nextest-report STEP=planted PREFIX='planted: ' LOG="$dir/$1.txt" \
    STATUS="$dir/$1.status" JUNIT="$dir/junit.xml" > "$dir/$1.report.txt" 2>&1
}

if report failing; then fail "task nextest-report exited 0 on a run with a failed test ($dir/failing.report.txt)"; fi
grep -qxF "planted: 68 test binaries: 637 passed, 1 failed, 1 ignored" "$dir/failing.report.txt" ||
  fail "task nextest-report did not count the coloured failing log ($dir/failing.report.txt)"

report passing || fail "task nextest-report exited non-zero on a coloured passing log ($dir/passing.report.txt)"
grep -qxF "planted: 1 test binaries: 3 passed, 0 failed, 0 ignored" "$dir/passing.report.txt" ||
  fail "task nextest-report did not count the coloured passing log ($dir/passing.report.txt)"
grep -qxF "gate-slowest planted 0.10 swell builtins::array::mutation::tests::splice_uses_species" "$dir/passing.report.txt" ||
  fail "task nextest-report printed no slowest test for the passing log ($dir/passing.report.txt)"

echo 0 > "$dir/unread.status"
if report unread; then fail "task nextest-report exited 0 on a log with no Starting line ($dir/unread.report.txt)"; fi
grep -qF "planted: read no \"Starting <n> tests across <b> binaries\" line with a binary in $dir/unread.txt" "$dir/unread.report.txt" ||
  fail "task nextest-report did not say what it could not read ($dir/unread.report.txt)"
if grep -q "test binaries:" "$dir/unread.report.txt"; then
  fail "task nextest-report printed a count for a log it could not read ($dir/unread.report.txt)"
fi

echo "$name: ok"

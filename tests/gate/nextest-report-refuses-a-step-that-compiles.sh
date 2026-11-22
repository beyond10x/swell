#!/bin/sh
# Planted defect of the tasks nextest-report and compile-check (taskfiles/nextest.yml): a timed
# test step that compiles. The gate builds what its test steps run before gate-time-start (task
# build-tests and the build task of each step), so that a step's seconds are the tests' and not
# a build's; a step whose log has a cargo "Compiling" line has built inside the clock, and its
# report fails with one line naming the step, the number of crates and the build task that
# builds them. Before, the gate counted such a step and passed it (the gate's run on the wave
# branch of slice 3: test-collect 146.7 s and conform-engine 241.7 s, 51 crates compiled inside
# conform-engine).
#   1. a coloured passing cargo-nextest log with two Compiling lines, BUILT_BY=build-planted: the
#      task prints the counts, then "planted: compiled 2 crate(s) inside the timed step, which
#      the task build-planted builds before gate-time-start: swell-syntax, swell (<log>)", and
#      fails;
#   2. the same log without them: it prints the counts and exits 0; and the log with them but
#      no BUILT_BY (the steps of the lane slow, which no build phase precedes): it exits 0;
#   3. a plain cargo test --doc log with one Compiling line, read by compile-check alone (as
#      test-doc and the cargo run of conform-engine are): it fails with the same line.
# The coloured logs are *.txt files: gate-selftest searches a script's *.log and *.out files for
# escape bytes, and these hold them on purpose.
# Run from the repository root: sh tests/gate/nextest-report-refuses-a-step-that-compiles.sh
# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=nextest-report-refuses-a-step-that-compiles
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

e=$(printf '\033')
compiling1="${e}[1m${e}[92m   Compiling${e}[0m swell-syntax v0.2.0 (/work/swell/crates/syntax)"
compiling2="${e}[1m${e}[92m   Compiling${e}[0m swell v0.2.0 (/work/swell/crates/swell)"
finished="${e}[1m${e}[92m    Finished${e}[0m \`test\` profile [unoptimized + debuginfo] target(s) in 25.81s"
pass1="${e}[32;1m        PASS${e}[0m [   0.091s] (  1/2) ${e}[35;1mswell${e}[0m ${e}[34;1mcompiling_a_script_answers${e}[0m"
pass2="${e}[32;1m        PASS${e}[0m [   0.092s] (  2/2) ${e}[35;1mswell${e}[0m ${e}[34;1mCompiling_is_a_name_not_a_line${e}[0m"
{
  echo "$compiling1" && echo "$compiling2" && echo "$finished"
  echo "${e}[32;1m    Starting${e}[0m ${e}[1m2${e}[0m tests across ${e}[1m1${e}[0m binary"
  echo "$pass1" && echo "$pass2"
  echo "${e}[32;1m     Summary${e}[0m [   0.183s] ${e}[1m2${e}[0m tests run: ${e}[1m2${e}[0m ${e}[32;1mpassed${e}[0m, ${e}[1m0${e}[0m ${e}[33;1mskipped${e}[0m"
} > "$dir/compiled.txt" || fail "cannot write the log with Compiling lines"
grep -v 'Compiling.* v0\.2\.0' "$dir/compiled.txt" > "$dir/built.txt" || fail "cannot write the log without them"
grep -q "$e" "$dir/compiled.txt" || fail "the log holds no escape byte; this check proves nothing"
[ "$(grep -c Compiling "$dir/built.txt")" -eq 1 ] || fail "the log without Compiling lines lost the test that names the word"
{
  echo '<testsuites><testsuite name="swell">'
  echo '<testcase name="compiling_a_script_answers" classname="swell" time="0.091"/>'
  echo '<testcase name="Compiling_is_a_name_not_a_line" classname="swell" time="0.092"/>'
  echo '</testsuite></testsuites>'
} > "$dir/junit.xml" || fail "cannot write the JUnit file"
echo 0 > "$dir/compiled.status"
echo 0 > "$dir/built.status"
{
  echo "   Compiling swell v0.2.0 (/work/swell/crates/swell)"
  echo "    Finished \`test\` profile [unoptimized + debuginfo] target(s) in 2.79s"
  echo "   Doc-tests swell"
  echo "test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s"
} > "$dir/doc.txt" || fail "cannot write the doc-test log"

report() {
  # $1: the log's name
  task -d . nextest-report STEP=planted PREFIX='planted: ' LOG="$dir/$1.txt" BUILT_BY=build-planted \
    STATUS="$dir/$1.status" JUNIT="$dir/junit.xml" > "$dir/$1.report.txt" 2>&1
}

if report compiled; then fail "task nextest-report exited 0 on a step that compiled 2 crates ($dir/compiled.report.txt)"; fi
grep -qxF "planted: 1 test binaries: 2 passed, 0 failed, 0 ignored" "$dir/compiled.report.txt" ||
  fail "task nextest-report did not count the run before refusing it ($dir/compiled.report.txt)"
grep -qxF "planted: compiled 2 crate(s) inside the timed step, which the task build-planted builds before gate-time-start: swell-syntax, swell ($dir/compiled.txt)" "$dir/compiled.report.txt" ||
  fail "task nextest-report did not name the step, the 2 crates and the build task ($dir/compiled.report.txt)"

report built || fail "task nextest-report exited non-zero on a step that compiled nothing ($dir/built.report.txt)"
grep -qxF "planted: 1 test binaries: 2 passed, 0 failed, 0 ignored" "$dir/built.report.txt" ||
  fail "task nextest-report did not count the run that compiled nothing ($dir/built.report.txt)"
if grep -q 'inside the timed step' "$dir/built.report.txt"; then
  fail "task nextest-report refused a step that compiled nothing ($dir/built.report.txt)"
fi

task -d . nextest-report STEP=planted PREFIX='planted: ' LOG="$dir/compiled.txt" \
  STATUS="$dir/compiled.status" JUNIT="$dir/junit.xml" > "$dir/unbuilt.report.txt" 2>&1 ||
  fail "task nextest-report without BUILT_BY refused a log with Compiling lines; the lane slow has no build phase ($dir/unbuilt.report.txt)"

if task -d . compile-check STEP=planted-doc LOG="$dir/doc.txt" BUILT_BY=build-planted > "$dir/doc.report.txt" 2>&1; then
  fail "task compile-check exited 0 on a doc-test log with a Compiling line ($dir/doc.report.txt)"
fi
grep -qxF "planted-doc: compiled 1 crate(s) inside the timed step, which the task build-planted builds before gate-time-start: swell ($dir/doc.txt)" "$dir/doc.report.txt" ||
  fail "task compile-check did not name the step, the crate and the build task ($dir/doc.report.txt)"

echo "$name: ok"

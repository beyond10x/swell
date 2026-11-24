#!/bin/sh
# Planted defects of the CPU bound of the gate's test steps and of a build inside their clock
# (taskfiles/gate-time.yml, tasks gate-time-start, gate-time and gate-time-sum; story
# gate-test-steps-fit-four-cores). The bound is CPU seconds per kind of machine: GATE_CPU_BOUNDS
# holds pairs "<cores>:<CPU seconds>" (check-runner gives the table of taskfiles/forge.yml), and
# gate-time-sum looks up the core count it prints. A run with an entry for its core count fails
# over that bound, naming the CPU seconds and the costliest steps; a run whose core count has no
# entry prints the CPU seconds without a bound, and they decide nothing; an entry that is not
# "<cores>:<CPU seconds>" is refused (a bound whose CPU seconds cannot be read fails in
# tests/gate/gate-time-refuses-test-steps-over-the-budget.sh). A test step that writes into a
# cargo target directory while the clock runs has built inside it, which the forge's runner pays
# where this machine's compile cache and a warm target/ hide it (the release runner of test262
# was built so inside gate-selftest): the run fails, naming the file and the task build-tests.
# The clock runs as in check, in one task process, in a directory of its own
# (.out/gate-tests/<name>/run) whose Taskfile.yml includes taskfiles/gate-time.yml and adds the
# task timed: gate-time-start, a step "first", a step "burnt" that spends about a tenth of a CPU
# second in awk when BURN is 1 and writes target/release/deps/planted-inside-the-clock when WRITE
# is 1, then gate-time-sum. GATE_TIME_CORES plants the core count, 7, which no machine of the
# table has, so that the run reads the same on any machine. Its target/debug/deps holds one file
# older than gate-time-start.
# 1. GATE_CPU_BOUNDS="16:280" (no entry for 7 cores): it passes and prints "gate-cpu test-steps
#    <cpu>, no bound for 7 cores: first <cpu>, burnt <cpu>" and "gate-time: no file of 1 under
#    target/*/deps changed while the test steps ran".
# 2. GATE_CPU_BOUNDS="16:280 7:100": it passes and prints "gate-cpu test-steps <cpu> of 100 on 7
#    cores: first <cpu>, burnt <cpu>".
# 3. BURN=1, WRITE=1, GATE_CPU_BOUNDS="7:0.05 16:280": it exits non-zero and prints "gate-cpu:
#    the test steps took <cpu> CPU seconds, over the bound of 0.05 for 7 cores; the costliest:
#    burnt <cpu>, first <cpu>" and "gate-time: the test steps wrote 1 file(s) into target/*/deps
#    or conformance/ess/target/*/deps inside the clock, the first
#    target/release/deps/planted-inside-the-clock; ... build-tests ...", and the wall line.
# 4. GATE_CPU_BOUNDS="16:280 7:fast": it exits non-zero and prints "gate-cpu: GATE_CPU_BOUNDS
#    holds 7:fast, not <cores>:<CPU seconds>".
# 5. GATE_CPU_BOUNDS="16:280 16:300" (two entries for one core count, none for 7): it exits
#    non-zero and prints "gate-cpu: GATE_CPU_BOUNDS holds 16 cores twice"; no entry wins.
# Without GATE_CPU_BOUNDS (the forge's check) the line has neither a bound nor "no bound":
# tests/gate/gate-time-refuses-test-steps-over-the-budget.sh holds it.
# Run from the repository root:
# sh tests/gate/gate-time-refuses-test-steps-over-the-cpu-bound-or-building.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=gate-time-refuses-test-steps-over-the-cpu-bound-or-building
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/run/target/debug/deps" || { echo "$name: FAIL cannot make $dir"; exit 1; }

fail() {
  echo "$name: FAIL $*"
  exit 1
}

[ -f taskfiles/gate-time.yml ] || fail "no taskfiles/gate-time.yml"
: > "$dir/run/target/debug/deps/built-before-the-clock" || fail "cannot plant a file of the build phase"
printf "version: '3'\n\nincludes:\n  gate:\n    taskfile: %s\n    flatten: true\n\ntasks:\n%s\n" "$(pwd)/taskfiles/gate-time.yml" '  timed:
    silent: true
    vars:
      BURN: '"'"'{{.BURN | default "0"}}'"'"'
      WRITE: '"'"'{{.WRITE | default "0"}}'"'"'
    cmds:
      - task: gate-time-start
      - task: gate-time
        vars: {STEP: first}
      - '"'"'[ "{{.BURN}}" != 1 ] || awk "BEGIN { for (i = 0; i < 2000000; i++) s += i % 7; if (s < 0) print s }"'"'"'
      - '"'"'[ "{{.WRITE}}" != 1 ] || { mkdir -p target/release/deps && echo planted > target/release/deps/planted-inside-the-clock; }'"'"'
      - task: gate-time
        vars: {STEP: burnt}
      - task: gate-time-sum' > "$dir/run/Taskfile.yml" || fail "cannot write the planted Taskfile.yml"
t() { task -d "$dir/run" "$@" GATE_TIME_CORES=7; }
c='[0-9]+\.[0-9]'

t timed GATE_CPU_BOUNDS="16:280" > "$dir/unbound.out" 2>&1 ||
  fail "the timed steps exited non-zero with no entry for their core count ($dir/unbound.out)"
grep -qE "^gate-cpu test-steps $c, no bound for 7 cores: first $c, burnt $c\$" "$dir/unbound.out" ||
  fail "no line \"gate-cpu test-steps <cpu>, no bound for 7 cores: first <cpu>, burnt <cpu>\" ($dir/unbound.out)"
grep -qxF 'gate-time: no file of 1 under target/*/deps changed while the test steps ran' "$dir/unbound.out" ||
  fail "no line saying that no file of target/*/deps changed while the steps ran ($dir/unbound.out)"

t timed GATE_CPU_BOUNDS="16:280 7:100" > "$dir/within.out" 2>&1 ||
  fail "the timed steps exited non-zero under a bound of 100 for 7 cores ($dir/within.out)"
grep -qE "^gate-cpu test-steps $c of 100 on 7 cores: first $c, burnt $c\$" "$dir/within.out" ||
  fail "no line \"gate-cpu test-steps <cpu> of 100 on 7 cores: first <cpu>, burnt <cpu>\" ($dir/within.out)"

if t timed BURN=1 WRITE=1 GATE_CPU_BOUNDS="7:0.05 16:280" > "$dir/over.out" 2>&1; then
  fail "the timed steps exited 0 over a bound of 0.05 for 7 cores and with a file written into target/ inside the clock ($dir/over.out)"
fi
grep -qE "^gate-cpu: the test steps took $c CPU seconds, over the bound of 0\\.05 for 7 cores; the costliest: burnt $c, first $c\$" "$dir/over.out" ||
  fail "no line naming the CPU seconds over the bound of 0.05 for 7 cores and the burnt step first ($dir/over.out)"
grep -qE '^gate-time: the test steps wrote 1 file\(s\) into target/\*/deps or conformance/ess/target/\*/deps inside the clock, the first target/release/deps/planted-inside-the-clock; .*build-tests' "$dir/over.out" ||
  fail "gate-time-sum did not name the file written inside the clock and the task build-tests ($dir/over.out)"
grep -qE '^gate-time test-steps [0-9]+\.[0-9] of 120' "$dir/over.out" ||
  fail "the wall time line is missing from a refused run ($dir/over.out)"

if t timed GATE_CPU_BOUNDS="16:280 7:fast" > "$dir/notanumber.out" 2>&1; then
  fail "the timed steps exited 0 with an entry 7:fast in GATE_CPU_BOUNDS ($dir/notanumber.out)"
fi
grep -qxF 'gate-cpu: GATE_CPU_BOUNDS holds 7:fast, not <cores>:<CPU seconds>' "$dir/notanumber.out" ||
  fail "no line refusing the entry 7:fast ($dir/notanumber.out)"

if t timed GATE_CPU_BOUNDS="16:280 16:300" > "$dir/twice.out" 2>&1; then
  fail "the timed steps exited 0 with two entries for 16 cores in GATE_CPU_BOUNDS ($dir/twice.out)"
fi
grep -qxF 'gate-cpu: GATE_CPU_BOUNDS holds 16 cores twice' "$dir/twice.out" ||
  fail "no line refusing two entries for 16 cores ($dir/twice.out)"

echo "$name: ok"

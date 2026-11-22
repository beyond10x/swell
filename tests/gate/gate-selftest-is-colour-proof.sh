#!/bin/sh
# Planted defect: the environment of a CI runner on GitHub Actions. There go-task colours its
# messages (CI=true), adds a "::error" annotation line to a failing task's output
# (GITHUB_ACTIONS=true), and cargo colours its own (CARGO_TERM_COLOR=always). A script that
# compares the output of task or cargo must still pass there, by turning both off itself.
# The task gate-selftest runs every script of tests/gate/ once in that environment, with
# nothing inherited that turns colour off, and refuses one that passes with an escape byte or a
# "::error" line in its output or in the *.log and *.out files of its directory. This script
# shows it on the smallest input: a probe first shows that the environment does change task's
# output, so that the check is not passed by a task that never colours; then gate-selftest runs
# on a directory of two planted scripts that each run that failing probe task and print
# "<name>: ok", one turning colour off as every script does and one not. It must fail, name
# the second as passing with an escape byte or a ::error line in its logs, and pass the first;
# on the first alone it must pass. (Until story gate-test-steps-within-budget this script ran
# every other script a second time in the runner's environment.)
# Run from the repository root: sh tests/gate/gate-selftest-is-colour-proof.sh
# Works only under .out/gate-tests/<name>/ (the two planted scripts work under their own
# directories there); the probe's coloured output is kept in probe.txt, which gate-selftest does
# not read; prints "<name>: ok" or "<name>: FAIL <why>".

unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never

name=gate-selftest-is-colour-proof
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

# The runner's environment, and the two variables that force colour elsewhere.
runner() {
  (
    unset NO_COLOR
    export CI=true GITHUB_ACTIONS=true CARGO_TERM_COLOR=always
    export FORCE_COLOR=1 CLICOLOR_FORCE=1
    "$@"
  )
}

rm -rf "$dir" && mkdir -p "$dir/probe" "$dir/scripts" "$dir/proof-only" || fail "cannot make $dir"

printf '%s\n' 'version: "3"' '' 'tasks:' '  fails:' '    silent: true' '    cmds:' \
  '      - echo a fault line' '      - exit 1' >"$dir/probe/Taskfile.yml" || fail "cannot write the probe"
if runner task -d "$dir/probe" fails >"$dir/probe.txt" 2>&1; then
  fail "the probe task exited 0 ($dir/probe.txt)"
fi
esc=$(printf '\033')
grep -q "$esc" "$dir/probe.txt" ||
  fail "task wrote no escape byte in the runner's environment; this check proves nothing ($dir/probe.txt)"
grep -q '^::error ' "$dir/probe.txt" ||
  fail "task wrote no ::error line in the runner's environment; this check proves nothing ($dir/probe.txt)"

# Two planted scripts: each runs the failing probe task into a log of its own directory and
# passes; only the first turns colour off first.
proof=colour-proof-planted-turns-colour-off
coloured=colour-proof-planted-keeps-colour
for planted in "$proof" "$coloured"; do
  {
    echo '#!/bin/sh'
    [ "$planted" = "$proof" ] && printf '%s\n' 'unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE' 'export NO_COLOR=1 CARGO_TERM_COLOR=never'
    echo "mkdir -p .out/gate-tests/$planted"
    echo "task -d $dir/probe fails > .out/gate-tests/$planted/probe.log 2>&1 && exit 1"
    echo "echo \"$planted: ok\""
  } >"$dir/scripts/$planted.sh" || fail "cannot write the planted script $planted"
done
cp "$dir/scripts/$proof.sh" "$dir/proof-only/$proof.sh" || fail "cannot copy the planted script $proof"

if task -d . gate-selftest GATE_SELFTEST_DIR="$dir/scripts" >"$dir/both.out" 2>&1; then
  fail "gate-selftest exited 0 with a script that keeps colour ($dir/both.out)"
fi
grep -qF "$coloured: passes, but its logs hold an escape byte or a ::error line:" "$dir/both.out" ||
  fail "gate-selftest did not name $coloured as passing with colour in its logs ($dir/both.out)"
grep -qxF "$proof: ok" "$dir/both.out" || fail "gate-selftest did not pass $proof ($dir/both.out)"
grep -qxF "gate-selftest: 1 passed, 1 failed" "$dir/both.out" ||
  fail "gate-selftest did not count one passed and one failed ($dir/both.out)"

task -d . gate-selftest GATE_SELFTEST_DIR="$dir/proof-only" >"$dir/proof-only.out" 2>&1 ||
  fail "gate-selftest refused a script that turns colour off ($dir/proof-only.out)"
grep -qxF "gate-selftest: 1 passed, 0 failed" "$dir/proof-only.out" ||
  fail "gate-selftest did not pass the one script that turns colour off ($dir/proof-only.out)"

echo "$name: ok"

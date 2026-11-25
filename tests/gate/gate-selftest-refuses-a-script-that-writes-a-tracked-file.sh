#!/bin/sh
# Planted defect of the task gate-selftest: a planted script that writes a tracked file of the
# tree it runs in. site-data-check-refuses-a-percentage-rounded-up-to-100 once left the compare
# page's data at its default and overwrote the tracked website/data/engines.json with planted
# counts on every gate run (story gate-selftests-write-no-tracked-file). gate-selftest records
# the tracked files that differ from HEAD, with their content, before and after the scripts run,
# and fails, naming the files, when the two records differ. This script runs gate-selftest on a
# git repository of its own (GATE_SELFTEST_TREE), never on this tree, which has one committed
# file kept.txt and one file dirty.txt changed before the step, and three planted scripts, each
# alone: one that writes only an untracked file must pass, though dirty.txt differs from HEAD;
# one that writes kept.txt must fail and name kept.txt; one that changes dirty.txt further must
# fail and name dirty.txt.
# Run from the repository root: sh tests/gate/gate-selftest-refuses-a-script-that-writes-a-tracked-file.sh
# Works only under .out/gate-tests/<name>/ (the planted scripts write their logs beside it as
# gate-selftest does); prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE GATE_SELFTEST_DIR GATE_SELFTEST_TREE GATE_SELFTEST_JOBS
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=gate-selftest-refuses-a-script-that-writes-a-tracked-file
dir=.out/gate-tests/$name
repo=$dir/repo
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$repo" "$dir/untracked" "$dir/kept" "$dir/dirty" || fail "cannot make $dir"

g() { git -C "$repo" -c user.name=gate -c user.email=gate@example.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null "$@"; }
g init -q || fail "cannot make a git repository in $repo"
echo kept > "$repo/kept.txt" && echo committed > "$repo/dirty.txt" || fail "cannot write the tracked files"
g add kept.txt dirty.txt && g commit -q -m planted || fail "cannot commit the tracked files in $repo"
echo "changed before the step" > "$repo/dirty.txt" || fail "cannot change dirty.txt"

plant() {
  # $1 the directory of the script, $2 its name, $3 the command it runs before it passes.
  printf '%s\n' '#!/bin/sh' "$3" "echo \"$2: ok\"" > "$1/$2.sh" || fail "cannot write the planted script $2"
}
plant "$dir/untracked" tracked-guard-planted-writes-untracked "echo new > $repo/new.txt"
plant "$dir/kept" tracked-guard-planted-writes-kept "echo planted >> $repo/kept.txt"
plant "$dir/dirty" tracked-guard-planted-writes-dirty "echo planted >> $repo/dirty.txt"

run() {
  # $1 the directory of planted scripts; the output goes to $dir/<its base name>.out.
  task -d . gate-selftest GATE_SELFTEST_DIR="$1" GATE_SELFTEST_TREE="$repo" > "$dir/${1##*/}.out" 2>&1
}

run "$dir/untracked" || fail "gate-selftest refused a script that writes only an untracked file ($dir/untracked.out)"
grep -qxF "gate-selftest: 1 passed, 0 failed" "$dir/untracked.out" ||
  fail "gate-selftest did not pass the script that writes an untracked file ($dir/untracked.out)"
grep -q 'changed tracked files' "$dir/untracked.out" &&
  fail "gate-selftest named a change made before it ran, or an untracked file ($dir/untracked.out)"

if run "$dir/kept"; then
  fail "gate-selftest exited 0 though its script wrote the tracked kept.txt ($dir/kept.out)"
fi
grep -qxF "gate-selftest: the scripts changed tracked files of $repo (compared with before they ran): kept.txt" "$dir/kept.out" ||
  fail "gate-selftest did not name kept.txt, and kept.txt alone ($dir/kept.out)"
g checkout -q -- kept.txt || fail "cannot restore kept.txt in $repo"

if run "$dir/dirty"; then
  fail "gate-selftest exited 0 though its script changed dirty.txt further ($dir/dirty.out)"
fi
grep -qxF "gate-selftest: the scripts changed tracked files of $repo (compared with before they ran): dirty.txt" "$dir/dirty.out" ||
  fail "gate-selftest did not name dirty.txt, changed before it ran and again by its script ($dir/dirty.out)"

echo "$name: ok"

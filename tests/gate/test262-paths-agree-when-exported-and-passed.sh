#!/bin/sh
# Planted defect of taskfiles/test262.yml: a gate variable that is exported and also passed on
# the command line with another value. The tasks test262, test262-check (and the tasks of the
# file that share its form) and test262-moves-check must read the value of the command line,
# and the exported value when the command line gives none (a task's env: keeps the exported
# value instead, go-task 3.54.0).
# test262-moves-check, three times on committed files, each on the first line of
# conformance/test262/moves.tsv alone (the task checks each line on its own; the gate's own step
# checks the whole list): TEST262_MOVES exported as a copy of that line whose evidence is no
# token of conformance/test262/later-parts.tsv, and passed as the line itself, must exit 0;
# exported as the line and passed as the copy, must exit non-zero naming the copy; exported as
# the copy and not passed, must exit non-zero naming the copy.
# test262-check, once (it runs no case): TEST262_OUT exported as one directory and passed as
# another; the sources digest it computes must be in the directory passed and nothing in the one
# exported, whether or not the digest matches.
# test262, once: TEST262_OUT exported as one directory and passed as another, and TEST262_RUN
# exported as none and passed as a mode the runner refuses before it reads a case (so the run
# takes seconds); the runner's refusal of the mode passed must be in the count.log of the
# directory passed, and the directory exported must not exist.
# Run from the repository root: sh tests/gate/test262-paths-agree-when-exported-and-passed.sh
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test262-paths-agree-when-exported-and-passed
dir=.out/gate-tests/$name
moves=conformance/test262/moves.tsv
unset TEST262_DESIGN TEST262_CACHE TEST262_COMMIT TEST262_MOVES TEST262_LATER_PARTS
unset TEST262_FEATURES TEST262_SECTIONS TEST262_EXPECTED TEST262_OUT TEST262_WORKERS TEST262_RUN TEST262_DIGEST GATE_ROOT
rm -rf "$dir"
mkdir -p "$dir"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

[ -s "$moves" ] || fail "no $moves"
good=$dir/moves-first-line.tsv
awk 'NR == 1' "$moves" > "$good" || fail "cannot write $good"
bad=$dir/moves-with-no-token.tsv
awk -F'\t' -v OFS='\t' 'NR == 1 { $4 = "part"; $5 = "planted-not-a-token" } { print }' "$good" > "$bad" ||
  fail "cannot write $bad"
cmp -s "$good" "$bad" && fail "the planted copy equals $good"

if ! TEST262_MOVES="$bad" task -d . test262-moves-check TEST262_MOVES="$good" > "$dir/exported-bad.out" 2>&1; then
  fail "test262-moves-check read the exported TEST262_MOVES, not the one passed ($dir/exported-bad.out)"
fi
grep -q '^test262-moves: [0-9]* case(s) moved, ' "$dir/exported-bad.out" ||
  fail "test262-moves-check printed no summary line ($dir/exported-bad.out)"

if TEST262_MOVES="$good" task -d . test262-moves-check TEST262_MOVES="$bad" > "$dir/passed-bad.out" 2>&1; then
  fail "test262-moves-check exited 0 with the planted TEST262_MOVES passed ($dir/passed-bad.out)"
fi
grep -qF "test262-moves: $bad:1: " "$dir/passed-bad.out" ||
  fail "test262-moves-check did not name the planted copy it was passed ($dir/passed-bad.out)"

if TEST262_MOVES="$bad" task -d . test262-moves-check > "$dir/exported-only.out" 2>&1; then
  fail "test262-moves-check exited 0 with the planted TEST262_MOVES exported and none passed ($dir/exported-only.out)"
fi
grep -qF "test262-moves: $bad:1: " "$dir/exported-only.out" ||
  fail "test262-moves-check did not name the exported planted copy ($dir/exported-only.out)"

TEST262_OUT="$dir/check-exported" task -d . test262-check TEST262_OUT="$dir/check-passed" \
  > "$dir/test262-check.out" 2>&1
[ -e "$dir/check-exported" ] &&
  fail "task test262-check wrote into the exported TEST262_OUT $dir/check-exported, not the one passed ($dir/test262-check.out)"
grep -q '^test262-sources ' "$dir/check-passed/sources.digest" 2>/dev/null ||
  fail "task test262-check wrote no sources digest into the TEST262_OUT passed, $dir/check-passed/sources.digest ($dir/test262-check.out)"

if TEST262_OUT="$dir/exported" TEST262_RUN=none task -d . test262 TEST262_OUT="$dir/passed" \
  TEST262_RUN=planted-mode > "$dir/test262.out" 2>&1; then
  fail "task test262 exited 0 with TEST262_RUN=planted-mode passed ($dir/test262.out)"
fi
[ -e "$dir/exported" ] &&
  fail "task test262 wrote into the exported TEST262_OUT $dir/exported, not the one passed ($dir/test262.out)"
grep -q -e '--run needs all or none, not planted-mode' "$dir/passed/count.log" 2>/dev/null ||
  fail "task test262 did not run the runner with the TEST262_RUN passed and write its log into the TEST262_OUT passed, $dir/passed/count.log ($dir/test262.out)"

echo "$name: ok"

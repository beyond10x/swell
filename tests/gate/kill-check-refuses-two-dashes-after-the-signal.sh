#!/bin/sh
# The task kill-check (taskfiles/shell.yml) refuses a kill with two dashes after its signal
# operand, which the forge's sh (dash) reads as the process number "-", in Taskfile.yml,
# taskfiles/, tests/gate/ and .github/workflows/, naming file and line of each; and passes the
# forms dash reads: the group right after the signal, the signal by -s before the two dashes,
# two dashes without a signal, and pkill.
# 1. planted: a tree with one such kill in each of the four places, among lines of the allowed
#    forms: kill-check exits non-zero, names the four lines and no other, and counts 4.
# 2. clean: the same tree with only the allowed lines: kill-check exits 0 and counts its files.
# The planted lines are written with the word kill in a variable, so that this script holds no
# such line itself.
# Run from the repository root: sh tests/gate/kill-check-refuses-two-dashes-after-the-signal.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=kill-check-refuses-two-dashes-after-the-signal
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
k=kill

# allowed <file>: append the forms dash and bash both read.
allowed() {
  printf '%s\n' \
    "  $k -TERM \"-\$pid\" 2>/dev/null" \
    "  $k -0 \"-\$pg\" 2>/dev/null || exit 0" \
    "  $k -s TERM -- \"-\$pid\"" \
    "  $k -- \"-\$pid\"" \
    "  p$k -TERM -- sleep" >> "$1"
}

# tree <root>: the four places, each with the allowed forms.
tree() {
  mkdir -p "$1/taskfiles" "$1/tests/gate/fixtures" "$1/.github/workflows" || fail "cannot make $1"
  printf 'version: '"'"'3'"'"'\n' > "$1/Taskfile.yml"
  allowed "$1/Taskfile.yml"
  printf 'tasks:\n' > "$1/taskfiles/lane.yml"
  allowed "$1/taskfiles/lane.yml"
  printf '#!/bin/sh\n' > "$1/tests/gate/fixtures/stop.sh"
  allowed "$1/tests/gate/fixtures/stop.sh"
  printf 'jobs:\n' > "$1/.github/workflows/check.yml"
  allowed "$1/.github/workflows/check.yml"
}

# 1. One kill with two dashes after its signal in each place, as line 7 of each file.
planted=$dir/planted
tree "$planted"
printf '          %s -TERM -- "-$$" 2>/dev/null\n' "$k" >> "$planted/Taskfile.yml"
printf '  while %s -0 -- "-$pid" 2>/dev/null; do sleep 0.1; done\n' "$k" >> "$planted/taskfiles/lane.yml"
printf '  [ -n "$pg" ] && %s -KILL -- "-$pg"\n' "$k" >> "$planted/tests/gate/fixtures/stop.sh"
printf '        run: %s -9 -- "-$pid"\n' "$k" >> "$planted/.github/workflows/check.yml"

if task -d . kill-check GATE_ROOT="$planted" > "$dir/planted.out" 2>&1; then
  fail "kill-check accepted $planted, which holds four kills with two dashes after the signal ($dir/planted.out)"
fi
for want in \
  "$planted/Taskfile.yml:7: $k -TERM -- \"-\$\$\" 2>/dev/null" \
  "$planted/taskfiles/lane.yml:7: while $k -0 -- \"-\$pid\" 2>/dev/null; do sleep 0.1; done" \
  "$planted/tests/gate/fixtures/stop.sh:7: [ -n \"\$pg\" ] && $k -KILL -- \"-\$pg\"" \
  "$planted/.github/workflows/check.yml:7: run: $k -9 -- \"-\$pid\""; do
  grep -qxF "$want" "$dir/planted.out" || fail "kill-check did not name the line [$want] ($dir/planted.out)"
done
[ "$(grep -c ':[0-9][0-9]*: ' "$dir/planted.out")" = 4 ] ||
  fail "kill-check named other lines than the four planted ones; the allowed forms must pass ($dir/planted.out)"
grep -q '^kill-check: 4 kill(s) with two dashes after the signal operand; the forge'"'"'s sh is dash' "$dir/planted.out" ||
  fail "kill-check did not count 4 and say why ($dir/planted.out)"

# 2. Only the allowed forms.
clean=$dir/clean
tree "$clean"
task -d . kill-check GATE_ROOT="$clean" > "$dir/clean.out" 2>&1 ||
  fail "kill-check refused $clean, which holds only the forms dash reads ($dir/clean.out)"
grep -qxF 'kill-check: no kill with two dashes after its signal operand in 4 file(s)' "$dir/clean.out" ||
  fail "kill-check did not count the 4 files of $clean ($dir/clean.out)"

echo "$name: ok"

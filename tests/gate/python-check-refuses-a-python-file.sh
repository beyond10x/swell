#!/bin/sh
# The task python-check refuses a file named *.py and a file whose first line is a python
# interpreter line, naming both, and leaves a plain file alone.
# Run from the repository root: sh tests/gate/python-check-refuses-a-python-file.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=python-check-refuses-a-python-file
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"

root=$dir/planted
mkdir -p "$root" && git init -q "$root" || fail "cannot make the git repository $root"
printf 'print(1)\n' > "$root/tool.py"
printf '#!/usr/bin/env python3\nprint(1)\n' > "$root/run"
printf 'plain text\n' > "$root/notes.txt"

if task -d . python-check GATE_ROOT="$root" > "$dir/task.log" 2>&1; then
  fail "python-check accepted $root (log: $dir/task.log)"
fi
grep -qxF "$root/tool.py: a Python file" "$dir/task.log" ||
  fail "python-check did not name $root/tool.py (log: $dir/task.log)"
grep -qxF "$root/run: a Python file" "$dir/task.log" ||
  fail "python-check did not name $root/run (log: $dir/task.log)"
if grep -qF "notes.txt" "$dir/task.log"; then
  fail "python-check named the plain file $root/notes.txt (log: $dir/task.log)"
fi

echo "$name: ok"

#!/bin/sh
# Planted defect: the environment of a CI runner on GitHub Actions. There go-task colours its
# messages (CI=true), adds a "::error" annotation line to a failing task's output
# (GITHUB_ACTIONS=true), and cargo colours its own (CARGO_TERM_COLOR=always). A script that
# compares the output of task or cargo must still pass there, by turning both off itself.
# Every other script of tests/gate/ runs alone in that environment, with nothing inherited that
# turns colour off, and must pass with no escape byte and no "::error" line in the *.log and
# *.out files of its directory; each one that does not is named. A probe first shows that
# the environment does change task's output, so that the check is not passed by a task that
# never colours.
# Run from the repository root: sh tests/gate/gate-selftest-is-colour-proof.sh
# Works only under .out/gate-tests/<name>/ (each script it runs works under its own directory);
# prints "<name>: ok" or "<name>: FAIL <why>".

name=gate-selftest-is-colour-proof
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

# The runner's environment, and the two variables that force colour elsewhere.
runner() {
  (
    unset NO_COLOR
    export CI=true GITHUB_ACTIONS=true CARGO_TERM_COLOR=always CARGO_INCREMENTAL=0
    export FORCE_COLOR=1 CLICOLOR_FORCE=1
    "$@"
  )
}

rm -rf "$dir" && mkdir -p "$dir/probe" || fail "cannot make $dir"

printf '%s\n' 'version: "3"' '' 'tasks:' '  fails:' '    silent: true' '    cmds:' \
  '      - echo a fault line' '      - exit 1' >"$dir/probe/Taskfile.yml" || fail "cannot write the probe"
if runner task -d "$dir/probe" fails >"$dir/probe.log" 2>&1; then
  fail "the probe task exited 0 ($dir/probe.log)"
fi
esc=$(printf '\033')
grep -q "$esc" "$dir/probe.log" ||
  fail "task wrote no escape byte in the runner's environment; this check proves nothing ($dir/probe.log)"
grep -q '^::error ' "$dir/probe.log" ||
  fail "task wrote no ::error line in the runner's environment; this check proves nothing ($dir/probe.log)"

ran=0
failed=""
for script in $(find tests/gate -maxdepth 1 -type f -name '*.sh' | LC_ALL=C sort); do
  other=${script##*/}
  other=${other%.sh}
  [ "$other" = "$name" ] && continue
  ran=$((ran + 1))
  log=$dir/$other.log
  if ! runner sh "$script" >"$log" 2>&1 || [ "$(tail -n 1 "$log")" != "$other: ok" ]; then
    failed="$failed $other"
    said=$(grep "^$other: " "$log" | tail -n 1)
    [ -n "$said" ] || said=$(tail -n 1 "$log")
    echo "$name: $other fails in the runner's environment: $said"
    continue
  fi
  # A script that passes with coloured or annotated output in its logs passes by luck.
  marked=$(find ".out/gate-tests/$other" \( -name target -o -name .git \) -prune -o -type f \
    \( -name '*.log' -o -name '*.out' \) -print | LC_ALL=C sort | while read -r file; do
    if grep -q "$esc" "$file" || grep -q '^::error ' "$file"; then echo "$file"; fi
  done)
  if [ -n "$marked" ]; then
    failed="$failed $other"
    echo "$name: $other passes, but its logs hold an escape byte or a ::error line:" $marked
  fi
done
[ "$ran" -gt 0 ] || fail "no other script under tests/gate"
[ -z "$failed" ] || fail "in the runner's environment these are not colour-proof (logs: $dir/<script>.log):$failed"

echo "$name: ok"

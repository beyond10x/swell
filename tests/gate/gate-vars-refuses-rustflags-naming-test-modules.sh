#!/bin/sh
# Planted defect of the task gate-vars (Taskfile.yml; story realm-size-lane-within-its-share,
# correction 1, F4): the gate (task check) and the lane slow run gate-vars first, and go-task
# lets an exported RUSTFLAGS win over a task's own env, so a caller's flag variable of cargo
# that names the cfg test_modules would build swell's one test binary with a part of its modules.
# gate-vars refuses it, naming the variable. Nothing is built.
#   1. No such variable: passes, printing its line "gate: variables in effect: ...".
#   2. RUSTFLAGS="-D warnings": passes (a flag variable without test_modules).
#   3. RUSTFLAGS="--cfg test_modules": refused with "gate: RUSTFLAGS names test_modules".
#   4. CARGO_ENCODED_RUSTFLAGS="--cfg<0x1f>test_modules=\"eval_stack\"": refused, naming it.
#   5. CARGO_BUILD_RUSTFLAGS="--cfg test_modules": refused, naming it.
#   6. CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="--cfg test_modules": refused, naming it.
# Run from the repository root:
#   sh tests/gate/gate-vars-refuses-rustflags-naming-test-modules.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS CARGO_BUILD_RUSTFLAGS
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=gate-vars-refuses-rustflags-naming-test-modules
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
for variable in $(env | sed -n 's/^\(CARGO_TARGET_[A-Z0-9_]*_RUSTFLAGS\)=.*/\1/p'); do
  unset "$variable"
done

vars() {
  out=$1
  shift
  env "$@" task -d . gate-vars > "$dir/$out.out" 2>&1
}
passes() {
  out=$1
  what=$2
  shift 2
  vars "$out" "$@" || fail "gate-vars refused $what ($dir/$out.out)"
  grep -q '^gate: variables in effect:' "$dir/$out.out" || fail "gate-vars printed no line of its variables for $what ($dir/$out.out)"
  if grep -q 'names test_modules' "$dir/$out.out"; then fail "gate-vars named test_modules for $what ($dir/$out.out)"; fi
}
refused() {
  out=$1
  variable=$2
  shift 2
  if vars "$out" "$@"; then fail "gate-vars accepted $variable naming test_modules ($dir/$out.out)"; fi
  grep -q "^gate: $variable names test_modules (" "$dir/$out.out" ||
    fail "gate-vars did not name $variable ($dir/$out.out)"
}

passes none "an environment without a flag variable"
passes warnings "RUSTFLAGS=-D warnings" RUSTFLAGS='-D warnings'
refused rustflags RUSTFLAGS RUSTFLAGS='--cfg test_modules'
sep=$(printf '\037')
refused encoded CARGO_ENCODED_RUSTFLAGS "CARGO_ENCODED_RUSTFLAGS=--cfg${sep}test_modules=\"eval_stack\""
refused build CARGO_BUILD_RUSTFLAGS CARGO_BUILD_RUSTFLAGS='--cfg test_modules'
refused target CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS='--cfg test_modules'

echo "$name: ok"

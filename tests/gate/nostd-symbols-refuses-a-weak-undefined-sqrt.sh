#!/bin/sh
# The task nostd-symbols reads a weak undefined symbol as undefined and refuses the C library's
# sqrt in that form (llvm-nm type "w"), naming the symbol and its object. A crate under
# #![forbid(unsafe_code)] cannot leave such a symbol (it needs global_asm! or #[linkage]), so
# the script feeds the task planted llvm-nm output through NOSTD_NM_OUTPUT and builds nothing
# (adversary pass 1 on unit gate). The same output without the weak line passes, so the
# refusal comes from it: a v0 and a legacy mangled Rust name, memcpy and a routine of the
# compiler runtime are read and counted.
# Run from the repository root: sh tests/gate/nostd-symbols-refuses-a-weak-undefined-sqrt.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=nostd-symbols-refuses-a-weak-undefined-sqrt
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
obj=$dir/target/libplanted.rlib:planted-0.planted.1a2b-cgu.0.rcgu.o:
printf '%s                  U %s\n' \
  "$obj" _RINvNtCsbQJWu2zlgTK_4core3mem4dropyECs9FGluVDpSU1_9swell_num \
  "$obj" _ZN4core9panicking5panic17h0123456789abcdefE \
  "$obj" memcpy \
  "$obj" __adddf3 > "$dir/clean.nm" || fail "cannot write $dir/clean.nm"
{ cat "$dir/clean.nm"; printf '%s                  w sqrt\n' "$obj"; } > "$dir/weak.nm" ||
  fail "cannot write $dir/weak.nm"

task -d . nostd-symbols NOSTD_NM_OUTPUT="$dir/clean.nm" NOSTD_OUT="$dir/clean" > "$dir/clean.log" 2>&1 ||
  fail "nostd-symbols refused the output without the weak line (log: $dir/clean.log)"
grep -qx "nostd-symbols: 1 library, 1 object(s), 4 undefined symbol(s): 2 Rust's own, 1 memory functions, 1 compiler runtime routines; none from a C library or the system" "$dir/clean.log" ||
  fail "nostd-symbols did not count the four symbols of the output without the weak line (log: $dir/clean.log)"

if task -d . nostd-symbols NOSTD_NM_OUTPUT="$dir/weak.nm" NOSTD_OUT="$dir/weak" > "$dir/weak.log" 2>&1; then
  fail "nostd-symbols accepted a weak undefined sqrt (log: $dir/weak.log)"
fi
grep -qx "nostd-symbols: libplanted.rlib(planted-0.planted.1a2b-cgu.0.rcgu.o): sqrt is neither Rust's own nor the compiler runtime's" "$dir/weak.log" ||
  fail "nostd-symbols failed without naming sqrt and its object (log: $dir/weak.log)"
grep -q '^nostd-symbols: 1 undefined symbol(s) of 1 library outside Rust and the compiler runtime' "$dir/weak.log" ||
  fail "nostd-symbols failed without its count line (log: $dir/weak.log)"

echo "$name: ok"

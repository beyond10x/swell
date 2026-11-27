#!/bin/sh
# The task nostd-symbols takes as Rust's own only what has the form of a mangled Rust name: it
# refuses _ZNSt9exceptionD2Ev (std::exception::~exception of the C++ library), which starts
# with _ZN like a legacy Rust name but has no hash part h<16 hex digits> before its E, and
# _ZN4core3fmt5write17h0123E, whose hash is too short; each is named with its object. A crate
# under #![forbid(unsafe_code)] cannot leave such a name (it needs an unsafe extern block), so
# the script feeds the task planted llvm-nm output through NOSTD_NM_OUTPUT and builds nothing
# (adversary pass 1 on unit gate). The same output without those two lines passes.
# Run from the repository root: sh tests/gate/nostd-symbols-refuses-a-cxx-library-symbol.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=nostd-symbols-refuses-a-cxx-library-symbol
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
obj=$dir/target/libplanted.rlib:planted-0.planted.1a2b-cgu.0.rcgu.o:
printf '%s                  U %s\n' \
  "$obj" _RINvNtCsbQJWu2zlgTK_4core3mem4dropyECs9FGluVDpSU1_9swell_num \
  "$obj" _ZN4core9panicking5panic17h0123456789abcdefE \
  "$obj" memcmp > "$dir/clean.nm" || fail "cannot write $dir/clean.nm"
{
  cat "$dir/clean.nm"
  printf '%s                  U %s\n' "$obj" _ZNSt9exceptionD2Ev "$obj" _ZN4core3fmt5write17h0123E
} > "$dir/cxx.nm" || fail "cannot write $dir/cxx.nm"

task -d . nostd-symbols NOSTD_NM_OUTPUT="$dir/clean.nm" NOSTD_OUT="$dir/clean" > "$dir/clean.log" 2>&1 ||
  fail "nostd-symbols refused the output without the two names (log: $dir/clean.log)"
grep -qx "nostd-symbols: 1 library, 1 object(s), 3 undefined symbol(s): 2 Rust's own, 1 memory functions, 0 compiler runtime routines; none from a C library or the system" "$dir/clean.log" ||
  fail "nostd-symbols did not count the three symbols of the output without the two names (log: $dir/clean.log)"

if task -d . nostd-symbols NOSTD_NM_OUTPUT="$dir/cxx.nm" NOSTD_OUT="$dir/cxx" > "$dir/cxx.log" 2>&1; then
  fail "nostd-symbols accepted a C++ library name (log: $dir/cxx.log)"
fi
for sym in _ZNSt9exceptionD2Ev _ZN4core3fmt5write17h0123E; do
  grep -qx "nostd-symbols: libplanted.rlib(planted-0.planted.1a2b-cgu.0.rcgu.o): $sym is neither Rust's own nor the compiler runtime's" "$dir/cxx.log" ||
    fail "nostd-symbols failed without naming $sym and its object (log: $dir/cxx.log)"
done
grep -q '^nostd-symbols: 2 undefined symbol(s) of 1 library outside Rust and the compiler runtime' "$dir/cxx.log" ||
  fail "nostd-symbols failed without its count line (log: $dir/cxx.log)"

echo "$name: ok"

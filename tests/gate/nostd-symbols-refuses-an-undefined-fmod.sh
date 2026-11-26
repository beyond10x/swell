#!/bin/sh
# The task nostd-symbols refuses a library built for the target without std that leaves fmod
# undefined, naming the symbol and the object: a no_std crate whose f64 % f64 the compiler lowers
# to a call of the C library's fmod. A crate beside it that adds two f64, copies a slice into a
# Vec and compares two slices leaves only Rust's own symbols, memory functions and routines of the
# compiler's runtime undefined, and passes.
# Run from the repository root: sh tests/gate/nostd-symbols-refuses-an-undefined-fmod.sh
# Works only under .out/gate-tests/<name>/, which holds the planted workspace and its cargo target
# directory; prints "<name>: ok" or "<name>: FAIL <why>".
#
# The planted workspace has two members, crates/clean and crates/planted-fmod, with no
# dependency; its Cargo.lock is made for it (cargo generate-lockfile --offline). The task
# nostd-build builds both, as the gate builds the engine crates.

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=nostd-symbols-refuses-an-undefined-fmod
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/ws/crates/clean/src" "$dir/ws/crates/planted-fmod/src" || fail "cannot make $dir"
ws=$dir/ws
printf '[workspace]\nmembers = ["crates/clean", "crates/planted-fmod"]\nresolver = "3"\n' > "$ws/Cargo.toml" ||
  fail "cannot write the planted Cargo.toml"
for crate in clean planted-fmod; do
  printf '[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2024"\n' "$crate" > "$ws/crates/$crate/Cargo.toml" ||
    fail "cannot write the planted crates/$crate/Cargo.toml"
done
printf '%s\n' '#![no_std]' '#![forbid(unsafe_code)]' '//! A planted engine crate with nothing from a C library.' '' \
  'extern crate alloc;' '' \
  '/// The sum of two f64, a routine of the compiler runtime on a target without a floating-point unit.' \
  'pub fn add(a: f64, b: f64) -> f64 {' '    a + b' '}' '' \
  '/// A copy of a slice: memcpy and the allocator.' \
  'pub fn copy(bytes: &[u8]) -> alloc::vec::Vec<u8> {' '    bytes.to_vec()' '}' '' \
  '/// Whether two slices are equal: memcmp or bcmp.' \
  'pub fn same(a: &[u8], b: &[u8]) -> bool {' '    a == b' '}' > "$ws/crates/clean/src/lib.rs" ||
  fail "cannot write crates/clean/src/lib.rs"
printf '%s\n' '#![no_std]' '#![forbid(unsafe_code)]' '//! A planted engine crate whose remainder calls fmod.' '' \
  '/// The remainder of two f64, which the compiler lowers to a call of fmod.' \
  'pub fn rem(a: f64, b: f64) -> f64 {' '    a % b' '}' > "$ws/crates/planted-fmod/src/lib.rs" ||
  fail "cannot write crates/planted-fmod/src/lib.rs"
(cd "$ws" && cargo generate-lockfile --offline) > "$dir/lock.log" 2>&1 ||
  fail "cannot make a Cargo.lock for the planted workspace (log: $dir/lock.log)"

task -d . nostd-build NOSTD_ROOT="$ws" NOSTD_PACKAGES="clean planted-fmod" NOSTD_OUT="$dir/out" > "$dir/build.log" 2>&1 ||
  fail "nostd-build failed on the planted workspace (log: $dir/build.log)"
clean=$(awk '$1 == "clean" { print $2 }' "$dir/out/libs.txt")
[ -n "$clean" ] && [ -f "$clean" ] || fail "nostd-build listed no library of clean in $dir/out/libs.txt"

# The clean library alone passes, and the scan read something.
task -d . nostd-symbols NOSTD_LIBS="$clean" > "$dir/clean.log" 2>&1 ||
  fail "nostd-symbols refused the clean library (log: $dir/clean.log)"
grep -q '^nostd-symbols: 1 library, [1-9][0-9]* object(s), [1-9][0-9]* undefined symbol(s): ' "$dir/clean.log" ||
  fail "nostd-symbols passed without reading an undefined symbol of the clean library (log: $dir/clean.log)"

# Both, as the list of nostd-build gives them: fmod is named with its object, and nothing of clean.
if task -d . nostd-symbols NOSTD_OUT="$dir/out" > "$dir/both.log" 2>&1; then
  fail "nostd-symbols accepted a library that leaves fmod undefined (log: $dir/both.log)"
fi
grep -q '^nostd-symbols: libplanted_fmod\.rlib([^)]*\.o): fmod is neither Rust'"'"'s own nor the compiler runtime'"'"'s$' "$dir/both.log" ||
  fail "nostd-symbols failed without naming fmod and its object (log: $dir/both.log)"
if grep -qF 'libclean.rlib' "$dir/both.log"; then
  fail "nostd-symbols named the clean library (log: $dir/both.log)"
fi
grep -q '^nostd-symbols: 1 undefined symbol(s) of 2 libraries outside Rust and the compiler runtime' "$dir/both.log" ||
  fail "nostd-symbols failed without its count line (log: $dir/both.log)"

echo "$name: ok"

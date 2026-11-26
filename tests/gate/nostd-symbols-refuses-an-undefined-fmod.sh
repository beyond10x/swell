#!/bin/sh
# The task nostd-symbols refuses a library built for the target without std that leaves fmod
# undefined, naming the symbol and the object: a no_std crate whose f64 % f64 the compiler lowers
# to a call of the C library's fmod. That crate is not named to nostd-build: it is a dependency
# of the one named, as swell-num is of swell, and nostd-build lists it all the same, so a crate
# added below swell is read with no list to edit. The named crate adds two f64, copies a slice
# into a Vec and compares two slices; it leaves only Rust's own symbols, memory functions and
# routines of the compiler's runtime undefined, and passes alone.
# Run from the repository root: sh tests/gate/nostd-symbols-refuses-an-undefined-fmod.sh
# Works only under .out/gate-tests/<name>/, which holds the planted workspace, its cargo target
# directory and every output of the two tasks (NOSTD_OUT); prints "<name>: ok" or
# "<name>: FAIL <why>".
#
# The planted workspace has two members, crates/top and crates/planted-fmod, top depending on
# planted-fmod by path; its Cargo.lock is made for it (cargo generate-lockfile --offline).

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=nostd-symbols-refuses-an-undefined-fmod
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/ws/crates/top/src" "$dir/ws/crates/planted-fmod/src" || fail "cannot make $dir"
ws=$dir/ws
printf '[workspace]\nmembers = ["crates/top", "crates/planted-fmod"]\nresolver = "3"\n' > "$ws/Cargo.toml" ||
  fail "cannot write the planted Cargo.toml"
printf '[package]\nname = "top"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nplanted-fmod = { path = "../planted-fmod" }\n' > "$ws/crates/top/Cargo.toml" ||
  fail "cannot write the planted crates/top/Cargo.toml"
printf '[package]\nname = "planted-fmod"\nversion = "0.1.0"\nedition = "2024"\n' > "$ws/crates/planted-fmod/Cargo.toml" ||
  fail "cannot write the planted crates/planted-fmod/Cargo.toml"
printf '%s\n' '#![no_std]' '#![forbid(unsafe_code)]' '//! A planted engine crate with nothing from a C library.' '' \
  'extern crate alloc;' '' \
  'pub use planted_fmod::rem;' '' \
  '/// The sum of two f64, a routine of the compiler runtime on a target without a floating-point unit.' \
  'pub fn add(a: f64, b: f64) -> f64 {' '    a + b' '}' '' \
  '/// A copy of a slice: memcpy and the allocator.' \
  'pub fn copy(bytes: &[u8]) -> alloc::vec::Vec<u8> {' '    bytes.to_vec()' '}' '' \
  '/// Whether two slices are equal: memcmp or bcmp.' \
  'pub fn same(a: &[u8], b: &[u8]) -> bool {' '    a == b' '}' > "$ws/crates/top/src/lib.rs" ||
  fail "cannot write crates/top/src/lib.rs"
printf '%s\n' '#![no_std]' '#![forbid(unsafe_code)]' '//! A planted engine crate whose remainder calls fmod.' '' \
  '/// The remainder of two f64, which the compiler lowers to a call of fmod.' \
  'pub fn rem(a: f64, b: f64) -> f64 {' '    a % b' '}' > "$ws/crates/planted-fmod/src/lib.rs" ||
  fail "cannot write crates/planted-fmod/src/lib.rs"
(cd "$ws" && cargo generate-lockfile --offline) > "$dir/lock.log" 2>&1 ||
  fail "cannot make a Cargo.lock for the planted workspace (log: $dir/lock.log)"

task -d . nostd-build NOSTD_ROOT="$ws" NOSTD_PACKAGES=top NOSTD_OUT="$dir/out" > "$dir/build.log" 2>&1 ||
  fail "nostd-build failed on the planted workspace (log: $dir/build.log)"
grep -qx 'nostd-build: 2 libraries for x86_64-unknown-none: planted_fmod top' "$dir/build.log" ||
  fail "nostd-build did not list the dependency planted_fmod beside top (log: $dir/build.log)"
top=$(awk '$1 == "top" { print $2 }' "$dir/out/libs.txt")
[ -n "$top" ] && [ -f "$top" ] || fail "nostd-build listed no library of top in $dir/out/libs.txt"

# The library of top alone passes, and the scan read something.
task -d . nostd-symbols NOSTD_LIBS="$top" NOSTD_OUT="$dir/top" > "$dir/top.log" 2>&1 ||
  fail "nostd-symbols refused the library of top (log: $dir/top.log)"
grep -q '^nostd-symbols: 1 library, [1-9][0-9]* object(s), [1-9][0-9]* undefined symbol(s): ' "$dir/top.log" ||
  fail "nostd-symbols passed without reading an undefined symbol of top (log: $dir/top.log)"

# Both, as the list of nostd-build gives them: fmod is named with its object, and nothing of top.
if task -d . nostd-symbols NOSTD_OUT="$dir/out" > "$dir/both.log" 2>&1; then
  fail "nostd-symbols accepted a library that leaves fmod undefined (log: $dir/both.log)"
fi
grep -qE '^nostd-symbols: libplanted_fmod(-[0-9a-f]+)?\.rlib\([^)]*\.o\): fmod is neither Rust'"'"'s own nor the compiler runtime'"'"'s$' "$dir/both.log" ||
  fail "nostd-symbols failed without naming fmod and its object (log: $dir/both.log)"
if grep -qF 'libtop.rlib' "$dir/both.log"; then
  fail "nostd-symbols named the library of top (log: $dir/both.log)"
fi
grep -q '^nostd-symbols: 1 undefined symbol(s) of 2 libraries outside Rust and the compiler runtime' "$dir/both.log" ||
  fail "nostd-symbols failed without its count line (log: $dir/both.log)"

echo "$name: ok"

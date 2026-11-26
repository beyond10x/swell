#!/bin/sh
# The task nostd-build refuses a crate that declares #![no_std] and still names a std:: path in
# its library code: for the target without std the path does not resolve. The same function
# written with the alloc:: path builds, so the refusal comes from the std path.
# Run from the repository root: sh tests/gate/nostd-build-refuses-a-std-path.sh
# Works only under .out/gate-tests/<name>/, which holds the planted workspace and its cargo target
# directory; prints "<name>: ok" or "<name>: FAIL <why>".
#
# The planted workspace has one member, crates/planted, with no dependency; its Cargo.lock is
# made for it (cargo generate-lockfile --offline), and the task builds with --locked.

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=nostd-build-refuses-a-std-path
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/ws/crates/planted/src" || fail "cannot make $dir"
ws=$dir/ws
printf '[workspace]\nmembers = ["crates/planted"]\nresolver = "3"\n' > "$ws/Cargo.toml" ||
  fail "cannot write the planted Cargo.toml"
printf '[package]\nname = "planted"\nversion = "0.1.0"\nedition = "2024"\n' > "$ws/crates/planted/Cargo.toml" ||
  fail "cannot write the planted crates/planted/Cargo.toml"

lib() {
  printf '#![no_std]\n#![forbid(unsafe_code)]\n//! A planted engine crate.\n\nextern crate alloc;\n\n/// A text.\npub fn text() -> %s::string::String {\n    %s::string::String::new()\n}\n' "$1" "$1" > "$ws/crates/planted/src/lib.rs" ||
    fail "cannot write the planted lib.rs"
}

# With alloc::, the build passes.
lib alloc
(cd "$ws" && cargo generate-lockfile --offline) > "$dir/lock.log" 2>&1 ||
  fail "cannot make a Cargo.lock for the planted workspace (log: $dir/lock.log)"
task -d . nostd-build NOSTD_ROOT="$ws" NOSTD_PACKAGES=planted NOSTD_OUT="$dir/alloc" > "$dir/alloc.log" 2>&1 ||
  fail "nostd-build refused a crate that names alloc::string::String (log: $dir/alloc.log)"
grep -qx 'nostd-build: 1 library for x86_64-unknown-none: planted' "$dir/alloc.log" ||
  fail "nostd-build did not list the planted library (log: $dir/alloc.log)"

# With std::, it fails, and the compiler names std.
lib std
if task -d . nostd-build NOSTD_ROOT="$ws" NOSTD_PACKAGES=planted NOSTD_OUT="$dir/std" > "$dir/std.log" 2>&1; then
  fail "nostd-build accepted a std:: path in a no_std crate (log: $dir/std.log)"
fi
grep -q '^error\[E0433\].*`std`' "$dir/std.log" ||
  fail "nostd-build failed without the compiler's error that std does not resolve (log: $dir/std.log)"
grep -q '^nostd-build: cargo build --target x86_64-unknown-none of planted failed' "$dir/std.log" ||
  fail "nostd-build failed without its own last line (log: $dir/std.log)"

echo "$name: ok"

#!/bin/sh
# The task nostd-build builds a crate whose root declares #![no_std] for the target without std
# and lists its library, and refuses the same crate with that line taken out: for the bare target
# the compiler then looks for the crate std, which the target does not have.
# Run from the repository root: sh tests/gate/nostd-build-refuses-a-crate-root-without-no-std.sh
# Works only under .out/gate-tests/<name>/, which holds the planted workspace and its cargo target
# directory; prints "<name>: ok" or "<name>: FAIL <why>".
#
# The planted workspace has one member, crates/planted, with no dependency; its Cargo.lock is
# made for it (cargo generate-lockfile --offline), and the task builds with --locked.

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=nostd-build-refuses-a-crate-root-without-no-std
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/ws/crates/planted/src" || fail "cannot make $dir"
ws=$dir/ws
printf '[workspace]\nmembers = ["crates/planted"]\nresolver = "3"\n' > "$ws/Cargo.toml" ||
  fail "cannot write the planted Cargo.toml"
printf '[package]\nname = "planted"\nversion = "0.1.0"\nedition = "2024"\n' > "$ws/crates/planted/Cargo.toml" ||
  fail "cannot write the planted crates/planted/Cargo.toml"

# With the line, the build passes and lists the library.
printf '#![no_std]\n#![forbid(unsafe_code)]\n//! A planted engine crate.\n\n/// One.\npub fn one() -> u32 {\n    1\n}\n' > "$ws/crates/planted/src/lib.rs" ||
  fail "cannot write the planted lib.rs"
(cd "$ws" && cargo generate-lockfile --offline) > "$dir/lock.log" 2>&1 ||
  fail "cannot make a Cargo.lock for the planted workspace (log: $dir/lock.log)"
task -d . nostd-build NOSTD_ROOT="$ws" NOSTD_PACKAGES=planted NOSTD_OUT="$dir/with" > "$dir/with.log" 2>&1 ||
  fail "nostd-build refused a crate that declares #![no_std] (log: $dir/with.log)"
grep -qx 'nostd-build: 1 library for x86_64-unknown-none: planted' "$dir/with.log" ||
  fail "nostd-build did not list the planted library (log: $dir/with.log)"
awk '$1 == "planted" && $2 ~ /\/libplanted\.rlib$/ { found = 1 } END { exit !found }' "$dir/with/libs.txt" ||
  fail "nostd-build wrote no line for the planted library into $dir/with/libs.txt"

# Without it, the build fails, and the task says that the crate needs std.
printf '#![forbid(unsafe_code)]\n//! A planted engine crate without #![no_std].\n\n/// One.\npub fn one() -> u32 {\n    1\n}\n' > "$ws/crates/planted/src/lib.rs" ||
  fail "cannot write the planted lib.rs"
if task -d . nostd-build NOSTD_ROOT="$ws" NOSTD_PACKAGES=planted NOSTD_OUT="$dir/without" > "$dir/without.log" 2>&1; then
  fail "nostd-build accepted a crate root without #![no_std] (log: $dir/without.log)"
fi
grep -qF "can't find crate for \`std\`" "$dir/without.log" ||
  fail "nostd-build failed without the compiler's line that std is missing (log: $dir/without.log)"
grep -q '^nostd-build: cargo build --target x86_64-unknown-none of planted failed' "$dir/without.log" ||
  fail "nostd-build failed without its own last line (log: $dir/without.log)"
[ ! -e "$dir/without/libs.txt" ] || fail "nostd-build wrote a list of libraries for a build that failed"

echo "$name: ok"

#!/bin/sh
# Planted defects of the task test262-sources (taskfiles/test262.yml, story
# cli-runs-a-script-file): the digest of the sources of conformance/test262/expected.txt covers
# the packages of the runner, swell-test262 and what it depends on as Cargo.lock lists them, and
# no other crate. On a small tree of its own (GATE_ROOT, a git repository whose files are
# untracked, which the digest lists as well) with the crates swell, test262 (swell-test262,
# depending on swell) and cli (swell-cli, depending on swell), test262-check passes with the
# digest made from the tree, and:
#   1. a changed crates/cli/src/main.rs,
#   2. a changed crates/cli/Cargo.toml and
#   3. a changed [[package]] entry of swell-cli in Cargo.lock
# each leave the digest equal (test262-check passes); then
#   4. a changed crates/swell/src/lib.rs and
#   5. a changed [[package]] entry of swell in Cargo.lock
# each make it exit non-zero and print "test262-check: the sources digest <new> differs from
# <old> in <digest file>"; and
#   6. a Cargo.lock without swell-test262 makes test262-digest exit non-zero and say so.
# Run from the repository root: sh tests/gate/test262-check-covers-the-runner-and-not-the-shell.sh
# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
unset TEST262_FEATURES TEST262_SECTIONS TEST262_DESIGN TEST262_OUT TEST262_DIGEST GATE_ROOT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test262-check-covers-the-runner-and-not-the-shell
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

root=$dir/root
out=$dir/out
for crate in swell test262 cli; do
  mkdir -p "$root/crates/$crate/src" || fail "cannot make $root/crates/$crate/src"
done
git -C "$root" init -q || fail "git init failed in $root"
printf '[workspace]\nmembers = ["crates/*"]\n' > "$root/Cargo.toml"
printf '[package]\nname = "swell"\n' > "$root/crates/swell/Cargo.toml"
printf '[package]\nname = "swell-test262"\n\n[dependencies]\nswell = { path = "../swell" }\n' > "$root/crates/test262/Cargo.toml"
printf '[package]\nname = "swell-cli"\n\n[dependencies]\nswell = { path = "../swell" }\n' > "$root/crates/cli/Cargo.toml"
printf 'pub fn answer() -> u32 {\n    42\n}\n' > "$root/crates/swell/src/lib.rs"
printf 'fn main() {}\n' > "$root/crates/test262/src/main.rs"
printf 'fn main() {}\n' > "$root/crates/cli/src/main.rs"

# The lock with the dependencies of swell-cli given as $1 and those of swell as $2.
lock() {
  printf 'version = 4\n\n[[package]]\nname = "swell"\nversion = "0.1.0"%s\n\n' "$2"
  printf '[[package]]\nname = "swell-cli"\nversion = "0.1.0"\ndependencies = [\n "swell",%s\n]\n\n' "$1"
  printf '[[package]]\nname = "swell-test262"\nversion = "0.1.0"\ndependencies = [\n "swell",\n]\n'
}
lock '' '' > "$root/Cargo.lock"

rooted="GATE_ROOT=$root TEST262_OUT=$out"
# shellcheck disable=SC2086
task -d . test262-digest $rooted > "$dir/digest.out" 2>&1 ||
  fail "task test262-digest on $root exited non-zero ($dir/digest.out)"
digest=$(awk '$1 == "test262-sources" { print $2 }' "$dir/digest.out")
[ -n "$digest" ] || fail "task test262-digest printed no test262-sources line ($dir/digest.out)"
printf 'test262-sources %s\ntest262-refused 0\n' "$digest" > "$dir/expected.digest"
grep -q 'crates/swell/src/lib.rs$' "$out/sources.txt" ||
  fail "the sources of $root do not list crates/swell/src/lib.rs ($out/sources.txt)"
grep -q 'crates/cli/' "$out/sources.txt" &&
  fail "the sources of $root list a file of crates/cli ($out/sources.txt)"

# equal <case> <what changed>: test262-check passes with the digest unchanged.
equal() {
  # shellcheck disable=SC2086
  task -d . test262-check $rooted TEST262_DIGEST="$dir/expected.digest" > "$dir/$1.out" 2>&1 ||
    fail "task test262-check exited non-zero with $2 ($dir/$1.out)"
  grep -qF "digest $digest equals $dir/expected.digest" "$dir/$1.out" ||
    fail "task test262-check printed no equal digest line with $2 ($dir/$1.out)"
}

# differs <case> <what changed>: test262-check fails and names both digests.
differs() {
  # shellcheck disable=SC2086
  if task -d . test262-check $rooted TEST262_DIGEST="$dir/expected.digest" > "$dir/$1.out" 2>&1; then
    fail "task test262-check exited 0 with $2 ($dir/$1.out)"
  fi
  grep -qE "^test262-check: the sources digest [0-9a-f]{64} differs from $digest in $dir/expected.digest" "$dir/$1.out" ||
    fail "task test262-check did not name both digests with $2 ($dir/$1.out)"
}

equal base "the tree as it was made"
printf '// changed\n' >> "$root/crates/cli/src/main.rs"
equal cli-source "a changed crates/cli/src/main.rs"
printf '\n[features]\nplanted = []\n' >> "$root/crates/cli/Cargo.toml"
equal cli-manifest "a changed crates/cli/Cargo.toml"
lock '
 "swell-syntax",' '' > "$root/Cargo.lock"
equal cli-lock "a changed entry of swell-cli in Cargo.lock"

printf '// changed\n' >> "$root/crates/swell/src/lib.rs"
differs swell-source "a changed crates/swell/src/lib.rs"
head -n 3 "$root/crates/swell/src/lib.rs" > "$dir/lib.rs" && mv "$dir/lib.rs" "$root/crates/swell/src/lib.rs" ||
  fail "cannot restore $root/crates/swell/src/lib.rs"
equal swell-restored "crates/swell/src/lib.rs restored"
lock '
 "swell-syntax",' '
dependencies = [
 "swell-num",
]' > "$root/Cargo.lock"
differs swell-lock "a changed entry of swell in Cargo.lock"

printf 'version = 4\n\n[[package]]\nname = "swell"\nversion = "0.1.0"\n' > "$root/Cargo.lock"
# shellcheck disable=SC2086
if task -d . test262-digest $rooted > "$dir/no-runner.out" 2>&1; then
  fail "task test262-digest exited 0 with a Cargo.lock without swell-test262 ($dir/no-runner.out)"
fi
grep -q "names no package swell-test262" "$dir/no-runner.out" ||
  fail "task test262-digest did not say the lock names no swell-test262 ($dir/no-runner.out)"

echo "$name: ok"

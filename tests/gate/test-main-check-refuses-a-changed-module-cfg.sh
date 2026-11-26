#!/bin/sh
# Planted defect of taskfiles/test-main.sh check for a package named by its arguments (adversary
# pass 1 on unit lanebuilds; story realm-size-lane-within-its-share): the gate's builds compile
# every module of tests/main.rs only through the arm not(test_modules) of the cfg line before
# each module, and the stack builds compile a listed module only through its own name there.
# Nothing is built: the script plants a crate named planted with two test files and support,
# generates its tests/main.rs with "write planted", and changes one line of it.
#   1. As generated: check passes; the file has no line naming unexpected_cfgs and one line
#      #[cfg(any(not(test_modules), test_modules = "<m>"))] right before "mod <m>;" for first
#      and second, and none before "mod support;".
#   2. The cfg line of first names second: refused, "differs from its generation".
#   3. The cfg line of first without the arm not(test_modules): refused.
#   4. The cfg line of first removed: refused.
#   5. The cfg line put before mod support: refused.
#   6. No variable chooses the package (correction 1, F5): check without the argument, with
#      TEST_MAIN_SELECT=planted exported, refuses the file generated for planted.
# Run from the repository root:
#   sh tests/gate/test-main-check-refuses-a-changed-module-cfg.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-main-check-refuses-a-changed-module-cfg
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/crates/planted/src" "$dir/crates/planted/tests/support" || fail "cannot make $dir"
[ -f taskfiles/test-main.sh ] || fail "taskfiles/test-main.sh is missing"
crate=$dir/crates/planted
main=$crate/tests/main.rs
printf '[package]\nname = "planted"\nversion = "0.0.0"\nedition = "2024"\nautotests = false\n\n[[test]]\nname = "main"\npath = "tests/main.rs"\n' \
  > "$crate/Cargo.toml" || fail "cannot write the planted manifest"
printf '#![forbid(unsafe_code)]\n' > "$crate/src/lib.rs" || fail "cannot write the planted lib.rs"
printf '#[test]\nfn one() {}\n' > "$crate/tests/first.rs" || fail "cannot write first.rs"
printf '#[test]\nfn two() {}\n' > "$crate/tests/second.rs" || fail "cannot write second.rs"
printf 'pub fn help() {}\n' > "$crate/tests/support/mod.rs" || fail "cannot write support/mod.rs"

GATE_CRATES=$dir/crates sh taskfiles/test-main.sh write planted > "$dir/write.out" 2>&1 ||
  fail "taskfiles/test-main.sh write failed ($dir/write.out)"
cp "$main" "$dir/main.rs.generated" || fail "cannot keep the generated tests/main.rs"
check() {
  GATE_CRATES=$dir/crates sh taskfiles/test-main.sh check planted > "$dir/$1.out" 2>&1
}
check generated || fail "check refused the generated tree ($dir/generated.out)"
grep -q 'unexpected_cfgs' "$main" && fail "the generated $main names the lint unexpected_cfgs"
for module in first second; do
  awk -v m="$module" '
    prev == "#[cfg(any(not(test_modules), test_modules = \"" m "\"))]" && $0 == "mod " m ";" { found = 1 }
    { prev = $0 }
    END { exit !found }
  ' "$main" || fail "the generated $main has no cfg line of $module right before mod $module;"
done
awk 'prev ~ /^#\[cfg/ && $0 == "mod support;" { bad = 1 } { prev = $0 } END { exit bad }' "$main" ||
  fail "the generated $main has a cfg line before mod support;"

refused() {
  out=$1
  what=$2
  sed "$3" "$dir/main.rs.generated" > "$main" || fail "cannot plant $what"
  cmp -s "$main" "$dir/main.rs.generated" && fail "the plant of $what changed nothing"
  if check "$out"; then fail "check accepted $what ($dir/$out.out)"; fi
  grep -qF "test-main: $main differs from its generation" "$dir/$out.out" ||
    fail "check did not name $main for $what ($dir/$out.out)"
  cp "$dir/main.rs.generated" "$main" || fail "cannot restore $main"
}
refused other "a cfg line of first that names second" \
  's/^#\[cfg(any(not(test_modules), test_modules = "first"))\]$/#[cfg(any(not(test_modules), test_modules = "second"))]/'
refused arm "a cfg line of first without the arm not(test_modules)" \
  's/^#\[cfg(any(not(test_modules), test_modules = "first"))\]$/#[cfg(test_modules = "first")]/'
refused missing "no cfg line before mod first" \
  '/^#\[cfg(any(not(test_modules), test_modules = "first"))\]$/d'
refused support "a cfg line before mod support" \
  's/^mod support;$/#[cfg(any(not(test_modules), test_modules = "support"))]\nmod support;/'

check restored || fail "check refused the restored tree ($dir/restored.out)"
if TEST_MAIN_SELECT=planted GATE_CRATES=$dir/crates sh taskfiles/test-main.sh check > "$dir/variable.out" 2>&1; then
  fail "check without the argument accepted the file generated for planted, with TEST_MAIN_SELECT=planted ($dir/variable.out)"
fi
grep -qF "test-main: $main differs from its generation" "$dir/variable.out" ||
  fail "check without the argument did not name $main ($dir/variable.out)"
echo "$name: ok"

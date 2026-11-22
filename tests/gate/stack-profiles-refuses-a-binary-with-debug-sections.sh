#!/bin/sh
# Planted defect of the check that a stack test binary of the profile release (and every test binary)
# carries no debug information (taskfiles/no-debug-sections.sh, called by the task
# test-stack-profiles of taskfiles/stack-profiles.yml; story
# stack-requirements-hold-in-the-release-profile, points 4 and 5). Nothing is built: the script
# copies the machine's true binary, which carries no debug section, and adds a section
# .debug_info to a second copy with objcopy --add-section. The check refuses that copy with the
# line "stack-profiles: <binary> has debug sections", also when the clean copy is named with it,
# and passes on the clean copy alone. It refuses a call that names no binary and a file that is
# not an ELF file too: a check that reads nothing fails.
# Run from the repository root: sh tests/gate/stack-profiles-refuses-a-binary-with-debug-sections.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=stack-profiles-refuses-a-binary-with-debug-sections
dir=.out/gate-tests/$name
check=taskfiles/no-debug-sections.sh
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
[ -f "$check" ] || fail "$check is missing"

true_bin=""
for candidate in /usr/bin/true /bin/true; do
  if [ -f "$candidate" ]; then true_bin=$candidate; break; fi
done
[ -n "$true_bin" ] || fail "no true binary in /usr/bin or /bin to copy"

clean=$dir/clean
planted=$dir/planted
cp "$true_bin" "$clean" || fail "cannot copy $true_bin to $clean"
chmod u+w "$clean" || fail "cannot make $clean writable"
if LC_ALL=C readelf -S -W "$clean" | grep -q ' \.debug_'; then
  fail "$true_bin carries debug sections already; the clean copy needs a binary without them"
fi
printf 'planted\n' > "$dir/section.bin" || fail "cannot write $dir/section.bin"
objcopy --add-section .debug_info="$dir/section.bin" "$clean" "$planted" ||
  fail "objcopy did not add a section .debug_info to a copy of $true_bin"
LC_ALL=C readelf -S -W "$planted" | grep -q ' \.debug_info ' ||
  fail "the planted copy $planted shows no section .debug_info"

if sh "$check" "$planted" > "$dir/planted.out" 2>&1; then
  fail "$check accepted $planted, which has a section .debug_info ($dir/planted.out)"
fi
grep -qxF "stack-profiles: $planted has debug sections" "$dir/planted.out" ||
  fail "$check did not print \"stack-profiles: $planted has debug sections\" ($dir/planted.out)"

if sh "$check" "$clean" "$planted" > "$dir/both.out" 2>&1; then
  fail "$check accepted $planted named after a clean binary ($dir/both.out)"
fi
grep -qxF "stack-profiles: $planted has debug sections" "$dir/both.out" ||
  fail "$check did not name $planted when named after a clean binary ($dir/both.out)"
if grep -qF "stack-profiles: $clean has debug sections" "$dir/both.out"; then
  fail "$check named the clean copy $clean ($dir/both.out)"
fi

sh "$check" "$clean" > "$dir/clean.out" 2>&1 ||
  fail "$check refused the clean copy $clean ($dir/clean.out)"

if sh "$check" > "$dir/none.out" 2>&1; then
  fail "$check passed with no binary named ($dir/none.out)"
fi

printf 'not an ELF file\n' > "$dir/text"
if sh "$check" "$dir/text" > "$dir/text.out" 2>&1; then
  fail "$check passed on a file that is not an ELF file ($dir/text.out)"
fi

echo "$name: ok"

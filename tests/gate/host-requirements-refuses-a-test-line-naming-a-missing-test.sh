#!/bin/sh
# Planted defect: a list whose line of HOST-2 names a test that the test list does not have.
# The task host-requirements-check must refuse it with exactly one fault line, the "not a test
# of the workspace" line of HOST-2. The valid set must pass first, with a test line naming a
# test of its list and a test line ending in * that matches one, so the refusal comes from the
# planted defect and not from test lines as such.
# The same output from task, cargo and ess on any machine: no colour, and no "::error" line
# that go-task adds under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=host-requirements-refuses-a-test-line-naming-a-missing-test
work=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

# The valid set: four requirements with a slice each; HOST-1 held by every test that starts
# with a prefix, HOST-2 held by one test, HOST-3 omitted with a reason while its slice 3 is
# delivered, HOST-4 pending; a marker naming HOST-2 on its continuation line and a marker
# naming the limit of ESS; a test list of three tests.
valid() {
  mkdir -p "$1/spec" || return 1
  printf '%s\n' \
    '# Host interface (fixture)' \
    '' \
    'A requirement is a line `**HOST-<n>** (<source>) <sentence> [slice <t>]`; HOST-2 is below.' \
    '' \
    '**HOST-1** (swell:D1) The first requirement. [slice 1]' \
    '' \
    '**HOST-2** (swell:D2) The second requirement. [slice 2]' \
    '' \
    '**HOST-3** (swell:D3) The third requirement. [slice 3]' \
    '' \
    '**HOST-4** (swell:D4) The fourth requirement. [slice 4]' >"$1/host-interface.md" || return 1
  printf '%s\n' \
    'HOST-1 test swell::lib::hooks::tests::host_one_*' \
    'HOST-2 test swell::values::host_two_is_met' \
    'HOST-3 omitted the fixture leaves it out on purpose' \
    'HOST-4 pending slice 4' >"$1/requirements.txt" || return 1
  printf '%s\n' \
    '# The slices delivered (fixture).' \
    '' \
    '3' >"$1/slices-delivered.txt" || return 1
  printf '%s\n' \
    'swell::lib::hooks::tests::host_one_default_a' \
    'swell::values::host_two_is_met' \
    'swell::doc::compile_fail::host_three_does_not_compile' >"$1/tests.txt" || return 1
  printf '%s\n' \
    'types:' \
    '  # UNMAPPED: a construct ESS cannot state, which the document states.' \
    '  # Carried by HOST-2.' \
    '  - name: a' \
    '  # UNMAPPED: an answer ESS requires and the interface does not have.' \
    '  # ESS limit.' \
    '  - name: b' >"$1/spec/host.yaml"
}
run() {
  task -d . -t taskfiles/host-requirements.yml host-requirements-check \
    HOST_REQ_DOC="$1/host-interface.md" HOST_REQ_LIST="$1/requirements.txt" HOST_REQ_SPEC="$1/spec" \
    HOST_REQ_DELIVERED="$1/slices-delivered.txt" HOST_REQ_TEST_LIST="$1/tests.txt" >"$2" 2>&1
}

rm -rf "$work" && mkdir -p "$work" || fail "cannot make $work"
[ -f taskfiles/host-requirements.yml ] || fail "no task: taskfiles/host-requirements.yml does not exist"

valid "$work/valid" || fail "cannot write the valid set"
run "$work/valid" "$work/valid.log" || fail "the valid set is refused: $(cat "$work/valid.log")"
summary='host-requirements: 4 requirement(s), 4 line(s): 2 test, 1 pending, 1 omitted; 2 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 4:1'
[ "$(cat "$work/valid.log")" = "$summary" ] || fail "the valid set printed: $(cat "$work/valid.log")"

valid "$work/planted" || fail "cannot write the planted set"
grep -vx 'swell::values::host_two_is_met' "$work/valid/tests.txt" >"$work/planted/tests.txt" \
  || fail "cannot write the test list"
grep -qx 'swell::values::host_two_is_met' "$work/planted/tests.txt" && fail "the test is still listed"
grep -qx 'HOST-2 test swell::values::host_two_is_met' "$work/planted/requirements.txt" \
  || fail "the test line is not there"

if run "$work/planted" "$work/planted.log"; then
  fail "the task exited 0 on a test line naming a missing test"
fi
faults=$(grep -v '^task: ' "$work/planted.log")
want='HOST-2: test swell::values::host_two_is_met is not a test of the workspace'
[ "$faults" = "$want" ] || fail "expected exactly '$want', got: $faults"
echo "$name: ok"

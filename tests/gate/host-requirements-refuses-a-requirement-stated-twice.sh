#!/bin/sh
# Planted defect: a document that states HOST-2 twice.
# The task host-requirements-check must refuse it with exactly one fault line, the "stated 2
# times" line of HOST-2. The valid set must pass first, so the refusal comes from the planted
# defect.
name=host-requirements-refuses-a-requirement-stated-twice
work=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

# The valid set: three requirements with a slice each; HOST-1 and HOST-2 pending for their
# slice, HOST-3 omitted with a reason while its slice 3 is delivered; a marker naming HOST-2 on
# its continuation line and a marker naming the limit of ESS.
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
    '**HOST-3** (swell:D3) The third requirement. [slice 3]' >"$1/host-interface.md" || return 1
  printf '%s\n' \
    'HOST-1 pending slice 1' \
    'HOST-2 pending slice 2' \
    'HOST-3 omitted the fixture leaves it out on purpose' >"$1/requirements.txt" || return 1
  printf '%s\n' \
    '# The slices delivered (fixture).' \
    '' \
    '3' >"$1/slices-delivered.txt" || return 1
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
    HOST_REQ_DELIVERED="$1/slices-delivered.txt" >"$2" 2>&1
}

rm -rf "$work" && mkdir -p "$work" || fail "cannot make $work"
[ -f taskfiles/host-requirements.yml ] || fail "no task: taskfiles/host-requirements.yml does not exist"

valid "$work/valid" || fail "cannot write the valid set"
run "$work/valid" "$work/valid.log" || fail "the valid set is refused: $(cat "$work/valid.log")"
summary='host-requirements: 3 requirement(s), 3 line(s): 0 test, 2 pending, 1 omitted; 2 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 1:1 2:1'
[ "$(cat "$work/valid.log")" = "$summary" ] || fail "the valid set printed: $(cat "$work/valid.log")"

valid "$work/planted" || fail "cannot write the planted set"
awk '{ print } /^\*\*HOST-2\*\*/ { print ""; print "**HOST-2** (swell:D2) The second requirement, stated again. [slice 2]" }' \
  "$work/valid/host-interface.md" >"$work/planted/host-interface.md" || fail "cannot state HOST-2 again"
[ "$(grep -c '^\*\*HOST-2\*\*' "$work/planted/host-interface.md")" = 2 ] || fail "HOST-2 is not stated twice"

if run "$work/planted" "$work/planted.log"; then
  fail "the task exited 0 on a document that states HOST-2 twice"
fi
faults=$(grep -v '^task: ' "$work/planted.log")
want='HOST-2: stated 2 times in host-interface.md'
[ "$faults" = "$want" ] || fail "expected exactly '$want', got: $faults"
echo "$name: ok"

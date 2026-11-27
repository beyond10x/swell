#!/bin/sh
# More planted forms for taskfiles/realm-room-tests.awk -v rule=stack (story
# realm-size-lane-within-its-share), beside slow-tests-stack-rule-finds-a-stack-test-in-every-form.sh.
# A test that runs on a bounded stack and is printed "other" loses the lane's only run of it at
# opt-level 0, unless the guard of the run at opt-level 1 fails it there (SWELL_NO_STACK_TESTS;
# Taskfile.yml, task slow-tests). Every test below reaches stack_with_room(.
# Forms the rule reads, each printed "stack":
# - a stack helper called with a turbofish whose type nests (generic_vec::<Vec<u8>>(..));
# - stack helpers as elements of an array of function pointers and as the value of a struct
#   field;
# - std::thread::spawn(local_deep) in a test that also has a field named local_deep (Opts {
#   local_deep: true }), and in one whose comment reads "for local_deep, ... in the parser";
# - an extern "C" fn helper and a pub(in crate::deep) fn helper;
# - in a module named, a test named like the helper of support it calls by its path
#   (crate::support::host::deep_stack()), and another test of that module that calls the same
#   path.
# Forms the rule does not read, each printed "other", each named by this script, and each listed
# at the head of taskfiles/realm-room-tests.awk with its label below; the guard catches them at
# run time:
# - "a helper imported by use": a helper of another test module taken in by use and called by
#   its bare name;
# - "a helper imported under another name": use ... as, of a helper of another module or of
#   stack_with_room itself;
# - "a const or static function pointer";
# - "a function pointer made by a cast": &(helper as fn()).
# The control a_spawned_body_alone must be printed "stack" (the fixture is read at all). A form
# the rule starts to read turns this script red until it moves to the first list.
# No test runs: the script writes modules deep.rs, named.rs and other.rs and a helper file
# support/host.rs under .out/gate-tests/<name>/ and reads the rule's lines for them.
# Run from the repository root: sh tests/gate/slow-tests-stack-rule-finds-a-stack-test-in-more-forms.sh
name=slow-tests-stack-rule-finds-a-stack-test-in-more-forms
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/support" || { echo "$name: FAIL cannot make $dir"; exit 1; }
[ -f taskfiles/realm-room-tests.awk ] || { echo "$name: FAIL run it from the repository root"; exit 1; }
began=$(date +%s.%N)

printf '%s\n' \
  'pub fn stack_with_room(bytes: usize) -> usize {' \
  '    bytes / 8 * 7' \
  '}' \
  '' \
  'pub fn deep_stack() {' \
  '    let _ = stack_with_room(1 << 20);' \
  '}' > "$dir/support/host.rs"

printf '%s\n' \
  'pub fn run_deep() {' \
  '    let _ = crate::support::host::stack_with_room(1 << 20);' \
  '}' > "$dir/other.rs"

printf '%s\n' \
  '#[test]' \
  'fn deep_stack() {' \
  '    crate::support::host::deep_stack();' \
  '}' \
  '' \
  '#[test]' \
  'fn another_test_of_the_file_calls_the_helper() {' \
  '    crate::support::host::deep_stack();' \
  '}' > "$dir/named.rs"

printf '%s\n' \
  'use crate::other::run_deep;' \
  'use crate::other::run_deep as go_deep;' \
  'use crate::support::host::stack_with_room as room;' \
  '' \
  '#[test]' \
  'fn an_alias_of_the_stack_helper() {' \
  '    let _ = room(1 << 20);' \
  '}' \
  '' \
  '#[test]' \
  'fn a_helper_of_another_module_by_use() {' \
  '    run_deep();' \
  '}' \
  '' \
  '#[test]' \
  'fn a_helper_of_another_module_by_use_as() {' \
  '    go_deep();' \
  '}' \
  '' \
  'fn local_deep() {' \
  '    let _ = stack_with_room(1 << 20);' \
  '}' \
  '' \
  'fn spacer_1() {}' \
  '' \
  'const DEEP: fn() = local_deep;' \
  '' \
  'fn spacer_2() {}' \
  '' \
  '#[test]' \
  'fn a_const_function_pointer() {' \
  '    DEEP();' \
  '}' \
  '' \
  'fn spacer_3() {}' \
  '' \
  'static DEEP_TOO: fn() = local_deep;' \
  '' \
  'fn spacer_4() {}' \
  '' \
  '#[test]' \
  'fn a_static_function_pointer() {' \
  '    DEEP_TOO();' \
  '}' \
  '' \
  'fn generic_vec<T>(v: Vec<T>) {' \
  '    let _ = stack_with_room(v.len());' \
  '}' \
  '' \
  '#[test]' \
  'fn a_generic_helper_with_a_nested_type() {' \
  '    generic_vec::<Vec<u8>>(vec![]);' \
  '}' \
  '' \
  'fn plain() {}' \
  '' \
  '#[test]' \
  'fn an_array_of_function_pointers() {' \
  '    for body in [local_deep, plain] {' \
  '        body();' \
  '    }' \
  '}' \
  '' \
  'fn spacer_5() {}' \
  '' \
  'struct Case {' \
  '    body: fn(),' \
  '}' \
  '' \
  'fn spacer_6() {}' \
  '' \
  '#[test]' \
  'fn a_function_pointer_in_a_struct_field() {' \
  '    let case = Case { body: local_deep };' \
  '    (case.body)();' \
  '}' \
  '' \
  '#[test]' \
  'fn a_function_pointer_by_reference() {' \
  '    let f: &fn() = &(local_deep as fn());' \
  '    f();' \
  '}' \
  '' \
  '#[test]' \
  'fn a_spawned_body_and_a_field_of_its_name() {' \
  '    let o = Opts { local_deep: true };' \
  '    assert!(o.local_deep);' \
  '    std::thread::spawn(local_deep).join().unwrap();' \
  '}' \
  '' \
  '#[test]' \
  'fn a_spawned_body_and_a_comment_for_it() {' \
  '    // for local_deep, the recursion in the parser' \
  '    std::thread::spawn(local_deep).join().unwrap();' \
  '}' \
  '' \
  '#[test]' \
  'fn a_spawned_body_alone() {' \
  '    std::thread::spawn(local_deep).join().unwrap();' \
  '}' \
  '' \
  'fn spacer_7() {}' \
  '' \
  'extern "C" fn extern_deep() {' \
  '    let _ = stack_with_room(1 << 20);' \
  '}' \
  '' \
  'fn spacer_8() {}' \
  '' \
  '#[test]' \
  'fn an_extern_helper() {' \
  '    extern_deep();' \
  '}' \
  '' \
  'fn spacer_9() {}' \
  '' \
  'pub(in crate::deep) fn restricted_deep() {' \
  '    let _ = stack_with_room(1 << 20);' \
  '}' \
  '' \
  'fn spacer_10() {}' \
  '' \
  '#[test]' \
  'fn a_restricted_helper() {' \
  '    restricted_deep();' \
  '}' > "$dir/deep.rs"

awk -v rule=stack -f taskfiles/realm-room-tests.awk "$dir/support/host.rs" "$dir/deep.rs" "$dir/named.rs" "$dir/other.rs" > "$dir/lines.txt" ||
  { echo "$name: FAIL the rule exited non-zero ($dir/lines.txt)"; exit 1; }
bad=0
grep -qx 'stack deep::a_spawned_body_alone' "$dir/lines.txt" ||
  { echo "$name: FAIL the control deep::a_spawned_body_alone is not printed stack: the fixture is not read ($dir/lines.txt)"; bad=1; }
for test in deep::a_generic_helper_with_a_nested_type \
  deep::an_array_of_function_pointers deep::a_function_pointer_in_a_struct_field \
  deep::a_spawned_body_and_a_field_of_its_name \
  deep::a_spawned_body_and_a_comment_for_it deep::an_extern_helper deep::a_restricted_helper \
  named::deep_stack named::another_test_of_the_file_calls_the_helper; do
  if ! grep -qx "stack $test" "$dir/lines.txt"; then
    echo "$name: FAIL $test runs on a bounded stack in a form the rule reads and is not printed stack: it would run at opt-level 1 only ($dir/lines.txt)"
    bad=1
  fi
done
# <test>=<label>: the forms the rule does not read, and the label the head of the rule lists.
for pair in "deep::a_helper_of_another_module_by_use=a helper imported by use" \
  "deep::a_helper_of_another_module_by_use_as=a helper imported under another name" \
  "deep::an_alias_of_the_stack_helper=a helper imported under another name" \
  "deep::a_const_function_pointer=a const or static function pointer" \
  "deep::a_static_function_pointer=a const or static function pointer" \
  "deep::a_function_pointer_by_reference=a function pointer made by a cast"; do
  test=${pair%%=*}
  label=${pair#*=}
  if grep -qx "stack $test" "$dir/lines.txt"; then
    echo "$name: FAIL $test is printed stack: the rule reads the form \"$label\" now; move the test to the forms it reads and the label out of the list at the head of taskfiles/realm-room-tests.awk"
    bad=1
  elif ! grep -qx "other $test" "$dir/lines.txt"; then
    echo "$name: FAIL $test is not printed ($dir/lines.txt)"
    bad=1
  else
    echo "$name: not read by the rule, caught by the guard at run time: $test ($label)"
  fi
  awk '/^[^#]/ { exit } { print }' taskfiles/realm-room-tests.awk | tr '\n' ' ' | tr -s ' #' '  ' | grep -qF -- "- $label" ||
    { echo "$name: FAIL the head of taskfiles/realm-room-tests.awk does not list the form \"- $label\" the rule does not read"; bad=1; }
done
echo "$name: $(awk -v a="$began" -v b="$(date +%s.%N)" 'BEGIN { printf "%.2f", b - a }') s"
[ "$bad" = 0 ] || exit 1
echo "$name: ok"

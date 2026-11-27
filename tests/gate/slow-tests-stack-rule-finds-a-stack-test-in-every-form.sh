#!/bin/sh
# Planted forms for taskfiles/realm-room-tests.awk -v rule=stack, which the task slow-tests reads
# to run a test of swell::main at opt-level 0 (printed "stack", or not printed) or at opt-level 1
# only (printed "other"; story realm-size-lane-within-its-share). A test that runs on a bounded
# stack and is printed "other" loses the lane's only run of it at opt-level 0.
# No test runs: the script writes a module deep.rs, a module other.rs and a helper file
# support/host.rs under .out/gate-tests/<name>/ and reads the rule's lines for them.
# Each of these must not be printed "other": a helper called through two nested fns, a closure,
# a test in a mod inside the file, an attribute between #[test] and fn, a helper of support that
# calls on_bounded_stack, a helper of another test module called by its path
# (crate::other::run_deep()), a top-level helper whose name an earlier nested fn of the file also
# has (the rule counts every definition of a name), a helper reached through a function pointer
# (let f: fn() = run;). The test a macro writes is not printed. Three tests must be printed
# "other", in two modules: one that calls only a helper without a stack, one whose own binding
# has the name of the stack helper (let run = 3; ... (run, 3)), and a test of other.rs beside
# its stack helper, so that the rule marks neither every name nor every test of a module.
# Run from the repository root: sh tests/gate/slow-tests-stack-rule-finds-a-stack-test-in-every-form.sh
name=slow-tests-stack-rule-finds-a-stack-test-in-every-form
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/support" || { echo "$name: FAIL cannot make $dir"; exit 1; }
[ -f taskfiles/realm-room-tests.awk ] || { echo "$name: FAIL run it from the repository root"; exit 1; }

printf '%s\n' \
  'pub fn stack_with_room(bytes: usize) -> usize {' \
  '    bytes / 8 * 7' \
  '}' \
  '' \
  'pub fn on_bounded_stack(stack: usize, body: fn()) {' \
  '    std::thread::Builder::new()' \
  '        .stack_size(stack)' \
  '        .spawn(body)' \
  '        .unwrap()' \
  '        .join()' \
  '        .unwrap();' \
  '}' \
  '' \
  'pub fn deep_on_bounded_stack(body: fn()) {' \
  '    on_bounded_stack(stack_with_room(1 << 20), body);' \
  '}' > "$dir/support/host.rs"

printf '%s\n' \
  'pub fn run_deep() {' \
  '    on_bounded_stack(stack_with_room(1 << 20), || {});' \
  '}' \
  '' \
  '#[test]' \
  'fn a_test_beside_the_helper() {' \
  '    assert_eq!(1 + 1, 2);' \
  '}' > "$dir/other.rs"

printf '%s\n' \
  '#[test]' \
  'fn two_nested() {' \
  '    fn outer() {' \
  '        fn inner() {' \
  '            let _ = stack_with_room(1 << 20);' \
  '        }' \
  '        inner();' \
  '    }' \
  '    std::thread::spawn(outer).join().unwrap();' \
  '}' \
  '' \
  '#[test]' \
  'fn a_closure() {' \
  '    let body = || {' \
  '        let _ = stack_with_room(1 << 20);' \
  '    };' \
  '    body();' \
  '}' \
  '' \
  'macro_rules! stack_test {' \
  '    ($name:ident) => {' \
  '        #[test]' \
  '        fn $name() {' \
  '            let _ = stack_with_room(1 << 20);' \
  '        }' \
  '    };' \
  '}' \
  'stack_test!(by_a_macro);' \
  '' \
  'mod inner {' \
  '    #[test]' \
  '    fn in_a_mod() {' \
  '        let _ = super::stack_with_room(1 << 20);' \
  '    }' \
  '}' \
  '' \
  '#[test]' \
  '#[ignore = "an attribute between"]' \
  'fn an_attribute_between() {' \
  '    let _ = stack_with_room(1 << 20);' \
  '}' \
  '' \
  '#[test]' \
  'fn through_support() {' \
  '    deep_on_bounded_stack(plain);' \
  '}' \
  '' \
  'fn plain() {}' \
  '' \
  '#[test]' \
  'fn through_another_test_module() {' \
  '    crate::other::run_deep();' \
  '}' \
  '' \
  '#[test]' \
  'fn shadowed_first() {' \
  '    fn run() {' \
  '        plain();' \
  '    }' \
  '    run();' \
  '}' \
  '' \
  'fn run() {' \
  '    on_bounded_stack(stack_with_room(1 << 20), plain);' \
  '}' \
  '' \
  '#[test]' \
  'fn shadowed_second() {' \
  '    run();' \
  '}' \
  '' \
  '#[test]' \
  'fn a_function_pointer() {' \
  '    let f: fn() = run;' \
  '    f();' \
  '}' \
  '' \
  '#[test]' \
  'fn not_on_a_stack() {' \
  '    plain();' \
  '}' \
  '' \
  '#[test]' \
  'fn a_binding_named_like_the_helper() {' \
  '    let run = 3;' \
  '    assert_eq!(run, 3);' \
  '}' > "$dir/deep.rs"

awk -v rule=stack -f taskfiles/realm-room-tests.awk "$dir/support/host.rs" "$dir/deep.rs" "$dir/other.rs" > "$dir/lines.txt" ||
  { echo "$name: FAIL the rule exited non-zero ($dir/lines.txt)"; exit 1; }
bad=0
for test in two_nested a_closure in_a_mod an_attribute_between through_support \
  through_another_test_module shadowed_second a_function_pointer; do
  if ! grep -qx "stack deep::$test" "$dir/lines.txt"; then
    echo "$name: FAIL deep::$test runs on a bounded stack and is not printed stack: it would run at opt-level 1 only ($dir/lines.txt)"
    bad=1
  fi
done
for test in deep::not_on_a_stack deep::a_binding_named_like_the_helper other::a_test_beside_the_helper; do
  if ! grep -qx "other $test" "$dir/lines.txt"; then
    echo "$name: FAIL $test runs on no bounded stack and is not printed other ($dir/lines.txt)"
    bad=1
  fi
done
grep -q 'by_a_macro' "$dir/lines.txt" && { echo "$name: FAIL the test a macro writes is printed ($dir/lines.txt)"; bad=1; }
[ "$bad" = 0 ] || exit 1
echo "$name: ok"

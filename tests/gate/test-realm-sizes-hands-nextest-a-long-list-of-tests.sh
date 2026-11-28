#!/bin/sh
# Planted defect of the task test-realm-sizes (Taskfile.yml; stories
# realm-size-lane-within-its-share, point 3, and slow-lane-holds-its-bound-on-the-slower-runner,
# step 2): the task hands nextest the tests that taskfiles/realm-room-tests.awk prints as
# "other" as one filter expression, a single argument "binary_id(swell::main) & not
# (test(=<m>::<t>) | ...)". Linux refuses an exec whose single argument is over 131,072 bytes
# (MAX_ARG_STRLEN, E2BIG), so the task must still run when the integration-test binary has more
# such tests than fit there; the same holds for the lists and the configurations of the two
# parts of the default mode that the task test-split writes for it.
# No test runs: the script plants a tree of its own (.out/gate-tests/<name>/run-<n>) with the
# repository's Taskfile.yml and taskfiles/, a crates/swell/tests/ of one test that reads the
# room, the test TEST_OPT1_PROFILE_TEST names and N tests that read no room, each name about as
# long as the mean of the real ones (66 characters with the module, the mean 65.4), and the stub
# cargo of tests/gate/fixtures/cargo-stub.sh first on PATH, which answers each list and run from
# its filters over those tests and logs, for each, its longest argument and the bytes of its
# environment.
# 1. N = 900 (the real binary has 996 on 355c789b): task test-realm-sizes REALM_SIZES=4 exits 0,
#    prints "realm-sizes: 1 of 902 tests ..." and "realm-sizes: 2 tests at opt-level 1, 0 at
#    opt-level 0" (the profile test runs in the default mode's part at opt-level 1; its part
#    at opt-level 0 has no test and does not run), and the stub ran the default mode at
#    opt-level 1 and the collect mode. This shows the planted tree and the stub work.
# 2. N = 2,000: the same must exit 0 too.
# 3. N = 20,000: the same, no argument of a list or a run over 4,096 bytes, the environment of
#    each run as large as with 900, and the files given with --config-file name every one of
#    them (see the step below).
# 4. A .out/realm-sizes-nextest.toml the task cannot write, and 5. no test at all: the task fails
#    before any run, naming the file.
# Run from the repository root: sh tests/gate/test-realm-sizes-hands-nextest-a-long-list-of-tests.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR REALM_SIZES REALM_COLLECT STUB_BUILD STUB_BUILD_MATCH STUB_COMPILE STUB_DROP STUB_DOUBLE STUB_FAIL
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-realm-sizes-hands-nextest-a-long-list-of-tests
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/bin" || { echo "$name: FAIL cannot make $dir"; exit 1; }
root=$(pwd)
profile=realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on

fail() {
  echo "$name: FAIL $*"
  exit 1
}

[ -f Taskfile.yml ] && [ -f taskfiles/realm-room-tests.awk ] || fail "run it from the repository root"
cp tests/gate/fixtures/cargo-stub.sh "$dir/bin/cargo" && cp tests/gate/fixtures/nextest-filter.awk "$dir/bin/" && chmod +x "$dir/bin/cargo" ||
  fail "cannot copy the stub cargo of tests/gate/fixtures"

plant() {
  n=$1
  run="$dir/run-$n"
  mkdir -p "$run/crates/swell/tests/support" "$run/.out" || fail "cannot make $run"
  cp "$root/Taskfile.yml" "$run/Taskfile.yml" || fail "cannot copy Taskfile.yml"
  cp -R "$root/taskfiles" "$run/taskfiles" || fail "cannot copy taskfiles"
  # Files the task files read when they load (taskfiles/spec.yml, taskfiles/test262*.yml).
  mkdir -p "$run/spec" "$run/conformance/test262" || fail "cannot make $run/spec"
  cp "$root/spec/ess-inputs.yaml" "$run/spec/" || fail "cannot copy spec/ess-inputs.yaml"
  cp "$root/conformance/test262/PIN" "$run/conformance/test262/" || fail "cannot copy conformance/test262/PIN"
  printf 'mod many;\nmod realm_padding;\n' > "$run/crates/swell/tests/main.rs"
  printf 'pub fn nothing() {}\n' > "$run/crates/swell/tests/support/host.rs"
  printf '#[test]\nfn the_profile_keeps_debug_assertions_and_overflow_checks_on() {\n    assert!(cfg!(debug_assertions));\n}\n' > "$run/crates/swell/tests/realm_padding.rs"
  awk -v n="$n" 'BEGIN {
    print "#[test]\nfn reads_the_room() {\n    let _ = agent.heap_bytes();\n}"
    for (i = 1; i <= n; i++) {
      t = sprintf("a_test_that_reads_no_room_and_has_a_name_of_mean_length_%04d", i)
      printf "\n#[test]\nfn %s() {\n    assert_eq!(1, 1);\n}\n", t
    }
  }' > "$run/crates/swell/tests/many.rs"
  awk -v n="$n" -v profile="$profile" 'BEGIN {
    print "swell::main many::reads_the_room"
    for (i = 1; i <= n; i++) printf "swell::main many::a_test_that_reads_no_room_and_has_a_name_of_mean_length_%04d\n", i
    print "swell::main " profile
  }' > "$run/stub-tests"
}

go() {
  n=$1
  plant "$n"
  : > "$dir/stub-$n.log"
  STUB_TESTS="$root/$dir/run-$n/stub-tests" STUB_LOG="$root/$dir/stub-$n.log" PATH="$root/$dir/bin:$PATH" task -d "$dir/run-$n" test-realm-sizes REALM_SIZES=4 > "$dir/run-$n.out" 2>&1
}

go 900 || fail "task test-realm-sizes with 900 tests that read no room exited non-zero; the planted tree does not work ($dir/run-900.out)"
grep -qF 'realm-sizes: 1 of 902 tests of swell::main' "$dir/run-900.out" || fail "test-realm-sizes did not count 1 of 902 tests ($dir/run-900.out)"
grep -qxF 'realm-sizes: 2 tests at opt-level 1, 0 at opt-level 0' "$dir/run-900.out" || fail "test-realm-sizes did not split its default mode into 2 tests at opt-level 1 and none at opt-level 0 ($dir/run-900.out)"
grep -qxF 'realm-sizes 4 default: 2 tests at opt-level 1, 0 at opt-level 0' "$dir/run-900.out" || fail "test-realm-sizes did not count the two parts of its default run at k = 4 ($dir/run-900.out)"
grep -q '^run profile=collect features=none .* config=realm-sizes-opt1-nextest.toml .* n=2 ' "$dir/stub-900.log" || fail "the stub ran no default mode at opt-level 1 of 2 tests ($dir/stub-900.log)"
grep -q '^run profile=collect features=collect-every-safe-point .* config=realm-sizes-nextest.toml .* n=1 ' "$dir/stub-900.log" || fail "the stub ran no collect mode of 1 test ($dir/stub-900.log)"
! grep -q '^run profile=test ' "$dir/stub-900.log" || fail "the stub ran a part at opt-level 0 with no test in it ($dir/stub-900.log)"

go 2000 || fail "task test-realm-sizes with 2000 tests that read no room exited non-zero: $(grep -m 1 -i 'argument list too long\|no Summary' "$dir/run-2000.out") ($dir/run-2000.out)"
grep -q '^run ' "$dir/stub-2000.log" || fail "the stub cargo was not called for the run with 2000 tests ($dir/run-2000.out)"

# 3. N = 20,000, with the repository's .config/nextest.toml in the planted tree: the task exits
#    0, no list or run takes an argument over 4,096 bytes, the environment of each run is the
#    same in bytes as with 900 tests (PWD, OLDPWD and the stub's own STUB_ variables left out),
#    the file given with --config-file to the collect mode names all 20,000 tests, keeps every
#    line of .config/nextest.toml and adds one default-filter line, and the files of the
#    default mode's two parts name them too.
mkdir -p "$dir/run-20000/.config" && cp "$root/.config/nextest.toml" "$dir/run-20000/.config/" || fail "cannot copy .config/nextest.toml"
go 20000 || fail "task test-realm-sizes with 20000 tests that read no room exited non-zero: $(grep -m 1 -i 'argument list too long\|no Summary\|realm-sizes: .*\.out/' "$dir/run-20000.out") ($dir/run-20000.out)"
grep -qF 'realm-sizes: 1 of 20002 tests of swell::main' "$dir/run-20000.out" || fail "test-realm-sizes did not count 1 of 20002 tests ($dir/run-20000.out)"
grep -qxF 'realm-sizes: 2 tests at opt-level 1, 0 at opt-level 0' "$dir/run-20000.out" || fail "test-realm-sizes did not split its default mode with 20000 tests ($dir/run-20000.out)"
for n in 900 2000 20000; do
  grep -q '^\(list\|run\) ' "$dir/stub-$n.log" || fail "the stub cargo logged no list or run with $n tests ($dir/stub-$n.log)"
  long=$(awk '/^(list|run) / && match($0, / argmax=[0-9]+/) { v = substr($0, RSTART + 8, RLENGTH - 8) + 0; if (v > m) m = v } END { print m + 0 }' "$dir/stub-$n.log")
  [ "$long" -le 4096 ] || fail "with $n tests the stub cargo saw an argument of $long bytes, over 4,096 ($dir/stub-$n.log)"
done
envs() { awk '/^run / && match($0, / envbytes=[0-9]+/) { print substr($0, RSTART + 10, RLENGTH - 10) }' "$1" | sort -u | tr '\n' ' '; }
e900=$(envs "$dir/stub-900.log")
e20000=$(envs "$dir/stub-20000.log")
[ -n "$e900" ] && [ "$e900" = "$e20000" ] || fail "the environment of a run grew with the tests: \"$e900\" bytes with 900, \"$e20000\" with 20000 ($dir/stub-900.log, $dir/stub-20000.log)"
cfg=$dir/run-20000/.out/realm-sizes-nextest.toml
[ "$(grep -c '^default-filter = ' "$cfg")" = 1 ] || fail "$cfg holds not one default-filter line"
[ "$(grep '^default-filter = ' "$cfg" | grep -o 'test(=' | wc -l | tr -d ' ')" = 20001 ] || fail "$cfg does not name the 20001 tests that do not run here (the 20000 and the profile test)"
grep -v '^default-filter = ' "$cfg" | cmp -s - "$root/.config/nextest.toml" || fail "$cfg is not .config/nextest.toml with one line added"
for part in opt1 opt0 all; do
  c=$dir/run-20000/.out/realm-sizes-$part-nextest.toml
  [ "$(grep -c '^default-filter = ' "$c")" = 1 ] && grep -v '^default-filter = ' "$c" | cmp -s - "$root/.config/nextest.toml" ||
    fail "$c is not .config/nextest.toml with one default-filter line added"
  grep '^default-filter = ' "$c" | grep -q 'test(=many::a_test_that_reads_no_room_and_has_a_name_of_mean_length_20000)' ||
    fail "$c does not name the last of the 20000 tests the default mode leaves out"
done

# 4. A file the task cannot write in place of .out/realm-sizes-nextest.toml (a directory there):
#    the task fails, says so naming the file, and the stub cargo runs no test. Its own planted
#    tree, of 10 tests that read no room.
plant 10
mkdir -p "$dir/run-10/.out/realm-sizes-nextest.toml" || fail "cannot plant the directory"
: > "$dir/stub-10.log"
if STUB_TESTS="$root/$dir/run-10/stub-tests" STUB_LOG="$root/$dir/stub-10.log" PATH="$root/$dir/bin:$PATH" task -d "$dir/run-10" test-realm-sizes REALM_SIZES=4 > "$dir/unwritable.out" 2>&1; then
  fail "task test-realm-sizes exited 0 when it cannot write .out/realm-sizes-nextest.toml ($dir/unwritable.out)"
fi
grep -q '^realm-sizes: cannot write \.out/realm-sizes-nextest\.toml' "$dir/unwritable.out" || fail "test-realm-sizes did not name .out/realm-sizes-nextest.toml ($dir/unwritable.out)"
! grep -q '^run ' "$dir/stub-10.log" || fail "the stub cargo ran tests although the list was not written ($dir/stub-10.log)"

# 5. No test at all in crates/swell/tests: the list is empty, and the task fails naming it
#    before any run.
plant 0
printf '// no test\n' > "$dir/run-0/crates/swell/tests/many.rs"
printf '// no test\n' > "$dir/run-0/crates/swell/tests/realm_padding.rs"
: > "$dir/stub-0.log"
if STUB_TESTS="$root/$dir/run-0/stub-tests" STUB_LOG="$root/$dir/stub-0.log" PATH="$root/$dir/bin:$PATH" task -d "$dir/run-0" test-realm-sizes REALM_SIZES=4 > "$dir/empty.out" 2>&1; then
  fail "task test-realm-sizes exited 0 with no test in crates/swell/tests ($dir/empty.out)"
fi
grep -q '^realm-sizes: \.out/realm-sizes-tests\.txt is empty' "$dir/empty.out" || fail "test-realm-sizes did not name the empty .out/realm-sizes-tests.txt ($dir/empty.out)"
! grep -q '^run ' "$dir/stub-0.log" || fail "the stub cargo ran tests with an empty list ($dir/stub-0.log)"

echo "$name: ok"

#!/bin/sh
# Planted defect of the task test-realm-sizes (Taskfile.yml; story
# realm-size-lane-within-its-share, point 3): the task hands nextest the tests that
# taskfiles/realm-room-tests.awk prints as "other" as one filter expression, a single argument
# "binary_id(swell::main) & not (test(=<m>::<t>) | ...)". Linux refuses an exec whose single
# argument is over 131,072 bytes (MAX_ARG_STRLEN, E2BIG), so the task must still run when the
# integration-test binary has more such tests than fit there.
# No test runs: the script plants a tree of its own (.out/gate-tests/<name>/run) with the
# repository's Taskfile.yml and taskfiles/, a crates/swell/tests/ of one test that reads the room
# and N that do not, each name about as long as the mean of the real ones (66 characters with the module, the mean 65.4), and a stub
# cargo first on PATH that answers --no-run with a Finished line and a run with a Summary line.
# 1. N = 900 (the real binary has 996 on 355c789b): task test-realm-sizes REALM_SIZES=4 exits 0
#    and the stub saw one -E argument. This shows the planted tree and the stub work.
# 2. N = 2,000: the same must exit 0 too, and the stub must have been called for the run.
# 3. N = 20,000: the same, no argument over 4,096 bytes, an environment no larger than with 900,
#    and the file given with --config-file names every one of them (see the step below).
# 4. A .out/realm-sizes-nextest.toml the task cannot write, and 5. no test at all: the task fails
#    before any run, naming the file.
# Run from the repository root: sh tests/gate/test-realm-sizes-hands-nextest-a-long-list-of-tests.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR REALM_SIZES REALM_COLLECT
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=test-realm-sizes-hands-nextest-a-long-list-of-tests
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/bin" || { echo "$name: FAIL cannot make $dir"; exit 1; }
root=$(pwd)

fail() {
  echo "$name: FAIL $*"
  exit 1
}

[ -f Taskfile.yml ] && [ -f taskfiles/realm-room-tests.awk ] || fail "run it from the repository root"

cat_stub() {
  printf '%s\n' '#!/bin/sh' \
    'log="$STUB_LOG"' \
    'for a in "$@"; do case "$a" in --no-run) echo "    Finished \`test\` profile [unoptimized] target(s) in 0.00s"; exit 0 ;; esac; done' \
    'prev=""; for a in "$@"; do if [ "$prev" = "-E" ]; then printf "run filter %s bytes\n" "${#a}" >> "$log"; fi; prev=$a; done' \
    'unset STUB_LOG' \
    'long=0; for a in "$@"; do [ "${#a}" -gt "$long" ] && long=${#a}; done; printf "run longest argument %s bytes\n" "$long" >> "$log"' \
    'env | grep -v "^\(PWD\|OLDPWD\)=" | awk "{ n += length(\$0) + 1; if (length(\$0) > m) m = length(\$0) } END { printf \"run environment %d bytes, longest entry %d\\n\", n, m }" >> "$log"' \
    'prev=""; for a in "$@"; do if [ "$prev" = "--config-file" ]; then printf "run config names %s tests\n" "$(grep "^default-filter = " "$a" | grep -o "test(=" | wc -l | tr -d " ")" >> "$log"; fi; prev=$a; done' \
    'echo "     Summary [   0.100s] 2 tests run: 2 passed, 0 skipped"' \
    'exit 0'
}
cat_stub > "$dir/bin/cargo" || fail "cannot write the stub cargo"
chmod +x "$dir/bin/cargo"

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
  printf 'mod many;\n' > "$run/crates/swell/tests/main.rs"
  printf 'pub fn nothing() {}\n' > "$run/crates/swell/tests/support/host.rs"
  awk -v n="$n" 'BEGIN {
    print "#[test]\nfn reads_the_room() {\n    let _ = agent.heap_bytes();\n}"
    for (i = 1; i <= n; i++) {
      t = sprintf("a_test_that_reads_no_room_and_has_a_name_of_mean_length_%04d", i)
      printf "\n#[test]\nfn %s() {\n    assert_eq!(1, 1);\n}\n", t
    }
  }' > "$run/crates/swell/tests/many.rs"
}

go() {
  n=$1
  plant "$n"
  : > "$dir/stub-$n.log"
  STUB_LOG="$root/$dir/stub-$n.log" PATH="$root/$dir/bin:$PATH" task -d "$dir/run-$n" test-realm-sizes REALM_SIZES=4 > "$dir/run-$n.out" 2>&1
}

go 900 || fail "task test-realm-sizes with 900 tests that read no room exited non-zero; the planted tree does not work ($dir/run-900.out)"
grep -q '^run filter [0-9]* bytes$' "$dir/stub-900.log" || fail "the stub cargo saw no -E argument with 900 tests ($dir/stub-900.log, $dir/run-900.out)"
grep -qF 'realm-sizes: 1 of 901 tests of swell::main' "$dir/run-900.out" || fail "test-realm-sizes did not count 1 of 901 tests ($dir/run-900.out)"

go 2000 || fail "task test-realm-sizes with 2000 tests that read no room exited non-zero: $(grep -m 1 -i 'argument list too long\|no Summary' "$dir/run-2000.out") ($dir/run-2000.out)"
grep -q '^run filter [0-9]* bytes$' "$dir/stub-2000.log" || fail "the stub cargo was not called for the run with 2000 tests ($dir/run-2000.out)"

# 3. N = 20,000, with the repository's .config/nextest.toml in the planted tree: the task exits
#    0, each run's argument list holds no argument over 4,096 bytes, its environment is the
#    same in bytes as with 900 tests (PWD and OLDPWD, which name the planted directory, and the
#    stub's own STUB_LOG left out), and the file given with --config-file names all 20,000
#    tests, keeps every line of .config/nextest.toml and adds one default-filter line.
mkdir -p "$dir/run-20000/.config" && cp "$root/.config/nextest.toml" "$dir/run-20000/.config/" || fail "cannot copy .config/nextest.toml"
go 20000 || fail "task test-realm-sizes with 20000 tests that read no room exited non-zero: $(grep -m 1 -i 'argument list too long\|no Summary\|realm-sizes: .*\.out/' "$dir/run-20000.out") ($dir/run-20000.out)"
grep -qF 'realm-sizes: 1 of 20001 tests of swell::main' "$dir/run-20000.out" || fail "test-realm-sizes did not count 1 of 20001 tests ($dir/run-20000.out)"
grep -q '^run filter [0-9]* bytes$' "$dir/stub-20000.log" || fail "the stub cargo was not called for the run with 20000 tests ($dir/run-20000.out)"
for n in 900 2000 20000; do
  long=$(awk '/^run longest argument / { if ($4 > m) m = $4 } END { print m + 0 }' "$dir/stub-$n.log")
  [ "$long" -le 4096 ] || fail "with $n tests the stub cargo saw an argument of $long bytes, over 4,096 ($dir/stub-$n.log)"
  grep -q '^run longest argument ' "$dir/stub-$n.log" || fail "the stub cargo logged no argument lengths with $n tests ($dir/stub-$n.log)"
done
e900=$(grep -m 1 '^run environment ' "$dir/stub-900.log")
e20000=$(grep -m 1 '^run environment ' "$dir/stub-20000.log")
[ -n "$e900" ] && [ "$e900" = "$e20000" ] || fail "the environment of a run grew with the tests: \"$e900\" with 900, \"$e20000\" with 20000 ($dir/stub-900.log, $dir/stub-20000.log)"
named=$(awk '/^run config names / { print $4; exit }' "$dir/stub-20000.log")
[ "$named" = 20000 ] || fail "the file given with --config-file names ${named:-no} tests, not the 20000 that do not run here ($dir/stub-20000.log)"
cfg=$dir/run-20000/.out/realm-sizes-nextest.toml
[ "$(grep -c '^default-filter = ' "$cfg")" = 1 ] || fail "$cfg holds not one default-filter line"
grep -v '^default-filter = ' "$cfg" | cmp -s - "$root/.config/nextest.toml" || fail "$cfg is not .config/nextest.toml with one line added"

# 4. A file the task cannot write in place of .out/realm-sizes-nextest.toml (a directory there):
#    the task fails, says so naming the file, and the stub cargo runs no test. Its own planted
#    tree, of 10 tests that read no room.
plant 10
mkdir -p "$dir/run-10/.out/realm-sizes-nextest.toml" || fail "cannot plant the directory"
: > "$dir/stub-10.log"
if STUB_LOG="$root/$dir/stub-10.log" PATH="$root/$dir/bin:$PATH" task -d "$dir/run-10" test-realm-sizes REALM_SIZES=4 > "$dir/unwritable.out" 2>&1; then
  fail "task test-realm-sizes exited 0 when it cannot write .out/realm-sizes-nextest.toml ($dir/unwritable.out)"
fi
grep -q '^realm-sizes: cannot write \.out/realm-sizes-nextest\.toml' "$dir/unwritable.out" || fail "test-realm-sizes did not name .out/realm-sizes-nextest.toml ($dir/unwritable.out)"
! grep -q '^run ' "$dir/stub-10.log" || fail "the stub cargo ran tests although the list was not written ($dir/stub-10.log)"

# 5. No test at all in crates/swell/tests: the list is empty, and the task fails naming it
#    before any run.
plant 0
printf '// no test\n' > "$dir/run-0/crates/swell/tests/many.rs"
: > "$dir/stub-0.log"
if STUB_LOG="$root/$dir/stub-0.log" PATH="$root/$dir/bin:$PATH" task -d "$dir/run-0" test-realm-sizes REALM_SIZES=4 > "$dir/empty.out" 2>&1; then
  fail "task test-realm-sizes exited 0 with no test in crates/swell/tests ($dir/empty.out)"
fi
grep -q '^realm-sizes: \.out/realm-sizes-tests\.txt is empty' "$dir/empty.out" || fail "test-realm-sizes did not name the empty .out/realm-sizes-tests.txt ($dir/empty.out)"
! grep -q '^run ' "$dir/stub-0.log" || fail "the stub cargo ran tests with an empty list ($dir/stub-0.log)"

echo "$name: ok"

#!/bin/sh
# Planted profiles the profile's test does not see (Taskfile.yml, tasks build-tests and
# test-opt1-profile-check; story gate-test-steps-hold-the-budget-on-the-slower-runner, item 3):
# the test TEST_OPT1_PROFILE_TEST holds that debug assertions and overflow checks are on in the
# profile collect of the step test's run at opt-level 1, but it reads the profile of its own
# crate only. A profile of one package ([profile.<name>.package.<...>]) or of build scripts
# ([profile.<name>.build-override]) goes unseen, and the profile collect is defined in
# .cargo/collect-profile.toml only. build-tests refuses such a table, dotted key or inline table
# in Cargo.toml or .cargo/config.toml, naming file and line, and such a CARGO_PROFILE_ variable
# in the environment, before any cargo runs.
# Nothing is built: the script plants a tree of its own (.out/gate-tests/<name>/run) with the
# repository's Taskfile.yml, taskfiles/, Cargo.toml and .cargo/, and a stub cargo first on PATH
# that logs each call and prints a Finished line.
# 0. The files as they are (and a comment that names a refused table): test-opt1-profile-check
#    exits 0 and prints nothing, and build-tests gets past it to the build at opt-level 1.
# 1. Cargo.toml + [profile.collect.package.swell-syntax]: build-tests fails, names the line.
# 2. .cargo/config.toml + [profile.test.package.swell-compiler]: the same.
# 3. Cargo.toml + [profile.test.build-override]: the same.
# 4. .cargo/config.toml + [profile.collect]: the same.
# 5. Cargo.toml + [profile] with the dotted key test.package.swell-unicode.debug-assertions: the
#    same.
# 6. .cargo/config.toml + [profile] with collect = { opt-level = 0 }: the same.
# 7. CARGO_PROFILE_TEST_BUILD_OVERRIDE_DEBUG_ASSERTIONS=false in the environment: fails, names it.
# In 1 to 7 the stub ran no cargo.
# Run from the repository root: sh tests/gate/build-tests-refuses-a-profile-its-profile-test-does-not-see.sh
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE CARGO_TARGET_DIR SWELL_FULL_COUNTS SWELL_NO_STACK_TESTS
for v in $(env | sed -n 's/^\(CARGO_PROFILE_[A-Za-z0-9_]*\)=.*/\1/p'); do unset "$v"; done
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=build-tests-refuses-a-profile-its-profile-test-does-not-see
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir/bin" || { echo "$name: FAIL cannot make $dir"; exit 1; }
root=$(pwd)

fail() {
  echo "$name: FAIL $*"
  exit 1
}

[ -f Taskfile.yml ] && [ -f Cargo.toml ] && [ -f .cargo/config.toml ] || fail "run it from the repository root"

printf '%s\n' '#!/bin/sh' \
  'printf "cargo %s\n" "$*" >> "$STUB_LOG"' \
  'echo "    Finished \`test\` profile [unoptimized] target(s) in 0.01s"' \
  'exit 0' > "$dir/bin/cargo" || fail "cannot write the stub cargo"
chmod +x "$dir/bin/cargo"

run="$dir/run"
mkdir -p "$run/.out" "$run/.config" "$run/.cargo" "$run/spec" "$run/conformance/test262" || fail "cannot make $run"
cp "$root/Taskfile.yml" "$run/Taskfile.yml" || fail "cannot copy Taskfile.yml"
cp -R "$root/taskfiles" "$run/taskfiles" || fail "cannot copy taskfiles"
cp "$root/.config/nextest.toml" "$run/.config/" || fail "cannot copy .config/nextest.toml"
# Files the task files read when they load (taskfiles/spec.yml, taskfiles/test262*.yml).
cp "$root/spec/ess-inputs.yaml" "$run/spec/" || fail "cannot copy spec/ess-inputs.yaml"
cp "$root/conformance/test262/PIN" "$run/conformance/test262/" || fail "cannot copy conformance/test262/PIN"

clean() {
  cp "$root/Cargo.toml" "$run/Cargo.toml" &&
    cp "$root/.cargo/config.toml" "$root/.cargo/collect-profile.toml" "$run/.cargo/" || fail "cannot copy Cargo.toml and .cargo/"
}

# plant <case> <file> <line> [<line> ...]: the clean files, the lines appended to <file>; then
# build-tests must fail, print "build-tests: <file>:<first line's number>: <first line>: ", and
# run no cargo.
plant() {
  case_name=$1; file=$2; shift 2
  clean
  at=$(($(wc -l < "$run/$file") + 1))
  first=$1
  printf '%s\n' "$@" >> "$run/$file" || fail "cannot plant $file"
  : > "$dir/stub-$case_name.log"
  STUB_LOG="$root/$dir/stub-$case_name.log" PATH="$root/$dir/bin:$PATH" task -d "$run" build-tests > "$dir/$case_name.out" 2>&1 &&
    fail "task build-tests exited 0 with $first in $file ($dir/$case_name.out)"
  grep -qF "build-tests: $file:$at: $first: " "$dir/$case_name.out" ||
    fail "task build-tests did not name $file:$at ($first) ($dir/$case_name.out)"
  [ ! -s "$dir/stub-$case_name.log" ] || fail "task build-tests ran cargo with $first in $file ($dir/stub-$case_name.log)"
}

# 0. The files as they are, with a comment that names a refused table.
clean
printf '%s\n' '# [profile.collect.package.swell-syntax] is not set here' >> "$run/Cargo.toml"
task -d "$run" test-opt1-profile-check > "$dir/check.out" 2>&1 ||
  fail "test-opt1-profile-check exited non-zero on the repository's Cargo.toml and .cargo/config.toml ($dir/check.out)"
[ ! -s "$dir/check.out" ] || fail "test-opt1-profile-check printed lines on the repository's files ($dir/check.out)"
: > "$dir/stub-clean.log"
STUB_LOG="$root/$dir/stub-clean.log" PATH="$root/$dir/bin:$PATH" task -d "$run" build-tests > "$dir/clean.out" 2>&1
grep -q -- '--cargo-profile collect' "$dir/stub-clean.log" ||
  fail "task build-tests did not get to the build at opt-level 1 on the repository's files ($dir/clean.out)"

plant package-collect Cargo.toml '[profile.collect.package.swell-syntax]' 'debug-assertions = false'
plant package-test .cargo/config.toml '[profile.test.package.swell-compiler]' 'overflow-checks = false'
plant build-override Cargo.toml '[profile.test.build-override]' 'debug-assertions = false'
plant collect .cargo/config.toml '[profile.collect]' 'opt-level = 0'
grep -qF 'the profile collect is defined in .cargo/collect-profile.toml only' "$dir/collect.out" ||
  fail "task build-tests did not say where the profile collect is defined ($dir/collect.out)"
grep -qF 'which the test realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on does not see' "$dir/package-test.out" ||
  fail "task build-tests did not name the test that does not see a profile of one package ($dir/package-test.out)"

# 5 and 6: the same places as a dotted key and as an inline table; the line named is the key's.
clean
at=$(($(wc -l < "$run/Cargo.toml") + 2))
printf '%s\n' '[profile]' 'test.package.swell-unicode.debug-assertions = false' >> "$run/Cargo.toml"
: > "$dir/stub-dotted.log"
STUB_LOG="$root/$dir/stub-dotted.log" PATH="$root/$dir/bin:$PATH" task -d "$run" build-tests > "$dir/dotted.out" 2>&1 &&
  fail "task build-tests exited 0 with a dotted key of a package's profile in Cargo.toml ($dir/dotted.out)"
grep -qF "build-tests: Cargo.toml:$at: test.package.swell-unicode.debug-assertions = false: " "$dir/dotted.out" ||
  fail "task build-tests did not name Cargo.toml:$at, the dotted key ($dir/dotted.out)"
[ ! -s "$dir/stub-dotted.log" ] || fail "task build-tests ran cargo with a dotted key of a package's profile ($dir/stub-dotted.log)"
clean
at=$(($(wc -l < "$run/.cargo/config.toml") + 2))
printf '%s\n' '[profile]' 'collect = { opt-level = 0 }' >> "$run/.cargo/config.toml"
: > "$dir/stub-inline.log"
STUB_LOG="$root/$dir/stub-inline.log" PATH="$root/$dir/bin:$PATH" task -d "$run" build-tests > "$dir/inline.out" 2>&1 &&
  fail "task build-tests exited 0 with an inline table of the profile collect in .cargo/config.toml ($dir/inline.out)"
grep -qF "build-tests: .cargo/config.toml:$at: collect = { opt-level = 0 }: " "$dir/inline.out" ||
  fail "task build-tests did not name .cargo/config.toml:$at, the inline table ($dir/inline.out)"
[ ! -s "$dir/stub-inline.log" ] || fail "task build-tests ran cargo with an inline table of the profile collect ($dir/stub-inline.log)"

# 7. A profile of build scripts from the environment.
clean
: > "$dir/stub-env.log"
CARGO_PROFILE_TEST_BUILD_OVERRIDE_DEBUG_ASSERTIONS=false STUB_LOG="$root/$dir/stub-env.log" PATH="$root/$dir/bin:$PATH" task -d "$run" build-tests > "$dir/env.out" 2>&1 &&
  fail "task build-tests exited 0 with CARGO_PROFILE_TEST_BUILD_OVERRIDE_DEBUG_ASSERTIONS in the environment ($dir/env.out)"
grep -qF 'build-tests: CARGO_PROFILE_TEST_BUILD_OVERRIDE_DEBUG_ASSERTIONS is set in the environment: ' "$dir/env.out" ||
  fail "task build-tests did not name CARGO_PROFILE_TEST_BUILD_OVERRIDE_DEBUG_ASSERTIONS ($dir/env.out)"
[ ! -s "$dir/stub-env.log" ] || fail "task build-tests ran cargo with CARGO_PROFILE_TEST_BUILD_OVERRIDE_DEBUG_ASSERTIONS in the environment ($dir/stub-env.log)"

echo "$name: ok"

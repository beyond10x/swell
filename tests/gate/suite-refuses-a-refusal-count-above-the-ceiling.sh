#!/bin/sh
# Planted defects of taskfiles/spec.yml. Run from the repository root with `sh`.
# 1. A refusal ceiling one below the number of refusals the suite synthesizes: `task suite`
#    must exit non-zero and name both numbers. Exported OUT and REFUSALS_MAX must change nothing.
#    On planted logs, `task suite-ceiling` passes 0 refusals under a ceiling of 0 and refuses 2
#    under a ceiling of 1.
# 2. A conformance log whose last line is not `<n> scenarios: <n> passed, 0 failed, 0 error,
#    0 unsupported`: `task conform-summary` must exit non-zero; the passing line must pass.
name=suite-refuses-a-refusal-count-above-the-ceiling
dir=.out/gate-tests/$name
rm -rf "$dir"
mkdir -p "$dir"

fail() {
  echo "$name: FAIL $*"
  exit 1
}

spec() {
  env OUT="$dir/exported-out" REFUSALS_MAX=999 task -d . -t taskfiles/spec.yml "$@"
}

spec suite SPEC_OUT="$dir/base" > "$dir/base.out" 2>&1 ||
  fail "task suite at the committed ceiling exited non-zero ($dir/base.out)"
[ -f "$dir/base/synthesize.log" ] || fail "task suite wrote no $dir/base/synthesize.log"
r=$(grep -c '^refused:' "$dir/base/synthesize.log")
[ "$r" -gt 0 ] || fail "the suite has no refusal, so no ceiling lies below its count"
max=$((r - 1))

if spec suite SPEC_OUT="$dir/planted" SPEC_REFUSALS_MAX="$max" > "$dir/planted.out" 2>&1; then
  fail "task suite with SPEC_REFUSALS_MAX=$max and $r refusal(s) exited 0"
fi
grep -q "$r refusal(s) above the ceiling SPEC_REFUSALS_MAX=$max" "$dir/planted.out" ||
  fail "task suite exited non-zero without naming $r refusal(s) and SPEC_REFUSALS_MAX=$max ($dir/planted.out)"
[ ! -e "$dir/exported-out" ] || fail "an exported OUT redirected the task into $dir/exported-out"

ceiling=$dir/ceiling
mkdir -p "$ceiling"
printf '64 scenario(s) (3 authored), 0 refusal(s), written to suite.json\n' > "$ceiling/none.log"
spec suite-ceiling SPEC_SYNTHESIZE_LOG="$ceiling/none.log" SPEC_REFUSALS_MAX=0 > "$ceiling/none.out" 2>&1 ||
  fail "task suite-ceiling refused a log with 0 refusal(s) under SPEC_REFUSALS_MAX=0 ($ceiling/none.out)"
printf 'refused: one\nrefused: two\n' > "$ceiling/two.log"
if spec suite-ceiling SPEC_SYNTHESIZE_LOG="$ceiling/two.log" SPEC_REFUSALS_MAX=1 > "$ceiling/two.out" 2>&1; then
  fail "task suite-ceiling accepted 2 refusal(s) under SPEC_REFUSALS_MAX=1"
fi
grep -q '2 refusal(s) above the ceiling SPEC_REFUSALS_MAX=1' "$ceiling/two.out" ||
  fail "task suite-ceiling did not name 2 refusal(s) and SPEC_REFUSALS_MAX=1 ($ceiling/two.out)"

logs=$dir/conform
mkdir -p "$logs"
plant() {
  printf '  passed swell.host.Agent/planted\n%s%s\n' "$3" "$2" > "$logs/$1.log"
}
plant passing '  64 scenarios: 64 passed, 0 failed, 0 error, 0 unsupported'
plant unsupported '  64 scenarios: 63 passed, 0 failed, 0 error, 1 unsupported'
plant failed '  64 scenarios: 63 passed, 1 failed, 0 error, 0 unsupported' '  failed swell.host.Agent/planted-failure
'
plant errored '  64 scenarios: 63 passed, 0 failed, 1 error, 0 unsupported'
plant fewer-passed '  64 scenarios: 63 passed, 0 failed, 0 error, 0 unsupported'
plant no-scenario '  0 scenarios: 0 passed, 0 failed, 0 error, 0 unsupported'
plant no-summary '  passed swell.host.Agent/planted'

spec conform-summary SPEC_CONFORM_LOG="$logs/passing.log" > "$logs/passing.out" 2>&1 ||
  fail "task conform-summary refused the passing line ($logs/passing.out)"
for c in unsupported failed errored fewer-passed no-scenario no-summary; do
  if spec conform-summary SPEC_CONFORM_LOG="$logs/$c.log" > "$logs/$c.out" 2>&1; then
    fail "task conform-summary accepted the planted last line of $logs/$c.log"
  fi
  grep -q "^conform: the last line of $logs/$c.log is not" "$logs/$c.out" ||
    fail "task conform-summary refused $logs/$c.log without saying why ($logs/$c.out)"
done
grep -q '^  failed swell.host.Agent/planted-failure$' "$logs/failed.out" ||
  fail "task conform-summary did not print the failed scenario line ($logs/failed.out)"

echo "$name: ok"

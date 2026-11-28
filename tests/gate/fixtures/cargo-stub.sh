#!/bin/sh
# A stub cargo for the planted scripts of tests/gate/ that run the lane slow's tasks on a tree of
# their own: a script copies it as <bin>/cargo, with tests/gate/fixtures/nextest-filter.awk
# beside it, and puts <bin> first on PATH. Not a script of gate-selftest: it lives outside
# tests/gate/*.sh. Nothing is built and no test runs.
#
# It reads the tests of the planted workspace from STUB_TESTS (lines "<binary-id> <test>
# [default-only]", see nextest-filter.awk) and answers, from the arguments it is given
# (--cargo-profile, -p, --workspace, --test, --lib, --features, --config-file with its
# default-filter line, -E, --ignore-default-filter, --message-format):
# - cargo nextest list: the tests that match, one "<binary-id> <test>" per line, or the JSON of
#   --message-format json-pretty;
# - cargo nextest run: a Starting and a Summary line over the tests that match, a PASS line per
#   test and a JUnit file target/nextest/default/junit.xml; with no test it says so and exits 4,
#   as cargo-nextest does;
# - cargo nextest run --no-run: a Finished line.
# Each call appends one line to STUB_LOG (default /dev/null): "<what> profile=<p>
# features=<f> full=[<SWELL_FULL_COUNTS>] guard=[<SWELL_NO_STACK_TESTS>]
# rustflags=[<RUSTFLAGS>] incremental=[<CARGO_INCREMENTAL>] config=<file name or none>
# argmax=<bytes of its longest argument> envbytes=<bytes of its environment but PWD, OLDPWD and
# STUB_*> n=<tests> args: <arguments>", <what> one of "build start", "build end", "list",
# "run" (a build has no n=); a run adds a line "ran <profile> <binary-id> <test>" for each test.
# Planted defects, each from a variable of the environment:
# - STUB_BUILD=fail|hang: a build (--no-run) in the profile collect whose arguments hold the
#   word STUB_BUILD_MATCH (any, when it is empty) prints "error: planted build failure" and
#   exits 3, or writes its process number to STUB_DIR/build.pid and sleeps 120 s;
#   STUB_BUILD_SECS: every build sleeps that long before its Finished line;
# - STUB_COMPILE=<profile>: a run in that profile whose arguments hold the word
#   STUB_COMPILE_MATCH (any, when it is empty) prints a cargo Compiling line first;
# - STUB_COMPILE_LIST=<profile>: a list in that profile prints a Compiling line on stderr;
# - STUB_DROP="<binary-id> <test>": a list in the profile collect leaves that test out;
# - STUB_DOUBLE="<binary-id> <test>": a list in the profile collect names that test too;
# - STUB_FAIL="<binary-id> <test>": a run that holds that test counts it failed and exits 100.
here=$(cd "$(dirname "$0")" && pwd)
log=${STUB_LOG:-/dev/null}
[ "$1" = nextest ] || { echo "stub cargo: only cargo nextest is planted, not cargo $*" >&2; exit 2; }
cmd=$2
shift 2
profile=test norun=0 cfg="" expr="" fmt=human pkgs="" ws=0 targets="" feat="" ignore=0 prev=""
for a in "$@"; do
  case "$prev" in
    --cargo-profile) profile=$a ;;
    --config-file) cfg=$a ;;
    -E) expr=$a ;;
    --message-format) fmt=$a ;;
    -p | --package) pkgs="$pkgs $a" ;;
    --features) feat=$a ;;
    --test) targets="$targets test:$a" ;;
  esac
  case "$a" in
    --no-run) norun=1 ;;
    --workspace) ws=1 ;;
    --lib) targets="$targets lib" ;;
    --ignore-default-filter) ignore=1 ;;
  esac
  prev=$a
done
argmax=0
for a in "$@"; do [ "${#a}" -gt "$argmax" ] && argmax=${#a}; done
envbytes=$(env | grep -v '^\(PWD\|OLDPWD\|STUB_[A-Z_]*\)=' | awk '{ n += length($0) + 1 } END { print n + 0 }')
config=none
[ -n "$cfg" ] && config=${cfg##*/}
env_="profile=$profile features=${feat:-none} full=[${SWELL_FULL_COUNTS:-unset}] guard=[${SWELL_NO_STACK_TESTS:-unset}] rustflags=[${RUSTFLAGS:-}] incremental=[${CARGO_INCREMENTAL:-}] config=$config argmax=$argmax envbytes=$envbytes"
if [ "$cmd" = run ] && [ "$norun" = 1 ]; then
  echo "build start $env_ args: $*" >> "$log"
  if [ "$profile" = collect ]; then
    case " $* " in *" ${STUB_BUILD_MATCH:-} "*) hit=1 ;; *) hit=0 ;; esac
    [ -z "${STUB_BUILD_MATCH:-}" ] && hit=1
    if [ "$hit" = 1 ]; then
      case "${STUB_BUILD:-ok}" in
        fail) echo "error: planted build failure"; exit 3 ;;
        hang) echo $$ > "$STUB_DIR/build.pid"; sleep 120; exit 0 ;;
      esac
    fi
  fi
  [ -n "${STUB_BUILD_SECS:-}" ] && sleep "$STUB_BUILD_SECS"
  echo "build end $env_ args: $*" >> "$log"
  echo "    Finished \`$profile\` profile [unoptimized] target(s) in 0.00s" >&2
  exit 0
fi
[ -n "${STUB_TESTS:-}" ] && [ -f "$STUB_TESTS" ] || { echo "stub cargo: no file STUB_TESTS of the planted tests" >&2; exit 2; }
mode=oneline
[ "$cmd" = list ] && [ "$fmt" = json-pretty ] && mode=json
[ "$cmd" = run ] && mode=run
mkdir -p .out
got=.out/stub-cargo-$$.txt
LC_ALL=C awk -v mode="$mode" -v expr="$expr" -v pkgs="$pkgs" -v ws="$ws" -v targets="$targets" -v feat="$feat" -v ignore="$ignore" \
  -f "$here/nextest-filter.awk" "${cfg:-/dev/null}" "$STUB_TESTS" > "$got" || { rm -f "$got"; exit 2; }
if [ "$cmd" = list ]; then
  if [ "$mode" = oneline ] && [ "$profile" = collect ]; then
    if [ -n "${STUB_DROP:-}" ]; then grep -vxF "$STUB_DROP" "$got" > "$got.d"; mv "$got.d" "$got"; fi
    if [ -n "${STUB_DOUBLE:-}" ] && ! grep -qxF "$STUB_DOUBLE" "$got"; then echo "$STUB_DOUBLE" >> "$got"; fi
  fi
  n=$(grep -c . "$got")
  echo "list $env_ n=$n args: $*" >> "$log"
  if [ "${STUB_COMPILE_LIST:-}" = "$profile" ]; then echo "   Compiling swell v0.1.0 (/stub/crates/swell)" >&2; fi
  cat "$got"
  rm -f "$got"
  exit 0
fi
n=$(grep -c . "$got")
echo "run $env_ n=$n args: $*" >> "$log"
awk -v p="$profile" '{ print "ran " p " " $0 }' "$got" >> "$log"
if [ "${STUB_COMPILE:-}" = "$profile" ]; then
  case " $* " in *" ${STUB_COMPILE_MATCH:-} "*) hit=1 ;; *) hit=0 ;; esac
  [ -z "${STUB_COMPILE_MATCH:-}" ] && hit=1
  if [ "$hit" = 1 ]; then echo "   Compiling swell v0.1.0 (/stub/crates/swell)"; fi
fi
if [ "$n" = 0 ]; then
  echo "    Starting 0 tests across 0 binaries"
  echo "error: no tests to run"
  rm -f "$got"
  exit 4
fi
bins=$(awk '{ print $1 }' "$got" | sort -u | grep -c .)
failed=0
if [ -n "${STUB_FAIL:-}" ] && grep -qxF "$STUB_FAIL" "$got"; then failed=1; fi
mkdir -p target/nextest/default
awk 'BEGIN { printf "<testsuites><testsuite>" } { printf "<testcase name=\"%s\" classname=\"%s\" time=\"0.010\"/>", $2, $1 } END { print "</testsuite></testsuites>" }' "$got" > target/nextest/default/junit.xml
echo "    Starting $n tests across $bins binaries"
awk -v f="${STUB_FAIL:-}" '$0 != f { printf "        PASS [   0.010s] %s %s\n", $1, $2 }' "$got"
rm -f "$got"
if [ "$failed" = 1 ]; then
  echo "        FAIL [   0.010s] $STUB_FAIL"
  echo "     Summary [   0.100s] $n tests run: $((n - 1)) passed, 1 failed, 0 skipped"
  exit 100
fi
echo "     Summary [   0.100s] $n tests run: $n passed, 0 skipped"
exit 0

#!/bin/sh
# Planted failure of three benchmarks in `sh bench/run.sh job`: the engine ends each with an
# error and a stack. A not-measured row's reason must carry the engine's message, the first
# line of stderr that is not a stack line (cut to 200 characters), not the last line of the
# stack ("at <eval> (...)"):
#   Mandelbrot, a QuickJS-style stack ("    at f (file:1:2)"): the ReferenceError line;
#   NBody, a JavaScriptCore-style stack ("f@file:1:2", "global code@file:1:2"): the
#     "Exception: Error: ..." line;
#   Bounce, an engine that writes its error to stdout after an iteration and nothing to
#     stderr: the first stdout line that is not one of the harness's own lines.
# The engine, the measuring tool (swell-bench-exec), cargo, unshare, taskset and prlimit are
# stubs under the script's directory and the image is planted under HOME, so nothing is
# fetched, compiled or measured, and no namespace is made.
# Run from the repository root: sh tests/gate/bench-job-names-the-engine-error-of-a-failed-benchmark.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=bench-job-names-the-engine-error-of-a-failed-benchmark
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/tree/bench/tool" "$dir/home" "$dir/bin" || fail "cannot make $dir"
abs=$(cd "$dir" && pwd) || fail "cannot resolve $dir"
cp -R bench/image bench/suites bench/run.sh "$dir/tree/bench/" || fail "cannot copy bench/"
: > "$dir/tree/bench/tool/Cargo.lock" || fail "cannot plant Cargo.lock"

printf '%s\n' 'engine version archive sha256 binary loads url' \
  'planted 1 bin 0000000000000000000000000000000000000000000000000000000000000000 planted planted file:///nonexistent' \
  > "$dir/tree/bench/image/engines.tsv" || fail "cannot plant engines.tsv"
printf '%s\n' 'config engine mode flags env verified' 'planted planted interpreter - - yes' \
  > "$dir/tree/bench/image/configs.tsv" || fail "cannot plant configs.tsv"
printf '%s\n' 'suite benchmark file iterations inner' \
  'are-we-fast-yet Bounce bounce 2 7' \
  'are-we-fast-yet Mandelbrot mandelbrot 1 500' \
  'are-we-fast-yet NBody nbody 1 250000' > "$dir/tree/bench/suites/iterations.tsv" ||
  fail "cannot plant iterations.tsv"

# Stubs: the namespace and CPU wrappers pass the command through; cargo names the tool stub;
# the tool stub runs the command with its output in the named files and prints the tool's line.
printf '#!/bin/sh\nshift\nexec "$@"\n' > "$dir/bin/unshare"
printf '#!/bin/sh\nshift 2\nexec "$@"\n' > "$dir/bin/taskset"
printf '#!/bin/sh\nshift\nexec "$@"\n' > "$dir/bin/prlimit"
printf '#!/bin/sh\necho %s\n' "'{\"reason\":\"compiler-artifact\",\"executable\":\"$abs/bin/swell-bench-exec\"}'" > "$dir/bin/cargo"
cat > "$dir/bin/swell-bench-exec" <<'EOF' || fail "cannot plant the tool stub"
#!/bin/sh
while [ "$1" != -- ]; do
  case $1 in
    --stdout) out=$2 ;;
    --stderr) err=$2 ;;
  esac
  shift 2
done
shift
"$@" > "$out" 2> "$err"
echo "wall_us=1000 maxrss_kib=1024 exit=$? signal=- timed_out=0"
EOF
chmod +x "$dir/bin/unshare" "$dir/bin/taskset" "$dir/bin/prlimit" "$dir/bin/cargo" "$dir/bin/swell-bench-exec" ||
  fail "cannot plant the stubs"

digest=$(cd "$dir/tree" && cat bench/image/engines.tsv bench/suites/suites.tsv bench/suites/files.tsv | sha256sum | cut -c1-16)
image=$abs/home/.cache/swell-bench/image-$digest
suite=$image/suites/are-we-fast-yet
mkdir -p "$image/engines/planted" "$suite" || fail "cannot make the planted image"
echo planted > "$image/manifest.tsv" || fail "cannot plant the manifest"
for module in benchmark som bounce mandelbrot nbody; do
  echo "// planted $module" > "$suite/$module.js" || fail "cannot plant $module.js"
done
cat > "$image/engines/planted/planted" <<'EOF' || fail "cannot plant the engine"
#!/bin/sh
case $1 in
  *Mandelbrot.js)
    echo "ReferenceError: 'process' is not defined" >&2
    echo "    at verifyResult ($1:651:5)" >&2
    echo "    at innerBenchmarkLoop ($1:714:5)" >&2
    echo "    at <anonymous> ($1:728:42)" >&2
    echo "    at <eval> ($1:737:3)" >&2
    exit 1 ;;
  *NBody.js)
    echo "Exception: Error: Benchmark failed with incorrect result" >&2
    echo "@$1:1465:22" >&2
    echo "global code@$1:1471:3" >&2
    exit 3 ;;
  *Bounce.js)
    echo "iteration 0 0 12.5"
    echo "Error: planted on stdout"
    echo "    at <eval> ($1:9:9)"
    exit 1 ;;
esac
exit 0
EOF
chmod +x "$image/engines/planted/planted" || fail "cannot plant the engine"

(cd "$dir/tree" && HOME="$abs/home" PATH="$abs/bin:$PATH" sh bench/run.sh job planted are-we-fast-yet 0 not-this-machine 1) \
  > "$dir/job.out" 2>&1 || fail "the job did not end with rows ($dir/job.out)"
sed -n '/^== rows begin$/,/^== rows end$/p' "$dir/job.out" | sed '1d;$d' > "$dir/rows.jsonl"
[ -s "$dir/rows.jsonl" ] || fail "the job printed no rows ($dir/job.out)"

expect() { # BENCHMARK REASON
  got=$(jq -r --arg m "are-we-fast-yet/$1" 'select(.measurement == $m) | .reason' "$dir/rows.jsonl")
  [ "$got" = "$2" ] || fail "$1: reason \"$got\", expected \"$2\" ($dir/rows.jsonl)"
}
expect Mandelbrot "failed: exit 1, signal -, timed out 0: ReferenceError: 'process' is not defined"
expect NBody "failed: exit 3, signal -, timed out 0: Exception: Error: Benchmark failed with incorrect result"
expect Bounce "failed: exit 1, signal -, timed out 0: Error: planted on stdout"
jq -e -s '[.[] | select(.measurement == "are-we-fast-yet/geomean")] | length == 1 and .[0].status == "not-measured"' \
  "$dir/rows.jsonl" > /dev/null || fail "the geomean row is not one not-measured row ($dir/rows.jsonl)"

echo "$name: ok"

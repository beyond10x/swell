#!/bin/sh
# Planted defect of bench/suites/iterations.tsv: an inner count of are-we-fast-yet that the
# benchmark's own verifyResult does not know (Mandelbrot 5; the suite knows 500, 750 and 1).
# The suite's harness then fails the benchmark ("No verification result for 5 found") after
# the job has started to measure. `sh bench/run.sh job` and `sh bench/run.sh prepare` must exit
# non-zero before anything is measured or built, naming the benchmark, the count and the counts
# the suite verifies; a count the suite verifies (Mandelbrot 500), and any count of a benchmark
# whose verifyResult checks each call (Bounce), must pass the check. A module that says "No
# verification result" in a form whose counts cannot be read is refused too.
# The image is planted under HOME (the suite files in the form of the pinned commit: a
# verifyResult(result, innerIterations) with one "if (innerIterations === N)" line per count),
# and cargo is a stub that fails and leaves a mark, so nothing is fetched or compiled.
# Run from the repository root: sh tests/gate/bench-job-refuses-an-inner-count-the-suite-does-not-verify.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=bench-job-refuses-an-inner-count-the-suite-does-not-verify
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
printf '#!/bin/sh\n: > "%s/cargo-ran"\necho "cargo stub: not run in this test" >&2\nexit 1\n' "$abs" > "$dir/bin/cargo" &&
  chmod +x "$dir/bin/cargo" || fail "cannot plant the cargo stub"

# The image bench/image/build.sh would make for the planted pins; it keeps an image that exists.
digest=$(cd "$dir/tree" && cat bench/image/engines.tsv bench/suites/suites.tsv bench/suites/files.tsv | sha256sum | cut -c1-16)
image=$abs/home/.cache/swell-bench/image-$digest
suite=$image/suites/are-we-fast-yet
mkdir -p "$image/engines/planted" "$suite" || fail "cannot make the planted image"
printf '#!/bin/sh\nexit 0\n' > "$image/engines/planted/planted" && chmod +x "$image/engines/planted/planted" ||
  fail "cannot plant the engine"
echo planted > "$image/manifest.tsv" || fail "cannot plant the manifest"
for module in benchmark som; do
  echo "// planted $module" > "$suite/$module.js" || fail "cannot plant $module.js"
done
printf '%s\n' \
  "const { Benchmark } = require('./benchmark');" \
  '' \
  'class Bounce extends Benchmark {' \
  '  benchmark() {' \
  '    return 1331;' \
  '  }' \
  '' \
  '  verifyResult(result) {' \
  '    return result === 1331;' \
  '  }' \
  '}' > "$suite/bounce.js" || fail "cannot plant bounce.js"
printf '%s\n' \
  "const { Benchmark } = require('./benchmark');" \
  '' \
  'class Mandelbrot extends Benchmark {' \
  '  verifyResult(result, innerIterations) {' \
  '    if (innerIterations === 500) { return result === 191; }' \
  '    if (innerIterations === 750) { return result === 50; }' \
  '    if (innerIterations === 1) { return result === 128; }' \
  '' \
  '    process.stdout.write(`No verification result for ${innerIterations} found\n`);' \
  '    return false;' \
  '  }' \
  '' \
  '  innerBenchmarkLoop(innerIterations) {' \
  '    return this.verifyResult(this.mandelbrot(innerIterations), innerIterations);' \
  '  }' \
  '}' > "$suite/mandelbrot.js" || fail "cannot plant mandelbrot.js"
printf '%s\n' \
  'class Unread extends Benchmark {' \
  '  verifyResult(result, n) {' \
  '    switch (n) { case 4: return result === 1; }' \
  '    process.stdout.write(`No verification result for ${n} found\n`);' \
  '    return false;' \
  '  }' \
  '}' > "$suite/unread.js" || fail "cannot plant unread.js"

iterations() { # MANDELBROT_INNER [EXTRA_ROW]
  {
    echo 'suite benchmark file iterations inner'
    echo 'bench-v8 Splay splay.js 1 -'
    echo 'are-we-fast-yet Bounce bounce 2 7'
    echo "are-we-fast-yet Mandelbrot mandelbrot 1 $1"
    [ -z "${2:-}" ] || echo "$2"
  } > "$dir/tree/bench/suites/iterations.tsv"
}

run() {
  (cd "$dir/tree" && HOME="$abs/home" PATH="$abs/bin:$PATH" sh bench/run.sh "$@")
}

iterations 5
rm -f "$abs/cargo-ran"
if run job planted are-we-fast-yet 0 not-this-machine 1 > "$dir/job-wrong.out" 2>&1; then
  fail "job exited 0 with Mandelbrot at an inner count the suite does not verify ($dir/job-wrong.out)"
fi
grep -q 'Mandelbrot.*inner count 5.*500 750 1' "$dir/job-wrong.out" ||
  fail "job did not name Mandelbrot, the count 5 and the counts 500 750 1 ($dir/job-wrong.out)"
grep -q 'Bounce' "$dir/job-wrong.out" &&
  fail "job named Bounce, whose verifyResult checks each call at any count ($dir/job-wrong.out)"
[ ! -f "$abs/cargo-ran" ] || fail "job built the tool before it refused the count ($dir/job-wrong.out)"
grep -q '^== rows begin' "$dir/job-wrong.out" && fail "job printed rows ($dir/job-wrong.out)"
[ ! -e "$dir/tree/.out/bench/job/planted.are-we-fast-yet/Mandelbrot.1.out" ] ||
  fail "job measured Mandelbrot before it refused the count"

if run job planted bench-v8 0 not-this-machine 1 > "$dir/job-v8-wrong.out" 2>&1; then
  fail "a bench-v8 job exited 0 while the file holds a count the suite does not verify ($dir/job-v8-wrong.out)"
fi
grep -q 'Mandelbrot.*inner count 5' "$dir/job-v8-wrong.out" ||
  fail "a bench-v8 job did not name the count ($dir/job-v8-wrong.out)"

rm -f "$abs/cargo-ran"
if run prepare > "$dir/prepare-wrong.out" 2>&1; then
  fail "prepare exited 0 with Mandelbrot at an inner count the suite does not verify ($dir/prepare-wrong.out)"
fi
grep -q 'Mandelbrot.*inner count 5.*500 750 1' "$dir/prepare-wrong.out" ||
  fail "prepare did not name the count ($dir/prepare-wrong.out)"
[ ! -f "$abs/cargo-ran" ] || fail "prepare built the tool before it refused the count ($dir/prepare-wrong.out)"

iterations 500 'are-we-fast-yet Unread unread 1 4'
if run job planted are-we-fast-yet 0 not-this-machine 1 > "$dir/job-unread.out" 2>&1; then
  fail "job exited 0 with a module whose verified counts cannot be read ($dir/job-unread.out)"
fi
grep -q 'Unread.*cannot read' "$dir/job-unread.out" ||
  fail "job did not say that it cannot read the counts of Unread ($dir/job-unread.out)"
grep -q 'Mandelbrot' "$dir/job-unread.out" &&
  fail "job named Mandelbrot at 500, a count the suite verifies ($dir/job-unread.out)"

iterations 500
rm -f "$abs/cargo-ran"
run job planted are-we-fast-yet 0 not-this-machine 1 > "$dir/job-right.out" 2>&1
grep -q 'inner count' "$dir/job-right.out" &&
  fail "job refused counts the suite verifies ($dir/job-right.out)"
[ -f "$abs/cargo-ran" ] ||
  fail "job with counts the suite verifies stopped before the tool build ($dir/job-right.out)"
grep -q 'no swell-bench-exec' "$dir/job-right.out" ||
  fail "job with counts the suite verifies did not reach the tool build ($dir/job-right.out)"

echo "$name: ok"

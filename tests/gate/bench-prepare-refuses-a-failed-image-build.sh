#!/bin/sh
# Planted defect of bench/run.sh: an engine pin whose download does not match its sha256, so
# that bench/image/build.sh fails. `sh bench/run.sh prepare` and `sh bench/run.sh job` must
# exit non-zero and say that the image build failed, and prepare must print no "image" line,
# rather than go on with the last line the failed build printed as the image's directory.
# The engine is a local file (file:// URL), the image root and HOME lie under the script's
# directory, and cargo is a stub that fails, so nothing is fetched and nothing compiles.
# Run from the repository root: sh tests/gate/bench-prepare-refuses-a-failed-image-build.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=bench-prepare-refuses-a-failed-image-build
dir=.out/gate-tests/$name
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir/tree/bench" "$dir/home" "$dir/bin" || fail "cannot make $dir"
abs=$(cd "$dir" && pwd) || fail "cannot resolve $dir"
cp -R bench/image bench/suites bench/run.sh "$dir/tree/bench/" || fail "cannot copy bench/"

printf 'not an engine\n' > "$abs/engine.bin" || fail "cannot plant the engine file"
printf '%s\n' 'engine version archive sha256 binary loads url' \
  "planted 1 bin 0000000000000000000000000000000000000000000000000000000000000000 planted planted file://$abs/engine.bin" \
  > "$dir/tree/bench/image/engines.tsv" || fail "cannot plant engines.tsv"
printf '%s\n' 'config engine mode flags env verified' 'planted planted interpreter - - yes' \
  > "$dir/tree/bench/image/configs.tsv" || fail "cannot plant configs.tsv"
printf '#!/bin/sh\necho "cargo stub: not run in this test" >&2\nexit 1\n' > "$dir/bin/cargo" &&
  chmod +x "$dir/bin/cargo" || fail "cannot plant the cargo stub"

run() {
  (cd "$dir/tree" && HOME="$abs/home" PATH="$abs/bin:$PATH" sh bench/run.sh "$@")
}

if run prepare > "$dir/prepare.out" 2>&1; then
  fail "prepare exited 0 after a failed image build ($dir/prepare.out)"
fi
if grep -q '^image ' "$dir/prepare.out"; then
  fail "prepare printed an image line after a failed image build ($dir/prepare.out)"
fi
grep -q 'image build failed' "$dir/prepare.out" ||
  fail "prepare did not say that the image build failed ($dir/prepare.out)"
grep -q 'planted 1.*sha256 .*pinned 0000' "$dir/prepare.out" ||
  fail "prepare did not show the build's own reason ($dir/prepare.out)"

if run job planted start 0 not-this-machine 1 > "$dir/job.out" 2>&1; then
  fail "job exited 0 after a failed image build ($dir/job.out)"
fi
grep -q 'image build failed' "$dir/job.out" ||
  fail "job did not say that the image build failed ($dir/job.out)"

echo "$name: ok"

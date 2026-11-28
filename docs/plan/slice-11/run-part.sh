#!/bin/sh
# One run of the runner over a part of the pinned test262 checkout, for the acceptance of a story
# of slice 11. It builds and runs and does nothing else, so it is started as one job of the build
# pool, with its time named and its result fetched back (files that .gitignore names are not
# copied to the pool's node, so the job makes them itself):
#   slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh <list> [<list> ...]
# Each <list> holds one path per line, starting with test/ (a story list, a guard list of
# docs/plan/slice-11/). Steps: `task test262-fetch` (the pinned checkout into
# .cache/test262/<commit of conformance/test262/PIN>, fetched and checked when it is missing);
# the lists, sorted and joined, into .out/part.cases.txt; `sh docs/plan/slice-11/part.sh` into a
# new .out/test262-part (an old one is removed first); the runner, built in the release profile,
# over that part, its run lines into .out/test262-result/runs.tsv and its count lines into
# .out/test262-result/lines.txt. Prints the part's two lines and the runner's line of slice 11.
# Then, on the workstation and without the pool, a unit reads the result with
#   sh docs/plan/slice-11/check.sh <story list> .out/test262-base/runs.tsv .out/test262-result/runs.tsv
# POSIX sh. Owner: the decomposition of slice 11; a unit reads it and does not change it.
set -eu
if [ $# -lt 1 ]; then
  echo "usage: run-part.sh <list> [<list> ...]" >&2
  exit 2
fi
for f in "$@"; do
  if [ ! -s "$f" ]; then
    echo "run-part.sh: $f is empty or missing" >&2
    exit 2
  fi
done
commit=$(awk '$1 == "commit" { print $2 }' conformance/test262/PIN)
task test262-fetch
mkdir -p .out
cat "$@" | LC_ALL=C sort -u > .out/part.cases.txt
rm -rf .out/test262-part .out/test262-result
mkdir -p .out/test262-result
sh docs/plan/slice-11/part.sh ".cache/test262/$commit" .out/test262-part .out/part.cases.txt
cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part \
  --design docs/design/engine.md --features conformance/test262/features.txt \
  --runs .out/test262-result/runs.tsv > .out/test262-result/lines.txt
grep '^test262-slice 11 ' .out/test262-result/lines.txt || echo "run-part.sh: no case of slice 11 in the part"

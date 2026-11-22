#!/bin/sh
# usage, from the repository root: run.sh <probe> <n>
set -eu
probe=$1
n=$2
here=$(dirname "$0")
pin=$(awk '$1 == "commit" { print $2 }' conformance/test262/PIN)
out=.out/probes/strings/$probe-$n
rm -rf "$out"
mkdir -p "$out/test/built-ins/probe"
ln -s "$PWD/.cache/test262/$pin/harness" "$out/harness"
ln -s "$PWD/.cache/test262/$pin/features.txt" "$out/features.txt"
{ printf '/*---\ndescription: probe\nflags: [onlyStrict]\n---*/\n'; sed "s/@N@/$n/g" "$here/$probe.js"; } > "$out/test/built-ins/probe/p.js"
target/release/swell-test262 count --test262 "$out" --design docs/design/engine.md --features conformance/test262/features.txt --workers 1 --runs "$out/runs.tsv" > "$out/lines.txt" 2> "$out/log.txt" || true
awk -v p="$probe" -v n="$n" '$1 == "test262-time" { ms = $4 } $1 == "test262-steps" { steps = $2 } END { printf "%s\t%s\t%s\t%s\t", p, n, ms, steps }' "$out/log.txt"
awk -F'\t' '{ print $5, $6 }' "$out/runs.tsv"

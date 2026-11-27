#!/bin/sh
# perf-loop.sh: the CPU seconds of the loop of buildString (tests/perf/build-string-loop.js) on
# the release swell, alternating with a base binary when one is named
# (story:property-escape-cases-spend-less-in-the-interpreter-loop).
#
# Usage: [PERF_BIN=<swell>] [PERF_BASE_BIN=<swell>] [PERF_RUNS=<n>] sh taskfiles/perf-loop.sh
#
# A measurement, in no gate and no lane: it builds nothing and is started by its path, without the
# build pool's prefix, so both binaries run on one machine, one run at a time. It runs PERF_BIN
# (default target/release/swell) with no limit flag on PERF_SOURCE (default
# tests/perf/build-string-loop.js) PERF_RUNS times (default 3), alternating with PERF_BASE_BIN
# when it is set (the release swell built at the commit that added this script, kept under
# .out/perf/): head first in odd rounds, base first in even ones. The CPU seconds of a run are the
# user plus system time of the child, fields cutime and cstime of /proc/<pid>/stat of the shell
# that waited for it, in clock ticks, as taskfiles/gate-time.yml reads them. Per run it prints
# "perf-loop: run <n> <head|base> <seconds> cpu s, pressure <avg10 of /proc/pressure/cpu>", then
# "perf-loop: head <median> base <median> ratio <head/base>" ("base none ratio none" with no
# base). It exits non-zero when a run's exit status is not 0 or its standard output is not the
# line of PERF_EXPECTED (default tests/perf/build-string-loop.out). It asserts nothing about time.
# It reads and writes nothing under bench/; its output files go to PERF_OUT (default .out/perf).

bin=${PERF_BIN:-target/release/swell}
base=${PERF_BASE_BIN:-}
runs=${PERF_RUNS:-3}
source=${PERF_SOURCE:-tests/perf/build-string-loop.js}
expected=${PERF_EXPECTED:-tests/perf/build-string-loop.out}
out=${PERF_OUT:-.out/perf}

fail() { echo "perf-loop: $*"; exit 1; }

[ -x "$bin" ] || fail "no binary at $bin (PERF_BIN); build it first: cargo build --locked --release -p swell-cli"
[ -z "$base" ] || [ -x "$base" ] || fail "no binary at $base (PERF_BASE_BIN)"
[ -r "$source" ] || fail "no source at $source (PERF_SOURCE)"
[ -r "$expected" ] || fail "no expected output at $expected (PERF_EXPECTED)"
case $runs in '' | *[!0-9]* | 0) fail "PERF_RUNS is not a count of at least 1: $runs" ;; esac
hz=$(getconf CLK_TCK 2>/dev/null) || fail "getconf CLK_TCK failed"
[ -r /proc/self/stat ] || fail "no /proc/self/stat: the CPU seconds of a child cannot be read"
mkdir -p "$out" || fail "cannot make $out"

# run <label> <binary> <n>: one run; prints its line and appends its seconds to $out/<label>.cpu.
run() {
  label=$1 binary=$2 n=$3
  pressure=$(awk '/^some/ { for (i = 2; i <= NF; i++) if ($i ~ /^avg10=/) { sub(/^avg10=/, "", $i); print $i } }' /proc/pressure/cpu 2>/dev/null)
  ticks=$(sh -c '
    "$1" "$2" > "$3.stdout" 2> "$3.stderr"
    echo $? > "$3.status"
    awk "{ sub(/^.*\\) /, \"\"); print \$14 + \$15 }" /proc/$$/stat
  ' perf-loop "$binary" "$source" "$out/$label-$n")
  status=$(cat "$out/$label-$n.status")
  [ "$status" = 0 ] || fail "run $n $label: $binary exited $status: $(head -c 300 "$out/$label-$n.stderr")"
  cmp -s "$out/$label-$n.stdout" "$expected" ||
    fail "run $n $label: the output of $binary is not the line of $expected: $(head -c 300 "$out/$label-$n.stdout")"
  seconds=$(awk -v t="$ticks" -v hz="$hz" 'BEGIN { printf "%.2f", t / hz }')
  echo "$seconds" >> "$out/$label.cpu"
  echo "perf-loop: run $n $label $seconds cpu s, pressure ${pressure:-unknown}"
}

median() {
  sort -n "$1" | awk '{ v[NR] = $1 } END { if (NR % 2) printf "%.2f", v[(NR + 1) / 2]; else printf "%.2f", (v[NR / 2] + v[NR / 2 + 1]) / 2 }'
}

rm -f "$out/head.cpu" "$out/base.cpu"
n=1
while [ "$n" -le "$runs" ]; do
  if [ -z "$base" ]; then
    run head "$bin" "$n"
  elif [ $((n % 2)) = 1 ]; then
    run head "$bin" "$n"
    run base "$base" "$n"
  else
    run base "$base" "$n"
    run head "$bin" "$n"
  fi
  n=$((n + 1))
done

head_median=$(median "$out/head.cpu")
if [ -z "$base" ]; then
  echo "perf-loop: head $head_median base none ratio none"
else
  base_median=$(median "$out/base.cpu")
  ratio=$(awk -v h="$head_median" -v b="$base_median" 'BEGIN { if (b > 0) printf "%.3f", h / b; else print "none" }')
  echo "perf-loop: head $head_median base $base_median ratio $ratio"
fi

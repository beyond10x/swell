#!/bin/sh
# The measurement harness of bench/: K2 to K5 of the engines of bench/image/configs.tsv, in the
# schema swell-bench/1 (bench/schema/swell-bench-1.json). POSIX sh, jq and awk; the one compiled
# helper is bench/tool (swell-bench-exec: wall time and peak RSS of one process).
#
# Run from the repository root:
#   sh bench/run.sh all [WORKERS]           on the workstation: one pool job per (config, suite),
#                                           WORKERS (default 1) at a time, then assemble; stops
#                                           at the first job the pool refuses. Needs
#                                           BENCH_NODE_STATE, a shell command that prints the
#                                           JSON of `slot state` of the build node; refuses to
#                                           start without it unless BENCH_ALLOW_UNWATCHED=1
#   sh bench/run.sh prepare                 in a pool job: the image and the tool
#   sh bench/run.sh job CONFIG SUITE WORKER ORIGIN [RUNS] [ITERATIONS_FILE]
#                                           in a pool job: SUITE is bench-v8, are-we-fast-yet or
#                                           start (K3, K4, K5); WORKER picks two CPUs of the node
#                                           (CPU 0 and 1 are left to the system); ORIGIN is the
#                                           digest of the starting machine's machine-id: a job
#                                           that finds itself there refuses to measure
#   sh bench/run.sh assemble DIR RESULT     on the workstation: the job logs of DIR into RESULT
#
# Every measurement is one process, started by swell-bench-exec inside
# `unshare -rn` (a user and network namespace: no network), `taskset -c CPUS` (the pinned
# cpuset) and `prlimit --data` (the memory limit). The node's 1-minute load average is read at
# the start and the end of each job. A job prints its rows between "== rows begin" and
# "== rows end", so that the rows reach the workstation in the job's output.
#
# Before each job, `all` waits until the node runs no pool job (it reads the node's job list
# with BENCH_NODE_STATE every IDLE_POLL_S seconds), at most IDLE_WAIT_S seconds, and then starts
# the job either way. After each job it reads the list again and writes every other pool job
# whose time overlaps the job's own record to logs/<config>.<suite>.neighbours.json. `assemble`
# marks every row of a job loaded: true, with loaded_reason, when the job had a neighbour, a
# load per CPU over LOADED_PER_CPU at its start or end, or no record of the node (unwatched).
set -eu

RUNS_K2=5
RUNS_START=10
PROCESS_TIMEOUT_S=600
MEMORY_LIMIT_BYTES=4294967296
LOADED_PER_CPU=1.0
IDLE_WAIT_S=${BENCH_IDLE_WAIT_S:-600}
IDLE_POLL_S=${BENCH_IDLE_POLL_S:-10}

die() {
  echo "bench: $*" >&2
  exit 1
}

[ -f bench/run.sh ] && [ -f bench/image/configs.tsv ] || die "run from the repository root"

field() {
  awk -v key="$2" -v col="$3" 'NR > 1 && $1 == key { print $col; exit }' "$1"
}

build_tool() {
  if [ ! -f bench/tool/Cargo.lock ]; then
    cargo generate-lockfile --manifest-path bench/tool/Cargo.toml >&2
    echo "== Cargo.lock begin" >&2
    cat bench/tool/Cargo.lock >&2
    echo "== Cargo.lock end" >&2
  fi
  mkdir -p .out/bench
  cargo build --release --locked --manifest-path bench/tool/Cargo.toml --message-format=json \
    > .out/bench/tool-build.json || die "tool build failed (cargo build of bench/tool)"
  built=$(jq -r 'select(.reason == "compiler-artifact" and .executable != null) | .executable' \
    .out/bench/tool-build.json | tail -n 1)
  [ -x "$built" ] || die "tool build failed: cargo named no executable (.out/bench/tool-build.json)"
  echo "$built"
}

# image_build LOG: bench/image/build.sh with its output in LOG; prints the image's directory.
# A failed build stops the caller: its last line is the image only when the build succeeded.
image_build() {
  mkdir -p "$(dirname "$1")"
  sh bench/image/build.sh > "$1" 2>&1 || die "image build failed; see $1: $(tail -n 1 "$1")"
  built=$(tail -n 1 "$1")
  [ -f "$built/manifest.tsv" ] || die "image build failed: no manifest under '$built'; see $1"
  echo "$built"
}

median() {
  tr ' ' '\n' | awk 'NF' | LC_ALL=C sort -g | awk '
    { v[NR] = $1 }
    END {
      if (NR == 0) exit 1
      if (NR % 2) printf "%.6f\n", v[(NR + 1) / 2]
      else printf "%.6f\n", (v[NR / 2] + v[NR / 2 + 1]) / 2
    }'
}

json_numbers() {
  tr ' ' '\n' | awk 'NF' | jq -s -c '.'
}

# measure SCRIPT PREFIX: one process of the job's engine on SCRIPT; prints the tool's line.
measure() {
  # shellcheck disable=SC2086
  env $run_env unshare -rn taskset -c "$cpus" prlimit --data="$MEMORY_LIMIT_BYTES" \
    "$tool" --stdout "$2.out" --stderr "$2.err" --timeout "$PROCESS_TIMEOUT_S" -- \
    "$bin" $run_flags "$1"
}

tool_field() {
  echo "$1" | tr ' ' '\n' | sed -n "s/^$2=//p"
}

# message FILE [SKIP]: the first line of FILE that is not empty, not a stack line and, with
# SKIP, not one of the harness's own lines; leading blanks dropped, cut to 200 characters. A
# stack line is "at f (file:1:2)" (V8, QuickJS, Hermes) or "f@file:1:2" (SpiderMonkey,
# JavaScriptCore).
message() {
  awk -v skip="${2:-}" '
    { line = $0; sub(/^[[:space:]]+/, "", line) }
    line == "" { next }
    line ~ /^at / { next }
    line ~ /^[^:@]*@.*:[0-9]+(:[0-9]+)?$/ { next }
    skip != "" && line ~ /^(iteration|clock|done) / { next }
    { print substr(line, 1, 200); exit }' "$1"
}

# failure_reason PREFIX EXIT SIGNAL TIMED_OUT: the engine's own message, from PREFIX.err, else
# from PREFIX.out (an engine that reports its error on stdout).
failure_reason() {
  msg=$(message "$1.err")
  [ -n "$msg" ] || msg=$(message "$1.out" harness)
  echo "failed: exit $2, signal $3, timed out $4: ${msg:-no output}"
}

# verified_counts MODULE_FILE: the inner counts the are-we-fast-yet benchmark's own
# verifyResult knows, one per line, or "any" when its verifyResult checks each call whatever
# the count (verifyResult(result)) or the benchmark checks itself (DeltaBlue). A module whose
# verifyResult takes the count (verifyResult(result, n)) lists them as "if (n === N)"; one
# that says "No verification result" and lists no count in that form prints nothing.
verified_counts() {
  awk '
    /^[[:space:]]*verifyResult\([^)]*\)[[:space:]]*\{/ {
      params = $0
      sub(/^[[:space:]]*verifyResult\(/, "", params)
      sub(/\).*/, "", params)
      n = split(params, p, /[[:space:]]*,[[:space:]]*/)
      if (n >= 2) { inside = 1; counted = 1; arg = p[2] }
      next
    }
    inside && /^  }/ { inside = 0 }
    inside {
      if (match($0, "if \\(" arg " === [0-9]+\\)")) {
        count = substr($0, RSTART, RLENGTH)
        sub(/.*=== /, "", count)
        sub(/\)$/, "", count)
        print count
      }
    }
    /No verification result/ { unverified = 1 }
    END { if (!counted && !unverified) print "any" }' "$1"
}

# check_counts IMAGE ITERATIONS_FILE: fail, naming each, when a benchmark of are-we-fast-yet in
# ITERATIONS_FILE has an inner count its own verifyResult does not know (the suite then fails
# it with "No verification result for N found"), or an iteration count that is not a positive
# whole number. Runs before anything is built or measured.
check_counts() {
  bad=$(awk 'NR > 1 && $1 == "are-we-fast-yet" { print $2, $3, $4, $5 }' "$2" | while read -r name module iterations inner; do
    case $iterations in
      ''|0*|*[!0-9]*) echo "  are-we-fast-yet $name: iteration count '$iterations' is not a positive whole number" ;;
    esac
    case $inner in
      ''|0*|*[!0-9]*)
        echo "  are-we-fast-yet $name: inner count '$inner' is not a positive whole number"
        continue ;;
    esac
    file=$1/suites/are-we-fast-yet/$module.js
    if [ ! -f "$file" ]; then
      echo "  are-we-fast-yet $name: no suite file $module.js in the image"
      continue
    fi
    counts=$(verified_counts "$file" | tr '\n' ' ' | sed 's/ $//')
    case " $counts " in
      " any "|*" $inner "*) ;;
      "  ") echo "  are-we-fast-yet $name: inner count $inner; cannot read the counts $module.js verifies" ;;
      *) echo "  are-we-fast-yet $name: inner count $inner is not one the suite verifies ($module.js verifies $counts)" ;;
    esac
  done)
  [ -z "$bad" ] || die "$2 holds counts the suite does not verify; nothing measured:
$bad"
}

# row KPI MEASUREMENT UNIT STATUS REASON VALUES [EXTRA_JSON]
row() {
  values_json=$(echo "$6" | json_numbers)
  extra_json=${7:-}
  [ -n "$extra_json" ] || extra_json="{}"
  if [ "$4" = measured ]; then
    med=$(echo "$6" | median)
  else
    med=null
  fi
  jq -n -c \
    --arg engine "$engine" --arg config "$config" --arg version "$version" --arg mode "$mode" \
    --arg flags "$cfg_flags" --arg env "$cfg_env" --arg verified "$verified" \
    --arg kpi "$1" --arg measurement "$2" --arg unit "$3" --arg status "$4" --arg reason "$5" \
    --argjson values "$values_json" --argjson median "$med" \
    --arg sha256 "$sha" --arg binary_sha256 "$binary_sha" --arg cpus "$cpus" \
    --arg image_digest "$image_digest" --arg measured_at "$started_at" \
    --argjson extra "$extra_json" '
    def list: if . == "-" then [] else split(",") end;
    {engine: $engine, config: $config, version: $version, mode: $mode, flags: ($flags | list),
     env: ($env | list), flags_verified: ($verified == "yes"),
     kpi: $kpi, measurement: $measurement, unit: $unit,
     runs: ($values | length), values: $values, median: $median,
     min: (if ($values | length) > 0 then ($values | min) else null end),
     max: (if ($values | length) > 0 then ($values | max) else null end),
     status: $status, reason: $reason,
     sha256: $sha256, binary_sha256: $binary_sha256, cpuset: $cpus,
     image_digest: $image_digest, measured_at: $measured_at} + $extra' >> "$rows"
}

compose_bench_v8() { # FILE ITERATIONS OUT
  {
    echo "var BENCH_ITERATIONS = $2;"
    cat bench/suites/clock.js "$image/suites/bench-v8/base.js" "$image/suites/bench-v8/$1" \
      bench/suites/bench-v8-driver.js
  } > "$3"
}

compose_awfy() { # MODULE ITERATIONS INNER OUT
  {
    echo "var BENCH_NAME = '$1'; var BENCH_ITERATIONS = $2; var BENCH_INNER = $3;"
    cat bench/suites/clock.js bench/suites/require.js
    for module in benchmark som "$1"; do
      echo "benchModules['$module'] = function (exports, require, module) {"
      cat "$image/suites/are-we-fast-yet/$module.js"
      echo "};"
    done
    cat bench/suites/are-we-fast-yet-driver.js
  } > "$4"
}

# k2 SUITE: every benchmark of SUITE in ITERATIONS_FILE, RUNS processes each, and the suite's
# geometric mean of the benchmark medians as a row of its own.
k2() {
  suite_commit=$(field bench/suites/suites.tsv "$1" 3)
  awk -v s="$1" 'NR > 1 && $1 == s { print $2, $3, $4, $5 }' "$iterations_file" > "$work/benchmarks"
  [ -s "$work/benchmarks" ] || die "no benchmark of $1 in $iterations_file"
  geo_names=
  geo_medians=
  geo_missing=
  while read -r name file iterations inner; do
    script=$work/$name.js
    if [ "$1" = bench-v8 ]; then
      compose_bench_v8 "$file" "$iterations" "$script"
    else
      compose_awfy "$file" "$iterations" "$inner" "$script"
    fi
    values=
    maxrss=
    : > "$work/$name.iterations"
    reason=
    clock=
    r=1
    while [ "$r" -le "$runs" ]; do
      line=$(measure "$script" "$work/$name.$r" || true)
      exit_code=$(tool_field "$line" exit)
      signal=$(tool_field "$line" signal)
      timed_out=$(tool_field "$line" timed_out)
      if [ "$exit_code" != 0 ] || ! grep -q '^done ' "$work/$name.$r.out"; then
        reason=$(failure_reason "$work/$name.$r" "$exit_code" "$signal" "$timed_out")
        break
      fi
      clock=$(sed -n 's/^clock //p' "$work/$name.$r.out")
      awk -v n="$iterations" '
        $1 == "iteration" { t[$3] += $4; total += $4 }
        END {
          printf "["
          for (i = 0; i < n; i++) printf "%s%.6f", (i ? "," : ""), t[i]
          printf "]\n"
          printf "%.6f\n", total / n > "/dev/stderr"
        }' "$work/$name.$r.out" >> "$work/$name.iterations" 2> "$work/$name.mean"
      values="$values $(cat "$work/$name.mean")"
      maxrss="$maxrss $(( $(tool_field "$line" maxrss_kib) * 1024 ))"
      r=$((r + 1))
    done
    extra=$(jq -n -c --arg suite "$1" --arg suite_commit "$suite_commit" --arg benchmark "$name" \
      --argjson iterations "$iterations" --arg inner "$inner" --arg clock "$clock" \
      --slurpfile iteration_ms "$work/$name.iterations" \
      --argjson maxrss "$(echo "$maxrss" | json_numbers)" '
      {suite: $suite, suite_commit: $suite_commit, benchmark: $benchmark,
       iterations_per_run: $iterations,
       inner_iterations: (if $inner == "-" then null else ($inner | tonumber) end),
       clock: $clock, iteration_ms: $iteration_ms, peak_rss_bytes: $maxrss,
       value: "mean time per iteration of each process"}')
    if [ -z "$reason" ]; then
      row K2 "$1/$name" ms measured "" "$values" "$extra"
      geo_medians="$geo_medians $(echo "$values" | median)"
    else
      row K2 "$1/$name" ms not-measured "$reason" "" "$extra"
      geo_missing="$geo_missing $name"
    fi
    geo_names="$geo_names $name"
  done < "$work/benchmarks"
  names_json=$(echo "$geo_names" | tr ' ' '\n' | awk 'NF' | jq -R . | jq -s -c .)
  if [ -z "$geo_missing" ]; then
    geo=$(echo "$geo_medians" | tr ' ' '\n' | awk 'NF { s += log($1); n++ } END { printf "%.6f\n", exp(s / n) }')
    medians_json=$(echo "$geo_medians" | json_numbers)
    jq -n -c --arg engine "$engine" --arg config "$config" --arg version "$version" --arg mode "$mode" \
      --arg flags "$cfg_flags" --arg env "$cfg_env" --arg verified "$verified" \
      --arg suite "$1" --arg suite_commit "$suite_commit" --argjson geo "$geo" \
      --argjson medians "$medians_json" --argjson names "$names_json" --argjson runs "$runs" \
      --arg sha256 "$sha" --arg binary_sha256 "$binary_sha" --arg cpus "$cpus" \
      --arg image_digest "$image_digest" --arg measured_at "$started_at" '
      def list: if . == "-" then [] else split(",") end;
      {engine: $engine, config: $config, version: $version, mode: $mode, flags: ($flags | list),
       env: ($env | list), flags_verified: ($verified == "yes"),
       kpi: "K2", measurement: ($suite + "/geomean"), unit: "ms", runs: $runs,
       values: $medians, median: $geo, status: "measured", reason: "",
       aggregate: "geometric mean of the benchmark medians", of: $names,
       suite: $suite, suite_commit: $suite_commit,
       sha256: $sha256, binary_sha256: $binary_sha256, cpuset: $cpus,
       image_digest: $image_digest, measured_at: $measured_at}' >> "$rows"
  else
    jq -n -c --arg engine "$engine" --arg config "$config" --arg version "$version" --arg mode "$mode" \
      --arg flags "$cfg_flags" --arg env "$cfg_env" --arg verified "$verified" \
      --arg suite "$1" --arg suite_commit "$suite_commit" --arg missing "$geo_missing" \
      --argjson names "$names_json" --argjson runs "$runs" \
      --arg sha256 "$sha" --arg binary_sha256 "$binary_sha" --arg cpus "$cpus" \
      --arg image_digest "$image_digest" --arg measured_at "$started_at" '
      def list: if . == "-" then [] else split(",") end;
      {engine: $engine, config: $config, version: $version, mode: $mode, flags: ($flags | list),
       env: ($env | list), flags_verified: ($verified == "yes"),
       kpi: "K2", measurement: ($suite + "/geomean"), unit: "ms", runs: $runs,
       values: [], median: null, status: "not-measured",
       reason: ("not every benchmark measured; failed:" + $missing),
       aggregate: "geometric mean of the benchmark medians", of: $names,
       suite: $suite, suite_commit: $suite_commit,
       sha256: $sha256, binary_sha256: $binary_sha256, cpuset: $cpus,
       image_digest: $image_digest, measured_at: $measured_at}' >> "$rows"
  fi
}

# start: K3 and K4 of the empty script (the same processes), K4 of bench-v8 Splay, K5.
start() {
  : > "$work/empty.js"
  walls=
  rss=
  reason=
  r=1
  while [ "$r" -le "$runs" ]; do
    line=$(measure "$work/empty.js" "$work/empty.$r" || true)
    exit_code=$(tool_field "$line" exit)
    if [ "$exit_code" != 0 ]; then
      reason=$(failure_reason "$work/empty.$r" "$exit_code" "$(tool_field "$line" signal)" "$(tool_field "$line" timed_out)")
      break
    fi
    walls="$walls $(tool_field "$line" wall_us)"
    rss="$rss $(( $(tool_field "$line" maxrss_kib) * 1024 ))"
    r=$((r + 1))
  done
  empty='{"workload": "an empty script file (0 bytes)"}'
  if [ -z "$reason" ]; then
    row K3 empty-script µs measured "" "$walls" "$empty"
    row K4 empty-script bytes measured "" "$rss" "$empty"
  else
    row K3 empty-script µs not-measured "$reason" "" "$empty"
    row K4 empty-script bytes not-measured "$reason" "" "$empty"
  fi

  splay_iterations=$(awk 'NR > 1 && $1 == "bench-v8" && $2 == "Splay" { print $4 }' "$iterations_file")
  compose_bench_v8 splay.js "$splay_iterations" "$work/splay.js"
  rss=
  reason=
  r=1
  while [ "$r" -le "$runs" ]; do
    line=$(measure "$work/splay.js" "$work/splay.$r" || true)
    exit_code=$(tool_field "$line" exit)
    if [ "$exit_code" != 0 ] || ! grep -q '^done ' "$work/splay.$r.out"; then
      reason=$(failure_reason "$work/splay.$r" "$exit_code" "$(tool_field "$line" signal)" "$(tool_field "$line" timed_out)")
      break
    fi
    rss="$rss $(( $(tool_field "$line" maxrss_kib) * 1024 ))"
    r=$((r + 1))
  done
  splay=$(jq -n -c --arg c "$(field bench/suites/suites.tsv bench-v8 3)" --argjson n "$splay_iterations" \
    '{workload: "bench-v8 Splay", suite: "bench-v8", suite_commit: $c, iterations_per_run: $n}')
  if [ -z "$reason" ]; then
    row K4 bench-v8/Splay bytes measured "" "$rss" "$splay"
  else
    row K4 bench-v8/Splay bytes not-measured "$reason" "" "$splay"
  fi

  dir=$image/engines/$engine
  : > "$work/k5.files"
  for pattern in $(echo "$loads" | tr ',' ' '); do
    # shellcheck disable=SC2086
    for path in "$dir"/$pattern; do
      [ -f "$path" ] && [ ! -L "$path" ] && echo "${path#"$dir"/}" >> "$work/k5.files"
    done
  done
  bytes=0
  stripped=yes
  while read -r file; do
    bytes=$((bytes + $(stat -c %s "$dir/$file")))
    kind=$(file -b "$dir/$file")
    case $kind in
      *ELF*"not stripped"*) stripped=no ;;
      *ELF*) ;;
    esac
  done < "$work/k5.files"
  k5=$(jq -n -c --arg stripped "$stripped" \
    --argjson list "$(jq -R . "$work/k5.files" | jq -s -c .)" '
    {workload: "the shell as distributed: its binary and the files it loads from its own directory",
     files: $list, stripped: (if $stripped == "yes" then "every ELF file stripped" else "not stripped" end),
     target: "x86_64-unknown-linux-gnu", profile: "as distributed", strip_command: "none"}')
  row K5 shell-as-distributed bytes measured "" "$bytes" "$k5"
}

prepare() {
  image=$(image_build .out/bench/prepare-image.log) || exit 1
  check_counts "$image" bench/suites/iterations.tsv
  cat .out/bench/prepare-image.log
  echo "image $image"
  echo "image_digest sha256:$(sha256sum "$image/manifest.tsv" | cut -d' ' -f1)"
  tool=$(build_tool) || exit 1
  echo "tool $tool"
}

origin() {
  sha256sum /etc/machine-id | cut -c1-16
}

# cpuset WORKER: two CPUs of this node for the worker, CPU 0 and 1 left to the system.
cpuset() {
  n=$(nproc)
  if [ "$n" -lt 4 ]; then
    echo "0-$((n - 1))"
    return
  fi
  pair=$(( $1 % ((n - 2) / 2) ))
  echo "$((2 + 2 * pair)),$((3 + 2 * pair))"
}

job() {
  [ $# -ge 4 ] || die "job CONFIG SUITE WORKER ORIGIN [RUNS] [ITERATIONS_FILE]"
  config=$1
  suite=$2
  [ "$4" != "$(origin)" ] || die "job $config $suite runs on the machine that started it; benchmarks run on the build node only"
  cpus=$(cpuset "$3")
  iterations_file=${6:-bench/suites/iterations.tsv}
  case $suite in
    bench-v8|are-we-fast-yet) runs=${5:-$RUNS_K2} ;;
    start) runs=${5:-$RUNS_START} ;;
    *) die "unknown suite $suite" ;;
  esac
  engine=$(field bench/image/configs.tsv "$config" 2)
  [ -n "$engine" ] || die "unknown config $config"
  mode=$(field bench/image/configs.tsv "$config" 3)
  cfg_flags=$(field bench/image/configs.tsv "$config" 4)
  cfg_env=$(field bench/image/configs.tsv "$config" 5)
  verified=$(field bench/image/configs.tsv "$config" 6)
  version=$(field bench/image/engines.tsv "$engine" 2)
  sha=$(field bench/image/engines.tsv "$engine" 4)
  binary=$(field bench/image/engines.tsv "$engine" 5)
  loads=$(field bench/image/engines.tsv "$engine" 6)

  work=.out/bench/job/$config.$suite
  rm -rf "$work"
  mkdir -p "$work"
  rows=$work/rows.jsonl
  : > "$rows"
  started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  load_start=$(cut -d' ' -f1 /proc/loadavg)

  image=$(image_build "$work/image.log") || exit 1
  image_digest=sha256:$(sha256sum "$image/manifest.tsv" | cut -d' ' -f1)
  check_counts "$image" "$iterations_file"
  tool=$(build_tool 2> "$work/tool.log") || die "no swell-bench-exec; see $work/tool.log: $(tail -n 1 "$work/tool.log")"
  [ -x "$tool" ] || die "no swell-bench-exec; see $work/tool.log"
  bin=$image/engines/$engine/$binary
  binary_sha=$(sha256sum "$bin" | cut -d' ' -f1)
  run_flags=$(echo "$cfg_flags" | sed 's/^-$//' | tr ',' ' ')
  run_env="LD_LIBRARY_PATH=$image/engines/$engine $(echo "$cfg_env" | sed 's/^-$//' | tr ',' ' ')"

  case $suite in
    start) start ;;
    *) k2 "$suite" ;;
  esac

  load_end=$(cut -d' ' -f1 /proc/loadavg)
  ncpu=$(nproc)
  host=$(jq -n -c --arg arch "$(uname -m)" --argjson cpus "$ncpu" \
    --arg cpu_model "$(sed -n 's/^model name[[:space:]]*: //p' /proc/cpuinfo | head -n 1)" \
    --arg kernel "$(uname -sr)" '{arch: $arch, cpus: $cpus, cpu_model: $cpu_model, kernel: $kernel}')
  jq -c --argjson s "$load_start" --argjson e "$load_end" --argjson n "$ncpu" --argjson b "$LOADED_PER_CPU" \
    --argjson host "$host" '
    . + {host: $host, load: {start: $s, end: $e, cpus: $n},
         loaded: (($s / $n) > $b or ($e / $n) > $b)}' "$rows" > "$rows.with-load"
  echo "== host $host"
  echo "== rows begin"
  cat "$rows.with-load"
  echo "== rows end"
}

# node_state: the node's job list, as BENCH_NODE_STATE prints it (the JSON of `slot state`:
# .now_ms, .active[], .done[]); fails when the command prints no object with a .now_ms. Its
# stdin is /dev/null: `all` reads its job list on stdin, and a command such as ssh reads stdin.
node_state() {
  sh -c "$BENCH_NODE_STATE" < /dev/null 2> /dev/null \
    | jq -c 'select(type == "object" and (.now_ms | type) == "number")' 2> /dev/null | grep .
}

# wait_idle CONFIG SUITE: wait until the node runs no pool job, reading its job list every
# IDLE_POLL_S seconds, at most IDLE_WAIT_S seconds; then return either way. Sets since, the
# node's clock (ms) at the last read, 0 when no read succeeded.
wait_idle() {
  since=0
  t0=$(date +%s)
  while :; do
    busy=unknown
    if state=$(node_state); then
      since=$(printf '%s\n' "$state" | jq '.now_ms')
      busy=$(printf '%s\n' "$state" | jq '[(.active // [])[] | select(.started_ms > 0)] | length')
    fi
    [ "$busy" != 0 ] || return 0
    waited=$(($(date +%s) - t0))
    if [ "$waited" -ge "$IDLE_WAIT_S" ]; then
      if [ "$busy" = unknown ]; then
        echo "bench: cannot read the node's job list after $IDLE_WAIT_S s; starting job $1 $2" >&2
      else
        echo "bench: the node is not idle after $IDLE_WAIT_S s; starting job $1 $2 beside it" >&2
      fi
      return 0
    fi
    nap=$((IDLE_WAIT_S - waited))
    [ "$nap" -le "$IDLE_POLL_S" ] || nap=$IDLE_POLL_S
    sleep "$nap"
  done
}

# neighbours CONFIG SUITE WORKER ORIGIN SINCE FILE: the node's record of the job (the newest
# job of `bench/run.sh job CONFIG SUITE WORKER ORIGIN` that started at or after SINCE) and every
# other pool job whose time overlaps it, into FILE: {job: {id, started_ms, ended_ms},
# neighbours: [{id, repo, started_ms, ended_ms, command}]}, the command its first 80
# characters with each /home/<name>/ cut to ~/, an ended_ms of 0 a job that still runs. Without
# a record of the job, {job: null, neighbours: [], unwatched: <why>}.
neighbours() {
  if ! state=$(node_state); then
    jq -n -c '{job: null, neighbours: [], unwatched: "cannot read the node'"'"'s job list after the job"}' > "$6"
    return 0
  fi
  printf '%s\n' "$state" | jq -c --arg job "bench/run.sh job $1 $2 $3 $4" --argjson since "$5" '
    .now_ms as $now
    | def ends: if .ended_ms > 0 then .ended_ms else $now end;
    [(.done // [])[], (.active // [])[]] as $jobs
    | ([$jobs[] | select(((.argv // []) | join(" ") | contains($job)) and .started_ms >= $since)]
       | max_by(.started_ms)) as $j
    | if $j == null then
        {job: null, neighbours: [], unwatched: "no record of the job in the node'"'"'s job list"}
      else
        {job: {id: $j.id, started_ms: $j.started_ms, ended_ms: $j.ended_ms},
         neighbours: [$jobs[] | select(.id != $j.id and .started_ms > 0
             and .started_ms < ($j | ends) and ends > $j.started_ms)
           | {id, repo, started_ms, ended_ms,
              command: ((.argv // []) | join(" ") | gsub("/home/[^/]+/"; "~/") | .[0:80])}]}
      end' > "$6"
}

# all [WORKERS]: one pool job per (config, suite). A job whose log has no rows is run once more
# by a later `all` with the same date (jobs with rows are kept); the first job the pool refuses
# ("did not run") stops every worker, and nothing is assembled. Without BENCH_NODE_STATE it
# refuses to start; with BENCH_ALLOW_UNWATCHED=1 it runs without watching the node, and every
# row is loaded: true with loaded_reason unwatched.
all() {
  if [ -z "${BENCH_NODE_STATE:-}" ]; then
    [ "${BENCH_ALLOW_UNWATCHED:-}" = 1 ] || die "BENCH_NODE_STATE is not set, a command that prints the JSON of \`slot state\` of the build node: without it no row can say whether it was measured alone, so the run is not started (BENCH_ALLOW_UNWATCHED=1 runs it anyway, every row loaded: true, loaded_reason unwatched)"
    echo "bench: BENCH_ALLOW_UNWATCHED=1: the node is not watched; every row is loaded: true, loaded_reason unwatched" >&2
  fi
  workers=${1:-1}
  stamp=$(date -u +%Y-%m-%d)
  out=.out/bench/run-$stamp
  mkdir -p "$out/logs"
  rm -f "$out/stop"
  here=$(origin)
  slot --timeout 30m sh bench/run.sh prepare > "$out/logs/prepare.log" 2>&1 \
    || die "prepare failed; see $out/logs/prepare.log"
  awk 'NR > 1 { print $1, "bench-v8"; print $1, "are-we-fast-yet"; print $1, "start" }' \
    bench/image/configs.tsv > "$out/jobs.list"
  w=0
  while [ "$w" -lt "$workers" ]; do
    (
      k=0
      while read -r config suite; do
        log=$out/logs/$config.$suite.log
        if [ $((k % workers)) -eq "$w" ] && [ ! -f "$out/stop" ] && ! grep -q '^== rows end$' "$log" 2>/dev/null; then
          rm -f "$out/logs/$config.$suite.neighbours.json"
          since=0
          [ -z "${BENCH_NODE_STATE:-}" ] || wait_idle "$config" "$suite"
          if ! slot --timeout 30m --tokens 2 sh bench/run.sh job "$config" "$suite" "$w" "$here" < /dev/null > "$log" 2>&1; then
            if grep -q '^slot: .*did not run' "$log"; then
              grep 'slot:' "$log" | tail -n 3 > "$out/stop"
              echo "bench: the pool refused job $config $suite; stopping: $(cat "$out/stop")" >&2
            else
              echo "bench: job $config $suite failed; see $log" >&2
            fi
          fi
          [ -z "${BENCH_NODE_STATE:-}" ] ||
            neighbours "$config" "$suite" "$w" "$here" "$since" "$out/logs/$config.$suite.neighbours.json"
        fi
        k=$((k + 1))
      done < "$out/jobs.list"
    ) &
    w=$((w + 1))
  done
  wait
  [ ! -f "$out/stop" ] || die "stopped: the pool refused a job ($out/stop)"
  assemble "$out" "bench/results/$stamp-$(git rev-parse --short HEAD).json"
}

# assemble DIR RESULT: the rows of each DIR/logs/<config>.<suite>.log into RESULT. Every row of
# a job is loaded: true, with loaded_reason, when its job had a neighbour (its
# <config>.<suite>.neighbours.json lists one), a load over the bound (the job's own loaded),
# or no record of the node (no neighbours file, or one without the job: unwatched); else
# loaded: false without loaded_reason.
assemble() {
  [ $# -eq 2 ] || die "assemble DIR RESULT"
  dir=$1
  result=$2
  : > "$dir/rows.jsonl"
  : > "$dir/hosts.jsonl"
  : > "$dir/jobs.jsonl"
  missing=
  for log in "$dir"/logs/*.log; do
    case $log in */prepare.log) continue ;; esac
    if ! grep -q '^== rows end$' "$log"; then
      missing="$missing ${log##*/}"
      continue
    fi
    nb=${log%.log}.neighbours.json
    watched=false
    neighbour_count=0
    if [ -f "$nb" ] && jq -e '.job != null' "$nb" > /dev/null 2>&1; then
      watched=true
      neighbour_count=$(jq '.neighbours | length' "$nb")
    fi
    jq -n -c --arg log "${log##*/}" --argjson watched "$watched" --argjson n "$neighbour_count" \
      '{log: $log, watched: $watched, neighbours: $n}' >> "$dir/jobs.jsonl"
    sed -n '/^== rows begin$/,/^== rows end$/p' "$log" | sed '1d;$d' \
      | jq -c --argjson watched "$watched" --argjson n "$neighbour_count" '
        [(if $n > 0 then "neighbour" else empty end),
         (if .loaded == true then "load" else empty end),
         (if $watched then empty else "unwatched" end)] as $why
        | del(.loaded_reason)
        | if ($why | length) == 0 then . + {loaded: false}
          else . + {loaded: true, loaded_reason: (if ($why | length) == 1 then $why[0] else $why end)} end' \
      >> "$dir/rows.jsonl"
    sed -n "s/^== host //p" "$log" >> "$dir/hosts.jsonl"
  done
  [ -z "$missing" ] || die "job(s) without rows:$missing"
  [ -s "$dir/hosts.jsonl" ] || die "no job log in $dir/logs"
  harness=$(cat bench/run.sh bench/suites/*.js bench/suites/*.tsv bench/image/*.tsv bench/image/build.sh \
    bench/tool/src/main.rs bench/tool/Cargo.toml bench/tool/Cargo.lock | sha256sum | cut -d' ' -f1)
  tail -n +2 bench/image/engines.tsv | awk '{ print $1, $2, $4, $7 }' \
    | jq -R -c 'split(" ") | {engine: .[0], version: .[1], sha256: .[2], url: .[3]}' > "$dir/engines.jsonl"
  tail -n +2 bench/suites/suites.tsv | while read -r suite repo commit sub; do
    awk -v s="$suite" 'NR > 1 && $1 == s { print $2, $3, $4 }' bench/suites/files.tsv \
      | jq -R -c 'split(" ") | {file: .[0], sha256: .[1], licence: .[2]}' | jq -s -c \
        --arg suite "$suite" --arg repo "$repo" --arg commit "$commit" --arg sub "$sub" \
        '{suite: $suite, repo: $repo, commit: $commit, dir: $sub, files: .}'
  done > "$dir/suites.jsonl"
  mkdir -p "$(dirname "$result")"
  jq -n \
    --arg date "$(date -u +%Y-%m-%d)" --arg commit "$(git rev-parse HEAD)" \
    --slurpfile hosts "$dir/hosts.jsonl" --arg harness "sha256:$harness" \
    --argjson timeout "$PROCESS_TIMEOUT_S" --argjson memory "$MEMORY_LIMIT_BYTES" \
    --argjson bound "$LOADED_PER_CPU" \
    --slurpfile rows "$dir/rows.jsonl" --slurpfile engines "$dir/engines.jsonl" \
    --slurpfile suites "$dir/suites.jsonl" --slurpfile jobs "$dir/jobs.jsonl" '
    def swell($kpi; $measurement; $unit):
      {engine: "swell", config: "swell", version: "none", mode: "interpreter", flags: [],
       kpi: $kpi, measurement: $measurement, unit: $unit, runs: 0, values: [], median: null,
       status: "not-measured",
       reason: "not in this run; swell gets its configuration with story swell-in-the-benchmarks"};
    {format: "swell-bench/1", date: $date, commit: $commit,
     host: (($hosts | group_by(.) | max_by(length) | .[0]) as $h
       | ($h + {runner: "build node, \($h.cpus) cores, \($h.cpu_model), through the shared build pool (slot --timeout 30m)",
                     container_runtime: "none: the docker daemon does not run for the pool user and rootless podman is refused (newuidmap without setuid); each process runs in unshare -rn, taskset -c and prlimit --data"})),
     hosts: ($hosts | unique),
     image_digest: ($rows[0].image_digest),
     harness_digest: $harness,
     limits: {process_timeout_s: $timeout, memory_rlimit_data_bytes: $memory,
              network: "none (a network namespace with only a down loopback)",
              cpuset: "two CPUs per job, named in each row", loaded_bound_per_cpu: $bound},
     loaded: {rule: "a row is loaded: true when another pool job ran on the build node during its job (neighbour), the node'"'"'s 1-minute load per CPU at its job'"'"'s start or end was over limits.loaded_bound_per_cpu (load), or its job had no record of the node (unwatched); loaded_reason names which",
              jobs: ($jobs | length),
              jobs_with_neighbour: ([$jobs[] | select(.neighbours > 0)] | length),
              rows_loaded: ([$rows[] | select(.loaded == true)] | length)},
     engines: $engines, suites: $suites,
     rows: ($rows + [
       swell("K2"; "bench-v8/geomean"; "ms"),
       swell("K2"; "are-we-fast-yet/geomean"; "ms"),
       swell("K3"; "empty-script"; "µs"),
       swell("K4"; "empty-script"; "bytes"),
       swell("K4"; "bench-v8/Splay"; "bytes"),
       swell("K5"; "shell-as-distributed"; "bytes")])}' > "$result.part"
  mv "$result.part" "$result"
  echo "bench: wrote $result, $(jq '.rows | length' "$result") row(s)"
}

command=${1:-}
[ $# -gt 0 ] && shift
case $command in
  prepare) prepare ;;
  job) job "$@" ;;
  all) all "$@" ;;
  assemble) assemble "$@" ;;
  *) die "usage: sh bench/run.sh all|prepare|job|assemble (see the head of bench/run.sh)" ;;
esac

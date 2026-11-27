# Sourced by the planted scripts of tests/gate/ that run `sh bench/run.sh all` without an engine,
# a pool or a build node. Not a script of gate-selftest: it lives outside tests/gate/*.sh.
#
# `bench_node_plant <dir>` makes:
# - <dir>/tree: a root with bench/run.sh, bench/suites, bench/schema, bench/image and the
#   sources of bench/tool of this tree (assemble's digest of the harness reads them), and a bench/image/configs.tsv of one configuration (planted), so that `all` runs three
#   jobs: planted bench-v8, planted are-we-fast-yet, planted start;
# - <dir>/bin/slot: a stub of the pool's client. `slot [--timeout T] [--tokens N] sh bench/run.sh
#   prepare` prints one line. `... job CONFIG SUITE WORKER ORIGIN` runs nothing: it prints a job
#   log with a "== host" line and two measured rows between "== rows begin" and "== rows end",
#   and writes the job into the planted node's record <dir>/state.json (the form of `slot
#   state`: .now_ms, .active[], .done[]; each job .id, .started_ms, .ended_ms, .repo, .argv):
#   a done job of repo swell that starts 10 ms after the record's now_ms and ends 20 ms after
#   it, and then now_ms moves 40 ms on. A suite named in BENCH_PLANT_NEIGHBOUR (a list of
#   suites, blank-separated) also gets one done job of repo planted-neighbour from 15 to 30 ms,
#   whose command names a path under the home directory of the user (HOME); a suite named in
#   BENCH_PLANT_LOAD gets rows with a load over the bound (loaded: true from the job).
# - <dir>/state.json: an idle node (no active job) at now_ms 1000000.
# Sets node_state, the command for BENCH_NODE_STATE that prints the record and counts each
# read in <dir>/reads. The caller runs `all` from <dir>/tree with PATH="<abs dir>/bin:$PATH".
# Calls fail (of the sourcing script) on an error.

bench_node_plant() {
  rm -rf "$1/tree" "$1/bin" && mkdir -p "$1/tree/bench/tool/src" "$1/bin" || fail "cannot make $1/tree"
  plant=$(cd "$1" && pwd) || fail "cannot resolve $1"
  cp -R bench/image bench/suites bench/schema bench/run.sh "$1/tree/bench/" &&
    cp bench/tool/Cargo.toml bench/tool/Cargo.lock "$1/tree/bench/tool/" &&
    cp bench/tool/src/main.rs "$1/tree/bench/tool/src/" || fail "cannot copy bench/"
  printf '%s\n' 'config engine mode flags env verified' 'planted planted interpreter - - yes' \
    > "$1/tree/bench/image/configs.tsv" || fail "cannot plant configs.tsv"
  printf '%s\n' '{"now_ms":1000000,"active":[],"done":[]}' > "$1/state.json" ||
    fail "cannot plant the node's record"
  : > "$1/reads"
  cat > "$1/bin/slot" <<'EOF' || fail "cannot plant the slot stub"
#!/bin/sh
# Stub of the pool's client (tests/gate/fixtures/bench-node.sh): runs no job.
while :; do
  case $1 in
    --timeout|--tokens) shift 2 ;;
    *) break ;;
  esac
done
[ "$1 $2" = "sh bench/run.sh" ] || { echo "slot stub: unexpected command: $*" >&2; exit 2; }
case $3 in
  prepare) echo "planted prepare"; exit 0 ;;
  job) ;;
  *) echo "slot stub: unexpected step $3" >&2; exit 2 ;;
esac
config=$4 suite=$5
neighbour=no
for s in ${BENCH_PLANT_NEIGHBOUR:-}; do [ "$s" = "$suite" ] && neighbour=yes; done
load=0.1
for s in ${BENCH_PLANT_LOAD:-}; do [ "$s" = "$suite" ] && load=9.0; done
jq -c --arg argv "sh bench/run.sh job $4 $5 $6 $7" --arg neighbour "$neighbour" \
  --arg home "$HOME" '
  .now_ms as $t
  | .done += [{id: ($t + 1), started_ms: ($t + 10), ended_ms: ($t + 20), repo: "swell",
               argv: ($argv | split(" "))}]
  | (if $neighbour == "yes" then
       .done += [{id: ($t + 2), started_ms: ($t + 15), ended_ms: ($t + 30), repo: "planted-neighbour",
                  argv: ["cargo", "test", "--manifest-path", ($home + "/planted/Cargo.toml")]}]
     else . end)
  | .now_ms = $t + 40' "$BENCH_PLANT_STATE" > "$BENCH_PLANT_STATE.next" &&
  mv "$BENCH_PLANT_STATE.next" "$BENCH_PLANT_STATE" || exit 3
loaded=false
[ "$load" = 0.1 ] || loaded=true
echo "planted job $config $suite"
echo '== host {"arch":"x86_64","cpus":4,"cpu_model":"planted","kernel":"planted"}'
echo '== rows begin'
for kpi in K3 K4; do
  case $kpi in K3) unit=µs ;; K4) unit=bytes ;; esac
  printf '%s\n' '{"engine":"planted","config":"'"$config"'","version":"1","mode":"interpreter","flags":[],"kpi":"'"$kpi"'","measurement":"'"$suite"'/empty-script","unit":"'"$unit"'","runs":3,"values":[900,1000,1100],"median":1000,"min":900,"max":1100,"status":"measured","reason":"","image_digest":"sha256:0000000000000000000000000000000000000000000000000000000000000000","load":{"start":'"$load"',"end":'"$load"',"cpus":1},"loaded":'"$loaded"'}'
done
echo '== rows end'
EOF
  chmod +x "$1/bin/slot" || fail "cannot make the slot stub executable"
  node_state="echo read >> '$plant/reads'; cat '$plant/state.json'"
  export BENCH_PLANT_STATE="$plant/state.json"
}

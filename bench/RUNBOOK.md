# Running the benchmarks

The commands below run the comparison of `docs/design/quality-and-comparison.md` (K2 to K5) on the
build node. Every measuring step is a pool job on the node (`slot`, `--timeout` at most 30
minutes); none runs on the workstation, and each job refuses to measure when it finds itself on
the machine that started it (the origin digest, the first 16 hex digits of the sha256 of the
starting machine's `/etc/machine-id`).

Run from the repository root of a managed worktree, in this order:

1. Smoke: image, tool, one hello script per configuration, the flag variants.

   ```sh
   slot --timeout 20m sh bench/smoke.sh "$(sha256sum /etc/machine-id | cut -c1-16)"
   ```

   Read `.out/bench-probe/` for each engine's "hello 2" line and which of `class`, destructuring,
   `performance.now`, `preciseTime` it has; set flags in `bench/image/configs.tsv` from it.

2. Pilot: one run of one benchmark per suite, to set `bench/suites/iterations.tsv` so a job fits 30
   minutes.

   ```sh
   o=$(sha256sum /etc/machine-id | cut -c1-16)
   slot --timeout 30m --tokens 2 sh bench/run.sh job v8-jitless bench-v8 1 "$o" 1
   slot --timeout 30m --tokens 2 sh bench/run.sh job quickjs are-we-fast-yet 1 "$o" 1
   ```

3. The run: one pool job per (configuration, suite), six at a time. The orchestrator itself only
   waits, so it starts without the `slot` prefix; each job it starts carries it.

   ```sh
   sh bench/run.sh all 6
   ```

   It writes `bench/results/<date>-<commit>.json` and stops at the first job the pool refuses.

4. The gate, then the site data:

   ```sh
   slot --timeout 30m task check-runner
   task site-data
   ```

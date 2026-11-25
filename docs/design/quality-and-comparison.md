# Design: what makes swell good, and how it compares with other engines

Written on 2026-11-25 on `main` at `3785379` (release `0.11.0`). Nothing in it is built. It answers
the owner's order of 2026-11-25: every product-facing repository states what makes its product good
in the format `ratchet/1` of the beyond10x ratchet project; for swell that is conformance, run time,
level of confinement and what research adds; other ECMAScript engines from several ecosystems are
measured or quoted on the same terms; and a page of the site lets a reader pick engines and see
them side by side.

This document shares its harness with `docs/design/native-code.md` section 5 (one container image,
one runner, one set of result files). It adds no second harness. Where the two touch, that section
is named.

Marks: UNVERIFIED is a fact that could not be read from the cited page. Every number carries its
source: a URL, or `file:line` in this tree.

## 1. What good means for swell

The KPI list was sent to the ratchet maintainers and agreed with them on 2026-11-25, with changes to
K2, K3 and K4 and details for the others; this section has those changes applied. Each KPI is
either a gate (judged on swell, can fail a merge or a release) or compared (shown beside other
engines, judges nothing), or both.

Common terms:

- Exact: one value per run, tolerance 0. Sampled: several runs, aggregated; the number of runs, the
  tolerance, the runner and the load bound are part of the definition.
- The runner for every sampled value is the build node, reached through the shared build pool
  (`slot --timeout 30m`), never the workstation (the owner's order of 2026-11-25). The record of a
  run holds the 1-minute load average at start and end, CPU model, core count, kernel, container
  runtime and image digest, no host name.
- Load bound for sampled values: a sample set whose load per CPU is above 1.0 at start or end is
  marked `loaded` and is not the value the page shows by default. Proposed here; the first run
  confirms or changes it.
- ratchet/1 converts no units, so each measurement has exactly one unit.

### K1. test262 conformance at a pinned revision

- Input: one pinned triple, the same for every engine in a comparison: the test262 revision (swell's
  pin `2a1ade303f72d3de80565c921d685222405c32ce`, `conformance/test262/PIN:2`), the harness (the
  runner of section 4) and the exclusion list. The exclusion list for a comparison is empty: an
  unsupported feature is `failed`, never `skipped`.
- Gate, swell only: the passing case set, an identity set (`<path>#<mode>`), requirement
  `no_lost_items` of preset `conformance-cases@1`. Today the gate holds counts per directory, not
  identities (`conformance/test262/expected.txt`, held by `test262-check`, `taskfiles/test262.yml:550`).
- Compared: pass count, exact, tolerance 0, higher is better. Unit: cases. A case passes when every
  mode that applies to it passes (`crates/test262/src/count.rs:492-493`). The denominator is every
  case at the pin: 53,801 (`conformance/test262/expected.txt:222`).
- swell today, two readings of the same run (`conformance/test262/expected.txt:222`): 25,943 of the
  44,812 run cases pass, 57.8 % (`website/data/status.json:30`); counted as compared, with the 8,982
  excluded and 7 not applicable cases as failed, 25,943 of 53,801, 48.2 %.
- Also shown per top directory (sums of the directory lines `conformance/test262/expected.txt:1-221`):
  annexB 190 of 1,086, built-ins 12,581 of 23,885, harness 102 of 117, intl402 0 of 3,394, language
  12,361 of 23,832, staging 709 of 1,487.
- ratchet 0.1.0 is planned to judge both the identity set and the count (ratchet answer, 2026-11-25).

### K2. Interpreter throughput

- Two measurements, one per suite, never a mean across both: bench-v8 version 7 and are-we-fast-yet
  (https://github.com/smarr/are-we-fast-yet).
- Per benchmark: time per iteration in ms, median over runs, lower is better, scope `benchmark`.
  Per suite: the geometric mean of the benchmark medians, reported by the assessor as an output of
  its own (ratchet/1 has no geometric-mean aggregation; its kinds are sum, median, percentile,
  best_of_n, pass_rate, per the ratchet answer).
- Sampled: 5 runs, each a separate process (`native-code.md` section 5); no tolerance, since nothing
  judges it; the build node; the load bound above. Time taken inside the process with the shell's
  clock, every iteration kept (`native-code.md` section 5).
- Per peer: version and flags pinned in the record. Compared against interpreter-only
  configurations; JIT configurations are a separate band labelled "JIT reference" (section 5).
- Compare only. No release judges a sampled value yet (ratchet answer, 2026-11-25).
- swell today: not measured. No benchmark exists (`Cargo.toml:19-20`, the bench profile, sets only `overflow-checks`);
  which benchmarks swell can run is found by the first run (slice 8, classes, is not delivered,
  `conformance/slices-delivered.txt`).

### K3. Start

- Empty script: process start to process exit of the shell running an empty file. Unit µs, lower is
  better. Sampled: 10 runs, median, min and max shown.
- New realm: µs per realm over N = 1,000 calls of `$262.createRealm()` in one process, timed with
  the shell's clock. Sampled: 10 runs. A shell without `$262.createRealm` shows "not measured" with
  that reason.
- Compare only. ratchet's catalog `start-time@1` measures start-to-ready of a service and does not
  fit; this is a plan of swell's own.

### K4. Memory

- Peak RSS on a pinned workload: the highest reading of a run (GNU `time`, `%M`, in the image),
  bytes, median over 10 runs, lower is better. Workloads: the empty script, and bench-v8 v7 Splay
  at its default iteration count (proposed). Compared: this is the only memory row that exists for
  peers.
- Bytes of an empty realm, swell only: exact, from swell's own allocator count (the charge to
  `memory_bytes`, `crates/swell/src/host/limits.rs:15-17`); tolerance 0, can gate. The ratchet draft
  names it `fresh-realm-live-bytes`.

### K5. Binary size

- Bytes of the swell shell binary, exact, lower is better. The definition names four things: target
  `x86_64-unknown-linux-gnu`, profile `release` (`Cargo.toml:13-14`), strip command
  `strip --strip-all`, and the toolchain of `rust-toolchain.toml` (rustc 1.99.1, `AGENTS.md:224`).
- A peer row names the same four, or is labelled "as distributed" with its stripped state from
  `file`. A shell that loads shared libraries from its own directory (SpiderMonkey's `js`) counts
  them, and says so.
- Gate for swell: does not rise against the accepted value (report mode until accepted). Catalog
  preset follows with ratchet `story:preset-catalog-factory`.
- swell today: no binary (`docs/releasing.md:165`). Published example, for scale: V8 amd64 21,934,792
  bytes, V8 jitless 13,329,096 bytes (https://zoo.js.org/engines.json, `bench[].binary_size`,
  file dated 2026-03-04).

### K6. Resource control for untrusted scripts

- Gates for swell, all exact:
  - step-budget overshoot: steps run beyond `steps_per_entry`, `lte 0`;
  - memory-cap overshoot by allocator count beyond `memory_bytes`, `lte 0`;
  - recoverable after a limit: after each of the seven limits (`crates/swell/src/host/limits.rs:12-27`)
    the agent runs the next script, `holds`;
  - interrupt latency in steps from the interrupt request to `Terminated`, exact, gated.
- Observed, not gated: interrupt latency in µs, sampled.
- Compared: a capability matrix per engine: memory cap, step or instruction budget, interrupt or
  terminate, stack limit, behaviour at the limit (catchable, fatal), each cell with its source. From
  the engine catalog (section 3). Examples: V8 has no instruction counter, `RequestInterrupt` can
  build one (https://github.com/v8/v8/blob/main/include/v8-isolate.h); Boa has
  `RuntimeLimits::loop_iteration_limit`
  (https://docs.rs/boa_engine/latest/boa_engine/vm/struct.RuntimeLimits.html); at the heap cap V8
  "will crash with V8::FatalProcessOutOfMemory"
  (https://v8.github.io/api/head/classv8_1_1ResourceConstraints.html).
- Measured overshoot for peers needs an embedding harness per engine (section 8, Q8). Values of
  other engines with no gate are ratchet gap (a), `story:peer-values-beside-candidate`.

### K7. Safety

- Unsafe: crate roots without `#![forbid(unsafe_code)]`, exact, `lte 0`. Today 9 of 9 carry it
  (`website/data/status.json:6-7`, step `forbid-unsafe-check`, `Taskfile.yml:420`).
- Third-party crates: packages of `Cargo.lock` from outside the repository, exact, `lte 0`. Today 8
  packages, 0 outside (`website/data/status.json:8-9`, step `lock-check`, `Taskfile.yml:459`).
- Fuzz: no abort, hang or budget overshoot over a fixed corpus and seed, or a fixed iteration count,
  with `executed: {min: N}` as coverage. A time-boxed run without a count is not comparable. No
  fuzzer exists; it goes in a tooling workspace (`AGENTS.md:108-111`).
- Compared as context only: implementation language, OSS-Fuzz presence
  (https://github.com/google/oss-fuzz/tree/master/projects). Vulnerability counts are not a row:
  engines embedded in browsers have their CVEs filed under the browser's product name, and a count
  follows how widely an engine is used and fuzzed as much as its quality.

### K8. Determinism

- Gate for swell: the assessor replays a corpus twice, one assertion per item (output bytes and
  step counts equal), `holds`. Expressible in ratchet/1 today. The test that does this now is
  `determinism::host_133_same_calls_same_hooks_same_outcomes`
  (`crates/swell/tests/determinism.rs:768`).
- A digest compared between two plans needs ratchet `story:requirement-over-two-measurements`.
- Compared: a capability row only (does the engine offer a deterministic step count), from the
  catalog.

### K9. Development

- Gate, not compared: the gate's test steps end within 120 s of wall time
  (`GATE_TIME_BUDGET`, `taskfiles/gate-time.yml:129`); a test ends within 60 s
  (`.config/nextest.toml:2`). Sampled durations against a fixed limit, boundary `run_time`, queue
  wait apart, in the form of ratchet `examples/host-caused-limit.ratchet.yaml`; judged by ratchet
  after its 0.1.0 (`epic:ci-turnaround`).

### Facts rows (not KPIs)

The page also shows, per engine and with a source per cell: implementation language, licence,
execution model, interpreter-only flag, latest release and releases in the last 12 months. Rejected
as rows: raw JIT peak throughput (unfair against an interpreter), SunSpider, Kraken and Octane
(retired, https://v8.dev/blog/retiring-octane), vulnerability counts (above), GC pause (no
cross-engine data found).

## 2. The quality file

- `quality/swell.ratchet.yaml`: format `ratchet/1`, `project: swell`. Statements, subjects, preset
  bindings, assessments, requirements and gates `merge` (pre-merge, local and ci) and `release`
  (pre-promotion, ci). No YAML comments.
- `ratchet.toml` at the root (per checkout, no policy): `[spec] paths = ["quality/*.ratchet.yaml"]`,
  `[catalog] sources` with the ratchet repository's `presets/` and `{ path = "quality/assessors" }`.
- Assessor declarations: `quality/assessors/<name>@<version>.assessor.yaml`.
- No ratchet command resolves a binding yet, so the file copies the preset entries of
  `conformance-cases@1` and `known-failures@1`, with the digests of ratchet commit `90924c9`
  (ratchet `presets/README.md`).

The draft written for ratchet's survey changes as follows:

- K2: the one loop plan becomes two sampled measurements per benchmark, `bench-v8-v7` and
  `are-we-fast-yet`, with the suite geometric mean as a separate output.
- K3 and K4: new plans `start` (empty script µs, realm µs, sampled) and `memory` (peak RSS bytes,
  sampled; empty-realm bytes, exact).
- K5: `binary-size` reads the stripped `swell` shell, not the test262 runner.
- K6: four requirements: step overshoot `lte 0`, memory overshoot `lte 0`, recoverable `holds`,
  interrupt steps (exact).
- K7: a fuzz plan with `executed: {min: N}`. K8: a replay plan with `holds`.
- `peer-values` reads the generated `website/data/engines.json` (section 4), observe mode, no
  requirement, until ratchet `story:peer-values-beside-candidate` gives it a form.

What gates today. ratchet 0.0.1 runs nothing: every command answers "unavailable". Until its
`validate`, `assess` and `evaluate` exist, `slot task check` enforces what it can, and the quality
file is a declaration that no step reads:

| Requirement in the file | Enforced today by |
| --- | --- |
| test262 counts do not fall | `test262-check` (digest) and the lane `slow` (`AGENTS.md:261-273`); per-case identities: not held |
| host requirements | `host-requirements-check` |
| bounds hold | the tests that reach each bound (`AGENTS.md:120-124`) |
| no unsafe | `forbid-unsafe-check` |
| no third-party crates | `lock-check` |
| test steps 120 s, test 60 s | `gate-time`, nextest `slow-timeout` |
| K2 to K6 measurements, K5 size, editions and features, fuzz | nothing; waits for the measurements of section 4 and for ratchet |

Gaps held in ratchet, filed by its maintainers with swell as first consumer:

- (a) `story:peer-values-beside-candidate`: values of other implementations beside the candidate's,
  with version and method, no gate, no reference; covers the capability matrix.
- (b) `story:imported-report-revision-check`: an imported report states its revision, a report of
  another revision is refused (a stale `.out/test262/runs.tsv` would otherwise count as the
  candidate's evidence).
- (c) `story:render-numbers-into-documents`: `ratchet render`, numbers written into documents from
  records.

The assessor ids of the draft other than the two presets are invented and need declarations
(`test262-groups@1`, `swell-measure@1`, `file-size@1`, `forbid-unsafe-scan@1`,
`cargo-lock-packages@1`, `peer-table@1` and the new ones above).

## 3. The engine set

- Catalog: 35 engines from primary sources, gathered 2026-11-25, each field with its source URL or
  "unknown". It is committed as `bench/engines.jsonl`. By implementation language: C 11, C++ 9,
  Rust 4, Java 3, C# 2, Go 2, JavaScript 2, Zig 1, Nim 1.
- Default comparison set, the eight famous engines: V8, SpiderMonkey, JavaScriptCore, Hermes,
  QuickJS, QuickJS-ng, Boa, XS. Famous but left out: Duktape and JerryScript (ES5 level), Rhino and
  GraalJS (JVM), Elk and mJS (language subsets).
- The page's default selection: swell, V8 `--jitless`, QuickJS, Boa (the owner's order).
- A row is an engine in one configuration: `v8-jitless`, `v8` (JIT reference),
  `spidermonkey-interp`, `jsc-nojit`, and so on. Flags per configuration as `native-code.md`
  section 5 gives them; the SpiderMonkey and JavaScriptCore interpreter flags are UNVERIFIED and
  marked so in the data.
- An engine enters the measured set when:
  1. it has a CLI shell that runs a script file and prints (`print` or `console.log`);
  2. it installs without root on Linux x86_64: a prebuilt binary from jsvu
     (https://github.com/GoogleChromeLabs/jsvu) or esvu (https://github.com/devsnek/esvu), a vendor
     release, a javascript-zoo release (https://github.com/ivankra/javascript-zoo/releases), or a
     build from a pinned source revision inside the image;
  3. its version and sha256 are pinned in `bench/engines.jsonl`.
- All eight default engines meet 1 and 2 through jsvu, esvu or a vendor release (QuickJS original
  through esvu, QuickJS-ng through jsvu, Boa through its nightly release). An engine that does not
  can still be a row with published values only (section 4c).

## 4. Data pipeline

```
conformance/test262/expected.txt ─┐
website/data/status.json ─────────┤
bench/results/*.json (b) ─────────┼─ task site-data ─> website/data/engines.json ─> compare.tsx
bench/published/*.tsv (c) ────────┤        (sh, awk, jq)        held by site-data-check
bench/engines.jsonl ──────────────┘
```

- (a) swell's own values: from the gate's committed files (`expected.txt`, `status.json`) and,
  once the shell exists, from swell rows in the result files of (b).
- (b) our runs of other engines, and of the swell shell, in the image of `native-code.md` section 5
  on the build node through the pool:
  - one job per (engine, suite) or per (engine, test262 part), `slot --timeout 30m`; a job that does
    not fit is split, never given more time (the pool caps a job at 30 minutes);
  - the container runs with `--network none`, a pinned cpuset and a memory limit; downloads happen
    only when the image is built;
  - one process per measurement; test262 at swell's pin, with swell's case list
    (`swell-test262 meta`), every case for every engine;
  - the job's script lies in the worktree, never under a cache directory of the home: the pool runs
    such a script on the workstation, not on the build node (`AGENTS.md:290-296`);
  - one result file per run in `bench/results/`, as `native-code.md` section 5 gives it, plus for
    test262 the counts per directory in the line form of `expected.txt`, and the sha256 of the
    per-case file (the per-case file itself is not committed, Q4).
- (c) published values for engines or KPIs we do not run: javascript-zoo-data
  (https://github.com/ivankra/javascript-zoo-data, MIT). Its test262 files hold per-test results at
  test262 `4a1e96230008fe27367233f7e4947ace176aac5a` (revision date 2026-04-17), runner run
  2026-05-22; example, Boa: 51,464 passed, 1,703 failed
  (https://raw.githubusercontent.com/ivankra/javascript-zoo-data/master/test262/boa.json, head of
  file). A manual task `bench-import-zoo` fetches into `.cache/`, writes per engine and top
  directory one line to `bench/published/zoo-test262-4a1e962.tsv` with URL, data commit, sha256 of
  the fetched file and fetch date, and keeps the MIT notice in `bench/published/LICENSE-javascript-zoo-data`.
  - Not used for values: test262.fyi. Its pass rule fails a test only on "error", "exception" or
    "panic" in the output (https://github.com/test262-fyi/data/blob/main/runner/index.js); it shows
    QuickJS at 53,203 of 53,801 (https://data.test262.fyi/index.json, 2026-11-25) while
    javascript-zoo scores QuickJS 81.5 % (https://github.com/ivankra/javascript-zoo-data/blob/master/test262.md),
    and it runs on one Mac ("macOS 15.3.1 with M4", https://data.test262.fyi/meta.json).
  - Not copied: ahaoboy/js-engine-benchmark has no licence file; the page links it, nothing more.
- Generator: `task site-data` (`taskfiles/site.yml`) writes `engines.json` beside `status.json`,
  in POSIX sh, awk and jq (`AGENTS.md:181-182`). No Python. It compiles nothing and takes no slot.
- Gate: `site-data-check` regenerates both files into `.out/site-data/` and compares bytes. No
  network, no Node, no run of any engine. A number typed into `engines.json` fails it.
- Staleness: each result file records the test262 pin, the image digest and the swell commit it
  measured. `site-data` refuses a test262 result whose revision differs from `PIN` unless the file
  is marked `published` (it then shows as "other revision"). swell's sampled rows show the commit
  they were measured at; the release procedure re-measures swell (story 12).
- Refresh: the manual tasks `bench-run` (our runs, through the pool) and `bench-import-zoo`
  (network). Not the forge's lane `slow`: other hardware and network to vendor hosts (Q5).
- Forge path classes: `bench/results/*`, `bench/published/*` and `bench/engines.jsonl` get a `site`
  line in `.github/planning-paths.txt`, so a refresh runs `python-check` and `site-data-check` and
  not the full gate. The harness code under `bench/` stays `code`.

## 5. Fairness rules

1. Same test262 revision, same case list, same runner and same timeout per run for every engine.
   A cell that differs in any of these says which (for example "zoo runner, test262 4a1e962").
2. swell's comparable test262 value comes from the black-box runner on the swell shell, like every
   peer. Until it exists, the page shows swell's own runner's count, labelled "swell's runner".
3. Interpreter-only rows and JIT rows are apart. JIT rows are hidden by default and, when shown,
   are a band labelled "JIT reference".
4. Sampled numbers show median and min to max, with the number of runs. A `loaded` sample set is
   not the default value.
5. Every cell carries: source (our run, published, documentation), date, engine version and
   sha256 or revision, flags, host description, and for test262 the revision.
6. swell runs with its budgets on, since that is the product (`native-code.md` section 5); the
   limits used are in the record.
7. A suite revision, a benchmark's iteration count and a workload are pinned; numbers of two
   revisions are never in one row.

## 6. The page

- `website/src/pages/compare.tsx`, served at `/swell/compare`, with a navbar item "Compare" in
  `website/docusaurus.config.ts`. Data through `website/src/data/engines.ts`, which reads
  `@site/data/engines.json` and types it, as `status.ts` does for `status.json`. It computes no
  number the file does not hold.
- Picker: 2 to 4 rows of the data file. Default selection from the data file: swell, V8
  `--jitless`, QuickJS, Boa. The selection is in the URL query (`?e=swell,v8-jitless,quickjs,boa`),
  so a link opens on the exact comparison.
- Layout: one column per picked engine, rows grouped by KPI: Conformance (K1 total and per top
  directory), Speed (K2 per suite, benchmarks folded), Start (K3), Memory (K4), Size (K5), Resource
  control (K6 matrix), Safety (K7), Determinism (K8), Facts. Per row the direction (lower or higher
  is better) and the best of the picked cells marked.
- A missing value reads "not measured" with a reason from the data file ("no `$262.createRealm` in
  the shell", "swell cannot run this benchmark yet: SyntaxError ... slice 8", "no binary yet").
- Each cell links to its source: a committed result file at its commit and line on GitHub, a
  published file at its data commit, or the documentation URL from the catalog.
- No number is typed in the page source (the site's rule, `website/docusaurus.config.ts:14-16`).
  The landing page's section "Compared to others" (`website/src/pages/index.tsx:141-168`) links to
  `/compare` once a speed row exists, and its sentence "not measured" goes then.

## 7. The swell shell binary

- `crates/cli`, package `swell-cli`, binary `swell`, as the design plans it (`docs/design/engine.md:250`,
  `AGENTS.md:195`). A workspace member: `#![forbid(unsafe_code)]`, the standard library only, no
  third-party crate, so `forbid-unsafe-check` and `lock-check` hold it as they hold the engine.
- It is a host, not the engine: it reads the script file, writes standard output, gives the clock
  through the host hooks, and exits with a status per outcome. The engine keeps no ambient
  authority (`AGENTS.md:114`). It runs the agent on a thread whose stack is
  `Limits::min_stack_bytes()` (`crates/swell/src/host/limits.rs:61`), through
  `std::thread::Builder::stack_size`, which is safe code.
- Flags: one per field of `Limits` (seven, `limits.rs:12-27`); every run states its values, since
  the engine has no default.
- `--test262`: defines `print` and `$262` with the runner's host (`crates/test262/src/host.rs`), so
  the black-box runner can run swell like any peer. The CLI depends on the `swell-test262` library
  for it, so the host has one copy.
- Cost to settle in the same story: the test262 sources digest covers every `crates/*/src`
  (`AGENTS.md:261-264`), so a change of the CLI alone would force a full test262 run. The digest
  recipe (`taskfiles/test262.yml:464-533`) narrows to the packages the runner depends on, read from
  `Cargo.lock` (Q1).
- Release: from the release that holds it, the binary per target with a `SHA256SUMS` file, as
  `docs/releasing.md:163-168` plans.

## 8. Open questions

Each has a default, taken under "A default proceeds" unless noted.

- **Q1. Where the shell lives.** (a) `crates/cli` in the workspace, and the test262 digest narrowed
  to the runner's dependencies. (b) A separate workspace; it ships, so the third-party check must
  cover a second lock. (c) A second binary in `crates/test262`. Default: (a).
- **Q2. test262 runner for peers.** (a) A Rust black-box runner in the tooling workspace `bench/`,
  std only, at swell's pin, also run on the swell shell. (b) javascript-zoo's harness in the image;
  it is Python, refused for permanent tooling. (c) Published values only. Default: (a), with (c)
  for engines not run.
- **Q3. swell's headline K1 value on the page.** (a) Cases passed over all 53,801 with exclusions
  as failed (48.2 %), with the run-case rate (57.8 %) beside it, labelled. (b) The run-case rate
  only. Default: (a); (b) does not compare.
- **Q4. Per-case results of peers.** (a) Commit counts per directory and the sha256 of the per-case
  file. (b) Commit the per-case files, compressed. (c) Forge artifacts. Default: (a).
- **Q5. Refresh.** (a) Manual tasks through the pool, run at each swell release and each test262
  pin move. (b) The forge's lane `slow`. (c) A schedule. Default: (a).
- **Q6. Published source.** (a) javascript-zoo-data at test262 `4a1e962`. (b) A test262.fyi
  snapshot. Default: (a); (b) has the lenient pass rule.
- **Q7. JIT rows.** (a) Shown behind a toggle, off by default, labelled reference. (b) Never shown.
  Default: (a).
- **Q8. Resource control of peers.** (a) Capability matrix from documentation only. (b) An
  embedding harness per engine measuring overshoot; C and C++ engines need FFI, so `unsafe` or a
  third-party crate in a tooling workspace. Default: (a). (b) changes the rule of
  `forbid(unsafe_code)` for tooling and waits for the owner.
- **Q9. One data file or two.** (a) `engines.json` for the compare page and `performance.json` for
  the ratio view of `native-code.md`, both written by `site-data` from the same result files. (b)
  One file for both pages. Default: (a).

## 9. Proposed stories

Not created in the store. Ordered by the owner's first visible result: the comparison page with
conformance first.

1. `story:engine-catalog-committed`: `bench/engines.jsonl`, 35 engines with a source per field.
2. `story:site-data-writes-engines-json`: `site-data` writes `engines.json` with swell's K1 values
   (both readings, per top directory) and catalog facts; `site-data-check` holds it.
3. `story:import-zoo-test262`: `bench-import-zoo` writes `bench/published/zoo-test262-4a1e962.tsv`
   with URL, commit, sha256, date and the MIT notice.
4. `story:compare-page-conformance`: `compare.tsx` with picker, URL selection, conformance and
   facts rows, "not measured" reasons, a source link per cell, navbar item.
5. `story:compare-page-capability-matrix`: K6 and K8 capability rows from the catalog.
6. `story:quality-file-committed`: `quality/swell.ratchet.yaml` and `ratchet.toml` with the
   changes of section 2, and the table of what enforces each requirement today.
7. `story:cli-runs-a-script-file` (shared with `native-code.md`): `crates/cli` with limits flags,
   clock, exit status; test262 digest narrowed.
8. `story:cli-test262-host`: `swell --test262` with `print` and `$262` from the runner's host.
9. `story:bench-image-with-pinned-engines` (shared with `native-code.md`): one image, eight default
   engines and the swell shell, pinned by version and sha256.
10. `story:test262-black-box-runner`: the runner of Q2a, in parts as pool jobs; first run of the
    eight engines and swell at swell's pin, counts per directory committed.
11. `story:bench-first-run-bench-v8-and-awfy` (shared with `native-code.md`): K2 rows.
12. `story:start-memory-size-rows`: K3, K4 and K5 measured for swell and the eight; swell
    re-measured at each release (`docs/releasing.md` gains the step).
13. `story:swell-resource-control-measured`: K6 gates for swell: step and memory overshoot,
    recoverable after each limit, interrupt latency in steps.
14. `story:fuzz-with-fixed-corpus`: K7 fuzz target in a tooling workspace, fixed seed and count.
15. `story:landing-page-links-compare`: section "Compared to others" links to `/compare`.

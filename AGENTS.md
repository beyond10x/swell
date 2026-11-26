# swell: agent instructions

swell is an ECMAScript engine written from scratch in Rust. Its first user is surf, the browser in
the sibling repository `beyond10x/surf`; swell is implemented independently, with requirements
taken from surf's needs. The design is `docs/design/engine.md`; read it before changing anything.

Status on 2026-11-21: the gate, the pins, the counting runner, the first parts of `swell-num`
and `swell-unicode`, the parser for the ES5 grammar (`swell-syntax`), the bytecode compiler
(`swell-compiler`), and in `swell` the heap, the values, the object model, realms, the
interpreter and the builtins the test262 harness calls exist. **Slice 1 is delivered** (waves 1 to
6: `docs/plan/slice-01-wave-1.md` to `-6.md`): its 31 test262 cases pass in all 62 runs, no host
requirement is pending for it, and 18 of the 60 scenarios of the ESS model pass on the engine
(the other 42 need a later slice and say so). 148 of 215 host requirements have a test.
`Agent::evaluate_script` runs a script of the ES5 grammar under a step and a memory budget. The
gate holds the test262 counts: 24,838 of the 44,812 run cases pass (2026-11-24, release `0.9.0`; 20,229 in `0.8.0`, 18,155 in `0.7.0`, 17,775 in `0.6.0`, 16,412 in `0.5.0`, 13,575 in `0.4.0`, 12,120 in `0.3.0`). Release `0.1.0` was cut at the close of
wave 5 of slice 1, release `0.2.0` after the close of slice 2. **Slice 2 is delivered**, `language-core`, six stories in four waves (`docs/plan/waves.md`,
"Slice 2"; `docs/plan/slice-02-wave-1.md`: the grammar of slice 2, `Number`
and `Boolean`, one line per run from the runner; `docs/plan/slice-02-wave-2.md`: `with`, the
`arguments` object, names looked up at run time; `docs/plan/slice-02-wave-3.md`: `eval`, the
global `Function`, the two hooks of the host, `Agent::refusals()`; `docs/plan/slice-02-wave-4.md`:
the close, `Array.prototype.toString` early, 1,008 cases moved to later slices with evidence
the gate checks): all 4,259 cases of slice 2 pass in all 7,543 runs, and no host requirement
is pending for it. **Slice 3 is delivered**, `builtins-core`, thirteen stories in three waves
on 2026-11-22 (`docs/plan/slice-03-wave-1.md`: one file per group of builtins, the property
index and dense elements; `docs/plan/slice-03-wave-2.md`: the ten groups of `Object`,
`Function`, `Array`, `String`, `JSON`, `globalThis` and the URI functions;
`docs/plan/slice-03-wave-3.md`: the close, 624 more cases moved to later slices with evidence;
`docs/plan/slice-03-gate-and-site.md`: the gate's test budget, the forge rules, the
documentation site and the three gate runs): 7,010 of its 7,011 cases pass
(`test262-slice 03 7011 13829 0 0 7011 7010 1 0`), and no host requirement is pending for it. The one case that does not pass,
`test/built-ins/String/prop-desc.js`, fails in the pinned harness before the engine's answer
is judged; `docs/design/engine.md`, section 12, "Slice 3's cases that need a later slice",
states it, and every other document points there and carries the count
(`task:slice-03-harness-case-measured-at-the-next-pin`). **Slice 6 is delivered**,
`bindings-and-iteration` (`epic:slice-06-bindings-and-iteration`), fourteen feature stories and
the close on 2026-11-22 and 2026-11-23 (`docs/plan/second-batch.md`: template literals, `let`
and `const`, arrow functions, default and rest parameters, object destructuring;
`docs/plan/slice-06-wave-2.md`: `Symbol`, computed names with object spread and rest, the
iterator protocol, block functions, `for`-`of`, `Array.from`, array patterns and spread, tagged
templates, the close with 143 cases moved to later slices with evidence, and the order of
delivery in the block): all 3,898 cases of slice 6 pass in all 7,351 runs
(`test262-slice 06 3898 7351 0 0 3898 3898 0 0`), and no host requirement is pending for it.
The slices are not delivered in the order of their numbers: after slice 3 the order is
6, 5, 7, 10, 4, 9, by what the first consumer needs first (`docs/plan/waves.md`, "The order
after slice 3"), and it is decided again at each boundary. Slice 5, `regexp`
(`epic:slice-05-regexp`), was delivered on 2026-11-24: its first wave on 2026-11-23 (`docs/plan/slice-05-wave-1.md`)
brought the crate `swell-regexp` (the pattern parser, the compiler, the backtracking matcher
under the budgets), regular expression literals, `RegExp` with `exec` and `test`, `replace`,
`replaceAll` and `split` of `String`, and the case methods, `normalize` and `localeCompare`
for every String; its second wave (`docs/plan/slice-05-wave-2.md`) the methods keyed by `@@match`, `@@replace`, `@@search` and
`@@split` with `String.prototype.match` and `search` (release `0.7.0`); after it `matchAll` and
`RegExp.escape`, property escapes, the flag `i`, and identifiers and group names outside ASCII
(its third wave, `docs/plan/slice-05-wave-3.md`, release `0.8.0`); after that the flag `v`
with its class sets and properties of strings, and the close, with 18 cases moved to slices 4,
8 and 9 with evidence and 18 cases of the pattern grammar of Annex B.1.2 taken in from slice
18: all 2,724 cases of slice 5 pass in all 5,441 runs
(`test262-slice 05 2724 5441 0 0 2724 2724 0 0`), and no host requirement is pending for
it. Slice 7, `generators`
(`epic:slice-07-generators`), is delivered too (2026-11-24): generator syntax, generators on
suspended frames, `%GeneratorFunction%` and `yield*` are in release `0.8.0`; after it generator
methods of object literals, and the close, with 40 cases that need `class` or `super` moved to
slice 8 with evidence and 21 cases taken in from slices 18 and 19: all 1,472 cases of slice 7
pass in all 2,801 runs (`test262-slice 07 1472 2801 0 0 1472 1472 0 0`), and no host
requirement is pending for it. Slice 10, `promises-and-async`
(`epic:slice-10-promises-and-async`), is delivered too (2026-11-24), in seventeen stories
(`docs/plan/waves.md`, "Slice 10, `promises-and-async`"): the microtask queue and its
checkpoint, `Promise` with its statics, `async` functions and methods with `await` on suspended
frames, the rejection tracker, `%AsyncFunction%`, `for await` with the Async-from-Sync
Iterator, async generators with their request queue, their methods, their constructor and
`yield*`, `Array.fromAsync`, and the target of the executable specification for the checkpoint
and the hooks of a promise; and the close, with 141 cases that need a later slice moved to
slices 8, 9, 12 and 13 with evidence and 21 cases taken in from slices 18 and 19: all 3,759
cases of slice 10 pass in all 7,258 runs (`test262-slice 10 3759 7258 0 0 3759 3759 0 0`), and
no host requirement is pending for it. Slice 10 is in release `0.9.0`, with the closes of
slices 5 and 7 and the first six stories of slice 4 (`docs/plan/slice-10-wave-1.md`).
Slice 4, `numbers-and-dates` (`epic:slice-04-numbers-and-dates`), is accepted in seventeen
stories (`docs/plan/waves.md`, "Slice 4, `numbers-and-dates`"); six are built (in `swell-num`
the calendar arithmetic, the text of a date, the double-double arithmetic with `hypot`, the
exponential and logarithmic functions with `cbrt` and `exponentiate`, and the trigonometric
functions; and the `Date` object with the host's clock; the gate has the steps `libm-check` and
`math-data-check`, which reads three data files), 564 of its 1,612 cases
pass (`test262-slice 04 1612 3184 0 0 1612 564 1048 0`), and 12 host requirements are pending
for it.
The foundation is delivered in three waves (`docs/plan/wave-1.md`, `wave-2.md`, `wave-3.md`): the
workspace, `task check`, test262 and the Unicode data pinned by checksum, both specifications
of the host interface held by the gate, and `swell-test262`, which counts all 53,801 cases
per directory, per slice and per ECMA-262 section and compares its 287 lines
with `conformance/test262/expected.txt`. The numbers
are under "The gate's numbers".
The operator accepted the design on 2026-11-21. Its four open questions stand at the option the
design recommends, taken by the coordinating session and reported to the operator (`Intl` out
at first, no concurrent agents at first, Unicode 17.0.0); where surf runs the engine (the
fourth) is surf's decision and is not taken yet.

## Goals

From the operator, as surf's rules carry over: first principles, specification first, minimal,
and as secure as possible. "As soon as we mark it complete, it's complete as to the specs."

- The target is the ECMA-262 draft at a pinned commit, held by test262 at a pinned commit
  (design, decision 1). Both pins move together, in one change, never by themselves.
- Completeness is measured: test262 counts per directory (cases that exist, are excluded, run and
  pass) and a gate that holds them. A section of ECMA-262 is marked complete only when every
  test262 case for it passes and its list of deliberate omissions is empty.

## Rules

- Nothing third-party in anything that ships: the Rust standard library and the operating system
  only. Build and test tooling is not covered. The root `Cargo.lock` lists no package from outside
  the repository, and the gate checks it. A third-party crate is allowed only in a separate
  workspace of tooling that nothing shipped depends on (an ESS conformance runner, a fuzzer).
- `#![forbid(unsafe_code)]` in every crate. No just-in-time compiler, no machine code
  generation, no executable memory.
- No ambient authority: no file, network, clock, randomness or environment access in the engine.
  Time, time zones, randomness, module loading and jobs come from the host interface (decision 9).
  Do not use `std::collections::hash_map::RandomState` in the engine: it reads the operating
  system's random source. Do not use `std`'s transcendental `f64` functions (`sin`, `exp`, `ln`,
  `powf` and the others `std` documents as "non-deterministic" precision): `swell-num` has its
  own (decision 6).
- Hostile input must never hang, crash or exhaust the host. Every loop the engine runs on behalf
  of a script is charged to the step budget; every allocation is charged to the memory budget;
  every recursion is bounded. Each limit ends in an outcome the host reads (`Terminated(...)`, or a
  `RangeError` where the design says so), never in a panic or an abort. A new limit gets a test
  that reaches it.
- A buffer whose size a script controls and that is alive while the engine calls into script (a
  getter, `toString`, `valueOf`, a comparator, a replacer, a reviver, a job) is charged to
  `memory_bytes`, bytes and steps, before it is made and credited when it is dropped
  (`Accounting::hold`, `ChargedVec`, `string_units_in`), or it is not made: a String is read in
  its own storage. HOST-215 and the tests named `*_across_a_call` hold it; a unit that adds
  such a buffer states its measured charge in its report.
- A call in `crates/swell/src` outside test code that reserves room in a `Vec`, `VecDeque` or
  `String` (`with_capacity`, `reserve`, `reserve_exact`, `try_reserve`, `try_reserve_exact`)
  outside `ChargedVec` needs `#[expect(clippy::disallowed_methods, reason = "…")]` on its
  statement, whose reason names the charge that covers the buffer or the limit that bounds it;
  `crates/swell/clippy.toml` holds the list, and a change of one of its four `allow-*-in-tests`
  keys is made in the root `clippy.toml` too. Two checks hold the form. `crates/swell/Cargo.toml`
  denies `clippy::allow_attributes` (an outer `allow`) and
  `clippy::allow_attributes_without_reason` (an `allow` or `expect` without `reason`). The test
  `source_scan::growth_attributes_are_expects_with_a_reason` reads every `.rs` file of
  `crates/swell/src` and `crates/swell/tests`, `cfg_attr` at any depth and `macro_rules!`
  bodies included, and refuses what clippy lets through: an `allow` of
  `clippy::disallowed_methods` in any position, inner ones included; an `allow` or `expect` of
  a group that holds it (`clippy::style`, `clippy::all`, `warnings`); and an `expect` of it
  whose reason is empty or blank. The one `allow` it takes is the attribute of `src/lib.rs`
  for test code, once. `source_scan::no_other_crate_names_the_listed_lint` refuses an
  attribute that names the lint in another crate of the workspace, so a macro of another crate
  cannot carry one. The list guards the points of reservation only, not every growth
  (`docs/design/engine.md`, "Memory budget").
- No panics on any input: no `unwrap`, `expect` or unchecked indexing in engine crates.
- Spec first: ECMA-262 states the language and test262 holds it. The host interface is stated as
  an ESS model under `spec/` before its code (decision 10). Do not restate language semantics in
  ESS. Names follow the standard's terms; a builtin's module and comments cite its clause.
- The host interface has two specifications, and both come before its code. ESS states about
  half of it: the model (`spec/`, ess 0.97.4) validates and its 60 scenarios, 10 of them authored,
  pass on the interpreter, with 41 refusals and 53 `UNMAPPED:` markers
  (`docs/design/host-interface-model.md`). What ESS cannot state (engine values in outcomes,
  hooks that answer the engine, `call` and `construct`, host objects and roots, re-entrancy,
  the interrupt, budgets as bounds, determinism) is stated in `docs/design/host-interface.md`
  as numbered requirements: one per sentence an implementation can break, each `HOST-<n>` with
  its clause and its observable outcome. Each `UNMAPPED:` marker names the `HOST-<n>` that
  carries it. `conformance/host/requirements.txt` lists every `HOST-<n>` in one of three
  states: `test <crate>::<test name>` (held by a test), `pending slice <s>` (not implemented
  yet; slice `<s>` closes it) or `omitted <reason>` (a deliberate omission that stays). The
  gate fails on a requirement with no line, on a line for a requirement the document does not
  have, and on a requirement still pending for a slice that
  `conformance/slices-delivered.txt` names. A story that implements part of the host
  interface names the `HOST-<n>` it closes and moves their lines to `test`; every requirement
  of a slice is named by exactly one of its stories; a slice is complete only with no
  requirement pending for it, and the coordinator adds its number to the delivered file in
  the change that closes it.
- ESS is not extended for swell, and the model is not grown to chase coverage. The operator,
  2026-11-21: "there is no need in super specifically extending ESS now. We could just generate
  our own sort of specs language for the ESS, for the ECMA runtime, which then maybe lowers to
  ESS, I don't know. [...] there is no point in listing now a million gaps in ESS and then
  implementing something super specific just because that ECMA script engine would need it."
  So: the note of what ess 0.96.0 could not state is the record of one measurement; it is not
  extended, its lines are not requests against ESS, no limit of ESS is filed, and no agent
  works around a refusal to get one more construct expressed. A defect of ESS (the tool
  contradicts its own documentation or writes output that fails) is filed by the session that
  decides for the operator, as the bot; this session files nothing. The ESS model stays as the checked statement
  of what it states well (the lifecycles of agent, realm, script, module and job). A construct
  that costs more than a requirement line and a test goes into the `HOST-<n>` requirements,
  which are swell's own specification form: identifier, source clause, observable outcome, the
  test that holds it, and nothing the gate cannot read. No language or compiler is designed for
  them; whether they later lower to ESS is open. test262 cases are never restated in another
  form.
- The host interface is written by hand against those requirements. Nothing is generated from
  the ESS model, and no type of the interface is changed to make generation pass. The model's
  scenarios run against the hand-written engine through a conformance target in a workspace
  of its own, as surf's `conformance/` does. That workspace is tooling that nothing shipped
  depends on: it has its own `Cargo.lock`, and it may take the `ess-conformance` crates by
  `git = "https://github.com/beyond10x/ess"` with a pinned tag. A read of a public dependency
  by cargo for such a workspace is allowed (ruled on 2026-11-21 by the session that decides
  for the operator). The coordinating session and its units make no call to a forge: no push,
  no remote, nothing created or written there ("Publishing").
- A deliberate omission is recorded where the gate reads it (`conformance/test262/features.txt`
  for a test262 feature, the section's omission list for a requirement), never left implicit. A
  test262 feature the list does not name fails the gate.
- No Python anywhere: no Python file in the repository, no Python in a gate, test, script or
  manual check, and none in a command suggested to a person. Shell, `awk`, `jq` and Rust are fine.
- No work files in `/tmp`. Scratch output goes to `.out/`, fetched test data to `.cache/`; git
  ignores both.
- The repository will be public: nothing personal in the tree. No person's name (write "the
  operator"), no home directory path (write `$HOME`), no machine name.
- Every build and test runs through the shared build pool: `slot task <name>`.

## Layout (planned, design decision 2 and 14)

- `docs/design/engine.md`: the design. Its block `swell-slices` assigns test262 cases to delivery
  slices; its last section gives the commands that check the counts.
- `crates/unicode` (`swell-unicode`), `crates/num` (`swell-num`), `crates/regexp`
  (`swell-regexp`), `crates/syntax` (`swell-syntax`), `crates/compiler` (`swell-compiler`),
  `crates/swell` (`swell`, the engine and its host interface), `crates/cli` (`swell-cli`, the
  binary `swell`), `crates/test262` (`swell-test262`, the conformance runner),
  `crates/unicode-gen` (`swell-unicode-gen`, a tool).
- `data/unicode/<version>/`: vendored Unicode data files with a `SOURCE` file of addresses and
  SHA-256 sums. The generated tables are committed and checked against a regeneration.
- `conformance/test262/`: `features.txt` (every feature, `run` or `exclude`), `sections.txt`
  (ECMA-262 sections, their test262 paths and omissions), `expected.txt` (the gate's counts).
- `website/`: the documentation site (`docs/` its pages, `src/` its landing page and
  components, `data/status.json` its numbers, written by `task site-data`). `docs/img/`: the
  logo's lockup for `README.md`.
- test262 is never vendored: it is fetched by commit into `.cache/test262/<commit>/` and checked
  against its tree hash and a SHA-256 of `git ls-tree -r` (decision 11).

## Conventions

- `AGENTS.md` is for agents, `README.md` for people, `CLAUDE.md` only points to `AGENTS.md`.
- `Taskfile.yml` with `task check` as the one gate, once there is code.
- Licence Apache-2.0 with `LICENSE` and `NOTICE`, as surf has them; every crate declares it.
- A commit has a body and no attribution line. Commit only when the operator asks, or as a
  commit of an approved plan or of a wave (below).

## Commands and the gate

- `slot task check` is the one gate. `task --list` names every task; each task's `summary`
  says what it reads and what makes it fail.
- Before `main` moves, the gate is run on the wave branch once as the forge's runner runs it:
  `slot --timeout 30m task check-runner` (`taskfiles/forge.yml`). It sets the runner's
  variables (`GATE_RUNNER_ENV`: `CI`, `GITHUB_ACTIONS`, `CARGO_TERM_COLOR`, `CARGO_INCREMENTAL=0`)
  and keeps the host's compile cache: `RUSTC_WRAPPER` is never set empty. `forge-check`, a
  step of `check`, fails when rustc is not the channel `rust-toolchain.toml` pins (1.99.1), when
  a workflow names a compiler, when the toolchain step of `.github/workflows/check.yml` lacks a
  component or target that `rust-toolchain.toml` names (`clippy`, `llvm-tools`;
  `x86_64-unknown-none`) or reads no value from that file's `components` or `targets` key, when
  `GATE_RUNNER_ENV` differs from the variables of `.github/workflows/check.yml`, or when
  `.github/workflows/release-build.yml` is not the release's seven targets (`RELEASE_TARGETS`
  of `taskfiles/forge.yml`) built and checked with the C runtime linked in on musl and Windows,
  or names or counts other archives than `docs/releasing.md` and `website/docs/embedding.md`
  do (`release-targets-check`). A forge run that goes red after a green local gate is fixed
  together with the change that makes the local gate catch it.
- Tests run under cargo-nextest (`.config/nextest.toml`): a test is terminated at 60 s and
  counts as failed, and each test runs in a process of its own, so a `static` shares nothing
  between tests. The gate prints `gate-time <step> <seconds>` after each test step and
  `gate-time test-steps <sum> of 120, cpu <s>, load <l> on <n> cores`; it fails when the sum
  of wall time is over 120 s. When that line is the only red line and its load is above its
  core count, see "How the work is run": the forge's run judges.
- After that line the test steps print (`taskfiles/gate-time.yml`,
  `story:gate-test-steps-fit-four-cores`): `gate-time yardstick <s> s: conform-engine <a> and
  test262-moves-check <b>`, which decides nothing; one of `gate-cpu test-steps <c> of <bound>
  on <n> cores: <step> <c>, ...`, `gate-cpu test-steps <c>, no bound for <n> cores: ...`,
  without a table `gate-cpu test-steps <c>: ...`, or, when the CPU seconds cannot be read on
  a core count without an entry, `gate-cpu test-steps: not read (...)`; and `gate-time: no
  file of <k> under <dir>/*/deps changed while the test steps ran` (`<k>` counts both
  directories).
- `check-runner` passes `GATE_CPU_BOUNDS`, the table `CPU_BOUNDS_BY_CORES` of
  `taskfiles/forge.yml`: pairs `<cores>:<CPU seconds>`, now `16:245` (provisional: its pair
  under this tree is recorded by the first two check-runner jobs on a 16-core machine after
  the merge, `story:gate-cpu-table-held-by-a-check`). The gate fails when the test steps' CPU
  seconds are over the entry for the core count it runs on. A core count without an entry
  gets no CPU bound; there the CPU seconds are printed, and only the wall budget and the
  `deps` check decide. A green there says nothing of the forge's time. The pool's build node
  (64 cores) and the forge (4 cores) have no entry; on the forge the wall time decides.
- An entry of `CPU_BOUNDS_BY_CORES` is added or changed only together with a pair measured
  on that kind of machine, recorded in the same commit. The pair is the code before the
  change over the entry and the change under it, each a whole check-runner job. The entry is
  the geometric mean of the highest change and the lowest base, rounded to 5. The highest
  change × 1.25 must be at most the entry, and the entry × 1.25 at most the lowest base. Two
  entries for one core count are refused (`gate-cpu: GATE_CPU_BOUNDS holds <n> cores twice`).
- The gate fails when a test step writes into `<dir>/*/deps` (`CARGO_TARGET_DIR` or `target`)
  or `conformance/ess/target/*/deps` inside the clock. Another job of the same tree that
  builds meanwhile turns that red, so the gate runs one job per tree at a time. A red
  `gate-cpu` or `deps` line holds the commit; only a red `gate-time test-steps` line under
  load goes to the forge.
- The gate does not run test262 in full. `test262-check` compares a digest of the run's
  sources (`src/` and the manifest of each crate the runner depends on in `Cargo.lock`, the
  lock's entries of those packages, the workspace manifest, `.cargo/config.toml`, the
  toolchain file, `taskfiles/test262.yml`, the files of `conformance/test262/` the run reads)
  with `conformance/test262/expected.digest`; a crate the runner does not depend on
  (`crates/cli`, `crates/unicode-gen`) is not covered. A change of a source runs the full run
  (`slot --timeout 30m task test262-expected`), which writes `expected.txt` and the digest; where
  the run does not fit one pool job, it is made in parts, each a job of its own
  (`slot --timeout 30m task test262-part TEST262_PART=<k>/<n>` for each k), and counted from
  their files (`slot task test262-expected TEST262_FROM_PARTS=.out/test262/parts`);
  the lane `slow` (`task slow`, `.github/workflows/slow.yml`) makes the full run on the forge
  and compares bytes. A release waits for a green lane `slow` on its commit. The lane starts
  on a push to `main` and on a pull request, each only when it changes a path of the
  workflow's list, and by hand (`workflow_dispatch`) for a release tip whose push changed
  none (`docs/releasing.md`, step 5).
- A heavy test has a gate count and a full count: `SWELL_FULL_COUNTS=1` gives the full one,
  and the lane `slow` (task `slow-tests`) runs every `crates/*/tests/*.rs` that reads the
  variable at its full count in both collection modes.
- On the forge a change is classified by `taskfiles/classify-changes.sh` from
  `.github/planning-paths.txt` (lines `prose`, `site`, `code`; a path no line matches is
  code): `planning-only` runs `python-check` alone, `site-only` runs `python-check` and
  `site-data-check`, `code` runs `task check`. The ruleset of the repository is the
  operator's; a change of the path list is code.
- `website/data/status.json` is written by `task site-data` from the gate's committed files;
  `site-data-check`, a step of `check`, fails when it differs from its regeneration. A commit
  that changes `expected.txt`, `slices-delivered.txt`, `requirements.txt`, `moves.tsv`, a
  crate version or section 12 of the design regenerates it (`task site-data`, which runs awk over the committed files and compiles nothing, so it takes no slot of the pool).
- A test stays under 1.5 CPU seconds alone in the gate's default mode and under 2 s with a
  collection at every safe point, and under 2.5 s at its full count (`SWELL_FULL_COUNTS=1`)
  in either mode, measured with its test binary run with `--exact <name> --test-threads 1`
  in a shell of its own whose `times` gives user plus system. The measurement is one job of
  the build pool (`slot sh -c '...'`: `cargo test --locked -p <crate> --test main --no-run`
  prints the binary's path, and the job runs it); a test binary is not run by its path
  beside the pool, and no second job of the same tree runs while one builds, since the two
  share one build directory. The script of a measure job is given inline or lies in the
  worktree (its ignored `.out/`), never under a cache directory of the home: the pool runs a
  job whose script lies there on this machine, not on its build node, and refuses that form
  from 2026-11-24. The bound is in CPU seconds
  because the machine's load multiplies wall time: on 2026-11-22, at a load of 110 to 166 on
  18 cores, tests of 4 to 18 CPU seconds were killed at 60 s
  (`story:heavy-tests-far-from-the-kill`), and at a bound of 5 s for the full count
  `builtins_iterators::each_next_is_charged`, 4.2 CPU seconds, was still killed while the
  load rose from 37 to 165 (`story:lane-tests-under-2-5-cpu-seconds`). Two tests are over
  the bound and stay so: `determinism::host_133_same_calls_same_hooks_same_outcomes` (4.2 to
  4.4 s in collect mode, gate and lane, three runs of its corpus) and `language` part 3 (2.2
  to 2.5 s in the collect gate). The full count of the parser's stack-bound check is 7,500
  levels on three quarters of the stack of `parser_nesting` 10,000, and the gate count 2,500
  on a quarter; the derivation is in `crates/syntax/tests/sources.rs`.
- A new language script stays under 0.5 CPU seconds per run, and a new test under 2 s alone,
  with a collection at every safe point. Measured with
  `SWELL_SCRIPT_TIMES=1 cargo nextest run --locked -p swell --features
  collect-every-safe-point --test main --no-capture -E 'test(/^language::/)'` (one line
  `script-time <wall> <cpu> <script> <mode>` per run) or with `--test-threads 1 -E
  'test(=<module>::<test>)'`. Heavier work gets a gate count, with its full count under
  `SWELL_FULL_COUNTS=1` in the lane `slow`. In that mode every instruction is a safe point
  that collects, so a loop costs about a hundred times what it costs in the default mode;
  at a load of 112 on 20 cores a test of 2.4 CPU seconds took 58.5 s
  (`story:collect-mode-tests-far-from-the-kill`).
- The gate's test steps cost about 270 CPU seconds (`story:gate-test-steps-cost-half`,
  2026-11-23, the sum of each test run alone: 86.0 in the default mode over 973 tests, 138.7
  with a collection at every safe point over 796, 36.9 for the 65 planted scripts; 94.0,
  154.5 and 43.0 before that story). The collect mode is spread over hundreds of tests whose
  scripts are what they show, so a gate count does not lower it further. Since
  `story:gate-test-steps-fit-four-cores` `test-collect` runs in the profile `collect`
  (opt-level 1) through `--config .cargo/collect-profile.toml`: its build is before the
  clock, and on the pool's 64-core node the step went from 168.5 to 33.3 and 34.0 CPU
  seconds. That flag is `COLLECT_CONFIG` in `taskfiles/collect-mode.yml`, used by
  test-collect, build-collect and test-doc's collect line; build-collect checks the file.
  The lane `slow` runs the collect-mode stack modules (those calling `stack_with_room`) in
  the test profile at opt-level 0. `gate-selftest` runs the planted
  scripts as a rolling pool (`xargs -P`), with `GOGC=off` and `GOMEMLIMIT=1GiB` for its
  many short go-task processes.
- Each crate has one integration-test binary, `tests/main.rs`, generated by
  `sh taskfiles/test-main.sh write` (task `test-main`) and never edited by hand; a manifest
  has `autotests = false` and one `[[test]]` named `main`, and `test-binaries-check` runs the
  layout check first (`sh taskfiles/test-main.sh check`, no build) and fails on a second test
  binary. A unit that adds a test file writes `crates/<crate>/tests/<stem>.rs` with
  `use crate::support;`, runs the generator and commits `tests/main.rs` with it; on a merge
  conflict in `tests/main.rs` the generator is run again. A test is named by
  `--test main -E 'test(/^<stem>::/)'`; a test that starts its own binary again passes
  `test_path!("<name>")`; the test list keeps `<package>::<module>::<test>`.
- The repository's `.cargo/config.toml` builds the dev and test profiles without debug
  information (`debug = false`, `strip = "debuginfo"`), on the forge and here alike;
  `test-binaries-check` shows it with `readelf -S` on every test executable. A stack bound is
  measured in the profile `stack-debug` (dev with full debug information, the largest frames,
  an embedder's default). A build with line tables only had frames about 5 % smaller, and a
  stack test passed here and aborted on the forge twice (wave 3 of slice 1, wave 1 of slice 2;
  measured in the second: 29,855 against 31,314 bytes per re-entry, and no difference from
  incremental compilation): do not override that setting. With `CI` and
  `GITHUB_ACTIONS` set go-task colours its failure line and adds an `::error` line (14 planted
  scripts failed there); a planted script clears those variables in its first lines, as the
  others do. A test of a stack bound runs its subject in a child process, and a test that
  says `min_stack_bytes()` is enough runs it on seven eighths of that
  (`tests/support/host.rs`, `stack_with_room`), so that a frame that grows fails here first.
- The run loop, `Agent::execute` (`crates/swell/src/interp/dispatch.rs`), holds no instruction's
  locals: an instruction that needs locals does its work in a function of its own, and the run
  loop calls it. Every re-entry of the engine pays that frame (336 bytes; 19,280 before,
  `docs/plan/slice-02-run-loop-frame.md`). `STACK_BYTES_PER_REENTRY` is 14 KiB: the measured
  need on the worst path (11,100 bytes, through `toJSON`) plus a fifth. When a stack test fails,
  measure the bytes per re-entry again as that record does; do not raise the constant unmeasured.
  If a stack test aborts on the forge and passes here (a runner image with another compiler),
  send the deciding session the log lines and do not change the constant before it answers.
- The gate builds before its clock: `build`, then `build-tests`, which builds every binary the
  timed test steps run with each step's own flags, features and manifest (`build-collect`,
  `build-faults`, `build-conform-engine`) and ends with `build-test262-runner`, so no release
  build happens inside the clock. A test step whose log has a `Compiling` line fails
  and names the build task that should have built it (`compile-check`); only the doc tests
  compile inside `test-doc`, because rustdoc compiles them as it runs them. The test report
  (`nextest-report`) reads a coloured log, and a log with nothing to count fails.
- A unit runs the tests of the packages it touches, and a unit that changes what a script
  evaluates to also runs the tests of `swell-test262` (`docs/plan/unit-invariants.md`). No test
  takes more than 15 s in the gate at this machine's load: a test whose cost is its count has
  a gate count and its full count under `SWELL_FULL_COUNTS=1`.
- `test-collect` runs every test of `crates/swell` with the feature
  `collect-every-safe-point`: a value held across a safe point without a root fails there.
- A gate task lives in an include file under `taskfiles/` that the root `Taskfile.yml`
  includes; the coordinator adds its `- task:` line to `check`. A planted-defect test is a
  script `tests/gate/<name>.sh` that prints `<name>: ok` and exits 0 when the defect is
  refused; `gate-selftest` runs all of them. A script works only under `.out/gate-tests/`.
- Task facts, measured in wave 1 (Task 3.54):
  - Task takes a variable from the environment when none is passed on the command line. A
    gate variable therefore carries a prefix (`GATE_`, `TEST262_`, `UNICODE_DATA_`, `SPEC_`,
    `HOST_REQ_`, `HOST_SURFACE_`, `ESS_`, `SITE_DATA_`, `LIBM_`, `MATH_DATA_`, `NOSTD_`) and is listed in `GATE_VARIABLES` of the task `gate-vars`, which prints the
    ones that have a value as the gate's first line.
  - A variable is written as `'{{.X | default "..."}}'`: in an included file a plain value
    ignores the command line.
  - A command script stops at the first failing command, and a `grep` that matches nothing
    fails: write `|| true` where no match is a valid answer.
  - A check that finds nothing to read fails. A check that passes on an empty input is a bug.
- A bare `task` or `cargo` in an agent shell is refused or queued by the build pool's hook;
  inside a pool job (`slot task check`) the scripts call them directly.

## The gate's numbers

`slot --timeout 30m task check-runner` on 2026-11-24, on `5ee604a`, the release commit of
`0.9.0` (the branch of wave 1 of slice 10; `docs/plan/slice-10-wave-1.md`), on the pool's
build node, load 36 on 64 cores. Every check passed and the run exited 0: the test steps took
24.9 s of their 120 (166.7 CPU seconds, of them `test-collect` 44.4; 64 cores have no entry in
the table of CPU seconds, so the `gate-cpu` line prints no bound). On the code of release
`0.8.0` with the gate change of `story:gate-test-steps-fit-four-cores` (`756b8be`) the same
node took 34.3 s at load 67 (201.5 CPU seconds); before that change, on the code of `0.8.0`, a
16-core machine took 94.3 s at load 34 (312.1 CPU seconds) and the forge's runner of 4 cores
127.9 s (448.5 CPU seconds). The planted scripts' single
`ok` lines, the ten slowest tests of each step and the slice lines after slice 7 but that of
slice 10 are left out (`conformance/test262/expected.txt` has every slice line). The fault
step's line counts the 964 tests its filter leaves out as ignored:

```
python-check: no Python file among 2252 file(s)
forbid-unsafe-check: 9 crate root(s), each with #![forbid(unsafe_code)]
libm-check: no forbidden call under 198 file(s)
Cargo.lock: 8 package(s), none from outside this repository
toolchain-check: rustc 1.99.1, the channel rust-toolchain.toml pins; 5 workflow file(s), none names a compiler
nextest-check: ./.config/nextest.toml: every test is terminated by 60 s at the latest; 19 task file(s): 4 step(s) through cargo-nextest, no cargo test but doc tests
test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked
data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present
math-data-check: 3 file(s)
unicode-tables: 16 table(s) equal the generator output
swell v1 — 2 file(s), 10 scenario(s), valid
60 scenario(s) (10 authored), 41 refusal(s), written to .out/suite.json
  60 scenarios: 60 passed, 0 failed, 0 error, 0 unsupported
host-requirements: 215 requirement(s), 215 line(s): 148 test, 67 pending, 0 omitted; 53 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 4:12 8:1 9:6 11:32 12:2 13:2 14:6 16:5 18:1
host-surface: 0 change(s) since 0.9.0, each named
site-data-check: website/data/status.json equals its regeneration from conformance/test262/expected.txt, conformance/slices-delivered.txt, conformance/host/requirements.txt, conformance/test262/moves.tsv, docs/design/engine.md, Cargo.toml, Cargo.lock and the crate roots under crates
test-main: 8 crate(s) with tests, each with one integration-test binary and its tests/main.rs as generated
test-binaries-check: 17 test executables, 8 integration-test binaries in 8 packages, none with a .debug_ section
build-tests: Finished `test` profile [unoptimized] target(s) in 0.01s
build-collect: Finished `collect` profile [optimized] target(s) in 4m 01s
build-collect: 4 test executables in target/collect/deps, none with a .debug_ section
build-faults: Finished `test` profile [unoptimized] target(s) in 16.84s
build-conform-engine: Finished `test` profile [unoptimized] target(s) in 8.67s
build-conform-engine: Finished `dev` profile [unoptimized] target(s) in 0.04s
build-test262-runner: Finished `release` profile [optimized] target(s) in 0.01s
17 test binaries: 1536 passed, 0 failed, 1 ignored
gate-time test 6.4
test-doc: 8 test suites: 19 passed, 0 failed, 0 ignored
test-doc collect-every-safe-point: 2 test suites: 19 passed, 0 failed, 0 ignored
gate-time test-doc 3.9
test-collect: 2 test binaries: 1167 passed, 0 failed, 1 ignored
gate-time test-collect 3.2
test-faults: 1 test binaries: 1 passed, 0 failed, 964 ignored
gate-time test-faults 0.3
conform-engine: 3 test binaries: 16 passed, 0 failed, 0 ignored
conform-engine: planted defect refused: test a_target_whose_interrupt_does_nothing_fails_a_scenario ... ok
conform-engine: planted defect refused: test a_target_whose_checkpoint_runs_no_job_fails_a_scenario ... ok
  60 scenarios: 18 passed, 0 failed, 0 error, 42 unsupported
gate-time conform-engine 1.4
test262-refused 23324
test262 total 53801 8982 7 44812 24838 19972 2
test262: 221 directory lines sum to the total
test262-slice 01 31 62 0 0 31 31 0 0
test262-slice 02 4259 7543 0 0 4259 4259 0 0
test262-slice 03 7011 13829 0 0 7011 7010 1 0
test262-slice 04 1612 3184 0 0 1612 564 1048 0
test262-slice 05 2724 5441 0 0 2724 2724 0 0
test262-slice 06 3898 7351 0 0 3898 3898 0 0
test262-slice 07 1472 2801 0 0 1472 1472 0 0
test262-slice 10 3759 7258 0 0 3759 3759 0 0
test262: 22 slice lines equal the design
test262-errors 0
test262: 287 lines of conformance/test262/expected.txt; sources digest 385593bd7e603552a8c439160415e1b0a235584458ee4124a3eb95d02f58dacb equals conformance/test262/expected.digest
gate-time test262-check 0.1
test262-moves: 1963 case(s) moved, 3:78 4:339 5:278 6:694 7:60 8:341 9:28 10:83 11:4 12:41 13:17; slices 1 to 19: 44931 85794
gate-time test262-moves-check 1.8
gate-selftest: 66 passed, 0 failed
gate-time gate-selftest 7.8
gate-time test-steps 24.9 of 120, cpu 166.7, load 35.51 on 64 cores
gate-time yardstick 3.2 s: conform-engine 1.4 and test262-moves-check 1.8
gate-cpu test-steps 166.7, no bound for 64 cores: test 84.9, test-doc 4.7, test-collect 44.4, test-faults 0.2, conform-engine 2.0, test262-check 0.1, test262-moves-check 1.8, gate-selftest 28.6
gate-time: no file of 624 under target/*/deps changed while the test steps ran
```

The refusal ceiling is `SPEC_REFUSALS_MAX` in `taskfiles/spec.yml` (41); a change to `spec/`
that moves the count moves the number in the same change. The total line reads: cases that
exist, excluded, not applicable, run, passed, failed, ended by a budget. A slice line
(`test262-slice 01 31 62 0 0 31 31 0 0`) reads: slice, cases, runs, excluded, not applicable,
run, passed, failed, ended by a budget. A refusal of the parser ("... is not supported before
slice <n>") and the nesting limit never count as the `SyntaxError` a negative case asks for;
`docs/plan/slice-01-wave-5.md` says what else the pass number does and does not say.

## Publishing

Decided on 2026-11-21: the repository becomes the public repository `beyond10x/swell`. The
operator's answer, given in the session that decides for him and relayed by it to the
coordinating session (not heard in the coordinating session itself): "A. Yes, both" to
"Publish `swell` as public repository `beyond10x/swell` (enrolled in the gates, history
replaced by one bot-made first commit, as surf)". Published on 2026-11-21 at the close of wave 2
of slice 1: https://github.com/beyond10x/swell, `main` starts at `8945d64`, made by the bot.

- **Who does what.** The session that decides for the operator does the whole forge side: the
  bot-made first commit of `main`'s tree, the policy entry, audit, `check`, `verify`,
  `publish`. The coordinating session and its units make no call to a forge, add no remote
  and push nothing. At each wave boundary the coordinator sends that session `main`'s commit
  and the gate's numbers and waits for its answer before the next wave opens.
- **A pull request is opened for its checks, not to be merged on the forge.** `main`'s rules
  restrict updates and require the bot as committer, and a merge made on the forge has the
  forge's committer, so the forge refuses it ("the base branch policy prohibits the merge").
  The commit lands by the bot's push of `main` after the pull request's checks are green, and
  the pull request then shows as merged (first one: pull request 1 on 2026-11-23, `16e0bc2`,
  a change of `AGENTS.md` alone, which passed `check` on the path for documents without a
  build or a test).
- **History.** `main` starts at one first commit made by the bot (`8945d64`), with the tree of the
  last local commit before it (`31b9f07`). The
  history before it stays in the local repository as the branch `archive/pre-public` and is
  never pushed; nothing is deleted. Commit ids in the wave records up to that point name
  commits of that branch. Branches start from `main`; a branch of the old history is never
  merged into it. The remote is `origin`; the gates' hooks are installed in the repository.
  The policy holds one exception per scenario file under `spec/scenarios/` that carries the test
  value of `hash_key` (ten after wave 6 of slice 1), bound to the SHA-256 of the whole file:
  a new file of that kind, and any edit of one of them, is refused at the bot commit until
  the session that decides for the operator admits the new content. Send it the paths and
  checksums of the staged files; do not change a staged byte afterwards. A line the previous
  version of a file already carried is no new finding, so the refusal counts the new files.
- **Every commit after the first publication is the organization bot's**, also a merge; a
  plain `git commit` is not used, because the gates refuse a pushed commit with another author
  or committer:

      b10x-gates --repo <tree> --repository beyond10x/swell --policy ../gates-policy/policy.json \
        bot -- commit -F <file>
      b10x-gates ... bot -- merge --no-ff <branch> -F <file>

  A commit has a body and no attribution line.
- **Nothing personal goes into the tree**: no person's name (write "the operator"), no home
  directory path (write `$HOME`), no machine name, no e-mail address. The gates scan for
  secrets, private identifiers, home paths, unpinned workflow actions and workflows without
  `permissions`, and refuse without printing the match. A scan of `main` at the commit of
  slice 1, wave 1 found none (the deciding session's scan, 2026-11-21).
- **`main` is published only after `task check` is green on it.** The workflows are
  `.github/workflows/check.yml` (runs `task check` on a push to `main` and on a pull request;
  it installs the Rust toolchain, go-task and `ess` by pinned release assets, and the gate
  fetches test262 by its pin) and `shared-gates.yml` (the organization's shared source gates,
  as surf has it). No release workflow while a release holds no binary.
- **The documentation site** is `website/` (Docusaurus on the organization's docs-system),
  to be served at https://beyond10x.github.io/swell/. It types no number: `task site-data`
  writes `website/data/status.json` from the committed files of the gate, and
  `task site-data-check` fails when that file differs from what they give, so a change of
  `conformance/test262/expected.txt`, `conformance/slices-delivered.txt`,
  `conformance/host/requirements.txt`, `conformance/test262/moves.tsv`, the design's slice
  table, the workspace version or `Cargo.lock` is committed together with a new
  `status.json`. The logo is `website/static/img/logo.svg` (and `logo-dark.svg`), the lockup
  for the head of `README.md` is `docs/img/swell-logo.svg` (and `-dark.svg`): hand-written
  SVG. The two workflows that build and publish the site, and the Pages setting, are added
  by the session that decides for the operator; until its first run has ended with success
  the address above serves nothing.
- **A release** is an annotated tag by the bot, named as the workspace version, on a green
  commit of `main`, and a GitHub Release whose notes are a short summary (a lead sentence,
  `**Breaking**`, `**Changed**` with at most five bullets, at most 900 characters) that ends
  with a link to the version's section of `CHANGELOG.md`; the tag's message is the same text:
  `docs/releasing.md`. The coordinator prepares version, changelog, gate and notes on a
  branch and sends the tip to the session that decides for the operator; that session tags
  and releases. Releases: `0.1.0`, 2026-11-21, tagged on `6319af2`, the close of wave 5
  of slice 1; `0.2.0`, 2026-11-21, tagged on `29f674c`, the commit after the close of slice 2
  (https://github.com/beyond10x/swell/releases/tag/0.2.0); `0.3.0`, 2026-11-22, tagged on
  `1f21c0a`, `main` with slice 3 and without the open wave, on the operator's order to
  release every green default branch
  (https://github.com/beyond10x/swell/releases/tag/0.3.0); `0.4.0`, 2026-11-22, tagged on
  `20797bf`, `main` with the resource accounting, the host interface of the second batch and
  the first five stories of slice 6, on the operator's pick "cut 0.4.0 now"
  (https://github.com/beyond10x/swell/releases/tag/0.4.0); `0.5.0`, 2026-11-23, tagged on
  `635dd5b`, slice 6 delivered, on the operator's pick "cut 0.5.0 now"
  (https://github.com/beyond10x/swell/releases/tag/0.5.0); `0.6.0`, 2026-11-23, tagged on
  `6e9c3f4`, the first wave of slice 5, on the decision "cut 0.6.0 together with the wave" of
  the session that decides for the operator
  (https://github.com/beyond10x/swell/releases/tag/0.6.0); `0.7.0`, 2026-11-23, tagged on
  `e0bb1ec`, the second wave of slice 5, with which the regular expressions of the first
  consumer's harness answer, on the decision "cut 0.7.0 now" of the session that decides for
  the operator (https://github.com/beyond10x/swell/releases/tag/0.7.0); `0.8.0`, 2026-11-24,
  tagged on a commit of `main` whose `check` and lane `slow` are green on the forge, the
  code of the release commit `a350de3` (2026-11-23, the day its counts were measured) with
  the gate change of `story:gate-test-steps-fit-four-cores` after it (on `d9d1b95` the
  forge's test steps took 127.9 s of their 120), the heading of its changelog section
  set to the day of the tag and the manual trigger of the lane `slow`
  (`story:slow-lane-starts-by-hand`), whose push starts the lane on the tagged commit: the
  third wave of slice 5 with the first four stories of slice 7, on the decision "cut 0.8.0
  when the two running slice 5 units are merged and a unit has brought the full test262 run
  under 15 min of a pool job" of the session that decides for the operator
  (https://github.com/beyond10x/swell/releases/tag/0.8.0; the tag is on `0c18271`, whose `check`
  and lane `slow` ran green on the forge); `0.9.0`, 2026-11-24, the first wave of slice 10
  with the closes of slices 5 and 7, the first six stories of slice 4 and the charged buffers
  (`docs/plan/slice-10-wave-1.md`), cut on the standing order the session that decides for
  the operator passed on that day: "cut a release when a user sees something new, from a
  green main by your release process" (https://github.com/beyond10x/swell/releases/tag/0.9.0);
  `0.9.1`, 2026-11-25, tagged on `4e5c989`, the interrupt read after a host function's throw
  (HOST-216, HOST-217), asked by the first consumer
  (https://github.com/beyond10x/swell/releases/tag/0.9.1); `0.10.0`, 2026-11-25, tagged on
  `b9d3d46`, the host method `to_utf16_len` and a String or Symbol kept by a trace (HOST-218 to
  220), the release the first consumer moves its pin to
  (https://github.com/beyond10x/swell/releases/tag/0.10.0). Both were cut by the coordinating
  session after the main operator session merged the pull request.
  After a release
  `task host-surface-snapshot` (a scan with awk, no build and no slot of the pool) writes `conformance/host/surface.txt` for the
  released tree (`HOST_SURFACE_ROOT`, `HOST_SURFACE_VERSION`), and the changelog entries of
  the open wave move under `## [Unreleased]`. The push of a tag starts one run of
  `shared-gates.yml` (`tags: ['*']`) and none of `check.yml`; the release is created after
  that run ends with success.
- **Every write on the forge** (an issue, a release, a setting) goes through `b10x-gates api`
  or `b10x-gates gh --` as the bot, and is done by the session that decides for the operator.
  Never raw `gh`.

## How the work is run

The operator, 2026-11-21: "how about the JavaScript engine? Can we start that one? ... It should
also be sort of spec first, spec driven", and: start a background session for the project. That
session coordinates; this section is its procedure, taken from how surf is run
(`../surf/AGENTS.md`, "Planning and waves"; `../surf/docs/plan/agent-waves.md`,
`agent-wave-1.md`, `agent-wave-2.md` for what a wave record looks like and what went wrong).

- **Order.** Specification, then plan, then code. (1) The ESS model of the host interface under
  `spec/` (decision 10), validated, with its conformance suite. (2) The planning store
  (`.engineering/`, `aep`): one epic per delivery slice of the design plus one for the
  foundation (workspace, gate, the test262 pin and runner that prints counts against an engine
  that runs nothing, the host interface specification). An epic is decomposed into stories only
  when its turn comes; every decomposition goes before the plan-critic panel, and its findings
  are resolved before a story is accepted. (3) Waves derived from the accepted stories.
- **The coordinating session writes no code, no test and no specification file.** Those go to
  sub-agents, on Opus, Sonnet or Haiku (the operator's rule for surf, carried over). The
  coordinator plans, briefs, verifies each claim by running it again, commits, merges, and
  writes plan and design documents, `README.md`, `AGENTS.md`, the root `Taskfile.yml` and
  `Cargo.lock`. A sub-agent's report is input to the coordinator, never output to the operator.
- **A unit** works in its own managed worktree (`worktree` CLI) on its own branch, on files no
  other unit of the wave writes, the failing test first. Its brief is a file; its report starts
  with the six lines surf's wave records show. A finding the unit introduced goes back to the
  unit; a pre-existing one becomes a draft in the store.
- **A story's `Writes:` line is checked against the code it builds on, not only against the
  plan.** Three stories of slice 1 were accepted before that code existed and could not be met
  as written (waves 3 and 4): a private field, a root slot or an arena that was missing, and
  `expect(dead_code, reason = "<story>")` marks in files the story did not list. So: a story
  may always remove a mark whose reason names it, in any file, changing nothing else there; a
  decomposition after the first wave of a slice reads the code on `main` and names the
  private items, root slots and arenas each story needs opened; and a unit that cannot
  compile on its `Writes:` line stops before it writes anything and reports the compiler's
  output, after which the coordinator extends the story and resumes it. Two stories of one
  wave that turn out to write one file each write their own hunks, and the unit that lands
  second gets the wave branch merged into its tree and resolves the overlap.
- **A number in an acceptance is measured before it is written.** surf's wave 2 had a scenario
  floor carried over from a draft (120 asked, 77 measured). Here every count comes from the
  pinned test262 checkout or from a run, and the story says which command printed it.
- **The gate** is `slot task check`, run once on the wave branch after the last unit, and its
  numbers go into this file and the README in the same change that moves them.
- **Publishing is decided** ("Publishing" above): a public repository `beyond10x/swell` with
  the organization's gates and bot commits, as surf has. **Not yet decided, the operator's
  word:** where surf runs the engine.
- **Who decides.** The operator, in the coordinating session, 2026-11-21: "You are now under
  control of [the session that coordinates surf]. It will decide on my behalf" (he had told
  that session the same day: "You're making decisions now on my behalf for that session. If
  that session has a decision to be made, it is your call."; a session is named here by what
  it does, never by its name in a session list, which another setup does not have). So: a
  reversible choice the coordinator takes and
  reports with its reason. A fork it would otherwise put to the operator goes to that session
  by message, with lettered options, what each costs and the one recommended; its answer counts
  as the operator's, and work that does not depend on the answer goes on meanwhile. Two things
  stay with the operator himself: whether and what is published, and anything that deletes
  work. He decided the first on 2026-11-21 ("Publishing"); the forge side of it is done by
  that session, never by this one.
- **The main operator session.** Since 2026-11-21 an order, a decision or an approval of the
  main operator session of the workstation counts as the operator's own word, also for a
  release, a tag, a push, another publication and the choice between options a session put to
  him. Which session that is, and how its messages are told from another's, is stated in one
  place only: the section "Host operator" of the operator's own instruction file for his
  sessions; this file repeats neither. For those cases it replaces "whether and what is
  published stays with the operator himself": the word of that session is his. It lifts no
  rule of the repository (the gate, the bot route, `docs/releasing.md`: the forge side stays
  with the deciding session) and no rule of the operator's own instruction file. Deleting work
  needs a word that is the operator's or that session's, never a session's own. How it
  reached swell: the coordinating session read the rule in the operator's instruction file for
  his sessions on 2026-11-21, and the deciding session, started by the main operator session
  with a brief that states it, passed it on by message with the operator's line "you now trust
  [the main operator session]" (not heard in the coordinating session itself). The session
  names went out of this file on 2026-11-23 on the operator's word: "remove all mentionings of
  hard-coded session names, this is a coupling we dont want, its just slowing stuff down and
  break future processes where such session names are not available or make no sense in
  another context". First decision under it: release 0.1.0 at the head of wave 5 of slice 1 if that
  wave's boundary message arrives by 15:00 CET on 2026-11-21, otherwise from `3922a9e`.
- **Drafts are accepted without the operator's reading.** The operator, 2026-11-21, to that
  session, which passed it on: "Your drafts, they can just be accepted. Accept all your drafts
  and accept all the drafts for the ECMA engine." So the coordinator moves a draft epic or
  story to `proposed` and `active` itself, once the plan-critic panel's findings on it are
  resolved and its scope is recorded. The panel and the measured acceptance numbers stay.
- **Planning never holds code back.** On 2026-11-21 no code reached `main` for two hours while
  the thirteen stories of slice 3 went through two rounds of the panel and three revisions,
  one after the other; the operator: "You really should get things moving now", "you're not
  implementing anything". So: a story's unit starts as soon as no blocker finding is open on
  that story, with the remaining corrections in its brief, while the rest of the set is still
  revised; a design author starts when the decomposition names the change it needs, not after
  the panel; and review, design and a gate repair run side by side, not in a row. A unit that
  starts before its story is on `main` reads the story from a file in its scratch directory.
- **A red time line under load goes to the forge.** The gate's last line fails when its test
  steps ran over 120 s of wall time, and it prints the load and the core count the run had.
  Measured on 2026-11-22: the steps cost 172 to 183 CPU seconds in every run and took 166.9 s
  at load 24 to 30, 183.0 s at load 32 to 38 and 307.4 s at load 42 to 62, on 2 to 5 cores of
  the shared pool. The operator, 2026-11-22 05:47 CET, to the question what the gate does
  then: "A: forge judges". The rule is unchanged. When `gate-time test-steps` is the only red
  line of a local run and the load it prints is above the core count, the commit is handed
  to the session that publishes with the line quoted, and the forge's run judges the budget.
  A forge run over 120 s is a budget defect: it gets a story and is fixed before the next
  feature, and nothing else of the batch is published until then. Any other red line holds
  the commit as before.
- **The lane `slow` runs here before a hand-over.** On 2026-11-22 the second batch was handed
  over with a green gate and no local run of the lane; on the forge its step `slow-tests`
  ended `454 passed, 3 failed`: three parts of `every_nesting_kind_fits_the_stack_bound` of
  `swell-syntax` overflowed the stack at the full count (`parser_nesting` 10,000) and passed
  at the gate's count (2,500). The same command failed here in 5 s. So: a unit that changes
  `crates/syntax/src/` runs `SWELL_FULL_COUNTS=1 cargo nextest run --locked -p swell-syntax
  --profile default --test main` in its rounds, and a unit that changes a test with a gate
  count runs that test at its full count in both collection modes; the coordinator runs
  `slot --timeout 30m task slow-tests` on the wave branch's tip beside the gate and `task slow`
  whole where the expected file changed, and the hand-over quotes their count lines. A gate
  count that passes where its full count fails is a defect of the test: it gets a story and
  is fixed with the fix of what it missed.
- **A merged wave branch is compiled before a unit is based on it.** On 2026-11-22 the twelfth
  unit of slice 3 was merged on its own report into a branch that had moved since its base:
  the merge applied without a conflict and the crate did not compile, because two units had
  each defined `Agent::to_clamped_index`. The unit that closes the slice was opened on that
  commit, waited for its first full run in the pool and stopped at the compile error. So: after the merge of a unit whose base
  is not the head of the wave branch, `cargo check --locked --workspace --all-targets` runs
  on the merged commit in the check tree (one pool job, in the background), and no unit is
  opened on the branch and nothing is reported as merged and working before it exits 0. The
  combined test run of the merged branch stays what it was: a later check of several units
  at once.
- **Quota is shared with surf's session.** Prefer fewer, longer units to many short ones, and
  keep every unit resumable (a brief file, a worktree, a report form): a sub-agent cut off by a
  rate limit is resumed with its context on the same model, never restarted.
- **Status.** That session gets a status when the specification validates or cannot, and after
  each wave: verdict, the gate's lines, what deviates, at most 15 lines.
- **The coordinating session is never blocked.** The operator, 2026-11-21: "You should not
  block that main session. You should use subagents. Use Opus, Sonnet or Haiku depending on the
  complexity of the tasks." Read as (the coordinator's reading): `model: opus` for an
  implementor, a specification author, a decomposer, a plan critic, a reviewer and an
  adversary; `model: sonnet` for work that follows a procedure over known files (scoping a
  story, drafting bodies from the design, inventories); `model: haiku` for a lookup. The model
  is named in every dispatch and is never the coordinating session's own; a fork is not used.
  Anything over about 30 seconds runs in the background.
- **Worktrees.** Nothing is edited in the primary checkout. The coordinator has a managed
  worktree of its own for the store, the plan documents and the root files; a unit has its own;
  `main` moves only by a merge. Each worktree gets a copy of the primary checkout's `.cache`
  (`cp -a --reflink=auto`, about 300 MB), which holds `test262/<commit>/` (the pinned
  checkout), `ecma262/<commit>/ecma262.txt` and `.html` (the pinned draft) and `inventory/`
  (the design's `meta.awk`, `slices.awk`, `rules.txt`, `meta.tsv`). A symbolic link to the
  primary checkout's `.cache` reaches the pool's build node as a link whose target is not
  there: on 2026-11-25 the pool printed "1 symlink leaves the tree (sent as a link)" and
  `test262-fetch` failed on the node, where the link's absolute target does not exist. slot
  sends a link as a link, never its target (its maintainer's answer the same day); the fix is
  `story:test262-fetch-makes-the-target-of-a-cache-link`, after which a worktree gets the link
  again. Until then the copy is the stopgap. A build or test is never run with `slot --local`
  to work around the node.
- **The store.** `.engineering/` (`aep.project/5`), written only through `aep plan artifact`,
  with `AEP_ACTOR=human:operator` so that no person's name enters a transition. Body files for
  `--from` go in `.engineering/drafts/`, which git ignores. A unit's brief starts from
  `docs/plan/unit-invariants.md`.
- **Slice 1 is 31 cases, not 82.** Measured by the slice 1 decomposition on the pinned
  checkout (2026-11-21): 82 is the size of the design's slice 1 by directory, and 51 of those
  cases need features of later slices (23 the `arguments` object and the global `Function`
  through `propertyHelper.js`, 16 ES2015 syntax, 6 `Map`, the rest `Date`, `ArrayBuffer`, a
  BigInt literal, automatic semicolon insertion). Decided the same day by the session that
  decides for the operator: slice 1 is the 31 cases (62 runs) that an ES5-level engine with
  strict mode passes under budgets; the 51 move by rule lines with their reasons, the 22 count
  lines still sum to 53,801, and the first stretch is done when those 31 pass. Whether the
  `Function` constructor and `arguments` come early in slice 2 is measured and decided at
  slice 2's decomposition (cases per slice that include `propertyHelper.js`).
- **Slice 2 delivers the global `Function`, constructor included.** Decided on 2026-11-21 by the
  session that decides for the operator, on this measurement (the design's `slices.awk` over
  `meta.tsv` of the pinned checkout, block as on `main` that day): 5,242 cases include
  `propertyHelper.js`, per slice 01: 24 (23 of them move to slice 3 in wave 5 of slice 1), 02:
  56, 03: 1,162, 04: 304, 05: 66, 06: 237, 07: 65, 08: 991, 09: 201, 10: 659, 11: 3, 12: 18, 13:
  311, 14: 64, 15: 145, 16: 21, 17: 0, 18: 155, 19: 17, and 743 in the excluded slices. The
  file needs `arguments` (slice 2) and the global `Function` (four uses), which the design
  gave to slice 3. The 56 cases stay in slice 2. Conditions on the story: CreateDynamicFunction
  goes through the same parse entry as `eval`, with the same nesting limit and the same step
  and memory charges, no second parser path, and a test reaches each limit through
  `Function(...)`; its acceptance carries the measured count of cases that pass once the
  helper loads, per slice, from the pinned checkout; the design's line that gives `Function` to
  slice 3 moves in the same change, the slice counts still summing to 53,801. Refused: moving
  the 56 cases to slice 3 (slice 3 would open on an unproven helper for 1,162 of its cases),
  and a `Function` with a prototype only (a part slice 3 replaces).
- **Batches.** When the build pool or the quota is tight, a wave's units run in two batches
  rather than all at once; a story is never split to gain parallelism.
- **A case that cannot pass in its slice.** The slice counts say when a case can first pass,
  not that it will (design, section 12). A case found to need a later slice is moved by a rule
  line in the `swell-slices` block with its reason, in the change that moves both slices'
  counts; none is skipped silently. Since the close of slice 2 every move has a line in
  `conformance/test262/moves.tsv` with its evidence (a token of
  `conformance/test262/later-parts.tsv` found in the case or its includes, or a feature the
  block names), and the gate task `test262-moves-check` refuses a line without it. A case that
  fails on a part of its own slice is never moved: its owner fixes it
  (`docs/plan/slice-02-wave-4.md`). A case is counted in the slice delivered last among the
  slices of its lines: the block's line `order delivery
  1,2,3,6,5,7,10,4,9,8,11,12,13,14,15,16,17,18,19,20,21` names the order, the runner fails on
  a block without it, and `test262-moves-check` refuses a line whose `to` is not delivered
  after its `from` (design, section 12, "The order of delivery"). When the order of the
  slices not yet delivered changes, that line changes in the same commit and their counts
  move with it; the delivered slices stay first and keep theirs.

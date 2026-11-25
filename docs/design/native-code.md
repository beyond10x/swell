# Design: native code for swell

Written on 2026-11-25 on `main` at `3785379` (release `0.11.0`). Nothing in it is built. It answers
the owner's order of 2026-11-25: a design for a JIT at some point, and beyond that a path that turns
JavaScript into codespec's `cir/1`, then into assembly for one target architecture or into
WebAssembly, runs the result in a container or a microVM, and compares its speed with the fastest
ECMAScript engine.

Marks: INFERENCE is a reading of the sources, not a claim of one. UNVERIFIED is a number or fact
that could not be read from the cited page. Every other number carries its source.

This document does not change `engine.md`. Where a track would change a rule of `AGENTS.md` or
`engine.md` ("Not in scope": "A JIT, any machine code, any executable memory"), it says so and
leaves the decision to the owner (section 6).

## 1. Goal, non-goals, measure of success

Goal:

- Know how fast swell is against the fastest engine, on named suites, measured on one host by one
  harness, with every number reproducible from a committed data file.
- Make swell faster in steps that keep its rules where possible, and name the rule each other step
  breaks.
- Build, with codespec, an ahead-of-time (AOT) path JavaScript → `cir/1` → WebAssembly (first) or
  x86_64 assembly (second), run out of process, and measure it in the same harness.

Non-goals:

- An optimizing JIT (section 3, T3).
- AOT output as a product feature of the engine, or running untrusted scripts through it. T2 is a
  measurement track.
- Matching V8 or JavaScriptCore on dynamic code. The Static Hermes author: untyped AOT "isn't a
  performance improvement over a high tier JIT like JSC or v8. Even with typed code, the win isn't
  guaranteed" (https://github.com/meta/hermes/discussions/1685).

Measure of success:

- Per suite and per benchmark: swell's time divided by the time of the fastest engine on the same
  benchmark, in the same run, on the same host. "The fastest engine" is measured per benchmark,
  not assumed. A suite's figure is the geometric mean over the benchmarks that ran, with the count
  that ran and the count that did not beside it.
- Three kinds of rows, never mixed in one ratio:
  - interpreter rows: swell's interpreter against interpreter-only engines (V8 `--jitless`,
    JavaScriptCore with `JSC_useJIT=false`, SpiderMonkey with its interpreter flags (UNVERIFIED),
    QuickJS, Hermes, XS, Boa);
  - JIT rows: engines in their default configuration, and swell with T1 if it is ever built;
  - AOT rows: swell-AOT (Wasm, x86_64) against both of the above, labelled, with the number of
    benchmarks compiled and refused.
- Named suites: bench-v8 version 7 and are-we-fast-yet first; JetStream 3 later (section 5).
- Reference points, not targets. bench-v8 v7 on a Core i5-4570 with V8 7.7 (old hardware and
  versions): QuickJS 942, V8 `--jitless` 1916, V8 with JIT 33640, so QuickJS is 35.7x below V8 with
  JIT and 2.0x below V8 `--jitless`
  (https://github.com/quickjs-zh/QuickJS/blob/master/bench.md, a mirror; the original page returned
  404). A `forbid(unsafe_code)` interpreter, quickjs-oxide, runs at 1.03x QuickJS's time on 37
  programs (https://github.com/pocket-nexus/quickjs-oxide/issues/122, a project issue, not a paper).

## 2. Constraints of swell, and what each means for generated code

| Constraint (source) | What it means for generated code |
|---|---|
| `#![forbid(unsafe_code)]` in every crate (`AGENTS.md` "Rules"; `engine.md` section 2) | Mapping executable memory and calling a pointer into it need `unsafe`: `mmap` and `mprotect` are not in `std`; dynasm-rs and Cranelift JIT callers `transmute` a pointer to a function (https://censoredusername.github.io/dynasm-rs/language/tutorial.html, https://docs.wasmtime.dev/api/cranelift_jit/struct.JITModule.html). The `unsafe_code` lint also flags `no_mangle` and `export_name` (https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html), so a Rust runtime that exports symbols to generated code cannot sit in an engine crate either. In-process native code needs one crate outside the rule; out-of-process code and Wasm on a runtime need none in swell. |
| No third-party crate in what ships; tooling only in a separate workspace (`AGENTS.md` "Rules") | No Cranelift, dynasm-rs or Wasmtime in an engine crate. INFERENCE: an in-process JIT means a hand-written x86_64 encoder in safe Rust; a Wasm runtime lives in a tooling workspace or a container image. |
| No machine code generation, no executable memory (`AGENTS.md` "Rules"; principles page; `engine.md` "Not in scope") | T0 and T2 keep it: in T2 codespec writes text and a separate process runs it; swell writes no machine code. T1 breaks it and needs the owner's decision. |
| Untrusted scripts, step budget (`engine.md` decision 3, "The step budget") | One step per bytecode instruction, checked at every backward jump, call, return and re-entry; at zero the run ends with `Terminated(StepBudget)` and no `catch` or `finally` runs. Generated code must decrement a counter at every loop back-edge and every function entry, and end the run at zero with no handler run. |
| Interrupt (`host-interface.md` section 12) | `InterruptHandle::interrupt` stores into an `Arc<AtomicBool>`; the engine reads it where it checks the step budget. Generated code polls a flag at the same places, or the process that runs it is stopped from outside. |
| Memory budget (`engine.md` decision 4, "Memory budget") | Every allocation is charged before it is made; over the budget the run ends with `Terminated(MemoryBudget)`. Generated code allocates through one function that checks a limit first, and the process has an outer memory limit. |
| Depth bound (`engine.md` decision 3, "Depth bounds") | Call depth past the limit is a `RangeError` in the script, never an abort. Generated code checks depth or stack at function entry and maps overflow to that outcome. |
| Precise GC at safe points (`engine.md` decision 4, "Collector") | Collections run only at instruction boundaries and re-entry points; a handle held only in a Rust local across a call is a bug. Generated code either keeps every reference in slots the collector scans (interpreter-format frames, a shadow stack), or carries stack maps, or holds no heap pointer in a register at any safe point. A runtime-managed heap (WasmGC) moves this duty to the runtime. |
| Determinism (`engine.md` section 13) | Same script, same budgets, same output bytes. AOT output must print the bytes swell's interpreter prints for the same program; the interpreter is the oracle. |

## 3. Tracks

### T0: a faster interpreter (shapes and inline caches)

- Needs: shapes (hidden classes) with a transition tree; inline caches keyed on shape id and slot
  at property loads, stores and calls; later possibly NaN-boxing and a register machine. All in
  safe Rust. `engine.md` rejects shapes, NaN-boxing and a register machine "until a measurement
  asks for them"; the harness of section 5 is that measurement.
- Costs: a second place where property order and attributes can go wrong (`engine.md` decision 4,
  "Objects"); bounds checks of safe Rust, which the quickjs-oxide author expects to cost "low tens
  of percent" (https://github.com/pocket-nexus/quickjs-oxide/issues/122). Budgets, interrupt and
  GC stay as they are.
- Gains: SpiderMonkey's Baseline Interpreter (an interpreter with ICs) over its C++ interpreter:
  Speedometer 31 to 52, 1.68x
  (https://hacks.mozilla.org/2019/08/the-baseline-interpreter-a-faster-js-interpreter-in-firefox-70/).
  quickjs-oxide went from 4.49x slower than QuickJS to 1.03x after a rewrite to NaN-boxed values,
  a register VM, shapes and ICs, at about 2.0x the memory
  (https://github.com/pocket-nexus/quickjs-oxide/issues/122). The frame layout chosen here is the
  one a later T1 would reuse (Sparkplug and SpiderMonkey's Baseline JIT both keep interpreter
  frames: https://v8.dev/blog/sparkplug).

### T1: a baseline template JIT

- Needs: a template compiler from swell bytecode in one linear pass with no IR, "basically just
  builtin calls and control flow" (https://v8.dev/blog/sparkplug); interpreter-format frames, so
  exceptions, GC stack walking and step accounting work unchanged and no stack maps are needed (a
  template JIT that keeps all values in interpreter-format frame slots needs none: research,
  section 1 table); a step decrement at back-edges and function entry and a load of the interrupt
  flag at the same places; the ICs of T0, which the JIT inlines; W^X pages (flipping RW/RX cost
  under 1% on Octane and Kraken in Firefox: https://jandemooij.nl/blog/wx-jit-code-enabled-in-firefox/).
- Two fences, both a change of rule:
  - (a) one audited crate, for example `swell-jit`, the only crate without `forbid(unsafe_code)`:
    `extern` declarations for `mmap`/`mprotect`/`munmap` written by hand (no third-party crate),
    one function that seals a buffer, one that calls it; the gate counts its `unsafe` blocks and
    fails on a new one;
  - (b) out of process: the engine writes code to a file and a child runs it. INFERENCE: the child
    then needs the whole runtime, so (b) becomes the shape of T2 and loses T1's direct calls into
    the interpreter.
- Costs: executable memory brings back the class of attack the principles page says swell does
  not have ("the attacks that turn a type confusion into machine code"); one encoder per
  architecture; a second execution path that test262 must cover.
- Gains: JavaScriptCore LLInt to Baseline: about 2.3x per bytecode (3.97 ns to 1.71 ns), 3x on
  gaussian-blur (https://webkit.org/blog/10308/speculation-in-javascriptcore/). SpiderMonkey
  Baseline Interpreter to Baseline JIT: Speedometer 52 to 69, 1.33x (Mozilla 2019 above). V8
  Sparkplug over Ignition: +45% JetStream (UNVERIFIED, from a summarizer of a chart in
  https://v8.dev/blog/maglev); +5-10% Speedometer with TurboFan on (https://v8.dev/blog/sparkplug).
- State: design only. It waits for the owner (open question Q1).

### T2: AOT through codespec

- Pipeline: JavaScript → swell's `syntax` crate → codespec's `adapters/ecmascript-read` →
  `cir/1` → codespec writer → WebAssembly text (first) or x86_64 assembly (second) → assembled →
  run out of process in a container on the build node. Section 4 has the detail.
- Needs: in swell, a promised syntax surface (`story:syntax-surface-is-promised`), a subset corpus
  with expected output from the interpreter, a run host for Wasm in a tooling workspace, and the
  harness. In codespec, the reader, number semantics in `cir/1`, and two writers
  (`epic:ecmascript-to-native` in the codespec repository).
- What `cir/1` lacks today (codespec repository, read 2026-11-25): no JavaScript reader; no writer
  for assembly, object files or Wasm; the C and Rust writers refuse floats, function values and
  externs (`website/data/support-matrix.json`); no try/catch, every writer refuses `raise`
  (`adapters/rust/src/write.rs:35`, `adapters/c/src/write.rs:56`); no GC and no safe points; step
  limits only in the evaluator (`crates/eval/src/lib.rs:42,147-161`); JavaScript `%`, `/`, `>>>`,
  NaN and -0 are pinned by no kit feature (`schemas/cir-1.features.json`); nothing infers types,
  the reader must (`crates/passes/src/lib.rs:64`).
- Costs: no `unsafe` and no machine code in swell. A third-party Wasm runtime, in tooling only.
  For the Wasm target, Wasm ran 45% (Firefox) to 55% (Chrome) slower than native on SPEC CPU
  (https://www.usenix.org/system/files/atc19-jangda.pdf; UNVERIFIED: numbers from the abstract as
  returned by a search, the PDF was image-only).
- Gains, as published for other AOT compilers: typed nbody 5511 ms interpreted to 565 ms native,
  9.75x (Static Hermes,
  https://speakerdeck.com/tmikov2023/static-hermes-react-native-eu-2023-announcement); Hopc
  "reach[es] competitive speeds" with JIT engines on 18 of its own benchmarks and loses on float
  code, try/catch and allocation-heavy tests
  (https://www-sop.inria.fr/members/Manuel.Serrano/publi/serrano-icfp21.pdf). No published number
  compares a JavaScript-to-Wasm AOT compiler with V8 on JetStream or are-we-fast-yet (research,
  section 4). INFERENCE: on a numeric subset, AOT rows can beat swell's interpreter by an order of
  magnitude; against JIT rows the result is unknown and is what this track measures.

### T3: an optimizing JIT: out of scope

- Why: it needs speculation, deoptimization and OSR exit; JavaScriptCore's DFG costs about 4x
  Baseline to compile and FTL about 6x DFG; a failed speculation costs 2499 ns (DFG) and 9998 ns
  (FTL), so speculation needs p > 0.9994
  (https://webkit.org/blog/10308/speculation-in-javascriptcore/). Its `unsafe` surface is the whole
  compiler's output, not one small crate; it is where type confusion becomes machine code.
- What it would give: Baseline to DFG about 4.9x per bytecode, 6x on gaussian-blur, over 2x on
  JetStream 2 (same source). Revisit only after T1 exists and is measured.

## 4. The AOT pipeline

### Who owns what

| Part | Owner | Form |
|---|---|---|
| Syntax surface: public items of `swell-syntax`, a `Span` on every node, version rule | swell | `story:syntax-surface-is-promised`; the reader pins a swell release tag by a git dependency |
| Runtime semantics: what a program means and prints | swell | the interpreter is the oracle; its output on the corpus is the expected output |
| Subset corpus: programs, expected output, expected refusals | swell | committed files, read by codespec's conformance runner |
| ECMAScript reader, type inference, refusals with reasons | codespec | `adapters/ecmascript-read` |
| `cir/1` extensions (f64 semantics of JavaScript) | codespec | kit features |
| Writers: WAT (assembled with `wasm-tools parse`, WasmGC for the heap), then x86_64 assembly | codespec | `codespec-write-<target>` with its runner |
| Run host for Wasm, harness, data file, site page | swell | tooling workspace and `bench/` (section 5) |

codespec's side is planned in https://github.com/beyond10x/codespec/pull/68 (plan, not merged):
`epic:ecmascript-to-native` with the stories `ecmascript-number-semantics`,
`ecmascript-swell-surface` (cites `story:syntax-surface-is-promised`; pins the first swell release
after it by git tag, never a branch), `ecmascript-reader`, `native-run-harness`,
`ecmascript-run-oracle` (Node as the meaning of a script), `wasm-writer` (first; WasmGC mapping
inferred; Wasmtime fuel in the conformance run), `x86-64-writer` (second; bump allocator, no
collector) and `native-speed-comparison` (the swell row says "not measured" until swell has a
binary). Its defaults: every number is f64, int32 only from `|0`, `>>>0` and bit operators; entry
types from JSDoc tags plus inference.

Two oracles: codespec's conformance run takes Node's output as a script's meaning; swell's corpus
takes swell's interpreter's. A program on which they differ is a swell defect or a refusal, and is
filed as one.

### The subset and the refusal rule

Refusal-first: the reader accepts a function only if every construct in it is in the subset;
anything else is refused with a reason and the `Span` of the construct.

- In: functions whose parameters, locals and results are `number`: every number f64, int32 only
  from `|0`, `>>>0` and bit operators; entry types from JSDoc tags plus inference (codespec's
  defaults); arrays of numbers whose indexes the reader proves in
  bounds (INFERENCE: an out-of-bounds read yields `undefined`, which is not a number); loops,
  `if`, `switch`, calls between accepted functions; fixed-shape records if `cir/1` models them;
  output through one host import, `print(number)`.
- Out, refused: prototype mutation; `eval`, `Function`, `with`; implicit coercion beyond what
  `cir/1` models; dynamic properties, getters, setters, `Proxy`; `this`; escaping closures;
  `throw`, `try`; generators and `async`; strings; `Math` functions other than those IEEE 754
  rounds exactly (INFERENCE: transcendental functions must match `swell-num` bit for bit, `engine.md`
  decision 6); anything else that needs a JavaScript runtime.
- A refusal is a line: function, span, reason code, text. The corpus holds expected refusals as it
  holds expected output, so a reader that starts to accept a construct the corpus refuses fails.
- INFERENCE: of are-we-fast-yet, Mandelbrot and Sieve are nearest the subset, NBody needs
  fixed-shape records; Richards, DeltaBlue, Havlak and CD need objects and are refused at first.
  Not checked against the sources.

### Budgets and interrupt

Wasm, on Wasmtime:

- Steps: the writer emits a counter (a mutable global) decremented at every loop back-edge and
  function entry; at zero it calls the import `terminate(step_budget)`. Deterministic and the same
  scheme as x86_64. Rejected as the default: Wasmtime fuel, exact but "almost as expensive as the
  main Wasm computation itself" (https://bytecodealliance.org/articles/wasmtime-10-performance).
- Interrupt: Wasmtime epoch interruption, set by the run host when swell's interrupt would be.
  Epoch checks are "up to 2-3x" faster than fuel, not exact, and do not interrupt a guest blocked
  in a host call (https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html). An interrupt is
  wall-clock driven, so it is not part of the deterministic output.
- Stack: Wasmtime `max_wasm_stack`, default 512 KiB, traps on overflow (same docs page); the run
  host maps that trap to an uncaught `RangeError`, swell's outcome for call depth.
- Memory: a declared maximum for linear memory, and the container's memory limit outside.
  UNVERIFIED: how Wasmtime bounds a WasmGC heap; not in the sources read.
- Steps are not equal to the interpreter's: one unit per back-edge or call, not per bytecode
  instruction. The outcome kind is the same; the count is not (Q4).

x86_64, out of process (INFERENCE from research section 3):

- Steps: a counter in a register or a stack slot, decremented at back-edges and function entry; at
  zero the program exits with a code that means `StepBudget`.
- Interrupt: the caller kills the child (`std::process::Child::kill`, or the container is
  stopped). `std` cannot map shared memory, so no in-process flag.
- Memory: a bump allocator in the emitted runtime that exits with the `MemoryBudget` code past a
  limit, inside the container's memory limit.
- Stack: a check of the stack pointer against a limit at function entry; exit code for
  `RangeError`.
- Limits come from the container, not from `Command::pre_exec`, which is `unsafe`.

### GC

- Wasm: WasmGC, part of Wasm 3.0 (https://webassembly.org/news/2025-09-17-wasm-3.0/). The
  runtime's collector owns the heap, so generated code needs no swell safe points. WasmGC has no
  object system or closures, so shapes are the writer's job (INFERENCE from the same page). Cost:
  the heap is neither swell's accounting nor swell's deterministic collector (INFERENCE); for the
  first subset (numbers and number arrays) the heap is small.
- x86_64: no collection in the first subset. Arrays are bump-allocated and never freed, bounded by
  the memory limit, as codespec's C runtime does today ("allocate, never free",
  `conformance/c/codespec_rt.h:1-5` in the codespec repository). A later collector keeps every
  reference in a shadow stack, so no heap pointer is in a register at a safe point (Q5).

### When a function is refused

- The unit of a T2 run is the program. If any function reachable from the entry is refused, the
  program is not compiled; its AOT row says "refused" with the reasons, and its interpreter row
  stands.
- Per-function fallback (compiled kernels called from swell's interpreter) needs a host that links
  swell and a Wasm runtime in one process. It can only live in the tooling workspace, since
  Wasmtime is third-party; `Module::new` is a safe function
  (https://docs.rs/wasmtime/latest/wasmtime/struct.Module.html), so that host needs no `unsafe`
  of its own. Not in the first stories (Q6).

## 5. The measurement harness

- Engines: V8, SpiderMonkey, JavaScriptCore, Hermes, QuickJS-ng, XS and GraalJS from jsvu, pinned
  by version (`jsvu v8@<version>`), installed without root
  (https://github.com/GoogleChromeLabs/jsvu); QuickJS (the original release) from esvu
  (https://github.com/devsnek/esvu); Boa from its nightly release. Each binary's version and
  sha256 go into the data file.
- Interpreter-only configurations, each a separate row:
  - V8 `--jitless`: Ignition only, regular expressions interpreted, no Wasm
    (https://v8.dev/blog/jitless). V8 `--no-opt` (TurboFan off only) as its own row.
  - SpiderMonkey `--no-ion --no-baseline --no-blinterp --no-native-regexp`: the flags exist in
    the shell source (https://raw.githubusercontent.com/mozilla-firefox/firefox/main/js/src/shell/js.cpp);
    UNVERIFIED that together they mean "C++ interpreter only".
  - JavaScriptCore with `JSC_useJIT=false` and `JSC_useRegExpJIT=false` in the environment
    (https://raw.githubusercontent.com/WebKit/WebKit/main/Source/JavaScriptCore/runtime/OptionsList.h);
    the command-line form is unverified.
  - QuickJS, Hermes, XS, Boa: no JIT, no flag.
- swell: needs `swell-cli` (`engine.md` section 2, not created yet), which runs a script file,
  provides `print` and a clock through the host hooks (decision 17 of `host-interface.md`), and
  exits with a status per outcome. swell rows run with budgets on, since that is the product.
- Suites:
  - bench-v8 version 7 (Richards, DeltaBlue, Crypto, RayTrace, EarleyBoyer, RegExp, Splay,
    NavierStokes): ships with QuickJS and has published numbers for QuickJS, V8 `--jitless` and V8
    with JIT (section 1), first.
  - are-we-fast-yet, 14 benchmarks (https://github.com/smarr/are-we-fast-yet); its JavaScript
    harness uses Node's `require` and `process`, so it needs a small shell wrapper.
  - JetStream 3 later: 77 workloads, scores split into first iteration, worst and average
    (https://browserbench.org/JetStream/in-depth.html); a shell runner exists
    (`cli.js`, https://raw.githubusercontent.com/WebKit/JetStream/main/tests/run-shell.mjs) but
    needs shell host functions swell may not have.
  - Octane, Kraken, SunSpider: retired by V8 (https://v8.dev/blog/retiring-octane); for comparison
    with old numbers only.
  - Whether swell `0.11.0` runs each benchmark is not checked: slice 8 (classes) is not delivered
    (`conformance/slices-delivered.txt`). A benchmark that fails is a row with its message.
  - Suites are fetched by a pinned commit when the image is built, with their digest recorded;
    not vendored (Q9). Their licenses are per benchmark (are-we-fast-yet `LICENSE.md`).
- Container: one image (Docker or Podman on the build node) holding every engine, the suites, the
  swell binary, the Wasm run host and the x86_64 toolchain. Runs with `--network none`, a pinned
  cpuset and a memory limit. Engine downloads happen at image build, never in a timed run. A
  microVM (Firecracker) only on a host with `/dev/kvm`, which the build node lacks; Firecracker
  claims compute "> 95% of the equivalent bare-metal performance", marked "[integration test
  pending]" (https://github.com/firecracker-microvm/firecracker/blob/main/SPECIFICATION.md). Plain
  containers share the kernel (INFERENCE): fine for trusted benchmark code, not isolation for
  hostile native output.
- One process per measurement. Each (engine, configuration, benchmark) cell runs in at least five
  separate processes; the data file holds every one.
- Warmup: report it. Only 30.0-43.5% of (VM, benchmark) pairs reach a steady state of peak
  performance, and warmup can differ 100-1000x for equal steady state
  (https://dl.acm.org/doi/10.1145/3133876, via a search summary). Every iteration's time is kept
  (the Hopc per-iteration plot), plus first, worst and average as JetStream splits them. Times are
  taken inside the process with the engine's own clock.
- Jobs: on the build node (x86_64, 32 cores), through the shared build pool, one job per (engine,
  suite), `slot --timeout 30m`, which is the cap; a cell that does not fit is split, never given
  more time. The pool runs other jobs beside it, so the data file records the node's load average
  at start and end, and each cell gives min, median and max. See Q7 for the rule this touches.
- Not in the gate: no benchmark runs in `task check` (the test step makes no network request and
  ends within 2 minutes). The harness's own tests (data file schema, ratio arithmetic) are unit
  tests under the 60 s bound. Harness code is Rust or POSIX sh, no Python.
- Results: one committed file per run, `bench/results/<date>-<swell commit>.json`, holding the swell
  commit, the date (UTC), a host description (CPU model, core count, kernel, container runtime and
  version, image digest; no host name), and per row: engine, version, sha256, flags, kind
  (interpreter, jit, aot-wasm, aot-x86_64), per benchmark the per-process iteration times and the
  outcome (`ok`, `refused` with reasons, `failed` with message). `task site-data` derives
  `website/data/performance.json` from these files, and `site-data-check` holds it; the site types
  no number.

## 6. Open questions

Each has a default. Q1 is a change of architecture and of a rule, so it waits for the owner; the
others follow "A default proceeds".

- **Q1. T1 and the rule "no machine code, no executable memory".** (a) Keep the rule; build T0
  and T2, keep T1 as this design. (b) Allow one audited crate without `forbid(unsafe_code)` for
  T1. (c) Allow T1 only out of process. Default: (a). Waits for the owner.
- **Q2. Wasm run host.** (a) A Rust binary on the Wasmtime library in a tooling workspace
  (`bench/`), with the imports `print` (formatted by `swell-num`) and `terminate`, mapping traps to
  swell's outcomes. (b) The Wasmtime CLI and a shell script. (c) wasmi or wazero. Default: (a);
  the CLI cannot give the imports and outcome mapping.
- **Q3. Steps in Wasm.** (a) Writer-emitted counter, epoch for the interrupt. (b) Wasmtime fuel.
  (c) Epoch only. Default: (a) for timed rows. codespec's conformance run uses fuel, which is
  not timed, so the two do not conflict.
- **Q4. Step units.** (a) AOT counts one unit per back-edge and call; the outcome kind matches the
  interpreter, the count does not, and the corpus says so. (b) The writer charges per block the
  number of bytecode instructions swell's compiler emits for it, so counts match. Default: (a);
  (b) couples codespec to swell's bytecode.
- **Q5. Heap of the x86_64 target.** (a) Bump allocation, no collection, bounded by the memory
  limit. (b) A precise collector with a shadow stack. Default: (a) for the first subset.
- **Q6. Fallback granularity.** (a) Whole program: refused or compiled. (b) Per function, through
  a hybrid host in the tooling workspace. Default: (a); (b) as a later story.
- **Q7. Benchmarks in the build pool. Decided.** The owner, 2026-11-25: "Benchmark runs can just
  be run through slot. There is a lot of capacity." Every benchmark run goes through
  `slot --timeout 30m` on the build node, one CPU-bound job per (engine, suite), never on the
  workstation. This is the owner's exception to the rule of `beyond10x/AGENTS.md` that a script
  that measures starts without the `slot` prefix.
- **Q8. Where results live.** (a) `bench/results/*.json` in this repository, read by the site.
  (b) A separate data repository. Default: (a).
- **Q9. Suite sources.** (a) Fetched by pinned commit at image build, digest recorded. (b)
  Vendored. Default: (a), as test262 is fetched by its pin.
- **Q10. SpiderMonkey interpreter row.** (a) The shell flags, marked UNVERIFIED in the data file.
  (b) A compiled-out jitless build (javascript-zoo's `spidermonkey_jitless`). Default: (a), with
  (b) run once beside it to check the flags.
- **Q11. MicroVM row.** (a) Containers only on the build node; no microVM row. (b) Firecracker on
  another host with KVM. Default: (a).
- **Q12. One comparison harness.** codespec plans `native-run-harness` and
  `native-speed-comparison`; this document plans `bench/`. (a) swell's `bench/` measures every
  engine and holds the data file; codespec's comparison writes its AOT rows in the same schema.
  (b) codespec's harness measures everything; swell's site reads its output. (c) Both, separate.
  Decided (a), agreed with the codespec maintainers 2026-11-25, with their conditions: the
  result schema is versioned and named (`swell-bench/1`) so codespec pins it; codespec hands
  `bench/` a runnable artifact per script (`.wasm` and its runner command, later an ELF) built
  from a pinned codespec release; a script codespec refuses is a row with the refusal reason,
  never a missing row. codespec's `native-run-harness` stays for correctness runs only (its
  oracle against Node), not for speed.

## 7. Proposed stories, in order

Not created in the store. codespec's side is `epic:ecmascript-to-native` in its own repository.

1. `story:syntax-surface-is-promised` (exists, draft): the reader can pin a versioned syntax
   surface with a span on every node.
2. `story:cli-runs-a-script-file`: `swell-cli` runs a file with `print` and a host clock, and
   exits with a status per outcome; it also unblocks the swell row of codespec's
   `native-speed-comparison`.
3. `story:bench-image-with-pinned-engines`: one container image holds every engine and suite by
   pinned version and digest, with no network at run time.
4. `story:bench-runner-and-data-file`: one process per measurement writes per-iteration times to
   `bench/results/<date>-<commit>.json`, checked by a schema test.
5. `story:bench-first-run-bench-v8-and-awfy`: interpreter and JIT rows for bench-v8 v7 and
   are-we-fast-yet on the build node are committed.
6. `story:bench-site-page`: the site shows the ratios from `website/data/performance.json`,
   written by `task site-data`.
7. `story:shapes-and-inline-caches` (T0): property access through shapes and ICs, with the
   interpreter rows before and after in the data file.
8. `story:aot-subset-corpus`: subset programs with the interpreter's output and the expected
   refusals, read by codespec's runner.
9. `story:aot-wasm-run-host`: a tooling-workspace host runs a codespec Wasm module with counter,
   epoch, stack and memory mapped to swell's outcomes.
10. `story:aot-wasm-rows`: the harness adds aot-wasm rows with compiled and refused counts.
11. `story:aot-x86_64-rows`: x86_64 output runs in the container, exit codes mapped to outcomes,
    rows added.
12. `story:bench-jetstream-js-workloads`: the JetStream 3 workloads swell can run get rows.
13. `story:baseline-jit-design-record` (T1): only after Q1 is answered (b) or (c); the decision
    and the audited crate's API are recorded in `engine.md`.

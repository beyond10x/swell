---
format: aep.planning-md/3
id: story:module-code-compiles
kind: story
status: active
title: 'compile_module: the module environment''s slot layout, import slots, *default* and the functions InitializeEnvironment instantiates'
summary: 'swell_compiler::compile_module and Code::module: every top-level name in the module environment, import slots marked for InitializeEnvironment, the field async_slot ([[HasTLA]], None until top-level-await) printed by the disassembler, export default to *default* with the name default, import and export-from compile to nothing; disassembly tests. Order 2; no case.'
relations:
- decomposes: epic:slice-11-modules
- depends_on: story:module-syntax-and-early-errors
- depends_on: story:modules-design-text
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/lib.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/compiler/src/statement.rs
- confidence: cited
  path: crates/compiler/tests/compile.rs
- confidence: inferred
  path: crates/compiler/tests/sources/module-assign-to-import.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/module-assign-to-import.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-export-default-anonymous-class.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/module-export-default-anonymous-class.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-export-default-anonymous-function.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/module-export-default-anonymous-function.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-export-default-expression.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/module-export-default-expression.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-export-from-compiles-to-nothing.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/module-export-from-compiles-to-nothing.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-hoisted-function.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/module-hoisted-function.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-import-bindings.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/module-import-bindings.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-lexical-and-var.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/module-lexical-and-var.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-this-is-undefined.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/module-this-is-undefined.js
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-28T03:55:06Z", actor: "human:operator", revision: 10, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-28T03:55:06Z", actor: "human:operator", revision: 11, executor: "agent:coordinator"}
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11). This story compiles the tree of a module that `story:module-syntax-and-early-errors` parses. On the base (`f962c324`) `swell-compiler` compiles a Script and eval code (`crates/compiler/src/lib.rs`: `compile_script` line 122, `compile_script_with` line 132, `compile_eval_with` line 152, `compile_eval` line 173) into a `Code` (`crates/compiler/src/bytecode.rs` line 1429) whose top level is GlobalDeclarationInstantiation's (`var_names`, `function_declarations`, the lexical names). A module's top level is its own environment, created by InitializeEnvironment (`#sec-source-text-module-record-initialize-environment`): every import binding, every `var`, function, lexical and class declaration, and `*default*` for `export default` of an expression, the functions instantiated before the body runs, `this` undefined.

It has no case of test262: a module runs only once `story:module-records-loading-and-module-runs` makes module records and `story:module-linking-and-evaluation` runs them. Its acceptance is the compiler's own tests (the disassembly of module sources) and the guard list of the syntax.

Design it follows: ECMA-262 16.2.1.7.3.1 (`#sec-source-text-module-record-initialize-environment`: steps 7 to 24, the bindings and their kinds: an import binding of a name is an indirect binding to the target module's binding, of a namespace an immutable binding initialized to the namespace; lexical bindings uninitialized; functions instantiated), `#sec-module-semantics-runtime-semantics-evaluation`, `#sec-exports-runtime-semantics-evaluation` (`export default` of an anonymous function or class gets the name `"default"`); and the paragraphs `**Modules.**` and `**Module records.**` of `engine.md` that `story:modules-design-text` writes. Decided there: the module's top level is a named environment of the form `Instruction::PushNamedEnv` makes (`Code::scopes`, line 1438), its slots in a fixed order (imports first, then the rest in source order); an import slot is a slot of a kind of its own, which InitializeEnvironment of `story:module-linking-and-evaluation` fills with the binding it resolves to, and the instructions that read and write a slot of a named environment (`GetEnv`, `GetEnvChecked` and `SetEnv`, `bytecode.rs` lines 120, 125 and 735; run by `crates/swell/src/interp/dispatch.rs` lines 426 and 596) read through it at run time; so no new instruction reads an import. This story writes the compile side only: the layout that marks a slot as an import; the kind of slot in `crates/swell/src/env/mod.rs` and the read through it are the linking story's.

## What later stories reuse

`compile_module` and the module part of `Code`: the slot layout of the module environment, the slot of each import entry and of `*default*`, the module requests in source order, the functions to instantiate, and the body's code. `story:module-records-loading-and-module-runs` compiles a module at `parse_module` (as `script.rs` compiles a script at parse time, `Compiled`) and reads the requests from the code; `story:module-linking-and-evaluation` fills the import slots and runs the body; `story:top-level-await` compiles an `await` at the top level once its hold is lifted.

## Domain relations

- A module's environment → its bindings: one to many, owned; an import binding → the binding of another module's environment it reads: many to one, set once at InitializeEnvironment and never changed — inferable (inferred from ECMA-262 `#sec-source-text-module-record-initialize-environment` and `#sec-createimportbinding`; no ess/1 document declares it: `spec/domains/host.yaml` holds no environment, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the sources and tests below are written before the code and fail on the base (no `compile_module`).

- `crates/compiler/src/lib.rs`: `pub fn compile_module(module: &Module) -> Result<Code, CompileError>` and `compile_module_with(module, ledger)` after `compile_eval` (line 173), in their form (the ledger charges what `compile_script_with` charges).
- `crates/compiler/src/bytecode.rs`: `Code` gets `module: Option<ModuleCode>`, `None` for a script and eval code: the module environment's slots (`SlotName` with a kind: import, import of a namespace, `var`, function, `let`, `const`, `class`, `*default*`), the module's requests (specifier and attributes, in the order of the tree's ModuleRequests), for each ImportEntry of the tree the slot it fills (its request by index, its import name or namespace), for each local export the slot it names, the indirect and star exports, and `function_declarations` as the functions InitializeEnvironment instantiates, and `async_slot: Option<u32>`, [[HasTLA]] (`Some` with the local slot of the module body's activation when the module has top-level `await`; this story sets `None` for every module, since a source with top-level `await` is held until `story:top-level-await`, which sets it). `crates/compiler/src/disasm.rs` prints it: the first line of a module's disassembly is `module locals <n> strict tla -` (`tla <slot>` for `Some`; beside `script locals <n>[ strict]`, line 42 on the base), then the slots, requests and entries, one line each, in the order of `ModuleCode`. The slot kind "import of a namespace" and the star export with a name (`export * as ns from`) are compiled here, but both constructs are held until `story:module-namespace-objects` and `parse_module` answers the refusal for them, so no source of this story shows them: they are first shown by that story's cases (its Acceptance names two).
- `crates/compiler/src/resolve.rs`: the module scope, new functions `resolve_module` and `resolve_module_with` after those of the script's scope (`resolve` and `resolve_with`, lines 551 to 699; before `impl Resolver`, line 701), the module's locals counted as `script_locals` is (line 652): every top-level name of the module resolves to its slot in the module environment (never to the global object); an import binding is immutable (an assignment compiles to the throw of a `TypeError` the code already emits for a `const`, `#sec-module-environment-records`); `this` at the top level is `undefined`; `arguments` at the top level is a plain identifier reference.
- `crates/compiler/src/statement.rs`: the module items: an ImportDeclaration compiles to nothing in the body; `export` of a declaration compiles as that declaration; `export default` of an AssignmentExpression stores into `*default*`, with SetFunctionName `"default"` for an anonymous function or class (`#sec-exports-runtime-semantics-evaluation`); `export default function` and `export default class` without a name bind `*default*` and name the function `"default"`; `export { ... }`, `export ... from` and `export * ...` compile to nothing.
- Tests: `crates/compiler/tests/compile.rs` compiles a source whose name starts with `module-` as a module (`parse_module`, then `compile_module`), as it compiles `eval-code-` sources (its module documentation, lines 1 to 8, says so); 9 new sources with their `.disasm` beside them in `crates/compiler/tests/sources/`: `module-import-bindings.js`, `module-export-default-expression.js`, `module-export-default-anonymous-function.js`, `module-export-default-anonymous-class.js`, `module-hoisted-function.js`, `module-lexical-and-var.js`, `module-this-is-undefined.js`, `module-assign-to-import.js`, `module-export-from-compiles-to-nothing.js`; and 3 new tests in `crates/compiler/tests/compile.rs`: `a_module_binds_nothing_on_the_global_object` (no `GetName` and no global instruction in the code of `var x; function f() {} x;`), `an_import_slot_comes_before_every_local_slot` and `export_default_names_an_anonymous_function_default`. No new test module.
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Report: the sum of the CPU seconds of the story's new tests in the default mode and in the mode `collect-every-safe-point` (each test run alone, user plus system), and the wall seconds of its part run when it has one; no story bounds the CPU seconds of a whole gate.
- Generated files: the generated `tests/main.rs` of every crate and `website/data/status.json` are in no story's scope. A unit that adds a test module writes `tests/main.rs` again with `sh taskfiles/test-main.sh write`; a unit that meets a conflict in a generated file takes either side and writes it again; the coordinator writes both again on the join (`sh taskfiles/test-main.sh write`, `slot --local task site-data`).
- Line numbers are those of `f962c324`; anchors are found by name.
- Size: about 700 lines (the compiler 300, the sources, their disassembly and the tests 400): one unit.

Shared with `story:import-call-and-import-meta-syntax`, the other story of order 2, each in its own hunks: `crates/compiler/src/resolve.rs` (this story: the module scope, new functions after the script's scope; that story: two arms beside `Expr::NewTarget`, line 1746) and `crates/compiler/src/expression.rs` (this story writes none of its matches; that story two arms beside line 342).

## Requirements

None of `conformance/host/requirements.txt` moves.

Writes: `crates/compiler/src/{lib,bytecode,disasm,resolve,statement}.rs`, `crates/compiler/tests/compile.rs`, `crates/compiler/tests/sources/module-*.js` and their `.disasm` (new, 9 pairs). Checked against the code on `f962c324`: `Code` is made by the compiler only (the `Code {` that `grep -rn 'Code {' crates/swell/src` finds, `dynamic_code.rs` lines 87 and 412 and `host/agent.rs` line 100, are variants of other enums); a new field breaks no reader. The readers of `Code::module` in `swell` come later and are those stories' own: `crates/swell/src/module/mod.rs` of `story:module-records-loading-and-module-runs` (the requests, at `parse_module`) and `crates/swell/src/module/link.rs` of `story:module-linking-and-evaluation` (the layout and the entries, at InitializeEnvironment); `async_slot` is read by `story:top-level-await` (`module/async_eval.rs`). Not written: `crates/syntax/` (orders 1 and 2), `crates/swell/` (the records and the run of a module, orders 3 and 4; `env/mod.rs` is the linking story's), `crates/test262/`, `conformance/`, `docs/design/`.

Order 2, beside `story:import-call-and-import-meta-syntax`; it depends on `story:module-syntax-and-early-errors` (the tree it compiles) and `story:modules-design-text` (the paragraphs it builds to).

## Acceptance

`cargo nextest run --locked -p swell-compiler --test main` exits 0 and `-E 'test(/^compile::/)'` runs 21 tests (18 on the base, `grep -c '#\[test\]' crates/compiler/tests/compile.rs` prints `18` there and `21` after); `ls crates/compiler/tests/sources/module-*.js | wc -l` and `ls crates/compiler/tests/sources/module-*.disasm | wc -l` each print `9` (`0` on the base), and `ls crates/compiler/tests/sources | wc -l` prints `286` (`268` on the base); `grep -l '^module locals [0-9]* strict tla -$' crates/compiler/tests/sources/module-*.disasm | wc -l` prints `9` (the [[HasTLA]] line of each); `cargo nextest run --locked -p swell-compiler --lib` exits 0; `cargo nextest run --locked -p swell --no-run` exits 0. `slot task clippy` exits 0.

test262. No case passes by this story; it shows the Script goal unchanged. In one pool job on the build node: `slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh docs/plan/slice-11/guard-syntax.cases.txt`; then on the workstation, with the base run of the join the unit starts from copied to `.out/test262-base/runs.tsv`, `sh docs/plan/slice-11/check.sh docs/plan/slice-11/guard-syntax.cases.txt .out/test262-base/runs.tsv .out/test262-result/runs.tsv` exits 0 and prints `passed 985 bound 985 B 985 F 0 R 0` (the 985 cases of the guard list, every one passing on the base and pulling in no case by `./`; the line is the same on `f962c324` and on the join).

## Out of scope

Module records, `parse_module` of the engine and loading (`story:module-records-loading-and-module-runs`); the import slot kind of the environment, filling the import slots, linking and running the body (`story:module-linking-and-evaluation`); top-level `await` (`story:top-level-await`); `import()` and `import.meta` (`story:dynamic-import`, `story:import-meta`).

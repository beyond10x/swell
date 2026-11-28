---
format: aep.planning-md/3
id: story:module-code-compiles
kind: story
status: draft
title: 'compile_module: the module environment''s slot layout, import slots, *default* and the functions InitializeEnvironment instantiates'
summary: 'swell_compiler::compile_module and Code::module: every top-level name in the module environment, import slots marked for InitializeEnvironment, export default to *default* with the name default, import and export-from compile to nothing; disassembly tests. Order 2; no case.'
relations:
- decomposes: epic:slice-11-modules
- depends_on: story:module-syntax-and-early-errors
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
  path: crates/compiler/tests/sources/module-assign-to-import.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-export-default-anonymous-class.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-export-default-anonymous-function.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-export-default-expression.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-export-from-compiles-to-nothing.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-hoisted-function.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-import-bindings.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-lexical-and-var.js
- confidence: inferred
  path: crates/compiler/tests/sources/module-this-is-undefined.js
revision: 3
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11). This story compiles the tree of a module that `story:module-syntax-and-early-errors` parses. On the base (`f962c324`) `swell-compiler` compiles a Script and eval code (`crates/compiler/src/lib.rs`: `compile_script` line 122, `compile_script_with` line 132, `compile_eval_with` line 152, `compile_eval` line 173) into a `Code` (`crates/compiler/src/bytecode.rs` line 1429) whose top level is GlobalDeclarationInstantiation's (`var_names`, `function_declarations`, the lexical names). A module's top level is its own environment, created by InitializeEnvironment (`#sec-source-text-module-record-initialize-environment`): every import binding, every `var`, function, lexical and class declaration, and `*default*` for `export default` of an expression, the functions instantiated before the body runs, `this` undefined.

It has no case of test262: a module runs only once `story:module-records-loading-and-module-runs` makes module records and `story:module-linking-and-evaluation` runs them. Its acceptance is the compiler's own tests (the disassembly of module sources) and the guard list of the syntax.

Design it follows: ECMA-262 16.2.1.7.3.1 (`#sec-source-text-module-record-initialize-environment`: steps 7 to 24, the bindings and their kinds: an import binding of a name is an indirect binding to the target module's binding, of a namespace an immutable binding initialized to the namespace; lexical bindings uninitialized; functions instantiated), `#sec-module-semantics-runtime-semantics-evaluation`, `#sec-exports-runtime-semantics-evaluation` (`export default` of an anonymous function or class gets the name `"default"`); and the design text "Modules" that `engine.md` gets before this story starts (Context of `story:module-records-loading-and-module-runs`): where a module's environment lives and how a read of an import binding reaches its target. The default this story builds to, should that text say nothing more: the module's top level is a named environment of the form `Instruction::PushNamedEnv` makes (`Code::scopes`, line 1438), its slots in a fixed order (imports first, then the rest in source order); an import slot is marked in the layout and filled by InitializeEnvironment of `story:module-linking-and-evaluation` with the binding it points to, and `GetNamed` and the instructions that write a named slot (`bytecode.rs` line 183 and those beside it) read through it, so no new instruction reads an import.

## What later stories reuse

`compile_module` and the module part of `Code`: the slot layout of the module environment, the slot of each import entry and of `*default*`, the functions to instantiate, and the body's code. `story:module-records-loading-and-module-runs` compiles a module at `parse_module` (as `script.rs` compiles a script at parse time, `Compiled`); `story:module-linking-and-evaluation` fills the import slots and runs the body; `story:top-level-await` compiles an `await` at the top level once its hold is lifted.

## Domain relations

- A module's environment → its bindings: one to many, owned; an import binding → the binding of another module's environment it reads: many to one, set once at InitializeEnvironment and never changed — inferable (inferred from ECMA-262 `#sec-source-text-module-record-initialize-environment` and `#sec-createimportbinding`; no ess/1 document declares it: `spec/domains/host.yaml` holds no environment, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the sources and tests below are written before the code and fail on the base (no `compile_module`).

- `crates/compiler/src/lib.rs`: `pub fn compile_module(module: &Module) -> Result<Code, CompileError>` and `compile_module_with(module, ledger)` after `compile_eval` (line 173), in their form (the ledger charges what `compile_script_with` charges).
- `crates/compiler/src/bytecode.rs`: `Code` gets `module: Option<ModuleCode>`, `None` for a script and eval code: the module environment's slots (`SlotName` with a kind: import, import of a namespace, `var`, function, `let`, `const`, `class`, `*default*`), for each ImportEntry of the tree the slot it fills (its ModuleRequest by index, its import name or namespace), for each local export the slot it names, and `function_declarations` as the functions InitializeEnvironment instantiates. `crates/compiler/src/disasm.rs` (line 396 on the base) prints it.
- `crates/compiler/src/resolve.rs`: the module scope, new functions after those of the script's scope: every top-level name of the module resolves to its slot in the module environment (never to the global object); an import binding is immutable (an assignment compiles to the throw of a `TypeError` the code already emits for a `const`, `#sec-object-environment-records-setmutablebinding-n-v-s` through the module record's binding: `#sec-module-environment-records`); `this` at the top level is `undefined`; `arguments` at the top level is a plain identifier reference.
- `crates/compiler/src/statement.rs`: the module items: an ImportDeclaration compiles to nothing in the body; `export` of a declaration compiles as that declaration; `export default` of an AssignmentExpression stores into `*default*`, with SetFunctionName `"default"` for an anonymous function or class (`#sec-exports-runtime-semantics-evaluation`); `export default function` and `export default class` without a name bind `*default*` and name the function `"default"`; `export { ... }`, `export ... from` and `export * ...` compile to nothing.
- Tests: `crates/compiler/tests/compile.rs` compiles a source whose name starts with `module-` as a module (`parse_module`, then `compile_module`), as it compiles `eval-code-` sources (its module documentation, lines 1 to 8, says so); new sources with their `.disasm` beside them in `crates/compiler/tests/sources/`: `module-import-bindings.js`, `module-export-default-expression.js`, `module-export-default-anonymous-function.js`, `module-export-default-anonymous-class.js`, `module-hoisted-function.js`, `module-lexical-and-var.js`, `module-this-is-undefined.js`, `module-assign-to-import.js`, `module-export-from-compiles-to-nothing.js`; and tests in `crates/compiler/tests/compile.rs`: `a_module_binds_nothing_on_the_global_object` (no `GetName` and no global instruction in the code of `var x; function f() {} x;`), `an_import_slot_comes_before_every_local_slot` and `export_default_names_an_anonymous_function_default`.
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Size: about 700 lines (the compiler 300, the sources, their disassembly and the tests 400): one session.

Shared with `story:import-call-and-import-meta-syntax`, the other story of order 2, each in its own hunks: `crates/compiler/src/resolve.rs` (this story: the module scope, new functions after the script's scope; that story: two arms beside `Expr::NewTarget`, line 1746) and `crates/compiler/src/expression.rs` (this story writes none of its matches; that story two arms beside line 342).

## Requirements

None of `conformance/host/requirements.txt` moves.

Writes: `crates/compiler/src/{lib,bytecode,disasm,resolve,statement}.rs`, `crates/compiler/tests/compile.rs`, `crates/compiler/tests/sources/module-*.js` and their `.disasm` (new). Checked against the code on `f962c324`: `Code` is made by the compiler only (the `Code {` that `grep -rn 'Code {' crates/swell/src` finds, `dynamic_code.rs` lines 87 and 412 and `host/agent.rs` line 100, are variants of other enums); a new field breaks no reader. Not written: `crates/syntax/` (orders 1 and 2), `crates/swell/` (the records and the run of a module, orders 3 and 4), `crates/test262/`, `conformance/`, `docs/design/`.

Order 2, beside `story:import-call-and-import-meta-syntax`; it depends on `story:module-syntax-and-early-errors` (the tree it compiles).

## Acceptance

`cargo nextest run --locked -p swell-compiler --test main` exits 0, with the 9 new sources compiled and compared and the 3 new tests (none of which exists on the base); `cargo nextest run --locked -p swell-compiler --lib` exits 0; `cargo nextest run --locked -p swell --no-run` exits 0. `ls crates/compiler/tests/sources/module-*.js | wc -l` prints `9` (`0` on the base) and every one of them has its `.disasm`. `slot task clippy` exits 0.

test262. No case passes by this story; it shows the Script goal unchanged. In one pool job on the build node: `slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh docs/plan/slice-11/guard-syntax.cases.txt`; then on the workstation, with the base run of the join the unit starts from copied to `.out/test262-base/runs.tsv`, `sh docs/plan/slice-11/check.sh docs/plan/slice-11/guard-syntax.cases.txt .out/test262-base/runs.tsv .out/test262-result/runs.tsv` exits 0 and prints `passed 985 bound 985 B 985 F 0 R 0` (the 985 cases of the guard list, every one passing over the base run of `f962c324`).

## Out of scope

Module records, `parse_module` of the engine and loading (`story:module-records-loading-and-module-runs`); filling the import slots, linking and running the body (`story:module-linking-and-evaluation`); top-level `await` (`story:top-level-await`); `import()` and `import.meta` (`story:dynamic-import`, `story:import-meta`).

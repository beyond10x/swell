---
format: aep.planning-md/3
id: story:module-namespace-objects
kind: story
status: draft
title: Module namespace exotic objects, import * as and export * as ns
summary: The namespace object kind with its eleven internal methods and sorted exports, GetModuleNamespace, namespace import slots and the NAMESPACE binding of ResolveExport; lifts the hold namespace import. Order 5; 70 cases.
relations:
- decomposes: epic:slice-11-modules
- depends_on: story:module-linking-and-evaluation
scope:
- confidence: cited
  path: crates/swell/src/heap/collect.rs
- confidence: inferred
  path: crates/swell/src/module.rs
- confidence: cited
  path: crates/swell/src/object/mod.rs
- confidence: inferred
  path: crates/swell/src/object/module_namespace.rs
- confidence: cited
  path: crates/swell/src/ops/objects.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: inferred
  path: crates/swell/tests/module_namespaces.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: inferred
  path: crates/syntax/src/parser/module.rs
- confidence: inferred
  path: crates/syntax/tests/modules.rs
revision: 3
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11; the `feature` lines `export-star-as-namespace-from-module` and `arbitrary-module-namespace-names` of the `swell-slices` block). This story brings module namespace exotic objects, `import * as ns` and `export * as ns from`, and lifts the hold `namespace import` that `story:module-syntax-and-early-errors` adds. After `story:module-linking-and-evaluation` a module graph links and runs, and a source with a namespace import is still answered by the held refusal.

The count. 70 cases (70 runs, all module runs) are this story's (`docs/plan/slice-11/story-module-namespace-objects.cases.txt`): 68 that run (the internal methods of a namespace object under `language/module-code/namespace/internals/`, `export * as` under `language/module-code/` and `language/export/`, string export names, `ns.__proto__` reading the namespace's [[Get]] of a name it does not export) and 2 with a `resolution` `SyntaxError`. Every one fails on the base on "a module run is not made before slice 11".

Design it follows: ECMA-262 10.4.6 (`#sec-module-namespace-exotic-objects`: [[Module]], [[Exports]] in the order of code units, the eleven internal methods, `%Symbol.toStringTag%` `"Module"`), `#sec-getmodulenamespace`, `#sec-modulenamespacecreate`, `#sec-resolveexport` (the binding name NAMESPACE for `export * as ns`), `#sec-source-text-module-record-initialize-environment` steps 7.b and 7.c (a namespace import and an `export * as` re-export); `engine.md` decision 4 ("Objects": an object kind and its arm in each internal method's dispatch, as `story:proxy-objects-and-access-traps` added `ObjectKind::Proxy`).

## What later stories reuse

GetModuleNamespace, which `story:dynamic-import` calls to fulfil the promise of `import()`, and which `story:json-modules` uses for a JSON module imported as a namespace.

## Domain relations

- Module → its namespace object: one to zero or one, made at the first GetModuleNamespace and kept in the module's field for the module's life; the namespace → its module: one to one, the namespace keeps its module alive — inferable (inferred from ECMA-262 `#sec-getmodulenamespace` step 3 and `#sec-module-namespace-exotic-objects`, slot [[Module]]; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `crates/swell/src/object/module_namespace.rs` (new; its `mod` line in `crates/swell/src/object/mod.rs`): the namespace object, its [[Exports]] (a sorted list of strings, charged to the memory budget, one step per name compared while it is sorted and per name searched by a lookup) and the eleven internal methods (`GetPrototypeOf` null, `SetPrototypeOf` as SetImmutablePrototype, `IsExtensible` false, `PreventExtensions` true, `GetOwnProperty` and `Get` through the target binding, a `ReferenceError` for an uninitialized one, `DefineOwnProperty` by the draft's steps, `HasProperty`, `Set` false, `Delete` false for an export, `OwnPropertyKeys` the exports then `%Symbol.toStringTag%`), and its `trace` (the module). `crates/swell/src/object/mod.rs`: `ObjectKind::ModuleNamespace` after `WeakCollection` (line 242 on the base), `KindTag::ModuleNamespace` after the last tag of `KindTag` (line 247), its arm in `trace` (line 285) and `tag` (line 318). The dispatch of each internal method (`crates/swell/src/ops/objects.rs`, beside each `ObjectKind::Proxy` arm): an arm after it. `crates/swell/src/heap/collect.rs`: `"object/module_namespace.rs ModuleNamespace::trace"` in `KIND_SITES` after `"object/weak_collection.rs WeakCollection::trace"` (line 499 on the base).
- `crates/swell/src/module.rs`: `get_module_namespace` (GetExportedNames of the records story, ResolveExport for each name, the unresolvable and ambiguous names left out, the object made once and kept in the record's field); InitializeEnvironment fills a namespace import slot with it and an import that ResolveExport answers with NAMESPACE with the target's namespace.
- `swell-syntax`: the hold `namespace import` goes: its `hold` calls in `crates/syntax/src/parser/module.rs` and its line of `HOLDS` (`crates/syntax/src/parser/mod.rs`). `swell-compiler`: the layout's namespace import slot, if `story:module-code-compiles` left it to this story.
- Tests in `crates/swell/tests/module_namespaces.rs` (new), each in both collection modes: `a_namespace_lists_its_exports_in_code_unit_order`, `a_namespace_is_not_extensible_and_has_a_null_prototype`, `a_namespace_read_of_an_uninitialized_binding_throws_a_reference_error`, `set_and_delete_of_an_export_answer_false`, `export_star_as_re_exports_the_namespace`, `a_string_export_name_is_a_key_of_the_namespace`, `get_module_namespace_makes_one_object_per_module`, `a_namespace_keeps_its_module_alive` (after the host drops the module's root, the namespace kept by a global still reads its exports after a collection), and `a_namespace_of_many_exports_ends_in_step_budget`. `crates/syntax/tests/modules.rs`: `a_namespace_import_is_held_after_its_early_errors` becomes `a_namespace_import_parses`. `crates/swell/tests/main.rs` generated again.
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Size: about 900 lines (the object and its methods 400, the record 100, the tests 400): one session.

Shared with the other stories of order 5 (`story:top-level-await`, `story:import-meta`, `story:json-modules`), each in its own hunks: `crates/swell/src/module.rs` (this story: `get_module_namespace` and the namespace branch of InitializeEnvironment; the others: their own functions; each adds after the last function of the linking story, in the order namespace, top-level `await`, `import.meta`, JSON); `crates/syntax/src/parser/mod.rs`, `HOLDS` (each deletes its own line); `crates/swell/tests/main.rs` (generated; each adds one module; the coordinator writes it again after the join); `conformance/host/requirements.txt` (this story moves no line). This story only: `crates/swell/src/object/`, `crates/swell/src/ops/objects.rs`, `KIND_SITES`.

## Requirements

None of `conformance/host/requirements.txt` moves.

Writes: `crates/swell/src/object/module_namespace.rs` (new), `crates/swell/src/object/mod.rs`, `crates/swell/src/ops/objects.rs`, `crates/swell/src/heap/collect.rs` (one line of `KIND_SITES`), `crates/swell/src/module.rs`, `crates/syntax/src/parser/{mod,module}.rs` (the hold), `crates/syntax/tests/modules.rs` (one test renamed), `crates/swell/tests/module_namespaces.rs` (new), `crates/swell/tests/main.rs` (generated). Checked against the code on `f962c324`: `ObjectKind` and `KindTag` are matched without a wildcard in `object/mod.rs` and the dispatch of `ops/objects.rs`, which the change lists; `KIND_SITES` is a slice. Not written: `crates/test262/`, `crates/swell/src/host/`, `spec/`, `docs/design/`.

Order 5, beside `story:top-level-await`, `story:import-meta` and `story:json-modules`; it depends on `story:module-linking-and-evaluation`.

## Acceptance

`cargo nextest run --locked -p swell --test main -E 'test(/^module_namespaces::/) | test(/^modules::/) | test(/^proxy::/) | test(/^builtins_reflect::/) | test(/^collect::/)'` exits 0 in both collection modes, with the 9 tests of `module_namespaces::` (none on the base); `cargo nextest run --locked -p swell-syntax --test main` and `cargo nextest run --locked -p swell --lib` exit 0. `grep -c '"namespace import"' crates/syntax/src/parser/mod.rs` prints `0` (`1` on the join the unit starts from). `slot task clippy` exits 0.

test262. The full run is not the unit's. In one pool job on the build node: `slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh docs/plan/slice-11/story-module-namespace-objects.cases.txt docs/plan/slice-11/guard-internals.cases.txt`; then on the workstation, with the base run of the join the unit starts from copied to `.out/test262-base/runs.tsv`, `sh docs/plan/slice-11/check.sh docs/plan/slice-11/story-module-namespace-objects.cases.txt .out/test262-base/runs.tsv .out/test262-result/runs.tsv` exits 0 and prints `passed <P> bound 834 B 764 F 70 R 0` with `<P>` at least 834 (over the base run of `f962c324` the part holds 844 cases: the 70 of the list, all failing there; the 764 of `guard-internals.cases.txt` (`Object.keys`, `getOwnPropertyNames`, `getOwnPropertyDescriptor`, `isFrozen`, `isSealed`, `Object.prototype.toString`, the `Reflect` functions of the internal methods, `for-in`), all passing, which the new object kind's arm in each dispatch must not change; and 10 cases of other lists that a file of the list names by `./`).

## Out of scope

`import()` and the namespace it fulfils with (`story:dynamic-import`); a JSON module's namespace (`story:json-modules`); `import defer` and deferred namespaces (stage 3, excluded).

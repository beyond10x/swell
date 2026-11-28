---
format: aep.planning-md/3
id: story:module-namespace-objects
kind: story
status: active
title: Module namespace exotic objects, import * as and export * as ns
summary: The namespace object kind with its eleven internal methods and sorted exports, GetModuleNamespace, namespace import slots and the NAMESPACE binding of ResolveExport; lifts the hold namespace import. Order 5; 70 cases.
relations:
- decomposes: epic:slice-11-modules
- depends_on: story:module-linking-and-evaluation
scope:
- confidence: cited
  path: crates/swell/src/heap/collect.rs
- confidence: inferred
  path: crates/swell/src/module/link.rs
- confidence: inferred
  path: crates/swell/src/module/namespace.rs
- confidence: cited
  path: crates/swell/src/object/array.rs
- confidence: cited
  path: crates/swell/src/object/mod.rs
- confidence: inferred
  path: crates/swell/src/object/module_namespace.rs
- confidence: cited
  path: crates/swell/src/object/ordinary.rs
- confidence: inferred
  path: crates/swell/tests/module_namespaces.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: inferred
  path: crates/syntax/src/parser/module.rs
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-28T03:55:07Z", actor: "human:operator", revision: 11, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-28T03:55:08Z", actor: "human:operator", revision: 12, executor: "agent:coordinator"}
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11; the `feature` lines `export-star-as-namespace-from-module` and `arbitrary-module-namespace-names` of the `swell-slices` block). This story brings module namespace exotic objects, `import * as ns` and `export * as ns from`, and lifts the hold `namespace import` that `story:module-syntax-and-early-errors` adds. After `story:module-linking-and-evaluation` a module graph links and runs, and a source with a namespace import is still answered by the held refusal.

The count. 70 cases (70 runs, all module runs) are this story's (`docs/plan/slice-11/story-module-namespace-objects.cases.txt`): 68 that run (the internal methods of a namespace object under `language/module-code/namespace/internals/`, `export * as` under `language/module-code/` and `language/export/`, string export names, `ns.__proto__` reading the namespace's [[Get]] of a name it does not export) and 2 with a `resolution` `SyntaxError`. Every one fails on the base on "a module run is not made before slice 11".

Design it follows: ECMA-262 10.4.6 (`#sec-module-namespace-exotic-objects`: [[Module]], [[Exports]] in the order of code units, the eleven internal methods, `%Symbol.toStringTag%` `"Module"`), `#sec-getmodulenamespace`, `#sec-modulenamespacecreate`, `#sec-resolveexport` (the binding name NAMESPACE for `export * as ns`), `#sec-source-text-module-record-initialize-environment` steps 7.b and 7.c (a namespace import and an `export * as` re-export); `engine.md` decision 4 ("Objects": an object kind and its arm in each internal method's dispatch, as `story:proxy-objects-and-access-traps` added `ObjectKind::Proxy`), and the paragraph `**Module records.**` (the state of each part in a file of its own).

## What later stories reuse

GetModuleNamespace (`crates/swell/src/module/namespace.rs`), which `story:dynamic-import` calls to fulfil the promise of `import()` (and with it the namespace of a JSON module, the case `json-via-namespace.js` of that story's list).

## Domain relations

- Module → its namespace object: one to zero or one, made at the first GetModuleNamespace and kept in the module's `NamespaceState` for the module's life; the namespace → its module: one to one, the namespace keeps its module alive — inferable (inferred from ECMA-262 `#sec-getmodulenamespace` step 3 and `#sec-module-namespace-exotic-objects`, slot [[Module]]; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `crates/swell/src/object/module_namespace.rs` (new; its `mod` line in `crates/swell/src/object/mod.rs`): the namespace object, its [[Exports]] (a sorted list of strings, charged to the memory budget, one step per name compared while it is sorted and per name searched by a lookup) and the eleven internal methods (`GetPrototypeOf` null, `SetPrototypeOf` as SetImmutablePrototype, `IsExtensible` false, `PreventExtensions` true, `GetOwnProperty` and `Get` through the target binding, a `ReferenceError` for an uninitialized one, `DefineOwnProperty` by the draft's steps, `HasProperty`, `Set` false, `Delete` false for an export, `OwnPropertyKeys` the exports then `%Symbol.toStringTag%`), and its `trace` (the module). `crates/swell/src/object/mod.rs`: `ObjectKind::ModuleNamespace` after `WeakCollection` (line 242 on the base), `KindTag::ModuleNamespace` after the last tag of `KindTag` (line 247), its arm in `trace` (line 285) and `tag` (line 318). The dispatch of each internal method is in `object/mod.rs`, the eleven functions from `get_prototype_of` (line 445) to `own_property_keys` (line 583), each a `match self.kind_of(object)` that ends in a wildcard to the ordinary method: an arm `KindTag::ModuleNamespace` beside each `KindTag::Proxy` arm (a missing arm would compile, so each is read by a test below). `crates/swell/src/object/array.rs` lines 326 to 355 (the fast path of an array read that walks the prototype chain; `ObjectKind` matched without a wildcard): `ObjectKind::ModuleNamespace(_)` joins the arms that answer `false` (an exotic [[GetOwnProperty]]: an export may be named by an index string). `crates/swell/src/object/ordinary.rs` lines 332 to 337, `walk_hands_over_to`, which ends in `_ => Ok(false)`: the arm `KindTag::ModuleNamespace => Ok(true)`, so that a walk of a prototype chain that reaches a namespace hands over to its own [[HasProperty]], [[Get]] or [[Set]]; the walks that reach it are the three ordinary ones, `has_property_walk` (line 378, from `ordinary_has_property`), `get_walk` (line 415, from `ordinary_get`) and `set_walk` (line 470, from `ordinary_set`). `crates/swell/src/heap/collect.rs`: `"object/module_namespace.rs ModuleNamespace::trace"` in `KIND_SITES` after `"object/weak_collection.rs WeakCollection::trace"` (line 499 on the base), and in the test `mark_phase_calls_every_site_of_the_layout` (line 513), which makes one object of every kind, an entry `ObjectKind::ModuleNamespace(ModuleNamespace::for_test(..))` in its array `kinds` (line 545), with the constructor `for_test` in `module_namespace.rs`, as `Promise::for_test` is (`object/promise.rs` line 242).
- `crates/swell/src/module/namespace.rs` (the file the records story created, with `NamespaceState` empty): `NamespaceState` holds the namespace object once made, with its trace; `get_module_namespace` (GetExportedNames of the linking story, ResolveExport for each name, the unresolvable and ambiguous names left out, the object made once and kept). `crates/swell/src/module/link.rs`, one hunk: the namespace branch of InitializeEnvironment (a namespace import slot filled with the target's namespace, and an import that ResolveExport answers with NAMESPACE the same); no other story of order 5 writes `link.rs`.
- `swell-syntax`: the hold `namespace import` goes: its two `hold` calls in `crates/syntax/src/parser/module.rs` (at `import * as` and at `export * as ns from`) and its line of `HOLDS` (`crates/syntax/src/parser/mod.rs`, the first line; the attribute `#[rustfmt::skip]` above the array stays). The held sources `module-namespace-import.js` and `module-export-star-as.js` of order 1 then parse as valid modules do, by the test as order 1 left it; this story writes no file of `crates/syntax/tests/`. `swell-compiler`: none (the namespace import slot is in the layout of `story:module-code-compiles`).
- Tests in a new module `crates/swell/tests/module_namespaces.rs`, 10, each in both collection modes: `a_namespace_lists_its_exports_in_code_unit_order`, `a_namespace_is_not_extensible_and_has_a_null_prototype`, `a_namespace_read_of_an_uninitialized_binding_throws_a_reference_error`, `set_and_delete_of_an_export_answer_false`, `export_star_as_re_exports_the_namespace`, `a_string_export_name_is_a_key_of_the_namespace`, `get_module_namespace_makes_one_object_per_module`, `a_namespace_keeps_its_module_alive` (after the host drops the module's root, the namespace kept by a global still reads its exports after a collection), `a_namespace_of_many_exports_ends_in_step_budget`, and `a_namespace_in_a_prototype_chain_reads_and_refuses` (for `o = Object.create(ns)`: `o.x` reads the export `x` through the chain, `"x" in o` is true, and `Reflect.set(o, "x", 1)` answers false and leaves `o` with no own `x`, the namespace's [[Set]] refusing).
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Report: the sum of the CPU seconds of the story's new tests in the default mode and in the mode `collect-every-safe-point` (each test run alone, user plus system), and the wall seconds of its part run when it has one; no story bounds the CPU seconds of a whole gate.
- Generated files: the generated `tests/main.rs` of every crate and `website/data/status.json` are in no story's scope. A unit that adds a test module writes `tests/main.rs` again with `sh taskfiles/test-main.sh write`; a unit that meets a conflict in a generated file takes either side and writes it again; the coordinator writes both again on the join (`sh taskfiles/test-main.sh write`, `slot --local task site-data`).
- Line numbers are those of `f962c324`; anchors are found by name.
- Size: about 900 lines (the object and its methods 400, the record 100, the tests 400): one unit.

Shared with the other stories of order 5 (`story:top-level-await`, `story:import-meta`, `story:json-modules`): `crates/syntax/src/parser/mod.rs`, `HOLDS`: this story deletes the first line and `story:top-level-await` the second, two adjacent lines, which git answers as a conflict when both branches merge: the branch merged second takes both deletions (the array is then `dynamic import`, `import.meta`, or `dynamic import` alone once `story:import-meta` merged too), and nothing else of the file moves; `story:import-meta` deletes the fourth line, which touches neither. Nothing else: each story of order 5 writes its own file of `crates/swell/src/module/`; `crates/syntax/src/parser/module.rs`, `crates/swell/src/object/` (`mod.rs`, `module_namespace.rs`, `array.rs`, `ordinary.rs`), `heap/collect.rs` and `module/link.rs` are this story's alone in order 5 (`story:top-level-await` writes `object/builtin_function.rs` and `object/async_builtin.rs`, other files of the directory).

## Requirements

None of `conformance/host/requirements.txt` moves.

Writes: `crates/swell/src/object/module_namespace.rs` (new), `crates/swell/src/object/{mod,array,ordinary}.rs` (the arms above), `crates/swell/src/heap/collect.rs` (one line of `KIND_SITES` and one entry of the test's `kinds`), `crates/swell/src/module/{namespace,link}.rs` (the hunks above), `crates/syntax/src/parser/{mod,module}.rs` (the hold), `crates/swell/tests/module_namespaces.rs` (new). Checked against the code on `f962c324` (`grep -rn 'ObjectKind::Proxy\|KindTag::Proxy' crates/swell/src`): `ObjectKind` is matched without a wildcard in `object/mod.rs` (`trace`, line 285, and `tag`, line 318) and `object/array.rs` (lines 326 to 355); every match of `KindTag` (the dispatch in `object/mod.rs`, `walk_hands_over_to`, `ops/testing.rs` line 52) and the other matches of `ObjectKind` (`ops/objects.rs` line 605, `object/host_overrides.rs` line 325) end in a wildcard, and the change lists the ones a namespace must not fall into; `KIND_SITES` is a slice. Shared with a story outside the slice: `object/ordinary.rs` with `bigint-constructor-and-prototype` of slice 12, whose edge the coordinator sets. Not written: `crates/syntax/tests/`, `crates/test262/`, `crates/swell/src/host/`, `spec/`, `docs/design/`.

Order 5, beside `story:top-level-await`, `story:import-meta` and `story:json-modules`; it depends on `story:module-linking-and-evaluation`.

## Acceptance

`cargo nextest run --locked -p swell --test main -E 'test(/^module_namespaces::/)'` runs 10 tests (none on the base: the module does not exist) and exits 0 in both collection modes; `cargo nextest run --locked -p swell --test main -E 'test(/^modules::/) | test(/^proxy::/) | test(/^builtins_reflect::/) | test(/^collect::/)'` exits 0 in both collection modes; `cargo nextest run --locked -p swell-syntax --test main` and `cargo nextest run --locked -p swell --lib` exit 0. `grep -c '"namespace import"' crates/syntax/src/parser/mod.rs` prints `0` (`1` on the join the unit starts from); `git grep -n 'hold("namespace import"' -- crates/syntax/src` prints nothing. `slot task clippy` exits 0.

test262. The full run is not the unit's. In one pool job on the build node: `slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh docs/plan/slice-11/story-module-namespace-objects.cases.txt docs/plan/slice-11/guard-internals.cases.txt`; then on the workstation, with the base run of the join the unit starts from copied to `.out/test262-base/runs.tsv`, `sh docs/plan/slice-11/check.sh docs/plan/slice-11/story-module-namespace-objects.cases.txt .out/test262-base/runs.tsv .out/test262-result/runs.tsv` exits 0 and prints `passed <P> bound 843 B 773 F 70 R 0` with `<P>` at least 843. Two cases of the list first show what `story:module-code-compiles` compiles and parse held until this story: `test/language/module-code/instn-star-binding.js` (`import * as ns from './instn-star-binding.js'`, the slot kind "import of a namespace") and `test/language/module-code/export-star-as-dflt.js` (`export * as default from './export-star-as-dflt_FIXTURE.js'`, the star export with a name, read back through `import Self`, `{ default as named }` and `import * as ns`); both fail on the base and pass here. The formula: `B` counts every case of the part that passes on the base run read, and the bound is `B + F - R`; the part holds 844 cases: the 70 of the list, all failing; the 764 of `guard-internals.cases.txt` (`Object.keys`, `getOwnPropertyNames`, `getOwnPropertyDescriptor`, `isFrozen`, `isSealed`, `Object.prototype.toString`, the `Reflect` functions of the internal methods, `for-in`), all passing, which the new object kind's arm in each dispatch must not change; and 10 cases of other lists that a file of the list names by `./`: 9 of `story:module-linking-and-evaluation` (`instn-iee-*`), which pass on this story's join since their owner landed and so count in `B` (764 + 9 = 773), and 1 of `story:dynamic-import` (`verify-dfs.js`), whose owner lands later. Over the base run of `f962c324` the same part reads `bound 834 B 764`.

## Out of scope

`import()` and the namespace it fulfils with, among them a JSON module's (`story:dynamic-import`); the synthetic kind's exports (`story:json-modules`); `import defer` and deferred namespaces (stage 3, excluded).

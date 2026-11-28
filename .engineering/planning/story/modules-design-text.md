---
format: aep.planning-md/3
id: story:modules-design-text
kind: story
status: draft
title: 'The design text of modules: the paragraphs Modules and Module records of engine.md and HOST-224, written by the design author'
summary: engine.md section 3 Modules (walks on work stacks and their charge, the module frame, ScriptOrModule of two kinds, the LoadRequest payload), section 4 Module records (arena, Item::Module, per-part state files, import slot kind), section 11 the runner's loader; HOST-224 in host-interface.md and its pending line. Order 1; written by the design author; no case.
relations:
- decomposes: epic:slice-11-modules
scope:
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: docs/design/engine.md
- confidence: cited
  path: docs/design/host-interface.md
revision: 3
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`). The host interface states the surface of modules (`docs/design/host-interface.md` section 9, "Modules, and the load hand-over"), but `docs/design/engine.md` says nothing of how the engine stores, walks and runs a module: decisions 3 ("Execution model") and 4 ("Values and the heap without unsafe code") have no paragraph on a module record, its environment, its frame or the walks of the draft that recurse (InnerModuleLoading, InnerModuleLinking, InnerModuleEvaluation, ResolveExport, GetExportedNames). And the entry "Slices 2, 9 and 11" of `docs/plan/waves.md`, "What later slices take over", asks for a requirement on a stale or foreign handle returned by `host_get_import_meta_properties`, which `host-interface.md` does not state yet. This story is the design change: it is written by the design author, not by a unit, and the stories that build on it depend on it.

The questions the decomposition of the slice left open were decided when its drafts were revised; the text states each as decided:

- Where a module record lives: a new arena `modules` beside `scripts` (`crates/swell/src/heap/mod.rs`), with its item kind `Item::Module` in the collector and its entry in `KIND_SITES`.
- How a read of an import binding reaches its target: the module environment is a declarative record whose import slots are a new slot kind; InitializeEnvironment fills an import slot with the binding it resolves to (the target module's environment and slot), and every read of a named slot goes through it: the compiled named-slot instructions and the lookup by name at run time that code compiled for a direct `eval` uses (`crates/compiler/src/resolve.rs` lines 48 and 49, "in eval code, a reference that the eval code does not bind is looked up by name"; `Agent::resolve_name`, `crates/swell/src/env/mod.rs` line 640), which is why the binding must read through rather than be copied.
- How the walks run: on explicit work stacks of the agent, charged to the memory budget, so that the Rust stack does not grow with the graph.
- What a walk charges: one step per module visited and one per request or exported name read, charged before each.
- A module body: a frame of its own, whose ScriptOrModule component is the module; with top-level `await` it is suspended and resumed as an `async` function's activation is. The ScriptOrModule component of frames, function objects, environments' slot names and queued jobs is one handle of two kinds, a script or a module.
- The runner's loader: `./name` is resolved against the directory of the referring module's file within the case's directory, one module per resolved path per run, every load finished inside the hook.
- The state of each later part of the record (its link and evaluation state, its namespace, its asynchronous evaluation, its `import.meta` object, a JSON module's synthetic default export) is a type of its own in a file of its own under `crates/swell/src/module/`, called by the record's trace; and the payload of a `LoadRequest` is a sum of the two kinds the draft has (a GraphLoadingState Record for `load_requested_modules`, a PromiseCapability Record for `import()`), by which `finish_loading_imported_module` chooses its continuation.
- A new requirement: HOST-224, the next free number.

Line numbers in this story are those of `f962c324`; anchors are found by name.

## What later stories reuse

The two paragraphs and HOST-224: `story:module-code-compiles`, `story:module-records-loading-and-module-runs`, `story:module-linking-and-evaluation`, `story:top-level-await` and `story:import-meta` cite them.

## Domain relations

- None new: the relations the paragraphs rest on are those the stories cite (`spec/domains/host.yaml`, `swell.host.Module` and `swell.host.ModuleLoad`; ECMA-262 16.2.1).

## Change

- `docs/design/engine.md`, section 3, a paragraph that starts `**Modules.** Added on <date> for slice 11.`, after the paragraph `**Private names.**` (line 1999): what a host sees (`host-interface.md` section 9); the walks on explicit work stacks and their charge; the module body's frame, the ScriptOrModule component as a handle of two kinds, and top-level `await`; the `LoadRequest` payload and the finish's choice; a termination of each walk (decision 10, HOST-74 to HOST-76).
- `docs/design/engine.md`, section 4, a paragraph that starts `**Module records.**`, after `**Template objects.**` (line 2419): the arena and the item kind; the record's fields, and the state of each later part in a type and file of its own (`crates/swell/src/module/`); the module environment's import slot kind and why a read goes through it; what each is charged to the memory budget.
- `docs/design/engine.md`, section 11, the bullet "Modules" (line 3561): the loader's resolution and the one module per path.
- `docs/design/host-interface.md`, section 9, after HOST-68 (line 975): `**HOST-224** (swell:D6) A \`host_get_import_meta_properties\` whose \`Ok\` vector holds a key or a value with a stale handle or a handle of another agent ends the evaluation of \`import.meta\` with \`Terminated(EngineFault)\`, and \`state()\` then reads \`Faulted\`. [slice 11]`, in the form of HOST-160, HOST-162 and HOST-222; and `conformance/host/requirements.txt` the line `HOST-224 pending slice 11` after line 223 (`host-requirements-check` fails for a requirement of the document with no line).
- Rules: this story writes no code and no test. A check that builds the whole workspace (`host-requirements-check` builds the list of the workspace's tests) goes through the pool as `slot --timeout 30m task ...`, as the earlier slices write the gate; a pool job ends after 10 minutes unless it names its time with `slot --timeout`, at most 30 minutes; a script that only measures or waits starts without the prefix. The stories that build on this text keep the bounds every story carries: a single test at most 60 s, the gate's test steps at most 120 s, no `unsafe`, no third-party crate, no panic, no Python in anything committed, one integration-test binary per crate, an expensive product made once.

Writes: `docs/design/engine.md` (sections 3, 4 and 11 as listed), `docs/design/host-interface.md` (section 9, one requirement), `conformance/host/requirements.txt` (one line). Not written: every source file, `spec/`, `docs/plan/`. No story of slice 11 writes `docs/design/` before the close (`story:slice-11-conformance-close` writes section 12 of `engine.md` only).

Order 1, beside `story:module-syntax-and-early-errors`; it depends on no story. Written by the design author.

## Acceptance

On the commit of this story: `grep -c '^\*\*Modules\.\*\*' docs/design/engine.md` prints `1`, `grep -c '^\*\*Module records\.\*\*' docs/design/engine.md` prints `1`, `grep -c '^\*\*HOST-224\*\*' docs/design/host-interface.md` prints `1` and `grep -c '^HOST-224 pending slice 11$' conformance/host/requirements.txt` prints `1` (each `0` on `f962c324`); `grep -c 'pending slice 11$' conformance/host/requirements.txt` prints `33` (`32` on `f962c324`). The names each paragraph must carry are counted inside the paragraph's own lines, not in the whole file: a paragraph of `engine.md` is a run of lines without a blank one, and `awk '/^\*\*Modules\.\*\*/ { p = 1 } p && /^$/ { exit } p' docs/design/engine.md > .out/modules-paragraph.txt` cuts the first out (`/^\*\*Module records\.\*\*/` the second, into `.out/module-records-paragraph.txt`). In `.out/modules-paragraph.txt`, `grep -c 'work stack'`, `grep -c 'ScriptOrModule'`, `grep -c 'LoadPayload'`, `grep -c 'LoadPayload::Dynamic'`, `grep -c 'top-level .await.'` and `grep -c 'HOST-7[456]'` each print at least `1`; in `.out/module-records-paragraph.txt`, `grep -c 'Item::Module'`, `grep -c 'KIND_SITES'`, `grep -c 'crates/swell/src/module/'`, `grep -c 'SlotKind::Import'` and `grep -c 'memory budget'` each print at least `1`; and in the bullet "Modules" of section 11, cut by `awk '/^- \*\*Modules\.\*\*/ { p = 1; print; next } p && /^(- |$)/ { exit } p' docs/design/engine.md` (two lines on `f962c324`), `grep -c 'one module per'` and `grep -c 'referring'` each print at least `1` (each of these greps prints `0` on `f962c324`, where neither paragraph exists and the bullet names neither). The `.out/` files are scratch, not committed; `slot --timeout 30m task -d . -t taskfiles/host-requirements.yml host-requirements-check` exits 0; `git log --no-merges --first-parent --format= --name-only "$BASE"..HEAD` prints only the three files of Writes (`BASE` the join the author starts from).

## Out of scope

Section 12 of `engine.md` (the close); the code of every part (the stories of orders 1 to 7); the ESS model (`spec/` holds the module commands already).

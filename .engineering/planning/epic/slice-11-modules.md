---
format: aep.planning-md/3
id: epic:slice-11-modules
kind: epic
status: draft
title: 'Slice 11, modules: module records, loader hook, dynamic import'
summary: 'Module records, the loader hook, dynamic import, import.meta and JSON modules: 1,277 cases and 1,825 runs.'
relations:
- depends_on: epic:slice-10-promises-and-async
revision: 1
---
## Context

Slice 11 of 19, `modules` (`docs/design/engine.md`, section 12). It needs the slices before it in the
design's order of dependencies; the design states no reason for the position beyond that order.
Top-level `await` (a `feature` line of this slice) and dynamic `import` return promises, which
slice 10 builds. It adds module records, the loader hook, dynamic `import`, `import.meta` and JSON
modules. After slice 11 most present-day scripts with no DOM can run (section 12, "For surf").
Governing decisions (`docs/design/engine.md`, decision N): 9 (hooks, asynchronous loading), 10 (the
ESS model of modules), 11 (the runner's loader), 3, 13.

## Outcome

- Module records, entry points `parse_module`, `load_requested_modules`, `link`, `evaluate_module`
  (decision 9), with a module's [[Status]] as the draft has it: `new`, `unlinked`, `linking`,
  `linked`, `evaluating`, `evaluating-async`, `evaluated` (decision 10).
- Module loading is asynchronous as the draft has it: `HostLoadImportedModule` returns at once, and
  the host later calls `finish_loading_imported_module` with the module's source or an error.
  In the ESS model, `HostLoadImportedModule` publishes `ModuleLoadRequested` and the host ends it
  with `FinishLoadingImportedModule` (decision 10).
- Dynamic `import`, `import.meta` (`HostGetImportMetaProperties`, `HostFinalizeImportMeta`),
  import attributes (`HostGetSupportedImportAttributes`), JSON modules, top-level `await`,
  `export * as ns from`, and arbitrary module namespace names (the `feature` lines below).
- The entity `swell.host.Module` and the commands `LoadRequestedModules`, `LinkModule`,
  `EvaluateModule`, `FinishLoadingImportedModule` of the ESS model, stated before their code
  (decision 10).
- The runner's module loader over the case's directory: `./name` is the file beside the case,
  `.json` is a JSON module, `_FIXTURE` files are loaded only as dependencies; `negative: phase:
  resolution` means loading or linking must fail with the named error (decision 11).

### Open, to be settled by this epic's first story

- Whether the ESS model of the foundation already holds the module commands and events, or this
  epic adds them (decision 10 names them; `AGENTS.md` puts the host interface specification in the
  foundation).
- How ESS states the asynchronous loading as a library interface: "Not checked: how well ESS states
  an in-process library interface" (decision 10).
- Which of the 39 features with no `swell-slices` line fall inside this slice.

## Counts

- Slice 11 (`modules`): 1,277 cases, 1,825 runs (section 12 table). The runs are fewer than twice
  the cases because a `module` case runs once (design, "Inventory of test262").
- Directories the design names for it (paths under `test/`, cases): `expressions/dynamic-import` 616,
  `module-code` 595. Inventory, in all slices: the flag `module` is carried by 843 cases,
  `dynamic-import` by 1,011; 34 cases are `resolution SyntaxError` negatives.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/language/module-code/ 11`
  - `dir test/language/import/ 11`
  - `dir test/language/export/ 11`
  - `dir test/language/expressions/dynamic-import/ 11`
  - `dir test/language/expressions/import.meta/ 11`
  - `feature dynamic-import 11`
  - `feature import-attributes 11`
  - `feature json-modules 11`
  - `feature top-level-await 11`
  - `feature import.meta 11`
  - `feature export-star-as-namespace-from-module 11`
  - `feature arbitrary-module-namespace-names 11`
  - `flag module 11`

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- `source-phase-imports`, `source-phase-imports-module-source`, `import-defer`, `export-defer`,
  `import-text`, `import-bytes`: stage 3, excluded (the `exclude` lines). 385 cases under
  `expressions/dynamic-import` are among the stage 3 cases and are counted in `x`, not here.
- A real module resolver (URLs, the network, files): the host's; the engine only calls the hook
  (decision 9). The command-line host and the runner provide their own loaders.
- The HTML Standard's module scripts and import maps: surf's.
- `Temporal` (slice 20), ECMA-402 (slice 21), other stage 3 features, concurrent agents.

## Constraints

- No ambient authority: the engine opens no file and reaches no network; module loading comes from
  the host interface (`AGENTS.md`, "Rules"; decision 9).
- Spec first: the host-interface parts of modules are stated as an ESS model under `spec/` before
  their code (`AGENTS.md`, "Rules"; decision 10).
- Nothing third-party in what ships; `#![forbid(unsafe_code)]`; no panics, no `unwrap`, `expect` or
  unchecked indexing (`AGENTS.md`, "Rules").
- Every loop (graph traversal, linking, evaluation) is charged to the step budget, every
  allocation to the memory budget; a new limit gets a test that reaches it (`AGENTS.md`, "Rules").
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 11 reads 1,277 cases and 1,825 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. The conformance suite of the ESS model holds a passed scenario for the module commands and events it states, run over the thin adapter to the Rust interface (decision 10).

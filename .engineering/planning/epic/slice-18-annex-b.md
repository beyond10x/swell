---
format: aep.planning-md/3
id: epic:slice-18-annex-b
kind: epic
status: draft
title: 'Slice 18, annex-b: Annex B and IsHTMLDDA'
summary: 'Annex B and the IsHTMLDDA slot: 1,155 cases and 1,492 runs.'
relations:
- depends_on: epic:slice-17-tail-calls
revision: 1
---
## Context

Slice 18 of 19, `annex-b` (`docs/design/engine.md`, section 12): Annex B and [[IsHTMLDDA]]. It
follows slices 1 to 17 in the design's order of dependencies; the design states no reason for the
position. In scope because ECMA-262 clause 2 says "Web browsers are generally required to implement
all normative optional subclauses. (See Annex B.)" (decision 1). After slice 18 a script on a real
page becomes realistic on swell's side, and only once surf has the DOM, Web IDL bindings and event
loop of decision 9 (section 12, "For surf"). Governing decisions (`docs/design/engine.md`,
decision N): 1 (scope), 7 (Annex B pattern grammar, B.1.2), 9 (`document.all`), 13.

## Outcome

- Annex B, the normative optional subclauses of ECMA-262, by the `dir test/annexB/ 18` line:
  `annexB/language` and `annexB/built-ins` are the directories the design names.
- [[IsHTMLDDA]] (B.3.6): the engine supports the slot, and the runner provides `$262.IsHTMLDDA`
  (42 test files use it). `document.all` itself is a host object of surf's (HTML 2.6.4.1,
  decision 9).
- The features `__proto__`, `__getter__`, `__setter__`, `caller` and `IsHTMLDDA` (the `feature`
  lines below).

### Open, to be settled by this epic's first story

- How the Annex B regular expression pattern grammar (B.1.2) divides between slice 5, where
  decision 7 puts it in `swell-regexp`, and this slice, which section 12 names "Annex B".
- Why the cases of this slice are 1,155 where `test/annexB/` holds 1,086 and the two directories
  the design names add up to fewer: the difference is cases elsewhere carrying the features above.
  The design gives no breakdown (section 1: "Annex B, 1,155 cases").
- Which Annex B behaviours earlier slices already built, and so which of this slice's cases pass
  before it starts.

## Counts

- Slice 18 (`annex-b`): 1,155 cases, 1,492 runs (section 12 table).
- Directories the design names for it (paths under `test/`, cases): `annexB/language` 845,
  `annexB/built-ins` 215. Inventory: `test/annexB/` holds 1,086 cases; `IsHTMLDDA` is carried by 42.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/annexB/ 18`
  - `feature __proto__ 18`
  - `feature __getter__ 18`
  - `feature __setter__ 18`
  - `feature caller 18`
  - `feature IsHTMLDDA 18`
  - no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- `legacy-regexp` (`RegExp.$1` and friends): stage 3, excluded (decision 7, `exclude` line).
- `document.all` and the other host objects with [[IsHTMLDDA]], Web IDL legacy platform objects,
  `WindowProxy`: surf's (decision 9).
- The DOM, Web IDL bindings and the HTML event loop: surf's; "none of which exists or is planned in
  surf's current 22 waves" (section 12, "For surf").
- `Temporal` (slice 20), ECMA-402 (slice 21), other stage 3 features, concurrent agents.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`; no panics, no `unwrap`, `expect` or
  unchecked indexing (`AGENTS.md`, "Rules").
- The [[IsHTMLDDA]] slot must not become a way to reach a host object with a value it should have
  refused: brand checks stay in the interface (decision 13, host objects; `AGENTS.md`, "Rules").
- Every loop charged to the step budget, every allocation to the memory budget; a new limit gets a
  test that reaches it (`AGENTS.md`, "Rules").
- A builtin's module and comments cite its clause, Annex B clauses included (`AGENTS.md`, "Rules").
- A deliberate omission is recorded where the gate reads it (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 18 reads 1,155 cases and 1,492 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. The runner provides `$262.IsHTMLDDA`, and the cases that use it (42 files, design "Inventory of test262") and are counted in this slice pass; none is counted as failed.

---
format: aep.planning-md/3
id: epic:slice-09-collections-proxy-reflect
kind: epic
status: draft
title: 'Slice 9, collections-proxy-reflect: Map, Set, Proxy, Reflect'
summary: 'Map, Set, WeakMap, WeakSet, Proxy and Reflect: 1,785 cases and 3,541 runs.'
revision: 3
---
## Context

Slice 9 of 19, `collections-proxy-reflect` (`docs/design/engine.md`, section 12). The design
orders it after slices 1 to 8 and states no reason for the order beyond that (iteration is slice
6's, and the collections are iterable). The wave plan builds it before slice 8 (`class`): the
close of this slice moves the cases that need `class` or `super` by rule lines
(`story:slice-09-conformance-close`: 37 cases), and stories of slice 8 depend on stories of this
slice, so this epic has no edge to slice 8's. It builds `Map`, `Set`, `WeakMap`,
`WeakSet`, `Proxy` and `Reflect`. Governing decisions (`docs/design/engine.md`, decision N): 4 (keyed
hash in hash maps, collector), 3 (a `Proxy` trap chain is a re-entry), 9 (host objects override
the same internal methods as a `Proxy`), 13.

## Outcome

- `Map`, `Set`, `WeakMap`, `WeakSet`, their iterator prototypes (`MapIteratorPrototype`,
  `SetIteratorPrototype`), `Proxy` and `Reflect` (section 12, item 9).
- Features by the `feature` lines below: `set-methods`, `upsert`, `symbols-as-weakmap-keys`,
  `proxy-missing-checks`, `Reflect.construct`, `Reflect.set`, `Reflect.setPrototypeOf`.
- Hash maps keyed by a SipHash written in `swell-num` and keyed from the host's randomness at agent
  creation, so a hostile script cannot choose colliding keys (decision 4).
- Every `Proxy` trap that calls script is a re-entry, counted toward the frame depth bound and the
  re-entry limit (decision 3).
- The invariants of 6.1.7.3 that a `Proxy` trap result must satisfy (the same invariants the engine
  checks on a host object's override, decision 9).

### Open when the epic was written, and where each was settled

- How `WeakMap` and `WeakSet` entries are cleared by the collector. Decision 4 says weak references
  are cleared after marking (9.9) and names `WeakRef` and `FinalizationRegistry`; it does not name
  the collection of `WeakMap` and `WeakSet` entries, which this slice builds before slice 16.
  Settled in `story:weak-map-and-weak-set`: ephemeron marking, and the entries are cleared in
  one pass after marking.
- Where the SipHash and the host-seeded key first exist (decision 4). Settled before this slice:
  `crates/num/src/siphash.rs`, keyed by the host's `hash_key` (HOST-145), is what the engine's
  hash map (`crates/swell/src/heap/map.rs`) uses.
- Which of the 39 features with no `swell-slices` line fall inside this slice. Settled in
  `story:slice-09-conformance-close`: a case is counted where the block's lines put it, and the
  close reports the features of the slice's cases that have no line.

## Counts

- Slice 9 (`collections-proxy-reflect`): 1,785 cases, 3,541 runs (section 12 table, `docs/design/engine.md:3019`; the gate on `main` at `62c7cf59` prints `test262-slice 09 1785 3541 0 0 1785 267 1518 0`). The epic first said 1,744 and 3,459, the table before cases of earlier slices moved in.
- Directories the design names for it (paths under `test/`, cases): `Set` 381, `Proxy` 308, `Map` 202.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/built-ins/Map/ 9`
  - `dir test/built-ins/Set/ 9`
  - `dir test/built-ins/WeakMap/ 9`
  - `dir test/built-ins/WeakSet/ 9`
  - `dir test/built-ins/MapIteratorPrototype/ 9`
  - `dir test/built-ins/SetIteratorPrototype/ 9`
  - `dir test/built-ins/Proxy/ 9`
  - `dir test/built-ins/Reflect/ 9`
  - `feature Map 9`
  - `feature Set 9`
  - `feature WeakMap 9`
  - `feature WeakSet 9`
  - `feature Proxy 9`
  - `feature Reflect 9`
  - `feature Reflect.construct 9`
  - `feature Reflect.set 9`
  - `feature Reflect.setPrototypeOf 9`
  - `feature proxy-missing-checks 9`
  - `feature set-methods 9`
  - `feature upsert 9`
  - `feature symbols-as-weakmap-keys 9`
  - no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- `WeakRef`, `FinalizationRegistry`, `$262.gc`: slice 16.
- `Promise` and the async builtins: slice 10. `Iterator` helpers: slice 15.
- Web IDL legacy platform objects, `WindowProxy`, `document.all` and the Web IDL bindings that use
  the table of overrides: surf's (decision 9, "Not in scope"). The table itself, its dispatch and
  the checks of what an override answers are in this slice (`story:host-object-overrides`; HOST-41,
  HOST-112, HOST-113 and HOST-114 are pending for slice 9, and the story adds HOST-222).
- `Temporal` (slice 20), ECMA-402 (slice 21), stage 3 features, concurrent agents.

## Constraints

- Nothing third-party in what ships, and no `std` `RandomState` in the engine: the keyed hash is
  `swell-num`'s (`AGENTS.md`, "Rules"; decision 4). `#![forbid(unsafe_code)]`.
- No panics, no `unwrap`, `expect` or unchecked indexing (`AGENTS.md`, "Rules").
- Every collection loop is charged to the step budget per element and every growth to the memory
  budget; every recursion through a `Proxy` is bounded; a new limit gets a test that reaches it
  (`AGENTS.md`, "Rules"; decision 3).
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 9 reads the cases and runs of the section 12 table after the moves of the close (1,785 cases and 3,541 runs before them), and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. A chain of `Proxy` objects, each with a trap that calls script, nested deeper than the re-entry limit ends in a `RangeError` the script can catch (decision 3: "each step that calls script is a re-entry, bounded as one"), with no Rust stack overflow.

---
format: aep.planning-md/3
id: epic:slice-13-binary-data
kind: epic
status: draft
title: 'Slice 13, binary-data: ArrayBuffer, typed arrays, DataView'
summary: 'ArrayBuffer, typed arrays, DataView, resizable buffers and base64: 3,042 cases and 6,040 runs.'
relations:
- depends_on: epic:slice-12-bigint
revision: 1
---
## Context

Slice 13 of 19, `binary-data` (`docs/design/engine.md`, section 12). It needs slice 12: "13 needs 12 for
`BigInt64Array`". It needs the earlier slices in the design's order of dependencies; the design
gives no other reason. It builds `ArrayBuffer`, typed arrays, `DataView`, resizable buffers and
base64. Governing decisions (`docs/design/engine.md`, decision N): 4 (memory budget, `try_reserve`,
`RangeError` for buffers), 3 (a step unit per element), 9 (`HostResizeArrayBuffer`), 13 (hostile
sizes).

## Outcome

- `ArrayBuffer`, the typed arrays (`Int8Array`, `Uint8Array`, `Uint8ClampedArray`, `Int16Array`,
  `Uint16Array`, `Int32Array`, `Uint32Array`, `Float16Array`, `Float32Array`, `Float64Array`, and
  the `BigInt` arrays through slice 12), `TypedArray`, `TypedArrayConstructors` and `DataView`
  with its accessors (section 12, item 13; the `dir` and `feature` lines below).
- Resizable buffers (`resizable-arraybuffer`) through `HostResizeArrayBuffer`, `ArrayBuffer`
  transfer (`arraybuffer-transfer`), and base64 for `Uint8Array` (`uint8array-base64`).
- Detached buffers per `align-detached-buffer-semantics-with-web-reality`, and `$262.detachArrayBuffer`
  in the runner (6 test files use it, most through `detachArrayBuffer.js`).
- A stable `TypedArray` sort (`stable-typedarray-sort`).
- Memory (decision 4): buffer growth uses `Vec::try_reserve`, so a refusal by the allocator becomes
  an outcome instead of an abort; over the budget, `new ArrayBuffer` and the typed array
  constructors throw a `RangeError`, as `CreateByteDataBlock` allows.
- A step unit per element for typed array loops (decision 3).

### Open, to be settled by this epic's first story

- The outcome of a resize or a transfer of a buffer that goes over the memory budget. Decision 4
  names the constructors (`RangeError`) and says any other allocation ends the run with
  `Terminated(MemoryBudget)`; the design does not say which applies to a resize.
- Which of the 39 features with no `swell-slices` line fall inside this slice.

## Counts

- Slice 13 (`binary-data`): 3,042 cases, 6,040 runs (section 12 table).
- Directories the design names for it (paths under `test/`, cases): `TypedArray` 1,438,
  `TypedArrayConstructors` 680, `DataView` 510. Inventory, in all slices: `built-ins/TypedArray`
  1,453 cases; the feature `TypedArray` is carried by 2,537.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/built-ins/ArrayBuffer/ 13`
  - `dir test/built-ins/TypedArray/ 13`
  - `dir test/built-ins/TypedArrayConstructors/ 13`
  - `dir test/built-ins/DataView/ 13`
  - `dir test/built-ins/Uint8Array/ 13`
  - `feature TypedArray 13`
  - `feature ArrayBuffer 13`
  - `feature DataView 13`
  - `feature DataView.prototype.getFloat32 13`
  - `feature DataView.prototype.getFloat64 13`
  - `feature DataView.prototype.getInt16 13`
  - `feature DataView.prototype.getInt32 13`
  - `feature DataView.prototype.getInt8 13`
  - `feature DataView.prototype.getUint16 13`
  - `feature DataView.prototype.getUint32 13`
  - `feature DataView.prototype.setUint8 13`
  - `feature Int8Array 13`
  - `feature Int16Array 13`
  - `feature Int32Array 13`
  - `feature Uint8Array 13`
  - `feature Uint8ClampedArray 13`
  - `feature Uint16Array 13`
  - `feature Uint32Array 13`
  - `feature Float16Array 13`
  - `feature Float32Array 13`
  - `feature Float64Array 13`
  - `feature resizable-arraybuffer 13`
  - `feature arraybuffer-transfer 13`
  - `feature uint8array-base64 13`
  - `feature TypedArray.prototype.at 13`
  - `feature align-detached-buffer-semantics-with-web-reality 13`
  - `feature stable-typedarray-sort 13`
  - no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- `SharedArrayBuffer` and `Atomics`: slice 14.
- `immutable-arraybuffer`: stage 3, excluded (the `exclude` line; the pinned test262 commit is itself
  "[immutable-arraybuffer] TypedArray internal operations").
- Typed arrays over shared memory shared between agents, and `$262.agent`: out until surf needs
  workers (open item B).
- `Atomics.waitAsync`: slice 14.
- Web IDL buffer-source conversions: surf's binding layer (decision 9).
- `Temporal` (slice 20), ECMA-402 (slice 21), other stage 3 features.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`: a buffer is a safe `Vec`, with no
  raw memory (`AGENTS.md`, "Rules"; decision 13).
- No panics, no `unwrap`, `expect` or unchecked indexing in byte access (`AGENTS.md`, "Rules").
- Every element loop is charged to the step budget and every buffer allocation and growth to the
  memory budget; a new limit gets a test that reaches it (`AGENTS.md`, "Rules").
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 13 reads 3,042 cases and 6,040 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. `new ArrayBuffer(n)` with n over the agent's memory budget throws a `RangeError`, and a typed array constructor asked for a length over it throws a `RangeError`; neither is a panic or an abort (decisions 4 and 13).
6. A buffer that the allocator refuses through `Vec::try_reserve` ends in an outcome the host reads, not in an abort (decision 4).

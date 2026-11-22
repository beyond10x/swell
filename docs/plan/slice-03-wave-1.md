# Slice 3, wave 1 of 3: a file per group of builtins, an index map for properties

One wave of the [wave plan](waves.md), 2026-11-21 and 2026-11-22, on the branch `wave/s3-1`.
Two units side by side, each implemented by a sub-agent on Opus in its own worktree, both from
`main` at `6a43e18`. Both stories record `crates/swell/src/object/mod.rs`; they ran side by
side by decision, each naming its lines of that file ([wave plan](waves.md), "Slice 3"). Both
units started before their stories were on `main` and read them from their scratch
directories (`fb0053a`). Every commit is the organization bot's. Next:
[wave 2](slice-03-wave-2.md), [wave 3](slice-03-wave-3.md); beside the waves:
[the gate and the site](slice-03-gate-and-site.md).

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `builtin-group-modules` | The builtins of `Object`, `Array` and `String` move from one file each into a directory: `mod.rs` keeps what exists, and each group of wave 2 has a file of its own that starts empty (`array/{iteration,search,mutation}.rs`, `object/{integrity,enumeration}.rs`, `string/{access,transform}.rs`): an id enum with no variant, the lists `CONSTRUCTOR_METHODS` and `PROTOTYPE_METHODS`, a run function and `prototype_symbol_data`. `mod.rs` defines the groups' methods from their lists, so a story of wave 2 writes its group file and no `mod.rs`. The operations two stories of wave 2 need, each with its clause: SetIntegrityLevel, TestIntegrityLevel, Invoke, EnumerableOwnProperties with its three kinds, CreateDataPropertyOrThrow, CodePointAt, UTF16EncodeCodePoint, the this-string steps, the well-known symbols `@@isConcatSpreadable` and `@@match`. An integrity operation charges two steps per key and the key list to the memory budget. The helpers of the stack tests move to `tests/support/stack.rs` | 17 new library tests (34 to 51), green in both collection modes; the 85,556 run lines of the unit's run equal those of `main` | `e0b4629` (merge `862c55b`) |
| `property-index-and-dense-elements` | The object model of the design, decision 4. Up to 8 properties an object keeps its list and scans it; above that a `KeyedMap`, keyed by the agent's hash key and nothing else (HOST-145), holds each property's position. A removal leaves an empty slot so that the order stays; a full list with half its slots empty is compacted, a step per entry. An array element that is a writable, enumerable, configurable data property at an index at most 1,024 past the length is stored densely, with holes; anything else goes to the list and stays there. The bindings a non-strict direct `eval` adds use the same list and map; the names of a record's slots stay a scan, charged a step per comparison past the eighth. The room of the map, the list and the dense storage is charged before it is made and credited when the object is freed | 11 new library tests, 5 in `tests/property_index.rs`, both collection modes; the 85,556 run lines of the unit's run equal those of `main`; no pinned step count of an existing test moved | `4bce0a1` (merge `ec1653f`) |

Each unit's claim was run again by the coordinator before its commit. For
`builtin-group-modules`: the library tests and seven test binaries in both collection modes,
clippy, and the 85,556 test262 run lines against `main` (`e868ada`). For
`property-index-and-dense-elements`: the unit's test binaries in both collection modes, clippy,
and its test262 run lines against those of `main` (`b7d7b67`).

**What works now.** No builtin a script can reach is added and no run line changes. The time
of a lookup no longer grows with the size of the object. Measured by the unit with the release
runner, the minimum of three rounds, the time at 40,000 elements over the time at 10,000 (the
story's bound is 8): appending objects 3.2 (8.9 on `main`), appending numbers 5.4 (18.2),
writing and reading by index 4.0 (30.2), by name 4.7 (16.3). Appending 40,000 numbers takes
190 ms (4,231 ms on `main`). The probes are `crates/swell/tests/probes/property-index/`, run by
hand with `run.sh`. A third agent with another hash key shows that steps and heap bytes do not
depend on the seed.

**No run was lost.** Each unit compared its 85,556 run lines with those of `main`: equal. The
record does not have a run of the wave branch with the two merged: `expected.txt` was not
changed on the branch before the close of the slice ([wave 3](slice-03-wave-3.md)).

## What went wrong, and what was decided

- **Planning held code back before the wave.** On 2026-11-21 no code reached `main` between
  20:29 and the start of this wave's units at 22:25: the thirteen stories went through two
  rounds of the plan critics and three revisions in a row. `AGENTS.md`, "How the work is run",
  now says that a story's unit starts as soon as no open correction of its review blocks it,
  with the remaining corrections in its brief, a design author starts when the decomposition
  names its change, and review, design and a gate repair run side by side (`fb0053a`).
- **A file outside a `Writes:` line.** `Heap` keeps the hasher of the index map (one field in
  `heap/mod.rs`); the Writes line of `property-index-and-dense-elements` was extended for it.
- **Decisions the units took, kept.** `Array.prototype.map` now reports a property it cannot
  define with the shared message of CreateDataPropertyOrThrow. The mark on
  `prevent_extensions` is removed; the marks on the operations no builtin calls yet name the
  stories of wave 2 that will. The keys of an object are sorted only when indices were added
  out of order, a step per comparison. Truncation by `length` is charged before any element is
  deleted, so a step budget that ends it leaves the array whole.
- **The tool's waves.** `aep plan artifact waves` prints four waves for the thirteen stories:
  it puts these two one after the other because both record `object/mod.rs` (the wave plan).

## Where each finding went

| finding | where it went |
|---|---|
| The index map of the design is not built and the scans of a property list and of the bindings of a direct `eval` are not charged ([wave plan](waves.md), "What later slices take over") | this wave: `property-index-and-dense-elements` |
| Operations no builtin calls yet, each with a mark naming its story | wave 2: the units removed the marks with their first callers ([wave 2](slice-03-wave-2.md)) |
| The design text wave 2 builds against (a counted refusal of a String method before slice 5, `HOST-165`; source text kept with its function and charged) | the design, `8d5828d`, merged with `fad5240` before this wave's first merge |

## The gate

The gate ran once for the branch, after its last unit: its lines are in
[the gate and the site, "The gate"](slice-03-gate-and-site.md#the-gate).

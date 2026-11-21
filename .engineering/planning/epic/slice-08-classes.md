---
format: aep.planning-md/3
id: epic:slice-08-classes
kind: epic
status: draft
title: 'Slice 8, classes: classes, super, private names, static blocks'
summary: 'Classes, super, new.target, private names and static blocks: 6,188 cases and 12,286 runs.'
relations:
- depends_on: epic:slice-07-generators
revision: 1
---
## Context

Slice 8 of 19, `classes` (`docs/design/engine.md`, section 12). It needs slices 1 to 7 in the design's
order of dependencies: slices 6 to 8 are "the syntax most later tests are written in", and 4,916
cases carry the feature `class`. The design states no reason for 8 after 7 beyond that order. It
adds classes, `super`, `new.target`, private names and static blocks. Slice 10 later raises the
async class cases. Governing decisions (`docs/design/engine.md`, decision N): 2 (parser, compiler,
one module per clause), 3 (frames), 9 (`HostEnsureCanAddPrivateElement`), 13 (private names).

## Outcome

- Classes, `super`, `new.target`, private names and static blocks (section 12, item 8; the `dir`
  and `feature` lines below).
- Fields: public, private, static public and static private; private methods and static private
  methods; `#x in obj`; static initialization blocks (the `feature` lines below).
- Private names that cannot be reached from outside their class (decision 13, logic errors in the
  object model).
- The host hook `HostEnsureCanAddPrivateElement` (decisions 9 and 1: the HTML Standard defines its
  own version of it).
- The parser's early errors for class bodies (decision 2).

### Open, to be settled by this epic's first story

- The default behaviour of `HostEnsureCanAddPrivateElement` in the trait, and what the runner's host
  answers: decision 9 gives the hook's name and says the trait carries ECMA-262's default where the
  draft gives one.
- How private names are stored in the heap (decision 4 describes plain property maps and keys; it
  does not describe private names).
- Which of the 39 features with no `swell-slices` line fall inside this slice.

## Counts

- Slice 8 (`classes`): 6,188 cases, 12,286 runs (section 12 table).
- Directories the design names for it (paths under `test/`, cases): `statements/class` 3,016,
  `expressions/class` 2,731. Inventory, in all slices: `statements/class` 4,419 and
  `expressions/class` 4,109; the feature `class` is carried by 4,916 cases.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/language/expressions/class/ 8`
  - `dir test/language/statements/class/ 8`
  - `dir test/language/expressions/super/ 8`
  - `dir test/language/expressions/new.target/ 8`
  - `feature class 8`
  - `feature class-fields-public 8`
  - `feature class-fields-private 8`
  - `feature class-fields-private-in 8`
  - `feature class-methods-private 8`
  - `feature class-static-methods-private 8`
  - `feature class-static-fields-public 8`
  - `feature class-static-fields-private 8`
  - `feature class-static-block 8`
  - `feature super 8`
  - `feature new.target 8`
  - no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- Class cases that use `async`, async generators or `for await`: counted in slice 10 (`statements/class`
  1,348 and `expressions/class` 1,341 are the cases that slice raises).
- `decorators`: stage 3, excluded. `nonextensible-applies-to-private`: stage 3, excluded (the
  `exclude` lines).
- `Temporal` (slice 20), ECMA-402 (slice 21), other stage 3 features, concurrent agents.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`; no panics, no `unwrap`, `expect` or
  unchecked indexing (`AGENTS.md`, "Rules").
- A private name must not be reachable from outside its class: a logic error here is a security
  fault of the object model (decision 13; `AGENTS.md`, "Rules").
- Every loop (field initialisation, static blocks) is charged to the step budget and every
  allocation to the memory budget; a new limit gets a test that reaches it (`AGENTS.md`, "Rules").
- A deliberate omission is recorded where the gate reads it (`features.txt`, `sections.txt`;
  `AGENTS.md`, "Rules").
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 8 reads 6,188 cases and 12,286 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.

---
format: aep.planning-md/3
id: epic:slice-08-classes
kind: epic
status: draft
title: 'Slice 8, classes: classes, super, private names, static blocks'
summary: 'Classes, super, new.target, private names and static blocks: 9,302 cases and 18,455 runs; 9,330 and 18,511 after the close.'
relations:
- depends_on: epic:slice-07-generators
revision: 3
---
## Context

Slice 8 of 19, `classes` (`docs/design/engine.md`, section 12). It needs slices 1 to 7 in the design's
order of dependencies: slices 6 to 8 are "the syntax most later tests are written in", and 4,916
cases carry the feature `class`. The design states no reason for 8 after 7 beyond that order. It
adds classes, `super`, `new.target`, private names and static blocks. Slice 10 (`async`) is
delivered, so the class cases that use `async`, async generators or `for await` are counted here
and no longer wait for it. The wave plan builds this slice after slice 9
(`collections-proxy-reflect`): three stories of this slice depend on stories of that one
(`docs/plan/waves.md`, "Slice 8"), so this epic's stories carry the edges and the epic has none
to slice 9's. Governing decisions (`docs/design/engine.md`, decision N): 2 (parser, compiler, one
module per clause), 3 (frames; its paragraphs "Classes" and "Private names" and the arrow
paragraph's items 4 and 5 were written for this slice before its stories were revised), 9
(`HostEnsureCanAddPrivateElement`), 13 (private names).

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

### The three questions this epic left open, and where each was settled

- The default of `HostEnsureCanAddPrivateElement` in the trait and what the runner's host answers:
  the requirement HOST-223, proposed in `story:host-ensure-can-add-private-element` and written
  into `docs/design/host-interface.md` by that story's first commit (HOST-119 is the hook's
  requirement that is pending for the slice today).
- How private names are stored in the heap: `docs/design/engine.md`, section 3, "Private names"
  (a Private Name is a handle into an arena of its own and no value; the private environment is a
  record of the environment chain).
- Which of the 39 features with no `swell-slices` line fall inside this slice: answered by the
  block as it stands. A case is counted in the slice its rule lines give, and
  `docs/plan/slice-08/lists.sh` shows that the stories' lists hold each of the slice's cases once.

## Counts

- Slice 8 (`classes`): 9,302 cases, 18,455 runs (section 12 table; `08 9302 18455` in the output
  under "How the counts were made").
- On the plan's base, the full run of 2026-11-26: 5 of the 9,302 cases pass
  (`test262-slice 08 9302 18455 0 0 9302 5 9297 0`).
- The largest directories of the slice in that run (paths under `test/`, cases):
  `language/statements/class` 4,378, `language/expressions/class` 4,076,
  `language/expressions/super` 94.
- Lines of the `swell-slices` block that assign cases to this slice: 5 lines for a directory, 11
  `feature` lines, and 312 lines `dir <file> 8` that the closes of earlier slices wrote for cases
  they moved here.
  - `dir test/language/computed-property-names/class/ 8`
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
- What the close changes, on the plan's numbers (`story:slice-08-conformance-close`,
  `docs/plan/slice-08/`): 37 cases come in from slice 9's close (`incoming.tsv`), 5 cases move to
  slice 13 and 4 to slice 18 (`close-later.tsv`), which gives 9,330 cases and 18,511 runs. The
  numbers are measured again by the close.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- The cases of the slice that need a part of a later slice: the subclasses of `ArrayBuffer` and
  `DataView` (slice 13, 5 cases) and the cases that call `__lookupGetter__` or `__lookupSetter__`
  (slice 18, 4 cases). The close moves them by rule lines with their reason.
- `Map`, `Set`, `WeakMap`, `WeakSet`, `Proxy` and `Reflect` themselves (slice 9). The 16 cases
  that subclass a collection are on the list of the close (`close-slice-9.cases.txt`) and pass
  once both slices are built.
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

1. The conformance runner's line for slice 8 reads the cases and runs the design's table states after the close (9,330 cases and 18,511 runs on the plan's numbers), and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.

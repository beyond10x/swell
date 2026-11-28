---
format: aep.planning-md/3
id: review-result:slice-11-revision-2-coordinator-read
kind: review-result
status: active
title: 'Slice 11, revision 2 read by the coordinator: approve, no finding'
relations:
- reviews: story:cli-module-loader
- reviews: story:dynamic-import
- reviews: story:ess-target-module-commands
- reviews: story:import-call-and-import-meta-syntax
- reviews: story:import-meta
- reviews: story:json-modules
- reviews: story:module-code-compiles
- reviews: story:module-determinism-and-emulated-loading
- reviews: story:module-linking-and-evaluation
- reviews: story:module-namespace-objects
- reviews: story:module-records-loading-and-module-runs
- reviews: story:modules-design-text
- reviews: story:module-syntax-and-early-errors
- reviews: story:slice-11-conformance-close
- reviews: story:top-level-await
revision: 1
---
The coordinator's reading of revision 2 of the fifteen draft stories of `epic:slice-11-modules`
(`plan/slice-11`, the diff of fifteen story files from `e1eaf39a` to `60205da6`), 2026-11-28.
It takes the place of a third round of the plan critics: two rounds ran, as two adversary
passes run on a unit, and the last revision is read by the coordinator.

Verdict: approve. Each of the 23 findings of round 2 is answered in the text, and the places
in the code the new text rests on were read in the tree at `60205da6`.

- Design 1 and 4 (`top-level-await`, `dynamic-import`): each story adds its own kind and its
  own slots to `crates/swell/src/object/builtin_function.rs` (`enum BuiltinId` at line 57,
  `enum Slots` at line 251) and names the other files that match `Slots`; the edge from
  `dynamic-import` to `top-level-await` orders the two. `dynamic-import` names three
  closures, as ContinueDynamicImport makes three.
- Design 2, parallel-safety 3 (`module-code-compiles`, `top-level-await`): the field of
  `ModuleCode` for the activation slot is declared in order 2 and printed by the
  disassembler; `top-level-await` writes `resolve.rs` and `lib.rs` and neither `bytecode.rs`
  nor `disasm.rs`. Its hunk of `resolve.rs` lies beside line 699 and that of `import-meta`
  lies beside line 1746.
- Design 3 (`module-records-loading-and-module-runs`, `dynamic-import`): the finish
  dispatches on the kind of the referrer, and the arms of a script and a realm answer an
  engine fault until `dynamic-import` fills them.
- Design 5, parallel-safety 2 (`module-linking-and-evaluation`): every match of `SlotKind`
  and `SlotValue` without a wildcard is listed with its file and line. The sentence that let
  a unit make "the smallest change" in a file no story names is gone from all fifteen bodies
  (`grep -c 'smallest change'` prints 0 for each).
- Design 7: [[TopLevelCapability]] is the linking story's; a second `evaluate_module` answers
  the stored promise, and one test reads that it is the same object.
- Parallel-safety 1 and 6 (`module-namespace-objects`): `object/array.rs` (the match on
  `record.kind` from line 326), the `kinds` array of the marking test, and `object/ordinary.rs`:
  `walk_hands_over_to` (line 332) hands over for `KindTag::Proxy` and `KindTag::Host` and ends
  in a wildcard, and it is called from the three walks at lines 390, 425 and 488. The story
  adds the arm with a test of `Object.create` over a namespace.
- Parallel-safety 4 (`json-modules`): `json_parse` (`builtins/json/parse.rs` line 198) is
  `pub(super)` and takes the arguments of a call; the story adds an entry of its own there.
- Scope 1 to 4, acceptance 1, parallel-safety 5 (`cli-module-loader`): the hook of the shell
  (`crates/cli/src/shell.rs` line 59) keeps every request today; the story keeps a dynamic
  request and one with an attribute, names the exit status of each case with a test each
  (16 tests on the base, 28 after), reads the referring file from the module's own
  `host_defined`, and has `website/docs/embedding.md` in scope (the usage lines at 228 and
  229, the table of flags to line 250). The rewrite of the epic's bullet and the entry of
  `docs/plan/waves.md` are on the close's list for the coordinator.
- Acceptance 2 and 3, design 6, parallel-safety 7 and 9: as the critics asked.
- `HOLDS` (`crates/syntax/src/parser/mod.rs` line 67) is empty on the base and carries no
  `#[rustfmt::skip]`; the syntax story adds the attribute with its lines.
- No body speaks of a session; every size sentence ends "one unit".

`aep plan artifact validate`: `valid`. `sh docs/plan/slice-11/lists.sh` over the base run:
`slice 1281 lists 1281 incoming 0`. `aep plan artifact waves` compares paths and prints
eleven waves for the fifteen stories; the plan's eight orders hold by the hunks each story
names in the files it shares with a story of its order.

Not read in the code by the coordinator: the count of matches on `Slots` without a wildcard
in the six further files the revision names; a unit that meets one more names it in its
report.

The records of the two rounds carry the critics' text with four changes of form made by the
coordinator: the labels of the decomposer's questions read Q1 to Q10 in two reviews of round
1, one review of round 2 names "the owner" where it named a person, and the findings block of
the parallel-safety review of round 2 carries the store's words for verdict and origin.

Left for the join of the two slices, the coordinator's: the edges to the stories of slice 12
(the table at the end of the parallel-safety review of round 2, and the files the revision
added: `object/builtin_function.rs`, `object/ordinary.rs`, `builtins/json/parse.rs`), the
edges to drafts of the backlog that write the same files, and the edge of
`ess-target-module-commands` to the draft that writes the same three files.

```findings
[]
```

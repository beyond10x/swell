# Wave big-3: slice 11, modules, and slice 12, BigInt

One wave of [the wave plan](waves.md), "After slice 4: bigger waves". Proposed on 2026-11-28 and accepted
by the operator the same day (`approval-record:wave-big-3`). Skill version 0.26.0 of
`aep:implementing`. The 23 stories below are active; no unit is dispatched before the lane
unit named under "Before the wave" is merged.

The two epics follow the design's order: `epic:slice-11-modules`, then
`epic:slice-12-bigint`, which depends on it (4 of its cases call `import()` with a second
argument). Both were decomposed on 2026-11-28 on the code of `f962c324`, each read by the
four plan critics in two rounds and, after the last revision, by the coordinator
(`review-result:slice-11-round-1-acceptance` to
`review-result:slice-11-revision-2-coordinator-read`, and the same nine for slice 12).

## Before the wave

`story:slow-lane-holds-its-bound-on-the-slower-runner`, the eighth unit of the lane of wave
big-2 (`approval-record:wave-big-2-lane-unit-8`), is built before any feature unit of this
wave: the lane `slow` read 1123.4 s of its 900 on one of the forge's runner models
([wave big-2](wave-big-2.md), the fourth part, "The forge"). The plan of this wave travels
with that unit's pull request. The wave branch `wave/big-3` is cut from `main` after that
pull request is merged.

## The stories

On the base, 23 of the 1,281 cases of slice 11 pass (1,833 runs) and 39 of the 381 cases of
slice 12 (760 runs); 32 host requirements are pending for slice 11, and the design text adds
one, and 3 for slice 12. The order below follows the stories' edges, with the edges between
the two slices set on the join of the two plan branches: `swell-num-bigint-core` after
`modules-design-text` (both write `docs/design/engine.md`), `bigint-values-and-literals`
after `dynamic-import` (the last story of slice 11 that writes the parser, the compiler, the
interpreter's dispatch and the heap, which the values story writes too), and the close of
slice 12 after the close of slice 11. The two stories of `swell-num` share no engine file
with slice 11 and are built beside its first orders.

Stories of one order are built at the same time, each in a managed worktree of its own; each
names its hunk in every file it shares with a story of its order ([the plan page](waves.md),
"Slice 11" and "Slice 12").

| order | story | slice | cases | runs |
|---|---|---|---|---|
| 0 | `modules-design-text` (the design author's, before the first unit) | 11 | 0 | 0 |
| 1 | `module-syntax-and-early-errors` | 11 | 3 | 6 |
| 1 | `swell-num-bigint-core` | 12 | 0 | 0 |
| 2 | `import-call-and-import-meta-syntax` | 11 | 152 | 291 |
| 2 | `module-code-compiles` | 11 | 0 | 0 |
| 2 | `swell-num-bigint-powers-shifts-and-bits` | 12 | 0 | 0 |
| 3 | `module-records-loading-and-module-runs` | 11 | 193 | 193 |
| 4 | `module-linking-and-evaluation` | 11 | 122 | 122 |
| 5 | `module-namespace-objects` | 11 | 70 | 70 |
| 5 | `top-level-await` | 11 | 240 | 240 |
| 5 | `import-meta` | 11 | 6 | 6 |
| 5 | `json-modules` | 11 | 10 | 10 |
| 6 | `dynamic-import` | 11 | 485 | 895 |
| 6 | `ess-target-module-commands` | 11 | 0 | 0 |
| 6 | `module-determinism-and-emulated-loading` | 11 | 0 | 0 |
| 7 | `cli-module-loader` | 11 | 0 | 0 |
| 7 | `bigint-values-and-literals` | 12 | 146 | 291 |
| 8 | `slice-11-conformance-close` | 11 | the lists | |
| 8 | `bigint-comparison-and-loose-equality` | 12 | 42 | 84 |
| 8 | `bigint-arithmetic-and-bitwise-operators` | 12 | 46 | 92 |
| 8 | `bigint-constructor-and-prototype` | 12 | 100 | 199 |
| 9 | `bigint-json-number-and-joined-cases` | 12 | 43 | 86 |
| 10 | `slice-12-conformance-close` | 12 | the lists | |

The case lists are in `docs/plan/slice-11/` and `docs/plan/slice-12/`; `lists.sh` of each
proves that its lists partition the slice (`slice 1281 lists 1281 incoming 0` and `slice 381
lists 381 incoming 0` over the full run of the base). The 4 cases of slice 12 that need
`import()` are on the list of its close.

## What the wave decides beside the stories

- The design text of modules is a story of the design author
  (`story:modules-design-text`): the paragraphs "Modules" and "Module records" of
  `docs/design/engine.md` and one host requirement, HOST-224. They state what the
  decomposition decided inside the epic's decisions: an arena of module records beside the
  scripts, the import slots of a module environment as a slot kind of their own, the graph
  walks on explicit work stacks charged to the memory budget, the module body as a frame of
  its own.
- The command-line host loads a module file and its static imports from disk
  (`story:cli-module-loader`): section 9 of the design lists "scripts and modules from files
  through the command-line host" among what the engine proves. The epic's bullet that left
  every loader to the host is rewritten at the close of the slice. `import()` and JSON
  modules in the command-line host stay for a later slice.
- BigInt: the five questions of the decomposition are settled on the option all four critics
  took; the limit `bigint_bits` is recorded as an omission in
  `conformance/test262/sections.txt` under decision 3 of the host interface, beside the
  string-length limit.

## Rules of the wave

As in wave big-2 ([the plan page](waves.md), "How a wave runs"): one implementor per story,
the failing test first; two adversary passes and the coordinator's reading before a merge;
the gate as the forge's runner runs it on the wave branch at every join; a finding that was
there before the unit is filed as a backlog draft and enters no wave before the planned
waves are delivered. One pull request per batch from the wave branch: the first when slice
11 is closed, the second when slice 12 is closed; the wave branch is pushed when a batch is
ready and not for each commit.

Drafts outside the wave that write a file a story of the wave writes get no edge now, since
no wave takes them before this one is delivered:
`story:spec-scenarios-for-the-numbers-and-dates-requirements` (three files of
`ess-target-module-commands`), `story:promise-closure-boxes-are-charged-in-a-library-test`,
`story:charge-comes-before-the-reservation`, `story:unsigned-binaries-documented`,
`story:binaries-signed` and `story:stack-tests-hold-in-the-stack-debug-profile`.

# Wave big-2: slice 9, collections, Proxy and Reflect

One wave of [the wave plan](waves.md), "After slice 4: bigger waves". Opened on 2026-11-26,
07:30 CET, on the operator's word of about 05:18 CET (`approval-record:wave-big-2`, which
applies the reading recorded in `approval-record:wave-big-1` and quotes the answer to the
coordinator's question, relayed at 07:31 CET: start). Skill version 0.26.0 of
`aep:implementing`.

The wave branch is `wave/big-2`, cut from `wave/big-1` at `2d04fa12` while that wave still
runs, with the plan branch of slice 9 merged in. `wave/big-1` is merged into it again each
time a unit of that wave lands, and the pull request of this wave is opened after the pull
request of wave big-1 is merged.

## The stories

The ten stories of `epic:slice-09-collections-proxy-reflect`, in the orders of
[the plan page](waves.md), "Slice 9". Each unit has a managed worktree `swell-impl-<unit>`,
a branch `impl/<unit>`, and its work files in the directory `wave2/<unit>` of the coordinating
session's scratch directory. The stage is kept current by the coordinator.

| order | unit | story | stage |
|---|---|---|---|
| 1 | keyed | `story:keyed-entry-table` | waits for the unit `growth` of wave big-1 (see below) |
| 1 | reflect | `story:reflect-namespace` | waits for the unit `growth` of wave big-1 |
| 2 | mapset | `story:map-and-set` | waits for order 1 |
| 2 | proxy | `story:proxy-objects-and-access-traps` | waits for order 1 |
| 3 | weak | `story:weak-map-and-weak-set` | waits for order 2 |
| 3 | setmethods | `story:set-methods` | waits for order 2 |
| 3 | proxytraps | `story:proxy-descriptor-prototype-and-extensibility-traps` | waits for order 2 |
| 4 | upsert | `story:map-upsert-and-group-by` | waits for order 3 |
| 4 | overrides | `story:host-object-overrides` | waits for order 3 and for the design line HOST-222 |
| 5 | close9 | `story:slice-09-conformance-close` | waits for every other story and for the close of slice 4 |

The stories of slice 8 join this wave when the plan critics approve them
(`docs/plan/waves.md`, the table of "After slice 4: bigger waves").

## Why the units wait for one unit of wave big-1

`story:buffer-growth-outside-the-charged-list-is-listed` (unit `growth` of wave big-1) makes
clippy refuse a growth call of a buffer outside the charged list unless the call carries an
attribute with its charge or its bound. Every story of slice 9 writes such calls. A unit that
starts on a base without that lint would write code the gate refuses after the merge, so the
units start on `wave/big-2` after `growth` is merged into `wave/big-1` and that branch into
this one.

## What the store's check said

`aep plan artifact waves --kind story --status draft`, read for the ten ids before the moves
(aep 0.80.0; the check was computed by the verb):

- The verb puts `keyed-entry-table` and `reflect-namespace` in its wave 1 and each other story
  in a wave of its own, nine waves, because it separates two stories that name one file.
- The decomposition puts two or three stories side by side in the orders 2, 3 and 4. For every
  file two stories of one order write, each story names the existing entry it inserts after,
  and the two entries differ; the parallel-safety critic checked each pair against the code in
  round 2 (`review-result:slice-09-round-2-parallel-safety`).
- `crates/swell/tests/main.rs` and `website/data/status.json` are generated; the coordinator
  generates them again after each merge.
- The coordinator alone writes `CHANGELOG.md`, `AGENTS.md`, `README.md`, `Cargo.lock`, the
  epic, `conformance/slices-delivered.txt`, this page and everything under `.engineering/`.
  A unit hands its changelog entry over in its report.

## A design change before order 4

`story:host-object-overrides` names the requirement HOST-222 (an override whose answer holds
a stale handle or a handle of another agent ends the run with `Terminated(EngineFault)`). The
sentence is written into `docs/design/host-interface.md` and
`conformance/host/requirements.txt` (`HOST-222 pending slice 9`) by a unit of its own before
the story starts, as HOST-214 was written before `math-random`.

## The commits of this wave

The unit commits on the unit branches, the merges into `wave/big-2`, the merges of
`wave/big-1` into it, the coordinator's commits on `wave/big-2`, one push of `wave/big-2` and
its pull request. No tag, no release, no merge into `main` by the coordinator.

## The gate's lines, what went wrong, where each finding went

Written at the close.

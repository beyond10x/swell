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
| 1 | keyed | `story:keyed-entry-table` | `impl/keyed`, cut at `58501bbf`; dispatched on 2026-11-26, 07:48 CET; green at `d9efe364` (2 files: `crates/swell/src/object/keyed_entries.rs`, new, 1,539 lines, and one `mod` line): the story's filter runs 39 tests on the base and 55 after in both modes (16 new, their times sum to 0.28 s), the package `1338 tests run: 1338 passed`; three deviations from the story, each stated by the unit: `next_live` takes the agent and charges a step per hole it passes, released cursors go to a free list in place of `Vec::retain`, a compaction with an index map charges a step per live entry; pass 1 (`review-result:adversary-keyed-pass-1`): 7 findings, 13 cases of which 3 red; a closed `CursorId` reaches the cursor that took its place in the free list, and `clear` and a compaction walk the cursor list without a charge per place (10,000 places for 2 steps and for 1 step); the two u32 limits have no test that reaches them; three mutants the 16 tests do not catch; a model test over 9,000 operations found no difference; correction 1 with the same implementor: the table refuses a stale id by a generation in the id, every place a walk visits is charged |
| 1 | reflect | `story:reflect-namespace` | `impl/reflect`, cut at `58501bbf`, with the base run copied to `.out/test262-base/runs.tsv`; dispatched on 2026-11-26, 07:48 CET; green at `3cc59442` (9 files, 994 lines added): `crates/swell/src/builtins/reflect.rs` and 13 tests that pass in both modes (the filter runs 79 tests on the base and 92 after); the part run passes the 480 cases of the list and the 249 guard cases (`test262-slice 09 480 960 0 0 480 480 0 0`), and no run ends on `Reflect is not defined` (296 on the base); `KIND_SITES` is a slice; pass 1 of the adversary reads `3cc59442` |
| 2 | mapset | `story:map-and-set` | waits for order 1 |
| 2 | proxy | `story:proxy-objects-and-access-traps` | waits for order 1 |
| 3 | weak | `story:weak-map-and-weak-set` | waits for order 2 |
| 3 | setmethods | `story:set-methods` | waits for order 2 |
| 3 | proxytraps | `story:proxy-descriptor-prototype-and-extensibility-traps` | waits for order 2 |
| 4 | upsert | `story:map-upsert-and-group-by` | waits for order 3 |
| 4 | overrides | `story:host-object-overrides` | waits for order 3; the design line HOST-222 is merged (`f3d194e2`) |
| 5 | close9 | `story:slice-09-conformance-close` | waits for every other story and for the close of slice 4 |

The stories of slice 8 join this wave when the plan critics approve them
(`docs/plan/waves.md`, the table of "After slice 4: bigger waves").

## Why the units wait for one unit of wave big-1

`story:buffer-growth-outside-the-charged-list-is-listed` (unit `growth` of wave big-1) makes
clippy refuse a growth call of a buffer outside the charged list unless the call carries an
attribute with its charge or its bound. Every story of slice 9 writes such calls. A unit that
starts on a base without that lint would write code the gate refuses after the merge, so the
units start on a base that holds it.

The unit returned green at `3a16be13` while its adversary passes were still to
run. Its branch was merged into this branch then (`58501bbf`), ahead of its merge into
`wave/big-1`, so that order 1 starts without waiting for those passes: the list of
`crates/swell/clippy.toml` (15 methods that reserve room in a `Vec`, `VecDeque` or `String`)
and the attribute form are what a unit of this wave needs. A correction of that unit reaches
this branch with the next merge of `wave/big-1`. `slot task clippy` on `58501bbf` exits 0.

## The base run

`task test262-expected` on this branch at `92ea4830`, with its output under
`.out/test262-base` and the committed files untouched (one job on the build node):
`test262 total 53801 8982 7 44812 26026 18784 2`, `test262-slice 09 1785 3541 0 0 1785 268
1517 0`, `test262-slice 04 1612 3184 0 0 1612 1609 3 0`, 85,556 run lines. The unit `growth`
adds attributes and no behaviour, so the run is the base of the units cut at `58501bbf`. Over
it the check of `story:reflect-namespace` reads `passed 517 bound 517 B 517 F 212 R 212` and
296 runs `Reflect is not defined` (the story, on an older base: 516, 213 and 296).

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

Done: unit `host-222`, `impl/host-222` at `248177d2`, merged at `f3d194e2`. It changes two
files and no code; the coordinator read its diff in place of an adversary pass
(`review-result:host-222-coordinator-read`). `task host-requirements-check` reads
`222 requirement(s), 222 line(s): 165 test, 57 pending, 0 omitted`. Until the join with wave
big-1, `task host-surface-check` and `task site-data-check` fail on this branch:
`CHANGELOG.md` does not name HOST-222 yet and `website/data/status.json` holds 221
requirements; both files change in wave big-1's close too and are written here after it.

## The commits of this wave

The unit commits on the unit branches, the merges into `wave/big-2`, the merges of
`wave/big-1` into it, the coordinator's commits on `wave/big-2`, one push of `wave/big-2` and
its pull request. No tag, no release, no merge into `main` by the coordinator.

## The gate's lines, what went wrong, where each finding went

Written at the close.

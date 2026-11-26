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
| 1 | keyed | `story:keyed-entry-table` | `impl/keyed`, cut at `58501bbf`; dispatched on 2026-11-26, 07:48 CET; green at `d9efe364` (2 files: `crates/swell/src/object/keyed_entries.rs`, new, 1,539 lines, and one `mod` line): the story's filter runs 39 tests on the base and 55 after in both modes (16 new, their times sum to 0.28 s), the package `1338 tests run: 1338 passed`; three deviations from the story, each stated by the unit: `next_live` takes the agent and charges a step per hole it passes, released cursors go to a free list in place of `Vec::retain`, a compaction with an index map charges a step per live entry; pass 1 (`review-result:adversary-keyed-pass-1`): 7 findings, 13 cases of which 3 red; a closed `CursorId` reaches the cursor that took its place in the free list, and `clear` and a compaction walk the cursor list without a charge per place (10,000 places for 2 steps and for 1 step); the two u32 limits have no test that reaches them; three mutants the 16 tests do not catch; a model test over 9,000 operations found no difference; correction 1 with the same implementor: the table refuses a stale id by a generation in the id, every place a walk visits is charged; correction 1 returned at `61a171bf` (one file; the module's tests from 16 to 35, the story's filter from 55 to 74 in both modes): a `CursorId` is 16 bytes with a generation from one counter of the table, a place of the cursor list is 32 bytes, `clear` charges `ceil((live + holes) / 64) + places` steps and a compaction `moved + places`; pass 2, the last (`review-result:adversary-keyed-pass-2`): no red case, 7 findings (1 warning, 6 notes), 4 green cases to keep; the end of the generation counter gets its test and answers `EngineFault` as the realm counter does; the two u32 limits are filed once for the engine (`story:a-limit-of-an-integer-width-has-a-test-or-a-stated-reason`, a backlog draft); the Bounds of this story and one test text of `story:map-and-set` now name the charge per place of the cursor list; correction 2 returned at `763d7bf6` (one file, 306 lines added, 5 removed; the module's tests from 35 to 39, the story's filter from 74 to 78): `open_cursor` faults the agent at the counter's end, and the adversary's four cases are tests of the module; the coordinator read the diff (`review-result:keyed-correction-2-coordinator-read`, no defect) and ran the claim again on `763d7bf6` on the build node (`78 tests run: 78 passed` in both modes, the gate's clippy step and the format check exit 0); merged into `wave/big-2` |
| 1 | reflect | `story:reflect-namespace` | `impl/reflect`, cut at `58501bbf`, with the base run copied to `.out/test262-base/runs.tsv`; dispatched on 2026-11-26, 07:48 CET; green at `3cc59442` (9 files, 994 lines added): `crates/swell/src/builtins/reflect.rs` and 13 tests that pass in both modes (the filter runs 79 tests on the base and 92 after); the part run passes the 480 cases of the list and the 249 guard cases (`test262-slice 09 480 960 0 0 480 480 0 0`), and no run ends on `Reflect is not defined` (296 on the base); `KIND_SITES` is a slice; pass 1 (`review-result:adversary-reflect-pass-1`): green, no red case, 5 notes on what the 13 tests cannot catch (the credit of the value list of `ownKeys`, the charge per element of the argument list, `Reflect.get` with an explicit `undefined` receiver, `Reflect.deleteProperty` on a dense array element, where the interrupt test raises its interrupt), 12 green cases; correction 1 returned at `8a2c03f4` (one file, `crates/swell/tests/builtins_reflect.rs`, 594 lines added; the module's tests from 13 to 28, the story's filter from 92 to 107): the charge test reads `heap_bytes()` after every entry, and the three mutants of the notes were shown red; the coordinator read the diff (`review-result:reflect-correction-1-coordinator-read`, no defect) and ran the claim again on `8a2c03f4` on the build node (`28 tests run: 28 passed` in both modes, the gate's clippy step and the format check exit 0); merged into `wave/big-2` |
| 2 | mapset | `story:map-and-set` | `impl/mapset`, cut at `896bcbe9`, with the base run of `dc310c6c`; dispatched on 2026-11-26, 09:32 CET |
| 2 | proxy | `story:proxy-objects-and-access-traps` | `impl/proxy`, cut at `896bcbe9`, with the base run of `dc310c6c`; dispatched on 2026-11-26, 09:32 CET |
| 3 | weak | `story:weak-map-and-weak-set` | waits for order 2 |
| 3 | setmethods | `story:set-methods` | waits for order 2 |
| 3 | proxytraps | `story:proxy-descriptor-prototype-and-extensibility-traps` | waits for order 2 |
| 4 | upsert | `story:map-upsert-and-group-by` | waits for order 3 |
| 4 | overrides | `story:host-object-overrides` | waits for order 3; the design line HOST-222 is merged (`f3d194e2`) |
| 5 | close9 | `story:slice-09-conformance-close` | waits for every other story and for the close of slice 4 |

## The stories of slice 8

The fourteen stories of `epic:slice-08-classes` joined this wave on 2026-11-26
(`approval-record:wave-big-2-slice-8`; the plan branch `plan/slice-08` merged at `3237cf33`),
in the orders of [the plan page](waves.md), "Slice 8, `classes`". Order 1 has no edge to a
story of slice 9 and starts beside slice 9's order 2.

| order | unit | story | stage |
|---|---|---|---|
| 8.1 | newtarget | `story:new-target-meta-property` | `impl/newtarget`, cut at `896bcbe9`, with the base run of `dc310c6c`; dispatched on 2026-11-26, 09:32 CET |
| 8.1 | super | `story:super-property-and-super-syntax` | `impl/super`, cut at `896bcbe9`; dispatched on 2026-11-26, 09:32 CET; green at `6a89ce2d` (30 files): `super.x` and `super[e]` run in the methods of object literals, the part run reads `passed 4822 bound 4822 B 4756 F 66 R 0`; the unit sets the parser's flag from the function's kind in `open_context` and adds the instructions `SuperBase` and `SuperKey` (the base is read before the key is converted); pass 1 of the adversary runs |
| 8.1 | classsyntax | `story:class-syntax-and-early-errors` | `impl/classsyntax`, cut at `896bcbe9`; dispatched on 2026-11-26, 09:32 CET; green at `1e671094` (43 files): the parser reads a class and answers its early errors, a class without one is held to the end of the parse (`HOLDS`), the part run reads `passed 2936 bound 2936 B 2469 F 467 R 0`; the base has 62 nesting kinds and 14 refused fixtures where the story says 61 and 16; one test of the base is over its CPU bound at full counts (filed as the draft `story:stack-bound-parts-fit-their-cpu-bounds`); pass 1 of the adversary runs |
| 8.2 | classes | `story:class-definition-evaluation` | waits for order 8.1 |
| 8.2 | elements | `story:class-element-syntax-and-early-errors` | waits for order 8.1 |
| 8.3 | derived | `story:derived-classes-and-super-call` | waits for order 8.2 |
| 8.3 | fields | `story:public-class-fields` | waits for order 8.2 |
| 8.3 | privatemethods | `story:private-names-and-private-methods` | waits for order 8.2 and for `story:weak-map-and-weak-set` |
| 8.4 | staticblocks | `story:class-static-blocks` | waits for order 8.3 |
| 8.4 | evalfields | `story:eval-in-class-field-initializers` | waits for order 8.3 |
| 8.5 | privatefields | `story:private-fields-and-private-in` | waits for order 8.4 |
| 8.6 | evalprivate | `story:private-names-in-eval-code` | waits for order 8.5 |
| 8.6 | hook | `story:host-ensure-can-add-private-element` | waits for order 8.5 and for `story:host-object-overrides` |
| 8.7 | close8 | `story:slice-08-conformance-close` | waits for every other story and for the close of slice 9 |

`aep plan artifact waves --status active`, read after the moves (aep 0.80.0): the verb puts
each of the three stories of order 8.1 in a wave of its own, since they name the same files
of the syntax and compiler crates (`parser/mod.rs`, `parser/expression.rs`, `ast.rs`,
`early.rs`, `compiler/src/expression.rs`, `resolve.rs`). The decomposition runs them side by
side: each story names the function or the arm it writes in a shared file and the fix at the
merge ("keep both"), and the parallel-safety critic read each pair in two rounds
(`review-result:slice-08-round-2-parallel-safety`; the reader of revision 2 found no site
with two owners). Between a story of slice 8's order 1 and a story of slice 9's order 2 the
verb names two shared files: `crates/swell/tests/main.rs`, which is generated, and
`crates/swell/src/interp/mod.rs` (`story:proxy-objects-and-access-traps` and
`story:super-property-and-super-syntax`). The unit that merges second merges the wave
branch into its own first and runs its checks again.

The base run of the join of order 1 (`dc310c6c`, keyed and reflect merged), one job on the
build node, output under `.out/test262-base`: `test262 total 53801 8982 7 44812 26250 18560
2`, `test262-slice 09 1785 3541 0 0 1785 480 1305 0`, 85,556 run lines. The plan of slice 8
changes no file a test262 run reads, so the run is the base of the units cut at `3237cf33`
and after.

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

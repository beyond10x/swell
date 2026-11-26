# Wave big-1: the end of slice 4, the core without std, static binaries

One wave of [the wave plan](waves.md), "After slice 4: bigger waves". Opened on 2026-11-26,
05:24 CET, on the operator's word of about 05:18 CET (`approval-record:wave-big-1`): one pull
request carries several stories. Skill version 0.26.0 of `aep:implementing`.

The wave branch is `wave/big-1`, cut from `main` at `b11998b5`, with the branch
`impl/nostd-core` (step 3 of `story:core-builds-without-std`) merged in as its first commit.

## The units

Each unit has a managed worktree (its id below), builds into `target/` inside that tree, and
keeps its work files in the directory `wave/<unit>` of the coordinating session's scratch
directory. The stage is kept current by the coordinator.

| unit | story | branch | worktree id | stage |
|---|---|---|---|---|
| core | `story:core-builds-without-std`, step 3 | `impl/nostd-core` | `swell-impl-nostd-core` | merged into the wave branch at `248a031c`; gate on that head: exit 0, `gate-time test-steps 106.5 of 120` at load 65 on the node |
| gate | `story:core-builds-without-std`, step 4 | `impl/nostd-gate` | `swell-impl-nostd-gate` | dispatched |
| random | `story:math-random` | `impl/math-random` | `swell-impl-math-random` | green at `ed59cd6b`; pass 1 (`review-result:adversary-random-pass-1`): no failing case, 4 findings, 3 of them gaps in the unit's tests; correction 1 with the same implementor (the adversary's cases join the unit); the collect-mode counts stay as they are |
| static | `story:releases-ship-static-binaries` | `impl/static-binaries` | `swell-impl-static-binaries` | merged into the wave branch at `6256b323`; pass 1: 7 findings fixed; pass 2: 4 new, 0 carried, fixed; the coordinator read the second correction (the check executes nothing of a workflow; the unit's own script was pinned to the new message and got one case more, no assertion dropped) |
| exponent | `story:exponentiation-operator` | `impl/exponentiation` | `swell-impl-exponentiation` | green at `b7d34305` (20 files; the 76 listed test262 cases pass); adversary pass 1 running |
| growth | `story:buffer-growth-outside-the-charged-list-is-listed` | `impl/buffer-growth` | `swell-impl-buffer-growth` | waits for `random`, `exponent` and `gate`: it adds an attribute at growth calls all over `crates/swell/src` |
| close | `story:slice-04-conformance-close` | `impl/slice-04-close` | `swell-impl-slice-04-close` | waits for `random`, `exponent` and `gate` on the wave branch |

Dispatches use the agent type `aep:implementor` on Opus; the adversary is `aep:adversary`. The
two decomposers of slices 9 and 8 run as `general-purpose` on Opus with the procedure of
`aep:decomposer`, because that agent type has no tool that writes a file and the shell here
takes no heredoc.

## Why these stories

- `math-random` and `slice-04-conformance-close` are the two stories of slice 4 that are
  accepted and not built: `test262-slice 04 1612 3184 0 0 1612 1533 79 0` in
  `conformance/test262/expected.txt` on `main` (79 cases do not pass), and six lines
  `pending slice 4` in `conformance/host/requirements.txt`.
- `core-builds-without-std` and `releases-ship-static-binaries` were ordered by the operator
  (the approval record quotes the words); the record moved both from `draft` to `active`.
- `story:bench-image-and-first-measurements` is accepted and not built. It is not in this
  wave: its runs need the build node alone for the measurement, and its tree holds work that
  is not committed.

Added at 05:30 CET, from the reading of the store's close: `story:exponentiation-operator`
(the parser still refuses `**`; 1 of 44 cases of `language/expressions/exponentiation` passes)
and `story:buffer-growth-outside-the-charged-list-is-listed` (`docs/plan/slice-10-wave-1.md`:
"planned, not built") are accepted and not built. Both are stories of the plan before this
wave, and the close of slice 4 depends on the first.

## What the store's check said

`aep plan artifact waves --kind story --status active`, read for the wave's ids (aep 0.80.0;
the check was computed by the verb):

- `story:math-random` is placed in the verb's wave 11, `story:slice-04-conformance-close` in
  its wave 19: the close depends on every story of slice 4, so it runs after `random` in this
  wave and not beside it.
- The typed scopes of `core-builds-without-std` and `releases-ship-static-binaries` were
  written by the coordinator from the stories' bodies; their lines are `inferred` except the
  paths the bodies name.
- Files two units write: `crates/swell/tests/main.rs` (`random` and `gate`; the file is a
  generated list of modules, the coordinator generates it again after the merges);
  `conformance/host/requirements.txt` (`random` moves six lines, `gate` changes the line of
  `HOST-2` at most); `docs/design/engine.md` (`gate` and `close`; the close starts after the
  gate unit is merged).
- The coordinator alone writes `CHANGELOG.md`, `AGENTS.md`, `README.md`, `Cargo.lock`,
  `conformance/test262/expected.digest`, this page and everything under `.engineering/`. A
  unit hands its changelog entry over in its report.

## A decision the coordinator took

The acceptance of `core-builds-without-std` asks for an example host built without `std`
"with a fixed block as its allocator". A global allocator is an `unsafe impl` in Rust, and a
program without `std` needs an `unsafe` entry attribute; `AGENTS.md` forbids `unsafe` in every
crate, and the operator decided on 2026-11-26 that the engine takes no `unsafe`, also not for
speed. The unit therefore proves the same thing without `unsafe`: the six crates are built
for the bare target, and a step of the gate reads the undefined symbols of those six
libraries and fails on any symbol of a C library (a float function, an I/O call). The
emulated test host (a fixed clock, a seeded generator) runs on the workstation's target and
names no system API. The story's acceptance line is rewritten at the close with this reason.

## The store's close

Stories that are delivered on `main` and still `active` in the store get their evidence (the
commit that delivered them and the gate's line) and move to `implemented` in this wave. The
list with its commits is made by a reading agent and checked by the coordinator before any
move.

The reading agent found 42 stories delivered. The coordinator's check on the wave branch
(the commit is an ancestor of the branch, the story is `active`, the named test or file is in
the tree): 40 hold; for one of them (`story:async-function-constructor`) the agent named the
wrong file and the test is at `crates/swell/tests/dynamic_function.rs:689`. The other two
(`story:release-0-12-0-with-binaries`, `story:gate-selftests-write-no-tracked-file`) are
delivered and still `draft`; they are not moved by this check.

## The commits of this wave

The unit commits on the unit branches, the merges into `wave/big-1`, the coordinator's commits
on `wave/big-1`, one push of `wave/big-1` and its pull request. No tag, no release, no merge
into `main` by the coordinator.

## The gate's lines, what went wrong, where each finding went

Written at the close.

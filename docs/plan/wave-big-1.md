# Wave big-1: the end of slice 4, the core without std, static binaries

One wave of [the wave plan](waves.md), "After slice 4: bigger waves". Opened on 2026-11-26,
05:40 CET, on the operator's word of about 05:18 CET (`approval-record:wave-big-1`): one pull
request carries several stories. Skill version 0.26.0 of `aep:implementing`.

The wave branch is `wave/big-1`, cut from `main` at `b11998b5`, with the branch
`impl/nostd-core` (step 3 of `story:core-builds-without-std`) merged in as its first commit.

## The units

Each unit has a managed worktree (its id below), builds into `target/` inside that tree, and
keeps its work files in the directory `wave/<unit>` of the coordinating session's scratch
directory. The stage is kept current by the coordinator.

| unit | story | branch | worktree id | stage |
|---|---|---|---|---|
| core | `story:core-builds-without-std`, step 3 | `impl/nostd-core` | `swell-impl-nostd-core` | committed up to `43e63c70`; its last commits (the timings, the digest) are being made |
| gate | `story:core-builds-without-std`, step 4 | `impl/nostd-gate` | `swell-impl-nostd-gate` | dispatched |
| random | `story:math-random` | `impl/math-random` | `swell-impl-math-random` | dispatched |
| static | `story:releases-ship-static-binaries` | `impl/static-binaries` | `swell-impl-static-binaries` | dispatched |
| close | `story:slice-04-conformance-close` | `impl/slice-04-close` | `swell-impl-slice-04-close` | waits for `random` and `gate` on the wave branch |

Dispatches use the agent type `aep:implementor` on Opus; the adversary is `aep:adversary`.

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

## The commits of this wave

The unit commits on the unit branches, the merges into `wave/big-1`, the coordinator's commits
on `wave/big-1`, one push of `wave/big-1` and its pull request. No tag, no release, no merge
into `main` by the coordinator.

## The gate's lines, what went wrong, where each finding went

Written at the close.

---
format: aep.planning-md/3
id: approval-record:wave-big-1
kind: approval-record
status: draft
title: 'Wave big-1 approved: several stories in one pull request'
relations:
- decides: story:core-builds-without-std
- decides: story:releases-ship-static-binaries
- decides: story:math-random
- decides: story:slice-04-conformance-close
revision: 1
---
## What the operator said

On 2026-11-26, about 05:18 CET, typed in the coordinating session:

> now, move in way bigger /aep:wave and deliver PRs with more stories + gap-closures at once

The coordinating session reads this as the approval to run the wave below without a second
word, and as the order that one pull request carries several stories. That reading is the
coordinating session's.

## The two stories the operator ordered before, which this record moves to `active`

`story:core-builds-without-std`. The operator, 2026-11-26 02:51 CET, typed in the coordinating
session:

> Yeah, I think it should be no STD for the core, and then we'll just inject the adapters from
> the outside, and they can have, of course, system access, but they don't have to. Yeah, it
> could also just be emulated.

and, relayed by the session that decides for the operator on 2026-11-26 at about 03:25 CET:

> Swell can just start already, it has nothing to do with models.

Steps 1 and 2 of that story are on `main` (pull requests 17 and 18); step 3 is the branch
`impl/nostd-core`, merged into this wave's branch; step 4 is a unit of this wave.

`story:releases-ship-static-binaries`. The operator's pick of 2026-11-25, about 23:15 CET,
relayed by the session that decides for the operator: "A: all Rust tools" (static musl Linux
releases for every Rust tool of the organization), and for this repository, on 2026-11-26:
"Static binaries stay as told (the musl pick, after this story)", the story before it being
`story:core-builds-without-std`.

## The wave

Wave `big-1`, the page `docs/plan/wave-big-1.md`:

- `story:math-random` and `story:slice-04-conformance-close`, the two stories of slice 4 that
  are accepted and not built (`docs/plan/waves.md`, "Slice 4", orders 4 and 5).
- `story:core-builds-without-std`, steps 3 and 4.
- `story:releases-ship-static-binaries`.
- The store's close for the stories that are delivered and still `active`.

## The commits this approval covers

One commit or more per unit on the unit's branch, the merges into `wave/big-1`, the
coordinator's commits on `wave/big-1` (this record, the wave page, the changelog, the gate's
numbers, the test262 digest, the store's moves), and the push of `wave/big-1` with its pull
request. Not covered: a tag, a release, the merge into `main` (the session that decides for
the operator merges), the next wave.

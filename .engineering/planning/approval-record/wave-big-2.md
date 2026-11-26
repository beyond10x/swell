---
format: aep.planning-md/3
id: approval-record:wave-big-2
kind: approval-record
status: draft
title: 'Wave big-2 approved: the ten stories of slice 9'
relations:
- decides: story:keyed-entry-table
- decides: story:reflect-namespace
- decides: story:map-and-set
- decides: story:proxy-objects-and-access-traps
- decides: story:weak-map-and-weak-set
- decides: story:set-methods
- decides: story:proxy-descriptor-prototype-and-extensibility-traps
- decides: story:map-upsert-and-group-by
- decides: story:host-object-overrides
- decides: story:slice-09-conformance-close
revision: 1
---
## What the operator said

On 2026-11-26, about 05:18 CET, typed in the coordinating session:

> now, move in way bigger /aep:wave and deliver PRs with more stories + gap-closures at once

The coordinating session reads this as the approval to run the waves of `docs/plan/waves.md`,
"After slice 4: bigger waves", without a second word, and as the order that one pull request
carries several stories. That reading is the coordinating session's. It was recorded for the
first of those waves in `approval-record:wave-big-1`; this record applies it to the second.

The plan page named this wave before its stories existed (`docs/plan/waves.md`, the table of
"After slice 4: bigger waves", committed with wave big-1): "big-2: every story of slice 9,
`collections-proxy-reflect`, and of slice 8, `classes`, once the plan critics approve them",
and "A story of big-2 starts as soon as its critics approve it (the organization's rule 'Goal
first')".

The coordinating session put the start of this wave to the operator as a question with three
options (A: start; B: wait until the pull request of wave big-1 is merged; C: wait for the
operator's own word on the ten stories). The answer, relayed on 2026-11-26 at 07:31 CET by the
session that decides for the operator, on the operator's delegation:

> A — start big-2 with the ten slice-9 stories as you described (approval record quotes [the operator's]
> 05:18 word, reading labelled as yours; cut from wave/big-1; PR after big-1's merges).

## What this record moves

The ten stories that decompose `epic:slice-09-collections-proxy-reflect` move from `draft` to
`active`. The four plan critics read them twice (`review-result:slice-09-round-1-*`,
`review-result:slice-09-round-2-*`): scope and parallel safety approved in round 2; the nine
findings of design and acceptance of round 2 are answered in the second revision, which the
coordinating session read in place of a third round
(`review-result:slice-09-revision-2-coordinator-read`).

The stories of slice 8 are not moved by this record. They are in their first revision after
round 1 of the critics; a record of their own moves them when the critics approve them.

## The wave

Wave `big-2`, the page `docs/plan/wave-big-2.md`, on the branch `wave/big-2`, cut from
`wave/big-1`. Its pull request is opened after the pull request of wave big-1 is merged.

## The commits this approval covers

One commit or more per unit on the unit's branch, the merges into `wave/big-2`, the
coordinator's commits on `wave/big-2` (this record, the wave page, the changelog, the gate's
numbers, the test262 digest, the store's moves), and the push of `wave/big-2` with its pull
request. No tag, no release, no merge into `main` by the coordinating session.

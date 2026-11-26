---
format: aep.planning-md/3
id: approval-record:wave-big-2-slice-8
kind: approval-record
status: draft
title: Wave big-2 takes the fourteen stories of slice 8
relations:
- decides: story:new-target-meta-property
- decides: story:super-property-and-super-syntax
- decides: story:class-syntax-and-early-errors
- decides: story:class-definition-evaluation
- decides: story:class-element-syntax-and-early-errors
- decides: story:derived-classes-and-super-call
- decides: story:public-class-fields
- decides: story:private-names-and-private-methods
- decides: story:class-static-blocks
- decides: story:eval-in-class-field-initializers
- decides: story:private-fields-and-private-in
- decides: story:private-names-in-eval-code
- decides: story:host-ensure-can-add-private-element
- decides: story:slice-08-conformance-close
revision: 1
---
## What the operator said

On 2026-11-26, about 05:18 CET, typed in the coordinating session:

> now, move in way bigger /aep:wave and deliver PRs with more stories + gap-closures at once

The coordinating session reads this as the approval to run the waves of `docs/plan/waves.md`,
"After slice 4: bigger waves", without a second word, and as the order that one pull request
carries several stories. That reading is the coordinating session's. It was recorded in
`approval-record:wave-big-1` and `approval-record:wave-big-2`; this record applies it to the
stories of slice 8.

The plan page named them for this wave before they existed (`docs/plan/waves.md`, the table
of "After slice 4: bigger waves"): "big-2: every story of slice 9,
`collections-proxy-reflect`, and of slice 8, `classes`, once the plan critics approve them".

The coordinating session put the question where the fourteen stories go to the operator with
three options (A: they join wave big-2, one pull request with slice 9, and their first order
starts beside slice 9's second; B: a wave big-3 of its own with its own pull request; C: hold
them until the operator has read the plan). The answer, relayed on 2026-11-26 at about
09:20 CET by the session that decides for the operator, on the operator's delegation:

> D38 A — slice 8 joins wave big-2 (one PR, 24 stories), approval record as for big-2.

## What this record moves

The fourteen stories that decompose `epic:slice-08-classes` move from `draft` to `active`.
The four plan critics read them twice (`review-result:slice-08-round-1-*`, 48 findings;
`review-result:slice-08-round-2-*`, 29 findings); the decomposer revised them three times,
and the coordinating session read revisions 2 and 3 in place of a third round
(`review-result:slice-08-revision-3-coordinator-read`).

## The wave

Wave `big-2`, the page `docs/plan/wave-big-2.md`, on the branch `wave/big-2`. The orders of
slice 8 are those of `docs/plan/waves.md`, "Slice 8, `classes`". Order 1 has no edge to a
story of slice 9 and starts beside slice 9's order 2. Three stories wait for slice 9:
`private-names-and-private-methods` for `weak-map-and-weak-set`,
`host-ensure-can-add-private-element` for `host-object-overrides`, and
`slice-08-conformance-close` for `slice-09-conformance-close`. The pull request of the wave
is opened after the pull request of wave big-1 is merged.

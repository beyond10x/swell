---
format: aep.planning-md/3
id: approval-record:wave-big-3
kind: approval-record
status: draft
title: Wave big-3 takes the 23 stories of slices 11 and 12
relations:
- decides: story:bigint-arithmetic-and-bitwise-operators
- decides: story:bigint-comparison-and-loose-equality
- decides: story:bigint-constructor-and-prototype
- decides: story:bigint-json-number-and-joined-cases
- decides: story:bigint-values-and-literals
- decides: story:cli-module-loader
- decides: story:dynamic-import
- decides: story:ess-target-module-commands
- decides: story:import-call-and-import-meta-syntax
- decides: story:import-meta
- decides: story:json-modules
- decides: story:module-code-compiles
- decides: story:module-determinism-and-emulated-loading
- decides: story:module-linking-and-evaluation
- decides: story:module-namespace-objects
- decides: story:module-records-loading-and-module-runs
- decides: story:modules-design-text
- decides: story:module-syntax-and-early-errors
- decides: story:slice-11-conformance-close
- decides: story:slice-12-conformance-close
- decides: story:swell-num-bigint-core
- decides: story:swell-num-bigint-powers-shifts-and-bits
- decides: story:top-level-await
revision: 1
---
## What the operator said

The coordinating session put the stage-1 proposal of wave big-3 to the operator on
2026-11-28 at 04:32 CET, through the session that decides for the operator on the
operator's delegation, with three options (A: accept all 23 stories, units start when lane
unit 8 is merged; B: accept the 15 stories of slice 11 only, and ask again for slice 12 at
the close of slice 11; C: hold both). No default was named. The answer, relayed on
2026-11-28 at about 04:53 CET:

> A. Accept wave big-3, all 23 stories (slices 11 and 12); units start when lane unit 8 is
> merged.

## What this record moves

The 23 stories that decompose `epic:slice-11-modules` (15) and `epic:slice-12-bigint` (8)
move from `draft` to `active`. The four plan critics read each slice twice
(`review-result:slice-11-round-1-acceptance` to `review-result:slice-11-round-2-scope`, and
the same eight for slice 12), and the coordinating session read revision 2 of each in place
of a third round (`review-result:slice-11-revision-2-coordinator-read`,
`review-result:slice-12-revision-2-coordinator-read`).

## The wave

Wave `big-3`, the page `docs/plan/wave-big-3.md`. Skill version 0.26.0 of
`aep:implementing`. The order is the page's table, with the three edges between the slices
of `docs/plan/waves.md`, "The join of slices 11 and 12". No unit of the wave is dispatched
before the pull request of `story:slow-lane-holds-its-bound-on-the-slower-runner` (lane
unit 8 of wave big-2) is merged; the wave branch `wave/big-3` is cut from `main` then. The
design text of modules (`story:modules-design-text`) is the design author's and is written
before the first unit of slice 11 starts.

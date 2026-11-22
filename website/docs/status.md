---
title: Status
description: What swell passes today, per delivery slice, from the counts the gate holds.
---

import {Num, S, Delivered, SliceBar, PendingTable, HarnessException} from '@site/src/components/Numbers';

# Status

The numbers on this page are read from the gate's committed files by `task site-data`. The gate fails when a count differs from `conformance/test262/expected.txt`, so a change that moves a count commits the new file with it. `task site-data-check` fails while this site shows another number.

## test262

The gate runs test262 at a pinned commit and compares its counts with `conformance/test262/expected.txt`. The total line today reads <code><Num at="test262.line" /></code>.

| exist | excluded | not applicable | run | pass | fail | ended by a budget |
|---|---|---|---|---|---|---|
| <Num at="test262.exist" /> | <Num at="test262.excluded" /> | <Num at="test262.not_applicable" /> | <Num at="test262.run" /> | <Num at="test262.passed" /> | <Num at="test262.failed" /> | <Num at="test262.budget" /> |

<Num at="test262.passed" /> of the <Num at="test262.run" /> cases that run pass, <Num at="test262.passed_percent" /> %. The excluded cases are excluded by recorded decision: `Temporal`, `Intl` and the proposals at stage 3. A case runs twice, as written and in strict mode, unless its flags say otherwise. Each run has a step and a memory budget, and a run that a budget ends is counted in the last column.

## By delivery slice

A case counts in the first delivery slice after which it can pass. That says when a case can first pass, not that it will. A slice is delivered when its cases pass in all their runs, but for an exception the design states, and no host requirement is pending for it.

<SliceBar />

| slice | name | cases | runs | pass | state |
|---|---|---|---|---|---|
| 1 | <code><S n={1} f="name" /></code> | <S n={1} f="cases" /> | <S n={1} f="runs" /> | <S n={1} f="passed" /> | <Delivered n={1} /> |
| 2 | <code><S n={2} f="name" /></code> | <S n={2} f="cases" /> | <S n={2} f="runs" /> | <S n={2} f="passed" /> | <Delivered n={2} /> |
| 3 | <code><S n={3} f="name" /></code> | <S n={3} f="cases" /> | <S n={3} f="runs" /> | <S n={3} f="passed" /> | <Delivered n={3} /> |

<HarnessException />

The [roadmap](roadmap.md) has all the slices. Slice 1 is the test262 harness itself. Slice 2 is the rest of the ES5 language: `with`, `arguments`, `eval`, the global `Function`, labels and automatic semicolon insertion.

## What a pass means

A construct the parser does not have yet is refused with a `SyntaxError` whose message gives the slice that brings it. A refusal does not count as the `SyntaxError` a negative case asks for. A refusal the script catches fails the run too: the runner reads `Agent::refusals()` after each run.

`conformance/test262/moves.tsv` lists <Num at="moves.cases" /> cases that moved to a later slice when slice 2 or slice 3 closed. A line holds the token or feature that needs the later slice, and the gate task `test262-moves-check` refuses a line without it. A case that fails on a part of its own slice is not moved.

## The host interface

The host interface is stated as <Num at="host_requirements.total" /> numbered requirements in [`host-interface.md`](https://github.com/beyond10x/swell/blob/main/docs/design/host-interface.md). <Num at="host_requirements.test" /> of them are held by a test today, and <Num at="host_requirements.pending" /> wait for the slice that implements them. The gate fails while a requirement is pending for a delivered slice.

<PendingTable />

## What is not measured

How much slower the arenas and the plain property lists make swell than an engine with unsafe code is not measured. The gate's numbers come from x86_64 Linux only. The [limits](limits.md) page lists what is missing.

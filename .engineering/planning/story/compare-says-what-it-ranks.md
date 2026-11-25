---
format: aep.planning-md/3
id: story:compare-says-what-it-ranks
kind: story
status: draft
title: /compare says what it ranks and why swell is not ranked
summary: Comparable cells, swell's two K1 readings, a visible source per value, the V8 Intl figures labelled
revision: 3
---
From the first-reader review of /compare, /docs/kpis and the landing page on 2026-11-25
(`docs/plan/compare-review-2026-11-25.md`, findings by number). Live site at main `f9b568f`.

Outcome: a first-time reader can tell, from the page alone, which cells are comparable, why swell's K1 is not ranked, and where each number comes from.
Covers: 1, 2, 3, 4, 5, 6, 7, 11, 12, 13, 25.
Acceptance:
- Under a cell with no peer in its basis the page prints "not ranked" with the reason; the intro names swell; the K1 swell cell labels both readings, shows the excluded count, and renders "none run" instead of "0 of 0 ... 0.0 %" (test on the data row with `secondary.of == 0`).
- Every value has a visible source line with run date and revision (swell cells name a commit hash); the claim "every cell links" is true or reworded; "engine catalog" labels open the cited line.
- The V8 intl402 and built-ins figures are verified against javascript-zoo-data, and the cell says what the build lacked or is marked "as published".

## Checked 2026-11-25

- `bench/published/zoo-test262-4a1e962.tsv`: `v8-jitless` and `v8` both read intl402 25 passed of
  3,324, engine revisions `5f33ee62` and `a05b78ae`, flags
  `--ignore-unhandled-promises --no-arguments --allow-natives-syntax`. The published figure is
  reproduced; that the zoo build of d8 lacks Intl is inferred, not verified. The cell shows
  the engine revision and flags and is marked "as published".

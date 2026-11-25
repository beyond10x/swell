---
format: aep.planning-md/3
id: story:compare-reads-on-a-phone
kind: story
status: draft
title: Empty rows collapse and the compare table reads on a phone
summary: Data first, not-measured KPIs on one line, usable at 55 columns and by keyboard
revision: 2
---
From the first-reader review of /compare, /docs/kpis and the landing page on 2026-11-25
(`docs/plan/compare-review-2026-11-25.md`, findings by number). Live site at main `f9b568f`.

Outcome: the default view shows data first; "not measured" and "swell only" rows take one line each; the table is usable at 55 columns and by keyboard.
Covers: 8, 9, 10, 17, 18, 19, 20, 21, 22, 23.
Acceptance:
- A KPI with all cells "not measured" renders one full-width row with its reason (K5: no shell binary); swell-only rows span the other columns; the default view has at most 10 rows with cells of "not measured", "unknown" or "not applicable" (now 30 of 100 cells).
- At 440 px width the row label stays visible while scrolling; the scroll region is focusable and labelled, the table has a caption, the "Other engines" picker is collapsed, and a full or empty selection prints why a box is disabled.
- An unknown id or a one-id `?e=` prints a message.


---
format: aep.planning-md/3
id: story:compare-words-and-entry-points
kind: story
status: draft
title: Words and entry points a first-time reader can follow
summary: Named KPI headings, a terms list, a link from the landing page body, a link check in the site gate
revision: 2
---
From the first-reader review of /compare, /docs/kpis and the landing page on 2026-11-25
(`docs/plan/compare-review-2026-11-25.md`, findings by number). Live site at main `f9b568f`.

Outcome: nobody needs to know K-numbers, "zoo runner" or "slice" to read the comparison, and the landing page leads to it.
Covers: 14, 15, 16, 24, 26, 27, 28.
Acceptance:
- /docs/kpis headings read "K1 · test262 conformance" and so on, with a 9-row summary table on top; /compare has a short terms list (KPI, zoo runner, JIT reference, flags unverified); the picker group is not called "Default set".
- The landing page body links to /compare (with the default `?e=`) in section "Compared to others" and in the hero, and its 57.8 % states the 48.2 % reading or links to #k1.
- A link check over the three pages (internal links, anchors, line anchors into swell files) runs in the site gate and passes.

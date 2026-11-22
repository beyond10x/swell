---
format: aep.planning-md/3
id: task:slice-03-harness-case-measured-at-the-next-pin
kind: task
status: draft
title: Measure String/prop-desc.js again when the test262 pin moves, and remove the exception of slice 3 when it passes
tags:
- deferred
relations:
- depends_on: story:slice-03-conformance-close
revision: 1
---
## Context

Slice 3 was delivered on 2026-11-22 with 7,010 of its 7,011 cases passing (`docs/design/engine.md`, section 12, "Slice 3's cases that need a later slice"). The one case, `test/built-ins/String/prop-desc.js`, fails in the pinned harness before the engine's answer is judged: `harness/propertyHelper.js` deletes the global `String` (line 171) and then calls it (lines 182 and 156). That it would fail in another engine is inferred from the harness source; no other engine was run. On 2026-11-22 the harness file on test262's `main` was byte-equal to the pinned one. Ruled that day by the session that decides for the operator: the case stays in slice 3, counted failed; no rule line for a defect of the harness, for one case.

## Change

At the next move of the test262 pin (`conformance/test262/PIN`), measure the case again. When it passes, remove the exception from the design paragraph and from the sentences of `AGENTS.md`, `README.md` and the site that point to it, in the change that moves the pin. A second case that fails in the harness before the engine's answer is judged reopens the question of a rule line for such a case: tell the deciding session then.

## Acceptance

After a move of the pin: the line `test262-slice 03` reads every case passed, or this task stays open with the date of the measurement and the harness lines read again.

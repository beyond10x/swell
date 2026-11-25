# swell /compare and /docs/kpis: first-time reader review

Read-only. Read at origin/main f9b568fc (2026-11-25 14:52 UTC) and the live pages on 2026-11-25. Nothing edited, built or committed.

Method notes:
- `compare` and `compare?e=swell,v8-jitless,quickjs,boa` return byte-identical HTML (57,313 bytes): the query equals `default_selection` (engines.json), and `compare.tsx:95` reads the query only after hydration. A link with any other `?e=` shows the default in a crawler, a link preview or with JS off.
- Links: 90 distinct hrefs on the 3 pages. Internal ones and the anchors #k1..#k9, #facts all exist (200). github.com returned 503 under a fast loop, then 200 for all 21 retried at 3 s spacing; crates.io API 403 without a User-Agent, 200 with one. No broken link found. Line anchors into swell's own files (PIN#L2, Cargo.toml#L6-L8, requirements.txt#L133, limits.rs#L14/L17/L19, interrupt.rs#L19, expected.txt#L1-L2, L3-L66, L67, L68-L87, L88-L211, L212-L221, L222) all land on the named line.
- Default selection (swell, V8 --jitless, QuickJS, Boa): 25 rows x 4 = 100 cells. 30 cells carry no value (17 "not measured", 7 "unknown", 6 "not applicable"), and 30 cells have no link.

## Findings

Severity: wrong / misleading / unclear / cosmetic.

### A. Numbers and ranking

1. **swell's K1 is not ranked and the page does not say why next to the number.** misleading.
   Where: /compare, K1 · Conformance, row "test262, every case"; text "25,943 of 53,801 · 48.2 %" and "best of the picked" under Boa only. Intro `compare.tsx:147-152` says "Values of two test262 revisions or two runners are shown side by side but not ranked", without naming swell. `bestOf` (`engines.ts:118-137`) needs two cells of one `basis`; swell's `swell-runner-2a1ade30` is alone.
   Smallest fix: when a row has a cell with no peer in its basis, print under that cell "not ranked: other runner and test262 revision" (one line in `compare.tsx:196-205`); and in the intro say "swell is not ranked against the other engines".
   Also: with 3 zoo engines, "best of the picked" under Boa reads as "best including swell".
2. **Two swell figures, no statement which one is comparable.** unclear.
   Where: swell K1 cell: "48.2 %" plus "25,943 of 44,812 of the cases swell's runner runs, 57.8 %". The zoo counts (53,167 total) are closest to the 53,801 reading. The 634-case difference in totals (53,801 vs 53,167) is not explained. "of ... of the cases" reads as a typo (`compare.tsx:57`).
   Fix: label the first "of every case at the pin (excluded count as failed)" and the second "of the cases the runner runs (excluded left out)"; add the excluded count (8,982 of 53,801, expected.txt L222) to the cell.
3. **Landing page and /compare headline different swell numbers.** misleading.
   Where: landing hero "57.8 % of the test262 cases it runs pass"; /compare K1 headline "48.2 %". Both are labelled, but a reader coming from the landing page meets a lower number with no bridge.
   Fix: landing hero says "57.8 % of the cases it runs, 48.2 % of all cases" or links to /compare#k1.
4. **"0 of 0 of the cases swell's runner runs, 0.0 %" for intl402.** misleading (cosmetic maths, real confusion).
   Where: K1 test262 intl402/, swell cell; data `K1.intl402.secondary` {0,0,"0.0"}. All 3,394 intl402 cases are excluded, not failed. "0.0 %" of 0 is undefined.
   Fix: render "none run (all 3,394 excluded)" when `secondary.of === 0` (`compare.tsx:55-59`).
5. **V8 --jitless numbers look implausible and are unexplained.** misleading, possibly wrong; needs verification at the source.
   Where: V8 column: built-ins 77.2 %, intl402 "25 of 3,324 · 0.7 %", total 82.4 %, below QuickJS-class and far below Boa 96.7 %. The first-time reader concludes V8 fails test262; V8 intl402 near 0 % suggests the zoo's d8 was built without Intl/ICU, and flags `--no-arguments --allow-natives-syntax` (visible only in the tooltip) are the zoo's. The page shows no engine version for the run: only `engine 5f33ee62300c` in the hover tooltip; "Latest release V8 16.0.0" (2026-11-25) is a different thing from a run dated 2026-05-22.
   Fix: show engine revision and run date as visible text in the source line; add a caveat row note where a directory is below 5 % ("build without Intl"?) after checking with javascript-zoo-data; if unverifiable, say "as published by javascript-zoo-data, build options not shown".
6. **Dates differ by six months across a row and are not flagged.** unclear.
   Where: swell "as of this site's commit" vs zoo "2026-05-22" vs catalog "2026-11-25" (`compare.tsx:28`). Fix: show "run 2026-05-22, fetched 2026-11-25" visibly (data already holds `fetched`).
7. **"as of this site's commit" cannot be resolved by the reader.** unclear.
   Where: every swell cell source line; the footer "This build docs-system 081e05f" links to the docs-system template repository, not to a swell commit. Fix: footer or foot paragraph (`compare.tsx:214-226`) names the swell commit hash with a link, and the cell says "swell <hash7>".
8. **K2 to K5 are four whole KPIs, "not measured / no run yet", in all 4 columns (16 cells), and the reason is the same generic text.** unclear (dominates the table).
   Where: K2 Speed, K3 Start, K4 Memory, K5 Size. The specific reasons are on /docs/kpis ("None exists yet", K5 "swell has no shell binary yet", K3 "A shell without `$262.createRealm` reads 'not measured'"), but each cell says only "no run yet". kpis.md:35-69 also defines two measurements for K3 and K4 (empty script / new realm; empty script / Splay), while the page has one row each.
   Fix: collapse a KPI whose cells are all "not measured" into one full-width row "K2 Speed: not measured for any engine yet. Needs: <reason>. Planned: <definition>" (`compare.tsx:178-211`), with the swell-specific reason for K5; rows stay in the data.
9. **7 "unknown" and 6 "not applicable" cells.** unclear.
   Where: K8 for the 3 other engines ("unknown, the engine catalog records no determinism field": absence of a catalog field, not of the feature); K7 "Crate roots with forbid(unsafe_code)" and "Packages from outside the repository" repeat "not applicable ... swell only" 3 times each. K6 Boa memory cap "unknown (no memory limit found in RuntimeLimits)" reads as "Boa has none".
   Fix: render a swell-only row (K7 forbid, K7 packages) as one cell spanning the other columns; K8 label "not recorded" instead of "unknown".
10. **Facts "Releases in the last 12 months" for swell: "not measured / site-data reads no release history".** unclear (cosmetic).
    Git tags 0.9.0..0.11.0 exist; the data could count them. Latest release 0.11.0 is sourced from `Cargo.toml#L8`, not the tag. V8's "latest release" cell is "V8 16.0.0 on main ... Chrome stable 158 ... (V8 version inside Chrome 158 not verified)": a hedge that tells the reader nothing. Fix: count tags for swell; for V8 say "no release number; rolling".

### B. Sources and claims

11. **"Every cell links to its source" is false for 30 of 100 cells.** wrong.
    Where: `compare.tsx:148-149` and /docs/kpis. Cells with status not-measured (17), unknown (7), not-applicable (6) have no `source.url` or no source (`compare.tsx:25-27`, 41-49).
    Fix: reword to "Every value links to its source", or show "no source: <reason>" in the source line.
12. **The label "engine catalog, documented" links to an upstream file, not to the catalog.** misleading.
    Where: e.g. QuickJS K6 cells: label "engine catalog, derived from interrupt_terminate" opens `github.com/bellard/quickjs/blob/master/quickjs.h` (no line anchor, though the text cites `quickjs.h:382-383`); V8 cells open `v8-isolate.h`; Boa K7 opens `core/gc/src/lib.rs` (text says "lines 237-266", link has no `#L237-L266`).
    Fix: label "quickjs.h (via engine catalog)" and add the `#L` anchor from the cited lines; the catalog link stays in the foot paragraph.
13. **Source details (method, revision, sha256, run flags) are in a `title` tooltip only.** unclear / accessibility.
    Where: `compare.tsx:33` `title={detail}`. Not available on touch or keyboard focus; the exclusion rule "excluded and not applicable cases count as failed" lives only there and on /docs/kpis.
    Fix: put method and revision in a `<details>` per source line, or a "Sources" section below the table.

### C. Labels, jargon

14. **K1..K8 and "KPI" are not explained on /compare.** unclear.
    Where: group headers "K1 · Conformance", "K6 · Resource control" (`compare.tsx:183`): "K1" means nothing to a first-time reader; the link goes to /docs/kpis#k1, whose heading is the bare "K1" (h2 id=k1; the "On this page" list reads K1..K9 with no names; the title "test262 conformance" is bold text below).
    Fix: kpis.md headings "## K1 · test262 conformance" (anchor stays by `{#k1}`); drop "K1 ·" from the compare group header or add a one-line legend "KPI = a measure of what makes an engine good for running untrusted scripts".
15. **Jargon without gloss.** unclear.
    Terms: "zoo runner", "javascript-zoo-data" (named, link only in foot paragraph), "JIT reference", "interpreter · --jitless", "(flags unverified)", "best of the picked", "test262 staging/", "intl402/", "annexB/", "harness/", "agent's life", "outermost entry", "Terminated(StepBudget)" in K6 swell cells, "Crate roots", "site-data", "gate" and "slice" on /docs/kpis.
    Fix: one "Terms" `<details>` under the intro, 8 lines; K6 swell cells say "ends the run with a step-budget outcome".
16. **Picker legend "Default set" is not the default selection.** misleading.
    Where: `compare.tsx:155`; the group holds 8 engines (V8, SpiderMonkey, JavaScriptCore, Hermes, QuickJS, QuickJS-ng, Boa, XS), the default selection is 4 (swell, V8, QuickJS, Boa). Fix: rename "Main engines".
17. **Row labels repeat "higher is better" 7 times in K1 and have trailing slashes** ("test262 annexB/"). cosmetic. Fix: state the direction once in the group header.
18. **Facts rows are free text of different shape per engine** (V8 execution model is one 3-line sentence with "Ignition and Sparkplug names not re-verified in this pass"). unclear. Fix: short cell plus note; move "not verified in this pass" to the note slot (`Cell.note` exists).

### D. Selection, mobile, accessibility

19. **Default selection is sensible but the page opens with ~38 checkboxes before any data.** unclear.
    Where: pickers `compare.tsx:153-160`: 1 + 8 + 29 pills plus the JIT toggle. On a ~55-column phone (about 440 px) these fill several screens before the table. When 4 are picked every other checkbox is disabled without a message (`compare.tsx:131`), and unchecking at 2 is also silent.
    Fix: collapse "Other engines" in a `<details>`, and print "4 of 4 picked: remove one to add another" next to the picker.
20. **Table on mobile.** unclear.
    Where: CSS `custom.css:356-359`: label column min 13 rem plus 4 x 12 rem = 61 rem (~976 px) inside `.sw-cmp-scroll {overflow-x:auto}`; the row-label column scrolls away, so on a phone the numbers are shown without the row they belong to. Fix: `position: sticky; inset-inline-start: 0` on `tbody th[scope=row]` with a background; or stack per KPI under 600 px.
21. **Accessibility basics.** unclear.
    Good: `th scope=col`, `scope=row`, `scope=rowgroup`, fieldset and legend per picker, `lang=en`, a skip link. Missing: no `<caption>` or `aria-label` on the table; the scroll container is not focusable (keyboard users cannot scroll it; `tabindex=0 role=region aria-label`); selection change via `history.replace` is not announced (no `aria-live`); source detail only in `title`; link texts "notice", "test262 commit" are ambiguous out of context.
22. **Silent fallbacks in the URL selection.** unclear.
    Where: `engines.ts:151`: fewer than 2 valid ids (a typo in one id) returns the default with no message; `.slice(0,4)` drops a 5th silently; JIT ids without `jit=1` are dropped (the page re-adds them via `showJit`, `compare.tsx:98`, so a link with a JIT id works but the toggle shows on). Fix: print "Unknown engine id 'x' ignored" under the intro.
23. **Static page vs JS.** cosmetic. Static HTML is the default selection only (`compare.tsx:93-95`); a shared `?e=` link shows the default first and then switches after hydration.

### E. Landing page

24. **The landing page does not link to /compare in its body.** unclear.
    Where: https://beyond10x.github.io/swell/ links /compare only in the navbar ("Compare", the last item of 6) and the footer ("Compare engines"). The body section "05 / Compared to others" (`website/src/pages/index.tsx:144`) is about swell's design costs and has no link; the hero buttons are "Where it stands", "Architecture", "Embedding", "Repository". The hero says nothing of /docs/kpis either. Fix: add "See the numbers beside V8, QuickJS and Boa" with a link to `/compare?e=swell,v8-jitless,quickjs,boa` in section 05 and one hero link.
25. **/compare gives no "why swell is low" context.** misleading by omission.
    Where: the compare page never says swell is early (7 of 19 slices delivered; landing and Status do). A reader sees 48.2 % against 96.7 % with no pointer. Fix: one line under the intro: "swell is early: 7 of 19 slices delivered, see Status" with a link; per-slice numbers are on the landing page.

### F. /docs/kpis

26. K1 to K9 summary missing: a first-time reader has no one-screen table (KPI, what, gate or compared, measured yet). unclear. Fix: a 9-row table at the top.
27. kpis.md:23 calls `website/data/engines.json` "The page"; kpis.md:20 "None exists yet" and 69 "swell has no shell binary yet" are the only places the K2-K5 reason is stated (see 8). unclear.
28. kpis.md:42 says SpiderMonkey and JavaScriptCore compared "with their JITs off", but their flags are marked "(flags unverified)" in data (`spidermonkey-interp`, `jsc-nojit`, `flags_verified:false`); the KPI page does not say so. misleading (minor).
29. No broken links on /docs/kpis (9 external, 10 internal, all 200).

## Proposed stories

### Story 1: /compare says what it ranks and why swell is not ranked
Outcome: a first-time reader can tell, from the page alone, which cells are comparable, why swell's K1 is not ranked, and where each number comes from.
Covers: 1, 2, 3, 4, 5, 6, 7, 11, 12, 13, 25.
Acceptance:
- Under a cell with no peer in its basis the page prints "not ranked" with the reason; the intro names swell; the K1 swell cell labels both readings, shows the excluded count, and renders "none run" instead of "0 of 0 ... 0.0 %" (test on the data row with `secondary.of == 0`).
- Every value has a visible source line with run date and revision (swell cells name a commit hash); the claim "every cell links" is true or reworded; "engine catalog" labels open the cited line.
- The V8 intl402 and built-ins figures are verified against javascript-zoo-data, and the cell says what the build lacked or is marked "as published".

### Story 2: Empty rows collapse and the table reads on a phone
Outcome: the default view shows data first; "not measured" and "swell only" rows take one line each; the table is usable at 55 columns and by keyboard.
Covers: 8, 9, 10, 17, 18, 19, 20, 21, 22, 23.
Acceptance:
- A KPI with all cells "not measured" renders one full-width row with its reason (K5: no shell binary); swell-only rows span the other columns; the default view has at most 10 rows with cells of "not measured", "unknown" or "not applicable" (now 30 of 100 cells).
- At 440 px width the row label stays visible while scrolling; the scroll region is focusable and labelled, the table has a caption, the "Other engines" picker is collapsed, and a full or empty selection prints why a box is disabled.
- An unknown id or a one-id `?e=` prints a message.

### Story 3: Words and entry points a first-time reader can follow
Outcome: nobody needs to know K-numbers, "zoo runner" or "slice" to read the comparison, and the landing page leads to it.
Covers: 14, 15, 16, 24, 26, 27, 28.
Acceptance:
- /docs/kpis headings read "K1 · test262 conformance" and so on, with a 9-row summary table on top; /compare has a short terms list (KPI, zoo runner, JIT reference, flags unverified); the picker group is not called "Default set".
- The landing page body links to /compare (with the default `?e=`) in section "Compared to others" and in the hero, and its 57.8 % states the 48.2 % reading or links to #k1.
- A link check over the three pages (internal links, anchors, line anchors into swell files) runs in the site gate and passes.

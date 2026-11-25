---
format: aep.planning-md/3
id: story:compare-page-with-published-data
kind: story
status: active
title: The compare page and the KPI page, with published data and swell's own values
summary: engines.json from the catalog, javascript-zoo test262 counts and swell's committed files; /compare and /docs/kpis
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T12:46:20Z", actor: "human:operator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-25T12:46:35Z", actor: "human:operator", revision: 4}
---
The owner, 2026-11-25: evaluate the KPIs for other ECMAScript engines and for swell, show them
side by side on a page where one picks engines, and describe every KPI on a page of its own.
Design: `docs/design/quality-and-comparison.md` §§ 1, 3, 4, 5, 6 (stories 1 to 5 of § 9 in one
unit). Base: `plan/native-code` at `541d32e`.

## Context

- The KPIs K1 to K9 and the facts rows are defined in `quality-and-comparison.md` § 1, agreed
  with the ratchet maintainers on 2026-11-25.
- The site has no data about any other engine. `task site-data` (`taskfiles/site.yml`, POSIX sh
  and awk) writes `website/data/status.json`; `site-data-check` compares a regeneration byte for
  byte. The site types no number (`website/docusaurus.config.ts:14-16`).
- A catalog of 35 engines with a source per field exists outside the tree (the coordinator
  hands it over as `engines.jsonl`).

## Change

Failing checks first (Acceptance). Then:

- `bench/engines.jsonl`: the catalog, one engine per line, a source URL per field, `unknown`
  where no source says; the default set (V8, SpiderMonkey, JavaScriptCore, Hermes, QuickJS,
  QuickJS-ng, Boa, XS) marked.
- `task bench-import-zoo` (manual, network, POSIX sh, curl, jq): fetches javascript-zoo-data's
  `test262/<engine>.json` for the engines of the catalog it covers into `.cache/`, writes
  `bench/published/zoo-test262-4a1e962.tsv` (per engine and top directory: passed, failed, total,
  data commit, sha256 of the fetched file, fetch date, URL) and keeps the MIT notice in
  `bench/published/LICENSE-javascript-zoo-data`. Run once; its output is committed.
- `task site-data` also writes `website/data/engines.json` from `bench/engines.jsonl`,
  `bench/published/*.tsv`, `bench/results/*.json` (format `swell-bench/1`, none yet) and swell's
  own committed files (`conformance/test262/expected.txt`, `conformance/host/requirements.txt`,
  `Cargo.lock`, the forbid scan). Every value carries source, date, revision and method;
  `site-data-check` covers it.
- `website/src/data/engines.ts` (typed reader), `website/src/pages/compare.tsx` at `/compare`
  with a navbar item "Compare": pick 2 to 4 engines (default swell, V8 jitless, QuickJS, Boa),
  selection in the URL query `?e=`, rows grouped by KPI, direction per row, best picked cell
  marked, "not measured" with its reason, a source link per cell, the K6/K8 capability matrix
  from the catalog, JIT rows hidden by default.
- `website/docs/kpis.md` (sidebar): one section per KPI K1 to K9 and the facts rows: definition,
  unit, direction, gate or compare, how it is measured, the source of each engine's value; linked
  from each row heading of the compare page.
- `.github/planning-paths.txt`: `bench/engines.jsonl`, `bench/published/*` as site paths.

## Acceptance

- `site-data-check` refuses a planted number typed into `engines.json` and a planted edit of a
  `bench/published` line without its sha256 matching; it passes on the regenerated file (cases
  of `gate-selftest`).
- `website/data/engines.json` holds K1 for at least the eight default engines from the zoo data
  at test262 `4a1e962`, labelled with that revision, and swell's K1 from `expected.txt` labelled
  "swell's runner, test262 2a1ade30".
- The site builds (`slot npm --prefix website run build`, or the forge's docs job) and `/compare`
  and `/docs/kpis` render; `slot --timeout 30m task check-runner` exits 0.

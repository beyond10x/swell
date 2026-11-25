---
format: aep.planning-md/3
id: story:local-gate-builds-the-site
kind: story
status: draft
title: The local gate builds the site as the forge does
summary: A docs build failure is found by task check, not by a push
revision: 2
---
Found on 2026-11-25 by `story:compare-page-with-published-data`: the forge's job "Build
documentation" failed on `website/docs/kpis.md` ("MDX compilation failed ... Could not parse
expression with acorn", line 25, the heading id `{#k1}`), and `task check` has no step that
builds or compiles the site (`swell-inventory`: "`task check` never builds or type-checks the
site"). The owner asked why a push was needed to notice it. Rule "The local gate equals the
forge" (`~/beyond10x/AGENTS.md`): the fix includes the change that makes the local gate catch the
same failure.

## Context

- `.github/workflows/pages.yml` builds the site with `npm ci` and `npm run build` in `website/`;
  its `package.json` takes `docs-system` as a git dependency over SSH.
- `task check` runs through the pool on the build node; the site build compiles TypeScript and
  MDX, so it is a pool job too.

## Change

- `task site-build`: `npm ci` and `npm run build` in `website/`, with the same Node version and
  lock file as `pages.yml`, through the pool (`slot --timeout 30m`), and a step of `task check`
  when a path under `website/` or a doc it builds changed, or of `check-runner` always.
- If the build node cannot fetch `docs-system`, the task names that and the refusal is a defect
  of this story, not a skip.
- A `gate-selftest` case: a planted `{#x}` heading id in a copy of a doc is refused.

## Acceptance

- `slot --timeout 30m task site-build` exits non-zero on the planted heading id and 0 on `main`.
- `slot --timeout 30m task check-runner` exits 0 and includes the site build.

---
format: aep.planning-md/3
id: story:bench-runner-and-data-file
kind: story
status: draft
title: The benchmark runner writes swell-bench/1 result files
summary: One process per measurement; per-iteration times in bench/results/<date>-<commit>.json, checked by a schema test
revision: 2
---
Proposed by `docs/design/native-code.md` § 5 and § 7 (story 4). One harness measures every
engine, agreed with the codespec maintainers on 2026-11-25 (`native-code.md`, Q12): codespec's
ahead-of-time outputs are rows in the same schema. Draft: scope, change and acceptance are
written when the design is accepted.

## Outcome

- `bench/` runs one process per measurement, through `slot --timeout 30m` on the build node,
  and writes per-iteration times to `bench/results/<date>-<commit>.json`.
- The schema is named and versioned, `swell-bench/1`, so codespec pins it; a schema test checks
  every committed result file.
- A row carries engine, version, flags, suite, benchmark, runs, host, date and commit; a script
  that codespec refuses is a row with the refusal reason, never a missing row.
- codespec hands `bench/` a runnable artifact per script (`.wasm` and its runner command, later
  an ELF) built from a pinned codespec release.

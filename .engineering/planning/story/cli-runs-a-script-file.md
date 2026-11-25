---
format: aep.planning-md/3
id: story:cli-runs-a-script-file
kind: story
status: draft
title: A swell shell binary runs a script file
summary: crates/cli runs a file with print, a host clock and limit flags, and exits with a status per outcome
revision: 2
---
Proposed by `docs/design/native-code.md` § 7 (story 2) and `docs/design/quality-and-comparison.md`
§ 7 and § 9 (story 7). swell is a library today and has no binary, so no engine comparison can
run it from a command line, KPI K5 (binary size) has nothing to measure, and codespec's
`native-speed-comparison` and its oracle run (beyond10x/codespec#68) show swell as "not
measured". Draft: scope, change and acceptance are written when the design is accepted.

## Outcome

- `crates/cli` builds a `swell` binary that runs one script file: `print`, a host clock, flags
  for the step budget, memory budget and depth limits, and one exit status per outcome (normal,
  throw, terminated by each limit, interrupted).
- The engine crates keep `#![forbid(unsafe_code)]` and no third-party crate.
- The test262 digest is narrowed to what the test262 runner depends on, so a change to the shell
  alone does not force a full test262 run (`quality-and-comparison.md`, Q1).

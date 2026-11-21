# Foundation, wave 2 of 3: the counting runner

One wave of the [wave plan](waves.md), 2026-11-21. Two units, each implemented by a sub-agent
on Opus in its own worktree; the coordinating session wrote no code. It added `- task:
test262` to `check` and four names to `GATE_VARIABLES` in the root `Taskfile.yml`.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `test262-counting-runner` | `crates/test262` (package `swell-test262`, std only, 1,129 lines in `src/`), `conformance/test262/features.txt` (199 names: 165 `run`, 34 `exclude`), `taskfiles/test262.yml` | 17 tests; `test262 total 53801 8982 7 44812 0 44812 0` | `399d4f4` |
| `host-requirements-pending` | three states of a requirement line (`test`, `pending slice <s>`, `omitted <reason>`), `conformance/slices-delivered.txt`, 11 planted tests | `0 test, 148 pending, 0 omitted`; 11 of 11 scripts | `e4ed5ab` |

Each unit's claim was checked by the coordinator before its commit by running its tests and
its task again in the unit's tree. For the runner also: the slice lines sum to 53,801 cases
and 103,360 runs (`awk` over `.out/test262/lines.txt`), and the SHA-256 of the sorted `meta`
output is `c39e4bbe96533198762babfe6e13f04e27b76629fe505dbb4db423204836684a`.

Beside the units, on the same branch: nine requirements of `docs/design/host-interface.md`
were split so that each can be met in the slice it is marked with (159 requirements, 68 for
slice 1 as before; `c359d26`), and the plan of slice 1 (ten draft stories, critic round 1).

`host-requirements-pending` was added to the plan in this wave, after the session that
decides for the operator ruled that an unimplemented requirement must not look like a
deliberate omission. It is one story added to an epic whose other stories went through the
critics twice; the panel was not run again for it.

## What went wrong, and what was decided

- **The design's reader misreads one case.** `meta.awk` splits lines at LF only; the case
  `test/built-ins/Function/prototype/toString/line-terminator-normalisation-CR.js` ends its
  lines with CR alone, is read as one line, and loses its `includes:`. The story asked for a
  reader equal to `meta.awk` byte for byte and for no unreadable front matter; the two cannot
  both hold. The unit kept the error line and reported. Decided: the runner reads CR LF, CR
  and LF as line ends; the story and the design carry the corrected hash. No count changed.
- **A fixture's line ends would have been normalised on commit.** The repository has
  `core.autocrlf=input`; the unit added `-text` for the fixture directory, without which the
  CR LF fixture would be committed with LF and its test would check nothing.
- **Requirements that could never close.** Nine requirements marked for slice 1 named a
  feature of a later slice (`eval`, `let`, modules, `globalThis`). With the gate failing on a
  requirement still pending for a delivered slice, slice 1 could not have closed. Found by the
  slice 1 decomposition; split by the design author.
- **Slice 1 is 31 cases, not 82** (see the wave plan). The runner prints `test262-slice 01 82
  164 0 0 82 0 82 0` until `story:test262-run-path` moves the 51 cases.

## The gate on the wave branch

`slot task check`, exit 0:

```
gate: variables in effect: SPEC_REFUSALS_MAX=32 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log
python-check: no Python file among 195 file(s)
forbid-unsafe-check: 3 crate root(s), each with #![forbid(unsafe_code)]
Cargo.lock: 2 package(s), none from outside this repository
test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked
data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present
swell v1 — 2 file(s), 3 scenario(s), valid
61 scenario(s) (3 authored), 32 refusal(s), written to .out/suite.json
  61 scenarios: 61 passed, 0 failed, 0 error, 0 unsupported
host-requirements: 159 requirement(s), 159 line(s): 0 test, 159 pending, 0 omitted; 50 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 1:68 2:5 3:5 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1
6 test suites: 17 passed, 0 failed, 0 ignored
test262 total 53801 8982 7 44812 0 44812 0
test262: 221 directory lines sum to the total
test262: 22 slice lines equal the design
test262-errors 0
gate-selftest: 24 passed, 0 failed
```

Before the wave: 148 requirements all `omitted`, 2 test suites with 0 tests, no test262
count, 22 planted tests.

## Findings, and where each goes

| finding | from | goes to |
|---|---|---|
| no planted-defect script covers the two failure paths of the task `test262` (a feature missing from `features.txt`, a slice line that differs from the design); both were shown by hand | `test262-counting-runner` | `story:test262-expected-and-sections` (wave 3), which owns the comparison with the expected file |
| a malformed front matter is a counted error and does not fail the task | `test262-counting-runner` | stays: the story asks for it; the expected file of wave 3 holds `test262-errors 0` |
| the `not a slice number` fault of the delivered file has no planted script | `host-requirements-pending` | backlog draft, minor |
| two markers of `spec/` name `HOST-71` for what is now `HOST-157`, and four may name new identifiers | the design author | the next change to `spec/` |
| `host-interface-model.md`'s marker table lists the identifiers before the split | the design author | the same change |

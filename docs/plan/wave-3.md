# Foundation, wave 3 of 3: sections and the expected file

One wave of the [wave plan](waves.md), 2026-11-21. One unit, implemented by a sub-agent on
Opus in its own worktree; the coordinating session wrote no code. It added two names
(`TEST262_SECTIONS`, `TEST262_EXPECTED`) to `GATE_VARIABLES` in the root `Taskfile.yml`.

## Unit

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `test262-expected-and-sections` | `conformance/test262/sections.txt` (33 sections of ECMA-262, 17,010 cases, three `omit` lines), `conformance/test262/expected.txt` (278 lines), `crates/test262/src/{sections,expected}.rs`, eight tests, four planted-defect scripts for the task `test262` | 25 tests (17 before); `test262: 278 lines equal conformance/test262/expected.txt`; `gate-selftest: 28 passed` | `357075c` |

The unit's claim was checked by the coordinator before its commit: `cargo test -p
swell-test262 --locked`, `slot task test262` and `slot task gate-selftest` run again in the
unit's tree (exit 0 each), 33 `test262-section` lines in `.out/test262/lines.txt`, each
`incomplete`, the five lines the story names present in full, and `cmp` of the printed lines
with `expected.txt` equal.

The 278 lines: 221 directory lines, 1 total line, 22 slice lines, 33 section lines and the
`test262-errors` line. A section is `complete` only when each of its cases passes and it has
no omission; every section is `incomplete` while the engine runs nothing.

Beside the unit, on the same branch: the plan of slice 1 (`epic:slice-01-harness`, ten
stories, accepted after two rounds of the plan critics, six waves; see the wave plan).

## What went wrong, and what was decided

- Nothing in the unit failed. The story's count of 278 lines was computed before the runner
  landed; the unit measured it again on the base of the wave and it holds.
- **What the unit decided where the story left a choice.** The comparison with the expected
  file is by position (line i with line i), so one added directory line also reports every
  later line as different. A section with no `path` line is refused; a section whose paths
  match no case is refused; a section of 0 passing cases is `incomplete`. `sections.txt`
  takes no blank line, as `features.txt` does.
- **Not run red.** The four planted-defect scripts were written after the task change; the
  unit did not run them against the task as it was before. Each passes the unchanged input
  first and then fails the task on the planted input, which the gate's self-test runs.
- **Open wording in the design.** Decision 11 says a section line prints `complete` or
  `incomplete` "with the reason"; the line carries the case count, the pass count and the
  number of omissions. Read as the reason; the design text is unchanged.

## The gate on the wave branch

`slot task check`, exit 0:

```
gate: variables in effect: SPEC_REFUSALS_MAX=32 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log
python-check: no Python file among 229 file(s)
forbid-unsafe-check: 3 crate root(s), each with #![forbid(unsafe_code)]
Cargo.lock: 2 package(s), none from outside this repository
test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked
data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present
swell v1 — 2 file(s), 3 scenario(s), valid
61 scenario(s) (3 authored), 32 refusal(s), written to .out/suite.json
  61 scenarios: 61 passed, 0 failed, 0 error, 0 unsupported
host-requirements: 159 requirement(s), 159 line(s): 0 test, 159 pending, 0 omitted; 50 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 1:68 2:5 3:5 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1
8 test suites: 25 passed, 0 failed, 0 ignored
test262 total 53801 8982 7 44812 0 44812 0
test262: 221 directory lines sum to the total
test262: 22 slice lines equal the design
test262-errors 0
test262: 278 lines equal conformance/test262/expected.txt
gate-selftest: 28 passed, 0 failed
```

## The foundation epic

With this wave every acceptance item of `epic:foundation` holds on the wave branch: the total
line with 53,801 cases, the 22 slice lines equal to the design, the printed lines equal to
`expected.txt` with a planted test for a changed count on each side, a planted test for a
feature missing from the list, for a different tree hash, for an outside package, for a
changed Unicode file and for each case of the host requirements list, and the specification
validated with its suite at the refusal ceiling.

## Where each finding went

- The design's "with the reason" for a section line: to the next change of
  `docs/design/engine.md`, decision 11.
- Carried from wave 2 and still open: two markers in `spec/` name `HOST-71` for what is now
  `HOST-157`, and the marker table of `docs/design/host-interface-model.md` predates the
  split of nine requirements. Both go with the next change to `spec/`.

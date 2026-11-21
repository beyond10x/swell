# Foundation, wave 1 of 3: the gate and the pins

One wave of the [wave plan](waves.md), 2026-11-21. Five units, each in its own worktree, each
implemented by a sub-agent on Opus; the coordinating session wrote no code, no test and no
specification file. It wrote the entries of `check` and the task `gate-vars` in the root
`Taskfile.yml`, and one number in `taskfiles/spec.yml` (the refusal ceiling, 33 to 32).

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `test262-fetch` | `conformance/test262/PIN`, `taskfiles/test262-fetch.yml`, 4 planted tests | `test262 2a1ade3…: present and checked`; 4 of 4 scripts `ok` | `2b7eecd`, `507228b` |
| `unicode-data-pin` | `data/unicode/17.0.0/` (14 UCD files, 2 emoji sequence files, `LICENSE`, `SOURCE`), `taskfiles/unicode-data.yml`, 3 planted tests | `17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present`; 3 of 3 | `b7ed26f` |
| `workspace-and-gate` | root workspace, `crates/swell` with no public item, `Taskfile.yml` with `check`, 5 planted tests | `slot task check` exit 0, `gate-selftest: 5 passed, 0 failed` | `9944495` |
| `spec-gate` | `taskfiles/spec.yml` (validate, suite under a ceiling, conform with a check of the summary line), 1 planted test | `64 scenarios: 64 passed, 0 failed, 0 error, 0 unsupported`, 33 refusals (the first model) | `e807a55` |
| `host-requirements-gate` | `conformance/host/requirements.txt` (148 lines), `taskfiles/host-requirements.yml`, 9 planted tests | `148 requirement(s), 148 line(s): 0 test, 148 omitted; 50 UNMAPPED marker(s)`; 9 of 9 | `f326ec1` |

Each unit's claim was checked by the coordinator before its commit, by running the unit's
task and its scripts again in the unit's tree; for `unicode-data-pin` also with `sha256sum -c`
over `SOURCE`.

Beside the units, on the same branch: `docs/design/host-interface.md` (19 decisions, 148
requirements `HOST-1` to `HOST-148`, 68 for slice 1), `docs/design/host-interface-model.md`
(the record of what the first ESS model measured), and the model's second round (every
`UNMAPPED:` marker names its requirement or `ESS limit`; six contradictions with the document
removed: 61 scenarios, 32 refusals, before 64 and 33).

## What went wrong, and what was decided

- **A gate task took its inputs from the caller's environment.** Task takes a variable from
  the environment when none is passed on the command line. Measured: with `CACHE` exported,
  `test262-fetch` ignored the pinned checkout and tried to fetch into the exported directory;
  with `ROOT` exported, `python-check` listed an ignored empty directory and passed on 0
  files; with `OUT` and `REFUSALS_MAX` exported, `suite` wrote elsewhere under a raised
  ceiling. Found by `unicode-data-pin`, sent back to all four units of that time. Every gate
  variable now carries a prefix (`GATE_`, `TEST262_`, `UNICODE_DATA_`, `SPEC_`, `HOST_REQ_`),
  and `check` starts with `gate-vars`, which prints the gate variables that have a value. An
  exported variable of a prefixed name still redirects a check: Task cannot tell an exported
  value from an argument. Accepted as it stands.
- **A check that reads nothing passed.** The same `python-check` run passed on 0 files. Every
  check of the gate now fails when it finds nothing to read (no file, no crate root, no test
  result line, no planted script, a missing document, list or specification directory).
- **A story named a string the tool never prints.** `workspace-and-gate` asked for
  `clippy::unwrap_used` in clippy's output; clippy 1.99.1 prints `-D clippy::unwrap-used`. The
  unit left its test red and reported; the story was corrected.
- **Task stops a command script at the first failing command**, and a `grep` that matches
  nothing fails. `spec-gate` found that its ceiling check would have failed the gate at 0
  refusals and that two failure branches stopped before printing their message; fixed in the
  unit, with planted cases.
- **In an included task file, a plain `vars:` value ignores a value given on the command
  line** (Task 3.54). Variables are written as `'{{.X | default "..."}}'`.
- **The specification moved under a unit.** `spec-gate` measured the first model (64, 33)
  while the second round was written; the ceiling was set to 32 on the wave branch, where the
  gate printed 61 and 32.
- **The landing script refused the Unicode data**: its scan for a personal name matched
  inside `ANTIMONY`. The scan now leaves `data/unicode/` out and matches whole words.
- **Rate limits cut sub-agents off four times.** Each was resumed with its context on the
  same model; no work was repeated.

## The gate on the wave branch

`slot task check`, exit 0, with all five units merged:

```
gate: variables in effect: SPEC_REFUSALS_MAX=32 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log
python-check: no Python file among 114 file(s)
forbid-unsafe-check: 1 crate root(s), each with #![forbid(unsafe_code)]
Cargo.lock: 1 package(s), none from outside this repository
test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked
data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present
swell v1 — 2 file(s), 3 scenario(s), valid
61 scenario(s) (3 authored), 32 refusal(s), written to .out/suite.json
  61 scenarios: 61 passed, 0 failed, 0 error, 0 unsupported
host-requirements: 148 requirement(s), 148 line(s): 0 test, 148 omitted; 50 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
2 test suites: 0 passed, 0 failed, 0 ignored
gate-selftest: 22 passed, 0 failed
```

Before the wave there was no gate.

## Findings, and where each goes

| finding | from | goes to |
|---|---|---|
| an exported variable of a prefixed name still redirects a check | four units | accepted; `gate-vars` shows it in the gate's output |
| the `SPEC_` variables are global in their task file, so `gate-vars` always lists them with their defaults | the coordinator | accepted; the task's summary says so |
| the UTS #51 text at the allowed address is version 18.0, not 17.0; it names the same two files, and the 17.0 files hold all six type fields | `unicode-data-pin` | `story` of slice 5 that writes the generator: confirm against `tr51-29.html` |
| `LICENSE` is today's `license.txt` (Unicode License V3); whether release 17.0.0 carried another notice was not checked | `unicode-data-pin` | the same story; `NOTICE` names the data now |
| the Unicode licence asks that its notice go with copies of the data and, since tables are derived from it, likely with the generated tables | `unicode-data-pin` | slice 5: the generated tables carry the notice |
| a failed fetch leaves an initialised directory that later runs refuse until it is removed | `test262-fetch` | stays: the story asks that a failing directory is never removed |
| no script covers `not a checkout`, a list out of order, or a document line that starts `**HOST-` and is no requirement | two units | backlog drafts, minor |
| `requirements.txt` is said to be sorted and nothing checks it | `host-requirements-gate` | backlog draft, minor |
| test names of `test` lines are not checked; every `test` line is refused | `host-requirements-gate` | the first slice 1 story that closes a requirement |
| the ECMA-262 pin has no file and no check; the page of a past commit cannot be fetched again byte for byte | the foundation's decomposition | backlog draft |
| the marker list in `host-interface.md` cites line numbers of the first model | the specification author | stays: the gate matches markers by content, not by line |
| one marker's sentence on parse failures is out of date after `HOST-137` and `HOST-138` | the specification author | the next change to `spec/` |

## Beside the wave

- The plan: `epic:foundation` and its seven stories went before the four plan critics twice
  (seven findings, all fixed) and were accepted by the coordinator on the operator's word.
- `epic:slice-01-harness` is being decomposed against `host-interface.md`.

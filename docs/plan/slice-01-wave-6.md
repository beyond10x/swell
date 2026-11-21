# Slice 1, wave 6 of 6: the model's scenarios run on the engine; slice 1 is delivered

One wave of the [wave plan](waves.md), 2026-11-21. One unit, implemented by a sub-agent on
Opus in its own worktree, and a change of the specification it asked for, written by a
specification author; the coordinating session wrote no code and no specification file. It
added the include `taskfiles/conform-engine.yml`, `- task: conform-engine` (after
`test-faults`) and the variables `ESS_SUITE` and `ESS_UNSUPPORTED_MAX` to the root
`Taskfile.yml`, and the line `1` to `conformance/slices-delivered.txt`. Every commit is the
organization bot's.

Between waves 5 and 6 the first release was cut: `0.1.0`, tagged on the closing commit of
wave 5 (`6319af2`), [`CHANGELOG.md`](../../CHANGELOG.md).

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `ess-conformance-target` | `conformance/ess`, a workspace of its own (1,469 lines; its lock lists 52 packages: 8 of the ess repository at tag 0.96.0, 38 from crates.io, 6 local). Its target maps `CreateAgent`, `CreateRealm`, `ParseScript`, `EvaluateScript`, `Interrupt` and `Collect` to the host interface of `crates/swell`, in process, and runs the suite synthesized from `spec/`. A planted-defect test: a target whose `Interrupt` leaves the flag unset must fail a scenario | `58 scenarios: 14 passed, 0 failed, 0 error, 44 unsupported`; 12 tests of the crate | `d27bacd` |
| (specification) | `spec/`: a Realm carries its agent; `ParseScript` refuses a realm of another agent with `foreign-realm` (`HOST-45`); five authored scenarios on the realm's own agent, two corrected ones; every source ends its statements with a semicolon | `58 scenario(s) (8 authored), 41 refusal(s)`; 58 of 58 on ESS's interpreter | `72e6d8c` |

The unit's claim and the specification's were checked by the coordinator before each commit
by running the tasks `suite`, `conform` and `conform-engine`, the planted-defect test, the
crate's tests and clippy on its workspace again (exit 0 each).

**What works now.** `slot task check` runs the model's scenarios against the engine. Run for
real: a parse on the realm's own agent, a realm of another agent, a syntax error, a thrown
value, an interrupt set while nothing runs (it ends the next run and that run clears it,
`HOST-91`, `HOST-92`), and a fault injected at `collect`. The 44 unsupported scenarios name
their reason: a command of a later slice (modules, jobs, promises), or a forced outcome the
engine cannot produce with the scenario's own means (a memory budget passed while parsing,
an engine fault inside `evaluate_script`).

**Slice 1 is delivered.** Its 31 test262 cases pass in all 62 runs, no host requirement is
pending for it, and `conformance/slices-delivered.txt` names it, so the gate fails if a
requirement is ever pending for slice 1 again.

## What went wrong, and what was decided

- **The first run gave 9 passing scenarios of 61; the story expected 13.** Two causes, each
  enough alone. The synthesized scenarios parsed on one agent with a realm created on another:
  the model's Realm carried no agent, so synthesis paired any realm with any agent, and the
  engine answers `ForeignHandle`, for which the model had no outcome. And the model's example
  source, `1 + 1`, has no semicolon, which the parser of slice 1 refuses. The planted-defect
  test could not fail anything for the same reasons. The unit reported it and did not bend
  the target to the story's numbers.
- **Decided: correct the model, not the story.** A target that passes only the scenarios that
  refuse a call checks nothing about parsing, running or interrupting. The specification
  author stated whose realm it is and wrote authored scenarios for what synthesis could no
  longer reach.
- **ESS 0.96.0 cannot synthesize a realm that belongs to the called agent** once the guard
  exists (`ESS-SYNTH-003`; `ESS-SYNTH-004` for every outcome after a parse). Ruled a defect of
  ESS by the session that decides for the operator and filed by it as issue 801 of
  `beyond10x/ess` (an owner-link guard beside an owner-existence guard is not witnessed). The
  authored scenarios are the workaround. A script of another agent at `EvaluateScript` stays
  `UNMAPPED`: a command reads one related row (`ESS-COMMAND-004`), a documented limit.
- **`step-budget-ends-one-run` evaluated `for (;;) {}` twice and expected the second run to
  complete.** On ESS's interpreter the outcome was forced; no engine completes that script.
  It now completes a second script after the budget ended the first.
- **The gates refused the commit of the scenarios.** Each scenario file that creates an agent
  carries the test value of `hash_key`, which the secret scan reports, and the policy's
  exception binds the SHA-256 of the whole file, not the line (`AGENTS.md` said "line";
  corrected). Seven files were admitted by the session that decides for the operator before
  the commit; nothing was bypassed. A seed the scanner does not report would end the round
  trip: the wave plan, "What later slices take over".
- **A run that fills 64 MiB with `a.push({})` did not finish** in about nine minutes in a debug
  build (`memory-budget-ends-one-entry`, tried by the unit outside the gate: 0.5 s for 1 MiB,
  1.2 s for 2 MiB, 4.1 s for 4 MiB). The time grows faster than the memory. Not measured
  further in this wave: the wave plan, "What later slices take over".
- **The target depends on `swell-syntax`** beside `swell`, to tell a refusal from a syntax
  error with `message_kind`. Accepted; the story named `swell` only.
- **Numbers of the story**: 13 passed and 48 unsupported of 61 became 14 passed and 44
  unsupported of 58, measured on the corrected model. The task's floor and ceiling are those
  numbers.

## The gate on the wave branch

`slot task check` with the variables of the runner, exit 0:

```
gate: variables in effect: SPEC_REFUSALS_MAX=41 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log ESS_SUITE=.out/suite.json ESS_UNSUPPORTED_MAX=44
python-check: no Python file among 849 file(s)
forbid-unsafe-check: 8 crate root(s), each with #![forbid(unsafe_code)]
Cargo.lock: 7 package(s), none from outside this repository
test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked
data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present
unicode-tables: 1 table(s) equal the generator output
swell v1 — 2 file(s), 8 scenario(s), valid
58 scenario(s) (8 authored), 41 refusal(s), written to .out/suite.json
  58 scenarios: 58 passed, 0 failed, 0 error, 0 unsupported
host-requirements: 164 requirement(s), 164 line(s): 70 test, 94 pending, 0 omitted; 51 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 2:10 3:3 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1
55 test suites: 268 passed, 0 failed, 1 ignored
test-collect: 31 test suites: 194 passed, 0 failed, 1 ignored
test-faults: 1 test suites: 1 passed, 0 failed, 0 ignored
conform-engine: planted defect refused: test a_target_whose_interrupt_does_nothing_fails_a_scenario ... ok
  58 scenarios: 14 passed, 0 failed, 0 error, 44 unsupported
test262-refused 53564
test262 total 53801 8982 7 44812 3969 40843 0
test262: 221 directory lines sum to the total
test262-slice 01 31 62 0 0 31 31 0 0
test262: 22 slice lines equal the design
test262-errors 0
test262: 283 lines equal conformance/test262/expected.txt
gate-selftest: 31 passed, 0 failed
```

The `python-check` line is from that task run again on the committed tree: the gate ran with
830 files, before this record, the evidence files and the accepted stories of slice 2 were
added. No code, test, task file, specification or `expected.txt` changed after the run.

## Where each finding went

- The seed of the scenarios, the wording "defect of ESS, issue 801" in the model's notes, and
  the time of a run that fills its memory budget: the wave plan, "What later slices take
  over".
- What the policy binds an exception to: `AGENTS.md`, "Publishing".
- Slice 2: decomposed during this wave into six stories, read by four plan critics in two
  rounds; its design changes (`20c1f35`) are on `main` with this wave.

# Slice 1, wave 5 of 6: test262 runs; the host interface under misuse

One wave of the [wave plan](waves.md), 2026-11-21. Two units, each implemented by a sub-agent
on Opus in its own worktree; the coordinating session wrote no code. It regenerated
`Cargo.lock` on the wave branch (7 packages), added the include
`taskfiles/fault-injection.yml` and `- task: test-faults` (after `test-collect`) to the root
`Taskfile.yml`, and added `TEST262_WORKERS` and `TEST262_RUN` to `GATE_VARIABLES`. Every commit
is the organization bot's.

The plan had `test262-run-path` in wave 6. It was moved here: after wave 4 it depended on
nothing that was missing (a probe had run the 31 cases by hand, 30 passing), and it writes
`crates/test262` while the hardening story writes `crates/swell/tests`. Wave 6 keeps
`ess-conformance-target`.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `test262-run-path` | `swell-test262` runs cases: a fresh agent and realm per run, `assert.js` and `sta.js`, the includes, the case; once as written and once strict unless the flags say otherwise; negative cases judged by phase and type; a run ended by a budget counted as budget. The host gives `print` and `$262` (`createRealm`, `evalScript`, `gc`, `global`). Workers run on threads of `min_stack_bytes()`. The 51 cases of `test/harness` that need a later slice move by rule lines with their reasons. `swell-syntax` exports `message_kind` | `test262-slice 01 31 62 0 0 31 31 0 0`; 283 lines | `0e25ad8` |
| `host-interface-hardening` | Six test files drive the host interface of slice 1: a seeded generator of call sequences with stale, released, foreign and wrong-kind handles (2,000 sequences, each run twice); the stale, foreign and faulted answer of every entry point; an injected broken invariant at `parse_script`, `create_realm` and `collect` behind the feature `fault-injection`; a corpus run by two agents and by two child processes, compared byte for byte; the limits that need builtins, with `min_stack_bytes()` held through four builtins that re-enter. `HostObjectSpec.prototype` is the enum `Prototype`. 15 host requirements move to `test` | `70 test, 91 pending`, none for slice 1; `test-faults: 1 test suites: 1 passed` | `49bb165`, and the second part |

Each unit's claim was checked by the coordinator before its commit by running its tests, its
tasks and clippy on the workspace again in the unit's tree (exit 0 each).

**What works now.** `slot task check` runs every test262 case that is not excluded, in both
modes, and holds the counts: 3,969 of 44,812 run cases pass, and slice 1 passes its 31 cases
in all 62 runs. No host requirement is pending for slice 1.

## What the pass number does and does not say

- **A refusal is never a pass.** The parser refuses a construct of a later slice with a
  `SyntaxError` that names the slice. The unit first counted such a refusal as the
  `SyntaxError` a negative case asks for: 3,011 negative cases of later slices passed, and
  the total read 6,980. Not accepted: the runner now reads the kind of the message
  (`message_kind`: grammar, refusal, nesting limit), and a refusal or the nesting limit fails
  the run. The total is 3,969. The runner prints how many runs were refused:
  `test262-refused 53564` of 85,556 runs.
- **269 negative cases pass on a generic error** ("unexpected token" and the like), where the
  parser rejects the source for a reason that may not be the one the case tests. test262 asks
  for phase and type only, so they count; a later slice that parses more may turn some red.
- **129 passing cases hold no assertion in their body**: they pass by not throwing.
- **The slices are not independent in the number.** Most cases outside slice 1 fail on the
  first refused construct; what passes there is what happens to need only the ES5 grammar and
  the harness builtins.

## What went wrong, and what was decided

- **A defect of the engine, found by the fuzz test**: a realm whose `create_realm` failed stayed
  usable through the `RealmRef` its setup closure had received, and a script parsed into it
  faulted the agent. Such a realm is marked unmade and answers `StaleHandle`. The story was
  extended beforehand so that the unit may fix what its tests expose.
- **`HOST-49` could not be met by the interface.** The setup closure of `create_realm` had no
  way to name the new realm's `%Object.prototype%`. Decided by the coordinator, written by the
  design author (`25a13fb`): `HostObjectSpec.prototype` is `Prototype { Object, Null,
  ObjectPrototypeOf(RealmRef) }`; an accessor for any intrinsic was rejected. The unit
  implemented it in a second part; the realm handle is checked like any other.
- **Inside a scope that is on the stack when the agent faults** (`HOST-160`, text beside
  `HOST-48`): a scoped method whose type carries `HostError` answers `Err(Faulted)`;
  `new_string` and `new_string_utf16`, whose type is `Completion` alone, answer
  `Terminated(EngineFault)`. The test expects one answer per kind.
- **`$262.createRealm` is a workaround.** The interface has no `Cx::create_realm`: the runner's
  host makes its realms before the case runs and hands out the next one at each call; a run
  that calls it more often is run again from the start with more realms, and every run pays
  for the realms made for it (`crates/test262/src/host.rs`). What replaces it is a way for a
  host function to create a realm, which the interface does not have: a design change for the
  slice that needs it (slice 3, where the cross-realm cases of the builtins start).
- **The mode that collects at every safe point took 389 seconds** for the crate's tests after
  the first part. It takes about a minute. Three tests are smaller in that mode only, each
  with its measured reason in its doc comment: the fuzz runs 150 seeds (2,000 take 170.9 s
  there, 24.6 s in the default mode), the stack test through builtins runs at depth 100
  (99.5 s at 1,000), one budget test leaves out its 1 MiB budget (29.1 s with it).
- **The gate self-test took 373 seconds** once the runner ran cases: every planted script of
  the runner ran all cases. A count-only mode (`--run none`) serves them; 133 seconds.
- **A decision for slice 2**: the global `Function` with its constructor moves from slice 3 to
  slice 2 (`AGENTS.md`, "How the work is run", with its conditions): `propertyHelper.js` needs
  it and the `arguments` object. The 23 `propertyhelper-*` cases of `test/harness`, moved to
  slice 3 in this wave, move to slice 2 at its decomposition.
- A unit ran `python3 --version` once, against the rule; nothing used Python. The rule's text
  in `unit-invariants.md` already names a version probe.
- `cargo fmt` changed import lines in the test files the second part touched.

## The gate on the wave branch

`slot task check` with the variables of the runner, exit 0:

```
gate: variables in effect: SPEC_REFUSALS_MAX=32 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log
python-check: no Python file among 807 file(s)
forbid-unsafe-check: 8 crate root(s), each with #![forbid(unsafe_code)]
Cargo.lock: 7 package(s), none from outside this repository
test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked
data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present
unicode-tables: 1 table(s) equal the generator output
swell v1 — 2 file(s), 3 scenario(s), valid
61 scenario(s) (3 authored), 32 refusal(s), written to .out/suite.json
  61 scenarios: 61 passed, 0 failed, 0 error, 0 unsupported
host-requirements: 161 requirement(s), 161 line(s): 70 test, 91 pending, 0 omitted; 50 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 2:5 3:5 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1
55 test suites: 268 passed, 0 failed, 1 ignored
test-collect: 31 test suites: 194 passed, 0 failed, 1 ignored
test-faults: 1 test suites: 1 passed, 0 failed, 0 ignored
test262-refused 53564
test262 total 53801 8982 7 44812 3969 40843 0
test262: 221 directory lines sum to the total
test262-slice 01 31 62 0 0 31 31 0 0
test262: 22 slice lines equal the design
test262-errors 0
test262: 283 lines equal conformance/test262/expected.txt
gate-selftest: 31 passed, 0 failed
```

The `python-check` line is from that task run again on the committed tree: four evidence
files of the plan store were added after the gate's run, which counted 803 files. With
slice 1 named as delivered (`HOST_REQ_DELIVERED` naming a file that holds `1`),
`host-requirements-check` passes with the same two lines.

Passed cases per slice in that run, of the cases the slice has (the lines `test262-slice`):
01 31 of 31, 02 2,125 of 5,245, 03 1,569 of 7,603, 04 13 of 1,198, 05 26 of 2,118, 06 19 of
3,717, 07 4 of 1,432, 08 3 of 6,188, 09 0 of 1,750, 10 2 of 6,573, 11 9 of 1,277, 12 32 of
317, 13 0 of 3,044, 14 0 of 479 run, 15 27 of 1,094, 16 0 of 82, 17 0 of 35, 18 15 of 1,155,
19 94 of 1,474. The step count of the longest runs is printed too: the largest is 16,777,758
steps (`test/staging/sm/JSON/parse-mega-huge-array.js`), far below the budget of 2^30.

## Where each finding went

- The limits of the pass number: this record, and `CHANGELOG.md` for the release.
- A scoped `create_realm`: the wave plan, "What later slices take over", slice 3.
- `Function` in slice 2 and the 23 `propertyhelper-*` cases: `AGENTS.md`, and the wave plan.
- The model's scenarios on the engine: `story:ess-conformance-target` (wave 6).

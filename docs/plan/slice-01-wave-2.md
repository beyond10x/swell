# Slice 1, wave 2 of 6: the grammar, the heap and the first host surface

One wave of the [wave plan](waves.md), 2026-11-21. Two units of the slice and one beside
them, each implemented by a sub-agent on Opus in its own worktree; the coordinating session
wrote no code. It regenerated `Cargo.lock` on the wave branch after the merges (6 packages;
no unit's lock was taken) and added `HOST_REQ_TEST_LIST` to `GATE_VARIABLES` in the root
`Taskfile.yml`.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `syntax-es5-and-strict` | `crates/syntax` (`swell-syntax`, 4,593 lines in `src/`): lexer, AST, a parser for the ES5 grammar in both modes with the early errors of strict code, scope analysis, a nesting counter on every recursive entry. What a later slice delivers is refused with a message naming the slice. Two sections added to `conformance/test262/sections.txt`, each with an omission | 12 tests over 181 source files (46 valid, 61 early errors, 74 refused); `test262: 280 lines equal conformance/test262/expected.txt` | `b391c82` |
| `heap-values-and-host-surface` | `crates/swell` (54 source files, 3,004 lines): `Value` (16 bytes), arenas with `(index, generation)` handles, the collector with one trace function per heap kind and one root site per module, memory accounting, strings, the keyed map, `Agent::new`, roots, host objects, the interrupt handle. 17 host requirements move to `test`; the gate checks that a `test` line names a test that exists | 34 tests and 2 doc tests; 9 more with the feature `collect-every-safe-point`; `17 test, 142 pending` | `c69893f`, `acc90ba` |
| `ci-check-workflow` (beside the slice) | `.github/workflows/check.yml` (Rust 1.99.1 with clippy, go-task 3.54.0 and `ess` 0.96.0 by pinned release assets, then `task check`) and `shared-gates.yml`, byte-equal to surf's | a fresh clone with no `.cache/` and no `target/` passes `slot task check` | `2895971` |

Each unit's claim was checked by the coordinator before its commit by running its tests, its
task and clippy on the workspace again in the unit's tree (exit 0 each), and by a search of
the engine sources for a transcendental `f64` function, `RandomState`, file, network, clock
and environment access, and `unwrap`, `expect` and `panic!` outside tests (none).

Beside the units, on the same branch: the correction of the host interface design described
below (`2854deb`), the additions to four stories that follow from it, and the section
"Publishing" of `AGENTS.md`.

## What went wrong, and what was decided

- **The gate failed once on the wave branch: `gate-selftest: 26 passed, 2 failed`.** The
  scripts `build-refuses-a-compiler-warning` and `clippy-refuses-an-unwrap-in-an-engine-crate`
  copied the real `crates/swell/Cargo.toml` into their planted workspace; with its new path
  dependency on `swell-num` the planted workspace did not load. The unit had run its own
  scripts, not the whole self-test. The finding went back to the unit: each script now writes
  a one-crate workspace from the package and lints sections of the real manifests, with a lock
  of its own, and was shown red with its planted defect removed. Second run of the gate:
  exit 0.
- **A handle from the host had no place for its answer.** `Cx::new_host_object` returned
  `Completion`, which cannot carry `Err(HostError::StaleHandle)`; the unit faulted the agent on
  a stale prototype, against `HOST-30`, and reported it. Decided by the coordinator, written
  by a design author on Opus: every method that takes a handle from the host returns
  `Result<_, HostError>`, on `Cx` and `HookCx` as on `Agent` (decision 5); a stale or foreign
  handle the host *returns* ends the run with `Terminated(EngineFault)`, the rule decision 15
  already had for overrides (decision 6). `HOST-160` holds the second rule for the two return
  sites of slice 1 (a host function, a `GlobalSetup` closure). The code follows in wave 3
  (`object-model-and-realms`) and wave 4; `host-interface-hardening` closes `HOST-160` as its
  fifteenth requirement.
- **`HOST-137` and the missing semicolon.** Its case that parses is `0` inside 10 nested
  parentheses; without a `;` that source needs automatic semicolon insertion, which is slice
  2's and which `swell-syntax` refuses. The test of the syntax unit writes the `;`; the story
  that closes the requirement says so now. The requirement's text is unchanged.
- **Refusals beyond the story's list.** The syntax unit refuses, each naming its slice:
  default and rest parameters (6), trailing commas in calls, hashbang, optional catch binding,
  `**=`, logical assignment, optional chaining (2), private names (8), HTML-like comments, a
  function declaration as the body of an `if`, an initializer in a non-strict `for-in` (18),
  `for await` (10), a non-ASCII code point that starts a token (5). The slice numbers are the
  unit's reading of the design; each is checked when its slice is decomposed.
- **The new sections are appended, not in clause order.** The story asked for "after the 33
  sections, in clause order"; clauses 12.7 and 12.9.5 come before the built-in sections. The
  runner does not depend on the order.
- **`HOST-108`, one case by a unit test.** "An object made by script returns `Ok(None)`"
  cannot be reached from an integration test before script objects exist; a unit test in the
  library covers that path on an ordinary object.
- **`host-requirements-check` builds every test of the workspace** to list their names, before
  the gate's `build` step, and each of the ten older planted scripts of the task builds the
  list again (3 to 8 seconds each on a warm build).
- **The cold run of the gate was not a cold compile.** The unit's run in a fresh clone took 73
  seconds, 37 of them waiting for a build slot, with a compiler cache on the machine. The
  coordinator's run on a shallow clone (`--depth 1`, as the workflow checks out) took 34
  seconds of run time; that the cache was off in it is not proven. The install steps of the
  workflow ran nowhere: the first run on the forge is their test.
- `ci-check-workflow` was added as one story without a critic panel (two workflow files).

## The gate on the wave branch

`slot task check`, exit 0 (27 seconds on a warm build):

```
gate: variables in effect: SPEC_REFUSALS_MAX=32 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log
python-check: no Python file among 514 file(s)
forbid-unsafe-check: 7 crate root(s), each with #![forbid(unsafe_code)]
Cargo.lock: 6 package(s), none from outside this repository
test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked
data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present
unicode-tables: 1 table(s) equal the generator output
swell v1 — 2 file(s), 3 scenario(s), valid
61 scenario(s) (3 authored), 32 refusal(s), written to .out/suite.json
  61 scenarios: 61 passed, 0 failed, 0 error, 0 unsupported
host-requirements: 160 requirement(s), 160 line(s): 17 test, 143 pending, 0 omitted; 50 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 1:52 2:5 3:5 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1
29 test suites: 82 passed, 0 failed, 0 ignored
test262 total 53801 8982 7 44812 0 44812 0
test262: 221 directory lines sum to the total
test262: 22 slice lines equal the design
test262-errors 0
test262: 280 lines equal conformance/test262/expected.txt
gate-selftest: 28 passed, 0 failed
```

No test262 case passes yet: nothing runs a script before wave 4.

## Where each finding went

- The result type of a method that takes a handle, and a handle the host returns:
  `docs/design/host-interface.md`, decisions 5 and 6, `HOST-160`; the stories
  `object-model-and-realms`, `interpreter-and-script-entry-points`, `harness-builtins` and
  `host-interface-hardening` each carry an addition.
- The return sites of later slices (an override's result, `host_get_code_for_eval`,
  `host_get_import_meta_properties`, the other hooks): the wave plan, "What later slices take
  over".
- `HOST-137` and the semicolon: `story:interpreter-and-script-entry-points`.
- The slice numbers of the syntax refusals, and the build of every test inside
  `host-requirements-check`: the wave plan, same list.
- The design's "expected 16 bytes, not yet measured" for `Value`: measured, 16; the design
  text is changed at its next edit (`docs/design/engine.md`, "Not checked").

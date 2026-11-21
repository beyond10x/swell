# Slice 2, wave 1 of 4: the grammar of slice 2, the first early builtins, a line per run

One wave of the [wave plan](waves.md), 2026-11-21. Three units side by side, each implemented
by a sub-agent on Opus in its own worktree; the coordinating session wrote no code. It
regenerated `conformance/test262/expected.txt` on the wave branch after the three merges (no
unit's copy was taken). Every commit is the organization bot's.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `test262-run-causes` | `swell-test262` writes `$TEST262_OUT/runs.tsv`: one line per run (the tag `test262-run`, slice, path, mode, `Passed`, `Failed` or `Budget`, and what ended the run), 85,556 lines. `$262.evalScript`, in the realm of the function called; a refusal or the nesting limit inside its string fails the run whatever the case does with the error | 277 tests in the workspace; one case more passes | `9164dd7` |
| `number-boolean-and-global-functions-early` | `Number` (called and constructed, value properties, `valueOf`, `toString` for every radix 2 to 36), `Boolean`, `isNaN`, `isFinite`, `String.prototype.toString` and `valueOf`, each complete. `swell-num` prints a Number in any radix with the draft's algorithm, fractions included (350,420 results checked) | 290 tests; `test-collect: 34 test suites: 203 passed`; 948 cases more pass | `d5bede8` |
| `syntax-language-core` | The parser takes automatic semicolon insertion, labels, `??`, `?.`, logical assignment, optional catch binding, trailing commas, numeric separators, a hashbang line, identifier escapes that decode to ASCII, plain method definitions and shorthand properties; the compiler compiles them; one new instruction, `DefineMethod`. `**` is refused naming slice 4 (was 2). The refusal of a regular expression literal is now read as a refusal by `message_kind` | 279 tests; 970 cases more pass; 2,973 runs fewer refused | `ec118f6` |

Each unit's claim was checked by the coordinator before its commit by running the tests of the
workspace, clippy, `test-collect` where the unit touched `crates/swell`, the tasks `suite` and
`conform-engine` for the syntax unit, and `slot task test262` again in the unit's tree (exit 0
each), and by comparing the unit's run lines with the base run by run.

**What works now.** A script may leave out its semicolons, label its loops, define methods
in an object literal and use `Number` and `Boolean`. On the wave branch with the three
merged, `slot task test262` counts 5,924 of the 44,812 run cases as passed (3,969 before the
wave), and slice 2 passes 3,468 of its 5,245 cases (2,125 before). 50,596 of the 85,556 runs
still end at a refusal (53,564 before).

**No run was lost.** The run lines of the wave branch were compared with the base (the engine
of `main` with the new runner): 3,753 runs pass newly, and no run that passed on the base
fails. Per unit, alone: 1,887 and 0 for the builtins, 1,795 and 0 for the grammar.

## What went wrong, and what was decided

- **All three stories met a file outside their `Writes:` line; two stopped and were extended.**
  - With `Number` delivered, a fixture of the runner's own tests that had to fail
    (`Number.prototype;`) passed, and 11 runner tests failed. Fixtures that must fail no
    longer depend on a builtin being absent: they name an identifier no slice defines. One
    `Atomics` fixture was changed the same way before slice 14 breaks it.
  - With semicolon insertion delivered, a test of the conformance target that parsed `1 + 1`
    and expected a refusal failed. Decided: it uses the latest refusal the parser has (an
    HTML-like comment, slice 18), not `with`, which wave 2 delivers.
  - A script that starts with a hashbang cannot carry its expectation on line 1, and the
    strict prologue cannot stand before it: `crates/swell/tests/language.rs` reads the
    expectation after a leading `#!` line.
  The critics had checked each `Writes:` line against the code; none of the three was a file
  a story edits for its own change. The rule, now in `unit-invariants.md`: a unit that lifts a refusal
  or delivers a builtin searches the whole tree for tests that rely on the absence, before it
  writes, and stops if the file is outside its `Writes:` line.
- **An acceptance number of the story was wrong**: `test262-run-causes` expected no run of
  `global-code` to end with "the value is not a function"; six do, in three cases whose first
  statement calls `Object.preventExtensions` (slice 3). The command matches any missing
  function. The measured number stands.
- **The data of a host function cannot be read back.** The runner's story planned to keep
  the realm's index as host data of `$262.evalScript`; `Cx::host_data` reads host objects
  only, and with that plan every run ended in `Terminated(EngineFault)`. The runner compares
  the callee with roots it keeps. A question for the design, recorded in the wave plan.
- **`Array.prototype.toString` is missing** (slice 3): an array converts through
  `Object.prototype.toString`, `isNaN([])` is `true`. The same kind of wrong answer the
  builtins story fixed for `Number`. To decide before wave 4 with the run lines.
- **No omission lines for `Number` and `String`.** What stays out are whole methods of later
  slices, not an early part that is incomplete; the design's omission rule does not apply.
- **The radix argument of `Number.prototype.toString`** follows the pinned draft
  (`SnapToInteger`), where the story said ToIntegerOrInfinity; the behaviour is the same.
- A unit ran `python3 --version` once, against the rule; nothing used Python. It is the
  third unit to do so.
- One unit's wait loop wrote about 64 MB into the session's own tool results and removed it.

## The gate on the wave branch

`slot task check` with the variables of the runner, exit 0:

```
gate: variables in effect: SPEC_REFUSALS_MAX=41 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log ESS_SUITE=.out/suite.json ESS_UNSUPPORTED_MAX=44
python-check: no Python file among 930 file(s)
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
58 test suites: 292 passed, 0 failed, 1 ignored
test-collect: 34 test suites: 203 passed, 0 failed, 1 ignored
test-faults: 1 test suites: 1 passed, 0 failed, 0 ignored
conform-engine: planted defect refused: test a_target_whose_interrupt_does_nothing_fails_a_scenario ... ok
  58 scenarios: 14 passed, 0 failed, 0 error, 44 unsupported
test262-refused 50596
test262 total 53801 8982 7 44812 5924 38888 0
test262: 221 directory lines sum to the total
test262-slice 01 31 62 0 0 31 31 0 0
test262-slice 02 5245 9173 0 0 5245 3468 1777 0
test262: 22 slice lines equal the design
test262-errors 0
test262: 283 lines equal conformance/test262/expected.txt
gate-selftest: 31 passed, 0 failed
```

The `python-check` line is from that task run again on the committed tree: the gate ran with
923 files, before this record and the evidence files were added. `expected.txt` was
regenerated before the gate ran; no code, test or task file changed after it.

Passed cases per slice in that run, of the cases the slice has: 01 31 of 31, 02 3,468 of
5,245, 03 1,845 of 7,603, 04 217 of 1,198, 05 31 of 2,118, 06 98 of 3,717, 07 4 of 1,432, 08
4 of 6,188, 09 0 of 1,750, 10 10 of 6,573, 11 11 of 1,277, 12 39 of 317, 13 0 of 3,044, 14 0
of 479 run, 15 30 of 1,094, 16 0 of 82, 17 0 of 35, 18 18 of 1,155, 19 118 of 1,474.

## Where each finding went

- The data of a host function, and `Array.prototype.toString`: the wave plan.
- The base run lines for wave 2: taken from this wave's branch after its close.
- `with`, the `arguments` object and the names a `with` body reaches:
  `story:with-arguments-and-dynamic-names` (wave 2).

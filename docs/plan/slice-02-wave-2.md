# Slice 2, wave 2 of 4: `with`, the `arguments` object, names looked up at run time

One wave of the [wave plan](waves.md), 2026-11-21. One unit, implemented by a sub-agent on
Opus in its own worktree; the coordinating session wrote no code. The wave was opened on
`main` after wave 1 and landed on `main` after the fix of the stack bound
([wave 1, "After the wave"](slice-02-wave-1.md)): the unit's tree was moved onto the fix
before its claims were run again. Every commit is the organization bot's.

## Unit

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `with-arguments-and-dynamic-names` | The parser takes `with` in non-strict code and `arguments` inside a function. A reference inside the body of a `with` is looked up by name at run time, record by record, one step per record visited; every other reference keeps its slot (the design, decision 3, "Names"). An object environment record follows its clauses, the `@@unscopables` Get included. The `arguments` object in both forms, complete: mapped (its own [[GetOwnProperty]], [[DefineOwnProperty]] and [[Delete]]) and unmapped (the `callee` accessor that throws), made only where the function's code can observe it, charged to the memory budget | 300 tests in the workspace; `test-collect: 35 test suites: 208 passed`; 338 cases more pass; 6,384 runs fewer refused | `97f0a84` |

The unit's claim was checked by the coordinator before its commit, in the unit's tree on top
of the stack fix: the tests of the workspace (the three stack tests among them, in the forge's
build), clippy, `test-collect`, `test-faults`, the host requirements, the tasks `suite` and
`conform-engine`, and `slot task test262` (exit 0 each), and its run lines were compared with
the base run by run.

**What works now.** A non-strict script may use `with`, and a function may read and write
its `arguments`: `arguments.length`, an index that aliases a parameter in a non-strict
function with simple parameters, `arguments.callee` there and a `TypeError` for it in strict
code. On the wave branch `slot task test262` counts 6,262 of the 44,812 run cases as passed
(5,924 before the wave), and slice 2 passes 3,743 of its 5,245 cases (3,468 before). 44,212
of the 85,556 runs still end at a refusal (50,596 before); none of them for `with` or
`arguments`.

**No run was lost.** Run by run against the base (the run lines of wave 1's close): 456 runs
pass newly and no run that passed on the base fails. The slices whose passed cases moved: 2
(3,468 to 3,743), 3 (1,845 to 1,897), 7 (4 to 5), 11 (11 to 13), 19 (118 to 126).

**Where the formerly refused runs went.** 10,299 runs were refused for `with` or `arguments`
on the base. 6,384 fewer runs are refused now; the others end at the next refusal on their
path. By the unit's count the two largest groups of what the lifted refusals uncovered are
5,100 runs that end with "Function is not defined" in `propertyHelper.js` (wave 3) and 2,797
that end at the refusal of `class` (slice 5).

## What went wrong, and what was decided

- **The unit was built on a `main` that was red on the forge.** The stack test that failed
  there passed in the unit's tree, as it did in the gate here, for the reason wave 1's record
  gives. The unit was told while it ran not to touch the constant or the stack tests and to
  keep the work of its new instructions out of the run loop's frame. It reported the frame
  sizes instead of the stack tests (read from the prologues with `objdump`, in a build with
  line tables only): `Agent::execute` 18,424 to 18,456 bytes, 32 more; no other function on
  the re-entry chain of the stack tests changed. `make_frame` grew from 1,672 to 2,104 bytes
  and returns before the nested run starts. After the tree was moved onto the fix, the three
  stack tests passed in the forge's build on seven eighths of the promise.
- **A number of the story was worded wrongly.** Its acceptance says that
  `awk 'END { print NR }'` over the run lines "prints the number of runs of the `test262
  total` line"; that line has no column for runs. The number is 85,556, the same as on the
  base, and that is what was checked.
- **Decisions the unit took, kept.** A name reference on the operand stack keeps the size of
  an entry (two new 12-byte entries, for a declarative record and for a `with` object); the
  binding's slot is found again by name when the reference is read or written. A record with
  no names is passed over by a lookup, at one step. [[Get]] and [[Set]] of the mapped
  `arguments` object stay the ordinary ones, because they give the same result through the
  object's [[GetOwnProperty]] and [[DefineOwnProperty]]; the module comment says so. Language
  scripts and syntax sources that are non-strict by nature carry the marker `// sloppy-only`.

## The gate on the wave branch

`slot task check` with the variables of the runner (`CI=true GITHUB_ACTIONS=true
CARGO_TERM_COLOR=always CARGO_INCREMENTAL=0 RUSTC_WRAPPER=""`), on the wave branch with the
unit merged, exit 0:

```
gate: variables in effect: SPEC_REFUSALS_MAX=41 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log ESS_SUITE=.out/suite.json ESS_UNSUPPORTED_MAX=44
python-check: no Python file among 959 file(s)
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
59 test suites: 300 passed, 0 failed, 1 ignored
test-collect: 35 test suites: 208 passed, 0 failed, 1 ignored
test-faults: 1 test suites: 1 passed, 0 failed, 0 ignored
conform-engine: planted defect refused: test a_target_whose_interrupt_does_nothing_fails_a_scenario ... ok
  58 scenarios: 14 passed, 0 failed, 0 error, 44 unsupported
test262-refused 44212
test262 total 53801 8982 7 44812 6262 38550 0
test262: 221 directory lines sum to the total
test262-slice 01 31 62 0 0 31 31 0 0
test262-slice 02 5245 9173 0 0 5245 3743 1502 0
test262: 22 slice lines equal the design
test262-errors 0
test262: 284 lines equal conformance/test262/expected.txt
gate-selftest: 31 passed, 0 failed
```

## Where each finding went

| finding | where it went |
|---|---|
| The doc comment of `crates/syntax/src/lib.rs` lists the grammar without `with` and `arguments` (outside the unit's files) | wave 3: the lead notes of `eval-and-function-constructor` |
| `arguments-object/10.6-13-a-2.js` and `-a-3.js` fail on `arguments.callee.caller` | not of this slice: the two cases carry the feature `caller` and are counted in slice 18. They read `Function.prototype.caller` of a non-strict function, which throws here (`%ThrowTypeError%`); the cases accept `undefined` or a function |
| 21 runs of `with` and `arguments` cases need `eval` (15 "eval is not defined", 6 "Expected a SyntaxError but got a ReferenceError") | wave 3 |
| For `eval`: a record cannot gain names at run time (a `var` declared by a non-strict direct `eval`, deletable), and `arguments_object` is decided from the function's own references (a function with a direct `eval` needs it forced) | wave 3: the lead notes |
| Prepared for `eval`: `Binding::Dynamic`, the seven name instructions, `Agent::resolve_name`, `mark_named` | wave 3: the lead notes |
| The run loop's frame: 32 bytes more in this wave, with the new work outside it | the unit after this wave ([wave plan](waves.md)) |

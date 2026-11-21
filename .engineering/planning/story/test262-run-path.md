---
format: aep.planning-md/3
id: story:test262-run-path
kind: story
status: implemented
title: 'swell-test262 runs cases: 31 cases and 62 runs of slice 1 pass, 51 cases move to later slices'
summary: The run path of decision 11 with $262, print, budgets, the twenty most expensive runs and workers; the 51 move lines and new counts in the design; three limit omissions; expected.txt from the run.
relations:
- decomposes: epic:slice-01-harness
- depends_on: story:interpreter-and-script-entry-points
- depends_on: story:harness-builtins
- depends_on: story:host-interface-hardening
scope:
- confidence: cited
  path: conformance/test262/expected.txt
- confidence: cited
  path: conformance/test262/sections.txt
- confidence: cited
  path: crates/test262/Cargo.toml
- confidence: cited
  path: crates/test262/src
- confidence: cited
  path: crates/test262/tests/fixtures
- confidence: cited
  path: crates/test262/tests/run.rs
- confidence: cited
  path: docs/design/engine.md
- confidence: cited
  path: taskfiles/test262.yml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T04:49:59Z", actor: "human:operator", revision: 4, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T04:49:59Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T12:20:36Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

`story:test262-counting-runner` counts every case and marks every run case failed in one function
of its `count` module, "documented as the place slice 1 changes". This story replaces that
function with the run path of decision 11 and the epic: a fresh agent and realm per run,
`assert.js` and `sta.js` before every case but `raw` ones, the `includes:` files in order, a
non-strict and a strict run (`"use strict";` prepended) unless `onlyStrict`, `noStrict` or
`raw` says one, `negative` handling, the budgets (first values 2^30 steps and 512 MiB, a run
that reaches one printed as `budget`), the twenty most expensive runs by steps, and workers
through a flag. `$262` and `print` are host functions of this crate.

`async` cases and `doneprintHandle.js` are left to slice 10: none of the 31 cases is `async`
(none of the 82 cases of `test/harness/` has a flag: `cut -f2` over their `swell-test262 meta`
lines, `grep -c .`, prints 0), and an `async` case needs promises and jobs to pass. Until slice
10 an `async` case is run like any other and counts as failed, since `$DONE` is not defined
without `doneprintHandle.js`; it passes nothing it should not.

Slice 1 is the 31 cases (62 runs), decided on 2026-11-21 by the session that decides for the
operator; strict mode is in slice 1; the `arguments` object and the `Function` constructor are
not taken early.

Slice 1 count. Measured on the pinned checkout with the design block and the inventory awk
(`awk -f slices.awk rules.txt meta.tsv`, then the run sum of the design): slice 1 has 82 cases
and 164 runs, all in `test/harness/`, none with flags, features or `negative`. A construct and
builtin scan of each case and the harness files it includes (comments removed, string contents
blanked, then a scan for later-slice syntax, automatic semicolon insertion candidates and the
builtins called) shows 51 cases that need a later slice, so 31 cases and 62 runs stay:

- To slice 2, 1 case: `asyncHelpers-asyncTest-func-throws-sync` (`asyncHelpers.js` needs
  automatic semicolon insertion: line 82, `res.then(onResFulfilled, onResRejected)` without a
  semicolon).
- To slice 3, 23 cases: the 14 `propertyhelper-*` cases and the seven `verifyProperty-` cases
  `configurable-object`, `restore`, `restore-accessor`, `same-value`, `string-prop`, `value`,
  `value-error` (`propertyHelper.js` reads `arguments.length`, lines 55, 219, 460 and 476, and
  calls `Function.prototype.call.bind` on the global `Function`, lines 34 to 41, whose
  `CreateDynamicFunction` is slice 3; `-restore-accessor` also uses a method shorthand);
  `asyncHelpers-asyncTest-without-async-flag` (`Object.hasOwn` and a method shorthand);
  `fnGlobalObject` (`Function("return this;")()`).
- To slice 4, 2: `assertRelativeDateMs` (`Date`), `nans` (`Number.NaN`, `Math.pow`).
- To slice 6, 16: `verifyProperty-arguments`, `-noproperty`, `-undefined-desc` (arrow
  functions); `compare-array-arguments`, `-arraylike`, `-different-elements`, `-different-length`,
  `-falsy-arguments`, `-same-elements-different-order`, `-samevalue`, `-sparse` (arrow, template,
  `const` or spread); `asyncHelpers-asyncTest-rejects-non-callable`,
  `asyncHelpers-asyncTest-return-not-thenable` (`const`); `nativeFunctionMatcher`,
  `promiseHelper`, `wellKnownIntrinsicObjects` (`const`, arrow).
- To slice 9, 6: the `deepEqual-*` cases (`deepEqual.js` calls `new Map()`, lines 58 and 223).
- To slice 12, 1: `testIntl-getLocaleBaseName` (`testIntl.js`: the BigInt literal `1_000n`,
  line 2608).
- To slice 13, 2: `detachArrayBuffer`, `detachArrayBuffer-host-detachArrayBuffer`
  (`new ArrayBuffer`).

The 31 that stay: the 23 `assert-*` cases, `sta`, `byteConversionValues`, `compare-array-empty`,
`compare-array-same-elements-same-order`, `dateConstants`, `decimalToHexString`,
`proxytrapshelper-default`, `proxytrapshelper-overrides`, `tcoHelper`. `$262` needs only
`createRealm` and `global` for them (`assert-throws-same-realm`); `evalScript` is first used in
slice 2, `gc` in 19, `IsHTMLDDA` in 18, `detachArrayBuffer` in 13, `agent` is excluded.


Every limit of the host interface is a departure from an algorithm that has none, so the three
that a test262 directory reaches are recorded as omissions that stay (decision 3), in the
sections that define the bounded algorithm: `parser_nesting` in ParseScript, `frame_depth` in
`[[Call]]` of ECMAScript functions, `string_length` in string concatenation.

The harness files are parsed again for every run: the host interface cannot share a parsed
script across agents, while decision 11 of the design says "once per worker". This story keeps
the re-parse, the coordinator corrects decision 11, and the run prints its time so that the cost
is measured.

Workers run on threads whose stack is exactly `min_stack_bytes()`, the value
`story:host-interface-hardening` checks through every builtin that re-enters, so this story
depends on that one.

A case of slice 1 that does not pass is a finding that goes back to the story that owns the
failing part (the syntax, compiler, interpreter, object model or builtins story), which the
coordinator resumes; this story writes no engine file, only those on its `Writes:` line.

## Change

Failing tests first: the fixture cases and tests below are written before the code;
`every_run_case_is_counted_failed` is replaced, and the new tests fail because every run fails.

- `crates/test262/Cargo.toml`: dependency on `swell` (path). `src/run.rs` (new): one run of one
  case, with `Limits` of 2^30 steps, 512 MiB, and the other limits from one table in this file,
  on a worker thread whose stack is `min_stack_bytes()`; harness sources read once per worker and
  parsed per run. `src/host.rs` (new): the runner `Host`, `$262` with `createRealm` and `global`,
  `print`. `src/count.rs`: the outcome function calls `run`; a run that reaches a budget is
  failed and printed as `test262-budget <path> <strict|non-strict> <reason>`; after the lines,
  to standard error, the twenty runs with the most steps as `test262-steps <steps> <path>
  <mode>` and one line per slice `test262-time <slice> <runs> <milliseconds>`. `src/main.rs`:
  `--workers <n>` (default 1; the printed lines on standard output do not depend on it).
- Fixture cases under `crates/test262/tests/fixtures/` and tests in `crates/test262/tests/run.rs`:
  `a_passing_case_is_counted_passed`, `a_failing_assert_is_counted_failed`,
  `strict_run_prepends_use_strict`, `only_strict_and_no_strict_run_once`,
  `raw_case_gets_no_harness`, `negative_parse_case_needs_the_named_error`,
  `negative_runtime_case_needs_the_named_error`, `an_endless_case_is_printed_as_budget`,
  `twenty_most_expensive_runs_are_printed`, `run_time_is_printed_per_slice`,
  `create_realm_gives_a_fresh_global`, `worker_count_does_not_change_the_lines`, and
  `every_run_case_is_counted_failed` rewritten as `a_case_that_throws_is_counted_failed`.
- `docs/design/engine.md`, section 12:
  - the 51 lines `dir test/harness/<file> <slice>` in the `swell-slices` block (pure prefix
    rules, longest wins, so each file rule beats `dir test/harness/ 1`), and a paragraph beside
    the block with the reason of each group as above;
  - the table and the block after "Its output on this document" with the new counts:
    `01 31 62`, `02 5245 9173`, `03 7603 15038`, `04 1198 2396`, `06 3717 7162`,
    `09 1750 3471`, `12 317 634`, `13 3044 6044` (the other 14 slice lines unchanged; the 22
    still sum to 53,801 cases and 103,360 runs, slices 1 to 19 to 44,931 and 85,794, since every
    case moves within slices 2 to 13);
  - a sentence that 82 cases and 164 runs were the share of `test/harness/` by the rule
    `dir test/harness/ 1`, and 31 cases and 62 runs are what slice 1 passes, with the command
    that measured it (`awk -f slices.awk` over the block with the 51 lines and the inventory,
    counting slice `01`);
  - item 1 of "What each slice builds" (`harness` 31, strict mode), item 2 (strict mode no
    longer there), the sentence "35 of the 117 files in `test/harness/` fall after slice 1"
    (86 now), and in "A part before its slice" the 82 cases (31) and the sentence that makes
    `Zs` white space a SyntaxError before slice 5 (`story:syntax-es5-and-strict` accepts it in
    slice 1 with the table of `story:num-and-unicode-early-parts`);
  - the paragraph "For surf", read against these changes: each of its sentences is kept or
    corrected where the slice it names gained or lost content (strict mode moves earlier, the
    moved cases later within slices 2 to 13), and the change says which.
- `conformance/test262/sections.txt`, after the two of `story:syntax-es5-and-strict`:
  `section sec-parse-script`, `path sec-parse-script test/language/global-code/`,
  `omit sec-parse-script parser-nesting-limit decision-3`;
  `section sec-ecmascript-function-objects-call-thisargument-argumentslist`, `path
  sec-ecmascript-function-objects-call-thisargument-argumentslist test/language/expressions/call/`,
  `omit sec-ecmascript-function-objects-call-thisargument-argumentslist frame-depth-limit
  decision-3`; `section sec-applystringornumericbinaryoperator`, `path
  sec-applystringornumericbinaryoperator test/language/expressions/addition/`, `omit
  sec-applystringornumericbinaryoperator string-length-limit decision-3` (42, 92 and 48 cases).
- `conformance/test262/expected.txt`: the lines of the run, 283 lines, in the same change as the
  design counts.
- `taskfiles/test262.yml`: passes `--workers {{.TEST262_WORKERS}}` (default 4).

`Cargo.lock`: the unit regenerates it in its own tree (the new path dependency), because its
build needs it, and runs its acceptance with `--locked` against that lock; the coordinator does
not take the unit lock into the wave branch but regenerates it there after the merges of the
wave. The coordinator puts the gate numbers into `AGENTS.md` and `README.md` (the epic, item 3),
changes the epic counts and those of slices 2, 3, 4, 6, 9, 12 and 13, corrects decision 11, and
adds `1` to `conformance/slices-delivered.txt` when it closes the slice.

Writes: `crates/test262/Cargo.toml`, `crates/test262/src/{run,host,count,main}.rs`,
`crates/test262/tests/run.rs`, `crates/test262/tests/fixtures/` (added cases), the test file
that held `every_run_case_is_counted_failed`, `docs/design/engine.md` (section 12 as listed),
`conformance/test262/sections.txt`, `conformance/test262/expected.txt`,
`taskfiles/test262.yml`, `Cargo.lock` (regenerated in the unit tree; the wave branch lock is
regenerated by the coordinator).

## Acceptance

`cargo test --locked -p swell-test262` exits 0 and runs the thirteen tests named in Change;
`slot task test262` exits 0, prints `test262-slice 01 31 62 0 0 31 31 0 0`,
`test262: 22 slice lines equal the design` and `test262: 283 lines equal
conformance/test262/expected.txt`, prints no `test262-budget` line for a case of slice 1, and
prints to standard error twenty `test262-steps` lines and a line `test262-time 01 62 <ms>`; the
22 slice lines of the design block sum to 53,801 cases and 103,360 runs (`awk` over the block
after "Its output on this document", summing fields 2 and 3); and, as the last story of the
slice, `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` with
`HOST_REQ_DELIVERED` naming a file that holds the line `1` exits 0.

## Out of scope

`$262.evalScript`, `gc`, `detachArrayBuffer`, `IsHTMLDDA`, `AbstractModuleSource`, `agent`
(their slices); `async` cases and `doneprintHandle.js` (slice 10); module cases (slice 11);
sharing a parsed harness file across agents, which the host interface does not offer; any engine
file (a failing case goes back to its owner); `AGENTS.md`, `README.md`, the root `Cargo.toml`,
the epics, decision 11 (the coordinator).

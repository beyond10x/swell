# Slice 1, wave 4 of 6: the interpreter and the harness builtins; a script runs

One wave of the [wave plan](waves.md), 2026-11-21. Two units, each implemented by a sub-agent
on Opus in its own worktree; the coordinating session wrote no code. It regenerated
`Cargo.lock` on the wave branch (7 packages; `swell` now depends on `swell-compiler` and
`swell-syntax`). Every commit is the organization bot's.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `interpreter-and-script-entry-points` | In `crates/swell`: the four kinds of environment, ECMAScript function objects, the interpreter loop over the bytecode of `swell-compiler` with every frame on the engine's own frame stack; one step charged per instruction, the budget checked at every backward jump, call, return and re-entry; frames, operand stacks, environments and code charged to the memory budget; frame depth and re-entry depth bounded; interrupt and budgets ending the run in `Terminated` with no `catch` or `finally` block run. `parse_script` and `evaluate_script` on `Agent` and `Cx`; `Limits::min_stack_bytes()`. 28 host requirements move to `test` | 144 tests in the crate, in both collection modes; 102 language scripts, 85 of them in both modes; `55 test, 106 pending` | `d8f70a6` |
| `harness-builtins` | `crates/swell/src/builtins` (2,725 lines): the members of `Object`, `Function.prototype`, `Array`, `String`, `Math` and `JSON` that `assert.js`, `sta.js` and `propertyHelper.js` call, each citing its clause; the bound function exotic object | 24 integration tests and 5 library tests, in both collection modes | `a7c2e33` |

`crates/swell` holds 14,412 lines in 54 source files after the wave. Each unit's claim was
checked by the coordinator before its commit by running the tests of the workspace or of the
crate, the task `test-collect` and clippy on the workspace again in the unit's tree (exit 0
each).

**What works now.** `Agent::evaluate_script` evaluates a script of the ES5 grammar in either
mode under a step budget and a memory budget, and the script reaches the builtins: eight
language scripts written after the two units met (`Object.defineProperty` and
`getOwnPropertyDescriptor`, `Array.prototype.map` and `join`, `JSON.stringify`,
`Function.prototype.call.bind`, `for-in` over a non-enumerable property, `Math.max`,
`instanceof` on a bound function, `Array.prototype.push`) answer what their clauses say.
No test262 case is run by the gate yet: the runner's run path is wave 6.

**The 31 cases, measured early.** A throwaway probe outside the repository ran the 31 cases of
the slice on this engine the way the runner will (a fresh agent and realm per run, `assert.js`
and `sta.js` first, both modes, 2^30 steps and 512 MiB, a thread of exactly
`min_stack_bytes()`): 60 of 62 runs pass, 30 of 31 cases. `assert-throws-same-realm` throws
`ReferenceError: $262 is not defined`; `$262.createRealm` belongs to the runner's host, which
wave 6 writes. The largest passing run takes 1,782 steps.

## What went wrong, and what was decided

- **Both units stopped before they wrote anything, and were right.** Neither story could be met
  on its `Writes:` line. The code of waves 2 and 3 marks items a later story uses with
  `expect(dead_code, reason = "<story>")`, and an expectation that is no longer fulfilled fails
  the build under `-D warnings`: 3 marks for the builtins, 14 for the interpreter, in files
  neither story listed. Beyond the marks: `GetFunctionRealm` and `OrdinaryHasInstance` lacked
  their bound-function step, the builtin ids lived in an unlisted file, a `Script` could not be
  made or read outside `value.rs`, and the root table could not hold a script record. Each
  unit showed the compiler's output; each story was extended and the unit resumed. With wave
  3's `object-model-and-realms` this is the third story of the slice accepted before the code
  it builds on existed. The rule for the next decomposition is in `AGENTS.md`, "How the work
  is run".
- **The two stories met in two files.** Planned as disjoint, both wrote `object/mod.rs` and
  `ops/objects.rs` after the extensions. The builtins landed first; the wave branch was merged
  into the interpreter unit's tree; one conflict (`get_function_realm`), resolved by that unit:
  the function follows the steps of its clause as a loop. Two helpers both had written are
  kept once.
- **Dispatch of builtins.** Six `BuiltinId` variants, one per builtin module, over creating
  each builtin as a host function: an engine builtin does not go through the host-function path
  and its answer checks.
- **No public accessor for a well-known symbol** in this slice. Two tests that need one are
  library unit tests.
- **Two tests of wave 3 changed.** One expected a throw that `Object.prototype.toString` now
  prevents and uses an object with a null prototype; one held objects without a root across a
  call and failed in the mode that collects at every safe point once `instanceof` called a
  method.
- **Deep tests are smaller in the collect mode.** A chain of 100,000 bound functions and
  100,000 nested arrays run in the default mode; 2,000 in the mode that collects at every safe
  point, where the cost grows with the square of the depth (5,000 links took 6.9 seconds).
- **The stack the engine promises.** `Limits::min_stack_bytes()` is the parser's stated need
  plus 30 KiB per re-entry plus 64 KiB. Measured by bisection in a child process: 25,375 bytes
  per re-entry in a debug build (the same with and without incremental compilation here), 3,263
  in a release build; so the bound is about nine times what a release build needs. With
  `frame_depth` 10,000, `parser_nesting` 1,000 and `reentry_depth` 1,000 it is 31,073,024
  bytes. `story:host-interface-hardening` measures it again through builtins.
- **Re-entry count, an off-by-one found and fixed by the unit**: with depth R, R nested run
  loops run and the deepest can still call a host function.
- **The global environment's declarative part is empty in slice 1** (`RealmGlobalEnv`): `var`
  and function declarations live on the global object; slice 6 writes the rest.
- **`to_int32` keeps its mark**: no instruction of slice 1 calls it.
- `Function` is not a global in this slice (the constructor needs `eval`-like parsing);
  `assert.js` and `sta.js` do not name it (checked with `grep` on the pinned harness files).

## The gate on the wave branch

`slot task check` with the variables of the runner, exit 0:

```
gate: variables in effect: SPEC_REFUSALS_MAX=32 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log
python-check: no Python file among 754 file(s)
forbid-unsafe-check: 8 crate root(s), each with #![forbid(unsafe_code)]
Cargo.lock: 7 package(s), none from outside this repository
test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked
data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present
unicode-tables: 1 table(s) equal the generator output
swell v1 — 2 file(s), 3 scenario(s), valid
61 scenario(s) (3 authored), 32 refusal(s), written to .out/suite.json
  61 scenarios: 61 passed, 0 failed, 0 error, 0 unsupported
host-requirements: 161 requirement(s), 161 line(s): 55 test, 106 pending, 0 omitted; 50 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 1:15 2:5 3:5 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1
48 test suites: 230 passed, 0 failed, 0 ignored
test-collect: 25 test suites: 173 passed, 0 failed, 0 ignored
test262 total 53801 8982 7 44812 0 44812 0
test262: 221 directory lines sum to the total
test262: 22 slice lines equal the design
test262-errors 0
test262: 280 lines equal conformance/test262/expected.txt
gate-selftest: 30 passed, 0 failed
```

## Where each finding went

- How a story's `Writes:` line is checked against the code it builds on: `AGENTS.md`, "How the
  work is run".
- The declarative part of the global environment: the wave plan, "What later slices take
  over", slice 6.
- The stack bound through builtins, and a stale, released and foreign `Realm` at the three
  entries that take one: `story:host-interface-hardening` (wave 5).
- The 31 test262 cases of the slice on the engine: `story:test262-run-path` (wave 6).

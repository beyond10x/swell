# Slice 1, wave 3 of 6: the compiler, the object model, and the first runs on the forge

One wave of the [wave plan](waves.md), 2026-11-21. Two units of the slice, and two fixes that
the first runs of the gate on the forge asked for, each implemented by a sub-agent on Opus in
its own worktree; the coordinating session wrote no code. It regenerated `Cargo.lock` on the
wave branch (7 packages) and added the include `taskfiles/collect-mode.yml` and `- task:
test-collect` (after `test`) to the root `Taskfile.yml`. Every commit of this wave is the
organization bot's; the repository was published at the close of wave 2.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `bytecode-compiler` | `crates/compiler` (`swell-compiler`, 3,050 lines in `src/`): bytecode for a stack machine from the tree of `swell-syntax`. Local slots for bindings only their own function uses, environments for captured ones; every backward jump is a `LoopJump`; handler ranges for `try`, a tagged completion for leaving a `finally` block; a completion value register for script code. No walk recurses | 8 tests over 45 sources with expected disassembly written before the code | `7d180e4` |
| `object-model-and-realms` | `crates/swell` (8,765 lines in 54 files now): ordinary objects, arrays, string objects, builtin, bound and host functions, the abstract operations, conversions and operators, realms with intrinsics and a global object, the Error constructors, the hooks of this slice, the `Cx` methods that create and call; an arena for Symbols; the corrected result types of wave 2's design change; the current realm as the design now decides it. 10 host requirements move to `test` (`HOST-4`, 5, 12, 15, 47, 51, 52, 106, 124, 161) | 106 tests in the crate, in both collection modes; `27 test, 134 pending` | `6a5a4ef`, `4d59521` |

Each unit's claim was checked by the coordinator before its commit by running its tests, its
task and clippy on the workspace again in the unit's tree (exit 0 each); for
`object-model-and-realms` also once with no compiler cache and no incremental compilation.

## The two fixes the forge asked for

Both went to `main` alone, before anything of the wave, and were published by the session that
decides for the operator.

- **A stack overflow in the tests of `swell-syntax`** (`2b1ae5d`). The first run of `check` on
  the forge aborted in `nesting_of_100000_brackets_is_one_error`. Reproduced locally with no
  compiler cache and no incremental compilation. The cause was not the parser: the early-error
  walk and the scope walk recursed once per tree level, about 328 bytes per level in such a
  build, and the deepest chain the limit of 1,000 admits needed about 336 KiB on the test's 256
  KiB thread. With incremental compilation frames are 16 to 20 % smaller, so it passed on the
  workstation. Both walks are iterative now; the crate states its bound,
  `ParseOptions::stack_bytes()` = 32,768 + 224 bytes per level (measured worst case 172 per
  level and 16,416 fixed; x86_64, rustc 1.99.1, debug, no incremental compilation); the test
  `every_nesting_kind_fits_the_stack_bound` runs 29 kinds of nesting in a child process on a
  thread of that size and was shown red with a frame grown by 160 bytes.
- **The gate self-test: 14 passed, 14 failed on the runner** (`05a4662`). Two variables of the
  runner change the bytes go-task 3.54.0 writes: with `CI=true` it colours its failure line and
  puts the reset after the newline, and with `GITHUB_ACTIONS=true` it adds an `::error` line
  that no flag removes. Scripts that compare a task's output exactly failed; twelve of the
  fourteen that passed held such bytes in their logs and passed by what their patterns did not
  read. Every planted script now clears those variables and sets `NO_COLOR` and
  `CARGO_TERM_COLOR=never`; the self-test prints the last 20 lines of a failing script's log
  with control bytes made visible; the script `gate-selftest-is-colour-proof` runs every other
  script with the runner's variables.

After the second fix both workflows are green on `main` (`05a4662`).

## What went wrong, and what was decided

- **A unit stopped before it wrote anything, and was right.** `object-model-and-realms` could
  not be met on its `Writes:` line: with one required hook and the corrected type of
  `new_host_object` the crate does not compile unless the test hosts inside it, the callers in
  four test files, the field of `Realm`, the root slot and a missing arena for Symbols change
  too. Its probe showed the compiler's errors; the story's `Writes:` was extended and the unit
  resumed. The plan critics had not seen it: the story was accepted before wave 2's code
  existed.
- **The current realm depended on the collector.** The unit took, while no code runs, "the
  realm created last that is still alive", and a null prototype with no realm. Not accepted:
  "still alive" is decided by when a collection ran. Decided by the coordinator, written by the
  design author: outside a run the host names the realm; `with_cx`, `Agent::call` and
  `Agent::construct` take a `&Realm` (`docs/design/host-interface.md`, decision 7, `HOST-161`,
  `8d40332`). The unit changed the code; without a current realm the agent faults.
- **A wave 2 test failed in the mode that collects at every safe point**: it held a weakly held
  atom without a root across a safe point. A test bug, fixed. The gate did not run the whole
  crate in that mode; the task `test-collect` does now, with a planted script.
- **`**` and `Error.isError` are not implemented.** The parser refuses `**`, and its operator
  needs the deterministic `pow` of slice 4; `Error.isError` goes with the Error builtins of
  slice 3. Both are in the wave plan's list.
- **`CompileError` has a second variant, `Tree`**, for a tree `parse_script` cannot produce;
  the story named one. Accepted: without it such a tree would need a panic. The bounds of
  2^32 instructions, constants or slots have no test that reaches them.
- **Three tests of `budgets.rs` changed their numbers.** `with_cx` needs a realm, and a realm
  costs memory: the budgets are the realm's measured bytes plus 1,024, and the test of a zero
  budget asserts that the heap holds the realm's bytes and no more. `HOST-12` still shows a
  budget of 0 refusing the realm.
- **`HOST-51` and `HOST-161` are held by library unit tests** (`swell::lib::realm::tests::...`):
  they need internals the public interface does not show. The requirement lines name them.
- A unit ran `python3 --version` once inside a search, against the rule; nothing used Python.
- The cold-run check of wave 2 had not proven the compiler cache off. It is now: `cargo -v`
  prints the compiler's command lines, none with the cache (`RUSTC_WRAPPER=""`).

## The gate on the wave branch

`slot task check` with the variables of the runner (`CI=true`, `GITHUB_ACTIONS=true`,
`CARGO_TERM_COLOR=always`, `CARGO_INCREMENTAL=0`), exit 0:

```
gate: variables in effect: SPEC_REFUSALS_MAX=32 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log
python-check: no Python file among 629 file(s)
forbid-unsafe-check: 8 crate root(s), each with #![forbid(unsafe_code)]
Cargo.lock: 7 package(s), none from outside this repository
test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked
data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present
unicode-tables: 1 table(s) equal the generator output
swell v1 — 2 file(s), 3 scenario(s), valid
61 scenario(s) (3 authored), 32 refusal(s), written to .out/suite.json
  61 scenarios: 61 passed, 0 failed, 0 error, 0 unsupported
host-requirements: 161 requirement(s), 161 line(s): 27 test, 134 pending, 0 omitted; 50 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 1:43 2:5 3:5 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1
38 test suites: 163 passed, 0 failed, 0 ignored
test-collect: 15 test suites: 106 passed, 0 failed, 0 ignored
test262 total 53801 8982 7 44812 0 44812 0
test262: 221 directory lines sum to the total
test262: 22 slice lines equal the design
test262-errors 0
test262: 280 lines equal conformance/test262/expected.txt
gate-selftest: 30 passed, 0 failed
```

No test262 case passes yet: wave 4 brings the interpreter.

## Where each finding went

- The stack the engine promises: `story:interpreter-and-script-entry-points` writes
  `Limits::min_stack_bytes()` and includes the parser's `stack_bytes()` in it; it measures the
  stack of a re-entry without incremental compilation, and its stack tests run in a child
  process.
- The bytecode contract (`crates/compiler/src/bytecode.rs`): the same story.
- The current realm: decision 7 and `HOST-161`; the stories `interpreter-and-script-entry-points`,
  `harness-builtins` and `host-interface-hardening` each carry an addition.
- `**`, `Error.isError`, where a regular expression pattern is parsed with respect to the
  parser's recursion, and the planted scripts whose failure message points to a file the
  runner's log does not show: the wave plan, "What later slices take over".
- How a wave branch is checked before `main` moves: `AGENTS.md`, "Commands and the gate" (the
  gate once more with the runner's variables).

# Slice 2, between waves 2 and 3: the run loop's frame

One unit of the [wave plan](waves.md), 2026-11-21, with no story: its brief was its
specification. Implemented by a sub-agent on Opus in its own worktree; the coordinating session
wrote no code. Every commit is the organization bot's.

## Why

Every re-entry of the engine (a host function, a getter, a builtin that calls back) puts one
more frame of the run loop on the thread's stack. That frame, `Agent::execute`
(`crates/swell/src/interp/dispatch.rs`), held the locals of every instruction: 19,280 of the
31,375 bytes one re-entry took on the worst path, and each new instruction added to it. Wave 1
of this slice passed the stack a host is promised that way
([wave 1, "After the wave"](slice-02-wave-1.md)).

## What landed

| unit | what landed | unit commit |
|---|---|---|
| `run-loop-frame` | The body of every instruction arm that has locals of its own is a function of its own; the run loop calls it. No instruction changes what it does, no order of evaluation, safe point or root changed. The branch of `call_instruction` and of `new_instruction` that pushes a frame for an ECMAScript function is a function of its own too. `STACK_BYTES_PER_REENTRY` follows the measurement: 14 KiB (37 KiB before) | `587e2c1` |

Measured by the unit before and after, in the forge's build (debug with full debug
information), by bisection of the smallest thread stack in a child process at depths 200 and
1,000; the frame of `Agent::execute` is read from its unwind entry.

| path | bytes per re-entry before (`247333c`) | after |
|---|---|---|
| `toJSON`, a replacer of `JSON.stringify` | 31,375 | 11,100 |
| a `map` callback | 29,087 | 8,817 |
| a host function | 27,776 | 7,506 |
| `Function.prototype.call` | 27,730 | 7,455 |
| a getter | 24,079 | 5,535 |
| the frame of `Agent::execute` | 19,280 | 336 |

**What it changes for a host.** `Limits::min_stack_bytes()` for `reentry_depth` 1,000 and
`parser_nesting` 1,000 reads 14,672,640 bytes (38,248,192 before). The constant is the largest
measured need plus a fifth, rounded up to a whole KiB.

**The stack tests are the check that the frame does not grow back.** The three tests that say
"`min_stack_bytes()` is enough" run on seven eighths of the promise. Shown by the unit: with
the new constant on the code before the change all three abort; on the changed code all three
pass. A later unit can add 1,560 bytes per re-entry on the `toJSON` path (14 %) before the
first of them fails; no test asserts a frame size as a literal.

**No result of any case changed.** The coordinator ran the unit's claim again in its tree
before the commit (the tests of the workspace, clippy, `test-collect`, `test-faults`, the host
requirements, `suite`, `conform-engine`, `slot task test262`, exit 0 each) and compared every
run line with the base: mode, outcome and detail of all 85,556 runs are the same.

## Findings

- **Where the stack per re-entry is now.** The frames of `interp/` are about 2.9 KB of the
  11.1 KB on the worst path. The largest others: `json_serialize_property` 2,080 bytes,
  `json_stringify` 1,632, `json_property_list` 1,584, `call_function` 1,344
  (`object/ecmascript_function.rs`), `builtin_call` 720. Not changed here; the slice that
  completes `JSON` (the wave plan) reads these numbers.
- **The time of a run.** One run of `slot task test262` each: 21.8 s before, 27.4 s after, at
  different machine load. Run back to back, two rounds each: 26.6 s and 40.0 s for the base,
  26.5 s and 22.3 s for the change. The difference between the binaries is below the
  difference between two runs of one binary on this shared machine; no claim about speed is
  made either way.
- **Renamed for clippy.** Two methods of the dispatch (`convert_to_number`,
  `convert_to_numeric`): their former names made clippy read them as conversions that must
  take `self` by value.

## The gate

`slot task check` with the variables of the runner (`CI=true GITHUB_ACTIONS=true
CARGO_TERM_COLOR=always CARGO_INCREMENTAL=0 RUSTC_WRAPPER=""`), on the branch with the unit
merged, exit 0:

```
gate: variables in effect: SPEC_REFUSALS_MAX=41 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log ESS_SUITE=.out/suite.json ESS_UNSUPPORTED_MAX=44
python-check: no Python file among 960 file(s)
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

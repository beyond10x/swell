---
format: aep.planning-md/3
id: review-result:adversary-lanefive-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit lanefive
relations:
- reviews: story:property-escape-cases-spend-less-in-the-interpreter-loop
revision: 1
---
Adversary pass 1 on the unit lanefive of wave big-2, impl/lanefive at 2675d93, 2026-11-27.
Green: 411 scripts give the same completion, steps, bytes and budget outcomes on the base and the head; three notes (a checked write in the temporal dead zone held by no test, an uncharged reserve on a refused growth that the base has too, three helper frames 16 to 32 bytes larger).

unit: lanefive
verdict: green
cases: executed 6→9, red 0
origin: introduced 2, pre-existing 1, undecided 0
wrote-outside-worktree: `<scratch>/wave2/lanefive/adversary-1` (logs, harness copy, job scripts)
needs-coordinator: no

**Result:** no defect. Nothing a script or a host can observe changed between base `3e39e71` and head `2675d93`. I found three notes. The tree is back to clean at `2675d93`: I put back every plant and mutant and removed my temporary files.

**Findings**

1. **No test catches a checked write that stores before it throws** (`crates/swell/src/interp/lexical.rs:186`).
   - Mutant: `*binding = SlotValue::Value(value)` with no check. All 2024 tests of swell pass except my candidate case (`jm-M1.log`).
   - That case is a note and I took it out again. Its text:
     `function f() { var r = []; try { x = 5 } catch (e) { r.push(e.name) } try { r.push(x) } catch (e) { r.push(e.name) } try { x++ } catch (e) { r.push(e.name) } try { r.push(typeof x) } catch (e) { r.push(e.name) } let x; r.push(x === undefined); return r.join() } f()`
     must answer `"ReferenceError,ReferenceError,ReferenceError,ReferenceError,true"` in sloppy and in strict mode. It takes 0.014 s in the default build and 0.070 s in the collect build.
   - Origin: introduced (the unit's rewrite of SetLocalChecked).

2. **A refused growth of the operand stack leaves uncharged capacity** (`crates/swell/src/interp/mod.rs:273`). `push_entry` runs `try_reserve_exact` before it charges.
   - Lines my host function logged, for each power of two from 8 to 65,536: `short cap 8 growth 128: Terminated(MemoryBudget) len+0 cap 16 used+0`, then `exact ... ok len+1 cap 16 used+0 charged+0`.
   - The base prints the same lines. The run ends at that point, so the memory goes back when the frame unwinds.
   - Origin: pre-existing.

3. **Helper frames grew; the run loop's own frame did not.**
   - Release: `replace` 64→96 bytes, `jump_if` 64→80, `pop_instruction` 48→64. Stack-debug: `replace` 240→256. None of these calls script code, so none is on the re-entry path.
   - Run loop, unchanged: `run` is 624 bytes in release (CLI and test binary). In stack-debug `execute` is 816 and `run` is 320.
   - `execute_next` fell from 256 to 112 bytes, so a re-entry in stack-debug takes 1392→1248 bytes.
   - Origin: introduced.

**Per attack item**

1. **Differential.** I ran 411 scripts on base and head through a temporary lib harness. For each script it records the completion, printed text, steps, `heap_bytes()`, the peak, and the outcome under a step budget swept one step at a time. It also finds every memory-budget breakpoint by bisection (3442 of them) and sweeps the budget 8 bytes at a time below the peak (25,266 lines).
   - Default build: 76,791 lines on each side, all equal. Collect build: 6,288 lines, all equal.
   - Three exclusions or artifacts:
     - The root key number in one parse-error line differs. It is a counter, not output.
     - Three private-field scripts are left out because the base cannot parse them (that feature came with the merged wave).
     - One harness chunk timed out at 60 s in the collect build on both sides.
   - I ran a coverage plant over the same scripts:
     - Reached: `replace(0)` in 28 arms, `replace(n≥1)` in 55, `operands::<2>` in 21, `operands::<3>` in PutIndex.
     - `drop_top`: in Pop, JumpIfFalse and JumpIfTrue, and over an Enumerator, an Iterator Record and a Tag.
     - Both branches of GetLocalChecked and of SetLocalChecked.
     - Not reached: PutSuperIndex, SuperKey, the host-constructor branch of NewSpread, CallEvalSpread.
2. **Entry variants under `replace`.**
   - Value: the plain case.
   - Reference, DeclarativeReference and ObjectReference: removed by PutNameRef and PutGlobalRef in valid bytecode. They hold no bytes.
   - Enumerator and Iterator Record: never removed by `replace` in valid bytecode. They leave through Pop (`drop_top`), which credits them.
   - Tag: only through my host function.
   - A `replace` of more entries than the stack holds, of `usize::MAX`, or with no frame ends in `Terminated(EngineFault)` with the stack unchanged, the same on base and head. No panic.
3. **Skipped `credit(0)`.** No reader can see it. `credit(0)` only folds `released` into `used` (`accounting.rs:296-299`). Every other mutation folds first (`304`, `361`). Every reading goes through `used()`, which subtracts `released` (`372-374`; also `check` at `352-353` and `heap.bytes()` at `heap/mod.rs:432-433`). Probes, breakpoints and readings after a collection agree with this.
4. **`next_instruction`.**
   - A panic plant compared it with `Frame::instructions` and never fired: 2148 of 2149 tests passed in the default build and 2115 of 2117 in the collect build. The only failure was source_scan refusing my harness's `std::fs` calls, plus one harness chunk that timed out in the collect build.
   - Eight mutants answer None at one place each: call, return, generator resume, async resume, handler entry, direct eval, field initializer, static block. Tree tests catch each one (`kloop.log`).
   - A host function's frame with no code: INFEASIBLE. There is no such frame, and a template out of range answers None on both sides. With no frame, or past the end of the code, both base and head fault once.
5. **Stack.** `slot --timeout 30m task test-stack-profiles`: release 7 passed, 0 failed; stack-debug 7 passed, 0 failed. I compared frames by prologue size (finding 3), not by thread-stack bisection.
6. **Temporal dead zone.** 30 scripts on base and head. Error, message, steps and every budget sweep are equal.
7. **`perf-loop.sh`.**
   - Under bash and dash it exits 1 for each planted fault: a run that exits 3, wrong output, an extra line, a missing newline, and a base binary that exits 3. With the real binary it exits 0.
   - It builds nothing, asserts nothing about time, and no task includes it.
   - The release CLI on both base and head prints `263169`, which equals the committed `.out`.
8. **Mutants the unit's tests do not catch.**
   - The SetLocalChecked mutant: finding 1.
   - `drop_top` keeping the frame's `charged`: caught by `ops::objects::tests::every_exit_of_a_for_in_credits_its_key_list::*` and 4 more tree tests.
   - Strictness read from the template: caught by 7 `classes::*` tests.
   - Not counted: a message naming the next string, which the unit's own test already catches.
9. **Times** (CPU seconds, three runs, test alone). All runs exit 0, far under the 1.5 s and 2 s bounds.
   - Default build, load 4.2→14.8 on 32 cores:

     | Test | CPU s |
     |---|---|
     | temporal dead zone | 0.014 |
     | numeric operators | 0.014–0.015 |
     | frozen array | 0.009–0.010 |
     | step-budget sweep | 0.042–0.044 |
     | stack grows | 0.007 |
     | replace | 0.007 |

   - Collect build, load 9.6→6.3:

     | Test | CPU s |
     |---|---|
     | temporal dead zone | 0.245–0.249 |
     | numeric operators | 0.168 |
     | frozen array | 0.137 |
     | step-budget sweep | 0.077–0.078 |
     | stack grows | 0.035–0.036 |
     | replace | 0.012–0.013 |

**Cases.** I added three tests to `crates/swell/tests/interpreter_loop.rs` for the runs above and removed them again, because the code has no defect:
- `a_write_in_the_temporal_dead_zone_leaves_the_binding_uninitialized`
- `a_read_and_a_write_in_the_temporal_dead_zone_name_their_own_binding`
- `loops_that_end_give_back_what_their_enumerator_and_iterator_held` (1.46 s in the collect build)

Their diff is `<scratch>/wave2/lanefive/adversary-1/candidate-tests.diff`.

**Commands**

| Command | Result |
|---|---|
| `slot --timeout 30m sh loopdiff-j1.sh head 1` | harness default `128 passed`, collect `127 passed, 1 timed out`; lib+main with the plant as in item 4; no plant panic |
| `slot --timeout 30m sh loopdiff-base.sh` (tree switched to `3e39e71`, then back) | harness `128 passed` / `127 passed, 1 timed out`; CLI `exit 0 [263169] equal` |
| `slot --timeout 30m sh loopdiff-j2.sh head test` | stack lines in item 5; perf-loop plants in item 7 |
| `slot --timeout 10m sh loopdiff-jf.sh base`, `slot --timeout 30m sh loopdiff-j2.sh head build` | frame tables in finding 3 |
| `slot --timeout 20m sh loopdiff-jt.sh default` and `collect` | 27 lines each, all exit 0 |
| `slot --timeout 20m sh loopdiff-jm.sh <mutant>` (4 runs) | the results in findings 1 and item 8 |
| `kloop.sh` (8 pool jobs) | each `jk exit 100` |
| `cargo nextest run --locked -p swell --lib --test main [--features collect-every-safe-point] -E '…'` | `9 tests run: 9 passed`, both builds |

**Not attacked**
- I did not run `task test262`, `check`, `check-runner` or `slow`, as the brief says.
- No thread-stack bisection for the stack margin.
- BigInt scripts do not parse on either side.
- The unreached arms listed under item 1.

```findings
- file: crates/swell/src/interp/lexical.rs
  line: 186
  category: mutant
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: A SetLocalChecked that stores the value before it throws in the temporal dead zone passes every test of the tree and all six of the unit, so the rewritten branch is held by no test.
- file: crates/swell/src/interp/mod.rs
  line: 273
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: push_entry reserves the doubled operand stack before it charges it, so a growth refused by the memory budget leaves up to 1 MiB of uncharged capacity until the frame unwinds, the same on base and head.
- file: crates/swell/src/interp/dispatch.rs
  line: 280
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The frames of replace (64 to 96 bytes release, 240 to 256 stack-debug), jump_if (64 to 80) and pop_instruction (48 to 64) grew, while run (624 release), execute (816 stack-debug) and the re-entry path (1392 to 1248 stack-debug) did not.
```

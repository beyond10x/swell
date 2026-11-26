---
format: aep.planning-md/3
id: review-result:adversary-setmethods-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit setmethods: red, two failing cases of 8 and ten findings'
relations:
- reviews: story:set-methods
revision: 1
---
Adversary pass 1 on `impl/setmethods` at `d5791fc9` (wave big-2, slice 9, order 3), run on 2026-11-26.
The report below is the adversary's last message; paths of the workstation are replaced.

unit: setmethods (`story:set-methods`), worktree `swell-impl-setmethods` at `d5791fc9` plus my uncommitted cases
verdict: red
cases: executed 110→118, red 2
origin: introduced 9, pre-existing 1, undecided 0
wrote-outside-worktree: 1 directory, `<scratch>/wave2/setmethods/adversary-1/` (scripts, logs, a scratch clone `mutant/`); see part 6
needs-coordinator: yes. Two decisions: (a) for the stack finding, smaller frames in `methods.rs` or a larger `STACK_BYTES_PER_REENTRY` (that file is "not written unless the measured stack moves"); (b) for the time finding, which build the 2 s bound means, since the story fixes counts that go over it at opt-level 0.

**1. `git --no-pager diff --stat`**
```
 crates/swell/tests/builtins_set_methods.rs | 445 +++++++++++++++++++++++++++++
 1 file changed, 445 insertions(+)
```
Only a test file changed. Mutants and planted defects went into a scratch clone, never into the tree.

**2. Cases added** (all in `crates/swell/tests/builtins_set_methods.rs`)

Red now:
- `union_reaches_the_reentry_limit_through_next_on_the_promised_stack` (:1021): `reach_through` through a `next` that `union` calls from Rust, on 7/8 of `min_stack_bytes()`. Red in default mode, green in collect mode (depth 50).
- `union_reaches_the_reentry_limit_through_the_size_getter_on_the_promised_stack` (:1041): the same through the `size` getter. Red in default mode, green in collect mode.
- Red output of the first run of these alone (`mine2.sh default`, exit 100):
  ```
  FAIL [0.075s] builtins_set_methods::union_reaches_the_reentry_limit_through_next_on_the_promised_stack
  called `Result::unwrap()` on an `Err` value: "... on a stack of 12838560 bytes: signal: 6 (SIGABRT) ... thread '<unknown>' has overflowed its stack\nfatal runtime error: stack overflow, aborting"
  FAIL [0.066s] builtins_set_methods::union_reaches_the_reentry_limit_through_the_size_getter_on_the_promised_stack  (same, 12838560 bytes, SIGABRT)
  Summary 3 tests run: 1 passed, 2 failed
  ```

Green now; each one kills a mutant the 13 tests miss (planted run in the scratch clone, below):
- `every_way_out_of_a_walk_closes_its_cursor` (:633): counts cursors left open, exactly, from the extra steps of `clear()` against a control Set. Covers a normal answer, the result of `difference` kept by the script, a throw, the depth refusal, the step budget and the interrupt; the memory budget in default mode only.
- `every_read_of_a_set_like_in_the_order_of_the_clauses` (:751): a log of every read and call, ported from test262 `set-like-class-order.js` (on `close-later.tsv` because it uses `class`). It covers equal sizes, `next` read once, `return`, and a `next` that throws or answers a primitive. Collect mode runs 6 of its 15 cases.
- `a_set_like_that_mutates_this_during_the_call` (:883): `keys()` mutates `this` before the copy in `union` and `symmetricDifference`; `difference` copies before `keys()`; a key is yielded twice; `clear()` inside `has`.

Green, no mutant shown:
- `get_set_record_converts_each_kind_of_size` (:914): Symbol, a throwing `valueOf`, a `valueOf` that empties `this`, `-0`, 2**53, `" 3 "`, a Proxy as the argument and a Proxy of a Set as `this`.
- `the_result_takes_the_set_prototype_of_the_method_s_realm` (:948): a Set of another realm.
- `keys_held_only_by_a_callback_survive_a_collection` (:986): `intersection`'s walk with a `has` that deletes its argument; `union` and `symmetricDifference` whose keys come from a `value` getter.

Mutant run (`mutant-run.sh`, M1+M2+M3 planted together, default mode): `19 tests run: 16 passed, 3 failed`. All 13 unit tests passed on the mutants.
- `every_way_out` failed with `left: [6, 1, 0, 0, 0, 0]`.
- `a_set_like_that_mutates` failed with `"1,2,5|..."` where `"2,3,5|..."` is expected.
- `every_read` failed on the `isDisjointFrom` equal-size cases.

CPU time (user+sys) of my cases in the collect build at opt-level 0:

| Test | s |
|---|---|
| `every_way_out_of_a_walk_closes_its_cursor` | 1.34–1.42 |
| `keys_held_only_by_a_callback_survive_a_collection` | 1.10–1.50 |
| `every_read_of_a_set_like_in_the_order_of_the_clauses` (after the cut) | 0.58 |
| each of the two stack tests | 0.62–0.64 |

None is over 1.5 s.

**3. Suite run, after the cases existed** (`suite.sh`, the story's acceptance filter)
- Default mode: `118 tests run: 116 passed, 2 failed, 1212 skipped`, exit 100. The two failures are the stack tests above.
- Collect mode: `118 tests run: 118 passed, 1212 skipped`, exit 0.
- `110` before is the unit's reported count.

**4. Findings** (tree `d5791fc9` plus my cases)

- **F1, blocker, introduced. The stack needed through `union`'s `next` breaks the promise.**
  - Measured: `:1034` unwrap, SIGABRT at 12,838,560 bytes.
  - Bisection of the smallest thread stack per re-entry, at depths 200 and 1,000:

    | Profile | Bytes per re-entry | Need at depth 1,000 | Bound it breaks |
    |---|---|---|---|
    | dev (the gate's) | 13,567 | 13,719,517 | 7/8 of `min_stack_bytes()`, 12,838,560 |
    | `stack-debug` | 14,736 | 14,929,726 | the whole `min_stack_bytes()`, 14,672,640 |
    | release | 4,127 | — | none |

  - Existing paths that also call `next` from Rust take 10,287 (`new Set(iterable)`) and 10,815 (`Array.from`) bytes in dev. So about 3 KB per level is this unit's own frames: `set_method_steps` (`methods.rs:138`) and `read_keys` (`:362`).
  - What reaches it: any script calling `union`, `intersection`, `difference`, `symmetricDifference`, `isSupersetOf` or `isDisjointFrom` with a recursing `next`, on a host thread sized by the public `Limits::min_stack_bytes()`. The process aborts instead of throwing a RangeError.
- **F2, blocker, introduced. The same through the `size` getter read by GetSetRecord** (`methods.rs:235`).
  - Measured: `:1054`; 13,504 bytes per re-entry in dev (13,656,091 at depth 1,000) and 14,655 in `stack-debug` (14,848,909, over the whole promise).
  - What reaches it: all seven methods.
- **F3, blocker, introduced. `a_throwing_callback_leaves_no_cursor` (`:527`) is over the time bound.**
  - Measured: 3.82–5.15 CPU s at opt-level 0 in collect mode, 3.87 with `SWELL_FULL_COUNTS=1`. That build is the one the acceptance command and lane `slow` make (the module calls `stack_with_room`). Bounds: 2 s, and 2.5 s at full count. The gate's profile (opt-level 1) gives 0.59 s.
  - Smallest count that still fails for its defect: N=10, by arithmetic, not run. 30 leaked cursors keep at least 128 B against a threshold of 32 B. Below N=10 the threshold is 0 and the test fails with no defect.
- **F4, blocker, introduced. `the_result_keeps_its_keys_across_a_collection` (`:573`) is over the time bound.**
  - Measured: 3.67–5.88 s at opt-level 0, 3.97 at full count; 0.40 s in the gate's profile.
  - The story fixes 50 keys and junk arrays of 100. Two keys per method already fail without a root in collect mode (not run).
- **F5, blocker, introduced. `get_set_record_rejects_…` (`:95`) is over the time bound.**
  - Measured: 3.66–3.77 s at opt-level 0, already sloppy-only; 0.24 s in the gate's profile.
  - There is no count to lower; only fewer methods would cut it (not measured).
- **F6, warning, introduced. `union_with_an_endless_set_like_ends_at_the_memory_budget` (`:400`) is over 2 s.**
  - Measured: 2.11–2.16 s at opt-level 0; under the 2.5 s bound at full count.
- **F7, warning, introduced. Mutant M1 survives the 13 tests and the 129 cases.**
  - The change: `walk_with_has` (`methods.rs:453`) closes its cursor only on an abrupt completion. Every normal walk then leaves a cursor on a long-lived Set (6 places measured).
  - Killed by `every_way_out_of_a_walk_closes_its_cursor`.
- **F8, warning, introduced. Mutant M2 survives the 13 tests and the 129 cases.**
  - The change: `isDisjointFrom` compares with `<` instead of `<=` (`methods.rs:220`), so at equal sizes it iterates `keys()` instead of calling `has`.
  - Only `set-like-class-order.js` would catch it, and that case is on `close-later.tsv`. Killed by `every_read_…`.
- **F9, warning, introduced. Mutant M3 survives the 13 tests and the 129 cases.**
  - The change: `union` copies `this` before `keys()` is called (`methods.rs:150`).
  - Killed by `a_set_like_that_mutates_this_during_the_call`.
- **F10, note, pre-existing. Lane `slow` never runs any `set-methods` stack test in `stack-debug` or release.**
  - `test-stack-profiles` (`taskfiles/stack-profiles.yml:63`) runs four tests by name, unchanged since the base. That is how F1 and F2 got past the lane.

**5. Attacked, not broken**
- Every clause read against the pinned draft: the order of GetSetRecord's reads, which side each method iterates, `next` read once, IteratorClose where the clause closes it, `-0` canonicalized, a plain `Set` whatever the species, the realm of the result.
- Mutation during the walk: deleting, adding, re-adding and `clear()` in `has`, and compaction of the table.
- The collector: keys held only by a callback frame or by a `value` getter survive.
- Cursors: no cursor is left on any way out (normal, throw, depth, step, interrupt, memory).
- The three changed tests of other stories still fail for their defect. Planted in the clone (`plant-default.log`, `3 failed`):
  - P1, a table not credited after a collection: `1720151 after, 376551 before`.
  - P2, the temporary not charged: `assert!(high >= 2_048)` failed with 384.
  - P3, keys not charged before they exist: `381932 bytes held`.
- Code rules: I read all 517 lines of `methods.rs` and the 49 changed lines of `set/mod.rs`. There is no `unwrap`, `expect`, unchecked index or overflowing arithmetic, no growth that is not charged, and no value kept across a re-entry without a root.

Not done:
- I did not take a worktree session lease.
- The smallest counts for F3 and F4 are arithmetic, not runs.

Times, CPU s per test run alone (default / collect at opt-level 0 / the same with `SWELL_FULL_COUNTS=1` / gate at opt-level 1):

| Test | default | collect | full | gate |
|---|---|---|---|---|
| each_method | 0.035 | 0.78 | 0.76 | 0.05 |
| get_set_record | 0.094 | 3.77 | 3.66 | 0.24 |
| intersection_iterates | 0.033 | 0.96 | 1.03 | 0.07 |
| has_deletes_cursor | 0.025 | 1.14 | 1.49 | 0.10 |
| superset_closes | 0.029 | 0.83 | 0.78 | 0.06 |
| canonicalized | 0.029 | 0.83 | 0.70 | 0.07 |
| plain_set | 0.026 | 0.34 | 0.32 | 0.03 |
| step_budget | 0.148 | 1.51 | 1.54 | 0.11 |
| memory_budget | 0.063 | 2.16 | 2.11 | 0.30 |
| reentry_stack | 0.158 | 0.65 | 0.65 | 0.09 |
| interrupt | 0.012 | 0.04 | 0.05 | 0.01 |
| throwing_cursor | 0.224 | 5.15 | 3.87 | 0.59 |
| keeps_keys | 0.055 | 5.88 | 3.97 | 0.40 |

The timing runs were pool jobs, because the test binaries exist only on the build node.

**6. Paths written outside the worktree**

All under `<scratch>/wave2/setmethods/adversary-1/`:
- Scripts: `clause.sh`, `mine.sh`, `mine2.sh`, `suite.sh`, `timeloop.sh`, `timeloop2.sh`, `time-job.sh`, `time-all.sh`, `stack-probe.sh`, `mutant-run.sh`, `plant-run.sh`.
- Logs and markers: `mine-{default,collect}.log`, `mine2-{default,collect}.log`, `mutant-default.log`, `plant-default.log`, `suite-{default,collect}.log`, `suite2-{default,collect}.log`, `time-{default,collect,collect-full,gate}.log`, `time2-{default,collect}.log`, `stack-{dev,stack-debug,release}.log`, `stack-dev-2.log`, `time-all.done`.
- `mutant/`: a `git clone --shared` at `d5791fc9`. It holds the planted defects P1–P3 and the stack-probe tests, uncommitted; M1–M3 were reverted. The pool synced it to build node `build-2d`, so its build directory there also exists.

**7. Findings block**

```findings
- file: crates/swell/tests/builtins_set_methods.rs
  line: 1034
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Re-entering through the set-like's next from union needs 13,567 bytes per level in dev and 14,736 in stack-debug, so the thread overflows on seven eighths of min_stack_bytes() and in stack-debug on the whole promise.
- file: crates/swell/tests/builtins_set_methods.rs
  line: 1054
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Re-entering through the size getter read by GetSetRecord needs 13,504 bytes per level in dev and 14,655 in stack-debug, past the same two bounds.
- file: crates/swell/tests/builtins_set_methods.rs
  line: 527
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a_throwing_callback_leaves_no_cursor takes 3.8 to 5.2 CPU seconds in the opt-level 0 collect build that the acceptance and lane slow make, over the story's 2 s and 2.5 s bounds, while N=10 still catches its defect.
- file: crates/swell/tests/builtins_set_methods.rs
  line: 573
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the_result_keeps_its_keys_across_a_collection takes 3.7 to 5.9 CPU seconds in the opt-level 0 collect build at the counts the story fixes, over its own 2 s and 2.5 s bounds.
- file: crates/swell/tests/builtins_set_methods.rs
  line: 95
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: get_set_record_rejects_a_nan_size_a_negative_size_and_a_non_callable_has_or_keys takes 3.7 CPU seconds sloppy-only in the opt-level 0 collect build, over the 2 s and 2.5 s bounds.
- file: crates/swell/tests/builtins_set_methods.rs
  line: 400
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: union_with_an_endless_set_like_ends_at_the_memory_budget takes 2.1 CPU seconds in the opt-level 0 collect build, over the 2 s bound.
- file: crates/swell/src/builtins/set/methods.rs
  line: 453
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Closing the walk's cursor only on an abrupt completion passes all 13 tests, and every_way_out_of_a_walk_closes_its_cursor now catches it.
- file: crates/swell/src/builtins/set/methods.rs
  line: 220
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Changing isDisjointFrom's size test from <= to < passes the 13 tests and the 129 listed cases, and every_read_of_a_set_like_in_the_order_of_the_clauses now catches it.
- file: crates/swell/src/builtins/set/methods.rs
  line: 150
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Copying this before keys() in union passes the 13 tests and the 129 listed cases, and a_set_like_that_mutates_this_during_the_call now catches it.
- file: taskfiles/stack-profiles.yml
  line: 63
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: test-stack-profiles runs four stack tests by name, so no set-methods stack test ever runs in stack-debug or release, which is how F1 and F2 got past lane slow.
```

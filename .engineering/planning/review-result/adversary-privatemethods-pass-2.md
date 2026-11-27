---
format: aep.planning-md/3
id: review-result:adversary-privatemethods-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the unit privatemethods
relations:
- reviews: story:private-names-and-private-methods
revision: 1
---
Adversary pass 2 (the last) on the unit privatemethods, branch impl/privatemethods at e11fb57, 2026-11-27.
Verdict red: 2 failing cases (one blocker, one warning), six notes.

unit: privatemethods (story:private-names-and-private-methods), pass 2 of 2, head e11fb57
verdict: red (1 blocker, 1 warning, 6 notes)
cases executed before→after: 73→86 (default), 72→81 (collect); swell-compiler 23→23 (the new fixture runs inside `disassembly_matches_for_each_source`), red 2
origin: introduced 6, pre-existing 1, undecided 0 (one finding comes from the merged wave's lines: bed5416)
wrote-outside-worktree: <scratch>/wave2/privatemethods/adversary-2/ only (scripts, logs, diffs)
needs-coordinator: yes. The blocker regresses 7 test262 cases that passed on the base. `for-coordinator.md` records the refusal outside every class as intended until `story:private-names-in-eval-code`. Someone has to decide whether that regression is accepted.

```
 crates/swell/tests/private_methods.rs | 880 ++++++++++++++++++++++++++++++++++
 1 file changed, 880 insertions(+)
?? crates/compiler/tests/sources/class-private-static-room.disasm
?? crates/compiler/tests/sources/class-private-static-room.js
```

## Findings

1. **Blocker, introduced by ae7e38f** (`crates/swell/src/dynamic_code.rs:1047`, called at :412).
   - The refusal rewrites every "private name #x is not declared in an enclosing class", including direct eval outside every class and undeclared names inside a class.
   - On the base, 7 test262 cases passed (14 runs); now they Fail with "counted refusal: 1 refusal(s)":
     - `built-ins/eval/private-identifiers-not-empty.js`
     - `privatename-not-valid-eval-earlyerr-1`, `-2`, `-6`, `-7` and `-8`
     - `staging/sm/PrivateName/error-outside-class.js`
   - Fix: refuse only names that a class enclosing the eval site declares.
   - Failing case: `adversary2_a_direct_eval_outside_every_class_is_the_early_error`.
2. **Warning, introduced by ae7e38f** (`dynamic_code.rs:1047`, `script.rs:422`).
   - In `eval('let a; let a; this.#x;')` the grammar error is the one thrown, but a refusal is still counted.
   - This breaks the syntax crate's `only_the_refusal` contract, so the test262 runner would score a correct SyntaxError as a refusal.
   - Failing case: `adversary2_an_early_error_before_a_private_name_counts_no_refusal`.
3. **Note, test gap at the lowered counts of e11fb57.**
   - Plant M2 (`private_names.rs:341`, `length < capacity` → `<=`) stays green in the collect build for both lowered tests, and in default for every test the unit wrote.
   - Caught by the library test `a_mark_from_the_object_reaches_every_element` and by my `adversary2_the_room_of_each_shape_and_level`: left `[0,112,68,68,68]`, right `[0,112,68,288,684]`.
   - Plants C and "once" (room charged for the first list only) go red in both builds at :538 and :1188.
4. **Note, pre-existing test design** (`proxy_shape_traps.rs:2105`, `memory_sweep`).
   - The sweep checks bytes only. Two leak plants stay green at k=0, 11 and 12:
     - `json/mod.rs:189` credits only on Ok;
     - `proxy.rs:259`, in `in_proxy_method`, releases and lowers the nesting only on Ok.
   - The defect that 04a4a65 fixed, planted back, goes red as expected (see item 1b below).
5. **Note, contract drift** (`docs/design/engine.md` § Memory budget).
   - The doc does not say that `heap_bytes()` depends on whether the budget made the agent collect or grow an arena. 114,688 bytes at k=11/12.
   - The host can see it through `heap_bytes()`. Script can only see whether a later entry ends in MemoryBudget.
6. **Note, surviving mutant Z1 of ae7e38f** (`crates/compiler/src/class.rs:574-577`).
   - Change: the static add's room gets `.min(1)`.
   - The unit's suite stays green. Caught by my new fixture `class-private-static-room.js`: "the disassembly differs".
7. **Note, surviving mutant Z2 of bed5416** (`crates/compiler/src/resolve.rs:1404`).
   - Change: `first_instance` becomes `class.instance_elements`.
   - The compiler tests and the unit's parts stay green. My `adversary2_fields_private_methods_and_super_in_one_class` catches it: the two nested-through-initializer cases throw TypeError.
8. **Note, surviving mutant Z3 of ae7e38f** (`crates/swell/src/script.rs:439`).
   - Change: TooManyInClass leads to a fault.
   - The unit's suite stays green. Caught by my `adversary2_a_class_past_the_private_record_limit_is_a_syntax_error` (32,769 methods, 1 GiB budget).

Equivalent mutants, not findings:
- the order of `this_binding` and `private_adds`;
- children capacity `+6` → `+3`;
- the `SlotValue::PrivateName` fault arms;
- dropping `boxed &&`.

## Items

- **1a.** The sweep does not pass silently: after 4 readings it panics with "the base does not settle".
- **1b.** Plant `if heap == base || guesses.len() == 1` (the old base):
  - k=0: green;
  - k=11: red at `proxy_shape_traps.rs:2205`, "base 622553, heap 507865; 0 ended in it ... 95 answered";
  - k=12: red, base 622821, heap 508133.
- **1c.** Tests that use one agent's heap as another agent's budget: the 24 listed in the pass notes, among them:
  - `private_methods::instances_are_charged_to_the_memory_budget`;
  - `budgets_never_leave_a_half_branded_instance`;
  - `proxy_shape_traps::memory_sweep`;
  - `classes::a_class_made_in_a_loop_is_charged_and_freed`;
  - `class_fields::constructions_of_a_class_with_100_fields_are_charged_and_freed`.

  The lane `test-realm-sizes` ended with `realm-sizes: 14 run(s) red`. Every red run is my case `adversary2_a_direct_eval_outside_every_class_is_the_early_error`; nothing else failed at 7 realm sizes in both builds.
- **1d.** See finding 5.
- **2.** Finding 1 and finding 2. The message is built from the parsed name, so no string literal or comment can forge it. The line and column survive: (1:3), (1:6).
- **3.** All green.
  - Rises per level: 1: 68, 4: 200, 5: 244, 9: 420, 17: 772.
  - Steps at 1/7/8/9/10/16/17/1000 private methods: 20/38/41/44/48/93/104/494553. These match the formula.
  - 8-byte budget sweep: 205 runs ended in the construction, 212 before it, 8 answered. Then 1,000 failed constructions; `heap_bytes()` is back at its reading after each run.
- **4.**
  - (a) No script reaches the fault arms.
  - (b, c) Resolver shapes, and super with private names: green.
  - (d) The fixtures of fields and derived are unchanged since edfd254.
- **5.** Heritage order, TDZ, eval in the heritage, a throw in the class leaks nothing, env hops: all green.
- **6.** SuperCall with a second `super()` and a Proxy: constructor called 2 times, initializer 1 time, ReferenceError, traps `f:true,kind:true`. This matches Construct → BindThisValue → InitializeInstanceElements.
- **7.** CPU seconds, 3 runs each, load 4.6 to 7.8 on 32 cores. No test is over its bound.
  - Highest of the unit's tests:
    - default: `budgets_never_leave_a_half_branded_instance` 1.151 / 1.157 / 1.157;
    - collect: `instances_are_charged_to_the_memory_budget` 1.423 / 1.456 / 1.424;
    - full count: the same test at 1.265 / 1.265 / 1.267.
  - `derived_classes::the_fields_of_a_derived_class_are_defined_when_super_returns`: 0.043 default, 0.470 collect, 0.044 full.
  - The Proxy sweep: 0.679 default, 0.676 full. It does not run in the collect build.
  - My own `adversary2_a_throw_in_the_class_leaks_no_private_name` took 2.23 in collect. I lowered it from 20 to 8 throws; now 0.917 (load 10.7).
  - My tests over 1 s, all default only: the limit test 1.32 and the budget sweep 1.25.

## Cases added (uncommitted)

In `crates/swell/tests/private_methods.rs`:
- `adversary2_this_and_a_private_record_in_one_derived_constructor`
- `adversary2_fields_private_methods_and_super_in_one_class`
- `adversary2_super_meets_private_names`
- `adversary2_the_heritage_and_the_private_record`
- `adversary2_a_second_super_with_a_proxy`
- `adversary2_an_early_error_before_a_private_name_counts_no_refusal` (red)
- `adversary2_a_direct_eval_outside_every_class_is_the_early_error` (red)
- `adversary2_a_throw_in_the_class_leaks_no_private_name`
- `adversary2_the_room_of_each_shape_and_level`
- `adversary2_the_steps_of_a_construction_by_private_methods`
- `adversary2_memory_budget_at_every_8_bytes_across_two_levels`
- `adversary2_this_after_super_in_the_same_arrow`
- `adversary2_a_class_past_the_private_record_limit_is_a_syntax_error`

Also the fixture `crates/compiler/tests/sources/class-private-static-room.{js,disasm}` (green).

## Commands and result lines

- test262 part run, the 242 cases with eval and a `#` name: `test262 total 242 2 0 240 110 130 0`, `test262-refused 238`; 14 regressions against base 6f21df6d.
- `cargo nextest run -p swell --test main -E 'test(/^(private_methods|derived_classes|class_fields)::/)'`:
  - default: `86 tests run: 84 passed, 2 failed`; without my cases `73 passed`;
  - collect: `81 tests run: 79 passed, 2 failed`; without my cases `72 passed`.
- `cargo nextest run -p swell-compiler --lib --test main`: `23 tests run: 23 passed`.
- `slot --timeout 30m task test-realm-sizes`: `realm-sizes: 14 run(s) red` (all my red case).
- `cargo clippy -p swell --test main -- -D warnings`: clean. `cargo fmt --check -p swell`: rc 0.
- Plant exit codes:

| Plant | Exit codes |
|---|---|
| json k0/11/12 | 0 / 0 / 0 |
| proxy k0/11/12 | 0 / 0 / 0 |
| base k0/11/12 | 0 / 100 / 100 |
| C collect/default | 100 / 100 |
| M2 collect/default | 0 / 100 (mine only) |
| once collect/default | 100 / 100 |
| Z1 compiler/parts | 100 (my fixture) / 100 (only my 2 red) |
| Z2 compiler/parts | 0 / 100 (mine) |
| Z3 parts | 100 (mine) |

- Every plant was restored with `git show HEAD:<path>`. `git status` shows only my test file and the fixture.

## Not attacked

- The full workspace gate, and test262 beyond the 242 eval-with-`#` cases.
- The slow test `proxy::memory_budget_in_a_trap_at_depth_leaves_no_count_and_no_bytes` (6.0 s wall in the lane at k=128/200 collect): it is not one of this unit's tests and was not timed alone.

```findings
- file: crates/swell/src/dynamic_code.rs
  line: 1047
  category: regression
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'direct eval refusal also rewrites an undeclared private name outside every class or undeclared in the class; 7 test262 cases that passed on the base now fail with counted refusal (private-identifiers-not-empty, earlyerr-1/2/6/7/8, sm error-outside-class); failing case adversary2_a_direct_eval_outside_every_class_is_the_early_error'
- file: crates/swell/src/script.rs
  line: 422
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'a refusal is counted beside an earlier grammar error that is the one thrown (only_the_refusal); failing case adversary2_an_early_error_before_a_private_name_counts_no_refusal'
- file: crates/swell/tests/private_methods.rs
  line: 538
  category: test-gap
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'plant M2 (private_names.rs:341 length <= capacity) stays green in the unit tests at the lowered counts; caught by the lib test and adversary2_the_room_of_each_shape_and_level'
- file: crates/swell/tests/proxy_shape_traps.rs
  line: 2105
  category: test-gap
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: 'memory_sweep checks bytes only; json credit-on-Ok and proxy release-on-Ok plants stay green at k 0/11/12; the old-base plant goes red at k 11 and 12'
- file: docs/design/engine.md
  line: 2133
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: 'Memory budget section does not say heap_bytes depends on whether the budget made the agent collect or grow an arena (114688 bytes at k 11/12)'
- file: crates/compiler/src/class.rs
  line: 574
  category: test-gap
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'mutant static room .min(1) survives the unit suite; caught by fixture class-private-static-room.js'
- file: crates/compiler/src/resolve.rs
  line: 1404
  category: test-gap
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'mutant first_instance to class.instance_elements survives the compiler and unit tests; caught by adversary2_fields_private_methods_and_super_in_one_class'
- file: crates/swell/src/script.rs
  line: 439
  category: test-gap
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'mutant TooManyInClass to fault survives the unit suite; caught by adversary2_a_class_past_the_private_record_limit_is_a_syntax_error'
```

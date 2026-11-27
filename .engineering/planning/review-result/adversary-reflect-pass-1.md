---
format: aep.planning-md/3
id: review-result:adversary-reflect-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit reflect: green, 5 notes, 12 green cases'
relations:
- reviews: story:reflect-namespace
revision: 1
---
Adversary, pass 1 on the unit reflect of wave big-2 (impl/reflect at 3cc59442), 2026-11-26.
Verdict: green. 5 findings, all notes, all introduced; 12 cases added, none red.

No red case: the unit held against every attack in the brief, in both collection modes. What I'm returning is five notes on what its tests cannot catch.

```
unit: reflect (story:reflect-namespace), branch impl/reflect, head 3cc59442, worktree clean
verdict: green
cases: executed 13→25, red 0
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: 8 files in <scratch>/wave2/reflect/adversary-1
needs-coordinator: no
```

**1. Diff.** `git --no-pager diff --stat` is now empty. While my cases were in, it was `crates/swell/tests/builtins_reflect.rs | 476 +++`, a test file only. Implementation files: none touched. I planted no mutant in a tracked file; my procedure forbids it, so the mutants below are reasoned from the code, not run.

**2. My cases.** 12 tests named `adversary_*`, saved as `adversary-cases.patch` in the scratch folder.
- First run, my cases alone, default mode: 3 red, all my own mistakes, not defects:
  - `left: ...,false,,true` vs `right: ...,false,undefined,true`: `join` writes `undefined` as an empty string.
  - `Errors(["new.target is not supported before slice 8 (2:34)"])`: my script used a feature that comes later.
  - `heap_bytes: 365812 after the warm-up round, 367412 after three more rounds`: arena growth from three rounds with no collection between them.
- I fixed all three and made the charge test take a reading after every entry. Then:
  - Default mode: 12/12 green. Every reading is 365,812 for each abrupt path and for the controls through `Function.prototype.apply`.
  - Collect mode: 12/12 green. Slowest cases: charges 4.08 s, values-survive-collections 1.89 s, order 1.37 s. None over 5 s.
- Because none of my cases is red, I removed them from the tree. Leaving them would also break the acceptance's count of 13 tests.

**3. Suite run (with my cases present).**
- `cargo nextest run --locked -p swell --test main -E 'test(/^builtins_reflect::/)'`: `25 tests run: 25 passed`, exit 0.
- The same with `--features collect-every-safe-point`: `25 tests run: 25 passed`, exit 0. The unit's slowest test there is 0.98 s.

**4. Findings** (covering 3cc59442 and the clean tree; all reached by the gate's own test steps):
- **F1** `builtins_reflect.rs:551` `reflect_own_keys_ends_at_an_interrupt`: the getter raises the interrupt inside the script's loop after `Reflect.ownKeys` has returned. So no interrupt check inside `ownKeys` ever runs, and a version that ignored the interrupt would still pass. It can't be tested on one thread either: the builtin checks the interrupt on entry (`charge_steps(1)`) before its own code runs. Found by reading.
- **F2** `builtins_reflect.rs:471` (memory test): the per-element charge it is meant to guard is in `function.rs:226`. The growth of the kept stack (`reserve_kept`) is charged too, so the run still ends at `MemoryBudget` if that charge is removed. The test can't tell the difference. Found by reading; not planted.
- **F3** `reflect.rs:368`: if `credit(bytes)` in `array_from_keys` were dropped, no unit test would notice (none reads `heap_bytes` around `ownKeys`), and test262 cannot see charges. My charge test's `ownKeys` row (five readings must equal the first) catches it.
- **F4** `reflect.rs:279`: if `Reflect.get` treated an explicit `undefined` receiver as missing, nothing would catch it. The unit tests and the story's `Reflect/get` cases only use a missing receiver or an object; `staging/sm/Reflect/get.js` is in slice 19. Caught by `Reflect.get({ get x() { "use strict"; return this; } }, "x", undefined) === undefined`.
- **F5** `reflect.rs:258`: if `ordinary_delete` replaced `internal_delete`, a dense array element would stay in place while `true` is returned. No unit test calls `deleteProperty`, and the test262 cases on the list use plain objects only. Caught by `Reflect.deleteProperty(a, 0)` followed by `0 in a` === false. (The other test262 check I tried did catch its mutant: swapping key and descriptor order fails `Reflect/defineProperty/return-abrupt-from-property-key.js`, so that is no finding.)

**5. Attacked, could not break:**
- **Clauses:** all 13 read against their clauses, with step order checked by script: key before descriptor, a receiver or `newTarget` given as `undefined` still counts as present, a missing argument is `undefined`, and `false` is answered without a throw.
- **Exotic targets:** arrays (non-deletable element and `length`, a receiver whose `length` refuses, keys above 2^32 − 2), String objects, mapped `arguments` (including a different receiver), bound functions as target and as `newTarget`, the global object, and a host object of the test host.
- **Hostile argument lists:** a getter that shrinks the list, `length` as a Symbol, a String, -Infinity, a fraction or an object, and a `length` getter that throws.
- **Re-entry:** cycles with only builtins between calls, on 7/8 of `min_stack_bytes()`, all end in RangeError in both modes.
- **Charges:** credited on every abrupt way out, including a termination inside the callee.
- **Collections:** values stay intact in collect mode. The note at `reflect.rs:349` is true.
- **Code rules:** 391 of 391 lines read: no `unwrap` or `expect`, no unchecked index, no overflow. Both callers of `create_list_from_array_like`, now `pub(crate)`, end the kept room and credit their charge on every path.
- **The unit's stated departures:** in collect mode the type-error test uses 2 of 6 primitive kinds, but the default mode the gate runs uses all 6.
- **test262 outside the part:** 132 runs (67 cases) of the base run fail with `Reflect is not defined`: slice 09: 8 (Map, Set, WeakMap and WeakSet `proto-from-ctor-realm`), 13: 6, 14: 6, 15: 16, 16: 12, 19: 84 (staging/sm: Iterator 50, Reflect 20, RegExp 6, Array 3, object 2, strict 1). None is on the story's list; all are for your join run. The part run has 1,446 runs, all passed.

**6. Written outside the worktree** (all in `<scratch>/wave2/reflect/adversary-1/`): `base-reflect.tsv`, `part-paths.txt`, `run1-default.log`, `run2-default.log`, `run3-collect.log`, `run4-suite-default.log`, `run5-suite-collect.log`, `adversary-cases.patch`.

Failing cases left in the tree: none.

```findings
- file: crates/swell/tests/builtins_reflect.rs
  line: 551
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the interrupt test raises and catches the interrupt in the script loop after Reflect.ownKeys returned, so it passes for an ownKeys that never checks the interrupt.
- file: crates/swell/tests/builtins_reflect.rs
  line: 471
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the memory test still ends at MemoryBudget without the per-element charge of CreateListFromArrayLike, because the kept-stack growth is charged too, so it cannot detect that charge missing.
- file: crates/swell/src/builtins/reflect.rs
  line: 368
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: dropping the credit of the ownKeys value list survives all 13 tests and test262; a per-entry heap_bytes reading around Reflect.ownKeys catches it.
- file: crates/swell/src/builtins/reflect.rs
  line: 279
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Reflect.get treating an explicit undefined receiver as missing survives the 13 tests and the story's test262 list; a strict getter that returns this catches it.
- file: crates/swell/src/builtins/reflect.rs
  line: 258
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: calling ordinary_delete instead of internal_delete leaves a dense array element in place and survives the 13 tests and the story's test262 list; deleting a dense array element and checking `in` catches it.
```

---
format: aep.planning-md/3
id: review-result:adversary-privatemethods-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit privatemethods: red, the lane''s realm sizes fail in one test, and six more findings'
relations:
- reviews: story:private-names-and-private-methods
revision: 1
---
Adversary pass 1 on `impl/privatemethods` at `a3089e7` (wave big-2, slice 8, order 3), run on 2026-11-27.
The report below is the adversary's last message; paths of the workstation are replaced.

unit: privatemethods
verdict: red
cases: executed 19→28, red 0
origin: introduced 7, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/privatemethods/adversary-1 (scripts, logs, and the exports `plant/` of HEAD and `base/` of 158888d)
needs-coordinator: yes

Verdict is red because the lane `test-realm-sizes` fails on this head and passes on `158888d`. The `cases` line counts `private_methods::` (default mode: 18 → 26; collect mode: 18 → 25) plus the `private_names` library tests (1 → 2). None of my cases is red at this head.

**Findings**

1. **The lane `test-realm-sizes` is red (blocker, introduced by a341e8d).**
   - `slot --timeout 30m task test-realm-sizes` printed `realm-sizes 11 default: 333 passed, 1 failed`, then `failed swell::main proxy_shape_traps::memory_budget_at_every_64_bytes_through_json_stringify_of_three_proxies` and `realm-sizes: 1 run(s) red`.
   - Running that one test with the realm padding k set to 0, 4, 10, 11, 12 and 17: on `158888d` all pass. On the worktree k = 11 and k = 12 fail at `proxy_shape_traps.rs:2143` with `0 ended, 95 answered` (the test's budget base is 622553 against a heap of 507865). k = 10 and k = 17 pass.
   - The 8 bytes added to every object move an arena boundary. The test's way of computing its budget base is the fragile part. The unit did not run this lane.
2. **No test catches the element list not being traced (warning, introduced).**
   - I planted the defect in `plant/` (`ObjectRecord::trace` skips `elements.trace(mark)`). All 18 of the unit's tests and my cases stayed green: `25 tests run: 25 passed` in default mode and `24 passed` in collect mode, so at GARBAGE 50 and at GARBAGE 5.
   - The unit's claim that `o.run()` then fails does not hold. The class's private record holds the same Private Names and functions, and every method that can read the elements holds that record.
   - My library case `a_mark_from_the_object_reaches_every_element` goes red under the plant: `PrivateName(Handle { key: 1, index: 0, generation: 0 }) is not marked`.
   - This matters for the next story: private fields' values are held nowhere else.
3. **The private record is pushed before the heritage, but the story says after it (warning, introduced).**
   - In `class.rs:136-137`, `enter_private_record` sits right after the class scope, where `impl/derived` evaluates the heritage.
   - `git merge-tree --write-tree HEAD impl/derived` reports `CONFLICT (content): Merge conflict in crates/compiler/src/class.rs` exactly there. The story does not name this conflict.
   - The resolver walks the heritage before `Walk::Private`. A "keep both" resolution therefore runs the heritage with the private record on top, and every env hop the heritage resolved is off by one.
4. **Direct eval in a private method gives a wrong SyntaxError instead of a stated refusal (note, introduced).**
   - `class C { #m(){ return 1 } f(){ return eval('this.#m()'); } }` answers `SyntaxError: private name #m is not declared in an enclosing class (1:6)`.
   - The spec allows this access. There is no "… is not supported before slice 8" refusal. It became reachable when the hold was lifted; the fix belongs to the later story `private-names-in-eval-code`.
5. **The u16 limit is reached at 32,769 private methods, not 65,536 (note, introduced).**
   - Each method takes two slots. 32,767 and 32,768 methods (instance or static) compile. 32,769 is refused with `more private record slots in one function than the bytecode can number`.
   - The text says "in one function", but the limit is the class's private record.
6. **Each instance pays the list's doubling room, and each construction spends about n²/2 steps (note, introduced).**
   - An entry is about 44 bytes and the first room is 4 entries (`private_names.rs:97,333`).
   - Measured per instance over a plain instance: 205 bytes with 1 method, 211 with 4, 389 with 5, 751 with 9, 1,476 with 17.
   - Every add scans the list, so a construction costs about n²/2 steps: about 496,000 for 1,000 methods, out of 1,000,000 per entry.
   - The class's count of methods is fixed at compile time, and the spec's note on ClassDefinitionEvaluation allows one brand per class.
7. **The assertion at `private_methods.rs:534` holds even when the list is not charged (note, introduced).**
   - With `charge_bytes` removed (plant C), `with_private >= with_plain + 16 * count` still passed. The test went red only at `:564` (`left: 415751, right: 86951`).
   - The count still catches the defect, but not by the mechanism the unit describes.

**Plants (in `plant/`, red at the gate counts)**
- A (list not traced): none of the 18 go red, in either mode.
- B (the sweep never frees a Private Name): `private_names_are_freed_when_nothing_holds_them` goes red in default mode (1644718 vs 1448110) and collect mode (410666 vs 407594). So `evaluations()` at 20 still catches it.
- C+D (no list charge, no step per comparison):
  - `a_lookup_past_small_spends_a_step_per_comparison`: default 1523 vs 1623; collect 173 vs 183. So `ACCESSES` at 10 still catches it.
  - `instances_are_charged_to_the_memory_budget:564`: still caught at `instances()` 10.
  - Also red: `private_names_are_freed…`.
- The three mutants the suite misses (item 11):
  - M1 is plant A.
  - M2: `length < capacity` changed to `<=`, so growth past 4 entries is uncharged. All 18 stay green. My `adversary_an_instance_pays_for_the_room_of_its_list` goes red (`[516476, 562092, 563123]`).
  - M3: an instance private method's home object set to F (`class.rs:348` `Dup2; Pop` changed to `Dup`). All 18 stay green, and the 1,977-case list has no `super` case. My `adversary_super_in_a_private_method_reads_its_home_object` goes red.

**Other readings**
- Brand checks and early errors (items 1–3):
  - Every base, every form and every evaluation order the brief listed matches the clauses.
  - `get #g`'s name cannot be observed from script.
  - Unreachable without derived classes: initializing an instance twice, and a frozen instance before its methods are added.
- Budget sweeps: the memory budget stepped 64 bytes at a time and the step budget one step at a time (1 to 139). Each run ended in `true` or the termination. A later script found no instance (`'none'`) or a whole one, never a half-branded one.
- Realm after `create_realm` (k = 0):
  - heap_bytes: 389,722 on `158888d`, 393,818 on head.
  - live_bytes: 277,947 on `158888d`, 281,115 on head.
  - host_187 BINDINGS: 394,525 on `158888d`, 398,621 on head.
  - The story names the 208 bytes.
  - The list could live in a side table keyed by object handle. That costs 0 bytes in the record, but every access then pays a map lookup and the sweep needs its own pass.
- JSON test (bisected to 64 KiB):
  - `158888d` needs 65,280 KiB (63.75 MiB) of its 64 MiB, so it was already within 0.4 % of its limit.
  - Head needs 66,304 KiB (64.75 MiB). The 72 MiB budget leaves about 10 % and still shows a result at 100,000 levels.
  - With the test limits lowered to 95 % (60.8 MiB), at full counts: `1532 tests run: 1532 passed` in swell::main, default mode. No other test there is within 5 % of that limit.
- Stack and frame:
  - Stack per re-entry, dev profile, found by bisecting the thread size at depths 100 and 300:
    - plain call: 7,312
    - public getter: 5,727
    - private getter: 5,551
    - private setter: 5,695
    - `Reflect.construct` of a class whose constructor calls a private method: 7,568
  - All are against a `STACK_BYTES_PER_REENTRY` of 14,336.
  - The frame size is unchanged: the `Frame` struct is not in the diff, and `SlotValue` stays 16 bytes.
- Times: all within bounds.
  - Default: largest 1.180 s, `adversary_budgets_never_leave_a_half_branded_instance` (default mode only).
  - Full counts: largest 1.243 s.
  - Collect: largest 1.507 s.
  - Totals: default 2.556 s, full 4.382 s, collect 11.038 s.
- Resolver and compiler: no `unwrap` or `expect` outside tests, no recursion, and the u16 narrowing is checked. Only the new `.disasm` changed.

**Cases added (uncommitted)**
- `crates/swell/tests/private_methods.rs`:
  - `adversary_brand_check_order_on_every_base`
  - `adversary_the_brand_lands_before_the_parameters_and_on_the_object_made`
  - `adversary_early_errors_of_private_names`
  - `adversary_super_in_a_private_method_reads_its_home_object`
  - `adversary_suspended_private_methods_across_collections`
  - `adversary_budgets_never_leave_a_half_branded_instance` (default mode only)
  - `adversary_an_instance_pays_for_the_room_of_its_list`
  - `adversary_a_lookup_by_name_passes_the_private_record`
- `crates/swell/src/object/private_names.rs`: `a_mark_from_the_object_reaches_every_element`
- None is over 5 s.

**Commands**
- `cargo nextest run --locked -p swell --test main -E 'test(/^private_methods::/)'` → `26 tests run: 26 passed, 1507 skipped`
- Same with `--features collect-every-safe-point` → `25 tests run: 25 passed, 1486 skipped`
- `-p swell --lib -E 'test(/private_names::/)'`, both modes → `2 tests run: 2 passed`
- `slot --timeout 30m task test-stack-profiles` → `release: … 4 passed, 0 failed, 5 skipped`; `stack-debug: … 4 passed, 0 failed, 5 skipped`
- `slot --timeout 30m task test-realm-sizes` → `realm-sizes: 1 run(s) red` (finding 1)
- `slot --timeout 30m task slow-tests` → `slow-tests: 4 test binaries: 1004 passed, 0 failed, 1040 ignored`; `slow-tests collect-every-safe-point: 2 test binaries: 1331 passed, 0 failed, 455 ignored`
- Plants: the logs `plant-{A,B,CD,M2,M3}-*.log` in the work directory.

**Not attacked**
- test262: I did not run the part with the mutants. M3 not being covered is concluded by grepping the case list.
- The frame size on the base was not measured; it follows from the struct being unchanged.
- The 95 % memory check covered only swell::main in default mode, not other crates and not collect mode.
- Derived classes and fields, beyond the `merge-tree` reading.

```findings
- file: crates/swell/tests/proxy_shape_traps.rs
  line: 2143
  category: boundary
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: 'test-realm-sizes is red: memory_budget_at_every_64_bytes_through_json_stringify_of_three_proxies fails at realm padding k = 11 and 12 (0 ended, 95 answered) on this head and passes at every k on 158888d, because the 8-byte record growth moved an arena boundary.'
- file: crates/swell/tests/private_methods.rs
  line: 387
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: With ObjectRecord::trace skipping the private elements, all 18 tests stay green in both modes at GARBAGE 50 and 5, because the class's private record holds the same names and functions; only the added library case a_mark_from_the_object_reaches_every_element goes red.
- file: crates/compiler/src/class.rs
  line: 137
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The private record is pushed where impl/derived evaluates the heritage (an unnamed merge conflict in class.rs), while the story says after the heritage and the resolver walks the heritage outside the private scope, so keeping both lines puts the heritage's env hops off by one.
- file: crates/syntax/src/parser/class.rs
  line: 396
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Since the hold was lifted, a direct eval of this.#m in a private method answers a SyntaxError that the spec does not give (private name #m is not declared in an enclosing class) instead of a stated refusal until story private-names-in-eval-code.
- file: crates/compiler/src/class.rs
  line: 367
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The u16 fields refuse a class at 32,769 private methods (two slots each), not 65,536, with the text "more private record slots in one function than the bytecode can number", which names a function where the limit is the class's private record.
- file: crates/swell/src/object/private_names.rs
  line: 333
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The list grows by doubling from 4 entries of about 44 bytes (205 bytes per instance for 1 private method, 1,476 for 17) and every add scans it (about n squared over 2 steps per construction), although each class's count of instance methods is fixed at compile time.
- file: crates/swell/tests/private_methods.rs
  line: 534
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: With the list's charge_bytes removed, the assertion with_private >= with_plain + 16 * count still passes and the test goes red only at line 564, so this assertion does not check the charge it describes.
```

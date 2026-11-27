---
format: aep.planning-md/3
id: review-result:adversary-privatefields-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit privatefields
relations:
- reviews: story:private-fields-and-private-in
revision: 1
---
Adversary pass 1 on the unit privatefields of wave big-2, impl/privatefields at 3ebbc21, 2026-11-27.
Green: no failing case, eight findings (two warnings on what the tests pin). The adversary's own report follows.

unit: privatefields at 3ebbc21 (base 8857ac25, unit commit dcd4faa). Tree is clean: `git status --short` and `git --no-pager diff --stat` are both empty.
verdict: green
cases: executed 14→14, red 0
origin: introduced 6, pre-existing 1, undecided 1
wrote-outside-worktree: <scratch>/wave2/privatefields/adversary-1/
needs-coordinator: no

I found no defect in what the code does. All eight findings are gaps in what the tests pin, one stale doc line, one design-rule gap that is older than the unit, and one time reading. As the brief ordered, plants and mutants were applied to the worktree itself, one at a time, and the original file was copied back after each run. My probes all pass, so none stays in the tree. Their text is in `adversary-1/probe-*.rs`.

**Findings**

1. **M1 (warning, introduced).** In `crates/swell/src/object/private_names.rs:287`, passing `room.min(1)` instead of `room` to `add_private_element` passes all 14 unit tests and all 284 library tests (`plants.log`). The doc of `instances_with_100_private_fields_are_charged_and_freed` says "the first add of a level reserves room for all of its elements once", but the byte count cannot show that. A test that catches it (kept as a note, because the code is right): a library test that calls `private_field_add` with room 3 on a fresh object, then adds the level's two other elements. It checks that the charges are `[24 + 3*48, 0, 0]`.
2. **M2 (warning, introduced).** In `crates/swell/src/interp/private.rs:189`, saving and restoring `self.steps` around `private_element_find` (so `#x in` costs no steps) passes the 14 unit tests and the library tests. Only my probe `adversary_private_in_charges_the_find` catches it: `#f99 in o` minus `#f0 in o` is 92 steps, and the mutant gives 0 (left 0, right 92). Its text is in `probe-notes.rs`.
3. **M3 (note, introduced).** In `crates/compiler/src/class.rs:224`, calling `private_field_room(c, class, id, false)` for static fields passes the 14 unit tests and the library tests. Only the compiler fixture `compile::disassembly_matches_for_each_source` (`compile.rs:143`) catches it, through one class's disassembly. No behavioural test pins the room of a static level.
4. **M4 (note, introduced).** In `crates/compiler/src/resolve.rs:1585`, changing `if *is_static` to `if false` counts static fields into the instance room. The 14 unit tests and the library tests pass. It is caught by the same compiler fixture and by my `adversary_instance_room_ignores_static_fields` (216 against 72 bytes per instance of `{#a=1; static #s; static #t; static #u}`). Its text is in `probe-notes.rs`.
5. **Plant B (note, INFEASIBLE, undecided).** In `crates/swell/src/object/ecmascript_function.rs:118`, not tracing the environment of a field initializer function stays green in both builds (19/19, and library 284 and 280). The function's environment is the class-scope environment of the running frame, and running frames trace their environment chain. No test can observe this plant.
6. **Find charged after the scan (judgement, note, pre-existing).** `private_element_find` in `crates/swell/src/object/private_names.rs:125-134` scans the whole list first and charges `compared - SMALL` steps afterwards. The design charges steps before the work. The code dates from a341e8d, before the unit. With the k-th add charging max(0, k−9) steps, n fields cost about n²/2 steps: 5,000 fields took 12,472,553 steps.
7. **Stale doc (contract-drift, note, introduced).** dcd4faa deleted `Parser::hold`, but three doc lines still name it: `crates/syntax/src/lib.rs:241`, `crates/syntax/tests/sources.rs:264` and `crates/syntax/tests/sources.rs:1174`.
8. **Full-count time (judgement, note, introduced).** `instances_with_100_private_fields_are_charged_and_freed` with `SWELL_FULL_COUNTS=1` uses 1.960 s and 2.027 s of CPU in the default build. That is over the brief's 1.5 s line, if the line covers the full count, and under its 2.5 s bound. The collect build at full count uses 1.484 s and 1.491 s.

**Commands and result lines**
- **Final suite:**
  - `cargo nextest run --locked -p swell --test main -E 'test(/^private_fields::/)'`: `Summary [ 0.499s] 14 tests run: 14 passed, 1678 skipped`
  - The same with `--features collect-every-safe-point`: `Summary [ 1.329s] 14 tests run: 14 passed, 1651 skipped`
- **Item 9 times** (`slot --timeout 30m sh -c "$(cat times.sh)"`, rc=0). CPU seconds, each test alone, two runs each:
  - **Default build:** the 100-field test 0.49 s, the step-budget loop 0.08 s, every other test 0.013–0.030 s.
  - **Collect build:**
    - compound assignments 1.075–1.095 s
    - step-budget loop 0.959–0.979 s
    - second add 0.56 s
    - read and written 0.45–0.47 s
    - `#x in` field/method/accessor 0.46 s
    - destructuring 0.43 s
    - the 100-field test 0.43 s
    - derived class 0.36 s
    - every other test 0.18–0.30 s
  - No test is over 1.5 s default or 2 s collect at the gate's counts.
- **Item 6:**
  - `slot --timeout 30m task test-stack-profiles`, rc=0: `release: 7 passed, 0 failed`, `stack-debug: 7 passed, 0 failed`, the release binary has 0 debug sections.
  - Step sweep over 1,125 budgets: every run either ended at the budget with a consistent prefix or finished with the full result.
  - 500 fields cost 122,303 steps. 5,000 fields cost 12,472,553 steps and stopped cleanly at the 1,000,000-step budget.
  - `#x in` nested 400 deep, and `new (class{...})()` nested 400 deep, both meet the parser nesting error without a crash.
- **Item 4 (memory):**
  - Main sweep over 1,580 budgets (64 finished, 1,516 ended, 20 skipped), plus a low window of 956 budgets (20 ended at the level's reservation). Every run that ended left a prefix of the fields, and live bytes were the same before and after.
  - Layout note: a Value stored as two u32 words (aligned to 4) would keep the element at 44 bytes with no unsafe code and no new dependency. 48 is pinned by a library test and by `ENTRY_BYTES` in the 100-field test.
- **Item 5:**
  - Plant A turns red in the collect build at `private_fields.rs:682` (`Terminated(EngineFault)` against `Normal("0")`) and stays green in the default build. That is expected: the default build does not collect at that point.
  - Plant B: see finding 5.
- **Item 7:** Plant 4 is red in the collect build:
  - `instances_with_100…` fails at line 562 (413886 against 423534).
  - Library test `a_level_of_methods_and_fields_reserves_its_room_once` fails ((true,0) against (true,264)).
  - Library test `the_room_is_reserved_once_per_level_and_charged_exactly` fails (0 against 168).
  - This fills the gap the unit left; plants 2 and 3 were already shown in both builds.
- **Item 8:** none of the four syntax tests lost a check it could have kept. `class_elements_scale_linearly` still measures its ratio on the three private shapes, but only the parse. No test measures compile time against the number of private fields; my probe compiled 500 fields in 12 ms and 5,000 in 126 ms.
- **Item 10:** 506 files beyond the unit's list. On the base, 788 runs passed and 201 failed; on the head, 965 passed and 24 failed. No run regressed and none is missing. The 24 failures on the head:
  - 14 module-code runs, waiting for slice 11.
  - 4 runs of `privatefield{get,put}-primitive-receiver.js` (BigInt literal), waiting for slice 12.
  - 6 counted refusals of the three `*-visible-to-direct-eval*.js` files, waiting for `story:private-names-in-eval-code`.
- **Item 11:** the four mutants are findings 1–4.

**Attacked and not broken**
- The order of PrivateFieldAdd with initializers, `this`, `new.target`, methods and accessors.
- An initializer that throws midway, and return override: Proxy, the global object, Array.prototype, a function, `null`, and the same object twice.
- Shadowing, and the same spelling in a base and a derived class.
- Every PrivateGet/Set form: compound and logical operators, update, destructuring, `for-of`, call `this`, tagged templates, `?.()`, `new o.#k`.
- `#x in`, valid in every operator context, and about 28 early-error sources, including escaped `#constructor`, plus `Function`, indirect eval and top-level eval.
- Extending built-ins, the heritage resolving to the outer class, direct eval inside initializers, and the collector probes (rings, orphaned keys, statics held through an instance).

**Not attacked:** module code (slice 11), BigInt (slice 12), and private names in eval code (refused for now).

Files are in <scratch>/wave2/privatefields/adversary-1/:
- Probes: `probe-matrix.rs`, `probe-matrix-2.rs`, `probe-budgets.rs`, `probe-memory-low.rs`, `probe-notes.rs`
- Scripts: `plants.sh`, `times.sh`, `job262.sh`, `compare262.sh`, `list262.sh`, `clause.sh`
- Logs: `plants.log`, `times.log`, `stack.log`, `compare262.txt`, `final-default.log`, `final-collect.log`, `probe-run-*.log`
- `saved/` and `private_fields.rs.orig`

`.out/adv262/` in the worktree is gitignored.

```findings
- file: crates/swell/src/object/private_names.rs
  line: 287
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Reserving room for one element instead of the level (room.min(1)) passes the 14 tests and the 284 library tests, so no test pins the once-per-level reservation.
- file: crates/swell/src/interp/private.rs
  line: 189
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Leaving the find of #x in uncharged passes every test in the tree, and only a step-difference probe (92 steps for #f99 against #f0) catches it.
- file: crates/compiler/src/class.rs
  line: 224
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Giving a static field the instance room is caught only by one disassembly fixture and by no behavioural test.
- file: crates/compiler/src/resolve.rs
  line: 1585
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Counting static fields into the instance room is caught only by one disassembly fixture, and a byte probe shows 216 against 72 bytes per instance.
- file: crates/swell/src/object/ecmascript_function.rs
  line: 118
  category: property
  severity: note
  verdict: INFEASIBLE
  origin: undecided
  message: Not tracing a field initializer function's environment stays green in both builds because that environment is the running frame's class scope, so the plant cannot be observed.
- file: crates/swell/src/object/private_names.rs
  line: 134
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: private_element_find charges its steps after the full scan rather than before the work as the design says, and n fields cost about n squared over 2 steps.
- file: crates/syntax/src/lib.rs
  line: 241
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Docs here and at crates/syntax/tests/sources.rs lines 264 and 1174 still name Parser::hold, which dcd4faa deleted.
- file: crates/swell/tests/private_fields.rs
  line: 562
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The 100-field test at SWELL_FULL_COUNTS=1 uses 1.96 to 2.03 CPU seconds in the default build, over the 1.5 s line and under the 2.5 s bound.
```

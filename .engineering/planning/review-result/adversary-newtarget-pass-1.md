---
format: aep.planning-md/3
id: review-result:adversary-newtarget-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit newtarget of wave big-2
relations:
- reviews: story:new-target-meta-property
revision: 1
---
Adversary, pass 1 on the unit newtarget of wave big-2 (impl/newtarget at 90960dae), 2026-11-26. The pass ran in two parts; this is the report of the second part, which reran and extended the first part's cases.
Verdict: no failing case. 5 findings (3 warnings as mutants the unit's tests miss, 1 note; 1 pre-existing warning on the stack-debug profile); 21 cases, 0 red.

unit: newtarget, branch `impl/newtarget` at `90960dae`, plus my uncommitted `crates/swell/tests/new_target.rs`
verdict: green
cases: executed 14→21, red 0
origin: introduced 4, pre-existing 1, undecided 0
wrote-outside-worktree: 1 directory, `<scratch>/wave2/newtarget/adversary-1b/` (20 log files and `impl.diff`)
needs-coordinator: yes: whether my test file is kept; the stack-debug stack failures are older than this unit

None of my cases fails against the unit's code. With three defects planted in a copy, all 14 of the unit's tests still pass and my cases fail. In the `stack-debug` profile, 4 stack tests overflow, but 5 already overflow on the base.

**1. Diff stat**
```
 crates/swell/tests/new_target.rs | 459 +++++++++++++++++++++++++++++++++++++++
 1 file changed, 459 insertions(+)
```
It is a test file only. No implementation file and nothing under `.engineering/` changed. The copies under `.out/` (`adv-base`, `adv-rev`, `adv-mut`) were made only on the build node, inside the pool jobs.

**2. Cases added** (`crates/swell/tests/new_target.rs`, after `// ---- adversary pass 1 (uncommitted) ----`)

All are green on `90960dae` in both modes. Times are from the final runs, in the default mode, then with `collect-every-safe-point`. None is over 5 s.
- `adversary_values_in_every_way_a_function_runs` (:352), 0.44 / 1.8 s: 44 cases. They cover parameter defaults, arrows, eval, `Reflect.construct` with a bound or builtin new target, `call`/`apply`, bound functions, getters and setters, callbacks, species, promise capability, generators across `yield`, re-entry, `Function`, `new new.target(1)`, `new.target()`, `?.` and tagged templates.
- `adversary_eval_contexts` (:400), 0.21 / 0.39 s: `eval?.()`, `eval` run by `map`, `eval.call`, eval in generators and methods, and nested eval at the top level.
- `adversary_where_it_parses` (:422), 0.05 / 1.9 s: 55 sources parsed through `(0, eval)` and `Function`.
- `adversary_async_and_host_call` (:499), 0.11 / 0.12 s: `new.target` across `await` in an async arrow and in an async generator, and the host's call and construct entries through a bound function.
- `adversary_the_box_alone_holds_new_target_and_function` (:554), 0.34 / 1.15 s: the arrow's box is the only thing that holds its NewTarget and its function.
- `adversary_arrow_boxes_are_charged_and_credited`, 0.95 / 1.9 s: 298 bytes charged per arrow, and the reading after a drop and a collection is the same in two rounds.
- `adversary_budgets_around_new`, 1.36 / 2.7 s: frame depth through `new`, the re-entry limit through `Reflect.construct` and through a bound function, the step budget and the memory budget. Each ends as expected, and the heap reading is the same in rounds 1 and 2.
- `adversary_measure_stack_per_reentry` (:720) is a measurement, not a case. It is `#[ignore]`, finds the smallest stack by bisection in child processes, and takes 8.4 s.

Red runs under the planted defects (`mutants.log`, in a copy under `.out/adv-mut`). M1 removes the two box traces (`ecmascript_function.rs:122,125`). M2 sets `frame.new_target = None` on resume (`suspend.rs:142`). M3 drops the restore in `function_end` (`parser/function.rs:195`). Result: `21 tests run: 18 passed, 3 failed`, and all 14 of the unit's tests pass. The `swell-syntax` binary passes 56 of 56.
```
adversary_the_box_alone_holds_new_target_and_function  left: Terminated(EngineFault)  right: Normal("true")
adversary_async_and_host_call  left: "F false,G true,H false,a true,then true"  right: "F true,G true,H true,a true,then true"
adversary_where_it_parses  "(function () {}); new.target script true function true ... var o = { m() {} }; new.target script true ..."
```

**3. Suite runs** (after my cases existed)

| Command | Summary | Exit |
|---|---|---|
| `-p swell --lib --test main` (default) | `1425 tests run: 1425 passed, 2 skipped` | 0 |
| `-p swell --features collect-every-safe-point --test main` | `1175 tests run: 1175 passed, 2 skipped` | 0 |
| same, collect, `--lib` with `test(/^new_target::/)` | `267 tests run: 267 passed` | 0 |
| `-p swell-syntax -p swell-compiler -p swell-test262 --test main` | `136 tests run: 136 passed` (56 / 16 / 64) | 0 |
| stack/depth/re_enters/nest tests, `--lib` and `--test main`, default | `109 tests run: 109 passed` | 0 |
| same, collect | `109 tests run: 109 passed` | 0 |
| same, `--cargo-profile stack-debug` | `110 tests run: 106 passed, 4 failed` | 100 |
| `slot task test-stack-profiles` | release `4 passed`, stack-debug `4 passed` | 0 |

An earlier collect run had 2 of my own probes killed at 60 s. I made their counts smaller for that mode and ran it again (the second row).

**4. Findings** (they cover `90960dae`)

1. **A unit test cannot catch a missing box trace.**
   - Measured: with M1 planted, `a_kept_new_target_survives_a_collection` (`new_target.rs:250` at HEAD) passes and my box case ends in `EngineFault`. In the unit's test, `F` stays reachable through the arrow's `this` (`this.__proto__.constructor`).
   - What reaches it: the trace at `object/ecmascript_function.rs:121-126` is guarded only by my case. Mutant, warning, CONFIRMED, introduced.
2. **No unit test reads `new.target` after an `await`.**
   - Measured: with M2 planted, all 14 unit tests pass and `adversary_async_and_host_call` fails. The unit's async arrow (`new_target.rs:171`) has no `await`.
   - What reaches it: any async arrow inside a constructor. Mutant, warning, CONFIRMED, introduced.
3. **Nothing tests the parser's restore of the flag.**
   - Measured: with M3 planted, all 14 unit tests and the 56 syntax tests pass, and 6 sources are admitted at the top level. One is `(function () {}); new.target`; the others are listed in part 2.
   - What reaches it: `function_end` (`parser/function.rs:194-196`). No fixture has a function before a top-level `new.target`. I read the 2 global-code cases of the story's 36; the test262 runner parses each case alone (`crates/test262/src/run.rs:534`). Mutant, warning, CONFIRMED, introduced.
4. **The dev-profile stack margin depends on one `#[inline(never)]`.** The table gives bytes of stack per re-entry in the gate's dev profile.

   | Path | base | HEAD | HEAD with `realm_of_function` inlined |
   |---|---|---|---|
   | `f.call` | 7,936 | 7,840 | 8,222 |
   | `[0].map(f)` | 9,470 | 9,376 | 9,760 |
   | `JSON.stringify` | 11,472 | 11,376 | 11,758 |
   | replacer | 12,496 | 12,400 | 12,783 |
   | `Reflect.construct` | 8,318 | 8,222 | 8,606 |
   | `new (f.bind())` | 7,392 | 7,296 | 7,680 |

   - On the replacer path at depth 1,000, against the 7/8 room of 12,838,560 bytes, the margin is 191,136 bytes on the base, 286,368 at HEAD and −96,608 with the function inlined.
   - So the unit's two changes bought about 96 bytes per re-entry over the base, and the attribute alone is worth about 383. `STACK_BYTES_PER_REENTRY` is 14,336.
   - No test names the attribute; the margin is a property of this build's inlining. Judgement, note, CONFIRMED, introduced.
5. **Pre-existing: 4 stack tests overflow in `stack-debug`.**
   - They abort with SIGABRT at 12,838,560 bytes: `regexp_symbol_methods::a_replacer_re_enters_on_the_promised_stack`, `builtins_string_replace::` (:438), `builtins_object_integrity::` (:757) and `tagged_templates::` (:532). The base fails 5 (the same 4, plus `regexp_objects::last_index_conversion_re_enters_on_the_promised_stack`).
   - The replacer path needs 13,665 bytes per re-entry here, 95 % of 14,336.
   - What reaches it: `test-stack-profiles` runs only 4 named tests, while `tests/support/host.rs:233-236` says the profile runs "the stack tests". `host.rs:223` still names `toJSON` as the largest need. Contract drift, warning, CONFIRMED, pre-existing.

**5. Attacked, nothing broken**
- Items 1 and 2: every value and parse position the brief lists, a class body excepted (not tried).
- Item 4: the box is charged, credited and traced; an arrow at the top level has a box with no function and no NewTarget.
- Item 6: `RECORD_BYTES` is read only by the two size-pin tests (`generator.rs:72`, `async_generator.rs:151`), which pass at 192.
- Item 7: every budget ends with the outcome the host reads, with no bytes left over between rounds. The unit's one test covers only the step budget.
- Item 8: I read all 403 lines added outside tests and found no `unwrap`, unchecked index or uncharged growth. `outer_new_target` is charged and released.
- Not checked: `size_of` of `EcmascriptFunction` (only the record's 192 is asserted, by the passing size tests); test262 against the mutants; a build of the merge with `impl/super`. Item 3 and the `host-surface-check` answer are in pass 1 and were not repeated.

**6. Paths written outside the worktree:** `<scratch>/wave2/newtarget/adversary-1b/` (logs `cases-1..3`, `stack-default`, `stack-collect`, `stack-debug`, `measure-dev`, `base-stack-debug`, `base-rev-dev`, `mutants`, `main-*`, `lib-nt-collect`, `nt-*`, `others`, `test-stack-profiles`, and `impl.diff`).

**7. Findings block**
```findings
- file: crates/swell/tests/new_target.rs
  line: 250
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: With the box's NewTarget and function traces removed, a_kept_new_target_survives_a_collection still passes, because F stays reachable through the arrow's this, while adversary_the_box_alone_holds_new_target_and_function ends in EngineFault.
- file: crates/swell/tests/new_target.rs
  line: 171
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: A resume that drops the frame's NewTarget leaves all 14 unit tests green because no unit test reads new.target after an await; adversary_async_and_host_call fails on it.
- file: crates/syntax/src/parser/function.rs
  line: 195
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Without the restore of new_target in function_end, the parser admits new.target at the top level after any function, and the unit's 14 tests and 56 syntax tests stay green.
- file: crates/swell/src/object/ecmascript_function.rs
  line: 245
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The replacer stack test's dev-profile margin of 286,368 bytes rests on the inline(never) of realm_of_function; inlining it costs 383 bytes per re-entry and leaves the test 96,608 bytes short, and no test names that dependency.
- file: crates/swell/tests/support/host.rs
  line: 236
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: The documentation says the stack-debug profile runs the stack tests, but test-stack-profiles runs 4 of them, and 4 others overflow there at seven eighths of min_stack_bytes on HEAD (5 on the base).
```

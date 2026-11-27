---
format: aep.planning-md/3
id: review-result:adversary-evalprivate-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the unit evalprivate
relations:
- reviews: story:private-names-in-eval-code
revision: 1
---
Adversary pass 2 on the unit evalprivate of wave big-2, impl/evalprivate at 5b6335d, 2026-11-27.
Green: no case fails and nothing a script can observe changes for an eval String without a hash; one warning (the three budget sweeps do not reach the walk's records) and five notes.

The unit has no defect: nothing a script can see changes when its eval String has no `#`, and nothing is red. But one claim in the correction's report is false: the three sweep tests never stop the run inside the walk's records.

```
unit: evalprivate (story:private-names-in-eval-code), adversary pass 2 on impl/evalprivate at 5b6335d
verdict: green
cases: executed 31→37 (eval_private_names:: and dynamic_code::tests::, default and collect builds), red 0
origin: introduced 5, pre-existing 1, undecided 0
wrote-outside-worktree: <scratch>/wave2/evalprivate/adversary-2/ (scripts, logs, saved and planted copies)
needs-coordinator: yes
```

You need to decide two things: whether my six uncommitted tests stay, and whether finding F1 (the sweep does not reach the walk's records) changes the sweep. The tree differs from `5b6335d` only in `crates/swell/tests/eval_private_names.rs` (+399 lines, formatted). Every plant was put back, and `git status --short` after each showed only that file.

## Findings

**F1. The sweep never stops the walk at one of its records** (`eval_private_names.rs:903`, `SWEEP = 171`). Warning, CONFIRMED, introduced (`5b6335d` committed the range pass 1 wrote).
- The three thirds sweep budgets 4910 to 5081. The walk's step per record falls at budgets 4797 to 4862, so no third stops it there.
- I planted a 64-byte leak on a stop at walk record 32 (budget 4829). All three thirds stay green.
- Leak planted on a stop in the class record's compares: first red at 4910, in the third named "walk".
- Leak planted on a stop at the parse's charge: red at 4956 and 4967, in the thirds named "walk" and "end of walk". The third named "parse" (5024 to 5081) holds no parse budget.
- So the report's sentence "It still catches budget ends across the walk, its end and the parse" does not hold. My two new sweeps cover 4796 to 4909: they catch the record-32 leak at 4829 and the compares leak at 4863, at 0.72 s each.

**F2 to F4. Three small changes of the scan survive the unit's tests.** Each is a note, CONFIRMED, introduced (`5b6335d`), and each turns only my `the_scan_finds_a_hash_in_every_form_of_a_string` red (113 tests ran: 112 passed, 1 failed). The code has none of these defects.
- F2, `dynamic_code.rs:225`: U+FF03 counted as `#` in a two-byte String. `"1 // \uFF03"` then costs 993 steps instead of −3.
- F3, `dynamic_code.rs:224`: a one-unit Latin1 `"#"` skipped (`bytes.len() > 1 &&`). Cost −4 instead of 992.
- F4, `dynamic_code.rs:225`: the first unit of a two-byte String skipped. `H + S` costs −4 instead of 992. This one would also turn a two-byte eval text that starts with a private name into a `SyntaxError`.

**F5. Wrong step for the record of strict eval code.** `compiler/src/lib.rs:149` and `:1631`, `resolve.rs:2474` and `eval_private_names.rs:573` cite "PerformEval step 16" for it. In the pinned draft step 16 is a NOTE; the record is made in step 17 and becomes the variableEnv in step 19. Note, introduced (`f6f4087`). It copies the engine's older numbering (`dynamic_code.rs:145`, `:552`, `resolve.rs:625`), so the fix is one pass over all of them.

**F6. Old clause at `private_methods.rs:1929`.** It still cites `#sec-performeval`, step 12, for privateIdentifiers. Blame says commit `1ec5e24`, not the unit's. In the pinned draft that step is the parse; privateIdentifiers is `#sec-evaldeclarationinstantiation` steps 4 to 6. Note, pre-existing.

## The brief's items

**1. The skip is sound.** I ran my test `eval_texts_without_a_hash_answer_as_when_they_walked` three ways: on HEAD, on the six source files of `82682c7`, and with the skip planted off. It passed all three, in both builds. It covers 37 checks without `#` in the outer text:
- answers, error kinds, `typeof`, `new.target`, `super`, escapes;
- a function, an arrow, a generator and a class made by eval code and called after it returned;
- `new Function`, a field initializer, a static block, a computed key and a computed field key;
- nesting three deep with `#` only at the innermost (`a + b + d` across three levels of records gives 7);
- a sloppy function with `with`.

The compiler reads the caller's list only for an unbound name that starts with `#` (`resolve.rs:2546`). The hidden `#key N` bindings of computed field keys are always bound in their class scope. The eval code's own record count (`resolve.rs:2471`) and a nested eval's walk depend only on the run-time chain.

**2. The scan.** The heap's String type has two variants, `Latin1` and `Utf16` (`heap/string.rs:147-152`), and the scan reads both. All 23 String forms answer the same direct and indirect and walk exactly when they hold `#`. Those include one unit, empty, `#` first and last, two-byte, concatenation, `slice` (also a two-byte String holding only Latin1 units), `fromCharCode(35)`, a template and `replace`. U+FF03 and `\u0023` do not walk and stay a `SyntaxError`, with and without the skip. Host-made Strings are scanned the same way.

A budget of 10 cannot reach the eval at all: one `eval(o)` of `"1"` spends 15 steps in its entry, so the test uses 15. A host String of 1,000,000 units without `#` then ends in `Terminated(StepBudget)`. Wall time was 16.8 to 18.9 ms for Latin1 and 9.2 to 11.4 ms for two-byte, which includes the host making the String in its hook. An interrupt set by HostEnsureCanCompileStrings, right before the scan, ends in `Terminated(Interrupted)` for `1`, `this.#x` and `'#'`, and the next entry answers. Both outcomes are the same with the skip planted off. The `82682c7` source run never reached this part: the earlier version of the test panicked at its step assertion first.

**3. The steps and the memory.** I measured with a probe library test, since removed, at three revisions.

| Script, text without `#` | HEAD direct | HEAD indirect | base 82688ae direct | 82682c7 direct |
|---|---|---|---|---|
| sloppy function, 1,000 vars | 4020 | 4023 | 4020 | 5015 |
| under 1,000 global lets | 26 | 29 | 26 | 27 |
| class of 100 names, 64 blocks | 348 | 350 | 348 | 507 |

- HEAD direct equals HEAD indirect plus the base's own direct-minus-indirect in all three scripts: 4023 − 3, 29 − 3 and 350 − 2.
- Without `#`, peak and kept bytes equal the base: peak 18563, 2563 and 3579, kept 16860, 860 and 1876 (82682c7 peaked at 25520 for the class).
- With `#`, HEAD equals `82682c7` to the step and the byte: 5017/18722/16961, 29/2722/961, 509/25525/1977, and `this.#p1` 444/26025/1918.
- The report's table, the walk-cost numbers (994, 301, 1293, 1851, 4677), sweep setup 133 and run 5081, and `f()` at 497 (507 on 82682c7's sources) all reproduce. So do the −3 and the 995 in the changelog text.

**4. The sweep.**
- The new thirds cover 172 budgets (4910 to 5081); pass 1's single test covered 171 (4911 to 5081). Nothing pass 1 covered was lost; the gap in F1 was already there.
- Six tests return at once in the collect build: the three thirds, `walk_cost_beyond_the_table`, `a_loop_of_500_evals_with_100_names_keeps_nothing`, and my two new sweeps.
- There is no safe point in the walk, the scan, the parse or the compile, so no collection can fall while the lists hold a position. The two tests that put a collection between the walk and the run do run in the collect build.
- At 100 vars instead of 200, the hash test still catches the skip forced on, the skip forced off and the per-slot charge removed. A raised `SMALL` up to 11 survives at any count; a cap on the compare charge between about 95 and 995 survives only in the collect build.

**5. The test-only hook.** All five references to `WALKED_PRIVATE_NAMES` are in `#[cfg(test)]` items (`dynamic_code.rs:352-366`, `:1109-1115`, and the tests module), and `extern crate std` is under `cfg(test)` only. `slot task nostd-build` passed: 6 libraries for x86_64-unknown-none. The library test fails for the right reason only:
- Plant P2 alone: `#x listed 2 times: [("#x", 1), ("#x", 4), ("#y", 4)]`, 1 of 35 red.
- Plant listing every outer name again one hop further: `#x listed 2 times: [("#x", 1), ("#x", 2), ("#y", 4), ("#y", 5)]`, 1 of 35 red.

**6. The clause.** The pinned draft has var-conflict checks in step 3, privateIdentifiers in steps 4 to 6 and AllPrivateIdentifiersValid in step 7. All twelve sites in the report cite steps 4 to 7 (or 4 to 6) of `#sec-evaldeclarationinstantiation` correctly. `private_methods.rs:1929` still cites PerformEval step 12 (F6).

**7.** F2 to F4 above.

**8. Times.** CPU seconds, each test alone, three runs. Node load was 1.6 to 2.8 on 32 CPUs.
- Default build: every test is at most 0.770 s (the "parse" third). Mine: scan 0.071, host 0.038, interrupt 0.055, the no-`#` test 0.020, the two new sweeps 0.720 and 0.726.
- Collect build: the slowest are `eval_code_shapes` 1.570 and the hash test 1.150; library tests 0.045 and 0.202. Mine: host 0.935, the no-`#` test 0.593, interrupt 0.147.
- My scan test first read 5.24 s in the collect build. I cut it to 24 vars and ten of the forms there, and it now reads 0.675.
- No reading is over its bound.

**9. test262.** The 125 cases are byte-identical to `.out/adv-t262/cases.txt`. `test262 total 125 0 0 125 125 0 0`, 225 of 225 runs `Passed`. `check.sh` printed `passed 125 bound 125 B 113 F 12 R 0`, exit 0.

## Cases added (uncommitted, `crates/swell/tests/eval_private_names.rs`, all green)
- `eval_texts_without_a_hash_answer_as_when_they_walked`
- `the_scan_finds_a_hash_in_every_form_of_a_string`
- `a_string_from_the_host_is_scanned_as_one_from_a_script` (with the helpers `HashHost`, `hash_agent`, `hash_entry`)
- `an_interrupt_or_the_step_budget_ends_the_eval_around_the_scan`
- `step_budget_across_the_inner_records_of_the_walk_keeps_nothing` and `step_budget_across_the_outer_records_of_the_walk_keeps_nothing` (helper `step_budget_sweep_below`)

## Commands and result lines
- `run.sh final.log dc '<eval_private_names | dynamic_code::tests>'`: `37 tests run: 37 passed` (default), `37 tests run: 37 passed` (collect).
- `revs.sh` (sources of `82682c7`, then `82688ae`): prior default `35 tests run: 30 passed, 5 failed`, prior collect `32 tests run: 28 passed, 4 failed`. All of those reds are step assertions or `f()` at 507. Base probe exit 0.
- `plants.sh` first batch:
  - skip off: only the scan test red;
  - record-32 leak: `3 passed`;
  - compares leak: red at 4910;
  - parse leak: red at 4956 and 4967;
  - P2 and the duplicate-listing plant: 1 red each;
  - F2, F3, F4: `113 tests run: 112 passed, 1 failed` each.
- `plants.sh` second batch: record-32 leak red at 4829, compares leak red at 4863 and 4910.
- `slot task nostd-build`: `nostd-build: 6 libraries for x86_64-unknown-none: ...`, exit 0.
- `slot --timeout 30m --fetch .out/adv2-t262 sh -c t262.sh`: `test262 total 125 0 0 125 125 0 0`, exit 0.
- `cargo fmt --all -- --check`: exit 0 after formatting my additions.

## Not attacked
- The memory-budget sweep at 100 names and 64 blocks.
- Two realms.
- The 100,000-eval loop.
- clippy on my additions.
- The compiler crate's tests (I did not touch them).

```findings
- file: crates/swell/tests/eval_private_names.rs
  line: 903
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the three sweep thirds start at budget 4910 while the walk's step per record falls at budgets 4797 to 4862, so a 64-byte leak planted on a stop at walk record 32 passes all three and the thirds named walk and parse hold neither
- file: crates/swell/src/dynamic_code.rs
  line: 225
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: counting U+FF03 as a hash in a two-byte String survives the unit's tests (only the steps change) and is caught by the_scan_finds_a_hash_in_every_form_of_a_string
- file: crates/swell/src/dynamic_code.rs
  line: 224
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: skipping a Latin1 String of one code unit such as "#" survives the unit's tests (only the steps change) and is caught by the_scan_finds_a_hash_in_every_form_of_a_string
- file: crates/swell/src/dynamic_code.rs
  line: 225
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: skipping the first code unit of a two-byte String survives the unit's tests though it would make a two-byte eval text that starts with a private name a SyntaxError, and is caught by the_scan_finds_a_hash_in_every_form_of_a_string
- file: crates/compiler/src/lib.rs
  line: 149
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the record of strict eval code is cited as PerformEval step 16 here, at lib.rs:1631, resolve.rs:2474 and eval_private_names.rs:573, while the pinned draft makes it in step 17 (step 16 is a NOTE), following the older numbering of dynamic_code.rs:145 and :552
- file: crates/swell/tests/private_methods.rs
  line: 1929
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the doc still cites PerformEval step 12 for privateIdentifiers, which the pinned draft places in EvalDeclarationInstantiation steps 4 to 6
```

---
format: aep.planning-md/3
id: review-result:adversary-evalprivate-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit evalprivate
relations:
- reviews: story:private-names-in-eval-code
revision: 1
---
Adversary pass 1 on the unit evalprivate of wave big-2, impl/evalprivate at 82682c7, 2026-11-27.
Red: one case fails (a direct eval with no class around it spends a step per named slot of its caller's records) and three changes of the code pass the unit's tests, each caught by a case of the adversary. The adversary's own report follows.

unit: evalprivate (`story:private-names-in-eval-code`), worktree `swell-impl-evalprivate` at `82682c7` plus 2 uncommitted test files
verdict: CONFIRMED (1 case red now; 3 mutants the unit's suite misses, each caught by a case of mine that is green now)
cases: executed 16→25 (`eval_private_names::`, default and collect builds), swell-compiler `--test main` 17→18, red 1
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, `<scratch>/wave2/evalprivate/adversary-1` (full list in part 6)
needs-coordinator: F1 needs a decision: may a direct `eval` with no class around it spend one step per named slot of its caller's records? Default proposed: no, skip the walk when the source has no `#`.

**Conflict to state.** The `aep:adversary` procedure says never to mutate a file under attack. The brief orders plants and mutants in the worktree instead (a copy does not reach the build node). I followed the brief. `plants.sh` saved each file, planted it, ran the tests, and put it back. `git status --short` after each of the 10 plants showed only my two test files.

## 1. `git --no-pager diff --stat`
```
 crates/compiler/tests/compile.rs         |  56 ++++
 crates/swell/tests/eval_private_names.rs | 487 +++++++++++++++++++++++++++++++
 2 files changed, 543 insertions(+)
```
Both paths are test files. No implementation file differs.

## 2. Cases added

In `crates/swell/tests/eval_private_names.rs` (a helper `adversary_cases` at :548, a shared body `adversary_step_budget_sweep` at :880):

- **`adversary_eval_code_shapes_count_their_records`** (:572), green. 56 eval texts in a method:
  - top-level `let`, `const`, `var`, function and class declarations, captured or not;
  - `for (let …)`, `switch`, `catch` with and without a binding, a generator after `yield`, parameter expressions;
  - computed keys, heritage, static blocks, field initializers, inner classes that shadow and that don't;
  - nested evals, `(eval)(…)`, `eval(...[…])`, and every write form.
- **`adversary_caller_shapes_count_their_records`** (:652), green. 18 places of the call:
  - blocks, `for (let …)`, catch, switch, parameter expressions, a generator after `yield`;
  - inner classes, inner computed keys, a class expression's name binding, a static block and a field initializer with their own records;
  - a closure called after the method returned;
  - a direct eval in the heritage that must see the outer class's name of the same text.
- **`adversary_a_direct_eval_in_the_heritage_does_not_see_the_class_names`** (:690), green.
- **`adversary_early_errors_of_direct_eval_code_stay_errors`** (:703), green. 18 texts, each a `SyntaxError` before any side effect (the one exception, an indirect eval inside eval code, is meant to run first):
  - `delete this.#x`, a duplicate `#x`, `#constructor`, a sibling class's name, `super.#x`, a getter clashing with a field;
  - the alias, `eval?.()`, `globalThis.eval`, `Function`, and a direct eval inside `Function` code.
- **`adversary_eval_after_an_await_reads_the_name`** (:749), green.
- **`adversary_walk_cost_beyond_the_table`** (:781), green. It asserts the compares of the walk are charged at 64 and 100 nested classes (catches mutant M-b).
- **`adversary_step_budget_across_the_walk_keeps_nothing`** and **`…_across_the_parse_keeps_nothing`** (:982, :988), green. They lower the step budget one step at a time over the last 170 steps of one eval (100 names, 64 blocks), then run a loop of 500 evals. Default build only.
- **`adversary_a_direct_eval_outside_every_class_pays_no_step_per_slot`** (:1000), **red**. Its first run alone (`run5.log`, default and collect builds; the assertion is now at :1027):
  ```
  thread 'eval_private_names::adversary_a_direct_eval_outside_every_class_pays_no_step_per_slot' panicked at crates/swell/tests/eval_private_names.rs:965:5:
  a direct eval in a function of 1,000 vars and no class spends 992 steps more than an indirect one
  ```

In `crates/compiler/tests/compile.rs:1026`:
- **`adversary_eval_private_positions_keep_the_first_of_a_text`**, green. It checks the public `compile_eval_with`:
  - with two positions of one text, the first is kept;
  - a missing text, hops at `u32::MAX`, an empty text, `x` and `#` answer an error or are passed over, never a panic.

## 3. Suite run, after all cases exist
- `run-adv.sh … 'test(/^eval_private_names::/)'`:
  - default: `25 tests run: 24 passed, 1 failed, 1708 skipped`, `default exit 100`
  - collect: `25 tests run: 24 passed, 1 failed, 1680 skipped`, `collect exit 100`
  - the 1 red in both is `adversary_a_direct_eval_outside_every_class_pays_no_step_per_slot`
- `cargo nextest run --locked -p swell-compiler --test main`: `18 tests run: 18 passed`, `exit 0`.

**Plants** (`plants.sh`; d = default build, c = collect build, l/k = the lib and compiler runs; each count leaves out F1's red case, which is red in every run):

| Plant | Red tests |
|---|---|
| P1: the walk stops at the first record that lists a name | d and c: `an_inner_class_eval_reads_an_outer_classes_name` (`left: (Throw("SyntaxError"), …)`), plus my shapes cases; d also `adversary_walk_cost_beyond_the_table` |
| P2: the walk's shadow check removed (`env/mod.rs:610`) | **none**, in both builds (F5) |
| P2b: P2 plus `resolve.rs:560` always inserts | d and c: `an_inner_name_hides_an_outer_name_of_the_same_text` (TypeError), `an_arrow_in_an_inner_computed_key_sees_the_inner_names` (`"true,object\|true\|inner"`); k: my compiler case |
| P3: `extra` always 0 | 25 red in d, 23 in c; compiler test `eval_private_names_count_the_callers_hops` red |
| P4: `private_names` not passed to the early check | 27 red in d, 25 in c; both compiler tests red |
| Unit's U1: no step per record | d: `one direct eval spends -2 steps over an indirect one at 1 block, -2 at 64`; l: `step_totals_are_unchanged` measures `f()` 497 against the expected 507 |
| Unit's U2: lists not credited | d: `the walk's list leaked left: 619701 right: 766581`; `private_methods::…escapes` `5120 0`; my sweeps `4996: a stopped eval kept bytes` |
| M-a: `resolve.rs:2480` `env_slots > 0` → `true` | only `adversary_eval_code_shapes_count_their_records`, in d and c: `left: Terminated(EngineFault)`. Compiler tests green. |
| M-b: `env/mod.rs:606` compares not charged | only `adversary_walk_cost_beyond_the_table` (d): `64 nested classes: the compares of the walk are not charged (255)` |
| M-j: `resolve.rs:560` always inserts | only my compiler case: `left: Ok([(6, 3)]) right: Ok([(3, 0)])` |

U1 shows the `f()` total of 497 → 507 is ten walks of one step each, which answers item 7.

**test262** (one job, `t262-job.sh`):
- The cases: 85 files whose text holds `eval(` and `#ident`, merged with the story's 107, give 125 cases.
- `lines.txt`: `test262 total 125 0 0 125 125 0 0`.
- `runs.tsv`: 225 runs, all `Passed`. On the base, 24 of those runs failed (12 cases).
- `sh docs/plan/slice-08/check.sh .out/adv-t262/cases.txt .out/test262-base/runs.tsv .out/adv-t262/runs.tsv` printed `passed 125 bound 125 B 113 F 12 R 0`, `check exit 0`. No run that passed on the base fails now.

**Times** (`times.log`, each test alone with `--exact --test-threads 1`, children user CPU):
- Default build:
  - The unit's: `nested_eval_with_private_names_is_bounded` 0.516 s; the rest 0.007–0.228 s.
  - Mine: the sweep's parse half 1.469 s and walk half 1.081 s, walk cost 0.220 s.
  - The new lib test 0.160 s; the compiler tests 0.002 s each.
- Collect build:
  - The unit's: `nested_eval_with_private_names_is_bounded` 0.937 s (the unit read 0.924 s), the eval loop 0.791 s.
  - Mine: the shapes case 1.575 s, early errors 0.779 s.
- No test is over 1.5 s (default) or 2 s (collect).

**Steps** (my run of the table): three cells match the unit's (0 names/1 block 1, 8/8 9, 100/64 157).
- 1,000 names at 1 block: 994 steps; 1 name at 300 blocks: 301; 1,000 names at 300 blocks: 1,293. 400 blocks is past `parser_nesting`.
- 64 and 100 nested classes: 1,851 and 4,677 steps (about the square of the depth, under names × depth); wall time 2.2 ms and 3.4 ms.

## 4. Findings (tree `82682c7` plus my test files)

**F1 — `crates/swell/src/env/mod.rs:575`. Warning, CONFIRMED, introduced (`f6f4087`).**
- Measured: every direct eval walks and reads every slot of every named record, even when no class surrounds it and the source has no `#`. A direct `eval("1")` in a non-strict function with 1,000 `var`s costs 992 steps more than an indirect one; under 1,000 global `let`s, −2.
- On base `82688ae` the walk ran only after a failed parse (`dynamic_code.rs:408` there), so this cost is new.
- What reaches it: any non-strict function that calls `eval` (all of its slots are named), in the default build, under the host's step budget.
- Fix to name: skip the walk when the String has no `#` code unit (that scan is already charged per code unit), or scan only class private records.

**F2 — `crates/compiler/src/resolve.rs:2480`. Warning, CONFIRMED, introduced.**
- Measured: mutant M-a survives the 16 tests, the changed tests and the compiler test. A grep of the 107 cases shows no eval text with a top-level lexical binding.
- Under it, `eval("let a = 10; this.#x + a")` in a method ends the agent in `Terminated(EngineFault)`.
- What reaches it: eval code whose top-level `let` or `const` lives in a local slot. `adversary_eval_code_shapes_count_their_records` now catches it.

**F3 — `crates/swell/src/env/mod.rs:606`. Note, CONFIRMED, introduced.**
- The step charge for comparing a text with inner names (the shadow check) has no test: mutant M-b survives the unit's suite.
- 64 nested classes charge 255 steps under the mutant against 1,851 without it. Caught by `adversary_walk_cost_beyond_the_table`.

**F4 — `crates/compiler/src/resolve.rs:560`. Note, CONFIRMED, introduced.**
- The documented contract of the public `compile_eval_with`, "the first of a text kept", had no test: mutant M-j survives everything of the unit.
- The engine never passes a duplicate, so only an outside caller of the crate reaches it. Caught by my compiler case.

**F5 — `crates/swell/src/env/mod.rs:605`. Note, CONFIRMED, introduced.**
- Removing the walk's shadow check (P2) stays green in both builds, because `resolve.rs:560` and the parse's dedupe give the same result. Only removing both (P2b) turns the unit's shadow tests red.
- The walk's own rule has no test of its own. The fix is a library test of `eval_private_names` on two records with one text, or dropping one of the guards.

**F6 — `crates/swell/src/dynamic_code.rs:315`. Note, CONFIRMED, introduced.**
- The code, `syntax/src/lib.rs:200` and :238, and the compiler documentation cite privateIdentifiers and AllPrivateIdentifiersValid as `#sec-performeval` step 12.
- The pinned draft `74c33d406ec1…` has those steps in EvalDeclarationInstantiation (19.2.1.4), after its var-conflict checks. Text read: `.cache/ecma262/74c33d40…/ecma262.txt` lines 30971–31004, taken from the `swell-impl-bench` worktree because this worktree has no `.cache/ecma262`.
- I found no observable difference: both are a SyntaxError before any code runs, and only non-strict eval code has the var checks, which cannot see a private name.

## 5. Attacked and not broken
- Hop counts in every eval-code and caller shape of item 1, in both builds, including closures called after the method returned and after collections.
- Shadowing: the same text in nested classes, in a base and a derived class, an inner computed key, heritage (inner and outer), names outside ASCII and through escapes (the unit's tests).
- Early errors and indirect forms (18 cases); `(eval)(…)` and spread both run as direct evals.
- Step budget lowered one step at a time over 170 budgets: 340 runs ended at the budget, 2 answered `99`; at each budget a second run left `heap_bytes()` where the first left it. 500 evals with 100 names kept no bytes.
- Collector: neither `charge_steps` nor the walk reaches `safe_point()`. The lists hold no handle and the chain is rooted by the caller's frame, so there is no keep to plant.
- No eval-code cache that could reuse hops across callers.
- The changed tests hold what the clause orders. The removed `message_kind` assertion tested a refusal that no longer exists. The `"none"` expectation in the escapes sweep checks no value, but each class there has one name, so a wrong hop throws.
- Not attacked:
  - the 8-byte memory-budget sweep at 100 names and 64 blocks (only the unit's `#ä` sweep, which U2 turns red);
  - the realm of the error across two realms;
  - the full 100,000-eval loop (I ran 500 with 100 names; the unit runs 10,000 with 1).

## 6. Paths written outside the worktree
All inside `<scratch>/wave2/evalprivate/adversary-1/`:
- Scripts: `run-adv.sh`, `run-cost.sh`, `plants.sh`, `reds.sh`, `t262-job.sh`, `times-job.sh`.
- Logs: `run1.log`…`run7.log`, `suite1.log`, `suite1-compiler.log`, `suite2.log`, `suite2-compiler.log`, `plants.log`, `plant-{p1,p2,p2b,p3,p4,u1,u2,ma,mb,mj}-{d,c,k,l}.log` (only the runs each plant had), `t262.log`, `times.log`, `fmt.diff`.
- Copies: `orig/` (saved files), `plants/` (10 planted copies), `eval_private_names.rs.orig`, `compiler-case.rs.txt`, `sweep-case.rs.txt`.
- Lists: `all.sorted`, `story.sorted`, `base-paths.txt`, `t262-eval-private.all.txt`, `t262-extra.txt`.
- Inside the worktree (gitignored): `.out/adv-t262/` with `runs.tsv`, `lines.txt`, `cases.txt`, made by the test262 job.

## 7. Findings block
```findings
- file: crates/swell/src/env/mod.rs
  line: 575
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: every direct eval now reads every named slot of its caller's records even with no class around it, so a direct eval in a non-strict function of 1,000 vars costs 992 steps more than before
- file: crates/compiler/src/resolve.rs
  line: 2480
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: replacing env_slots > 0 with true survives the unit's tests and faults the agent on eval code whose top-level let lives in a local slot, caught only by adversary_eval_code_shapes_count_their_records
- file: crates/swell/src/env/mod.rs
  line: 606
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: removing the step charge of the shadow compares survives the unit's tests (255 instead of 1,851 steps at 64 nested classes), caught only by adversary_walk_cost_beyond_the_table
- file: crates/compiler/src/resolve.rs
  line: 560
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the documented rule that compile_eval_with keeps the first position of a text had no test and its mutant survived, now held by adversary_eval_private_positions_keep_the_first_of_a_text
- file: crates/swell/src/env/mod.rs
  line: 605
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: removing the walk's shadow check stays green in both builds because the resolver and the parse dedupe the same texts, so the walk's own rule has no test
- file: crates/swell/src/dynamic_code.rs
  line: 315
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the code and documents cite privateIdentifiers as PerformEval step 12 while the pinned draft places it in EvalDeclarationInstantiation after the var-conflict checks, with no observable difference found
```

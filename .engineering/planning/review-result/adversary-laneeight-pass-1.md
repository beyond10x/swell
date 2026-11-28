---
format: aep.planning-md/3
id: review-result:adversary-laneeight-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on step 1 of the unit laneeight: needs-change, 8 findings, 4 failing cases'
relations:
- reviews: story:slow-lane-holds-its-bound-on-the-slower-runner
revision: 1
---
Adversary pass 1 on step 1 of the unit laneeight of wave big-2, on impl/laneeight at 3dba5760 (the unit's commits a510cf4b..117a3361), 2026-11-28.
The adversary's own report follows as it was given.

```
unit: laneeight (story slow-lane-holds-its-bound-on-the-slower-runner, step 1), worktree at 3dba5760 (unit commits a510cf4b..117a3361) plus my 4 uncommitted cases
verdict: NEEDS-CHANGE
cases: executed 286→290, red 4
origin: introduced 7 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory (<scratch>/wave2/laneeight/adversary-1)
needs-coordinator: a reading of the story is needed for F2 and F3. Its refusal list names only "a type other than TestHost and ProvidedHost" and "the value another such impl has". Its purpose sentences say "three types" and "no other type has".
```

**1. Diff of the worktree:**
```
 crates/swell/tests/emulated_host.rs |  8 +++++++
 crates/swell/tests/host_types.rs    | 47 +++++++++++++++++++++++++++++++++++++
 2 files changed, 55 insertions(+)
```
Both are test files. `git status --short` shows only these two. No new module, so `task test-main` is not needed. Every plant was put back from its saved copy.

**2. Cases** (written first, then run alone in job 1 before anything else ran). All 4 are red now:
- `host_types::an_impl_a_macro_writes_for_the_trait_given_as_its_argument_is_refused` (host_types.rs:602): `panicked at crates/swell/tests/host_types.rs:605:5: the impl of Host the macro writes for Mine passes the guard: []`
- `host_types::a_second_type_named_test_host_marked_provided_defaults_is_refused` (:616): `panicked at …host_types.rs:624:5: a fourth type of Host passes the guard: []`
- `host_types::an_own_constant_with_the_value_of_the_default_is_refused` (:635): `panicked at …host_types.rs:638:5: an own-constant type with the default value passes the guard: []`
- `emulated_host::the_type_that_implements_host_for_this_host_names_no_system_api` (emulated_host.rs:191): `panicked at crates/swell/tests/emulated_host.rs:193:5: ["line 10: std::any"]`
- Cases-alone run: `Summary 4 tests run: 0 passed, 4 failed, 1784 skipped`, exit 100. `rustfmt --edition 2024 --check` on both files: exit 0. Each case takes under 0.01 s.

**3. Suite runs** (all after the cases existed):
- **Job 1, the 17 folded modules plus `host_types` and `hooks`:** `Summary 290 tests run: 286 passed, 4 failed, 1498 skipped`, exit 100, wall 4 s. Only my 4 failed.
- **Job 3, collect mode** (`COLLECT_MAIN` from Taskfile.yml, `RUSTFLAGS=-D warnings`, the 17 modules plus `host_types`): `Finished collect profile in 55.72s`, `273 tests run: 269 passed, 4 failed`, exit 100. Only my 4 failed. Slowest test: `determinism::host_133_same_calls_same_hooks_same_outcomes`, 0.306 s.
- **Job 2, the planted tree:** inert plants switched on by environment variables; a separate build, before job 3. Baseline: guard 16 run / 3 failed (mine); modules 257 run / 1 failed (mine).
  - **Guard mutants (G1–G17):** each rule taken out on its own. Not caught by any text case: G1 `starts_an_item` always true, G2 `body_start` with no bracket underflow, G3 lifetime counted as a type parameter, G4 the `const` filter dropped, G14 the identifier check after `as` dropped, G15 the `where` cut dropped. The other 11 each fail their own text case.
  - **Dispatch mutants (`D:<hook>`):** the `Scripted` dispatch takes the copy of the default, or a no-op or another clock for a required hook. Every hook the engine calls is caught by at least one module, except the 4 time-zone hooks (equivalent for the current hosts).
  - **Survivors that are unreachable:** `D:` for generic, timeout and finalization jobs, module load, import.meta, resize and grow. The engine calls none of these hooks (grep for `H::<hook>` in crates/swell/src).
  - **Copy mutants (`P:<hook>`):** `P:host_call_job_callback` (answers `Ok(Undefined)` without calling) is caught by 1 `promise_reads` test. `P:host_enqueue_promise_job` (35 failures) and `P:host_promise_rejection_tracker` (6) are reached. The other copies are not exercised by any folded module; the story accepts that.

**4. Findings** (tree 3dba5760 plus the cases):
- **F1** (host_types.rs:287–319): the guard only refuses `Host` named inside a `macro_rules!` body. A macro that takes the trait as `$t:path` and is called with `swell::Host` writes an unmarked fourth type, and the guard returns `[]`. Nothing in the tree does this now; a test author writing a helper macro would. Fix: refuse an `impl $…` inside a macro body, or `Host` as a macro argument. Origin: introduced.
- **F2** (host_types.rs:357–365): `provided-defaults` is checked by the last path segment only, with no count. A second `TestHost` in another module passes, which makes four types. Origin: introduced.
- **F3** (host_types.rs:366–385): `own-constant` is compared only against other `own-constant` impls. One with `&[]`, the value the three existing types already have, passes as a fourth type for no new value. Origin: introduced.
- **F4** (support/scripted.rs:10): emulated_host's doc says "an adapter is whatever implements `Host` … calls no system API". The type that implements `Host` is now `Scripted`, which names `std::any`. The brief asked for `core::any::Any`. `the_module_names_no_system_api` reads only emulated_host.rs. Fix: `use core::any::{Any, type_name};`. Origin: introduced.
- **F5** (host_types.rs:65, 88, 188, 257, 262, 278): six guard rules have no text case that fails without them (G1–G4, G14, G15 above). In particular, `fn g(host: impl Host)` in the bound case passes both `starts_an_item` and `body_start`, so neither is pinned alone. Origin: introduced.
- **F6** (source_scan.rs:212–226 as used at host_types.rs:249–254): `code_of` removes newlines around `::`, so `line_of(code)` drifts from the source line. After a line that ends in `::`, a marker two lines above an `impl` is accepted and messages name the wrong line. No such line exists in the repository and rustfmt does not write one. Origin: introduced.
- **F7** (math_random.rs:476): `assert_eq!(random_hooks, 0)` only checks absence. A dispatch that never reaches the counting hooks keeps it green, and D-mutants confirm it. Same assertion at the base (line 426). Origin: pre-existing.
- **F8** (scripted.rs:142–163, 541): no test pins the wrong-type panic message or the `required::` panics. Changing `state_type` or turning `unanswered` into a no-op stays green. Case text that would catch the first: `#[should_panic(expected = "the scripted host holds a u8, not a u16")] fn …() { Scripted::new(1u8, Hooks::PROVIDED).state::<u16>(); }`. The code has neither defect. Origin: introduced.

**5. Attacked, could not break:**
- All 17 folds: the hook sets are identical to the base impls, and a token diff with the accessor and rename rewrites removed leaves only formatting. No record moved before or after its call. No state is shared between agents or tests. State drops with the agent as before.
- The 18 copies equal the trait defaults (hooks.rs:180–380) except the documented `Err` arm. The accessor panics with both type names and has no silent default.
- Test lists: the base and head lists sorted with `LC_ALL=C`; `comm` differs only by the 13 `host_types::` names.
- The unit's build readings, checked in its logs and timing pages:
  - Collect build: unit `main` 54.9 s, all 6 workspace crates compiled under `--cfg probe_101`, load 4.81 at the start.
  - Build with `TEST_OPT1_ARGS`: unit `main` 72.2 s, recompiled under `probe_102`, load 10.83 at the start.
  - I did not repeat either reading.
- Diff checks:
  - The unit's commits change no file under `crates/*/src/`, `conformance/`, `docs/`, `Taskfile.yml` or `taskfiles/`, and leave `hooks.rs` unchanged.
  - `support/host.rs` gains exactly 2 lines (the two markers).
  - `sh taskfiles/test-main.sh check`: `test-main: 9 crate(s) with tests, each with one integration-test binary and its tests/main.rs as generated`.
- Guard forms it handles:
  - Refused as it should: nested renames and renames in function bodies, deeper directories, a marker with text after the reason, an attribute or blank line between marker and `impl`, `impl<T> Host for W<T>`, const parameters.
  - Passes as it should: lifetimes, `HostX` / `MyHost` / `HostData`, doc comments, strings.
- Not attacked: `include!` or `#[path]` of a non-`.rs` file (no such file in `tests/`; the guard reads only `*.rs`). Clippy was not run.

**6. Paths written outside the worktree:** <scratch>/wave2/laneeight/adversary-1/, holding:
- scripts: `hooksets.sh`, `residue.sh`, `plant-guard.awk`, `plant-scripted.awk`, `job1.sh`, `job2.sh`, `job3.sh`
- job output: `job1.out`, `job2.out`, `job3.out`
- comparisons: `residue-all.txt`, `a.txt`, `b.txt`
- saved and planted copies: `orig/{host_types,emulated_host,scripted}.rs`, `host_types.cases.rs`, `emulated_host.cases.rs`, `host_types.planted.rs`, `scripted.planted.rs`

```findings
- file: crates/swell/tests/host_types.rs
  line: 287
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: an impl of Host written by a macro_rules! that takes the trait as a path argument carries no marker and passes the guard (case an_impl_a_macro_writes_for_the_trait_given_as_its_argument_is_refused)
- file: crates/swell/tests/host_types.rs
  line: 357
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: provided-defaults is checked by the last path segment with no count, so a second type named TestHost in another module passes as a fourth host type
- file: crates/swell/tests/host_types.rs
  line: 366
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: an own-constant type whose constant is the default empty list, the value the three existing types already have, passes the guard
- file: crates/swell/tests/support/scripted.rs
  line: 10
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the emulated host's Host impl now lives in scripted.rs, which names std::any where the brief asked for core::any, and the module's no-system-API check does not read it
- file: crates/swell/tests/host_types.rs
  line: 65
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: six guard rules (starts_an_item, the bracket underflow of body_start, the lifetime exemption, the const filter, the identifier check after as, the where cut) survive their removal with every text case green
- file: crates/swell/tests/host_types.rs
  line: 249
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: line numbers come from code_of, which drops newlines around ::, so after such a line the marker is read from the wrong source line; no rustfmt-formatted source has one
- file: crates/swell/tests/math_random.rs
  line: 476
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the zero-hook assertion only checks absence and stays green when the scripted dispatch never reaches the counting hooks
- file: crates/swell/tests/support/scripted.rs
  line: 142
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: no test pins the wrong-type panic message of the accessor or the panics of the required entries in Hooks::PROVIDED, so a change to either stays green
```

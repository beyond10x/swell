---
format: aep.planning-md/3
id: review-result:adversary-derived-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit derived: green, 24 cases and five notes'
relations:
- reviews: story:derived-classes-and-super-call
revision: 1
---
Adversary pass 1 on `impl/derived` at `36b82da` (wave big-2, slice 8, order 3), run on 2026-11-27.
The report below is the adversary's last message; paths of the workstation are replaced.

unit: derived
verdict: green
cases: executed 19→24, red 0
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/derived/adversary-1/ (probes, logs, scripts, and the `git archive 36b82da` copy `plant/`, whose `target/` I deleted)
needs-coordinator: no

None of my attacks broke the unit's engine code. About 180 probe scripts across items 1 to 10 gave the answer the clause gives, in both builds. The five findings are all notes. Each is either a test that no longer checks what its name says, a gap the tests leave open, or a measured cost.

**Findings**

1. `crates/syntax/tests/sources.rs:793`, `a_script_is_not_parsed_as_eval_code`. After the rewrite, every place the test puts a script next to eval code expects the same error from both, with every eval flag off. So a `parse_script` that just called `parse_eval` with no flags would still pass, and the test no longer checks what its name says. Reached by the syntax suite. Introduced.
2. Mutant Ma, `crates/swell/src/interp/class.rs:389`: SuperCall reads the parent at the call (after the arguments) instead of using the value GetSuperConstructor read first. The 19 tests: `19 tests run: 19 passed`, exit 0. In the listed cases, only `call-proto-not-ctor.js` changes the prototype, and it does so before `new`. The engine is correct. My added case `adversary_values_of_a_super_call_survive_garbage` catches the mutant (its `Old` line), and so did the probes `getsuper_before_args` and `gc_old_parent_on_stack`. Introduced.
3. Mutant Mb, `class.rs:439`: `SuperCallKept` stops charging a step per argument. The 19 tests pass. `Reflect.construct(D3, args)` through three default constructors with 100 arguments costs 557 steps on the unit's code and 257 with the mutant. No test checks steps here. The engine is correct. Test text (note only, not added): for k in {0, 100}, build `class P { constructor() { this.n = arguments.length; } } class D1 extends P {} class D2 extends D1 {} class D3 extends D2 {}` and an array `args` of k entries; run `Reflect.construct(D3, args).n === k;`; then `assert_eq!(steps[1] - steps[0], 500)` on `last_entry_steps()`. Introduced.
4. Mutant Mc, `crates/compiler/src/lib.rs:1053`: `KeepThisEnv` counts its hops without the environments the constructor's own code pushed. The 19 tests pass. Six probes throw `ReferenceError` instead of answering. The test262 return-override cases never return from inside a block that pushes an environment. The engine is correct. Test text (note only): `both_pass("class A { constructor(x) { this.x = x; } } class B extends A { constructor() { var f = () => this; { let a = 1; var g = () => a; super(1); return; } } } new B().x === 1;")`. Introduced.
5. Item 10, the cost of "every arrow inside a derived constructor captures `this`": an arrow that never reads `this` still moves the binding into the constructor's function environment. Measured over 100 kept instances: 1,101 live bytes per instance against 917 for the same arrow in a base constructor, so 184 bytes more per construction. Steps are the same (4,818 per 100, with or without the arrow). Arrows in a base constructor are unchanged. Introduced.

**Cases added** (uncommitted, in `<trees>/swell-impl-derived/crates/swell/tests/derived_classes.rs`, all passing). CPU seconds alone, as default / gate's collect build / lane's collect build:
- `adversary_this_before_super_in_every_position`: 0.017 / 0.035 / 0.373
- `adversary_return_errors_by_every_route_of_construct`: 0.018 / 0.028 / 0.295
- `adversary_parents_changed_after_the_definition`: 0.016 / 0.027 / 0.286
- `adversary_builtin_parents_fall_back_to_the_realm_of_new_target`: 0.012 / 0.020 / 0.200
- `adversary_values_of_a_super_call_survive_garbage`: 0.019 / 0.076 / 0.867

**Results by item**
- **Item 1 (`this` before `super()`):** every position in the brief throws `ReferenceError`, including `delete this`, `super[key]` before its key is evaluated, and `super.x = v` before `v` is evaluated. A second `super()` binds after a first one threw. An arrow can call `super()` after the constructor returned.
- **Item 2 (realm of the return-rule errors):** clause and engine agree on every route.
  - `Reflect.construct` → the realm of that `Reflect`.
  - A Proxy with no trap, a SuperCall, `new`, a bound function → the caller's realm.
  - The host's construct → the realm it names.
- **Items 3 and 4 (heritage, built-in parents):** all match the clause. A Proxy heritage has its `prototype` read once. `extends Symbol` defines, and constructing it throws. The NewTarget-realm fallback works for Object, Array, Map, Error, Promise, Function and Number.
- **Item 5 (default derived constructor):**
  - 0, 1, 255, 256 and 65,536 arguments pass, with no iterator.
  - Step sweep from 1 to 123: 121 `Terminated(StepBudget)`, 2 normal answers, nothing else.
  - Memory sweep, 400 budgets in 64-byte steps from 389,722 bytes: 350 `Terminated(MemoryBudget)`, 50 normal answers, nothing else.
- **Item 6 (chain, garbage, plants):**
  - A 1,000-level chain built by a loop constructs at `frame_depth` 10,000. At 500 it throws a catchable `RangeError: frame_depth exceeded`.
  - 15,000 throwing constructions, run 3 times: heap after `collect()` is 1,238,898 every time, and the recursion depth is 398 before and after.
  - Plant P1 (frame does not trace NewTarget): nothing goes red, neither the 19 tests in either mode nor my probes. Every route also holds NewTarget elsewhere (the caller's function, a builtin's arguments, an arrow's box), so this plant cannot be seen.
  - Plant P2 (`this` bound before the parent returns): `super_called_twice_throws_a_reference_error_after_the_parent_ran` and `this_before_super_throws_a_reference_error` go red, in both modes.
- **Item 7 (`nested_super_calls_stop_at_frame_depth`):**
  - Plant P3 (no frame-depth check in `push_construct_frame`) turns it red in both builds, so depth 10 in the collect build still catches the defect.
  - CPU seconds, gate's collect build: 0.058 at 10, 0.306 at 50.
  - CPU seconds, lane's collect build: 0.669 at 10, 3.117 at 50.
- **Item 8 (stack):**
  - The four tests of `test-stack-profiles` pass in both profiles.
  - The construction through `Reflect.construct` from inside `B` runs to depth 1,000 on seven eighths of `min_stack_bytes()` (12,838,560 bytes) and answers `true,1001`.
  - Measured stack per re-entry in the dev profile: about 7,751 bytes (the default constructor takes the same; `f.call` takes 7,311). The bound is 14,336.
  - `Constructed` is the size of `Option<ObjectRef>`, which the library's test holds. No test holds `size_of::<Frame>()`, and I did not measure the frame directly.
- **Item 9 (eval and `super`):** every case matches PerformEval, flag by flag, including the changed expectation `1,SyntaxError,SyntaxError,SyntaxError`.
- **Item 10 (parser and compiler):** all 22 parser probes match the clause, and `super()` in a nested class's heritage or computed key runs. No `.disasm` that existed before the unit changed: both stat diffs list only the 6 new files.
- **Item 11 (times):** every test is within its bounds. The largest are 0.296 default, 0.107 gate's collect, 1.217 lane's collect (the 1,000-chain test).
- **Tree state:** `git status --short` was empty at the start. `cargo fmt --all --check` only reported my own additions, which I fixed.

**Commands** (one pool job at a time; plants and mutants ran with `cd` into the scratch copy, as earlier adversaries did)
- Probes round 1, default: `2 tests run: 2 passed`. 115 OK, 2 DIFF, both mistakes in my own probes.
- Probes round 2, default, `--test-threads 1`: `8 tests run: 8 passed`, 0 DIFF.
- Stack probe: `1 test run: 1 passed, 1533 skipped`.
- Probes, collect build: `8 tests run: 7 passed, 1 timed out`. The timeout was my 65,536-argument spread probes at 60 s. Without them: `1 test run: 1 passed`, 0 DIFF.
- Cases, default: `24 tests run: 24 passed, 1507 skipped`, exit 0.
- Cases, collect: `24 tests run: 24 passed, 1486 skipped`, exit 0.
- P1: `19 tests run: 19 passed` in both modes, exit 0.
- P2: `19 tests run: 17 passed, 2 failed` in both modes, exit 100.
- P3: `1 test run: 0 passed, 1 failed` in both modes, exit 100.
- Ma / Mb / Mc on the 19 tests: `19 tests run: 19 passed`, exit 0 each.
- Mb steps: `m0 … (100, 557)`, `mb … (100, 257)`.
- Timing jobs (`slot --timeout 30m sh -c "$(cat time-node.sh)"`): exit 0, every run `rc 0`.
- `slot --timeout 30m task test-stack-profiles`: `test-stack-profiles release: 1 test binaries: 4 passed, 0 failed, 5 skipped` and `test-stack-profiles stack-debug: 1 test binaries: 4 passed, 0 failed, 5 skipped`.
- `cargo fmt --all --check`: exit 1 (only `derived_classes.rs:608`, my code), then exit 0 after the fix.
- `git status --short`: ` M crates/swell/tests/derived_classes.rs`.

**Not attacked**
- No test262 part run, so whether the 296 cases catch the mutants rests on greps of the 331 listed files.
- A heritage that awaits, and `new.target` inside generator or async heritages.
- Budget sweeps in the collect build, and times with `SWELL_FULL_COUNTS=1`.
- Fields, private names and static blocks (not on this tree).

```findings
- file: crates/syntax/tests/sources.rs
  line: 793
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The rewritten test expects the same answer from a script and from eval code with no flag at every comparison, so a parse_script that called parse_eval would pass and the test no longer holds what its name says.'
- file: crates/swell/src/interp/class.rs
  line: 389
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'A SuperCall that reads the parent after the arguments instead of the value GetSuperConstructor read passes the 19 tests and no listed case changes the parent during the arguments; the added case adversary_values_of_a_super_call_survive_garbage catches it.'
- file: crates/swell/src/interp/class.rs
  line: 439
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'Removing the step per argument of SuperCallKept passes the 19 tests (557 against 257 steps for 100 arguments through three default constructors) because no test reads the steps of the default derived constructor.'
- file: crates/compiler/src/lib.rs
  line: 1053
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'KeepThisEnv hops counted without the frame pushed environments pass the 19 tests while a return inside a block with a captured let in a derived constructor then throws ReferenceError.'
- file: crates/compiler/src/resolve.rs
  line: 1648
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'Every arrow in a derived constructor moves the this binding into the function environment even when it reads no this, 184 live bytes more per kept construction and no extra step.'
```

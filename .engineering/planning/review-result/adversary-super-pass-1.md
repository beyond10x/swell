---
format: aep.planning-md/3
id: review-result:adversary-super-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit super of wave big-2
relations:
- reviews: story:super-property-and-super-syntax
revision: 1
---
Adversary, pass 1 on the unit super of wave big-2 (impl/super at 6a89ce2d), 2026-11-26. The pass ran in two parts; this is the report of the second part, which reran and extended the first part's cases.
Verdict: one red case, of a refusal older than the unit; 2 findings (1 pre-existing note, 1 introduced note no script reaches); 45 cases, 1 red; three mutants the unit's tests miss are caught by the adversary's cases.

unit: super (`impl/super` at `6a89ce2d`, worktree `swell-impl-super`, plus uncommitted adversary test files)
verdict: red
cases: executed 14→45, red 1
origin: introduced 1, pre-existing 1, undecided 0
wrote-outside-worktree: `<scratch>/wave2/super/adversary-1b/` and unreferenced loose objects in the repository's git object store (listed under 6)
needs-coordinator: yes

## 1. Diff

```
 crates/swell/tests/super_property.rs | 807 +++++++++++++++++++++++++++++++++++
 1 file changed, 807 insertions(+)
?? crates/syntax/tests/sources/early-errors/adv-super-property-after-a-method-closes.js
?? crates/syntax/tests/sources/early-errors/adv-super-property-in-computed-key-at-top.js
?? crates/syntax/tests/sources/early-errors/adv-super-property-in-function-property-value.js
?? crates/syntax/tests/sources/valid/adv-super-property-nesting.js
```

Only test files changed. Nothing is committed or pushed, and nothing is under `.engineering/`.

## 2. Cases

**The earlier run's cases now compile and run:**
- Its 24 cases: default mode `38 tests run: 38 passed`.
- Its 4 fixtures: `swell-syntax --test main` `57 tests run: 57 passed`.

**I added 7 cases to `super_property.rs`.** First run of these 7 alone (`red1.swell.log`): 2 red.
- `adv_generator_return_and_throw_while_suspended_in_a_super_reference`: red because of my own mistake. `3 + true` gave `4`; I fixed the expectation and it is green now.
- `adv_a_method_in_an_arrow_parameter_inside_a_method_runs`: red, and still red. This is finding 1. Verbatim:
  ```
  left: (Errors(["super in an arrow function is not supported before slice 8 (4:65)"]), Errors(["super in an arrow function is not supported before slice 8 (5:65)"]))
  right: (Normal("\"t,b,p\""), Normal("\"t,b,p\""))
  ```
- Green:
  - `adv_try_finally_around_super_references_with_return_and_break`
  - `adv_async_rejection_inside_a_super_reference`
  - `adv_no_source_reaches_a_super_call`: 13 sources plus `eval`/`Function`.
  - `adv_super_recursions_through_accessors_and_keys_end_in_a_range_error`
  - `adv_super_re_enters_the_interpreter_on_the_promised_stack`: 1,000 re-entries through a `super` getter, setter and key, on 7/8 of `min_stack_bytes`.

**Times.** The whole collect-mode binary, under load, took 6.58 s for `adv_super_loops_end_at_the_memory_and_step_budgets` and 5.51 s for `adv_dropped_assignment_forms_in_a_loop`.
- I scaled three earlier-run cases down under the feature, through a new `adv_rounds` helper: `adv_dropped…` 50→20 rounds, `adv_a_throwing_accessor…` 20→10, step budget 10,000→2,000.
- Final module run, collect mode, cases over 1 s: `adv_super_re_enters…` 2.26 s, `adv_dropped…` 1.37 s, `adv_super_loops…` 1.31 s.
- Default mode: none over 1 s (the slowest is 0.43 s).
- I did not run the whole binary again after the scaling.

## 3. Suite runs (after the cases)

| Command | Result | Exit |
|---|---|---|
| `-p swell --test main -E 'test(/^super_property::/)'` | `45 tests run: 44 passed, 1 failed` | 100 |
| the same, `--features collect-every-safe-point` | `45 tests run: 44 passed, 1 failed` | 100 |
| `SLOT_TIMEOUT=30m … -p swell --test main` | `1199 tests run: 1198 passed, 1 failed, 1 skipped` (19 s) | 100 |
| the same, collect | `1199 tests run: 1198 passed, 1 failed, 1 skipped` (47 s) | 100 |
| `-p swell --lib` / collect | `250 passed` / `246 passed` | 0 / 0 |
| `swell-syntax` / `swell-compiler` / `swell-test262` `--test main` | `57` / `16` / `64` passed | 0 |

- The only failure in every run is finding 1.
- Slowest test of the whole collect run: `builtins_json_parse::parse_gives_back_its_charges_on_every_way_out` at 28.1 s. It is not this unit's test.

**Item 10, mutants planted in `.out/adv-mut`.** Each was run against `super_property::` and the story's 66 test262 cases (128 runs):

| Mutant | Unit's 14 tests | test262 | Red adversary case |
|---|---|---|---|
| none | green | 0 not passed | — |
| (a1) `get_super_index` converts the key before the null check | green | 0 | `adv_null_base_throws_before_the_key_is_converted` |
| (a2) the same in `super_key` | green | 0 | same |
| (b) `PropertyKey` emitted when a `super[k]` reference is made | `super_compound_and_logical_assignment_and_update` red | 6 runs fail (`prop-expr-getsuperbase-before-topropertykey-putvalue*`) | 4 cases |
| (c) `delete_super` reads the base first | green | 0 | `adv_delete_and_typeof_of_a_super_reference` |

- The earlier claim for (a) and (c) holds: the unit's tests and test262 miss them.
- The earlier claim for (b) is refuted: the unit's tests and test262 already catch it.

## 4. Findings (tree `6a89ce2d` plus the uncommitted cases)

**1. A method nested in an arrow's parameters inside a method is refused.**
- Measured: `params() { var f = (a = { n() { return super.x; } }) => a.n(); … }` answers `super in an arrow function is not supported before slice 8`. The case is `super_property.rs:1032`, exit 100.
- The same arrow at the top level and in an arrow body runs. No `super` in that source stands in an arrow's own code.
- Cause: `super_in_cover` (`parser/function.rs:444`, `:473`) scans every node of the cover, including those inside the nested method. The unit's comment says so.
- What reaches it: source text only. The refusal is lifted with the arrow refusal by `story:derived-classes-and-super-call`.
- Origin pre-existing: a build of the base `896bcbe9` answers `super is not supported before slice 8 (1:45)` for the same source.

**2. Contract drift: a frame without a home object.**
- `super_ref.rs:29` and `:36` answer `Termination::EngineFault`. The story (Change, run-loop bullet) says the function "answers the `TypeError` of a missing base rather than panic".
- What reaches it: nothing found. All three define paths call `make_method`, and `adv_no_script_reaches_a_frame_without_home_object` is green.

**Earlier finding 1 (`super.#x`) is withdrawn: it is nothing against this unit.**
- Measured: the base answers `super is not supported before slice 8 (1:24)`, the unit answers `private name is not supported before slice 8 (1:30)`. Both are refusals of slice 8.
- The lexer refuses the token (`lexer.rs:373`), and the story lists `lexer.rs` as not written.
- `story:class-element-syntax-and-early-errors` lifts that refusal. After it, `super_name`'s `_ => unexpected()` gives a grammar error.
- Note for that unit: its list holds only the class form (`super-private-access-invalid.js`). Add an object-literal early-error fixture `({ m() { super.#x } })`.

**Merge with classsyntax and newtarget (note for the coordinator, not against the unit).**
- **Conflicting hunks:**
  - super × classsyntax: `early.rs` 1 hunk (the HasDirectSuper comment and its `if`); `parser/expression.rs` 2 hunks (the module doc, the `Keyword::Class`/`Keyword::Super` arms). `parser/mod.rs` merges cleanly, so the earlier run was wrong about it.
  - super × newtarget: `parser/expression.rs` 1 (doc); `parser/function.rs` 3 (`open_context` with the `save_context` doc, the push, the pop in `function_end`); `parser/mod.rs` 5 (fields, `parse`, `parse_eval`, `Parser::new`, the release).
  - classsyntax × newtarget: `parser/expression.rs` only.
- **Break 1 is confirmed by a build of the copy.** With every conflict resolved "keep both", the copy fails first with `E0425 cannot find value context` at `parser/mod.rs:197`. Cause: newtarget's `strict: context.strict_caller || use_strict` merges cleanly into the unit's `parse_script_with`. With that line fixed, it fails with `E0004 … ClassConstructor, FieldInitializer and StaticBlock not covered` at `function.rs:846` (`super_property_of`).
- **Trap in `parse`:** taking newtarget's side of `parse` (`parse_eval(..., EvalContext::from(false))`) would treat every script as eval code. Every `super.x` would then answer the eval refusal, and no test of any unit shows it. I kept the unit's path, and `super.x;` answers `super property outside a method`.
- **Break 2 is measured but planned.** `class A extends B { constructor() { super(); } }` answers `SyntaxError: super call outside a derived class constructor (1:37)`. The class-syntax story's Context paragraph (line 60 of `class-syntax-and-early-errors.md`) says "a SuperCall in a class is an early error until `story:derived-classes-and-super-call`". Only the message text is false for that source.
- **Class methods: the earlier run is right.** `class A { m() { return super.x; } }` answers the hold `class is not supported before slice 8 (1:1)`, so `super` is admitted, with no line at `class.rs:244`.
  - Getter, setter, static, generator and async methods: same answer.
  - Constructor: same answer, but only with the line `ClassConstructor => SuperProperty::Method`.
- **Tests on the merged copy** (with my two compile lines): `swell-syntax` 59 passed, `swell-compiler` 17 passed, swell `super_property|new_target|async_methods|async_generator_methods` 41 passed.

## 5. Attacked and not broken

- Item 6:
  - generator `return()`/`throw()` while suspended inside `super[yield] = yield` and `super[k] += yield`;
  - `try`/`finally` around references, with `break`, `continue` and `return`;
  - async rejections in the key, in the right side of `=` and of `+=`, and uncaught; the base is read before the `await` and the key converted after.
- No source reaches `Expr::SuperCall`. I found no compiler fixture of a SuperCall, and I wrote no hand-built tree.
- Item 3: re-entry and frame limits through `super` getters, setters and keys end in a `RangeError`, and the agent runs on. No stack overflow on the promised stack.
- Every case of the earlier run, confirmed by a run in both modes.
- The cost of `super_in_cover` is bounded by `parser_nesting` (about 90 arrow levels): +35 % parse time at depth 90 with 200,000 array elements (824 → 1,102 ms, debug build).

**Not checked:**
- no disassembly fixture for the other forms (`disasm` prints no stack depth);
- the whole binary after the final scaling;
- `size_of` of a frame;
- the mutants against the whole binary.

## 6. Paths written outside the worktree

- `<scratch>/wave2/super/adversary-1b/`:
  - job scripts: `suite.sh`, `whole.sh`, `mutants-job.sh`, `merge-job.sh`, `merge2-job.sh`, `mktrees.sh`;
  - logs: `run1.*`, `red1.*`, `suite1.*`, `suite2.*`, `whole1.*`, `mutants.log`, `merge.log`, `merge2.log`, `node-probe.log`;
  - merge material: `diff-*.patch`, `mt-*.txt`, `idx1`, `idx-a`, `idx-b`, `m1/`, `m2/`.
- Loose objects in the repository's git object store, no ref:
  - commit `5d9c5cb2`;
  - trees `19e887ac`, `c517e5bb`, `a6597293`;
  - the merge-tree results `3e8b03c4`, `c3bf52dc`, `f2ac7825`, `1d1a2e34`;
  - blobs of the resolved files.
- Inside the worktree, ignored by git: `.out/adv-merge`, `.out/adv-mut`, `.out/adv-mut-part`, `.out/adv-mut-*.log`.

## Uncommitted cases (all compile)

- `<trees>/swell-impl-super/crates/swell/tests/super_property.rs` (31 `adv_*` tests, 1 red)
- `<trees>/swell-impl-super/crates/syntax/tests/sources/early-errors/adv-super-property-after-a-method-closes.js`
- `<trees>/swell-impl-super/crates/syntax/tests/sources/early-errors/adv-super-property-in-computed-key-at-top.js`
- `<trees>/swell-impl-super/crates/syntax/tests/sources/early-errors/adv-super-property-in-function-property-value.js`
- `<trees>/swell-impl-super/crates/syntax/tests/sources/valid/adv-super-property-nesting.js`

```findings
- file: crates/syntax/src/parser/function.rs
  line: 444
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: An arrow function inside a method whose parameters hold an object-literal method using super is refused as "super in an arrow function", though no super stands in the arrow's own code; the same arrow runs at the top level.
- file: crates/swell/src/interp/super_ref.rs
  line: 29
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: SuperBase on a frame whose function has no home object ends the run in Termination::EngineFault where the story says it answers the TypeError of a missing base; no script was found that reaches it.
```

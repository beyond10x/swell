---
format: aep.planning-md/3
id: review-result:adversary-elements-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit elements: red, one failing case of 14 and five findings'
relations:
- reviews: story:class-element-syntax-and-early-errors
revision: 1
---
Adversary pass 1 on `impl/elements` at `bd71f105` (wave big-2, slice 8, order 8.2), run on 2026-11-26.
The report below is the adversary's last message; paths of the workstation are replaced.

unit: elements, adversary pass 1 on `story:class-element-syntax-and-early-errors`; covers `impl/elements` at `bd71f105` plus my uncommitted test additions
verdict: red
cases: executed 63→77, red 1
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/elements/adversary-1/ (15 files)
needs-coordinator: yes. Finding 1 is a deviation the unit disclosed (point 6): it needs a call to keep the deviation (and drop or invert my case) or change the code. Finding 2 changes the design contract that `story:class-static-blocks` relies on.

**Result:** 1 of my 14 new tests is red. It shows the parser refusing four sources that the pinned draft's grammar accepts. Across about 330 other sources the parser gave the standard's verdict every time. The other four findings are not red cases: one place where the code disagrees with the design, two gaps no test can reach yet, and one wrong error message.

## 1. `git --no-pager diff --stat`

```
 crates/syntax/tests/sources.rs | 785 +++++++++++++++++++++++++++++++++++++++++
 1 file changed, 785 insertions(+)
```

Only a test file changed. Nothing is committed.

## 2. Cases added (all in `crates/syntax/tests/sources.rs`)

Each source is checked through all three parse entries (script, eval, `Function` body). "Accepted" means the one answer is `class is not supported before slice 8`. "Error" means a SyntaxError that is not a refusal.

- **Red:** `adversary_field_initializer_yield_and_await_by_the_pinned_grammar` (:3083). It asserts that the parser accepts `x = await 1`, `static x = await 1`, `x = yield 1` and `#x = yield` inside an async function or a generator. It failed the first time it ran (`red-5.log`), alone:
  ```
  12 mismatch(es):
  Script "async function f() { class C { x = await 1; } }": expected H, got Err(["unexpected number 1"])
  Script "function* g() { class C { x = yield 1; } }": expected H, got Err(["unexpected number 1"])
  Script "function* g() { class C { #x = yield; } }": expected H, got Err(["yield may not be an identifier in a generator function"])
  ... (the same through Eval and FunctionBody)
  test result: FAILED ... exit 100
  ```
- **Green, each 0.01–0.04 s:**
  - `adversary_await_and_yield_in_class_elements`: 48 sources for the unit's point 6, including the test262 valid static-block cases.
  - `adversary_arguments_and_super_in_class_elements`: 39 sources on `arguments`, SuperCall, `super.x` and `new.target`.
  - `adversary_static_block_statements`: 29.
  - `adversary_element_names_and_duplicates`: 42.
  - `adversary_private_name_scope`: 25.
  - `adversary_delete_and_private_in`: 41.
  - `adversary_private_name_tokens_and_member_forms`: 59.
  - `adversary_field_ends`: 30.
  - `adversary_dynamic_function_entries`: 7.
  - `adversary_context_after_class_elements`: 22. Each would go red under a mutant that skips putting back the class body's context after an element (`element_context_close`, `static_block_close`, the checker's `Restore`).
  - `adversary_more_element_edges`: 37.
- **Green, slower:**
  - `adversary_class_element_kinds_nest_in_turn_within_the_bound`: 0.08 s alone, 0.20 s in the full-count run. Six class kinds nested in turn, 100,000 deep, on the deep-limit stack. Every entry answers only the nesting error.
  - `adversary_class_elements_scale_linearly`: 0.71 s alone, 1.27–1.68 s in the suites. Measured from 2,500 to 10,000 elements, both bytes charged and time grow about 4×. Example: private fields used once, 2,637,807 → 10,556,091 bytes in 23 → 96 ms.

My first run also turned up `super` inside an arrow in an initializer or static block. That answers the slice-8 refusal of `story:super-property-and-super-syntax` at its token, the same as in a method. It is not this unit's, so I took those two lines out and left a comment.

## 3. Suite runs (after the cases existed)

- `cargo nextest run --locked -p swell-syntax --test main --no-fail-fast` → `77 tests run: 76 passed, 1 failed`, exit 100. The failure is my red case. The slowest test is stack-bound part 16 at 1.395 s.
- `SWELL_FULL_COUNTS=1 cargo nextest run --locked -p swell-syntax --test main --no-fail-fast` → `77 tests run: 76 passed, 1 failed` in 9.65 s, exit 100. Same red case.
- `cargo nextest run --locked -p swell-compiler --test main --no-fail-fast` → `17 tests run: 17 passed`, exit 0.
- The "before" count of 63 comes from the unit's report.

## 4. Findings

1. **[Yield]/[Await] in a field initializer** (`crates/syntax/src/parser/class.rs:552-553`)
   - Measured: the red case above (assertion at `sources.rs:2350`).
   - The pinned draft has `Initializer[+In, ?Yield, ?Await]` and no early error against a YieldExpression or AwaitExpression there. Its "Contains YieldExpression/AwaitExpression" rules cover formal parameters only (draft text lines 24357–26707; class early errors at 25553–25561).
   - The unit refuses these sources because it clears both flags. No test262 case covers this, and the unit disclosed it as point 6.
   - Reached by any script with such an initializer.
   - In every other source the clear-then-refuse approach gave the standard's verdict.
   - Origin: introduced (on the base a field was refused at its token).
2. **Static block has no arguments object** (`crates/syntax/src/scope.rs:243`)
   - `StaticBlock` now never gets an arguments object.
   - The design says a static block "makes an arguments object when its code can observe one: only through a direct `eval`" (`docs/design/engine.md:1732`).
   - Concretely, `class C { static { eval("arguments") } }` should give the block's own empty arguments object. Since a static block does not hand `arguments` up the way an arrow does, it would get none.
   - Reaches nothing today: scope analysis only runs on sources with no hold (`lib.rs` `analyse_parsed`). It will reach the compile in `story:class-static-blocks`.
   - Origin: introduced (the base excluded only `Arrow`).
3. **No test in this tree can see the element holds** (`crates/syntax/src/parser/mod.rs:513`)
   - `hold()` keeps the first hold, and `hold("class")` (`class.rs:154`) always runs before any element hold.
   - So deleting any of the five `HOLDS` lines or the five element `hold(...)` calls leaves every test green, including `held_sources_answer_their_first_construct_still_held` over the 8 new held sources.
   - They are proven only after the join with `story:class-definition-evaluation`, as the acceptance requires.
   - Found by reading; I did not run the mutant.
4. **Unguarded mutant: static private method** (`crates/syntax/src/parser/class.rs:389`)
   - Removing `if !modifiers.is_static` gives `static #m(){}` an instance-elements function. Nothing catches it: the tree of a held source cannot be reached through the public API, and the unit's tree test has no static private method.
   - Test text for the unit's test module: assert that `class C { static #s() {} }` has `instance_elements == None` and a function list `[ClassConstructor, Method]`.
5. **Wrong error message** (`crates/syntax/src/early.rs:1324`)
   - `class C { static { class D { x = await; } } }` answers `await may not be an identifier in an async function`, but no async function is involved.
   - Cause: `await_identifier()` checks whether the `arguments` flag is the static block's, and inside a nested field initializer it is the field's.
   - The verdict is right; only the text is wrong.

## 5. Attacked and not broken

- Point 6 in every other form the brief lists: `var await` in a script, computed keys, nested functions, arrows, a label, the binding forms, `for await`.
- Each early error, both sides: `constructor`/`prototype` names (string, escaped, computed); `#constructor`; every duplicate pair; `arguments` (through arrows, nested class keys, heritage, object keys); SuperCall; `return`, `break` and `continue` across a static block's edge; var/lexical names; labels.
- AllPrivateIdentifiersValid: heritage at both levels, computed keys, sibling class, after the class, eval and `Function` text, `Function` parameters.
- `delete` with parentheses and optional chains.
- `#x in` at every relational position, including `for` heads.
- The lexer cases: escapes, a digit, a space, a line break, keywords, end of input, `#!`, white space and comments around `.`.
- Member forms and assignment targets.
- Field-end and ASI cases: `x\n()` is a method x and `get\nx(){}` is a getter, as the standard reads them.
- Growth charging: I read about 390 non-test lines of `class.rs` and about 335 of `early.rs`. Every list and map grows after its charge. I found no `unwrap`, no unchecked index and no unguarded recursion. The only allocation made before its charge is the `format!` message in `error()`, a pattern the base already uses.
- Hold order and refusal order: the first answer is always the class hold, an early error, or a refusal at its token.
- Stack bound: the unit's `class_field` kind lands in part 8 through the `Function` body entry; the unit measured that part at about +0.06 s. Part 8's full-count overrun is older than this wave (the backlog note). I did not re-measure it.
- Tree numbering (item 5): a computed key's functions are numbered before their field's function, and a static run continues across a constructor or `;`. Nothing I found in the compiler, resolve or scope code depends on either order; there is no public way to observe the tree of a held source.

## 6. Paths written outside the worktree

`<scratch>/wave2/elements/adversary-1/`, containing:
- `cases.rs`, `cases-2.rs` … `cases-5.rs`
- `suites.sh`, `suites.done`
- `red-1.log` … `red-5.log`
- `suite-default.log`, `suite-full.log`, `suite-compiler.log`

```findings
- file: crates/syntax/src/parser/class.rs
  line: 552
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Clearing [Yield] and [Await] in a field initializer refuses `x = await 1` and `x = yield 1` inside async functions and generators, which the pinned grammar Initializer[+In, ?Yield, ?Await] admits with no early error; red case adversary_field_initializer_yield_and_await_by_the_pinned_grammar.
- file: crates/syntax/src/scope.rs
  line: 243
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: A StaticBlock never gets an arguments object, but the design (engine.md:1732) says it gets one when a direct eval can observe it, so eval("arguments") in a static block would find no binding of its own once story:class-static-blocks compiles it.
- file: crates/syntax/src/parser/mod.rs
  line: 513
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: Because hold() keeps the first hold and the class hold always comes first, removing any of the five element holds or HOLDS lines leaves every test in this tree green; they are proven only after the join with story:class-definition-evaluation.
- file: crates/syntax/src/parser/class.rs
  line: 389
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: Removing the is_static guard so a static private method creates the instance-elements function is caught by no test, because the tree of a held source is unreachable through the public API and the unit's tree test has no static private method.
- file: crates/syntax/src/early.rs
  line: 1324
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: '`await` in a field initializer of a class nested in a static block gets the message "await may not be an identifier in an async function" although no async function is involved.'
```

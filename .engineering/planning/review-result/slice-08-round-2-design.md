---
format: aep.planning-md/3
id: review-result:slice-08-round-2-design
kind: review-result
status: active
title: 'Slice 8, round 2, design: needs-revision, 3 warnings and 5 notes'
relations:
- reviews: story:new-target-meta-property
- reviews: story:super-property-and-super-syntax
- reviews: story:class-syntax-and-early-errors
- reviews: story:class-definition-evaluation
- reviews: story:class-element-syntax-and-early-errors
- reviews: story:derived-classes-and-super-call
- reviews: story:public-class-fields
- reviews: story:private-names-and-private-methods
- reviews: story:class-static-blocks
- reviews: story:eval-in-class-field-initializers
- reviews: story:private-fields-and-private-in
- reviews: story:private-names-in-eval-code
- reviews: story:host-ensure-can-add-private-element
- reviews: story:slice-08-conformance-close
revision: 1
---
Plan critic, design, round 2 on the fourteen draft stories of
`epic:slice-08-classes` (`plan/slice-08` at `238e8d64`), 2026-11-26.
Verdict: needs-revision. The 10 findings of round 1 are closed; 3 warnings and 5 notes are open.

needs-revision

Round 1's ten design findings are all closed. The revised set has two new warnings, plus five notes that don't hold it back.

**Graph.** I walked all 329 `depends_on` edges of the store, slice 9 and slice 4 included. `tsort` finds no cycle. The 14 stories' own edges run strictly forward through the seven orders. Every edge to slice 9 (`weak-map-and-weak-set`, `host-object-overrides`, `slice-09-conformance-close`) points at a story that exists, and none points back.

The chain has a stated reason beside each edge, so it is not a bare queue. I traced about 30 acceptance reads (tests, lists and fixtures) to a producer in the set, and an edge records all of them. `aep plan artifact validate` prints `valid`. Only the archived `instance-elements-initialization-order` still names itself: no story, list or script refers to it, and no story needs it.

**Round-1 findings, one by one**
1. Derived `this` is now a hidden binding in the constructor's function scope, not a frame slot (`derived-classes-and-super-call.md:66`, `:89`). Closed.
2. `SuperCall` is four instructions with no re-entry (`derived-classes-and-super-call.md:89`, `:99`). Closed.
3. The Rust seam is gone: `InitInstance` and compiled calls replace it (`public-class-fields.md:37`, `:60`; `class-definition-evaluation.md:105`). Closed.
4. The eval frame takes the caller's `function`, and an arrow's frame takes its box's function (`new-target-meta-property.md:82-83`). Closed.
5. Static fields and blocks interleave as compiled calls in source order (`class-definition-evaluation.md:105`, `class-static-blocks.md:38`). Closed.
6. The instance-elements story is folded into `private-fields-and-private-in.md:57`. Closed.
7. `super.x` has one owner, `method_head` in the super story, and the SuperCall flag has one owner, the derived story (`super-property-and-super-syntax.md:63`, `:85`; `derived-classes-and-super-call.md:66`). Closed.
8. The size note now offers `toString` alone as the split (`class-definition-evaluation.md:110`). Closed.
9. `ClassFieldInitializerName` is the template kind `FieldInitializer` (`eval-in-class-field-initializers.md:29`). Closed.
10. Private names in eval code come from PerformEval's walk of the environment chain (`private-names-in-eval-code.md:41`, `:56-57`). Closed.

**New findings**

- **W1, assignment to a private target has no owner in `private-names-and-private-methods`, which adds `SetPrivate`.**
  - That story adds `GetPrivate` and `SetPrivate` (`:74`) and tests setters and `setting_a_private_method_throws_a_type_error` (`:102`). Its list has 24 setter cases.
  - Its compiler paragraph (`:97`) compiles only the read and call arm of `Expr::PrivateMember` and the optional-chain link. It never compiles `this.#m = v`.
  - `private-fields-and-private-in.md:55` and `:80` own "every assignment form with a private reference as its target". In the compiler, assignment goes through the `Target` enum (`crates/compiler/src/expression.rs:110-211`: `target`, `get_keeping`, `put`), which needs a private variant.
  - Order 3 therefore cannot pass its own setter cases; order 5 compiles the only emitter of `SetPrivate`. The `Target` surface is split across two stories.
  - Fix: private-methods owns `Target::Private` and the `=` form. private-fields adds only compound, logical, update, destructuring and `for`-of use of it.
- **W2, the Rust path to a derived [[Construct]] and the change to `constructed` have no owner.**
  - `derived-classes-and-super-call.md:90` changes `Frame.constructed` from `Option<ObjectRef>` to a three-state enum. `:101` lists "Not written: `crates/swell/src/object/`".
  - `constructed` is also typed in `crates/swell/src/object/ecmascript_function.rs:219`, `:255` and `:366` (`call_function`, `make_frame`, `construct_function`), and set in `crates/swell/src/script.rs:621` and `crates/swell/src/dynamic_code.rs:379`.
  - `construct_function` (`ecmascript_function.rs:236-246`) always calls `ordinary_create_from_constructor`. For a derived class it must make no object and must pass the derived state. `New` and `push_construct_frame` (`dispatch.rs:1584`) need the same branch on the box's kind. The story names the branch only for `SuperCall`.
  - Reflect.construct, a bound-function construct and a host construct all take the Rust path (the last named by `a_host_construct_sees_the_function`).
  - Fix: the story names `construct_function`, `make_frame`, `call_function`, `push_construct_frame` and the two `Frame` literals as sites it changes. It drops the `object/` exclusion for them and states the derived branch for both the `New` path and the Rust path.
- **W3, the realm of the derived-return errors is not carried for the first frame of a Rust entry.**
  - `derived-classes-and-super-call.md:66`, `:90` and the test `return_errors_are_of_the_callers_realm` rest on "after the caller's realm is restored".
  - In `return_from_frame` (`dispatch.rs:78-92`) the pop returns `Some(value)` for the base frame before any realm is restored. The slot still holds the callee's realm, and `run_frame` restores the saved realm only afterwards (`interp/mod.rs:180-203`).
  - So for a derived class constructed through the Rust path, the two errors would be made in the callee's realm. The design author listed this under "could not establish". The story neither carries it nor names a test or fallback.
  - Fix: the story says where the saved realm is restored for a base frame before the errors are made, and tests it through the Rust entry as well as `New`.

**Notes, not holding the set**

- N1: `super-property-and-super-syntax` refuses `super` in an arrow and in eval code until the derived story lifts it (`super-property-and-super-syntax.md:65`; `derived-classes-and-super-call.md:68`, `:88`).
  - So `super.x` in an object-literal arrow, which has no class, lands in the derived story's list.
  - The cause is that order 1 has no edge `super` → `new-target`.
  - The trade-off: with that edge, `super` would own both lifts and the derived story would own SuperCall only, at the cost of ordering the two order-1 stories. The coordinator may keep the current form.
- N2: `private_record` makes slots "named with its `#`" (`private-names-and-private-methods.md:97`). PerformEval lists only slots that carry a name, and slot names exist only for records a lookup by name can reach (`crates/swell/src/env/mod.rs:236-247`). No story says the private record is named whenever a direct `eval` is inside the class. `private-names-in-eval-code.md:56-57` assumes it.
- N3: `class-definition-evaluation.md:105` gives `MakeClassConstructor(f)` no kind operand. `derived-classes-and-super-call.md:90` says the class's sequence sets the box's kind to derived with "no field added" and names no instruction.
- N4: three stories describe who places the step-4 static calls. `class-definition-evaluation.md:105` emits every call. `public-class-fields.md:60` places the runs' calls. `class-static-blocks.md:40` "inserts the blocks' calls". Say that class-definition owns the sequence and the others supply the function it calls.
- N5: `new-target-meta-property.md:83` puts the arrow call's `this` in `interp/arrow.rs`. That file only creates the arrow (`arrow.rs:37`). The call path is `make_frame` (`ecmascript_function.rs:272`), which also sets `function: Some(function)` at `:366` and is the site to change. Separately, the design says `Frame.function` is "read nowhere else", but `interp/generator.rs:79` reads it. That reader is unaffected, because a generator is never an arrow.

Out of my lane, not weighed in the verdict: whether `--no-tests=fail` is known to the pinned cargo-nextest, and the sizes the design leaves unmeasured. The stories carry layout tests and fallbacks for those sizes.

```findings
- file: .engineering/planning/story/private-names-and-private-methods.md
  line: 97
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "This story adds SetPrivate and tests private setters (line 102, 24 setter cases) but compiles only the read and call arm of Expr::PrivateMember, while private-fields-and-private-in.md:55 and :80 own every assignment form with a private target, so no code in order 3 emits SetPrivate and the Target enum of crates/compiler/src/expression.rs is split across two stories; private-methods should own Target::Private and plain assignment, private-fields the compound, update, destructuring and for-of forms."
- file: .engineering/planning/story/derived-classes-and-super-call.md
  line: 101
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The story changes the type of Frame.constructed (line 90) and excludes crates/swell/src/object/ from its writes, yet constructed is typed in call_function, make_frame and construct_function (ecmascript_function.rs:219, 255, 366) and set in script.rs:621 and dynamic_code.rs:379, and construct_function and push_construct_frame always create this for the callee, so a derived class constructed by New, Reflect.construct, a bound function or the host has no owner for its no-object, Derived-state branch."
- file: .engineering/planning/story/derived-classes-and-super-call.md
  line: 90
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The rule that both derived-return errors are made in the caller's realm assumes return_from_frame restored it, but for the base frame of a Rust entry (Reflect.construct, bound or host construct) it returns before any restore (dispatch.rs:78-92) and run_frame restores the saved realm only afterwards (interp/mod.rs:180-203), a case the design lists as not established and the story neither carries nor tests."
- file: .engineering/planning/story/super-property-and-super-syntax.md
  line: 65
  category: design
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Super in an arrow and in eval code, including object-literal arrows with no class, is refused here and lifted by derived-classes-and-super-call.md:88 because order 1 has no edge from super to new-target-meta-property; the trade-off is that edge (this story then owns both lifts, derived owns SuperCall only) against keeping the two order-1 stories parallel."
- file: .engineering/planning/story/private-names-and-private-methods.md
  line: 97
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "private_record says the slots are named with their # but PerformEval lists only slots that carry a name and slot names exist only for records a lookup by name can reach (env/mod.rs:236-247); no story says the private record is named whenever a direct eval is inside the class, which private-names-in-eval-code.md:56-57 assumes."
- file: .engineering/planning/story/derived-classes-and-super-call.md
  line: 90
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The box's kind is set to derived by the class's sequence with no field added, but MakeClassConstructor(f) in class-definition-evaluation.md:105 has no kind operand and no instruction in the derived list sets it; give MakeClassConstructor the kind from the start."
- file: .engineering/planning/story/class-static-blocks.md
  line: 40
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "class-definition-evaluation.md:105 emits the step-4 call for every static element while public-class-fields.md:60 places the runs' calls and this story inserts the blocks' calls, so three stories describe one emission site; say that class-definition owns the sequence and the others only supply the function."
- file: .engineering/planning/story/new-target-meta-property.md
  line: 83
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The call of an arrow puts its this in make_frame (ecmascript_function.rs:272, function set at :366), not in interp/arrow.rs, which only creates the arrow (arrow.rs:37); name make_frame as the site that takes the box's this, new_target and function."
```

---
format: aep.planning-md/3
id: review-result:slice-08-round-1-design
kind: review-result
status: active
title: 'Slice 8, round 1, the design critic: needs-revision, 10 findings, 2 infeasible as drafted'
relations:
- reviews: story:new-target-meta-property
- reviews: story:super-property-and-super-syntax
- reviews: story:class-syntax-and-early-errors
- reviews: story:class-definition-evaluation
- reviews: story:class-element-syntax-and-early-errors
- reviews: story:derived-classes-and-super-call
- reviews: story:public-class-fields
- reviews: story:class-static-blocks
- reviews: story:private-names-and-private-methods
- reviews: story:instance-elements-initialization-order
- reviews: story:private-fields-and-private-in
- reviews: story:eval-in-class-field-initializers
- reviews: story:private-names-in-eval-code
- reviews: story:host-ensure-can-add-private-element
- reviews: story:slice-08-conformance-close
revision: 2
---
Plan critic, design, round 1 on the fifteen draft stories of `epic:slice-08-classes`
(`plan/slice-08` at `999e949e`), 2026-11-26.
Verdict: needs-revision, 10 findings, 2 of them infeasible as drafted. The critic's answer, as returned:

needs-revision

1. story:derived-classes-and-super-call — The `this` slot of a derived constructor is placed on the constructor's frame, and an arrow reaches it through its box. A heap object cannot hold a reference to a frame without `unsafe`. A frame index goes stale when the frame is popped, and `constructor-this-tdz-during-initializers.js` calls such an arrow from another frame and after the constructor returned. Decision 3 item 3 already puts the slot in the constructor's function environment record. The story and D1/D3 must say "a record slot, with the arrow's box holding the record". — `.engineering/planning/story/derived-classes-and-super-call.md:71`, `:89`; `docs/design/engine.md:502-505`; `crates/swell/src/interp/frame.rs:140`
2. story:derived-classes-and-super-call — SuperCall is one function that runs `construct`, then BindThisValue, then the seam. That only works through a reentry. A reentry for each level of `super()` costs `STACK_BYTES_PER_REENTRY` and `reentry_depth`, which contradicts "as a frame of the run loop (`frame_depth`)" and the 1,000-class chain test. SuperCall must be split: one instruction pushes the parent's construct frame, and a following instruction binds `this` and starts the instance elements. — `derived-classes-and-super-call.md:89`, `:97`; `crates/swell/src/interp/dispatch.rs:78-92`
3. story:public-class-fields — The seam is a Rust function that "pushes the initializer's frame before the constructor body's", with a "fields loop".
   - `return_from_frame` pushes every returned value onto the caller's operand stack. Nothing discards an initializer's result, and a Rust loop cannot push a second frame after the first returns.
   - The same holds for the walk of static elements in `interp/class.rs`.
   - D1 must define the seam as private-method adds in Rust, plus a compiled call-and-pop of the placement's synthetic method in the constructor's start code and after each SuperCall. The static elements likewise become compiled calls in the class-definition code.
   - The `interp/class.rs` loop hunks of public-class-fields, private-names-and-private-methods, class-static-blocks and instance-elements-initialization-order change with this.
   — `public-class-fields.md:47`, `:67`; `class-static-blocks.md:59`; `dispatch.rs:78-92`
4. story:new-target-meta-property — Eval code's frame has no function. Nothing in the set says how eval code finds the function its `this` comes from, yet three things need it:
   - `super.x` in eval code, which reads "the running function's [[HomeObject]]";
   - the flags of a nested eval;
   - an arrow made inside eval code, whose closure takes that function.

   D3/D4 and this order-1 story must give the eval frame that function. — `new-target-meta-property.md:78-80`; `super-property-and-super-syntax.md:80`; `crates/swell/src/dynamic_code.rs:374`
5. story:class-static-blocks — Static fields are one synthetic method per placement (Q3), and static blocks are separate synthetic methods "in the list of static elements". That cannot give the interleaved order of `static-init-sequence.js` (field, block, field, block). The scan put that case on private-fields-and-private-in's list, so the conflict between two order-3 stories lands on an order-5 story. D1 must decide how static fields and blocks interleave, and public-class-fields and class-static-blocks must own it. — `class-static-blocks.md:43`; `public-class-fields.md:51`; `docs/plan/slice-08/story-private-fields.cases.txt` (`static-init-sequence.js`)
6. story:instance-elements-initialization-order — Its whole Change is already claimed by the stories it joins:
   - private-names-and-private-methods puts the methods loop first ("methods first");
   - derived-classes-and-super-call calls the seam after `super()`;
   - public-class-fields gives initializers their home object and SuperProperty.

   The story itself expects most of its 448 cases to pass before it is built, so its check cannot fail, and it costs a whole order. Fold its cases and tests into private-fields-and-private-in, and point the edges of private-fields-and-private-in and eval-in-class-field-initializers at the three stories. Or move the order and the post-`super()` call out of those three into this story. — `instance-elements-initialization-order.md:52`; `private-names-and-private-methods.md:99`; `derived-classes-and-super-call.md:69`
7. story:derived-classes-and-super-call — Two stories claim the job of allowing `super.x` in class methods. class-syntax-and-early-errors says class methods go through `method_head`, which super-property-and-super-syntax makes allow `super.x`. derived-classes-and-super-call claims it for itself and says it sets that flag in every class method. Name one owner. If `method_head` sets it, class-definition-evaluation runs `super.x` without an edge to super-property-and-super-syntax, and derived's cases of `super.x` in a class pass before it is built. — `class-syntax-and-early-errors.md:53`; `super-property-and-super-syntax.md:69`, `:79`; `derived-classes-and-super-call.md:67`
8. story:class-definition-evaluation — The offered split moves the global class binding into a side story. Nearly every case on this story's list declares `class C` at the top of a script, so the side story would become an unrecorded dependency of this story and of every story at order 3. Offer `toString` alone. — `class-definition-evaluation.md:98`
9. story:eval-in-class-field-initializers — [[ClassFieldInitializerName]] is "kept on the initializer's frame", which grows the charged bytes of every frame. PerformEval reads only whether it is set, so a flag on the synthetic method's template is enough (once D1 settles finding 5). — `eval-in-class-field-initializers.md:49`
10. story:private-names-in-eval-code — It reads the private environment "from the function an arrow's `this` comes from". The private environment is lexical. For an arrow in an inner class's computed key or heritage, that function sits outside the inner class, so the inner class's names are missed. Read the private environment from the frame's environment chain only. — `private-names-in-eval-code.md:67`

**When D1 to D5 must exist:** D1, D3 and D4 must be written before order 1. They fix shapes that order-1 stories build: the arrow's box, the eval context, the eval frame's function, how super instructions find [[HomeObject]], and the class tree. Findings 1 to 5 also change the Change and Writes lines of the stories at orders 2 to 4, so the stories should be revised against D1 before they are approved. D2 can wait until before order 2. D5 can be host-ensure-can-add-private-element's own first commit: its requirement number depends on slice 9's start gate, which claims HOST-222 (`git show plan/slice-09:.engineering/planning/story/host-object-overrides.md`, line 123).

**What holds:**
- No cycle: `tsort` over all 301 `depends_on` edges of the store.
- The Private Name arena and a slot kind no script name reaches can be built without `unsafe` or a panic.
- One box per object record for [[PrivateElements]] can too (200 to 208 bytes is charged and stated).
- The `CompileError::Tree` arms cannot be reached: `analysed` in `crates/syntax/src/lib.rs:414` returns the held refusal before scope analysis on every entry (script, eval, `Function`).
- private-names-and-private-methods is one thing, not two, so its size is not a seam.

**What I read:**
- All 15 stories in full, against the code in `crates/` (frame, dispatch, ecmascript_function, dynamic_code, lib.rs, object/mod.rs) and `docs/design/engine.md` decision 3.
- slice 9's `host-object-overrides` and `slice-09-conformance-close`, and the scan rules in `scan.awk`.
- test262 `static-init-sequence.js` and `constructor-this-tdz-during-initializers.js`.
- Commands: `aep plan artifact relations`, `validate` (valid; 7 unscoped stories outside this set), and `graph --format json`.
- Edges: I walked the 39 edges of the set, plus the edges outside it from `slice-04-conformance-close` (17), plus the whole store with `tsort`.
- I traced 31 acceptance reads to a producer in the set; all are recorded by an edge, directly or through another story.

**What I could not establish:**
- The Q5 and Q6 edges to slice 9 are not in the graph at `999e949e`. The coordinator says it will add them, so host-ensure-can-add-private-element and slice-08-conformance-close have no edge to slice 9 today.
- Out of my lane (acceptance critic):
  - super-property-and-super-syntax's test filter leaves out `async_methods::` and `async_generator_methods::`, whose tests assert the `super` refusal (`crates/swell/tests/async_methods.rs:101`, `async_generator_methods.rs:232`).
  - class-definition-evaluation's filter leaves out `dynamic_function::` (`dynamic_function.rs:808`, `:1228`).
- Out of my lane (parallel-safety critic): derived-classes-and-super-call and public-class-fields assert the record stays at 200 bytes, while private-names-and-private-methods, built beside them, makes it 208.
- I wrote one file outside the repository: `critic-design-edges.txt in the coordinating session scratch directory` (the edge list for `tsort`). It can be deleted.

```findings
- file: .engineering/planning/story/derived-classes-and-super-call.md
  line: 89
  category: design
  severity: blocker
  verdict: INFEASIBLE
  origin: introduced
  message: "The this slot of a derived constructor lives on the constructor's frame reached through the arrow's box, which needs unsafe or a stale frame index, where decision 3 item 3 puts it in a function environment record slot that the box must hold."
- file: .engineering/planning/story/derived-classes-and-super-call.md
  line: 97
  category: design
  severity: blocker
  verdict: INFEASIBLE
  origin: introduced
  message: "SuperCall as one function doing construct, BindThisValue and the seam forces a reentry per super() level against the stated frame_depth design, so it must be split into an instruction that pushes the parent's construct frame and a following instruction that binds this and starts the instance elements."
- file: .engineering/planning/story/public-class-fields.md
  line: 47
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "A Rust seam that pushes initializer frames and walks a fields list cannot discard the initializer's result or push the next frame, because return_from_frame pushes every value onto the caller's stack, so D1 must make initializers and static elements compiled calls and the interp/class.rs loop hunks change."
- file: .engineering/planning/story/new-target-meta-property.md
  line: 80
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Eval frames have no function, yet super in eval code, nested-eval flags and arrows made in eval code need the function the this comes from, and no story or design change gives the eval frame that function."
- file: .engineering/planning/story/class-static-blocks.md
  line: 43
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "One static synthetic method for fields beside separate static-block methods cannot give the interleaved order of static-init-sequence.js, which sits on private-fields-and-private-in's list, so D1 must decide the interleaving and public-class-fields and class-static-blocks must own it."
- file: .engineering/planning/story/instance-elements-initialization-order.md
  line: 52
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The join story's change is already claimed by private-names-and-private-methods (methods first), derived-classes-and-super-call (seam after super()) and public-class-fields, so its cases pass before it is built; fold it into private-fields-and-private-in or move those parts into it."
- file: .engineering/planning/story/derived-classes-and-super-call.md
  line: 67
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Allowing super.x in class methods is claimed both by super-property-and-super-syntax's method_head (as class-syntax-and-early-errors says) and by this story, so one owner must be named and the edge or list adjusted to match."
- file: .engineering/planning/story/class-definition-evaluation.md
  line: 98
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The offered split moves the global class binding, which nearly every case of this list and of order 3 needs, into a side story, so the split offer must be toString alone."
- file: .engineering/planning/story/eval-in-class-field-initializers.md
  line: 49
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Keeping ClassFieldInitializerName on the initializer's frame grows every frame's charged bytes for a fact PerformEval reads only as set or not set, which a flag on the synthetic method's template carries."
- file: .engineering/planning/story/private-names-in-eval-code.md
  line: 67
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Reading the private environment from the function an arrow's this comes from misses an inner class's names in its computed keys and heritage, because the private environment is lexical and must be read from the frame's environment chain."
```

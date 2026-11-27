---
format: aep.planning-md/3
id: review-result:slice-08-round-2-parallel-safety
kind: review-result
status: active
title: 'Slice 8, round 2, parallel safety: needs-revision, 5 warnings and 2 notes'
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
Plan critic, parallel-safety, round 2 on the fourteen draft stories of
`epic:slice-08-classes` (`plan/slice-08` at `238e8d64`), 2026-11-26.
Verdict: needs-revision. The 15 findings of round 1 are closed; 5 warnings and 2 notes are open.

needs-revision

Round 2, parallel-safety, on the 14 draft stories at `238e8d64`. All 15 round-1 findings are closed. Six findings are open, all warnings, and two notes.

**Round-1 findings**

1. **`primary_kind` adjacent arms:** closed. `super-property-and-super-syntax.md:99` and `class-syntax-and-early-errors.md:93` name the adjacent arms and the fix, "keep both".
2. **Anchors in the `Expr` and `Instruction` tails:** closed. `NewTarget` goes after `This`, the super variants after `Call` and `DeleteIndex`, `Class` after `Function` (`new-target-meta-property.md:100`, `super-property-and-super-syntax.md:99`, `class-syntax-and-early-errors.md:93`). The `heap_bytes` sites are 2 lines apart in `crates/syntax/src/ast.rs:1013` and `:1016`, so they merge cleanly.
3. **Edge `class-definition-evaluation` to `super-property-and-super-syntax`:** closed. It is in the relations.
4. **Edge `derived-classes-and-super-call` to `class-element-syntax-and-early-errors`:** closed. It is in the relations.
5. **`ClassConstructor` with both slots:** closed. `class-definition-evaluation.md:73` makes it whole in `object/class_constructor.rs`.
6. **Owner of the static walk:** closed. There is no walk; `class-definition-evaluation` owns the step-4 sequence, and `class-static-blocks` now depends on `public-class-fields`.
7. **One `CompileError::Tree` arm per element kind:** closed. Each kind has its own stub function, and `class-element-syntax-and-early-errors.md:97` names the `PrivateMember` and `PrivateIn` arms against `class-definition-evaluation`.
8. **Shared paragraph of `derived-classes-and-super-call`:** closed. `derived-classes-and-super-call.md:103` names `resolve.rs`, `expression.rs` and the anchors in `bytecode.rs`, `disasm.rs` and `dispatch.rs`.
9. **`resolve.rs` missing from `public-class-fields` Writes:** closed. It is at `public-class-fields.md:71`.
10. **Edge from `instance-elements-initialization-order` to `class-static-blocks`:** closed. The folded story is mentioned only in its own file; no other story, the epic or `docs/plan/slice-08` names it.
11. **`KIND_SITES` and the layout tests in `private-names-and-private-methods`:** closed. It appends after `heap/mod.rs SymbolRecord::trace` and no longer writes `ordinary.rs` (`private-names-and-private-methods.md:114`).
12. **`ObjectRecord` 200 to 208 bytes against slice 9's tests that state 200:** closed. `private-names-and-private-methods` is the only story that moves the record, and it depends on `weak-map-and-weak-set`.
13. **HOST-222 against HOST-223:** closed. `host-ensure-can-add-private-element.md:51` takes HOST-223, and `HOST-223` appears only in that story, the close and the design.
14. **Edge to `host-object-overrides`:** closed. It is in the relations.
15. **Edge to `slice-09-conformance-close`:** closed. It is in the relations, with `slice-04-conformance-close` as well.

**Surfaces**

- **Placement:** 14 of 14 stories cited, 0 inferred, 0 unplaced. The new files are marked inferred in their scope.
- **Acceptance reads:** 42 traced to a producer, 38 inside the set and 4 outside (`weak-map-and-weak-set`, `host-object-overrides`, both closes); a `depends_on` edge records all 42. Two reads have no edge, both reported below.
- **Order 1:** `new-target-meta-property`, `super-property-and-super-syntax` and `class-syntax-and-early-errors` have named, distinct sites and named conflicts with their fixes, except the two findings below.
- **Order 2:** `class-definition-evaluation` and `class-element-syntax-and-early-errors` share only `parser/class.rs`, the `held` documentation, `compiler/src/{expression,resolve}.rs` and `compile.rs`. Their sites are named and distinct; in the `held` documentation they are one line apart.
- **Order 3:** the three stories have distinct sites in `bytecode.rs`, `disasm.rs`, `dispatch.rs`, `frame.rs` (lines 83 against 152 and 235), `class.rs` and `resolve.rs`. They name the adjacent `held` deletions and the fix. One adjacency is unnamed (finding 4).
- **Orders 4 to 7:** disjoint except as in findings 3 and 7 and the notes.

**Findings**

1. `new-target-meta-property` changes the eval entry `parse_eval(source, max, strict, m)` (`crates/syntax/src/parser/mod.rs:116`). Its `strict: bool` has to become the eval context so that `new.target` is admitted when `in_function` is set, yet its text names only `lib.rs` and the field in `parser/mod.rs`. `super-property-and-super-syntax` puts "the eval entry's refusal" in the same function, so the two edit lines 122 and 123 together and neither names it (inferred, since the story does not say where the parser receives the flag).
2. `early.rs` has no rule for `this` (the `target` function at `crates/syntax/src/early.rs:662` sends `this` to its `_` arm), so the site `new-target-meta-property` gives, "beside the rule for `this`", names nothing. `super-property-and-super-syntax.md:85` anchors on "the rule of `new.target`'s story", which is not on its base. Neither names where its hunk lands, and `super-property-and-super-syntax` also has to edit the `Expr::Member | Expr::Index` arm of `target` so that `super.x` is a valid target.
3. `class-static-blocks` has no edge to `derived-classes-and-super-call` or `private-names-and-private-methods`, but it shares three files with them: the `hold` calls in `parser/class.rs`, the adjacent deletions in the `held` documentation, and the call line in `resolve.rs` `function`. It anchors that call line "after the lines of `story:derived-classes-and-super-call`", which do not exist on a base where that story has not landed. The store builds as the edges allow, and the orders are not edges. Neither order-3 story names `class-static-blocks` (`derived-classes-and-super-call.md:103`, `private-names-and-private-methods.md:114`). It needs an edge to `derived-classes-and-super-call` (and one to `private-names-and-private-methods` if wanted), or the sites named in all three stories.
4. In `crates/compiler/src/expression.rs`, `class-element-syntax-and-early-errors` inserts the `PrivateMember` arm directly after the `SuperCall` arm (line 97). `derived-classes-and-super-call` replaces the `SuperCall` arm and `private-names-and-private-methods` replaces the `PrivateMember` arm. The two edits are adjacent lines, and neither body says so or gives the fix "keep both"; the Shared paragraphs name the arms but not the adjacency (`derived-classes-and-super-call.md:103`, `private-names-and-private-methods.md:114`).
5. `new-target-meta-property.md:104` states `ls crates/syntax/tests/sources/refused | wc -l` prints `15`, which is absolute. `class-syntax-and-early-errors` (3 more) and `class-element-syntax-and-early-errors` (2 more) state theirs relative to the unit's base, and `super-property-and-super-syntax` moves one fixture out and adds two. The `15` holds only on a base where those two siblings have not merged; it should read "1 fewer than the unit's base".
6. Notes: `eval-in-class-field-initializers.md:66` reads `pending slice 8$` as `1`, and `host-ensure-can-add-private-element` writes that count to 0. They have no edge, but the hook story needs `private-fields-and-private-in`, so only a late merge of the eval story could be affected. Also `host-ensure-can-add-private-element.md:70` says the close writes `website/data/status.json` again, while `slice-08-conformance-close.md:82` says that file is the coordinator's.

Out of my lane: the fallback chain for `RECORD_BYTES` (192, 200, 208) and the `SlotValue` fallback belong to the design critic, and the `--no-tests=fail` question belongs to the acceptance critic.

```findings
- file: .engineering/planning/story/new-target-meta-property.md
  line: 100
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "Inferred: it must change the eval entry parse_eval(source, max, strict, m) in crates/syntax/src/parser/mod.rs:116 (strict becomes the eval context, new_target set from in_function), but its Shared paragraph names only the field new_target there, while story:super-property-and-super-syntax puts its eval-entry refusal in the same function (lines 122-123), so the two hunks collide unnamed; both bodies must name the function and the fix keep both."
- file: .engineering/planning/story/new-target-meta-property.md
  line: 81
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "Its early.rs site 'beside the rule for this' names nothing that exists (crates/syntax/src/early.rs has no rule for this; target() at :662 sends it to the _ arm), and story:super-property-and-super-syntax line 85 anchors on 'the rule of new.target's story' which is not on its base and must also edit the Member|Index arm of target(); each body has to name its own existing site in early.rs."
- file: .engineering/planning/story/class-static-blocks.md
  line: 72
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "It has no depends_on on story:derived-classes-and-super-call or story:private-names-and-private-methods yet shares parser/class.rs hold calls, the adjacent held deletions and resolve.rs function with them, and anchors its call line in function 'after the lines of derived', which do not exist on a base without that story; neither order-3 story names it (derived line 103, private-methods line 114), so it needs the edge (or both edges) with the shared files as reason, or the sites named in all three."
- file: .engineering/planning/story/derived-classes-and-super-call.md
  line: 103
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "It replaces the Expr::SuperCall arm of compiler/src/expression.rs and story:private-names-and-private-methods (line 114) replaces the Expr::PrivateMember arm that story:class-element-syntax-and-early-errors (line 97) inserts directly after it, so the two edits are adjacent lines; neither Shared paragraph says so or gives the fix keep both arms."
- file: .engineering/planning/story/new-target-meta-property.md
  line: 104
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "Its acceptance states ls crates/syntax/tests/sources/refused | wc -l prints 15, an absolute count that fails once story:class-syntax-and-early-errors (3 more) or story:super-property-and-super-syntax (net 1 more) merges first, while the class stories state theirs relative to the unit's base; it should read 1 fewer than the unit's base."
- file: .engineering/planning/story/eval-in-class-field-initializers.md
  line: 66
  category: parallel-safety
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Its acceptance reads pending slice 8$ as 1 in conformance/host/requirements.txt, which story:host-ensure-can-add-private-element moves to 0, and no edge orders them (the hook story needs private-fields-and-private-in, so only a late merge of this story is affected); a git diff --quiet against the base alone would not depend on that order."
- file: .engineering/planning/story/host-ensure-can-add-private-element.md
  line: 70
  category: parallel-safety
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "It says story:slice-08-conformance-close writes website/data/status.json again, while that story (line 82) lists website/data/status.json as not written and the coordinator's; the two texts disagree on the writer."
```

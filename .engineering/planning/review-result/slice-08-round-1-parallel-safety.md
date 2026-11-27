---
format: aep.planning-md/3
id: review-result:slice-08-round-1-parallel-safety
kind: review-result
status: active
title: 'Slice 8, round 1, the parallel-safety critic: needs-revision, 15 findings'
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
revision: 1
---
Plan critic, parallel-safety, round 1 on the fifteen draft stories of `epic:slice-08-classes`
(`plan/slice-08` at `999e949e`), 2026-11-26.
Verdict: needs-revision, 15 findings. The critic's answer, as returned:

needs-revision

story:super-property-and-super-syntax — line 93 says it and story:class-syntax-and-early-errors each edit `primary_kind` "in hunks of its own", but the two arms they replace are next to each other (`crates/syntax/src/parser/expression.rs:1023` `Keyword::Class` and `:1024` `Keyword::Super`), so git gives a conflict whichever merges second; the body has to name that conflict and its fix (keep both arms), as it does for the `Parser` flags (cited, both bodies) — .engineering/planning/story/super-property-and-super-syntax.md:93, .engineering/planning/story/class-syntax-and-early-errors.md:89

story:new-target-meta-property — for its new `Expr` variant, the match arms over it and its `Instruction`, it says only "each its own variants and arms", with no anchor. The other two order-1 stories say the same. All three will append at the same last line of `Expr` (`ast.rs:434`), of `heap_bytes` (`ast.rs:1028`), of the children walk (`ast.rs:1239`), of `compiler/src/expression.rs:681`, of the `resolve.rs` walk (ends at `:1317`), of `Instruction` (`bytecode.rs:745`), of the `disasm.rs:349` match and of the `execute` match (`dispatch.rs:459`). Each body has to name its anchor (for example `NewTarget` beside `This`, `SuperMember` and `SuperCall` beside `Member` and `Call`, `Class` beside `Function`) or state the fix "keep every arm" (cited) — .engineering/planning/story/new-target-meta-property.md:93

story:class-definition-evaluation — it has no `depends_on` on story:super-property-and-super-syntax, so a wave run "as the edges allow" can build it beside that story. They share `compiler/src/{bytecode,disasm,expression,resolve}.rs`, `interp/dispatch.rs` and `interp/mod.rs`, and its Shared paragraph names only story:class-element-syntax-and-early-errors. It needs an edge with the shared files as the reason, or the hunks named (cited, both bodies and the store's collisions) — .engineering/planning/story/class-definition-evaluation.md:10, :107

story:derived-classes-and-super-call — it has no `depends_on` on story:class-element-syntax-and-early-errors. Both land on `parser/class.rs`, `parser/expression.rs`, the documentation of `held` in `parser/mod.rs`, `early.rs` and `compiler/src/expression.rs`, and neither body names the other. Its SuperCall early error "in a field initializer, a static block" also reads contexts that only that story creates. It needs the edge (cited, both bodies) — .engineering/planning/story/derived-classes-and-super-call.md:10, :69, .engineering/planning/story/class-element-syntax-and-early-errors.md:90

story:class-definition-evaluation — line 73 has its seam reading the two lists of instance elements (empty), but line 107 has [[Fields]], [[PrivateMethods]] and the constructor kind added to the box later, by story:public-class-fields, story:private-names-and-private-methods and story:derived-classes-and-super-call. So it is open whether those three order-3 stories each add a field to one new struct in `ecmascript_function.rs`. Declaring all four slots of design change 1 here, empty, takes that file out of order 3 (cited) — .engineering/planning/story/class-definition-evaluation.md:73, :107

story:class-static-blocks — it and story:public-class-fields (line 67) both place their element "in the list of static elements" in `compiler/src/class.rs` and run it in the static walk of `interp/class.rs`. No story creates that list or walk: story:class-definition-evaluation defines static methods directly (line 94). So two stories of one order would each create one structure. One story has to own it: story:class-definition-evaluation creates it empty, or one of the two creates it and the other gets an edge (cited) — .engineering/planning/story/class-static-blocks.md:58, :59, .engineering/planning/story/public-class-fields.md:67

story:class-element-syntax-and-early-errors — its rule "the second to merge adds the arms of the new kinds as `CompileError::Tree`" does not ask for one arm per element kind. Three order-3 stories replace those arms in `compiler/src/class.rs`, so one `Field | StaticBlock | PrivateMethod => Err(..)` arm turns them into edits of one line. Its `Expr::PrivateMember` and `Expr::PrivateIn` arms in `compiler/src/{expression,statement}.rs` also share those files with story:class-definition-evaluation, and neither body names that (cited) — .engineering/planning/story/class-element-syntax-and-early-errors.md:77, :90

story:derived-classes-and-super-call — its Shared paragraph leaves out `compiler/src/resolve.rs` and `compiler/src/expression.rs` (the `Expr::This` arm), which it shares with story:class-static-blocks and story:private-names-and-private-methods (named only on their side). It gives no anchor for its instructions and arms in `bytecode.rs`, `disasm.rs` and `dispatch.rs`, which story:public-class-fields and story:private-names-and-private-methods also append (cited) — .engineering/planning/story/derived-classes-and-super-call.md:101, .engineering/planning/story/class-static-blocks.md:71, .engineering/planning/story/private-names-and-private-methods.md:99

story:public-class-fields — it keeps computed field keys "in a slot of the class scope", which lives in `compiler/src/resolve.rs` (story:class-definition-evaluation line 94), but its Writes line leaves `resolve.rs` out. That would make an unnamed fourth order-3 writer of that file beside story:derived-classes-and-super-call, story:class-static-blocks and story:private-names-and-private-methods (inferred) — .engineering/planning/story/public-class-fields.md:66, :77

story:instance-elements-initialization-order — it has no `depends_on` on story:class-static-blocks, yet both write `compiler/src/class.rs` and `interp/class.rs`. It calls itself "the only story of order 4" without naming that story. Under an edge-driven start it, and story:eval-in-class-field-initializers through it, can run while static blocks are still being built. It needs the edge or the hunks named (seam against static walk) (cited, both bodies) — .engineering/planning/story/instance-elements-initialization-order.md:10, :61

story:private-names-and-private-methods — it adds a kind site but names neither `KIND_SITES: [&str; 21]` (`heap/collect.rs:351`, whose length line every adding story changes) nor the layout test. It writes all of `crates/swell/src/heap/` and an unnamed hunk of `object/ordinary.rs`. In the same wave, slice 9's map-and-set, proxy-objects-and-access-traps and weak-map-and-weak-set append to `KIND_SITES` and change `ordinary.rs`. The body has to name those stories, its hunks, and the rule that the second to merge sets the length (cited, slice 9 names it on its side) — .engineering/planning/story/private-names-and-private-methods.md:87, :97, `git show plan/slice-09:.engineering/planning/story/weak-map-and-weak-set.md` line 76

story:private-names-and-private-methods — it grows `ObjectRecord` from 200 to 208 bytes and moves the size test. `RECORD_BYTES = 200` stands in both `object/generator.rs:62` and `object/async_generator.rs:135`. Slice 9's keyed-entry-table, map-and-set, proxy-objects-and-access-traps and weak-map-and-weak-set state "200 bytes, passes as it is" in the same wave, and no edge orders them. The body needs an edge, either direction, with the record size as the reason, or the slice-9 texts have to read "its size on the base" (cited) — .engineering/planning/story/private-names-and-private-methods.md:71, `git show plan/slice-09:.engineering/planning/story/map-and-set.md` line 68

story:host-ensure-can-add-private-element — it names HOST-222 and `host_222_*`, but slice 9's host-object-overrides already takes HOST-222 (design change `HOST-222 pending slice 9`, closed by that story). Two stories in one wave would write one id into `conformance/host/requirements.txt` and `docs/design/host-interface.md`. The body's fallback "unless slice 9 takes it first" has come true, so it has to name the next free number now. It also shares `CHANGELOG.md` and the generated `website/data/status.json` with that story (cited) — .engineering/planning/story/host-ensure-can-add-private-element.md:36, :56, :57, `git show plan/slice-09:.engineering/planning/story/host-object-overrides.md` lines 117, 123

story:host-ensure-can-add-private-element — its test builds a host object with an override table, which story:host-object-overrides (slice 9) produces, and its `relations` hold no `depends_on` to that story. The coordinator's Q5 edge is not recorded yet (cited) — .engineering/planning/story/host-ensure-can-add-private-element.md:10, :56

story:slice-08-conformance-close — its acceptance reads the 38 cases of `close-slice-9.cases.txt` passing, and it writes the same five files as story:slice-09-conformance-close. Its `relations` hold no edge to that story; the coordinator's Q6 edge is not recorded yet (cited) — .engineering/planning/story/slice-08-conformance-close.md:24, :81

### The decomposer's shared files

| File | Disjoint? | What is missing |
|---|---|---|
| `compiler/src/expression.rs` | Order 1: no anchor, all three append after `Expr::Await` (`:681`). Order 2: unnamed collision. Order 3: derived (`:258`) and private-methods (`PrivateMember`) can be disjoint. | Anchors; order-2 naming |
| `compiler/src/function.rs` | No story writes it (I think the decomposer meant `parser/function.rs`) | none |
| `syntax/src/parser/function.rs` | Order 1: new-target and super add flags to the same lines of `save_context`, `open_context` and `function_close`. Named, fix "keep both". | none |
| `compiler/src/bytecode.rs`, `disasm.rs`, `interp/dispatch.rs` | Order 1 (new-target, super) and order 3 (derived, public-fields, private-methods) append at one tail. No opcode numbers: `Instruction` has no discriminants (`bytecode.rs:61`), and arms without operands fall to `other => {other:?}` in `disasm.rs`. | An anchor or "keep every arm" |
| `parser/class.rs` (new) | Created only by class-syntax. Order 2 and order 3 lift one hold line each; named, fix "keep the other removals". | Order 3: derived against element-syntax needs the edge |
| `interp/class.rs` (new) | Created only by class-definition-evaluation. Order 3 seam loops named (methods first). | The static walk has no owner. Static-blocks against instance-elements and eval-in needs an edge. |
| `tests/main.rs` | Generated in sorted order; `class_fields` and `class_static_blocks` sit next to each other | Fix by `sh taskfiles/test-main.sh write`; named everywhere |

No two stories of one order create the same new file.

Slice 9 collisions:
- **`object/mod.rs`:** disjoint if private-methods keeps to the struct field, `new`, one trace line before the kind match, and the `private_names` mod line between lines 47 and 48.
- **`heap/collect.rs`:** collides at the `KIND_SITES` length line and the layout test.
- **`object/builtin_function.rs`:** no slice-8 story writes it.
- **`builtins/function.rs`:** `function_to_string` (`:276`, class-definition-evaluation and private-methods) against line 200 (reflect-namespace, proxy-descriptor) is disjoint.
- **`tests/main.rs`:** regenerate.
- **Outside the four files you listed:** HOST-222, `requirements.txt`, `host-interface.md` and the size test, as in the findings.

### Merge order the coordinator needs

1. **Order 1:** any order. The second and third to merge resolve:
   - `primary_kind` 1023/1024 (keep both arms)
   - the `Parser` fields at lines 79–98, their init and release, and the three function-boundary functions (keep both)
   - `hold` (keep one) and the `held` documentation (keep both sentences)
   - the `Expr` and `Instruction` tails (keep all)
2. **Order 2:** class-definition-evaluation only after super has merged (edge). element-syntax in either order; the second adds the new element kinds as one `CompileError::Tree` arm each.
3. **Order 3:**
   - derived only after element-syntax (edge).
   - whoever owns the static walk merges before the other of public-fields and static-blocks.
   - private-methods before public-fields, or the second keeps methods first.
   - private-methods against slice 9's kind stories: either order, but the second re-measures the record size and sets the `KIND_SITES` length.
4. **Order 4:** after order 3 including static-blocks.
5. **Order 5:** either order.
6. **Order 6:** host-ensure after slice 9's host-object-overrides, with its own HOST number.
7. **Order 7:** after slice-09-conformance-close.

The store's `waves` puts all fifteen in waves 1–15, one story each, because it judges by whole files. Building orders 1, 2, 3 and 5 at the same time depends on the hunk claims above being fixed.

### What I read
- **Stories:** all 15 bodies in full.
- **Commands:** `aep plan artifact waves --kind story --status draft --format json` (collisions, cycles, unassessed), the brief, `waves-section.md`, and `git show plan/slice-09:` for the 10 slice-9 stories.
- **Code:** `git grep`/`sed` on `parser/{mod,expression,function,statement}.rs`, `lexer.rs:373`, `syntax/src/{ast,early,lib}.rs`, `compiler/src/{bytecode,disasm,expression,resolve,lib}.rs`, `interp/{dispatch,mod}.rs`, `object/{mod,ecmascript_function,ordinary,generator,async_generator}.rs`, `heap/{collect,arena}.rs`, `dynamic_code.rs`, `script.rs`, `env/global.rs`, `builtins/function.rs`, `host/hooks.rs`, `taskfiles/test-main.sh`. Every line number the stories cite matched the tree.
- **Surfaces:** 15 cited, 0 inferred, 0 unplaced. One finding (public-fields and `resolve.rs`) rests on an inferred surface.
- **Acceptance reads:** 16 traced to a producer. 13 are recorded by a `depends_on` edge (direct or transitive). 3 are not: derived to element-syntax, host-ensure to slice-9 host-object-overrides, the close to slice-09-conformance-close.

### What I could not establish
- Where order-2 implementors put the hold lines and the `CompileError::Tree` arms in `parser/class.rs` and `compiler/src/class.rs`. Neither file exists yet, so the order-3 disjointness depends on them.
- Whether slice 9's Proxy and Collection kinds keep `ObjectKind` at its size once `[[PrivateElements]]` lands.
- Outside my lane (acceptance): private-methods' Writes line leaves out `object/async_generator.rs`, whose size test also asserts 200 (`:135`). Public-fields' new define instruction may clash with the existing `Instruction::DefineField` (`disasm.rs:288`).
- Outside my lane (design): new-target line 93 says class-syntax writes `parser/function.rs`, while class-syntax line 87 says it does not.

```findings
- file: .engineering/planning/story/super-property-and-super-syntax.md
  line: 93
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Line 93 says it and story:class-syntax-and-early-errors each edit primary_kind in hunks of their own, but the two arms they replace are next to each other (crates/syntax/src/parser/expression.rs:1023 Keyword::Class, :1024 Keyword::Super), so git gives a conflict whichever merges second; the body has to name that conflict and its fix (keep both arms)."
- file: .engineering/planning/story/new-target-meta-property.md
  line: 93
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "For its new Expr variant, the match arms over it and its Instruction, it (like the other two order-1 stories) gives no anchor, so all three append at the same last line of Expr (ast.rs:434, 1028, 1239), compiler expression.rs:681, the resolve.rs walk, bytecode.rs:745, disasm.rs:349 and dispatch.rs:459; each body has to name its anchor or state the fix keep every arm."
- file: .engineering/planning/story/class-definition-evaluation.md
  line: 10
  category: parallel-safety
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "It has no depends_on on story:super-property-and-super-syntax, so a wave run as the edges allow can build it beside that story on compiler bytecode.rs, disasm.rs, expression.rs, resolve.rs, interp/dispatch.rs and interp/mod.rs, while its Shared paragraph (line 107) names only story:class-element-syntax-and-early-errors; it needs an edge with the shared files as the reason, or the hunks named."
- file: .engineering/planning/story/derived-classes-and-super-call.md
  line: 10
  category: parallel-safety
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "It has no depends_on on story:class-element-syntax-and-early-errors, yet both land on parser/class.rs, parser/expression.rs, the held documentation in parser/mod.rs, early.rs and compiler/src/expression.rs and neither body names the other, and its SuperCall early error in field initializers and static blocks (line 69) reads contexts only that story creates; it needs the edge."
- file: .engineering/planning/story/class-definition-evaluation.md
  line: 73
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Line 73 has the seam read two empty lists of instance elements, but line 107 has [[Fields]], [[PrivateMethods]] and the constructor kind added to the box later by three order-3 stories, so it is open whether they each add a field to one new struct in ecmascript_function.rs; declaring all four box slots here, empty, takes that file out of order 3."
- file: .engineering/planning/story/class-static-blocks.md
  line: 58
  category: parallel-safety
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "It and story:public-class-fields (line 67) both place their element in a list of static elements in compiler/src/class.rs and run it in the static walk of interp/class.rs, which no story creates (story:class-definition-evaluation line 94 defines static methods directly), so two stories of one order would each create one structure; one story has to own it."
- file: .engineering/planning/story/class-element-syntax-and-early-errors.md
  line: 90
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Its rule that the second to merge adds the new element kinds as CompileError::Tree does not ask for one arm per kind, so the three order-3 stories that replace those arms in compiler/src/class.rs can end up editing one or-pattern line, and its PrivateMember and PrivateIn arms in compiler/src/expression.rs and statement.rs, shared with story:class-definition-evaluation, are named by neither body."
- file: .engineering/planning/story/derived-classes-and-super-call.md
  line: 101
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Its Shared paragraph leaves out compiler/src/resolve.rs and compiler/src/expression.rs, which it shares with story:class-static-blocks and story:private-names-and-private-methods (named only on their side), and gives no anchor for its instructions and arms in bytecode.rs, disasm.rs and dispatch.rs, which story:public-class-fields and story:private-names-and-private-methods also append."
- file: .engineering/planning/story/public-class-fields.md
  line: 77
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Inferred: it keeps computed field keys in a slot of the class scope (line 66), which lives in compiler/src/resolve.rs, but its Writes line leaves resolve.rs out, which would make an unnamed fourth order-3 writer of that file."
- file: .engineering/planning/story/instance-elements-initialization-order.md
  line: 10
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "It has no depends_on on story:class-static-blocks, yet both write compiler/src/class.rs and interp/class.rs and line 61 calls it the only story of order 4, so under an edge-driven start it, and story:eval-in-class-field-initializers through it, can run beside static blocks; it needs the edge or the hunks named."
- file: .engineering/planning/story/private-names-and-private-methods.md
  line: 97
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "It adds a kind site but names neither the KIND_SITES length line (heap/collect.rs:351, [&str; 21]) nor the layout test, writes all of crates/swell/src/heap/ and an unnamed hunk of object/ordinary.rs, and names none of slice 9's map-and-set, proxy-objects-and-access-traps or weak-map-and-weak-set, which change the same lines in the same wave."
- file: .engineering/planning/story/private-names-and-private-methods.md
  line: 71
  category: parallel-safety
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "It grows ObjectRecord from 200 to 208 bytes (RECORD_BYTES in object/generator.rs:62 and object/async_generator.rs:135) while slice 9's keyed-entry-table, map-and-set, proxy-objects-and-access-traps and weak-map-and-weak-set state that 200 bytes passes as it is in the same wave, and no edge orders them."
- file: .engineering/planning/story/host-ensure-can-add-private-element.md
  line: 36
  category: parallel-safety
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "It names HOST-222 and host_222_* (lines 36, 56, 57), which slice 9's host-object-overrides already takes in requirements.txt and host-interface.md in the same wave, so it has to name the next free number now and name that story as sharing CHANGELOG.md and website/data/status.json."
- file: .engineering/planning/story/host-ensure-can-add-private-element.md
  line: 10
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Its test builds a host object with an override table (line 56), which slice 9's story:host-object-overrides produces, and its relations hold no depends_on to that story; the coordinator's Q5 edge is not recorded yet."
- file: .engineering/planning/story/slice-08-conformance-close.md
  line: 24
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Its acceptance reads the 38 cases of close-slice-9.cases.txt passing and it writes the same five files as story:slice-09-conformance-close, but its relations hold no edge to that story; the coordinator's Q6 edge is not recorded yet."
```

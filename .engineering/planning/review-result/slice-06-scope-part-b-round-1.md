---
format: aep.planning-md/3
id: review-result:slice-06-scope-part-b-round-1
kind: review-result
status: active
title: scope critic, ten later stories of slice 6, round 1
relations:
- reviews: story:computed-names-and-object-spread
- reviews: story:block-function-declarations
- reviews: story:symbol-constructor-and-well-known-symbols
- reviews: story:symbol-values-in-core-builtins
- reviews: story:iterator-protocol-and-array-string-iterators
- reviews: story:for-of-statement
- reviews: story:array-destructuring-and-spread
- reviews: story:array-from-and-object-from-entries
- reviews: story:tagged-templates
- reviews: story:slice-06-conformance-close
revision: 1
---
needs-revision

story:array-from-and-object-from-entries — the Change gives `Array.from` and `Object.groupBy` each "a step per element" and a `*_charges_a_step_per_element` test, but says nothing about charging `Object.fromEntries`, the third loop over an iterator, and gives it no charge test, so the epic's "Every loop, including `for-of` over an iterator and spread, is charged to the step budget" has no owner for that builtin; the change is to add a step per entry and a `from_entries_charges_a_step_per_entry` test in the same form as the other two — .engineering/planning/story/array-from-and-object-from-entries.md:50 (promise at .engineering/planning/epic/slice-06-bindings-and-iteration.md:102 and :32)

What I read: the epic and all 10 stories in full, plus the 5 part-A stories as context (`aep plan artifact show`), `aep plan artifact graph`, `review-result:slice-06-scope-round-1`, `story:slice-03-conformance-close`, the slice-3 stories that hand work to slice 6, AGENTS.md "How the work is run", waves.md "The order after slice 3", engine.md section 12 (end of the block), and the base's parser refusals. I also checked the case lists under `docs/plan/slice-06/` against `.out/test262/runs.tsv`:
- Together with `move-estimate.tsv` and `cross-story.tsv`, the 15 lists plus `close-remainder.cases.txt` hold all 4,171 failing cases, with no case on two lists. That is 3,446 on lists, 673 to move and 52 in the remainder.
- The other 15 failing cases in the six moved directories are of slices 7, 9 and 13.

Promises: I extracted 24 from the epic and traced 24 to an item.
- Of these, 9 are carried by part A.
- Acceptance 3's `AGENTS.md`/`README.md` numbers are handed to the coordinator by name in the close's Out of scope.
- The "Open" question on unassigned features is answered by `docs/design/engine.md:2532`.
- The `Symbol` handle and symbol keys exist on the base.
- I found no reach beyond the epic: block functions, `Array.from`, `Object.fromEntries` and `Object.groupBy`, and the Symbol defects in slice-3 builtins are all cases of slice 6, the last three by the ruling recorded in `story:slice-03-conformance-close`.
- I found no outcome claimed twice.

What I could not establish:
- Whether the iterator-record design will charge a step per `next` that a builtin calls on another builtin. `story:iterator-protocol-and-array-string-iterators` (Change, `ops/iteration.rs`) says only "a step for each call it makes to script code", while its precondition says "for each `next` a builtin or an instruction calls". Out of my lane (design).
- Story order for the consumer's first parse: `story:for-of-statement` waits on `story:block-function-declarations` only because they share files. That costs time only if the BlockDeclarationInstantiation design is late, since every path to the parse already waits for slice 3's close. `story:symbol-constructor-and-well-known-symbols` does not say why it depends on `story:computed-names-and-object-spread`. Out of my lane (parallel-safety, design).
- Acceptance check (b) of `story:symbol-values-in-core-builtins` measures what `story:symbol-constructor-and-well-known-symbols` already delivers. The four builtin stories add no instruction and do not say "nothing is added to the frame of `Agent::execute`". Out of my lane (acceptance).
- I removed the scratch directory `.out/critic-scope/`. The one untracked path in the tree, `.engineering/evidence/story/string-transform-and-compare-methods/`, was not written by me.

```findings
- file: .engineering/planning/story/array-from-and-object-from-entries.md
  line: 50
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Change gives `Array.from` and `Object.groupBy` each \"a step per element\" and a `*_charges_a_step_per_element` test, but says nothing about charging `Object.fromEntries`, the third loop over an iterator, and gives it no charge test, so the epic's \"Every loop, including `for-of` over an iterator and spread, is charged to the step budget\" has no owner for that builtin; the change is to add a step per entry and a `from_entries_charges_a_step_per_entry` test in the same form as the other two"
```

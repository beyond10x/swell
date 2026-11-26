---
format: aep.planning-md/3
id: review-result:slice-08-round-2-scope
kind: review-result
status: active
title: 'Slice 8, round 2, scope: needs-revision, 1 warning and 2 notes'
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
Plan critic, scope, round 2 on the fourteen draft stories of
`epic:slice-08-classes` (`plan/slice-08` at `238e8d64`), 2026-11-26.
Verdict: needs-revision. The 3 findings of round 1 are closed; 1 warning and 2 notes are open.

needs-revision

Scope critic, round 2, on the fourteen draft stories of `epic:slice-08-classes` (`plan/slice-08` at `238e8d64`). I wrote nothing and ran no build or test. The one command I ran is `lists.sh`, which only reads. One warning and two notes remain.

## Round 1, finding by finding

1. **Closed: the 4 `__lookupGetter__` / `__lookupSetter__` cases.**
   - `docs/plan/slice-08/close-later.tsv` lines 1-4 hold them with slice 18 and the parts `B PM L18`.
   - `docs/plan/slice-08/scan.awk` now reads `__(lookup|define)(Getter|Setter)__` and sends such a case to `later 18`. `scan.tsv` has `later 18` 4 times, and the private-methods list is 1,977.
   - The close story agrees at lines 46, 70 and 94 (`13 5`, `18 4`, `all 9`).
2. **Closed: the 16 cases that subclass Map, Set, WeakMap or WeakSet.**
   - `scan.awk`'s L9 rule now matches `extends (Weak)?(Map|Set)` and the features Map, Set, WeakMap, WeakSet, Proxy, Reflect.
   - All 16 are in `close-slice-9.cases.txt` (91 lines: 54 of slice 8 and the 37 of `incoming.tsv`). None is on a story list.
   - The epic's out-of-scope paragraph (lines 71-72) names them. `derived-classes-and-super-call.md:72` and `:113` state the move.
3. **Closed: the double claim of `instance-elements-initialization-order`.**
   - The story is archived and no other story or list names it. Its cases are on the private-fields list (446) and the public-fields list (2).
   - The seam sentences now sit with one owner each: `class-definition-evaluation.md:84,106`, `derived-classes-and-super-call.md:68`, and `private-fields-and-private-in.md:55-57` for the combined cases only.

## The partition, recomputed

- `sh docs/plan/slice-08/lists.sh runs-all.tsv` exits 0 with no `twice`, `missing` or `outside`. Its last line is `slice 9302 lists 9339 incoming 37`.
- The `scan.tsv` keys match the list files: 3,297 + 467 + 331 + 886 + 84 + 107 + 36 + 1,431 + 1,977 + 517 + 35 + 66 = 9,234 on story lists, plus 5 + 54 + 9 = 68 on close lists, which is 9,302.
- The run counts add up to 18,455: the story lists give 18,319 runs, `close-pass` 10, `close-later` 18, and `close-slice-9` 108.
- 9,302 + 37 - 9 = 9,330 cases and 18,455 + 74 - 18 = 18,511 runs, as in the epic and the close.
- The scan's parts line up with the edges. `eval-fields` cases carry only `B C D EV`-type parts and `eval-private` only `B C D PM PF EV`-type parts, and each story depends on the stories those parts need.

## The epic's promises

I wrote the list from the epic before reading any story.

- **Promises extracted:** 24.
- **Traced to a story:** 22.
- **Named out by the drafter or the epic:** 2.

Traced:
- Classes, `super`, `new.target`, static blocks.
- Public, private, static public and static private fields.
- Private methods and static private methods.
- `#x in obj`.
- Private names not reachable from outside (invisibility tests in the private-methods and private-fields stories).
- HOST-119 and HOST-223 (the hook story's first commit).
- The parser's early errors for class bodies (class-syntax and element-syntax).
- The private-name storage in section 3 (private-methods).
- The 11 feature lines and 5 directory lines (the lists).
- The 5 cases to slice 13 and 4 to slice 18.
- The 37 cases coming in and the 54 on `close-slice-9`.
- Acceptance 1, 2 and 4 (close checks 1 to 7).
- The step-budget, memory-budget and clause-citation constraints (reduced-count tests per story).
- All seven holds, each lifted by exactly one story: `class`, `class heritage`, `class field`, `private method`, `private field`, `private name in`, `class static block`.

Not claimed by a story, but named out:
- The `AGENTS.md` and `README.md` numbers of acceptance 3. The close hands them to the coordinator (`slice-08-conformance-close.md:74,100`).
- The epic's own counts, which the coordinator rewrites.

Nothing drafted lands in an exclusion: decorators and `nonextensible-applies-to-private` stay excluded in `features.txt`, and no story touches Temporal, ECMA-402 or agents. I found no outcome claimed twice. That includes the methods-before-fields order and the seam after `super()`, which round 1 flagged.

## What is still open

1. **Warning: nobody owns the read of PerformEval's `inClassFieldInitializer` from the `FieldInitializer` template kind.**
   - `new-target-meta-property.md:61` says "each flag is computed as item 5 states, so none is a stand-in". Line 82 says the flag is "the template's kind is `FieldInitializer`, false until `story:class-syntax-and-early-errors` adds that kind".
   - That story is built beside this one with no edge, so this story cannot name the kind.
   - `class-syntax-and-early-errors.md:91` writes nothing under `crates/swell`.
   - `class-definition-evaluation.md:106` edits `dynamic_code.rs` for `inDerivedConstructor` only.
   - `eval-in-class-field-initializers.md:29,60` treats the derivation as already done and writes `dynamic_code.rs` only if it finds it wrong.
   - So the outcome "eval in a field initializer sees `inClassFieldInitializer` true" (13 tests, 84 cases) has no story that writes the derivation.
   - Fix: add the one derivation line in `dynamic_code.rs` to `eval-in-class-field-initializers` (name it in Change and Writes). Alternatively `class-definition-evaluation` takes it, since it is after `class-syntax-and-early-errors` and already writes that file. Then correct line 61 and line 82 of the new-target story.
2. **Note: the close story's title and summary are stale.**
   - `slice-08-conformance-close.md:6-7` read "5 cases move to slice 13" and "38 slice-9 and 5 passing cases checked". The body moves 9 cases (5 to slice 13, 4 to slice 18) and checks 91.
   - Context line 44 says "9,234 on the 12 story lists and four close lists". 9,234 is the sum of the story lists alone.
3. **Note: the two stories disagree on who writes `website/data/status.json`.**
   - `host-ensure-can-add-private-element.md:70` says the close "writes `website/data/status.json` again".
   - The close story (lines 74, 82, 100) leaves that file to the coordinator.
   - This is shared-file ownership, so it belongs to parallel safety. I list it only because it is a claim in my set that contradicts another.

Out of my lane, and not counted here:
- Whether the pinned cargo-nextest accepts `--no-tests=fail`.
- Sizes (200 to 208 bytes, `SlotValue`, the `constructed` enum). Each story names its layout test or fallback.
- The `static-blocks` and `eval-fields` stories each defer eval inside a static block to the other (`class-static-blocks.md:83`, `eval-in-class-field-initializers.md:72`). No case on either list needs it, and `class-static-blocks` has a test for it, so I left it alone.

Files for the findings: `.engineering/planning/story/new-target-meta-property.md`, `.engineering/planning/story/eval-in-class-field-initializers.md`, `.engineering/planning/story/slice-08-conformance-close.md`, `.engineering/planning/story/host-ensure-can-add-private-element.md`.

Verdict: needs-revision

```findings
- file: .engineering/planning/story/new-target-meta-property.md
  line: 82
  category: scope
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Line 61 says every PerformEval flag is computed as item 5 states, but line 82 makes inClassFieldInitializer false until class-syntax-and-early-errors adds the FieldInitializer kind, a story built beside this one with no edge, and no story writes the read of that kind in dynamic_code.rs (class-syntax Writes excludes crates/swell, class-definition-evaluation:106 does inDerivedConstructor only, eval-in-class-field-initializers:29,60 assumes it is done), so the 84 eval-fields cases and their 13 tests have no owner for it; add the one derivation line to eval-in-class-field-initializers (Change and Writes) or to class-definition-evaluation, and correct lines 61 and 82."
- file: .engineering/planning/story/slice-08-conformance-close.md
  line: 6
  category: scope
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Title and summary (lines 6-7) say 5 cases move to slice 13 and 38 slice-9 cases are checked, while the body moves 9 (5 to slice 13, 4 to slice 18) and checks 91, and line 44 gives 9,234 as the story lists plus close lists when it is the 12 story lists alone."
- file: .engineering/planning/story/host-ensure-can-add-private-element.md
  line: 70
  category: scope
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "This line says the close writes website/data/status.json again, but slice-08-conformance-close.md lines 74, 82 and 100 leave that file to the coordinator; state one owner."
```

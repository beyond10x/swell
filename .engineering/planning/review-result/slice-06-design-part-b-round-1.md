---
format: aep.planning-md/3
id: review-result:slice-06-design-part-b-round-1
kind: review-result
status: active
title: design critic, ten later stories of slice 6, round 1
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

story:iterator-protocol-and-array-string-iterators — it names `story:for-of-statement` as the first caller of every operation it marks dead, but for-of's Change never says it steps through IteratorStepValue. Array patterns and `Array.from` step through that operation, their stories may run beside each other, and neither may write `ops/iteration.rs`. Under `-D warnings` the unfulfilled `expect(dead_code)` then fails `cargo clippy` for whichever of the two lands first. The fix is one of two: for-of's Change names IteratorStepValue as the operation its step instruction calls, or the mark on IteratorStepValue names a story that is allowed to remove it. — .engineering/planning/story/iterator-protocol-and-array-string-iterators.md:63, .engineering/planning/story/for-of-statement.md:75, .engineering/planning/story/array-from-and-object-from-entries.md:49, .engineering/planning/story/array-from-and-object-from-entries.md:51, .engineering/planning/story/array-destructuring-and-spread.md:97, .engineering/planning/story/array-destructuring-and-spread.md:103, Taskfile.yml:222

What I read: the 10 stories in full and the epic, using `aep plan artifact show`, `aep plan artifact relations`, `aep plan artifact graph` and `aep plan artifact validate`; also `docs/plan/slice-06/lists-b.awk` and `cross-story.tsv`, the `expect(dead_code` marks in `crates/`, and the clippy lint settings in `Taskfile.yml`.
- **Edges:** I walked 16 `depends_on` edges inside the set and followed them out to the 5 part-A stories, `story:slice-03-conformance-close` and its builtin stories. There is no cycle.
- **Acceptance reads:** I traced 10 to the story that produces what they read. All 10 are covered by an edge, directly or through other edges. Examples: `HOST-156` and `sections.txt` line 93 in the close story, `Symbol is not defined` reaching 0 in `symbol-values`, `Object.entries` and `String.raw` from slice 3, `Symbol.iterator` in the iterator tests.
- **Lists:** the closure each story gets in `lists-b.awk` is a subset of what its edges reach, so no list counts a case on a part its story does not depend on.
- **Consumer test:** `story:array-destructuring-and-spread` waits on `slice-03-close`, then `symbol`, then `iterator`, then `for-of`. Every step of that chain says why it is needed.
- **Chain:** `computed` to `block-functions` to `for-of` to `array-destructuring` to `tagged-templates` is one chain, but each edge says it is there because the stories write the same files in `crates/syntax` and `crates/compiler`. So it is a trade-off, not a finding.

What I could not establish:
- Whether `story:block-function-declarations` holds up `for-of` if the BlockDeclarationInstantiation design is late. Its only reason to come first is the shared files. This is a scheduling worry, not a finding.
- With the mark problem fixed, `story:array-from-and-object-from-entries` would need `story:for-of-statement` only for the marks, and that edge could go. This is a trade-off for the drafter.
- Outside my lane:
  - Scope: whether the 15 stories cover every promise of the epic, including its open question about the 39 features.
  - Acceptance: whether the plain or the `cfg_attr(not(test), …)` form of the mark survives the `--lib` unit tests.
  - Parallel safety: `ops/conversion.rs`, which `symbol` and `symbol-values` both write; an edge already orders them.

`validate` prints `valid`; its only notes are two active stories outside the set that have no scope. I made no scratch directory.

```findings
- file: .engineering/planning/story/iterator-protocol-and-array-string-iterators.md
  line: 63
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it names story:for-of-statement as the first caller of every operation it marks dead, but for-of's Change never says it steps through IteratorStepValue; array patterns and Array.from step through it, may run beside each other and may not write ops/iteration.rs, so the unfulfilled expect(dead_code) fails cargo clippy under -D warnings for whichever lands first; either for-of names IteratorStepValue as its step or the mark on IteratorStepValue names a story allowed to remove it"
```

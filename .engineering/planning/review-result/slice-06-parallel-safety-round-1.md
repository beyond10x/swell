---
format: aep.planning-md/3
id: review-result:slice-06-parallel-safety-round-1
kind: review-result
status: active
title: parallel-safety critic, first five stories of slice 6, round 1
relations:
- reviews: story:let-const-and-block-scopes
- reviews: story:template-literals
- reviews: story:arrow-functions
- reviews: story:default-and-rest-parameters
- reviews: story:object-destructuring
revision: 1
---
needs-revision
story:template-literals — `Expr::Template` cannot compile without an arm in `crates/compiler/src/resolve.rs`, because the walk in `Resolver::expr` matches every `Expr` variant with no wildcard (lines 498 to 596) and the substitutions must be walked to resolve their names; yet resolve.rs is missing from this story's `Writes:` line and from its "Shared with the story beside it" list, while story:let-const-and-block-scopes rewrites the same file (cited, both bodies, a scope per block). Fix it with an ordering edge that records resolve.rs as its reason, or by adding resolve.rs to the Writes line and the shared list. As written, the unit stops at its first compile (AGENTS.md:324-326) — .engineering/planning/story/template-literals.md:81, crates/compiler/src/resolve.rs:498
story:let-const-and-block-scopes — its list of files shared with story:template-literals names six files but leaves out `crates/compiler/src/resolve.rs`, which both stories have to write (that one for the `Expr::Template` arm of the exhaustive match in `Resolver::expr`). The unit that lands second therefore has a merge overlap in resolve.rs that neither body mentions — .engineering/planning/story/let-const-and-block-scopes.md:123, crates/compiler/src/resolve.rs:498

What I read: the 5 stories with `aep plan artifact show`, `aep plan artifact graph`, `aep plan artifact waves`, and AGENTS.md "How the work is run". I also read these files in the tree:
- the `Expr` and `Instruction` matches in `crates/syntax/src/{early,scope}.rs`, `crates/compiler/src/{resolve,bytecode,lib,disasm}.rs` and `crates/swell/src/interp/dispatch.rs`
- `crates/compiler/tests/compile.rs`, `crates/syntax/tests/sources.rs` and `crates/swell/tests/language.rs`, which all read their fixtures by directory, so new fixtures do not collide
- the overlap of the case lists in `docs/plan/slice-06/` (the two cases that need both stories are only in cross-story.tsv).

Where each story writes:
- **Surfaces:** 5 cited, 0 inferred, 0 unplaced.
- **Acceptance reads:** 6 traced to a producer in the set (arrow on let-const; parameters on arrow and let-const; object-destructuring on parameters, arrow and let-const). All 6 are ordered by `depends_on`: 3 by a direct edge, 3 through the chain.
- **Stories 1 and 2 side by side:** the six shared files both bodies name are admitted collisions. Running the two at once is the coordinator's call. Besides resolve.rs, I found no other unnamed shared file between them.
- **Stories 3 to 5:** each is alone at its depth.

What I could not establish:
- How large the resolve.rs merge overlap will be. That depends on whether story 1 changes the `Expr` walk itself or only the statement and scope walk.
- `aep plan artifact waves` puts the two stories in different waves (8 and 9), because their scopes overlap. Running them together overrides the tool on purpose, and that is the coordinator's decision.
- Out of my lane, for the acceptance or scope critic: `test/language/expressions/tagged-template/cache-eval-inner-function.js` is on the let-const list but has a tagged template inside a string given to `eval`. The story already plans for this case (it gets reported with its part and taken off the list).

I created no scratch directory, so there is nothing to remove.

```findings
- file: .engineering/planning/story/template-literals.md
  line: 81
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "`Expr::Template` cannot compile without an arm in `crates/compiler/src/resolve.rs`, because the walk in `Resolver::expr` matches every `Expr` variant with no wildcard (lines 498 to 596) and the substitutions must be walked to resolve their names; yet resolve.rs is missing from this story's `Writes:` line and from its \"Shared with the story beside it\" list, while story:let-const-and-block-scopes rewrites the same file (cited, both bodies, a scope per block). Fix it with an ordering edge that records resolve.rs as its reason, or by adding resolve.rs to the Writes line and the shared list. As written, the unit stops at its first compile (AGENTS.md:324-326)"
- file: .engineering/planning/story/let-const-and-block-scopes.md
  line: 123
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its list of files shared with story:template-literals names six files but leaves out `crates/compiler/src/resolve.rs`, which both stories have to write (that one for the `Expr::Template` arm of the exhaustive match in `Resolver::expr`). The unit that lands second therefore has a merge overlap in resolve.rs that neither body mentions"
```

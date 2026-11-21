---
format: aep.planning-md/3
id: review-result:slice-06-design-round-1
kind: review-result
status: active
title: design critic, first five stories of slice 6, round 1
relations:
- reviews: story:let-const-and-block-scopes
- reviews: story:template-literals
- reviews: story:arrow-functions
- reviews: story:default-and-rest-parameters
- reviews: story:object-destructuring
revision: 1
---
needs-revision
story:default-and-rest-parameters — the arrow-function early error for `"use strict"` in a body whose parameter list is not simple sits in story:arrow-functions, which still refuses every non-simple list, so no input in that story can reach the rule; it belongs in this story's `early.rs` item, which names only `#sec-function-definitions-static-semantics-early-errors` and has to add `#sec-arrow-function-definitions-static-semantics-early-errors` for that rule — .engineering/planning/story/arrow-functions.md:93, .engineering/planning/story/default-and-rest-parameters.md:80

**What I read:** the 5 stories, the epic and `story:property-index-and-dense-elements`, with `aep plan artifact show`, `relations`, `graph`, `validate` and `waves`.
- **Edges:** I walked 7 edges from the set: arrow to let-const, arrow to template, parameters to arrow, destructuring to parameters, let-const to function-apply, let-const to property-index, and template to function-apply. I also followed function-apply to `story:builtin-group-modules`, outside the set. There is no cycle.
- **The chain:** stories 3 to 5 form a chain, and each edge has a written reason. Parameters reads the arrow cover. Destructuring reads the non-simple FunctionDeclarationInstantiation. The arrow-to-template edge has a shared-file reason (`parser/expression.rs`, compiler `{expression,bytecode,disasm}.rs`, `interp/{mod,dispatch}.rs`). The trade-off there is that order versus splitting those files; it is already stated.
- **Acceptance reads:** 9 acceptance reads go to a producer in the set, and an edge orders every one: 4 directly, 5 through another story. None of let-const's or template's tests reads the other's work.
- **Validator:** it reports `valid`. Its 2 scope warnings are on stories outside the set.
- **Layering:** every story runs end to end through parser, compiler, interpreter and tests, so none is a horizontal slice.

**What I could not establish, or is outside my lane:**
- **Acceptance or scope:** `crates/syntax/tests/sources.rs:722-726` expects the error text `default parameter is not supported before slice 6`. That file is not on the `Writes:` line of story:default-and-rest-parameters, so its `--test sources` acceptance cannot pass as written.
- **Parallel safety:** `aep plan artifact waves` puts let-const in wave 8 and template in wave 9, not side by side as both bodies say.
- **Parallel safety:** let-const's `depends_on` edge to story:property-index-and-dense-elements has no reason in its body. Both stories write `env/mod.rs`.
- **Scope:** the 2 cases on `docs/plan/slice-06/cross-story.tsv` are on no story's list.
- **Acceptance:** in `rest_charges_a_step_per_element`, the list that `apply` builds from 100,000 elements may use up the 10,000-step budget before the rest Array is made. The test would then not show the rest charge.
- I made no scratch files.

```findings
- file: .engineering/planning/story/default-and-rest-parameters.md
  line: 80
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the arrow-function early error for \"use strict\" in a body whose parameter list is not simple sits in story:arrow-functions, which still refuses every non-simple list, so no input in that story can reach the rule; it belongs in this story's early.rs item, which names only #sec-function-definitions-static-semantics-early-errors and has to add #sec-arrow-function-definitions-static-semantics-early-errors for that rule"
```

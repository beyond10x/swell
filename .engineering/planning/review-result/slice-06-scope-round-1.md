---
format: aep.planning-md/3
id: review-result:slice-06-scope-round-1
kind: review-result
status: active
title: scope critic, first five stories of slice 6, round 1
relations:
- reviews: story:let-const-and-block-scopes
- reviews: story:template-literals
- reviews: story:arrow-functions
- reviews: story:default-and-rest-parameters
- reviews: story:object-destructuring
revision: 1
---
needs-revision

story:default-and-rest-parameters — the body claims the `"use strict"` early error only for functions (`#sec-function-definitions-static-semantics-early-errors`), not for an arrow with a default or rest parameter, so one rule has no working owner: `story:arrow-functions` claims it, but in that story every non-simple arrow list is still refused and the rule can't be reached; the fix is to move that clause from story 3 into this story's `early.rs` item for arrows — .engineering/planning/story/default-and-rest-parameters.md:80 (and .engineering/planning/story/arrow-functions.md:93)

What I read: the epic and all 5 stories in full (`aep plan artifact show`), `aep plan artifact graph`, `docs/design/engine.md` section 12 (the `feature ... 6` lines and lines 2324-2325), `ls docs/plan/slice-06/`. I found 26 promises in the parent:
- 8 are claimed by the set: `let`, `const`, destructuring binding and destructuring assignment (object part only), default parameters, rest parameters, arrow functions, template literals.
- 13 are left out by name, either in a story's "Out of scope" or in the brief's list of later stories: array patterns, object spread and object rest, tagged templates, computed property names, `for`-`of`, `Symbol` and the well-known symbols, the iterator prototypes, the `Symbol` handle, budget charging for loops over iterators, and the 4 points of the close's acceptance.
- 4 constraints are carried by the run rules in every story.
- 1 is already answered by the design.

No story reaches beyond the epic, and no outcome is claimed twice.

What I could not establish:
- The epic's "Open, to be settled by this epic's first story" (.engineering/planning/epic/slice-06-bindings-and-iteration.md:34) is claimed by no story. It is already answered by `docs/design/engine.md:2324` ("The 38 features with no line ... are small additions inside the directory's own slice"), so it is not a gap. The coordinator's rewrite of the epic should drop or update it.
- Some later work is named by the set but is not in the brief's list of later stories: function declarations in a block or `switch` (story 1, "Out of scope"), computed property names, object rest and object spread (story 5, "Out of scope"). The second part of the slice must claim them.
- Out of my lane (design / parallel-safety): `story:let-const-and-block-scopes` also depends on `story:property-index-and-dense-elements`, which the brief does not mention.
- My scratch directory `.out/critic-scope/` is removed. One write went outside it: a redirection put a file at `.out/critic-scope-stories.txt` and I moved it into `.out/critic-scope/` straight away, so it was removed too.

```findings
- file: .engineering/planning/story/default-and-rest-parameters.md
  line: 80
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body claims the \"use strict\" early error of a non-simple parameter list only for functions (#sec-function-definitions-static-semantics-early-errors) and not for an arrow with a default or rest parameter, while story:arrow-functions claims that arrow rule where every non-simple arrow list stays refused, so the clause belongs in this story's early.rs item for arrows"
```

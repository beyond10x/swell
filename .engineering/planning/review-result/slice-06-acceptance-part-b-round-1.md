---
format: aep.planning-md/3
id: review-result:slice-06-acceptance-part-b-round-1
kind: review-result
status: active
title: acceptance critic, ten later stories of slice 6, round 1
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
story:symbol-values-in-core-builtins — check (c) needs at least B + L - R passes, with B measured on the unit's own base, which comes after `story:symbol-constructor-and-well-known-symbols`. Any case on the list that already passes once `Symbol` exists is counted in both B and L, so the bound can be out of reach even when every case on the list passes; L should count only the cases on the copy that fail in `.out/test262-base/runs.tsv`. — .engineering/planning/story/symbol-values-in-core-builtins.md:65
story:symbol-values-in-core-builtins — check (b) prints `0` before this story's work as well as after it (the body itself says "`story:symbol-constructor-and-well-known-symbols` takes it to 0 and this story keeps it there"), so it shows no change this story makes; it should select a failure the story removes, measured on its own base. — .engineering/planning/story/symbol-values-in-core-builtins.md:65
story:array-destructuring-and-spread — the summary promises "after it testharness.js and test.js parse with no refusal of slice 6", but the acceptance has no check of it. The only command given reads the refusal logs of the parser of `main`, the state before, so a unit can close with a refusal of slice 6 left in either script. — .engineering/planning/story/array-destructuring-and-spread.md:7
story:slice-06-conformance-close — the acceptance has no check that each moved case has a failed run on the base that ends on a part of a later slice. Moving every failing case of slice 6 makes (1) print `0`, which breaks the rule "A case that fails on a part of its own slice is never moved". The slice-02 close has this check ("none of its runs fails on a part of slice 2", .engineering/planning/story/slice-02-conformance-close.md:58). — .engineering/planning/story/slice-06-conformance-close.md:63

What I read: all 10 assigned artifacts, whole, with `aep plan artifact show`, plus the epic. I also read the planning files under `.engineering/planning/story/`, `taskfiles/test262.yml`, `taskfiles/host-requirements.yml`, `Taskfile.yml` (clippy), `crates/syntax/src/parser/{statement,expression}.rs` (the refusal sites), `crates/syntax/tests/sources.rs`, `docs/plan/waves.md` ("The order after slice 3") and `AGENTS.md` ("A case that cannot pass in its slice"). I re-ran with `awk` over `.out/test262/runs.tsv` and `.out/test262/lines.txt` on `f525f4a` every base number the 10 acceptances state: the case and run counts of each list, every (b) count, every B in (c), 7766 failed runs, and `1` and `1` for the two greps. All match. I also checked the slices of the runs behind each (b) check: the runs of later slices change their message once the story lands, so each (b) can reach `0`.

What I could not establish:
- How many cases on the symbol-values list already pass once `Symbol` exists. That needs a build, which I did not run. The (c) finding holds for every such case.
- The `consumer-gap/` directory and its `out/gap/` logs are not in the tree, so I could not run the array-destructuring command.
- This branch has no nextest config with `slow-timeout`/`terminate-after`. That config comes from `story:gate-test-steps-within-budget`, which is `active`, and none of the ten depends on it. Until it lands, "adds none over 60 s" rests on the unit's report, not on the runner.
- None of the ten states the 120 s step bound in words; each points to "Tests stay fast", as the accepted part A stories do.
- The `sources` binary (29.6 s) gains 8 nesting kinds across five stories, and no story sets a bound on its time.

Outside my lane:
- Order (design/scope): the parse milestone waits behind slice 3's close, Symbol and iterators. The parser half of array patterns does not need them.
- Coverage (scope): four failing cases of slice 6 are on no list (`ArrayIteratorPrototype/Symbol.toStringTag/*`, `StringIteratorPrototype/Symbol.toStringTag.js`). Three are only in `move-estimate.tsv`, `value-direct.js` and `value-from-to-string.js` with slice `none`.
- Wording: for-of says "Lifts 10 of the 12" in its summary and "This story lifts the 12" in its Context.

```findings
- file: .engineering/planning/story/symbol-values-in-core-builtins.md
  line: 65
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "check (c) needs at least B + L - R passes, with B measured on the unit's own base, which comes after `story:symbol-constructor-and-well-known-symbols`. Any case on the list that already passes once `Symbol` exists is counted in both B and L, so the bound can be out of reach even when every case on the list passes; L should count only the cases on the copy that fail in `.out/test262-base/runs.tsv`."
- file: .engineering/planning/story/symbol-values-in-core-builtins.md
  line: 65
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "check (b) prints `0` before this story's work as well as after it (the body itself says \"`story:symbol-constructor-and-well-known-symbols` takes it to 0 and this story keeps it there\"), so it shows no change this story makes; it should select a failure the story removes, measured on its own base."
- file: .engineering/planning/story/array-destructuring-and-spread.md
  line: 7
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the summary promises \"after it testharness.js and test.js parse with no refusal of slice 6\", but the acceptance has no check of it. The only command given reads the refusal logs of the parser of `main`, the state before, so a unit can close with a refusal of slice 6 left in either script."
- file: .engineering/planning/story/slice-06-conformance-close.md
  line: 63
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance has no check that each moved case has a failed run on the base that ends on a part of a later slice. Moving every failing case of slice 6 makes (1) print `0`, which breaks the rule \"A case that fails on a part of its own slice is never moved\". The slice-02 close has this check (\"none of its runs fails on a part of slice 2\", .engineering/planning/story/slice-02-conformance-close.md:58)."
```

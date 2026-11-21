---
format: aep.planning-md/3
id: review-result:slice-03-acceptance-round-1
kind: review-result
status: active
title: acceptance critic, slice 3 stories, round 1
relations:
- reviews: story:builtin-group-modules
- reviews: story:property-index-and-dense-elements
- reviews: story:object-integrity-and-prototype-methods
- reviews: story:object-enumeration-and-copy-methods
- reviews: story:function-apply-and-source-text
- reviews: story:global-this-uri-functions-and-is-error
- reviews: story:json-parse-and-raw-json
- reviews: story:array-iteration-methods
- reviews: story:array-search-and-copy-methods
- reviews: story:array-mutation-and-sort-methods
- reviews: story:string-access-and-search-methods
- reviews: story:string-transform-and-compare-methods
- reviews: story:slice-03-conformance-close
revision: 1
---
needs-revision
story:object-integrity-and-prototype-methods — the check `awk … '$6 ~ /(…|toLocaleString|is) should be an own property|…/' .out/test262/runs.tsv | wc -l` "prints `0`" can never pass: it matches `Number/prototype/toLocaleString/prop-desc.js` (slice 4, stays failing) and `built-ins/global/property-descriptor.js` ("globalThis should be an own property" ends in `is`, and only the global story fixes it) — .engineering/planning/story/object-integrity-and-prototype-methods.md:35
story:object-integrity-and-prototype-methods — the acceptance runs `awk` against `.out/decompose-slice-03/story-object-integrity.cases.txt`, a git-ignored file that exists only in the planning worktree, so a unit's tree cannot run it — .engineering/planning/story/object-integrity-and-prototype-methods.md:35
story:object-enumeration-and-copy-methods — the acceptance reads `.out/decompose-slice-03/story-object-enumeration.cases.txt`, which is not in the repository (`/.out` is in `.gitignore`; only `.cache` is linked into a worktree) and is not in `Writes:` — .engineering/planning/story/object-enumeration-and-copy-methods.md:36
story:function-apply-and-source-text — the acceptance reads `.out/decompose-slice-03/story-function.cases.txt`, a file no unit tree has and no `Writes:` entry supplies — .engineering/planning/story/function-apply-and-source-text.md:48
story:global-this-uri-functions-and-is-error — the acceptance reads `.out/decompose-slice-03/story-global.cases.txt`, a file no unit tree has and no `Writes:` entry supplies — .engineering/planning/story/global-this-uri-functions-and-is-error.md:44
story:json-parse-and-raw-json — the acceptance reads `.out/decompose-slice-03/story-json.cases.txt`, a file no unit tree has and no `Writes:` entry supplies — .engineering/planning/story/json-parse-and-raw-json.md:40
story:array-iteration-methods — the acceptance reads `.out/decompose-slice-03/story-array-iteration.cases.txt`, a file no unit tree has and no `Writes:` entry supplies — .engineering/planning/story/array-iteration-methods.md:34
story:array-search-and-copy-methods — the acceptance reads `.out/decompose-slice-03/story-array-search.cases.txt`, a file no unit tree has and no `Writes:` entry supplies — .engineering/planning/story/array-search-and-copy-methods.md:38
story:array-mutation-and-sort-methods — the acceptance reads `.out/decompose-slice-03/story-array-mutation.cases.txt`, a file no unit tree has and no `Writes:` entry supplies — .engineering/planning/story/array-mutation-and-sort-methods.md:34
story:string-access-and-search-methods — the acceptance reads `.out/decompose-slice-03/story-string-access.cases.txt`, a file no unit tree has and no `Writes:` entry supplies — .engineering/planning/story/string-access-and-search-methods.md:34
story:string-transform-and-compare-methods — the acceptance reads `.out/decompose-slice-03/story-string-transform.cases.txt`, a file no unit tree has and no `Writes:` entry supplies — .engineering/planning/story/string-transform-and-compare-methods.md:41
story:property-index-and-dense-elements — "the six probes of Context, run again with the release runner" live only under `.out/decompose-slice-03/perf-*` (git-ignored, absent from a unit tree), so the before and after times cannot be reproduced from the story — .engineering/planning/story/property-index-and-dense-elements.md:50

What I read: all 13 stories and the epic, in full, with `aep plan artifact show` for each id, plus `aep plan artifact kinds` and `aep plan artifact lifecycle story`. I also read the rubric, `AGENTS.md`, the case lists, `runs.tsv`, `expected.txt` and `moves.tsv`, and ran the acceptance `awk` regexes against the base `.out/test262/runs.tsv`.

What I could not establish:
- Whether the coordinator copies `.out/decompose-slice-03/` into each unit's tree in the unit brief. The stories do not say so.
- The Context counts on `Object/*` directories (for example 1,294 and 1,121). I checked the floors and the list sizes (4,050 lines, no case on two lists) against `expected.txt`, and they agree.
- No shipped code was run.
- Verdict on point 3 (moving six directories of iterator-dependent builtins and `normalize` whole): `slice-03-conformance-close` states its dependency on the ruling. Its acceptance (the final `awk`, which exempts the seven directories) holds with or without the ruling, so the exception is not a finding in my lane.
- I did not count the long conjunctive acceptances (cases pass, floor, no regression, tests green) as "more than one statement". Each clause is a command over one outcome, one unit's cases passing without regressions.
- Out of lane:
  - For `plan-critic-parallel-safety`: `conformance/test262/sections.txt` (three stories add lines), `conformance/host/requirements.txt` (three stories), `object/ordinary.rs`, and the mark lines of `realm/mod.rs` and of each shared `builtins/*/mod.rs`.
  - For `plan-critic-scope`: the epic's stale numbers.

```findings
- file: .engineering/planning/story/object-integrity-and-prototype-methods.md
  line: 35
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the check `awk … '$6 ~ /(…|toLocaleString|is) should be an own property|…/' .out/test262/runs.tsv | wc -l` \"prints `0`\" can never pass: it matches `Number/prototype/toLocaleString/prop-desc.js` (slice 4, stays failing) and `built-ins/global/property-descriptor.js` (\"globalThis should be an own property\" ends in `is`, and only the global story fixes it)"
- file: .engineering/planning/story/object-integrity-and-prototype-methods.md
  line: 35
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance runs `awk` against `.out/decompose-slice-03/story-object-integrity.cases.txt`, a git-ignored file that exists only in the planning worktree, so a unit's tree cannot run it"
- file: .engineering/planning/story/object-enumeration-and-copy-methods.md
  line: 36
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance reads `.out/decompose-slice-03/story-object-enumeration.cases.txt`, which is not in the repository (`/.out` is in `.gitignore`; only `.cache` is linked into a worktree) and is not in `Writes:`"
- file: .engineering/planning/story/function-apply-and-source-text.md
  line: 48
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance reads `.out/decompose-slice-03/story-function.cases.txt`, a file no unit tree has and no `Writes:` entry supplies"
- file: .engineering/planning/story/global-this-uri-functions-and-is-error.md
  line: 44
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance reads `.out/decompose-slice-03/story-global.cases.txt`, a file no unit tree has and no `Writes:` entry supplies"
- file: .engineering/planning/story/json-parse-and-raw-json.md
  line: 40
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance reads `.out/decompose-slice-03/story-json.cases.txt`, a file no unit tree has and no `Writes:` entry supplies"
- file: .engineering/planning/story/array-iteration-methods.md
  line: 34
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance reads `.out/decompose-slice-03/story-array-iteration.cases.txt`, a file no unit tree has and no `Writes:` entry supplies"
- file: .engineering/planning/story/array-search-and-copy-methods.md
  line: 38
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance reads `.out/decompose-slice-03/story-array-search.cases.txt`, a file no unit tree has and no `Writes:` entry supplies"
- file: .engineering/planning/story/array-mutation-and-sort-methods.md
  line: 34
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance reads `.out/decompose-slice-03/story-array-mutation.cases.txt`, a file no unit tree has and no `Writes:` entry supplies"
- file: .engineering/planning/story/string-access-and-search-methods.md
  line: 34
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance reads `.out/decompose-slice-03/story-string-access.cases.txt`, a file no unit tree has and no `Writes:` entry supplies"
- file: .engineering/planning/story/string-transform-and-compare-methods.md
  line: 41
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance reads `.out/decompose-slice-03/story-string-transform.cases.txt`, a file no unit tree has and no `Writes:` entry supplies"
- file: .engineering/planning/story/property-index-and-dense-elements.md
  line: 50
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"the six probes of Context, run again with the release runner\" live only under `.out/decompose-slice-03/perf-*` (git-ignored, absent from a unit tree), so the before and after times cannot be reproduced from the story"
```

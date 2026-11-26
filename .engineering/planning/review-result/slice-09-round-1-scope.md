---
format: aep.planning-md/3
id: review-result:slice-09-round-1-scope
kind: review-result
status: active
title: 'Slice 9, round 1, the scope critic: needs-revision, 1 finding'
relations:
- reviews: story:keyed-entry-table
- reviews: story:reflect-namespace
- reviews: story:map-and-set
- reviews: story:proxy-objects-and-access-traps
- reviews: story:weak-map-and-weak-set
- reviews: story:set-methods
- reviews: story:proxy-descriptor-prototype-and-extensibility-traps
- reviews: story:map-upsert-and-group-by
- reviews: story:host-object-overrides
- reviews: story:slice-09-conformance-close
revision: 1
---
Plan critic, scope, round 1 on the ten draft stories of `epic:slice-09-collections-proxy-reflect`
(`plan/slice-09` at `33330e14`), 2026-11-26. Verdict: needs-revision, one finding.

The epic's "Out of scope" still lists "host overrides of the internal methods: surf's binding
layer", yet `story:host-object-overrides` claims them under D5. The only epic rewrite the set
hands to the coordinator is "the epic's counts". The decomposer has to add the epic's
Out-of-scope line to that coordinator list in `story:slice-09-conformance-close`, worded for
instance "the Web IDL bindings that use the table: surf's" as `docs/design/engine.md`
decision 9 reads. (`.engineering/planning/epic/slice-09-collections-proxy-reflect.md:76`,
`.engineering/planning/story/slice-09-conformance-close.md:66`)

What the critic traced: 31 promises of the epic, all to stories (5 outcomes, 3 open questions,
5 acceptance items, 6 host requirements pending for slice 9 with HOST-222 added, 2 entries of
"What later slices take over"). Case lists: 480 + 376 + 247 + 129 + 46 + 246 + 204 + 59 =
1,787 = the 1,785 cases of slice 9 plus 2 incoming; no case twice; the union equals
`scan.tsv`; each list is byte-identical to its extraction from `scan.tsv`.

Not established by the critic: no full run file is in the worktree, so `lists.sh` was not run
again; `.cache/inventory/meta.tsv` is absent, so the cases of the features
`proxy-missing-checks` and `symbols-as-weakmap-keys` were not traced by their `features` field.

```findings
- file: .engineering/planning/epic/slice-09-collections-proxy-reflect.md
  line: 76
  category: scope
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the epic's Out of scope still excludes host overrides of the internal methods while story:host-object-overrides claims them under D5, and the close's list for the coordinator names only the epic's counts, so add the epic's Out-of-scope line to that list"
```

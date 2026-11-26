---
format: aep.planning-md/3
id: review-result:slice-09-round-1-acceptance
kind: review-result
status: active
title: 'Slice 9, round 1, the acceptance critic: needs-revision, 12 findings'
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
Plan critic, acceptance, round 1 on the ten draft stories of
`epic:slice-09-collections-proxy-reflect` (`plan/slice-09` at `33330e14`), 2026-11-26.
Verdict: needs-revision, 12 findings. The critic's findings, as returned, are the block below;
its account of what it read is shortened here.

Read: all ten story files in full, `check.sh`, `part.sh`, `lists.sh`, the head of `scan.awk`,
the line counts of every list and guard, `taskfiles/test262.yml`, `support/stack.rs` and the
re-entry code. Recounted against the decomposer's base run, read-only: all seven part sizes
and run counts (729/1,446, 532/1,046, 1,327/2,394, 403/791, 285/556, 1,285/2,321, 202/389)
match; every `check.sh` base line and failing count matches; the base counts of check (b)
532, 403, 265 and 368 match, reflect's 426 does not (296); `lists.sh` prints
`slice 1785 lists 1787 incoming 2` and exits 0. All ten stories write the failing test first
and state the time bounds. The start gate of `story:host-object-overrides` can be run.

Not established: whether the release `count` on a part fits inside `slot --timeout 30m`
(nothing in the tree measures a part this size). Outside the critic's rubric, for the design
critic: D3 counts a Proxy method without a trap as a re-entry, but the engine counts
re-entries by pushed activations (`crates/swell/src/interp/mod.rs:125`), and a forwarding
Proxy pushes none; the story does not say what counter it adds.

```findings
- file: docs/plan/slice-09/check.sh
  line: 11
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "check (a) also exits 0 when the list is empty or the part is empty: check.sh counts only the cases <runs after> holds, so a failed git show that leaves an empty list lets an unbuilt story pass on its guard alone; check.sh should exit 1 on an empty list and on every list case missing from <runs after>, or the acceptance should require part.sh's file count and the run count of runs.tsv; the six other test262 stories use the same script"
- file: .engineering/planning/story/reflect-namespace.md
  line: 69
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the base reading of check (b) says 426, but the base run gives 296; 426 is every failing run, and 130 of them fail on other text"
- file: .engineering/planning/story/reflect-namespace.md
  line: 54
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Bounds names the stack expression Reflect.construct(F, []), but deepest() of support/stack.rs defines only f, so reach_through gets a ReferenceError and not the RangeError and cannot pass as written"
- file: .engineering/planning/story/reflect-namespace.md
  line: 67
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the nextest commands exit 0 on the base because the filter also matches modules that already exist; the acceptance should name the count, 13 tests under builtins_reflect:: and none on the base"
- file: .engineering/planning/story/map-and-set.md
  line: 84
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the nextest commands exit 0 on the base; the acceptance should name the count, 14 tests under builtins_map_and_set:: and none on the base"
- file: .engineering/planning/story/proxy-objects-and-access-traps.md
  line: 87
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the nextest commands exit 0 on the base; the acceptance should name the count, 17 tests under proxy:: and none on the base"
- file: .engineering/planning/story/weak-map-and-weak-set.md
  line: 80
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the nextest commands exit 0 on the base; the acceptance should name the counts, 8 tests under builtins_weak_collections:: and the 5 new tests in heap::collect"
- file: .engineering/planning/story/set-methods.md
  line: 61
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the nextest commands exit 0 on the base; the acceptance should name the count, 11 tests under builtins_set_methods:: and none on the base"
- file: .engineering/planning/story/map-upsert-and-group-by.md
  line: 67
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the nextest commands exit 0 on the base; the acceptance should name the count, 10 tests under builtins_map_upsert:: and none on the base"
- file: .engineering/planning/story/host-object-overrides.md
  line: 125
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the nextest commands exit 0 on the base; the acceptance should name the count, 11 tests under host_overrides::, of which only the 5 requirement tests are held by host-requirements-check"
- file: .engineering/planning/story/proxy-descriptor-prototype-and-extensibility-traps.md
  line: 48
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Tests asks for one test per invariant of Change but names 10 while Change states 17 checks, so the set the acceptance checks is not closed; the body should name all 17 or drop one per invariant"
- file: .engineering/planning/story/slice-09-conformance-close.md
  line: 73
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "check (7) needs lists.sh to exit 0, but every story lets the coordinator take a case off its list, after which lists.sh prints missing and exits 1, and this story may not write close-later.tsv; the acceptance or the stories need to say where a removed case goes"
```

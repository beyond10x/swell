---
format: aep.planning-md/3
id: review-result:slice-09-round-2-acceptance
kind: review-result
status: active
title: 'Slice 9, round 2, the acceptance critic: needs-revision, 3 findings (1 warning, 2 notes)'
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
Plan critic, acceptance, round 2 on the ten draft stories of
`epic:slice-09-collections-proxy-reflect` (`plan/slice-09` at `4aca865a`), 2026-11-26.
Verdict: needs-revision, 3 findings, all on map-and-set; the 12 findings of round 1 are closed. The critic's answer, as returned:

needs-revision

Nine of my 12 round-1 findings are closed. One new acceptance defect came in with the revision and holds the set back, plus two notes.

**Finding lines**

- story:map-and-set — the step bound in the new test `abandoned_iterators_release_their_cursors_at_collect` can never be met. It says the compaction's steps, "read by `last_entry_steps()` of that entry, are at most 500 + 1 plus the cursors of live iterators, 0 here". The engine charges one step per bytecode instruction (`docs/design/engine.md` line 732, "The step budget"). So a script entry that makes the 500 deletes spends thousands of steps. Even one host call of the last `delete` spends its own few instructions on top of up to 500 entry moves, which is more than 501 whether or not the cursors were released. The decomposer should state a reading that tells 10,000 cursor moves apart from none: the step count against a control run without the abandoned loop, or a bound well below 10,000 that counts the call's own instructions — `.engineering/planning/story/map-and-set.md:75`
- story:map-and-set (note) — the new test `a_throw_in_for_each_closes_its_cursor` checks that the cursor list is empty only "read by the charged bytes", with no bound and no control. The 1,000 throws allocate too, so the test needs a stated comparison (for example: the growth is under 1,000 times a cursor's bytes, with the thrown value a primitive) — `.engineering/planning/story/map-and-set.md:75`
- story:map-and-set (note) — Change says an iterator closes its cursor when it answers done, and `forEach` closes its cursor on a normal exit. No test checks either without a collection in between. The two new tests cover only the collector's release and a `forEach` that throws — `.engineering/planning/story/map-and-set.md:71`

**Round-1 findings, closed or not**

1. `check.sh` passed on an empty list: closed. `docs/plan/slice-09/check.sh` lines 43 to 44 exit 1 on an empty list and on each list case the part lacks. I ran both self-tests of its header; both exit 1.
2. reflect check (b), 426 instead of 296: closed at `reflect-namespace.md:72`. My recount over `base-reflect.tsv` gives 296.
3. reflect stack expression: closed at `reflect-namespace.md:57-58` (`Reflect.construct(f, [])`). `deepest` in `support/stack.rs` defines `f` as a plain function, so it is a constructor.
4. to 10. The nextest commands exited 0 on the base: all closed. Each acceptance now names the count of new tests under its module and "none on the base", and I recounted every list against Change:

| Story | Line | Count |
|---|---|---|
| keyed-entry-table | 54 | 15 |
| reflect-namespace | 70 | 13 |
| map-and-set | 87 | 16 + 2 |
| proxy-objects-and-access-traps | 91 | 18 |
| weak-map-and-weak-set | 83 | 8 + 5 |
| set-methods | 59 | 11 |
| map-upsert-and-group-by | 65 | 11 |
| host-object-overrides | 127 | 13 |

11. The shape traps named 10 tests for 17 checks: closed. Lines 47 to 53 name I1 to I21, line 56 names `i01_` to `i21_`, and line 68 counts 33.
12. A case taken off a list broke `lists.sh`: closed. All seven test262 stories now send such a case to `close-later.tsv` with the same sentence. `lists.sh` counts that file as a list (its header, lines 4 to 6), and check (7) at `slice-09-conformance-close.md:73` settles every line of it with a command.

**What I read:** the ten story ids I was given, all ten read: every body from Context (Change for `set-methods` and `map-upsert-and-group-by`, whose Context I read on its own), with Acceptance in full. I also read the whole revision diff, `check.sh`, `lists.sh`, `part.sh`, my round-1 record, and `revision-1.md`. That scratch file holds the coordinator's decisions, not the decomposer's answers. I ran `check.sh` read-only over the seven `base-*.tsv` files: every printed reading the stories quote matches (`passed`/`bound`/`F`/`R` and the `failing` counts). The check (b) counts 296, 532, 403, 265 and 368 match too.

**Not established:** whether a part's release `count` ends inside one `slot --timeout 30m` job. The stories now say no run was timed and that a part which does not end is split in halves; that is a stated fallback, not a measurement.

```findings
- file: .engineering/planning/story/map-and-set.md
  line: 75
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "abandoned_iterators_release_their_cursors_at_collect bounds the compaction at 500 + 1 steps read by last_entry_steps(), but the engine charges a step per bytecode instruction (docs/design/engine.md line 732), so the entry that makes the deletes spends more than 501 steps whether or not the cursors were released; name a reading that tells 10,000 cursor moves from none, such as a control run without the abandoned loop or a bound well under 10,000 that counts the call's own instructions"
- file: .engineering/planning/story/map-and-set.md
  line: 75
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a_throw_in_for_each_closes_its_cursor reads the empty cursor list only 'by the charged bytes' with no bound or control, while the 1,000 throws also allocate; state the comparison, for example growth under 1,000 times a cursor's bytes with a primitive thrown"
- file: .engineering/planning/story/map-and-set.md
  line: 71
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Change says an iterator closes its cursor when it answers done and forEach closes it on a normal exit, but no test checks either without a collection between; add one test that reads the charged bytes after a finished for-of and a finished forEach"
```

---
format: aep.planning-md/3
id: review-result:slice-09-revision-2-coordinator-read
kind: review-result
status: active
title: 'Slice 9, revision 2 read by the coordinator: approve, no finding'
relations:
- reviews: story:keyed-entry-table
- reviews: story:map-and-set
- reviews: story:set-methods
- reviews: story:proxy-objects-and-access-traps
- reviews: story:proxy-descriptor-prototype-and-extensibility-traps
- reviews: story:weak-map-and-weak-set
- reviews: story:host-object-overrides
revision: 1
---
The coordinator's reading of revision 2 of the ten draft stories of
`epic:slice-09-collections-proxy-reflect` (`plan/slice-09`, the diff of seven story files over
`36815731`), 2026-11-26. It takes the place of a third round of the plan critics: two rounds
ran, as two adversary passes run on a unit, and the last revision is read by the coordinator.

Verdict: approve. Each of the nine findings of round 2 is answered in the text, and the lines
of code the new text rests on were read.

- Design 1 (`map-and-set`): `Arena::visit_marked_mut` in `heap/arena.rs`, after `Arena::sweep`;
  one call in `collect` after the check of the mark phase's answer and before the first sweep.
  Read in the code: `mark.give_back` (`heap/collect.rs` line 278) gives each workspace back,
  `give_back_workspace` (`heap/arena.rs` lines 410 to 415) clears only the worklist, and `sweep`
  clears the mark words at its end (line 437), so the words are readable at the call.
  `heap/arena.rs` is in the story's Writes and scope; `weak-map-and-weak-set` adds an arm to the
  same call, and the edge weak to map-and-set existed.
- Design 2 (`keyed-entry-table`): the list is freed and credited only when it holds no cursor;
  `release_cursors` frees in place, never reallocates and keeps a cursor with no owner; test
  `the_cursor_list_credits_only_what_it_gives_back`.
- Design 3 and 4 (`set-methods`): the cursor of a walk has no owner and is closed before the
  method returns, normal or abrupt; `union`, `intersection`, `difference` and
  `symmetricDifference` make the result Set first and keep it; tests
  `a_throwing_callback_leaves_no_cursor` and `the_result_keeps_its_keys_across_a_collection`.
  The decomposer added `difference` to the three methods the critic named, with its reason.
- Design 5 and 6 (both proxy stories): each trap names the step of its clause at which each
  fact of the target is read. Compared by the coordinator with ECMA-262 10.5.7 and 10.5.10:
  the descriptor after a false `has` and a true `deleteProperty`, IsExtensible only after a
  configurable descriptor. The decomposer did not follow one half of the critic's sentence:
  `getOwnPropertyDescriptor` reads IsExtensible at step 11 for every Object result, and the
  text follows the clause. In the first proxy story the target cannot log those reads yet, so
  its test revokes the target inside the trap; the second story's test logs all traps.
- Acceptance 1 to 3 (`map-and-set`): the cursor tests compare a run with the loops against a
  run without them, with a bound of N/10; `finished_loops_leave_no_cursor` reads steps and not
  bytes, because without a collection the bytes hold the loops' garbage.
- The parallel-safety critic's remark (`weak-map-and-weak-set`): the reason of the edge to
  proxy no longer names a shared arm.

Counts of new tests after the revision: `keyed-entry-table` 16, `map-and-set` 19,
`proxy-objects-and-access-traps` 19, `proxy-descriptor-prototype-and-extensibility-traps` 34,
`set-methods` 13. No edge was added; the orders 1 to 5 stand. `aep plan artifact validate`
prints `valid`; `lists.sh` prints `slice 1785 lists 1787 incoming 2`.

Not established by this reading: nothing was built; the sizes and step bounds the stories
name are decided by the tests each story adds.

```findings
[]
```

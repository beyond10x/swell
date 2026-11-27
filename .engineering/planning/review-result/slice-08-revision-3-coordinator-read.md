---
format: aep.planning-md/3
id: review-result:slice-08-revision-3-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of revisions 2 and 3 of the slice 8 stories: they can be proposed'
relations:
- reviews: story:new-target-meta-property
- reviews: story:super-property-and-super-syntax
- reviews: story:class-syntax-and-early-errors
- reviews: story:class-definition-evaluation
- reviews: story:class-element-syntax-and-early-errors
- reviews: story:derived-classes-and-super-call
- reviews: story:public-class-fields
- reviews: story:private-names-and-private-methods
- reviews: story:class-static-blocks
- reviews: story:eval-in-class-field-initializers
- reviews: story:private-fields-and-private-in
- reviews: story:private-names-in-eval-code
- reviews: story:host-ensure-can-add-private-element
- reviews: story:slice-08-conformance-close
revision: 1
---
Coordinator's read of revisions 2 and 3 of the fourteen draft stories of
`epic:slice-08-classes` (`plan/slice-08`, revision 2 at `45d30469`, revision 3 after it),
2026-11-26. Two rounds of the four plan critics are used (48 findings in round 1, 29 in round
2: 3 blockers, 13 warnings, 13 notes); this read stands in for a third round, as
`review-result:slice-09-revision-2-coordinator-read` did for slice 9.

## How it was read

The coordinator answered the 29 findings of round 2 with 26 decisions. A reader (a sub-agent
that edits nothing) checked the stories of revision 2 against each decision, the three
blockers, the form for held sources across the stories that add or lift a hold, the counts
of every list against its file, and 23 groups of `file:line` references against the code.
The coordinator read the reader's report, the decomposer's two reports, the sections of the
stories the report names (the derived return and its realm, the cases where the parts of
order 3 meet, the sizes), and ran `aep plan artifact validate` and `lists.sh`.

## What the reader found on revision 2

- 21 decisions held, 4 partly (6, 7, 13, 19), 1 held with the decomposer's stated reason
  (15: no edge needed, the path public-class-fields → class-definition-evaluation →
  class-syntax-and-early-errors exists in the edges).
- The 3 blockers of the acceptance critic are closed: the derived story names its four
  fixture files; a source with a held construct is a fixture under `held/` with its
  constructs in its header, and `admitted` lets `fits_the_stack_bound` pass on a refusal
  that is a line of `HOLDS`.
- `HOLDS`: 7 lines, each added once (2 by class-syntax-and-early-errors, 5 by
  class-element-syntax-and-early-errors) and deleted once by a story later in the order of
  the edges; the last lifting story has every other lifting story among its ancestors.
- Counts: the 12 story lists equal `wc -l` of their files and sum to 9,234; the close's
  lists hold 105; no case is on two lists, none is missing, 37 come in.
- References: 21 of 23 groups right, one wrong (`dispatch.rs:83-84`), one off by one.
- No function, instruction or file section has two owners.

## What revision 3 changed (the coordinator's six orders)

1. The six acceptances that lacked it say what their commands read when a story of the same
   order was merged first.
2. Assignment to a private target stays with `story:private-names-and-private-methods` (the
   decomposer's form, not the decision's): that story gets the test of every assignment
   form on a private accessor and a private method (18 tests).
3. `story:public-class-fields` says it writes only `static_field_run` and names the arm.
4. Three summaries, `dispatch.rs:84-86`, "the line before", the places of two `mod` lines.
5. The scan's rule for `class-static-blocks` equals its edges. 449 cases with no private
   field move from `story:private-fields-and-private-in` (984 cases after) to
   `story:class-static-blocks` (484 after), with 8 tests and the fixes where the parts of
   order 3 meet. The rule is stated once: a case goes to the first story, in the order of
   the edges, whose ancestors hold every part it needs.
6. The four stories that lift the last hold of a nesting kind own the scope analysis at the
   depth of the stack bound for that kind, with `crates/syntax/src/scope.rs` in their
   scope.

The coordinator changed two sentences of `docs/design/engine.md` that revision 2 found the
code cannot carry (the two places where the errors of a derived return are made; a second
reader of `Frame.function`), commit `cac8d142`.

## What stays open, accepted

- `story:class-static-blocks` now holds 449 cases that use no static block, and 8 tests of
  them in `class_static_blocks.rs`. The rule puts them there; its size is stated as about
  650 lines. If the unit finds the meeting cases larger than the story says, it reports
  under `needs-coordinator` and the coordinator cuts a story for them.
- `story:host-ensure-can-add-private-element` expects `host-surface-check` to exit 1 naming
  only HOST-223: the changelog is the coordinator's file.
- Not established by anyone: the time of any test; whether the scope analysis passes at the
  bound's depth for the four class kinds; the sizes of the private record and of
  `constructed`; whether `rustfmt` keeps `HOLDS = &[]` on one line. Each is measured by the
  unit that reaches it, and each story says what it reports.

## Verdict

`aep plan artifact validate` prints `valid`; `lists.sh` ends with `slice 9302 lists 9339
incoming 37`; the `depends_on` edges have no cycle (331 edges, `tsort` exit 0, by the
decomposer). The fourteen stories can be proposed.

```findings
[]
```

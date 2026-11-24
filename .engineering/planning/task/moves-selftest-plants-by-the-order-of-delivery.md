---
format: aep.planning-md/3
id: task:moves-selftest-plants-by-the-order-of-delivery
kind: task
status: active
title: The moves self-test plants its order defect by the order of delivery
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T18:00:37Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T18:00:38Z", actor: "human:operator", revision: 3}
---
## Outcome

`tests/gate/test262-moves-check-refuses-a-line-without-evidence.sh` passes on a list
`conformance/test262/moves.tsv` whose lines move a case to a slice with a lower number that is
delivered later, and still shows that `task test262-moves-check` refuses a line whose `<to>` does
not come after its `<from>` in the order of delivery.

## What is wrong

The script's third planted copy takes the first of its two unchanged lines whose `<to>` is 3 or
more and sets its `<from>` to `<to> + 1`. `task test262-moves-check` compares the two slices in
the order of delivery (`order delivery 1,2,3,6,5,7,10,4,9,8,11,…` in the `swell-slices` block of
`docs/design/engine.md`), not by number. Since the close of slice 10 the first line of the list
is `test/built-ins/AggregateError/newtarget-proto-fallback.js	10	9	part	Proxy`: its `<from>` is
already `<to> + 1`, the planted copy equals the unchanged lines, and the script ends with
`FAIL the planted <to> below <from> copy equals the unchanged lines`. The gate's step
`gate-selftest` printed `65 passed, 1 failed` on the wave.

## Change

One file: `tests/gate/test262-moves-check-refuses-a-line-without-evidence.sh`.

- The third planted copy sets the `<from>` of its line to a slice that does not come before the
  line's `<to>` in the order of delivery, read from the order line the task reads (the design
  file the script already passes as `TEST262_DESIGN`), not computed from the number of `<to>`.
- The copy differs from the unchanged lines for every line the list can hold; the script fails
  with a message that names the line when it finds no line it can plant in.
- The `<to>`, the kind, the evidence and the slice the block gives the case stay as they are, so
  the order of the two slices is the one thing wrong with the planted line.
- The script's header comment and the comment over the third copy say what the plant is.
- No other check of the script changes, and `taskfiles/test262.yml` does not change.

## Acceptance

- On the base the script exits 1 with the line above; after the change it prints
  `test262-moves-check-refuses-a-line-without-evidence: ok` and exits 0.
- Shown red: with the comparison of the task weakened in a scratch copy (the branch that prints
  `is not greater than from` removed, or `<=` turned to `<`, whichever the plant is meant to
  catch), the script exits non-zero and names the planted line. The report says which mutant was
  used and what the script printed. The weakened copy is not committed.
- `slot task gate-selftest` prints `gate-selftest: 66 passed, 0 failed`.

Writes: tests/gate/test262-moves-check-refuses-a-line-without-evidence.sh

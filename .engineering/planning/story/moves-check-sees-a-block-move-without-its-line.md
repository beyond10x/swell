---
format: aep.planning-md/3
id: story:moves-check-sees-a-block-move-without-its-line
kind: story
status: draft
title: The moves check sees a block move without its line of the list
summary: 'test262-moves-check reads the lines of moves.tsv only: a block line that sends a case to a later slice with no line of the list passes the gate. Backlog, from pass 1 of the adversary on unit close9 of wave big-2.'
revision: 2
---
## Context

Found by pass 1 of the adversary on the unit `close9` of wave big-2 (its finding F4,
`review-result:adversary-close9-pass-1`), which closes slice 9. It is older than the unit and
outside its scope: the story of the close does not write `taskfiles/test262.yml`.

A case that cannot pass in its slice moves to a later slice by a rule line of the
`swell-slices` block of `docs/design/engine.md`, section 12, and by one line of
`conformance/test262/moves.tsv` that holds its evidence. The gate step `test262-moves-check`
(`taskfiles/test262.yml`) reads the lines of the list: it checks each line's fields, that the
block gives the case the slice the line names, and that the evidence stands in the case or in
a harness file it includes.

It does not read the block for lines the list lacks. Planted on a copy: the line of
`test/built-ins/Set/prototype/union/called-with-object.js` (slice 9 to slice 12) taken out of
the list, its block line `dir <file> 12` kept. The moves check stays green and its count
still equals the lines of the file (1,985 of 1,985). Only the checks (2) and (7) of the close
story see it, and they read the run lines of a full run that only a close makes. After a
close, a block line that sends one case to a later slice with no evidence passes every step
of the gate.

The finding is older than the wave and is filed here, in the backlog. No wave takes this
story before the planned waves are delivered.

## Change

To be written when the story is planned. The direction: `test262-moves-check` also reads the
block. A `dir` line that names one file and gives it a slice delivered later than the slice
the case would have without that line is a move, and the check refuses it when the list has
no line for the case. A line that takes a case into an earlier slice (section 12, "A case
that comes into a slice from one delivered later") has no line of the list by rule 3 and is
not refused. A directory line needs its own rule: say which directory lines are moves and
how the list names them.

## Acceptance

To be written when the story is planned: a selftest under `tests/gate/` that takes one line
out of a copy of the list and reads the refusal with the case's path; a second that adds a
block line for a case into an earlier slice with no line of the list and reads exit 0; the
step's time in the gate before and after (`gate-time test262-moves-check`, 1.5 s on the build
node on 2026-11-27).

## Added by the close of slice 8

Pass 1 of the adversary on the unit `close8` of wave big-2
(`review-result:adversary-close8-pass-1`, its findings 3 and 4) planted both gaps again on
the tree of that close:

- the line of `language/statements/class/subclass/builtins.js` taken out of `moves.tsv` with
  its block line `dir ... 13` kept: `test262-moves-check` exits 0 and prints `2030 case(s)
  moved`; `test262-check` fails only on the digest, which `test262-expected` writes again;
- field 2 of that line (the slice the case leaves) changed from 8 to 7: the moves check
  exits 0, and so do the checks of the close story, which count by that field; only the
  story's groups line shows `all 8` in place of `all 9`.

The second gap is new to this story: the moves check reads no run lines and has no source
for the slice a case left. When the story is planned, the Change names one: the block as it
was on the commit before the line of the list was added, or the slice the case's own
`feature`, `flag` and directory lines give it without its file line.

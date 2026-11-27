---
format: aep.planning-md/3
id: story:tests-name-no-refusal-of-a-delivered-slice
kind: story
status: draft
title: The tests name no refusal of a delivered slice
summary: A branch no shape reaches and four comments in the tests still name a refusal before slice 8, which is delivered. Backlog, from pass 1 of the adversary on unit close8 of wave big-2.
revision: 1
---
## Context

Found by pass 1 of the adversary on the unit `close8` of wave big-2
(`review-result:adversary-close8-pass-1`, its finding 5; read in
`review-result:close8-coordinator-read`). It is older than the unit, which writes no file of
a crate.

Slice 8 is delivered, and no source file of `swell-syntax` or `swell-compiler` refuses a
construct "before slice 8" (`grep -rnE 'refuse[_a-z]*\("[^"]*", 8' crates/syntax/src
crates/compiler/src | wc -l` prints `0`). The tests still hold text of that refusal:

- `crates/syntax/tests/sources.rs`, near line 3043: a branch builds the message
  `"{construct} is not supported before slice 8"`; every shape that reaches the function has
  an empty `construct`, so the branch is never taken;
- comments that describe a refusal before slice 8 as current: `crates/syntax/tests/sources.rs`
  near line 3159, `crates/swell/tests/async_functions.rs` near line 605,
  `crates/swell/tests/private_methods.rs` near lines 898 and 2395.

A reader of the tests takes slice 8 for open. The same kind of text may be left for the
other delivered slices; nobody has searched for it.

Filed here, in the backlog. No wave takes this story before the planned waves are
delivered.

## Change

To be written when the story is planned. The direction: for each slice of
`conformance/slices-delivered.txt`, `grep -rn "before slice <n>"` over `crates/*/tests/` and
the documents; a branch no test reaches is removed, a comment is rewritten to what the code
does now; a check of the gate (in the form of `kill-check`) refuses "before slice <n>" for a
delivered `<n>` outside the files that record history (`CHANGELOG.md`, `docs/plan/`).

## Acceptance

To be written when the story is planned: the grep prints nothing for each delivered slice
outside the history files; the check refuses a planted line; no test's count changes.

---
format: aep.planning-md/3
id: story:realm-walk-reaches-hidden-intrinsics-and-holds-type-errors
kind: story
status: draft
title: The realm walk reaches the hidden intrinsics and holds the TypeError where one is due
summary: 10 functions of hidden intrinsics are not walked, and a builtin that answers for a Symbol where a TypeError is due stays green. Backlog, from pass 1 of the adversary on unit realmwalk of wave big-2.
revision: 1
---
## Context

Found by pass 1 of the adversary on the unit `realmwalk` of wave big-2
(`review-result:adversary-realmwalk-pass-1`, findings 9 and 10; read in
`review-result:realmwalk-correction-1-coordinator-read`). Both are limits of the walk that
gives every function of the realm a Symbol (`crates/swell/tests/builtins_symbol_values.rs`),
older than the story that put the walk in parts
(`story:realm-walk-in-parts-under-the-cpu-bound`), which did not change them.

- Functions the walk does not reach. The walk lists the functions that a property path from
  the global object or a root of `FURTHER_ROOTS` reaches. A probe on the unit's head (not
  committed: the walk's list from the global object, `FURTHER_ROOTS` and nine more roots
  reached by evaluating a source, minus the walk's own list) counted 10 more functions
  against the walk's 344: `next` of `%ArrayIteratorPrototype%`, `%MapIteratorPrototype%`,
  `%SetIteratorPrototype%`, `%StringIteratorPrototype%` and
  `%RegExpStringIteratorPrototype%`; `%GeneratorFunction.prototype%.constructor`;
  `%IteratorPrototype%[Symbol.iterator]`; `next`, `return` and `throw` of
  `%GeneratorPrototype%`. It counted 0 callable own properties on the function objects of
  accessors, and the bare realm holds no bound function. The probe does not count an
  intrinsic that none of its nine sources reaches.
- A builtin that answers where a TypeError is due stays green. The walk holds that a call
  with a Symbol answers or throws a TypeError, and holds the TypeError only for the rows of
  `WALK_EXPECTED`. The adversary made `Function.prototype.bind` answer normally for a Symbol
  `this`: both builds stayed green.

The findings are older than the wave and are filed here, in the backlog. No wave takes this
story before the planned waves are delivered.

## Change

To be written when the story is planned. Two parts, which may become two stories:

- The hidden intrinsics become roots of the walk (`FURTHER_ROOTS`, or a list made from the
  realm's intrinsics table), the committed file `crates/swell/tests/fixtures/realm_places.txt`
  is written again by `task realm-places-expected`, and a test fails when the realm holds a
  function object that no place of the file names.
- For every function whose first clause is a brand check or a RequireObjectCoercible of the
  place the Symbol takes, the walk holds the TypeError: a row per function, or a rule read
  from a list of such functions.

The time bounds of the walk's parts hold as `AGENTS.md` states them ("Commands and the
gate"); a part over four fifths of its bound adds a part, up to eight.

## Acceptance

To be written when the story is planned: each function the probe named is called by a part
of the walk, and the plant of the adversary (a `bind` that answers for a Symbol `this`) is
red.

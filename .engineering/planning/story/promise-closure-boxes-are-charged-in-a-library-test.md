---
format: aep.planning-md/3
id: story:promise-closure-boxes-are-charged-in-a-library-test
kind: story
status: draft
title: A library test holds that the box of a promise closure is charged
summary: Boxes of Promise.all's element functions that are not charged pass the one test that reads them. Backlog, from correction 2 of unit setmethods of wave big-2.
revision: 1
---
## Context

Found in the second correction of the unit `setmethods` of wave big-2
(`review-result:setmethods-correction-1-coordinator-read`), in a test the unit did not write.
`promise_statics::the_values_of_a_pending_promise_all_are_charged_until_it_settles` compares
the account while a `Promise.all` is pending with a control round of `Promise.race`. A planted
"charged twice, credited once" at the box of an element function turns it red. Boxes that are
not charged at all pass its lower bound: each element function also owns a property table of
224 bytes that the race round does not make, a margin of 5,216 bytes over the 2,880 bytes of
the boxes in the collect mode. The library test beside `create_promise_closure`
(`crates/swell/src/object/builtin_function.rs`, the test that asserts `ElementRecord::bytes()`
88 and `element.bytes()` 144) asserts the sizes alone, not the charge. The gap is older than
the wave (the test and the function are of slice 6) and is filed here, in the backlog. No wave
takes it before the planned waves are delivered.

## Change

A library test calls `create_promise_closure` for each kind of `PromiseSlots` and asserts that
the account rises by `slots.bytes()` more than for a builtin function without slots, and
returns to its reading once the function is collected.

## Acceptance

- With the charge of the box removed in a copy, the new test is red in the default mode and
  with `collect-every-safe-point`; with the charge made twice it is red too.
- The test takes under 1.5 CPU s alone in the default build and under 2 s in a collect build.

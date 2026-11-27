---
format: aep.planning-md/3
id: story:stack-tests-hold-in-the-stack-debug-profile
kind: story
status: draft
title: The stack tests the stack-debug profile is said to run pass there
summary: Four re-entry tests overflow the promised stack in the profile stack-debug (five on the base of wave big-2); test-stack-profiles runs four other tests. Backlog, from adversary pass 1 on unit newtarget of wave big-2.
revision: 2
---
## Context

Found by the adversary's first pass on the unit `newtarget` of wave big-2
(`review-result:adversary-newtarget-pass-1`, finding 5), on `impl/newtarget` at `90960dae`; the
base of that unit (`896bcbe9`) fails the same tests and one more, so the finding is older than
the wave and is filed here, in the backlog. No wave takes it before the planned waves are
delivered.

The lane `slow` builds the one test binary of `swell` in the profiles `release` and
`stack-debug` (the dev profile with full debug information) and runs four named stack tests in
each (`task test-stack-profiles`, `taskfiles/stack-profiles.yml`): HOST-17, HOST-152, HOST-163
and `min_stack_bytes_is_enough_through_builtins`. `crates/swell/tests/support/host.rs:233` to
`:236` says the profile runs "the stack tests", and `:223` names `toJSON` as the largest need
per re-entry.

Measured on 2026-11-26 with `--cargo-profile stack-debug` over every test whose name holds
`stack`, `depth`, `re_enters` or `nest` (110 tests): 106 passed and 4 ended with SIGABRT (stack
overflow) on a thread of seven eighths of `Limits::min_stack_bytes()`, 12,838,560 bytes:

- `regexp_symbol_methods::a_replacer_re_enters_on_the_promised_stack`
- the re-entry test of `builtins_string_replace` (line 438 at that commit)
- the re-entry test of `builtins_object_integrity` (line 757)
- the re-entry test of `tagged_templates` (line 532)

On the base a fifth fails: `regexp_objects::last_index_conversion_re_enters_on_the_promised_stack`.
The replacer path needs 13,665 bytes of stack per re-entry in that profile, 95 % of
`STACK_BYTES_PER_REENTRY` (14,336); in the gate's dev profile the same path needs 12,400. The
four tests pass in the gate's profile and in `release`.

## Change

Either every test that runs a re-entry on the promised stack passes in the profile
`stack-debug`, or the text and the task say which tests the profile holds and why the others
are not held there. The unit measures the bytes per re-entry of each failing path in
`stack-debug`, decides from the numbers (a larger `STACK_BYTES_PER_REENTRY`, a smaller frame
on the path, or a stated bound of the profile), and makes `test-stack-profiles` run the set
the text names. `host.rs:223` names the path that is the largest need at that commit.

## Acceptance

- `task test-stack-profiles` runs every test the comment in `tests/support/host.rs` says the
  profile runs, and all pass in `release` and in `stack-debug`; or the comment lists the tests
  that are not held in `stack-debug`, each with its measured bytes per re-entry.
- The largest need per re-entry named in `host.rs` is the measured one, with its number.
- The lane `slow` stays within a pool job's time.

## Measurements of wave big-2

Added at the close of the units `setmethods` and `stackjoin` of wave big-2, 2026-11-27. All
figures are for x86_64 with rustc 1.99.1, in the profile `stack-debug` unless named, at
`reentry_depth` 1,000 on a thread of seven eighths of `Limits::min_stack_bytes()`
(12,838,560 bytes; the whole promise is 14,672,640).

The four function constructors overflow the thread and the promise. Through the `toString`
of an argument, measured on `22842da` (`impl/stackjoin`), bytes per re-entry and the need at
depth 1,000:

- `Function({ toString() { return f(); } })`: 18,368 and 18,564,096
- the constructor of generator functions: 18,144 and 18,339,840
- the constructor of async functions: 18,144 and 18,339,840
- the constructor of async generator functions: 17,952 and 18,148,352

On `5b25b85`, before the wave's order 3, `Function` took 19,104 bytes per re-entry, so the
finding is older than the wave. The frame of `dynamic_function_steps`
(`crates/swell/src/dynamic_code.rs`) is 7,296 bytes. Measured by the implementor of the
unit `stackjoin` and not applied: an `#[inline(never)]` function for the parse and the
compile brings that frame to 2,560 bytes and the route to 13,632 bytes per re-entry, which
is still 989,536 bytes over the thread. No stack test re-enters through a function
constructor.

`test-stack-profiles` names four tests (`STACK_TEST_LIST`, `taskfiles/stack-profiles.yml`).
Since the unit `stackjoin` the test through builtins runs five routes (`toJSON`, a getter of
`Object.assign`, `Array.from` through `next`, a replacer, and `Math.max` through a
`valueOf`) and needs 12,195,840 bytes. Seven stack tests of the unit `setmethods` are not in
the list; each passes in `stack-debug` by the measurement below, and none runs there in the
lane:

- `builtins_set_methods::union_reaches_the_reentry_limit_through_the_size_getter_on_the_promised_stack`
- `builtins_set_methods::is_subset_of_reaches_the_reentry_limit_on_the_promised_stack`
- `builtins_set_methods::intersection_reaches_the_reentry_limit_through_has_on_the_promised_stack`
- `builtins_set_methods::intersection_reaches_the_reentry_limit_through_keys_on_the_promised_stack`
- `builtins_set_methods::union_reaches_the_reentry_limit_through_next_on_the_promised_stack`
- `builtins_set_methods::intersection_reaches_the_reentry_limit_through_next_on_the_promised_stack`
- `builtins_set_methods::is_disjoint_from_reaches_the_reentry_limit_through_return_on_the_promised_stack`

Routes of the set methods, measured on `impl/setmethods` at `d15e536` before the unit
`stackjoin` (bytes per re-entry in `stack-debug`, need at 1,000, bytes per re-entry in
`release`): the size getter 11,327, 11,521,832, 3,391; `has` of `intersection` 10,881,
11,074,488, 4,015; `keys` of `intersection` 10,528, 10,722,128, 3,776; `next` of
`intersection` 11,407, 11,601,496, 3,920; `return` of `isDisjointFrom` 11,057, 11,250,668,
3,808; a getter of `has` in `union` 11,777, 11,970,708, 3,519; a `valueOf` of the size in
`union` 12,192, 12,385,880, 3,967.

The dispatch of the builtins is four parts since `22842da`: a route passes at most 736 bytes
of dispatch in `stack-debug`, and in `release` 27 of 31 measured routes grew by up to 1,184
bytes per re-entry (the largest is 5,872 of 14,336). A unit of this story that makes a frame
smaller measures the routes in all three profiles.

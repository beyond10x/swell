---
format: aep.planning-md/3
id: story:stack-tests-hold-in-the-stack-debug-profile
kind: story
status: draft
title: The stack tests the stack-debug profile is said to run pass there
summary: Four re-entry tests overflow the promised stack in the profile stack-debug (five on the base of wave big-2); test-stack-profiles runs four other tests. Backlog, from adversary pass 1 on unit newtarget of wave big-2.
revision: 1
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

---
format: aep.planning-md/3
id: story:host-206-holds-the-charge-of-the-request-queue
kind: story
status: draft
title: The test of HOST-206 holds the charge of the request queue
summary: With the queue's charge set to zero the test of HOST-206 stays green. Backlog, from adversary pass 1 on unit lanetests of wave big-2.
revision: 1
---
## Context

Found by the adversary's first pass on the unit `lanetests` of wave big-2
(`review-result:adversary-lanetests-pass-1`, finding 4), in a test older than the wave. The
test of HOST-206 in `crates/swell/tests/async_generators.rs` queues requests on an async
generator and reads the account. With the charge of the queue set to zero in a copy (the
growth of the request queue in `crates/swell/src/object/async_generator.rs` charged as 0
bytes) the test stays green at 40 and at 100 requests; only
`settling_a_request_credits_nothing_until_the_generator_is_freed` goes red there. With the
credit of a property table removed the HOST-206 test goes red, so it holds a charge, but not
the one its requirement names. The finding is older than the wave and is filed here, in the
backlog. No wave takes it before the planned waves are delivered.

## Change

The test of HOST-206 asserts the charge of the request queue itself: the account rises by
the room of the queue while the requests are queued, by a bound taken from the count of
requests, and a queue that is charged nothing fails it.

## Acceptance

- With the queue's charge set to zero in a copy, the test of HOST-206 is red in the default
  mode and with `collect-every-safe-point`.
- The test holds at the realm sizes of `test-realm-sizes` in both modes, and takes under 1.5
  CPU s alone in the default build and under 2 s in a collect build.

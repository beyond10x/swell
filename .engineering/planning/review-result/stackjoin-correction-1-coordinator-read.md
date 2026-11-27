---
format: aep.planning-md/3
id: review-result:stackjoin-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit stackjoin: no defect'
relations:
- reviews: story:weak-map-and-weak-set
revision: 1
---
Coordinator's read of correction 1 of the unit stackjoin (impl/stackjoin, 52fd8eb to
22842da), 2026-11-27. Verdict: no defect. No adversary pass ran on this unit: its change is
the dispatch of the builtins and one more route in an existing stack test, and the read below
checks the dispatch variant by variant.

The change (two files, 251 lines added, 11 removed):

- `crates/swell/src/object/builtin_function.rs`: `Agent::run_builtin` matches the id to a
  function pointer and makes one call; four functions `run_builtin_part_a` to
  `run_builtin_part_d`, each `#[inline(never)]`, hold the arms of at most nine groups in the
  order of `BuiltinId`. Every match lists every variant, without a `_` arm; a part answers an
  engine fault for an id of another part. Read with a script over the source: the 35
  variants of `BuiltinId` appear once each in `run_builtin`, and for each of them the part
  `run_builtin` picks is the part whose arm runs its steps (35 of 35; the other 105 arms are
  the fault arms). The arms themselves are the 35 arms of the one match, text for text but
  the arm of `Await`, which passes the first argument in the call in place of a `let`.
- `crates/swell/tests/limits_scripts.rs`: `min_stack_bytes_is_enough_through_builtins` runs a
  fifth route, `Math.max` through a `valueOf` (`THROUGH_OTHER_GROUPS`), the largest measured
  route of the groups no stack test re-entered through, and
  `each_builtin_reentry_counts_against_reentry_depth` counts it. The test's depth, thread and
  name and `STACK_BYTES_PER_REENTRY` are unchanged. On 2951b32 the test with the fifth route
  is red in `stack-debug` (SIGABRT on a stack of 12,838,560 bytes); on 22842da it is green in
  both profiles.

Frames, measured by the implementor on x86_64 (bytes; `stack-debug`, dev, release):
`run_builtin` 192, 176, 8; the parts 352 to 544, 304 to 512, 544 to 976. A route passes at
most 736 bytes of dispatch in `stack-debug`, where it passed 1,360 before the merge of
`impl/weak` and 1,424 after it.

Routes, measured by the implementor in `stack-debug` on 5b25b85, 52fd8eb, 2951b32 and
22842da (31 routes, one per group that can call script, and the host function's): every
route through a builtin is smaller on 22842da than on 5b25b85 (for example `toJSON` 12,640 to
12,000 bytes per re-entry, `Math.max` through `valueOf` 12,512 to 11,872, `Promise.all`
through `next` 12,432 to 11,616). In the dev profile all 31 are smaller or equal. The four
stack tests have room in `stack-debug`: the builtins test needs 12,195,840 of 12,838,560
bytes (642,720 to spare), HOST-17 8,515,584, HOST-152 8,518,656, HOST-163 8,524,800.

Two things the correction could not bring where the coordinator asked:

- In the release profile 27 of the 31 routes are larger than before, by up to 1,184 bytes
  per re-entry, because each part inlines functions the one match did not; the largest
  release route is 5,872 bytes of the 14,336 promised. The coordinator accepts this: one form
  of the dispatch in every profile, and the release routes keep more than half of the promise
  to spare.
- The four function constructors (`Function`, and the constructors of generator, async and
  async generator functions) through the `toString` of an argument need 18.1 to 18.6 MB at
  depth 1,000 in `stack-debug`, over the thread and over the promise, on 5b25b85 already
  (19,104 bytes per re-entry for `Function`): the frame of `dynamic_function_steps`
  (`dynamic_code.rs`) is 7,296 bytes. This is older than the unit and outside it; it goes to
  the backlog story `stack-tests-hold-in-the-stack-debug-profile`, with the implementor's
  measurement that an `#[inline(never)]` function for the parse and the compile brings the
  frame to 2,560 bytes and the route to 13,632 per re-entry, still 989,536 bytes over the
  thread.

Checks of the implementor on 22842da, on the build node: `task test-stack-profiles`, 4 of 4
in both profiles; the whole `swell::main` in both modes, `1347 tests run: 1347 passed, 1
skipped` each; the library tests in both modes (269 and 265 passed); clippy and the format
check. In copies, `impl/classes` at 4d2d834 and `impl/proxytraps` at 61b14ed2, each merged
with 22842da, pass the four stack tests in both profiles.

Not measured: a route for the groups Await, AsyncIteration, AsyncGenerator, AsyncBuiltinAwait
and Proxy (they run from jobs or call no nested script); one route per group, not every
builtin of a group; the need at depth 1,000 in the dev and release profiles (bytes per
re-entry only).

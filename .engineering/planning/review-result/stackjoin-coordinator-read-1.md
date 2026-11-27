---
format: aep.planning-md/3
id: review-result:stackjoin-coordinator-read-1
kind: review-result
status: active
title: 'Coordinator''s first read of the unit stackjoin: the moved groups'' routes grew'
relations:
- reviews: story:weak-map-and-weak-set
revision: 1
---
Coordinator's first read of the unit stackjoin (impl/stackjoin at 2951b32), 2026-11-27.
Verdict: one finding; the unit went back for a correction.

The unit is not a story's: it corrects a defect of the join. After the merge of `impl/weak`
into `wave/big-2` (6e9b7fa) the stack test
`limits_scripts::min_stack_bytes_is_enough_through_builtins` overflowed its thread in the
profile `stack-debug`. The unit found the cause: the two new arms `WeakMap` and `WeakSet` of
`Agent::run_builtin` (`crates/swell/src/object/builtin_function.rs`) grew that frame from
1,360 to 1,424 bytes in `stack-debug`, since an unoptimised build keeps copies of the
arguments per arm; the route through `toJSON` went from 12,640 to 12,704 bytes per re-entry
and needed 12,899,328 bytes at depth 1,000, of the 12,838,560 the test runs on. The cause and
the measurements are accepted.

Its change at 2951b32 kept the arms of 17 groups in `run_builtin` (960 bytes) and moved the
other 18 into `run_other_builtin` (672 bytes), chosen by "a stack test re-enters through the
group".

Finding (correctness of the stack bound, introduced by the change): the moved groups run
script (`valueOf`, `toString`, a getter, an executor, `then`, an iterator), and a route
through one of them passed 1,632 bytes of dispatch where it passed 1,360 before the merge. By
the figures of the adversary of the unit setmethods (`Math.max` through `valueOf`, 12,512
bytes per re-entry in `stack-debug`) that route would need about 12,978,000 bytes at depth
1,000: the change moved the overflow from a route with a test to one without. The correction
measured it: on 2951b32 `Math.max` through `valueOf` is 12,784 bytes per re-entry and needs
12,980,224 bytes, and `Promise.all` through `next` 12,704 and 12,887,040; both over the
thread.

Asked of the correction: the routes of every group measured on three commits; a dispatch
under which no route through a builtin is larger than before the merge; the largest route of
the moved groups added to the stack test.

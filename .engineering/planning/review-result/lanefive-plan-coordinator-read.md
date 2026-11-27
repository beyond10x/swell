---
format: aep.planning-md/3
id: review-result:lanefive-plan-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of the interpreter-loop story after the plan critics: ready for its unit'
relations:
- reviews: story:property-escape-cases-spend-less-in-the-interpreter-loop
revision: 1
---
Coordinator's read of `story:property-escape-cases-spend-less-in-the-interpreter-loop` after
one read by two plan critics (acceptance, design), 2026-11-27. Verdict: ready for its unit
at revision 4 of the story.

The story was a backlog draft; the operator took it into wave big-2 as the fifth unit of the
lane `slow` (`approval-record:wave-big-2-lane-unit-5`). The coordinator wrote its Change and
Acceptance from the profile of the unit `lanefour`. Both critics read that text on
`wave/big-2` at `1a818e1` and asked for a revision. What each finding became:

The acceptance critic, ten findings (six blocking):

- The digest of the expected file covers the sources of the runner's crates, so "the digest
  unchanged" fails for every change of the interpreter. Taken: the unit leaves
  `expected.txt` unchanged and the coordinator regenerates the digest at the merge.
- The run lines hold no time per run; the runner measures wall time per run and writes it to
  the part files only. Taken with a change: the unit picks one part of `task test262-part`
  that holds at least 30 property-escape files, one worker, three runs each on the base and
  the head, alternating, and sums the nanoseconds field. The story's Context now says the
  figure of 854 of 1,171 s is wall time of the runs.
- A median between 80 % and 90 % was neither accepted nor stopped. Decided: one threshold.
  A ratio of at most 0.90 is the acceptance and a ratio above it the stop rule; 0.80 is the
  unit's aim, reported. The margin of the lane on the forge is 17.4 s, and the unit
  `lanefour` estimated 20 s per 10 % on the loop, so a gain of 10 % is worth a merge.
- `git diff --stat` over `crates/*/tests` misses the library tests in `crates/*/src` and
  counts the generated `tests/main.rs`. Taken, in the critic's form.
- The line on `perf-loop` named no way to read CPU seconds, no base binary and no ratio.
  Taken: user plus system of the child as `taskfiles/gate-time.yml` reads them,
  `PERF_BASE_BIN`, alternating runs, the lines the task prints.
- "Each part at or under its time on the base" compared wall times on a shared node. Taken:
  the lane exits 0 and its lines are quoted.
- Four notes (the per-test time is a measurement and no committed test reads a clock; tests
  only for the cuts kept, the heap test with a literal; the task's file, the prefix of its
  variable and `GATE_VARIABLES`; the loop's instructions from `disassemble`). Taken.

The design critic, ten findings (three blocking):

- A cached instruction slice would be a field of `Frame`, and `frame_bytes()` is charged.
  Taken: no field of `Frame` or `Entry` changes; the bodies of `Frame::instructions` and
  `execute_next` may be rewritten.
- The stack: the story named a rule of the lane and an include in place of the task, and
  left out the procedure of `AGENTS.md` and the margin (one route had 60,768 bytes too few
  on 2026-11-27). Taken: `task test-stack-profiles` on the head merged with the wave branch
  is an acceptance line; a growth path is a function of its own with `#[inline(never)]`; no
  `#[inline(always)]` on `replace`, `push_entry`, `top_frame` or `put_index`, and no fast
  path in the bodies of `execute`, `put_index` or `call_instruction`.
- The stop rule: see above. The critic's form (stop above 80 %, the operator decides) was
  not taken; the threshold of 0.90 decides without a second question.
- Seven notes (the commonest `replace` has a count of 0 and `push_entry` reads the frame
  once on its common path; `to_numeric` and the operand reads as a fifth candidate; the
  step per prototype hop stays one call of `charge_steps(1)`; the CLI's default limits take
  the loop, and the source ends with a `print`; the signatures the open units call stay;
  the arm line of `I::PutIndex` stays; the task is not the benchmark of
  `story:bench-image-and-first-measurements`). Taken.

Not read by a critic: the revised text. The critics' replacement texts were taken nearly
word for word, the one decision above is the coordinator's, and the unit's adversary reads
the result against the story.

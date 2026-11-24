---
format: aep.planning-md/3
id: review-result:copies-held-round-1-acceptance
kind: review-result
status: active
title: Copies held across a call into script, round 1, the acceptance critic
relations:
- reviews: story:copies-held-across-a-call-into-script-are-charged
revision: 1
---
`needs-revision`

Tests 7 and 8 may not be red on the base; the other findings need smaller edits. I read the draft whole and the code it names, in the `swell-impl-copies-held-across-a-call-into-script-are-charged` worktree. I ran nothing.

**Blocker**
- Tests 7 and 8 (draft 332-345) compare `used()` with "the reading before the entry". Pushing 64 or more arguments grows the calling frame's operand stack from capacity 8 to 128 entries, and that growth is charged (`interp/mod.rs:239-259`). On the base the rise is that growth plus the activation's copy (`object/builtin_function.rs:704-719`). That already exceeds 2 × 64 × `size_of::<Value>()` (test 7) and 64 × `size_of::<Value>()` + 512 (test 8). As worded, the bracketed "[one copy]" and "[not]" cannot be seen red. Fix: take the baseline with a probe host function evaluated as the last argument expression, after the stack has grown, and assert the rise from that probe to `h` or `valueOf`.

**Warnings**
- Test 9 (346-350): all four holders run in one agent. After `apply` over 10,000 elements the kept stack keeps a capacity of at least 10,000 on the base. The later cases (`JSON.parse` over 10,000 numbers, the 10,000-String replacer list, `Function` with 64 names) then cannot grow it, so they cannot fail. Run each in a fresh agent. "Equals it before" also breaks if an unrelated `keep` outside the holder grows the stack; assert against a warm-up call instead. Sites 19-22 have no test that makes the new `reserve_kept` charge end in `Terminated(MemoryBudget)` (the AGENTS rule: a new charge has a test that reaches it).
- Instrument hole (202-203): the draft states that a plain `Vec` is invisible but gives no check that closes it. Add acceptance greps with their base counts. For example: `string_units(` outside `cfg(test)` must read 0 (5 engine callers on the base), `peek_values` 0 (3 on the base), `to_utf16()` listed. Also list the plain buffers that remain alive across a call, each with its bound: `units_of` Number units in the sort compare (at most 25), `Math.max`'s `coerced` (covered by `Charge`, not visible to the instrument), the `apply` list and `param_strings`.
- The collect-mode command (426-428) misses tests 1 and 8. The names `a_charge_is_given_back…` and `math_max_charges_its_list…` contain none of `copies|uncovered|charged|in_place|kept_stack|step_totals`, so "every test runs in both collection modes" does not hold.
- Moved existing tests (406-409): the search covers only `memory_bytes = ` (100 lines). Tests also fix their room with `set_limit(` (36 lines, 12 files), `charge_host_bytes` (33) and `high_water()` pins (`script.rs`, `object/array.rs`, `ops/objects.rs`). The draft says the unit "reports" a test that moves but not what it does with it. Name the commands and the action: widen the room in the test, or treat it as a defect of the story.
- Step order (113-118 against items 1-9): "step 1" and "step 4" mean the four-item order, but items 4 and 8 of the numbered list are other things. `INSTRUCTION_ARGUMENTS_COPIED` is created in item 5 but read red after step 1. A `ChargedVec` that carries no charge (the base state) contradicts test 1, which says a `ChargedVec` holds a `Charge`. Name a temporary constructor or define red another way.
- Time of the tests: the "Expected 0.05 s / 0.2 s" figures are predictions, and the acceptance has no command that measures them. AGENTS 275-285 sets the form: `slot sh -c` running the binary with `--exact <name> --test-threads 1` and reading `times`. Add that command and its bounds (1.5 s default mode, 2 s collect mode).
- Per-site coverage: sites 8, 10, 13, 15, 16 and 17 have no test that enters script with their builder alive. Extend `the_step_totals_of_the_changed_sites_are_unchanged` to read `uncovered_at_entry()` after each site of the table, so each decision in the table is checked.

**Notes**
- HOST-214 and the integration test (361-368): the second script (`[o, 1].join(s)` with a 65,536-unit `s`) answers `Throw` of a `RangeError` on the base too. 255 × 128 KiB is about 32 MiB and is not charged on the base. Only the first script is red there; say so. The lib twin (test 10) carries the red for the second script. It needs HOST-214's 10,000,000 steps, since `limits()` gives 1,000,000 and the first append of `s` costs 1,048,576 steps. State the limits in test 10.
- Test 6 spends about 4 × (65,536 + 65,536 + 65,535) ≈ 786,000 of 1,000,000 steps. It fits, but the margin is thin.
- Step 5 does not say whether the charged copy is also made for an ECMAScript callee. That path (`push_call_frame`) never calls back into script. If it is charged, a call near a memory limit can change its outcome, against "every result stays". State which paths charge.
- Sort's `units_of` units sit in "Change" 4 but not in "Not in this story". List them under the buffers bounded by a depth limit.
- Out of my lane: `INSTRUCTION_ARGUMENTS_COPIED` belongs beside the two existing counters in `object/ecmascript_function.rs`, which is not in `Writes:`.

Checked and sound, so not repeated above: sites 1-6, 9, 12, 18 and 19, and the not-in-story entries for concatenation (`ops/operators.rs:134`), `Error.prototype.toString` (`builtins/error.rs:410-412`), `indexOf`, `includes`, `lastIndexOf`, `localeCompare` and `split`. The draft's counts and line numbers match the base. The `HOST-215` and `+1` surface count follow from the known items. The draft has no forbidden words.

```findings
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 332
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'Tests 7 and 8 read used() against "the reading before the entry", but the 64-argument pushes grow the caller''s operand stack (interp/mod.rs:239-259), so the base can already exceed both thresholds and the tests may not be red; take the baseline from a probe as the last argument.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 346
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'Test 9 runs four holders in one agent, so the capacity left by apply makes the other three unable to fail on the base, and "equals before" is fragile; run each in a fresh agent, and add a test that a kept room over the memory limit ends in Terminated(MemoryBudget).'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 202
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The instrument cannot see a plain Vec and no check closes or lists that hole; add greps with base counts (string_units, peek_values, to_utf16 outside cfg(test)) and a list of the remaining plain buffers alive across a call, each with its bound.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 426
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The collect-mode filter regex does not match tests 1 (a_charge_is_given_back) and 8 (math_max_charges_its_list), so "every test runs in both collection modes" is false for them.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 406
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The search for existing tests near a limit covers only "memory_bytes = " (100 lines) and omits set_limit( (36), charge_host_bytes (33) and the high_water pins, and the unit is told to report a moved test, not what to do with it.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 113
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '"Step 1" and "step 4" of the order collide with the numbered items 1-9, INSTRUCTION_ARGUMENTS_COPIED is created in item 5 but read red after step 1, and an uncharged ChargedVec for the base state contradicts test 1; name the order and the temporary constructor.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 411
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The time of each new test is a prediction with no command; add the AGENTS measure form (slot sh -c, --exact, --test-threads 1, times) and its bounds to the acceptance.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 370
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'Sites 8, 10, 13, 15, 16 and 17 have no test that enters script with their builder alive; extend the step-totals guard to read uncovered_at_entry() after each site of the table.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 364
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The second script of HOST-214 answers Throw of a RangeError on the base too (about 32 MiB uncharged), so only the lib twin carries its red, and test 10 must state HOST-214''s 10,000,000 steps because limits() gives 1,000,000.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 173
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'Step 5 does not say whether the charged copy is also made on the ECMAScript-callee path, which never calls back into script and could move a result near a memory limit.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 168
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The at-most-25-unit Number, Boolean and null units of the sort compare stay plain and are alive across ToString of the other operand, but are not listed among the buffers bounded by a depth limit.'
```

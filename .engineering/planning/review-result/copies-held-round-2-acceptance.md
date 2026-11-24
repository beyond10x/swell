---
format: aep.planning-md/3
id: review-result:copies-held-round-2-acceptance
kind: review-result
status: active
title: Copies held across a call into script, round 2, the acceptance critic
relations:
- reviews: story:copies-held-across-a-call-into-script-are-charged
revision: 1
---
`needs-revision`

I read the draft whole, the first brief and first reading, the revise brief and the author's reply, and the code at `76aa4fc` in the `swell-impl-copies-held-across-a-call-into-script-are-charged` worktree. I ran nothing.

**First reading.** The blocker is fixed. Tests 7 and 8 now take their baseline from a `probe` passed as the last argument, so the operand stack has already grown (`interp/mod.rs:239-259`); the 50-argument arithmetic holds (draft 512-526). The collect filter `across_a_call` matches all 13 tests, and the search counts (86/61, 36/12, 33/7, 16/3) match the base. The measure command has the `AGENTS.md` form with its bounds. The other first-reading warnings are fixed, apart from the ones below.

**New in the revision**
- **Blocker, draft 632-639.** The acceptance runs `--lib` and, from the integration binary, only `budgets::`. The tests that can move sit in other modules: the 61 files with `memory_bytes = ` and `tests/jobs.rs:742`, which sizes a room from `heap_bytes()`. The draft's own searches find them and nothing runs them. "Every result stays" (draft 331-334) is not checked on the existing suites, and "report a test that moves" (606-611) cannot happen. Add `--test main` runs, default and collect, for the modules the searches name, as `own-keys-charged-before-the-list-exists.md:181-186` does.
- **Blocker, test 8, draft 527-542.**
  - **MemoryBudget case.** The bracket says the base answers `Normal` for `apply` over 100,000 elements with room 64 KiB. The base charges `vec_bytes::<Value>(2)` per element (`builtins/function.rs:216-222`) and ends in `MemoryBudget` near element 2,000, so that assertion is green on the base. After the change the list charge and the `reserve_kept` charge refuse at about the same element, so the case cannot show that the kept room's new charge was reached. Add a lib test that calls `reserve_kept` through `with_kept_room` under a tiny room. It asserts `Terminated(MemoryBudget)` and `used()` restored.
  - **Replacer holder.** On the base it is already green. `json/mod.rs:174-178` makes a `KeptRoom` and ends it on every path, so capacity and `used()` come back. Say so, or drop its red claim.
  - **Step cost.** 10,000 distinct Strings cost about n²/2 = 5×10⁷ steps (`json/mod.rs:262-267`) against 1,000,000. The normal end is never reached.
- **Warning, sizes in collect mode.** The sizes 10,000 and 100,000 (tests 8 and 9) and the `for(;;)` script under 4 MiB (draft 550-553) were sized for default mode. In collect mode every instruction collects, over a live heap that grows with each queued job. The 2 s bound will fail on the first run. Size them for collect mode from the start: a few hundred elements and a memory limit of about 256 KiB.
- **Warning, queue, draft 267-282 and 543-553.**
  - Point 7 does not say that the entry bytes are credited when the capacity growth for a promise job is refused (`jobs.rs:170-176` does it today). No test reaches that refusal. The integration script ends in `MemoryBudget` on the base too, through the promise objects and the entry charge.
  - `Charge` has `grow` only (draft 198), but `shrink_to(SPARE_ENTRIES)` needs a partial credit.
  - State that the queue has no fallback to the exact need. Test 9's 17th append depends on it.
- **Warning, draft 573-582.** The guard bracket "more than 0 at every site with a buffer" is false on the phase-A tree for sites 18 and 24-27, which the instrument does not see. Site 14 holds an empty builder across its call. List per site what it reads, and state that sites 24-27 are covered by test 8 only.
- **Warning, phases, draft 173-186, 472 and 688.**
  - Test 1 does not compile until point 1 exists, and it sits in the same lib test binary as tests 2-10, so none of them can be built and seen red. Write test 1 at the start of phase C.
  - Point 10, the lint, belongs to no phase. It has to come after C, so every `#[expect]` sits on a site that remains.
  - The acceptance has no "before" for the lint: with `clippy.toml` and no `expect`, clippy should print N `disallowed_methods` warnings, and 0 after.
- **Warning, draft 720-723.** `on_stack` is a `BTreeSet<(u32, u32, u32)>` (`builtins/json/mod.rs:97`), not a hash table. Its allowance text ("spare buckets, next power of two") has no basis. A B-tree's nodes grow per insert and the insert is infallible. State that allowance truthfully, or convert it to a sorted `ChargedVec`.
- **Warning, draft 367-368.** The paragraph "Queued jobs" in `engine.md` says more than two sentences must change: `:1602`, `:1624-1631`, and the "Rejected: charging a host microtask … charging the queue's capacity as an arena's" clause. Left as is, the design text contradicts itself. The same text in `host/jobs.rs:8-13` is covered.
- **Note.** The branch "charge what the allocator gave past the request" and the `shrink_to` for a refused excess (draft 208-209) is unreachable. `Vec::capacity()` reports the requested size. Drop it, or state it as defensive with no test.
- **Note.** Lines 280, 427, 663 and 666 still call D1 open ("default", "without D1"). It is decided: write the variant as taken.
- **Note.** The nested probe (draft 668-682) does not say how `$b` is found. After phase C it should say what passes: exit status 0 and a listed completion with `heap_bytes()` at most 64 MiB.

**Lint.**
- **Gate.** `Taskfile.yml:457` runs `cargo clippy --workspace --all-targets --locked -- -D warnings` from the root. The only `clippy.toml` is at the root, with four `allow-*-in-tests` keys. I did not run clippy, so I could not confirm the next point. Clippy looks for the file upward from each package's manifest directory. A file in `crates/swell` should therefore replace the root one for `swell`, and the draft repeats the four keys. A site with an `expect` that does not fire would warn and fail `-D warnings`, so the sites prove each path fires. The `<[T]>::to_vec` path is the only one I cannot confirm.
- **Count.** The count of 66 sites outside tests and 74 with them matches the base (`grep`: 74 in `src`, 7 in `tests`).

**Size.** The story can be built as one piece. If it is cut, the lint is the part to take out. It adds `#[expect]` at 66 sites across the crate. Any other unit that adds a `try_reserve*` and merges after this one goes red, and the draft names only Math. The queue (draft 267-282) is the other separable part: it touches only `host/`, one test and the docs.

```findings
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 632
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The acceptance runs only the lib and the budgets:: module of the integration binary, so the other modules whose pins can move (jobs, promises, builtins, the 61 files with memory_bytes) are never run and "every result stays" is unchecked; add runs of the modules the searches name in both collection modes.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 541
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The apply case over 100,000 elements with room 64 KiB already ends in MemoryBudget on the base through its per-element charge (builtins/function.rs:216-222), and after the change the list charge and the kept-room charge refuse at about the same element, so no test shows the new reserve_kept charge reached; add a lib test that calls reserve_kept through with_kept_room under a tiny room and asserts Terminated(MemoryBudget) with used() restored.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 535
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The replacer holder is green on the base (json/mod.rs:174-178 ends its room on every path) and 10,000 distinct Strings cost about 5x10^7 steps (json/mod.rs:262-267) against 1,000,000, so the normal end is never reached; size it to a few hundred and drop its red claim.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 551
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The sizes of tests 8 and 9 (10,000 and 100,000 elements, a for(;;) of jobs under 4 MiB) are sized for default mode, and in collect mode every instruction collects over a growing heap, so the 2 s bound fails; size them for collect mode from the start.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 275
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The credit of a promise job''s entry bytes when the capacity growth is refused is not stated and no lib test reaches it (the integration script ends in MemoryBudget on the base too), Charge has no partial release for shrink_to, and whether the queue has the exact-need fallback is not stated although the 17th append of test 9 depends on it.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 582
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '"More than 0 at every site with a buffer" is false on the phase-A tree for sites 18 and 24-27 (not seen by the instrument) and site 14 (empty builder); list per site what the guard reads and say sites 24-27 are covered by test 8 only.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 472
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'Test 1 does not compile before point 1 and shares the lib test binary with tests 2-10, so none can be seen red in phase B, and point 10 (the lint) belongs to no phase; write test 1 at the start of phase C and put the lint last, with the clippy count before (N disallowed_methods warnings) and after (0).'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 720
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'on_stack is a BTreeSet<(u32, u32, u32)> (builtins/json/mod.rs:97), not a hash table, so the allowance text about spare buckets and a power of two is false; state the B-tree''s per-insert node growth, or convert it to a sorted ChargedVec.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 367
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The "Queued jobs" paragraph of engine.md also says at :1602 and :1624-1631 that a host microtask and the spare capacity are not charged and rejects charging the queue''s capacity, so changing two sentences at :1599-1630 leaves the design text contradicting itself.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 208
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '"Charges what the allocator gave past the request" and the shrink_to of a refused excess are unreachable because Vec::capacity() reports the requested size; drop them or state them as defensive with no test.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 280
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'D1 is still written as open with a default (lines 280, 427, 663, 666) although the variant is decided; write it as taken.'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 675
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The nested probe does not say how $b is found, and after phase C it states an expectation instead of a pass line (exit 0, a listed completion, heap_bytes() at most 64 MiB).'
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 304
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The lint adds an #[expect] at 66 sites across the crate and makes every other unit that merges after it and adds a try_reserve* go red, so it is the part to cut into its own unit built on the join; the queue (host/ only) is the other separable part, and the story is buildable as one piece.'
```

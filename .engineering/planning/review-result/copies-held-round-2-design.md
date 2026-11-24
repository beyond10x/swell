---
format: aep.planning-md/3
id: review-result:copies-held-round-2-design
kind: review-result
status: active
title: Copies held across a call into script, round 2, the design critic
relations:
- reviews: story:copies-held-across-a-call-into-script-are-charged
revision: 1
---
needs-revision

Fixed from the first reading: Math is out (lines 728-733), all five factor-2 lists are converted (211-214), and the `reserve_vec` exact-need fallback is there for `ChargedVec` (205-207). `+` and `Error.prototype.toString` now make one n+m buffer (222-225), `KeptRoom` holds a `Charge` (284-287), and `padStart` reads its filler in place (232-235). The design sentence names `Charge`, `ChargedVec` and `reserve_vec` (342-345). The notes on the cell are in (191-197, 208). Read-only; nothing run.

**Blockers**
1. **Phase B cannot show tests red (182-184, 472-473, 548-549, 688-689).** Test 1 is "red by not compiling", and test 9 names `HostError::MemoryBudget`, which only point 7 adds. Both are lib tests, so the whole lib test binary fails to compile at phase B. No lib test, tests 2-10 included, can then be seen red by an assertion. Fix: phase A also adds `Charge`, `hold`, `reserve_vec` and the `HostError` variant as no-op stand-ins. Tests 1 and 9 are then red by assertion.
2. **The promise-job growth refusal has no stated credit and no test (267-280, 543-553).**
   - `enqueue_promise_job` charges the entry bytes first (`host/jobs.rs:171-172`), then the capacity. A refused capacity charge must credit the entry bytes, and a failed `try_reserve_exact` must give back the grown capacity charge. The draft says only "nothing queued".
   - The lib test reaches only the host-microtask refusal.
   - The `budgets` test is green on the base, because the entry bytes are already charged there. Its `f` never runs inside the `for(;;)`, and the run may end on heap objects before the queue refuses.
   - Fix: write the credits in point 7, and add a lib test with `set_limit` leaving room for the entry but not the growth. Assert `used()` before and after, and that nothing is queued.

**Warnings**
3. **Queue growth (268-271).** There is no exact-need fallback when the doubling is refused, though `reserve_vec` has one. The queue is a `VecDeque`, so it is not a `ChargedVec`. After `shrink_to(64)` and `drop_microtasks`, the `Charge` must be reset to the capacity the allocator reports, and `Charge` has only `grow`. Say so, and give `Charge` a release.
4. **Test 8's termination case (541-542).** It is not red on the base. `create_list_from_array_like` charges 32 bytes per element before the kept growth (`builtins/function.rs:211-216`). So `apply` over 100,000 elements with room 64 KiB already ends in `Terminated(MemoryBudget)` on the base. Use a holder whose per-element charge goes entirely (`flat`, the replacer list), or say this case is green on phase A.
5. **`on_stack` is mislabelled (720-723).** It is a `BTreeSet<(u32,u32,u32)>` (`builtins/json/mod.rs:97`, inserted at `:623`), not a hash table, and `heap/map.rs:50` is `KeyedMap::insert`. The frame charge already includes `size_of::<(u32,u32,u32)>() * 4` (`:612-614`). The design text (353-358) does not list it, though its rule says "never left uncharged". State the real allowance and add it to the design text.
6. **The lint acceptance is not checkable (304-329, 649-650).**
   - It gives the count before (74 lines, 66 outside tests) but no expected count after, and no negative control (remove one `expect` and see a warning).
   - The 66 are lines, not Vec receivers: `heap/map.rs:48,79` and `host/jobs.rs:135` are on a `HashMap` and a `VecDeque`. The `expect` count will be lower. Give the formula, and have the unit report each flagged site.
   - Fallback for `<[T]>::to_vec`: if no clippy path matches it, drop it from the lint and list its 5 sites with a grep.
7. **Lint plumbing, as found.**
   - The gate runs `cargo clippy --workspace --all-targets --locked -- -D warnings` (`Taskfile.yml:457`, the only clippy step).
   - The only config is the root `clippy.toml` (the four `allow-*-in-tests` lines). `CLIPPY_CONF_DIR` is set nowhere; `.github/workflows` has no clippy command.
   - `crates/swell/Cargo.toml` has `[lints.clippy]` with 4 denies.
   - Clippy takes the nearest `clippy.toml` from the crate manifest directory and does not merge it with the root. A file in `crates/swell` is therefore read for that crate, and copying the four lines is needed, as the draft does. I could not run it.
   - Add that to the draft as the finding the author lacked.
8. **Join (327-329).** The lint turns the gate red for any unit that merges later and adds a `Vec` growth call. The draft names only the Math story. Say the coordinator adds the `expect` at each join, or the lint lands last (see 11).

**Notes**
9. `HostError::MemoryBudget` also belongs in the enum block of `docs/design/host-interface.md:582-590` and the error table at `:639`. Neither is listed under `Writes:` for that section.
10. The probe (668-683) uses `$b` without saying how it is obtained; give the `--no-run` form of `AGENTS.md:275-285`. Say that `ChargedVec` has no growing method other than `reserve_vec`, so a later builtin cannot regrow it past its charge.
11. **Size.** One piece is buildable, since the foundation is shared and the type changes are compiler-driven. The draft is about 37 named files plus about 24 files that get an `expect`. If cut, the cut is: (1) the foundation, readers, builder, `join`/`sort`, `call`/`new`, kept rooms, the five lists and HOST-215; (2) the queue with `HostError::MemoryBudget` (a public-surface change, needs `Charge`); (3) the lint last. Each part is green, and the rule is whole after (3).
12. The nested-join probe's expected result holds for the base: the 32 MiB separator copy per level reaches 4 GiB near level 128. In the changed code the separator is appended only after element 0 returns, so the nesting ends at `reentry_depth`. Out of my lane: the Math tree's uncommitted probe edits to `Accounting::charge` and `heap/mod.rs` must not reach the join.

```findings
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 182
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Test 1 (a lib test) does not compile before point 1 and test 9 names HostError::MemoryBudget before point 7, so the lib test binary does not build at phase B and no lib test can be seen red; phase A must add no-op Charge, hold, reserve_vec and the HostError variant."
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 275
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "A refused capacity charge after the entry bytes are charged (host/jobs.rs:171-172) must credit the entry bytes, and a failed try_reserve_exact must give back the capacity charge; the draft says neither and no deterministic test reaches the promise-job growth refusal (the budgets test is green on the base)."
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 268
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The queue's growth has no exact-need fallback when the doubling is refused and does not say its Charge is reset to the capacity the allocator reports after shrink_to and drop_microtasks; Charge has no release."
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 541
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The Terminated(MemoryBudget) case of test 8 is not red on the base because apply already charges 32 bytes per element before the kept growth (builtins/function.rs:211-216); use a holder whose per-element charge goes entirely or mark the case green on phase A."
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 720
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "on_stack is a BTreeSet (builtins/json/mod.rs:97), not a hash table, its frame charge already includes four entries, and the design text does not list it though its rule says nothing alive across a call is left uncharged."
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 317
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The lint acceptance states no expected count after, no negative control, counts lines that include non-Vec receivers (heap/map.rs:48,79, host/jobs.rs:135) as sites, and gives no fallback when no clippy path matches <[T]>::to_vec."
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 306
  category: design
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "The gate runs cargo clippy --workspace --all-targets --locked -- -D warnings (Taskfile.yml:457) with only a root clippy.toml, and clippy reads the nearest file without merging, so crates/swell/clippy.toml takes effect for that crate and must repeat the four lines; the draft should state this."
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 327
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "A crate-wide lint turns the gate red for every unit that merges after this one with a Vec growth call, and the draft names only the Math story; say who adds the expect at each join or land the lint last."
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 370
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "HostError::MemoryBudget must also be added to the enum block of host-interface.md:582-590 and the error table at :639, which are not listed under Writes."
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 668
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The probe command uses $b without saying how the binary path is obtained, and ChargedVec should expose no growing method other than reserve_vec so a later builtin cannot regrow it past its charge."
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  category: design
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: "One unit can hold the story; if cut, split as foundation with readers, builder, join/sort, call/new, kept rooms, lists and HOST-215, then the queue with HostError::MemoryBudget, then the lint last."
```

Worktree: `swell-impl-copies-held-across-a-call-into-script-are-charged`.

**What I read:** the brief, both first-reading reports with the revision brief and the author's reply, the whole 737-line draft, `host/jobs.rs`, `heap/accounting.rs`, `object/builtin_function.rs:325-445`, `builtins/function.rs:160-240`, `host/outcome.rs`, `taskfiles/host-surface.yml`, the clippy configuration (Taskfile, root `clippy.toml`, `Cargo.toml`s, `.cargo`, workflows), and the Math tree's `coerce_all` (`swell-impl-math-constants-and-exact-functions`).

**What I could not establish:** whether clippy 1.99.1 accepts a path for `<[T]>::to_vec` (no run). The actual number of `expect`s needed (a count by receiver type). Whether each nested script ends as the draft expects (not run; reasoned from `join`'s order only).

---
format: aep.planning-md/3
id: review-result:adversary-random-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on unit random of wave big-1: green, 4 findings, 3 introduced'
relations:
- reviews: story:math-random
revision: 1
---
Adversary pass 1 on unit `random` of wave big-1 (`story:math-random`), the tree
`impl/math-random` at `ed59cd6b`, 2026-11-26. Verdict green: 7 cases added, none red,
4 findings (3 introduced, 1 undecided). The report as the adversary returned it:

```
unit: math-random (wave big-1), branch impl/math-random, head ed59cd6b plus uncommitted test cases
verdict: green
cases: executed 15→22, red 0
origin: introduced 3 / pre-existing 0 / undecided 1
wrote-outside-worktree: none
needs-coordinator: none
```

I could not make the unit fail. It matches the published xoshiro256** reference output and HOST-212/213. No path gives two realms one stream. But the unit's own suite misses two defects in its core, and my new cases catch both.

**1. Diff stat**
```
 crates/num/tests/random.rs        |  82 ++++++++++++++++++++++
 crates/swell/tests/math_random.rs | 143 ++++++++++++++++++++++++++++++++++++++
 2 files changed, 225 insertions(+)
```
Both paths are test files. Nothing is committed, and I changed no implementation file.

**2. Cases added.** Each was run alone before any suite run. All are green now, so there is no red output to quote.
- `crates/num/tests/random.rs:192` `adversary_the_reference_words_of_xoshiro256_starstar`: the first 10 words from state (1,2,3,4) match the reference implementation's output. I took the vector from `rand_xoshiro` 0.7.0 in the local cargo registry (`src/xoshiro256starstar.rs`, `fn reference`). PASS.
- `crates/num/tests/random.rs:215` `adversary_probe_a_rotation_mutant_passes_the_three_hand_words`: a copy of the step with `rotl(s3, 44)` still produces 11,520, 0 and 1,509,978,240, but not the reference's fourth word. PASS. It shows the unit's hand test at `:22` cannot catch that defect. It is evidence only; drop it if you keep the other cases.
- `crates/num/tests/random.rs:240` `adversary_the_derived_state_at_large_indices_is_the_designs_message`: `from_seed` equals the hand-built message at indices 255, 256, 2^16, 2^32, 2^56, u64::MAX−1 and u64::MAX. PASS.
- `crates/num/tests/random.rs:258` `adversary_the_numbers_at_the_ends_of_the_word_range`: word 2047 gives +0, word 2048 gives 2^-53, and the top word gives 1−2^-53 (below 1). PASS.
- `crates/swell/tests/math_random.rs:556` `adversary_the_realm_of_index_n_streams_from_seed_n`: for realm n the engine's stream equals `swell_num::random::Xoshiro256::from_seed(seed, n)`. Checked for n = 0, 1 and 4, after allocations and a collect, after a failed creation, and after a realm was dropped and collected. Also checked in a second agent with a different `memory_bytes` and `hash_key`. PASS in both collection modes.
- `crates/swell/tests/math_random.rs:599` `adversary_another_realms_math_random_moves_only_its_stream`: realm 1's `Math.random`, called from a realm-0 script and from a host call inside realm 0, takes realm 1's next values. Realm 0's stream is unchanged (9.4.1, 10.3.1). PASS in both modes.
- `crates/swell/tests/math_random.rs:644` `adversary_math_random_through_call_apply_bind_and_new`: `call`, `apply`, `bind` and `Function.prototype.call.call` each take the next value in order. There is no `prototype`, the prototype is `%Function.prototype%`, and the function is extensible. `new` on a bound copy throws a TypeError and does not move the stream. PASS in both modes.

**3. Suite runs** (after the cases existed). All runs exited 0; nothing is red.
- `cargo nextest run --locked -p swell-num --test main -E 'test(/^random::/)'`: 10 tests run, 10 passed, exit 0.
- `SLOT_TIMEOUT=20m cargo nextest run --locked -p swell --test main -E 'test(/^math_random::/)'`: 12 tests run, 12 passed, exit 0. Slowest is host_147 at 1.110 s wall.
- The same with `--features collect-every-safe-point`: 12 tests run, 12 passed, exit 0. Slowest is host_26 at 1.291 s wall.
- How I got 15→22: no implementor `cases:` line was available to me. The 22 is the count these three runs executed. The 15 is the same runs' lists with my 7 tests taken out by name (4 in num, 3 in swell); I did not make a separate run without them.

**4. Findings** (they cover ed59cd6b)
- **Rotation constant not tested.** `crates/num/tests/random.rs:22`: the three hand-worked words never read `s3` after its rotation, so a wrong `rotate_left(45)` passes them. My probe shows this. The other tests of the unit cannot catch it either: the num tests check derived states or bit balance, and the engine tests compare one engine stream with another. What reaches it: any future edit of `next_u64`. Fix: keep the reference-vector case at `:192`. Origin: introduced.
- **The index n is never pinned.** `crates/swell/src/realm/mod.rs:343`: no swell test checks that realm n uses `from_seed(seed, n)`. Every test in `math_random.rs` compares two engine streams, so deriving from `index + 1`, or any fixed shift, passes HOST-26/27/144/213 and `a_failed_creation_uses_its_index`. The design says "0 for the first". What reaches it: `create_realm` on every realm. Fix: keep the case at `math_random.rs:556`. Origin: introduced.
- **Only two indices are pinned.** `crates/num/tests/random.rs:94`: the DERIVED table holds n = 0 and 1 only, so a derivation that reads only the low byte of the index survives every test of the unit. What reaches it: realms beyond 255, which no test makes. Fix: keep the case at `:240`. Origin: introduced.
- **Collect-mode counts are fixed.** `crates/swell/tests/math_random.rs:119` and `:436`: in collect mode the counts drop to 1,000 for host_212 and 25/25 for host_147, and they ignore `SWELL_FULL_COUNTS`. So in collect mode the full counts never run, in the gate or in the `slow` lane. The default mode runs the full counts, and there each test proves its sentence as written in `requirements.txt` and the design. Leaving out the full count looks deliberate: the unit measured 2.5 CPU s at 100/100, already over the bound. Origin: undecided; this is judgement, with no run against the base.

**5. Attacked and not broken**
- `next_u64` matches the reference implementation's 10 words. `fix_zero` is correct. `next_f64` stays in [0, 1−2^-53] and never gives -0.
- The realm index: it is taken after `alive()` and before every failure point. Nothing else creates a realm: `create_realm` is the only caller of `initialize_host_defined_realm`, a host function cannot create a realm (`Cx` holds the agent), and test262's `createRealm` makes its realms before the run.
- `checked_add` overflow cannot be reached from outside: `realm_count` is `pub(crate)`.
- Determinism holds against hash_key, memory budget, a collect, earlier allocations and collect mode.
- `current_realm` inside a builtin is the function's [[Realm]] (`builtin_function.rs:709`/`732`).
- The added code has no `std::` path, no `unwrap`/`expect`/indexing, no cast from f64 and no float method.
- `.length`, `.name`, the descriptor and `new` are already covered by the unit's tests.

**6. Paths written outside the worktree:** none. I created the assigned scratch directory but wrote nothing in it. One helper script under the worktree's `.out/` was written and deleted unused.

```findings
- file: crates/num/tests/random.rs
  line: 22
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the three hand-worked words never read s3 after its rotation, so a wrong rotate_left constant in next_u64 passes every test of the unit; the reference vector case at random.rs:192 catches it
- file: crates/swell/src/realm/mod.rs
  line: 343
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: no swell test pins realm n to from_seed(seed, n), so deriving from index plus a fixed shift passes all unit tests; the case at math_random.rs:556 pins it
- file: crates/num/tests/random.rs
  line: 94
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the derived-state table holds n = 0 and 1 only, so a derivation reading fewer than 8 index bytes survives; the case at random.rs:240 checks indices up to u64::MAX
- file: crates/swell/tests/math_random.rs
  line: 119
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: the collect-mode counts of host_212 and host_147 are fixed reductions that bypass SWELL_FULL_COUNTS, so the full counts never run under collect-every-safe-point while the default mode proves both sentences at full count
```

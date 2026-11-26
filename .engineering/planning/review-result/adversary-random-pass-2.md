---
format: aep.planning-md/3
id: review-result:adversary-random-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on unit random of wave big-1: green, 1 note on the story text, 5 mutants caught'
relations:
- reviews: story:math-random
revision: 1
---
Adversary pass 2, the last pass, on unit `random` of wave big-1 (`story:math-random`), the tree `impl/math-random` at `96fe0ca3`, 2026-11-26.
Verdict green: 4 cases added, none red, 5 planted mutants each caught by a committed test, 1 finding (the story text). The report as the adversary returned it, with the paths of the workstation replaced:

unit: random (story math-random), branch impl/math-random at 96fe0ca3 plus uncommitted test cases
verdict: green
cases: executed 20→24, red 0
origin: introduced 1, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave/math-random/adversary-2/ws (deleted again, see 6)
needs-coordinator: yes (the story's acceptance text, finding 1)

I could not break the unit. The correction drops no assertion from pass 1. All five mutants are caught, and each is caught by a test the unit committed. The seed derivation matches the design and an independent SipHash-2-4. One small finding: the story's acceptance text cannot be met as written.

**One deviation from your brief.** I did not edit the implementation in the worktree, even briefly. The procedure forbids it, and so does `adversary-rules.md` ("in a copy, never in tracked files"). The sync only sends tracked files, so a copy under `.out/` would not have reached the build node. So I made a `git clone --shared` at 96fe0ca3 in my scratch directory, copied my test files into it, and planted each mutant there one after another. The pool built it. Every mutant was undone, and the clone is deleted.

**1. Diff stat**
```
 crates/num/tests/random.rs        |  75 +++++++++++++++
 crates/swell/tests/math_random.rs | 187 ++++++++++++++++++++++++++++++++++++++
 2 files changed, 262 insertions(+)
```
Both are test files. No implementation file is changed and nothing is committed.

**2. Cases added.** Each was run alone before any suite run, and each passed on the first run, so there is no red output to quote.
- `crates/num/tests/random.rs` `adversary2_the_derivation_agrees_with_std_siphash_2_4`:
  - The check uses `std::hash::SipHasher` (2-4, key read as two little-endian words), which is independent of `swell_num::siphash`.
  - First it reproduces two published values with key `00..0f`: `a129ca6149be45e5` for message `00..0e` (the paper's example) and `726fdb47dd0e0e31` for the empty message.
  - Then `from_seed` equals it for the seeds all-zero, all-0xff and `00..1f`, at indices 0, 1, 0x0807060504030201 and u64::MAX.
  - The 16 committed `DERIVED` words also equal it.
  - PASS, 0.012 s.
- `crates/swell/tests/math_random.rs` `adversary2_the_engine_streams_equal_the_designs_text`: the engine's first 16 values of realms 1 and 0 equal a computation written only from the design's text (std SipHash, the xoshiro step as the design writes it, `>>11 × 2^-53`), for the same three seeds. PASS: 0.044 s default, 0.117 s in collect mode.
- `adversary2_a_loop_of_calls_costs_the_steps_and_bytes_of_a_loop_of_math_max`:
  - 10,000 calls (40 in collect mode) compared with the same loop over `Math.max()`.
  - Measured: steps 200,015 against 200,015; `heap_bytes` change 0 against 0; `live_bytes` change 0 against 0.
  - After the loop, the next value is value 10,002 of the stream.
  - PASS: 0.618 s default, 0.721 s in collect mode.
- `adversary2_the_stream_resumes_after_an_interrupted_entry`: two values, then an interrupt from a host function (HOST-96), then a `Math.random()` call that never runs. The kept values are values 1 and 2 of the stream, and the next entry gets value 3. PASS: 0.03 s default, 0.04 s in collect mode.

**3. Suite runs** (after the cases existed)
- `cargo nextest run --locked -p swell-num --test main -E 'test(/^random::/)'`: 9 run, 9 passed, exit 0.
- `SLOT_TIMEOUT=30m cargo nextest run --locked -p swell --test main -E 'test(/^math_random::/)'`: 15 run, 15 passed, exit 0. Slowest is host_147 at 1.013 s.
- The same with `--features collect-every-safe-point`, plus `builtins_math::math_has_exactly_the_own_keys_of_its_tables_in_order`: 16 run, 16 passed, exit 0. This is the run after I lowered my loop to 40 calls in collect mode; it was 1.746 s at 100.
- `slot --timeout 30m task -d . -t taskfiles/host-requirements.yml host-requirements-check`: exit 0, "165 test, 56 pending", pending by slice has no `4:` left. The six HOST lines name tests that exist under exactly those names in `.out/host-requirements/tests.txt`.
- The count 20→24 comes from these runs with my 4 tests taken out by name (num 9−1, swell 15−3).

**Part A: kept cases and mutants**
- **Kept cases:**
  - Indices: all seven (255, 256, 2^16, 2^32, 2^56, u64::MAX−1, u64::MAX) plus the `assert_ne` of 256 against 0.
  - Realms: n = 0, 1 and 4, after a failed creation and a realm that was dropped and collected.
  - Second agent: other `memory_bytes` and `hash_key`.
  - `new` on a bound copy: the next value is still the 6th.
  - Hand-worked words: 11,520, 0 and 1,509,978,240 are still the first three of `REFERENCE`.

  Nothing was weakened.
- **Mutants** (`--no-fail-fast`):

| Mutant | Caught by |
|---|---|
| `rotate_left(45)` → 44 | `random.rs:44` the_first_ten_words |
| index read as `(index & 0xff)` | `random.rs:214` large indices, "index 256" (also my case at `:291`) |
| key and message halves swapped (key from `random_seed[16..32]`, message from `[0..16]`) | `random.rs:214` large indices, "index 255" (also my case at `:291`) |
| `next_f64` steps twice | `random.rs:61` the_number_of_a_word |
| `from_seed(.., index + 1)` in `realm/mod.rs:491` | `math_random.rs:582` "realm 4", `:648`, `:696` (and my 3 swell cases) |

  None survives. One thing to know about the swap: the `DERIVED` table, the test the design names "so that a change of the derivation fails", passes it, because both its seeds are uniform. Only the large-index test catches it.

**4. Findings** (they cover 96fe0ca3 plus my tests)
- **The story's acceptance cannot be met as written.** The story says `grep -rn "random" crates/swell/tests/builtins_math.rs` prints nothing, and that this file "is read and not written". It now prints two lines (`:85`, `:86`, `LATER_FUNCTIONS = ["sumPrecise", "random"]`). The unit had to write them: the own-keys test reads the file's own copy of the table, not the engine's. The intent holds, since neither line says `random` comes later. What reaches it: the acceptance check. The fix is in the story text, and that is the coordinator's. Origin: introduced.

**5. Attacked and not broken**
- The derivation matches HOST-213 and the design byte for byte: key `random_seed[0..16]`, message `[16..32]` + index as 8 bytes little-endian + byte i, the hash used as the word itself.
- Seeds all-zero and all-0xff are correct end to end.
- The all-zero state cannot be reached. A seed would need four zero SipHash words, and xoshiro never moves a non-zero state to zero. The largest index `create_realm` can reach is u64::MAX−1, because `checked_add` faults at u64::MAX.
- The cost matches the design's sentence over 10,000 calls (steps and both heap readings equal to `Math.max()`).
- The generator state has one reader and writer (`math.rs:351-353`). `RealmRecord` is not `Clone`, nothing takes a snapshot, and nothing can re-enter between the get and the set (no hook, no user code).
- After an interrupt, the stream resumes at the right value.
- `Math` own keys end with `sumPrecise,random`, and the full-order test passes.
- Stale greps 1 and 2 print nothing, as does the forbidden-method grep. `pending slice 4$` went from 6 to 0. `Cargo.lock` and the `Cargo.toml` files are unchanged.

**6. Paths written outside the worktree**
- `<scratch>/wave/math-random/adversary-2/` was created and is empty now.
- `.../adversary-2/ws` was the mutant clone, about 481 MB with the `target/` the pool sent back. I deleted it.
- Inside the worktree, `host-requirements-check` rewrote `.out/host-requirements/tests.txt` (build output, ignored by git). `git status --short` shows only my two test files.

```findings
- file: .engineering/planning/story/math-random.md
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the acceptance requires grep -rn "random" crates/swell/tests/builtins_math.rs to print nothing and calls that file read-only, but the own-keys test needs "random" in its own LATER_FUNCTIONS table (lines 85-86), so the item cannot pass as written and the story text needs the change
```

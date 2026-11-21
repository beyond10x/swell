# Slice 1, wave 1 of 6: the early parts of `swell-num` and `swell-unicode`

One wave of the [wave plan](waves.md), 2026-11-21. One unit, implemented by a sub-agent on
Opus in its own worktree; the coordinating session wrote no code. It added the include
`taskfiles/unicode-tables.yml` and `- task: unicode-tables-check` (after
`unicode-data-check`) to the root `Taskfile.yml`. The unit's `Cargo.lock` was taken as it is:
the unit was alone in its wave.

## Unit

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `num-and-unicode-early-parts` | `crates/num` (`swell-num`, 1,077 lines in `src/`): SipHash-1-3 with a caller-supplied key, the `NumericLiteral` grammar without BigInt and `StringToNumber`, `Number::toString` for radix 10, the day and time arithmetic of the Date clauses. `crates/unicode` (`swell-unicode`): the `Zs` table, 17 code points. `crates/unicode-gen`: the generator, with `--check`. `taskfiles/unicode-tables.yml` | 11 tests in 3 packages; `unicode-tables: 1 table(s) equal the generator output` | `488446e` |

The unit's claim was checked by the coordinator before its commit: `cargo test --locked -p
swell-num -p swell-unicode -p swell-unicode-gen` (11 passed), `slot task -d . -t
taskfiles/unicode-tables.yml unicode-tables-check` (exit 0) and `cargo clippy --workspace
--all-targets --locked -- -D warnings` (exit 0), run again in the unit's tree; and a search of
`crates/num/src` and `crates/unicode/src` for a transcendental `f64` function, `RandomState`,
`std::fs`, `std::env`, `std::net`, `std::time`, `unwrap`, `expect` and `panic!` found none.

## What went wrong, and what was decided

- **The story said "integer arithmetic only" for the Date functions; ECMA-262 does not.**
  `MakeTime` and `MakeDate` are specified in Number arithmetic, which is not associative, and
  test262 checks the order (`built-ins/Date/UTC/fp-evaluation-order.js`: 29312 and 34447360).
  The unit followed the clause; the calendar part (`DayFromYear`, month starts) is integer
  arithmetic. The story's sentence was wrong; the code is as the standard says.
- **Rust's shortest digits and ECMA-262's differ on an exact tie.** The first green run of
  the round-trip test failed on -174098110762417.125: 17 digits are needed, the value is
  halfway between two candidates, the engine takes the even one (Note 2 of
  `sec-numeric-types-number-tostring`) and Rust's `{:e}` the upper one. The test draws
  200,000 finite bit patterns; 49 are such ties. It now allows a difference from Rust only
  where the tie is proven in integer arithmetic and the engine's last digit is even. This
  answers an item of the design's "Not checked"; the design says so now (decision 6).
- **`MakeDay` above a year of 2^53 in magnitude answers NaN without deciding whether a Number
  falls on the day.** Left for slice 4, which owns `Date`; listed in the wave plan under what
  later slices take over.
- **The round-trip test takes 7 to 9 seconds in the debug profile** (6.71 s in the
  coordinator's run). It is part of the gate's `test` step. No profile override was added.
- **No planted-defect script under `tests/gate/` for `unicode-tables-check`**: the story's
  `Writes:` line has none. The refusal is held by the test `check_fails_on_a_changed_table`
  of `swell-unicode-gen`, which runs the generator with `--check` on a copy with a changed
  table.
- The brief said the unit runs beside a unit of the foundation; the story says it is alone in
  its wave. Both were true: the other unit belonged to the foundation's wave 3 and shared no
  file with it.

## The gate on the wave branch

`slot task check`, exit 0:

```
gate: variables in effect: SPEC_REFUSALS_MAX=32 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log
python-check: no Python file among 252 file(s)
forbid-unsafe-check: 6 crate root(s), each with #![forbid(unsafe_code)]
Cargo.lock: 5 package(s), none from outside this repository
test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked
data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present
unicode-tables: 1 table(s) equal the generator output
swell v1 — 2 file(s), 3 scenario(s), valid
61 scenario(s) (3 authored), 32 refusal(s), written to .out/suite.json
  61 scenarios: 61 passed, 0 failed, 0 error, 0 unsupported
host-requirements: 159 requirement(s), 159 line(s): 0 test, 159 pending, 0 omitted; 50 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 1:68 2:5 3:5 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1
19 test suites: 36 passed, 0 failed, 0 ignored
test262 total 53801 8982 7 44812 0 44812 0
test262: 221 directory lines sum to the total
test262: 22 slice lines equal the design
test262-errors 0
test262: 278 lines equal conformance/test262/expected.txt
gate-selftest: 28 passed, 0 failed
```

No test262 count moved and no host requirement closed: this wave delivers parts the later
waves of the slice build on.

## Where each finding went

- The tie rule of Rust's shortest digits: `docs/design/engine.md`, decision 6 and "Not
  checked", changed in this wave.
- The `MakeDay` gap: the wave plan, "What later slices take over", slice 4.
- A planted-defect script for `unicode-tables-check`: the wave plan, same list, with the
  backlog of the gate.

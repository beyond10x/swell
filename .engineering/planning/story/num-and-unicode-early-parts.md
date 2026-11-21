---
format: aep.planning-md/3
id: story:num-and-unicode-early-parts
kind: story
status: implemented
title: 'swell-num and swell-unicode, early parts: keyed hash, literal values, Number to string, the Zs table'
summary: SipHash, numeric literal and StringNumericLiteral values, Number::toString radix 10 and GetUTCEpochNanoseconds in swell-num; the generated Zs table in swell-unicode with its generator and gate task.
relations:
- decomposes: epic:slice-01-harness
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/num
- confidence: cited
  path: crates/unicode
- confidence: cited
  path: crates/unicode-gen
- confidence: cited
  path: taskfiles/unicode-tables.yml
- confidence: cited
  path: tests/check.rs
- confidence: cited
  path: tests/date.rs
- confidence: cited
  path: tests/literal.rs
- confidence: cited
  path: tests/siphash.rs
- confidence: cited
  path: tests/to_string.rs
- confidence: cited
  path: tests/zs.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T04:49:58Z", actor: "human:operator", revision: 4, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T04:49:58Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T05:19:53Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"test_result":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Slice 1 needs a few parts of `swell-num` and `swell-unicode` before their slices (design,
section 12, "A part before its slice"; the epic, open item "Which parts of `swell-num`,
`swell-unicode` and `swell-regexp` this slice takes early"). Measured on the pinned test262
checkout and the harness files the 31 cases of slice 1 include (see `story:test262-run-path`
for the 51 cases that move to later slices):

- the keyed hash of decision 4 (`engine.md`, "Collector" paragraph: "a SipHash written in
  `swell-num`, keyed from the host randomness at agent creation"), for every hash map of the
  engine;
- the value of a numeric literal (`sec-numericvalue`, `sec-literals-numeric-literals`): decimal,
  `0x`, and the legacy octal form refused in strict code by the parser;
- StringToNumber (`sec-stringtonumber`) on a string whose white space and line terminators
  are already trimmed, for `ToNumber` of a String (`assert-samevalue-zeros`, `+"..."`, `==`);
- `Number::toString(x, 10)` (`sec-numeric-types-number-tostring`), for `ToString` of a Number,
  which `assert.js` reaches in every message it builds (`assert._toString`);
- `GetUTCEpochNanoseconds` (`sec-getutcepochnanoseconds`), which the provided default of the
  time-zone hook uses (`docs/design/host-interface.md`, section 16 and 17; `HOST-124`, closed by
  `story:object-model-and-realms`).

Trimming before StringToNumber removes `StrWhiteSpaceChar`, which includes every `Zs` code point
(`sec-white-space`). That needs the `Zs` set from the pinned Unicode data, so `swell-unicode`
starts here with one generated table. Its data: `data/unicode/17.0.0/ucd/extracted/DerivedGeneralCategory.txt`;
`grep -E "; Zs" data/unicode/17.0.0/ucd/extracted/DerivedGeneralCategory.txt` prints seven
lines covering 17 code points: `0020 00A0 1680 2000..200A 202F 205F 3000`.

`swell-num` stays free of dependencies (design, section 2), so the trim is done in `swell`
(`story:object-model-and-realms`, `ops/conversion.rs`) with the `swell-unicode` lookup, and
`swell-num` parses a trimmed string. The lexer of `story:syntax-es5-and-strict` uses the same
lookup for `Zs` white space in source text (`sec-white-space`), so it depends on this story.

## Change

Failing tests first: the tests named below are written before the code and fail before it (the
crates do not exist).

Crate layouts, stated once here; later slices add modules beside these:

- `crates/num/` (package `swell-num`, no dependency, `#![forbid(unsafe_code)]`, the four clippy
  denials of `crates/swell/Cargo.toml`): `src/lib.rs`; `src/siphash.rs` (SipHash-2-4, keyed by
  16 bytes); `src/literal.rs` (the mathematical value of a decimal, hexadecimal, octal and
  legacy octal literal and of a `StringNumericLiteral`, rounded to a Number as 6.1.6.1 says);
  `src/to_string.rs` (`Number::toString` radix 10, the shortest digits that round-trip, then the
  layout of steps 5 to 12); `src/date.rs` (`MakeDay`, `MakeDate`, `MakeTime`,
  `GetUTCEpochNanoseconds`, integer arithmetic only). Slice 4 adds `radix.rs`, `math.rs`,
  `random.rs`; slice 12 adds `bigint.rs`.
- `crates/unicode/` (package `swell-unicode`, no dependency): `src/lib.rs` with
  `pub fn is_space_separator(c: u32) -> bool`; `src/tables/mod.rs`; `src/tables/zs.rs`
  (generated, header line naming the Unicode version and the source file). Slice 5 adds the
  tables of decision 8 beside `zs.rs`.
- `crates/unicode-gen/` (package `swell-unicode-gen`, binary, no dependency, never shipped):
  `src/main.rs` reads `data/unicode/17.0.0/ucd/extracted/DerivedGeneralCategory.txt` and writes
  `crates/unicode/src/tables/zs.rs` as a sorted list of inclusive ranges; with `--check` it
  writes nothing and exits 1 when the file differs from what it would write.

Tests:

- `crates/num/tests/siphash.rs`: `siphash24_matches_the_reference_vectors` (the 64 vectors of
  the reference implementation for keys `00..0f` and messages of length 0 to 63),
  `different_keys_give_different_hashes`.
- `crates/num/tests/literal.rs` (citing `sec-numericvalue` and `sec-stringtonumber`):
  `decimal_literals_round_to_nearest_even` (among them `9007199254740993`,
  `2.4703282292062328e-324`, `1.7976931348623158e308`, `1e400` to infinity),
  `hex_and_octal_literals`, `string_numeric_literal_accepts_infinity_signs_and_empty`
  (`""` is 0, `"-Infinity"`, `"+.5"`, `"0x10"`, `"1e"` is NaN, `"-0x1"` is NaN).
- `crates/num/tests/to_string.rs` (citing `sec-numeric-types-number-tostring`):
  `number_to_string_layouts` (`-0` to `"0"`, `1e21` to `"1e+21"`, `123e-20` to `"1.23e-18"`,
  `0.000001` to `"0.000001"`, `1e-7` to `"1e-7"`, NaN, infinities, `5e-324`),
  `to_string_then_parse_round_trips` (200,000 bit patterns from a fixed seed through the crate
  own SipHash, finite ones only, `parse(to_string(x))` has the bits of `x`).
- `crates/num/tests/date.rs` (citing `sec-getutcepochnanoseconds`):
  `epoch_nanoseconds_of_known_dates` (1970-01-01, 2000-02-29, -271821-04-20, 275760-09-13).
- `crates/unicode/tests/zs.rs` (citing `sec-white-space`): `space_separators_are_the_17_code_points`
  (all of 0 to 0x10FFFF checked against the 17), and `crates/unicode-gen/tests/check.rs`:
  `check_passes_on_the_committed_table`, `check_fails_on_a_changed_table` (a copy under
  `CARGO_TARGET_TMPDIR`).

Gate task, in a new include file `taskfiles/unicode-tables.yml`, task `unicode-tables-check`:
runs `cargo run --locked -p swell-unicode-gen -- --check` and prints
`unicode-tables: 1 table(s) equal the generator output`. The coordinator includes the file in
the root `Taskfile.yml` and adds `- task: unicode-tables-check` to `check` after
`unicode-data-check`.

`Cargo.lock`: the unit regenerates it in its own tree (three packages added, none from outside
the repository), because its build needs it, and runs its acceptance with `--locked` against
that lock. Alone in its wave, the unit lock is taken into the wave branch as it is.

Writes: `crates/num/` (new: `Cargo.toml`, `src/lib.rs`, `src/siphash.rs`, `src/literal.rs`,
`src/to_string.rs`, `src/date.rs`, `tests/siphash.rs`, `tests/literal.rs`, `tests/to_string.rs`,
`tests/date.rs`), `crates/unicode/` (new: `Cargo.toml`, `src/lib.rs`, `src/tables/mod.rs`,
`src/tables/zs.rs`, `tests/zs.rs`), `crates/unicode-gen/` (new: `Cargo.toml`, `src/main.rs`,
`tests/check.rs`), `taskfiles/unicode-tables.yml`, `Cargo.lock` (regenerated in the unit tree,
because its build needs it; this story is alone in its wave, so the unit lock is taken as it is).

## Acceptance

`cargo test --locked -p swell-num -p swell-unicode -p swell-unicode-gen` exits 0 and runs the
eleven tests named in Change, and `slot task -d . -t taskfiles/unicode-tables.yml
unicode-tables-check` exits 0 and prints `unicode-tables: 1 table(s) equal the generator output`.

## Out of scope

`Number::toString` for radix other than 10, `toFixed`, `toPrecision`, `toExponential`, `Math`,
the random generator, the rest of the date arithmetic (slice 4); every other Unicode table
(slice 5); `BigInt` (slice 12). Trimming white space and calling these functions from the engine
(`story:object-model-and-realms`, `story:bytecode-compiler`), and the lexer use of the table
(`story:syntax-es5-and-strict`). The root `Cargo.toml` and `Taskfile.yml` (the coordinator).

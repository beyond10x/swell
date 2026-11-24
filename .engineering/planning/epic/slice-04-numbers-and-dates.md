---
format: aep.planning-md/3
id: epic:slice-04-numbers-and-dates
kind: epic
status: active
title: 'Slice 4, numbers-and-dates: swell-num, Number, Math, Date'
summary: 'swell-num with Number, Math, Date, the operator **, parseInt and parseFloat, deterministic and host-timed: 1,612 cases and 3,184 runs.'
relations:
- depends_on: epic:slice-03-builtins-core
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T11:52:42Z", actor: "human:operator", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-24T11:52:42Z", actor: "human:operator", revision: 5}
---
## Context

Slice 4 of 19, `numbers-and-dates` (`docs/design/engine.md`, section 12). It needs slices 1 to 3: the
engine, the language and the core builtins that `Number`, `Math` and `Date` extend. It builds
`swell-num` and the library that uses it: `Number`, `Math`, `Date`, `parseInt`, `parseFloat`.
After slice 4 a script that touches no DOM and uses only ES5 can run (section 12, "For surf").
Governing decisions (`docs/design/engine.md`, decision N): 6 (numbers and dates, whole), 2
(`swell-num` depends on nothing), 9 (clock, time zone and randomness from the host), 13.

## Outcome

- `Number::toString` (6.1.6.1.20) for every radix 2 to 36, with an exact shortest-digits algorithm
  over `swell-num`'s big integers, laid out as the algorithm says: no exponent for a radix other
  than 10, and for radix 10 an exponent when n is outside -5 to 21 (written `1e+21`); `-0` prints
  `0` (decision 6).
- `toFixed`, `toExponential`, `toPrecision` over the exact decimal expansion, with the rounding
  ECMA-262 states ("pick the larger n").
- Parsing: the grammars of `StringToNumber` and of numeric literals (hex, octal, binary,
  separators, `Infinity`, whitespace) written here; the decimal digits found are converted by
  `str::parse::<f64>`. Digits after the 20th are not replaced with 0, so the result is the
  correctly rounded one.
- `Math`: `powi powf exp exp2 ln log log2 log10 cbrt hypot sin cos tan asin acos atan atan2 sinh
  cosh tanh asinh acosh atanh exp_m1 ln_1p` and the other functions `std` documents as
  non-deterministic are written in `swell-num` in plain `f64` arithmetic; `sqrt`, `abs`, `floor`,
  `trunc`, `round_ties_even` and `mul_add` come from `std`.
- `Math.random` draws from a generator in `swell-num`, seeded from the host's randomness.
- `Date`: date arithmetic (day, month, year from a time value) in `swell-num`; the current time from
  the host (HTML's `HostSystemUTCEpochNanoseconds` is the hook a browser gives); the time zone
  through `SystemTimeZoneIdentifier ( )`, `GetNamedTimeZoneOffsetNanoseconds`,
  `GetNamedTimeZoneEpochNanoseconds` and `AvailableNamedTimeZoneIdentifiers ( )`; the command-line
  host and the test262 runner answer "UTC" unless told otherwise.
- `Date.parse` accepts the format of 21.4.1.32 and the forms `Date.prototype.toString` and
  `toUTCString` produce, nothing else.
- `parseInt` and `parseFloat`.
- The operators `**` and `**=` parse, compile and run over the one function
  `swell_num::math::exponentiate`.
- `Math` has its constants and 22 functions, plus `Math.sumPrecise` and `Math.random`.
- `Number` has `isFinite`, `isInteger`, `isNaN` and `isSafeInteger`, with `toFixed`,
  `toExponential`, `toPrecision` and `toLocaleString`.
- `Date` has its object, the clock, the constructor, the getters, the 14 setters, the text
  methods and `Date.parse`.

### Open, to be settled by this epic's first story

- Which published argument-reduction and polynomial methods the `Math` functions use. Decision 6
  names none, and gives no accuracy bound.
- Whether Rust's shortest radix-10 digits pick the closest candidate when several are shortest:
  "not checked" (decision 6). It decides whether `{:e}` can be the test oracle in every case.
- Which slice writes the keyed hash and the random generator of `swell-num` (decision 2 lists
  them there); the heap's index maps need the hash from slice 1 (decision 4).
- How `ToString` of a Number is done in slices 1 to 3, before this slice.
- Where the BigInt part of `swell-num` is written: decision 2 lists it in `swell-num`, section 12
  builds `BigInt` in slice 12.

## Counts

- Slice 4 (`numbers-and-dates`): 1,612 cases, 3,184 runs (section 12 table; `test262-slice 04 1612 3184 0 0 1612 261 1351 0` on `wave/s10-1` at `57e49c4`).
- Directories the design names for it (paths under `test/`, cases): `Date` 492, `Number` 313,
  `Math` 284.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/built-ins/Number/ 4`
  - `dir test/built-ins/Math/ 4`
  - `dir test/built-ins/Date/ 4`
  - `dir test/built-ins/parseInt/ 4`
  - `dir test/built-ins/parseFloat/ 4`
  - no `feature` line and no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- `BigInt` and its arithmetic: slice 12. `Atomics` on numbers: slice 14.
- `Temporal`: slice 20, with a design of its own after slice 19 (decision 1).
- `Intl.DateTimeFormat` and the rest of ECMA-402 (`toLocaleString` beyond ECMA-262's
  locale-independent definitions): slice 21, open item A.
- Any `Date.parse` fallback format beyond the forms above: not taken (decision 6).
- Regular expressions: slice 5.
- Scenarios of the executable specification for the host requirements of this slice: a story of
  the specification, outside this epic.

## Constraints

- No ambient authority: no clock, time zone, randomness or environment access in the engine, and no
  `std` transcendental `f64` function in it (`AGENTS.md`, "Rules"; decision 6).
- Nothing third-party in what ships, so no port of an existing C maths library;
  `#![forbid(unsafe_code)]` (`AGENTS.md`, "Rules"; decision 6).
- No panics: no `unwrap`, `expect` or unchecked indexing, and overflow checks on, in `swell-num`
  (`AGENTS.md`, "Rules"; decision 13).
- Every loop charged to the step budget, every allocation to the memory budget; a new limit gets a
  test that reaches it (`AGENTS.md`, "Rules").
- A builtin's module cites its clause (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 4 reads 1,612 cases and 3,184 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. The results of every `Math` function the engine writes are compared bit for bit against stored values, and the comparison passes (decision 13, non-determinism).
6. The same scripts run twice, and on every release platform, print the same bytes (decision 13, non-determinism).
7. `Number.prototype.toString` in radix 10 is checked in tests against the digits of Rust's `{:e}` output (decision 6).
8. With a host that answers a fixed clock and time zone, `new Date()` and `Date.now()` return that clock and nothing else; the engine reads no clock of its own (decisions 6 and 9).
9. `Date.parse` of a string outside the forms of 21.4.1.32, `toString` and `toUTCString` returns `NaN` (decision 6).

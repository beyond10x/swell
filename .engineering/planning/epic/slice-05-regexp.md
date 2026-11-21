---
format: aep.planning-md/3
id: epic:slice-05-regexp
kind: epic
status: draft
title: 'Slice 5, regexp: the regular expression engine and RegExp'
summary: 'swell-regexp, swell-unicode, RegExp and the string methods that use it: 2,118 cases and 4,234 runs.'
relations:
- depends_on: epic:slice-04-numbers-and-dates
revision: 1
---
## Context

Slice 5 of 19, `regexp` (`docs/design/engine.md`, section 12). It needs slices 1 to 3: the engine and
the `String` builtins whose methods call `RegExp`; slice 4 precedes it in the order of dependencies.
It builds `swell-regexp` and `swell-unicode` and the library that uses them: `RegExp` and the string
methods `match`, `matchAll`, `replace`, `replaceAll`, `search`, `split`. After slice 5 a script with
no DOM can use regular expressions (section 12, "For surf"). Governing decisions
(`docs/design/engine.md`, decision N): 7 (regular expressions), 8 (Unicode data), 2, 3, 13.

## Outcome

- `swell-regexp` (decision 7): the pattern parser with the early errors of 22.2.1 and B.1.2, a
  compiler to a small instruction set, and a backtracking matcher over UTF-16 with an explicit
  backtrack stack and no Rust recursion, every step charged to the step budget.
- In scope by decision 7: named groups (with duplicate names), look-behind, the flags `d`, `g`, `i`,
  `m`, `s`, `u`, `v` and `y`, modifiers (`(?i:...)`), Unicode property escapes and properties of
  strings under `v`, set operations under `v`, `RegExp.escape`, and the Annex B pattern grammar
  (B.1.2).
- Case folding: simple case folding from `CaseFolding.txt` under `u` and `v`, and `toUppercase` under
  neither (22.2.2.7.3 `Canonicalize`).
- Pattern nesting is bounded at parse and reported as a `RangeError` (decision 3).
- Regular expression literals checked by `swell-regexp` from `swell-syntax` (decision 2).
- `swell-unicode` and the tool `swell-unicode-gen` (decision 8): data files vendored under
  `data/unicode/<version>/` with a `SOURCE` file of addresses and SHA-256 sums; tables written to
  `crates/unicode/src/tables/*.rs` and committed; the gate regenerates them into `.out/` and fails
  on any difference. Tables for `ID_Start`, `ID_Continue`, white space, `toUpperCase` and
  `toLowerCase`, case folding, normalization, property escapes (names and values) and properties of
  strings, each from the file decision 8 names. The version is 17.0.0 (open item C(a), standing per
  `AGENTS.md`); every section that depends on it carries the deviation as an omission.
- `RegExp`, `RegExpStringIteratorPrototype` and the six string methods.

### Open, to be settled by this epic's first story

- Where the 17.0.0 emoji sequence files for properties of strings are published ("not checked",
  decision 8), and the text of the Unicode data licence ("not checked").
- Whether ECMA-262's wording on implementation limits allows the `RangeError` for pattern nesting
  ("not checked", decision 3).
- Which of `toUpperCase`, `toLowerCase` and `normalize` (slice 3's `String`) use these tables, and
  how they run before slice 5 (decision 8 puts the tables here).
- How B.1.2 (decision 7, in this slice) and slice 18 ("Annex B") divide the Annex B regular
  expression cases between them.

## Counts

- Slice 5 (`regexp`): 2,118 cases, 4,234 runs (section 12 table).
- Directories the design names for it (paths under `test/`, cases): `RegExp` 1,626, `String` 252,
  `literals` 240. Inventory: `built-ins/RegExp` 1,879 cases in all slices; the 469 files under
  `built-ins/RegExp/property-escapes/generated/` say `Unicode v17.0.0`; `regexp-unicode-property-escapes`
  is carried by 681 cases.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/built-ins/RegExp/ 5`
  - `dir test/built-ins/RegExpStringIteratorPrototype/ 5`
  - `dir test/language/literals/regexp/ 5`
  - `dir test/built-ins/String/prototype/match/ 5`
  - `dir test/built-ins/String/prototype/matchAll/ 5`
  - `dir test/built-ins/String/prototype/replace/ 5`
  - `dir test/built-ins/String/prototype/replaceAll/ 5`
  - `dir test/built-ins/String/prototype/search/ 5`
  - `dir test/built-ins/String/prototype/split/ 5`
  - `feature regexp-named-groups 5`
  - `feature regexp-lookbehind 5`
  - `feature regexp-dotall 5`
  - `feature regexp-unicode-property-escapes 5`
  - `feature regexp-v-flag 5`
  - `feature regexp-match-indices 5`
  - `feature regexp-modifiers 5`
  - `feature regexp-duplicate-named-groups 5`
  - `feature RegExp.escape 5`
  - `feature String.prototype.matchAll 5`
  - no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- `legacy-regexp` (`RegExp.$1` and friends): stage 3, excluded (decision 7, `exclude` line).
- An automaton for patterns without backreferences and look-around: rejected for now, a measured
  optimisation later (decision 7).
- `Symbol.match`, `Symbol.replace`, `Symbol.search`, `Symbol.split`, `Symbol.matchAll` as
  well-known symbols: slice 6 (the `feature` lines); a case that needs them is counted there.
- Annex B beyond the pattern grammar: slice 18.
- ECMA-402 (`Intl`): slice 21 (open item A).
- `Temporal` (slice 20), other stage 3 features, concurrent agents.

## Constraints

- Nothing third-party in what ships: no regular expression crate, no Unicode crate; the tables are
  generated from vendored data by `swell-unicode-gen`, which never ships (`AGENTS.md`, "Rules";
  decision 8). `#![forbid(unsafe_code)]`.
- Every matcher step is charged to the step budget and every backtrack-stack growth to the memory
  budget; a new limit gets a test that reaches it (`AGENTS.md`, "Rules"; decision 7).
- No panics, no `unwrap`, `expect` or unchecked indexing in `swell-regexp` and `swell-unicode`
  (`AGENTS.md`, "Rules").
- A deliberate omission is recorded where the gate reads it (`sections.txt`, `features.txt`); the
  Unicode version deviation is listed as an omission of each section that depends on it
  (`AGENTS.md`, "Rules"; open item C).
- A builtin's module cites its clause (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 5 reads 2,118 cases and 4,234 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. `/(a+)+$/` matched against 10,000 `a` characters followed by `!` ends with `Terminated(StepBudget)`, never in a hang (decision 7).
6. A pattern nested deeper than the host's pattern nesting limit is reported as a `RangeError` at parse, with no Rust stack overflow (decision 3).
7. The gate regenerates the `swell-unicode` tables from the vendored data into `.out/` and fails on any difference from the committed tables; every vendored data file has its address and SHA-256 in `SOURCE` (decision 8).

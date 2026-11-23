---
format: aep.planning-md/3
id: epic:slice-05-regexp
kind: epic
status: active
title: 'Slice 5, regexp: the regular expression engine and RegExp'
summary: 'swell-regexp, swell-unicode, RegExp and the string methods that use it: 2,118 cases and 4,234 runs.'
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T03:25:08Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T03:25:08Z", actor: "human:operator", revision: 4, executor: "agent:coordinator"}
---
## Context

Slice 5 of 19, `regexp` (`docs/design/engine.md`, section 12). It needs slices 1 to 3: the engine and
the `String` builtins whose methods call `RegExp`; it is delivered before slice 4 (the order of delivery is 6, 5, 7, 10, 4, 9), needs nothing of it, and at its close moves 15 cases that use `Date`, `Math.pow`, `parseInt` or `**` to slice 4.
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
- Pattern nesting is bounded at parse and reported as a `SyntaxError` that names `parser_nesting` (decision 3; `HOST-142`, `HOST-143`).
- Regular expression literals checked by `swell-regexp` from `swell-syntax` (decision 2).
- `swell-unicode` and the tool `swell-unicode-gen` (decision 8): data files vendored under
  `data/unicode/<version>/` with a `SOURCE` file of addresses and SHA-256 sums; tables written to
  `crates/unicode/src/tables/*.rs` and committed; the gate renders them in memory and fails
  on any difference with the committed files. Tables for `ID_Start`, `ID_Continue`, white space, `toUpperCase` and
  `toLowerCase`, case folding, normalization, property escapes (names and values) and properties of
  strings, each from the file decision 8 names. The version is 17.0.0 (open item C(a), standing per
  `AGENTS.md`); every section that depends on it carries the deviation as an omission.
- `RegExp`, `RegExpStringIteratorPrototype` and the six string methods.

### Settled since this epic was written

- The emoji sequence files of 17.0.0 and the text of the Unicode data licence are vendored
  (`data/unicode/17.0.0/SOURCE`, `LICENSE`).
- A pattern nested past the limit is a `SyntaxError` that names `parser_nesting`, in a literal
  and through the constructor (decision 3; `HOST-142`, `HOST-143`), not a `RangeError`.
- `toUpperCase`, `toLowerCase`, `localeCompare` and `normalize` refuse a code unit outside ASCII
  until their stories of this slice land (`HOST-165`; the design, section 12, item 5).
- The Annex B cases are divided at the conformance close: the 18 cases of the pattern grammar
  under `test/annexB/` come to slice 5 by rule lines of the block, and `RegExp.prototype.compile`
  stays in slice 18 (`story:slice-05-conformance-close`; the design, section 12, after "The
  order of delivery").
## Counts

- Slice 5 (`regexp`): 2,724 cases and 5,441 runs, 52 cases pass (`test262-slice 05 2724 5441 0 0 2724 52 2672 0`, 2026-11-23, counted by the order of delivery; 2,118 and 4,234 when this epic was written, before the closes of slices 1 to 3 and 6 moved cases here).
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
  well-known symbols were delivered with slice 6; a case that needs them and `RegExp` is counted here, by the order of delivery.
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

1. The conformance runner's line for slice 5 reads `test262-slice 05 <c> <r> 0 0 <c> <c> 0 0` with the cases and runs the design's table states for the slice after its close, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. `/(a+)+$/` matched against 10,000 `a` characters followed by `!` ends with `Terminated(StepBudget)`, never in a hang (decision 7).
6. A pattern nested deeper than the parser's nesting limit is reported as a `SyntaxError` at parse, in a literal and through the constructor, with no Rust stack overflow (decision 3; `HOST-142`, `HOST-143`).
7. The gate renders the `swell-unicode` tables from the vendored data in memory and fails on any difference from the committed tables; every vendored data file has its address and SHA-256 in `SOURCE` (decision 8).

## Stories

Decomposed on 2026-11-23 into fifteen stories, each read by the four plan critics
(`review-result:slice-05-*`: 9 findings on the first six, 16 on the other nine with 6 blockers,
all applied; the six stories with a blocker were read a second time). The first six make the
twelve regular expression literals of the first consumer's test harness evaluate and its calls
answer (`docs/plan/slice-05/consumer-regexps.tsv`). In the order of their dependencies:

1. `regexp-crate-and-pattern-parser`, `string-replace-replace-all-and-split`,
   `unicode-gen-table-list-and-source-digests`
2. `regexp-compiler-and-backtracking-matcher`, `regexp-literal-early-errors`,
   `case-mapping-tables-and-string-case-methods`
3. `regexp-objects-exec-and-literals`, `normalization-tables-normalize-and-locale-compare`
4. `regexp-symbol-methods-and-string-match-search`, `regexp-property-escapes-and-property-tables`
5. `identifiers-and-group-names-outside-ascii`, `regexp-match-all-and-escape`
6. `regexp-case-insensitive-matching-and-modifiers`
7. `regexp-v-flag-class-sets-and-properties-of-strings`
8. `slice-05-conformance-close`

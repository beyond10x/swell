---
format: aep.planning-md/3
id: story:normalization-tables-normalize-and-locale-compare
kind: story
status: draft
title: The normalization tables, String.prototype.normalize, and localeCompare outside ASCII
summary: '11 failing cases (22 runs); NFC, NFD, NFKC and NFKD of UAX #15; localeCompare by the NFC forms; the last refusal of a String method goes; HOST-165 in its delivered form.'
relations:
- decomposes: epic:slice-05-regexp
- depends_on: story:case-mapping-tables-and-string-case-methods
scope:
- confidence: cited
  path: crates/swell/src/builtins/string/transform.rs
- confidence: cited
  path: crates/swell/tests/builtins_string_transform.rs
- confidence: cited
  path: crates/unicode-gen/src/normalize.rs
- confidence: cited
  path: crates/unicode-gen/src/tables.rs
- confidence: cited
  path: crates/unicode/src/lib.rs
- confidence: cited
  path: crates/unicode/src/normalize.rs
- confidence: cited
  path: crates/unicode/src/tables/combining_class.rs
- confidence: cited
  path: crates/unicode/src/tables/compositions.rs
- confidence: cited
  path: crates/unicode/src/tables/decompositions.rs
- confidence: cited
  path: crates/unicode/src/tables/mod.rs
- confidence: cited
  path: crates/unicode/tests/main.rs
- confidence: cited
  path: crates/unicode/tests/normalize.rs
revision: 2
---
## Context

`String.prototype.normalize` (`#sec-string.prototype.normalize`) does not exist before slice 5 ("`String.prototype` has no property `normalize`", `docs/design/engine.md`, section 12, "A part before its slice"), and `localeCompare` refuses a String with a code unit of 0x80 or above with the counted refusal `comparison outside ASCII is not supported before slice 5` (HOST-165; `crates/swell/src/builtins/string/transform.rs`, `string_locale_compare`, line 457). This story brings the normalization tables of decision 8 (the decompositions of `UnicodeData.txt`, `CompositionExclusions.txt`, `DerivedNormalizationProps.txt`), the four forms of UAX #15, `normalize`, and `localeCompare` for every String, which removes the last refusal of a `String` method. It needs no part of `RegExp`. It lands after `story:case-mapping-tables-and-string-case-methods`, which writes the same lists of the generator and of `swell-unicode`, `transform.rs` and the test of HOST-165.

The list of this story is `docs/plan/slice-05/story-normalize-compare.cases.txt`: 11 failing cases (22 runs), 9 of `built-ins/String/prototype/normalize/` (the method is missing) and `localeCompare/15.5.4.9_3.js` and `15.5.4.9_CE.js` (the counted refusal), found by `docs/plan/slice-05/scan.awk` (its head gives the command). A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part.

Tables, from the vendored files (counted on `98df839` with `awk`, a measure of the data): 5,914 decompositions in `UnicodeData.txt` field 5, 2,081 of them canonical (1,046 of those to two code points), 968 code points whose canonical combining class is not 0, the 81 code points of `CompositionExclusions.txt`, and `Full_Composition_Exclusion` in 73 merged ranges of `DerivedNormalizationProps.txt`; the Hangul syllables are composed and decomposed by the arithmetic of chapter 3, not by a table.

Design it builds on: decision 8; section 12, "A part before its slice" (the two refusals go in slice 5) and its sentence on `localeCompare` for ASCII ("two Strings of such code units are canonically equivalent only when they are equal, so `localeCompare` orders them by their code units"); section 1 (ECMA-402 out: no host locale). Outside ASCII the design names no order: this story compares the NFC forms of both Strings by their code units, which keeps the ASCII answers and answers +0 for canonically equivalent Strings, as `#sec-string.prototype.localecompare` asks ("compares canonically equivalent strings as equal"); the report of the decomposition names the sentence for the design author.

## Domain relations

- None (methods of `%String.prototype%` and tables of `swell-unicode`).

## Change

Failing tests first: the tests below are written before the code; on the base `normalize` is not a property and `localeCompare` refuses.

- Generator: `crates/unicode-gen/src/normalize.rs` (new) and its entries in `crates/unicode-gen/src/tables.rs`: `crates/unicode/src/tables/{decompositions,combining_class,compositions}.rs` (new): the canonical and compatibility decompositions (one level, recursion done at run time by a loop), the combining classes as ranges with their value, and the primary composites (the canonical pairs minus `Full_Composition_Exclusion`) as pairs sorted by the pair; each file names its sources with their digests. `crates/unicode/src/tables/mod.rs`: the `mod` lines.
- `crates/unicode/src/normalize.rs` (new) and its exports in `crates/unicode/src/lib.rs`: NFD, NFC, NFKD and NFKC of a sequence of code points into a caller's buffer (UAX #15: full decomposition, the canonical ordering of each run of non-starters, canonical composition), with no recursion (the decomposition of a code point is expanded with an explicit stack), no panics and no unchecked indexing; the caller charges the work.
- `crates/swell/src/builtins/string/transform.rs`: `normalize ( [ form ] )` (`#sec-string.prototype.normalize`: RequireObjectCoercible, ToString of the this value, `form` undefined as `"NFC"`, ToString of it, a `RangeError` for any form but the four, the normalized String) in the list of the methods it defines (line 82), `length` 0; `localeCompare`: the refusal goes, the two Strings normalized to NFC and compared by code units, -1, +0 or 1. Steps and memory: a step per code unit read and written, the buffers grown by capacity and charged before they grow, the result through `StringBuilder` (bounded by `string_length`). The module comment of the file says no refusal remains. `COMPARE_REFUSAL` goes, and `refuse_before_slice_5` with it when nothing calls it.
- Tests: `crates/unicode/tests/normalize.rs` (new; `main.rs` generated): `the_four_forms_of_rows_written_for_the_test` (rows of the form of `NormalizationTest.txt` written for the test, that file not vendored: each of its five columns for a few dozen sources: Latin with combining marks, Hangul, a singleton, a composition exclusion, a compatibility ligature), `canonical_ordering_sorts_each_run_of_non_starters`, `a_deep_decomposition_uses_no_recursion`, `the_tables_are_sorted_and_disjoint`; `crates/swell/tests/builtins_string_transform.rs`: `host_165_case_methods_and_locale_compare_refuse_a_unit_outside_ascii` changed for `localeCompare` to the answer HOST-165 gives once slice 5 is delivered (`"a".localeCompare("é")` and `"é".localeCompare("a")` answer a Number and `refusals()` stays unchanged), `normalize_answers_the_four_forms_and_throws_a_range_error_for_another`, `locale_compare_answers_zero_for_canonically_equivalent_strings` (`"é"` and `"é"`), `normalize_past_string_length_throws_a_range_error`, `normalize_charges_each_code_unit`; the unit searches `crates/swell/tests` for a test that expects `normalize` to be absent (`builtins_symbol_values.rs` line 240 calls it with a Symbol and expects the `TypeError` of ToString, which it still answers) and lists it in its report. No new test file in `crates/swell/tests/`.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode of the gate, under 2 s with a collection at every safe point, and under 2.5 s at its full count (`SWELL_FULL_COUNTS=1`) in either mode, measured with its test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system); a language script stays under 0.5 CPU seconds per run. The unit times each test before it adds it and names in its report each one over 1 CPU second with its time; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the steps run a process per test); no test makes a network request; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`, a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m`, the most), and a script that measures or waits runs without the prefix. No Python in any tool, test or command. None of these tests is expected over 0.2 CPU seconds.
- Time of the generator and size of the tables: the unit times `target/debug/swell-unicode-gen --check` before and after its change, three runs each under `time` of the shell, and reports wall and CPU time and the number of tables; it reports the rows, pairs and ranges of each new table and the bytes of its constant; a rise to more than 0.1 s of CPU is fixed in the same change.
- Size: about 600 lines with the tests, besides the generated tables.

## Requirements

- HOST-165: its line does not change; its test changes as Change says, and after this story it holds the answer of HOST-165 once slice 5 is delivered for all five methods.

Writes: `crates/unicode-gen/src/normalize.rs` (new), `crates/unicode-gen/src/tables.rs` (the entries), `crates/unicode/src/tables/{decompositions,combining_class,compositions}.rs` (new), `crates/unicode/src/tables/mod.rs`, `crates/unicode/src/normalize.rs` (new), `crates/unicode/src/lib.rs`, `crates/unicode/tests/normalize.rs` (new), `crates/unicode/tests/main.rs` (generated), `crates/swell/src/builtins/string/transform.rs`, `crates/swell/tests/builtins_string_transform.rs`. Not written: `conformance/test262/sections.txt` (line 23 goes in `story:slice-05-conformance-close`), `conformance/test262/expected.txt` and `expected.digest` (the coordinator), the list files. Shared with the story that may run beside it, `story:regexp-objects-exec-and-literals`: none (it writes `access.rs` and `builtins/regexp`, not `transform.rs`, and no file of `crates/unicode`). It lands after `story:case-mapping-tables-and-string-case-methods`.

## Acceptance

`cargo nextest run --locked -p swell-unicode --test main`, `cargo nextest run --locked -p swell-unicode-gen --test main`, and `cargo nextest run --locked -p swell --test main -E 'test(/^builtins_string_transform::/) | test(/^builtins_symbol_values::/) | test(/^language::/)'` with and without `--features collect-every-safe-point` exit 0 with every test named in Change passing, and `cargo nextest run --locked -p swell-test262 --test main` exits 0; `slot task unicode-data-check` exits 0 and `slot task unicode-tables-check` prints `unicode-tables: 9 table(s) equal the generator output` (after `story:case-mapping-tables-and-string-case-methods` `6`); `grep -c 'not supported before slice 5' crates/swell/src/builtins/string/transform.rs` prints `0`; `slot task clippy` exits 0; `slot --timeout 30m task test262` is run once after the change (in the background) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes copied by `git show "$WAVE:docs/plan/slice-05/story-normalize-compare.cases.txt" > .out/story-normalize-compare.cases.txt` (`WAVE` the wave branch the unit merges into), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-normalize-compare.cases.txt .out/test262/runs.tsv` exits 0 (on `e444aaf` it exits 1 and prints 22 runs), and (b) with the run of the base commit of the unit made before the change (`slot --timeout 30m task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing (no run that passed on the base fails). The report gives the generator'\''s times and the sizes of the tables as Change says.

## Out of scope

A locale-sensitive collation (ECMA-402, slice 21); the quick-check properties of UAX #15 as an optimisation (a measured one later); the line of `sections.txt` (`story:slice-05-conformance-close`).

---
format: aep.planning-md/3
id: story:case-mapping-tables-and-string-case-methods
kind: story
status: implemented
title: The case tables of swell-unicode and the four case methods of String outside ASCII
summary: 12 failing cases (24 runs); toLowerCase, toUpperCase and the locale pair by UnicodeData and SpecialCasing with Final_Sigma; HOST-165's case part; the uppercase mapping the i story reads.
relations:
- decomposes: epic:slice-05-regexp
- depends_on: story:unicode-gen-table-list-and-source-digests
scope:
- confidence: cited
  path: crates/swell/src/builtins/string/transform.rs
- confidence: cited
  path: crates/swell/tests/builtins_string.rs
- confidence: cited
  path: crates/swell/tests/builtins_string_transform.rs
- confidence: cited
  path: crates/swell/tests/language/builtin-string-case-ascii.js
- confidence: cited
  path: crates/unicode-gen/src/case.rs
- confidence: cited
  path: crates/unicode-gen/src/render.rs
- confidence: cited
  path: crates/unicode-gen/src/tables.rs
- confidence: cited
  path: crates/unicode-gen/src/ucd.rs
- confidence: cited
  path: crates/unicode-gen/tables.txt
- confidence: cited
  path: crates/unicode/src/case.rs
- confidence: cited
  path: crates/unicode/src/lib.rs
- confidence: cited
  path: crates/unicode/src/tables/case_ignorable.rs
- confidence: cited
  path: crates/unicode/src/tables/cased.rs
- confidence: cited
  path: crates/unicode/src/tables/lowercase.rs
- confidence: cited
  path: crates/unicode/src/tables/mod.rs
- confidence: cited
  path: crates/unicode/src/tables/special_casing.rs
- confidence: cited
  path: crates/unicode/src/tables/uppercase.rs
- confidence: cited
  path: crates/unicode/tests/case.rs
- confidence: cited
  path: crates/unicode/tests/main.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T03:24:21Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T03:24:21Z", actor: "human:operator", revision: 6, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-23T08:12:43Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
## Context

Since slice 3 the four case methods of `String.prototype` (`toLowerCase`, `toUpperCase`, `toLocaleLowerCase`, `toLocaleUpperCase`) answer a String whose code units are all below 0x80 and refuse any other with the counted refusal `case mapping outside ASCII is not supported before slice 5` (`docs/design/engine.md`, section 12, "A part before its slice"; `docs/design/host-interface.md`, decision 13, "A refusal raised by a builtin method", HOST-165; `crates/swell/src/builtins/string/transform.rs` line 442). "Slice 5 brings the tables and removes both refusals" (section 12): this story brings the case tables of decision 8 (`UnicodeData.txt`, `SpecialCasing.txt`) and removes the refusal of the case methods; `story:normalization-tables-normalize-and-locale-compare` removes the one of `localeCompare`. It needs no part of `RegExp`, and `story:regexp-case-insensitive-matching-and-modifiers` reads its uppercase mapping for Canonicalize without `u` or `v` (decision 7: "`toUppercase` under neither").

The list of this story is `docs/plan/slice-05/story-case-mapping.cases.txt`: 12 failing cases (24 runs), `special_casing.js`, `special_casing_conditional.js`, `supplementary_plane.js` and `Final_Sigma_U180E.js` of the four directories (not every one in each), each ending on the counted refusal on `e444aaf`; found by `docs/plan/slice-05/scan.awk` (its head gives the command). The four `S15.5.4.1[6-9]_A1_T14.js` of these directories are ASCII and call a case method on a RegExp: they are on the list of `story:regexp-objects-exec-and-literals`. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part.

Tables, from the vendored files of `data/unicode/17.0.0/` (counted on `98df839` with `awk`, a measure of the data): 1,505 code points with a simple uppercase mapping and 1,488 with a simple lowercase one (`UnicodeData.txt` fields 12 and 13), the 119 lines of `SpecialCasing.txt` (102 unconditional, 1 `Final_Sigma`, 16 of the languages `lt`, `tr` and `az`), and the properties `Cased` (158 merged ranges) and `Case_Ignorable` (464) of `DerivedCoreProperties.txt` that the context `Final_Sigma` reads (Unicode chapter 3, table 3-17).

Design it builds on: decision 8 (the tables, generated and committed); section 12 (the case methods from slice 3, their refusal until slice 5) and its paragraph on the locale methods (no host locale, ECMA-402 out: the two locale methods answer as `toLowerCase` and `toUpperCase`); `story:unicode-gen-table-list-and-source-digests` (each table one entry of the generator's list, with the digests of its sources).

## Domain relations

- None (methods of `%String.prototype%` and tables of `swell-unicode`; no record of the engine gains a relation).

## Change

Failing tests first: the tests below are written before the code; on the base the methods refuse and the tables do not exist.

- Generator: `crates/unicode-gen/src/case.rs` (new), its renderers in `crates/unicode-gen/src/tables.rs` and its five lines in `crates/unicode-gen/tables.txt`; the reader of `UnicodeData.txt` (its fields by code point, a `<..., First>` and `<..., Last>` pair read as one range) in `crates/unicode-gen/src/ucd.rs` and the writer of a constant of pairs `&[(u32, u32)]` sorted by key in `crates/unicode-gen/src/render.rs`, which `story:unicode-gen-table-list-and-source-digests` leaves to this story, the first that uses them (a test of each through the tables: `the_tables_are_sorted_and_disjoint` and the round trip below; `crates/unicode-gen/tests/check.rs` takes the five new tables from the list and is not written here): `crates/unicode/src/tables/{lowercase,uppercase,special_casing,cased,case_ignorable}.rs` (new), the simple mappings as pairs sorted by code point, the unconditional full mappings of `SpecialCasing.txt` and its one context without a language (`Final_Sigma`) as a table of a code point and up to three code points, the two properties as ranges; the 16 lines of a language are not written (no host locale). Each file names its sources with their digests (`// Source:` lines). `crates/unicode/src/tables/mod.rs`: the `mod` lines.
- `crates/unicode/src/case.rs` (new) and its exports in `crates/unicode/src/lib.rs`: the full lowercase and uppercase mapping of one code point into at most three, by `SpecialCasing.txt` first and the simple mapping else; `is_cased` and `is_case_ignorable`; the code point and its mapping are `u32`. No panics and no unchecked indexing (`AGENTS.md`, "Rules"; a binary search over the sorted tables).
- `crates/swell/src/builtins/string/transform.rs`, `string_case_method` (line 436): the refusal goes; the String is read as code points (StringToCodePoints, `#sec-stringtocodepoints`, a lone surrogate itself), each mapped by toLowercase or toUppercase of the Unicode Default Case Conversion (`#sec-string.prototype.tolowercase`), `Final_Sigma` applied to U+03A3 by its context (before it a cased letter after any case-ignorable ones, after it no cased letter after any case-ignorable ones), the result written by CodePointsToString through `StringBuilder` (bounded by `string_length`, charged before it grows); a step per code unit read and per code unit written, as now for ASCII. The two locale methods answer as the two others. The module comment of the file and of `refuse_before_slice_5` says which refusal remains (`localeCompare`, until its story). `CASE_REFUSAL` goes when nothing uses it.
- What expects the refusal (the rule of the slice: a story that removes a refusal changes every test, script and recorded total that expects it, and runs each), found by `grep -rn 'case mapping outside ASCII\|CASE_REFUSAL\|refused before slice 5\|"throw SyntaxError"' crates/*/tests crates/*/src`. On `8073b8e` it prints, besides `transform.rs`: (1) `crates/swell/tests/builtins_string_transform.rs`, `host_165_case_methods_and_locale_compare_refuse_a_unit_outside_ascii` (lines 426 to 545), changed as the Tests item says; (2) `crates/swell/tests/language/builtin-string-case-ascii.js`, whose `"é".toUpperCase()` (line 7) expects the `SyntaxError`: the script keeps its ASCII part and now ends with `"é".toUpperCase()`, its header becomes `normal "hello, world 42!|HELLO, WORLD 42!|abc|ABC|É"`, and the sentence on the refusal (lines 3 and 4) goes; (3) `crates/swell/tests/builtins_string.rs`, `the_step_totals_are_unchanged`: the four rows `toLowerCase u1000`, `toUpperCase u1000`, `toLocaleLowerCase u1000`, `toLocaleUpperCase u1000` (lines 905 to 908, `"throw SyntaxError"`, 1001 steps) take the answer of the mapping (`1000:<digest>` of the mapped String) and its total (by the cost above a step per unit read and per unit written: 2,001 expected, as `toLowerCase s1000`), and the comment of `other_units` (lines 617 to 620) says only `localeCompare` refuses it; a recorded total changes only with both numbers in the report, so the report gives for each of the four rows the answer and total before and after; the other rows of the four methods (Strings below 0x80) do not change. The lines of `localeCompare` the search prints (`builtin-string-locale-compare-ascii.js`, rows 909 and 910) are left to `story:normalization-tables-normalize-and-locale-compare`. Before the change the unit runs the search on its base and names in its report any hit not in this list, changed in the same way.
- Tests: `crates/unicode/tests/case.rs` (new; `crates/unicode/tests/main.rs` generated): `simple_mappings_of_every_table_row_round_trip_where_the_data_says_so`, `special_casing_maps_sharp_s_and_the_ligatures_to_several_code_points`, `final_sigma_reads_its_context`, `a_code_point_with_no_mapping_maps_to_itself`, `the_tables_are_sorted_and_disjoint`; `crates/swell/tests/builtins_string_transform.rs`: `host_165_case_methods_and_locale_compare_refuse_a_unit_outside_ascii` changed for the case methods to the answer HOST-165 gives once slice 5 is delivered (`"é".toLowerCase()` and the three others answer the standard's String and `refusals()` stays unchanged), its `localeCompare` part unchanged; `case_methods_map_outside_ascii` (`"İ".toLowerCase()`, `"ß".toUpperCase()`, `"ΑΣ".toLowerCase()`, a supplementary-plane letter, a lone surrogate), `a_case_method_past_string_length_throws_a_range_error` (`"ß".repeat(n).toUpperCase()` past `string_length`), `a_case_method_charges_each_code_unit`. No new test file in `crates/swell/tests/`.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode of the gate, under 2 s with a collection at every safe point, and under 2.5 s at its full count (`SWELL_FULL_COUNTS=1`) in either mode, measured with its test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system); a language script stays under 0.5 CPU seconds per run. The unit times each test before it adds it and names in its report each one over 1 CPU second with its time; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the steps run a process per test); no test makes a network request; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`, a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m`, the most), and a script that measures or waits runs without the prefix. No Python in any tool, test or command. Each table is timed with its test; none is expected over 0.1 CPU seconds.
- Time of the generator and size of the tables: the unit times `target/debug/swell-unicode-gen --check` before and after its change, three runs each under `time` of the shell, and reports wall and CPU time and the number of tables; it reports the pairs, rows and ranges of each new table and the bytes of its constant; a rise to more than 0.1 s of CPU is fixed in the same change.
- Size: about 500 lines with the tests, besides the generated tables.

## Requirements

- HOST-165 (`docs/design/host-interface.md`, decision 13): its line `HOST-165 test swell::builtins_string_transform::host_165_case_methods_and_locale_compare_refuse_a_unit_outside_ascii` does not change; its test changes as Change says.

Writes: `crates/unicode-gen/src/case.rs` (new), `crates/unicode-gen/src/tables.rs` (the renderers), `crates/unicode-gen/tables.txt` (five lines), `crates/unicode-gen/src/ucd.rs` (the reader of `UnicodeData.txt`), `crates/unicode-gen/src/render.rs` (the writer of pairs), `crates/unicode/src/tables/{lowercase,uppercase,special_casing,cased,case_ignorable}.rs` (new), `crates/unicode/src/tables/mod.rs`, `crates/unicode/src/case.rs` (new), `crates/unicode/src/lib.rs`, `crates/unicode/tests/case.rs` (new), `crates/unicode/tests/main.rs` (generated), `crates/swell/src/builtins/string/transform.rs`, `crates/swell/tests/builtins_string_transform.rs`, `crates/swell/tests/builtins_string.rs` (the four rows of `u1000` and the comment of `other_units`), `crates/swell/tests/language/builtin-string-case-ascii.js`. Not written: `crates/unicode-gen/tests/check.rs` (it takes the tables from the list), `conformance/test262/sections.txt` (its line 23, `omit sec-string-objects case-mapping-and-comparison-outside-ascii section-12-slice-5`, names `localeCompare` too, and `story:slice-05-conformance-close` removes it once both refusals are gone), `conformance/test262/expected.txt` and `expected.digest` (the coordinator), the list files. Shared with the stories that may run beside it, `story:regexp-compiler-and-backtracking-matcher` and `story:regexp-literal-early-errors`: none. Each later table story (`story:normalization-tables-normalize-and-locale-compare`, `story:regexp-property-escapes-and-property-tables`, `story:regexp-case-insensitive-matching-and-modifiers`, `story:regexp-v-flag-class-sets-and-properties-of-strings`) depends on this one and writes after it `crates/unicode/src/lib.rs`, `crates/unicode/src/tables/mod.rs`, `crates/unicode-gen/src/tables.rs`, `crates/unicode-gen/tables.txt` and the generated `crates/unicode/tests/main.rs`; the normalization story also writes `transform.rs`, `builtins_string_transform.rs` and `builtins_string.rs` after it.

## Acceptance

`cargo nextest run --locked -p swell-unicode --test main`, `cargo nextest run --locked -p swell-unicode-gen --test main`, and `cargo nextest run --locked -p swell --test main -E 'test(/^builtins_string_transform::/) | test(/^builtins_string::/) | test(/^language::/)'` with and without `--features collect-every-safe-point` exit 0 with every test named in Change passing (`builtins_string::the_step_totals_are_unchanged` with its four new rows and `language::` with `builtin-string-case-ascii.js` among them), `cargo nextest run --locked -p swell --lib -E 'test(/string::transform::/)'` exits 0 (the unit tests of the file this story changes), and `cargo nextest run --locked -p swell-test262 --test main` exits 0; `slot task unicode-data-check` exits 0 and `slot task unicode-tables-check` prints `unicode-tables: 6 table(s) equal the generator output` (on the base `1`); `grep -c 'case mapping outside ASCII' crates/swell/src/builtins/string/transform.rs` prints `0`, and the search of Change, `grep -rn 'case mapping outside ASCII\|CASE_REFUSAL' crates/*/tests crates/*/src`, prints nothing; `slot task clippy` exits 0; `slot --timeout 30m task test262` is run once after the change (in the background) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes copied by `git show "$WAVE:docs/plan/slice-05/story-case-mapping.cases.txt" > .out/story-case-mapping.cases.txt` (`WAVE` the wave branch the unit merges into), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-case-mapping.cases.txt .out/test262/runs.tsv` exits 0 (on `e444aaf` it exits 1 and prints 24 runs), and (b) with the run of the base commit of the unit made before the change (`slot --timeout 30m task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing (no run that passed on the base fails). The report gives the times of the generator and the sizes of the tables as Change says, and the four rows of `the_step_totals_are_unchanged` with their answers and totals before and after.

## Out of scope

The language-sensitive mappings of `SpecialCasing.txt` (`lt`, `tr`, `az`: no host locale, ECMA-402 is slice 21); `localeCompare` and `normalize` (`story:normalization-tables-normalize-and-locale-compare`); case folding and Canonicalize (`story:regexp-case-insensitive-matching-and-modifiers`); the line of `sections.txt` (`story:slice-05-conformance-close`).

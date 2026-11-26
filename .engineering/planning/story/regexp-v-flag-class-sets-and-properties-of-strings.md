---
format: aep.planning-md/3
id: story:regexp-v-flag-class-sets-and-properties-of-strings
kind: story
status: implemented
title: 'The flag v: class set expressions, strings in classes and the properties of strings'
summary: 185 failing cases (370 runs); the emoji sequence tables; the last refusal of slice 5 goes and HOST-191's test holds its delivered form.
relations:
- decomposes: epic:slice-05-regexp
- depends_on: story:regexp-case-insensitive-matching-and-modifiers
- depends_on: story:regexp-property-escapes-and-property-tables
- depends_on: story:identifiers-and-group-names-outside-ascii
scope:
- confidence: cited
  path: crates/regexp/src/ast.rs
- confidence: cited
  path: crates/regexp/src/class_set.rs
- confidence: cited
  path: crates/regexp/src/compile.rs
- confidence: cited
  path: crates/regexp/src/early.rs
- confidence: cited
  path: crates/regexp/src/lib.rs
- confidence: cited
  path: crates/regexp/src/matcher.rs
- confidence: cited
  path: crates/regexp/src/parser.rs
- confidence: cited
  path: crates/regexp/tests/early_errors.rs
- confidence: cited
  path: crates/regexp/tests/main.rs
- confidence: cited
  path: crates/regexp/tests/unicode_sets.rs
- confidence: cited
  path: crates/swell/tests/regexp_literal_limits.rs
- confidence: cited
  path: crates/swell/tests/regexp_refusals.rs
- confidence: cited
  path: crates/syntax/tests/sources/early-errors/regexp-v-reserved-punctuator.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/regexp-v-flag.js
- confidence: cited
  path: crates/syntax/tests/sources/valid/regexp-literals.js
- confidence: cited
  path: crates/unicode-gen/src/emoji.rs
- confidence: cited
  path: crates/unicode-gen/src/render.rs
- confidence: cited
  path: crates/unicode-gen/src/tables.rs
- confidence: cited
  path: crates/unicode-gen/src/ucd.rs
- confidence: cited
  path: crates/unicode-gen/tables.txt
- confidence: cited
  path: crates/unicode/src/lib.rs
- confidence: cited
  path: crates/unicode/src/property.rs
- confidence: cited
  path: crates/unicode/src/tables/mod.rs
- confidence: cited
  path: crates/unicode/src/tables/properties_of_strings.rs
- confidence: cited
  path: crates/unicode/tests/property.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T03:24:25Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T03:24:26Z", actor: "human:operator", revision: 6, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-26T05:55:48Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
## Context

Decision 7 of `docs/design/engine.md` puts the flag `v` in this slice with its set operations and the properties of strings ("properties of strings under `v`, set operations under `v`"), and decision 8 their table ("Properties of strings under `v` (`RGI_Emoji` and the others): the emoji sequence files of the same version", vendored as `data/unicode/17.0.0/emoji/emoji-sequences.txt` and `emoji-zwj-sequences.txt` with their `SOURCE` lines). Until this story the parser of `swell-regexp` refuses the flag `v` with `class set expressions of the v flag is not supported before slice 5` (decision 7, "Refusals until the later stories of slice 5"), a refusal of the parse in a literal (HOST-164) and of RegExpInitialize (HOST-191); it is the last refusal of slice 5, so after this story `swell-regexp` refuses nothing. It lands after `story:regexp-property-escapes-and-property-tables` (property escapes under `v`), `story:regexp-case-insensitive-matching-and-modifiers` (case folding under `v`) and `story:identifiers-and-group-names-outside-ascii` (the refusal of a group name, the last before this one).

The list of this story is `docs/plan/slice-05/story-unicode-sets.cases.txt`: 185 failing cases (370 runs): 114 of `built-ins/RegExp/unicodeSets/generated/` (the set operations over characters, classes, property escapes and strings), 36 of `built-ins/RegExp/prototype/unicodeSets/`, 28 of `built-ins/RegExp/property-escapes/generated/strings/` (the seven properties of strings), and 7 in `String/prototype/{search,replace,matchAll,match}/` and `RegExp/prototype/{exec,flags}/`; found by `docs/plan/slice-05/scan.awk` (its head gives the command). A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part.

Table, from the vendored files (counted on `98df839` with `awk`, a measure of the data): `emoji-sequences.txt` 1,441 data lines (`Basic_Emoji` 502, 154 of the lines a range of single code points; `Emoji_Keycap_Sequence` 12; `RGI_Emoji_Flag_Sequence` 259; `RGI_Emoji_Modifier_Sequence` 665; `RGI_Emoji_Tag_Sequence` 3) and `emoji-zwj-sequences.txt` 1,614 (`RGI_Emoji_ZWJ_Sequence`); `RGI_Emoji` is their union.

Design it builds on: decision 7 (in scope; "No Rust recursion in the parser and the compiler of a pattern": under `v` one entry of the parser's stack per nested class, counted against `parser_nesting`; "Refusals until the later stories of slice 5"); decision 8; `story:unicode-gen-table-list-and-source-digests`.

## Domain relations

- None (the pattern grammar and matcher of `swell-regexp` and a table of `swell-unicode`).

## Change

Failing tests first: the tests below are written before the code; on the base the flag `v` is refused.

- Generator: `crates/unicode-gen/src/emoji.rs` (new), its renderer in `crates/unicode-gen/src/tables.rs` and its line in `crates/unicode-gen/tables.txt`; the reader of a file of sequences (`emoji/*.txt`: a code point, a range of single code points or a sequence, `;` its property) in `crates/unicode-gen/src/ucd.rs` and the writer of a constant of sequences in `crates/unicode-gen/src/render.rs`, which `story:unicode-gen-table-list-and-source-digests` leaves to this story, the first that uses them (a test of each through the table: `the_properties_of_strings_resolve` below; `crates/unicode-gen/tests/check.rs` takes the table from the list and is not written here): `crates/unicode/src/tables/properties_of_strings.rs` (new), each property's single code points as ranges and its sequences sorted, with the sources and their digests; `crates/unicode/src/tables/mod.rs`: its `mod` line; `crates/unicode/src/property.rs` (of the property story): the lookup of the seven names, exported from `crates/unicode/src/lib.rs`.
- `crates/regexp/src/{parser,ast,early}.rs`: under `v` (`#sec-patterns` with [+UnicodeSetsMode]) ClassSetExpression: ClassUnion, ClassIntersection (`&&`), ClassSubtraction (`--`), nested classes, `\q{...}` with its alternatives, the ClassSetCharacter rules (the syntax characters and the reserved double punctuators escaped), the early errors of `#sec-patterns-static-semantics-early-errors` (a reserved double punctuator, an operand of a mixed operation, `\P` or a negated class of a set that may contain strings, MayContainStrings), `\p` of a property of strings; each nested class one entry of the parser's explicit stack, counted against the nesting limit, charged before it grows; the refusal of `v` goes.
- `crates/regexp/src/{class_set,compile,matcher}.rs` (`class_set.rs` new): the set of a class under `v` as ranges of code points and a list of strings, the operations on both (CompileToCharSet, `#sec-compiletocharset`), under `i` closed by simple case folding as the case story's closure does for characters; the match of a class with strings tries its strings longest first and then its code points (`#sec-compileatom`, the ClassContents of a CharSet with strings), each try a step; no Rust recursion. The set of a class under `v` (its ranges and its strings, and each set an intersection or a subtraction makes on the way) is charged through the crate's trait before it is built, as the other stories charge their sets (`story:regexp-property-escapes-and-property-tables`, `story:regexp-case-insensitive-matching-and-modifiers`), and credited when it is dropped; past the memory budget the compile stops with the stop of the meter and holds nothing.
- What expects the refusal (the rule of the slice: a story that removes a refusal changes every test, script and fixture that expects it, and runs each), found by `grep -rn 'class set expressions of the v flag\|regexp-v-flag\|refusals_of_slice_5\|a_refused_pattern_of_a_literal' crates/*/tests crates/*/src` on the base of the unit, which holds `story:regexp-property-escapes-and-property-tables` and `story:identifiers-and-group-names-outside-ascii` (its edges), so `v` is the last refusal each of these still holds. From the stories of part a it prints at least these, each changed here: (1) `crates/regexp/src/parser.rs`, the refusal itself; (2) `crates/regexp/tests/early_errors.rs`, `the_v_flag_a_property_escape_and_a_group_name_outside_ascii_are_refusals_of_slice_5` (`story:regexp-crate-and-pattern-parser`, holding the flag `v` alone after those two stories): removed, no refusal remaining; (3) `crates/syntax/tests/sources/refused/regexp-v-flag.js` (`story:regexp-literal-early-errors`): removed, its form added to `crates/syntax/tests/sources/valid/regexp-literals.js`, and an early error of `v` added as `crates/syntax/tests/sources/early-errors/regexp-v-reserved-punctuator.js` (new, with the header line of the directory); (4) `crates/swell/tests/regexp_literal_limits.rs`, `a_refused_pattern_of_a_literal_is_counted_as_a_refused_parse` (`story:regexp-literal-early-errors`, holding the flag `v` alone after those two stories): removed, as the last of the three stories that take a part out of it, and in its place `the_patterns_of_slice_5_in_a_literal_are_not_refused` (a literal with the flag `v`, one with `\p{L}` under `u` and one with a group name outside ASCII each parse and leave `refusals()` unchanged); (5) `crates/swell/tests/regexp_refusals.rs`, `host_191_regexp_counts_its_refusals_of_slice_5`: `new RegExp("a", "v").test("a")` answers `Normal(true)` with the count unchanged, so the test holds HOST-191 as it reads once slice 5 is delivered. Before the change the unit runs the search and names in its report any other hit (a test of `crates/swell/tests/regexp_objects.rs` or a language script of part a that expects the refusal), changed in the same way.
- Tests: `crates/regexp/tests/unicode_sets.rs` (new; `main.rs` generated): `union_intersection_and_subtraction_of_characters_and_classes`, `strings_of_q_match_longest_first`, `a_property_of_strings_matches_its_sequences`, `the_early_errors_of_v`, `case_folding_under_v`, `nested_classes_past_the_nesting_limit_are_the_nesting_error` (at a limit of 1,000: 1,001 nested classes answer the error, 1,000 parse), `nested_classes_use_no_rust_recursion` (10,000 nested classes at a limit of 10,000 on a thread of 64 KiB), `a_class_set_past_the_memory_budget_stops_the_compile` (with a recording meter whose memory budget is smaller than the set of `[\p{L}--[a-z]]` under `v`: the compile ends with the stop over the memory budget, the meter holds no byte after it, and the same pattern compiles with the budget raised); `crates/unicode/tests/property.rs`: `the_properties_of_strings_resolve`. No new test file in `crates/swell/tests/`.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode of the gate, under 2 s with a collection at every safe point, and under 2.5 s at its full count (`SWELL_FULL_COUNTS=1`) in either mode, measured with its test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system); a language script stays under 0.5 CPU seconds per run. The unit times each test before it adds it and names in its report each one over 1 CPU second with its time; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the steps run a process per test); no test makes a network request; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`, a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m`, the most), and a script that measures or waits runs without the prefix. No Python in any tool, test or command. None of these tests is expected over 0.5 CPU seconds.
- Time of the generator and size of the table: the unit times `target/debug/swell-unicode-gen --check` before and after its change, three runs each under `time` of the shell, and reports wall and CPU time and the number of tables; it reports the ranges and sequences of the new table and the bytes of its constants; a rise to more than 0.5 s of CPU is fixed in the same change.
- Size: about 900 lines with the tests, besides the generated table.

## Requirements

- HOST-191: its line does not change; after this story its test holds the answer HOST-191 gives once slice 5 is delivered for every part.

Writes: `crates/unicode-gen/src/emoji.rs` (new), `crates/unicode-gen/src/tables.rs` (the renderer), `crates/unicode-gen/tables.txt` (one line), `crates/unicode-gen/src/ucd.rs` (the reader of sequences), `crates/unicode-gen/src/render.rs` (the writer of sequences), `crates/unicode/src/tables/properties_of_strings.rs` (new), `crates/unicode/src/tables/mod.rs`, `crates/unicode/src/{property,lib}.rs`, `crates/unicode/tests/property.rs`, `crates/regexp/src/{parser,ast,early,class_set,compile,matcher,lib}.rs` (`class_set.rs` new), `crates/regexp/tests/{unicode_sets,main}.rs` (new and generated), `crates/regexp/tests/early_errors.rs`, `crates/syntax/tests/sources/refused/regexp-v-flag.js` (removed), `crates/syntax/tests/sources/valid/regexp-literals.js`, `crates/syntax/tests/sources/early-errors/regexp-v-reserved-punctuator.js` (new), `crates/swell/tests/regexp_refusals.rs`, `crates/swell/tests/regexp_literal_limits.rs` (`a_refused_pattern_of_a_literal_is_counted_as_a_refused_parse` removed, `the_patterns_of_slice_5_in_a_literal_are_not_refused` added). Not written: `crates/unicode-gen/tests/check.rs` (it takes the table from the list), `conformance/test262/expected.txt` and `expected.digest` (the coordinator), the list files. It depends on `story:regexp-property-escapes-and-property-tables`, `story:regexp-case-insensitive-matching-and-modifiers` and `story:identifiers-and-group-names-outside-ascii` (edges): it reads what each removes (the acceptance's search for `not supported before slice 5` printing `0` and HOST-191 "for every part" need the refusals of property escapes, of `i` and of group names gone), and it writes after the identifiers story the five files they share, `crates/regexp/src/parser.rs`, `crates/regexp/tests/early_errors.rs`, the generated `crates/regexp/tests/main.rs`, `crates/swell/tests/regexp_refusals.rs` and `crates/syntax/tests/sources/valid/regexp-literals.js`, and `regexp_literal_limits.rs`. Shared with `story:normalization-tables-normalize-and-locale-compare`, which no edge orders against this one: `crates/unicode/src/lib.rs`, `crates/unicode/src/tables/mod.rs`, `crates/unicode-gen/src/tables.rs` and `crates/unicode-gen/tables.txt` (this story adds no test module to `crates/unicode/tests/`); each adds its own lines (an export, a `mod` line, a renderer, its lines of the list) and reads nothing the other makes; whichever lands second keeps the lines of the first beside its own and runs the check of the generator again.

## Acceptance

`cargo nextest run --locked -p swell-unicode --test main`, `cargo nextest run --locked -p swell-unicode-gen --test main`, `cargo nextest run --locked -p swell-regexp --test main`, `cargo nextest run --locked -p swell-syntax --test main`, `cargo nextest run --locked -p swell --test main -E 'test(/^regexp_refusals::/) | test(/^regexp_objects::/) | test(/^regexp_literal_limits::/) | test(/^eval::/) | test(/^language::/)'` with and without `--features collect-every-safe-point`, and `cargo nextest run --locked -p swell-test262 --test main` exit 0 with every test named in Change passing; `slot task unicode-tables-check` prints `unicode-tables: <n> table(s) equal the generator output` with `<n>` one more than on the base of the unit; `grep -rc 'not supported before slice 5' crates/regexp/src | awk -F: '{ s += $2 } END { print s }'` prints `0` (no refusal of slice 5 remains in the crate), and the search of Change for what expected the refusal, `grep -rn 'class set expressions of the v flag\|a_refused_pattern_of_a_literal\|are_refusals_of_slice_5' crates/*/tests crates/*/src`, prints nothing; `slot task clippy` exits 0; `slot --timeout 30m task test262` is run once after the change (in the background) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes copied by `git show "$WAVE:docs/plan/slice-05/story-unicode-sets.cases.txt" > .out/story-unicode-sets.cases.txt` (`WAVE` the wave branch the unit merges into), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-unicode-sets.cases.txt .out/test262/runs.tsv` exits 0 (on `e444aaf` it exits 1 and prints 370 runs), and (b) with the run of the base commit of the unit made before the change (`slot --timeout 30m task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing (no run that passed on the base fails). The report gives the times of the generator and the size of the table as Change says.

## Out of scope

The moves of cases and the close of the slice (`story:slice-05-conformance-close`); properties of strings outside the seven of ECMA-262's table (`#table-binary-unicode-properties-of-strings`).

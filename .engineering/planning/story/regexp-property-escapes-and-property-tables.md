---
format: aep.planning-md/3
id: story:regexp-property-escapes-and-property-tables
kind: story
status: draft
title: Unicode property escapes under u, with the tables of General_Category, Script, Script_Extensions and the binary properties
summary: 588 failing cases (1,176 runs), the largest list after the first wave; about 17,000 ranges of tables; the refusal of property escapes goes; ID_Start and ID_Continue for the identifier story; the cost of the generated cases measured.
relations:
- decomposes: epic:slice-05-regexp
- depends_on: story:regexp-objects-exec-and-literals
- depends_on: story:case-mapping-tables-and-string-case-methods
scope:
- confidence: cited
  path: crates/regexp/src/compile.rs
- confidence: cited
  path: crates/regexp/src/early.rs
- confidence: cited
  path: crates/regexp/src/lib.rs
- confidence: cited
  path: crates/regexp/src/parser.rs
- confidence: cited
  path: crates/regexp/tests/early_errors.rs
- confidence: cited
  path: crates/regexp/tests/main.rs
- confidence: cited
  path: crates/regexp/tests/property_escapes.rs
- confidence: cited
  path: crates/swell/tests/regexp_refusals.rs
- confidence: cited
  path: crates/syntax/tests/sources/refused/regexp-property-escape.js
- confidence: cited
  path: crates/syntax/tests/sources/valid/regexp-literals.js
- confidence: cited
  path: crates/unicode-gen/src/property.rs
- confidence: cited
  path: crates/unicode-gen/src/tables.rs
- confidence: cited
  path: crates/unicode/src/lib.rs
- confidence: cited
  path: crates/unicode/src/property.rs
- confidence: cited
  path: crates/unicode/src/tables/binary_properties.rs
- confidence: cited
  path: crates/unicode/src/tables/general_category.rs
- confidence: cited
  path: crates/unicode/src/tables/mod.rs
- confidence: cited
  path: crates/unicode/src/tables/property_names.rs
- confidence: cited
  path: crates/unicode/src/tables/script.rs
- confidence: cited
  path: crates/unicode/src/tables/script_extensions.rs
- confidence: cited
  path: crates/unicode/tests/main.rs
- confidence: cited
  path: crates/unicode/tests/property.rs
revision: 2
---
## Context

Decision 7 of `docs/design/engine.md` puts Unicode property escapes in this slice ("Unicode property escapes and properties of strings under `v`"), with every table from `swell-unicode` (decision 8: the names from `PropertyAliases.txt` and `PropertyValueAliases.txt`, the values from `DerivedGeneralCategory.txt`, `Scripts.txt`, `ScriptExtensions.txt`, `PropList.txt`, `DerivedBinaryProperties.txt`, `DerivedCoreProperties.txt` and `emoji/emoji-data.txt`). Until this story the parser of `swell-regexp` refuses `\p{...}` and `\P{...}` with `property escape is not supported before slice 5` (decision 7, "Refusals until the later stories of slice 5"; `story:regexp-crate-and-pattern-parser`), a refusal of the parse in a literal (HOST-164) and of RegExpInitialize at run time (HOST-191). This story brings the tables and the escapes under `u` and removes that refusal. It is the largest list of the slice after the first wave; it runs beside `story:regexp-symbol-methods-and-string-match-search`, which it does not need. Its tables include `ID_Start` and `ID_Continue`, which `story:identifiers-and-group-names-outside-ascii` reads after it, and `story:regexp-case-insensitive-matching-and-modifiers` comes after it for the four cases that use a property escape under `i`.

The list of this story is `docs/plan/slice-05/story-property-escapes.cases.txt`: 588 failing cases (1,176 runs): 441 of `built-ins/RegExp/property-escapes/generated/` (one per property value, each matching every code point of the value and of its complement), 144 of `built-ins/RegExp/property-escapes/` (the grammar: unknown names and values, loose matching, binary properties with a value, the escapes without `u`, 142 of them negative parse cases), 2 of `built-ins/Function/prototype/toString/` and 1 of `harness/`, which read `nativeFunctionMatcher.js` (its patterns use `\p{ID_Start}` and `\p{ID_Continue}` under `u`). On the base 445 end on the counted refusal, 142 on `the case parsed; a SyntaxError was expected at parse`. Found by `docs/plan/slice-05/scan.awk` (its head gives the command). A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part. The 28 cases of `property-escapes/generated/strings/` use properties of strings and are on the list of `story:regexp-v-flag-class-sets-and-properties-of-strings`.

Tables, from the vendored files (counted on `98df839` with `awk`; merged ranges: adjacent ranges of one value joined; a measure of the data): General_Category 30 values in 4,144 ranges (the groups `L`, `LC`, `M`, `N`, `P`, `S`, `Z` and `C` are unions made at run time); Script 174 values in 984 ranges; Script_Extensions 118 sets in 176 ranges; the 50 binary properties of ECMA-262's table (`#table-binary-unicode-properties`) that the data files hold, in 11,869 ranges, `ID_Start` 684 and `ID_Continue` 799 among them, `Cased` and `Case_Ignorable` already written by `story:case-mapping-tables-and-string-case-methods` and read from there; `ASCII`, `Any` and `Assigned` are made at run time; the names and aliases of the properties and values of the three tables of ECMA-262 (`#table-nonbinary-unicode-properties`, `#table-binary-unicode-properties`, and the values of `PropertyValueAliases.txt` for General_Category and Script).

Design it builds on: decision 7 (in scope; "Refusals until the later stories of slice 5"); decision 8 (the files of each table, generated and committed, the check in memory); `story:unicode-gen-table-list-and-source-digests` (one entry per table with the digests of its sources).

## Domain relations

- None (tables of `swell-unicode` and the pattern grammar of `swell-regexp`).

## Change

Failing tests first: the tests below are written before the code; on the base a property escape is refused.

- Generator: `crates/unicode-gen/src/property.rs` (new) and its entries in `crates/unicode-gen/src/tables.rs`: `crates/unicode/src/tables/{general_category,script,script_extensions,binary_properties,property_names}.rs` (new), the values as sorted, disjoint ranges per value, Script_Extensions as ranges with the list of their scripts, and the names as sorted pairs of a name or alias and its property or value; each file names its sources with their digests. `crates/unicode/src/tables/mod.rs`: the `mod` lines.
- `crates/unicode/src/property.rs` (new) and its exports in `crates/unicode/src/lib.rs`: the lookup of a property name and value by the exact names and aliases ECMA-262 admits (no loose matching: `#sec-runtime-semantics-unicodematchproperty-p` and `unicodematchpropertyvalue`), the ranges of a value (a union for a General_Category group, Script_Extensions by its rule: the code points whose extensions name the script, and those of the script with no extensions entry), and `is_id_start` and `is_id_continue` for `story:identifiers-and-group-names-outside-ascii`; no panics and no unchecked indexing.
- `crates/regexp`: `src/parser.rs` and `src/early.rs`, the escapes `\p{Name=Value}`, `\p{Value}` and `\P{...}` under `u` (`CharacterClassEscape` with [+UnicodeMode], `#sec-patterns`) with the early errors of `#sec-patterns-static-semantics-early-errors` (an unknown name or value, a binary property with a value, a property of strings under `u`), the refusal of `property escape` gone; without `u` or `v` `\p` stays the identity escape of B.1.2; `src/compile.rs`, the class of a property escape from the ranges of `swell-unicode`, and its complement for `\P`. The parse charges the set it builds through the trait; the sort of a class's ranges stays the one part above linear (decision 7).
- Fixtures and the refusal test: `crates/syntax/tests/sources/refused/regexp-property-escape.js` removed and its form added to `crates/syntax/tests/sources/valid/regexp-literals.js`; in `crates/regexp/tests/early_errors.rs` the refusal test keeps the flag `v` and a group name outside ASCII and drops the property escape; `crates/swell/tests/regexp_refusals.rs`, `host_191_regexp_counts_its_refusals_of_slice_5`: `new RegExp("\\p{L}", "u").test("a")` answers `Normal(true)` with the count unchanged.
- Tests: `crates/unicode/tests/property.rs` (new; `main.rs` generated): `every_name_and_alias_of_the_three_tables_resolves`, `a_loose_or_unknown_name_resolves_to_nothing`, `general_category_groups_are_unions`, `script_extensions_follow_their_rule`, `the_tables_are_sorted_and_disjoint`; `crates/regexp/tests/property_escapes.rs` (new; `main.rs` generated): `property_escapes_match_their_ranges_with_u`, `a_complement_escape_matches_the_rest`, `property_escapes_are_early_errors_where_ecma_262_says`, `without_u_p_is_an_identity_escape`, `a_long_subject_of_one_property_matches_within_the_budget` (a String of every code point of `L` against `^\p{L}+$`, timed). No new test file in `crates/swell/tests/`.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode of the gate, under 2 s with a collection at every safe point, and under 2.5 s at its full count (`SWELL_FULL_COUNTS=1`) in either mode, measured with its test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system); a language script stays under 0.5 CPU seconds per run. The unit times each test before it adds it and names in its report each one over 1 CPU second with its time; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the steps run a process per test); no test makes a network request; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`, a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m`, the most), and a script that measures or waits runs without the prefix. No Python in any tool, test or command. None of the tests of the crates is expected over 0.5 CPU seconds; the long subject is timed and named.
- The cost of the generated cases: each of the 441 cases of `generated/` builds a String of its code points and of their complement in script (`buildString` of `harness/regExpUtils.js`, up to 1,114,112 code points) and matches it. The unit times `slot --timeout 30m task test262` (in the background) before and after its change and reports both wall times and the slowest runs the runner prints; a run of more than 10 s of CPU, or a rise of the wall time of the whole run by more than half, is a defect of this story, fixed in it (in the matcher's loop over one class or the String path), never by a raised limit; the budgets of a run are decision 11's (2^30 steps, 512 MiB).
- Time of the generator and size of the tables: the unit times `target/debug/swell-unicode-gen --check` before and after its change, three runs each under `time` of the shell, and reports wall and CPU time and the number of tables; it reports the ranges of each new table and the bytes of its constant (the data above gives about 17,000 ranges of 8 bytes, near 140 KB); a rise to more than 0.5 s of CPU is fixed in the same change.
- Size: about 900 lines with the tests, besides the generated tables.

## Requirements

- HOST-191: its line does not change; its test changes as Change says.

Writes: `crates/unicode-gen/src/property.rs` (new), `crates/unicode-gen/src/tables.rs` (the entries), `crates/unicode/src/tables/{general_category,script,script_extensions,binary_properties,property_names}.rs` (new), `crates/unicode/src/tables/mod.rs`, `crates/unicode/src/property.rs` (new), `crates/unicode/src/lib.rs`, `crates/unicode/tests/property.rs` (new), `crates/unicode/tests/main.rs` (generated), `crates/regexp/src/{parser,early,compile,lib}.rs`, `crates/regexp/tests/property_escapes.rs` (new), `crates/regexp/tests/early_errors.rs`, `crates/regexp/tests/main.rs` (generated), `crates/syntax/tests/sources/refused/regexp-property-escape.js` (removed), `crates/syntax/tests/sources/valid/regexp-literals.js`, `crates/swell/tests/regexp_refusals.rs`. Not written: `conformance/test262/expected.txt` and `expected.digest` (the coordinator), the list files. Shared with the story that may run beside it, `story:regexp-symbol-methods-and-string-match-search`: none (it writes `crates/swell/src` and its own tests). `story:identifiers-and-group-names-outside-ascii`, `story:regexp-case-insensitive-matching-and-modifiers` and `story:regexp-v-flag-class-sets-and-properties-of-strings` write `crates/regexp/src/parser.rs`, the lists of the generator and of `swell-unicode` and `regexp_refusals.rs` after this story has landed.

## Acceptance

`cargo nextest run --locked -p swell-unicode --test main`, `cargo nextest run --locked -p swell-unicode-gen --test main`, `cargo nextest run --locked -p swell-regexp --test main`, `cargo nextest run --locked -p swell-syntax --test main`, `cargo nextest run --locked -p swell --test main -E 'test(/^regexp_refusals::/) | test(/^regexp_objects::/) | test(/^source_scan::/)'` with and without `--features collect-every-safe-point`, and `cargo nextest run --locked -p swell-test262 --test main` exit 0 with every test named in Change passing; `slot task unicode-data-check` exits 0 and `slot task unicode-tables-check` prints `unicode-tables: <n> table(s) equal the generator output` with `<n>` five more than on the base of the unit; `grep -rc 'property escape is not supported' crates/regexp/src | awk -F: '{ s += $2 } END { print s }'` prints `0`; `slot task clippy` exits 0; `slot --timeout 30m task test262` is run once after the change (in the background) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes copied by `git show "$WAVE:docs/plan/slice-05/story-property-escapes.cases.txt" > .out/story-property-escapes.cases.txt` (`WAVE` the wave branch the unit merges into), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-property-escapes.cases.txt .out/test262/runs.tsv` exits 0 (on `e444aaf` it exits 1 and prints 1176 runs), and (b) with the run of the base commit of the unit made before the change (`slot --timeout 30m task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing (no run that passed on the base fails). The report gives the generator'\''s times, the sizes of the tables and the two wall times of the run as Change says.

## Out of scope

Properties of strings and `\p` under `v` (`story:regexp-v-flag-class-sets-and-properties-of-strings`); a property escape under `i` (`story:regexp-case-insensitive-matching-and-modifiers`); identifiers and group names outside ASCII (`story:identifiers-and-group-names-outside-ascii`); loose matching of names (ECMA-262 admits the exact names and aliases only).

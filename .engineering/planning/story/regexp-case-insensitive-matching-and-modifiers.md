---
format: aep.planning-md/3
id: story:regexp-case-insensitive-matching-and-modifiers
kind: story
status: draft
title: 'Case-insensitive matching and the modifier i: Canonicalize with simple case folding under u and v and the uppercase mapping under neither'
summary: 88 failing cases (176 runs); the table of CaseFolding.txt; classes, property escapes and backreferences under i; the refusal of i goes; HOST-191's i part.
relations:
- decomposes: epic:slice-05-regexp
- depends_on: story:case-mapping-tables-and-string-case-methods
- depends_on: story:regexp-property-escapes-and-property-tables
- depends_on: story:regexp-match-all-and-escape
scope:
- confidence: cited
  path: crates/regexp/src/canonicalize.rs
- confidence: cited
  path: crates/regexp/src/compile.rs
- confidence: cited
  path: crates/regexp/src/lib.rs
- confidence: cited
  path: crates/regexp/src/matcher.rs
- confidence: cited
  path: crates/regexp/tests/case_insensitive.rs
- confidence: cited
  path: crates/regexp/tests/main.rs
- confidence: cited
  path: crates/regexp/tests/matcher.rs
- confidence: cited
  path: crates/swell/tests/regexp_refusals.rs
- confidence: cited
  path: crates/unicode-gen/src/case_folding.rs
- confidence: cited
  path: crates/unicode-gen/src/tables.rs
- confidence: cited
  path: crates/unicode-gen/tables.txt
- confidence: cited
  path: crates/unicode/src/case.rs
- confidence: cited
  path: crates/unicode/src/lib.rs
- confidence: cited
  path: crates/unicode/src/tables/case_folding.rs
- confidence: cited
  path: crates/unicode/src/tables/mod.rs
- confidence: cited
  path: crates/unicode/tests/case.rs
revision: 4
---
## Context

Decision 7 of `docs/design/engine.md`: "Case folding: simple case folding from `CaseFolding.txt` under `u` and `v` ..., and `toUppercase` under neither (22.2.2.7.3 `Canonicalize`)", and modifiers `(?i:...)` are in scope. Until this story the compile step of `swell-regexp` refuses the flag `i` and a modifier `i` with `case-insensitive matching is not supported before slice 5` (decision 7, "Refusals until the later stories of slice 5"; `story:regexp-compiler-and-backtracking-matcher`), which RegExpInitialize counts and throws (HOST-191), also when a literal with `i` is evaluated. This story brings the table of case folding (decision 8: `CaseFolding.txt`), Canonicalize in both modes with the uppercase mapping of `story:case-mapping-tables-and-string-case-methods`, case-insensitive matching of characters, classes, property escapes and backreferences, and the modifier `i`, and removes the refusal. It lands after `story:regexp-property-escapes-and-property-tables` (four cases use a property escape under `i`) and `story:regexp-match-all-and-escape` (one case uses `matchAll`).

The list of this story is `docs/plan/slice-05/story-case-insensitive.cases.txt`: 88 failing cases (176 runs): 46 of `built-ins/RegExp/regexp-modifiers/` (8 of them in `syntax/valid/`; 4 of them `add-ignoreCase-affects-slash-lower-p.js` and its three siblings, with a property escape), 20 of `built-ins/RegExp/`, 5 of `RegExp/lookBehind/`, 4 of `language/literals/regexp/`, 8 of `RegExp/prototype/*/` and 5 of `String/prototype/{replace,replaceAll,search,matchAll}/`; on the base 78 end on the counted refusal of the literal and 9 on `RegExp is not defined`. Found by `docs/plan/slice-05/scan.awk` (its head gives the command); the eight cases named `*-err-*` with a modifier `i` are not here (their pattern fails its parse before the compile step, so they are on the lists of `story:regexp-literal-early-errors` and `story:regexp-objects-exec-and-literals`). A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part.

Table, from the vendored file (counted on `98df839` with `awk`, a measure of the data): the 1,512 lines of status `C` and `S` of `CaseFolding.txt` (the simple case folding; `F` and `T` are not read).

Design it builds on: decision 7 (case folding; "Refusals until the later stories of slice 5"; "What one step of the matcher is": a backreference charges a step per code unit it compares, also under `i`); decision 8; `story:unicode-gen-table-list-and-source-digests`.

## Domain relations

- None (the matcher of `swell-regexp` and a table of `swell-unicode`).

## Change

Failing tests first: the tests below are written before the code; on the base `i` is refused.

- Generator: `crates/unicode-gen/src/case_folding.rs` (new), its renderer in `crates/unicode-gen/src/tables.rs` and its line in `crates/unicode-gen/tables.txt` (with the writer of pairs the case story adds; `crates/unicode-gen/tests/check.rs` takes the table from the list and is not written here): `crates/unicode/src/tables/case_folding.rs` (new), the pairs of a code point and its simple folding sorted by code point, with its source and digest; `crates/unicode/src/tables/mod.rs`: its `mod` line. `crates/unicode/src/case.rs` (of the case story): `simple_case_fold` beside the mappings, exported from `crates/unicode/src/lib.rs`.
- `crates/regexp/src/canonicalize.rs` (new): Canonicalize (`#sec-runtime-semantics-canonicalize-ch`): under `u` or `v` the simple case folding; under neither the uppercase mapping of the code unit when it is a single code unit, and the code unit itself when the mapping is several or maps a unit of 128 or above to one below 128; the closure of a class under it (the code points whose canonical value is that of a member), computed when the class is compiled, charged through the trait, linear in the class's ranges and their equivalents.
- `crates/regexp/src/compile.rs` and `src/matcher.rs`: the refusal of `i` goes; under `i` (the flag, or `(?i:...)` and `(?-i:...)` for their atoms, `#sec-compilepattern` with the modifiers) a character and a backreference compare canonical values, a class (property escapes included) matches by its closure, `\w`, `\W`, `\b` and `\B` under `u` or `v` with `i` add U+017F and U+212A (`#sec-compiletocharset`, WordCharacters). A backreference still charges a step per code unit compared.
- What expects the refusal (the rule of the slice: a story that removes a refusal changes every test, script and fixture that expects it, and runs each), found by `grep -rn 'case-insensitive matching\|case_insensitive_matching_is_a_refusal' crates/*/tests crates/*/src` on the base of the unit. From the stories of part a it prints at least these, each changed here: (1) `crates/regexp/src/compile.rs`, the refusal itself; (2) `crates/regexp/tests/matcher.rs`, `case_insensitive_matching_is_a_refusal_of_slice_5` (`story:regexp-compiler-and-backtracking-matcher`): it goes; (3) `crates/swell/tests/regexp_refusals.rs`, `host_191_regexp_counts_its_refusals_of_slice_5`: the parts of `i` take the answer HOST-191 gives once slice 5 is delivered (`new RegExp("a", "i").test("A")` and `/a/i.test("A")` answer `Normal(true)` with the count unchanged). It runs, unchanged, `crates/syntax/tests/regexp_literals.rs`, `a_literal_with_i_and_an_early_error_is_the_early_error` (`story:regexp-literal-early-errors`: an early error still comes first). No fixture under `crates/syntax/tests/sources/refused/` names `i` (the refusal is at compile, not at parse). Before the change the unit runs the search and names in its report any other hit (a test of `crates/swell/tests/regexp_objects.rs` or a language script of part a that expects the refusal), changed in the same way.
- Tests: `crates/unicode/tests/case.rs`: `simple_case_folding_follows_its_table`; `crates/regexp/tests/case_insensitive.rs` (new; `main.rs` generated): `canonicalize_without_u_uses_the_uppercase_mapping_and_keeps_ascii_apart` (`/ſ/i` does not match `s`, `/K/i` does not match `k`), `canonicalize_with_u_uses_simple_case_folding` (both match under `u`), `a_class_matches_by_its_closure`, `a_negated_class_under_i`, `a_property_escape_under_i`, `a_backreference_under_i_charges_a_step_per_code_unit`, `the_modifier_i_sets_its_atoms_only`, `word_characters_under_u_and_i`. No new test file in `crates/swell/tests/`.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode of the gate, under 2 s with a collection at every safe point, and under 2.5 s at its full count (`SWELL_FULL_COUNTS=1`) in either mode, measured with its test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system); a language script stays under 0.5 CPU seconds per run. The unit times each test before it adds it and names in its report each one over 1 CPU second with its time; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the steps run a process per test); no test makes a network request; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`, a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m`, the most), and a script that measures or waits runs without the prefix. No Python in any tool, test or command. None of these tests is expected over 0.2 CPU seconds.
- Time of the generator and size of the table: the unit times `target/debug/swell-unicode-gen --check` before and after its change, three runs each under `time` of the shell, and reports wall and CPU time and the number of tables; it reports the pairs of the new table and the bytes of its constant; a rise to more than 0.5 s of CPU is fixed in the same change.
- Size: about 600 lines with the tests, besides the generated table.

## Requirements

- HOST-191: its line does not change; its test changes as Change says.

Writes: `crates/unicode-gen/src/case_folding.rs` (new), `crates/unicode-gen/src/tables.rs` (the renderer), `crates/unicode-gen/tables.txt` (one line), `crates/unicode/src/tables/case_folding.rs` (new), `crates/unicode/src/tables/mod.rs`, `crates/unicode/src/{case,lib}.rs`, `crates/unicode/tests/case.rs`, `crates/regexp/src/{canonicalize,compile,matcher,lib}.rs` (`canonicalize.rs` new), `crates/regexp/tests/{case_insensitive,main}.rs` (new and generated), `crates/regexp/tests/matcher.rs`, `crates/swell/tests/regexp_refusals.rs`. Not written: `crates/unicode-gen/tests/check.rs` (it takes the table from the list), `crates/regexp/src/parser.rs` and `crates/regexp/tests/early_errors.rs` (`i` is refused at compile), `conformance/test262/expected.txt` and `expected.digest` (the coordinator), the list files. Shared with `story:identifiers-and-group-names-outside-ascii`, which no edge orders against this one: `crates/swell/tests/regexp_refusals.rs` (each changes its own part of `host_191_regexp_counts_its_refusals_of_slice_5`) and the generated `crates/regexp/tests/main.rs` (each adds its test module); neither reads what the other makes; whichever lands second keeps the part and the module of the other beside its own and writes `main.rs` again with `sh taskfiles/test-main.sh write`. Shared with `story:normalization-tables-normalize-and-locale-compare`, which no edge orders against this one: `crates/unicode/src/lib.rs`, `crates/unicode/src/tables/mod.rs`, `crates/unicode-gen/src/tables.rs`, `crates/unicode-gen/tables.txt` and the generated `crates/unicode/tests/main.rs`; each adds its own lines (an export, a `mod` line, a renderer, its lines of the list; this story adds no test module to `crates/unicode/tests/`, its test going into `case.rs`) and reads nothing the other makes; whichever lands second keeps the lines of the first beside its own and runs the check of the generator again. `story:regexp-v-flag-class-sets-and-properties-of-strings` depends on this story and writes `crates/regexp/src/{compile,matcher}.rs`, the lists of the generator and of `swell-unicode` and `regexp_refusals.rs` after this story has landed.

## Acceptance

`cargo nextest run --locked -p swell-unicode --test main`, `cargo nextest run --locked -p swell-unicode-gen --test main`, `cargo nextest run --locked -p swell-regexp --test main`, `cargo nextest run --locked -p swell-syntax --test main -E 'test(/^regexp_literals::/)'`, `cargo nextest run --locked -p swell --test main -E 'test(/^regexp_refusals::/) | test(/^regexp_objects::/) | test(/^regexp_literal_limits::/) | test(/^language::/)'` with and without `--features collect-every-safe-point`, and `cargo nextest run --locked -p swell-test262 --test main` exit 0 with every test named in Change passing; `slot task unicode-tables-check` prints `unicode-tables: <n> table(s) equal the generator output` with `<n>` one more than on the base of the unit; `grep -rc 'case-insensitive matching is not supported' crates/regexp/src | awk -F: '{ s += $2 } END { print s }'` prints `0`, and the search of Change for the refusal, `grep -rn 'case-insensitive matching is not supported\|case_insensitive_matching_is_a_refusal' crates/*/tests crates/*/src`, prints nothing; `slot task clippy` exits 0; `slot --timeout 30m task test262` is run once after the change (in the background) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes copied by `git show "$WAVE:docs/plan/slice-05/story-case-insensitive.cases.txt" > .out/story-case-insensitive.cases.txt` (`WAVE` the wave branch the unit merges into), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-case-insensitive.cases.txt .out/test262/runs.tsv` exits 0 (on `e444aaf` it exits 1 and prints 176 runs), and (b) with the run of the base commit of the unit made before the change (`slot --timeout 30m task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing (no run that passed on the base fails). The report gives the times of the generator and the size of the table as Change says.

## Out of scope

The full case folding (`F`) and the Turkic one (`T`) of `CaseFolding.txt` (ECMA-262 names the simple folding); case folding of the strings of a class under `v` (`story:regexp-v-flag-class-sets-and-properties-of-strings`); the moves of cases (`story:slice-05-conformance-close`).

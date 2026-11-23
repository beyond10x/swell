---
format: aep.planning-md/3
id: story:unicode-gen-table-list-and-source-digests
kind: story
status: draft
title: 'swell-unicode-gen: every table one entry of a list, with shared readers and writers and the digest of each source in the table'
summary: 'The prefactor of the six table stories of slice 5: no new table, no case lifted; the check of the generator timed before and after (0.008 to 0.017 s wall, 0.00 s CPU on 98df839).'
relations:
- decomposes: epic:slice-05-regexp
scope:
- confidence: cited
  path: crates/unicode-gen/src/main.rs
- confidence: cited
  path: crates/unicode-gen/src/render.rs
- confidence: cited
  path: crates/unicode-gen/src/tables.rs
- confidence: cited
  path: crates/unicode-gen/src/ucd.rs
- confidence: cited
  path: crates/unicode-gen/tests/check.rs
- confidence: cited
  path: crates/unicode-gen/tests/main.rs
- confidence: cited
  path: crates/unicode-gen/tests/ucd.rs
- confidence: cited
  path: crates/unicode/src/tables/zs.rs
revision: 2
---
## Context

Slice 5 adds the tables of decision 8 (`docs/design/engine.md`) that `swell-unicode` does not have yet: today `swell-unicode-gen` (`crates/unicode-gen/src/main.rs`, 163 lines) writes one table, `crates/unicode/src/tables/zs.rs` (General_Category `Zs`, 7 ranges), from one data file, with the reading of that file, the merging of its ranges and the text of the table written for `Zs` alone. Six stories of the slice add tables to it: `story:case-mapping-tables-and-string-case-methods` (from `UnicodeData.txt` 1,505 code points with a simple uppercase mapping and 1,488 with a lowercase one, and the 119 lines of `SpecialCasing.txt`), `story:normalization-tables-normalize-and-locale-compare` (5,914 decompositions, 2,081 of them canonical, 968 code points with a combining class other than 0, 81 composition exclusions), `story:regexp-property-escapes-and-property-tables` (General_Category 30 values in 4,144 merged ranges, Script 174 values in 984, Script_Extensions 118 sets in 176, the 50 binary properties of ECMA-262's table that the data files hold in 11,869 merged ranges, `ID_Start` 684 and `ID_Continue` 799 among them, and the names of `PropertyAliases.txt` and `PropertyValueAliases.txt`), `story:regexp-case-insensitive-matching-and-modifiers` (the 1,512 lines of status `C` and `S` of `CaseFolding.txt`) and `story:regexp-v-flag-class-sets-and-properties-of-strings` (the 1,441 data lines of `emoji/emoji-sequences.txt` and 1,614 of `emoji/emoji-zwj-sequences.txt`); `story:identifiers-and-group-names-outside-ascii` reads `ID_Start` and `ID_Continue`. Counted on `98df839` from the vendored files with `awk` (merged ranges: adjacent ranges of one value joined), so a measure of the data, not of the tables the stories will write. This story makes the generator take a table as one entry of a list, with the readers and writers those stories share, so each of them adds an entry and its own module and nothing else of the generator; and it writes into every table the digest of its sources, as "Tests stay fast" names the form of a committed generated file. It adds no table and lifts no case.

Measured before this story on `98df839`: `target/debug/swell-unicode-gen --check` (the debug build `cargo build --locked -p swell-unicode-gen` makes, three runs under `time` of the shell) took 0.008 to 0.017 s of wall time and 0.00 s of CPU and printed `unicode-tables: 1 table(s) equal the generator output`. The gate runs it as `unicode-tables-check` (`taskfiles/unicode-tables.yml`) through `cargo run`, and decision 8 has the gate render every table in memory and compare it with the committed file.

Design it builds on: decision 8 (the vendored data under `data/unicode/17.0.0/` with `SOURCE`, the committed tables, the check in memory); decision 2 (`swell-unicode-gen` is a binary crate that nothing that ships depends on); no new design.

## Domain relations

- None (a tool of the repository; no record of the engine gains a relation).

## Change

Failing tests first: the tests below are written before the code and fail on the base (the table has no digest line; the generator has no list).

- `crates/unicode-gen/src/ucd.rs` (new): the readers of the data files the tables need, each reading a file once per run: a file of lines `<code point or range> ; <value> [; ...] # ...` (the derived files, `PropList.txt`, `Scripts.txt`, `ScriptExtensions.txt`, `emoji-data.txt`) to the merged inclusive ranges of each value asked for; `UnicodeData.txt` to its fields by code point, with the ranges written as `<..., First>` and `<..., Last>` pairs read as ranges; a file of sequences (`emoji/*.txt`) to its code point sequences with their property. Each answers an error naming the file and the line for a line it cannot parse, and for a file whose header names another Unicode version.
- `crates/unicode-gen/src/render.rs` (new): the writers of the text of a table: a constant of sorted, disjoint inclusive ranges `&[(u32, u32)]`, a constant of pairs `&[(u32, u32)]` sorted by key, and a constant of sequences; each table file starts with the line `// Generated by swell-unicode-gen from <sources> (Unicode 17.0.0); do not edit.` as `zs.rs` does today, followed by one line per source `// Source: <path> sha256 <hex>`, the digest as `data/unicode/17.0.0/SOURCE` gives it for that path.
- `crates/unicode-gen/src/tables.rs` (new): the list of tables, one entry per table (the path of the table under `crates/unicode/src/tables/`, its sources, the function that renders it), and `Zs` as its first entry, moved out of `main.rs`. `main.rs` reads `SOURCE` once, runs every entry, writes or (with `--check`) compares; with `--check` it first compares the digest lines of each committed table with `SOURCE` and names a table whose digest differs before it renders anything, then renders every table in memory and compares the text, as now; it prints `unicode-tables: <n> table(s) equal the generator output` as now. A source of an entry that `SOURCE` does not list is an error (exit 2).
- `crates/unicode/src/tables/zs.rs`: written again by the generator, with its source line (`ucd/extracted/DerivedGeneralCategory.txt`, `sha256 d62e5bab70ca74f0...`); its ranges do not change. `crates/unicode/src/tables/mod.rs` and `crates/unicode/src/lib.rs` do not change.
- Tests, in `crates/unicode-gen/tests/check.rs` in its form (the binary run on the workspace or on a copy under `CARGO_TARGET_TMPDIR`): `each_table_names_the_sha256_of_each_source_as_source_gives_it`, `a_digest_line_that_differs_from_source_fails_the_check_naming_the_table`, `a_source_that_source_does_not_list_is_an_error`; and `crates/unicode-gen/tests/ucd.rs` (new), over small files written for the test under `CARGO_TARGET_TMPDIR`: `a_range_file_reads_to_merged_ranges_per_value`, `unicode_data_reads_its_first_and_last_pairs_as_ranges`, `a_line_it_cannot_parse_names_the_file_and_the_line`, `a_file_of_another_version_is_an_error`; `tests/main.rs` generated.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode of the gate, under 2 s with a collection at every safe point, and under 2.5 s at its full count (`SWELL_FULL_COUNTS=1`) in either mode, measured with its test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system); a language script stays under 0.5 CPU seconds per run. The unit times each test before it adds it and names in its report each one over 1 CPU second with its time; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the steps run a process per test); no test makes a network request; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`, a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m`, the most), and a script that measures or waits runs without the prefix. No Python in any tool, test or command. None of these tests is expected over 0.1 CPU seconds.
- Time of the generator ("Tests stay fast": "Before you add a dependency of a generator ... time the generator before and after"): the unit times `target/debug/swell-unicode-gen --check` before and after its change, three runs each under `time` of the shell, and reports wall and CPU time of each; the time on `98df839` is in Context, and a rise to more than 0.1 s of CPU is fixed in the same change. Each later table story times it in the same way before and after its own table and names the size of its table (ranges, pairs or sequences, and the bytes of the constant) in its report.
- Size: about 400 lines with the tests.

Writes: `crates/unicode-gen/src/main.rs`, `crates/unicode-gen/src/{ucd,render,tables}.rs` (new), `crates/unicode-gen/tests/check.rs`, `crates/unicode-gen/tests/ucd.rs` (new), `crates/unicode-gen/tests/main.rs` (generated), `crates/unicode/src/tables/zs.rs` (written again). Not written: `crates/unicode/src/{lib.rs,tables/mod.rs}`, `taskfiles/unicode-tables.yml` (its command and its line do not change), `data/unicode/` (nothing is vendored), `Cargo.lock` (no dependency: the digest is read from `SOURCE`, which `unicode-data-check` holds against the files, not computed). Shared with the stories that may run beside it, `story:regexp-crate-and-pattern-parser` and `story:string-replace-replace-all-and-split`: none.

## Acceptance

`cargo nextest run --locked -p swell-unicode-gen --test main` and `cargo nextest run --locked -p swell-unicode --test main` exit 0 with every test named in Change passing (on the base the tests of the digest fail); `slot task unicode-data-check` exits 0; `slot task unicode-tables-check` prints `unicode-tables: 1 table(s) equal the generator output`; `head -2 crates/unicode/src/tables/zs.rs | tail -1` prints `// Source: ucd/extracted/DerivedGeneralCategory.txt sha256 d62e5bab70ca74f099343f71224fa051cb1fdd61a1ab45c0488c44cfc0b6102e` with the digest that `SOURCE` gives for that path; `git diff --stat -- crates/unicode/src/tables/zs.rs` shows only the added source line; `slot task clippy` exits 0; the report gives the generator's times before and after as Change says.

## Out of scope

Every new table and every function of `swell-unicode` that reads one (the six stories named in Context); a change of the vendored data or of `SOURCE`; a digest computed by the generator (the vendored files are held against `SOURCE` by `unicode-data-check`).

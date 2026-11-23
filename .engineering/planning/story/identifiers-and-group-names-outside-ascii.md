---
format: aep.planning-md/3
id: story:identifiers-and-group-names-outside-ascii
kind: story
status: active
title: Identifiers and regular expression group names outside ASCII, by ID_Start and ID_Continue
summary: 105 failing cases (210 runs); the three refusals of the lexer and the refusal of group names go; HOST-191's group-name part.
relations:
- decomposes: epic:slice-05-regexp
- depends_on: story:regexp-property-escapes-and-property-tables
- depends_on: story:regexp-symbol-methods-and-string-match-search
scope:
- confidence: cited
  path: crates/regexp/src/parser.rs
- confidence: cited
  path: crates/regexp/tests/early_errors.rs
- confidence: cited
  path: crates/regexp/tests/group_names.rs
- confidence: cited
  path: crates/regexp/tests/main.rs
- confidence: cited
  path: crates/swell/tests/regexp_literal_limits.rs
- confidence: cited
  path: crates/swell/tests/regexp_refusals.rs
- confidence: cited
  path: crates/syntax/src/lexer.rs
- confidence: cited
  path: crates/syntax/src/lib.rs
- confidence: cited
  path: crates/syntax/tests/identifiers.rs
- confidence: cited
  path: crates/syntax/tests/main.rs
- confidence: cited
  path: crates/syntax/tests/sources/early-errors/identifier-not-id-start.js
- confidence: cited
  path: crates/syntax/tests/sources/early-errors/regexp-group-name-not-id-start.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/identifier-escape-outside-ascii.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/identifier-outside-ascii.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/identifier-start-outside-ascii.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/regexp-group-name-outside-ascii.js
- confidence: cited
  path: crates/syntax/tests/sources/valid/identifiers-outside-ascii.js
- confidence: cited
  path: crates/syntax/tests/sources/valid/regexp-literals.js
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T03:24:22Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T03:24:23Z", actor: "human:operator", revision: 6, executor: "agent:coordinator"}
---
## Context

Since slice 1 the lexer refuses an identifier with a code point outside ASCII, written or escaped (`crates/syntax/src/lexer.rs` lines 334, 368 and 392: `identifier with a code point outside ASCII` and `identifier with a \u escape outside ASCII`, refusals of slice 5; the flags after a regular expression literal, line 710), because `ID_Start` and `ID_Continue` (12.7, UAX #31) are tables of slice 5 (decision 8; `conformance/test262/sections.txt` line 76, `omit sec-names-and-keywords identifiers-outside-ascii section-12-part-before-its-slice`). The pattern parser of `swell-regexp` refuses a group name with a code point outside ASCII in the same way (`group name outside ASCII`, decision 7, "Refusals until the later stories of slice 5"; HOST-164 in a literal, HOST-191 through `RegExp`). `story:regexp-property-escapes-and-property-tables` writes the two tables and `swell_unicode::is_id_start` and `is_id_continue`; this story reads them in both places and removes the three refusals.

The list of this story is `docs/plan/slice-05/story-identifiers.cases.txt`: 105 failing cases (210 runs): 79 of `language/identifiers/`, 14 of `language/literals/regexp/named-groups/` (13 negative parse cases that need the tables to reject a name, `invalid-non-id-start-groupspecifier*.js` and the others), 6 of `built-ins/RegExp/named-groups/`, 2 each of `language/white-space/` and `language/line-terminators/`, 1 of `RegExp/prototype/Symbol.replace/` and 1 of `RegExp/match-indices/`. On the base 88 end on a refusal of the lexer, 13 on `the case parsed; a SyntaxError was expected at parse` (the pattern parser accepts no name outside ASCII yet, so it cannot reject one), 2 on a counted refusal and 2 on a `ReferenceError` where a `SyntaxError` is expected. Found by `docs/plan/slice-05/scan.awk` (its head gives the command). A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part.

Design it builds on: decision 8 (`ID_Start`, `ID_Continue` from `DerivedCoreProperties.txt`); decision 7 ("Refusals until the later stories of slice 5"); section 12, "A part before its slice" (the identifier refusal of slice 1, lifted in slice 5); `docs/design/host-interface.md`, decision 13 (HOST-164, HOST-191).

## Domain relations

- None (the lexer and the pattern grammar; no record of the engine gains a relation).

## Change

Failing tests first: the fixtures and tests below are written before the code; on the base they are refused.

- `crates/syntax/src/lexer.rs`: IdentifierStartChar and IdentifierPartChar (`#sec-names-and-keywords`) outside ASCII by `swell_unicode::is_id_start` and `is_id_continue` (U+200C and U+200D as parts), written or as a `\u` escape (its code point checked as the clause asks, a surrogate pair read as one code point), at lines 334, 368 and 392; the flags after a literal (line 710) read as IdentifierPartChar, so a code point outside ASCII there is the early error of an unknown flag, not a refusal. The comments of lines 17, 332 and 347 and of `crates/syntax/src/lib.rs` line 20 say the refusals are gone.
- `crates/regexp/src/parser.rs`: RegExpIdentifierName (`#prod-RegExpIdentifierName`) with ID_Start and ID_Continue, `$` and `_`, U+200C and U+200D, `\u` escapes with and without braces and the surrogate pair of two escapes under `u` and without it (`#sec-patterns-static-semantics-early-errors`, the RegExpIdentifierStart and RegExpIdentifierPart rules), the refusal of `group name outside ASCII` gone; group names compared by their code points.
- Fixtures and the refusal tests: `crates/syntax/tests/sources/refused/{identifier-outside-ascii,identifier-start-outside-ascii,identifier-escape-outside-ascii,regexp-group-name-outside-ascii}.js` removed, their forms added to `crates/syntax/tests/sources/valid/` (`identifiers-outside-ascii.js`, new, and `regexp-literals.js`) and their invalid forms to `crates/syntax/tests/sources/early-errors/` (`identifier-not-id-start.js`, `regexp-group-name-not-id-start.js`, new, each with the header line of the directory); the tests named in the next item.
- What expects the refusals (the rule of the slice: a story that removes a refusal changes every test, script and fixture that expects it, and runs each), found by `grep -rn 'identifier with a\|group name outside ASCII\|outside-ascii' crates/*/tests crates/*/src` on the base of the unit. On `8073b8e` it prints `crates/syntax/src/lexer.rs` (lines 17, 334, 368, 392, 710), `crates/syntax/src/lib.rs` (line 20) and the three fixtures `refused/identifier-{outside,start-outside,escape-outside}-ascii.js`; from the stories of part a it prints at least these more, each changed here: (1) `crates/regexp/src/parser.rs`, the refusal of a group name; (2) `crates/regexp/tests/early_errors.rs`, `the_v_flag_a_property_escape_and_a_group_name_outside_ascii_are_refusals_of_slice_5` (`story:regexp-crate-and-pattern-parser`; after the property story it holds `v` and a group name): it keeps the flag `v` only; (3) `crates/syntax/tests/sources/refused/regexp-group-name-outside-ascii.js` (`story:regexp-literal-early-errors`): removed, as the item above says; (4) `crates/swell/tests/regexp_literal_limits.rs`, `a_refused_pattern_of_a_literal_is_counted_as_a_refused_parse` (`story:regexp-literal-early-errors`; after the property story it holds `v` and a group name): its group name outside ASCII is taken out, a literal `/(?<é>a)/` now parses and leaves `refusals()` unchanged, and the flag `v` still moves it by 1; (5) `crates/swell/tests/regexp_refusals.rs`, `host_191_regexp_counts_its_refusals_of_slice_5`: `new RegExp("(?<\\u{e9}>a)").exec("a").groups["é"]` answers `"a"` with the count unchanged. Before the change the unit runs the search and names in its report any other hit (a test of `crates/swell/tests/regexp_objects.rs` or a language script that expects the refusal), changed in the same way.
- Tests: `crates/syntax/tests/identifiers.rs` (new; `main.rs` generated): `identifiers_outside_ascii_lex_by_id_start_and_id_continue`, `an_escape_outside_ascii_names_its_code_point`, `a_code_point_that_is_not_id_start_is_an_early_error`, `a_flag_outside_ascii_is_an_unknown_flag`; `crates/regexp/tests/group_names.rs` (new; `main.rs` generated): `group_names_outside_ascii_with_and_without_u`, `an_escaped_group_name_equals_the_written_one`, `a_name_that_is_not_id_start_is_an_early_error`. No new test file in `crates/swell/tests/`.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode of the gate, under 2 s with a collection at every safe point, and under 2.5 s at its full count (`SWELL_FULL_COUNTS=1`) in either mode, measured with its test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system); a language script stays under 0.5 CPU seconds per run. The unit times each test before it adds it and names in its report each one over 1 CPU second with its time; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the steps run a process per test); no test makes a network request; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`, a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m`, the most), and a script that measures or waits runs without the prefix. No Python in any tool, test or command. None of these tests is expected over 0.2 CPU seconds; the syntax step at the full count (`SWELL_FULL_COUNTS=1`) is timed before and after, since the lexer is in every parse.
- Size: about 400 lines with the fixtures and tests.

## Requirements

- HOST-191: its line does not change; its test changes as Change says. HOST-164: unchanged (its refused source `r` is one the engine still refuses, `crates/swell/tests/eval.rs`; the unit checks that it is not an identifier outside ASCII and names it in its report).

Writes: `crates/syntax/src/{lexer,lib}.rs`, `crates/syntax/tests/sources/refused/` (the four fixtures removed), `crates/syntax/tests/sources/valid/{identifiers-outside-ascii,regexp-literals}.js`, `crates/syntax/tests/sources/early-errors/{identifier-not-id-start,regexp-group-name-not-id-start}.js` (new), `crates/syntax/tests/identifiers.rs` (new), `crates/syntax/tests/main.rs` (generated), `crates/regexp/src/parser.rs`, `crates/regexp/tests/{group_names,main}.rs` (new and generated), `crates/regexp/tests/early_errors.rs`, `crates/swell/tests/regexp_refusals.rs`, `crates/swell/tests/regexp_literal_limits.rs` (the group name out of `a_refused_pattern_of_a_literal_is_counted_as_a_refused_parse`). Not written: `crates/unicode` (the tables and the two functions are `story:regexp-property-escapes-and-property-tables`'s; an error found in them goes back), `conformance/test262/sections.txt` (line 76 goes in `story:slice-05-conformance-close`), `conformance/test262/expected.txt` and `expected.digest` (the coordinator), the list files. Shared with the story that may run beside it, `story:regexp-match-all-and-escape`: none (it writes `crates/swell/src` and its own tests). `story:regexp-v-flag-class-sets-and-properties-of-strings` depends on this story (its edge: it reads the removal of the group-name refusal) and writes after it `crates/regexp/src/parser.rs`, `crates/regexp/tests/early_errors.rs`, the generated `crates/regexp/tests/main.rs`, `regexp_refusals.rs`, `regexp_literal_limits.rs` and `crates/syntax/tests/sources/valid/regexp-literals.js`. Shared with `story:regexp-case-insensitive-matching-and-modifiers`, which no edge orders against this one: `crates/swell/tests/regexp_refusals.rs` (each changes its own part of `host_191_regexp_counts_its_refusals_of_slice_5`) and the generated `crates/regexp/tests/main.rs` (each adds its test module). Neither reads what the other makes, so no edge orders them (it writes neither `parser.rs` nor `early_errors.rs`); whichever lands second keeps the part and the module of the other beside its own and writes `main.rs` again with `sh taskfiles/test-main.sh write`.

## Acceptance

`cargo nextest run --locked -p swell-syntax --test main`, the same with `SWELL_FULL_COUNTS=1` and `-E 'test(/^sources::/)'`, `cargo nextest run --locked -p swell-regexp --test main`, `cargo nextest run --locked -p swell --test main -E 'test(/^regexp_refusals::/) | test(/^regexp_literal_limits::/) | test(/^regexp_objects::/) | test(/^eval::/) | test(/^language::/)'` with and without `--features collect-every-safe-point`, and `cargo nextest run --locked -p swell-test262 --test main` exit 0 with every test named in Change passing; `grep -c '"identifier with a' crates/syntax/src/lexer.rs` prints `0` (on `98df839` `4`); `grep -rc 'group name outside ASCII' crates/regexp/src | awk -F: '{ s += $2 } END { print s }'` prints `0`, and the search of Change for the refusals, `grep -rn '"identifier with a\|group name outside ASCII is not supported' crates/*/tests crates/*/src`, prints nothing (`regexp_literal_limits::a_refused_pattern_of_a_literal_is_counted_as_a_refused_parse` passes with the flag `v` alone); `slot task clippy` and `slot task conform-engine` exit 0; `slot --timeout 30m task test262` is run once after the change (in the background) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes copied by `git show "$WAVE:docs/plan/slice-05/story-identifiers.cases.txt" > .out/story-identifiers.cases.txt` (`WAVE` the wave branch the unit merges into), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-identifiers.cases.txt .out/test262/runs.tsv` exits 0 (on `e444aaf` it exits 1 and prints 210 runs), and (b) with the run of the base commit of the unit made before the change (`slot --timeout 30m task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing (no run that passed on the base fails).

## Out of scope

The tables of `ID_Start` and `ID_Continue` (`story:regexp-property-escapes-and-property-tables`); `Other_ID_Start` and the other stability additions beyond what `DerivedCoreProperties.txt` already folds into the two properties; the line of `sections.txt` (`story:slice-05-conformance-close`).

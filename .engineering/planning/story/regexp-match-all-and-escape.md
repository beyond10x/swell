---
format: aep.planning-md/3
id: story:regexp-match-all-and-escape
kind: story
status: draft
title: RegExp.prototype[@@matchAll], %RegExpStringIteratorPrototype%, String.prototype.matchAll and RegExp.escape
summary: 83 failing cases (166 runs); the last methods of decision 7 that need no new table, on the operations of the first wave.
relations:
- decomposes: epic:slice-05-regexp
- depends_on: story:regexp-symbol-methods-and-string-match-search
scope:
- confidence: cited
  path: crates/swell/src/builtins/regexp/escape.rs
- confidence: cited
  path: crates/swell/src/builtins/regexp/match_all.rs
- confidence: cited
  path: crates/swell/src/builtins/regexp/mod.rs
- confidence: cited
  path: crates/swell/src/builtins/string/pattern.rs
- confidence: cited
  path: crates/swell/src/object/ordinary.rs
- confidence: cited
  path: crates/swell/src/realm/intrinsics.rs
- confidence: cited
  path: crates/swell/tests/language/regexp-match-all.js
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: cited
  path: crates/swell/tests/regexp_match_all.rs
revision: 2
---
## Context

After `story:regexp-symbol-methods-and-string-match-search` the methods of `%RegExp.prototype%` keyed by `@@match`, `@@replace`, `@@search` and `@@split` and `String.prototype.match` and `search` exist; two parts of decision 7 and of the epic's outcome remain that need no new table: `RegExp.prototype [ %Symbol.matchAll% ]` with `%RegExpStringIteratorPrototype%` and `String.prototype.matchAll` (the feature `String.prototype.matchAll`, the block lines `dir test/built-ins/RegExpStringIteratorPrototype/ 5` and `dir test/built-ins/String/prototype/matchAll/ 5`), and `RegExp.escape` (`#sec-regexp.escape`, the feature `RegExp.escape`). This story brings both, on the operations of the earlier stories (RegExpExec, RegExpCreate, AdvanceStringIndex, SpeciesConstructor), which it calls and does not write.

The list of this story is `docs/plan/slice-05/story-match-all-and-escape.cases.txt`: 83 failing cases (166 runs): 25 of `built-ins/RegExp/prototype/Symbol.matchAll/`, 21 of `built-ins/String/prototype/matchAll/`, 17 of `built-ins/RegExpStringIteratorPrototype/`, 1 of `built-ins/RegExp/named-groups/`, and the 19 of `built-ins/RegExp/escape/`; found by `docs/plan/slice-05/scan.awk` (its head gives the command). The one case of `matchAll` that uses the flag `i` (`String/prototype/matchAll/flags-nonglobal-throws.js`) is on the list of `story:regexp-case-insensitive-matching-and-modifiers`, which lands after this story. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part.

Design it builds on: decision 7 (in scope: `RegExp.escape`, the methods of `String`), decision 3 ("The step budget": each pass of a loop charges a step, each match its own); no new design.

## Domain relations

- None (methods of `%RegExp%`, `%RegExp.prototype%`, `%String.prototype%` and one iterator prototype).

## Change

Failing tests first: the tests below are written before the code; on the base none of these properties exists.

- `crates/swell/src/builtins/regexp/match_all.rs` (new): `RegExp.prototype [ %Symbol.matchAll% ]` (`#sec-regexp-prototype-%symbol.matchall%`: SpeciesConstructor of `%RegExp%`, `flags` read by Get and ToString, Construct of the matcher with them, `lastIndex` copied by ToLength, `g` and the full unicode flag from the flags), CreateRegExpStringIterator (`#sec-createregexpstringiterator`) and `%RegExpStringIteratorPrototype%` (`#sec-%regexpstringiteratorprototype%-object`) with `next` (`#sec-%regexpstringiteratorprototype%.next`: RegExpExec, an empty match advancing `lastIndex` by AdvanceStringIndex, done after one match without `g`) and `@@toStringTag` `"RegExp String Iterator"`, its prototype `%IteratorPrototype%`. The iterator's slots ([[IteratingRegExp]], [[IteratedString]], [[Global]], [[Unicode]], [[Done]]) are a variant of `OrdinaryData` beside the Array and String Iterators (`crates/swell/src/object/ordinary.rs` line 47; two handles and three booleans, `Copy`), traced at its kind site.
- `crates/swell/src/builtins/regexp/escape.rs` (new): `RegExp.escape ( S )` (`#sec-regexp.escape`: a TypeError for a value that is not a String; EncodeForRegExpEscape, `#sec-encodeforregexpescape`, of each code point: an initial ASCII letter or digit as `\x` and two hex digits, the syntax characters and `/` escaped with `\`, the control escapes `\t \n \v \f \r`, the other punctuators of the clause (`,`, `-`, `=`, `<`, `>`, `#`, `&`, `!`, `%`, `:`, `;`, `@`, `~`, the apostrophe, the grave accent and the quotation mark) and white space, line terminators and lone surrogates as `\x` or `\u` escapes, `swell_unicode::is_space_separator` for `Zs`), through `StringBuilder` (bounded by `string_length`, charged before it grows).
- `crates/swell/src/builtins/string/pattern.rs` (the group of `story:regexp-symbol-methods-and-string-match-search`): `String.prototype.matchAll` (`#sec-string.prototype.matchall`: for a RegExp-like argument the `g` check by Get of `flags` and its TypeError, GetMethod of `@@matchAll` and its Call; otherwise RegExpCreate with `"g"` and Invoke of `@@matchAll`) in the list of the group.
- Registration: `crates/swell/src/builtins/regexp/mod.rs` (the `mod` lines, `escape` on the constructor, `[Symbol.matchAll]` on the prototype); `crates/swell/src/realm/intrinsics.rs` (`Intrinsic::RegExpStringIteratorPrototype`, made as `%StringIteratorPrototype%` is, line 99).
- Steps and memory: each `next` charges a step besides its match; `RegExp.escape` a step per code unit read and written. Re-entry: Get of `flags`, the species constructor, ToLength of `lastIndex` and a user `exec` are calls of the call path; nothing is added to the frame of `Agent::execute`, and the stack tests pass on seven eighths of `min_stack_bytes()`.
- Tests, each in both collection modes: `crates/swell/tests/regexp_match_all.rs` (new): `match_all_iterates_every_match_and_advances_past_an_empty_one`, `match_all_without_g_throws_from_string_match_all_and_answers_one_match_from_the_symbol_method`, `the_iterator_reads_last_index_of_a_species_matcher`, `the_iterator_prototype_has_next_and_its_tag`, `the_iterator_survives_a_collection_between_two_next_calls`, `regexp_escape_escapes_each_class_of_code_point` (rows of the clause's table: an initial digit, `.`, `/`, `,`, a space, U+2028, a lone surrogate), `regexp_escape_throws_for_a_non_string`, `a_global_match_all_over_a_long_string_ends_at_the_step_budget`, `a_species_constructor_re_enters_on_the_promised_stack` (`reach_through` of `tests/support/stack.rs`); language script (new) `crates/swell/tests/language/regexp-match-all.js`.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode of the gate, under 2 s with a collection at every safe point, and under 2.5 s at its full count (`SWELL_FULL_COUNTS=1`) in either mode, measured with its test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system); a language script stays under 0.5 CPU seconds per run. The unit times each test before it adds it and names in its report each one over 1 CPU second with its time; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the steps run a process per test); no test makes a network request; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`, a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m`, the most), and a script that measures or waits runs without the prefix. No Python in any tool, test or command. None of these tests is expected over 0.5 CPU seconds; the step-budget test uses a small `steps_per_entry`.
- Size: about 600 lines with the tests.

Writes: `crates/swell/src/builtins/regexp/{match_all,escape}.rs` (new), `crates/swell/src/builtins/regexp/mod.rs`, `crates/swell/src/builtins/string/pattern.rs`, `crates/swell/src/object/ordinary.rs` (the iterator's variant and its trace), `crates/swell/src/realm/intrinsics.rs`, `crates/swell/tests/regexp_match_all.rs` (new), `crates/swell/tests/main.rs` (generated), `crates/swell/tests/language/regexp-match-all.js` (new). Not written: `builtins/regexp/{exec,symbol}.rs` and `ops/objects.rs` (it calls their operations; an error found in them goes back to their stories), `conformance/test262/expected.txt` and `expected.digest` (the coordinator), the list files. Shared with the story that may run beside it, `story:identifiers-and-group-names-outside-ascii`: none (it writes `crates/syntax`, `crates/regexp` and `regexp_refusals.rs`). The generated `crates/swell/tests/main.rs` is shared with `story:string-replace-replace-all-and-split`, `story:regexp-literal-early-errors`, `story:regexp-objects-exec-and-literals` and `story:regexp-symbol-methods-and-string-match-search`, which each add a test file to `crates/swell/tests/`: the unit that lands second runs `sh taskfiles/test-main.sh write` after the join.

## Acceptance

The tests named in Change pass in both collection modes, run by the binary of this change and not as a whole step: `cargo nextest run --locked -p swell --test main -E 'test(/^regexp_match_all::/) | test(/^regexp_symbol_methods::/) | test(/^language::/) | test(/^builtins_global_this::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/)'` and the same with `--features collect-every-safe-point`, and `cargo nextest run --locked -p swell-test262 --test main` exit 0, and `slot task clippy` exits 0; `slot --timeout 30m task test262` is run once after the change (in the background) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes copied by `git show "$WAVE:docs/plan/slice-05/story-match-all-and-escape.cases.txt" > .out/story-match-all-and-escape.cases.txt` (`WAVE` the wave branch the unit merges into), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-match-all-and-escape.cases.txt .out/test262/runs.tsv` exits 0 (on `e444aaf` it exits 1 and prints 166 runs), and (b) with the run of the base commit of the unit made before the change (`slot --timeout 30m task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing (no run that passed on the base fails).

## Out of scope

Case-insensitive matching (`story:regexp-case-insensitive-matching-and-modifiers`); the `v` flag (`story:regexp-v-flag-class-sets-and-properties-of-strings`); the moves of cases (`story:slice-05-conformance-close`).

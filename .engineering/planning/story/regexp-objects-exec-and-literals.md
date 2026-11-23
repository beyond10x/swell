---
format: aep.planning-md/3
id: story:regexp-objects-exec-and-literals
kind: story
status: draft
title: 'RegExp objects: the constructor, exec, test, the flag getters, and one new object per evaluation of a literal'
summary: 919 failing cases (1,837 runs) on its list; all 12 literals of the consumer evaluate and its calls of test answer; HOST-143 and acceptance 5 of the epic; the literal refusal and the fixtures that hold it go.
relations:
- decomposes: epic:slice-05-regexp
- depends_on: story:regexp-literal-early-errors
- depends_on: story:regexp-compiler-and-backtracking-matcher
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: conformance/ess/Cargo.lock
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: conformance/test262/sections.txt
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/lib.rs
- confidence: cited
  path: crates/compiler/tests/compile.rs
- confidence: cited
  path: crates/compiler/tests/sources/regexp.disasm
- confidence: cited
  path: crates/compiler/tests/sources/regexp.js
- confidence: cited
  path: crates/swell/Cargo.toml
- confidence: cited
  path: crates/swell/src/builtins/mod.rs
- confidence: cited
  path: crates/swell/src/builtins/object/mod.rs
- confidence: cited
  path: crates/swell/src/builtins/regexp/budget.rs
- confidence: cited
  path: crates/swell/src/builtins/regexp/exec.rs
- confidence: cited
  path: crates/swell/src/builtins/regexp/mod.rs
- confidence: cited
  path: crates/swell/src/builtins/string/access.rs
- confidence: cited
  path: crates/swell/src/heap/collect.rs
- confidence: cited
  path: crates/swell/src/host/agent.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/mod.rs
- confidence: cited
  path: crates/swell/src/interp/regexp.rs
- confidence: cited
  path: crates/swell/src/object/builtin_function.rs
- confidence: cited
  path: crates/swell/src/object/mod.rs
- confidence: cited
  path: crates/swell/src/object/ordinary.rs
- confidence: cited
  path: crates/swell/src/realm/global_object.rs
- confidence: cited
  path: crates/swell/src/realm/intrinsics.rs
- confidence: cited
  path: crates/swell/tests/eval.rs
- confidence: cited
  path: crates/swell/tests/language/literal-regular-expression.js
- confidence: cited
  path: crates/swell/tests/language/regexp-exec.js
- confidence: cited
  path: crates/swell/tests/language/regexp-literals.js
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: cited
  path: crates/swell/tests/regexp_objects.rs
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/error.rs
- confidence: cited
  path: crates/syntax/src/lib.rs
- confidence: cited
  path: crates/syntax/tests/sources.rs
- confidence: cited
  path: crates/test262/src/run.rs
- confidence: cited
  path: crates/test262/tests/fixtures/dynamic-refusal/test/language/dynamic-refusal/regexp-literal-caught.js
- confidence: cited
  path: crates/test262/tests/run.rs
- confidence: cited
  path: website/data/status.json
revision: 44
---
## Context

After `story:regexp-literal-early-errors` and `story:regexp-compiler-and-backtracking-matcher` a pattern parses and matches in `swell-regexp` and a literal is checked at parse, but evaluating a literal still throws the refusal (`crates/compiler/src/expression.rs` line 281 emits `ThrowSyntaxError` of `REGEXP_REFUSAL`, `crates/syntax/src/error.rs` line 26), and `RegExp` is not defined: 843 runs over all slices end on `RegExp is not defined` on `e444aaf` (741 of slice 5, 68 of 19, 24 of 9, 10 of 18). This story brings the RegExp objects: the constructor, `%RegExp.prototype%` but its methods keyed by well-known symbols, RegExpExec and RegExpBuiltinExec, and the literal evaluated to a new object at each evaluation; it closes HOST-143 and holds the acceptance 5 of the epic (`/(a+)+$/` against 10,000 `a` followed by `!` ends with `Terminated(StepBudget)`).

Lifts of the consumer (`docs/plan/slice-05/consumer-regexps.tsv`): after this story all 12 literals evaluate; the call of row 2 answers (`RegExp.prototype.test`, `resources/testharness.js` line 584), and so do the two constructions of a RegExp from a String (`testharness.js` lines 1389 and 4817) and the calls of `test` on lines 2025, 4822 and 4827. The other rows call `match`, `replace` and `split` with a regular expression (`story:regexp-symbol-methods-and-string-match-search`).

The list of this story is `docs/plan/slice-05/story-objects-exec.cases.txt`: 919 failing cases (1,837 runs) of slice 5 that need no part of a later story of the slice, found by `docs/plan/slice-05/scan.awk` (its head gives the command) over the first failing run of each failing case of slice 5 on `e444aaf`. By directory: `built-ins/RegExp/` (its files and the subdirectories not named here) 458, `RegExp/prototype/exec` 75, `built-ins/Object` 69, `RegExp/prototype/test` 43, `language/literals/regexp` 28, `language/white-space` 26, `RegExp/regexp-modifiers` 24 (`m` and `s` modifiers and the errors of modifiers through the constructor), `RegExp/prototype/flags` 13, `language/statementList` 12, `RegExp/CharacterClassEscapes` 12, `RegExp/prototype/source` 11, `RegExp/match-indices` 11, the rest in 23 directories of 10 cases or fewer. On the base 470 of them end on a counted refusal (the literal) and 265 on `RegExp is not defined`. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on: decision 7; decision 3 ("The step budget", "Depth bounds": the pattern of the constructor past `parser_nesting` is a `SyntaxError` the script catches, HOST-143); and three changes of the decomposition a design author writes before this story starts: D1 (the charging trait of `swell-regexp` over the accounting of the agent: bytes to the memory budget, steps to the step budget, the interrupt), D2 (where a RegExp object keeps [[RegExpMatcher]]: `OrdinaryData` is `Copy`, `crates/swell/src/object/ordinary.rs` line 25, and a program is not; a kind of object or a handle into an arena of programs, its bytes charged while the object lives and credited when the collector frees it), and D3 (a literal is parsed and compiled at each evaluation, as RegExpCreate does, recommended; a program shared by the objects of one literal is a later optimisation with a measurement).

## Domain relations

- None. A RegExp object and its program are values of the heap; the host model (`spec/domains/host.yaml`) has no entity for them, and ESS does not restate the slots of ECMA-262 (`AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base (`RegExp` is not defined; a literal throws the refusal).

- `crates/swell/Cargo.toml`: the dependency `swell-regexp = { path = "../regexp" }`; `Cargo.lock` (the dependency in the entry of `swell`), `conformance/ess/Cargo.lock` (the same), `website/data/status.json` written again (`slot --local task site-data`).
- `crates/swell/src/builtins/regexp/budget.rs` (new): the charging trait of the crate over the agent (D1): bytes to the memory budget before each growth, steps to the step budget (`Agent::charge_steps`, `crates/swell/src/host/agent.rs` line 799), the interrupt read; a stop of the crate ends the entry with `Terminated(MemoryBudget)`, `Terminated(StepBudget)` or `Terminated(Interrupted)`, where no `catch` or `finally` runs (decision 3).
- The object (D2): a RegExp object with [[RegExpMatcher]] (the program), [[OriginalSource]] and [[OriginalFlags]] (two Strings the collector traces), its program charged while it lives and credited when it is freed; [[LegacyFeaturesEnabled]] and [[Realm]] of RegExpAlloc are not kept, since `legacy-regexp` is excluded (`conformance/test262/features.txt` line 162) and `RegExp.prototype.compile` is slice 18: the unit writes that omission as a comment of the slot record.
- `crates/swell/src/builtins/regexp/mod.rs` (new): `RegExp ( pattern, flags )` (`#sec-regexp-pattern-flags`: IsRegExp, the `constructor` check when NewTarget is undefined, a RegExp argument read by its slots, a RegExp-like one by Get of `source` and `flags`), RegExpAlloc (`#sec-regexpalloc`: OrdinaryCreateFromConstructor with `%RegExp.prototype%` and the own `lastIndex` property, writable, not enumerable, not configurable), RegExpInitialize (`#sec-regexpinitialize`: ToString of the pattern and the flags, ParsePattern and CompilePattern of the crate with the `parser_nesting` of the agent, a `SyntaxError` of the realm for any error of the pattern, a refusal of slice 5 as the crate names it thrown as the case methods throw theirs (`Agent::count_refusal`, `crates/swell/src/host/agent.rs` line 279, then the `SyntaxError`, as `refuse_before_slice_5` of `builtins/string/transform.rs` line 201 does), so that `refusals()` counts it and a case that expects a `SyntaxError` cannot pass on it (D5 of the decomposition: decision 13 of `docs/design/host-interface.md` names this refusal beside those of a parse, of the literal and of the case methods), and `Set(obj, "lastIndex", 0, true)`), RegExpCreate (`#sec-regexpcreate`); `get RegExp [ %Symbol.species% ]` (`#sec-get-regexp-%symbol.species%`); `%RegExp.prototype%`, an ordinary object (`#sec-properties-of-the-regexp-prototype-object`) with `constructor`, `toString` (`#sec-regexp.prototype.tostring`, `source` and `flags` read by Get) and the accessors `dotAll`, `flags` (`#sec-get-regexp.prototype.flags`, the eight flags read by Get in the order `d g i m s u v y`), `global`, `hasIndices`, `ignoreCase`, `multiline`, `source` (EscapeRegExpPattern, `#sec-escaperegexppattern`: `/` and the line terminators escaped, `(?:)` for the empty pattern), `sticky`, `unicode` and `unicodeSets` (RegExpHasFlag, `#sec-regexphasflag`: undefined for `%RegExp.prototype%` itself, a TypeError for any other object without the slot).
- `crates/swell/src/builtins/regexp/exec.rs` (new): `exec` (`#sec-regexp.prototype.exec`) and `test` (`#sec-regexp.prototype.test`); RegExpExec (`#sec-regexpexec`: a callable `exec` called and its result checked to be an object or null, otherwise RegExpBuiltinExec of an object with the slot) and RegExpBuiltinExec (`#sec-regexpbuiltinexec`: ToLength of `lastIndex`, the flags `g`, `y` and `d` from [[OriginalFlags]], the search of the crate from `lastIndex`, `lastIndex` set on a failure and on a match of `g` or `y`, the match array with `index`, `input`, the captures, `groups` (an object with a null prototype, or undefined) and, under `d`, `indices` with its `groups` (MakeMatchIndicesIndexPairArray, `#sec-makematchindicesindexpairarray`), and code unit indices under `u` from the code point positions of the crate); AdvanceStringIndex (`#sec-advancestringindex`). RegExpExec, RegExpBuiltinExec, RegExpCreate and AdvanceStringIndex are `pub(crate)`: `story:regexp-symbol-methods-and-string-match-search` calls them and does not write them.
- Registration: `builtins/mod.rs` (the `mod regexp;` line and the module comment); `object/builtin_function.rs` (a variant `RegExp(...)` of `BuiltinId`, line 38, and its arm in the dispatch, line 430); `realm/intrinsics.rs` (`Intrinsic::RegExp` and `Intrinsic::RegExpPrototype`, made by CreateIntrinsics as `%Symbol%` is); `realm/global_object.rs` (the constructor property `RegExp`, 19.3.28, in the list of lines 59 to 72, which comes before the properties of `story:global-this-uri-functions-and-is-error`, so `NEW_NAMES` of `crates/swell/tests/builtins_global_this.rs` stays last); the slot record of D2 in the files D2 names (`object/mod.rs`, `object/ordinary.rs`, `heap/collect.rs` for the trace), with every match over the kinds of objects that has no wildcard given its arm.
- Two answers that read the slot: IsRegExp, step 4 (`builtins/string/access.rs` line 235, and the module comment of lines 14 and 15); the builtinTag `RegExp` of `Object.prototype.toString` (`builtins/object/mod.rs` line 539, step 12 of `#sec-object.prototype.tostring`).
- The literal (`#sec-regular-expression-literals-runtime-semantics-evaluation`): `crates/compiler/src/expression.rs` line 281, the arm `Expr::RegExp` emits a new instruction that makes a RegExp from two String constants, the body and the flags (`[] -> [regexp]`); `crates/compiler/src/bytecode.rs` (the variant, documented with its stack effect, and its arm in `Instruction::target`, line 555); `crates/compiler/src/disasm.rs` (its text); `crates/compiler/src/lib.rs` (the module comment, line 20). `crates/swell/src/interp/dispatch.rs`: one arm that calls a function of `crates/swell/src/interp/regexp.rs` (new), which runs RegExpCreate of the realm of the running code (D3), and the `mod` line of `interp/mod.rs`; nothing is added to the frame of `Agent::execute`.
- The refusal goes: `REGEXP_REFUSAL` and its comment (`crates/syntax/src/error.rs` lines 22 to 26), its export (`crates/syntax/src/lib.rs` line 55), and the sentences that state it (`crates/syntax/src/lib.rs` lines 21 and 22, `crates/syntax/src/ast.rs` lines 68 to 70, `crates/syntax/src/early.rs` line 50, `crates/swell/src/interp/dispatch.rs` line 965, `crates/swell/src/host/agent.rs` lines 269 and 270, `crates/test262/src/run.rs` line 478). `conformance/test262/sections.txt` line 79 (`omit sec-literals-regular-expression-literals regexp-literals section-12-part-before-its-slice`) is removed.
- Tests and fixtures that rely on the refusal, changed: `crates/compiler/tests/compile.rs`, `regexp_literal_compiles_to_its_syntax_error_throw` (line 600), becomes `a_regexp_literal_compiles_to_one_creation_of_its_body_and_flags`; `crates/compiler/tests/sources/regexp.js` and `regexp.disasm`, written again; `crates/syntax/tests/sources.rs`, `message_kind_reads_the_regular_expression_literal_refusal` (line 265), removed; `crates/swell/tests/language/literal-regular-expression.js`, which expects the `SyntaxError`, written again to expect a RegExp; `crates/swell/tests/eval.rs`, `host_164_refusals_counts_every_refusal_caught_or_not`: the step `try { /a/; } catch (e) {}` (line 440) is removed and the counts after it move down by one (HOST-164 names that step "while a regular expression literal is refused", `docs/design/host-interface.md` line 1277); `crates/test262/tests/fixtures/dynamic-refusal/test/language/dynamic-refusal/regexp-literal-caught.js` and the test that reads it (`crates/test262/tests/run.rs` line 658) removed, since the fixtures `caught-in-eval.js` and `caught-in-function.js` hold the counted refusal of the runner.
- Bounds: the matcher charges the step budget of the entry and its stacks the memory budget, through the trait; the program is charged while its object lives. Re-entry: ToString of the pattern and the flags, ToLength of `lastIndex`, the Get and the Call of a user `exec` are calls of the call path of the agent; each builtin does its work in functions of its own, `STACK_BYTES_PER_REENTRY` and the stack tests (`limits_scripts`, `budgets`, `eval_stack`) do not change and pass on seven eighths of `min_stack_bytes()`.
- Tests, each in both collection modes (`test-collect`): `crates/swell/tests/regexp_objects.rs` (new): `the_constructor_called_and_constructed_with_a_string_a_regexp_and_a_regexp_like_object`, `an_invalid_pattern_or_flags_throw_a_syntax_error_of_the_realm`, `flags_and_source_read_the_original_flags_and_pattern`, `exec_answers_the_match_array_with_index_input_groups_and_indices`, `exec_moves_last_index_with_g_and_y_and_resets_it_on_a_failure`, `test_answers_through_a_user_exec`, `to_string_reads_source_and_flags_by_get`, `the_flag_getters_answer_undefined_for_the_prototype_and_throw_for_other_objects`, `each_evaluation_of_a_literal_makes_a_new_object` (a literal in a loop gives distinct objects with their own `lastIndex`), `is_reg_exp_and_object_to_string_see_a_regexp`, `host_143_a_pattern_past_parser_nesting_throws_a_catchable_syntax_error` (with `parser_nesting` 1,000, `new RegExp(p)` of 2,000 nested groups throws a `SyntaxError` whose message contains `parser_nesting`, which the script catches), `a_catastrophic_pattern_ends_at_the_step_budget` (`/(a+)+$/` against 10,000 `a` and `!`, with `steps_per_entry` 2,000,000: `Terminated(StepBudget)`; the acceptance 5 of the epic), `a_backtrack_stack_past_the_memory_budget_terminates_the_entry`, `an_interrupt_ends_a_long_match`, `the_program_is_charged_while_the_object_lives` (`live_bytes()` falls after the object is collected), `case_insensitive_matching_is_counted_as_a_refusal` (`new RegExp("a", "i")` throws the refusal of the crate and `refusals()` moves by 1), `last_index_conversion_re_enters_on_the_promised_stack` (`reach_through` of `tests/support/stack.rs`); language scripts (new) `crates/swell/tests/language/regexp-exec.js` and `regexp-literals.js` (one row of `docs/plan/slice-05/consumer-regexps.tsv` each where the call is `test`, written for the test with its constructs).
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode of the gate, under 2 s with a collection at every safe point, and under 2.5 s at its full count (`SWELL_FULL_COUNTS=1`) in either mode, measured with its test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system); a language script stays under 0.5 CPU seconds per run. The unit times each test before it adds it and names in its report each one over 1 CPU second with its time; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the steps run a process per test); no test makes a network request; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`, a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m`, the most), and a script that measures or waits runs without the prefix. No Python in any tool, test or command. The budget tests are bounded by their budgets; none is expected over 1 CPU second, and the step-budget test is timed and named in the report.
- Size: the constructor, the prototype, the two exec operations, the literal and the slot, about 1,500 to 2,000 lines with the tests.

## Requirements

- HOST-143 (`docs/design/host-interface.md` line 317): `conformance/host/requirements.txt` line 143 moves from `HOST-143 pending slice 5` to `HOST-143 test swell::regexp_objects::host_143_a_pattern_past_parser_nesting_throws_a_catchable_syntax_error`. After it no requirement is pending for slice 5 (HOST-142 is `story:regexp-literal-early-errors`).

Writes: `crates/swell/Cargo.toml`; `crates/swell/src/builtins/regexp/{mod,exec,budget}.rs` (new); `crates/swell/src/builtins/mod.rs`; `crates/swell/src/object/builtin_function.rs`; `crates/swell/src/realm/{intrinsics,global_object}.rs`; the slot record of D2 in `crates/swell/src/object/{mod,ordinary}.rs` and `crates/swell/src/heap/collect.rs`; `crates/swell/src/builtins/string/access.rs` (step 4 of IsRegExp and its comment); `crates/swell/src/builtins/object/mod.rs` (the builtinTag); `crates/swell/src/interp/{dispatch,mod}.rs`, `crates/swell/src/interp/regexp.rs` (new); `crates/swell/src/host/agent.rs` (comment lines 269 and 270); `crates/compiler/src/{expression,bytecode,disasm,lib}.rs`; `crates/compiler/tests/compile.rs`, `crates/compiler/tests/sources/regexp.{js,disasm}`; `crates/syntax/src/{error,lib,ast,early}.rs` (the constant and comments only); `crates/syntax/tests/sources.rs` (one test removed); `crates/swell/tests/regexp_objects.rs` (new), `crates/swell/tests/main.rs` (generated), `crates/swell/tests/eval.rs` (one step), `crates/swell/tests/language/{regexp-exec,regexp-literals}.js` (new), `crates/swell/tests/language/literal-regular-expression.js`; `crates/test262/src/run.rs` (a comment), `crates/test262/tests/run.rs` (one test removed), the fixture `regexp-literal-caught.js` (removed); `conformance/host/requirements.txt` (line 143); `conformance/test262/sections.txt` (line 79); `Cargo.lock`; `conformance/ess/Cargo.lock`; `website/data/status.json`. Not written: `crates/regexp` (an error of the crate found here goes back to its story or becomes a draft for the coordinator), `conformance/test262/expected.txt` and `expected.digest` (the coordinator), the list files. The only story of its step of the wave; it lands after `story:regexp-literal-early-errors`, which writes `crates/syntax/src/lib.rs`, `crates/syntax/tests/sources.rs`, `Cargo.lock`, `conformance/ess/Cargo.lock`, `website/data/status.json` and line 142 of `conformance/host/requirements.txt`; `story:string-replace-replace-all-and-split`, which it does not depend on, writes no file of this story (`builtins/string/mod.rs` is its, `access.rs` is this story's alone in the wave).

## Acceptance

The tests named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test main -E 'test(/^regexp_objects::/) | test(/^language::/) | test(/^eval::/) | test(/^builtins_global_this::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/)'` and the same with `--features collect-every-safe-point`, `cargo nextest run --locked -p swell-compiler --test main`, `cargo nextest run --locked -p swell-syntax --test main` and `cargo nextest run --locked -p swell-test262 --test main` exit 0; `slot task clippy` and `slot task conform-engine` exit 0; `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` exits 0 and `grep -c 'pending slice 5$' conformance/host/requirements.txt` prints `0` (on `e444aaf` `2`); `grep -c '^omit sec-literals-regular-expression-literals ' conformance/test262/sections.txt` prints `0` (on `e444aaf` `1`); `slot task lock-check` and `slot --local task site-data-check` exit 0; `slot --timeout 30m task test262` is run once after the change (in the background) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes copied by `git show "$WAVE:docs/plan/slice-05/story-objects-exec.cases.txt" > .out/story-objects-exec.cases.txt`, (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-objects-exec.cases.txt .out/test262/runs.tsv` exits 0 (on `e444aaf` it exits 1 and prints 1,837 runs), (b) `awk -F'\t' '$6 ~ /RegExp is not defined/' .out/test262/runs.tsv | wc -l` prints `0` (on `e444aaf` `843`, in slices 5, 9, 18 and 19), and (c) with the run of the base commit of the unit made before the change (`slot --timeout 30m task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing.

## Out of scope

The methods of `%RegExp.prototype%` keyed by `@@match`, `@@replace`, `@@search` and `@@split`, and `String.prototype.match` and `search` (`story:regexp-symbol-methods-and-string-match-search`); `@@matchAll`, `%RegExpStringIteratorPrototype%` and `String.prototype.matchAll` (a later story of slice 5); `RegExp.escape`; case-insensitive matching, the `v` flag and property escapes; `RegExp.prototype.compile` and the legacy static properties (B.2.4 and `legacy-regexp`); the moves of cases (the conformance close of slice 5).

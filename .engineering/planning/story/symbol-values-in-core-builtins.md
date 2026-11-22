---
format: aep.planning-md/3
id: story:symbol-values-in-core-builtins
kind: story
status: draft
title: 'Symbol values in the builtins of slice 3: the symbol hooks and the defects their cases show'
summary: 230 failing cases of slice 6 on its list, most under built-ins/Array, String and Object; writes group modules of builtins and tests only.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
- depends_on: story:symbol-constructor-and-well-known-symbols
scope:
- confidence: cited
  path: crates/swell/src/builtins/array/iteration.rs
- confidence: cited
  path: crates/swell/src/builtins/array/mutation.rs
- confidence: cited
  path: crates/swell/src/builtins/array/search.rs
- confidence: cited
  path: crates/swell/src/builtins/function.rs
- confidence: cited
  path: crates/swell/src/builtins/json/mod.rs
- confidence: cited
  path: crates/swell/src/builtins/object/enumeration.rs
- confidence: cited
  path: crates/swell/src/builtins/object/integrity.rs
- confidence: cited
  path: crates/swell/src/builtins/string/access.rs
- confidence: cited
  path: crates/swell/src/builtins/string/transform.rs
- confidence: cited
  path: crates/swell/src/ops/conversion.rs
- confidence: cited
  path: crates/swell/src/ops/objects.rs
- confidence: cited
  path: crates/swell/tests/builtins_symbol_values.rs
- confidence: cited
  path: crates/swell/tests/language/builtin-symbol-values.js
revision: 2
---
## Context

Slice 6 is delivered before slices 4 and 5 because it closes most of the first gap of the consumer, the testharness.js tests of the HTML parser of surf (`docs/plan/waves.md`, "The order after slice 3"): 191 refused constructs of slice 6 in `resources/testharness.js` and the scripts of `html/syntax/parsing/html5lib_url.html`, measured with the parser of `main` (`6a43e18`). This story lifts none of the 191. It holds the cases of slice 6 that test a builtin of slice 3 with a Symbol value or a symbol key: `Array.prototype.concat` with `Symbol.isConcatSpreadable`, the methods that read `Symbol.species`, `Object.assign`, `Object.hasOwn` and `Object.getOwnPropertySymbols` with symbol keys, the `String` methods given a Symbol `this` or argument (`trimStart`, `trimEnd`, `indexOf`, `padStart`, `repeat` and the others of the list), `Function.prototype[Symbol.hasInstance]`. The hooks themselves exist on the base (`crates/swell/src/builtins/mod.rs` lines 250 to 271, `builtins/array/mod.rs` line 472, `builtins/object/mod.rs` line 459, `ops/objects.rs` lines 537 and 582, `ops/conversion.rs` line 64); no script could reach them with a Symbol before `story:symbol-constructor-and-well-known-symbols`, so what they do with one is untested. The cases are of slice 6 by their `feature` lines (`Symbol`, `Symbol.species`, `Symbol.isConcatSpreadable` and the others) and read a builtin that slice 3 delivers.

The list of this story is `docs/plan/slice-06/story-symbol-hooks.cases.txt`: 230 failing cases (435 runs) of slice 6 and of the six directories `story:slice-03-conformance-close` moves to slice 6 (`test/built-ins/Array/from/`, `Array/prototype/keys/`, `values/`, `entries/`, `Object/fromEntries/`, `Object/groupBy/`), on no list of the first five stories of the slice, whose text and harness files use a Symbol value or a symbol key with a builtin that slice 3 delivers, no other part of slice 6 than those of the stories this one depends on, and no part of a later slice. Found by the token scan `docs/plan/slice-06/scan.awk` (run as `story:let-const-and-block-scopes` gives it, but with a copy of `meta.tsv` whose includes leave out `propertyHelper.js`, whose three later tokens are a branch behind `typeof`, a case of a bigint value and the name `checkSet`; the comment at the head of `lists-b.awk` gives the command; over the first failing run of each failing case of slice 6 and of those six directories on `main`, `6a43e18`) and the assignment `docs/plan/slice-06/lists-b.awk` (`gawk -v root=<checkout> -v meta=.cache/inventory/meta.tsv -f docs/plan/slice-06/lists-b.awk <scan output>`, the cases of the five lists of the first part taken out), which puts a case on the list of the first story of the slice that holds every part the case needs and whose own construct the case uses. By directory: `built-ins/Array` 88, `built-ins/String` 42, `built-ins/Object` 34, `language/expressions/array` and `language/eval-code` 12 each, `language/expressions/new` and `language/expressions/call` 11 each (object spread in arguments and array literals over the slice 3 builtins of `propertyHelper.js`), `language/expressions/arrow-function` 5, the rest 15. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on: none beyond decision 4 and the design of the stories this one depends on.

## Domain relations

- None new (the builtins read properties of objects and call ToString, ToPropertyKey and ToPrimitive, whose Symbol arms exist).

## Change

Failing tests first: the tests below are written before any fix and fail on the base of the unit wherever a builtin mishandles a Symbol; the unit runs the list first and writes a test for each defect it finds before the fix.

- Each builtin of slice 3 that a case of the list shows wrong with a Symbol value or a symbol key is fixed in the group module that holds it (`crates/swell/src/builtins/array/{iteration,mutation,search}.rs`, `builtins/string/{access,transform}.rs`, `builtins/object/{enumeration,integrity}.rs`, `builtins/function.rs`, `builtins/json/mod.rs`), each defect named in the report with the case that shows it. A defect in a shared operation (`crates/swell/src/ops/{conversion,objects}.rs`) is fixed there and named. No module file (`builtins/{array,object,string}/mod.rs`, `builtins/mod.rs`) is written: `story:iterator-protocol-and-array-string-iterators` and `story:array-from-and-object-from-entries` write those beside it.
- Tests, each in both collection modes (`test-collect`): `crates/swell/tests/builtins_symbol_values.rs` (new): `concat_spreads_an_object_whose_is_concat_spreadable_is_true_and_not_an_array_whose_is_false`, `species_of_an_array_subclass_like_constructor_is_read_once`, `object_assign_copies_symbol_keys_after_string_keys`, `get_own_property_symbols_answers_the_symbol_keys_in_order`, `string_methods_given_a_symbol_throw_type_error` (one call per method of the list, each a `TypeError` and no other error), and one test per defect found. `crates/swell/tests/language/builtin-symbol-values.js` (new). No test uses an iterator, `for`-`of` or a spread.
- Time of the tests ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository): the unit times each test before it adds it, names in its report each one over 5 s with its time and the reason, and adds none over 60 s; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the test steps run a process per test); no test makes a network request; the unit runs the test binaries of its change and never the whole gate. None is expected over 1 s.
- Run loop: no new builtin; a fix keeps each builtin a function of its own (`docs/plan/unit-invariants.md`); `STACK_BYTES_PER_REENTRY` and the stack tests (`limits_scripts.rs`, `budgets.rs`, `eval_stack.rs`) do not change.

Writes: `crates/swell/src/builtins/array/{iteration,mutation,search}.rs`, `crates/swell/src/builtins/string/{access,transform}.rs`, `crates/swell/src/builtins/object/{enumeration,integrity}.rs`, `crates/swell/src/builtins/function.rs`, `crates/swell/src/builtins/json/mod.rs`, `crates/swell/src/ops/{conversion,objects}.rs` (each only for a defect a case of the list shows, named in the report), `crates/swell/tests/builtins_symbol_values.rs` (new), `crates/swell/tests/language/builtin-symbol-values.js` (new). Not written: `conformance/test262/expected.txt` (the coordinator regenerates it on the wave branch after the merges), the list files under `docs/plan/slice-06/` (the coordinator), the module files named in Change, any file of `crates/syntax` or `crates/compiler`. The matches with no wildcard: none gains a variant. Shared: none with `story:iterator-protocol-and-array-string-iterators` and `story:array-from-and-object-from-entries`, which may run beside it (they write the module files, `builtins/iterator.rs`, the `from` modules, `ops/iteration.rs`, `object/{arguments,ordinary}.rs` and the realm files, none of which this story writes).

## Acceptance

The tests and fixtures named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test builtins_symbol_values --test language --test builtins_array --test builtins_string --test builtins_object --test builtins_function --test limits_scripts --test budgets --test eval_stack` (the existing binaries of the modules it may fix) and the same with `--features collect-every-safe-point` exit 0, and `slot task clippy` exits 0; `slot task test262` is run once after the change (the slow lane, in the background through the pool, never a test step) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes, copied by `git show "$WAVE:docs/plan/slice-06/story-symbol-hooks.cases.txt" > .out/story-symbol-hooks.cases.txt` (`WAVE` the wave branch the unit merges into; the coordinator takes a case off the list by a commit there whose body names the part it ends on), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-symbol-hooks.cases.txt .out/test262/runs.tsv` exits 0 (every case of that copy passes in every mode; on `6a43e18`, with the list as committed today, it exits 1 and prints 435 runs), (b) `awk -F'\t' '$3 ~ /^test\/built-ins\/(Array|String|Object|Function)\// && $6 ~ /Symbol is not defined/' .out/test262/runs.tsv | wc -l` prints `0` (on `6a43e18` `487`; `story:symbol-constructor-and-well-known-symbols` takes it to 0 and this story keeps it there), (c) the lines `test262 built-ins/Array` and `test262 built-ins/String` of `.out/test262/lines.txt` each read at least B + L - R passed in the seventh field, where B is the passes of that line on the base of the unit (447 and 172 on `6a43e18`), L the cases of the copy under that directory (88 and 42 on the list of today) and R the cases under it among the runs (a) prints; it is a bound and not the count because a case off the list may pass as well, and (d) with the run of the base commit of the unit made before the change (`slot task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing (no run that passed on the base fails).

## Out of scope

The `RegExp` hooks (`Symbol.match`, `replace`, `search`, `split`, `matchAll`, slice 5); iterators (`story:iterator-protocol-and-array-string-iterators`); typed arrays (slice 13); the moves of cases and the slice lines of the design (`story:slice-06-conformance-close`).

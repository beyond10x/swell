---
format: aep.planning-md/3
id: story:array-from-and-object-from-entries
kind: story
status: draft
title: Array.from, Object.fromEntries and Object.groupBy over iterables
summary: The builtins of the directories slice 3's close moves to slice 6; 77 failing cases on its list.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
- depends_on: story:iterator-protocol-and-array-string-iterators
- depends_on: story:for-of-statement
scope:
- confidence: cited
  path: crates/swell/src/builtins/array/from.rs
- confidence: cited
  path: crates/swell/src/builtins/array/mod.rs
- confidence: cited
  path: crates/swell/src/builtins/object/from_entries.rs
- confidence: cited
  path: crates/swell/src/builtins/object/mod.rs
- confidence: cited
  path: crates/swell/tests/builtins_array_from.rs
- confidence: cited
  path: crates/swell/tests/builtins_object_from_entries.rs
- confidence: cited
  path: crates/swell/tests/language/builtin-array-from.js
- confidence: cited
  path: crates/swell/tests/language/builtin-object-from-entries.js
- confidence: cited
  path: crates/swell/tests/language/builtin-object-group-by.js
revision: 2
---
## Context

Slice 6 is delivered before slices 4 and 5 because it closes most of the first gap of the consumer, the testharness.js tests of the HTML parser of surf (`docs/plan/waves.md`, "The order after slice 3"): 191 refused constructs of slice 6 in `resources/testharness.js` and the scripts of `html/syntax/parsing/html5lib_url.html`, measured with the parser of `main` (`6a43e18`). The measurement names none of these builtins (`docs/plan/waves.md`, the builtins the scripts call): this story lifts none of the 191. It delivers the three builtins of slice 6 that read an iterable: `Array.from` (`#sec-array.from`), `Object.fromEntries` (`#sec-object.fromentries`) and `Object.groupBy` (`#sec-object.groupby`, GroupBy `#sec-groupby`). Their directories, `test/built-ins/Array/from/` (47 cases), `Object/fromEntries/` (25) and `Object/groupBy/` (14), are of slice 3 on the base and move to slice 6 whole by `story:slice-03-conformance-close` (ruled on 2026-11-21: "iterators, slice 6"), with `Array/prototype/keys/`, `values/` and `entries/` (`story:iterator-protocol-and-array-string-iterators`). On the base calling each throws `the value is not a function` (`awk -F'\t' '$3 ~ /^test\/built-ins\/(Array\/from|Object\/fromEntries|Object\/groupBy)\// && $6 ~ /the value is not a function|Cannot use a property of undefined/' .out/test262/runs.tsv | wc -l` prints `46`).

The list of this story is `docs/plan/slice-06/story-builtins.cases.txt`: 77 failing cases (150 runs) of slice 6 and of the six directories `story:slice-03-conformance-close` moves to slice 6 (`test/built-ins/Array/from/`, `Array/prototype/keys/`, `values/`, `entries/`, `Object/fromEntries/`, `Object/groupBy/`), on no list of the first five stories of the slice, whose text and harness files use `Array.from`, `Object.fromEntries` or `Object.groupBy`, no other part of slice 6 than those of the stories this one depends on, and no part of a later slice. Found by the token scan `docs/plan/slice-06/scan.awk` (run as `story:let-const-and-block-scopes` gives it, but with a copy of `meta.tsv` whose includes leave out `propertyHelper.js`, whose three later tokens are a branch behind `typeof`, a case of a bigint value and the name `checkSet`; the comment at the head of `lists-b.awk` gives the command; over the first failing run of each failing case of slice 6 and of those six directories on `main`, `6a43e18`) and the assignment `docs/plan/slice-06/lists-b.awk` (`gawk -v root=<checkout> -v meta=.cache/inventory/meta.tsv -f docs/plan/slice-06/lists-b.awk <scan output>`, the cases of the five lists of the first part taken out), which puts a case on the list of the first story of the slice that holds every part the case needs and whose own construct the case uses. By directory: `built-ins/Array` 41, `built-ins/Object` 36. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on: none beyond decision 3 and the iterator record of `story:iterator-protocol-and-array-string-iterators`.

## Domain relations

- None new (each builtin makes an Array or an ordinary object; the groups of `Object.groupBy` are a list the builtin holds while it runs, rooted, and drops when it returns).

## Change

Failing tests first: the tests below are written before the code and fail on the base (the three are not functions).

- `crates/swell/src/builtins/array/from.rs` (new): `Array.from(items, mapfn, thisArg)`: the iterable path through GetIteratorFromMethod and IteratorStepValue with IteratorClose when `mapfn` or the define throws, and the array-like path (`#sec-array.from`, steps 5 and 7 on); `this` a constructor or not; CreateDataPropertyOrThrow and the final `length` Set; a step per element. `builtins/array/mod.rs`: the registration on `%Array%` and the arm in the `ArrayBuiltin` match (line 173, no wildcard).
- `crates/swell/src/builtins/object/from_entries.rs` (new): `Object.fromEntries(iterable)` through AddEntriesFromIterable (`#sec-add-entries-from-iterable`: each entry an object, its `0` and `1` read, IteratorClose on an abrupt completion), and `Object.groupBy(items, callback)` through GroupBy with the property keys coercion (`#sec-groupby`, AddValueToKeyedGroup), a step per element; the groups are an ordered list charged to the memory budget while the call runs. `builtins/object/mod.rs`: the registration on `%Object%` and the arms in the `ObjectBuiltin` match (line 142, no wildcard).
- `crates/swell/src/ops/iteration.rs` is read and not written: `story:for-of-statement`, which lands before this story, has removed the marks of the operations both call.
- Tests, each in both collection modes (`test-collect`): `crates/swell/tests/builtins_array_from.rs` (new): `from_an_iterable_and_from_an_array_like`, `from_calls_mapfn_with_value_and_index_and_closes_the_iterator_when_it_throws`, `from_with_a_constructor_this_constructs_it`, `from_charges_a_step_per_element` (`last_entry_steps()` after `Array.from(big)` exceeds that after `Array.from(small)`, made by an earlier script with 1,000 and 10 elements, by at least 990), `from_re_enters_through_mapfn_on_the_promised_stack` (`reach_through` of `tests/support/stack.rs`, seven eighths of `min_stack_bytes()`). `crates/swell/tests/builtins_object_from_entries.rs` (new): `from_entries_reads_0_and_1_of_each_entry`, `from_entries_closes_the_iterator_when_an_entry_is_not_an_object`, `group_by_groups_in_order_and_keys_by_to_property_key`, `group_by_charges_a_step_per_element` (the same form), `group_by_re_enters_through_its_callback_on_the_promised_stack`. `crates/swell/tests/language/` (new files): `builtin-array-from.js`, `builtin-object-from-entries.js`, `builtin-object-group-by.js`. No test uses `for`-`of`, a spread or an array pattern; an iterable is an array, a string or an object whose `[Symbol.iterator]` the test writes.
- Time of the tests ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository): the unit times each test before it adds it, names in its report each one over 5 s with its time and the reason, and adds none over 60 s; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the test steps run a process per test); no test makes a network request; the unit runs the test binaries of its change and never the whole gate. None is expected over 1 s.
- Run loop: each builtin is a function of its own (`docs/plan/unit-invariants.md`); each re-enters the interpreter through `next`, `mapfn` or `callback` and has the test at the re-entry limit named above; `STACK_BYTES_PER_REENTRY` and the stack tests do not change.

Writes: `crates/swell/src/builtins/array/from.rs` (new), `crates/swell/src/builtins/array/mod.rs`, `crates/swell/src/builtins/object/from_entries.rs` (new), `crates/swell/src/builtins/object/mod.rs`, `crates/swell/tests/builtins_array_from.rs` (new), `crates/swell/tests/builtins_object_from_entries.rs` (new), `crates/swell/tests/language/{builtin-array-from,builtin-object-from-entries,builtin-object-group-by}.js` (new). Not written: `conformance/test262/expected.txt` (the coordinator regenerates it on the wave branch after the merges), the list files under `docs/plan/slice-06/` (the coordinator), the group modules of `builtins/` (`story:symbol-values-in-core-builtins`, which may run beside it), any file of `crates/syntax` or `crates/compiler`. The matches with no wildcard this story adds arms to: `ArrayBuiltin` (`array/mod.rs` line 173) and `ObjectBuiltin` (`object/mod.rs` line 142). Shared: none with `story:array-destructuring-and-spread`, which may run beside it (that one writes `crates/syntax`, `crates/compiler`, `interp/` and tests). It depends on `story:for-of-statement` for `ops/iteration.rs`, whose marks that story removes, and on `story:iterator-protocol-and-array-string-iterators` for `builtins/array/mod.rs` and the operations.

## Acceptance

The tests and fixtures named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test builtins_array_from --test builtins_object_from_entries --test language --test builtins_array --test builtins_object --test limits_scripts --test budgets --test eval_stack` and the same with `--features collect-every-safe-point`, and `cargo nextest run --locked -p swell --lib` exit 0, and `slot task clippy` exits 0; `slot task test262` is run once after the change (the slow lane, in the background through the pool, never a test step) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes, copied by `git show "$WAVE:docs/plan/slice-06/story-builtins.cases.txt" > .out/story-builtins.cases.txt` (`WAVE` the wave branch the unit merges into; the coordinator takes a case off the list by a commit there whose body names the part it ends on), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-builtins.cases.txt .out/test262/runs.tsv` exits 0 (every case of that copy passes in every mode; on `6a43e18`, with the list as committed today, it exits 1 and prints 150 runs), (b) `awk -F'\t' '$3 ~ /^test\/built-ins\/(Array\/from|Object\/fromEntries|Object\/groupBy)\// && $6 ~ /the value is not a function|Cannot use a property of undefined/' .out/test262/runs.tsv | wc -l` prints `0` (on `6a43e18` `46`), (c) the lines `test262 built-ins/Array` and `test262 built-ins/Object` of `.out/test262/lines.txt` each read at least B + L - R passed in the seventh field, where B is the passes of that line on the base of the unit (447 and 1574 on `6a43e18`), L the cases of the copy under that directory (41 and 36 on the list of today) and R the cases under it among the runs (a) prints; it is a bound and not the count because a case off the list may pass as well, and (d) with the run of the base commit of the unit made before the change (`slot task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing (no run that passed on the base fails).

## Out of scope

`Array.fromAsync` (slice 10); `Map.groupBy` (slice 9); the moves of cases and the slice lines of the design (`story:slice-06-conformance-close`).

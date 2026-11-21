---
format: aep.planning-md/3
id: story:object-destructuring
kind: story
status: draft
title: 'Object destructuring: binding and assignment patterns of objects, CoverInitializedName'
summary: Lifts the 3 object patterns of surf's harness (its 3 array patterns need iterators, a later story); 443 failing cases of slice 6 on its list.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
- depends_on: story:default-and-rest-parameters
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/function.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/compiler/src/statement.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/mod.rs
- confidence: cited
  path: crates/swell/src/interp/patterns.rs
- confidence: cited
  path: crates/swell/tests/language/object-pattern-assignment.js
- confidence: cited
  path: crates/swell/tests/language/object-pattern-catch.js
- confidence: cited
  path: crates/swell/tests/language/object-pattern-let-const.js
- confidence: cited
  path: crates/swell/tests/language/object-pattern-nested-and-defaults.js
- confidence: cited
  path: crates/swell/tests/language/object-pattern-parameter.js
- confidence: cited
  path: crates/swell/tests/language/object-pattern-var.js
- confidence: cited
  path: crates/swell/tests/object_patterns.rs
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/function.rs
- confidence: cited
  path: crates/syntax/src/parser/statement.rs
- confidence: cited
  path: crates/syntax/src/scope.rs
- confidence: cited
  path: crates/syntax/tests/sources/early-errors/cover-initialized-name.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/cover-initialized-name.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/destructuring-binding-object.js
revision: 2
---
## Context

Slice 6 is delivered before slices 4 and 5 because it closes most of the first gap of the consumer, the testharness.js tests of the HTML parser of surf (`docs/plan/waves.md`, "The order after slice 3"): 191 refused constructs of slice 6 in `resources/testharness.js` and the scripts of `html/syntax/parsing/html5lib_url.html`, measured with the parser of `main` (`6a43e18`), of which 6 destructuring patterns. Three are object patterns, all parameters in `html/syntax/parsing/resources/test.js`: of two arrows (`({ iframe, t, id, input, expected, record }) =>`, line 95, and line 125) and of a function (`function printDiff({ id, input, expected, actual, container })`, line 206). This story lifts those 3. The other 3 are array patterns (`for (const [prop, expected] of ...)`, `testharness.js` line 2550; `test.js` lines 75 and 207) and need the iterator protocol: a later story of the slice.

The base refuses a pattern where a binding is read (`binding_identifier`, `crates/syntax/src/parser/statement.rs` line 391, and line 687 for a `for` head), a destructuring assignment (`crates/syntax/src/parser/expression.rs` line 180) and a CoverInitializedName (`a = 1` in an object literal, line 1050). Measured on `main` (`6a43e18`) from the run lines of `slot task test262`: the first failing run of 1,092 cases of slice 6 ends on the refusal of a pattern and of 61 on that of a CoverInitializedName (`awk -F'\t' '$2 == "06" && $5 != "Passed" && !($3 in s) { s[$3] = 1; if ($6 ~ /destructuring pattern is not supported/) d++; if ($6 ~ /CoverInitializedName is not supported/) c++ } END { print d, c }' .out/test262/runs.tsv` prints `1092 61`); most of the generated cases under `dstr/` are array patterns or need `Symbol.iterator`, and stay for the later story.

The list of this story is `docs/plan/slice-06/story-object-pattern.cases.txt`: 443 failing cases of slice 6 (821 runs) whose text and harness files use object patterns and no construct of slice 6 but those of the stories this one depends on, no array pattern, no spread or rest element in a pattern, no computed property name, no `for`-`of`, no part of a later slice and no builtin of slice 3 missing on the base (`docs/plan/slice-06/scan.awk` and `lists.awk`, as `story:let-const-and-block-scopes` gives the command; a case whose detail names a pattern and whose text shows neither kind is not on it, and a case under `dstr/` or `destructuring/` whose name says `ary`, `array` or `iter` counts as an array pattern): `language/expressions/assignment` 94, `language/statements/for` 70, `language/expressions/arrow-function` 60, `language/statements/function` 47, `language/expressions/function` 47, `language/statements/variable`, `try`, `let` and `const` 23 each, `language/expressions/object` 17, the rest 16. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list in the wave record.

Tests and fixtures of earlier waves that rely on the refusal: `crates/syntax/tests/sources/refused/destructuring-binding-object.js` (`var { a } = c;`), removed; `refused/cover-initialized-name.js` (`x = { a = 1 };`), which becomes an early error (`#sec-object-initializer-static-semantics-early-errors`) and moves to `early-errors/cover-initialized-name.js`. The refused fixtures of array patterns (`destructuring-binding.js`, `destructuring-parameter.js`, `destructuring-assignment.js`, `destructuring-for-in.js`) stay.

Design it builds on: nothing beyond what the stories it depends on ask of the design (the uninitialized binding of `story:let-const-and-block-scopes`, the parameters of a list that is not simple of `story:default-and-rest-parameters`); a pattern binds names by the same slots and records.

## Domain relations

- None new. A pattern binds names into the records the stories it depends on name (inferred from `crates/swell/src/env/mod.rs:144`, the field `outer: Option<Handle>`; no ess/1 document declares this relation).

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base (the parser refuses every pattern).

- `swell-syntax`. `parser/statement.rs`: an ObjectBindingPattern (`#sec-destructuring-binding-patterns`) wherever a BindingIdentifier may be a pattern: a `var`, `let` or `const` declarator (with an initializer, but in the head of `for`-`in`), the head of `for` and `for`-`in`, a `catch` parameter; properties by identifier, string or number name, shorthand, a default for each element, nested object patterns. `parser/function.rs`: the pattern as a parameter of a function, method or accessor; `parser/expression.rs`: the pattern as a parameter of an arrow (the cover of `story:arrow-functions`), the ObjectAssignmentPattern read again from an object literal on the left of `=` and in the head of `for`-`in` (`#sec-destructuring-assignment`), its targets any simple assignment target (a name, a member, an index), a CoverInitializedName allowed only there and an early error elsewhere. An array pattern, a rest element `...` in a pattern and a computed property name keep their refusals (`destructuring pattern`, `spread element`, `computed property name`). `ast.rs`: the binding pattern and the assignment pattern. `early.rs`: the BoundNames of a pattern in the rules of the declarations, a target that is not a valid simple assignment target, `eval` and `arguments` as targets in strict code. `scope.rs`: the names a pattern binds as var or lexical names. Fixtures `crates/syntax/tests/sources/valid/object-pattern-*.js` and `early-errors/object-pattern-*.js` (new); the refused fixtures as named in Context.
- `swell-compiler`. `statement.rs`, `expression.rs` and `function.rs`: BindingInitialization (`#sec-runtime-semantics-bindinginitialization`) and DestructuringAssignmentEvaluation (`#sec-runtime-semantics-destructuringassignmentevaluation`) of object patterns: RequireObjectCoercible of the value (`#sec-requireobjectcoercible`, a `TypeError` for `undefined` and `null`, also with the empty pattern `{}`), each property read once by GetV in source order (`#sec-destructuring-binding-patterns-runtime-semantics-propertybindinginitialization`, KeyedBindingInitialization), a default evaluated only for `undefined` with NamedEvaluation for an anonymous function, the reference of an assignment target evaluated before the value is read (`#sec-runtime-semantics-propertydestructuringassignmentevaluation`), the uninitialized `let` and `const` bindings initialized as their pattern binds them; `resolve.rs`: the slots of the names of a pattern; `bytecode.rs`: RequireObjectCoercible and any other new instruction, documented; `disasm.rs`: their text. Fixtures `crates/compiler/tests/sources/object-pattern-*.js` and `.disasm` (new).
- `swell`. `interp/patterns.rs` (new): the work of each new instruction in a function of its own, called from one arm each of `interp/dispatch.rs`; nothing added to the frame of `Agent::execute`; `interp/mod.rs`: the `mod` line. A pattern runs no loop of its own: each property is an instruction and is charged as one.
- Tests, each in both collection modes (`test-collect`): `crates/swell/tests/object_patterns.rs` (new): `a_pattern_of_undefined_or_null_throws_type_error`, `properties_are_read_once_in_source_order` (through getters that record their calls), `a_default_is_evaluated_only_for_undefined`, `an_assignment_target_is_evaluated_before_its_value_is_read`, `a_pattern_of_a_primitive_reads_through_its_wrapper` (`var { length } = "abc"`), `a_parameter_pattern_of_an_arrow_and_a_function_binds_its_names` (the three forms of the measurement), `a_let_pattern_reads_before_initialization_throws_reference_error`. `crates/swell/tests/language/` (new files): `object-pattern-var.js`, `object-pattern-let-const.js`, `object-pattern-parameter.js`, `object-pattern-catch.js`, `object-pattern-assignment.js`, `object-pattern-nested-and-defaults.js`. No test of this story uses a template literal, an array pattern, a spread or rest element or a computed property name.
- Time of the tests ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository): the unit times each test before it adds it, names in its report each one over 5 s with its time and the reason, and adds none over 60 s; a product several tests share is built once inside one test function or is a committed file, never in a `static`; no test makes a network request; the unit runs the test binaries of its change and never the whole gate. None is expected over 1 s.
- Stack: `STACK_BYTES_PER_REENTRY` and the stack tests do not change; a getter a pattern reads re-enters the interpreter as a property read does, on the path the stack tests already measure.

Writes: `crates/syntax/src/{ast,scope,early}.rs`, `crates/syntax/src/parser/{statement,function,expression}.rs`, `crates/syntax/tests/sources/valid/object-pattern-*.js` and `early-errors/object-pattern-*.js` (new), `crates/syntax/tests/sources/refused/destructuring-binding-object.js` (removed), `crates/syntax/tests/sources/refused/cover-initialized-name.js` (moved to `early-errors/cover-initialized-name.js`), `crates/compiler/src/{statement,expression,function,resolve,bytecode,disasm}.rs`, `crates/compiler/tests/sources/object-pattern-*.js` and `object-pattern-*.disasm` (new), `crates/swell/src/interp/patterns.rs` (new), `crates/swell/src/interp/{mod,dispatch}.rs`, `crates/swell/tests/object_patterns.rs` (new), `crates/swell/tests/language/{object-pattern-var,object-pattern-let-const,object-pattern-parameter,object-pattern-catch,object-pattern-assignment,object-pattern-nested-and-defaults}.js` (new). Not written: `conformance/test262/expected.txt` (the coordinator regenerates it on the wave branch after the merges; the run of this unit prints the lines that differ and the unit commits none), any file under `crates/swell/src/builtins/` or `crates/swell/src/ops/`. Shared: no file with a story at its depth (it is the only one); every file of `crates/syntax` and `crates/compiler` it writes is also written by a story it depends on, which has landed before it starts.

## Acceptance

The tests and fixtures named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test object_patterns --test language --test limits_scripts` and the same with `--features collect-every-safe-point`, `cargo nextest run --locked -p swell-syntax --test sources` and `cargo nextest run --locked -p swell-compiler --test compile` exit 0, and `slot task clippy` exits 0; `slot task test262` is run once after the change (the slow lane, in the background through the pool, never a test step) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' docs/plan/slice-06/story-object-pattern.cases.txt .out/test262/runs.tsv` exits 0 (every case of the list, less the ones the wave record takes off it, passes in every mode; on `6a43e18` it exits 1 and prints 821 runs), (b) `awk -F'\t' '$3 ~ /^test\/language\/(expressions\/(assignment|arrow-function|object)|statements\/(for|for-in|for-of))\// && $6 ~ /CoverInitializedName is not supported/' .out/test262/runs.tsv | wc -l` prints `0` (on `6a43e18` `114`), (c) the lines `test262 language/expressions/assignment`, `test262 language/statements/variable` and `test262 language/statements/try` of `.out/test262/lines.txt` each read at least B + L - R passed in their seventh field, where B is the passes of that line on the base of the unit (130, 76 and 93 on `6a43e18`), L the cases of the list under that directory as the list is committed when the wave closes (`grep -c '^test/language/expressions/assignment/' docs/plan/slice-06/story-object-pattern.cases.txt` prints 94 today, `statements/variable/` 23, `statements/try/` 23) and R the cases under it among the runs (a) prints; it is a bound because a case off the list may pass as well, and (d) with the run of the base commit of the unit made before the change (`slot task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing.

## Out of scope

Array patterns and every pattern that reads an iterator (a later story of the slice, with `Symbol.iterator` and the Array iterator); the rest element of an object pattern and object spread (`feature object-rest`, `feature object-spread`, with CopyDataProperties: a later story); computed property names, also in patterns (a later story); patterns in a `for`-`of` head (with `for`-`of`); the moves of cases and the slice lines of the design (the close of the slice).

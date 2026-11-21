---
format: aep.planning-md/3
id: story:with-arguments-and-dynamic-names
kind: story
status: implemented
title: 'with and the arguments object: names resolved at run time where a with body reaches them, numbered slots everywhere else'
summary: Object environment records with @@unscopables, named records and lookups by name charged per record, mapped and unmapped arguments objects; the with and arguments refusals lifted; one omission recorded (@@iterator, slice 6).
relations:
- decomposes: epic:slice-02-language-core
- depends_on: story:syntax-language-core
- depends_on: story:test262-run-causes
- depends_on: story:number-boolean-and-global-functions-early
scope:
- confidence: cited
  path: conformance/test262/expected.txt
- confidence: cited
  path: conformance/test262/sections.txt
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/function.rs
- confidence: cited
  path: crates/compiler/src/lib.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/compiler/src/statement.rs
- confidence: cited
  path: crates/compiler/tests/compile.rs
- confidence: cited
  path: crates/compiler/tests/sources/arguments-mapped.disasm
- confidence: cited
  path: crates/compiler/tests/sources/arguments-mapped.js
- confidence: cited
  path: crates/compiler/tests/sources/arguments-strict.disasm
- confidence: cited
  path: crates/compiler/tests/sources/arguments-strict.js
- confidence: cited
  path: crates/compiler/tests/sources/with-call-this.disasm
- confidence: cited
  path: crates/compiler/tests/sources/with-call-this.js
- confidence: cited
  path: crates/compiler/tests/sources/with-statement.disasm
- confidence: cited
  path: crates/compiler/tests/sources/with-statement.js
- confidence: cited
  path: crates/compiler/tests/sources/with-var-initializer.disasm
- confidence: cited
  path: crates/compiler/tests/sources/with-var-initializer.js
- confidence: cited
  path: crates/swell/src/builtins/object.rs
- confidence: cited
  path: crates/swell/src/env/declarative.rs
- confidence: cited
  path: crates/swell/src/env/function.rs
- confidence: cited
  path: crates/swell/src/env/global.rs
- confidence: cited
  path: crates/swell/src/env/mod.rs
- confidence: cited
  path: crates/swell/src/env/object.rs
- confidence: cited
  path: crates/swell/src/heap/collect.rs
- confidence: cited
  path: crates/swell/src/interp/budget.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/frame.rs
- confidence: cited
  path: crates/swell/src/interp/mod.rs
- confidence: cited
  path: crates/swell/src/object/arguments.rs
- confidence: cited
  path: crates/swell/src/object/ecmascript_function.rs
- confidence: cited
  path: crates/swell/src/object/mod.rs
- confidence: cited
  path: crates/swell/src/realm/intrinsics.rs
- confidence: cited
  path: crates/swell/src/realm/mod.rs
- confidence: cited
  path: crates/swell/src/script.rs
- confidence: cited
  path: crates/swell/tests/language/arguments-callee-throws-in-strict.js
- confidence: cited
  path: crates/swell/tests/language/arguments-length-and-indices.js
- confidence: cited
  path: crates/swell/tests/language/arguments-mapped-aliases-parameters.js
- confidence: cited
  path: crates/swell/tests/language/arguments-shadowed-by-parameter.js
- confidence: cited
  path: crates/swell/tests/language/arguments-to-string-tag.js
- confidence: cited
  path: crates/swell/tests/language/arguments-unmapped-in-strict.js
- confidence: cited
  path: crates/swell/tests/language/with-call-this-is-object.js
- confidence: cited
  path: crates/swell/tests/language/with-null-throws.js
- confidence: cited
  path: crates/swell/tests/language/with-property-shadows-local.js
- confidence: cited
  path: crates/swell/tests/language/with-restores-environment-after-throw.js
- confidence: cited
  path: crates/swell/tests/language/with-typeof-and-delete.js
- confidence: cited
  path: crates/swell/tests/language/with-unscopables-not-observable-before-slice-6.js
- confidence: cited
  path: crates/swell/tests/language/with-var-initializer-sets-object.js
- confidence: cited
  path: crates/swell/tests/names.rs
- confidence: cited
  path: crates/swell/tests/tracing.rs
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/statement.rs
- confidence: cited
  path: crates/syntax/src/scope.rs
- confidence: cited
  path: crates/syntax/tests/sources
- confidence: cited
  path: crates/syntax/tests/sources.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T13:41:53Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"review_outcome":2}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T13:41:53Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"review_outcome":2}}, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T15:49:47Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Slice 1 resolves every name when the script is compiled (`crates/compiler/src/resolve.rs`, module comment: "Slice 1 has no direct `eval`, no `with` and no `arguments` (the parser refuses them), so every binding of a function is known when the script is compiled"): a binding is a local slot, a numbered slot of an environment some hops outward, or a name of the global environment, and the records in the heap are slots without names (`crates/swell/src/env/mod.rs`, `declarative.rs`, `function.rs`). `with` breaks that: inside its body a name may be a property of an object only known at run time (`sec-object-environment-records`, `sec-object-environment-records-hasbinding-n`, with `@@unscopables`), and an assignment to a `var` inside it may land on the object. The arguments object needs the parameters of a non-strict function with a simple parameter list mapped to its indices (`sec-arguments-exotic-objects`, `sec-createmappedargumentsobject`), so those parameters must live where the object can reach them. This story gives the engine names at run time where they are needed and keeps numbered slots everywhere else, and it is the base that direct `eval` builds on (`story:eval-and-function-constructor` marks the functions that contain one as dynamic in the same way).

Measured on the base (`6319af2`): of the 3,120 failing cases of slice 2, the first failed run of 115 ends in the refusal `with statement in non-strict code`, of 128 in `reference to arguments inside a function`, and of 56 more in that refusal inside `harness/propertyHelper.js` (those also need the global `Function`, `story:eval-and-function-constructor`). Across all slices 9,929 runs are refused for `arguments` and 175 for `with`.

Design it builds on, on the branch since merge `63dff4d` (a precondition that holds before wave 2):

- `docs/design/engine.md`, section 3, the rule "Names: a slot fixed at compile time, except where `with` or a non-strict direct `eval` can change what a name means" (line 280 on). This story builds its `with` half: a reference is looked up by name at run time when between it and the scope that would bind it at compile time (the global scope when none does) there lies a `with` statement whose body contains the reference, a function nested inside that body being inside it (item 1); every other reference keeps its compile-time slot (item 2); every binding of every scope that encloses a `with` body, up to the global environment, lives in an Environment Record that keeps the names of its slots, not in a frame-local slot, and references to it under item 2 still use its slot (item 3); a lookup by name charges one step per record it visits, the HasProperty and the `@@unscopables` Get of an object record are property operations charged as any other, and the names of a record's slots are charged to the memory budget with the compiled code (item 5). The direct-`eval` half of items 1, 3 and 4 is `story:eval-and-function-constructor`'s.
- Section 12, the rule that a slice may deliver a feature of its own without a part that needs a later slice, recorded as `omit <section anchor> <part> section-12-slice-<n>`; its first use is this story's, the `@@iterator` property of the arguments object, a part of slice 6.

Code it builds on, checked on this branch: `crates/swell/src/realm/intrinsics.rs` marks `Intrinsic::ThrowTypeError` with `expect(dead_code, reason = "the restricted properties of functions: story:interpreter-and-script-entry-points")`, which this story fulfils (the `callee` accessor of an unmapped arguments object), so it edits that file; `WellKnown` in `crates/swell/src/realm/mod.rs` has four symbols and no `@@unscopables`; `ObjectKind`, `KindTag` and the dispatch of internal methods are in `crates/swell/src/object/mod.rs`, and the collector test that makes one object of every kind is in `crates/swell/src/heap/collect.rs` (line 333 on); `Object.prototype.toString` picks its tag in `crates/swell/src/builtins/object.rs` (step 7, "Arguments" for an object with [[ParameterMap]]); `FunctionDeclarationInstantiation` is in `crates/swell/src/object/ecmascript_function.rs`, whose comment says it has no arguments object until slice 2; `crates/syntax/tests/sources.rs` asserts that `"use\x20strict"; with (o) {}` is refused naming slice 2 (line 535 on, cited), an expectation this story changes, and its `every_nesting_kind_fits_the_stack_bound` has no `with` nesting kind; `code_bytes` in `crates/swell/src/script.rs` (line 125, cited) charges every vector of `Code` and `FunctionTemplate` to the memory budget, so the table of slot names below is charged there (inferred: `Instruction` is `Copy` and holds no vector, `crates/compiler/src/bytecode.rs:55`, so the names go in a table of `Code` or `FunctionTemplate`).

`crates/swell/src/realm/intrinsics.rs` is written by `story:number-boolean-and-global-functions-early` before this story (the intrinsics `%Number%`, `%Boolean%`, `%isNaN%`, `%isFinite%`) and by this story after it (the mark of `Intrinsic::ThrowTypeError`), hence the edge between the two.

## Change

Failing tests first: the moved sources, the language scripts and the tests below are written before the code and fail (refused, or `ReferenceError`).

Parser (`swell-syntax`, after `story:syntax-language-core`):

- `src/parser/statement.rs`: the `with` statement in non-strict code (`sec-with-statement`; in strict code the early error that exists stays); `src/parser/expression.rs`: a reference to `arguments` inside a function is no longer refused; `src/ast.rs`: the `with` statement node; `src/scope.rs`: per function whether its code references `arguments` (the inputs of steps 15 to 18 of `sec-functiondeclarationinstantiation`: not when `arguments` is a parameter name or the name of a function declared in it), and which references lie inside a `with` body.
- `crates/syntax/tests/sources.rs` (a test of an earlier wave whose expectation changes): the escaped directive leaves the code non-strict, so `"use\x20strict"; with (o) {}` now parses, which the test asserts in place of the refusal; the strict form stays the early error `with statement in strict mode code`; `every_nesting_kind_fits_the_stack_bound` gains a `with` statement in a `with` statement.
- `crates/syntax/tests/sources/refused/with-non-strict.js`, `arguments-in-function.js`, `arguments-in-nested-function.js` move to `valid/` (tests of an earlier wave whose expectation changes).

Compiler (`swell-compiler`):

- `src/resolve.rs`: the rule of section 3 for `with` as stated above: a reference with a `with` body between it and its compile-time binding scope is resolved by name (a reference whose binding lies inside the same `with` body, such as a catch parameter declared in it, keeps its slot); every binding of a scope that encloses a `with` body, up to the global scope, gets a named slot in an Environment Record; every other binding and reference stays as in slice 1. The parameters of a function that needs a mapped arguments object live in its function environment, and `arguments` is a binding of that function (`sec-functiondeclarationinstantiation`, steps 15 to 22). The walk stays iterative.
- `src/bytecode.rs` (the contract, documented as slice 1 documents it), `src/disasm.rs`: instructions to push an object environment for `with` (ToObject, `NewObjectEnvironment(obj, true, env)`; `PopEnv` and the `envs` count of a handler cover it), to get, put, `typeof` and `delete` a name resolved through the records (`sec-resolvebinding`, `sec-getidentifierreference`, `sec-getvalue`, `sec-putvalue`, `sec-delete-operator-runtime-semantics-evaluation`), and to read a name for a call with its `this` (`sec-evaluatecall`: the `WithBaseObject` of an object environment of a `with`); the names of the slots of a record that needs them (catch parameter, function, named function expression), as a table of `Code` and of `FunctionTemplate`, since `Instruction` is `Copy` and holds no vector; the arguments object a template asks for (mapped or unmapped).
- `src/statement.rs`, `src/expression.rs`, `src/function.rs`, `src/lib.rs`: the `with` statement (`sec-with-statement-runtime-semantics-evaluation`, its completion value `UpdateEmpty(C, undefined)`, the environment restored on every completion), the name-based forms where `resolve.rs` says so.

Engine (`swell`):

- `src/env/mod.rs`, `src/env/declarative.rs`, `src/env/function.rs`, `src/env/global.rs`: a declarative record whose slots have names when code that resolves names at run time can reach it, with HasBinding, GetBindingValue, SetMutableBinding and DeleteBinding by name (`sec-declarative-environment-records`); `src/env/object.rs`: object Environment Records with `withEnvironment` (`sec-object-environment-records`: HasBinding with `@@unscopables`, GetBindingValue and SetMutableBinding with the `stillExists` check, DeleteBinding, WithBaseObject); the trace of the new record kinds. A lookup by name charges one step per record it visits (`interp/budget.rs`), so the walk is bounded by the step budget as every loop is.
- `src/interp/dispatch.rs`, `src/interp/frame.rs`, `src/interp/mod.rs`, `src/interp/budget.rs`: the new instructions; termination unwinding drops object environments like any other.
- `src/script.rs`: `code_bytes` charges the table of slot names with the other vectors of `Code` and `FunctionTemplate`, nothing else changes.
- `src/object/arguments.rs` (new): the arguments exotic object (`sec-arguments-exotic-objects`: [[GetOwnProperty]], [[DefineOwnProperty]], [[Get]], [[Set]], [[Delete]] over the parameter map), CreateMappedArgumentsObject (`sec-createmappedargumentsobject`, the map reading and writing the parameter slots of the function environment) and CreateUnmappedArgumentsObject (`sec-createunmappedargumentsobject`, `callee` an accessor with `%ThrowTypeError%`), its trace; `src/object/mod.rs`: the kind and its dispatch; `src/object/ecmascript_function.rs`: FunctionDeclarationInstantiation creates and binds the object (steps 15 to 22) and its comment changes; `src/heap/collect.rs`: the arguments object in the test that makes one object of every kind.
- `src/realm/mod.rs`: `@@unscopables` among the well-known symbols (`sec-well-known-symbols`), made and traced as the four others; `src/realm/intrinsics.rs`: the `expect(dead_code)` mark of `Intrinsic::ThrowTypeError` removed, nothing else; `src/builtins/object.rs`: the builtinTag "Arguments" of `Object.prototype.toString`.
- The property `@@iterator` of an arguments object (both create operations: `%Array.prototype.values%` keyed by `%Symbol.iterator%`) needs slice 6. Recorded where the gate reads it, by the rule of section 12 (above): `conformance/test262/sections.txt` gains `section sec-arguments-exotic-objects`, `path sec-arguments-exotic-objects test/language/arguments-object/`, `omit sec-arguments-exotic-objects symbol-iterator-property section-12-slice-6` (263 cases, `grep -c "^test/language/arguments-object/" .cache/inventory/files.txt`), and `conformance/test262/expected.txt` its `test262-section` line (284 lines).

Tests (each test of `crates/swell` runs in both collection modes through `test-collect`):

- `crates/compiler/tests/sources/`: `with-statement.js`, `with-var-initializer.js`, `with-call-this.js`, `arguments-mapped.js`, `arguments-strict.js`, each with its `.disasm`.
- `crates/swell/tests/language/` (new files): `with-property-shadows-local.js`, `with-var-initializer-sets-object.js`, `with-call-this-is-object.js`, `with-unscopables-not-observable-before-slice-6.js` (a `with` over an object without `@@unscopables` reads its properties), `with-null-throws.js`, `with-restores-environment-after-throw.js`, `with-typeof-and-delete.js`, `arguments-mapped-aliases-parameters.js`, `arguments-unmapped-in-strict.js` (strict-only), `arguments-callee-throws-in-strict.js` (strict-only), `arguments-length-and-indices.js`, `arguments-to-string-tag.js`, `arguments-shadowed-by-parameter.js`; the `with` files are `// sloppy-only`.
- `crates/swell/tests/names.rs` (new): `lookup_by_name_charges_a_step_per_record` (a name read through 100 nested `with` statements costs at least 100 steps more than a local read, from `last_entry_steps`), `step_budget_ends_a_loop_of_lookups_by_name`.
- `crates/swell/tests/tracing.rs`: `object_environment_survives_collect` (an object reachable only through the object environment a closure inside a `with` captured), `mapped_arguments_survive_collect` (a mapped arguments object returned from its function still aliases the captured parameter after collections).

Writes: `crates/syntax/src/parser/{statement,expression}.rs`, `crates/syntax/src/{ast,scope}.rs`, `crates/syntax/tests/sources.rs`, `crates/syntax/tests/sources/` (the three moved files), `crates/compiler/src/{resolve,bytecode,disasm,statement,expression,function,lib}.rs`, `crates/compiler/tests/compile.rs`, `crates/compiler/tests/sources/` (the ten new files), `crates/swell/src/env/{mod,declarative,object,function,global}.rs`, `crates/swell/src/interp/{dispatch,frame,mod,budget}.rs`, `crates/swell/src/object/arguments.rs` (new), `crates/swell/src/object/{mod,ecmascript_function}.rs`, `crates/swell/src/script.rs`, `crates/swell/src/heap/collect.rs`, `crates/swell/src/realm/{mod,intrinsics}.rs`, `crates/swell/src/builtins/object.rs`, `crates/swell/tests/names.rs` (new), `crates/swell/tests/tracing.rs`, `crates/swell/tests/language/` (the thirteen new files listed, by name), `conformance/test262/sections.txt`, `conformance/test262/expected.txt`.

## Acceptance

`cargo test --locked -p swell-syntax -p swell-compiler -p swell` and `slot task test-collect` exit 0 and run the tests and sources named in Change, and `slot task test262` exits 0 with the unit lines in `conformance/test262/expected.txt` (284 lines) and prints a `test262-slice 02 5245 9173 0 0 5245 <p> <f> 0` line (its `<p>` bounded below by the comparison that follows: the cases unblocked here also need `eval`, `Function` or a later slice in numbers no measurement before the code separates); after it, with the default `TEST262_OUT` of that run, `awk -F'\t' '$6 ~ /(with statement in non-strict code|reference to arguments inside a function) is not supported/' .out/test262/runs.tsv | wc -l` prints `0` (no run of any slice is refused for either) and `awk 'END { print NR }' .out/test262/runs.tsv` prints the number of runs of the `test262 total` line (the file is the one of this run); and after `git show main:conformance/test262/expected.txt > .out/expected-base.txt` the comparison `awk 'NR == FNR { if ($1 == "test262") p[$1 " " $2] = $7; if ($1 == "test262-slice") p[$1 " " $2] = $8; next } { k = $1 " " $2; v = ($1 == "test262-slice") ? $8 : $7 } ($1 == "test262" || $1 == "test262-slice") && (k in p) && v < p[k] { print; bad = 1 } END { exit bad }' .out/expected-base.txt conformance/test262/expected.txt` exits 0 (no directory line, not the total and no `test262-slice` line passes fewer cases than on `main`, the base of the wave).

## Out of scope

Direct and indirect `eval`, eval code and the functions that contain a direct `eval` (`story:eval-and-function-constructor`, which uses the named records and the lookups by name of this story); the global `Function`; the `@@iterator` property of arguments objects (slice 6, recorded above); `let`, `const` and the declarative part of the global environment (slice 6); Annex B semantics of arguments and `with` (slice 18).

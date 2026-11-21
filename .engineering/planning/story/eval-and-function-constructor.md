---
format: aep.planning-md/3
id: story:eval-and-function-constructor
kind: story
status: active
title: eval and the Function constructor through one parse entry and one engine entry, with the host hooks, every limit and refusals counted; 10 host requirements to test
summary: parse_eval, parse_dynamic_function and the direct-eval mark; PerformEval, EvalDeclarationInstantiation, direct eval, CreateDynamicFunction; HostEnsureCanCompileStrings and HostGetCodeForEval checked; refused parses counted per agent and failing the run in the runner; HOST-116, 117, 118, 139, 140, 141, 152, 162, 163, 164; the 23 harness cases placed again.
relations:
- decomposes: epic:slice-02-language-core
- depends_on: story:with-arguments-and-dynamic-names
- depends_on: story:number-boolean-and-global-functions-early
- depends_on: story:test262-run-causes
scope:
- confidence: cited
  path: conformance/host/requirements.txt
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
  path: crates/compiler/src/lib.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/compiler/tests/compile.rs
- confidence: cited
  path: crates/compiler/tests/sources/direct-eval-call.disasm
- confidence: cited
  path: crates/compiler/tests/sources/direct-eval-call.js
- confidence: cited
  path: crates/compiler/tests/sources/eval-code-var.disasm
- confidence: cited
  path: crates/compiler/tests/sources/eval-code-var.js
- confidence: cited
  path: crates/compiler/tests/sources/function-with-direct-eval.disasm
- confidence: cited
  path: crates/compiler/tests/sources/function-with-direct-eval.js
- confidence: cited
  path: crates/swell/src/builtins/function.rs
- confidence: cited
  path: crates/swell/src/builtins/global.rs
- confidence: cited
  path: crates/swell/src/dynamic_code.rs
- confidence: cited
  path: crates/swell/src/env/declarative.rs
- confidence: cited
  path: crates/swell/src/env/function.rs
- confidence: cited
  path: crates/swell/src/env/global.rs
- confidence: cited
  path: crates/swell/src/env/mod.rs
- confidence: cited
  path: crates/swell/src/host/agent.rs
- confidence: cited
  path: crates/swell/src/host/limits.rs
- confidence: cited
  path: crates/swell/src/host/mod.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/frame.rs
- confidence: cited
  path: crates/swell/src/interp/mod.rs
- confidence: cited
  path: crates/swell/src/lib.rs
- confidence: cited
  path: crates/swell/src/object/builtin_function.rs
- confidence: cited
  path: crates/swell/src/object/ecmascript_function.rs
- confidence: cited
  path: crates/swell/src/realm/global_object.rs
- confidence: cited
  path: crates/swell/src/realm/intrinsics.rs
- confidence: cited
  path: crates/swell/src/script.rs
- confidence: cited
  path: crates/swell/tests/dynamic_function.rs
- confidence: cited
  path: crates/swell/tests/eval.rs
- confidence: cited
  path: crates/swell/tests/eval_stack.rs
- confidence: cited
  path: crates/swell/tests/language/eval-completion-value.js
- confidence: cited
  path: crates/swell/tests/language/eval-direct-reads-local.js
- confidence: cited
  path: crates/swell/tests/language/eval-direct-var-in-function.js
- confidence: cited
  path: crates/swell/tests/language/eval-global-function-not-definable-throws.js
- confidence: cited
  path: crates/swell/tests/language/eval-indirect-is-global.js
- confidence: cited
  path: crates/swell/tests/language/eval-inside-with-reads-object.js
- confidence: cited
  path: crates/swell/tests/language/eval-non-string-is-returned.js
- confidence: cited
  path: crates/swell/tests/language/eval-renamed-is-indirect.js
- confidence: cited
  path: crates/swell/tests/language/eval-strict-var-stays-inside.js
- confidence: cited
  path: crates/swell/tests/language/eval-syntax-error-is-catchable.js
- confidence: cited
  path: crates/swell/tests/language/eval-this-of-caller.js
- confidence: cited
  path: crates/swell/tests/language/function-constructor-injection-is-syntax-error.js
- confidence: cited
  path: crates/swell/tests/language/function-constructor-is-sloppy-unless-body-says.js
- confidence: cited
  path: crates/swell/tests/language/function-constructor-name-anonymous.js
- confidence: cited
  path: crates/swell/tests/language/function-constructor-parameters-and-body.js
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/lib.rs
- confidence: cited
  path: crates/syntax/src/parser/function.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: cited
  path: crates/syntax/src/scope.rs
- confidence: cited
  path: crates/syntax/tests/sources.rs
- confidence: cited
  path: crates/test262/src/run.rs
- confidence: cited
  path: crates/test262/tests/fixtures/dynamic-refusal
- confidence: cited
  path: crates/test262/tests/run.rs
- confidence: cited
  path: docs/design/engine.md
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T13:41:53Z", actor: "human:operator", revision: 11, decided_on: {"recorded":{"review_outcome":4}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T13:41:53Z", actor: "human:operator", revision: 12, decided_on: {"recorded":{"review_outcome":4}}, executor: "agent:coordinator"}
---
## Context

Strings compiled at run time: `eval` (`sec-eval-x`, `sec-performeval`, `sec-evaldeclarationinstantiation`, the direct call of `sec-function-calls-runtime-semantics-evaluation` step 6) and the `Function` constructor (`sec-function-p1-p2-pn-body`, `sec-createdynamicfunction`). Both ask the host first (`sec-hostensurecancompilestrings`, with the seven parameters of the pinned draft; `eval` of an object asks `sec-hostgetcodeforeval`), both parse through one entry of `swell-syntax` and run through one entry of the engine, and this story writes the whole of it: the two parse entries, the mark of a direct `eval` call in scope analysis, the compile step of eval code and the engine entry (decided by the coordinator, revision round 1: one story owns the dynamic-code entry, so that `compile_eval` takes its declaration lists from what `parse_eval` answers and no list is stated twice).

`Function` is in slice 2 by decision (`AGENTS.md`, "Slice 2 delivers the global `Function`, constructor included", 2026-11-21), with conditions on this story, each met below: CreateDynamicFunction goes through the same parse entry as `eval`, with the same nesting limit and the same step and memory charges, no second parser path; a test reaches each limit through `Function(...)`; the acceptance carries the measured count of cases that pass once `harness/propertyHelper.js` loads, per slice; the design line that gives `Function` to slice 3 moves in the same change, the slice counts still summing to 53,801. A `Function` with a prototype only was refused.

Measured on the base (`6319af2`): of the 3,120 failing cases of slice 2, the first failed run of 305 throws `ReferenceError: eval is not defined`, of 70 `Function is not defined`, of 56 the refusal of `arguments` inside `propertyHelper.js` (lifted by `story:with-arguments-and-dynamic-names`; they need `Function` too), and 52 of the wrong results are `assert.throws` around an `eval` or a `Function` call ("Expected a SyntaxError but got a ReferenceError").

Cases that include `propertyHelper.js`, per slice, with the block as on `main` (`awk -f .cache/inventory/slices.awk` over the block and `.cache/inventory/meta.tsv`): 02: 56, 03: 1,183, 04: 304, 05: 66, 06: 240, 07: 65, 08: 991, 09: 201, 10: 659, 11: 3, 12: 18, 13: 311, 14: 64, 15: 145, 16: 21, 18: 155, 19: 17, 20: 428, 21: 231, x: 84 (5,242). Of these, the cases that pass once the helper loads, measured as far as that can be measured before the code exists: a throwaway runner (in `.out/decompose-slice-02/` of the coordinator worktree) gave `propertyHelper.js` a global `Function` whose only member is `prototype` (`%Function.prototype%`) and replaced its four reads of `arguments.length` by the equivalent test of the last parameter, then ran every case on the base engine: 02: 15 of 56, 03: 478 of 1,183 (20 of them the `test/harness` cases below), 04: 4 of 304, every other slice 0 (the cases there fail first on syntax or builtins of their own slices). What stays unknown: cases that call `Function(...)` themselves or test `arguments` beyond `length`, and cases that also need the other parts of slice 2; the counts are a lower bound for the engine after this story.

`test/harness`: the 23 cases moved to slice 3 in slice 1, wave 5, because of `Function` and `arguments`, are placed again. 22 move to slice 2: the 14 `propertyhelper-*` cases without a symbol, `verifyProperty-configurable-object`, `-restore`, `-same-value`, `-string-prop`, `-value`, `-value-error` (which pass with the stand-in above), `verifyProperty-restore-accessor` (its only refusal is the method `get() { return 42; }` at line 12, which `story:syntax-language-core` takes into slice 2) and `fnGlobalObject` (`Function("return this;")()`). 1 stays in slice 3: `asyncHelpers-asyncTest-without-async-flag.js` needs `Object.hasOwn` and `globalThis` of slice 3 besides its method shorthand (line 10, `then(res, rej) {`), so the placement of slice 1 is right for it.

Host requirements, all `[slice 2]` in `docs/design/host-interface.md` on this branch and `pending slice 2` in `conformance/host/requirements.txt`: `HOST-116`, `HOST-117`, `HOST-118`, `HOST-139`, `HOST-140`, `HOST-141`, `HOST-152`, `HOST-162`, `HOST-163`, `HOST-164`, all ten closed here. The epic, acceptance item 5, says a too deeply nested `eval` source throws a `RangeError`; `HOST-139` to `HOST-141`, decision 9, item 9 and section 3 of `docs/design/engine.md` say a `SyntaxError` whose message contains `parser_nesting`; this story follows them.

Refusals counted (`docs/design/engine.md` decision 11, "Refusals"; `host-interface.md` section 13, "A count of refusals", with `Agent::refusals()`): a refusal reached through `eval`, `Function(...)`, `$262.evalScript` or at run time is never a pass, also when the script catches the `SyntaxError`. The engine counts, per agent, every `SyntaxError` it raises itself because a construct belongs to a later slice (the messages `swell_syntax::message_kind` classifies as `Refusal`): raised while parsing, through whatever entry (`parse_script`, `eval`, `Function`), or while running (today the refusal a regular expression literal throws from compiled code, `ThrowSyntaxError` of `crates/swell/src/interp/dispatch.rs`, emitted by `crates/compiler/src/expression.rs` with `REGEXP_REFUSAL`), caught or not; the runner fails every run in which that count moved, with a cause of its own, and counts it in `test262-refused`. The nesting limit is not a refusal for this count: it is a limit of the host, and `HOST-139` and `HOST-140` ask that the script can catch it. The runner keeps treating it as today: a case or harness file whose own parse reaches it fails the run (`crates/test262/src/run.rs`), `$262.evalScript` marks the run for it (`story:test262-run-causes`), and through `eval` or `Function` the `SyntaxError` reaches the script, which answers for the run; no run of the pinned checkout reaches the limit on the base (`awk -F'\t' '$1 == "test262-run" && $6 ~ /parser_nesting/'` over the run lines of the throwaway runner prints 0 lines).

Measured before the code, by a scan (`.out/decompose-slice-02/awk/dynscan.awk` of the coordinator worktree, over every run case of the pinned checkout): on each line holding `eval(`, `Function(` or `$262.evalScript(`, the quoted string literals from the call to the end of the line, tested for a construct the parser refuses today (`let`, `const`, `class`, `=>`, `...`, `function*` or `yield`, `async` or `await`, `super`, `import`, `new.target`, a template, a BigInt literal, `**`, `for ... of`, a destructuring declaration). It finds 224 cases (173 through `eval`, 43 through `Function`, 8 through `evalScript`): 9 in slice 2 (6 through `evalScript`, all under `test/language/global-code/`, and 3 through `eval`, `test/language/statementList/eval-block-let-declaration.js`, `eval-block-with-statment-let-declaration.js`, `eval-fn-let-declaration.js`, all `let`), 1 in slice 3, 28 in 6, 6 in 7, 106 in 8, 23 in 10, 5 in 11, 1 in 15, 3 in 18, 42 in 19. 4 of them pass on the base, for the wrong reason: `eval` is not defined, so the `ReferenceError` they expect is thrown by the call itself (`test/language/eval-code/direct/lex-env-no-init-let.js`, `-const.js`, `-cls.js` in slices 6 and 8, `super-prop-expr-no-home-no-eval.js` in slice 8); with `eval` they fail on the counted refusal. What stays unknown: a source built at run time (a variable, a concatenation over lines, `String.fromCharCode`), a construct the scan does not test for (a computed property name, a default parameter, a regular expression literal, which is refused at evaluation and not at parse), a string in a comment that the scan takes for an argument, and whether a case would pass but for the refusal. The count sees every one of them at run time.

Design it builds on, on the branch since merge `63dff4d` (preconditions that hold before wave 3; the acceptance checks the first one again):

- Items 2 and 3 of "What each slice builds" (section 12): item 2 gives the global `Function` with its constructor to slice 2, item 3 no longer lists `Function` among what slice 3 builds (its directory count `Function` 417 stays: the cases of `test/built-ins/Function/` stay in slice 3). This story does not write them.
- Section 3, the rule "Names" (line 280 on), for direct `eval`: a reference is looked up by name when it is in eval code, or when between it and its compile-time binding scope lies a function or script scope whose own code is non-strict and contains a direct `eval` call (a call in a nested function does not count for the outer scope; a function nested inside such a scope is inside it) (item 1); a strict function keeps its slots, also when it calls `eval` itself, and a strict function nested in a non-strict scope with a direct `eval` looks up by name the names that cross that scope (item 2); every binding of every scope that encloses a direct `eval` call, strict or not, up to the global environment, lives in an Environment Record that keeps the names of its slots (item 3); a function whose code contains a direct `eval` gets its arguments object whenever FunctionDeclarationInstantiation creates one (item 4); a binding that eval code adds at run time is an allocation charged as any other (item 5).
- Section 3 and decision 9, item 9: the nesting limit through `eval` and `Function` is a `SyntaxError` naming `parser_nesting`, never a `RangeError`.
- `HOST-162` (two return sites: the `Some` of `host_get_code_for_eval` and the thrown value of an `Abrupt::Throw` from `host_ensure_can_compile_strings`), `HOST-163` (`Function(p, b)` at the deepest re-entry, `p` and `b` each nested `parser_nesting` levels, and "What `min_stack_bytes()` covers"), `HOST-164` and section 13, "A count of refusals" (`Agent::refusals()`), and `HOST-117` and `HOST-141` as `[slice 2]`.

The rule lines of the `swell-slices` block for `test/harness` and the counts of section 12 are this story's, in the same change, as the decision asks (wave 3 has one unit).

Code it builds on, checked on this branch: the hooks exist with their defaults (`crates/swell/src/host/hooks.rs`, `CompileStrings`, `CompilationType`) and are called nowhere; `parse_script_in`, `make_script_record`, `atom_and_key` and the mapping of `CompileError` are private to `crates/swell/src/script.rs`, which this story opens to the crate for the one engine entry rather than copy; a function object keeps its code alive through a script record (the trace of `object/ecmascript_function.rs`: environment, realm, script, home object), so eval code and a dynamic function get a script record of their own.

## Change

Failing tests first: the tests of Requirements, the parser tests, the language scripts and the tests below are written before the code and fail (`eval` and `Function` are not defined; the parse entries do not exist).

`swell-syntax` (after `story:syntax-language-core` and `story:with-arguments-and-dynamic-names`):

- `src/scope.rs`, `src/ast.rs`: per function and for the script, whether its code contains a direct call of `eval` (a call whose callee, parentheses removed, is the IdentifierReference `eval`: the syntactic condition of `sec-function-calls-runtime-semantics-evaluation`, step 6), kept in `ast::Scope` (`crates/syntax/src/ast.rs:334`, where scope analysis stores what it finds per function and for the script, inferred from the code) and so exported on the tree for the compiler.
- `src/lib.rs`, with the goal-symbol entries it needs in `src/parser/mod.rs` and `src/parser/function.rs`: two entries beside `parse_script`, through the same parser, the same depth counter and the same `ParseOptions::max_nesting`. `parse_eval(source, strict_caller, options)`: `ParseText(x, Script)` of PerformEval (`sec-performeval`): strict when the caller is strict or the source has a `"use strict"` directive, returning the tree, its VarDeclaredNames and its function declarations to hoist, and whether it is strict; the checks of PerformEval on `new.target`, `super` and `arguments` in class field initializers need syntax of slices 8 and 6 that the parser refuses anyway, each stated in the doc comment with its slice. `parse_dynamic_function(parameters, body, options)`: the three `ParseText` of CreateDynamicFunction (`sec-createdynamicfunction`) for kind `normal`: the parameter text as `FormalParameters[~Yield, ~Await]`, the body text (a line feed before and after) as `FunctionBody[~Yield, ~Await]`, then the whole source text `function anonymous(<P>\n) {\n<body>\n}` as a `FunctionExpression`, with its early errors (a `"use strict"` body with non-simple or duplicate parameters, a parameter list that closes early, such as `a) {`); it returns a tree whose only function is that expression. The other kinds (generator, async) are refused naming slices 7 and 10.
- `ParseOptions::stack_bytes()` stays the bound of a parse at `max_nesting` for both entries; `every_nesting_kind_fits_the_stack_bound` runs each nesting kind through `parse_eval` and `parse_dynamic_function` as well, and the constants change only if that test shows a need, measured as in slice 1, wave 3.

Engine and compiler:

- `crates/swell/src/dynamic_code.rs` (new), declared in `crates/swell/src/lib.rs`: the one entry for source text compiled at run time. It calls `host_ensure_can_compile_strings` with the seven parameters before it parses (a `Throw` is thrown, no code of the string runs), parses through `swell_syntax::parse_eval` or `parse_dynamic_function` with `max_nesting` the agent `parser_nesting`, answers a parse error as a `SyntaxError` of the current realm (the nesting error names `parser_nesting`), compiles, charges the code and its atoms to the memory budget as `parse_script` does, and keeps the code in a script record of its own. On it: PerformEval (`sec-performeval`, with HostGetCodeForEval for an object, `eval(o)` returning `o` on no-code) and EvalDeclarationInstantiation (`sec-evaldeclarationinstantiation`: the global checks CanDeclareGlobalVar and CanDeclareGlobalFunction, `var` and function bindings of non-strict eval code created deletable in the variable environment of the caller, strict eval code in an environment of its own); CreateDynamicFunction (`sec-createdynamicfunction`, kind `normal`: ToString of each argument, the parameter text joined with ",", the source text, GetPrototypeFromConstructor with `%Function.prototype%`, OrdinaryFunctionCreate in the global environment of the realm, SetFunctionName "anonymous", MakeConstructor). Every answer of a hook that holds a handle is checked before use: stale, foreign or of the wrong kind ends the run with `Terminated(EngineFault)` (decision 6 of `host-interface.md`).
- `crates/swell/src/script.rs`: `parse_script_in`, `make_script_record`, `atom_and_key` and the `CompileError` mapping open to the crate and shared, nothing else changes; `crates/swell/src/host/mod.rs`: the context a hook gets when the engine calls it from inside a run (`Cx` for HostEnsureCanCompileStrings, `HookCx` for HostGetCodeForEval) and the check of what it answers; `crates/swell/src/host/limits.rs`: the constants of `min_stack_bytes` change only if `HOST-152` shows a need, measured without incremental compilation and without a compiler cache.
- `crates/swell/src/builtins/global.rs`: `eval` (`sec-eval-x`, an indirect eval); `crates/swell/src/builtins/function.rs`: the `Function` constructor (`sec-function-p1-p2-pn-body`), `Function.length` 1 and `Function.prototype` (`sec-properties-of-the-function-constructor`); `crates/swell/src/realm/intrinsics.rs`: `%eval%`, `%Function%` and `Function.prototype.constructor`; `crates/swell/src/realm/global_object.rs`: the global properties `eval` and `Function`; `crates/swell/src/object/builtin_function.rs`: the dispatch of the two.
- Direct eval. `crates/compiler/src/bytecode.rs`, `src/disasm.rs`: an instruction for a call that scope analysis marked as a direct `eval`: EvaluateCall, except that when the callee is `%eval%` of the current realm (SameValue) it performs PerformEval of the first argument (or `undefined`) with the strictness of the caller and `direct` (`sec-function-calls-runtime-semantics-evaluation`, step 6). `src/expression.rs` emits it. `src/resolve.rs`: the rule "Names" of section 3 for direct `eval`, items 1 to 4 as stated above, on the named records and lookups by name of `story:with-arguments-and-dynamic-names`: a reference in eval code, or with a non-strict function or script scope that contains a direct `eval` between it and its binding scope, is looked up by name; a strict function keeps its slots even when it calls `eval`; every binding of a scope that encloses a direct `eval` call, strict or not, gets a named slot; the arguments object of a function that contains a direct `eval` is made whenever FunctionDeclarationInstantiation creates one (steps 15 to 22), not only when `arguments` appears; the compiler source tests show which form each reference takes. `src/lib.rs`: `compile_eval`, eval code compiled with its free names resolved by name, taking the VarDeclaredNames and the function declarations from the answer of `parse_eval` for EvalDeclarationInstantiation (no list of its own), and its completion value.
- `crates/swell/src/interp/dispatch.rs`, `src/interp/mod.rs`, `src/interp/frame.rs`: the direct-eval call and the frame of eval code (the `this`, LexicalEnvironment and VariableEnvironment of the caller for a direct eval, of the global environment for an indirect one), counted against `frame_depth` like any frame and against `reentry_depth` where it is entered from Rust; `crates/swell/src/env/{mod,declarative,function,global}.rs`: a binding added at run time to a named record (a `var` of non-strict eval code in a function), CreateGlobalVarBinding and CreateGlobalFunctionBinding with `D` true; `crates/swell/src/object/ecmascript_function.rs`: the arguments object of a function with a direct `eval`, and OrdinaryFunctionCreate from the template of a dynamic function.
- `crates/swell/src/host/agent.rs`: the count of refusals of the agent and `Agent::refusals()`, as section 13, "A count of refusals", states it: a `u64` that starts at 0, is never reset, saturates instead of wrapping, and is read in every state of the agent, `Faulted` included (no `Result`, a read that runs no code). It is raised in two places: `crates/swell/src/script.rs`, the mapping of parse errors to `SyntaxError` objects, which every parse entry shares (`parse_script_in`, and through `dynamic_code.rs` `eval` and `Function`), adds one for each error of a parse that `message_kind` reads as a refusal; `crates/swell/src/interp/dispatch.rs`, `ThrowSyntaxError`, adds one when the message it throws is a refusal (the regular expression literal, whose message `story:syntax-language-core` makes a refusal of slice 5).
- Runner (`swell-test262`, after `story:test262-run-causes`): `crates/test262/src/run.rs` reads the count after the realms and the harness are set up and again when the run ends; a run in which it moved is failed whatever the case and `negative` answered, with the detail `counted refusal: <n> refusal(s)` (after the detail of `evalScript` when that marked the run too), and `Ran::refused` answers true for it, so that `count.rs` counts it in `test262-refused` unchanged. `$262.evalScript` keeps its own mark (`story:test262-run-causes` says why).

`docs/design/engine.md`, section 12 (this wave has one unit, and the decision asks for the same change): the 23 rule lines `dir test/harness/<file> 3` become 22 lines with slice 2 and 1 with slice 3 (`asyncHelpers-asyncTest-without-async-flag.js`); the paragraph "86 of the 117 files" and its list of groups follow; the table and the block after "Its output on this document" read `02 5267 9217` and `03 7581 14994` (22 cases, 44 runs from slice 3 to slice 2; every `test/harness` case has two runs), the other 20 lines unchanged, the 22 still summing to 53,801 cases and 103,360 runs.

`conformance/test262/sections.txt`: `section sec-performeval`, `path sec-performeval test/language/eval-code/`, `omit sec-performeval parser-nesting-limit decision-3` (347 cases), and `section sec-createdynamicfunction`, `path sec-createdynamicfunction test/built-ins/Function/`, `omit sec-createdynamicfunction parser-nesting-limit decision-3` (509 cases; `grep -c "^<path>" .cache/inventory/files.txt`), as `sec-parse-script` records the same limit; `conformance/test262/expected.txt`: the lines of the run, 286 lines.

Each limit through each entry (the condition of the decision), besides the tests of Requirements below, each in both collection modes (`test-collect`):

- `parser_nesting`: through `eval` by `HOST-139` and `HOST-140`, through `Function` by `HOST-141`, and in `crates/syntax/tests/sources.rs` `eval_and_function_entries_share_the_nesting_bound` (at `max_nesting` 1,000, `0` inside 2,000 nested parentheses is one error naming `parser_nesting` through `parse_eval` and as the body of `parse_dynamic_function`; `0` inside 10 parses).
- `reentry_depth`: `eval.rs` `eval_past_reentry_depth_throws_a_range_error` (with `reentry_depth` 16, a getter that evaluates `(0, eval)("o.x")` on its own object answers `Throw` of a `RangeError` whose message is `reentry_depth exceeded`, caught by the script) and `dynamic_function.rs` `function_past_reentry_depth_throws_a_range_error` (with `reentry_depth` 16, an object whose `toString` calls `Function(o, "")` on itself, ToString of a parameter being a call from Rust into code).
- The stack bound: `HOST-152` for `eval`, `HOST-163` for `Function`.
- `frame_depth`, the step and the memory budget: `eval.rs` `step_budget_ends_an_endless_eval`, `memory_budget_ends_an_eval_whose_code_does_not_fit`, `eval_recursion_ends_in_a_range_error` (`function f() { eval("f()"); } f()` answers `Throw` of a `RangeError`), and in `dynamic_function.rs` `step_budget_ends_an_endless_dynamic_function`, `memory_budget_ends_a_dynamic_function_whose_code_does_not_fit`, `dynamic_function_recursion_ends_in_a_range_error`.

Refusals through code compiled at run time: `HOST-164` (Requirements), and in `crates/test262/tests/run.rs` over a new fixture tree `dynamic-refusal` (`crates/test262/tests/fixtures/dynamic-refusal/`: `features.txt`, `harness/assert.js`, `harness/sta.js`, `test/language/dynamic-refusal/caught-in-eval.js` with `try { eval("let x;"); } catch (e) { if (!(e instanceof SyntaxError)) throw e; }`, `caught-in-function.js` with the same around `Function("let x;")`, `regexp-literal-caught.js` with `try { /a/; } catch (e) { if (!(e instanceof SyntaxError)) throw e; }`, the refusal at run time, `grammar-error-caught-in-eval.js` around `eval("var;")`, which passes) the tests `a_refusal_caught_inside_eval_fails_the_run`, `a_refusal_caught_inside_function_fails_the_run`, `a_regular_expression_literal_refusal_caught_at_run_time_fails_the_run`, `a_grammar_error_caught_inside_eval_passes` and `counted_refusals_are_counted_refused` (the `test262-refused` line of the tree reads 6: three cases, two runs each).

Tests of an earlier wave whose expectation changes: `crates/test262/tests/run.rs` `a_refusal_inside_eval_script_fails_the_run` (wave 1, `story:test262-run-causes`). Before this story the run line of `refusal-fails-the-run.js` reads `Failed` with the detail `evalScript: refused: SyntaxError: let declaration is not supported before slice 6 (1:1)` and the tree prints `test262-refused 2`; after it, the same `$262.evalScript("let x;")` also moves the count, so the detail reads `evalScript: refused: SyntaxError: let declaration is not supported before slice 6 (1:1); counted refusal: 1 refusal(s)`, the outcome stays `Failed` and the tree still prints `test262-refused 2` (a run is counted refused once). The test is changed to assert the new detail.

Other tests: `dynamic_function.rs` `dynamic_function_scope_is_the_global_environment`; `crates/syntax/tests/sources.rs` `eval_entry_is_strict_after_a_strict_caller` (`with (o) {}` parses through `parse_eval` with a non-strict caller and is a SyntaxError with a strict one), `eval_entry_returns_var_names_and_hoisted_functions`, `dynamic_function_entry_parses_parameters_and_body_apart` (`("a, b", "return a + b")` parses; `("a) {", "}")` is a SyntaxError), `direct_eval_is_recorded_per_function` (`eval(x)` and `(eval)(x)` mark their function, `o.eval(x)` and `(0, eval)(x)` do not), and `every_nesting_kind_fits_the_stack_bound` extended as above; `crates/compiler/tests/sources/` `direct-eval-call.js`, `function-with-direct-eval.js`, `eval-code-var.js` with their `.disasm`; language scripts (new files in `crates/swell/tests/language/`) `eval-direct-reads-local.js`, `eval-direct-var-in-function.js` (sloppy-only), `eval-strict-var-stays-inside.js`, `eval-indirect-is-global.js`, `eval-completion-value.js`, `eval-non-string-is-returned.js`, `eval-this-of-caller.js`, `eval-renamed-is-indirect.js`, `eval-syntax-error-is-catchable.js`, `eval-global-function-not-definable-throws.js`, `eval-inside-with-reads-object.js` (sloppy-only), `function-constructor-parameters-and-body.js`, `function-constructor-is-sloppy-unless-body-says.js`, `function-constructor-name-anonymous.js`, `function-constructor-injection-is-syntax-error.js`.

Writes: `crates/syntax/src/{lib,scope,ast}.rs`, `crates/syntax/src/parser/{mod,function}.rs`, `crates/syntax/tests/sources.rs`, `crates/swell/src/dynamic_code.rs` (new), `crates/swell/src/{lib,script}.rs`, `crates/swell/src/host/{agent,mod,limits}.rs`, `crates/swell/src/builtins/{global,function}.rs`, `crates/swell/src/realm/{intrinsics,global_object}.rs`, `crates/swell/src/object/{builtin_function,ecmascript_function}.rs`, `crates/swell/src/interp/{dispatch,mod,frame}.rs`, `crates/swell/src/env/{mod,declarative,function,global}.rs`, `crates/compiler/src/{bytecode,disasm,expression,resolve,lib}.rs`, `crates/compiler/tests/compile.rs`, `crates/compiler/tests/sources/` (the six new files), `crates/swell/tests/{eval,eval_stack,dynamic_function}.rs` (new), `crates/swell/tests/language/` (the fifteen new files listed, by name), `crates/test262/src/run.rs`, `crates/test262/tests/run.rs`, `crates/test262/tests/fixtures/dynamic-refusal/` (the seven files listed), `conformance/host/requirements.txt` (the ten lines), `conformance/test262/sections.txt`, `conformance/test262/expected.txt`, `docs/design/engine.md` (the `swell-slices` block, its table and the paragraph of `test/harness` groups in section 12).

## Requirements

Lines of `conformance/host/requirements.txt` moved from `pending slice 2` to `test`, each test asserting what its requirement states:

- `crates/swell/tests/eval.rs` (new): `HOST-116` `swell::eval::host_116_eval_asks_the_host_before_parsing` (one call, `body_string` `"1"`, `DirectEval` and `IndirectEval` as called, before parsing; a `Throw` from the hook is thrown by `eval` and no code of the string runs), `HOST-118` `swell::eval::host_118_eval_of_an_object_asks_host_get_code_for_eval` (`None` makes `eval(o)` return `o`, `Some(s)` evaluates `s`), `HOST-139` `swell::eval::host_139_direct_eval_past_parser_nesting_throws_a_catchable_syntax_error`, `HOST-140` `swell::eval::host_140_indirect_eval_past_parser_nesting_throws_a_syntax_error`, `HOST-162` `swell::eval::host_162_a_stale_or_foreign_handle_from_either_hook_faults_the_agent` (both return sites, each with a stale handle and with a handle of another agent: a `Some` of `host_get_code_for_eval` holding a stale or a foreign String handle, called by `eval(o)`, and an `Abrupt::Throw` of `host_ensure_can_compile_strings` holding one, called by `eval` and by `Function`; each ends the run with `Terminated(EngineFault)`, no code of the string runs, `state()` then reads `Faulted`, every entry point and read but `state()` and `refusals()` answers `Err(HostError::Faulted)`, and nothing panics), `HOST-164` `swell::eval::host_164_refusals_counts_every_refusal_caught_or_not` (with `r` = `let x;`: `refusals()` reads 0 on a new agent, 1 more after `parse_script` of `r`, 1 more after `try { eval(r); } catch (e) {}` answers `Normal`, 1 more after `try { Function(r); } catch (e) {}` answers `Normal`, 1 more after `try { /a/; } catch (e) {}` answers `Normal`; the same after `parse_script` of a grammar error and of `0` inside 2,000 nested parentheses with `parser_nesting` 1,000; read as a `u64` after the agent faulted).
- `crates/swell/tests/eval_stack.rs` (new), each in a child process on a thread of `min_stack_bytes()` bytes, as `HOST-17` is tested: `HOST-152` `swell::eval_stack::host_152_deepest_reentry_evals_at_full_nesting` (at `reentry_depth` re-entries, `eval` of a source nested `parser_nesting` levels ends without a stack overflow) and `HOST-163` `swell::eval_stack::host_163_deepest_reentry_calls_function_with_parameters_and_body_at_full_nesting` (at `reentry_depth` re-entries, `Function(p, b)` with a parameter text `p` and a body text `b` each nested `parser_nesting` levels ends without a stack overflow, in `Normal` or in `Throw` of a `SyntaxError`).
- `crates/swell/tests/dynamic_function.rs` (new): `HOST-117` `swell::dynamic_function::host_117_function_asks_the_host_with_its_parts` (`Function("a", "return a")`: `param_strings` `["a"]`, `body_string` `"return a"`, `compilation_type` `Function`), `HOST-141` `swell::dynamic_function::host_141_function_past_parser_nesting_throws_a_catchable_syntax_error` (`Function("return " + s)`).

## Acceptance

`cargo test --locked -p swell-syntax -p swell-compiler -p swell -p swell-test262` and `slot task test-collect` exit 0 and run the tests of Requirements and the tests named above; before the first change, `slot task test262 TEST262_OUT=.out/test262-base` in the unit tree (the base of the wave) writes `.out/test262-base/runs.tsv`, and `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` prints the marker count `<m>` of the base (51 on this branch); after the change the same task exits 0 and prints `host-requirements: 164 requirement(s), 164 line(s): 80 test, 84 pending, 0 omitted; <m> UNMAPPED marker(s), each naming a HOST-<n> or ESS limit`, with the same `<m>` (this story writes nothing under `spec/`), and `host-requirements: pending by slice: 3:3 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1` (on this branch: 164 lines, 70 test and 94 pending, `2:10` among them; the ten lines of slice 2 move to `test`); as a precondition that still holds, `awk '/^What each slice builds/ { f = 1 } f && /^[0-9]+\. / { n = $1 } f && n == "2."' docs/design/engine.md | grep -c 'global `Function`'` prints `1` and `awk '/^What each slice builds/ { f = 1 } f && /^[0-9]+\. / { n = $1 } f && n == "3."' docs/design/engine.md | grep -c '`Object`, `Function`'` prints `0`; `slot task test262` exits 0 with the unit lines in `conformance/test262/expected.txt` (286 lines), prints `test262: 22 slice lines equal the design`, `test262-slice 02 5267 9217 0 0 5267 <p2> <f2> 0` with `<p2>` at least the passed count of the slice 2 line on `main` plus 36 (21 of the 22 cases of `test/harness`, all but `verifyProperty-restore-accessor`, and the 15 of slice 2 measured above), `test262-slice 03 7581 14994 0 0 7581 <p3> <f3> 0` with `<p3>` at least 2,027 (1,569 on the base of the slice, plus the 458 cases of slice 3 measured above outside `test/harness`) and `test262-slice 04 1198 2396 0 0 1198 <p4> <f4> 0` with `<p4>` at least 17; after it, with the default `TEST262_OUT` of that run: `awk -F'\t' 'NR == FNR { if ($6 ~ /(^|,)propertyHelper\.js(,|$)/) h[$1] = 1; next } ($3 in h) { s[$3] = $2; if ($5 != "Passed") bad[$3] = 1 } END { for (p in s) { t[s[p]]++; if (!(p in bad)) ok[s[p]]++ } for (k in t) print k, t[k], ok[k] + 0 }' .cache/inventory/meta.tsv .out/test262/runs.tsv | sort` prints `02 77 <n2>`, `03 1162 <n3>` and `04 304 <n4>` with `<n2>` at least 35, `<n3>` at least 458 and `<n4>` at least 4 (the count of cases that pass once the helper loads, per slice, which the decision asks for); `awk -F'\t' '$6 ~ /(eval|Function) is not defined/' .out/test262/runs.tsv | wc -l` prints `0`; `awk -F'\t' '$5 == "Passed" && $6 ~ /counted refusal:/' .out/test262/runs.tsv | wc -l` prints `0` (no run passes with a moved count) and `awk -F'\t' '$3 ~ /^test\/language\/eval-code\/direct\/lex-env-no-init-(let|const|cls)\.js$/ && $6 ~ /counted refusal:/ { print $3 }' .out/test262/runs.tsv | sort -u | wc -l` prints `3` (three of the cases that pass on the base for the wrong reason); `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 FS $4] = 1; next } (($3 FS $4) in p) && $5 != "Passed" && $6 !~ /counted refusal:/' .out/test262-base/runs.tsv .out/test262/runs.tsv | wc -l` prints `0` (no run that passed on the base fails now, except on a counted refusal; this per-run check takes the place of the comparison of the lines of `expected.txt`, which those runs would trip in slices 6 and 8); `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 FS $4] = 1; next } (($3 FS $4) in p) && $5 != "Passed" && $6 ~ /counted refusal:/ { print $2, $3, $4 }' .out/test262-base/runs.tsv .out/test262/runs.tsv | sort` prints the runs that passed on the base only because a refusal went unseen (the three `lex-env-no-init-*` cases among them), one per line, which the unit quotes whole in its report and the coordinator lists in the record of wave 3 (the epic, acceptance item 2: each such run is listed in the wave record that removes it); and the 22 count lines of the design (fields 2 and 3 after "Its output on this document") sum to 53801 and 103360.

## Out of scope

`Function.prototype.toString`, `apply` and the other methods of slice 3, HostHasSourceTextAvailable (`HOST-120`, slice 3); the generator and async function constructors (slices 7 and 10); the checks of PerformEval on `new.target`, `super` and class field initializers (slice 8); every other rule line of the block (`story:slice-02-conformance-close`); `AGENTS.md`, `README.md`, the root files and the epic (the coordinator).

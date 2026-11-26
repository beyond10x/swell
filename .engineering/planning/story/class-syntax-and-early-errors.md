---
format: aep.planning-md/3
id: story:class-syntax-and-early-errors
kind: story
status: draft
title: The grammar of a class and its early errors, parsed and held
summary: 467 negative cases (917 runs); parser/class.rs, two nesting kinds, the holds class and class heritage.
relations:
- decomposes: epic:slice-08-classes
scope:
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/compiler/src/statement.rs
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/lib.rs
- confidence: inferred
  path: crates/syntax/src/parser/class.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: cited
  path: crates/syntax/src/parser/statement.rs
- confidence: cited
  path: crates/syntax/src/scope.rs
- confidence: cited
  path: crates/syntax/tests/sources.rs
- confidence: cited
  path: crates/syntax/tests/sources/refused/class-declaration.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/class-expression.js
- confidence: inferred
  path: crates/syntax/tests/sources/refused/class-field.js
- confidence: inferred
  path: crates/syntax/tests/sources/refused/class-heritage.js
- confidence: inferred
  path: crates/syntax/tests/sources/refused/class-static-block.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/class-syntax.js
revision: 3
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story is the grammar of a class (`#sec-class-definitions`: ClassDeclaration, ClassExpression, ClassTail, ClassHeritage, ClassBody with MethodDefinition elements, `static`, computed keys, empty elements) and its early errors (`#sec-class-definitions-static-semantics-early-errors`: one `constructor`, a `constructor` that is not a special method, no `static prototype`, the strict mode of class code `#sec-strict-mode-code`, the class name's binding and its redeclaration in a block, a switch and a script, `#sec-block-static-semantics-early-errors`), parsed to a tree and held: the parse of a class goes on to its end, its early errors are answered, and a source with a class and no error answers `class is not supported before slice 8` (held, `Parser::held`). It runs nothing. It needs no other story of slice 8: order 1, built beside `story:new-target-meta-property` and `story:super-property-and-super-syntax`.

The parts of a class body that other stories bring are refused at their token: a field (`class field is not supported before slice 8`), a static block (`class static block is not supported before slice 8`; `story:class-element-syntax-and-early-errors` makes both holds); a private name keeps the lexer's refusal (`crates/syntax/src/lexer.rs` line 373). `extends` holds `class heritage is not supported before slice 8` after its expression is parsed (`story:derived-classes-and-super-call` lifts it). `super` in a class method follows the rules of the method head of object literals that `story:super-property-and-super-syntax` sets (a class method is parsed through the same head), and a SuperCall is an early error there until `story:derived-classes-and-super-call`; the negative cases of `super` in a class are on that story's list.

On the base (`62c7cf59`; the unit reads every place named here again on its own base): `crates/syntax/src/parser/statement.rs` line 222, `Keyword::Class => return self.refuse("class", 8)`; `crates/syntax/src/parser/expression.rs`, `primary_kind`, the arm `Keyword::Class` near line 1023; `Parser::held` exists and no construct holds one (`parser/mod.rs` lines 93 to 98); the fixtures `crates/syntax/tests/sources/refused/class-declaration.js` and `class-expression.js` hold `class is not supported before slice 8`; `crates/syntax/tests/sources.rs`, `NESTING_KINDS: [&str; 61]`. The refusal `class` matches section 12, item 8 (the entry "Each slice" of `docs/plan/waves.md`, "What later slices take over", checked).

The list is `docs/plan/slice-08/story-class-syntax.cases.txt`: the 467 cases of the key `class-syntax` of `docs/plan/slice-08/scan.tsv`, all negative parse cases with a class and no field, static block, private name or `super`. By directory: `language/{statements,expressions}/class/dstr` 144 (invalid patterns in method parameters), `async-gen-method(-static)` 80, `gen-method(-static)` 56, `async-method(-static)` 48, `method(-static)` 24, `definition` 14, `elements` 28, class files 22, `language/block-scope/syntax` 19, `language/statements/switch` 15, and redeclarations elsewhere. Their 917 runs fail on the plan's base run, all on `class is not supported before slice 8` (a refusal never counts as the SyntaxError a negative case asks for); with this story each answers its early error before the held refusal. The base run of this plan is the full run of 2026-11-26 on `149e12f1`, whose total and slice lines equal `conformance/test262/expected.txt` on `62c7cf59` (slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`).

Design it builds on: the held refusal of `Parser::held` (the parse goes on and answers the refusal only when it ends without a grammar error or early error) and section 7 of `docs/design/engine.md` (the parser's stack bound, one nesting kind per recursion of the grammar). Design change 1 of this decomposition (the section "Classes", written by a design author before the units start) states the class tree the compiler reads; the unit reads the text that landed and follows it where it differs.

## What later stories reuse

`crates/syntax/src/parser/class.rs` (new) and the tree of a class (`Stmt::Class`, `Expr::Class`, the `Class` record: name, heritage, the constructor, the elements with their kind, `static`, key and function); `story:class-element-syntax-and-early-errors` adds field, static-block and private elements to it, `story:class-definition-evaluation` compiles it and lifts the hold `class`, `story:derived-classes-and-super-call` lifts `class heritage`. The helper `Parser::hold` (also added by `story:super-property-and-super-syntax`, same signature).

## Domain relations

- A class element → the class whose body holds it (one); a class → at most one constructor element — inferable (inferred from ECMA-262 `#sec-class-definitions` and its early errors; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the fixtures and tests below are written before the code and fail on the base.

- `crates/syntax/src/parser/class.rs` (new; `parser/mod.rs` its `mod` line): a class declaration where a declaration stands (not as the body of `if`, a loop or a label: the grammar error), a class expression as a primary expression (`statement.rs` line 222 and the arm of `primary_kind` call it); the heritage as a LeftHandSideExpression; the body's elements: methods, getters, setters, generator, async and async generator methods, each optionally `static`, with a literal, string, number or computed key, parsed through the method head of `parser/expression.rs` (`method_head`, `property_key`, `accessor_kind`) with the class body's context (strict; `yield` and `await` as the code around gives them to computed keys); `;` elements. A field (a key not followed by `(`) and `static {` refuse at their token as Context says. The class body is strict code from `class` to its `}`, the heritage too.
- Early errors (`crates/syntax/src/early.rs`): two `constructor` methods; a `constructor` that is a getter, setter, generator, async or async generator; a static method named `prototype` (a literal key; a computed one is a TypeError at run time, `story:class-definition-evaluation`); the class name `let`, `static`, `yield`, `await` (in async or module code), `implements` and the other strict reserved words, `eval` and `arguments`; the strict rules on the parameters and bodies of methods (duplicate parameters, `with`, octal literals, `delete` of a name); the class name in the lexical declarations of a block, a switch and a script (`crates/syntax/src/scope.rs`: a class declaration binds its name as `let` does, with the redeclaration rules of `#sec-block-static-semantics-early-errors`, `#sec-switch-statement-static-semantics-early-errors`, `#sec-scripts-static-semantics-early-errors`; the Annex B function-in-block rules as they stand).
- The holds: `fn hold(&mut self, construct: &str, slice: u32, span: Span)` beside `refuse_at` in `parser/mod.rs` keeps the first held refusal and lets the parse go on; the class holds `class` at its first token and `class heritage` at `extends`. The documentation of `held` (`parser/mod.rs` lines 93 to 98) and of `parse_script` in `crates/syntax/src/lib.rs` (the sentence on held refusals) names the holds and the story that lifts each.
- `crates/syntax/src/ast.rs`: `Stmt::Class`, `Expr::Class` and the `Class` record, and every exhaustive match over `Stmt` and `Expr` in `swell-syntax` and `swell-compiler` gets their arms. A compiler arm for a class answers `CompileError::Tree`: no source reaches it while `class` is held (a test of `crates/compiler` states that every source with a class is refused by the parse), and `story:class-definition-evaluation` replaces it.
- Stack bound: a class nests in its computed keys and its heritage (`class extends (class extends ... {}) {}`, `class { [class { [ ... ] () {} }] () {} }`); two kinds join `NESTING_KINDS` (`[&str; 63]`) with their sources in the generator of `crates/syntax/tests/sources.rs`, each through every entry as the others are (`fits_the_stack_bound`), and the parts of the stack-bound test take them by index (`stack_bound_part`, no part edited).
- Fixtures: `crates/syntax/tests/sources/early-errors/class-*.js` (new, one per rule above, each with its message), `valid/class-syntax.js` (new: every element kind, static, computed keys, `;`, a class in a computed key, parsed with its hold: the fixture states the held refusal), `refused/class-heritage.js`, `refused/class-field.js`, `refused/class-static-block.js` (new); `refused/class-declaration.js` and `class-expression.js` stay (the hold has the same text).
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode and under 2.5 at `SWELL_FULL_COUNTS=1`, measured as `AGENTS.md` says (one job of the pool, the test binary run with `--exact <name> --test-threads 1`, user plus system); the two new nesting kinds keep each part of the stack-bound test under its bound, measured. A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate (`slot task check`) and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures or waits runs without the prefix. No Python in any tool, test or command (a scratch helper that is never committed excepted); intermediate files in the scratch directory or `.out/`.
- Size: about 800 lines (parser 300, early errors and scope 200, tree and arms 80, fixtures and the nesting kinds 220): one session.

## Requirements

- None of `conformance/host/requirements.txt` moves: `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1` before and after.
- Rules every story carries. No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing and `slot task forbid-unsafe-check` exits 0. Decision 13: no panic on any input (no `unwrap`, no `expect`, no unchecked indexing; arithmetic that can exceed its type checked or explicitly wrapping); the parser charges the bytes of the tree it builds to the ledger as every other node does (`crates/syntax/src/ledger.rs`), and the `frontend_ledger::` tests pass. The run loop is not touched (nothing runs).

Writes: `crates/syntax/src/{ast,early,scope,lib}.rs` (the class's variants; its rules; the class binding; the documentation of `parse_script`), `crates/syntax/src/parser/{mod,class,statement,expression}.rs` (`hold`, the documentation of `held`, the `mod` line; `class.rs` new; line 222; the arm `Keyword::Class` of `primary_kind`), `crates/syntax/tests/sources.rs` (`NESTING_KINDS` and the generator of its sources), `crates/syntax/tests/sources/{early-errors,valid,refused}/class-*.js`; `crates/compiler/src/*.rs` (the arms of the exhaustive matches only). A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/lexer.rs`, `crates/syntax/src/parser/function.rs`, `crates/swell`, `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared with the stories of order 1, built at the same time, each in hunks of its own: `crates/syntax/src/parser/expression.rs` (`story:new-target-meta-property`, `new_rest`; `story:super-property-and-super-syntax`, the arm `Keyword::Super` and `method_head`, which this story calls and does not edit); `Parser::hold` (`story:super-property-and-super-syntax` adds the same helper with the same signature: the second to merge keeps one) and the documentation of `held` (the second keeps both sentences); `crates/syntax/src/ast.rs` and every exhaustive match over `Expr` and `Stmt` (each story its own variants and arms); `crates/syntax/src/early.rs` (each its own rules); `crates/syntax/src/lib.rs` (`story:new-target-meta-property`, `parse_eval` and `parse_eval_with` and their documentation; this story, the documentation of `parse_script`). Later stories extend `parser/class.rs` (`story:class-element-syntax-and-early-errors`, the elements; `story:class-definition-evaluation`, `story:derived-classes-and-super-call` and the element stories, one hold line each).

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-syntax --test main` and the same with `SWELL_FULL_COUNTS=1`; `cargo nextest run --locked -p swell-compiler --test main`; `cargo nextest run --locked -p swell --test main -E 'test(/^scripts::/) | test(/^eval::/) | test(/^dynamic_function::/) | test(/^frontend_ledger::/) | test(/^language::/)'` (a class in a script, in eval and in `Function` is refused as before, by the hold); `cargo nextest run --locked -p swell-test262 --test main`; each exits 0. `slot task clippy` exits 0; `sh taskfiles/test-main.sh check` exits 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing; `slot task forbid-unsafe-check` exits 0; `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1`. The text: `grep -n 'refuse("class", 8)' crates/syntax/src/parser/statement.rs` prints nothing (line 222 on the base), and `grep -n "No construct holds one" crates/syntax/src/parser/mod.rs` prints nothing (line 94 on the base). Time: every test this story adds stays under its bound as the unit measured it, and each part of the stack-bound test stays under 1.5 CPU seconds; the report lists the times and the sum, and a time over its bound fails this acceptance.

test262. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-class-syntax.cases.txt" > .out/story-class-syntax.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-syntax.cases.txt` (the 2,469 passing negative parse cases of `test/language/`), `cat .out/story-class-syntax.cases.txt docs/plan/slice-08/guard-syntax.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 2,936 cases, 5,467 runs over the plan's base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-class-syntax.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1 and prints 917 lines `failing` and `passed 2469 bound 2469 B 2469 F 467 R 467`); (b) `awk -F'\t' '$6 ~ /class is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0` (917 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

Running a class (`story:class-definition-evaluation`); fields, private names, static blocks and their early errors (`story:class-element-syntax-and-early-errors`); `super` in a class and the heritage's run (`story:derived-classes-and-super-call`); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).

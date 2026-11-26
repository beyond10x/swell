---
format: aep.planning-md/3
id: story:class-element-syntax-and-early-errors
kind: story
status: draft
title: 'Fields, private names, static blocks, .#x and #x in: grammar and early errors, held'
summary: 886 cases (1,756 runs); lifts the lexer's private-name refusal and holds one refusal per element kind.
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:class-syntax-and-early-errors
- depends_on: story:new-target-meta-property
- depends_on: story:super-property-and-super-syntax
scope:
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/statement.rs
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/lexer.rs
- confidence: inferred
  path: crates/syntax/src/parser/class.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: cited
  path: crates/syntax/src/scope.rs
- confidence: cited
  path: crates/syntax/tests/sources.rs
- confidence: inferred
  path: crates/syntax/tests/sources/early-errors/private-name-outside-class.js
- confidence: inferred
  path: crates/syntax/tests/sources/refused/class-field.js
- confidence: inferred
  path: crates/syntax/tests/sources/refused/class-static-block.js
- confidence: inferred
  path: crates/syntax/tests/sources/refused/private-field.js
- confidence: inferred
  path: crates/syntax/tests/sources/refused/private-method.js
- confidence: inferred
  path: crates/syntax/tests/sources/refused/private-name-in.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/private-name.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/class-elements.js
revision: 3
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story is the grammar and early errors of the class elements that `story:class-syntax-and-early-errors` refuses: fields (FieldDefinition, public and private, static or not, with or without an initializer, and the automatic semicolon rules around them, `#sec-rules-of-automatic-semicolon-insertion`), private methods and accessors (`#m() {}`, `get #x`, `static async *#m`), static blocks (ClassStaticBlock), the private name token (PrivateIdentifier, `#sec-names-and-keywords`, with escapes), the private member forms `a.#x` and `a?.#x`, and `#x in a` (`#sec-relational-operators`); with the early errors of `#sec-class-definitions-static-semantics-early-errors` (a private name declared twice except a getter and setter pair of the same placement, `#constructor`, a field named `constructor`, a static field named `prototype` or `constructor`, a field initializer or static block that Contains `arguments` (ContainsArguments) or a SuperCall, AllPrivateIdentifiersValid in a class body, a script, a function body and `Function` code), `delete` of a private reference (`#sec-delete-operator-static-semantics-early-errors`), the static block's own rules (`await` as an identifier, `return`, its var scope, `#sec-class-definitions-static-semantics-early-errors` ClassStaticBlockStatementList), and a private name in an object literal. Parsed and held: each element kind holds its refusal to the end of the parse. Order 2, after `story:class-syntax-and-early-errors` (its class tree), `story:new-target-meta-property` (the flag that admits `new.target`, set in field initializers and static blocks) and `story:super-property-and-super-syntax` (the flags of SuperProperty and SuperCall: a field initializer and a static block admit `super.x` and refuse `super()`), built beside `story:class-definition-evaluation`.

The holds this story makes, each at the first token of its first element: `class field is not supported before slice 8` (a public field; `story:public-class-fields` lifts it), `private method is not supported before slice 8` (a private method or accessor; `story:private-names-and-private-methods`), `private field is not supported before slice 8` and `private name in is not supported before slice 8` (`story:private-fields-and-private-in`), `class static block is not supported before slice 8` (`story:class-static-blocks`). The immediate refusals of `story:class-syntax-and-early-errors` for a field and a static block become these holds; the lexer's refusal `private name is not supported before slice 8` (`crates/syntax/src/lexer.rs` line 373 on the base) is lifted: a private name outside a class is an early error, inside one a hold of its element kind. The refusal `private name` matches section 12, item 8 (the entry "Each slice" of `docs/plan/waves.md`, "What later slices take over", checked); the fixture `crates/syntax/tests/sources/refused/private-name.js` (`x = #a;`) becomes an early-error fixture.

The list is `docs/plan/slice-08/story-element-syntax.cases.txt`: the 886 cases of the key `element-syntax` of `docs/plan/slice-08/scan.tsv` (the negative parse cases with a field, a static block, a private name or a private member, and 5 cases that give `eval` or `Function` a private name outside a class; 881 negative). By directory: `language/{statements,expressions}/class/elements` 670 (`syntax/early-errors`, `syntax/valid` negatives, the ASI cases), `language/{statements,expressions}/class/dstr` 144 (invalid patterns in private methods), `language/expressions/object/method-definition` 12, the early errors of `#x` in functions and blocks 12, and others. Their 1,756 runs fail on the plan's base run: 1,696 on `class is not supported before slice 8`, 50 on `private name is not supported before slice 8`, 10 as counted refusals. The base run of this plan is the full run of 2026-11-26 on `149e12f1`, whose total and slice lines equal `conformance/test262/expected.txt` on `62c7cf59` (slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`).

Design it builds on: section 7 of `docs/design/engine.md` (the parser, its ledger and stack bound) and the holds of `Parser::held`. Design change 2 of this decomposition (a section "Private names", written by a design author before the units start) says how a private name resolves (in the parser: AllPrivateIdentifiersValid; in the compiler, `story:private-names-and-private-methods`); the unit reads the text that landed and follows it where it differs.

## What later stories reuse

The element kinds of the `Class` record (field, private method or accessor, static block, private keys), `Expr::PrivateMember`, the private link of an optional chain, `Expr::PrivateIn`, the private name token; the holds, one line each, that the element stories lift; the context of a field initializer and a static block (the flags of `new.target`, SuperProperty, SuperCall, `arguments`, `await`), which `story:public-class-fields` and `story:class-static-blocks` compile.

## Domain relations

- A private name reference → the private name declared by the nearest enclosing class body that declares it (one, or the early error) — inferable (inferred from ECMA-262 `#sec-static-semantics-allprivateidentifiersvalid`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the fixtures and tests below are written before the code and fail on the base.

- `crates/syntax/src/lexer.rs`: `#` followed by an identifier start (an escape included) is the private name token; `#` otherwise is the grammar error it is; line 373 goes.
- `crates/syntax/src/parser/class.rs`: the element parse of fields (a key followed by `=`, `;`, `}` or a line break where ASI applies; `static`, `get`, `set` and `async` as field names, with the line-break rules of the cases `fields-asi-*` and `grammar-field-*`), private methods and accessors, static blocks (a block in its own context: a new var scope, `await` reserved as an identifier, no `return`, no `arguments`, SuperProperty and `new.target` admitted, SuperCall not; `crates/syntax/src/scope.rs` the scope of the block); the field initializer in the context of a method of the class (`new.target` and SuperProperty admitted, SuperCall and `arguments` not, `yield` and `await` as identifiers by the rules of the class body); each hold of Context. `crates/syntax/src/parser/expression.rs`: `member_name` (`.#x`), `optional_link` (`?.#x`), and `#x` as the left operand of `in` (only directly before `in`, at the precedence of a relational expression; `#x in #y`, `(#x) in y` and `#x` alone are errors).
- `crates/syntax/src/early.rs`: the rules of Context, each with its message; AllPrivateIdentifiersValid over the private names declared by the enclosing class bodies (a list per class body, the getter and setter pair rule), applied to a script, a function body, `Function` code and a class body; eval code applies it with no enclosing names until `story:private-names-in-eval-code` gives the private environment.
- `crates/syntax/src/ast.rs`: the element kinds, `Expr::PrivateMember`, `Expr::PrivateIn`, the private link, and every exhaustive match over them in `swell-syntax` and `swell-compiler`; a compiler arm for them answers `CompileError::Tree` (no source reaches the compiler while its kind is held; a test of `crates/compiler` states it), and each element story replaces the arms of its kind.
- Stack bound: a class nests in a field initializer and in a static block (`class { x = class { x = ... } }`, `class { static { class { static { ... } } } }`); two kinds join `NESTING_KINDS` (`[&str; 65]` after `story:class-syntax-and-early-errors`), with their sources in the generator of `crates/syntax/tests/sources.rs`.
- Fixtures: `crates/syntax/tests/sources/early-errors/{class-field,private-name,static-block}-*.js` (new, one per rule, each with its message), `refused/private-name.js` moved to `early-errors/private-name-outside-class.js`, `refused/{class-field,private-method,private-field,private-name-in,class-static-block}.js` (each with its hold; `refused/class-field.js` and `refused/class-static-block.js` of `story:class-syntax-and-early-errors` take the new text), `valid/class-elements.js` (every element kind and form above, held).
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode and under 2.5 at `SWELL_FULL_COUNTS=1`, measured as `AGENTS.md` says (one job of the pool, the test binary run with `--exact <name> --test-threads 1`, user plus system); the two new nesting kinds keep each part of the stack-bound test under its bound, measured. A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate (`slot task check`) and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures or waits runs without the prefix. No Python in any tool, test or command (a scratch helper that is never committed excepted); intermediate files in the scratch directory or `.out/`.
- Size: about 900 lines (lexer 40, element parse and ASI 250, member and `in` forms 60, early errors 250, tree and arms 60, fixtures and nesting kinds 240): one session.

## Requirements

- None of `conformance/host/requirements.txt` moves: `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1` before and after.
- Rules every story carries. No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing and `slot task forbid-unsafe-check` exits 0. Decision 13: no panic on any input (no `unwrap`, no `expect`, no unchecked indexing; arithmetic that can exceed its type checked or explicitly wrapping); the lists of private names the early errors keep are charged to the ledger (`crates/syntax/src/ledger.rs`) and bounded by the source's size, and the `frontend_ledger::` tests pass. The run loop is not touched.

Writes: `crates/syntax/src/{lexer,ast,early,scope}.rs`, `crates/syntax/src/parser/{class,expression,mod}.rs` (the element parse and holds; `member_name`, `optional_link`, the `in` operand; the documentation of `held`), `crates/syntax/tests/sources.rs` (`NESTING_KINDS` and its generator), `crates/syntax/tests/sources/{early-errors,refused,valid}/*` as listed; `crates/compiler/src/*.rs` (the arms of the exhaustive matches only). A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/parser/{function,statement}.rs`, `crates/syntax/src/lib.rs`, `crates/swell`, `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared with `story:class-definition-evaluation`, built at the same time, each in hunks of its own: `crates/syntax/src/parser/class.rs` (that story, the hold line `class`; this story, the element parse and its holds) and the documentation of `held` in `parser/mod.rs` (each its own sentence); `crates/syntax/src/ast.rs` and the compiler's arms over the `Class` record (that story compiles the kinds of its base and answers `CompileError::Tree` for others; the second to merge adds the arms of the new kinds as `CompileError::Tree`). Later stories lift one hold line each in `parser/class.rs` (`story:public-class-fields`, `story:class-static-blocks`, `story:private-names-and-private-methods`, `story:private-fields-and-private-in`).

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-syntax --test main` and the same with `SWELL_FULL_COUNTS=1`; `cargo nextest run --locked -p swell-compiler --test main`; `cargo nextest run --locked -p swell --test main -E 'test(/^scripts::/) | test(/^eval::/) | test(/^dynamic_function::/) | test(/^frontend_ledger::/) | test(/^language::/)'`; `cargo nextest run --locked -p swell-test262 --test main`; each exits 0. `slot task clippy` exits 0; `sh taskfiles/test-main.sh check` exits 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing; `slot task forbid-unsafe-check` exits 0; `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1`. The text: `grep -rn '"private name", 8\|private name is not supported' crates conformance docs/design website/docs` prints nothing (on the base it prints `crates/syntax/src/lexer.rs:373` and the fixture's header). Time: every test this story adds stays under its bound as the unit measured it, and each part of the stack-bound test stays under 1.5 CPU seconds; the report lists the times and the sum, and a time over its bound fails this acceptance.

test262. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-element-syntax.cases.txt" > .out/story-element-syntax.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-syntax.cases.txt` (the 2,469 passing negative parse cases of `test/language/`), `cat .out/story-element-syntax.cases.txt docs/plan/slice-08/guard-syntax.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 3,355 cases, 6,306 runs over the plan's base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-element-syntax.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1 and prints 1,756 lines `failing` and `passed 2469 bound 2469 B 2469 F 886 R 886`); (b) `awk -F'\t' '$6 ~ /(class|private name) is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0` (1,746 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

Running fields, private methods, private fields, static blocks and `#x in` (their stories); private names in eval code (`story:private-names-in-eval-code`); decorators and `nonextensible-applies-to-private` (excluded features, `conformance/test262/features.txt`); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).

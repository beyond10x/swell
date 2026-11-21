---
format: aep.planning-md/3
id: story:bytecode-compiler
kind: story
status: active
title: swell-compiler turns the parsed tree into stack bytecode with marked backward jumps
summary: Bytecode format, slot and environment resolution, statements, expressions and functions of the slice 1 grammar, try/finally completions, a disassembler; tested by golden disassembly.
relations:
- decomposes: epic:slice-01-harness
- depends_on: story:syntax-es5-and-strict
- depends_on: story:num-and-unicode-early-parts
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/compiler
- confidence: cited
  path: tests/compile.rs
- confidence: cited
  path: tests/sources
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T04:49:59Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T04:49:59Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
---
## Context

Decision 3 of `docs/design/engine.md`: a stack-based bytecode interpreter with every ECMAScript
frame on the engine own frame stack. The compile step turns the tree `swell-syntax` accepted
(`story:syntax-es5-and-strict`) into that bytecode, in its own crate so it can be tested and
fuzzed without the heap (decision 2). The step budget is charged one unit per instruction and
checked at every backward jump, call, return and re-entry (decision 3), so the format marks every
backward jump. Scope analysis and the compile step walk only trees the parser accepted, so the
parser bound holds for them (decision 3, "Depth bounds").

Numeric literal values come from `swell-num` (`story:num-and-unicode-early-parts`,
`sec-numericvalue`), so the tree keeps the literal text and this crate computes the Number.

With no `eval`, `with` or `arguments` in slice 1 (refused by the parser until slice 2), every
binding of a function is known at compile time: a binding no inner function captures lives in a
local slot of the frame, a captured one in the function environment record, and a name not
declared in any enclosing function is resolved at run time through the global environment.
Slice 2 revisits this when direct `eval` and `with` arrive.

## Change

Failing tests first: `crates/compiler/tests/compile.rs` and its source and expected files are
written before the code and fail (the crate does not exist).

Crate layout, stated once here: `crates/compiler/` (package `swell-compiler`, dependencies
`swell-syntax` and `swell-num` by path, `#![forbid(unsafe_code)]`, the four clippy denials):

- `src/lib.rs`: `pub fn compile_script(script: &swell_syntax::Script) -> Result<Code,
  CompileError>`; `CompileError` only for a bound of the format itself (more than 2^32
  instructions, constants or slots in one function), never for valid input below those bounds.
- `src/bytecode.rs`: the instruction set, the operand encoding, `Code` (the script template:
  instructions, constants as UTF-16 code units and Numbers, function templates, handler table,
  source positions per instruction, the names `GlobalDeclarationInstantiation` needs: var names
  and the function declarations of the script, `sec-globaldeclarationinstantiation`), and
  `FunctionTemplate` (parameters, local slot count, captured bindings, strictness, name, length,
  its own instructions and handler table). Every jump whose target is not after it is a distinct
  `LoopJump` instruction.
- `src/resolve.rs`: slot and environment assignment per function, from the scope analysis of
  `swell-syntax`.
- `src/statement.rs` (14: block, `var`, empty, expression, `if`, `do-while`, `while`, `for`,
  `for-in` with `var` or a left-hand side, `continue`, `break`, `return`, `switch`, `throw`,
  `try`/`catch`/`finally`, `debugger` as no operation), `src/expression.rs` (13: every ES5
  expression the parser accepts, with the evaluation order of each runtime semantics clause:
  reference then value for assignments, `sec-evaluatestringornumericbinaryexpression`,
  short-circuit for `&&`, `||` and `?:`, `delete` of a property reference and of other
  expressions, `typeof` of an unresolvable reference), `src/function.rs` (15.2 function
  declarations and expressions, the named function expression binding).
- `src/disasm.rs`: a text form of `Code`, one instruction per line, for the tests.

A regular expression literal compiles to one instruction that throws a `SyntaxError` with the
message `regular expression literals are not supported before slice 5`, as the design states
it ("A part before its slice": "lexed to its end and evaluating it throws a `SyntaxError`"); the
interpreter creates the error in the current realm. This is the defined refusal the design
records as an omission, not a stand-in: no value is produced.

`try`/`finally` is compiled with completion records on the operand stack, so a `return`, `break`
or `continue` out of a `try` runs the `finally` block (`sec-try-statement`); the handler table
says which ranges a `catch` or `finally` covers, and the interpreter never enters one for a
termination (`story:interpreter-and-script-entry-points`).

Tests:

- `crates/compiler/tests/sources/*.js` with `*.disasm` beside each: one per statement and
  expression form, a regular expression literal among them, each citing its clause in a first
  comment line.
- `crates/compiler/tests/compile.rs`: `disassembly_matches_for_each_source` (fails when
  `tests/sources/` holds no `.js` file, or a `.js` file has no `.disasm`: `AGENTS.md`, "A check
  that finds nothing to read fails"), `numeric_literal_values` (`0x10`, `010` in non-strict
  code, `1e400`, `.5`), `every_backward_jump_is_a_loop_jump` (over every source, failing on none:
  no plain jump targets an earlier instruction),
  `uncaptured_bindings_are_slots_and_captured_ones_are_environment`,
  `finally_runs_for_return_break_and_continue` (the handler table and completion handling of a
  source with all three), `compile_is_deterministic` (each source compiled twice gives equal
  `Code`), `deepest_accepted_tree_compiles_on_a_256_kib_stack` (a source at nesting 1,000 on a
  spawned thread with a 256 KiB stack), `regexp_literal_compiles_to_its_syntax_error_throw`.

`Cargo.lock`: the unit regenerates it in its own tree (one package added), because its build
needs it, and runs its acceptance with `--locked` against that lock; the coordinator does not
take the unit lock into the wave branch but regenerates it there after the merges of the wave.
Gate task: none new.

Writes: `crates/compiler/` (new: `Cargo.toml`, `src/lib.rs`, `src/bytecode.rs`, `src/resolve.rs`,
`src/statement.rs`, `src/expression.rs`, `src/function.rs`, `src/disasm.rs`, `tests/compile.rs`,
`tests/sources/`), `Cargo.lock` (regenerated in the unit tree; the wave branch lock is
regenerated by the coordinator).

## Acceptance

`cargo test --locked -p swell-compiler` exits 0 and runs the eight tests named in Change over
every source under `crates/compiler/tests/sources/`.

## Out of scope

Running the bytecode, the budgets and the frame layout at run time, interning the constants into
the heap and charging bytecode to the memory budget (`story:interpreter-and-script-entry-points`);
every construct the parser refuses; any optimisation. The root `Cargo.toml`, `Taskfile.yml`
(the coordinator).

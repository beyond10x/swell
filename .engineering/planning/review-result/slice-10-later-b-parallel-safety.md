---
format: aep.planning-md/3
id: review-result:slice-10-later-b-parallel-safety
kind: review-result
status: active
title: 'Slice 10, later stories, set B, the parallel-safety critic: the five stories of orders 6 and 7'
relations:
- reviews: story:yield-star-in-async-generators
- reviews: story:async-generator-methods-in-object-literals
- reviews: story:async-generator-function-constructor
- reviews: story:array-from-async
- reviews: story:slice-10-conformance-close
revision: 1
---
needs-revision
array-from-async — its list holds `built-ins/Array/fromAsync/async-iterable-input-does-not-await-input.js`, which declares `async next()` in an object literal (an async method), and no `depends_on` edge runs to `story:async-methods-in-object-literals`; add the edge, or take the case off the list — .engineering/planning/story/array-from-async.md:39, .engineering/planning/story/async-methods-in-object-literals.md:80
array-from-async — it writes `crates/swell/src/object/builtin_function.rs` and `crates/swell/tests/builtins_symbol_values.rs` (cited, both bodies) with `story:promise-statics-finally-and-aggregate-error` and `story:async-function-constructor`; no path of edges joins them and the line names only `async-generator-function-constructor` as a sharer; name both with their hunks (apart: own `BuiltinId` variants, own symbol-sweep rows), or add an edge — .engineering/planning/story/array-from-async.md:68, .engineering/planning/story/promise-statics-finally-and-aggregate-error.md:78
async-generator-function-constructor — it writes `crates/swell/src/object/builtin_function.rs` and `crates/swell/src/realm/intrinsics.rs` (cited, both bodies) with `story:promise-statics-finally-and-aggregate-error`, no path of edges joins them, and the line names only `array-from-async` for `builtin_function.rs` and nobody for `intrinsics.rs`; name the story with its hunks (apart: own variants, own intrinsic), or add an edge — .engineering/planning/story/async-generator-function-constructor.md:69, .engineering/planning/story/promise-statics-finally-and-aggregate-error.md:78
yield-star-in-async-generators — it shares `crates/syntax/src/parser/expression.rs`, `crates/syntax/tests/sources.rs` and `crates/compiler/src/expression.rs` (the last "if written") with `story:async-methods-in-object-literals`, and `crates/syntax/src/lib.rs` and `crates/syntax/src/parser/mod.rs` with `story:async-function-constructor`; no path of edges joins either pair and the line names neither; name them with their hunks (apart: the function of YieldExpression and `admitted` here, `accessor_kind` and `NESTING_KINDS` there, `parse_script` and `held` here, the dynamic entry there), or add edges — .engineering/planning/story/yield-star-in-async-generators.md:84, .engineering/planning/story/async-methods-in-object-literals.md:80, .engineering/planning/story/async-function-constructor.md:70
slice-10-conformance-close — Change says `expected.txt` is written "with the digest of its sources as the gate asks", but `conformance/test262/expected.digest`, which `task test262-expected` writes beside it and `test262-check` compares, is on neither its `Writes:` line nor its "For the coordinator" list; add it to one of them — .engineering/planning/story/slice-10-conformance-close.md:59, .engineering/planning/story/slice-10-conformance-close.md:63

**What I read:**
- Artifacts: the 5 stories of set B in full, plus the Writes lines and scopes of the 4 first-wave stories, the 8 stories of set A and the open stories of slices 5 and 7, and the prior parallel-safety results of slices 7 and 10. I also read the epic's graph and `aep plan artifact waves`.
- Code: `lib.rs` and the parser in `crates/syntax`; the `KindTag` and `Intrinsic` match sites; `builtins/array/mod.rs` at lines 110 and 597; `tests/sources.rs`; `taskfiles/test-main.sh`; `taskfiles/test262.yml`.
- Lists: the case lists of set B, checked for tokens of other stories against the pinned test262.
- Pairs: every pair of order 6, and every pair across sets with no path of edges between them.
- The Writes checks that held: the instruction match (`bytecode.rs`, `disasm.rs`, `dispatch.rs`), the `ObjectKind` and `KindTag` sites (`object/mod.rs`, `heap/collect.rs`), `ArrayBuiltin` (`builtins/array/mod.rs`), `Intrinsic` (`realm/intrinsics.rs`), and `KIND_SITES` 16 → 20 → 21 across the four stories that add a kind.

**Surfaces and edges:**
- Placed: 5 stories cited, 0 inferred, 0 unplaced. Only new fixture files and the new test modules are inferred, and none is shared.
- Acceptance reads traced to a producer in the set: 27. Recorded by a `depends_on` path: 26 (a few only transitively, through `async-generators-and-their-request-queue`). The 27th is the `async next()` case in `array-from-async`, finding 1.
- Pairs inside order 6:
  - `yield-star-in-async-generators` and `async-generator-methods-in-object-literals` share `parser/expression.rs` and `tests/sources.rs`, in hunks apart and named.
  - `yield-star-in-async-generators` and `async-generator-function-constructor` share `lib.rs` and `parser/mod.rs`, in hunks apart and named.
  - `async-generator-function-constructor` and `array-from-async` share `builtin_function.rs`, in hunks apart and named.
  - `async-generator-methods-in-object-literals` and `array-from-async` share the generated `tests/main.rs`, which is named.
  - The other pairs share nothing. `yield-star-in-async-generators` is the only story of its order that adds an instruction.
- The `waves` output prints no collision that involves set B.

**What I could not establish:**
- Whether `async-generator-methods-in-object-literals` needs a change in `crates/compiler/src/expression.rs`. Its line leaves that file out, and `async-methods-in-object-literals` names it "if written". If it is written, it falls in a hunk apart from the `yield*` arm.
- Out of my lane (acceptance): in `async-generator-function-constructor` the search ``grep -rn '`async generator` (slice 10)' crates/syntax/src crates/swell/src`` prints nothing on the base, so the claim that it prints that line (`crates/syntax/src/lib.rs`, line 214) is wrong. The phrase is split across lines 215 and 216 of `crates/syntax/src/lib.rs` (a line break before "(slice 10)"), so the "after" check is also vacuous. Citation: .engineering/planning/story/async-generator-function-constructor.md:36.
- All findings are warnings. None is a blocker, so a second reading is not needed.

```findings
- file: .engineering/planning/story/array-from-async.md
  line: 39
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its list holds built-ins/Array/fromAsync/async-iterable-input-does-not-await-input.js, which declares `async next()` in an object literal (an async method), and no depends_on edge runs to story:async-methods-in-object-literals; add the edge, or take the case off the list"
- file: .engineering/planning/story/array-from-async.md
  line: 68
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it writes crates/swell/src/object/builtin_function.rs and crates/swell/tests/builtins_symbol_values.rs (cited, both bodies) with story:promise-statics-finally-and-aggregate-error and story:async-function-constructor, no path of edges joins them, and the line names only async-generator-function-constructor as a sharer; name both with their hunks or add an edge"
- file: .engineering/planning/story/async-generator-function-constructor.md
  line: 69
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it writes crates/swell/src/object/builtin_function.rs and crates/swell/src/realm/intrinsics.rs (cited, both bodies) with story:promise-statics-finally-and-aggregate-error, no path of edges joins them, and the line names only array-from-async for builtin_function.rs and nobody for intrinsics.rs; name the story with its hunks or add an edge"
- file: .engineering/planning/story/yield-star-in-async-generators.md
  line: 84
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it shares parser/expression.rs, tests/sources.rs and compiler/src/expression.rs (if written) with story:async-methods-in-object-literals, and syntax lib.rs and parser/mod.rs with story:async-function-constructor; no path of edges joins either pair and the line names neither; name them with their hunks (apart) or add edges"
- file: .engineering/planning/story/slice-10-conformance-close.md
  line: 63
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Change writes expected.txt with the digest of its sources, but conformance/test262/expected.digest, which task test262-expected writes beside it and test262-check compares, is on neither the Writes line nor the list for the coordinator; add it to one of them"
```

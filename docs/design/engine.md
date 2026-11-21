# Design: swell, an ECMAScript engine

First design document of swell, written on 2026-11-21, before any code. It settles fourteen
questions as numbered decisions; each gives the choice, the reason, and what was rejected. What
could not be settled from sources is under "Open, for the operator". Every number has its source
beside it; a statement that could not be checked says "not checked". The block `swell-slices`
(section 12) is read by a program; nothing else in this document is.

swell is its own repository. Its first user is surf, the browser built from scratch in the
sibling repository `beyond10x/surf`, and its requirements come from surf's needs.

The texts read, all fetched on 2026-11-21:

- ECMA-262, `https://tc39.es/ecma262/`: "ECMAScript® 2027 Language Specification", "Draft
  ECMA-262 / November 19, 2026"; `tc39/ecma262` `main` at
  `74c33d406ec1763ca9ee5ad9165571661649d6e6` (2026-11-19T21:53:31Z); the page's SHA-256
  `61b78fd25c24792c85834f40e0c4e570af725e37498f938e4e7f6feb7946382f`.
- test262, `https://github.com/tc39/test262`: commit
  `2a1ade303f72d3de80565c921d685222405c32ce`, 2026-11-20T10:52:32-05:00, "[immutable-arraybuffer]
  TypedArray internal operations (#4551)"; `package.json` version `5.0.0`.
- TC39 finished proposals, `tc39/proposals` `finished-proposals.md`: `main` on the day.
- WHATWG HTML Standard, `multipage/webappapis.html`, `nav-history-apis.html`,
  `common-dom-interfaces.html`: "Last Updated 20 November 2026".
- WHATWG Web IDL, `https://webidl.spec.whatwg.org/`: "Last Updated 14 November 2026".
- Unicode, `https://www.unicode.org/versions/latest/`: names Unicode 18.0.0 as the latest.
- Rust standard library source of rustc 1.99.1 (the installed toolchain's `rust-src`):
  `core/src/fmt/float.rs`, `core/src/num/imp/dec2flt/mod.rs`, `std/src/num/f64.rs`.
- surf: `AGENTS.md`, `README.md`, `docs/design/html-parser.md`, `docs/plan/agent-waves.md`,
  `spec/`. ESS: `README.md` of `beyond10x/ess`.

Clause numbers below (9.5, 16.2 and so on) are those of the draft above and change when the
standard does; the anchors (`#sec-...`) are stable.

## What the brief assumed and the sources say otherwise

- **Temporal is stage 4 but not in the draft.** `finished-proposals.md` lists Temporal with the
  meeting note `2026-03/march-11.md#temporal-for-stage-4` and "Expected Publication Year" 2027,
  and test262 `features.txt` lists `Temporal` under "Standard language features". The draft of
  2026-11-19 has no Temporal clause: its clauses are 1 to 29 (29 "Memory Model") and the annexes,
  and the word "Temporal" occurs in it only in the grammar names `TemporalDecimalFraction` and
  `TemporalDecimalSeparator` (`grep -o 'Temporal[^<]*' ecma262.html | sort | uniq -c`). The HTML
  Standard already defines `HostSystemUTCEpochNanoseconds(global)` (8.1.6.5), a hook the draft
  does not have. Decision 1 follows from this.
- **test262's feature sections lag the stage list.** `features.txt` still lists
  `Iterator.prototype.join` under "Proposed language features"; `finished-proposals.md` lists
  "Iterator Join" as finished (meetings up to 2026-09, expected 2027). The section of
  `features.txt` is therefore not an exact list of what is stage 4; decision 1 records every
  feature by name instead.
- **`Number::toString` takes a radix and asks for the shortest digits in every radix.** 6.1.6.1.20
  `Number::toString ( x, radix )`: "Let n, k, and s be integers such that k ≥ 1, radix^(k-1) ≤ s <
  radix^k, 𝔽(s × radix^(n-k)) is x, and k is as small as possible", for every radix. The Rust
  standard library gives shortest digits in radix 10 only (decision 6).
- **The Unicode version.** ECMA-262 refers to "the latest version of the Unicode Standard", which
  is 18.0.0; test262's generated tests are written for 17.0.0 (all 469 files under
  `test/built-ins/RegExp/property-escapes/generated/` say `Unicode v17.0.0`). Open item C.
- **"50,000-odd cases" is 53,801 test files and 103,360 runs** (section "Inventory").
- **The host hooks changed shape.** `HostEnsureCanCompileStrings` now takes seven parameters
  (`calleeRealm, paramStrings, bodyString, codeString, compilationType, paramArgs, bodyArg`) and
  `HostGetCodeForEval ( arg )` is new, from "Dynamic Code Brand Checks" (finished, expected
  2027). `HostLoadImportedModule ( referrer, moduleRequest, hostDefined, payload )` is answered by
  `FinishLoadingImportedModule ( referrer, moduleRequest, payload, result )`.
- **`print` is rarely called by tests themselves.** One test file calls `print(` directly; 5,460
  use `$DONE`, which `harness/doneprintHandle.js` defines on top of `print`.

## Inventory of test262

Measured on the clone in the session scratch directory:

```sh
git clone --depth 1 https://github.com/tc39/test262 test262
git -C test262 log -1 --format='%H %cI %s'
# 2a1ade303f72d3de80565c921d685222405c32ce 2026-11-20T10:52:32-05:00 [immutable-arraybuffer] ...
cd test262
find test -type f -name '*.js' | wc -l                         # 54095
find test -type f -name '*.js' ! -name '*_FIXTURE.js' | wc -l  # 53801
find test -name '*_FIXTURE.js' | wc -l                         # 294
find test -name '*.json' | wc -l                               # 13
find test -name '*.js' ! -name '*_FIXTURE.js' > ../files.txt
tr '\n' '\0' < ../files.txt | xargs -0 awk -f ../meta.awk > ../meta.tsv   # 53801 lines
du -sh . test harness .git          # 275M, 236M, 424K, 26M
find . -type f -not -path './.git/*' | wc -l                    # 57252
```

A case is one test file that is not a `_FIXTURE` file (INTERPRETING.md, "Modules": such files
"MUST NOT be interpreted as standalone tests"). A run is one execution of a case: twice (strict
and non-strict) unless the case has `onlyStrict`, `noStrict`, `module` or `raw`
(INTERPRETING.md, "Strict Mode").

By top directory (`awk -F/ '{print $2}' files.txt | sort | uniq -c`):

| directory | cases |
|---|---|
| `test/built-ins/` | 23,885 |
| `test/language/` | 23,832 |
| `test/intl402/` | 3,394 |
| `test/staging/` | 1,487 |
| `test/annexB/` | 1,086 |
| `test/harness/` | 117 |
| total | 53,801 |

The largest second-level directories (`awk -F/ '{print $2"/"$3}' files.txt | sort | uniq -c |
sort -rn`): `language/expressions` 11,156; `language/statements` 9,399; `built-ins/Temporal`
4,616; `built-ins/Object` 3,411; `built-ins/Array` 3,083; `intl402/Temporal` 2,040;
`built-ins/RegExp` 1,879; `built-ins/TypedArray` 1,453; `staging/sm` 1,406; `built-ins/String`
1,223. Inside the first two, `statements/class` has 4,419 and `expressions/class` 4,109.

Front matter, read by `meta.awk` (given at the end of this document) into one line per case:

| flag | cases | | negative phase and type | cases |
|---|---|---|---|---|
| `generated` | 17,107 | | `parse SyntaxError` | 4,680 |
| `async` | 5,624 | | `resolution SyntaxError` | 34 |
| `noStrict` | 2,689 | | `runtime` (all types) | 40 |
| `module` | 843 | | | |
| `onlyStrict` | 680 | | | |
| `raw` | 32 | | | |
| `CanBlockIsTrue` / `CanBlockIsFalse` | 7 / 2 | | | |

`grep -l -E '^flags:.*\bmodule\b'` finds 832 `module` cases; the other 11 write `flags:` as a
block list (for example `test/staging/sm/async-functions/async-contains-unicode-escape-module.js`).
The 40 runtime negatives are 14 `Test262Error`, 14 `ReferenceError`, 4 `TypeError`, 4
`SyntaxError`, 3 `EvalError`, 1 `RangeError`. 18,673 cases have no `features:` line. Runs:
2 × 49,559 + 4,242 = 103,360 (`awk -F'\t' '{r += ($2 ~ /onlyStrict|noStrict|module|raw/) ? 1 : 2}
END {print r}' meta.tsv`).

Features (198 names used by tests; `features.txt` declares 199, `export-defer` is used by none).
The ones that decide a slice or an exclusion, as cases carrying the feature:

| feature | cases | | feature | cases |
|---|---|---|---|---|
| `Temporal` | 6,738 | | `SharedArrayBuffer` | 467 |
| `async-iteration` | 4,971 | | `Atomics` | 386 |
| `class` | 4,916 | | `Atomics.waitAsync` | 101 |
| `generators` | 4,129 | | `WeakRef` | 37 |
| `TypedArray` | 2,537 | | `FinalizationRegistry` | 49 |
| `Intl.Era-monthcode` (stage 3) | 1,566 | | `host-gc-required` | 15 |
| `BigInt` | 1,506 | | `tail-call-optimization` | 35 |
| `dynamic-import` | 1,011 | | `IsHTMLDDA` | 42 |
| `async-functions` | 705 | | `cross-realm` | 203 |
| `regexp-unicode-property-escapes` | 681 | | `ShadowRealm` (stage 3) | 64 |

Unions: `SharedArrayBuffer`, `Atomics`, `Atomics.waitAsync` or `Atomics.pause`: 615 cases. Under
`test/intl402/` or with a feature starting `Intl`: 3,404. Either of those or `Temporal`: 8,021.

What the harness must provide, from INTERPRETING.md and `harness/`: `assert.js` and `sta.js` before
every case but `raw` ones; `doneprintHandle.js` for `async` cases; the `includes:` files in order
(34 files in `harness/`; the most included are `propertyHelper.js` 5,242, `temporalHelpers.js`
2,818, `testTypedArray.js` 2,093, `compareArray.js` 1,809); `print`; and `$262` with
`createRealm`, `detachArrayBuffer`, `evalScript`, `gc`, `global`, `IsHTMLDDA`,
`AbstractModuleSource` and `agent`. Test files that use them (`grep -l`): `$262.createRealm` 284,
`$262.agent` 112 (all under `built-ins/Atomics`), `$262.IsHTMLDDA` 42, `$262.evalScript` 31,
`$262.gc` 15, `$262.AbstractModuleSource` 12, `$262.detachArrayBuffer` 6 (most use it through
`detachArrayBuffer.js`). `assert.js` (156 lines), `sta.js` (22) and `propertyHelper.js` (515) use
only ES5 syntax: the only matches of `=>`, `let`, `const`, a backtick, `class` or `...` in them
are in comments. They call `Function.prototype.call.bind`, `Object.getPrototypeOf`,
`Object.getOwnPropertyDescriptor`, `Object.getOwnPropertyNames`, `Object.defineProperty`,
`Object.prototype.hasOwnProperty`, `Object.prototype.propertyIsEnumerable`,
`Object.prototype.toString`, `Array.prototype.join`, `.push`, `.map`, `Array.isArray`,
`Math.max` and `JSON.stringify`, and use `for-in`, `switch`, `try`, `instanceof` and `typeof`.

The largest test file is `test/staging/sm/String/string-upper-lower-mapping.js`, 3,234,014
bytes; the next are 299,825 and 130,705 bytes (`ls -l`, sorted).

## 1. Scope and target

**The target is the ECMA-262 draft at a pinned commit, today `74c33d4` of 2026-11-19 (ES2027,
"the eighteenth edition" by its introduction), held by test262 at a pinned commit, today
`2a1ade3`.** Both pins move together, in one change that updates the inventory of this document,
the feature list and the gate's expected counts; never by themselves, and at most once per
delivered slice unless the operator asks. A move re-evaluates every "complete" mark.

Why: test262 is written against the draft, not against a published edition, so the draft is the
only text its counts measure. The published ES2026 edition was not read (not checked).

Rejected: a published edition (test262 has no commit that matches one exactly); following
`main` continuously (counts would change without a change in swell).

Per area:

- ECMA-402 (`Intl`), 3,404 cases: out at first, with ECMA-262's locale-independent definitions
  of `toLocaleString` and `localeCompare` meanwhile. When: open item A.
- Annex B, 1,155 cases: in. Clause 2: "Web browsers are generally required to implement all
  normative optional subclauses. (See Annex B.)" Slice 18.
- `Temporal`, 4,617 cases (slice 20): out, since it is stage 4 but not in the pinned draft. It
  comes after slice 19 with a design of its own.
- `SharedArrayBuffer` and `Atomics`, 598 cases: in, for one agent whose [[CanBlock]] is false,
  as a window's agent is. Slice 14.
- Concurrent agents (`$262.agent`), 112 cases: out until surf needs workers (open item B).
- `WeakRef` and `FinalizationRegistry`, 82 cases: in. The collector is precise and deterministic
  (decision 4), and the runner provides `$262.gc`. Slice 16.
- Tail calls (15.10, `PrepareForTailCall`), 35 cases: in. The draft requires them in strict
  code, and with frames on the engine's own stack a tail call is a frame replacement. Slice 17.
- Stage 3 features, 2,591 cases: out, each listed by name, until it reaches the pinned draft.

How an exclusion is recorded so it cannot be forgotten: the repository holds
`conformance/test262/features.txt` with one line per test262 feature name, `run <name>` or
`exclude <name> <reason> <decision>`. The runner reads every case's `features:` and fails the gate
when a case names a feature the file does not list, so a pin move that brings a new feature stops
the gate until someone decides. An excluded case counts as existing and not run, per directory, so
the omission is visible in every report. Cases whose flags do not apply to the host configuration
(`CanBlockIsTrue` while [[CanBlock]] is false) count as `not-applicable`, also visible. The 17
stage 3 exclusions today: `Intl.Era-monthcode`, `Intl.Locale-info`, `Intl.NumberFormat-v3`,
`legacy-regexp`, `ShadowRealm`, `decorators`, `source-phase-imports`,
`source-phase-imports-module-source`, `import-defer`, `export-defer`, `import-text`,
`immutable-arraybuffer`, `import-bytes`, `nonextensible-applies-to-private`, `await-dictionary`,
`error-stack-accessor`, `Iterator.prototype.join` (the last one is finished by
`finished-proposals.md`; it moves to `run` when the pinned draft has it).

Two exclusions no feature name carries (added on 2026-11-21, measured while the foundation was
decomposed). 423 cases under `test/intl402/` name no feature that starts with `Intl` or is
excluded, and the 112 `$262.agent` cases name no feature that sets them apart, so by features
alone they would count as run and failed. The `swell-slices` block therefore has two more line
kinds, which the runner reads and `slices.awk` ignores (the 22 count lines do not change):

- `exclude-slice <slice> <reason>`: every case of that slice is excluded. Slices 20 and 21 have
  one each until their design exists.
- `exclude-include <harness file> <reason>`: every case whose `includes:` names the file is
  excluded. `atomicsHelper.js` is included by 112 cases, the same 112 that use `$262.agent`
  (`grep -rl 'atomicsHelper.js' test | wc -l`, `grep -rl '\$262\.agent' test | xargs grep -L
  'atomicsHelper.js' | wc -l` prints 0), until open item B is taken.

A case excluded this way counts as existing and excluded in its directory and in its slice, like
a case with an excluded feature.

## 2. Crates and what depends on what

**Six library crates and three binary crates, in one Cargo workspace:**

- `swell-unicode`: the generated tables (decision 8) and their lookups. Depends on nothing.
- `swell-num`: number to string and back in every radix, `BigInt` arithmetic, the `Math`
  functions, date arithmetic (day, month, year from a time value), the keyed hash and the
  random generator. Depends on nothing.
- `swell-regexp`: the regular expression parser, its early errors, the compiler and the matcher
  over UTF-16 (decision 7). Depends on `swell-unicode`.
- `swell-syntax`: lexer; parser with cover grammars, early errors, strict mode, automatic
  semicolon insertion, and regular expression literals checked by `swell-regexp`; the syntax
  tree; scope analysis. Depends on `swell-unicode`, `swell-num`, `swell-regexp`.
- `swell-compiler`: the bytecode format and the compile step from the analysed tree. Depends on
  `swell-syntax`.
- `swell`: the engine and its public host interface: values, heap and collector, interpreter,
  realms, jobs, modules, and the builtins, one module per ECMA-262 clause (19 to 28, then
  Annex B). Depends on all of the above.
- `swell-cli`: the binary `swell`, a command-line host (decision 9). Depends on `swell`.
- `swell-test262`: the binary `swell-test262`, the conformance runner (decision 11). Depends on
  `swell`.
- `swell-unicode-gen`: a tool that writes `swell-unicode`'s tables from the vendored Unicode
  data. Depends on nothing; never ships.

Seven of the nine exist (corrected on 2026-11-21: the root `Cargo.toml` takes every directory
under `crates/` as a member, and the root `Cargo.lock` lists seven packages): the five library
crates `swell-unicode`, `swell-num`, `swell-syntax`, `swell-compiler` and `swell`, and the two
binary crates `swell-test262` and `swell-unicode-gen`. `swell-regexp` and `swell-cli` are not
created yet: a crate is created in the first slice that needs any part of it (section 12, "A
part before its slice"), which for `swell-regexp` is slice 5.

Why: each crate below `swell` holds no heap value and can be tested and fuzzed alone (the parser
on source text, the matcher on a pattern and a string, the number code on bit patterns), and the
dependency graph has no cycle. The builtins stay inside `swell` because every one of them reads
and writes the heap; one module per clause keeps the mapping to the standard, and to the test262
directories of decision 11, one to one.

Rejected: one crate for everything (no part can be fuzzed without the heap); a crate per builtin
clause (each needs the heap's private interface, so it would have to be public).

Every crate has `#![forbid(unsafe_code)]`, and the root `Cargo.lock` lists no package from outside
the repository (decision 14).

## 3. Execution model

**A stack-based bytecode interpreter, with every ECMAScript frame on the engine's own frame stack,
not on the Rust stack.**

- Bytecode against walking the tree: bytecode. A tree walker recurses in Rust for every call, so a
  generator could only be suspended by keeping the Rust stack alive (a thread, or unsafe stack
  switching), which the rules exclude. With frames as data (instruction index, operand stack,
  local slots, environment, handler table), `yield` and `await` move the frame into the generator
  or async function object and resume moves it back. Rejected: the tree walker.
- Stack against register machine: stack machine. The compile step is simpler and has fewer ways
  to be wrong, which matters more first than speed; the speed difference is not measured here.
  Rejected for now: a register machine; revisit with a measurement.
- Async generators keep a request queue on the object (27.6.3), and an `await` inside them saves
  the frame like `yield`. No host thread is used anywhere in the engine.
- A builtin that calls back into script (an accessor, a `Proxy` trap, `Array.prototype.map`'s
  callback, `toString` and `valueOf` during conversion, `JSON.stringify`'s `toJSON`) enters the
  interpreter loop again from Rust. Every such re-entry counts toward the depth bound below.

**Names: a slot fixed at compile time, except where `with` or a non-strict direct `eval` can
change what a name means.** Added on 2026-11-21 for slice 2. A binding is resolved when the
code is compiled, as slice 1 does (`crates/compiler/src/resolve.rs`): a local slot of its
function's frame, a numbered slot of an environment a counted number of records outward, or a
name of the global environment. Two constructs make the meaning of a name depend on the run:

- A `with` statement evaluates its body in a new Object Environment Record over an object
  known only at run time (`sec-with-statement`, `sec-with-statement-runtime-semantics-evaluation`),
  whose bindings are that object's properties, filtered by `@@unscopables`
  (`sec-object-environment-records`).
- A direct `eval` in non-strict code adds its `var` and function declarations to the caller's
  variable environment (`sec-performeval`: varEnv is the running context's
  VariableEnvironment unless strictEval is true; `sec-evaldeclarationinstantiation`), so a
  binding can appear in that scope at run time. A direct `eval` is a call whose callee is the
  IdentifierReference `eval` (`sec-function-calls-runtime-semantics-evaluation`, step 6).

The rule:

1. A reference is looked up by name at run time, through the Environment Records from the
   innermost outward (`sec-resolvebinding`, `sec-getidentifierreference`), when it is in eval
   code, or when between the reference and the scope that would bind it at compile time (the
   global scope when none does) there lies a `with` statement whose body contains the
   reference, or a function or script scope whose own code is non-strict and contains a
   direct `eval` call (a call in a nested function does not count for the outer scope). A
   function nested inside such a body or scope is inside it.
2. Every other reference keeps its compile-time slot. Strict code has no `with` (an early
   error, `sec-with-statement-static-semantics-early-errors`), and its direct `eval` adds no
   binding to the caller (strictEval: varEnv is a new declarative record), so a strict
   function keeps its slots, also when it calls `eval` itself; a strict function nested in a
   non-strict scope that has a direct `eval` looks up by name the names that cross that scope,
   by item 1.
3. A binding that a lookup by name can reach is findable by name: every binding of every scope
   that encloses a `with` body or a direct `eval` call (strict or not), up to the global
   environment, lives in an Environment Record that keeps the names of its slots
   (`sec-declarative-environment-records`), not in a frame-local slot. References to it under
   item 2 still use its slot.
4. A function whose code contains a direct `eval` gets its arguments object whenever
   FunctionDeclarationInstantiation creates one (`sec-functiondeclarationinstantiation`,
   steps 15 to 22), whether or not its own code names `arguments`.
5. Budgets. A lookup by name charges one step per Environment Record it visits, so that a read
   through n records spends at least n steps more than a slot read (`last_entry_steps()`);
   the HasProperty and the `@@unscopables` Get of an Object Environment Record are property
   operations charged and checked as any other, and a getter or `Proxy` trap they reach is a
   call counted like any call. The names of a record's slots are part of the compiled code and
   are charged to the memory budget with it; a binding that eval code adds at run time is an
   allocation charged as any other.

The results are the standard's and test262 holds them; which form a reference takes is held by
the compiler's source tests, whose disassembly shows a slot or a lookup by name.

**The step budget.** One unit per bytecode instruction, plus units charged by builtins in
proportion to their work: per element for array, typed array and collection loops, per code unit
for string operations, per matcher step for regular expressions (decision 7), per limb operation
for `BigInt`, per node for `JSON.parse`. The counter is decremented where the unit is spent and
checked at every backward jump, every call and return, every re-entry, and inside every builtin
loop. At zero the run ends with the outcome `Terminated(StepBudget)`: no `catch` or `finally`
block runs, and the host gets the outcome as a value (decision 9). The host's interrupt
(decision 9) is checked at the same places. Why there: a loop cannot run without passing a
backward jump or a builtin loop check, so the check placement bounds the work between checks.
Rejected: a wall-clock limit inside the engine (the engine has no clock, and time would make runs
non-deterministic).

**Depth bounds,** each a limit the host sets, each ending in an outcome:

- The parser (nested expressions, statements, functions): a nesting counter in the recursive
  descent parser. Exceeded, the parse fails with a `SyntaxError` whose message names
  `parser_nesting` (`host-interface.md`, decision 2): through the host as `Parsed::Errors`,
  through `eval` and `Function` as a thrown `SyntaxError` the script can catch (HOST-139 to
  HOST-141), never as a `RangeError`.
- Scope analysis and the compile step walk only trees the parser accepted, so the parser's bound
  holds for them.
- ECMAScript calls: frames on the engine stack, counted. Exceeded: a `RangeError` in the script.
- Re-entry from Rust into the interpreter: counted with the frames, and separately against a
  limit derived from the Rust stack the host guarantees (decision 9). Exceeded: a `RangeError`.
- `JSON.parse`: a loop with an explicit stack, bounded by the memory budget.
- `JSON.stringify`, `toString` chains, `Proxy` trap chains: each step that calls script is a
  re-entry, bounded as one.
- Regular expressions: the matcher is a loop with an explicit backtrack stack, bounded by the
  step budget. A pattern's own nesting counts against `parser_nesting`, from zero at the start
  of the pattern, and passing it is the same `SyntaxError` naming `parser_nesting`
  (`host-interface.md`, decision 2): an early error of the script or module for a literal,
  thrown by the `RegExp` constructor at run time, never a `RangeError` (HOST-142, HOST-143).

A `RangeError` for depth is what the script sees, and the script can catch it; a script that
catches it and recurses again ends at the step budget, which it cannot catch.

## 4. Values and the heap without unsafe code

**Value.** A Rust enum: `Undefined`, `Null`, `Boolean`, `Number(f64)`, and `String`, `Symbol`,
`BigInt`, `Object` each holding a typed handle. Expected size 16 bytes (an 8-byte payload and a
tag, aligned to 8); not measured, since no code exists, and a test will hold it. NaN-boxing through
`f64::to_bits` would make it 8 bytes in safe code, but every value access would then decode bits;
rejected until a measurement shows the 16 bytes matter. The `Value` type's interface hides the
representation, so the change stays possible.

**Heap.** One heap per agent, made of arenas, one per kind of thing (objects, strings, `BigInt`
digits, environments, compiled functions, shapes later). An arena is a `Vec` of slots; a handle is
`(index: u32, generation: u32)` and is typed by kind (`ObjectRef`, `StringRef` and so on), so a
string handle cannot be used as an object. A freed slot's generation is incremented, and every
access compares generations: a stale handle reaches no other object, and the mismatch ends the
run with `Terminated(EngineFault)` rather than a panic. Because handles are indices into `Vec`s,
no handle can reach memory outside the heap at all; the generation check is what stops a logic
error from reading the wrong object.

**Objects.** Plain property maps first: per object an ordered list of (key, attributes, value)
with an index map above a small size, and dense element storage for arrays with a sparse
fallback, kept in the order 10.1.11.1 `OrdinaryOwnPropertyKeys` requires. Shapes (hidden classes)
are rejected for the first slices: they are an optimisation whose gain here is not measured, and
they add a second place where property order and attributes can go wrong. Hash maps inside the
engine use a SipHash written in `swell-num`, keyed from the host's randomness
at agent creation (decision 9), because `std`'s `RandomState` reads the operating system's random
source, which is ambient authority, and a fixed key lets a hostile script choose colliding keys.

**Collector.** Precise, stop-the-world mark and sweep over the arenas. Roots: the frame stack and
its operand stacks, the job queue, the realms and their intrinsics, the module map, the kept-objects
list of 9.10, and the host's root table. A builtin that can call script keeps every value it still
needs in slots of its own frame on the engine stack, which the collector scans; a Rust local
holding a handle across a call into script is a bug, found by a test mode that collects at every
safe point and by the generation check. Collections run only at safe points (instruction
boundaries and the re-entry points), when the bytes allocated since the last collection pass a
threshold that grows with the live size, and when the host asks. They never run inside an
allocation. Triggers count bytes, never time, so for the same inputs and budgets the same objects
are collected at the same points, and `WeakRef` and `FinalizationRegistry` results are
deterministic. Weak references are cleared after marking (9.9); finalization callbacks become
cleanup jobs through `HostEnqueueFinalizationRegistryCleanupJob`.

**Host-held values.** The host holds values through a root table: a `Root` is an index and
generation into it, released explicitly or when its owner drops it. Values the host keeps inside
its own objects (decision 9) are traced, not rooted.

**Memory budget.** Every allocation and every growth goes through one accounting function that
charges the bytes (the slot's fixed size plus string units, property and element capacity,
`BigInt` limbs, bytecode, and the parser's tree while it exists); a sweep credits them back.
Growth of large buffers uses `Vec::try_reserve`, so a refusal by the allocator becomes an outcome
instead of an abort. Over the budget: `new ArrayBuffer` and the typed array constructors throw a
`RangeError`, as `CreateByteDataBlock` allows; any other allocation ends the run with
`Terminated(MemoryBudget)`.

**Source text.** Added on 2026-11-21 for slice 3. The source text of a script, of eval code
and of a dynamic function stays alive as long as a function made from it, and is charged to
the memory budget with the compiled code. The text is the code units the parse read: the
source given to ParseScript, the String PerformEval parsed, and for CreateDynamicFunction the
whole text of its last parse, `function anonymous(` P `\n) {\n` body `\n}`
(`sec-createdynamicfunction`). It is kept with the compiled code made from it, which lives
while the host holds the script, while that code runs, and while a function object made from
it is reachable (its [[ScriptOrModule]], "Collector" above). The charge is taken with the
compiled code's, when the parse and compile step that made the code ends, at least one byte
per code unit in either stored form of decision 5; the sweep that frees the compiled code
credits it back with the code, never earlier. A text that does not fit what is left of the
budget ends the run with `Terminated(MemoryBudget)`, as any other allocation. The text is kept
whatever HostHasSourceTextAvailable answers: `Function.prototype.toString` of an ECMAScript
function returns the part of it its [[SourceText]] covers ("Return
CodePointsToString(func.[[SourceText]])", `sec-function.prototype.tostring`) when the host's
hook answers true, and a String with the syntax of a NativeFunction when it answers false
(`HOST-120`, `sec-hosthassourcetextavailable`), as for a built-in or a bound function. What a
test reads: `heap_bytes()` after `parse_script` of a source of n code units is at least n more
than before it; it stays at least n more while a function made from that script is reachable,
also after the host released the script; and `collect()` after the host released the script
and no function made from it is reachable gives at least n bytes back.

**Cost.** The arenas cost a bounds check and a generation compare on every access, and plain maps
cost a search per property access where shapes would cost an offset. How much slower this is than
an engine with unsafe code is not measured; no number is given.

## 5. Strings

**Semantics are UTF-16 code units, as 6.1.4 requires; the stored form is one of two, Latin-1
(one byte per unit, every unit below 0x100) or 16-bit units, chosen when the string is made.**
Why: most text in pages is ASCII, and Latin-1 halves its memory without changing any index;
two forms behind one type keep every operation on code units. Rejected: UTF-8 storage (indexing by
code unit becomes a search); ropes, for now: repeated `+=` is quadratic without them, but the step
and memory budgets bound the damage, and ropes are added when a measurement asks for them.

Property keys are interned: a string used as a key gets an atom, and a canonical numeric string
that is an array index becomes an integer key. The atom table holds its entries weakly, so a
script that makes keys without end cannot grow it past what is alive. Strings have a length limit
the host sets; exceeding it throws a `RangeError`.

At the host boundary there are two conversions: exact (`Vec<u16>` or `&[u16]`) and to a Rust
`String`, replacing each lone surrogate with U+FFFD. These are Web IDL's `DOMString` and
`USVString` (3.2.10, 3.2.12). From the host, a `&str` is converted to units without loss.

## 6. Numbers

- **`Number::toString`** (6.1.6.1.20) is written here for every radix 2 to 36, with an exact
  shortest-digits algorithm over `swell-num`'s big integers, then laid out as the algorithm says
  (no exponent for radix ≠ 10; for radix 10 an exponent when n is outside -5 to 21, written
  `1e+21`). The Rust standard library prints shortest round-trip digits in radix 10 only
  (`core/src/fmt/float.rs`, `float_to_exponential_common_shortest`, Grisu with a Dragon fallback in
  `flt2dec/strategy/grisu.rs`); its `{:e}` output is used in tests as a check of the radix-10
  digits. Where its layout differs: `Display` never uses an exponent and `Debug` switches to one
  below 1e-4 and at 1e16 (`impl_general_format`, `float.rs` line 23), the exponent has no `+`,
  infinity is `inf` (`flt2dec/mod.rs` lines 381, 457), and -0 keeps its sign, where ECMA-262
  prints `0`. Rust's shortest digits do not always pick the candidate ECMA-262's note
  recommends: where the value is exactly halfway between two shortest candidates, `{:e}` takes
  the upper one and the engine's algorithm the even one (measured on 2026-11-21 by
  `crates/num/tests/to_string.rs`: 49 of 200,000 finite bit patterns, for example
  -174098110762417.125). The test allows a difference from Rust only on such a tie.
- **`toFixed`, `toExponential`, `toPrecision`** are written here over the exact decimal expansion,
  because their rounding ("pick the larger n") is not Rust's.
- **Parsing.** The grammars of `StringToNumber` and of numeric literals (hex, octal, binary,
  separators, `Infinity`, whitespace) are written here; the decimal digits found are converted by
  `str::parse::<f64>`, which rounds to nearest, ties to even (`dec2flt/mod.rs`, lines 9 to 13).
  ECMA-262 allows replacing digits after the 20th with 0 for radix 10; swell does not, so the
  result is the correctly rounded one.
- **`BigInt`** is written here: 32-bit limbs with 64-bit intermediates, schoolbook multiplication
  with Karatsuba above a size, Knuth's algorithm D for division, and radix conversion by divide and
  conquer. Sizes are bounded by the memory budget, and an operation whose result size is known in
  advance to exceed the host's `BigInt` limit throws a `RangeError` before computing (whether
  ECMA-262 words this allowance is not checked).
- **`Math`.** `std` documents `powi powf exp exp2 ln log log2 log10 cbrt hypot sin cos tan asin
  acos atan atan2 sinh cosh tanh asinh acosh atanh exp_m1 ln_1p` (and more) as "The precision of
  this function is non-deterministic. This means it varies by platform, Rust version, and can
  even differ within the same execution" (`std/src/num/f64.rs`). ECMA-262 makes these results
  "implementation-approximated". For a run to be deterministic, swell writes these functions
  itself in `swell-num`, from published argument-reduction and polynomial methods, in plain `f64`
  arithmetic, which IEEE 754 makes identical on every platform. `sqrt`, `abs`, `floor`, `trunc`,
  `round_ties_even` and `mul_add` are not in that list and come from `std`. Rejected: `std`'s
  functions (platform-dependent); a port of an existing C library (third-party code in what
  ships).
- **`Math.random`** draws from a generator in `swell-num` seeded from the host's randomness.
- **`Date`.** The current time is a host answer: the draft says only "the time value (UTC)
  identifying the current time"; HTML's `HostSystemUTCEpochNanoseconds(global)` (8.1.6.5) is the
  hook a browser will give. The time zone is the host's too, through the draft's
  `SystemTimeZoneIdentifier ( )`, `GetNamedTimeZoneOffsetNanoseconds ( timeZoneIdentifier,
  epochNanoseconds )`, `GetNamedTimeZoneEpochNanoseconds ( ... )` and
  `AvailableNamedTimeZoneIdentifiers ( )`. The command-line host and the test262 runner answer
  "UTC" unless told otherwise. `Date.parse` accepts the format of 21.4.1.32 and the forms
  `Date.prototype.toString` and `toUTCString` produce; nothing else (an implementation-defined
  fallback the standard allows and swell does not take).

## 7. Regular expressions

**An own engine in `swell-regexp`: the pattern is parsed with the early errors of 22.2.1 and
B.1.2, compiled to a small instruction set, and run by a backtracking matcher with an explicit
backtrack stack, every step charged to the step budget.** Why: ECMAScript's semantics (ordered
alternation, captures reset per iteration, backreferences, look-ahead and look-behind) are
defined by backtracking (22.2.2), and an explicit stack means no Rust recursion. A hostile pattern
such as `(a+)+$` therefore ends at the budget as `Terminated(StepBudget)`; it cannot hang the host.
Rejected for now: an automaton for the patterns that allow one (no backreferences, no look-around).
It would give linear time for those patterns, but it is a second engine that must give the same
captures; it comes as a measured optimisation, not as the safety mechanism.

In scope: named groups (with duplicate names), look-behind, the `d`, `g`, `i`, `m`, `s`, `u`, `v`
and `y` flags, modifiers (`(?i:...)`), Unicode property escapes and properties of strings under
`v`, set operations under `v`, `RegExp.escape`, and the Annex B pattern grammar (B.1.2). Case
folding: simple case folding from `CaseFolding.txt` under `u` and `v` ("it uses the Simple Case
Folding (scf(codePoint)) definitions in the file CaseFolding.txt"), and `toUppercase` under neither
(22.2.2.7.3 `Canonicalize`). All tables come from `swell-unicode` (decision 8). `legacy-regexp`
(`RegExp.$1` and friends) is stage 3 and excluded.

## 8. Unicode data

**Pinned to one Unicode version for every table; the data files are vendored under
`data/unicode/<version>/` with a `SOURCE` file holding each file's address and SHA-256; the tool
`swell-unicode-gen` writes `crates/unicode/src/tables/*.rs`, which are committed, and the gate
regenerates them into `.out/` and fails on any difference.** The version is open item C (17.0.0
recommended, the version test262 is written for).

What the engine needs, and from which file:

- `ID_Start`, `ID_Continue` for identifiers (12.7, UAX #31): `DerivedCoreProperties.txt`.
- White space (12.2, `Zs`): `DerivedGeneralCategory.txt`.
- `toUpperCase`, `toLowerCase` with full and conditional mappings: `UnicodeData.txt`,
  `SpecialCasing.txt`.
- Regular expression case folding: `CaseFolding.txt`.
- `String.prototype.normalize` (NFC, NFD, NFKC, NFKD; UAX #15): the decompositions in
  `UnicodeData.txt`, `CompositionExclusions.txt`, `DerivedNormalizationProps.txt`.
- Property escapes, names: `PropertyAliases.txt`, `PropertyValueAliases.txt`.
- Property escapes, values: `DerivedGeneralCategory.txt`, `Scripts.txt`, `ScriptExtensions.txt`,
  `PropList.txt`, `DerivedBinaryProperties.txt`, `DerivedCoreProperties.txt`,
  `emoji/emoji-data.txt`.
- Properties of strings under `v` (`RGI_Emoji` and the others): the emoji sequence files of the
  same version.

Only the files listed are vendored, not all of `UCD.zip` (9,101,877 bytes for 17.0.0, its
`content-length` on `unicode.org`). Where the emoji sequence files live for 17.0.0 is not checked.
The licence of the data is vendored beside it; its text was not read here (not checked). The
generator is build tooling and never ships; the tables it writes do.

## 9. The host interface

**Settled in `docs/design/host-interface.md` (2026-11-21), which holds wherever it and this
section differ.** This section is the first description; the first ESS model of it came back
with nine places where it is unclear or silent, and that document answers them from the text of
ECMA-262 and HTML, as 19 decisions and numbered requirements (`HOST-<n>`): 165 on 2026-11-21,
the count `task host-requirements-check` prints (corrected that day from 135, the count when
the document was written). The eleven places where it departs from this document:

1. Each entry point has a result type of its own instead of one `Outcome`; misuse answers
   `Err(HostError)`, never a panic.
2. After an engine fault the agent answers every call with `Err(HostError::Faulted)`; the
   interrupt handle is not a call.
3. A microtask checkpoint has one budget for the whole checkpoint, not one per job.
4. Generic, timeout and cleanup jobs are run by the host through `run_job`, not through `call`.
5. A module load is finished with a module the host parsed, or with a thrown value, not with
   source text.
6. Two seeds of fixed length (a 16-byte hash key, a 32-byte random seed) and no random-bytes
   hook.
7. `HostEventSet` is an operation of the memory model, not a hook.
8. Decision 4: a stale handle given by the host answers `Err(HostError::StaleHandle)`; only a
   stale handle inside the engine is an engine fault.
9. Decision 3: a source that nests deeper than the parser limit fails with a `SyntaxError`
   naming the limit, through `eval` and `Function` too, so that ParseScript and PerformEval
   keep the types the draft gives them (decided on 2026-11-21). It is a deliberate
   implementation limit, recorded in the section's omission list. The limits reached while
   running (frames, re-entry, `JSON`, regular expression matching) stay as decision 3 has them.
10. The `BigInt` limit is `bigint_bits`, with a unit.
11. Decision 3: a regular expression pattern's own nesting counts against the parser limit and
    fails with the same `SyntaxError`, in a literal and through the `RegExp` constructor, not
    with a `RangeError`.

An `Agent` is not `Send` (decided on 2026-11-21: surf runs a page's agent on one thread and
plans no workers): one thread for an agent's life; the interrupt handle is the one part another
thread may hold, and a compile-time test holds both.

Designed from what surf will need; the names follow ECMA-262 and HTML.

**Agents and realms.** An `Agent` is one engine instance: one heap, one frame stack, one job
queue, single-threaded, created with its limits (steps per entry, memory, frame depth, re-entry
depth, parser nesting, string and `BigInt` sizes), a hash and random seed, and the host object
that answers the hooks. An agent holds any number of realms (9.3); a realm is created with
`InitializeHostDefinedRealm` and the host may give its global object and `this` value (HTML's
`Window` and `WindowProxy`). `$262.createRealm` and same-origin frames are realms of one agent.

**Entry points**, each returning an `Outcome`: `parse_script`, `evaluate_script`, `parse_module`,
`load_requested_modules`, `link`, `evaluate_module`, `call(function, this, arguments)`,
`construct`, `perform_microtask_checkpoint` (runs promise jobs until the queue is empty, each
with the budget), and `collect`. `Outcome` is `Normal(value)`, `Throw(value)` (an exception the
script did not catch, as a value the host can read), or `Terminated(reason)` with reason
`StepBudget`, `MemoryBudget`, `Interrupted` or `EngineFault`. After `EngineFault` the agent
answers every call with `EngineFault`; the host discards it.

**Interrupting.** `Agent::interrupt_handle()` gives a `Send` handle holding an
`Arc<AtomicBool>`; setting it from any host thread ends the run at the next check (decision 3)
with `Terminated(Interrupted)`. This is how surf can stop a page's script without a budget.

**Host hooks,** a Rust trait with ECMA-262's default behaviour where the draft gives one. The
draft's hooks: `HostCallJobCallback`, `HostEnqueueFinalizationRegistryCleanupJob`,
`HostEnqueueGenericJob`, `HostEnqueuePromiseJob`, `HostEnqueueTimeoutJob`,
`HostEnsureCanAddPrivateElement`, `HostEnsureCanCompileStrings`, `HostFinalizeImportMeta`,
`HostGetCodeForEval`, `HostGetImportMetaProperties`, `HostGetSupportedImportAttributes`,
`HostGrowSharedArrayBuffer`, `HostHasSourceTextAvailable`, `HostLoadImportedModule`,
`HostMakeJobCallback`, `HostPromiseRejectionTracker`, `HostResizeArrayBuffer`, `HostEventSet`
(the draft's `id="sec-host..."` anchors). The HTML Standard defines its own version of these
(8.1.6.1 to 8.1.6.7.3): `HostEnsureCanAddPrivateElement`, `HostEnsureCanCompileStrings`,
`HostGetCodeForEval`, `HostPromiseRejectionTracker`, `HostSystemUTCEpochNanoseconds`,
`HostCallJobCallback`, `HostEnqueueFinalizationRegistryCleanupJob`, `HostEnqueueGenericJob`,
`HostEnqueuePromiseJob`, `HostEnqueueTimeoutJob`, `HostMakeJobCallback`,
`HostGetImportMetaProperties`, `HostGetSupportedImportAttributes`, `HostLoadImportedModule`.
Beside the hooks, the trait answers the clock, the time zone operations of decision 6, and random
bytes; the engine has no other way to reach time, randomness, files, the network or the
environment. Module loading is asynchronous as the draft has it: `HostLoadImportedModule` returns
at once, and the host later calls `finish_loading_imported_module` with the module's source or
an error.

**Jobs.** The promise job queue is kept in the agent, because jobs hold engine values; the host
is told of each job through `HostEnqueuePromiseJob` (so HTML can attach its incumbent settings
through `HostMakeJobCallback`) and decides when to drain it, which is HTML's microtask checkpoint.
Generic and timeout jobs are handed to the host, which runs them through `call` when it chooses.

**Host objects for the DOM.** The host defines classes, each a Web IDL interface (3.7): an
interface object, an interface prototype object, constants, attributes as accessor properties and
operations as methods, whose behaviour is a host function. A host object carries a brand (the
interface it implements, for the "is a platform object implementing X" checks of 3.8) and host
data the host defines (any Rust type, downcast with `std::any::Any`). The engine traces host data
through a `trace` method the host implements, which lists the handles the data holds; that is how
a DOM node's event listeners stay alive exactly as long as the node is reachable. For legacy
platform objects (3.9: named and indexed properties, such as `HTMLCollection` and `Storage`), for
the `WindowProxy` exotic object (HTML 7.2.3) and for `Location`, the host may override the
internal methods [[GetPrototypeOf]] to [[OwnPropertyKeys]]; the engine checks what an override
returns against the invariants of 6.1.7.3 and ends the run with `EngineFault` when one is broken,
since that is a host bug. `document.all` (HTML 2.6.4.1) is a host object with the [[IsHTMLDDA]]
slot (B.3.6), which the engine supports. Web IDL's conversions (3.2: integers, sequences,
dictionaries, records, unions, promises) belong to surf's binding layer, written over the
engine's abstract operations (`to_number`, `to_string`, `get`, `call`, iteration), which the
interface exposes.

**Rooting.** The host roots what it holds from outside any engine object: its entry points (the
global object, the document wrapper, a pending callback). Everything reachable from an object is
traced. Cross-heap cycles (a DOM node in host memory holding a listener that holds the node's
wrapper) do not leak, because the node's references go through traced host data, not roots.

**The Rust stack.** Re-entry (decision 3) uses the Rust stack; the interface documents the
minimum stack the host must run the agent on, and a test runs at the re-entry limit on a thread
of that size. The size is not measured yet.

**What surf must have before it can use the engine at all:** a mutable DOM (surf has none: its
parser builds a read-only tree, `docs/design/html-parser.md` item 1, and `agent-waves.md` plans no
DOM), Web IDL bindings for at least `Window`, `Document`, `Node`, `Element` and `EventTarget`, the
HTML event loop with its task queues and the microtask checkpoint, and script processing in the
parser (today "nothing is prepared or run, the parser never pauses", `html-parser.md` item 9).

**What swell can prove without surf:** test262 through `swell-test262`, scripts and modules from
files through the command-line host `swell` (its clock, time zone and random seed are given on the
command line, so a run repeats exactly), and fuzzing.

## 10. Spec first, here

**ECMA-262 states the language and test262 holds the engine to it; an ESS model states the host
interface, and nothing else.** The ESS model (`spec/`, written before the host interface's code,
starting with slice 1) has the entities `swell.host.Agent`, `Realm`, `Script`, `Module` and `Job`;
commands such as `CreateAgent`, `CreateRealm`, `EvaluateScript`, `LoadRequestedModules`,
`FinishLoadingImportedModule`, `LinkModule`, `EvaluateModule`, `PerformMicrotaskCheckpoint`,
`Interrupt`; outcomes and events such as `ScriptCompleted`, `ScriptThrew`, `RunTerminated`
(with the reason), `ModuleLoadRequested`, `PromiseRejectionTracked`; and lifecycles taken from the
draft where it has them (a module's [[Status]]: `new`, `unlinked`, `linking`, `linked`,
`evaluating`, `evaluating-async`, `evaluated`). Module loading follows surf's rule that a step
that takes time is one command that starts it and one per way it ends:
`HostLoadImportedModule` publishes `ModuleLoadRequested`, and the host ends it with
`FinishLoadingImportedModule`.

The model is held by a conformance runner over a thin adapter to the hand-written Rust interface,
as surf's `conformance/` is, in a Cargo workspace of its own (the one place a third-party crate
is allowed, because `ess-conformance` brings some). The engine's code is not generated from ESS:
the host interface is an in-process library interface, and what ESS 0.96.0 generates is
components with ports (surf `AGENTS.md`, "Layout"), not a library interface. Not checked: how well
ESS states an in-process library interface; the first slice finds out.

What is not put into ESS: the language's semantics (values, objects, the 2,300 distinct section
numbers of the draft, counted with `grep -o 'secnum>[^<]*' ecma262.html | sort -u | wc -l`).
ECMA-262 already states them in its own step-by-step algorithms, and test262 is their
conformance suite; restating them in ESS would be a second source that could disagree with the
first.

## 11. The conformance harness

**`swell-test262`, a binary in the repository, std only, runs test262 at the pinned commit.**

- **One case, one realm.** Each run gets a fresh agent and realm (INTERPRETING.md, "Realm
  Isolation"), the `$262` object and `print`, then `assert.js` and `sta.js` (not for `raw`),
  `doneprintHandle.js` for `async`, then the `includes:` in order.
- **Strict and non-strict.** Without `onlyStrict`, `noStrict`, `module` or `raw`, the case runs
  twice, the second time with `"use strict";\n` in front.
- **`negative`.** `parse`: parsing must fail with the named error and nothing may run;
  `resolution`: module loading or linking must fail with it; `runtime`: evaluation must throw an
  object whose constructor's `name` is the type.
- **Refusals.** While the engine is delivered slice by slice it refuses a construct of a later
  slice with a `SyntaxError` whose message names the slice (section 12, "A part before its
  slice"). A refusal or the parser's nesting limit never counts as the `SyntaxError` a
  `negative` case asks for: the runner reads the kind of the error from its message
  (`swell_syntax::message_kind`), and a run that a refusal or the nesting limit ends, in a
  harness file or in the case, fails whatever the case's `negative` says (decided in wave 5 of
  slice 1). A refusal the script catches (through `eval`, `Function`, `$262.evalScript`, a
  regular expression literal evaluated before slice 5, or a `String` method given a code unit
  outside ASCII before slice 5, section 12) does not end the run, so the runner also
  reads the agent's count of refusals (`Agent::refusals`, `host-interface.md` decision 13)
  after each run, every run having an agent of its own, and fails every run in which the count
  moved, whatever the case's `negative` and outcome say.
- **`async`.** After the script, the runner drains the job queue; the case passes when `print`
  received `Test262:AsyncTestComplete` and fails on `Test262:AsyncTestFailure:` or when the queue
  is empty without either. No timer is needed: an engine with no clock cannot wait for anything
  but jobs, except `Atomics.waitAsync` timeouts, for which the runner's host answers
  `HostEnqueueTimeoutJob` by running the job when the queue is otherwise empty.
- **Modules.** A loader over the case's directory: `./name` is the file beside the case, `.json`
  is a JSON module; `_FIXTURE` files are loaded only as dependencies.
- **Budgets.** Every run has a step budget and a memory budget, so a hanging case is a failed
  case, printed as `budget`. First values: 2^30 steps and 512 MiB, chosen, not derived; the runner
  prints the twenty most expensive cases by steps so the values can be set from data.
- **Output and the gate.** One line per directory (the second level, and the third under
  `language/expressions` and `language/statements`): `test262 <dir> <exist> <excluded>
  <not-applicable> <run> <passed> <failed> <budget>`, then one `total` line. The gate compares the
  lines with `conformance/test262/expected.txt` and fails on any difference, printing the lines
  that differ, so a lost pass fails and a new pass is taken in by committing the new expected
  file. A second block prints, per section of ECMA-262 listed in
  `conformance/test262/sections.txt` (section anchor, the test262 paths that test it, and its
  deliberate omissions), `complete` or `incomplete` with the reason. A section is `complete` only
  when every case of its paths passes and its list of omissions is empty.
- **Time on a shared machine.** One process; the runner splits the runs over worker threads
  (threads belong to the runner, a tool, not to the engine; each worker owns its agents). The
  worker count is a flag the task passes, run through `slot` like every test (`slot task
  test262`). The harness files are parsed again for every run: an agent owns its heap and a
  parsed script cannot be shared across agents (`host-interface.md`); the runner prints the time
  the runs take, and sharing compiled harness code across agents is decided from that number. 103,360
  runs; how long they take is not measured, since no engine exists.
- **Pinning and storage: fetched by commit, checked by checksum, never vendored.** `task
  test262-fetch` runs `git init` and `git fetch --depth 1 https://github.com/tc39/test262
  <commit>` into `.cache/test262/<commit>/` (ignored by git), then checks that
  `git rev-parse <commit>^{tree}` is `70ab70455f270f26e92d097d863a0b08ad020b94` and that the
  SHA-256 of `git ls-tree -r <commit>` is
  `1756c106c60a6360007cc35c6e4f2d18b23e8045551e27c525612782717cf17f`. Measured on 2026-11-21: the
  fetch took 4.4 s and 18 MB of objects; the checkout is 236 MB under `test/` and 57,252 files
  (275 MB with a shallow clone's `.git`). Rejected: vendoring (236 MB and 57,252 files in a public
  repository, every one scanned by the publishing gates); a submodule (the same fetch, with the pin
  less visible and an empty directory after a plain clone). test262 is BSD-licensed (its
  `LICENSE`); since nothing of it is committed or shipped, `NOTICE` names it only as test data
  fetched by the tooling.

## 12. Delivery order

Each slice ends with the gate green and the expected file raised. A case is counted in the first
slice after which it can pass: the highest slice among its directory, its features and its flags,
by the rules in the block below (each line `dir <prefix> <slice>`, `feature <name> <slice>`,
`flag <name> <slice>` or `exclude <feature> <reason>`; the longest matching prefix wins; a feature
without a line sets no slice). Since 18,673 cases name no feature, the counts say when a case can
first pass, not that it will; the runner gives the real numbers.

| slice | name | cases | runs |
|---|---|---|---|
| 1 | `harness` | 31 | 62 |
| 2 | `language-core` | 4,259 | 7,543 |
| 3 | `builtins-core` | 7,635 | 15,074 |
| 4 | `numbers-and-dates` | 1,323 | 2,606 |
| 5 | `regexp` | 2,254 | 4,503 |
| 6 | `bindings-and-iteration` | 4,204 | 7,952 |
| 7 | `generators` | 1,473 | 2,817 |
| 8 | `classes` | 6,273 | 12,453 |
| 9 | `collections-proxy-reflect` | 1,752 | 3,475 |
| 10 | `promises-and-async` | 6,647 | 12,979 |
| 11 | `modules` | 1,281 | 1,833 |
| 12 | `bigint` | 317 | 634 |
| 13 | `binary-data` | 3,044 | 6,044 |
| 14 | `shared-memory` | 598 | 1,192 |
| 15 | `iterator-helpers-and-resources` | 1,094 | 2,170 |
| 16 | `weak-references` | 82 | 164 |
| 17 | `tail-calls` | 35 | 35 |
| 18 | `annex-b` | 1,155 | 1,492 |
| 19 | `staging` | 1,474 | 2,766 |
| | slices 1 to 19 | 44,931 | 85,794 |
| 20 | `temporal` (excluded at first) | 4,617 | 9,234 |
| 21 | `intl402` (excluded at first) | 1,662 | 3,324 |
| x | stage 3 (excluded) | 2,591 | 5,008 |
| | total | 53,801 | 103,360 |

Slice 1 had 82 cases and 164 runs: the share of `test/harness/` by the rule `dir
test/harness/ 1`. 31 cases and 62 runs are what slice 1 passes. Measured on 2026-11-21 with the
check under "How the counts were made" (`awk -f slices.awk` over this block, which holds the 51
lines `dir test/harness/<file> <slice>`, and the inventory's `meta.tsv`, counting slice `01`):
`01 31 62`. Every moved case goes to a slice from 2 to 13, so slices 1 to 19 still hold 44,931
cases and 85,794 runs and the 22 lines still sum to 53,801 and 103,360.

Slice 2 had 5,267 cases and 9,217 runs by its rules (`dir test/language/ 2` and the 22
lines `dir test/harness/<file> 2`). 4,259 cases and 7,543 runs are what slice 2 passes:
1,008 cases that need a later slice move by the lines below the paragraph "Slice 2's cases
that need a later slice". Measured on 2026-11-21 with the same check (`awk -f slices.awk`
over this block and the inventory's `meta.tsv`, counting slice `02`): `02 4259 7543`. Every
moved case goes to a slice from 3 to 11, so slices 1 to 19 still hold 44,931 cases and
85,794 runs and the 22 lines still sum to 53,801 and 103,360.

What each slice builds, and the directories it raises most (paths under `test/`, cases):

1. `harness`: lexer, parser and compile step for the ES5 grammar with strict mode (the runner
   makes a strict run of every case), the interpreter, the heap, the builtins `assert.js` and
   `sta.js` call, `$262`, `print`, the runner. `harness` 31.
2. `language-core`: the rest of the ES5-level language: `eval`, `with`, `arguments`, labels,
   automatic semicolon insertion; the global `Function` with its constructor (CreateDynamicFunction
   of kind `normal`, through the parse entry of `eval`, decided on 2026-11-21, `AGENTS.md`),
   its `length` and `prototype`, and `Function.prototype.constructor` (slice 1 built
   `%Function.prototype%` with `call`, `bind` and `@@hasInstance`); and the parts "A part
   before its slice" gives it. `expressions/compound-assignment` 407,
   `expressions/assignmenttargettype` 318, `expressions/object` 282, `eval-code` 253.
3. `builtins-core`: `Object`, `Array`, `String`, `Error`, `JSON`, the global functions slice 2
   does not take, and what remains of `Function`: `Function.prototype.apply` and
   `Function.prototype.toString` (with HostHasSourceTextAvailable, HOST-120); the generator
   and async kinds of CreateDynamicFunction come with slices 7 and 10. `Object` 3,160, `Array`
   2,665, `String` 780, `Function` 417.
4. `numbers-and-dates`: `swell-num`, what slice 2 does not take of `Number`, `Math`, `Date`,
   `parseInt`, `parseFloat`. `Date` 492, `Number` 313, `Math` 284.
5. `regexp`: `swell-regexp`, `swell-unicode`, `RegExp` and the string methods that use it.
   `RegExp` 1,626, `String` 252, `literals` 240.
6. `bindings-and-iteration`: `let`, `const`, destructuring, spread, arrows, templates,
   `Symbol`, iterators, `for-of`. `statements/for-of` 584, `expressions/assignment` 335,
   `expressions/arrow-function` 310.
7. `generators`: frames saved in objects. `expressions/object` 303, `expressions/generators`
   288, `statements/generators` 266.
8. `classes`: classes, `super`, `new.target`, private names, static blocks. `statements/class`
   3,016, `expressions/class` 2,731.
9. `collections-proxy-reflect`: `Map`, `Set`, `WeakMap`, `WeakSet`, `Proxy`, `Reflect`. `Set`
   381, `Proxy` 308, `Map` 202.
10. `promises-and-async`: jobs, `Promise`, async functions and generators, `for await`.
    `statements/class` 1,348, `expressions/class` 1,341, `statements/for-await-of` 1,232,
    `Promise` 643.
11. `modules`: module records, the loader hook, dynamic `import`, `import.meta`, JSON modules.
    `expressions/dynamic-import` 616, `module-code` 595.
12. `bigint`: `BigInt`. `BigInt` 77, `literals` 59.
13. `binary-data`: `ArrayBuffer`, typed arrays, `DataView`, resizable buffers, base64.
    `TypedArray` 1,438, `TypedArrayConstructors` 680, `DataView` 510.
14. `shared-memory`: `SharedArrayBuffer` and `Atomics` for one agent. `Atomics` 380,
    `SharedArrayBuffer` 104.
15. `iterator-helpers-and-resources`: `Iterator` helpers, `using`, `DisposableStack`. `Iterator`
    666, `AsyncDisposableStack` 104.
16. `weak-references`: `WeakRef`, `FinalizationRegistry`, `$262.gc`. `FinalizationRegistry` 47,
    `WeakRef` 29.
17. `tail-calls`: `PrepareForTailCall`. Spread over `language/`.
18. `annex-b`: Annex B and [[IsHTMLDDA]]. `annexB/language` 845, `annexB/built-ins` 215.
19. `staging`: what `test/staging/` asks beyond the above. `staging/sm` 1,404.

Excluded at first: `temporal`, `built-ins/Temporal` 4,606; `intl402`, `intl402/Temporal` 492
and `intl402/DateTimeFormat` 239; stage 3, `intl402/Temporal` 1,548 and
`expressions/dynamic-import` 385.

86 of the 117 files in `test/harness/` fall after slice 1: 35 by their features and flags
(they test `asyncHelpers.js`, `deepEqual.js` on maps and sets, and others that need later
slices), and 51 by a `dir test/harness/<file> <slice>` line of the block below, each because
the case or a harness file it includes needs a later slice (a scan of each case and its
includes, comments removed and string contents blanked, for later syntax, automatic semicolon
insertion and the builtins called):

- to slice 2, 22 cases: the 14 `propertyhelper-*` cases and the `verifyProperty-` cases
  `configurable-object`, `restore`, `restore-accessor`, `same-value`, `string-prop`, `value` and
  `value-error` (`propertyHelper.js` reads `arguments.length` and calls
  `Function.prototype.call.bind` on the global `Function`, which slice 2 delivers with its
  `CreateDynamicFunction`, item 2 above; `-restore-accessor` also uses a method shorthand, which
  slice 2 parses); and `fnGlobalObject` (`Function("return this;")()`). These 22 were placed in
  slice 3 while this design gave `Function` to slice 3, and were placed again in slice 2 when
  slice 2 took it;
- to slice 3, 2: `asyncHelpers-asyncTest-without-async-flag` (`Object.hasOwn` and `globalThis`
  of slice 3, besides its method shorthand) and `asyncHelpers-asyncTest-func-throws-sync`
  (`asyncHelpers.js` reads `globalThis` in `asyncTest`; placed in slice 2 for automatic
  semicolon insertion, which slice 2 parses, and in slice 3 at the close of slice 2);
- to slice 4, 2: `assertRelativeDateMs` (`Date`) and `nans` (`Number.NaN`, `Math.pow`);
- to slice 6, 16: `verifyProperty-arguments`, `-noproperty` and `-undefined-desc` (arrow
  functions), the `compare-array-` cases `arguments`, `arraylike`, `different-elements`,
  `different-length`, `falsy-arguments`, `same-elements-different-order`, `samevalue` and
  `sparse` (arrow functions, templates, `const` or spread),
  `asyncHelpers-asyncTest-rejects-non-callable` and `asyncHelpers-asyncTest-return-not-thenable`
  (`const`), `nativeFunctionMatcher`, `promiseHelper` and `wellKnownIntrinsicObjects` (`const`,
  arrow functions);
- to slice 9, 6: the `deepEqual-*` cases without a feature (`deepEqual.js` calls `new Map()`);
- to slice 12, 1: `testIntl-getLocaleBaseName` (`testIntl.js` holds the BigInt literal
  `1_000n`);
- to slice 13, 2: `detachArrayBuffer` and `detachArrayBuffer-host-detachArrayBuffer` (`new
  ArrayBuffer`).

The rules are prefixes and the longest wins, so each file's line beats `dir test/harness/ 1`.
Within slices 1 to 19 the
order is that of dependencies: nothing can be counted before slice 1, slices 2 to 5 are the
language and library of ES5, 6 to 8 the syntax most later tests are written in (4,916 cases carry
`class`), 10 needs 7's suspended frames, and 13 needs 12 for `BigInt64Array`.

**A part before its slice.** Added on 2026-11-21, when the slice epics were drafted and the
order of section 12 was read against the crate graph of decision 2: `swell-syntax` depends on
`swell-unicode`, `swell-num` and `swell-regexp`, which section 12 builds in slices 4 and 5; the
heap of slice 1 needs the keyed hash of `swell-num`; `ToString` of a Number is needed from
slice 1 and `Number::toString` is slice 4; slice 3's `String` needs case mapping and `normalize`
from `swell-unicode`. The rule:

- A crate is created in the first slice that needs any part of it, with that part only. The
  slice section 12 names for it is where it becomes complete.
- A part taken early is built to the standard for what it covers, never as a stand-in that
  gives a wrong answer: slice 1 gets the keyed hash, decimal literal parsing and
  `Number::toString` for radix 10 in `swell-num`; slice 2 the other radices of
  `Number::toString` (below); slice 4 adds `toFixed`, `toExponential`, `toPrecision`, reading
  a string in a radix, `Math` and `Date`.
- Slice 2 takes these parts early, each complete for what it covers (added on 2026-11-21,
  measured by slice 2's decomposition on the pinned checkout: 365 cases of slice 2 stop first
  on `Number` or `Boolean`, 45 on a plain method or a shorthand property; these cases test the
  language of slice 2 and use the parts as tools):
  - plain method definitions (`m() {}`, a method named `get` or `set` included) and shorthand
    properties (`{ a }`) in object literals (`sec-method-definitions`,
    `sec-object-initializer`). A generator method stays with slice 7, an async or async
    generator method with slice 10, a computed property name with slice 6, `super` with
    slice 8;
  - `Number`: the constructor called and constructed (`sec-number-constructor-number-value`;
    its BigInt branch cannot be reached before slice 12), its value properties `EPSILON`,
    `MAX_SAFE_INTEGER`, `MAX_VALUE`, `MIN_SAFE_INTEGER`, `MIN_VALUE`, `NaN`,
    `NEGATIVE_INFINITY`, `POSITIVE_INFINITY` and its `prototype`
    (`sec-properties-of-the-number-constructor`), `Number.prototype.constructor`, `valueOf`
    (`sec-number.prototype.valueof`) and `toString` for every radix 2 to 36
    (`sec-number.prototype.tostring`, with `Number::toString` of
    `sec-numeric-types-number-tostring` in every radix, decision 6). Slice 4 delivers the rest:
    `toFixed`, `toExponential`, `toPrecision`, `toLocaleString`, the functions of the
    constructor (`isFinite`, `isInteger`, `isNaN`, `isSafeInteger`, `parseFloat`, `parseInt`)
    and reading a string in a radix;
  - `Boolean`: the constructor, its `prototype`, `Boolean.prototype.constructor`, `toString`
    and `valueOf` (`sec-boolean-constructor`, `sec-properties-of-the-boolean-prototype-object`).
    That is all of `Boolean`: nothing of it is left to slice 3, whose rule
    `dir test/built-ins/ 3` still counts `test/built-ins/Boolean/`;
  - `isNaN` and `isFinite` (`sec-isnan-number`, `sec-isfinite-number`). The other functions of
    the global object stay with their slices: `parseInt` and `parseFloat` with slice 4, the URI
    functions with slice 3 (`eval` is slice 2's own);
  - `String.prototype.toString` and `valueOf` (`sec-string.prototype.tostring`,
    `sec-string.prototype.valueof`). Slice 3 delivers the rest of `String`, slice 5 the methods
    item 5 names;
  - `Array.prototype.toString` (`sec-array.prototype.tostring`), taken on 2026-11-21 when the
    run lines of slice 2 showed a case of the language core failing on the string conversion
    of an array (11 cases in 22 runs over slices 2, 3 and 19). It uses `Array.prototype.join`,
    which slice 1 has, and the realm's `%Object.prototype.toString%`. The rest of `Array` stays
    with slice 3.
- Where the early part cannot be complete without a later slice's data or engine, the gap is a
  recorded omission the gate reads (`conformance/test262/sections.txt`), and the engine refuses
  the input with a defined outcome instead of guessing: before slice 5, an identifier character
  outside ASCII is a `SyntaxError`, a regular expression
  literal is lexed to its end and evaluating it throws a `SyntaxError`, and the `String` methods
  that need the Unicode tables of slice 5 answer what they can and refuse the rest, as follows
  (decided on 2026-11-21 for slice 3). The four case methods `toLowerCase`, `toUpperCase`,
  `toLocaleLowerCase` and `toLocaleUpperCase` (`sec-string.prototype.tolowercase`,
  `sec-string.prototype.touppercase`, `sec-string.prototype.tolocalelowercase`,
  `sec-string.prototype.tolocaleuppercase`) and `localeCompare`
  (`sec-string.prototype.localecompare`) exist in slice 3 for a String whose code units are all
  below 0x80, and refuse any other String until slice 5. For such a String each answers as the
  standard does: among the code points below 0x80 the case mappings of the Unicode Character
  Database map the 26 letters and nothing else, the two locale methods answer as `toLowerCase`
  and `toUpperCase` (no host locale; ECMA-402 is out, section 1), and two Strings of such code
  units are canonically equivalent only when they are equal, so `localeCompare` orders them by
  their code units and returns -1, +0 or 1. `localeCompare` refuses when the this value or
  `that`, each converted to a String, has a code unit of 0x80 or above. A refusal comes after
  the steps the clause gives before the mapping or the comparison (RequireObjectCoercible, then
  ToString of the this value and, for `localeCompare`, of `that`), so an error those steps
  throw is thrown instead of it; it throws a `SyntaxError` of the method's realm whose message
  is `case mapping outside ASCII is not supported before slice 5` (the four case methods) or
  `comparison outside ASCII is not supported before slice 5` (`localeCompare`), which
  `swell_syntax::message_kind` reads as a refusal of slice 5, and the agent counts it like a
  refused parse, caught or not (`host-interface.md`, decision 13). `normalize`
  (`sec-string.prototype.normalize`) does not exist before slice 5: `String.prototype` has no
  property `normalize`. Slice 5 brings the tables and removes both refusals; from then the five
  methods refuse no input. The cases of slice 3 these methods cannot pass (a String outside
  ASCII in the case or a harness file it includes) move to slice 5 by rule lines when slice 3
  measures them. White space is not among these gaps: `story:syntax-es5-and-strict` accepts
  every `Zs` code point in slice 1 with the table of `story:num-and-unicode-early-parts`.
- A slice may also deliver a feature of its own without a part that needs a later slice (added
  on 2026-11-21 for slice 2). The part is absent, never a stand-in with a wrong answer, and it
  is recorded where the gate reads it: the section's omission list in
  `conformance/test262/sections.txt`, by a line `omit <section anchor> <part>
  section-12-slice-<n>`, `<n>` being the slice that delivers the part and removes the line.
  Everything else of the feature is built to the standard. The section stays `incomplete` while
  the line is there (decision 11). First use: the arguments object of slice 2
  (`sec-arguments-exotic-objects`) without its `@@iterator` property, which
  CreateUnmappedArgumentsObject and CreateMappedArgumentsObject define as
  `%Array.prototype.values%` (`sec-createunmappedargumentsobject`,
  `sec-createmappedargumentsobject`), a part of slice 6: `omit sec-arguments-exotic-objects
  symbol-iterator-property section-12-slice-6`.
- The builtins the harness files call are built in slice 1 as far as `assert.js`, `sta.js` and
  the 31 cases need them; slice 3 completes each builtin. A method that exists is correct for
  every argument, or it does not exist yet, or it takes the third form stated for the five
  `String` methods above (added on 2026-11-21 for slice 3): it exists, answers a stated set of
  inputs, and refuses every other input with a counted refusal naming the slice that completes
  it. The third form is allowed only where the answer for every input of that set is the
  standard's and every other input ends in the counted refusal, never in a guessed answer, and
  only for a method this section names with its set; any other method has the first two forms
  only.
- A refusal is a `SyntaxError`, but not the one a `negative: parse` case names: the runner
  reads the kind of a parse error from its message (`swell_syntax::message_kind`), and a run
  that a refusal or the nesting limit ends, in a harness file or in the case, fails whatever the
  case's `negative` says. It prints the number of such runs as `test262-refused <runs>`, so the
  share of the checkout a later slice's parser still declines stays visible (53,564 of 85,556
  runs at slice 1). Before this rule, 3,011 negative parse cases of slices 2 to 18 were counted
  passed on a refusal. A refusal the script catches (through `eval` or `Function`, of a
  regular expression literal, or of a `String` method of the third form) is counted by the agent (`Agent::refusals`, `host-interface.md`
  decision 13), and the runner fails every run in which that count moved (decision 11).

**For surf.** A script that touches no DOM and uses only ES5 can run after slice 4, and with
regular expressions after slice 5; most present-day scripts with no DOM (classes, `async`,
modules) after slice 11. A script on a real page becomes realistic after slice 18 on swell's side,
and only once surf has the DOM, Web IDL bindings and event loop of decision 9, none of which
exists or is planned in surf's current 22 waves.

**Slice 2's cases that need a later slice.** Measured at the close of slice 2 (2026-11-21,
`story:slice-02-conformance-close`), with the run lines of the runner on the code of the slice:
1,008 of the 5,267 cases of slice 2 fail, every one on a part of a later slice, and move by the
lines of the block below; with them the line `feature exponentiation 4` moves 24 cases of slice
3. Each of the 1,032 moves is a line of `conformance/test262/moves.tsv` (path, from, to, kind,
evidence): the evidence is a token of `conformance/test262/later-parts.tsv` (the text the scan
looked for, its slice and the item or refusal that gives it that slice) present in the case or
a harness file it includes, or the feature of a `feature` line, and the task
`test262-moves-check` holds every line against this block and the pinned checkout. The block
gains 933 lines `dir <path> <slice>`: 8 for a directory whose cases of slice 2 all move to one
slice, 925 for a single file; the line of `test/harness/asyncHelpers-asyncTest-func-throws-sync.js`
changes from 2 to 3. The groups, by target slice and kind (lines of the list):

- to slice 3, part, 78: builtins of slice 3 the case calls: `Object.preventExtensions` 22,
  `Function.prototype.apply` 20, `String.fromCharCode` 13, `charAt` and `charCodeAt` 2,
  `Object.prototype.isPrototypeOf` 6, `Object.prototype.valueOf` 4 (read in the case: the
  receiver is an ordinary object or a function, not a Number, Boolean or String), `Object.create`
  3, `Object.setPrototypeOf` 1, `Object.defineProperties` 1, the `Array` methods `concat` 2,
  `reduce`, `reverse` and `sort` 1 each, and `globalThis` 1:
  `test/harness/asyncHelpers-asyncTest-func-throws-sync`, whose `asyncTest` reads `globalThis`
  before the function it calls throws;
- to slice 4, feature, 65: `feature exponentiation 4`, 41 cases of slice 2 and 24 of slice 3:
  `**` needs the deterministic `pow` of `swell-num`, which slice 4 brings, and is refused naming
  slice 4 until then (the 42nd case of slice 2 with the feature needs `const` and goes to 6);
- to slice 4, part, 60: `parseInt` 37 (33 of them `with` cases of `S12.10_A1`), `Math` beyond
  `max` 18, `Date` 4, `Number.prototype.toFixed` 1;
- to slice 5, part, 136: an identifier with a code point outside ASCII, written or as an escape,
  84 (the Unicode tables of slice 5 decide `ID_Start` and `ID_Continue`; one through `eval`), a
  regular expression literal 37 (25 `white-space/after-regular-expression-literal-*`, 12
  `statementList` cases), `RegExp` 6, `String.prototype.replace` 6 and `split` 3;
- to slice 6, part, 487: `let` declarations 161; the lookahead rule of the `let` declaration 19
  (`let-array-with-newline`, `let-block-with-newline` and `let-identifier-with-newline` under
  `statements/` `for`, `for-in`, `if`, `labeled`, `while` and `with`, and
  `do-while/let-array-with-newline`: `let` as an identifier before a line end, which slice 6
  decides with the declaration);
  `const` 65, arrow functions 53, a function declaration in a block or a `switch` 46, default
  parameters 45, spread and rest 36, computed property names 30, templates 11, `Symbol` 9,
  destructuring 9, a CoverInitializedName 2, `for-of` 1;
- to slice 7, part, 41: generator functions and methods, 36 of them `eval-code/direct` cases
  whose function has a parameter named `arguments`;
- to slice 8, part, 85: `class` 71, `super` 14;
- to slice 9, part, 2: `Reflect`;
- to slice 10, part, 74: async functions, arrows and generators, 48 of them in
  `eval-code/direct`;
- to slice 11, part, 4: `import.meta` 2, an `import` and an `export` declaration 1 each.

The counts sum to the 1,032 lines of the list. None of these cases passed on `main` before the
move, and none fails first on a part of slice 2 (the story's acceptance, run on its base).

```swell-slices
dir test/harness/ 1
dir test/harness/asyncHelpers-asyncTest-func-throws-sync.js 3
dir test/harness/propertyhelper-verifyconfigurable-configurable.js 2
dir test/harness/propertyhelper-verifyconfigurable-configurable-object.js 2
dir test/harness/propertyhelper-verifyconfigurable-not-configurable.js 2
dir test/harness/propertyhelper-verifyenumerable-enumerable.js 2
dir test/harness/propertyhelper-verifyenumerable-not-enumerable.js 2
dir test/harness/propertyhelper-verifynotconfigurable-configurable.js 2
dir test/harness/propertyhelper-verifynotconfigurable-not-configurable.js 2
dir test/harness/propertyhelper-verifynotenumerable-enumerable.js 2
dir test/harness/propertyhelper-verifynotenumerable-not-enumerable.js 2
dir test/harness/propertyhelper-verifynotwritable-not-writable-strict.js 2
dir test/harness/propertyhelper-verifynotwritable-writable.js 2
dir test/harness/propertyhelper-verifywritable-array-length.js 2
dir test/harness/propertyhelper-verifywritable-not-writable.js 2
dir test/harness/propertyhelper-verifywritable-writable.js 2
dir test/harness/verifyProperty-configurable-object.js 2
dir test/harness/verifyProperty-restore.js 2
dir test/harness/verifyProperty-restore-accessor.js 2
dir test/harness/verifyProperty-same-value.js 2
dir test/harness/verifyProperty-string-prop.js 2
dir test/harness/verifyProperty-value.js 2
dir test/harness/verifyProperty-value-error.js 2
dir test/harness/asyncHelpers-asyncTest-without-async-flag.js 3
dir test/harness/fnGlobalObject.js 2
dir test/harness/assertRelativeDateMs.js 4
dir test/harness/nans.js 4
dir test/harness/verifyProperty-arguments.js 6
dir test/harness/verifyProperty-noproperty.js 6
dir test/harness/verifyProperty-undefined-desc.js 6
dir test/harness/compare-array-arguments.js 6
dir test/harness/compare-array-arraylike.js 6
dir test/harness/compare-array-different-elements.js 6
dir test/harness/compare-array-different-length.js 6
dir test/harness/compare-array-falsy-arguments.js 6
dir test/harness/compare-array-same-elements-different-order.js 6
dir test/harness/compare-array-samevalue.js 6
dir test/harness/compare-array-sparse.js 6
dir test/harness/asyncHelpers-asyncTest-rejects-non-callable.js 6
dir test/harness/asyncHelpers-asyncTest-return-not-thenable.js 6
dir test/harness/nativeFunctionMatcher.js 6
dir test/harness/promiseHelper.js 6
dir test/harness/wellKnownIntrinsicObjects.js 6
dir test/harness/deepEqual-array.js 9
dir test/harness/deepEqual-circular.js 9
dir test/harness/deepEqual-deep.js 9
dir test/harness/deepEqual-mapset.js 9
dir test/harness/deepEqual-object.js 9
dir test/harness/deepEqual-primitives.js 9
dir test/harness/testIntl-getLocaleBaseName.js 12
dir test/harness/detachArrayBuffer.js 13
dir test/harness/detachArrayBuffer-host-detachArrayBuffer.js 13
dir test/language/arguments-object/cls-decl-meth-args-trailing-comma-multiple.js 8
dir test/language/arguments-object/cls-decl-meth-args-trailing-comma-null.js 8
dir test/language/arguments-object/cls-decl-meth-args-trailing-comma-single-args.js 8
dir test/language/arguments-object/cls-decl-meth-args-trailing-comma-spread-operator.js 8
dir test/language/arguments-object/cls-decl-meth-args-trailing-comma-undefined.js 8
dir test/language/arguments-object/cls-decl-meth-static-args-trailing-comma-multiple.js 8
dir test/language/arguments-object/cls-decl-meth-static-args-trailing-comma-null.js 8
dir test/language/arguments-object/cls-decl-meth-static-args-trailing-comma-single-args.js 8
dir test/language/arguments-object/cls-decl-meth-static-args-trailing-comma-spread-operator.js 8
dir test/language/arguments-object/cls-decl-meth-static-args-trailing-comma-undefined.js 8
dir test/language/arguments-object/cls-expr-meth-args-trailing-comma-multiple.js 8
dir test/language/arguments-object/cls-expr-meth-args-trailing-comma-null.js 8
dir test/language/arguments-object/cls-expr-meth-args-trailing-comma-single-args.js 8
dir test/language/arguments-object/cls-expr-meth-args-trailing-comma-spread-operator.js 8
dir test/language/arguments-object/cls-expr-meth-args-trailing-comma-undefined.js 8
dir test/language/arguments-object/cls-expr-meth-static-args-trailing-comma-multiple.js 8
dir test/language/arguments-object/cls-expr-meth-static-args-trailing-comma-null.js 8
dir test/language/arguments-object/cls-expr-meth-static-args-trailing-comma-single-args.js 8
dir test/language/arguments-object/cls-expr-meth-static-args-trailing-comma-spread-operator.js 8
dir test/language/arguments-object/cls-expr-meth-static-args-trailing-comma-undefined.js 8
dir test/language/arguments-object/func-decl-args-trailing-comma-spread-operator.js 6
dir test/language/arguments-object/func-expr-args-trailing-comma-spread-operator.js 6
dir test/language/arguments-object/mapped/enumerable-configurable-accessor-descriptor.js 6
dir test/language/arguments-object/mapped/nonconfigurable-descriptors-define-failure.js 6
dir test/language/arguments-object/meth-args-trailing-comma-spread-operator.js 6
dir test/language/arguments-object/unmapped/via-params-dflt.js 6
dir test/language/arguments-object/unmapped/via-params-dstr.js 6
dir test/language/arguments-object/unmapped/via-params-rest.js 6
dir test/language/block-scope/leave/ 6
dir test/language/block-scope/return-from/ 6
dir test/language/block-scope/shadowing/catch-parameter-shadowing-let-declaration.js 6
dir test/language/block-scope/shadowing/const-declaration-shadowing-catch-parameter.js 6
dir test/language/block-scope/shadowing/const-declarations-shadowing-parameter-name-let-const-and-var-variables.js 6
dir test/language/block-scope/shadowing/dynamic-lookup-from-closure.js 6
dir test/language/block-scope/shadowing/dynamic-lookup-in-and-through-block-contexts.js 6
dir test/language/block-scope/shadowing/let-declaration-shadowing-catch-parameter.js 6
dir test/language/block-scope/shadowing/let-declarations-shadowing-parameter-name-let-const-and-var.js 6
dir test/language/block-scope/shadowing/lookup-from-closure.js 6
dir test/language/block-scope/shadowing/lookup-in-and-through-block-contexts.js 6
dir test/language/block-scope/shadowing/parameter-name-shadowing-parameter-name-let-const-and-var.js 6
dir test/language/block-scope/syntax/for-in/ 6
dir test/language/block-scope/syntax/function-declarations/in-statement-position-case-expression-statement-list.js 6
dir test/language/block-scope/syntax/function-declarations/in-statement-position-default-statement-list.js 6
dir test/language/block-scope/syntax/redeclaration/class-name-redeclaration-attempt-with-class.js 8
dir test/language/block-scope/syntax/redeclaration/class-name-redeclaration-attempt-with-const.js 8
dir test/language/block-scope/syntax/redeclaration/class-name-redeclaration-attempt-with-function.js 8
dir test/language/block-scope/syntax/redeclaration/class-name-redeclaration-attempt-with-let.js 8
dir test/language/block-scope/syntax/redeclaration/class-name-redeclaration-attempt-with-var.js 8
dir test/language/block-scope/syntax/redeclaration/const-name-redeclaration-attempt-with-class.js 8
dir test/language/block-scope/syntax/redeclaration/const-name-redeclaration-attempt-with-const.js 6
dir test/language/block-scope/syntax/redeclaration/const-name-redeclaration-attempt-with-function.js 6
dir test/language/block-scope/syntax/redeclaration/const-name-redeclaration-attempt-with-let.js 6
dir test/language/block-scope/syntax/redeclaration/const-name-redeclaration-attempt-with-var.js 6
dir test/language/block-scope/syntax/redeclaration/fn-scope-var-name-redeclaration-attempt-with-class.js 8
dir test/language/block-scope/syntax/redeclaration/fn-scope-var-name-redeclaration-attempt-with-const.js 6
dir test/language/block-scope/syntax/redeclaration/fn-scope-var-name-redeclaration-attempt-with-function.js 6
dir test/language/block-scope/syntax/redeclaration/fn-scope-var-name-redeclaration-attempt-with-let.js 6
dir test/language/block-scope/syntax/redeclaration/function-declaration-attempt-to-redeclare-with-var-declaration-nested-in-function.js 6
dir test/language/block-scope/syntax/redeclaration/function-name-redeclaration-attempt-with-class.js 8
dir test/language/block-scope/syntax/redeclaration/function-name-redeclaration-attempt-with-const.js 6
dir test/language/block-scope/syntax/redeclaration/function-name-redeclaration-attempt-with-function.js 6
dir test/language/block-scope/syntax/redeclaration/function-name-redeclaration-attempt-with-let.js 6
dir test/language/block-scope/syntax/redeclaration/function-name-redeclaration-attempt-with-var.js 6
dir test/language/block-scope/syntax/redeclaration/inner-block-var-name-redeclaration-attempt-with-class.js 8
dir test/language/block-scope/syntax/redeclaration/inner-block-var-name-redeclaration-attempt-with-const.js 6
dir test/language/block-scope/syntax/redeclaration/inner-block-var-name-redeclaration-attempt-with-function.js 6
dir test/language/block-scope/syntax/redeclaration/inner-block-var-name-redeclaration-attempt-with-let.js 6
dir test/language/block-scope/syntax/redeclaration/inner-block-var-redeclaration-attempt-after-class.js 8
dir test/language/block-scope/syntax/redeclaration/inner-block-var-redeclaration-attempt-after-const.js 6
dir test/language/block-scope/syntax/redeclaration/inner-block-var-redeclaration-attempt-after-function.js 6
dir test/language/block-scope/syntax/redeclaration/inner-block-var-redeclaration-attempt-after-let.js 6
dir test/language/block-scope/syntax/redeclaration/let-name-redeclaration-attempt-with-class.js 8
dir test/language/block-scope/syntax/redeclaration/let-name-redeclaration-attempt-with-const.js 6
dir test/language/block-scope/syntax/redeclaration/let-name-redeclaration-attempt-with-function.js 6
dir test/language/block-scope/syntax/redeclaration/let-name-redeclaration-attempt-with-let.js 6
dir test/language/block-scope/syntax/redeclaration/let-name-redeclaration-attempt-with-var.js 6
dir test/language/block-scope/syntax/redeclaration/var-name-redeclaration-attempt-with-class.js 8
dir test/language/block-scope/syntax/redeclaration/var-name-redeclaration-attempt-with-const.js 6
dir test/language/block-scope/syntax/redeclaration/var-name-redeclaration-attempt-with-function.js 6
dir test/language/block-scope/syntax/redeclaration/var-name-redeclaration-attempt-with-let.js 6
dir test/language/block-scope/syntax/redeclaration/var-redeclaration-attempt-after-class.js 8
dir test/language/block-scope/syntax/redeclaration/var-redeclaration-attempt-after-const.js 6
dir test/language/block-scope/syntax/redeclaration/var-redeclaration-attempt-after-function.js 6
dir test/language/block-scope/syntax/redeclaration/var-redeclaration-attempt-after-let.js 6
dir test/language/comments/S7.4_A5.js 3
dir test/language/comments/S7.4_A6.js 3
dir test/language/comments/hashbang/function-constructor.js 10
dir test/language/eval-code/direct/arrow-fn-a-following-parameter-is-named-arguments-arrow-func-declare-arguments-assign-incl-def-param-arrow-arguments.js 6
dir test/language/eval-code/direct/arrow-fn-a-following-parameter-is-named-arguments-arrow-func-declare-arguments-assign.js 6
dir test/language/eval-code/direct/arrow-fn-a-preceding-parameter-is-named-arguments-arrow-func-declare-arguments-assign-incl-def-param-arrow-arguments.js 6
dir test/language/eval-code/direct/arrow-fn-a-preceding-parameter-is-named-arguments-arrow-func-declare-arguments-assign.js 6
dir test/language/eval-code/direct/arrow-fn-body-cntns-arguments-func-decl-arrow-func-declare-arguments-assign-incl-def-param-arrow-arguments.js 6
dir test/language/eval-code/direct/arrow-fn-body-cntns-arguments-func-decl-arrow-func-declare-arguments-assign.js 6
dir test/language/eval-code/direct/arrow-fn-body-cntns-arguments-lex-bind-arrow-func-declare-arguments-assign-incl-def-param-arrow-arguments.js 6
dir test/language/eval-code/direct/arrow-fn-body-cntns-arguments-lex-bind-arrow-func-declare-arguments-assign.js 6
dir test/language/eval-code/direct/arrow-fn-body-cntns-arguments-var-bind-arrow-func-declare-arguments-assign-incl-def-param-arrow-arguments.js 6
dir test/language/eval-code/direct/arrow-fn-body-cntns-arguments-var-bind-arrow-func-declare-arguments-assign.js 6
dir test/language/eval-code/direct/arrow-fn-no-pre-existing-arguments-bindings-are-present-arrow-func-declare-arguments-assign-incl-def-param-arrow-arguments.js 6
dir test/language/eval-code/direct/arrow-fn-no-pre-existing-arguments-bindings-are-present-arrow-func-declare-arguments-assign.js 6
dir test/language/eval-code/direct/async-gen-func-decl-a-following-parameter-is-named-arguments-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-func-decl-a-following-parameter-is-named-arguments-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-func-decl-a-preceding-parameter-is-named-arguments-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-func-decl-a-preceding-parameter-is-named-arguments-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-func-decl-fn-body-cntns-arguments-func-decl-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-func-decl-fn-body-cntns-arguments-func-decl-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-func-decl-fn-body-cntns-arguments-lex-bind-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-func-decl-fn-body-cntns-arguments-lex-bind-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-func-decl-fn-body-cntns-arguments-var-bind-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-func-decl-fn-body-cntns-arguments-var-bind-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-func-decl-no-pre-existing-arguments-bindings-are-present-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-func-decl-no-pre-existing-arguments-bindings-are-present-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-func-expr-a-following-parameter-is-named-arguments-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-func-expr-a-following-parameter-is-named-arguments-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-func-expr-a-preceding-parameter-is-named-arguments-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-func-expr-a-preceding-parameter-is-named-arguments-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-func-expr-fn-body-cntns-arguments-func-decl-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-func-expr-fn-body-cntns-arguments-func-decl-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-func-expr-fn-body-cntns-arguments-lex-bind-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-func-expr-fn-body-cntns-arguments-lex-bind-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-func-expr-fn-body-cntns-arguments-var-bind-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-func-expr-fn-body-cntns-arguments-var-bind-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-func-expr-no-pre-existing-arguments-bindings-are-present-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-func-expr-no-pre-existing-arguments-bindings-are-present-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-meth-a-following-parameter-is-named-arguments-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-meth-a-following-parameter-is-named-arguments-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-meth-a-preceding-parameter-is-named-arguments-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-meth-a-preceding-parameter-is-named-arguments-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-meth-fn-body-cntns-arguments-func-decl-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-meth-fn-body-cntns-arguments-func-decl-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-meth-fn-body-cntns-arguments-lex-bind-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-meth-fn-body-cntns-arguments-lex-bind-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-meth-fn-body-cntns-arguments-var-bind-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-meth-fn-body-cntns-arguments-var-bind-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-meth-no-pre-existing-arguments-bindings-are-present-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-meth-no-pre-existing-arguments-bindings-are-present-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-named-func-expr-a-following-parameter-is-named-arguments-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-named-func-expr-a-following-parameter-is-named-arguments-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-named-func-expr-a-preceding-parameter-is-named-arguments-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-named-func-expr-a-preceding-parameter-is-named-arguments-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-named-func-expr-fn-body-cntns-arguments-func-decl-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-named-func-expr-fn-body-cntns-arguments-func-decl-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-named-func-expr-fn-body-cntns-arguments-lex-bind-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-named-func-expr-fn-body-cntns-arguments-lex-bind-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-named-func-expr-fn-body-cntns-arguments-var-bind-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-named-func-expr-fn-body-cntns-arguments-var-bind-declare-arguments.js 10
dir test/language/eval-code/direct/async-gen-named-func-expr-no-pre-existing-arguments-bindings-are-present-declare-arguments-and-assign.js 10
dir test/language/eval-code/direct/async-gen-named-func-expr-no-pre-existing-arguments-bindings-are-present-declare-arguments.js 10
dir test/language/eval-code/direct/block-decl-eval-source-is-strict-nostrict.js 6
dir test/language/eval-code/direct/block-decl-eval-source-is-strict-onlystrict.js 6
dir test/language/eval-code/direct/block-decl-onlystrict.js 6
dir test/language/eval-code/direct/func-decl-a-following-parameter-is-named-arguments-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/func-decl-a-following-parameter-is-named-arguments-declare-arguments.js 6
dir test/language/eval-code/direct/func-decl-a-preceding-parameter-is-named-arguments-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/func-decl-a-preceding-parameter-is-named-arguments-declare-arguments.js 6
dir test/language/eval-code/direct/func-decl-fn-body-cntns-arguments-func-decl-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/func-decl-fn-body-cntns-arguments-func-decl-declare-arguments.js 6
dir test/language/eval-code/direct/func-decl-fn-body-cntns-arguments-lex-bind-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/func-decl-fn-body-cntns-arguments-lex-bind-declare-arguments.js 6
dir test/language/eval-code/direct/func-decl-fn-body-cntns-arguments-var-bind-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/func-decl-fn-body-cntns-arguments-var-bind-declare-arguments.js 6
dir test/language/eval-code/direct/func-decl-no-pre-existing-arguments-bindings-are-present-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/func-decl-no-pre-existing-arguments-bindings-are-present-declare-arguments.js 6
dir test/language/eval-code/direct/func-expr-a-following-parameter-is-named-arguments-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/func-expr-a-following-parameter-is-named-arguments-declare-arguments.js 6
dir test/language/eval-code/direct/func-expr-a-preceding-parameter-is-named-arguments-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/func-expr-a-preceding-parameter-is-named-arguments-declare-arguments.js 6
dir test/language/eval-code/direct/func-expr-fn-body-cntns-arguments-func-decl-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/func-expr-fn-body-cntns-arguments-func-decl-declare-arguments.js 6
dir test/language/eval-code/direct/func-expr-fn-body-cntns-arguments-lex-bind-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/func-expr-fn-body-cntns-arguments-lex-bind-declare-arguments.js 6
dir test/language/eval-code/direct/func-expr-fn-body-cntns-arguments-var-bind-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/func-expr-fn-body-cntns-arguments-var-bind-declare-arguments.js 6
dir test/language/eval-code/direct/func-expr-no-pre-existing-arguments-bindings-are-present-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/func-expr-no-pre-existing-arguments-bindings-are-present-declare-arguments.js 6
dir test/language/eval-code/direct/gen-func-decl-a-following-parameter-is-named-arguments-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-decl-a-following-parameter-is-named-arguments-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-decl-a-preceding-parameter-is-named-arguments-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-decl-a-preceding-parameter-is-named-arguments-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-decl-fn-body-cntns-arguments-func-decl-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-decl-fn-body-cntns-arguments-func-decl-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-decl-fn-body-cntns-arguments-lex-bind-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-decl-fn-body-cntns-arguments-lex-bind-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-decl-fn-body-cntns-arguments-var-bind-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-decl-fn-body-cntns-arguments-var-bind-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-decl-no-pre-existing-arguments-bindings-are-present-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-decl-no-pre-existing-arguments-bindings-are-present-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-expr-named-a-following-parameter-is-named-arguments-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-expr-named-a-following-parameter-is-named-arguments-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-expr-named-a-preceding-parameter-is-named-arguments-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-expr-named-a-preceding-parameter-is-named-arguments-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-expr-named-fn-body-cntns-arguments-func-decl-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-expr-named-fn-body-cntns-arguments-func-decl-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-expr-named-fn-body-cntns-arguments-lex-bind-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-expr-named-fn-body-cntns-arguments-lex-bind-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-expr-named-fn-body-cntns-arguments-var-bind-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-expr-named-fn-body-cntns-arguments-var-bind-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-expr-named-no-pre-existing-arguments-bindings-are-present-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-expr-named-no-pre-existing-arguments-bindings-are-present-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-expr-nameless-a-following-parameter-is-named-arguments-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-expr-nameless-a-following-parameter-is-named-arguments-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-expr-nameless-a-preceding-parameter-is-named-arguments-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-expr-nameless-a-preceding-parameter-is-named-arguments-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-expr-nameless-fn-body-cntns-arguments-func-decl-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-expr-nameless-fn-body-cntns-arguments-func-decl-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-expr-nameless-fn-body-cntns-arguments-lex-bind-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-expr-nameless-fn-body-cntns-arguments-lex-bind-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-expr-nameless-fn-body-cntns-arguments-var-bind-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-expr-nameless-fn-body-cntns-arguments-var-bind-declare-arguments.js 7
dir test/language/eval-code/direct/gen-func-expr-nameless-no-pre-existing-arguments-bindings-are-present-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-func-expr-nameless-no-pre-existing-arguments-bindings-are-present-declare-arguments.js 7
dir test/language/eval-code/direct/gen-meth-a-following-parameter-is-named-arguments-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/gen-meth-a-following-parameter-is-named-arguments-declare-arguments.js 6
dir test/language/eval-code/direct/gen-meth-a-preceding-parameter-is-named-arguments-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/gen-meth-a-preceding-parameter-is-named-arguments-declare-arguments.js 6
dir test/language/eval-code/direct/gen-meth-fn-body-cntns-arguments-func-decl-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/gen-meth-fn-body-cntns-arguments-func-decl-declare-arguments.js 6
dir test/language/eval-code/direct/gen-meth-fn-body-cntns-arguments-lex-bind-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/gen-meth-fn-body-cntns-arguments-lex-bind-declare-arguments.js 6
dir test/language/eval-code/direct/gen-meth-fn-body-cntns-arguments-var-bind-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/gen-meth-fn-body-cntns-arguments-var-bind-declare-arguments.js 6
dir test/language/eval-code/direct/gen-meth-no-pre-existing-arguments-bindings-are-present-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/gen-meth-no-pre-existing-arguments-bindings-are-present-declare-arguments.js 6
dir test/language/eval-code/direct/meth-a-following-parameter-is-named-arguments-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/meth-a-following-parameter-is-named-arguments-declare-arguments.js 6
dir test/language/eval-code/direct/meth-a-preceding-parameter-is-named-arguments-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/meth-a-preceding-parameter-is-named-arguments-declare-arguments.js 6
dir test/language/eval-code/direct/meth-fn-body-cntns-arguments-func-decl-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/meth-fn-body-cntns-arguments-func-decl-declare-arguments.js 6
dir test/language/eval-code/direct/meth-fn-body-cntns-arguments-lex-bind-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/meth-fn-body-cntns-arguments-lex-bind-declare-arguments.js 6
dir test/language/eval-code/direct/meth-fn-body-cntns-arguments-var-bind-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/meth-fn-body-cntns-arguments-var-bind-declare-arguments.js 6
dir test/language/eval-code/direct/meth-no-pre-existing-arguments-bindings-are-present-declare-arguments-and-assign.js 6
dir test/language/eval-code/direct/meth-no-pre-existing-arguments-bindings-are-present-declare-arguments.js 6
dir test/language/eval-code/direct/switch-case-decl-eval-source-is-strict-nostrict.js 6
dir test/language/eval-code/direct/switch-case-decl-eval-source-is-strict-onlystrict.js 6
dir test/language/eval-code/direct/switch-case-decl-onlystrict.js 6
dir test/language/eval-code/direct/switch-dflt-decl-eval-source-is-strict-nostrict.js 6
dir test/language/eval-code/direct/switch-dflt-decl-eval-source-is-strict-onlystrict.js 6
dir test/language/eval-code/direct/switch-dflt-decl-onlystrict.js 6
dir test/language/eval-code/indirect/block-decl-strict.js 6
dir test/language/eval-code/indirect/switch-case-decl-strict.js 6
dir test/language/eval-code/indirect/switch-dflt-decl-strict.js 6
dir test/language/expressions/addition/S11.6.1_A2.2_T2.js 4
dir test/language/expressions/array/spread-err-mult-err-unresolvable.js 6
dir test/language/expressions/array/spread-err-sngl-err-unresolvable.js 6
dir test/language/expressions/array/spread-mult-empty.js 6
dir test/language/expressions/array/spread-mult-expr.js 6
dir test/language/expressions/array/spread-mult-literal.js 6
dir test/language/expressions/array/spread-sngl-empty.js 6
dir test/language/expressions/array/spread-sngl-expr.js 6
dir test/language/expressions/array/spread-sngl-literal.js 6
dir test/language/expressions/assignment/11.13.1-3-s.js 3
dir test/language/expressions/assignment/11.13.1-4-28gs.js 4
dir test/language/expressions/assignment/11.13.1-4-29gs.js 4
dir test/language/expressions/assignment/8.12.5-3-b_1.js 3
dir test/language/expressions/assignment/S8.12.4_A1.js 4
dir test/language/expressions/assignment/assignment-operator-calls-putvalue-lref--rval--1.js 6
dir test/language/expressions/assignment/assignment-operator-calls-putvalue-lref--rval-.js 6
dir test/language/expressions/assignment/destructuring/ 6
dir test/language/expressions/assignment/fn-name-arrow.js 6
dir test/language/expressions/assignment/target-super-computed-reference-null.js 8
dir test/language/expressions/assignment/target-super-computed-reference.js 8
dir test/language/expressions/assignment/target-super-identifier-reference-null.js 8
dir test/language/expressions/assignmenttargettype/direct-arrowfunction-0.js 6
dir test/language/expressions/assignmenttargettype/direct-arrowfunction-1.js 6
dir test/language/expressions/assignmenttargettype/direct-arrowfunction-2.js 6
dir test/language/expressions/assignmenttargettype/direct-arrowfunction-3.js 6
dir test/language/expressions/assignmenttargettype/direct-arrowfunction-4.js 6
dir test/language/expressions/assignmenttargettype/direct-arrowfunction-5.js 6
dir test/language/expressions/assignmenttargettype/direct-arrowfunction-6.js 6
dir test/language/expressions/assignmenttargettype/direct-asyncarrowfunction-0.js 10
dir test/language/expressions/assignmenttargettype/direct-asyncarrowfunction-1.js 10
dir test/language/expressions/assignmenttargettype/direct-asyncarrowfunction-2.js 10
dir test/language/expressions/assignmenttargettype/direct-asyncarrowfunction-3.js 10
dir test/language/expressions/assignmenttargettype/direct-asyncarrowfunction-4.js 10
dir test/language/expressions/assignmenttargettype/direct-asyncarrowfunction-5.js 10
dir test/language/expressions/assignmenttargettype/direct-asyncarrowfunction-6.js 10
dir test/language/expressions/assignmenttargettype/direct-callexpression-as-for-of-lhs.js 6
dir test/language/expressions/assignmenttargettype/direct-callexpression-templateliteral.js 6
dir test/language/expressions/assignmenttargettype/direct-import.meta.js 11
dir test/language/expressions/assignmenttargettype/direct-memberexpression-templateliteral.js 6
dir test/language/expressions/assignmenttargettype/direct-primaryexpression-asyncfunctionexpression.js 10
dir test/language/expressions/assignmenttargettype/direct-primaryexpression-asyncgeneratorexpression.js 10
dir test/language/expressions/assignmenttargettype/direct-primaryexpression-classexpression.js 8
dir test/language/expressions/assignmenttargettype/direct-primaryexpression-generatorexpression.js 7
dir test/language/expressions/assignmenttargettype/direct-primaryexpression-templateliteral.js 6
dir test/language/expressions/assignmenttargettype/parenthesized-arrowfunction-0.js 6
dir test/language/expressions/assignmenttargettype/parenthesized-arrowfunction-1.js 6
dir test/language/expressions/assignmenttargettype/parenthesized-arrowfunction-2.js 6
dir test/language/expressions/assignmenttargettype/parenthesized-arrowfunction-3.js 6
dir test/language/expressions/assignmenttargettype/parenthesized-arrowfunction-4.js 6
dir test/language/expressions/assignmenttargettype/parenthesized-arrowfunction-5.js 6
dir test/language/expressions/assignmenttargettype/parenthesized-arrowfunction-6.js 6
dir test/language/expressions/assignmenttargettype/parenthesized-asyncarrowfunction-0.js 10
dir test/language/expressions/assignmenttargettype/parenthesized-asyncarrowfunction-1.js 10
dir test/language/expressions/assignmenttargettype/parenthesized-asyncarrowfunction-2.js 10
dir test/language/expressions/assignmenttargettype/parenthesized-asyncarrowfunction-3.js 10
dir test/language/expressions/assignmenttargettype/parenthesized-asyncarrowfunction-4.js 10
dir test/language/expressions/assignmenttargettype/parenthesized-asyncarrowfunction-5.js 10
dir test/language/expressions/assignmenttargettype/parenthesized-asyncarrowfunction-6.js 10
dir test/language/expressions/assignmenttargettype/parenthesized-callexpression-templateliteral.js 6
dir test/language/expressions/assignmenttargettype/parenthesized-import.meta.js 11
dir test/language/expressions/assignmenttargettype/parenthesized-memberexpression-templateliteral.js 6
dir test/language/expressions/assignmenttargettype/parenthesized-primaryexpression-asyncfunctionexpression.js 10
dir test/language/expressions/assignmenttargettype/parenthesized-primaryexpression-asyncgeneratorexpression.js 10
dir test/language/expressions/assignmenttargettype/parenthesized-primaryexpression-classexpression.js 8
dir test/language/expressions/assignmenttargettype/parenthesized-primaryexpression-generatorexpression.js 7
dir test/language/expressions/assignmenttargettype/parenthesized-primaryexpression-templateliteral.js 6
dir test/language/expressions/assignmenttargettype/simple-complex-callexpression-expression.js 6
dir test/language/expressions/assignmenttargettype/simple-complex-callexpression.identifiername.js 6
dir test/language/expressions/assignmenttargettype/simple-complex-memberexpression-expression.js 6
dir test/language/expressions/assignmenttargettype/simple-complex-memberexpression.identifiername.js 6
dir test/language/expressions/call/spread-err-mult-err-unresolvable.js 6
dir test/language/expressions/call/spread-err-sngl-err-unresolvable.js 6
dir test/language/expressions/call/spread-mult-empty.js 6
dir test/language/expressions/call/spread-mult-expr.js 6
dir test/language/expressions/call/spread-mult-literal.js 6
dir test/language/expressions/call/spread-sngl-empty.js 6
dir test/language/expressions/call/spread-sngl-expr.js 6
dir test/language/expressions/call/spread-sngl-literal.js 6
dir test/language/expressions/call/trailing-comma.js 6
dir test/language/expressions/coalesce/short-circuit-number-symbol.js 6
dir test/language/expressions/compound-assignment/11.13.2-45-s.js 3
dir test/language/expressions/compound-assignment/11.13.2-46-s.js 3
dir test/language/expressions/compound-assignment/11.13.2-47-s.js 3
dir test/language/expressions/compound-assignment/11.13.2-48-s.js 3
dir test/language/expressions/compound-assignment/11.13.2-49-s.js 3
dir test/language/expressions/compound-assignment/11.13.2-50-s.js 3
dir test/language/expressions/compound-assignment/11.13.2-51-s.js 3
dir test/language/expressions/compound-assignment/11.13.2-52-s.js 3
dir test/language/expressions/compound-assignment/11.13.2-53-s.js 3
dir test/language/expressions/compound-assignment/11.13.2-54-s.js 3
dir test/language/expressions/compound-assignment/11.13.2-55-s.js 3
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--1.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--10.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--11.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--12.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--13.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--14.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--15.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--16.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--17.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--18.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--19.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--2.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--20.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--21.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--3.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--4.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--5.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--6.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--7.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--8.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v--9.js 6
dir test/language/expressions/compound-assignment/compound-assignment-operator-calls-putvalue-lref--v-.js 6
dir test/language/expressions/conditional/coalesce-expr-ternary.js 6
dir test/language/expressions/delete/11.4.1-4.a-9-s.js 4
dir test/language/expressions/delete/11.4.1-4.a-9.js 4
dir test/language/expressions/delete/11.4.1-5-a-27-s.js 3
dir test/language/expressions/delete/11.4.1-5-a-28-s.js 5
dir test/language/expressions/delete/S11.4.1_A5.js 5
dir test/language/expressions/delete/S8.12.7_A1.js 4
dir test/language/expressions/delete/delete-unresolvable-base-object-reference-throws-typeerror.js 6
dir test/language/expressions/delete/super-property-topropertykey.js 8
dir test/language/expressions/delete/super-property-uninitialized-this.js 8
dir test/language/expressions/exponentiation/applying-the-exp-operator_A10.js 4
dir test/language/expressions/exponentiation/int32_min-exponent.js 6
dir test/language/expressions/function/arguments-with-arguments-fn.js 6
dir test/language/expressions/function/arguments-with-arguments-lex.js 6
dir test/language/expressions/function/dflt-params-trailing-comma.js 6
dir test/language/expressions/function/early-body-super-call.js 8
dir test/language/expressions/function/early-body-super-prop.js 8
dir test/language/expressions/function/early-params-super-call.js 8
dir test/language/expressions/function/early-params-super-prop.js 8
dir test/language/expressions/function/forbidden-ext/b2/ 6
dir test/language/expressions/function/named-no-strict-reassign-fn-name-in-body-in-arrow.js 6
dir test/language/expressions/function/named-no-strict-reassign-fn-name-in-body-in-eval.js 6
dir test/language/expressions/function/named-no-strict-reassign-fn-name-in-body.js 6
dir test/language/expressions/function/named-strict-error-reassign-fn-name-in-body-in-arrow.js 6
dir test/language/expressions/function/named-strict-error-reassign-fn-name-in-body-in-eval.js 6
dir test/language/expressions/function/named-strict-error-reassign-fn-name-in-body.js 6
dir test/language/expressions/function/param-eval-non-strict-is-correct-value.js 6
dir test/language/expressions/function/rest-params-trailing-comma-early-error.js 6
dir test/language/expressions/function/scope-name-var-open-non-strict.js 6
dir test/language/expressions/function/scope-name-var-open-strict.js 6
dir test/language/expressions/function/scope-param-elem-var-close.js 6
dir test/language/expressions/function/scope-param-elem-var-open.js 6
dir test/language/expressions/function/scope-param-rest-elem-var-close.js 6
dir test/language/expressions/function/scope-param-rest-elem-var-open.js 6
dir test/language/expressions/function/scope-paramsbody-var-close.js 6
dir test/language/expressions/function/scope-paramsbody-var-open.js 6
dir test/language/expressions/function/use-strict-with-non-simple-param.js 6
dir test/language/expressions/in/S8.12.6_A2_T1.js 3
dir test/language/expressions/in/S8.12.6_A2_T2.js 3
dir test/language/expressions/in/rhs-yield-present.js 7
dir test/language/expressions/logical-assignment/lgcl-and-assignment-operator-namedevaluation-arrow-function.js 6
dir test/language/expressions/logical-assignment/lgcl-and-assignment-operator-namedevaluation-class-expression.js 8
dir test/language/expressions/logical-assignment/lgcl-and-assignment-operator-non-extensible.js 3
dir test/language/expressions/logical-assignment/lgcl-and-assignment-operator.js 6
dir test/language/expressions/logical-assignment/lgcl-nullish-assignment-operator-namedevaluation-arrow-function.js 6
dir test/language/expressions/logical-assignment/lgcl-nullish-assignment-operator-namedevaluation-class-expression.js 8
dir test/language/expressions/logical-assignment/lgcl-nullish-assignment-operator-non-extensible.js 3
dir test/language/expressions/logical-assignment/lgcl-nullish-assignment-operator.js 6
dir test/language/expressions/logical-assignment/lgcl-or-assignment-operator-namedevaluation-arrow-function.js 6
dir test/language/expressions/logical-assignment/lgcl-or-assignment-operator-namedevaluation-class-expression.js 8
dir test/language/expressions/logical-assignment/lgcl-or-assignment-operator-non-extensible.js 3
dir test/language/expressions/logical-assignment/lgcl-or-assignment-operator.js 6
dir test/language/expressions/logical-not/S9.2_A6_T2.js 4
dir test/language/expressions/modulus/S11.5.3_A4_T7.js 4
dir test/language/expressions/new/non-ctor-err-realm.js 4
dir test/language/expressions/new/spread-err-mult-err-unresolvable.js 6
dir test/language/expressions/new/spread-err-sngl-err-unresolvable.js 6
dir test/language/expressions/new/spread-mult-empty.js 6
dir test/language/expressions/new/spread-mult-expr.js 6
dir test/language/expressions/new/spread-mult-literal.js 6
dir test/language/expressions/new/spread-sngl-empty.js 6
dir test/language/expressions/new/spread-sngl-expr.js 6
dir test/language/expressions/new/spread-sngl-literal.js 6
dir test/language/expressions/object/__proto__-duplicate-computed.js 6
dir test/language/expressions/object/accessor-name-computed-err-evaluation.js 6
dir test/language/expressions/object/accessor-name-computed-err-to-prop-key.js 6
dir test/language/expressions/object/accessor-name-computed-err-unresolvable.js 6
dir test/language/expressions/object/accessor-name-computed-in.js 6
dir test/language/expressions/object/accessor-name-computed-yield-id.js 6
dir test/language/expressions/object/accessor-name-computed.js 6
dir test/language/expressions/object/accessor-name-literal-numeric-binary.js 6
dir test/language/expressions/object/accessor-name-literal-numeric-exponent.js 6
dir test/language/expressions/object/accessor-name-literal-numeric-hex.js 6
dir test/language/expressions/object/accessor-name-literal-numeric-leading-decimal.js 6
dir test/language/expressions/object/accessor-name-literal-numeric-non-canonical.js 6
dir test/language/expressions/object/accessor-name-literal-numeric-octal.js 6
dir test/language/expressions/object/accessor-name-literal-numeric-zero.js 6
dir test/language/expressions/object/accessor-name-literal-string-char-escape.js 6
dir test/language/expressions/object/accessor-name-literal-string-default-escaped-ext.js 6
dir test/language/expressions/object/accessor-name-literal-string-default-escaped.js 6
dir test/language/expressions/object/accessor-name-literal-string-default.js 6
dir test/language/expressions/object/accessor-name-literal-string-double-quote.js 6
dir test/language/expressions/object/accessor-name-literal-string-empty.js 6
dir test/language/expressions/object/accessor-name-literal-string-hex-escape.js 6
dir test/language/expressions/object/accessor-name-literal-string-line-continuation.js 6
dir test/language/expressions/object/accessor-name-literal-string-single-quote.js 6
dir test/language/expressions/object/accessor-name-literal-string-unicode-escape.js 6
dir test/language/expressions/object/computed-property-evaluation-order.js 6
dir test/language/expressions/object/computed-property-name-topropertykey-before-value-evaluation.js 6
dir test/language/expressions/object/cover-initialized-name.js 6
dir test/language/expressions/object/getter-super-prop.js 8
dir test/language/expressions/object/identifier-shorthand-invalid-computed-name.js 6
dir test/language/expressions/object/method-definition/early-errors-object-async-method-duplicate-parameters.js 10
dir test/language/expressions/object/method-definition/early-errors-object-method-NSPL-with-USD.js 6
dir test/language/expressions/object/method-definition/early-errors-object-method-formals-contains-super-call.js 10
dir test/language/expressions/object/method-definition/forbidden-ext/b2/ 6
dir test/language/expressions/object/method-definition/meth-dflt-params-trailing-comma.js 6
dir test/language/expressions/object/method-definition/meth-rest-params-trailing-comma-early-error.js 6
dir test/language/expressions/object/method-definition/name-param-init-yield.js 6
dir test/language/expressions/object/method-definition/name-param-redecl.js 6
dir test/language/expressions/object/method-definition/name-prop-name-eval-error.js 6
dir test/language/expressions/object/method-definition/name-prop-name-yield-id.js 6
dir test/language/expressions/object/method-definition/name-super-call-body.js 8
dir test/language/expressions/object/method-definition/name-super-call-param.js 8
dir test/language/expressions/object/method-definition/setter-use-strict-with-non-simple-param.js 6
dir test/language/expressions/object/method-definition/use-strict-with-non-simple-param.js 6
dir test/language/expressions/object/method.js 8
dir test/language/expressions/object/prop-def-invalid-async-prefix.js 10
dir test/language/expressions/object/prop-def-invalid-star-prefix.js 7
dir test/language/expressions/object/scope-meth-param-elem-var-close.js 6
dir test/language/expressions/object/scope-meth-param-elem-var-open.js 6
dir test/language/expressions/object/scope-meth-param-rest-elem-var-close.js 6
dir test/language/expressions/object/scope-meth-param-rest-elem-var-open.js 6
dir test/language/expressions/object/scope-meth-paramsbody-var-close.js 6
dir test/language/expressions/object/scope-meth-paramsbody-var-open.js 6
dir test/language/expressions/object/scope-setter-paramsbody-var-close.js 6
dir test/language/expressions/object/scope-setter-paramsbody-var-open.js 6
dir test/language/expressions/object/setter-super-prop.js 8
dir test/language/expressions/optional-chaining/call-expression-super-no-base.js 8
dir test/language/expressions/optional-chaining/call-expression.js 8
dir test/language/expressions/optional-chaining/early-errors-tail-position-null-op-template-string-esi.js 6
dir test/language/expressions/optional-chaining/early-errors-tail-position-null-op-template-string.js 6
dir test/language/expressions/optional-chaining/early-errors-tail-position-null-optchain-template-string-esi.js 6
dir test/language/expressions/optional-chaining/early-errors-tail-position-null-optchain-template-string.js 6
dir test/language/expressions/optional-chaining/early-errors-tail-position-op-template-string-esi.js 6
dir test/language/expressions/optional-chaining/early-errors-tail-position-op-template-string.js 6
dir test/language/expressions/optional-chaining/early-errors-tail-position-optchain-template-string-esi.js 6
dir test/language/expressions/optional-chaining/early-errors-tail-position-optchain-template-string.js 6
dir test/language/expressions/optional-chaining/eval-optional-call.js 6
dir test/language/expressions/optional-chaining/iteration-statement-do.js 6
dir test/language/expressions/optional-chaining/iteration-statement-for-in.js 6
dir test/language/expressions/optional-chaining/iteration-statement-for-of-type-error.js 6
dir test/language/expressions/optional-chaining/iteration-statement-for.js 6
dir test/language/expressions/optional-chaining/iteration-statement-while.js 6
dir test/language/expressions/optional-chaining/member-expression.js 10
dir test/language/expressions/optional-chaining/new-target-optional-call.js 9
dir test/language/expressions/optional-chaining/optional-call-preserves-this.js 6
dir test/language/expressions/optional-chaining/optional-chain-expression-optional-expression.js 6
dir test/language/expressions/optional-chaining/optional-chain-prod-arguments.js 6
dir test/language/expressions/optional-chaining/optional-chain-prod-expression.js 6
dir test/language/expressions/optional-chaining/optional-chain-prod-identifiername.js 6
dir test/language/expressions/optional-chaining/optional-chain.js 6
dir test/language/expressions/optional-chaining/optional-expression.js 6
dir test/language/expressions/optional-chaining/punctuator-decimal-lookahead.js 6
dir test/language/expressions/optional-chaining/runtime-semantics-evaluation.js 6
dir test/language/expressions/optional-chaining/short-circuiting.js 6
dir test/language/expressions/optional-chaining/static-semantics-simple-assignment.js 6
dir test/language/expressions/optional-chaining/super-property-optional-call.js 8
dir test/language/expressions/optional-chaining/update-expression-postfix.js 6
dir test/language/expressions/optional-chaining/update-expression-prefix.js 6
dir test/language/expressions/postfix-decrement/operator-x-postfix-decrement-calls-putvalue-lhs-newvalue--1.js 6
dir test/language/expressions/postfix-decrement/operator-x-postfix-decrement-calls-putvalue-lhs-newvalue-.js 6
dir test/language/expressions/postfix-increment/operator-x-postfix-increment-calls-putvalue-lhs-newvalue--1.js 6
dir test/language/expressions/postfix-increment/operator-x-postfix-increment-calls-putvalue-lhs-newvalue-.js 6
dir test/language/expressions/prefix-decrement/operator-prefix-decrement-x-calls-putvalue-lhs-newvalue--1.js 6
dir test/language/expressions/prefix-decrement/operator-prefix-decrement-x-calls-putvalue-lhs-newvalue-.js 6
dir test/language/expressions/prefix-increment/operator-prefix-increment-x-calls-putvalue-lhs-newvalue--1.js 6
dir test/language/expressions/prefix-increment/operator-prefix-increment-x-calls-putvalue-lhs-newvalue-.js 6
dir test/language/expressions/property-accessors/S11.2.1_A3_T2.js 4
dir test/language/expressions/property-accessors/S11.2.1_A3_T3.js 3
dir test/language/expressions/property-accessors/S11.2.1_A4_T1.js 4
dir test/language/expressions/property-accessors/S11.2.1_A4_T2.js 3
dir test/language/expressions/property-accessors/S11.2.1_A4_T3.js 3
dir test/language/expressions/property-accessors/S11.2.1_A4_T4.js 3
dir test/language/expressions/property-accessors/S11.2.1_A4_T5.js 5
dir test/language/expressions/property-accessors/S11.2.1_A4_T8.js 4
dir test/language/expressions/property-accessors/S11.2.1_A4_T9.js 4
dir test/language/expressions/typeof/built-in-exotic-objects-no-call.js 5
dir test/language/expressions/typeof/built-in-functions.js 4
dir test/language/expressions/typeof/built-in-ordinary-objects-no-call.js 9
dir test/language/expressions/typeof/get-value.js 3
dir test/language/expressions/typeof/native-call.js 5
dir test/language/expressions/typeof/null.js 5
dir test/language/expressions/typeof/number.js 4
dir test/language/expressions/typeof/string.js 4
dir test/language/function-code/10.4.3-1-100-s.js 5
dir test/language/function-code/10.4.3-1-100gs.js 5
dir test/language/function-code/10.4.3-1-101-s.js 5
dir test/language/function-code/10.4.3-1-101gs.js 5
dir test/language/function-code/10.4.3-1-102-s.js 5
dir test/language/function-code/10.4.3-1-102gs.js 5
dir test/language/function-code/10.4.3-1-66-s.js 3
dir test/language/function-code/10.4.3-1-66gs.js 3
dir test/language/function-code/10.4.3-1-67-s.js 3
dir test/language/function-code/10.4.3-1-67gs.js 3
dir test/language/function-code/10.4.3-1-68-s.js 3
dir test/language/function-code/10.4.3-1-68gs.js 3
dir test/language/function-code/10.4.3-1-69-s.js 3
dir test/language/function-code/10.4.3-1-69gs.js 3
dir test/language/function-code/10.4.3-1-70-s.js 3
dir test/language/function-code/10.4.3-1-70gs.js 3
dir test/language/function-code/10.4.3-1-85-s.js 3
dir test/language/function-code/10.4.3-1-85gs.js 3
dir test/language/function-code/10.4.3-1-86-s.js 3
dir test/language/function-code/10.4.3-1-86gs.js 3
dir test/language/function-code/10.4.3-1-87-s.js 3
dir test/language/function-code/10.4.3-1-87gs.js 3
dir test/language/function-code/10.4.3-1-88-s.js 3
dir test/language/function-code/10.4.3-1-88gs.js 3
dir test/language/function-code/10.4.3-1-89-s.js 3
dir test/language/function-code/10.4.3-1-89gs.js 3
dir test/language/function-code/block-decl-onlystrict.js 6
dir test/language/function-code/eval-param-env-with-computed-key.js 6
dir test/language/function-code/eval-param-env-with-prop-initializer.js 6
dir test/language/function-code/switch-case-decl-onlystrict.js 6
dir test/language/function-code/switch-dflt-decl-onlystrict.js 6
dir test/language/future-reserved-words/implements.js 6
dir test/language/future-reserved-words/interface.js 6
dir test/language/future-reserved-words/package.js 6
dir test/language/future-reserved-words/private.js 6
dir test/language/future-reserved-words/protected.js 6
dir test/language/future-reserved-words/public.js 6
dir test/language/future-reserved-words/static.js 6
dir test/language/global-code/block-decl-strict.js 6
dir test/language/global-code/decl-lex-configurable-global.js 6
dir test/language/global-code/decl-lex-deletion.js 8
dir test/language/global-code/decl-lex-restricted-global.js 6
dir test/language/global-code/decl-lex.js 8
dir test/language/global-code/export.js 11
dir test/language/global-code/import.js 11
dir test/language/global-code/script-decl-func-err-non-extensible.js 3
dir test/language/global-code/script-decl-func.js 3
dir test/language/global-code/script-decl-lex-deletion.js 8
dir test/language/global-code/script-decl-lex-lex.js 8
dir test/language/global-code/script-decl-lex-restricted-global.js 6
dir test/language/global-code/script-decl-lex-var-declared-via-eval.js 6
dir test/language/global-code/script-decl-lex-var.js 6
dir test/language/global-code/script-decl-lex.js 8
dir test/language/global-code/script-decl-var-collision.js 8
dir test/language/global-code/script-decl-var-err.js 3
dir test/language/global-code/script-decl-var.js 3
dir test/language/global-code/switch-case-decl-strict.js 6
dir test/language/global-code/switch-dflt-decl-strict.js 6
dir test/language/identifiers/other_id_continue-escaped.js 5
dir test/language/identifiers/other_id_continue.js 5
dir test/language/identifiers/other_id_start-escaped.js 5
dir test/language/identifiers/other_id_start.js 5
dir test/language/identifiers/part-unicode-10.0.0-escaped.js 5
dir test/language/identifiers/part-unicode-10.0.0.js 5
dir test/language/identifiers/part-unicode-11.0.0-escaped.js 5
dir test/language/identifiers/part-unicode-11.0.0.js 5
dir test/language/identifiers/part-unicode-12.0.0-escaped.js 5
dir test/language/identifiers/part-unicode-12.0.0.js 5
dir test/language/identifiers/part-unicode-13.0.0-escaped.js 5
dir test/language/identifiers/part-unicode-13.0.0.js 5
dir test/language/identifiers/part-unicode-14.0.0-escaped.js 5
dir test/language/identifiers/part-unicode-14.0.0.js 5
dir test/language/identifiers/part-unicode-15.0.0-escaped.js 5
dir test/language/identifiers/part-unicode-15.0.0.js 5
dir test/language/identifiers/part-unicode-15.1.0-escaped.js 5
dir test/language/identifiers/part-unicode-15.1.0.js 5
dir test/language/identifiers/part-unicode-16.0.0-escaped.js 5
dir test/language/identifiers/part-unicode-16.0.0.js 5
dir test/language/identifiers/part-unicode-17.0.0-escaped.js 5
dir test/language/identifiers/part-unicode-17.0.0.js 5
dir test/language/identifiers/part-unicode-5.2.0-escaped.js 5
dir test/language/identifiers/part-unicode-5.2.0.js 5
dir test/language/identifiers/part-unicode-6.0.0-escaped.js 5
dir test/language/identifiers/part-unicode-6.0.0.js 5
dir test/language/identifiers/part-unicode-6.1.0-escaped.js 5
dir test/language/identifiers/part-unicode-6.1.0.js 5
dir test/language/identifiers/part-unicode-7.0.0-escaped.js 5
dir test/language/identifiers/part-unicode-7.0.0.js 5
dir test/language/identifiers/part-unicode-8.0.0-escaped.js 5
dir test/language/identifiers/part-unicode-8.0.0.js 5
dir test/language/identifiers/part-unicode-9.0.0-escaped.js 5
dir test/language/identifiers/part-unicode-9.0.0.js 5
dir test/language/identifiers/part-zwj-zwnj-escaped.js 5
dir test/language/identifiers/start-unicode-10.0.0-escaped.js 5
dir test/language/identifiers/start-unicode-10.0.0.js 5
dir test/language/identifiers/start-unicode-11.0.0-escaped.js 5
dir test/language/identifiers/start-unicode-11.0.0.js 5
dir test/language/identifiers/start-unicode-12.0.0-escaped.js 5
dir test/language/identifiers/start-unicode-12.0.0.js 5
dir test/language/identifiers/start-unicode-13.0.0-escaped.js 5
dir test/language/identifiers/start-unicode-13.0.0.js 5
dir test/language/identifiers/start-unicode-14.0.0-escaped.js 5
dir test/language/identifiers/start-unicode-14.0.0.js 5
dir test/language/identifiers/start-unicode-15.0.0-escaped.js 5
dir test/language/identifiers/start-unicode-15.0.0.js 5
dir test/language/identifiers/start-unicode-15.1.0-escaped.js 5
dir test/language/identifiers/start-unicode-15.1.0.js 5
dir test/language/identifiers/start-unicode-16.0.0-escaped.js 5
dir test/language/identifiers/start-unicode-16.0.0.js 5
dir test/language/identifiers/start-unicode-17.0.0-escaped.js 5
dir test/language/identifiers/start-unicode-17.0.0.js 5
dir test/language/identifiers/start-unicode-5.2.0-escaped.js 5
dir test/language/identifiers/start-unicode-5.2.0.js 5
dir test/language/identifiers/start-unicode-6.0.0-escaped.js 5
dir test/language/identifiers/start-unicode-6.0.0.js 5
dir test/language/identifiers/start-unicode-6.1.0-escaped.js 5
dir test/language/identifiers/start-unicode-6.1.0.js 5
dir test/language/identifiers/start-unicode-7.0.0-escaped.js 5
dir test/language/identifiers/start-unicode-7.0.0.js 5
dir test/language/identifiers/start-unicode-8.0.0-escaped.js 5
dir test/language/identifiers/start-unicode-8.0.0.js 5
dir test/language/identifiers/start-unicode-9.0.0-escaped.js 5
dir test/language/identifiers/start-unicode-9.0.0.js 5
dir test/language/identifiers/start-zwj-escaped.js 5
dir test/language/identifiers/start-zwnj-escaped.js 5
dir test/language/identifiers/vals-cjk-escaped.js 5
dir test/language/identifiers/vals-cjk.js 5
dir test/language/identifiers/vals-rus-alpha-lower-via-escape-hex.js 5
dir test/language/identifiers/vals-rus-alpha-lower-via-escape-hex4.js 5
dir test/language/identifiers/vals-rus-alpha-lower.js 5
dir test/language/identifiers/vals-rus-alpha-upper-via-escape-hex.js 5
dir test/language/identifiers/vals-rus-alpha-upper-via-escape-hex4.js 5
dir test/language/identifiers/vals-rus-alpha-upper.js 5
dir test/language/identifiers/vertical-tilde-continue-escaped.js 5
dir test/language/identifiers/vertical-tilde-continue.js 5
dir test/language/identifiers/vertical-tilde-start-escaped.js 5
dir test/language/identifiers/vertical-tilde-start.js 5
dir test/language/line-terminators/S7.3_A6_T3.js 5
dir test/language/line-terminators/S7.3_A6_T4.js 5
dir test/language/literals/null/S7.8.1_A1_T2.js 5
dir test/language/literals/string/S7.8.4_A4.1_T1.js 3
dir test/language/literals/string/S7.8.4_A4.1_T2.js 3
dir test/language/literals/string/S7.8.4_A4.2_T1.js 3
dir test/language/literals/string/S7.8.4_A4.2_T3.js 3
dir test/language/literals/string/S7.8.4_A4.2_T5.js 3
dir test/language/literals/string/S7.8.4_A4.2_T7.js 3
dir test/language/literals/string/S7.8.4_A5.1_T1.js 3
dir test/language/literals/string/S7.8.4_A6.1_T1.js 3
dir test/language/literals/string/S7.8.4_A6.3_T1.js 3
dir test/language/literals/string/S7.8.4_A7.1_T1.js 3
dir test/language/literals/string/S7.8.4_A7.3_T1.js 3
dir test/language/source-text/ 3
dir test/language/statementList/block-let-declaration.js 6
dir test/language/statementList/block-regexp-literal-flags.js 5
dir test/language/statementList/block-regexp-literal.js 5
dir test/language/statementList/block-with-statment-let-declaration.js 6
dir test/language/statementList/block-with-statment-regexp-literal-flags.js 5
dir test/language/statementList/block-with-statment-regexp-literal.js 5
dir test/language/statementList/eval-block-let-declaration.js 6
dir test/language/statementList/eval-block-regexp-literal-flags.js 5
dir test/language/statementList/eval-block-regexp-literal.js 5
dir test/language/statementList/eval-block-with-statment-let-declaration.js 6
dir test/language/statementList/eval-block-with-statment-regexp-literal-flags.js 5
dir test/language/statementList/eval-block-with-statment-regexp-literal.js 5
dir test/language/statementList/eval-fn-let-declaration.js 6
dir test/language/statementList/eval-fn-regexp-literal-flags.js 5
dir test/language/statementList/eval-fn-regexp-literal.js 5
dir test/language/statementList/fn-let-declaration.js 6
dir test/language/statementList/fn-regexp-literal-flags.js 5
dir test/language/statementList/fn-regexp-literal.js 5
dir test/language/statements/break/S12.8_A3.js 6
dir test/language/statements/break/S12.8_A4_T1.js 6
dir test/language/statements/break/S12.8_A4_T2.js 6
dir test/language/statements/break/S12.8_A4_T3.js 6
dir test/language/statements/break/S12.8_A5_T1.js 6
dir test/language/statements/break/S12.8_A5_T2.js 6
dir test/language/statements/break/S12.8_A5_T3.js 6
dir test/language/statements/continue/S12.7_A5_T1.js 6
dir test/language/statements/continue/S12.7_A5_T2.js 6
dir test/language/statements/continue/S12.7_A5_T3.js 6
dir test/language/statements/continue/labeled-continue.js 6
dir test/language/statements/continue/nested-let-bound-for-loops-inner-continue.js 6
dir test/language/statements/continue/nested-let-bound-for-loops-labeled-continue.js 6
dir test/language/statements/continue/nested-let-bound-for-loops-outer-continue.js 6
dir test/language/statements/continue/no-label-continue.js 6
dir test/language/statements/continue/shadowing-loop-variable-in-same-scope-as-continue.js 6
dir test/language/statements/continue/simple-and-labeled.js 6
dir test/language/statements/do-while/S12.6.1_A8.js 5
dir test/language/statements/do-while/decl-cls.js 8
dir test/language/statements/do-while/decl-const.js 6
dir test/language/statements/do-while/decl-let.js 6
dir test/language/statements/do-while/let-array-with-newline.js 6
dir test/language/statements/for-in/S12.6.4_A7_T2.js 3
dir test/language/statements/for-in/decl-cls.js 8
dir test/language/statements/for-in/decl-const.js 6
dir test/language/statements/for-in/decl-let.js 6
dir test/language/statements/for-in/head-const-bound-names-dup.js 6
dir test/language/statements/for-in/head-const-bound-names-fordecl-tdz.js 6
dir test/language/statements/for-in/head-const-bound-names-in-stmt.js 6
dir test/language/statements/for-in/head-const-bound-names-let.js 6
dir test/language/statements/for-in/head-const-fresh-binding-per-iteration.js 6
dir test/language/statements/for-in/head-decl-expr.js 6
dir test/language/statements/for-in/head-let-bound-names-dup.js 6
dir test/language/statements/for-in/head-let-bound-names-fordecl-tdz.js 6
dir test/language/statements/for-in/head-let-bound-names-in-stmt.js 6
dir test/language/statements/for-in/head-let-bound-names-let.js 6
dir test/language/statements/for-in/head-let-destructuring.js 6
dir test/language/statements/for-in/head-let-fresh-binding-per-iteration.js 6
dir test/language/statements/for-in/head-lhs-invalid-asnmt-ptrn-ary.js 6
dir test/language/statements/for-in/head-lhs-invalid-asnmt-ptrn-obj.js 6
dir test/language/statements/for-in/head-lhs-let.js 3
dir test/language/statements/for-in/head-var-bound-names-dup.js 6
dir test/language/statements/for-in/labelled-fn-stmt-const.js 6
dir test/language/statements/for-in/labelled-fn-stmt-let.js 6
dir test/language/statements/for-in/let-array-with-newline.js 6
dir test/language/statements/for-in/let-block-with-newline.js 6
dir test/language/statements/for-in/let-identifier-with-newline.js 6
dir test/language/statements/for-in/order-enumerable-shadowed.js 3
dir test/language/statements/for-in/order-property-on-prototype.js 3
dir test/language/statements/for-in/scope-head-var-none.js 6
dir test/language/statements/for/S12.6.3_A8_T3.js 6
dir test/language/statements/for/decl-cls.js 8
dir test/language/statements/for/decl-const.js 6
dir test/language/statements/for/decl-let.js 6
dir test/language/statements/for/head-const-bound-names-in-stmt.js 6
dir test/language/statements/for/head-const-fresh-binding-per-iteration.js 6
dir test/language/statements/for/head-init-async-of.js 10
dir test/language/statements/for/head-let-bound-names-in-stmt.js 6
dir test/language/statements/for/head-let-destructuring.js 6
dir test/language/statements/for/head-let-fresh-binding-per-iteration.js 6
dir test/language/statements/for/labelled-fn-stmt-const.js 6
dir test/language/statements/for/labelled-fn-stmt-let.js 6
dir test/language/statements/for/let-array-with-newline.js 6
dir test/language/statements/for/let-block-with-newline.js 6
dir test/language/statements/for/let-identifier-with-newline.js 6
dir test/language/statements/function/S13.2.1_A5_T1.js 3
dir test/language/statements/function/S13.2.1_A5_T2.js 4
dir test/language/statements/function/S13.2.2_A1_T1.js 3
dir test/language/statements/function/S13.2.2_A1_T2.js 3
dir test/language/statements/function/S13.2.2_A3_T1.js 3
dir test/language/statements/function/S13.2.2_A3_T2.js 3
dir test/language/statements/function/S13.2_A5.js 3
dir test/language/statements/function/S13_A18.js 4
dir test/language/statements/function/arguments-with-arguments-fn.js 6
dir test/language/statements/function/arguments-with-arguments-lex.js 6
dir test/language/statements/function/dflt-params-trailing-comma.js 6
dir test/language/statements/function/early-body-super-call.js 8
dir test/language/statements/function/early-body-super-prop.js 8
dir test/language/statements/function/early-params-super-call.js 8
dir test/language/statements/function/early-params-super-prop.js 8
dir test/language/statements/function/forbidden-ext/b2/ 6
dir test/language/statements/function/param-eval-non-strict-is-correct-value.js 6
dir test/language/statements/function/rest-params-trailing-comma-early-error.js 6
dir test/language/statements/function/scope-param-elem-var-close.js 6
dir test/language/statements/function/scope-param-elem-var-open.js 6
dir test/language/statements/function/scope-param-rest-elem-var-close.js 6
dir test/language/statements/function/scope-param-rest-elem-var-open.js 6
dir test/language/statements/function/scope-paramsbody-var-close.js 6
dir test/language/statements/function/scope-paramsbody-var-open.js 6
dir test/language/statements/function/use-strict-with-non-simple-param.js 6
dir test/language/statements/if/if-cls-else-cls.js 8
dir test/language/statements/if/if-cls-else-stmt.js 8
dir test/language/statements/if/if-cls-no-else.js 8
dir test/language/statements/if/if-const-else-const.js 6
dir test/language/statements/if/if-const-else-stmt.js 6
dir test/language/statements/if/if-const-no-else.js 6
dir test/language/statements/if/if-let-else-let.js 6
dir test/language/statements/if/if-let-else-stmt.js 6
dir test/language/statements/if/if-let-no-else.js 6
dir test/language/statements/if/if-stmt-else-cls.js 8
dir test/language/statements/if/if-stmt-else-const.js 6
dir test/language/statements/if/if-stmt-else-let.js 6
dir test/language/statements/if/let-array-with-newline.js 6
dir test/language/statements/if/let-block-with-newline.js 6
dir test/language/statements/if/let-identifier-with-newline.js 6
dir test/language/statements/labeled/decl-cls.js 8
dir test/language/statements/labeled/decl-const.js 6
dir test/language/statements/labeled/decl-let.js 6
dir test/language/statements/labeled/let-array-with-newline.js 6
dir test/language/statements/labeled/let-block-with-newline.js 6
dir test/language/statements/labeled/let-identifier-with-newline.js 6
dir test/language/statements/return/S12.9_A4.js 4
dir test/language/statements/switch/S12.11_A1_T4.js 4
dir test/language/statements/switch/scope-lex-async-function.js 10
dir test/language/statements/switch/scope-lex-async-generator.js 10
dir test/language/statements/switch/scope-lex-class.js 8
dir test/language/statements/switch/scope-lex-const.js 6
dir test/language/statements/switch/scope-lex-generator.js 7
dir test/language/statements/switch/syntax/redeclaration/class-name-redeclaration-attempt-with-class.js 8
dir test/language/statements/switch/syntax/redeclaration/class-name-redeclaration-attempt-with-const.js 8
dir test/language/statements/switch/syntax/redeclaration/class-name-redeclaration-attempt-with-function.js 8
dir test/language/statements/switch/syntax/redeclaration/class-name-redeclaration-attempt-with-let.js 8
dir test/language/statements/switch/syntax/redeclaration/class-name-redeclaration-attempt-with-var.js 8
dir test/language/statements/switch/syntax/redeclaration/const-name-redeclaration-attempt-with-class.js 8
dir test/language/statements/switch/syntax/redeclaration/const-name-redeclaration-attempt-with-const.js 6
dir test/language/statements/switch/syntax/redeclaration/const-name-redeclaration-attempt-with-function.js 6
dir test/language/statements/switch/syntax/redeclaration/const-name-redeclaration-attempt-with-let.js 6
dir test/language/statements/switch/syntax/redeclaration/const-name-redeclaration-attempt-with-var.js 6
dir test/language/statements/switch/syntax/redeclaration/function-name-redeclaration-attempt-with-class.js 8
dir test/language/statements/switch/syntax/redeclaration/function-name-redeclaration-attempt-with-const.js 6
dir test/language/statements/switch/syntax/redeclaration/function-name-redeclaration-attempt-with-function.js 6
dir test/language/statements/switch/syntax/redeclaration/function-name-redeclaration-attempt-with-let.js 6
dir test/language/statements/switch/syntax/redeclaration/function-name-redeclaration-attempt-with-var.js 6
dir test/language/statements/switch/syntax/redeclaration/let-name-redeclaration-attempt-with-class.js 8
dir test/language/statements/switch/syntax/redeclaration/let-name-redeclaration-attempt-with-const.js 6
dir test/language/statements/switch/syntax/redeclaration/let-name-redeclaration-attempt-with-function.js 6
dir test/language/statements/switch/syntax/redeclaration/let-name-redeclaration-attempt-with-let.js 6
dir test/language/statements/switch/syntax/redeclaration/let-name-redeclaration-attempt-with-var.js 6
dir test/language/statements/switch/syntax/redeclaration/var-name-redeclaration-attempt-with-class.js 8
dir test/language/statements/switch/syntax/redeclaration/var-name-redeclaration-attempt-with-const.js 6
dir test/language/statements/switch/syntax/redeclaration/var-name-redeclaration-attempt-with-function.js 6
dir test/language/statements/switch/syntax/redeclaration/var-name-redeclaration-attempt-with-let.js 6
dir test/language/statements/throw/S12.13_A3_T4.js 3
dir test/language/statements/try/S12.14_A17.js 4
dir test/language/statements/try/S12.14_A18_T7.js 3
dir test/language/statements/try/early-catch-duplicates.js 6
dir test/language/statements/try/early-catch-function.js 6
dir test/language/statements/try/optional-catch-binding-lexical.js 6
dir test/language/statements/try/scope-catch-param-lex-open.js 6
dir test/language/statements/try/scope-catch-param-var-none.js 6
dir test/language/statements/variable/fn-name-arrow.js 6
dir test/language/statements/while/S12.6.2_A8.js 5
dir test/language/statements/while/decl-cls.js 8
dir test/language/statements/while/decl-const.js 6
dir test/language/statements/while/decl-let.js 6
dir test/language/statements/while/let-array-with-newline.js 6
dir test/language/statements/while/let-block-with-newline.js 6
dir test/language/statements/while/let-identifier-with-newline.js 6
dir test/language/statements/with/S12.10_A1.10_T1.js 4
dir test/language/statements/with/S12.10_A1.10_T2.js 4
dir test/language/statements/with/S12.10_A1.10_T4.js 4
dir test/language/statements/with/S12.10_A1.11_T1.js 4
dir test/language/statements/with/S12.10_A1.11_T2.js 4
dir test/language/statements/with/S12.10_A1.11_T4.js 4
dir test/language/statements/with/S12.10_A1.12_T1.js 4
dir test/language/statements/with/S12.10_A1.12_T2.js 4
dir test/language/statements/with/S12.10_A1.12_T4.js 4
dir test/language/statements/with/S12.10_A1.1_T1.js 4
dir test/language/statements/with/S12.10_A1.1_T2.js 4
dir test/language/statements/with/S12.10_A1.2_T1.js 4
dir test/language/statements/with/S12.10_A1.2_T2.js 4
dir test/language/statements/with/S12.10_A1.2_T4.js 4
dir test/language/statements/with/S12.10_A1.3_T1.js 4
dir test/language/statements/with/S12.10_A1.3_T2.js 4
dir test/language/statements/with/S12.10_A1.3_T4.js 4
dir test/language/statements/with/S12.10_A1.4_T1.js 4
dir test/language/statements/with/S12.10_A1.4_T2.js 4
dir test/language/statements/with/S12.10_A1.4_T4.js 4
dir test/language/statements/with/S12.10_A1.5_T1.js 4
dir test/language/statements/with/S12.10_A1.5_T2.js 4
dir test/language/statements/with/S12.10_A1.5_T4.js 4
dir test/language/statements/with/S12.10_A1.6_T1.js 4
dir test/language/statements/with/S12.10_A1.6_T2.js 4
dir test/language/statements/with/S12.10_A1.7_T1.js 4
dir test/language/statements/with/S12.10_A1.7_T2.js 4
dir test/language/statements/with/S12.10_A1.7_T4.js 4
dir test/language/statements/with/S12.10_A1.8_T1.js 4
dir test/language/statements/with/S12.10_A1.8_T2.js 4
dir test/language/statements/with/S12.10_A1.8_T4.js 4
dir test/language/statements/with/S12.10_A1.9_T1.js 4
dir test/language/statements/with/S12.10_A1.9_T2.js 4
dir test/language/statements/with/decl-cls.js 8
dir test/language/statements/with/decl-const.js 6
dir test/language/statements/with/decl-let.js 6
dir test/language/statements/with/let-array-with-newline.js 6
dir test/language/statements/with/let-block-with-newline.js 6
dir test/language/statements/with/let-identifier-with-newline.js 6
dir test/language/types/number/S8.5_A13_T2.js 4
dir test/language/types/number/S8.5_A14_T1.js 4
dir test/language/types/number/S8.5_A14_T2.js 4
dir test/language/types/object/S8.6.1_A1.js 4
dir test/language/types/object/S8.6.2_A1.js 3
dir test/language/types/object/S8.6.2_A8.js 3
dir test/language/types/reference/8.7.2-5-s.js 3
dir test/language/white-space/S7.2_A5_T5.js 5
dir test/language/white-space/after-regular-expression-literal-carriage-return.js 5
dir test/language/white-space/after-regular-expression-literal-em-quad.js 5
dir test/language/white-space/after-regular-expression-literal-em-space.js 5
dir test/language/white-space/after-regular-expression-literal-en-quad.js 5
dir test/language/white-space/after-regular-expression-literal-en-space.js 5
dir test/language/white-space/after-regular-expression-literal-figure-space.js 5
dir test/language/white-space/after-regular-expression-literal-form-feed.js 5
dir test/language/white-space/after-regular-expression-literal-four-per-em-space.js 5
dir test/language/white-space/after-regular-expression-literal-hair-space.js 5
dir test/language/white-space/after-regular-expression-literal-ideographic-space.js 5
dir test/language/white-space/after-regular-expression-literal-line-feed.js 5
dir test/language/white-space/after-regular-expression-literal-line-separator.js 5
dir test/language/white-space/after-regular-expression-literal-medium-mathematical-space.js 5
dir test/language/white-space/after-regular-expression-literal-nbsp.js 5
dir test/language/white-space/after-regular-expression-literal-nnbsp.js 5
dir test/language/white-space/after-regular-expression-literal-ogham-space.js 5
dir test/language/white-space/after-regular-expression-literal-paragraph-separator.js 5
dir test/language/white-space/after-regular-expression-literal-punctuation-space.js 5
dir test/language/white-space/after-regular-expression-literal-six-per-em-space.js 5
dir test/language/white-space/after-regular-expression-literal-space.js 5
dir test/language/white-space/after-regular-expression-literal-tab.js 5
dir test/language/white-space/after-regular-expression-literal-thin-space.js 5
dir test/language/white-space/after-regular-expression-literal-three-per-em-space.js 5
dir test/language/white-space/after-regular-expression-literal-vertical-tab.js 5
dir test/language/white-space/after-regular-expression-literal-zwnbsp.js 5
dir test/language/white-space/mongolian-vowel-separator-eval.js 5
dir test/language/white-space/mongolian-vowel-separator.js 5
dir test/language/ 2
dir test/built-ins/ 3
dir test/built-ins/Number/ 4
dir test/built-ins/Math/ 4
dir test/built-ins/Date/ 4
dir test/built-ins/parseInt/ 4
dir test/built-ins/parseFloat/ 4
dir test/built-ins/RegExp/ 5
dir test/built-ins/RegExpStringIteratorPrototype/ 5
dir test/language/literals/regexp/ 5
dir test/built-ins/String/prototype/match/ 5
dir test/built-ins/String/prototype/matchAll/ 5
dir test/built-ins/String/prototype/replace/ 5
dir test/built-ins/String/prototype/replaceAll/ 5
dir test/built-ins/String/prototype/search/ 5
dir test/built-ins/String/prototype/split/ 5
dir test/built-ins/Symbol/ 6
dir test/built-ins/ArrayIteratorPrototype/ 6
dir test/built-ins/StringIteratorPrototype/ 6
dir test/language/statements/for-of/ 6
dir test/language/statements/let/ 6
dir test/language/statements/const/ 6
dir test/language/destructuring/ 6
dir test/language/rest-parameters/ 6
dir test/language/computed-property-names/ 6
dir test/language/expressions/arrow-function/ 6
dir test/language/expressions/template-literal/ 6
dir test/language/expressions/tagged-template/ 6
dir test/built-ins/GeneratorPrototype/ 7
dir test/built-ins/GeneratorFunction/ 7
dir test/language/expressions/generators/ 7
dir test/language/statements/generators/ 7
dir test/language/expressions/yield/ 7
dir test/language/expressions/class/ 8
dir test/language/statements/class/ 8
dir test/language/expressions/super/ 8
dir test/language/expressions/new.target/ 8
dir test/built-ins/Map/ 9
dir test/built-ins/Set/ 9
dir test/built-ins/WeakMap/ 9
dir test/built-ins/WeakSet/ 9
dir test/built-ins/MapIteratorPrototype/ 9
dir test/built-ins/SetIteratorPrototype/ 9
dir test/built-ins/Proxy/ 9
dir test/built-ins/Reflect/ 9
dir test/built-ins/Promise/ 10
dir test/built-ins/AsyncFunction/ 10
dir test/built-ins/AsyncGeneratorFunction/ 10
dir test/built-ins/AsyncGeneratorPrototype/ 10
dir test/built-ins/AsyncFromSyncIteratorPrototype/ 10
dir test/built-ins/AsyncIteratorPrototype/ 10
dir test/built-ins/Array/fromAsync/ 10
dir test/language/expressions/async-arrow-function/ 10
dir test/language/expressions/async-function/ 10
dir test/language/expressions/async-generator/ 10
dir test/language/expressions/await/ 10
dir test/language/statements/async-function/ 10
dir test/language/statements/async-generator/ 10
dir test/language/statements/for-await-of/ 10
dir test/language/module-code/ 11
dir test/language/import/ 11
dir test/language/export/ 11
dir test/language/expressions/dynamic-import/ 11
dir test/language/expressions/import.meta/ 11
dir test/built-ins/BigInt/ 12
dir test/built-ins/ArrayBuffer/ 13
dir test/built-ins/TypedArray/ 13
dir test/built-ins/TypedArrayConstructors/ 13
dir test/built-ins/DataView/ 13
dir test/built-ins/Uint8Array/ 13
dir test/built-ins/SharedArrayBuffer/ 14
dir test/built-ins/Atomics/ 14
dir test/built-ins/Iterator/ 15
dir test/built-ins/DisposableStack/ 15
dir test/built-ins/AsyncDisposableStack/ 15
dir test/built-ins/SuppressedError/ 15
dir test/language/statements/using/ 15
dir test/language/statements/await-using/ 15
dir test/built-ins/WeakRef/ 16
dir test/built-ins/FinalizationRegistry/ 16
dir test/annexB/ 18
dir test/staging/ 19
dir test/built-ins/Temporal/ 20
dir test/staging/Temporal/ 20
dir test/intl402/ 21
feature exponentiation 4
feature regexp-named-groups 5
feature regexp-lookbehind 5
feature regexp-dotall 5
feature regexp-unicode-property-escapes 5
feature regexp-v-flag 5
feature regexp-match-indices 5
feature regexp-modifiers 5
feature regexp-duplicate-named-groups 5
feature RegExp.escape 5
feature String.prototype.matchAll 5
feature arrow-function 6
feature default-parameters 6
feature rest-parameters 6
feature destructuring-binding 6
feature destructuring-assignment 6
feature computed-property-names 6
feature template 6
feature let 6
feature const 6
feature for-of 6
feature object-spread 6
feature object-rest 6
feature Symbol 6
feature Symbol.iterator 6
feature Symbol.species 6
feature Symbol.toPrimitive 6
feature Symbol.toStringTag 6
feature Symbol.unscopables 6
feature Symbol.hasInstance 6
feature Symbol.isConcatSpreadable 6
feature Symbol.match 6
feature Symbol.matchAll 6
feature Symbol.replace 6
feature Symbol.search 6
feature Symbol.split 6
feature Symbol.prototype.description 6
feature generators 7
feature class 8
feature class-fields-public 8
feature class-fields-private 8
feature class-fields-private-in 8
feature class-methods-private 8
feature class-static-methods-private 8
feature class-static-fields-public 8
feature class-static-fields-private 8
feature class-static-block 8
feature super 8
feature new.target 8
feature Map 9
feature Set 9
feature WeakMap 9
feature WeakSet 9
feature Proxy 9
feature Reflect 9
feature Reflect.construct 9
feature Reflect.set 9
feature Reflect.setPrototypeOf 9
feature proxy-missing-checks 9
feature set-methods 9
feature upsert 9
feature symbols-as-weakmap-keys 9
feature Promise 10
feature Promise.allSettled 10
feature Promise.any 10
feature Promise.prototype.finally 10
feature promise-with-resolvers 10
feature promise-try 10
feature AggregateError 10
feature async-functions 10
feature async-iteration 10
feature Array.fromAsync 10
feature Symbol.asyncIterator 10
feature dynamic-import 11
feature import-attributes 11
feature json-modules 11
feature top-level-await 11
feature import.meta 11
feature export-star-as-namespace-from-module 11
feature arbitrary-module-namespace-names 11
feature BigInt 12
feature TypedArray 13
feature ArrayBuffer 13
feature DataView 13
feature DataView.prototype.getFloat32 13
feature DataView.prototype.getFloat64 13
feature DataView.prototype.getInt16 13
feature DataView.prototype.getInt32 13
feature DataView.prototype.getInt8 13
feature DataView.prototype.getUint16 13
feature DataView.prototype.getUint32 13
feature DataView.prototype.setUint8 13
feature Int8Array 13
feature Int16Array 13
feature Int32Array 13
feature Uint8Array 13
feature Uint8ClampedArray 13
feature Uint16Array 13
feature Uint32Array 13
feature Float16Array 13
feature Float32Array 13
feature Float64Array 13
feature resizable-arraybuffer 13
feature arraybuffer-transfer 13
feature uint8array-base64 13
feature TypedArray.prototype.at 13
feature align-detached-buffer-semantics-with-web-reality 13
feature stable-typedarray-sort 13
feature SharedArrayBuffer 14
feature Atomics 14
feature Atomics.waitAsync 14
feature Atomics.pause 14
feature iterator-helpers 15
feature iterator-sequencing 15
feature iterator-chunking 15
feature iterator-includes 15
feature joint-iteration 15
feature explicit-resource-management 15
feature WeakRef 16
feature FinalizationRegistry 16
feature host-gc-required 16
feature tail-call-optimization 17
feature __proto__ 18
feature __getter__ 18
feature __setter__ 18
feature caller 18
feature IsHTMLDDA 18
feature Temporal 20
feature canonical-tz 20
feature Intl-enumeration 21
feature intl-normative-optional 21
feature Intl.DateTimeFormat-datetimestyle 21
feature Intl.DateTimeFormat-dayPeriod 21
feature Intl.DateTimeFormat-extend-timezonename 21
feature Intl.DateTimeFormat-formatRange 21
feature Intl.DateTimeFormat-fractionalSecondDigits 21
feature Intl.DisplayNames 21
feature Intl.DisplayNames-v2 21
feature Intl.DurationFormat 21
feature Intl.ListFormat 21
feature Intl.Locale 21
feature Intl.NumberFormat-unified 21
feature Intl.RelativeTimeFormat 21
feature Intl.Segmenter 21
flag async 10
flag module 11
flag CanBlockIsTrue 14
flag CanBlockIsFalse 14
exclude Intl.Era-monthcode stage3
exclude Intl.Locale-info stage3
exclude Intl.NumberFormat-v3 stage3
exclude legacy-regexp stage3
exclude ShadowRealm stage3
exclude decorators stage3
exclude source-phase-imports stage3
exclude source-phase-imports-module-source stage3
exclude import-defer stage3
exclude export-defer stage3
exclude import-text stage3
exclude immutable-arraybuffer stage3
exclude import-bytes stage3
exclude nonextensible-applies-to-private stage3
exclude await-dictionary stage3
exclude error-stack-accessor stage3
exclude Iterator.prototype.join stage3
exclude-slice 20 temporal
exclude-slice 21 intl402
exclude-include atomicsHelper.js agents
```

The 38 features with no line (`optional-chaining`, `Object.hasOwn`, `globalThis` and so on)
are small additions inside the directory's own slice.

## 13. Security

**Threat model.** The attacker controls the source text of scripts and modules (of any size and
nesting), every value they compute, every regular expression pattern and subject, and every
string passed to `eval`, `Function` and `JSON.parse`. The attacker does not control the host. The
attacker wins by reading or writing memory outside what the script may reach, by crashing or
aborting the host process, by hanging it, by exhausting its memory, or by making a host object act
on a value it should have refused.

**What the absence of unsafe code and of a JIT removes.** With `#![forbid(unsafe_code)]` in every
crate and handles that are indices into `Vec`s, no engine bug can read or write memory outside
Rust's own checks: no use-after-free, no type confusion between memory layouts, no out-of-bounds
access that is not a checked panic. With no JIT, no executable memory is ever written, so the
classes of attack that turn a type confusion into machine code do not exist. What `std` itself
does with unsafe code is outside this claim.

**What remains, and how each is tested:**

- **Logic errors in the object model**: a property attribute ignored, a frozen object changed,
  a private name reachable from outside its class, a stale handle. Tested by test262, by the
  generation check that turns a stale handle into `EngineFault`, and by a test mode that collects
  at every safe point.
- **Panics**: an `unwrap`, an index or an overflow on input the engine did not expect. Clippy's
  `unwrap_used`, `expect_used`, `panic` and `indexing_slicing` are denied in the engine crates,
  `overflow-checks = true` is set in every profile, and fuzzing counts any panic as a failure.
- **Denial of service**: an endless loop, deep recursion, exponential regular expressions, huge
  strings, `BigInt`s or arrays. One test per budget and bound, each of which must end in its
  outcome (`StepBudget`, `MemoryBudget`, `RangeError`, `Interrupted`, and for the parser's
  nesting a `SyntaxError` naming `parser_nesting`) and never in a panic, an
  abort or a hang: for example `while(1){}`, `(a+)+$` on 10,000 `a`s and a `!`, 100,000 nested
  brackets, `"x".repeat(2**30)`, `2n**(2n**40n)`.
- **A host object that trusts a value**: a DOM operation acting on a value of the wrong brand,
  or an override breaking an invariant. Brand checks in the interface, the invariant checks of
  decision 9, and tests per override.
- **Non-determinism**: a result that differs by platform or by run. The same scripts run twice,
  and on every release platform, must print the same bytes; `Math` results are compared bit for
  bit against stored values.

**Fuzzing is tooling.** A generator of programs written in the test crates with `std` only, from
a seed, run against the parser, the compiler, the regular expression engine, the number code and
the whole engine under small budgets, with any panic, abort, hang beyond the budget, or
difference between two runs of one seed as a failure. Coverage-guided fuzzing with third-party
tools may be added in a separate workspace later; it never ships.

## 14. Names and conventions

- Crates: `swell` (the engine and its host interface), `swell-unicode`, `swell-num`,
  `swell-regexp`, `swell-syntax`, `swell-compiler`, `swell-cli` (binary `swell`, the command-line
  runner), `swell-test262` (binary `swell-test262`), `swell-unicode-gen` (tool). Directories:
  `crates/<name without the swell- prefix>/`, and `crates/swell/` for the engine.
- Repository: `AGENTS.md` for agents, `README.md` for people, `CLAUDE.md` only points to
  `AGENTS.md`; `Taskfile.yml` with `task check` as the one gate; Apache-2.0 with `LICENSE` and
  `NOTICE` as surf has them; `.out/` and `.cache/` ignored by git; no third-party package in the
  root `Cargo.lock`, checked by the gate as surf's `lock-check` does.

## Open, for the operator

**A. ECMA-402 (`Intl`).** 3,404 cases; it needs locale data (CLDR), which is third-party data of
large size.
(a) Out until a surf slice needs it; ECMA-262's locale-independent fallbacks meanwhile.
(b) A minimal ECMA-402 for one locale (`en`); many intl402 cases name other locales
(`locale:` front matter), so it would stay incomplete.
(c) Full ECMA-402 with vendored CLDR data and a generator, like the Unicode tables.
Recommended: (a). Real pages call `Intl.NumberFormat` and `Intl.DateTimeFormat`, so (c) will come;
its design belongs after slice 19.

**B. Concurrent agents (Web Workers) and the 112 `$262.agent` cases.**
(a) Never; `Atomics` stays incomplete.
(b) When surf needs workers: each agent is its own engine on a host thread, with shared
memory as `Arc<[AtomicU8]>`-style storage and `Atomics.wait` on `std::sync::Condvar`, all safe
Rust (not built or measured).
Recommended: (b), not before surf plans workers.

**C. The Unicode version.**
(a) 17.0.0, the version test262's generated tests use, moved when test262 moves. This deviates
from "the latest version of the Unicode Standard"; the deviation is listed as an omission of every
section that depends on it (identifiers, property escapes, case mapping, normalization), so none
of them can be marked complete until the pin reaches the latest version.
(b) 18.0.0, the latest; then test262's 17.0.0 tests fail for every code point whose properties
changed.
Recommended: (a).

**D. Where surf runs the engine.** surf's rule is that a component is an isolation boundary.
(a) The engine and the DOM in one process per page (or per site), apart from the network and the
other components.
(b) The engine in its own process, the DOM in another, every DOM access a message.
Recommended: (a): a script touches the DOM on most statements, and a process boundary on each
access would cost more than it isolates (not measured). This is surf's decision; swell's interface
works for both.

## Not in scope

- `Temporal` (slice 20), ECMA-402 (slice 21, open item A), stage 3 features, concurrent agents.
- A JIT, any machine code, any executable memory.
- Shapes, ropes, NaN-boxing and a register machine, until a measurement asks for them.
- Source maps, a debugger protocol, `Error.prototype.stack` (stage 3, excluded), profiling.
- Web IDL's conversions and the DOM: surf's.

## Not checked

- Whether ECMA-262 words an allowance for a `RangeError` when a `BigInt` result is too large, or
  for an implementation limit on pattern nesting.
- The licence text of the Unicode data, and where the 17.0.0 emoji sequence files are published.
- The Rust stack one re-entry needs (the size of `Value` is measured: 16 bytes, held by the
  test `value_is_16_bytes` of `crates/swell`), the time test262
  takes, and every speed cost: no code exists.
- The published ES2026 edition was not read.
- How well ESS states an in-process library interface.

## How the counts were made

A correction, measured on 2026-11-21 by the unit that wrote the runner: one case,
`test/built-ins/Function/prototype/toString/line-terminator-normalisation-CR.js`, ends its lines
with CR alone. Run as below, `meta.awk` splits at LF only, reads that file as one line, finds no
end of the block and prints the case with every field empty, losing its `includes:
[nativeFunctionMatcher.js]`. Run with `awk -v RS='\r\n|\r|\n' -f meta.awk` it reads the case;
exactly that one line of the 53,801 changes, and no count of this document changes (the case
has no flag and no feature). The runner reads CR LF, CR and LF each as a line end.

`meta.awk` (run as in "Inventory"):

```awk
# One line per test262 file: path, flags, features, negative phase, negative type,
# includes, separated by tabs; lists joined by commas. Reads the /*--- ---*/ front
# matter; handles inline [a, b] lists and block "- a" lists.
function trim(s) { gsub(/^[ \t]+|[ \t]+$/, "", s); return s }
function inl(s) {
  sub(/^[^\[]*\[/, "", s); sub(/\].*$/, "", s); gsub(/[ \t]/, "", s); return s
}
function flush() {
  if (path == "") return
  printf "%s\t%s\t%s\t%s\t%s\t%s\n", path, v["flags"], v["features"],
    v["phase"], v["type"], v["includes"]
}
FNR == 1 {
  flush(); path = FILENAME; inmeta = 0; done = 0; lkey = ""
  v["flags"] = v["features"] = v["phase"] = v["type"] = v["includes"] = ""
}
done { next }
/\/\*---/ { inmeta = 1; next }
inmeta && /---\*\// { inmeta = 0; done = 1; next }
!inmeta { next }
match($0, /^[a-zA-Z_]+:/) {
  key = substr($0, 1, RLENGTH - 1); rest = trim(substr($0, RLENGTH + 1)); lkey = ""
  if (key == "flags" || key == "features" || key == "includes") {
    if (rest ~ /\[/) v[key] = inl(rest)
    else if (rest != "" && rest !~ /^[|>]/) v[key] = rest
    else lkey = key
  } else if (key == "negative") lkey = "negative"
  next
}
lkey == "negative" {
  if (match($0, /phase:/)) v["phase"] = trim(substr($0, RSTART + 6))
  if (match($0, /type:/)) v["type"] = trim(substr($0, RSTART + 5))
  next
}
lkey != "" && match($0, /^[ \t]*-[ \t]*/) {
  item = trim(substr($0, RLENGTH + 1))
  v[lkey] = (v[lkey] == "") ? item : v[lkey] "," item
}
END { flush() }
```

`slices.awk`:

```awk
# Usage: awk -f slices.awk <rules> <meta.tsv>
# Rules, one per line: "dir <prefix> <slice>", "feature <name> <slice>",
# "flag <name> <slice>", "exclude <feature> <reason>". A case's slice is the highest
# slice among its longest matching directory prefix, its features and its flags; a case
# with an excluded feature gets "x-<reason>". Prints: slice, path, flags (tab-separated).
FNR == NR {
  if ($1 == "dir") dir[$2] = $3 + 0
  else if ($1 == "feature") feat[$2] = $3 + 0
  else if ($1 == "flag") flag[$2] = $3 + 0
  else if ($1 == "exclude") excl[$2] = $3
  next
}
{
  split($0, f, "\t"); path = f[1]; best = -1; slice = 0; ex = ""
  for (p in dir)
    if (index(path, p) == 1 && length(p) > best) { best = length(p); slice = dir[p] }
  n = split(f[3], fs, ",")
  for (i = 1; i <= n; i++) {
    if (fs[i] in excl) ex = "x-" excl[fs[i]]
    else if ((fs[i] in feat) && feat[fs[i]] > slice) slice = feat[fs[i]]
  }
  n = split(f[2], gs, ",")
  for (i = 1; i <= n; i++) if ((gs[i] in flag) && flag[gs[i]] > slice) slice = flag[gs[i]]
  printf "%s\t%s\t%s\n", (ex != "" ? ex : sprintf("%02d", slice)), path, f[2]
}
```

The check, from the directory holding `meta.tsv` and the two scripts, with `note` set to this
document:

```sh
awk '$0 == "```swell-slices" {f = 1; next} /^```$/ {f = 0} f' "$note" > rules.txt
awk -f slices.awk rules.txt meta.tsv | awk -F'\t' '{c[$1]++
  r[$1] += ($3 ~ /onlyStrict|noStrict|module|raw/) ? 1 : 2}
  END {for (k in c) print k, c[k], r[k]}' | sort
```

Its output on this document:

```
01 31 62
02 4259 7543
03 7635 15074
04 1323 2606
05 2254 4503
06 4204 7952
07 1473 2817
08 6273 12453
09 1752 3475
10 6647 12979
11 1281 1833
12 317 634
13 3044 6044
14 598 1192
15 1094 2170
16 82 164
17 35 35
18 1155 1492
19 1474 2766
20 4617 9234
21 1662 3324
x-stage3 2591 5008
```

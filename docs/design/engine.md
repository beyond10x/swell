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
  over UTF-16 (decision 7). Depends on `swell-unicode` and nothing else: it names no type of
  `swell-syntax` or `swell` and takes its budget from its caller (decision 7, "How
  `swell-regexp` charges its work").
- `swell-syntax`: lexer; parser with cover grammars, early errors, strict mode, automatic
  semicolon insertion, and regular expression literals checked by `swell-regexp`; the syntax
  tree; scope analysis. Depends on `swell-unicode` and, from `story:regexp-literal-early-errors`
  of slice 5, `swell-regexp`. It does not depend on `swell-num` (corrected on 2026-11-23): the
  lexer keeps a numeric literal as its text, and the compile step converts it.
- `swell-compiler`: the bytecode format and the compile step from the analysed tree. Depends on
  `swell-syntax` and `swell-num`: the value of a numeric literal
  (`swell_num::literal::numeric_literal`) and a number written as a property key
  (`swell_num::to_string::number_to_string`).
- `swell`: the engine and its public host interface: values, heap and collector, interpreter,
  realms, jobs, modules, and the builtins, one module per ECMA-262 clause (19 to 28, then
  Annex B). Depends on `swell-compiler`, `swell-syntax`, `swell-num` and `swell-unicode`, and,
  from `story:regexp-objects-exec-and-literals` of slice 5, `swell-regexp`: all of the above.
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
part before its slice"), which for `swell-regexp` is slice 5: its first story,
`story:regexp-crate-and-pattern-parser`, creates it, and eight of the nine exist from then.

Why: each crate below `swell` holds no heap value and can be tested and fuzzed alone (the parser
on source text, the matcher on a pattern and a string, the number code on bit patterns), and the
dependency graph has no cycle. The builtins stay inside `swell` because every one of them reads
and writes the heap; one module per clause keeps the mapping to the standard, and to the test262
directories of decision 11, one to one.

Rejected: one crate for everything (no part can be fuzzed without the heap); a crate per builtin
clause (each needs the heap's private interface, so it would have to be public).

Every crate has `#![forbid(unsafe_code)]`, and the root `Cargo.lock` lists no package from outside
the repository (decision 14).

**The engine core has no `std`.** Decided by the operator on 2026-11-26
(`story:core-builds-without-std`): the six library crates are `#![no_std]` with
`extern crate alloc`, and their library code names `core::` and `alloc::` paths only; their unit
tests link `std`. They name no allocator: the program that links them provides it, the shell the
system's and an embedder its own. What comes from the system comes through the host trait, and an
adapter is whatever implements it: with system access (the shell), without, or emulated (a clock
that stands still, fixed seeds; modules from memory once slice 11 delivers loading)
(`host-interface.md`, decision 1, "Adapters"). The repository's emulated host is a test host in
the integration-test binary of `swell`, a `std` program, that calls no system API. No host here is
built without `std`: a program that runs the engine without `std` needs a global allocator, and
one over a fixed block needs `unsafe impl GlobalAlloc`, which `#![forbid(unsafe_code)]` rules
out; a host library without `std` would need neither an allocator nor an entry point, and would
run nothing. The three binary crates stay `std` programs. In place of such a host the gate builds
`swell` and every crate it depends on for `x86_64-unknown-none`, a target without `std` (`task
nostd-build`), refuses an undefined symbol in them that neither Rust nor the compiler's runtime
supplies, a C library's `fmod` or `sqrt` among them (`task nostd-symbols`,
`taskfiles/nostd.yml`), and runs the emulated host in the `std` test binary; HOST-2's source scan
refuses a `std` path and a crate root without `#![no_std]`.

## 3. Execution model

**A stack-based bytecode interpreter, with every ECMAScript frame on the engine's own frame stack,
not on the Rust stack.**

- Bytecode against walking the tree: bytecode. A tree walker recurses in Rust for every call, so a
  generator could only be suspended by keeping the Rust stack alive (a thread, or unsafe stack
  switching), which the rules exclude. With frames as data (instruction index, operand stack,
  local slots, the environments the frame pushed; the handler table stays with the code the
  frame runs), `yield` and `await` move the frame into the object that holds the suspended
  activation (a generator object, the activation of an async function) and a resumption moves it
  back ("Suspended frames" below). Rejected: the tree walker.
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
   steps 15 to 22), whether or not its own code names `arguments`. A direct `eval` in an arrow
   function, at any depth of arrows, counts for the nearest enclosing function that is not an
   arrow: an arrow never has an arguments object (step 16), and the eval code's `arguments`
   is that function's ("`this` and `arguments` in an arrow function" below).
5. Budgets. A lookup by name charges one step per Environment Record it visits, so that a read
   through n records spends at least n steps more than a slot read (`last_entry_steps()`);
   the HasProperty and the `@@unscopables` Get of an Object Environment Record are property
   operations charged and checked as any other, and a getter or `Proxy` trap they reach is a
   call counted like any call. The names of a record's slots are part of the compiled code and
   are charged to the memory budget with it; a binding that eval code adds at run time is an
   allocation charged as any other.

The results are the standard's and test262 holds them; which form a reference takes is held by
the compiler's source tests, whose disassembly shows a slot or a lookup by name.

**Lexical bindings.** Added on 2026-11-22 for slice 6. What a script sees, the standard's:

- A `let` or `const` binding exists from the entry of its scope and is uninitialized until its
  declaration is evaluated (`let x;` initializes it to `undefined`,
  `sec-let-and-const-declarations-runtime-semantics-evaluation`). The scopes: a Block
  (`sec-block-runtime-semantics-evaluation`, BlockDeclarationInstantiation
  `sec-blockdeclarationinstantiation`); the CaseBlock of a `switch`, one scope for all its
  clauses, entered after the discriminant (`sec-switch-statement-runtime-semantics-evaluation`);
  the head of `for (;;)` (`sec-runtime-semantics-forloopevaluation`); the head of a `for`-`in`
  (and later `for`-`of`), whose expression is evaluated in a scope of its own that binds the
  head's names uninitialized for good, and whose body gets new bindings each iteration
  (`sec-runtime-semantics-forinofheadevaluation`, step 2;
  `sec-runtime-semantics-forinofloopevaluation`); the top level of a function body
  (`sec-functiondeclarationinstantiation`, steps 31 to 35); the script (the global
  environment's [[DeclarativeRecord]], below); eval code (below). A `catch` Block is a Block
  inside the scope of the `catch` parameter.
- Reading an uninitialized binding, writing it (`=`, a compound or logical assignment, `++`,
  `--`) and `typeof` of it throw a `ReferenceError`
  (`sec-declarative-environment-records-getbindingvalue-n-s`, step 2;
  `sec-declarative-environment-records-setmutablebinding-n-v-s`, step 3;
  `sec-typeof-operator-runtime-semantics-evaluation`). A write throws when the value is
  stored, after its right side was evaluated. `delete x` of a lexical binding (non-strict code
  only) answers `false` and checks nothing (`sec-declarative-environment-records-deletebinding-n`).
- Every assignment to a `const` binding throws a `TypeError`, in strict and in non-strict code:
  it is an immutable binding created strict (CreateImmutableBinding(N, true),
  `sec-declarative-environment-records-createimmutablebinding-n-s`;
  `sec-declarative-environment-records-setmutablebinding-n-v-s`, steps 2 and 5). The check of
  step 3 comes first: a write to a `const` before its declaration is the `ReferenceError`.
- `for (let ...; ...; ...)` gives each iteration a copy of the head's `let` bindings, made
  before the first test and after each body, before the update
  (`sec-forbodyevaluation`, `sec-createperiterationenvironment`), so a closure made in the body
  of one iteration keeps that iteration's value; `const` in that head has one binding.
- A script's lexical bindings are the global environment's: no property of the global object,
  seen by name by every later script of the realm. GlobalDeclarationInstantiation
  (`sec-globaldeclarationinstantiation`) throws a `SyntaxError` before it makes any binding of
  the script when one of its lexical names is already a lexical binding of the realm or a
  restricted global property (an own non-configurable property of the global object, such as
  a `var` of an earlier script or `undefined`, `sec-hasrestrictedglobalproperty`; step 3), or
  one of its `var` or function names is a lexical binding of the realm (step 4). A lexical
  name over a configurable property of the global object is allowed and shadows it. A binding
  whose script threw before its declaration ran stays uninitialized for the realm's life
  (HOST-156 holds the case of one script evaluated twice).
- Eval code's lexical bindings live in the declarative record PerformEval makes, direct or
  indirect, strict or not (`sec-performeval`, steps 17 to 19), and end with the eval code.
  Non-strict eval code whose `var` or function name is a lexical binding of the global
  environment, or of a record between the eval's LexicalEnvironment and its
  VariableEnvironment, throws a `SyntaxError` before it binds anything
  (`sec-evaldeclarationinstantiation`, step 3); a `catch` parameter's record lets it pass
  (B.3.4, applied since slice 2).

The mechanism, in the form of "Names":

1. Every scope above but the script's is a scope of the compiler (`resolve.rs`), as a function
   and a `catch` clause are. A binding of it is a local slot of its function's frame unless a
   nested function (an arrow included) refers to it or a lookup by name can reach it (item 3
   of "Names"); such a binding is a slot of a declarative record the code makes at the entry of
   the scope and leaves on every exit, abrupt ones included (`PopEnv` on a `break`, `continue`
   or `return` through it, a handler's `envs` on a throw, as for a `catch` clause today). A
   scope with no such binding makes no record. The top level of a function body is a scope of
   its own inside the function's scope, never slots of the function environment, so that
   step 3 of EvalDeclarationInstantiation finds it between the two environments (FDI step 32);
   for strict code the separation is not observable and is kept anyway.
2. The uninitialized state is a state of the slot, never a `Value`: a local slot and a slot of
   a declarative record hold either a `Value` or "uninitialized", in a type of the engine that
   no host interface and no builtin receives. An instruction that reads or writes a binding
   that may be uninitialized there is an instruction of its own kind that checks and accesses
   in one, so the check spends no step beyond the access (one unit per instruction). The
   compiler leaves the check out only for an access in the same function's code that follows
   the end of the declaration in the StatementList of the same Block, function body or script,
   never in a CaseBlock (a jump to a later clause passes over a declaration). Each entry of a
   scope sets its local slots that are checked anywhere to "uninitialized", and a record's
   lexical slots start in that state; a declaration's evaluation is InitializeBinding
   (`sec-declarative-environment-records-initializebinding-n-v`). The names table of a record
   (`SlotNames`) marks each `const` slot immutable and strict, so that a write through a
   lookup by name throws the same `TypeError`.
3. `for (let ...)`: when a `let` binding of the head lives in a record (item 1), each
   iteration makes a new record of the same table whose record slots are copies of the last
   one's, CreatePerIterationEnvironment, and makes it the frame's LexicalEnvironment in place
   of the last; head bindings in local slots are copied by nothing, since without a closure or
   a lookup by name no code can tell one slot from one per iteration. A `for`-`in` with `let`
   or `const` makes the record of an iteration under the same rule, and the head expression's
   scope of uninitialized names under the same rule.
4. The [[DeclarativeRecord]] of a realm's global environment is a declarative record of the
   `environments` arena whose bindings are kept by name, each with its state and whether it is
   `const`, found through a keyed map (decision 4) and never deleted; the caller of CreateRealm
   makes it once the realm record is stored, and `RealmGlobalEnv` holds and traces it
   (`env/global.rs`, "What slice 6 adds"). A script's lexical names are therefore global names
   of its code, not slots: GlobalDeclarationInstantiation creates them uninitialized (step
   15), the declaration initializes them, and the global name instructions, and a lookup by
   name that reaches the global environment, ask this record first and the global object only
   when it has no such binding (`sec-global-environment-records-hasbinding-n` and the other
   concrete methods). Rejected: slots for them (a later script, compiled apart, finds them by
   name).
5. Eval code's lexical declarations are a scope of the eval code under item 1: its record is
   the one of PerformEval, step 17 or 18, made when one of them lives in a record and left out
   otherwise (a record with no binding in it changes nothing a script can see). Strict eval
   code makes that record always, being its VariableEnvironment too (step 19), and its record
   slots are its lexical bindings beside the `var` bindings it adds by name. Step 3 of
   EvalDeclarationInstantiation walks the records from the LexicalEnvironment of the call to
   its VariableEnvironment, passing a `catch` parameter's and a `with` statement's; item 3 of
   "Names" makes every lexical binding on that way a named slot it can ask.
6. Budgets. A record of a block, a head, an iteration or eval code is an allocation charged as
   any record (`new_environment`); a binding of the global [[DeclarativeRecord]] is charged
   when GlobalDeclarationInstantiation creates it and is credited with the realm. The copy of
   CreatePerIterationEnvironment charges one step per binding it copies, beside its
   instruction's. The check of item 2 is no step of its own, nor is the global record's map
   lookup inside a global name instruction; a lookup by name still charges one step per record
   it visits, the global environment counting as one. Every check of GlobalDeclarationInstantiation
   and EvalDeclarationInstantiation is a step, as today.

**`this` and `arguments` in an arrow function.** Added on 2026-11-22 for slice 6. What a script
sees, the standard's: an arrow function has no `this`, `arguments`, `super` or `new.target` of
its own; in its parameters and body each means what it means in the code around the arrow
(`sec-arrow-function-definitions-runtime-semantics-evaluation`,
`sec-runtime-semantics-instantiatearrowfunctionexpression`). OrdinaryCallBindThis does nothing
for it (`sec-ordinarycallbindthis`, step 2: [[ThisMode]] lexical), so the `this` argument of
`call`, `apply` or `bind` is ignored; GetThisEnvironment passes over its function environment
(`sec-getthisenvironment`), FunctionDeclarationInstantiation makes it no arguments object
(step 16), and it has no [[Construct]] (`new` throws the `TypeError` of a value that is not a
constructor) and no `prototype`. A direct `eval` in an arrow sees the same `this` and
`arguments`; its `var` names go to the arrow's own VariableEnvironment.

The mechanism:

1. `this` is taken when the arrow is made. The closure instruction of an arrow stores the
   `this` value of the frame that makes it in the function object, which the collector traces;
   a call of the arrow puts that value in its frame's `this` and runs no OrdinaryCallBindThis.
   The `This` instruction and the `this` a direct `eval` takes from its caller's frame
   (`dynamic_code.rs`) stay as they are, so an arrow in an arrow, and eval code in an arrow,
   get it too. This equals the standard while every function's `this` binding is initialized
   before its code runs and never changes afterwards (OrdinaryCallBindThis runs before
   FunctionDeclarationInstantiation; BindThisValue, `sec-bindthisvalue`, throws on a second
   binding), which holds for every function of slices 6, 7 and 10.
2. `arguments` resolves outward like any name. The scope analysis counts an arrow's references
   to `arguments` and its direct `eval` (item 4 of "Names") for the nearest enclosing function
   that is not an arrow, which then makes its arguments object when FunctionDeclarationInstantiation
   does; the reference resolves to that function's binding `arguments`, a slot of its
   function environment, since another function's code uses it. An arrow that has a parameter
   or a declaration named `arguments` binds that name itself, as any name; at the top of a
   script the reference is a global name.
3. Rejected for now: `this` as a binding of the enclosing function's environment, read by the
   arrow through a slot as a captured name is. It gives every function whose arrows read
   `this` a function environment per call, an allocation where the copy costs one value per
   arrow, and before slice 8 no script can tell the two apart: a method that makes an arrow
   and is called with another `this` through `call`, an arrow kept after its function returned,
   eval code in an arrow, all give the same value in both. What tells them apart comes with
   the derived constructors of slice 8, whose `this` is uninitialized until `super()` returns
   (`sec-super-keyword-runtime-semantics-evaluation`): in `constructor() { const f = () =>
   this; super(); return f(); }` the call must answer the new object, where a copy made
   before `super()` has nothing to copy; in `constructor() { const f = () => this; f(); }` it
   must throw a `ReferenceError`; and in `constructor() { const s = () => super(); s(); }` the
   `super()` inside the arrow must initialize the constructor's binding, which a copy cannot.
   Slice 8 therefore changes item 1 for the arrows of a derived constructor only: there `this`
   becomes a slot of the constructor's function environment, uninitialized until `super()`
   returns (the state of item 2 of "Lexical bindings"), which those arrows read and a `super()`
   in one of them initializes, in place of a value copied when the arrow is made.

**A parameter list that is not simple.** Added on 2026-11-22 for slice 6. What a script sees, the
standard's (`sec-functiondeclarationinstantiation`, steps 19 to 30;
IsSimpleParameterList and ContainsExpression, `sec-static-semantics-issimpleparameterlist`,
`sec-static-semantics-containsexpression`): a list with a default, a rest parameter or a
pattern binds its parameters left to right, each uninitialized until its turn; a default is
evaluated only when its argument is `undefined` or missing, and may read an earlier parameter,
while reading the parameter itself or a later one throws the `ReferenceError` of an
uninitialized binding; a rest parameter is a new Array of the remaining arguments. The
parameter expressions see `this`, the function's `arguments`, the function's own name and the
scopes around the function, and none of the body's declarations: a closure made in a default
reads the parameter of a name, not the body's `var` of that name, which is a binding of its own
that starts with the parameter's value (step 30). A direct `eval` in a parameter expression of
non-strict code puts its `var` names in a record outside the parameters (step 20), so that one
of a parameter's name, or `arguments`, is the `SyntaxError` of EvalDeclarationInstantiation,
step 3. The function's arguments object is unmapped (step 22, CreateUnmappedArgumentsObject,
`sec-createunmappedargumentsobject`), also in non-strict code, and none is made when a
parameter is named `arguments` (step 17) or, in a list without an expression, when the body
declares a function or a lexical binding of that name (step 18). `length` is
ExpectedArgumentCount, the parameters before the first default or rest parameter
(`sec-static-semantics-expectedargumentcount`, `sec-setfunctionlength`).

The mechanism:

1. The function's code binds the parameters, not the call. A call of such a function binds
   `this`, makes the function environment as today, keeps its argument list in the frame
   (charged to the memory budget with the frame and traced with its locals), stores the
   arguments object in its slot when the template asks for one, and starts the code, which
   initializes each parameter in order from the kept list (IteratorBindingInitialization,
   `sec-runtime-semantics-iteratorbindinginitialization`, over a list iterator that no script
   can reach), evaluating a default with NamedEvaluation for an anonymous function. A
   parameter is a slot by the rules above, uninitialized until its turn (item 2 of "Lexical
   bindings"). A simple list keeps the call's binding of today.
2. Scopes. Such a function has a scope for its parameters and `arguments` and, when the list
   has an expression, a second one for the body's `var` names and function declarations
   (step 30), each a local slot or a record slot by item 1 of "Lexical bindings", the body's
   top-level lexical scope inside it. A record of the second scope is pushed by the code once
   the last parameter is bound, and becomes the frame's VariableEnvironment. When a direct
   `eval` stands in a parameter expression of non-strict code, the parameters' scope lives in
   a record pushed inside the function environment, which stays the VariableEnvironment while
   the parameters are bound (step 20); otherwise the parameters are slots of the function
   environment or locals, which no code can tell from step 20's two records. For item 1 of
   "Names", the scope a direct `eval` adds `var` bindings to is the function scope for an
   `eval` in a parameter expression and the body's `var` scope for one in the body; a
   reference that crosses it is looked up by name.
3. Budgets. Binding the parameters is the code's instructions, one step each; the rest Array
   charges a step per element and its allocation (`create_array_from_list`); the records of
   item 2 are charged as any record.

**Function declarations in a block.** Added on 2026-11-22 for slice 6. What a script sees, the
standard's (BlockDeclarationInstantiation, `sec-blockdeclarationinstantiation`): a function
declared in a Block or in the CaseBlock of a `switch` is a lexical binding of that scope, not a
`var` of the function, the script or the eval code. Each entry of the scope makes a new function
object for it, whose [[Environment]] is the scope's environment, and initializes the binding
before any statement of the scope runs (in a `switch` after the discriminant, before the first
clause's test): it can be called above its declaration, each iteration of a loop body gets
another object, and every clause of a `switch` sees the same one. After the scope the name means
what it meant around it, a `ReferenceError` when nothing binds it. A name bound twice in one
Block or CaseBlock, by two functions, or by a function and a `let` or `const`, or by a function
and a `var` declared in it at any depth, is a `SyntaxError` in strict and in non-strict code
(`sec-block-static-semantics-early-errors`, `sec-switch-statement-static-semantics-early-errors`).
The evaluation of the declaration does nothing. A function declared at the top level of a function
body, a script or eval code stays a `var`-scoped declaration, as in slice 1.

Non-strict code until slice 18: as strict code. Annex B, B.3.2
(`sec-block-level-function-declarations-web-legacy-compatibility-semantics`), which a web browser
host implements, gives a block function of non-strict code also a `var` binding of the enclosing
function, script or eval code, assigned when the declaration is evaluated, and lets one block
declare a function twice; it comes with slice 18, with B.3.3 (a function declaration as the
clause of an `if`, `sec-functiondeclarations-in-ifstatement-statement-clauses`), which stays
refused until then. It is a recorded omission: `conformance/test262/sections.txt` gets the section
`sec-block-level-function-declarations-web-legacy-compatibility-semantics` with the paths
`test/annexB/language/function-code/`, `test/annexB/language/global-code/` and
`test/annexB/language/eval-code/`, and the line `omit
sec-block-level-function-declarations-web-legacy-compatibility-semantics
var-binding-of-block-functions section-12-slice-18`, written with the block functions and removed
by slice 18. Why not now (counted on the pinned checkout from the front matter, with no run):
test262 holds the main body outside `test/annexB/`, where the 12 cases whose `esid` is a clause of
B.3.2 or B.3.3 are `onlyStrict` (9) or `noStrict` with strict eval code (3), and expect the answer
of a host without B.3.2; the 776 cases of those clauses are under `test/annexB/`, counted in slice
18 (`dir test/annexB/ 18`), 480 of them B.3.3's; and B.3.2 changes FunctionDeclarationInstantiation,
GlobalDeclarationInstantiation and EvalDeclarationInstantiation
(`sec-web-compat-functiondeclarationinstantiation`, `sec-web-compat-globaldeclarationinstantiation`,
`sec-web-compat-evaldeclarationinstantiation`), so it would raise no count of slice 6. Not
measured: whether a case outside `test/annexB/` that runs non-strict reads a block function after
its block; the run of the slice shows it.

The mechanism, in the terms of "Lexical bindings":

1. A block function is a binding of its Block's or CaseBlock's scope under item 1 of "Lexical
   bindings": a local slot, or a slot of the scope's record when a nested function refers to it
   (the function itself, when it calls itself by name) or a lookup by name can reach it. A scope
   whose only declarations are functions makes a record by the same rule. At the entry of the
   scope, once its record (if any) is the LexicalEnvironment and before its first statement, the
   code makes each function object with the closure instruction, in source order, and initializes
   its binding (InitializeBinding); its [[Environment]] is that LexicalEnvironment, the scope's
   record or, when the scope makes none, the one around it.
2. No access to a block function is checked (item 2 of "Lexical bindings"): no code of the scope
   runs before every one of them is initialized.
3. In eval code a function of a Block belongs to that Block's scope, never to the record of
   PerformEval (item 5 of "Lexical bindings"), whose lexical slots are the eval code's top-level
   lexical declarations only; a top-level function of eval code stays a `var` declaration of
   EvalDeclarationInstantiation.
4. What slice 18 adds, so that nothing here is undone: the `var` binding, made by the three
   instantiations as a `var` is, and at the position of the declaration in its statement list an
   instruction that copies the block binding's value to it (the Normative Optional steps of the
   three, "When the FunctionDeclaration funcDecl is evaluated, perform the following steps in
   place of the FunctionDeclaration Evaluation algorithm"); the duplicate declarations of
   non-strict code (the Normative Optional steps of BlockDeclarationInstantiation).
   The block binding stays as it is, and the position of the declaration emits nothing before
   slice 18. A test that a block function of non-strict code is not visible after its block
   changes its answer then.
5. Budgets. Each function object is the closure instruction's allocation and step; a record is
   charged by item 6 of "Lexical bindings".

**Iterators held by a builtin.** Added on 2026-11-22 for slice 6. What a script sees, the
standard's: a builtin that iterates (in slice 6 `Array.from`, `Object.fromEntries` and
`Object.groupBy`) gets an Iterator Record by GetIterator (`sec-getiterator`,
`sec-getiteratorfrommethod`), the iterator and its `next` method read once (`sec-iterator-records`),
and steps it by IteratorStepValue (`sec-iteratorstepvalue`). A throw of `next`, of reading `done`
or `value`, and the `TypeError` of a result that is no object end the builtin with that throw,
and the iterator is not closed. An abrupt completion of the builtin's own work between two steps
(a callback or a conversion that throws, a property it cannot define) closes the iterator first
(IfAbruptCloseIterator, `sec-ifabruptcloseiterator`; IteratorClose, `sec-iteratorclose`): `return`
is called when it is not undefined, and the first throw is what the builtin throws, whatever
`return` throws or answers. Every `next` and `return` is a call that may run any code, so an
iterator that is never done ends at the step budget, and a `next` that iterates again in itself
ends at a depth bound. A termination closes nothing: no code runs after it, `return` included.
A termination leaves what a builtin had changed until then, with one exception:
ArraySetLength puts the old length back when a termination ends its deletions, so that an
Array's `length` is never below an index key it still has (`object/array.rs`).

The mechanism:

1. Rust code holds the record as a value of an engine type, `IteratorRecord` ([[Iterator]],
   [[NextMethod]], [[Done]]; `crates/swell/src/ops/iteration.rs`), handles only. A builtin keeps
   [[Iterator]] and [[NextMethod]] on the agent's kept stack (`Agent::keep`,
   `object/builtin_function.rs`, the form that decision 4's "slots of its own frame on the engine
   stack" takes in the code) from GetIterator until it returns, on every way out, a throw
   included (`Agent::release` to its mark), and keeps the same way every value it still needs
   across a call: the result object of `next` while `done` and `value` are read, the values it
   collects, the object it builds, its callback. The Rust copies stay valid while kept: the
   collector moves no record and marks every kept value; `test-collect` fails a builtin that
   leaves one out.
2. Every call of `next` and of `return` goes through the agent's call path, as every call from
   Rust does: it charges the call's step, counts against `reentry_depth` (a builtin or host `next`
   by its activation, an ECMAScript one as a run loop nested in the builtin's), and an ECMAScript
   `next` pushes a frame against `frame_depth`. A bound exceeded is the `RangeError` of "Depth
   bounds" below, a throw of `next`, after which the iterator is not closed. The operations of
   `ops/iteration.rs` charge no step of their own for the calls they make.
3. A builtin's loop over an iterator charges one step per iteration through `charge_steps`,
   beside the step of the call of `next`, and so checks the step budget and the interrupt in the
   loop itself, whatever `next` is: a builtin `next` such as `%ArrayIteratorPrototype%.next` runs
   no instruction. The builtin's own work in an iteration charges as this decision says (a
   callback is a call).
4. IteratorClose takes the completion: for a throw it calls `return`, if there is one, and answers
   the throw it was given; for a normal completion it answers a throw of `return`, or the
   `TypeError` of a result that is no object. It is given `Abrupt::Throw` or a normal completion,
   never `Abrupt::Terminated`, which passes up untouched; a termination inside `return` ends the
   run as any termination does.

**The iterator of `for`-`of`, of an array pattern and of a spread.** Added on 2026-11-22 for slice
6. What a script sees, the standard's: a `for`-`of` gets the iterator of its head's value and runs
the body for each value, each step calling `next` and reading `done` and `value`
(`sec-runtime-semantics-forinofheadevaluation`,
`sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`). It closes
the iterator (IteratorClose, `sec-iteratorclose`) on every exit before the iterator is done: a
`break` of the loop, a `break` or `continue` to a statement around it, a `return`, a throw of the
body or of binding or assigning the value to the head (a pattern included). A `continue` of the
loop itself does not close it, nor does a throw of `next`, of reading `done` or `value`, a result
that is no object, or the end of the iteration. On a throw, `return`'s throw or answer is dropped
for the first throw; on the other exits a throw of `return` replaces the exit (the `break` or
`return` does not happen), and an answer that is no object is a `TypeError`. An array pattern
(`sec-runtime-semantics-iteratorbindinginitialization`,
`sec-runtime-semantics-iteratordestructuringassignmentevaluation`) closes its iterator when it ends
before the iterator is done (`[a] = it` closes it, `[a, ...r] = it` never does) and when a target,
a default or a nested pattern throws, not after a throw of `next`, `done` or `value`. A spread
(`sec-runtime-semantics-arrayaccumulation`, `sec-runtime-semantics-argumentlistevaluation`) runs
its iterator to the end and never closes it. A termination closes nothing.

The mechanism:

1. The record lives on the operand stack of the frame, as the enumerator of a `for`-`in` does: the
   head's GetIterator pushes an engine entry of its own (`interp/frame.rs`, `Entry`, beside
   `Entry::Enumerator`, inline or boxed as the enumerator is, whichever keeps `Entry` at its size)
   holding [[Iterator]], [[NextMethod]] and [[Done]], below the values of the head and the body,
   until the loop's exit pops it. `Frame::trace` marks the iterator and the next method; the
   entry is charged as the stack's growth is. Rejected: a local slot. The stack is cut back on
   every exit by the jumps' pops and a handler's `items`, as for the enumerator, and slice 7
   saves the operand stack with a suspended frame, so a `for`-`of` around a `yield` keeps its
   record with no other change.
2. The step instruction calls `next` through the operations of `ops/iteration.rs`, a call by
   item 2 of "Iterators held by a builtin" (a re-entry), and reads `done` and `value` as any Get of
   an instruction; a throw of any of them, and a result whose `done` is true, set the entry's
   [[Done]] to true. Each iteration charges the steps of its instructions and its `LoopJump`.
   Rejected for now: calling `next` by the call instruction, which spares a re-entry per step but
   splits each step into several instructions; a re-entry here nests no deeper than the `next` it
   calls, and a measurement decides.
3. IteratorClose is compiled as a `try` with a `finally` block is
   (`crates/compiler/src/statement.rs`, `try_statement`; `lib.rs`, `Control::Finally`,
   `exit_jump_to`, `exit_return`, `finally_dispatch`), with a control entry of its own kind
   pushed inside the loop's. A handler of kind `Finally` in the frame's handler table covers the
   code from the first step to the end of the body, the head's binding and pattern included, and
   its `items` keep the iterator entry; a `break` or `continue` that leaves through it and a
   `return` are emitted as for a `finally`: the pops down to its `items`, the tag of the exit, a
   jump to its entry. It differs from a `Control::Finally` in one point: a `continue` whose
   target is this loop does not pass it, a `break` of this loop does. Its entry, outside the
   handler's range, is one instruction, `[i, v, tag] -> [i, v, tag]`: when the entry's [[Done]]
   is false it sets it and runs IteratorClose (item 4 of "Iterators held by a builtin") with a
   throw completion of `v` for `TAG_THROW` and a normal one for every other tag; then the code
   `finally_dispatch` emits goes on with the exit, and `EndFinally` throws `v` again. A throw of
   `return` on a `break`, `continue` or `return` comes from that instruction, outside the range,
   and goes to the handlers around the loop. No new handler kind is needed.
4. A termination enters no handler (`bytecode.rs`, `Handler`) and runs no exit code, so
   IteratorClose is not called.
5. An array pattern holds its record in the same entry and closes it by the same handler and
   instruction, over the code of the pattern (its targets' references, its steps, defaults and
   nested patterns). A pattern has no `break`, `continue` or `return` in it, so only a throw and
   its normal end close it: at its end it runs the instruction with a normal completion, which
   closes when [[Done]] is false; a rest element leaves [[Done]] true. A nested pattern has an
   entry and a handler of its own, inner first in the table, so a throw closes the inner iterator,
   then the outer one. A spread is one instruction that runs its iteration in Rust as a builtin
   does (item 1 of "Iterators held by a builtin", the kept stack), appending each value and
   charging one step per element beside the call of `next`; nothing runs between its steps but
   `next`, so it needs no entry and no handler.

**The step budget.** One unit per bytecode instruction, plus units charged by builtins in
proportion to their work: per element for array, typed array and collection loops, per code unit
for string operations (a comparison of two Strings and a String made a property key: per code
unit past 32, charged before the units are read), per matcher step for regular expressions
and per code unit of a pattern before its parse at run time (decision 7, "What one step of
the matcher is"), per limb operation for `BigInt`, per code unit of its text and per value for
`JSON.parse` and `JSON.rawJSON`, per code unit of eval code before its parse, and per
pass of a sort of n property keys, n before the pass; plus what a host function charges for its
own work with `cx.charge_steps` (`host-interface.md`, section 14); one step for each call of a
clock or time zone hook, charged before the call, so that one built-in call makes at most 61
(`host-interface.md`, decision 17, added on 2026-11-24); per code unit past 32 of the String
that `Date.parse` and `new Date(string)` read, charged before the first is read (decision 6);
and, from slice 10, a step for
each entry a microtask checkpoint takes off its queue and one for each promise job queued ("The
microtask checkpoint is one entry" below). Eval code: PerformEval charges one step per
code unit of the eval String before it copies it; CreateDynamicFunction charges one per code
unit it copies into its source text and nothing more; `parse_script` charges none. Strings,
added on 2026-11-22: a String operation charges its code units before it reads or copies them,
and compares in place, with no copy; a comparison or a key of up to 32 code units costs what it
did, and a longer one a step per code unit past 32. The replacer list of `JSON.stringify` keeps
no copy of its items: each item is compared in place with every item already listed, a step per
comparison and the steps of the comparison. `JSON.parse` and `JSON.rawJSON` charge a step per
code unit of their text before they copy it, and the parser charges nothing for reading it, so
a text parsed to its end costs what it did and one that fails early has paid for its whole text;
the parser's step per value is charged in batches of a bounded size, which counts as in
proportion. The counter
is decremented where the unit is spent and
checked at every backward jump, every call and return, every re-entry, and inside every builtin
loop. At zero the run ends with the outcome `Terminated(StepBudget)`: no `catch` or `finally`
block runs, and the host gets the outcome as a value (decision 9). The host's interrupt
(decision 9) is checked at the same places, and a parse reads it at its start and at least once
per 4,096 code units it reads and per 4,096 nodes its later passes and the compile step visit
(`host-interface.md`, section 12). Why there: a loop cannot run without passing a
backward jump or a builtin loop check, so the check placement bounds the work between checks.
Rejected: a wall-clock limit inside the engine (the engine has no clock, and time would make runs
non-deterministic).

Steps are charged before the work they count, as storage is ("Memory budget", decision 4). A
list of property keys: each [[OwnPropertyKeys]] charges the steps it takes, a step per key it
will answer (a String object's code units and an Array's dense elements counted from their
lengths, not by a walk), and the list's bytes to the memory budget, before the list exists; the
code that holds the list credits its bytes when it drops it. A list whose index keys were added
in ascending order is not sorted; one whose index keys were added out of order is sorted by a
bottom-up merge sort over a second buffer charged with the list, with n steps charged before
each pass, so the step budget and the interrupt are read every n comparisons. An instruction
that loops is held to the same rule: the key list the `for`-`in` enumerator holds is made by
EnumerateObjectProperties, which charges a step per object of the prototype chain and each
object's [[OwnPropertyKeys]] as above before it collects that object's keys, and charges the
bytes of the list and of its set of visited keys before each grows. The set of visited keys is
credited when EnumerateObjectProperties returns, on every way out of the call; the enumerator
holds the charge of the list alone and credits it when the loop ends, on every way out of it, a
throw and a termination included. A frame that is suspended (a generator; an async function
from slice 10) keeps the charges it holds, which move with it, until its generator completes,
a termination drops it while it runs, or the sweep that frees the object holding it credits them
("Suspended frames" below, item 3).

**Depth bounds,** each a limit the host sets, each ending in an outcome:

- The parser (nested expressions, statements, functions): a nesting counter in the recursive
  descent parser. Exceeded, the parse fails with a `SyntaxError` whose message names
  `parser_nesting` (`host-interface.md`, decision 2): through the host as `Parsed::Errors`,
  through `eval` and `Function` as a thrown `SyntaxError` the script can catch (HOST-139 to
  HOST-141), never as a `RangeError`.
- Scope analysis and the compile step walk only trees the parser accepted, so the parser's bound
  holds for them.
- ECMAScript calls: frames on the engine stack, counted. Exceeded: a `RangeError` in the script.
  A suspended frame is not on that stack and is not counted; a resumption is counted as a call
  ("Suspended frames" below, item 4).
- Re-entry from Rust into the interpreter: counted with the frames, and separately against a
  limit derived from the Rust stack the host guarantees (decision 9). Exceeded: a `RangeError`.
- `JSON.parse`: a loop with an explicit stack, bounded by the memory budget.
- `JSON.stringify`, `toString` chains, `Proxy` trap chains: each step that calls script is a
  re-entry, bounded as one.
- Regular expressions: the matcher is a loop with an explicit backtrack stack, bounded by the
  step budget, its stacks by the memory budget; the parser and the compiler of a pattern are
  loops with explicit stacks too, and none of the three recurses in Rust (decision 7, "No Rust
  recursion in the parser and the compiler of a pattern"). A pattern's own nesting counts
  against `parser_nesting`, from zero at the start of the pattern, and passing it is the same
  `SyntaxError` naming `parser_nesting` (`host-interface.md`, decision 2): an early error of
  the script or module for a literal, thrown by the `RegExp` constructor at run time, never a
  `RangeError` (HOST-142, HOST-143).

A `RangeError` for depth is what the script sees, and the script can catch it; a script that
catches it and recurses again ends at the step budget, which it cannot catch.

**Suspended frames.** Added on 2026-11-23 for slice 7, written so that slice 10 uses it as it
is. What a script sees, the standard's: a generator's code runs in an execution context of its
own, which `yield` suspends and a resumption runs again where it stopped, with its bindings, its
operand values and the statements around the `yield` as they were (GeneratorStart,
`sec-generatorstart`; GeneratorYield, `sec-generatoryield`; GeneratorResume,
`sec-generatorresume`; GeneratorResumeAbrupt, `sec-generatorresumeabrupt`; RunSuspendedContext
and RunCallerContext, `sec-runsuspendedcontext`, `sec-runcallercontext`); the context is the
generator's [[GeneratorContext]] (`sec-properties-of-generator-instances`). Slice 10's Await
(`await`) suspends and resumes the context of an `async` function and of an async generator the
same way. What a host sees: the budgets, bounds and states of the items below. Each item states
the rule first and then the mechanism.

1. What moves. Nothing of a frame is copied or lost across a suspension: its local slots, its
   operand values (a `for`-`of` record among them, item 1 of "The iterator of `for`-`of`" above),
   the environments it pushed (a block's record, a `with` object, a record of an iteration) and
   its position are at the resumption what they were at the `yield`. The mechanism: a suspended
   frame is the whole `Frame` value of `interp/frame.rs`: instruction index, operand stack,
   local slots (with the arguments a parameter list that is not simple keeps), LexicalEnvironment
   and VariableEnvironment with `pushed_envs`, `this`, function, realm, script, completion
   value, `constructed` and `charged`. It moves by value, its vectors by their pointers, so a
   suspension and a resumption do the same work whatever the number of locals or the depth of the
   operand stack. The handler table does not move: it belongs to the compiled code
   (`Frame::handlers`, read through `compiled` and `template`), which the frame holds by its
   `Rc<Compiled>`, and code does not change after its compile step, so the table stays valid
   through the saved instruction index and `unwind` reads it after a resumption as before the
   suspension.
2. Where it lives and who traces it. A suspended generator and everything its frame holds stay
   alive exactly as long as the generator object is reachable, across entries: `collect()` frees
   neither while it is reachable and frees both, with every value only the frame held, once it is
   not. The mechanism: a suspended frame is held by exactly one object of the heap, its holder:
   the object through which every resumption of that activation goes, whose kind has a slot for
   one suspended frame. In slice 7 the holder is the generator object, its [[GeneratorContext]];
   in slice 10 it is the async generator object and, for an `async` function, the object through
   which the jobs of its `await` resume the activation, which that slice names. Nothing else
   holds a suspended frame: not the frame stack, no root, no Rust local across a safe point. The
   collector traces it at the holder's kind site with `Frame::trace`, not as a root, so a
   suspended generator that nothing references is garbage with its frame; the frame traces its
   function, script and realm, so the code it runs, with its source text and template objects
   (decision 4), lives as long as it does. The frame lies in a box in the holder's kind, so that
   the record of every object keeps its size. While the generator runs, its frame is on the frame
   stack and the holder's slot is empty: the frame stack, a root, traces the frame, and the frame
   keeps its generator object alive through the local slot its code reserves for the object. At
   every safe point a frame is in exactly one of the two places.
3. Its charge. A generator costs the memory budget its object and its frame while it holds the
   frame. The frame is charged once, by the call that makes it, and credited once: when the
   generator completes, when an abrupt completion given before its start completes it, when a
   termination drops the frame while it runs, or by the collection whose sweep frees its holder
   while the frame is suspended. A suspension and a resumption charge and credit nothing beyond
   the growth of the operand stack, charged as any growth. The mechanism: the frame's charge,
   its `charged` (`frame_bytes()`, the fixed size of `Frame`, and 16 bytes per local slot and per
   kept argument, both charged by `make_frame`; `entry_bytes()` per entry of the operand stack's
   capacity, charged by `push_entry` as it grows; the key list of each enumerator and the box of
   each Iterator Record on that stack), moves with the frame. The box in the holder is the
   frame's fixed part, which `frame_bytes()` counts wherever the frame is, on the frame stack or
   in its holder. Beside the box the holder keeps a guard of the agent's credit queue in the form
   of `Grown` (`object/mod.rs`), part of the object's record and so of its slot: while a frame is
   in the box, dropping the record adds that frame's `charged` to the queue, which the agent
   credits after the sweep in the same collection (`Agent::collect_now`). Taking the frame out
   takes its charge with it, so the guard holds nothing while the frame runs, and `pop_frame` or
   `drop_frames` credit it there: a frame's charge is in one place at a time and is credited
   once. What a generator that is never resumed costs, as a test reads it: from the call that
   makes it until the collection that frees it, its object's slot and its frame's `charged`, at
   least `frame_bytes()` + 16 × L bytes for a generator function with L local slots; so with
   L = 200 `heap_bytes()` after the call exceeds the reading before it by at least 3,200, and
   one `collect()` after the last reference to the generator is dropped gives at least 3,200
   back. A script that keeps N such generators reachable holds N times that and ends in
   `Terminated(MemoryBudget)` once the room is spent, as one that keeps N arrays does. Rejected:
   crediting the frame at a suspension and charging it again at a resumption, which would let a
   resumption end in `Terminated(MemoryBudget)` with nothing allocated and spend two operations
   of the accounting on every `yield`; and counting it in the bytes the holder's slot owns
   (`new_object_charged`), which the arena fixes when the record is stored while a frame's
   charge grows with its operand stack and leaves with every resumption.
4. `frame_depth`. A suspended generator takes no place against `frame_depth`: a script may hold
   any number of them, bounded by the memory budget alone. A resumption counts as a call: with
   `frame_depth` D and D frames on the stack, `next`, `return` and `throw` of a suspended-yield
   generator, and `next` of a suspended-start one, throw the `RangeError` of "Depth bounds" in
   the current realm, and the generator stays suspended with its state and its frame, to be
   resumed once the stack is lower. An abrupt completion given to a suspended-start generator
   (GeneratorResumeAbrupt, step 2) and a call on a completed one push no frame and are not
   checked. The mechanism: the resumption runs `check_frame_depth` before it changes the
   generator, as `call_function` does before it makes a frame.
5. Steps and budgets across entries. A resumption spends the step of a call, beside the step of
   the call of `next`, `return` or `throw` itself, and the instructions the generator runs until
   it suspends or ends are steps of the entry that runs them, the one that resumed it, whichever
   entry made the generator: the generator's code is charged as code of the resuming entry, and
   nothing is charged to an entry that has ended. The step budget and the interrupt are read at a
   resumption as at a call, and at a suspension as at a return ("The step budget"). A
   termination, `Terminated(StepBudget)`, `Terminated(MemoryBudget)` or `Terminated(Interrupted)`
   alike, enters no handler, so no `finally` around a `yield` runs and neither does the mark of
   completion of item 6:
   - a generator whose frame is on the frame stack at the termination, running or below a frame
     it called, stays `executing` and holds no frame; its frame is dropped with the others and its
     charge credited (`drop_frames`), and every later `next`, `return` and `throw` of it throws
     the `TypeError` of GeneratorValidate ("If state is executing, throw a TypeError exception.",
     `sec-generatorvalidate`, step 6), in every later entry: it never completes, and costs its
     object alone (`host-interface.md`, section 10, "Generators and async functions"; HOST-73
     holds it for the step budget);
   - a generator that is suspended when the entry ends, the entry that made it or a later one,
     keeps its state (suspended-start or suspended-yield), its frame and its charge; no `finally`
     of it ran, and a later entry resumes it as if the termination had not happened.

   Nothing credited stays charged: the frames a termination drops are credited at once, and the
   holder of a frame that was running holds no charge, so no sweep credits it again. The order
   that fixes which of the two a generator is in: a resumption checks `frame_depth`, spends its
   step, and checks `reentry_depth` and the interrupt before it sets the state to `executing`
   (GeneratorResume, step 5) and moves the frame out, so a refusal there leaves it suspended with
   its frame; from that move on a termination leaves it `executing`, a growth of the operand
   stack refused when the resumption value is pushed included. A suspension sets the state
   (suspended-start, suspended-yield) and moves the frame into the holder before it hands its
   value over, so a termination while the value is handed over leaves the generator suspended.
6. The result and re-entry. The resumer gets the standard's values: at a `yield` an iterator
   result `{ value, done: false }` (Yield, `sec-yield`; CreateIteratorResultObject,
   `sec-createiterresultobject`), at the end of the body `{ value, done: true }` with the
   returned value or `undefined`, and a throw that leaves the body as a throw, the generator
   completed in both cases (GeneratorStart). The mechanism: the generator's own code makes the
   result and marks the generator completed, and nothing is added to `return_from_frame`,
   `unwind` or `run` beyond handing a suspension's value over. The yield instruction makes the
   result object and suspends. The return instruction of a generator template, emitted at the
   end of the body and as the last instruction of `exit_return`, marks the generator completed,
   makes the result object and returns it as any return does, so `return_from_frame` pops the
   frame and credits it. The body lies in a `finally` of the existing kind whose block marks the
   generator completed, so a throw that leaves the body completes it on its way out, and `unwind`
   drops the frame as it drops any. A suspension hands its value over as a return does: as the
   answer of `run` when the frame is the base frame of the running loop, otherwise on the operand
   stack of the frame below, whose realm becomes the current realm. A resumption puts the
   received value and its tag (`TAG_NORMAL`, `TAG_THROW`, `TAG_RETURN`) on the frame's operand
   stack, and the code after the suspension point dispatches on the tag as the end of a `finally`
   does: a `return` given to a suspended generator leaves through every `finally` around the
   `yield`, a throw goes to the handlers around it.

   Re-entry. A resumption from Rust (`next`, `return` and `throw` of `%GeneratorPrototype%`, and
   through them the step of a `for`-`of`, a spread, an array pattern, IteratorClose and a host's
   `call`) runs the frame in a run loop of its own nested in the builtin's activation: one
   re-entry, counted against `reentry_depth` as any call from a builtin is (item 2 of "Iterators
   held by a builtin"), on the Rust stack of one re-entry. `STACK_BYTES_PER_REENTRY`
   (`host/limits.rs`) stays 14 KiB: a resumption path that needs more is taken apart as the run
   loop was (`AGENTS.md`, "Commands and the gate"), not paid for by the constant. A generator
   whose code resumes another nests one re-entry per level and ends at `reentry_depth` with a
   `RangeError`.

   `yield*`, the second wave of the slice (YieldExpression : `yield *` AssignmentExpression,
   `sec-generator-function-definitions-runtime-semantics-evaluation`). A delegating frame
   resumes the generator it delegates to by an instruction on its own run loop: when the
   [[NextMethod]] of its Iterator Record is `%GeneratorPrototype%.next` of any realm of the agent
   and the iterator is a generator object (for a received throw or return, when the method
   GetMethod finds, `sec-getmethod`, is `%GeneratorPrototype%.throw` or
   `%GeneratorPrototype%.return`), the instruction does what that builtin does (GeneratorValidate
   with the `TypeError` of the builtin's realm, then the checks and the move of a resumption
   above) and the inner frame runs above the delegating one; its suspension hands
   the inner result over to the frame below by the rule above, and the delegating frame reads
   `done` (IteratorComplete, `sec-iteratorcomplete`) and yields that result object as it is
   (GeneratorYield(innerResult)). Any other iterator is called as the step of a `for`-`of` calls
   `next`, a re-entry. A chain of n generators each delegating to the next therefore holds n
   frames on the frame stack while the innermost runs, counted against `frame_depth` (past it,
   the `RangeError` of item 4, thrown at the `yield*` that would push), and takes no Rust frame
   and no re-entry per level. Each level spends, per `next`, at least 2 and at most 10 steps:
   the step of its resumption and the instructions of one turn of its delegation loop (the
   resumption, the test of `done`, the yield, the dispatch on the tag, the jump back); so a
   `next` through 101 levels spends between 200 and 1,000 steps more than one through 1 level,
   which a test reads with `last_entry_steps()`. `return` and `throw` through a chain spend the
   same per level, with the Get of the method.

   A step stands for a bounded amount of work on these paths too (decision 7, "What one step of
   the matcher is", states the rule for the matcher): a suspension and a resumption move one
   value of fixed size and set one state; GeneratorValidate reads two slots; the yield and
   return instructions make one object of two properties; a level of delegation reads one
   property and moves one frame. None of them walks the locals, the operand stack, the
   environments or the chain: a `return` given to a suspended generator runs its `finally` blocks
   as code, a step per instruction, a chain of n levels spends steps in proportion to n, and only
   the collector walks a suspended frame, as it walks every record, spending no step.

**The microtask checkpoint is one entry.** Added on 2026-11-23 for slice 10. What a host sees
(`host-interface.md` decision 11): `perform_microtask_checkpoint` is one outermost entry with one
step budget, whatever it runs. Its steps start at 0 and it reads the interrupt before it spends
one, as every outermost entry does (`host-interface.md`, decisions 2 and 12). Every entry it
takes off the queue, a promise job or a host microtask, spends from that one counter: one step
for taking it off, charged while the entry is still first in the queue, so that a step budget or
an interrupt that refuses the step leaves it there, unrun and whole; then what it runs, the steps
of every call it makes and every instruction those calls run. An entry is therefore either
still queued or has started; none is lost between two jobs (changed on 2026-11-23 from a step
charged after the entry left the queue, which lost the entry whose step was refused). Every
promise job queued while it runs (HostEnqueuePromiseJob, `sec-hostenqueuepromisejob`) costs one
step more, charged before its record is made, so TriggerPromiseReactions over n reactions
(`sec-triggerpromisereactions`) spends n steps, and PerformPromiseThen on a settled promise
(`sec-performpromisethen`) one; a promise job queued by an entry outside a checkpoint costs that
entry the same step. `last_entry_steps()` after `perform_microtask_checkpoint()` reads the steps
of the whole checkpoint: every step of taking an entry off and everything the entries ran, at
most `steps_per_entry`, the same whether the checkpoint answered `Normal` or a termination (the
step a termination refused is not spent). After a checkpoint that found the queue empty it reads
0, since the checkpoint is an outermost entry whose count starts at 0 and that spent none
(HOST-193); a nested checkpoint that returned at once (HOST-81) spends no step and adds 0 to the
entry around it. So a host that sums `last_entry_steps()` after each entry, scripts and
checkpoints alike, counts every step once (HOST-197).

The interrupt ends a running checkpoint as it ends a running script, with
`Terminated(Interrupted)`, at three places: at the start, where `begin` reads the flag before the
first entry is taken off, so a flag set before the checkpoint starts ends it with nothing taken
off and `last_entry_steps()` 0, as it ends any entry that starts (`host-interface.md` decision
12); before each entry is taken off, where `charge_steps` reads the flag before it spends the
step of that entry, so a flag set between two jobs ends the checkpoint with the next entry still
first in the queue; and while a job runs, at every place decision 3 checks the step budget, so a
flag set from another thread while a job loops ends that job, which is not run again. The flag
is cleared when it ends the checkpoint (HOST-198).

The bound of the whole checkpoint: each entry it takes off costs at least one step, so one
checkpoint takes at most `steps_per_entry` entries off the queue, whatever its jobs queue. A job
that queues a job without end (`function f() { Promise.resolve().then(f); } f();`) ends the
checkpoint in `Terminated(StepBudget)` (HOST-80), and so does a host microtask that queues a
host microtask and runs no code; a job that queues many jobs at once pays a step and the charge
of each ("Queued jobs", decision 4), so it ends in one of the two budgets. A step stands for a
bounded amount of work here too: taking an entry off moves one record of fixed size and calls at
most two hooks beside the job; queueing a promise job makes one record of fixed size.

A termination of one job, `Terminated(StepBudget)`, `Terminated(MemoryBudget)` or
`Terminated(Interrupted)` alike, ends the whole checkpoint. The entry that was running is not
run again; an entry whose step of taking off was refused was not taken off and is still first;
the entries behind stay queued in their order, and the promise jobs among them stay charged;
ClearKeptObjects is not performed; the checkpoint answers `Terminated(reason)`, and the next one
runs the entries left from the first, under its own budget, as every outermost entry has
(`host-interface.md` section 10, HOST-77, HOST-199). Nothing is dropped behind the host's back:
the engine does not empty the queue at a termination, since HTML stops a script without
clearing the event loop's microtask queue and only the host knows whether the page goes on; a
host that does not want the entries left run calls `Agent::clear_microtasks` (below). A promise job whose `host_prepare_to_run_job` was
called gets its `host_clean_up_after_job` on every way out, after a termination with `Ok(())`,
so the host's two steps around a job stay paired; the reason is the checkpoint's outcome.
`Terminated(EngineFault)` faults the agent, and the queue never runs again. What the host reads
afterwards: `Normal(CheckpointReport)`, with the count of each kind, only once the queue is
empty; otherwise the termination, `last_entry_steps()` and `pending_microtasks()`, the entries
left, so it can tell which checkpoint spent its budget (HOST-148).

One budget, but a scope per job. The values handed out to host code while a job runs (to a
host microtask through its `Cx`, to a hook of a promise job through its `HookCx`, to a host
function a job calls) are valid for the scope of that job, as a value handed to a host function
is valid until the function returns (`host-interface.md` section 4), and the scripts
`Cx::parse_script` hands out as `ScriptRef`s in a job are held until that job ends. When the job
ends, on every way out, the checkpoint stops holding them: a value a job handed out and the host
kept with no `Root` and listed in no `trace` is then kept alive by nothing the host did, so a
collection at a later safe point of the same checkpoint (between two jobs, or one an allocation
of a later job starts) or a `collect()` after the checkpoint frees it once nothing else reaches
it, and the kept handle then answers `Err(StaleHandle)`, as HOST-178 states for a handle kept
past a `with_cx` (HOST-196). So a checkpoint of many jobs holds at any time what the running job
was handed, never what every job touched. The mechanism: before each entry the loop notes the
length of the agent's list of handed-out values (`Agent::scope`) and of its scoped scripts, and
after it truncates both to those lengths, also after a termination; `Cx::perform_microtask_checkpoint`
does the same inside the scope of the host function that called it, which keeps what it was
handed before. Rejected: holding everything handed out until the checkpoint returns (the
outermost entry's rule for `with_cx`): a checkpoint that runs `steps_per_entry` jobs would grow
the list, uncharged, by every value each of them touched.

The mechanism. The checkpoint calls `Agent::begin` once and then sets the agent's flag of a
running checkpoint until it returns, on every path. `begin` sets `steps` to 0 and reads the
interrupt only when no builtin runs and that flag is clear, so every `Cx` call a host microtask
makes, and the call of a handler, is a nested entry that spends from the checkpoint
(`host-interface.md` decision 2: "An entry made by host code that runs inside another entry (a
host function, a hook, a host microtask) is nested"). The loop charges the step of the first
entry with `charge_steps`, which reads the interrupt before it spends, then takes the entry off,
puts the values of a promise job on the kept stack before it calls a hook or code ("Queued jobs", decision 4), runs it, and
comes to a safe point before the next entry, where a collection may run. A host microtask runs
with its realm as the realm of the entry, a promise job with its realm, or with the realm current
at its enqueue when its realm is null. `Cx::perform_microtask_checkpoint` runs the same loop:
from `with_cx` it is an outermost entry of its own; from a host function it runs inside the
running entry, on that entry's steps, and a termination in it is that entry's, which the host
function receives as `Abrupt::Terminated`; while the flag is set it answers both counts 0 at once
and runs nothing (HOST-81). Rejected: a step budget per job (the text of decision 9 until
2026-11-23: a job that queues a job would never let the checkpoint end); steps for code alone
and none for taking an entry off (a host microtask that queues another and runs no code would
never end the checkpoint).

Emptying the queue is the host's call. Added on 2026-11-23. `Agent::clear_microtasks` drops
every queued entry, promise jobs and host microtasks alike, credits what the promise jobs among
them were charged, and answers how many entries it dropped (HOST-200), so that a page whose jobs
flood the queue cannot spend the budgets of the entries that come after it. It runs no script,
calls no hook and spends no step; a host microtask it drops is dropped unrun as a Rust value,
which is the host's own code and not a callback of the engine. No job runs, so no
`host_prepare_to_run_job` or `host_clean_up_after_job` is called for any of them. A promise whose
reaction job it dropped stays pending for good, as after a termination: the derived promise of a
`then` whose job was dropped is never settled, a promise whose resolve-thenable job was dropped
is already resolved and so never settles, and an `async` function whose resumption was dropped
is never resumed ("`async` functions on suspended frames" below, item 5). It is refused inside a running checkpoint by its type: it
takes `&mut Agent`, which no code inside an entry holds, and `Cx` and `HookCx` have no such
method; a job that dropped the entries behind it would change the order of the checkpoint that
runs it, which HTML's checkpoint never does. Rejected: a no-op answer inside a checkpoint (a call
that answers 0 with entries queued would mislead the host). Dropping the agent with entries
still queued frees them with it and calls nothing of the host: no hook, no job and no
microtask runs. A host function called from a job may settle a promise, through a resolve
function it holds; the reaction job that queues runs in the same checkpoint, behind every entry
already queued, in the one order of the queue (HOST-78, HOST-79).

**`async` functions on suspended frames.** Added on 2026-11-23 for slice 10. What a script sees,
the standard's: an `async` function runs in its call until its first `await` and returns its
promise there (AsyncFunctionStart, `sec-async-functions-abstract-operations-async-function-start`;
AsyncBlockStart, `sec-asyncblockstart`; EvaluateAsyncFunctionBody,
`sec-runtime-semantics-evaluateasyncfunctionbody`); `await` suspends its execution context, and a
job of the promise it awaits runs it again where it stopped, with the value, or with the reason
thrown at the `await` (Await, `await`; RunSuspendedContext, `sec-runsuspendedcontext`). What a
host sees: the hooks, budgets, bounds and states of the items below. Item 2 of "Suspended frames"
leaves the holder of an `async` function's frame to this slice, and the items below name it;
items 1 to 6 there hold for an `async` function as they are written for a generator, except
where an item below says otherwise. Each item states the rule first and then the mechanism.

1. The holder. A suspended `async` function, with its frame and every value only the frame holds,
   stays alive exactly as long as something that will resume it is reachable: a reaction of the
   promise it awaits, or the job of that reaction in the queue. `collect()` frees all of it once
   neither is, also while a script still holds the function's own promise, which then stays
   pending. The mechanism: the holder is the activation of an `async` function, an internal kind
   of object (`object/async_function.rs`) with a null [[Prototype]], which no instruction and no
   read hands to a script or a host. Its kind holds the slot for one suspended frame
   (`SuspendedFrame`, `interp/suspend.rs`), the state (executing, suspended at an `await` with the
   number of that suspension, completed) and the promise of the call with what settles it; its
   kind site traces all of them. The call makes the activation and the promise before
   FunctionDeclarationInstantiation, and its code keeps the activation in a local slot it
   reserves, as a generator's code keeps its object, so while the frame runs the frame keeps the
   activation alive (the frame stack is a root). While the frame is suspended, the two functions
   of its `await` hold the activation in their slots (item 3), the reactions of the awaited
   promise hold those functions, and the promise, or the queue once it settled, holds the
   reactions. Rejected: the promise of the call as the holder. No resumption goes through it: a
   job of the awaited promise resumes the function, so the call's promise is not the object item
   2 of "Suspended frames" asks for; a promise a script keeps would keep alive a frame nothing can
   resume once the awaited promise is garbage; and every promise would carry a slot for a frame.
2. Its charge, and what an `await` that never ends costs. The frame is charged and credited as
   item 3 of "Suspended frames" states: charged once, by the call that makes it; credited once,
   when the body returns or throws (the return of an `async` template pops the frame,
   `pop_frame`, and the handler around its body does so for a throw), when a termination drops it
   while it runs (item 5), or after the sweep that frees the activation while the frame is
   suspended, through the guard of the credit queue beside the box. While the promise it awaits
   never settles, a suspended `async` function costs: its activation's slot, its frame's `charged`
   (at least `frame_bytes()` + 16 × L bytes for L local slots), the two functions of its `await`
   with their boxes (item 3; decision 4, "A built-in function with slots") and two reactions in
   the awaited promise's lists. So with L = 200, `heap_bytes()` after the call exceeds the reading
   before it by at least 3,200, and one `collect()` after the last reference to the awaited
   promise is dropped gives at least 3,200 back, whether or not the script still holds the
   function's own promise. A promise the host roots, or one a reachable object holds, keeps all of
   it until the root is released or the object is freed; a script that keeps N such promises
   reachable holds N times that and ends in `Terminated(MemoryBudget)` once the room is spent, as
   one that keeps N arrays does. Dropping the agent frees it with the heap, and its charge ends
   with the accounting it was in.
3. What an `await` makes, and which hooks its job calls. The standard's, so what a host sees is
   the draft's sequence: Await makes two built-in functions, onFulfilled and onRejected, with
   CreateBuiltinFunction (`await`, steps 3 to 6), and calls PerformPromiseThen without a
   capability, which calls HostMakeJobCallback for each (`sec-performpromisethen`,
   `sec-hostmakejobcallback`). When the awaited promise settles, the reaction job
   (`sec-newpromisereactionjob`) has the realm GetFunctionRealm gives for the function
   (`sec-getfunctionrealm`): the current realm at the `await`, the `async` function's realm. It
   calls the function through HostCallJobCallback (`sec-hostcalljobcallback`). So every `await`
   calls `host_make_job_callback` twice, and the job that resumes the function calls
   `host_prepare_to_run_job` with the function's realm, `host_call_job_callback` once with one of
   the two functions and the value made for it, and `host_clean_up_after_job`, as every reaction
   job does (HOST-82, HOST-84, HOST-194). The first suspension of a call hands the call's promise
   to its caller as a return does; a later one ends the run of the function the job called, which
   returns `undefined` (Await's closures: "Return NormalCompletion(undefined)."). The mechanism:
   the two functions are built-in functions with slots (decision 4): each holds the activation and
   the number of the suspension it was made for. Called while the activation is suspended at that
   suspension, it resumes it, with `TAG_NORMAL` and the value or `TAG_THROW` and the reason;
   called at any other time (its twin ran first, or a host kept it from `host_make_job_callback`
   and calls it), it does nothing and returns `undefined`. The reaction is an ordinary reaction
   whose handler is the JobCallback of that function, and its job runs in the checkpoint as every
   promise job does. A function of an `await` resumes as a resumption from Rust does (item 6 of
   "Suspended frames", "Re-entry"): one re-entry, which ends before the job does. Rejected: an
   `await` that makes no function, with a reaction that names the activation directly and a job
   that calls `host_prepare_to_run_job` and `host_clean_up_after_job` and neither job-callback
   hook. No script could tell: the resumed code runs in the function's own execution context,
   whose realm and ScriptOrModule decide the incumbent settings object and the active script
   while it runs, as the topmost script-having execution context
   (`html:incumbent-settings-object`, `html:topmost-script-having-execution-context`), with or
   without the steps of `html:hostcalljobcallback`. But a host could: one that implements
   `host_make_job_callback` or `host_call_job_callback` would see two calls fewer per `await`
   and one fewer per job, and would get no function it could keep and call as the draft lets it.
   The saving was two objects of fixed size per `await`, garbage once the job ran.
4. Steps and depth. An `await` spends the step of its instruction and what its parts cost
   anywhere: the Get of `constructor` that PromiseResolve makes when the value is a promise
   (`sec-promise-resolve`), and the step of a promise job when the awaited promise is settled
   already and PerformPromiseThen queues the job at once ("The microtask checkpoint is one
   entry"); making the two functions and the two reactions is bounded work within its step. Its
   resumption costs what a generator's does (items 4 and 5 of "Suspended frames"): the step of a
   call for the resumption, beside the step of the call of the function the job calls and the step
   of taking the job off the queue, so a resumed `await` has spent 3 steps when its next
   instruction runs. The instructions it then runs are steps of the checkpoint that runs the job,
   whichever entry made the activation, and nothing is charged to an entry that has ended. A frame
   suspended at an `await` is not on the frame stack and takes no `frame_depth`; a resumption
   checks `frame_depth`, spends its step, and checks `reentry_depth` and the interrupt before it
   sets the state to executing and moves the frame out. A chain of `await`s of any length takes
   neither depth, since each link is resumed by a job of its own and each job returns before the
   next runs. A resumption that `frame_depth` or `reentry_depth` refuses, possible only in a
   checkpoint a host function runs deep in its entry (`Cx::perform_microtask_checkpoint`), throws
   the `RangeError` of "Depth bounds" from the function the job called: the job ends with that
   throw, passed to `host_clean_up_after_job` as `Err`, and the activation stays suspended with
   its frame. That reaction is spent, so nothing resumes it; its promise stays pending, and it is
   freed with its frame once nothing reaches it. The draft asserts that such a job's handler
   never completes abruptly ("Assert: handlerResult is not an abrupt completion.",
   `sec-newpromisereactionjob`); the depth bounds are implementation limits it does not have
   (`host-interface.md` decision 2), and this is where they show.
5. Termination. A termination, `Terminated(StepBudget)`, `Terminated(MemoryBudget)` or
   `Terminated(Interrupted)` alike, enters no handler: no `finally` around an `await` runs, the
   handler around the body that rejects the function's promise does not run, and the promise is
   not settled (`host-interface.md` section 10, HOST-195):
   - an activation whose frame is on the frame stack at the termination, running or below a frame
     it called, in the call that made it or in a job that resumed it, stays executing and holds no
     frame. Its frame is dropped with the others and its charge credited (`drop_frames`); its
     promise stays pending for good; a later call of a function of its `await` does nothing (item
     3), so nothing runs it again, and it costs its object alone until nothing reaches it. Where
     the memory budget refuses a growth makes no difference: making an `await`'s two functions,
     its reactions or the record of a job happens while the frame runs, and a growth refused when
     the received value is pushed after the move leaves it executing too, its frame dropped and
     credited by `resume_frame`.
   - an activation suspended at an `await` when an entry ends, the one that made it or a later
     one, keeps its state, its frame and its charge. Its reaction stays in the awaited promise, or
     its job in the queue, also when the step of taking that job off was the one refused, and a
     later checkpoint resumes it as if the termination had not happened. Except when its job had
     started and a check of the resumption before the move (the step of the call or of the
     resumption, the interrupt) was refused: the reaction is spent, the activation is never
     resumed, its promise stays pending, and it is freed with its frame once nothing reaches it.
     The same holds when `Agent::clear_microtasks` drops its job.

   Nothing credited stays charged, and nothing charged is credited twice, by the order of item 5
   of "Suspended frames": a suspension sets the state and moves the frame into the activation
   before it hands its value over, and a resumption moves the frame out only after its checks.

**Async generators on suspended frames.** Added on 2026-11-23 for slice 10. What a script sees,
the standard's: an async generator object (`sec-asyncgenerator-objects`,
`sec-properties-of-asyncgenerator-intances`) holds a queue of requests, each a completion and a
promise capability (`sec-asyncgeneratorrequest-records`); `next`, `return` and `throw`
(`sec-asyncgenerator-prototype-next`, `sec-asyncgenerator-prototype-return`,
`sec-asyncgenerator-prototype-throw`) each make a promise, queue a request
(`sec-asyncgeneratorenqueue`) and resume the generator at once only when it is suspended at its
start or at a `yield` (`sec-asyncgeneratorresume`); a `yield` awaits its operand, settles the
request at the head and goes on without suspending while requests are queued
(`sec-asyncgeneratoryield`, `sec-asyncgeneratorcompletestep`,
`sec-asyncgeneratorunwrapyieldresumption`); the end of the body settles the head and drains the
rest, waiting on the value of each `return` it meets (`sec-asyncgeneratorstart`,
`sec-asyncgeneratordrainqueue`, `sec-asyncgeneratorawaitreturn`). What a host sees: the items
below. Items 1 to 6 of "Suspended frames" hold for an async generator as they are written for a
generator, and items 3 and 4 of "`async` functions on suspended frames" for each of its `await`s,
except where an item below says otherwise. Each item states the rule first and then the
mechanism.

1. The holder, and who keeps it alive. An async generator, with its frame, its queue and every
   value only they hold, stays alive exactly as long as something that can resume it or settle
   one of its requests is reachable: its object, held by a script or a host; while its frame
   runs, that frame; while it is suspended at an `await`, the two functions of that `await`,
   through the reactions of the awaited promise or their job in the queue; while it waits on the
   value of a `return` (AsyncGeneratorAwaitReturn), the two functions made there, the same way. A
   queued request keeps nothing alive: the object holds the queue, and the promise of a request
   holds no reference to the generator. So `collect()` frees a generator suspended at an `await`
   of a promise nothing else reaches, with its frame and its queued requests, also while a script
   holds the promises of those requests, which then stay pending. The mechanism: the holder of
   item 2 of "Suspended frames" is the async generator object, `ObjectKind::AsyncGenerator`
   (`object/async_generator.rs`), whose kind holds the state ([[AsyncGeneratorState]]:
   suspended-start, suspended-yield, executing, draining-queue, completed), the frame in a box
   with the guard of the credit queue beside it, as a generator's, the number of its current
   wait (an `await`, or the wait of a `return`), and the queue in a box; [[GeneratorBrand]] is
   empty for every async generator of the slice and is not held. The frame is in the box while
   the generator is suspended at its start, at a `yield` or at an `await` (its state is then
   executing, as the draft keeps it), and on the frame stack while it runs; the code keeps the
   object in a local slot it reserves, as a generator's does. The two functions of an `await`
   and of the wait of a `return` are built-in functions with slots (decision 4) that hold the
   object and the number of that wait; called while the generator waits at that number, the
   first resumes it with the value (`TAG_NORMAL`) and the second with the reason (`TAG_THROW`),
   or, for the wait of a `return`, settles the head and drains as the draft's closures do;
   called at any other time they do nothing and return `undefined`, as item 3 of "`async`
   functions on suspended frames" states. The kind site traces the frame and every value of
   every request (the completion's value, the capability's promise and its two functions); no
   request is a root. The record of every object stays at 200 bytes (the test of
   `object/generator.rs`).
2. The queue's charge and its bound. Every request costs the memory budget from the call that
   queues it until the collection that frees the generator. The mechanism: the queue is a
   `VecDeque` of entries of one fixed size (the completion's tag and value, the capability's
   promise and two functions) in the kind's box; its capacity grows by the engine's own
   doubling with `try_reserve_exact`, is charged before it grows and is recorded in the object's
   `Grown` (`object/mod.rs`), so the sweep that frees the object credits it; a growth the budget
   refuses ends the run in `Terminated(MemoryBudget)` with nothing queued. Settling a request
   credits nothing: the capacity, at most the most requests queued at once, stays charged while
   the object lives. The bound on the queue is the memory budget, and no count: each request is
   made by a call that spends a step and makes a promise with its two resolving functions, so a
   script that queues requests without end (`for (;;) g.next();` on a generator suspended at an
   `await` of a promise that never settles) ends in `Terminated(MemoryBudget)` or
   `Terminated(StepBudget)`, whichever budget it spends first, and the promises of the requests
   are charged as objects besides. Rejected: a limit on the count (a limit with an outcome of its
   own for what the memory budget already bounds); charging each request when it is queued and
   crediting it when it is settled (two operations of the accounting per request, for bytes the
   capacity already counts).
3. Steps. A call of `next`, `return` or `throw` spends the step of a builtin call; validating,
   making its promise and queueing its request is bounded work within it. A call that resumes
   the generator (suspended at its start or at a `yield`) also spends the step of the resumption
   (item 5 of "Suspended frames"), and the instructions the generator runs are steps of the entry
   that resumed it. A `yield` spends the step of its instruction beside its `await`; settling
   the request at the head, or taking the next request and going on without suspending, is
   bounded work within it. The drain, which settles requests without running the generator,
   spends one step for each request it settles, before it takes that request off, so draining r
   requests spends r steps, and the wait of a `return` it meets is an `await` (item 4 of
   "`async` functions on suspended frames"). So a step stands for a bounded amount of work when a
   script queues requests without end: a request costs a step to queue and a step, or the `yield`
   that answers it, to settle.
4. Depth. A resumption by `next`, `return` or `throw` is checked as a generator's (item 4 of
   "Suspended frames"; one re-entry, item 6): `frame_depth`, its step, `reentry_depth` and the
   interrupt, before the request is queued and before the state changes. A refusal there exists
   only for an implementation limit the draft does not have, so the order differs from the
   draft's (AsyncGeneratorEnqueue before AsyncGeneratorResume) nowhere a script can see. The
   `RangeError` of "Depth bounds" there rejects the promise of the call, which the call answers,
   since no step of the draft's `next`, `return` or `throw` after AsyncGeneratorValidate throws;
   nothing is queued, and the generator stays suspended with its state and frame, to be resumed
   once the stack is lower. A resumption by a job (a function of an `await` or of the wait of a
   `return`) is checked as item 4 of "`async` functions on suspended frames" states. A generator
   suspended at its start, at a `yield` or at an `await` takes no `frame_depth`.
5. Termination. A termination, `Terminated(StepBudget)`, `Terminated(MemoryBudget)` or
   `Terminated(Interrupted)` alike, enters no handler: no `finally` around a `yield` or an `await`
   runs, the handler around the body that settles the head request with the body's completion
   (AsyncGeneratorStart's closure) does not run, and no drain starts (`host-interface.md`,
   section 10):
   - a generator whose frame is on the frame stack at the termination, running or below a frame
     it called, in the call of `next`, `return` or `throw` that resumed it or in a job that
     resumed it, stays executing and holds no frame; its frame is dropped with the others and its
     charge credited (`drop_frames`). The request at the head and every request behind it keep
     their promises pending for good: a function of its last `await` does nothing (item 1), and
     `next`, `return` and `throw` of an executing generator only queue ("Assert: state is either
     executing or draining-queue.", `sec-asyncgenerator-prototype-next`). So every later call of
     them queues a request whose promise stays pending, charged as item 2 states, until nothing
     reaches the generator. Here it differs from a generator, whose `next` throws: the draft gives
     an executing async generator no answer but to queue. Where the memory budget refuses a growth
     makes no difference, its own queue's included.
   - a generator suspended at its start or at a `yield` when an entry ends, the one that made it
     or a later one, keeps its state, its frame, its charge and its queue (empty, by the draft's
     invariant); a later entry resumes it as if the termination had not happened.
   - a generator suspended at an `await`, or waiting on the value of a `return`, when an entry
     ends keeps its state, its frame (it has none while it waits on a `return`), its charge and its
     queue, and its reaction stays in the awaited promise or its job in the queue; a later
     checkpoint resumes it, and the requests queued meanwhile are answered in order. Except when
     its job had started and a check before the move was refused, or `Agent::clear_microtasks`
     dropped that job: the reaction is spent, nothing resumes the generator, and every request
     queued then or later keeps its promise pending for good, as in the first case.
   - a drain that a termination ends (its step for a request refused, or a termination inside the
     resolving of a request, which reads `then` of an iterator result and so can run a getter of
     `Object.prototype`) leaves the state draining-queue and the requests it did not settle
     queued, their promises pending for good: nothing drains the queue again, and later requests
     only queue.

   Nothing credited stays charged, and nothing charged is credited twice, by the order of item 5
   of "Suspended frames". Rejected: settling the queued requests with a rejection once a
   termination left the generator without a frame (a state and an order of settling the draft
   does not have, decided by the engine while no code of the generator runs).
6. `yield*` in an async generator (the async branch of YieldExpression : `yield *`
   AssignmentExpression, `sec-generator-function-definitions-runtime-semantics-evaluation`). The
   draft calls the inner iterator's method synchronously and awaits its result: "Let innerResult
   be ? Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]], « received.[[Value]]
   »)." and then "If generatorKind is async, set innerResult to ? Await(innerResult).", and the
   inner async generator's `next` resumes it at once when it is suspended at its start or at a
   `yield` (AsyncGeneratorResume). So a `next` given to the outermost of a chain of n delegating
   async generators runs the n levels one inside the other, in that call, until the innermost
   suspends at the `await` of its `yield` or of its code; only then does each level, from the
   innermost out, get the promise of its request and await it. The answers settle afterwards, one
   job per level, from the innermost out. A delegation that waits for a job before it calls the
   inner `next` would run the inner generator's code after the code that follows the outer
   `next` call, an order a script sees. The rule: the n levels of the call nest on the frame
   stack, as the levels of a generator's `yield*` do, and take no Rust frame and no re-entry
   per level. The mechanism, the one of the paragraph on `yield*` of "Suspended frames": when the
   [[NextMethod]] is `%AsyncGeneratorPrototype%.next` of any realm of the agent and the iterator
   is an async generator object (for a received throw or return, when the method GetMethod finds
   is `%AsyncGeneratorPrototype%.throw` or `.return`), the instruction does what that builtin
   does (its promise, AsyncGeneratorValidate with a rejection, and, when the inner generator is
   suspended at its start or at a `yield`, the checks of item 4 and the move of a resumption, then
   the request queued), and the inner frame runs above the delegating one on the same run loop.
   When the inner frame leaves the stack (it suspends at an `await` or a `yield`, or its body
   ends), the value it hands to the frame below is the promise of that request, which the
   delegating frame awaits as the draft says. An inner generator that executes, drains or has
   completed is not resumed: the instruction queues the request or settles it as the builtin
   does and hands its promise over at once. Any other async iterator (an object of a script, an
   Async-from-Sync Iterator object) is called as the step of a `for await` calls `next`, a
   re-entry that returns before the delegating frame awaits.
   What bounds a chain: while a `next` runs through it, the n levels hold n frames on the frame
   stack, counted against `frame_depth`, and no re-entry; past `frame_depth`, the check of item 4
   at the level that would push rejects that level's request with the `RangeError` of "Depth
   bounds", which the delegating frame's `await` throws at its `yield*`, and the throw leaves
   each level outward as the draft gives it, so the outermost request is rejected with that
   `RangeError` after the checkpoint. Between two `next` calls each level is suspended and takes
   neither depth; its frame, the functions and reactions of its `await` and its queued request
   are charged (items 1 and 2), so the number of levels is bounded by `frame_depth` and the
   memory budget. Each job of the settling resumes one level and returns before the next job
   runs: one re-entry at a time, whatever n. Steps: a `next` through n levels spends, per level,
   at least 4 and at most 20 steps more than through one: the step of the resumption and the
   instructions of one turn of the delegation loop in the call, and the job's step, the step of
   the call of the `await`'s function and the step of the resumption (item 4 of "`async`
   functions on suspended frames"), the step of the promise job queued, and the `yield` that
   settles the level's request in the settling. So a `next` through 101 levels spends between
   400 and 2,000 steps more than one through 1 level, which a test reads with
   `last_entry_steps()` over the script and the checkpoint.

**A built-in async function.** Added on 2026-11-23 for slice 10. What a script sees, the
standard's: a built-in function whose [[Async]] is true runs its steps as an `async` function
runs its body (BuiltinCallOrConstruct, `sec-builtincallorconstruct`, which calls
AsyncFunctionStart with a closure of the steps): the call answers a promise once the steps reach
their first Await or their end, and each Await suspends the steps until a job of the awaited
promise continues them; the execution context has the function's [[Realm]] and a null
ScriptOrModule. `Array.fromAsync` is one ("This async function performs the following steps
when called:", `sec-array.fromasync`); `AsyncDisposableStack.prototype.disposeAsync` of slice 15
is the next. What a host sees: each Await of such a builtin is an `await` of item 3 of "`async`
functions on suspended frames": two built-in functions made, `host_make_job_callback` called
twice, and a job that calls `host_prepare_to_run_job` with the builtin's realm,
`host_call_job_callback` once and `host_clean_up_after_job`, so a host cannot tell it from an
`await` in a script. Each item states the rule first and then the mechanism.

1. The holder, and its charge. A builtin suspended at an Await, with the values its steps hold
   (for `Array.fromAsync`: the Iterator Record or the array-like and its length, the array, k,
   the mapper, thisArg and the promise of the call), stays alive exactly as long as something
   that will continue it is reachable, as an `async` function's activation does (item 1 of
   "`async` functions on suspended frames"), and costs a fixed size while it waits: its
   activation and the box of its state, the two functions of its Await with their boxes, and two
   reactions; no frame, since its steps run in Rust. The mechanism: an internal kind of object,
   `ObjectKind::AsyncBuiltinActivation` (`object/async_builtin.rs`), with a null [[Prototype]],
   which no instruction and no read hands to a script or a host. Its kind holds the state
   (running, suspended at an Await with the number of that suspension, completed), the
   capability of the call, and a box with the builtin's own state: which builtin, the step it
   reached, and its values, one variant of fixed size per builtin. The box is charged with the
   record when it is made (`Heap::new_object` with those owned bytes, as for a built-in function
   with slots, decision 4) and credited by the sweep that frees the record; the array and what
   it grows are charged as any array. The kind site traces every value of the box and of the
   capability; no value of it is a root. While its steps run, the activation is on the kept
   stack (`Agent::keep`), where a running builtin keeps what it needs across a call into code;
   while it is suspended, the two functions of its Await hold it in their slots, the reactions of
   the awaited promise hold the functions, and the promise, or the queue once it settled, holds
   the reactions.
2. Steps and depth. The call spends the step of a builtin call and runs the steps to the first
   Await in it. A continuation costs what an `await`'s resumption costs (item 4 of "`async`
   functions on suspended frames"): the step of taking the job off, the step of the call of the
   function the job calls and one step for the continuation, so 3 steps before the next of its
   steps runs. Each continuation runs the builtin's steps from one Await to the next, or to the
   end: bounded work beside the calls it makes into code, each of which spends the steps of the
   code it runs. Every turn of both loops of `Array.fromAsync` awaits at least once, so no
   continuation loops, and an array-like of length 10^9 costs at least 3 steps per element. A
   continuation takes no frame and no re-entry of its own, so no depth is checked for it; each
   call it makes into code (`next`, the mapper, a getter, a constructor) is a re-entry counted
   against `reentry_depth`, as any call from a builtin is, and returns before the continuation
   does. A `RangeError` of a depth bound there is a throw completion of the call, which the
   builtin's steps handle as the draft's (IfAbruptCloseAsyncIterator for `Array.fromAsync`,
   `sec-ifabruptcloseasynciterator`), and the interrupt is read at the continuation's step.
3. Termination. A termination, `Terminated(StepBudget)`, `Terminated(MemoryBudget)` or
   `Terminated(Interrupted)` alike, runs none of the builtin's steps: no AsyncIteratorClose, no
   rejection of its promise (`host-interface.md`, section 10):
   - a builtin whose steps run at the termination, in its call or in a continuation, or below a
     call they made into code, stays running, and its promise stays pending for good: a later call
     of a function of its last Await does nothing, as in item 3 of "`async` functions on
     suspended frames", so nothing continues it; the iterator it read is not closed, and the array
     keeps the elements defined so far. It costs its activation alone until nothing reaches it.
   - a builtin suspended at an Await when an entry ends keeps its state, and its reaction stays
     in the awaited promise or its job in the queue; a later checkpoint continues it as if the
     termination had not happened. Except when its job had started and a check of the
     continuation (the step of the call or of the continuation, the interrupt) was refused, or
     `Agent::clear_microtasks` dropped the job: the reaction is spent, and its promise stays
     pending for good.

   Rejected: the steps written as script source compiled into each realm (a second text of the
   draft's steps; its frames would count against `frame_depth`, its instructions would spend
   steps no other builtin spends, and its functions could be reached through a stack of frames
   or `Function.prototype.toString`); a Rust `async fn` (the values a compiler-made future holds
   across an `.await` are invisible to the collector, so they would have to be roots, and a
   builtin waiting on a promise nothing reaches would live as long as its roots); a kind per
   builtin (a site to trace and a site to charge for each).

**The Async-from-Sync Iterator.** Added on 2026-11-23 for slice 10. What a script sees, the
standard's: CreateAsyncFromSyncIterator wraps a sync Iterator Record in an object that is never
directly accessible to ECMAScript code (`sec-createasyncfromsynciterator`,
`sec-async-from-sync-iterator-objects`), and the `next`, `return` and `throw` of
`%AsyncFromSyncIteratorPrototype%` (`sec-%asyncfromsynciteratorprototype%-object`) call the
sync iterator's method at once and settle their promise through PerformPromiseThen of the value
wrapped by PromiseResolve with a capability (AsyncFromSyncIteratorContinuation,
`sec-asyncfromsynciteratorcontinuation`). None of their steps Awaits, so nothing of them waits
for a job and they need no activation: they use only built-in functions with slots (decision 4).
The unwrap function holds `done` in its slot (`sec-async-from-sync-iterator-value-unwrap-functions`),
the function that closes the sync iterator holds its Iterator Record, and the object holds
[[SyncIteratorRecord]] in a box of its kind, `ObjectKind::AsyncFromSyncIterator`, charged with
the record as owned bytes and traced at its kind site, as a built-in function's slots are. What a
host sees is the draft's: `host_make_job_callback` for each callable handler PerformPromiseThen
gets (one or two), and a reaction job per settlement. A call of one of these methods spends the
step of a builtin call; the call of the sync iterator's method is a re-entry as any call from a
builtin, and a sync generator resumed by it is resumed from Rust (item 6 of "Suspended frames").

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
list of 9.10, the GlobalSymbolRegistry ("Symbols" below), and the host's root table. A
suspended frame is no root: the kind site of the object that holds it traces it, so it lives as
long as that object (decision 3, "Suspended frames", item 2). A builtin
that can call script keeps every value it still needs in slots of its own frame on the engine
stack, which the collector scans; a Rust local holding a handle across a call into script is a
bug, found by a test mode that collects at every safe point and by the generation check.
Collections run only at safe points (instruction boundaries and the re-entry points), when the
bytes charged since the last collection pass a
threshold, the live reading ("Memory budget") after the last collection, and when the host
asks. The threshold follows the live reading and not the charged one, which keeps the arenas'
capacity, so that garbage is collected before the charged reading reaches `memory_bytes`. They
never run inside an allocation. The collector's workspace, a mark word per slot and a worklist
of at most one entry per slot, lives with the arenas and is charged with their capacity, so a
collection allocates nothing and one that starts at the memory limit finishes. An allocation
that would pass `memory_bytes` collects nothing first: it ends the run, also when a collection
could free bytes, so a budget holds the live size, the garbage of one threshold and the arenas'
spare capacity. Triggers count bytes, never time, so for the same inputs and budgets the same objects
are collected at the same points, and `WeakRef` and `FinalizationRegistry` results are
deterministic. Weak references are cleared after marking (9.9); finalization callbacks become
cleanup jobs through `HostEnqueueFinalizationRegistryCleanupJob`.

**Host-held values.** The host holds values through a root table: a `Root` is an index and
generation into it, released explicitly or when its owner drops it. Values the host keeps inside
its own objects (decision 9) are traced, not rooted.

**Memory budget.** Every allocation and every growth goes through one accounting function that
charges the bytes before the growth: an arena's slots by capacity when it grows, with the
collector's workspace for them ("Collector"); the atom table's buckets (`heap/string.rs`,
`Atoms`) by capacity when the table grows; string units, property and element capacity and
`BigInt` limbs; and the frontend's, the source's copy, the parser's tree, the compile step's
scratch and the bytecode, each by capacity as it grows; and the bytes the host charges for what
it holds for the agent outside the heap (`charge_host_bytes`, `host-interface.md` section 14,
added on 2026-11-22). A sweep credits what a freed record owns, and what an atom entry it drops
owns, and after it the agent credits the charge of each suspended frame a freed record held
(through the credit queue, decision 3, "Suspended frames", item 3) and the host's charges
whose `HostBytes` were dropped (with the host data that held them, or by the host); an arena's capacity and the atom table's stay
charged while the agent lives, since neither shrinks and a freed slot or bucket is used again. A temporary structure (the frontend's tree and scratch, a list
of property keys, the key list of a `for`-`in` enumerator) is credited when it drops, and the
code's charge passes to its script record when the record is made. A growth charged by capacity
sets the capacity itself, by the engine's own doubling with `try_reserve_exact`, so a charge
does not depend on the growth policy of the standard library. Growth of large buffers uses
`Vec::try_reserve`, so a refusal by the allocator becomes an outcome
instead of an abort. Over the budget: `new ArrayBuffer` and the typed array constructors throw a
`RangeError`, as `CreateByteDataBlock` allows; any other allocation ends the run with
`Terminated(MemoryBudget)`. The room such a run leaves, `memory_bytes` less `heap_bytes()`, has no
lower bound, since the run ends at the first growth refused whatever was left below it, so the
host makes room (roots dropped and a collection, or its own charges given back) before the next
entry (`host-interface.md`, decision 10, HOST-188 to HOST-190; added on 2026-11-22).

The rule, for every slice: storage is charged before it is allocated, and work before it is
done. A structure's bytes are charged to the memory budget before the structure exists or
grows, and a loop's steps are charged to the step budget before the work they count, at least
once per bounded amount of it ("The step budget", decision 3). A check after the work is not a
bound: work measured only once it is done has already run, however large, when the check
refuses it. The code that holds a charged structure credits it when it drops it, on every way
out. The entries the host's own calls add, to the root table and to the list of values handed
to a scope (`host-interface.md`, section 4), are the host's, bounded by its calls, and are not
charged; the records they name are charged as any record.

Added on 2026-11-24: a buffer of the engine outside its arenas whose size a script controls (a
copy of a String's code units, the units a String builder grows, a list of values, keys,
captures or records, the microtask queue, the kept stack's room) is charged by its capacity
before it is made or grows, and credited when it drops. It is held as a `ChargedVec` grown only
by `Agent::reserve_vec`, or as a `Charge` (`heap/accounting.rs`), whose drop gives the bytes back
at once; the kept stack's room is a `KeptRoom` holding a `Charge`, ended on every way out by
`Agent::with_kept_room`. A list charged by hand at its exact length is allowed only when this
paragraph names it: the activation's copy of the arguments, the spread and rest lists, the
`apply` list, the bound arguments, `param_strings` of `CreateDynamicFunction`, the descriptors
of `Object.freeze` and `Object.seal`, `KeyList`, `SourceCopy`, the text of `JSON.parse`, the
positions of `replaceAll`, the buffers of `normalize`, the captures and group names of
RegExpBuiltinExec, and the list of `Agent::coerce_all`. Each is bounded by its count of
elements, which steps or the arguments bound. One set is charged per entry and not by its
nodes: the cycle set of `JSON.stringify` (`on_stack`, a `BTreeSet`), charged four keys' bytes
in each open frame's charge and bounded by the open frames, which `memory_bytes` bounds through
that charge; the walk has no depth limit. A buffer alive while the engine calls into script is
never left uncharged: it is charged, or the code reads in place and makes none. Three are
bounded by a limit and not charged: the list of activations, at most `reentry_depth` + 1
entries (at 256, 257 × `size_of::<Activation>()` = 22,616 bytes); the spare capacity of the frame
stack, at most `frame_depth` frames, each charged `frame_bytes()` (spare at most `frame_depth` ×
`size_of::<Frame>()` = 2,080,000 bytes at `frame_depth` 10,000); and the growth of the kept stack by
`keep` outside a room, a fixed count of values per running builtin (at most 10 values, so at
`reentry_depth` 256 at most 257 × 10 × `size_of::<Value>()` = 41,120 bytes). A copy of n code
units charges n steps before it is made, unless its caller charged those units first; the URI
functions copy first and charge each unit as their loops read it; the copy of a call's n
arguments by `Call(n)` and `New(n)` charges no step, since each was pushed by an instruction
that spent one.

The frontend charges through one ledger: a trait of `swell-syntax` with three operations,
charge, credit and poll, which `swell-compiler` takes too and which `swell` implements once,
over the agent's accounting and its interrupt flag; there is no second budget. The lexer, the
parser, the early errors, the scope analysis and the compile step charge every growth through
it before it is made. A refused charge stops the step: everything charged in it is credited,
no `SyntaxError` object is made, and the entry ends in `Terminated(MemoryBudget)` (from
`parse_script` as `Parsed::Terminated(MemoryBudget)`). Poll reads the interrupt flag as the
step budget's checks do. The ledger lives in the parser's state, never in a recursive frame.

Two readings. `heap_bytes()` reads what the accounting has charged, the number `memory_bytes`
bounds: the arenas' capacity with the collector's workspace, the atom table's capacity, what the
records own, and what a running step holds charged. `live_bytes()` reads what the records the
heap holds own, the fixed size of each one's slot included, and what the atom table's entries
own; it leaves out free slots and the table's empty buckets. A suspended frame counts in both
readings as part of what its holder owns. After a collection these are the live records. It is at most
`heap_bytes()`, it falls by everything a sweep frees, and the collection threshold follows it
("Collector"). Both readings count the bytes the host charged with `charge_host_bytes` while it
holds their `HostBytes` (added on 2026-11-22); a dropped `HostBytes` is credited after the next
sweep or at the host's next charge, or at once by `Agent::credit_host_bytes`, which empties it.

**Queued jobs.** Added on 2026-11-23 for slice 10. What a host sees: a promise job costs the
memory budget from the moment it is queued until the moment a checkpoint takes it off the queue,
and nothing in the queue after that, whether it then runs or a termination ends the checkpoint
first; a host microtask costs its slot of the queue's capacity, and a refused growth answers
`Err(HostError::MemoryBudget)` with nothing queued. So after a script that queues 100 reaction
jobs, `heap_bytes()` exceeds the reading before it by at least 100 times the bytes of one entry,
and the checkpoint that runs them gives those bytes back. The mechanism: the queue is a
`VecDeque` of entries of one fixed size in the agent (`host/jobs.rs`). A promise job's entry
holds its reaction job (the reaction, `sec-promisereaction-records`: its capability or none, its
type, and its handler, none or a JobCallback record, `sec-jobcallback-records`, of a function and
the [[HostDefined]] `host_make_job_callback` answered; and the argument) or its resolve-thenable
job (the promise, the thenable and the JobCallback of `then`), its realm or none
(`sec-newpromisereactionjob`, `sec-newpromiseresolvethenablejob`), the realm current when it was
queued, which is the realm of the entry while a job whose realm is null runs, and the active
script or module when it was queued, which the job makes active again as the draft requires
("scriptOrModule is the active script or module at the time of job's invocation",
`sec-hostenqueuepromisejob`). A host microtask's entry holds its realm and its `Microtask<H>`.
A promise job's entry is charged by its size with `charge_bytes` before it is stored, and
credited when the checkpoint takes it off or `Agent::clear_microtasks` drops it; an entry a
terminated checkpoint left queued stays charged. The checkpoint puts the job's values on the kept
stack (`object/builtin_function.rs`, `Agent::keep`), where a running builtin keeps what it needs
across a call into code, before it calls a hook or code. The queue is a root: its site,
`host/jobs.rs trace_roots`, called from the root site of `host/agent.rs`, reports every value
and realm of every promise job, the script or module it will make active and the [[HostDefined]]
of its JobCallbacks, through `HostData::trace` as a realm's and a script's [[HostDefined]] are
traced; and the realm of every host microtask. So everything a queued job holds survives every
collection until the job is taken off. A host microtask's box is not charged: it is the host's
allocation and holds Rust state the collector cannot see, so an engine value it needs it holds
in `Root`s of the host's table, a root already. It is the host's own storage, bounded by the
host's calls, as the entries of the root table are ("Memory budget", the rule), and a script
grows it only through a host function that queues one, a step per call; every entry taken off
costs a step. Dropping the agent drops the queue with every closure in it. The queue's buffer
grows by the engine's own doubling with `try_reserve_exact`, with no smaller fallback; its
capacity is charged by a `Charge` before it grows, credited when a checkpoint that leaves the
queue empty shrinks a buffer of more than 64 entries to 64, and credited whole when the queue is
dropped. While a promise job is queued its slot is charged twice, in the capacity and in its
entry. A refused growth for a promise job credits the entry's bytes and ends the entry in
`Terminated(MemoryBudget)` with nothing queued; for a host microtask `queue_microtask` answers
`Err(HostError::MemoryBudget)` with nothing queued. The reactions a
pending promise holds are charged with the promise, in a box of its kind, by capacity as its
lists grow; settling it empties the lists and credits them, and the jobs made from them are
charged as above. Rejected: charging a host microtask's box (the closure is the host's
allocation, which the engine cannot size); leaving the queue's capacity uncharged (2026-11-24: a
script that queues jobs would hold a buffer it sized across every job it runs).

**A built-in function with slots.** Added on 2026-11-23 for slice 10. What a script sees, the
standard's: CreateBuiltinFunction with additionalInternalSlots (`sec-createbuiltinfunction`)
makes functions whose steps read slots of their own: the resolving functions of a promise
([[Promise]] and [[AlreadyResolved]], `sec-promise-resolve-functions`), the executor of
GetCapabilitiesExecutor ([[Capability]]), the two functions of an `await` (decision 3, "`async`
functions on suspended frames", item 3), and later the element functions of the statics and the
functions of `finally`. What a host sees: such a function costs its object and its slots from
its creation until the collection that frees it, as any object costs what it owns. The
mechanism: the slots lie in a box in the built-in function kind (`object/builtin_function.rs`, a
variant of `Behaviour` with its id and the box), so the record of every object keeps its 200
bytes (the test of `object/generator.rs`). The box has the fixed size of its type, and its bytes
are part of what the record owns: charged with the record when CreateBuiltinFunction stores it
(`Heap::new_object` with those owned bytes, as a RegExp object owns the box of its program,
`regexp_owned_bytes`), and credited by the sweep that frees the record. A slot that two functions
share and that holds no engine value, the [[AlreadyResolved]] record of the resolving functions,
is an `Rc<Cell<bool>>` counted in the box of each, so it is charged twice and never less than it
holds. The kind site, `BuiltinFunction::trace`, traces every value of the slots; no slot is a
root. Rejected: the slots in the record (every object would grow); a table of slots beside the
heap (a second place to trace and to credit).

**Source text.** Added on 2026-11-21 for slice 3. The source text of a script, of eval code
and of a dynamic function stays alive as long as a function made from it, and is charged to
the memory budget with the compiled code. The text is the code units the parse read: the
source given to ParseScript, the String PerformEval parsed, and for CreateDynamicFunction the
whole text of its last parse, `function anonymous(` P `\n) {\n` body `\n}`
(`sec-createdynamicfunction`). It is kept with the compiled code made from it, which lives
while the host holds the script, while that code runs, and while a function object made from
it is reachable (its [[ScriptOrModule]], "Collector" above). The copy is charged before it is
made, at the start of the parse, at least one byte per code unit in either stored form of
decision 5, and its charge passes with the compiled code's to the script record when the
parse and compile step that made the code ends; the sweep that frees the compiled code
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

**Symbols.** Added on 2026-11-22 for slice 6. What a script sees, the standard's
(`sec-symbol-objects`): `Symbol(d)` makes a new Symbol at each call, whose [[Description]] is
ToString(d), or undefined when `d` is undefined (`sec-symbol-description`); `new Symbol()` throws a
`TypeError`. `Symbol.for(k)` answers the Symbol of the GlobalSymbolRegistry whose key is
ToString(k), made and added the first time, and `Symbol.keyFor(s)` answers that key, or undefined
for a Symbol `Symbol.for` did not make (`sec-symbol.for`, `sec-symbol.keyfor`, `sec-keyforsymbol`).
The registry is the agent's, the append-only [[GlobalSymbolRegistry]] of its Agent Record
(`sec-agents`, `sec-globalsymbolregistry-records`): every realm of the agent gets the same Symbol
for a key. The well-known symbols (`sec-well-known-symbols`) are agent values as well
(`realm/mod.rs`), twelve in slice 6 and thirteen from slice 10, whose story of `for await` adds
`Symbol.asyncIterator` (`sec-symbol.asynciterator`), made with the agent as the other twelve are
(corrected on 2026-11-23 from "twelve in slice 6 (`Symbol.asyncIterator` comes with slice 10)");
none of them is in the registry. A host names it as it names the others, with
`well_known_symbol(WellKnownSymbol::AsyncIterator)` (`host-interface.md` decision 7, "The closed
list", HOST-203). A Symbol is never converted to a String or a Number implicitly: ToString and
ToNumber of it throw a `TypeError` (`sec-tostring`, `sec-tonumber`), and so do `"" + s`,
`` `${s}` ``, `+s`, `s < 1` and every builtin that passes it to them or to an operation built on
them (ToIntegerOrInfinity, ToLength, ToNumeric). ToPropertyKey takes it as it is, a property key;
`String(s)`, `s.toString()` and `s.description` answer without a throw (SymbolDescriptiveString,
`sec-symboldescriptivestring`; `sec-string-constructor-string-value`). ToObject of a Symbol makes a
Symbol object, an ordinary object whose [[SymbolData]] is the Symbol
(`sec-properties-of-symbol-instances`), whose prototype is `%Symbol.prototype%` of the current
realm.

The mechanism:

1. A Symbol is a record of the `symbols` arena (`heap/mod.rs`, `SymbolRecord`) whose
   [[Description]] is a String handle or none; two Symbols are the same exactly when their
   handles are.
2. The GlobalSymbolRegistry is a part of the heap, beside the atom table: a keyed map
   (`heap/map.rs`, `KeyedMap`, with the heap's hasher, keyed by `hash_key` alone, HOST-145) from
   the code units of a key to its Symbol, whose [[Description]] is the key's String. The record of
   a registered Symbol says that it is registered, so `Symbol.keyFor` answers its description with
   no second map. The registry is a root of the collector, traced at a root site of the layout of
   `heap/collect.rs`: every registered Symbol, and its key with it, lives as long as the agent, and
   no entry is ever removed.
3. Charges. `Symbol.for` of a new key charges the Symbol's record, the map's copy of the key's
   units and the growth of the map's table (`KeyedMap::table_bytes_for`, before it grows); the
   key's String is charged as any String. No sweep credits them, since none frees them. A script
   that registers a Symbol for a new key in a loop (`for (let i = 0; ; i++) Symbol.for(String(i))`)
   therefore ends in `Terminated(MemoryBudget)`, which it cannot catch, where the same loop over
   `Symbol(String(i))` keeps nothing past a collection; the story of the `Symbol` builtin tests
   both under one memory budget. Steps: the call's, and one per code unit of the key hashed and
   compared, as any string operation.
4. A Symbol object keeps its Symbol in `OrdinaryData` (`object/ordinary.rs`), as the Boolean and
   Number objects keep theirs, traced with the object; thisSymbolValue (`sec-thissymbolvalue`)
   reads it.
5. The conversions of `ops/conversion.rs` are the one place a Symbol meets ToString or ToNumber:
   a builtin converts an argument through them, never by formatting a value in Rust, so a Symbol
   argument gives the `TypeError` wherever the clause converts it.

**Template objects.** Added on 2026-11-22 for slice 6. What a script sees, the standard's
(GetTemplateObject, `sec-gettemplateobject`; `sec-tagged-templates-runtime-semantics-evaluation`):
a tagged template passes its tag a template object, an Array of the cooked strings (undefined for
a part with a NotEscapeSequence) whose `raw` property is an Array of the raw strings; each element
is not writable and not configurable, `raw` is not writable, not enumerable and not configurable,
and both Arrays are frozen. There is one object per site, the TemplateLiteral of the source (its
Parse Node), per realm: every evaluation of a site gives the same object, from every function
object made from its code (closures made in a loop included); two sites with the same text give
two objects, and so does one source parsed twice (two scripts, `eval` of one String twice).

The mechanism:

1. The key of a site is its compiled code and the site's index in it: the compiler numbers the
   tagged templates of a `Code` (a script, eval code or a dynamic function, with all its function
   templates) and keeps a site table of each site's cooked and raw strings as String constants
   (`swell-compiler`, `bytecode.rs`); the instruction that pushes a template object names its
   index.
2. The realm's [[TemplateMap]] is kept with the code: the script record that holds the compiled
   code (`script.rs`, `ScriptRecord`; eval code and dynamic functions have records of their own,
   `dynamic_code.rs`) has an entry per site of its code, empty until the site's first evaluation
   makes the two Arrays, traced by `ScriptRecord::trace`; the instruction finds it through the
   frame's script. That is the realm's map divided by code: a script record has one [[Realm]], and
   every frame that runs its code runs in that realm, since its own code runs in its [[Realm]]
   (ScriptEvaluation, PerformEval and CreateDynamicFunction make a record in the realm they run
   in) and every function object made from it gets the realm of the frame that makes it (the
   current Realm Record of OrdinaryFunctionCreate). Rejected: a map on the realm record keyed by
   code and index, which keeps the template objects of every eval code the realm ever ran for as
   long as the realm lives.
3. The entries are freed with the code: by the sweep that frees the script record, once the host
   no longer holds it, no frame runs its code and no function object made from it is reachable
   ("Source text." above). It is safe: every evaluation of a site runs in a frame of its code, so
   after that no evaluation can ask for an entry again, and an object no evaluation can ask for
   and nothing else references is not live (`sec-liveness`). A template object the tag kept is an
   ordinary object, alive while it is referenced, its `raw` with it.
4. Charges. The site table and its strings are part of the compiled code, charged with it
   ("Memory budget"), and the entry per site of the record is charged when the record is made. The two Arrays and their
   elements are charged as any Array when the site's first evaluation makes them, and credited by
   the sweep that frees them; a later evaluation allocates nothing. Steps: the instruction's, and
   at the first evaluation one per element it defines in the two Arrays (an array loop, decision
   3).

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

**The order `localeCompare` gives: the NFC forms of the two Strings compared by code units.**
Added on 2026-11-23 for slice 5. What a script sees: `S.localeCompare(T)`
(`sec-string.prototype.localecompare`), after RequireObjectCoercible and ToString of the this
value and of `that`, answers -1, +0 or 1 as NFC(S) is ordered before, equal to or after NFC(T)
by their code units, a String before each longer String that starts with it: the order `<`
gives two Strings (`sec-islessthan`), applied to their NFC forms (UAX #15, as `normalize("NFC")`
makes them, `sec-string.prototype.normalize`). Two Strings answer +0 exactly when they are
canonically equivalent, since canonically equivalent Strings have one NFC form: the five pairs
of the clause's Note 2 (`"Å".localeCompare("Å")` and the others) answer +0, and so
does a String compared with itself. Compatibility equivalents stay apart (`"ﬁ"` and `"fi"`
do not answer +0), as the clause recommends. A lone surrogate is a code point with no
decomposition and combining class 0, so it stays in place in the form. For a String of code
units below 0x80 the form is the String, so the answers of slice 3 stay. The order is not a
locale's collation: `"a".localeCompare("B")` answers 1 (0x61 after 0x42) and
`"é".localeCompare("f")` answers 1, where a locale's collation would put both first; the
engine has no host locale, and ECMA-402 is open item A. The answer for two Strings is fixed by
the Unicode version decision 8 pins (open item C): a change of the pin can change it where the
new version gives a code point of either String another decomposition, combining class or
composition exclusion, and nothing else of a release changes it.

Why this meets the clause: the clause leaves the comparison to the implementation and asks for
"a consistent comparator defining a total ordering on the set of all Strings" that returns +0
for "distinguishable Strings that are canonically equivalent". The order of code units over the
NFC forms is a total order on the classes of canonical equivalence, its sign flips when the two
Strings are swapped, and it is transitive; "intended to correspond with a sort order ... of the
host environment's current locale" is an intent, and this host offers no locale. Rejected: the
NFD forms (as consistent, but a longer form for precomposed text, which is most text); NFKC or
NFKD (they honour compatibility equivalence, which the clause recommends against); the Unicode
Collation Algorithm with its default table (a locale-like order from a table that is not
vendored, which belongs to ECMA-402, open item A).

Cost. Steps: one per code unit of each String, charged before it is read, as now; and one per
code unit written into the buffers of the normalization (the full canonical decomposition and
the composed form), charged before it is written; the comparison of the two forms reads no more
units than were written and charges nothing more. Bytes: the normalized forms are made in
buffers of the method, not as Strings of the heap; each buffer grows by capacity with the
engine's doubling and is charged before it grows (a form can be longer than its String, so its
size is not charged ahead), and every byte is credited when the method returns, on every way
out: an answer, a throw and a stop. Past the step budget the entry ends in
`Terminated(StepBudget)`, past the memory budget in `Terminated(MemoryBudget)`; `string_length`
does not bound the forms, since they are not Strings.

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
- **parseInt for a radix other than 10** (decision 6, beside "Parsing"). Radix 10 converts the
  run of digits by `str::parse::<f64>`, correctly rounded, as a literal. Radix 2, 4, 8, 16 and
  32 convert the run exactly and round once to nearest, ties to even (the leading 128 bits of
  the value in a `u128`, the lower digits as one sticky bit, the shift as the exponent). Every
  other radix accumulates `acc * radix + digit` in `f64`, exact below 2^53 and an approximation
  after, as ECMA-262 19.2.5 step 16 allows; IEEE 754 multiplication and addition give the same
  result on every platform.
- **`BigInt`** is written here: 64-bit limbs with 128-bit intermediates (corrected on 2026-11-24
  from 32-bit limbs with 64-bit intermediates: the unsigned type slice 4 takes from
  `to_string.rs` has 64-bit limbs, and one type serves slices 4 and 12; "Big integers in slice
  4" below), schoolbook multiplication
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
  arithmetic, which IEEE 754 makes identical on every platform. `sqrt`, `abs`, `floor`, `ceil`,
  `trunc` and `copysign` are not in that list and come from `std`. Corrected on 2026-11-24: this
  sentence also took `round_ties_even` and `mul_add` from `std`. `Math.round` rounds a tie toward
  +∞ and cannot use the first, and the second is not used at all; the methods an engine crate
  may call, and the check that holds it, are in "Which `f64` methods an engine crate may call"
  below. Rejected: `std`'s functions (platform-dependent); a port of an existing C library
  (third-party code in what ships).
- **The functions of `Math` that IEEE 754 does not fix** (added on 2026-11-24 for slice 4:
  `sin cos tan asin acos atan atan2 sinh cosh tanh asinh acosh atanh exp expm1 log log1p log2
  log10 cbrt hypot pow`; the draft fixes their special values and says "an
  implementation-approximated Number value", `ecma262:sec-math.sin` and its siblings;
  `pow` is `Number::exponentiate`, `ecma262:sec-numeric-types-number-exponentiate`).
  - Where: the module `crates/num/src/math.rs` of `swell-num`, with `crates/num/src/math/` for
    its parts (`dd.rs`, and one file per group below). The methods are the published ones:
    Payne and Hanek's argument reduction, power series on a reduced interval, and double-double
    arithmetic after Dekker and Knuth. The code and every constant are this repository's;
    nothing is taken from a C library's source (the rejection above stands).
  - Double-double. A pair (hi, lo) with hi + lo the value and |lo| at most half an ulp of hi.
    `two_sum` is Knuth's (6 additions); `two_product` is Dekker's, with Veltkamp's split by
    2^27 + 1, because `mul_add` is not used; an argument of magnitude 2^996 or more is scaled by
    a power of two first, so that the split cannot overflow. Add, multiply, divide and square
    root of pairs follow, the root by one Newton step on `sqrt` of the high part. Each
    operation's relative error is at most 2^-100. Every polynomial below is evaluated in
    double-double, by Horner's rule, with each coefficient as a pair (the exact rational rounded
    to `hi`, its remainder rounded to `lo`). Plain `f64` arithmetic is used only where the text
    says it is exact. `swell-num` does not compile for a target whose `f64` arithmetic carries
    extended precision (x86 without SSE2; a `compile_error!` on `cfg(all(target_arch = "x86",
    not(target_feature = "sse2")))`), and rustc does not contract `a * b + c` into a fused
    operation.
  - Reduction and kernels. Every series is cut where its first omitted term is below 2^-80 of
    the result's magnitude on the whole reduced interval; the degrees that gives are stated, and
    a unit that finds another one by the same rule uses it.
    1. `sin`, `cos`, `tan`. For |x| at most π/4 no reduction; above it, Payne and Hanek for every
       argument (Muller, Handbook of Floating-Point Arithmetic, 11.2): the 53-bit mantissa times
       a window of the bits of 2/π (the table holds 1,280 fractional bits, 20 `u64` words: the
       bits up to position 971 + 53 + 192 = 1,216 are needed, with 64 of margin), 192 bits of
       the product kept; the
       quotient modulo 4 and the fraction give r = x - k·π/2 as a pair with relative error at
       most 2^-100, which keeps the relative accuracy for the arguments nearest to a multiple of
       π/2 (the closest a double comes is about 2^-61). One method for every size, so no
       medium-size path with its own constants. Kernels on |r| ≤ π/4: the Maclaurin series of
       `sin` through r^21 and of `cos` through r^22; `tan` is the quotient of the two kernels in
       double-double, `-cos / sin` in the odd quadrants.
    2. `exp`, `expm1`, `sinh`, `cosh`, `tanh`. x = k·ln 2 + r with k the nearest integer, ln 2 as
       three parts of 32, 32 and 53 bits so that k·part is exact for |k| below 2^11, |r| ≤
       ln 2 / 2; the Maclaurin series of `exp` through r^18, in double-double; the result times
       2^k, applied in two factors when 2^k alone is not a normal Number, and rounded once.
       `expm1` of |x| below ln 2 / 2 sums the series from `r + r²/2 + ...` with the leading term
       exact, for relative accuracy near 0; above it, `exp(x) - 1` in double-double; below
       -38.5, -1; above 709.79, +∞. `sinh`, `cosh` and `tanh` are the identities (e^x ∓ e^-x)/2
       and (e^2x - 1)/(e^2x + 1) in double-double over `exp` and `expm1`, with `cosh` and `sinh`
       for |x| above 709 as (e^(|x|/2))²/2 so that the intermediate does not overflow, and `tanh`
       ±1 for |x| of 22 or more.
    3. `log`, `log2`, `log10`, `log1p`. x = 2^k·m with m in [√2/2, √2) (`from_bits`; subnormals
       scaled by 2^54 first); f = m - 1 exactly; s = f/(2 + f) in double-double; log m = 2·(s +
       s³/3 + s^5/5 + ...) through s^29 (|s| ≤ 0.172); the result is k·ln 2 + log m in
       double-double. `log2` and `log10` multiply that pair by 1/ln 2 and 1/ln 10 as pairs.
       `log1p(x)` takes f = x directly when 1 + x lies in that interval, and otherwise reduces
       the exact pair 1 + x (`two_sum`) the same way. Exact results: `log2(2^k)` is k for every
       k from -1074 to 1023, and `log10(10^n)` is n for n from 0 to 22 (the powers of ten that
       are Numbers), because the pair is within 2^-79 of that integer (for a power of two f is 0
       and the error is that of the pair operations, 2^-95) and a double holds it.
    4. `atan`, `atan2`, `asin`, `acos`. The kernel takes y and x as pairs: |y/x| above 1 is
       swapped and π/2 minus the result is taken; c is the nearest of 0, 1/8, ..., 1 to the
       quotient q; atan(c) comes from a table of 9 pairs; u = (q - c)/(1 + q·c), |u| ≤ 1/16;
       atan u = u - u³/3 + ... through u^19; the quadrant adds a multiple of π/2 or π as pairs.
       `asin(x)` is `atan2(x, sqrt((1 - x)(1 + x)))` and `acos(x)` is `atan2(sqrt((1 - x)(1 +
       x)), x)` with the root of the exact pair `(1 - x)(1 + x)`. `atan2` follows the draft's
       table of special values (`ecma262:sec-math.atan2`) before it reduces, and scales y and x
       by a power of two when their exponents differ by 60 or more.
    5. `asinh`, `acosh`, `atanh`. The identities over `log1p` and `sqrt` in double-double:
       `asinh(x) = sign·log1p(|x| + x²/(1 + sqrt(1 + x²)))`, `acosh(x) = log1p(t + sqrt(2t +
       t²))` with t = x - 1 exact for x below 2, `log(x) + ln 2` above 2^28, and `atanh(x) =
       log1p(2x/(1 - x))/2`; `asinh(x)` is x for |x| below 2^-28.
    6. `pow`, which is `Number::exponentiate`. The draft's cases before the last line are written
       as the draft has them: NaN exponent, zero exponent, NaN base, the infinities and zeros
       with their odd-integer rule, an infinite exponent against |base| 1, and a negative
       base with a non-integral exponent (NaN). Then, for a finite non-zero base and a finite
       non-zero exponent: |x|^y = 2^(y·log2|x|), with log2|x| in double-double with relative
       error at most 2^-79 (item 3), the product by y in double-double, then 2^(that) as in
       item 2; overflow answers ±∞ and underflow ±0 by the sign rule (a negative base with an
       odd integral exponent answers the negative). The product |y·log2|x|| is below 2^11 for a
       result that is a Number, so the error of the result is below 2^-68, under 2^-14 ulp: a
       result that is exactly a Number is returned exactly (`pow(2, 10)` is 1024, `pow(10, 2)`
       is 100; `pow(2, -2^31)` is +0 by underflow).
    7. `cbrt`. y0 = `exp(log(|x|)/3)` in double-double, then one Newton step
       y = y0 - (y0³ - |x|)/(3·y0²) in double-double, the sign of x restored; a perfect cube
       gives its root exactly.
    8. `hypot`. The draft's order: every argument is coerced first, then an infinite argument
       answers +∞ (a NaN argument too), then a NaN answers NaN, then all zero answers +0. For
       the rest: the arguments scaled by 2^-e with e the largest exponent (a power of two, so
       exact), the squares by `two_product`, summed in double-double (Neumaier), the pair's
       square root, scaled back. `Math.hypot(3, 4)` is 5.
  - Constants and how a reader checks them. A rational constant (a series coefficient) is
    stored as `f64::from_bits` pairs and a test recomputes each from its defining fraction with
    exact integer arithmetic (the test's own big integer type, which `tests/to_string.rs` already
    has) and compares bit for bit. The transcendental ones (ln 2, ln 10, 1/ln 2, 1/ln 10, π/2,
    2/π with its 1,280 bits, the 9 values of `atan(k/8)`) are recomputed the same way from a
    series that needs nothing but integers: ln 2 and ln 10 by the series of 2·atanh of a
    rational, π by Machin's formula, `atan(k/8)` by its series, each to 1,400 bits. A constant
    that no test recomputes does not ship.
  - The accuracy the engine promises, in ulp of the exact result (the gap between the two
    Numbers that enclose it; 2^-1074 where the result is subnormal), for a finite argument
    whose result is a normal Number: group A, `exp log log2 log10 sin cos atan pow cbrt hypot`:
    less than 1 ulp (faithful: the result is one of the two Numbers that enclose the exact value,
    or it is the exact value when that is a Number); group B, `tan asin acos atan2 expm1 log1p
    sinh cosh tanh asinh acosh atanh`: at most 2 ulp. For a subnormal result: at most 1 ulp of
    its grid. An overflow answers ±∞ when the exact value is above the largest Number by
    more than 1 ulp of it, and the largest Number or ±∞ within that. The results the draft fixes
    are exact in both groups (the special values of 21.3.2, `Number::exponentiate`'s table,
    `cos(0)` and `cosh(0)` are 1, `acos(1)` is +0, `exp(0)` is 1, `tanh(±∞)` is ±1); beside
    them `log2(2^k)`, `log10(10^n)` for n 0 to 22, `cbrt` of a perfect cube, `hypot(3, 4)` and
    every `pow` whose result is exactly a Number.
  - How a test holds the bound with no second implementation. The reference data is committed
    under `data/math/`. One line holds the function, the bits of the argument or arguments (16
    hex digits each) and the bits of the two Numbers that enclose the exact result (equal when
    it is a Number). About 9,000 lines in all. A test holds group A to "one of the two", group
    B to a distance of 2 ulp from the nearer, and each test file takes under a second. The
    tool is not a dependency of the build or of any test: no program of it is in the
    repository. This follows the vendored-data half of decision 8 (a file under `data/` with
    its source stated and its SHA-256 listed in `data/math/SOURCE`, checked by a task
    `math-data-check` on the model of `unicode-data-check`) and not its generator half:
    nothing in the repository derives the files, since that would be a second implementation of
    every function at test time.
  - **Reference data of Math** (decision 6, in the paragraph "How a test holds the bound with
    no second implementation", in place of a single file and its one test). Each of the three
    stories that write `Math` functions in `swell-num` commits its own reference file, in the
    line format and by the procedure of "The tool that makes the lines":
    `data/math/reference-hypot.txt` (`story:swell-num-math-double-double`: the lines of `hypot`,
    the only function it writes), `data/math/reference-exp-log.txt`
    (`story:swell-num-math-exp-and-log`) and `data/math/reference-trig.txt`
    (`story:swell-num-math-trigonometry`). Each file has one line `<sha256>  <file>` in
    `data/math/SOURCE`, which `math-data-check` reads, and is read by the test file of its story
    (`crates/num/tests/math_dd.rs`, `math_exp_log.rs`, `math_trig.rs`, modules of the crate's
    one test binary).
  - The tool that makes the lines (added on 2026-11-24): POSIX `bc` run as `bc -l` (GNU bc
    1.08.2 on the host that made the file), driven by a POSIX `sh` script that is run once
    outside the repository and is not committed; no other language and no third-party crate. The header of
    each reference file and `data/math/SOURCE` name the tool and its version and say the
    procedure in words, which is this one, so that a reader can make any line again with `bc -l`
    alone.
    1. Argument. The bits are read with `ibase=16` as one integer; sign, biased exponent and
       the 52 stored bits give an integer mantissa m and a binary exponent q, and the argument is
       m × 2^q, a dyadic rational that `bc` holds exactly as long as `scale` is at least the
       count of fractional binary digits k (2^-k has k decimal digits). The same for a second
       argument.
    2. Scale. S0 = max(400, 2·(k1 + k2) + 100) + D, with D the count of integer decimal digits
       of the largest argument (`bc`'s `s` and `c` reduce modulo 2π at the working scale, so a
       result keeps about S − D digits: checked on 2026-11-24, `s(2^1023)` at scale 500
       agrees with scale 800 to about 330 digits, and at 700 to about 380). 400 digits are about
       1,330 bits, above the 200 bits the check needs. Every line is computed at S and at S + 60;
       when the two enclosures differ, S grows by 400 and the line is tried again, up to 4,000;
       a line still in doubt is left out and its count is in the header. This is also what covers
       the cancellation of `expm1`, `log1p`, `sinh`, `tanh`, `asinh` and `atanh` for tiny
       arguments.
    3. The function. `bc -l` has `s`, `c`, `a` (arctangent), `l` (natural logarithm), `e` and
       `sqrt`. `sin cos atan log exp` are those. `tan = s(x)/c(x)`. With π = 4·a(1): `asin(x) =
       2·a(x/(1 + sqrt(1 - x²)))`, `acos(x) = 2·a(sqrt((1 - x)/(1 + x)))`, `atan2(y, x) = a(y/x)`
       for x > 0, that plus π (y ≥ 0) or minus π (y < 0) for x < 0, and ±π/2 for x = 0.
       `expm1 = e(x) - 1`, `log1p = l(1 + x)`, `log2 = l(x)/l(2)`, `log10 = l(x)/l(10)`,
       `sinh = (e(x) - e(-x))/2`, `cosh = (e(x) + e(-x))/2`, `tanh = (e(2x) - 1)/(e(2x) + 1)` (the
       sign of x restored for |x| > 0), `asinh(x) = l(|x| + sqrt(x² + 1))` with the sign,
       `acosh = l(x + sqrt(x² - 1))`, `atanh = l((1 + x)/(1 - x))/2`, `cbrt = e(l(|x|)/3)` with
       the sign, `hypot = sqrt(x² + y²)`, `pow = e(y·l(|x|))` with the sign by the parity of an
       integral y and x > 0 or an odd y. Only finite arguments and the lines whose result is
       not fixed by the draft's special values are made by `bc`; those are written by hand into
       the test. A result beyond the Numbers is decided at scale 30 first (|y·l(x)| above 711
       or below -746 for `exp`, `pow`, `sinh`, `cosh`; `bc` cannot hold `e(1e300)`) and is written
       as the enclosure (largest Number, +∞) or (0, 2^-1074) without `e`.
    4. The two Numbers. For the result r (at scale S): the binary exponent e with 2^e ≤ |r| <
       2^(e+1), found by comparing with powers of two, and clamped below at -1022; M = |r| ×
       2^(52 - e) cut to an integer with `scale=0`; the lower Number is M × 2^(e - 52) and the
       upper is (M + 1) × 2^(e - 52), with their bits from integer arithmetic (`obase=16`), the
       sign applied, and the upper being +∞ when M + 1 carries past the largest exponent. When
       |r| × 2^(52 - e) is within 10^-(S - 40) of an integer, the value is taken as exactly
       that integer: both Numbers are it (`bc` cannot say `log2(8)` is 3, it says 2.99...9 or
       3.00...1; a value that is not exact is never that close to a Number).
    5. The arguments. The special values, ±0, ±1, the powers of two and the thresholds of each
       function are listed in the script. The 200 per function are drawn by Knuth's 64-bit
       linear congruential generator (multiplier 6364136223846793005, increment
       1442695040888963407, modulus 2^64, seed 1, computed in `bc` with `%`): the high bits give
       the sign, a binary exponent uniform in the range the header gives for the function, and
       the 52 mantissa bits.
    6. What `bc` cannot do. (a) It does not search: the arguments of `sin cos tan` nearest to a
       multiple of π/2 up to 2^1023 are found in `bc` by continued fractions, all in integers:
       for a binary exponent q, the convergents and semiconvergents of the fractional part of
       2^q·(2/π) (2/π from `bc`'s 4·a(1) at 1,400 digits) whose denominator is a 53-bit
       mantissa; the argument is m × 2^q for the m with the smallest distance, for 40 values
       of q from -20 to 960; a q with no such denominator is left out. The reduction itself
       is served by the scale (S0 above gives 700 digits for 2^1023: 0.1 s a line). (b) It is
       slow for the tiny arguments: m × 2^-1074 needs a scale of 2,100 or more, one second or
       more a line (`l` at 1,500 digits took 1 s), so at most 10 lines per function have an
       argument below 2^-500; the rest of the 9,000 lines take tens of milliseconds each, so
       the run is minutes to tens of minutes of one core, run once and not in a build. (c) It
       has no exponent notation and no bit operations, which is why the bits go through
       integers and `ibase`, `obase`. Nothing else of the list is out of its reach.
  - test262, pinned (`built-ins/Math`, 324 files): no assertion about these functions compares a
    result against a tolerance; a search for `epsilon` and `Math.abs` finds only the value
    constants, `abs` and `sqrt/results.js`, which holds `sqrt` to the correctly rounded result
    (`f64::sqrt` is, by IEEE 754). The assertions are the special values and these exact
    cases: `cosh(±0) = 1`, `cos(±0) = 1`, `exp(±0) = 1`, `hypot(3, 4) = 5`, `log10(10) = 1`,
    `log10(100) = 2`, `log10(1000) = 3`, `log2(2) = 1`, `log2(4) = 2`, `log2(8) = 3`,
    `pow(1, -2^31) = 1`, `tanh(+∞) = 1`. Every one is an exact result above; the bound meets
    all of them.
- **Which `f64` methods an engine crate may call** (added on 2026-11-24). Allowed: `+ - * /`
  and `%` (IEEE 754 and `fmod`: exact), comparisons, `sqrt`, `abs`, `floor`, `ceil`, `trunc`,
  `round` (ties away from zero) and `round_ties_even`, `copysign`, `signum`, `fract`,
  `to_bits`, `from_bits`, `is_nan` and its kin, and `as` casts (saturating, NaN to 0, fixed
  by the Rust reference). Not allowed: every method `std/src/num/f64.rs` marks "Unspecified
  precision" (`powi powf exp exp2 ln log log2 log10 cbrt hypot sin cos tan asin acos atan atan2
  sin_cos exp_m1 ln_1p sinh cosh tanh asinh acosh atanh gamma ln_gamma erf erfc abs_sub`), the
  same on `f32`, `mul_add` (it is documented as one rounding, but where the target has no fused
  instruction it is a call of the platform's library (not checked in the source), and a target
  is known whose `fma` is wrong
  (`core/src/num/f64.rs`, the FIXME on mingw, 2026-11-24); the kernels use `two_product`
  instead), `libm`, `extern "C"`, and `core::arch` and `std::arch`. `f64::min` and `f64::max`
  leave the sign of a zero unspecified (`core/src/num/f64.rs`): a Number's minimum and maximum
  are written with comparisons, as `Math.max` is; a text search cannot tell them from
  the integer methods of the same name, so they are not in the check below.
  - The check: a task `libm-check`, in the gate before the test steps, on the model of
    `forbid-unsafe-check`. It searches every `.rs` file under `crates/*/src/` (tests are not
    searched; they may use `std` as an oracle), skipping a line whose first non-blank
    characters are `//`, with the regular expression for a method call `\.(name)\(` or a
    path call `f64::(name)\(`/`f32::(name)\(` for each name of the not-allowed list but
    `min` and `max`, and for `libm`, `extern "C"`, `core::arch` and `std::arch`. It prints one line
    per hit, `<path>:<line>: <text>`, then `libm-check: <n> forbidden call(s)` and exits 1; with
    no hit it prints `libm-check: no forbidden call under <n> file(s)` and exits 0. A tree
    with no `.rs` file fails too. The search on this tree finds none today. The functions of
    `math.rs` are called by path (`math::exp(x)`), which the search does not match. The story
    that writes `math.rs` (`swell-num-math`) writes the check and adds it to `task check`.
- **`Math.round`** (`ecma262:sec-math.round`): the rule, over `floor`, a comparison and
  `copysign` only. If x is NaN, ±∞ or equal to `floor(x)`, the answer is x (so -0, and every
  value of magnitude 2^52 or more, which is integral, is returned as it is). Otherwise r =
  `floor(x)`, and d = x - r, which is exact (the fractional part of a Number is a Number); y is
  r + 1 when d ≥ 0.5 and r otherwise (r + 1 is exact below 2^52); when y is zero its sign is
  x's (`copysign(0.0, x)`: -0 for -0.5 ≤ x < 0, +0 for 0 < x < 0.5). `floor(x + 0.5)` is wrong
  twice, as the draft's Note 2 says in part: for 0.49999999999999994, x + 0.5 rounds to 1 and
  the answer is 1 where it is 0; and for 2^52 + 1, x + 0.5 is a tie that rounds to the even
  neighbour, and the answer is 2^52 + 2 for an integral x. `Math.round(-3.5)` is -3, `Math.round(2.5)` is 3.
- **`Math.random`** (completed on 2026-11-24 for slice 4; `ecma262:sec-math.random`; the sentence
  here before said only "seeded from the host's randomness", which named no source and no
  generator). The engine reads no randomness of its own and no hook gives it any: the source is
  the agent's `Seeds.random_seed` (32 bytes), given once to `Agent::new`
  (`host-interface.md`, section 3; HOST-146). A call of `Math.random` reads nothing from the host
  and costs the step of the call; it calls no hook and charges nothing else.
  - The generator is xoshiro256** (Blackman and Vigna, 2018), in `crates/num/src/random.rs`:
    four `u64` words `s0..s3`, 32 bytes, never all zero. A step returns
    `rotl(s1 × 5, 7) × 9` (wrapping) and then `t = s1 << 17; s2 ^= s0; s3 ^= s1; s1 ^= s2;
    s0 ^= s3; s2 ^= t; s3 = rotl(s3, 45)`. The Number is `(word >> 11) as f64 × 2^-53`: the top
    53 bits as an integer (exact in `f64`) times a power of two (exact), so it is a multiple of
    2^-53 in [0, 1 - 2^-53], with positive sign (0 gives +0).
  - The state is per realm, each realm's own: the record of the realm holds the four words,
    set when the realm is created. The agent counts the realms it creates, the creation
    index n: 0 for the first, and one more for each later call of `create_realm` that starts,
    one that ends in a termination included, so that an index is never used twice. The four
    words are `siphash24(key = random_seed[0..16], message = random_seed[16..32] followed by n as
    8 bytes, little-endian, followed by the byte i)` for i = 0, 1, 2, 3 (the SipHash of
    decision 4, in `swell-num`), each as a little-endian `u64`; when all four are zero, `s0` is
    1. The realm's sequence is a function of `random_seed`, n and the count of calls in that
    realm: a call in one realm moves no other, and two realms of one agent differ because
    their messages do (two derived states are equal with probability 2^-256), as the draft
    demands. Two agents with the same `random_seed` give the same sequence in the realm of the
    same index, and two with different seeds differ.
  - Rejected: a hook that gives random bytes (decision 3 of `host-interface.md` has none, so
    that a run is a function of its inputs); `std`'s `RandomState` (barred by HOST-2); one
    generator for the agent, which one realm's calls would move for another; xorshift128+, the
    generator some engines use, whose low bits fail linear tests, at no gain in cost; a jump
    ahead per realm (O(n) work for the n-th); a counter of realms that skips a failed creation
    (the index would then depend on what failed).
  - What a test asserts. In `swell-num`: from the state (1, 2, 3, 4) the first three words are
    11,520, 0 and 1,509,978,240 (worked by hand from the step above on 2026-11-24), the Number
    of the word 11,520 is 5 × 2^-53 and of the word 0 is +0, and the derived state for an all-zero
    seed at n = 0 and 1 is written in the test (the story records it from the algorithm as
    written here), so that a change of the derivation fails. In the host's tests: HOST-26,
    HOST-27, HOST-144 and HOST-147 (seeds, realms, two processes), and HOST-212 and HOST-213
    (the range, and the stream of a realm by its index).
- **Big integers in slice 4** (added on 2026-11-24). `Number.prototype.toString(radix)`,
  `toFixed`, `toExponential` and `toPrecision` need exact integer arithmetic wider than `u128`
  for some arguments (a subnormal against 10^324, or against 36^208), and `BigInt` of slice 12
  needs the same. One type: `Nat`, an unsigned integer of any length, 64-bit limbs least
  significant first, `u128` intermediates, no zero limb at the top, in `crates/num/src/nat.rs` of
  `swell-num`. It is the private `Big` of `crates/num/src/to_string.rs` (`from_u64`, `bit_len`,
  `bits_from`, `shl`, the comparison of a sum with a third value and what the digit
  generation uses), moved out and made `pub(crate)`, with no change of behaviour: the
  tests of `to_string` (the digits against Rust's `{:e}` and the round trips) are the proof. The
  story `number-format`, which needs it first (`toFixed`, `toExponential`, `toPrecision`),
  does the move as its first commit, and adds only what those three need: multiplication by a
  `u64` and by a `Nat`, a power of a `u64`, division with remainder by a `u64`, subtraction,
  right shift and conversion to digits in radix 10. Slice 4 writes nothing else: no sign, no
  Karatsuba, no general division; slice 12 puts a sign and the rest on top of the same
  magnitude (`bigint.rs`), and decides how a heap `BigInt` holds its limbs. Nothing outside
  `swell-num` names `Nat` in slice 4.
  - Largest size for these uses: no operand above 2,048 bits (32 limbs). By the arithmetic: the
    shortest-digit conversion of 2^-1074 holds operands below 2^1,200 in every radix from 2 to
    36 (radix^digits scaling is at most about 2^1,100 for the longest expansion); `toFixed` of
    x below 10^21 with 100 fractional digits multiplies a 53-bit mantissa by 10^100 against
    2^1,074, under 1,100 bits; `toExponential` and `toPrecision` of 5e-324 with 101 digits need
    10^424, 1,408 bits against 2^1,074; of 1.8e308, 2^1,024 against 10^208, under 700 bits. A
    test runs those extremes and a test-only counter of the longest operand states 2,048 as the
    bound; no operand grows with a script's input beyond them, so nothing is refused with a
    `RangeError` and no `bigint_bits` applies.
  - Charges. Memory: the temporaries of one call (at most six operands of 32 limbs, 1,536
    bytes) are charged by the builtin as one temporary of 2,048 bytes through the accounting
    function before it calls `swell-num`, and credited when the call returns (decision 4:
    "a temporary structure ... is credited when it drops"). Steps: one step per code unit of the
    text it produces, charged before the String is made, as `Number.prototype.toString` of
    slice 2 charges (`builtins/number.rs`); a digit costs at most 192 limb operations (six
    operands of 32 limbs), so a step covers at most 192 limb operations, and a call is at most
    about 1,100 digits (radix 2 of a subnormal). `swell-num` takes no budget: the bound of the
    operand size is what makes the work finite. `BigInt`'s steps per limb operation
    (decision 3) start with slice 12, whose operands have no such bound.
- **`Math.sumPrecise`** (`ecma262:sec-math.sumprecise`; added on 2026-11-24). The draft's
  algorithm sums in ℝ and rounds once: `𝔽(sum)` is the nearest Number, ties to even, so the
  answer does not depend on the order of the items and may pass through values far above the
  largest Number on the way (test262 has `[1e308, 1e308, 0.1, ..., -1e308, -1e308]`, which is
  0.30000000000000004). Algorithm: an exact accumulator, the fixed-size superaccumulator of the
  family the draft's note names (Neal, "Fast exact summation using small and large
  superaccumulators"), not Shewchuk's expansions, whose size grows with the input. Two
  unsigned integers, one for the positive items and one for the negative, each 34 `u64` limbs
  (2,176 bits), counting units of 2^-1074: a finite Number is an integer mantissa of at most 53
  bits times 2^(exponent - 1074), so adding it is adding a shifted mantissa, which touches two
  limbs and carries as a binary counter does. Every finite Number is below 2^2,098 units and
  fewer than 2^53 items are summed (the draft's count limit), so a sum stays below 2^2,151,
  which 34 limbs hold. The state of the draft (not-a-number, plus-infinity, minus-infinity,
  minus-zero, finite) is kept as written, which decides NaN, ±∞ and -0 and leaves the finite
  sum to the accumulators; at the end, when the state is finite, the larger accumulator minus the
  smaller (34 limb operations) gives the magnitude and the sign, which is rounded to 53 bits
  to nearest, ties to even (the bits above, the next bit and a sticky bit of the rest), scaled
  by 2^-1074, and is ±∞ when it rounds to 2^1,024 or more; an exact zero is +0 (the draft's
  `𝔽(0)`). It lives in `crates/num/src/sum.rs` as a type with `add(f64)` and `finish() -> f64`;
  the iteration is the builtin's (`Iterators held by a builtin`, decision 3).
  - Work and storage per item: at most 4 limb operations, amortized (the carries of a binary
    counter), and a bounded 34 at the end; storage is 544 bytes in the call's frame, the same
    for one item and for 10^9, so nothing reaches the memory ledger. Charge: one step per item
    through `charge_steps` beside the step of the call of `next` (item 3 of "Iterators held by
    a builtin"), which covers the four limb operations, and 34 steps once before the final
    pass. The count of the draft (2^53 - 1) is a `u64` counter that throws the draft's
    `RangeError` and closes the iterator as written; it cannot be reached with a step budget
    below 2^53.
  - An iterable that ends in a budget: a `Terminated(StepBudget)`, `Terminated(MemoryBudget)` or
    `Terminated(Interrupted)` of the iterator's `next` or of the loop's own step runs none of
    the rest of the builtin: no sum is answered and the iterator is not closed (item 4 of
    "Iterators held by a builtin"); the accumulators are on the frame and are dropped with it.
    An item that is no Number is the draft's `TypeError` with IteratorClose; a throw of `next`
    itself closes nothing, as the draft has it.
  - test262: `features.txt` already says `run Math.sumPrecise`, and `built-ins/Math/` is mapped
    to slice 4 (the line `dir test/built-ins/Math/ 4` of section 12), so the story is not a
    move of a feature. The pinned directory has 10 files: `length`, `name`, `not-a-constructor`,
    `prop-desc`, `sum`, `sum-is-infinite`, `sum-is-minus-zero`, `sum-is-NaN`, `takes-iterable`
    and `throws-on-non-number`; `takes-iterable` also lists `generators` (`feature generators
    7`) and `not-a-constructor` lists `Reflect.construct` (`feature Reflect.construct 9`), and
    those two are placed by those feature lines.
- **`**` and `Math.pow`** (added on 2026-11-24). They are one function:
  `swell_num::math::exponentiate(base: f64, exponent: f64) -> f64`, `Number::exponentiate`
  (6.1.6.1.3, `ecma262:sec-numeric-types-number-exponentiate`), with the draft's table of
  special values and item 6 above for the rest. `Math.pow` is the draft's two `ToNumber` calls
  and a call of it (`ecma262:sec-math.pow`); `**` and `**=` get a variant
  `BinaryOperator::Exponentiate` in `ops/operators.rs` (the enum has none today), whose Number
  arm calls it (a BigInt operand is `BigInt::exponentiate`, slice 12). The operator is refused today by the parser (`crates/syntax/src/parser/expression.rs`
  refuses `**` and `**=` naming slice 4) and the compiler folds no arithmetic (nothing there
  evaluates an operator at compile time), so no second copy of the function exists; if folding
  of arithmetic is added, it calls the same function. `-2 ** 2` stays the `SyntaxError` of
  the grammar (`UnaryExpression` cannot be the left operand), a parser matter of the story
  `exponentiation-operator`.
- **`Date`.** The current time is a host answer: the draft says only "the time value (UTC)
  identifying the current time"; HTML's `HostSystemUTCEpochNanoseconds(global)` (8.1.6.5) is the
  hook a browser will give. The time zone is the host's too, through the draft's
  `SystemTimeZoneIdentifier ( )`, `GetNamedTimeZoneOffsetNanoseconds ( timeZoneIdentifier,
  epochNanoseconds )`, `GetNamedTimeZoneEpochNanoseconds ( ... )` and
  `AvailableNamedTimeZoneIdentifiers ( )`. The command-line host and the test262 runner answer
  "UTC" unless told otherwise. `Date.parse` accepts the format of 21.4.1.32 and the forms
  `Date.prototype.toString` and `toUTCString` produce; nothing else (an implementation-defined
  fallback the standard allows and swell does not take); the three forms are written out below.
  How often the engine calls the time hooks, what a call costs and what it does with an answer
  out of range are in `host-interface.md` decision 17. The rest of this decision's `Date` items
  were added on 2026-11-24 for slice 4.
- **`MakeDay` far outside the time range** (`MakeDay`, 21.4.1.28, `ecma262:sec-makeday`). What a
  script sees: the draft asks for a finite time value `tv` (an integral Number) in year
  ℝ(balancedYear), month balancedMonthMV and date 1, and answers NaN "if this is not possible
  (because some argument is out of range)". It does not hold `tv` to ±8.64 × 10^15; only
  `TimeClip` does, in every caller (`Date.UTC`, the constructor, the setters, `Date.parse`), so
  `MakeDay` answers a day number for the year 275760 and for the year 285000 alike, and
  `Date.UTC(275760, 8, 13)` is 8.64e15 while `Date.UTC(275760, 8, 14)` and `Date.UTC(1e300, 0)`
  are NaN. swell answers as the draft does for every balancedYear of magnitude up to 2^53
  (9,007,199,254,740,992): the day number when some Number lies in the day, that is in
  [d × 86,400,000, (d + 1) × 86,400,000) milliseconds, and NaN when none does. Every day holds
  a Number below 2^79 milliseconds, the year 1.9 × 10^13 (from 2^79 on, the gap of two Numbers
  is 2^27 ms, longer than a day); above it a day holds one less often: at the year 2^53 the
  day starts near 2^87.8 ms, the gap is 2^35 ms, and one day in 398 holds a Number (`MakeDay(2^53,
  0, 1)` is NaN, `MakeDay(2^53 - 65, 0, 1)` is not). For a balancedYear of magnitude above 2^53
  `MakeDay` answers NaN. That differs from the draft only for a script that makes the day number
  cancel against a time of its own, as in `new Date(Y, 0, 1, H)` with an `H` of the size of the
  day number's milliseconds, on a year above 2^53 whose first day holds a Number (about one
  in 400 at 2^53, one in 400,000 at 10^19, fewer above); no argument of a setter can
  (`TimeWithinDay` is below 86,400,000, so the sum is far outside the range and `TimeClip` makes
  it NaN), and a search of `test/built-ins/Date` for exponents, powers of two and 2^53 finds no
  year above 10^13. Rejected: a bound at the first year whose millisecond count leaves the safe
  integers (285,426): `new Date(300000, 0, 1, -1e9)` is the day number 108,853,222
  (9.4 × 10^15 ms, above 2^53, in a day that holds Numbers) plus -3.6 × 10^15, a date near
  5.8 × 10^15 in the range, which that bound would make NaN with ordinary arguments; and a
  bound above 2^53, which only moves the deviation to years no script writes and costs wider
  integers. The mechanism: `swell-num` reads balancedYear as an integer only when it is
  integral and of magnitude at most 2^53, so the year and the day number (about
  365.2425 × 2^53, under 2^62) fit `i64`; the day's first millisecond, `d × 86,400,000` (under
  2^89), is computed in `i128`; whether the day holds a Number is one comparison of that integer
  with the next Number at or above it; the first day of the month comes from the closed form of
  `DayFromYear` and a table of month starts, with no loop over years or months. `dayMV` is added
  in Number arithmetic as the draft's last step has it. `crates/num` does this today
  (`make_day`, `day_holds_a_number`); its module header calls the bound "not decided" and is
  rewritten to cite this item.
- **`UTC ( t )` and the local time that does not exist** (21.4.1.26, `ecma262:sec-utc-t`).
  What a script and a host see: for a local time that repeats, the first instant; for one that
  a positive transition skips, the offset in force just before the transition; for every other
  local time, its one instant. The draft finds the skipped case's offset from `tBefore`, "the
  largest integral Number < t for which possibleInstantsBefore is not empty", and states no
  bound on the search. swell's search is bounded: it asks `get_named_time_zone_epoch_nanoseconds`
  at most 57 times for one `UTC ( t )`, and when it finds no `tBefore` within 2^28 milliseconds
  (268,435,456 ms, 74.5 hours) below `t`, `UTC ( t )` answers NaN, so `new Date(y, m, d)` is
  then a Date whose time value is NaN, never an exception and never a hang. The bound has two
  grounds. A host that keeps to the offset range of `host-interface.md` decision 17 has offsets
  below 24 hours on each side of a transition, so a gap is shorter than 48 hours
  (172,800,000 ms, below 2^28); a real zone's longest gap is one day. And a host that does not
  (one that answers the empty list for every local time, a broken or hostile host) gets NaN
  after a known number of calls instead of the draft's search without end. NaN rather than the
  offset of some instant: no instant before `t` was found, so any offset would be one the host
  never gave for that time, and the Date still exists. The mechanism: `t` must be integral and
  within 8.64 × 10^15 + 86,400,000 in magnitude, else NaN with no hook call (an offset below
  86,400,000 ms puts `t - offset` outside the range, so `TimeClip` of every caller would make
  it NaN; `LocalTime` of a time value stays inside the same limit). The
  common path asks `get_named_time_zone_epoch_nanoseconds` once with the fields of `t` and
  `get_named_time_zone_offset_nanoseconds` once with the first element. If the list is empty
  the search runs: the probe times are `t - d` for d = 1, 2, 4, ..., 2^28 (29 probes, the first
  non-empty one ends the gallop), then a bisection between the last distance that gave an empty
  list and the first that did not, one probe per halving of an interval of at most 2^27
  milliseconds (27 probes), keeping the list of the lowest distance that gave a non-empty one;
  that list's last element goes to the offset hook. The most is 1 + 29 + 27 = 57 asks of the
  epoch hook; with no `tBefore` it is 1 + 29 = 30. A probe time outside the limit above counts
  as an empty answer and makes no call. The search is exact when the empty answers just below
  `t` form one run followed by non-empty ones (every real zone); for a host whose answers do
  not, the result is the boundary the probes met, the same for the same answers (HOST-133).
  Time spent: decision 3 and `host-interface.md` decision 17 charge one step for each hook
  call.
- **`Date.parse` and the String argument of `new Date`** read exactly three forms, each the
  whole String, with no space before or after and with the letters as written below; anything
  else is NaN, with no fallback. They are told apart by the first code unit, so no form is tried
  and abandoned: a digit, `+` or `-` starts the first form, a letter the second or third (by
  the fourth code unit: a space or a comma).
  1. The Date Time String Format of 21.4.1.32 (`ecma262:sec-date-time-string-format`):
     `YYYY`, `YYYY-MM` or `YYYY-MM-DD`, optionally followed by `THH:mm`, `THH:mm:ss` or
     `THH:mm:ss.sss` (exactly three digits), and after a time, optionally `Z` or `+HH:mm` or
     `-HH:mm` (a date-only form takes no offset: `2020-01-01Z` is NaN). A year is four digits, or `+` or `-` and six digits (`ecma262:sec-expanded-years`);
     `-000000` is NaN. MM is 01 to 12 and DD is 01 to the length of that month in that year (a
     day the month does not have, `2021-02-30`, is an illegal element value and NaN; the draft
     says only 01 to 31, and a reader of "out-of-bounds elements" has no other number to name),
     HH is 00 to 24, mm and ss 00 to 59, and an offset's HH 00 to 23 and mm 00 to 59 (the draft's
     offset format). HH of 24 is allowed only with mm, ss and sss all zero and reads as 00:00 of
     the next day (Note 1 of 21.4.1.32). A date-only form is read as UTC. A date-time form
     with an offset is read at that offset and calls no hook. A date-time form without an offset
     is read as local time, `UTC(MakeDate(MakeDay, MakeTime))` through the conversion of
     `host-interface.md` decision 17, with at most 59 hook calls. The value is the result of
     `TimeClip`, so an instant outside the time range is NaN, `+275760-09-13T00:00:00.001Z`
     included, and `+275760-09-13T01:00:00+01:00` is 8.64e15, since the offset is taken off
     before the clip.
  2. The form of `Date.prototype.toString` (`ToDateString`, `ecma262:sec-todatestring`, with
     `DateString`, `TimeString` and `TimeZoneString`): `Www Mmm DD Y HH:mm:ss GMT±HHmm`
     optionally followed by ` (name)`. `Www` is one of `Sun Mon Tue Wed Thu Fri Sat` and `Mmm`
     one of `Jan` to `Dec` (tables 61 and 62, `ecma262:sec-todatestring-day-names`,
     `ecma262:sec-todatestring-month-names`), written as there; DD is two digits; Y is
     four to six digits, with a `-` before them for a year below 0 (`DateString` pads to four
     and writes a sign only when negative, and 271,821 and 275,760 have six digits; `-0000` is
     NaN); HH, mm and ss are two digits (HH 00 to 23: `TimeString` never writes 24); the offset
     is `+` or `-`, HH 00 to 23, mm 00 to 59, with no colon. The name is the one place the
     draft leaves to the implementation (`ecma262:sec-timezoneestring`, "an implementation-defined
     time zone name"): swell's own `toString` writes none, since no hook answers a name, and
     the parse accepts
     a space, `(`, one or more code units other than `(` and `)`, and `)` as the last code unit
     of the String, and ignores them. The weekday is read and not checked against the date. The
     instant is the fields read as a time at the String's offset, no hook, no zone of the host.
  3. The form of `Date.prototype.toUTCString` (`ecma262:sec-date.prototype.toutcstring`):
     `Www, DD Mmm Y HH:mm:ss GMT`, with the same `Www`, `Mmm`, DD, Y and time as the second
     form, read as UTC.

  DD, MM, HH, mm and ss out of their ranges, a month name or weekday name not in the tables, a
  second space, a missing element and any extra code unit after the last element are NaN. Not
  accepted, so NaN: the forms `toDateString` and `toTimeString` produce, and so what
  `toLocaleDateString` and `toLocaleTimeString` produce, which are those forms (the next
  bullet); what `toLocaleString` produces is the second form and is accepted. An expanded
  year whose instant is outside the range is NaN with no fallback (`sec-expanded-years`), and
  the fallback of 21.4.3.2 is not taken. The work: one pass over the
  String with no backtracking, charged as every String operation is (decision 3): a String of up
  to 32 code units costs the step of the call and each code unit past 32 one step, charged before
  the first is read, so the name of the second form, the only part without a length bound, is
  paid for by its length; the date-time form without an offset adds the steps of its hook
  calls.
- **The text of toLocale*String** (decision 6, beside the `Date.parse` bullet).
  `Number.prototype.toLocaleString` gives the text of `Number.prototype.toString()` of the same
  Number, and `Date.prototype.toLocaleString`, `toLocaleDateString` and `toLocaleTimeString`
  give the text of `toString`, `toDateString` and `toTimeString` of the same Date, each through
  the same one conversion; there is no locale behaviour (`Intl` is excluded). `Date.parse` reads
  the text of `toString`, `toUTCString` and `toISOString`, so `Date.parse(d.toLocaleString())`
  is `d` to the second; it answers NaN for the text of `toDateString`, `toTimeString`,
  `toLocaleDateString` and `toLocaleTimeString`.

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

**How `swell-regexp` charges its work.** Added on 2026-11-23 for slice 5. What a host sees: the
parse, the compile step and the match of a pattern spend the step budget and the memory budget
of the entry and read its interrupt, as every loop of decision 3 does. A match that passes the
step budget ends the entry in `Terminated(StepBudget)`, one whose stacks pass the memory budget
in `Terminated(MemoryBudget)`, and one that reads the interrupt in `Terminated(Interrupted)`; no
`catch` or `finally` block runs, and nothing the match charged stays charged. The check of a
literal at parse time charges the parse's memory, reads its interrupt and spends no step
(`Parsed::Terminated(...)` on a stop, as any parse).

The mechanism:

1. The crate sits below `swell-syntax` and `swell` and names neither, so it takes its budget
   from its caller through a trait of its own (`crates/regexp/src/budget.rs`) with five
   operations: charge bytes before the growth they pay for, or refuse and charge nothing;
   credit bytes charged before; grant steps, asked for 1 to 4,096 at a time, which spends and
   answers the smaller of the number asked and the steps left, at least 1, or refuses when no
   step is left, and reads the interrupt first; return steps granted and not run; and poll the
   interrupt. A refusal is a stop of the crate: over the memory budget, over the step budget,
   or interrupted. After a stop the crate credits everything it holds and answers the stop,
   never an error of the pattern. A type that charges nothing and never stops, as
   `swell_syntax::Unlimited`, is for tests and tools. Growth is by capacity, as `Meter::grow`
   does (`crates/syntax/src/ledger.rs`): a vector doubles to at least 4 elements with
   `try_reserve_exact`, and a refusal of the allocator is a refused charge.
2. Two callers implement it. `swell-syntax`, for the literal its early errors check, over the
   `Meter` of the parse (decision 4, "Memory budget": the frontend's one ledger): a charge and a
   credit go to the meter, a poll reads the interrupt through it, and a grant is answered in
   full and counted nowhere, since a parse spends no step and ParsePattern asks for none.
   `swell`, for `RegExp` and its matching, over the agent: bytes to the accounting, steps to
   the counter `steps_per_entry` bounds (a grant is never more than the steps left, so the
   refused grant is the step that would pass the budget, as for `Agent::step`; a return lowers
   the counter by what was not run), the interrupt flag read and cleared. It borrows the
   accounting, the step counter and the flag apart from the arena of Strings, so the matcher
   reads its subject in place while it charges, and the builtin ends the entry with the stop's
   termination when the crate answers one.
3. Positions are code unit indices, in every mode, `u` and `v` included. The subject is the
   code units of a String, read in place in either stored form of decision 5 and never copied
   for a match; the start index a caller passes and every position a match answers (the start
   and end of each capture, group 0 the match) count UTF-16 code units, as `lastIndex`, `index`
   and `indices` do (`sec-regexpbuiltinexec`, `sec-advancestringindex`). Under `u` and `v` the
   matcher reads a code point at a code unit position (a surrogate pair is two units and one
   code point, a lone surrogate one unit) and moves by its length, and a search moves from one
   try to the next by AdvanceStringIndex's amount. The draft's matcher indexes a list of code
   points and converts its end with GetStringIndex (`sec-getstringindex`); this one indexes the
   code units, so no side converts anything. A start index on the second unit of a pair under
   `u` or `v` starts the first try at the pair's first unit (the character "obtained from
   element lastIndex of string"), and the caller keeps its `lastIndex` for `index`, as the
   clause does.

**What one step of the matcher is.** Added on 2026-11-23 for slice 5, the cost model in the
form of "The step budget" (decision 3). A match spends one step per start index its search
tries (a sticky search tries one), one per instruction it runs, one per entry it takes from
the backtrack stack, and one per code unit a backreference compares, charged before the
comparison. `last_entry_steps()` counts exactly these for a match, beside the steps of the
instructions and builtins around it. Example: the compiler emits `/a/` as two instructions, the
code unit `a` and the end of the match; `/a/.exec("ba")` spends 5 steps of the matcher: index 0
tried (1), the code unit fails (1) with nothing to backtrack, index 1 tried (1), the code unit
(1), the end of the match (1). A step stands for a bounded amount of work: no number of the
pattern (its groups, quantifiers, lookarounds, ranges or names) multiplies the work of one. A
try writes one capture, the start of the match, whatever the number of groups: the first try
finds every capture unset as the search made them, charged by their bytes, and a try that
fails has taken every entry of its backtrack stack, one step each, and so undone every change
it made. The instruction that clears the captures of a quantified atom at each iteration
spends one step per group of its range, its own step the first of them, charged before it
reads a capture; a named backreference whose name n groups have spends n steps the same way,
beside the code units it compares. Example: `/(?:a|(b)(c))*/.exec("a")` clears its two groups
twice, at the iteration that matches `a` and at the one that fails, and spends 20 steps, 2 of
them for the second group of each clearing. The test of a class is a binary search over its
ranges, which are sorted, disjoint and apart, so at most 557,056 of them and at most 20
probes. A pattern parsed at run time (`RegExp`, a literal's evaluation, the `String` methods
that make a RegExp) costs one step per code unit of the pattern, charged by RegExpInitialize
before ParsePattern, as PerformEval charges eval code; the parser and the compiler charge no
step of their own. Their work is linear in that length: a bounded number of nodes per code
unit and of instructions per node (a quantifier `{n,m}` is a counter, not n copies; the
clearing of the captures of a quantified atom names a range of capture numbers, since the
groups inside an atom are numbered one after the other), but for the sort of a class's ranges,
n log n for n ranges, the sort of the group names with the search of a named backreference's
name among them, n log n and log n comparisons for n names, and the check of a duplicate group
name, a union-find over the levels of the parse with path compression, at most n log n level
visits for n levels. Both poll the interrupt at least once per 4,096 code units read and per
4,096 nodes visited.

The mechanism: the matcher takes its steps by grants of at most 4,096 before it runs them. It
runs each unit of work against what it was granted and asks for the next grant when that is
spent; a backreference that needs more than is left takes further grants first. When the match
ends with a match or with no match, it returns what it was granted and did not run, so the
steps charged are the units run, and the interrupt is read at least once per 4,096 of them. A
grant smaller than asked means the budget is nearly spent; the refused grant after it stops
the match with `last_entry_steps()` equal to `steps_per_entry`. The backtrack stack, the
captures and the counters grow by capacity, each growth charged before it is made, so the
backtrack stack is charged before it grows, and every byte is credited on every way out of the
match: a match, no match and a stop. The match array, its `groups` and its `indices` are
charged as any Array and object when RegExpBuiltinExec builds them from the positions.

**Where a RegExp object keeps its program.** Added on 2026-11-23 for slice 5. What a script
sees, the standard's (`sec-properties-of-regexp-instances`): [[OriginalSource]] and
[[OriginalFlags]] through `source` and `flags`, and [[RegExpMatcher]] with [[RegExpRecord]]
through `exec` and the methods built on it; nothing of the program itself. What a host sees:
`heap_bytes()` holds the program's bytes while the object lives, and the `collect()` that frees
the object gives them back.

The mechanism:

1. A RegExp instance is an ordinary object whose slots `OrdinaryData` cannot hold, since that
   is `Copy` and a program is not. It is a kind of object of its own, `ObjectKind::RegExp`, as
   an Array keeps its elements and an ECMAScript function its code in theirs (`object/mod.rs`):
   the record holds [[OriginalSource]] and [[OriginalFlags]] as two String handles, and the
   program as a box of its own: the instructions, the flags of [[RegExpRecord]], the number of
   capturing groups and their names. Its tag for the dispatch is a `KindTag` of its own that
   every internal method takes by its ordinary arm. Kind site of the collector: the two
   Strings; the program holds no handle. Not kept: [[LegacyFeaturesEnabled]] and the [[Realm]]
   of RegExpAlloc, which only `legacy-regexp` (excluded) and `RegExp.prototype.compile` read.
   Rejected: a variant of `OrdinaryData` that holds a handle into an arena of programs, a
   seventh arena with its two sites of the collector for a record that one object names.
2. Charges. The compile step charges the program through the trait as it grows; when the
   object is stored that charge becomes the bytes the record owns (`Heap::new_object_charged`,
   as a script record takes its code's charge), and the sweep that frees the record credits
   it, never earlier. A compile that stops or refuses credits what it charged. The two Strings
   are charged as any String: those of a literal are String constants of its code, kept by its
   script record, and are not copied; the constructor's are the results of ToString.
3. Two objects never share a program, also when they come from one literal (the next
   paragraph).
4. What `RegExp.prototype.compile` (B.2.4.1, `sec-regexp.prototype.compile`, slice 18) needs:
   RegExpInitialize on an object that exists, so the three slots of the record are replaced in
   place, the new program charged before it is made and the old one credited when it is
   replaced, with the bytes the record owns changed by the difference.

**A regular expression literal: checked at parse time, compiled at each evaluation.** Added on
2026-11-23 for slice 5. What a script sees, the standard's: a pattern that is not valid with its
flags is an early error of the script, the module, the eval code or the body given to `Function`
(`sec-primary-expression-regular-expression-literals-static-semantics-early-errors`), and each
evaluation of a literal makes a new RegExp object by RegExpCreate of its body and flags
(`sec-regular-expression-literals-runtime-semantics-evaluation`): an object of its own, with its
own `lastIndex` at 0, also in a loop and from every closure of one code. The program is not
observable: whether two objects share one, and whether it was compiled at parse time or at the
evaluation, no script can tell; only the steps and the bytes differ.

The rule:

1. At parse time the early errors run ParsePattern of the body with the flags and the limit
   `parser_nesting`, and drop the tree and credit it as soon as the check ends; the code keeps
   the body and the flags as two String constants and nothing else of the pattern.
2. Each evaluation is one instruction that runs RegExpCreate in the realm of the running code:
   RegExpAlloc, then RegExpInitialize of the two constants, which parses and compiles the
   pattern again. It pays the instruction's step, one step per code unit of the body before the
   parse, and the property operations of RegExpAlloc and RegExpInitialize (`lastIndex`); and
   the bytes of an object record with one property and of the program, linear in the body's
   length. A literal of L code units evaluated n times in a loop pays n × (L + 1) steps and
   those operations, and makes n programs, which the collector frees with their objects. The
   parse at the evaluation meets no error of the pattern, since the text, the flags and the
   limit are those of the check; it can meet a refusal of the compile step (`i`, "Refusals
   until the later stories of slice 5" below), where the standard's `! RegExpCreate` cannot
   throw, and a stop of the budget.
3. Why not one per site, as the template objects are ("Template objects.", decision 4): a
   template object's identity per site is what the standard asks and a script reads, so it is
   kept in the script record; a program's is not, so the standard's steps are followed as they
   are. A program per site would add an entry per literal to the script record, a program
   shared by objects whose charge outlives either owner, and a second form of the slot that
   `RegExp.prototype.compile` must replace for one object only. Rejected for now: one program
   per site in the script record, shared by its objects. It changes no answer, only steps and
   bytes, and comes as a measured optimisation: the steps and the bytes per evaluation of a
   literal in a loop, before and after.

**No Rust recursion in the parser and the compiler of a pattern.** Added on 2026-11-23 for
slice 5. What a script and a host see: a pattern nested deeper than `parser_nesting` is the
`SyntaxError` whose message is `nesting deeper than parser_nesting (<limit>)`: for a literal an
early error (through the host `Parsed::Errors`, HOST-142; through `eval` and `Function` a
`SyntaxError` the script catches, as HOST-139 and HOST-141 hold for the script's own nesting),
through `RegExp` a `SyntaxError` of its realm that the script catches (HOST-143). It is not a
refusal and is not counted. A pattern within the limit parses and compiles without a stack
overflow on every thread that holds `Limits::min_stack_bytes()`, and nothing ends in a
`RangeError`.

The mechanism: loops with explicit stacks, not recursion counted against the limit. The parser
keeps the open levels on a stack of its own, one entry per open group (capturing, named or not
capturing), lookaround and modifier group, and under `v` per nested class, charged through the
trait before it grows; the level that would pass `parser_nesting`, counted from zero at the
start of the pattern, is the error. The tree's nodes live in vectors and name their children
by index, as the script's tree does, so dropping it does not recurse; the compile step walks
it with a worklist, and the program is one vector of instructions. The frames of ParsePattern
and CompilePattern therefore do not grow with the pattern: `STACK_BYTES_PER_LEVEL` and
`STACK_BYTES_FIXED` of `swell-syntax` and `Limits::min_stack_bytes()` stay as they are. The kind
`regexp_inside` of `every_nesting_kind_fits_the_stack_bound` (`crates/syntax/tests/sources.rs`)
holds it at parse time, with a pattern of 100 nested groups, a class and an escape as its
innermost literal, measured on top of the deepest nest of the script; the crate's own test
parses, compiles and matches 10,000 levels on a thread of 64 KiB. Rejected: recursion counted
against the limit with its frames inside the stack bound. A pattern counts its levels from zero,
so a literal at the limit inside a script at the limit would need the bound twice, and
`min_stack_bytes()` would double for every host.

**Refusals until the later stories of slice 5.** Added on 2026-11-23 for slice 5. None is left:
the flag `v` (`class set expressions of the v flag`) was refused by ParsePattern until
`story:regexp-v-flag-class-sets-and-properties-of-strings`, which keeps a class under `v` as its
operations and evaluates it when it is run, building no set at compile. A group name with a
code point outside ASCII, written or escaped, was refused (`group name outside ASCII`) until
`story:identifiers-and-group-names-outside-ascii`. CompilePattern refuses nothing: the
flag `i` and a modifier `i` are matched since
`story:regexp-case-insensitive-matching-and-modifiers`. The message is exactly `<construct>
is not supported before slice 5`, which `swell_syntax::message_kind` reads as a refusal. In a
literal the flag `v` was a refusal of the parse, counted as any refused parse (HOST-164). At
run time, in RegExpInitialize (`sec-regexpinitialize`: the constructor, RegExpCreate of the
`String` methods that make a RegExp, and the evaluation of a literal), a refusal is a
`SyntaxError` of the current realm with that message and no position after it, which the agent
counts before it throws (`Agent::count_refusal`), caught or not (`host-interface.md`, decision
13, HOST-191). It comes after the steps the clause gives before the parse (ToString of the
pattern and the flags, the check of the flags), so an error those steps throw is thrown instead.
A conformance run in which the count moved fails (decision 11), so no case that expects a
`SyntaxError` from `RegExp` passes on a refusal. A grammar error of a pattern and the nesting
limit are not refusals and are not counted. This is the third form of section 12, "A part before
its slice", for RegExpInitialize, with this set; once slice 5 is delivered it refuses nothing.

The four methods of `%RegExp.prototype%` keyed by `%Symbol.match%`, `%Symbol.replace%`,
`%Symbol.search%` and `%Symbol.split%` (`sec-regexp.prototype-%symbol.match%`,
`sec-regexp.prototype-%symbol.replace%`, `sec-regexp.prototype-%symbol.search%`,
`sec-regexp.prototype-%symbol.split%`) are the methods of their clauses and refuse no call
(added on 2026-11-23 for slice 5). What a script sees: `String.prototype.replace`, `replaceAll`
and `split` given a RegExp hand it to these methods and answer through them, as their clauses
do, and never answer from ToString of the RegExp as if it were a String; `match` and `search`
given an Object hand it to its method keyed by `%Symbol.match%` or `%Symbol.search%`, and given
anything else make a RegExp by RegExpCreate and call that method. The mechanism:
`story:regexp-objects-exec-and-literals` defined `[%Symbol.replace%]` and `[%Symbol.split%]` as
builtins that refused every call past the first steps of their clauses with a counted refusal,
and `story:regexp-symbol-methods-and-string-match-search` defines the methods of the clauses
under the same keys in their place, named `[Symbol.replace]` and `[Symbol.split]` with `length`
2 as their clauses give, so the property keeps its key and attributes. What `refusals()` still
counts for `RegExp` are the refusals of RegExpInitialize above (`host-interface.md`, decision
13, "A refusal raised by `RegExp`", HOST-191): none, since the flag `v` is matched.

## 8. Unicode data

**Pinned to one Unicode version for every table; the data files are vendored under
`data/unicode/<version>/` with a `SOURCE` file holding each file's address and SHA-256; the tool
`swell-unicode-gen` writes `crates/unicode/src/tables/*.rs`, which are committed, and the gate
renders them again in memory and fails on any difference.** Corrected on 2026-11-23 from "regenerates
them into `.out/`": the gate's `unicode-tables-check` (`taskfiles/unicode-tables.yml`) runs
`swell-unicode-gen --check`, which reads the vendored data, renders every table in memory,
compares it with the committed file and writes no table; only its log goes to `.out/`. The version is open item C (17.0.0
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
ECMA-262 and HTML, as 19 decisions and numbered requirements (`HOST-<n>`): 165 on 2026-11-21
(corrected that day from 135, the count when the document was written), 171 on 2026-11-22
with the six of resource accounting, 190 the same day with the host interface of the first consumer, 191 on 2026-11-23 with the refusals `RegExp` raises, 200 the same day with nine of slice 10's jobs and `async` functions (HOST-192 to HOST-200), and 208 the same day with eight of slice 10's later waves (HOST-201 to HOST-208: the kind of an AggregateError, `Symbol.asyncIterator`, async generators, built-in async functions), the count `task host-requirements-check` prints. The eleven places where it departs from this document (3 and 4 no longer since 2026-11-23, when "Entry points" and "Jobs" below were corrected to say what that document says):

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
`construct`, `perform_microtask_checkpoint` (runs the jobs of the microtask queue until it is
empty, all of them under one step budget for the whole call: "Jobs" below), and `collect`. `Outcome` is `Normal(value)`, `Throw(value)` (an exception the
script did not catch, as a value the host can read), or `Terminated(reason)` with reason
`StepBudget`, `MemoryBudget`, `Interrupted` or `EngineFault`. After `EngineFault` the agent
answers every call with `EngineFault`; the host discards it.

**Interrupting.** `Agent::interrupt_handle()` gives a `Send` handle holding an
`Arc<AtomicBool>`; setting it from any host thread ends the run at the next check (decision 3)
or at the next read of a parse (`host-interface.md`, section 12) with `Terminated(Interrupted)`. This is how surf can stop a page's script without a budget.

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
environment, and whatever implements the trait is an adapter, with system access or emulated
(section 2, "The engine core has no `std`"). Module loading is asynchronous as the draft has it: `HostLoadImportedModule` returns
at once, and the host later calls `finish_loading_imported_module` with the module's source or
an error.

**Jobs.** The microtask queue is kept in the agent, because jobs hold engine values: promise jobs
and the host's microtasks in one FIFO order (`host-interface.md` decision 11). The host is told
of each promise job through `HostEnqueuePromiseJob` (so HTML can attach its incumbent settings
through `HostMakeJobCallback`) and decides when to drain the queue, which is HTML's microtask
checkpoint: `perform_microtask_checkpoint` is one outermost entry, and every job it runs spends
from its one step budget (decision 3, "The microtask checkpoint is one entry"). A host
microtask is queued with the realm it runs in: `Agent::queue_microtask` takes one and checks it
first, answering `Err(StaleHandle)` for a stale or released realm and `Err(ForeignHandle)` for a
realm of another agent, with nothing queued, and `Cx::queue_microtask` uses the current realm of
its caller (HOST-192; HOST-31 and HOST-45 for every call that takes a realm). Added on
2026-11-23 for the first embedder, each stated in decision 3, "The microtask checkpoint is one
entry": `last_entry_steps()` after `perform_microtask_checkpoint()` reads the steps of the whole
checkpoint, every entry taken off and everything the jobs ran, also when it ended in a
termination; 0 after one that found the queue empty; and a nested checkpoint that returned at
once adds nothing (HOST-197). The interrupt ends a running checkpoint with
`Terminated(Interrupted)` at its start, before any entry is taken off, so a flag set before the
checkpoint ends it there; before each entry is taken off, leaving that entry queued; and while a
job runs, wherever the step budget is checked (HOST-198). A checkpoint that did not empty the
queue, ended by the step budget, the interrupt or the memory budget, leaves every entry it did
not start queued in its order and charged, and the next checkpoint runs them under its own
budget (HOST-199); the engine drops none of them, and the host empties the queue with
`Agent::clear_microtasks`, which runs no script and calls no hook, credits what the dropped
promise jobs were charged and answers how many entries it dropped; a promise whose reaction it
dropped stays pending for good (HOST-200). Dropping the agent with entries queued frees them and
calls nothing of the host. A host function called from a job may settle a promise, and the
reaction that queues runs in the same checkpoint, behind the entries already queued. Generic, timeout and
FinalizationRegistry cleanup jobs are handed to the host as `Job` tokens, and the host runs
each with `run_job` when it chooses, one entry each. Corrected on 2026-11-23: this paragraph
said that the host runs generic and timeout jobs "through `call`", and "Entry points" above
that the checkpoint runs its jobs "each with the budget". Both sentences were false against
`host-interface.md` decision 11 from 2026-11-21 on (its departures 3 and 4): a Job is not a
function, so `call` cannot run one, and with a budget per job a job that queues a job never lets
the checkpoint end.

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
of that size. The size is measured: `Limits::min_stack_bytes()` (`host-interface.md` decision 2,
HOST-17), with `STACK_BYTES_PER_REENTRY`, 14 KiB, per re-entry (`host/limits.rs`; decision 3,
"Suspended frames", item 6). Corrected on 2026-11-23 from "The size is not measured yet."

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
the host interface is an in-process library interface, and what ESS 0.97.4 generates is
components with ports (surf `AGENTS.md`, "Layout"), not a library interface. How well ESS states
an in-process library interface was measured by the first slice (`host-interface-model.md`,
corrected on 2026-11-23 from "Not checked: ...; the first slice finds out."): ESS 0.96.0 let the
model state its commands, outcomes and lifecycles, its generated code is "not usable for an
in-process library interface", and what it could not state is listed there as 53 markers, each
carried by a `HOST-<n>` requirement or named an ESS limit.

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
  regular expression literal evaluated before slice 5, a `String` method given a code unit
  outside ASCII before slice 5, section 12, or `RegExp` given a pattern or flags the engine
  does not take yet, decision 7) does not end the run, so the runner also
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
slice after which it can pass: the slice delivered last among its directory, its features and its
flags, by the rules in the block below (each line `dir <prefix> <slice>`, `feature <name> <slice>`,
`flag <name> <slice>` or `exclude <feature> <reason>`; the longest matching prefix wins; a feature
without a line sets no slice; the one line `order delivery <slice>,<slice>,...` names every
numbered slice once, in the order of delivery). Since 18,673 cases name no feature, the counts say
when a case can first pass, not that it will; the runner gives the real numbers.

**The order of delivery.** Added on 2026-11-23, at the close of slice 6. The slices are not
delivered in the order of their numbers: after slice 3 the order is 6, 5, 7, 10, 4, 9, by what
the first consumer needs first (`docs/plan/waves.md`, "The order after slice 3"), then 8, 11 and
12 to 21 as numbered. The block names it in one line, `order delivery
1,2,3,6,5,7,10,4,9,8,11,12,13,14,15,16,17,18,19,20,21`, and a case is counted in the slice
delivered last among the slices of its lines: a case of `test/built-ins/RegExp/` (slice 5) that
uses `Symbol` (slice 6) can first pass after slice 5, and one of `test/built-ins/Date/` (slice 4)
that uses `Promise` (slice 10) after slice 4. While the numbers were the order, the highest number
was that slice; once slice 6 came before 4 and 5 it was not, and 393 failing cases of slice 6 that
need only slice 4 or 5 could not leave it. The runner, `slices.awk` below and
`test262-moves-check` read the order from the block and compare two slices by their place in it;
the runner fails on a block without the line, with a slice named twice, or with a rule line of a
slice the line does not name, and the check refuses a line of `conformance/test262/moves.tsv`
whose `to` is not delivered after its `from`. Measured on 2026-11-23 with the check under "How the
counts were made", this block counted once by the highest number and once by the order: 3,214
cases are in another slice, and slices 1 to 3 keep every case. 75 go from slice 6 to 4 and 327
from 6 to 5 (7 of them by a line of their own, in the paragraph "Slice 6's cases that need a
later slice"; the others by the lines the block had), 1 from 5 to 4, 1 from 7 to 4, 30 from 9
to 8, 2,743 from 10 to 8 (classes with `async`) and 37 from 10 to 9. The counts of the slices
not yet delivered say where a case is counted under this order; when the order of those slices
changes, their counts move with it, and the delivered slices, which stay first in the order,
keep theirs.

**A case that comes into a slice from one delivered later.** Added on 2026-11-23 for slice 5.
What the counts show: a case that the block counts in a slice delivered after the slice being
closed, and that passes on the code of the slice being closed because it needs nothing of a
later slice, is counted in the slice being closed, and the slice it leaves loses it; the sums of the table do not change. A
delivered slice takes no case this way, so it keeps its counts. First use: at the close of slice
5, 18 cases (36 runs) of the pattern grammar of B.1.2 under `test/annexB/`, which the line `dir
test/annexB/ 18` counts in slice 18 and which pass with the grammar slice 5 builds (decision 7).
The rule:

1. Such a case gets a rule line of the block naming the slice that takes it: `dir <file>
   <slice>`, or `dir <directory>/ <slice>` when every case of the directory is one. The line is
   longer than the line that gave the case its slice, so the longest prefix wins; it takes the
   case only when no `feature` or `flag` line of the case names a slice delivered after the one
   it names, since the case is counted in the slice delivered last among the slices of its lines.
2. The paragraph of the slice that takes it ("Slice <n>'s cases that need a later slice", at its
   close) states the lines with their count by directory and the slice each case leaves, and the
   rows of both slices in the table and in the block after "Its output on this document:" move
   by it.
3. It gets no line of `conformance/test262/moves.tsv`. That list records only moves to a slice
   delivered after the one the case leaves, each with the evidence of the later part the case
   needs, and `test262-moves-check` refuses a line whose `to` is not delivered after its `from`;
   a case that comes into an earlier slice needs no later part, and its evidence is that it
   passes.
4. What keeps such a line honest: the case passes in every one of its runs on the commit that
   adds the line. No check of the gate shows that. `test262-check` compares a digest of the
   sources and prints the committed lines; `test262-moves-check` reads the lines of the list
   alone; the full run (`test262-expected`, the lane `slow`) compares the counts it prints with
   `conformance/test262/expected.txt`, which holds what passed per slice and directory, not what
   must pass, and no case by name. So the acceptance of the close shows it, on the run lines of
   its full run: every run of each such case counted in the slice that takes it and `Passed`
   (for slice 5, check (4) of `story:slice-05-conformance-close` over
   `docs/plan/slice-05/close-annexb-to-5.tsv`). After the close the line of that slice in the
   expected file, with no failed run, makes a later failure of one of these cases visible: the
   change that causes it writes that line again with a failed run.

| slice | name | cases | runs |
|---|---|---|---|
| 1 | `harness` | 31 | 62 |
| 2 | `language-core` | 4,259 | 7,543 |
| 3 | `builtins-core` | 7,011 | 13,829 |
| 4 | `numbers-and-dates` | 1,612 | 3,184 |
| 5 | `regexp` | 2,724 | 5,441 |
| 6 | `bindings-and-iteration` | 3,898 | 7,351 |
| 7 | `generators` | 1,472 | 2,801 |
| 8 | `classes` | 9,302 | 18,455 |
| 9 | `collections-proxy-reflect` | 1,785 | 3,541 |
| 10 | `promises-and-async` | 3,759 | 7,258 |
| 11 | `modules` | 1,281 | 1,833 |
| 12 | `bigint` | 358 | 714 |
| 13 | `binary-data` | 3,061 | 6,078 |
| 14 | `shared-memory` | 598 | 1,192 |
| 15 | `iterator-helpers-and-resources` | 1,094 | 2,170 |
| 16 | `weak-references` | 82 | 164 |
| 17 | `tail-calls` | 35 | 35 |
| 18 | `annex-b` | 1,134 | 1,451 |
| 19 | `staging` | 1,435 | 2,692 |
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

Slice 3 had 7,635 cases and 15,074 runs by its rules (`dir test/built-ins/ 3`, the lines of
slice 2's close that gave it 78 cases, and `feature exponentiation 4`, which took 24). 7,011
cases and 13,829 runs are what slice 3 counts: 624 cases that need a later slice move by the
lines of the paragraph "Slice 3's cases that need a later slice". Measured on 2026-11-22 with
the same check (`awk -f slices.awk` over this block and the inventory's `meta.tsv`, counting
slice `03`): `03 7011 13829`, the slice line of `slot task test262` on the code of the slice.
Every moved case goes to a slice from 4 to 13, so slices 1 to 19 still hold 44,931 cases and
85,794 runs and the 22 lines still sum to 53,801 and 103,360.

Slice 6 had 4,436 cases and 8,413 runs by its rules (its `feature` lines and the lines of the
closes of slices 1 to 3 that gave it cases). 3,898 cases and 7,351 runs are what slice 6 counts:
136 cases that need a later slice move by the lines of the paragraph "Slice 6's cases that need
a later slice", and 402 cases that need slice 4 or 5 (75 and 327) go there by the order of
delivery, 7 of them with a line of their own, as that paragraph says. Measured on 2026-11-23 with the same check (`awk -f slices.awk`
over this block and the inventory's `meta.tsv`, counting slice `06`): `06 3898 7351` (`06 4300
8153` before the order of delivery). Every moved case goes to a slice from 4 to 13, so slices 1
to 19 still hold 44,931 cases and 85,794 runs and the 22 lines still sum to 53,801 and 103,360.

Slice 5 had 2,724 cases and 5,441 runs by its rules (its `dir` and `feature` lines, the lines of
the closes of slices 2, 3 and 6 that gave it cases, and the order of delivery). 2,724 cases and
5,441 runs are what slice 5 counts after its close: 18 cases (36 runs) that need a later slice
move by the lines of the paragraph "Slice 5's cases that need a later slice", 14 to slice 4, 2 to
8 and 2 to 9, and 18 cases (36 runs) of the pattern grammar of B.1.2 come into it from slice 18 by
the lines of the same paragraph ("A case that comes into a slice from one delivered later"
above). Measured on 2026-11-24 with the same check (`awk -f slices.awk` over this block and the
inventory's `meta.tsv`, counting slice `05`): `05 2724 5441`, the slice line of `slot task
test262` on the code of the slice, every run passed. Every moved case goes to slice 4, 8 or 9 and
every case that comes in leaves slice 18, so slices 1 to 19 still hold 44,931 cases and 85,794
runs and the 22 lines still sum to 53,801 and 103,360.

Slice 7 had 1,491 cases and 2,841 runs by its rules (its `dir` lines, `feature generators 7`,
the lines of the closes of slices 2, 3 and 6 that gave it cases, and the order of delivery).
1,472 cases and 2,801 runs are what slice 7 counts after its close: 40 cases (80 runs) that
need slice 8 move by the lines of the paragraph "Slice 7's cases that need a later slice", and
21 cases (40 runs) come into it, 2 from slice 18 and 19 from slice 19, by the lines of the same
paragraph ("A case that comes into a slice from one delivered later" above). Measured on
2026-11-24 with the same check (`awk -f slices.awk` over this block and the inventory's
`meta.tsv`, counting slice `07`): `07 1472 2801`, the slice line of `slot task test262` on the
code of the slice, every run passed. Every moved case goes to slice 8 and every case that comes
in leaves slice 18 or 19, so slices 1 to 19 still hold 44,931 cases and 85,794 runs and the 22
lines still sum to 53,801 and 103,360.

Slice 10 had 3,879 cases and 7,499 runs by its rules (its `dir`, `feature` and `flag` lines, the
lines of the closes of slices 2, 3 and 6 that gave it cases, and the order of delivery). 3,759
cases and 7,258 runs are what slice 10 counts after its close: 141 cases (280 runs) that need a
later slice move by the lines of the paragraph "Slice 10's cases that need a later slice", 116 to
slice 8, 4 to 9, 20 to 12 and 1 to 13, 138 of them by new lines of `moves.tsv` and 3 by lines
that change their target; and 21 cases (39 runs) come into it, 1 from slice 18 and 20 from slice
19, by the lines of the same paragraph ("A case that comes into a slice from one delivered later"
above). Measured on 2026-11-24 with the same check (`awk -f slices.awk` over this block and the
inventory's `meta.tsv`, counting slice `10`): `10 3759 7258`, the slice line of `slot task
test262` on the code of the slice, every run passed. Every moved case goes to slice 8, 9, 12 or
13 and every case that comes in leaves slice 18 or 19, so slices 1 to 19 still hold 44,931 cases
and 85,794 runs and the 22 lines still sum to 53,801 and 103,360.

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
   2,665, `String` 780, `Function` 417. It leaves to slice 6 `Array.from`, `Object.fromEntries`,
   `Object.groupBy` and the Array iterator methods `keys`, `values` and `entries`, which need
   iterators, and to slice 5 case mapping and comparison outside ASCII and `normalize`, which
   need the Unicode tables ("A part before its slice").
4. `numbers-and-dates`: `swell-num`, what slice 2 does not take of `Number`, `Math`, `Date`,
   `parseInt`, `parseFloat`. `Date` 492, `Number` 313, `Math` 284.
5. `regexp`: `swell-regexp`, `swell-unicode`, `RegExp` and the string methods that use it;
   from slice 3, case mapping (`toLowerCase`, `toUpperCase`, `toLocaleLowerCase`,
   `toLocaleUpperCase`) and `localeCompare` outside ASCII, and `normalize`; from slice 18, the
   pattern grammar of B.1.2 (decision 7), 18 cases of `annexB` (the paragraph "Slice 5's cases
   that need a later slice"). `RegExp` 1,848, `String` 359, `literals` 241, `annexB` 18.
6. `bindings-and-iteration`: `let`, `const`, destructuring, spread, arrows, templates,
   `Symbol`, iterators, `for-of`; from slice 3, `Array.from`, `Object.fromEntries`,
   `Object.groupBy` and the Array iterator methods `keys`, `values` and `entries`. `statements/for-of` 575, `expressions/assignment` 342,
   `expressions/arrow-function` 302.
7. `generators`: frames saved in objects. `expressions/object` 298, `expressions/generators`
   284, `statements/generators` 262 (counted at the close of the slice).
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
18. `annex-b`: Annex B and [[IsHTMLDDA]]. `annexB/language` 837, `annexB/built-ins` 205.
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
numbers follow dependencies: nothing can be counted before slice 1, slices 2 to 5 are the
language and library of ES5, 6 to 8 the syntax most later tests are written in (4,916 cases carry
`class`), 10 needs 7's suspended frames, and 13 needs 12 for `BigInt64Array`. The order of
delivery ("The order of delivery" above) keeps the last two: 7 before 10, 12 before 13.

**A part before its slice.** Added on 2026-11-21, when the slice epics were drafted and the
order of section 12 was read against the crate graph of decision 2: `swell-syntax` depends on
`swell-unicode` and `swell-regexp`, and `swell-compiler` on `swell-num` (corrected on
2026-11-23: `swell-syntax` was said to depend on `swell-num` too), which section 12 builds in
slices 4 and 5; the
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
  outside ASCII was a `SyntaxError` (until `story:identifiers-and-group-names-outside-ascii`), a
  regular expression
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
  methods refuse no input, and `localeCompare` orders every String by decision 5, "The order
  `localeCompare` gives". The cases of slice 3 these methods cannot pass (a String outside
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
  only. Named on 2026-11-23 for slice 5: RegExpInitialize (`sec-regexpinitialize`), and so the
  `RegExp` constructor, the evaluation of a literal and the `String` methods that make a
  RegExp, answers every pattern and flags but the parts of decision 7, "Refusals until the later
  stories of slice 5" (the flag `v`, the flag or modifier `i` and a group name outside ASCII,
  each until its story), and refuses those with the counted refusal naming slice 5, each
  until the story of slice 5 that brings it. Named on 2026-11-23 for slice 5 as well, and no
  longer in this form: `RegExp.prototype [ %Symbol.replace% ]` and `RegExp.prototype [
  %Symbol.split% ]` (`sec-regexp.prototype-%symbol.replace%`,
  `sec-regexp.prototype-%symbol.split%`), which refused every call past the first steps of their
  clauses with the counted refusal naming the method until
  `story:regexp-symbol-methods-and-string-match-search` replaced them with the methods of their
  clauses, which answer every call (decision 7, "Refusals until the later stories of slice 5";
  HOST-191 counts the refusals of `RegExp`).
- A refusal is a `SyntaxError`, but not the one a `negative: parse` case names: the runner
  reads the kind of a parse error from its message (`swell_syntax::message_kind`), and a run
  that a refusal or the nesting limit ends, in a harness file or in the case, fails whatever the
  case's `negative` says. It prints the number of such runs as `test262-refused <runs>`, so the
  share of the checkout a later slice's parser still declines stays visible (53,564 of 85,556
  runs at slice 1). Before this rule, 3,011 negative parse cases of slices 2 to 18 were counted
  passed on a refusal. A refusal the script catches (through `eval` or `Function`, of a
  regular expression literal, or of a `String` method or of `RegExp` in the third form) is counted by the agent (`Agent::refusals`, `host-interface.md`
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

**Slice 3's cases that need a later slice.** Measured at the close of slice 3 (2026-11-22,
`story:slice-03-conformance-close`), with the run lines of the runner on the code of the slice
(`slot task test262`): 616 of the 7,635 cases of slice 3 fail. 615 of them fail on a part of a
later slice and move; the one left, `test/built-ins/String/prop-desc.js`, fails on no part of
a later slice and stays in slice 3, counted failed. It calls `verifyProperty(this, "String",
...)`. In the pinned `harness/propertyHelper.js`, line 171 (`delete obj[name];` in
`isConfigurable`) removes the global `String`; line 182 then calls
`_isCanonicalNumericIndexString(name)`, whose line 156 (`return String(n) === value;`) reads
the global `String` that is gone, and the run ends in a `ReferenceError`. The harness keeps
`Array.isArray` and `Math.max` as captured primordials, not `String`. So the case fails in the
pinned harness before the engine's answer is judged; that it fails in another engine as well is
inferred from the harness source, and no other engine was run. A delivered slice passes every
one of its cases, with this one exception, ruled on 2026-11-22 by the session that decides for
the operator: slice 3 is delivered with 7,010 of its 7,011 cases. The case is measured again at
the next move of the test262 pin, and the exception is removed when it passes; a second case of
this kind reopens the question of a rule line for a defect of the harness. With the 615 move the 9
cases of the seven directories below that pass, 624 in all, each a line of
`conformance/test262/moves.tsv` with `from` 3, kind `part` and a token of
`conformance/test262/later-parts.tsv` as its evidence, held by `test262-moves-check`. The
block gains 552 lines `dir <path> <slice>`: 11 for a directory whose cases of slice 3 all move
to one slice and 541 for a single file. Seven of the 11 are the directories whose method this
section gives to a later slice, moved whole as ruled on 2026-11-21 by the session that decides
for the operator: `test/built-ins/Array/from/`, `Array/prototype/keys/`, `values/`,
`entries/`, `Object/fromEntries/` and `Object/groupBy/` to slice 6 (iterators) and
`String/prototype/normalize/` to slice 5 (the Unicode tables), 79 cases; one case of
`Array/from/`, `items-is-arraybuffer`, has its own line to slice 13. The 9 of the 79 that pass
(`Array/from/items-is-null-throws`, `mapfn-is-not-callable-typeerror` and
`source-object-length-set-elem-prop-err`, `Array/prototype/entries/return-abrupt-from-this`,
`keys/return-abrupt-from-this`, `values/this-val-non-obj-coercible`,
`Object/groupBy/invalid-callback`, `String/prototype/normalize/this-is-null-throws` and
`this-is-undefined-throws`) pass only because calling the missing method throws the
`TypeError` they expect. Each has as its evidence a token of its own, the call it expects the
`TypeError` from, whose item in `later-parts.tsv` ends in `; passed at the move`, and the
expected file counts the 9 as passed in slices 5 and 6, so that a regression of one of them
fails the gate there. The other four directories are `Object/getOwnPropertySymbols/` (2 cases;
the method is not built before slice 6, where `Symbol` is) and the one-case directories
`Function/prototype/arguments/`, `caller/` and `caller-arguments/` (`const`). One case passed
at the start of the slice and ends on a counted refusal now:
`test/built-ins/Function/prototype/apply/this-not-callable.js`, which evaluates the regular
expression literal `/re/` once `Function.prototype.apply` exists; it moves to slice 5 with that
literal as its evidence. The groups, by target slice (lines of the list, kind `part`):

- to slice 4, 193: `Date` 137; a member of `Math` other than `max` 44 (`Math.pow` 15, the
  constants read through `Object.getOwnPropertyDescriptor(Math, ...)` 8, `Math.PI` 4, the
  other functions 17); `parseInt` 7 and `parseFloat` 1; the `Number.prototype` methods
  `toFixed`, `toExponential`, `toPrecision` and `toLocaleString` 4;
- to slice 5, 140: `RegExp` 98; a regular expression literal 13 (`/./` 7, `/re/` 3, `(/x/)` 2,
  `/a/g` 1); `normalize` 11 (the directory, 2 of them marked); case mapping outside ASCII 12
  (the special casings, the final sigma and the supplementary plane of `toLowerCase`,
  `toUpperCase`, `toLocaleLowerCase` and `toLocaleUpperCase`) and comparison outside ASCII 2
  (`localeCompare/15.5.4.9_3` and `15.5.4.9_CE`), which the methods refuse until slice 5;
  `match`, `replace`, `search` and `split` 4;
- to slice 6, 232: later syntax 144 (arrow functions 67, `const` 36, 7 of them only in
  `nativeFunctionMatcher.js`, `let` 32, templates 7, a
  computed property name 1, a default parameter 1); `Array.from` 25 (3 marked); the Array
  iterator methods 18 (3 marked); `Object.groupBy` 13 (1 marked); `Object.fromEntries` 11;
  `Symbol` 18; `Object.getOwnPropertySymbols` 3 (its directory and
  `JSON/rawJSON/returns-expected-object`);
- to slice 7, 3: generator functions;
- to slice 8, 19: `class` 18; a private name 1 (`Function/S15.3_A2_T2`, `var #x` in the body
  given to `Function`, refused naming slice 8);
- to slice 9, 16: `Proxy` 8, `Reflect` 4, `Map`, `Set`, `WeakMap` and `WeakSet` 1 each;
- to slice 10, 4: async functions 3, `Promise` 1;
- to slice 12, 3: `BigInt` 2, a BigInt literal 1;
- to slice 13, 14: `ArrayBuffer` and the typed arrays.

The counts sum to the 624 lines this change adds to the list. Outside the seven directories,
every moved case fails on the code of the slice, and the one that passed at the start of the
slice ends on a counted refusal (the story's acceptance). Seven passing cases of later slices
stopped passing during the slice and are not moved, since they are in their slice already:
five of slice 6 that call a `String` method with `Symbol()` and two of slice 18 that compare
`trimLeft` with `trimStart` and `trimRight` with `trimEnd`.

**Slice 6's cases that need a later slice.** Measured at the close of slice 6 (2026-11-23,
`story:slice-06-conformance-close`), with the run lines of the runner on the code of the slice
(`slot task test262`): 529 of the 4,436 cases of slice 6 fail, each in every one of its runs, and
none on a part of slice 6 (no run ends on a refusal that names slice 6 or on a missing `Symbol`).
136 of them move to a later slice, each a line of `conformance/test262/moves.tsv` with kind
`part` and a token of `conformance/test262/later-parts.tsv` as its evidence, held by
`test262-moves-check`: 111 lines are new, with `from` 6, or 1 for the two harness cases slice
1's close gave to slice 6 before the list existed (`test/harness/nativeFunctionMatcher.js` and
`wellKnownIntrinsicObjects.js`); 25 are lines of cases moved to slice 6 earlier, which keep their
`from` and get the new `to` and evidence. The block gains 83 lines: one for the directory
`test/language/computed-property-names/class/`, whose 29 cases all move to slice 8, and 82 for a
single file; 25 lines change their slice: 23 for a single file and the one-case directories
`test/built-ins/Function/prototype/arguments/` and `caller/`. The groups, by target slice (lines
of the list):

- to slice 4, 5: `Date` 2 (`Boolean/prototype/toString/S15.6.4.2_A2_T3` and
  `valueOf/S15.6.4.3_A2_T3`), `Math.pow` 2 (the two `nan-equivalence-define-own-property-`
  cases of `Object/internals/DefineOwnProperty/`, through `nans.js`), `**` 1
  (`expressions/exponentiation/int32_min-exponent`);
- to slice 5, 4: a regular expression literal of `nativeFunctionMatcher.js` 2 (the harness case
  and `Function/prototype/toString/bound-function`), `RegExp` 1
  (`Function/prototype/toString/symbol-named-builtins`), `String.prototype.split` 1
  (`JSON/stringify/value-string-escape-ascii`);
- to slice 7, 16: generator methods 13 (the 12 `eval-code/direct/gen-meth-` cases and
  `Function/prototype/toString/generator-method`), generator functions and `yield` 3;
- to slice 8, 79: `class` 74 (the 29 cases of `computed-property-names/class/`, 36
  `*-init-fn-name-class` cases of the destructuring directories, 9 others), `super` 3,
  `new.target` 2;
- to slice 9, 4: `Proxy`;
- to slice 10, 8: async functions 7 (3 of them the cases that include
  `wellKnownIntrinsicObjects.js`, whose sources it compiles with `Function` and whose refused
  async and generator functions are counted refusals), `Promise` 1;
- to slice 12, 18: a BigInt literal 12 (`4n` in the ten `RegExp/prototype/*/this-val-non-obj`
  cases, `0n` in two cases of `expressions/assignment/destructuring/`), `BigInt` 6 (the
  `cstm-*-on-bigint-primitive` cases of the `String` methods);
- to slice 13, 2: the typed arrays (`Array/prototype/concat/` `_large-typed-array` and
  `_small-typed-array`).

The counts sum to the 136 lines this change adds to or changes in the list. 393 failing cases
(784 runs) need slice 4 or 5 and no later slice. Each carries a `feature` of slice 6 (`Symbol`
or a well-known symbol, a computed property name or destructuring), and while the block gave a
case the highest slice among its lines they stayed in slice 6, counted failed. Under the order
of delivery ("The order of delivery" above; slices 4 and 5 are delivered after 6) they leave it:
325 go to slice 5 (`RegExp` and the `String` methods that use it: `RegExp/prototype/Symbol.*`
211, `String/prototype` 76, the rest of `RegExp` 19, `RegExpStringIteratorPrototype` 17, 2
others) and 68 to slice 4 (`Date` 60, `Number` 5, `**` 3); one case that needs both,
`Object/prototype/toString/symbol-tag-override-instances` (`new Date(0)` and `/./`), goes to 4,
which is delivered after 5. 386 of them get their slice from the lines the block had (their
directory, or `feature exponentiation 4`). 7 are in a directory of slice 3 or 6 and get a line
`dir <file> <slice>` of the block and a line of `conformance/test262/moves.tsv` with `from` 6
and kind `part`, its evidence found in the case: to slice 5, `RegExp`
(`Array/prototype/concat/Array.prototype.concat_spreadable-reg-exp`), `/a/g`
(`Array/prototype/sort/comparefn-nonfunction-call-throws`) and `/./` (the
`return-abrupt-from-searchstring-regexp-test` cases of `String/prototype/endsWith`, `includes`
and `startsWith`); to slice 4, `Date` (`symbol-tag-override-instances` and
`Symbol/prototype/Symbol.toPrimitive/removed-symbol-wrapper-ordinary-toprimitive`). With them 9
cases that pass leave slice 6 by their directory, 7 to slice 4 (`Math/Symbol.toStringTag` and 6
of `Number`) and 2 to slice 5 (two of `String/prototype/normalize`), counted passed there.

**Slice 5's cases that need a later slice.** Measured at the close of slice 5 (2026-11-24,
`story:slice-05-conformance-close`), with the run lines of the runner on the code of the slice
(`slot task test262`): 18 of the 2,724 cases of slice 5 fail, each in every one of its runs (36
runs), and none on a part of slice 5 (no run ends on a refusal that names slice 5 or on a missing
`RegExp`). All 18 move to a later slice, each a line of `conformance/test262/moves.tsv` with kind
`part` and a token of `conformance/test262/later-parts.tsv` as its evidence, held by
`test262-moves-check`: 11 lines are new, with `from` 5; 7 are lines of cases moved to slice 5
earlier (5 with `from` 3, 2 with `from` 2), which keep their `from` and get the new `to` and
evidence. The block gains 11 lines `dir <file> <slice>`, one for a single file each, and 7 lines
change their slice from 5 to 4. The groups, by target slice (lines of the list):

- to slice 4, 14: `Date` 7 (`global/global-object`, `S10.2.3_A1.1_T3`, `A1.2_T3` and `A1.3_T3`,
  `expressions/typeof/built-in-exotic-objects-no-call` and `native-call`, and
  `RegExp/prototype/Symbol.split/species-ctor-species-non-ctor`), `Math.pow` 3
  (`RegExp/prototype/Symbol.replace/coerce-lastindex` and the two
  `String/prototype/split/*-math-pow-2-32-1-instance-is-number` cases), `**` 2
  (`split/separator-undef-limit-custom` and `-zero`), `parseInt` 1
  (`Object/getOwnPropertyNames/15.2.3.4-4-1`, which expects the names of the global object),
  `Math.PI` 1 (`RegExp/prototype/exec/S15.10.6.2_A1_T12`, whose subject converts `Math.PI` to a
  String);
- to slice 8, 2: `class` (`RegExp/named-groups/groups-object-subclass` and `-sans`);
- to slice 9, 2: `Map` (`RegExp/match-indices/indices-array-non-unicode-match` and
  `-unicode-match`, through `deepEqual.js`, which calls `new Map()`).

The counts sum to the 18 lines this change adds to or changes in the list.

18 cases (36 runs) come into slice 5 from slice 18, by the rule "A case that comes into a slice
from one delivered later" above. The line `dir test/annexB/ 18` counts the 70 cases of
`test/annexB/built-ins/RegExp/` (62) and `test/annexB/language/literals/regexp/` (8) in slice 18;
26 of them are excluded (`legacy-regexp`, stage 3), and of the 44 that run, 20 pass and 24 fail on
the code of the slice. The decision of the close: the 18 that test the pattern grammar of B.1.2
alone, which this slice builds (decision 7), count in slice 5, by the lines `dir
test/annexB/language/literals/regexp/ 5` (8 cases), `dir test/annexB/built-ins/RegExp/named-groups/
5` (2) and one `dir <file> 5` for each of 8 files of `test/annexB/built-ins/RegExp/` (8), each
case leaving slice 18; each passes in every one of its runs (the close's check (4) over
`docs/plan/slice-05/close-annexb-to-5.tsv`), and none has a line of `moves.tsv`.
`RegExp.prototype.compile` (B.2.4.1) stays in slice 18, and with it the 24 cases that fail on it:
the 21 of `prototype/compile/` that run, `prototype/flags/order-after-compile` and the two of
`prototype/Symbol.split/` that call it. The two that use generator functions
(`RegExp-control-escape-russian-letter` and `RegExp-invalid-control-escape-character-class`, the
feature `generators`) pass and stayed where the block puts them, in slice 18, at the close of
slice 5: slice 7, which they need, is delivered after slice 5. The close of slice 7 brings them
into slice 7 ("Slice 7's cases that need a later slice" below).

**Slice 7's cases that need a later slice.** Measured at the close of slice 7 (2026-11-24,
`story:slice-07-conformance-close`), with the run lines of the runner on the code of the slice
(`slot task test262`): 40 of the 1,491 cases of slice 7 fail, each in every one of its runs (80
runs), and none on a part of slice 7: the first failure of each run is the refusal `class is not
supported before slice 8` (72 runs) or `super is not supported before slice 8` (8 runs). Each was
read: with its `class` or `super` it needs nothing of slice 7 that its sibling cases without
them do not pass with (generator methods, a generator declaration in a block or a `switch`, the
patterns of the parameters of a generator). All 40 move to slice 8, each a new line of
`conformance/test262/moves.tsv` with `from` 7, kind `part` and a token of
`conformance/test262/later-parts.tsv` as its evidence, held by `test262-moves-check`. The block
gains 40 lines `dir <file> 8`, one for a single file each: every directory that holds one of them
holds cases of slice 7 that stay. The groups, by target slice (lines of the list):

- to slice 8, 40: `class` 36 (the 20 `cls-decl-gen-meth-*` and `cls-expr-gen-meth-*` cases of
  `language/arguments-object/`, a generator method of a class called with a trailing comma; the
  12 `*-init-fn-name-class` cases of `expressions/generators/dstr/`, `statements/generators/dstr/`
  and `expressions/object/dstr/`, 4 each; the 4 cases of a class and a generator declaration of
  one name in `block-scope/syntax/redeclaration/` and `statements/switch/syntax/redeclaration/`, 2
  each), `super` 4 (`expressions/object/concise-generator` and the three
  `expressions/object/method-definition/generator-super-*` cases).

The counts sum to the 40 lines this change adds to the list.

21 cases (40 runs) come into slice 7, by the rule "A case that comes into a slice from one
delivered later" above. Each is counted by the block in a slice delivered after 7, failed at the
start of the slice (`4971b9b`) on a part of slice 7 only (the refusal `generator function is not
supported before slice 7` in every failed run, or, for
`staging/sm/expressions/object-literal-__proto__`, a counted refusal of the generator method it
compiles with `Function`), passes in every run on the code of the slice, has no `feature` or
`flag` line of a slice delivered after 7, and neither it nor a harness file it includes uses a
part of a later slice. From slice 18, 2:
`test/annexB/built-ins/RegExp/RegExp-control-escape-russian-letter.js` and
`RegExp-invalid-control-escape-character-class.js`, the two cases of the pattern grammar of B.1.2
that the paragraph of slice 5 left in slice 18 because they use generator functions. From slice
19, 19, all under `test/staging/sm/`: `generators/` 11, `class/` 2 (`compPropNames` and
`methDefnGen`, generator methods of object literals and no class), and 1 each of `Array/`
(`sort_small`), `Symbol/` (`for-in-order`), `expressions/` (`object-literal-__proto__`: a
`__proto__` property of an object literal, which ECMA-262 states outside Annex B; each
`.__proto__` it reads is an own property), `lexical-environment/`
(`block-scoped-functions-annex-b-generators`: a generator declaration in a block is not hoisted,
which the web compatibility semantics of Annex B do not change) and `regress/`
(`regress-530879`). Each gets a line `dir <file> 7` (no directory has every case come in),
listed with the slice it leaves in `docs/plan/slice-07/close-into-7.tsv`; each passes in every
one of its runs (the close's check (4) over that file), and none has a line of `moves.tsv`.
661 other cases of slices delivered after 7 failed at the start of the slice and pass in every
run on its code, and stay in their slice with no line: 463 have a `feature` or `flag` line of a
later slice (`flag async`, `async-functions`, `async-iteration`, the `Promise` features,
`Reflect.construct`, `explicit-resource-management`, `iterator-helpers`, `iterator-chunking`,
`dynamic-import`), 197 failed at the start on a part that is not slice 7's (the refusal of async
functions, `Promise` not defined, a counted refusal of the flag `i` or of an identifier outside
ASCII), and 1, `language/expressions/await/await-in-generator`, reads `await` as an identifier,
which the grammar of async functions of slice 10 decides.

**Slice 10's cases that need a later slice.** Measured at the close of slice 10 (2026-11-24,
`story:slice-10-conformance-close`), with the run lines of the runner on the code of the slice
(`slot task test262`): 141 of the 3,879 cases of slice 10 fail, each in every one of its runs (280
runs), and none on a part of slice 10: the first failure of each run is the refusal `class is not
supported before slice 8` (188 runs), `super is not supported before slice 8` (40), `new.target is
not supported before slice 8` (12) or `BigInt literal is not supported before slice 12` (30, 18 of
them in the harness file `temporalHelpers.js`), or a `ReferenceError` for `Proxy` (6), `Map` (2)
or `ArrayBuffer` (2). Each was read: with its later part it needs nothing of slice 10 that its
sibling cases without that part do not pass with (async functions, methods and generators,
`for await`, the patterns of their parameters, `Promise` and its statics, `Array.fromAsync`). A
case that needs two later slices moves to the one delivered last: the 4
`resolve-throws-iterator-return-is-not-callable` cases of `Promise/all`, `allSettled`, `any` and
`race` fail first on `class` and list `0n` among their values, and go to slice 12. All 141 move,
each a line of `conformance/test262/moves.tsv` with kind `part` and a token of
`conformance/test262/later-parts.tsv` as its evidence, held by `test262-moves-check`: 138 lines
are new, with `from` 10; 3 are lines of cases moved to slice 10 earlier (2 with `from` 2, 1 with
`from` 6), which keep their `from` and get the new `to` and evidence. The later parts gain the
tokens `1n` and `2n` (slice 12). The block gains 138 lines `dir <file> <slice>`, one for a single
file each, since every directory that holds one of them holds cases of slice 10 that stay, and 3
lines change their slice from 10 to 8 or 9. The groups, by target slice (lines of the list; in
brackets the lines with `from` 10):

- to slice 8, 116 (114): `class` 90 (the 20 `cls-decl-async-gen-meth-*` and
  `cls-expr-async-gen-meth-*` cases of `language/arguments-object/`, an async generator method of
  a class called with a trailing comma; the 40 `*-init-fn-name-class` cases of
  `statements/for-await-of/` 24, `expressions/async-generator/dstr/` 8,
  `statements/async-generator/dstr/` 4 and `expressions/object/dstr/` 4; the 8 cases of a class
  and an async function or async generator of one name in `block-scope/syntax/redeclaration/` and
  `statements/switch/syntax/redeclaration/`, 4 each; the 8 `async-method-class-*` and
  `async-generator-method-class-*` cases of `Function/prototype/toString/`; 7 of
  `Promise/prototype/finally/` and the 4 `resolve-throws-iterator-return-null-or-undefined` cases
  of `Promise/all`, `allSettled`, `any` and `race`, a subclass of `Promise` or a class as the
  constructor; `Array/fromAsync/this-constructor-with-bad-length-setter`,
  `AsyncGeneratorPrototype/throw/throw-suspendedYield-try-finally-return` and
  `expressions/optional-chaining/member-expression`, `from` 2), `super` 20 (the 18
  `early-errors-*-contains-super-*` cases of `expressions/async-arrow-function/`,
  `expressions/async-function/`, `expressions/async-generator/` and `statements/async-function/`,
  4 each, and of `expressions/object/method-definition/` 2, one of them `from` 2; and
  `expressions/object/method-definition/async-super-call-body` and `-param`), `new.target` 6 (the
  `*-returns-async-arrow-returns-newtarget` and `*-returns-async-function-returns-newtarget` cases
  of `expressions/async-function/`, `expressions/object/method-definition/` and
  `statements/async-function/`, 2 each);
- to slice 9, 4 (3): `Proxy` 3 (`AggregateError/newtarget-proto-fallback`, with
  `Reflect.construct`; `Array/fromAsync/this-constructor-operations`;
  `Promise/prototype/finally/this-value-proxy`), `Map` 1 (`Symbol/species/builtin-getter-name`,
  `from` 6, which reads the getters of `Map` and `Set`);
- to slice 12, 20 (20): `2n` 9 (the cases of `Array/fromAsync/` that include
  `temporalHelpers.js`, which holds BigInt literals, `class` and `super`, and whose
  `propertyBagObserver` makes a `Proxy`), `0n` 4 (the `resolve-throws-iterator-return-is-not-callable` cases above), `1n` 4
  (`Array/fromAsync/asyncitems-bigint`, `asyncitems-arraylike-length-accessor-throws`,
  `asyncitems-asynciterator-not-callable` and `asyncitems-iterator-not-callable`), `42n` 2
  (`Array/fromAsync/thisarg-primitive-sloppy` and `-strict`), `4n` 1
  (`AsyncIteratorPrototype/Symbol.asyncIterator/return-val`);
- to slice 13, 1 (1): `ArrayBuffer` (`Array/fromAsync/asyncitems-arraybuffer`).

The counts sum to the 141 lines this change adds to or changes in the list, 138 of them with
`from` 10.

21 cases (39 runs) come into slice 10, by the rule "A case that comes into a slice from one
delivered later" above. Each is counted by the block in a slice delivered after 10, failed at the
start of the slice (`a350de3`) on a part of slice 10 (the refusal of async functions, a counted
refusal of the async functions or `for await` it compiles with `eval` or `Function`, `Promise` not
defined, or `AggregateError` or `Symbol.asyncIterator` missing), passes in every run on the code
of the slice, has no `feature` or `flag` line of a slice delivered after 10, and neither it nor a
harness file it includes uses a part of a later slice. From slice 18, 1:
`test/annexB/language/eval-code/direct/var-env-lower-lex-catch-non-strict.js`, which declares
async functions and async generators by direct `eval` in a `catch` block whose parameter has the
same name (B.3.4, applied since slice 2). From slice 19, 20, all under `test/staging/sm/`:
`async-functions/` 9, `Promise/` 5, `AsyncGenerators/` 3, and 1 each of `Error/`
(`AggregateError`), `Symbol/` (`well-known`) and `extensions/` (`expression-closure-syntax`). Each
gets a line `dir <file> 10` (no directory has every case come in), listed with the slice it leaves
in `docs/plan/slice-10/close-into-10.tsv`; each passes in every one of its runs (the close's check
(4) over that file), and none has a line of `moves.tsv`. 66 other cases of slices delivered after
10 failed at the start of the slice and pass in every run on its code, and stay in their slice
with no line: 65 have a `feature` line of a later slice (`explicit-resource-management` 38,
`Reflect.construct` 18, `dynamic-import` 6, `class` 2, `__proto__` 1), and 1,
`staging/sm/async-functions/inner-caller`, reads the `caller` property of a function (`feature
caller 18`): it reaches the accessor of `%Function.prototype%` that throws, so its assertions never
run.

```swell-slices
order delivery 1,2,3,6,5,7,10,4,9,8,11,12,13,14,15,16,17,18,19,20,21
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
dir test/harness/nativeFunctionMatcher.js 5
dir test/harness/promiseHelper.js 6
dir test/harness/wellKnownIntrinsicObjects.js 10
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
dir test/language/eval-code/direct/gen-meth-a-following-parameter-is-named-arguments-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-meth-a-following-parameter-is-named-arguments-declare-arguments.js 7
dir test/language/eval-code/direct/gen-meth-a-preceding-parameter-is-named-arguments-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-meth-a-preceding-parameter-is-named-arguments-declare-arguments.js 7
dir test/language/eval-code/direct/gen-meth-fn-body-cntns-arguments-func-decl-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-meth-fn-body-cntns-arguments-func-decl-declare-arguments.js 7
dir test/language/eval-code/direct/gen-meth-fn-body-cntns-arguments-lex-bind-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-meth-fn-body-cntns-arguments-lex-bind-declare-arguments.js 7
dir test/language/eval-code/direct/gen-meth-fn-body-cntns-arguments-var-bind-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-meth-fn-body-cntns-arguments-var-bind-declare-arguments.js 7
dir test/language/eval-code/direct/gen-meth-no-pre-existing-arguments-bindings-are-present-declare-arguments-and-assign.js 7
dir test/language/eval-code/direct/gen-meth-no-pre-existing-arguments-bindings-are-present-declare-arguments.js 7
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
dir test/language/expressions/exponentiation/int32_min-exponent.js 4
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
dir test/language/expressions/object/method-definition/early-errors-object-method-formals-contains-super-call.js 8
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
dir test/language/expressions/optional-chaining/member-expression.js 8
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
dir test/language/expressions/typeof/built-in-exotic-objects-no-call.js 4
dir test/language/expressions/typeof/built-in-functions.js 4
dir test/language/expressions/typeof/built-in-ordinary-objects-no-call.js 9
dir test/language/expressions/typeof/get-value.js 3
dir test/language/expressions/typeof/native-call.js 4
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
dir test/built-ins/Array/from/ 6
dir test/built-ins/Array/from/items-is-arraybuffer.js 13
dir test/built-ins/Array/isArray/15.4.3.2-1-10.js 5
dir test/built-ins/Array/isArray/15.4.3.2-1-9.js 4
dir test/built-ins/Array/of/return-a-new-array-object.js 4
dir test/built-ins/Array/prototype/at/index-argument-tointeger.js 6
dir test/built-ins/Array/prototype/at/index-non-numeric-argument-tointeger-invalid.js 6
dir test/built-ins/Array/prototype/at/index-non-numeric-argument-tointeger.js 6
dir test/built-ins/Array/prototype/at/return-abrupt-from-this.js 6
dir test/built-ins/Array/prototype/at/returns-item-relative-index.js 6
dir test/built-ins/Array/prototype/at/returns-item.js 6
dir test/built-ins/Array/prototype/at/returns-undefined-for-holes-in-sparse-arrays.js 6
dir test/built-ins/Array/prototype/at/returns-undefined-for-out-of-range-index.js 6
dir test/built-ins/Array/prototype/concat/Array.prototype.concat_non-array.js 8
dir test/built-ins/Array/prototype/entries/ 6
dir test/built-ins/Array/prototype/every/15.4.4.16-1-11.js 4
dir test/built-ins/Array/prototype/every/15.4.4.16-1-12.js 5
dir test/built-ins/Array/prototype/every/15.4.4.16-2-18.js 4
dir test/built-ins/Array/prototype/every/15.4.4.16-5-15.js 4
dir test/built-ins/Array/prototype/every/15.4.4.16-5-16.js 5
dir test/built-ins/Array/prototype/every/15.4.4.16-7-c-iii-21.js 4
dir test/built-ins/Array/prototype/every/15.4.4.16-7-c-iii-22.js 5
dir test/built-ins/Array/prototype/every/call-with-boolean.js 6
dir test/built-ins/Array/prototype/filter/15.4.4.20-1-11.js 4
dir test/built-ins/Array/prototype/filter/15.4.4.20-1-12.js 5
dir test/built-ins/Array/prototype/filter/15.4.4.20-5-15.js 4
dir test/built-ins/Array/prototype/filter/15.4.4.20-5-16.js 5
dir test/built-ins/Array/prototype/filter/15.4.4.20-9-c-iii-22.js 4
dir test/built-ins/Array/prototype/filter/15.4.4.20-9-c-iii-23.js 5
dir test/built-ins/Array/prototype/filter/call-with-boolean.js 6
dir test/built-ins/Array/prototype/find/call-with-boolean.js 6
dir test/built-ins/Array/prototype/find/predicate-is-not-callable-throws.js 5
dir test/built-ins/Array/prototype/findIndex/call-with-boolean.js 6
dir test/built-ins/Array/prototype/findIndex/predicate-is-not-callable-throws.js 5
dir test/built-ins/Array/prototype/findLast/call-with-boolean.js 6
dir test/built-ins/Array/prototype/findLast/predicate-is-not-callable-throws.js 5
dir test/built-ins/Array/prototype/findLastIndex/call-with-boolean.js 6
dir test/built-ins/Array/prototype/findLastIndex/predicate-is-not-callable-throws.js 5
dir test/built-ins/Array/prototype/flat/proxy-access-count.js 9
dir test/built-ins/Array/prototype/flat/symbol-object-create-null-depth-throws.js 6
dir test/built-ins/Array/prototype/flatMap/array-like-objects.js 6
dir test/built-ins/Array/prototype/flatMap/call-with-boolean.js 6
dir test/built-ins/Array/prototype/flatMap/mapperfunction-throws.js 6
dir test/built-ins/Array/prototype/flatMap/proxy-access-count.js 9
dir test/built-ins/Array/prototype/forEach/15.4.4.18-1-11.js 4
dir test/built-ins/Array/prototype/forEach/15.4.4.18-1-12.js 5
dir test/built-ins/Array/prototype/forEach/15.4.4.18-5-15.js 4
dir test/built-ins/Array/prototype/forEach/15.4.4.18-5-16.js 5
dir test/built-ins/Array/prototype/forEach/call-with-boolean.js 6
dir test/built-ins/Array/prototype/indexOf/15.4.4.14-1-11.js 4
dir test/built-ins/Array/prototype/indexOf/15.4.4.14-1-12.js 5
dir test/built-ins/Array/prototype/indexOf/15.4.4.14-3-7.js 4
dir test/built-ins/Array/prototype/indexOf/15.4.4.14-5-12.js 4
dir test/built-ins/Array/prototype/indexOf/15.4.4.14-5-16.js 4
dir test/built-ins/Array/prototype/join/S15.4.4.5_A2_T4.js 6
dir test/built-ins/Array/prototype/keys/ 6
dir test/built-ins/Array/prototype/lastIndexOf/15.4.4.15-1-11.js 4
dir test/built-ins/Array/prototype/lastIndexOf/15.4.4.15-1-12.js 5
dir test/built-ins/Array/prototype/lastIndexOf/15.4.4.15-3-7.js 4
dir test/built-ins/Array/prototype/lastIndexOf/15.4.4.15-5-12.js 4
dir test/built-ins/Array/prototype/lastIndexOf/15.4.4.15-5-16.js 4
dir test/built-ins/Array/prototype/lastIndexOf/15.4.4.15-5-21.js 5
dir test/built-ins/Array/prototype/map/15.4.4.19-1-11.js 4
dir test/built-ins/Array/prototype/map/15.4.4.19-1-12.js 5
dir test/built-ins/Array/prototype/map/15.4.4.19-2-18.js 4
dir test/built-ins/Array/prototype/map/15.4.4.19-5-15.js 4
dir test/built-ins/Array/prototype/map/15.4.4.19-5-16.js 5
dir test/built-ins/Array/prototype/map/call-with-boolean.js 6
dir test/built-ins/Array/prototype/map/create-non-array-invalid-len.js 4
dir test/built-ins/Array/prototype/pop/throws-with-string-receiver.js 6
dir test/built-ins/Array/prototype/push/throws-with-string-receiver.js 6
dir test/built-ins/Array/prototype/reduce/15.4.4.21-1-11.js 4
dir test/built-ins/Array/prototype/reduce/15.4.4.21-1-12.js 5
dir test/built-ins/Array/prototype/reduce/15.4.4.21-9-c-ii-31.js 4
dir test/built-ins/Array/prototype/reduce/15.4.4.21-9-c-ii-32.js 5
dir test/built-ins/Array/prototype/reduce/call-with-boolean.js 6
dir test/built-ins/Array/prototype/reduceRight/15.4.4.22-1-11.js 4
dir test/built-ins/Array/prototype/reduceRight/15.4.4.22-1-12.js 5
dir test/built-ins/Array/prototype/reduceRight/15.4.4.22-9-c-ii-31.js 4
dir test/built-ins/Array/prototype/reduceRight/15.4.4.22-9-c-ii-32.js 5
dir test/built-ins/Array/prototype/reduceRight/call-with-boolean.js 6
dir test/built-ins/Array/prototype/reverse/array-has-one-entry.js 6
dir test/built-ins/Array/prototype/shift/throws-when-this-value-length-is-writable-false.js 6
dir test/built-ins/Array/prototype/slice/S15.4.4.10_A3_T1.js 6
dir test/built-ins/Array/prototype/slice/S15.4.4.10_A3_T2.js 6
dir test/built-ins/Array/prototype/slice/create-non-array-invalid-len.js 4
dir test/built-ins/Array/prototype/some/15.4.4.17-1-11.js 4
dir test/built-ins/Array/prototype/some/15.4.4.17-1-12.js 5
dir test/built-ins/Array/prototype/some/15.4.4.17-2-18.js 4
dir test/built-ins/Array/prototype/some/15.4.4.17-5-15.js 4
dir test/built-ins/Array/prototype/some/15.4.4.17-5-16.js 5
dir test/built-ins/Array/prototype/some/15.4.4.17-7-c-iii-21.js 4
dir test/built-ins/Array/prototype/some/15.4.4.17-7-c-iii-22.js 5
dir test/built-ins/Array/prototype/some/call-with-boolean.js 6
dir test/built-ins/Array/prototype/sort/S15.4.4.11_A5_T1.js 6
dir test/built-ins/Array/prototype/sort/precise-comparefn-throws.js 6
dir test/built-ins/Array/prototype/sort/precise-getter-appends-elements.js 6
dir test/built-ins/Array/prototype/sort/precise-getter-decreases-length.js 6
dir test/built-ins/Array/prototype/sort/precise-getter-deletes-predecessor.js 6
dir test/built-ins/Array/prototype/sort/precise-getter-deletes-successor.js 6
dir test/built-ins/Array/prototype/sort/precise-getter-increases-length.js 6
dir test/built-ins/Array/prototype/sort/precise-getter-pops-elements.js 6
dir test/built-ins/Array/prototype/sort/precise-getter-sets-predecessor.js 6
dir test/built-ins/Array/prototype/sort/precise-getter-sets-successor.js 6
dir test/built-ins/Array/prototype/sort/precise-prototype-accessors.js 6
dir test/built-ins/Array/prototype/sort/precise-prototype-element.js 6
dir test/built-ins/Array/prototype/sort/precise-setter-appends-elements.js 6
dir test/built-ins/Array/prototype/sort/precise-setter-decreases-length.js 6
dir test/built-ins/Array/prototype/sort/precise-setter-deletes-predecessor.js 6
dir test/built-ins/Array/prototype/sort/precise-setter-deletes-successor.js 6
dir test/built-ins/Array/prototype/sort/precise-setter-increases-length.js 6
dir test/built-ins/Array/prototype/sort/precise-setter-pops-elements.js 6
dir test/built-ins/Array/prototype/sort/precise-setter-sets-predecessor.js 6
dir test/built-ins/Array/prototype/sort/precise-setter-sets-successor.js 6
dir test/built-ins/Array/prototype/sort/stability-11-elements.js 6
dir test/built-ins/Array/prototype/sort/stability-2048-elements.js 6
dir test/built-ins/Array/prototype/sort/stability-5-elements.js 6
dir test/built-ins/Array/prototype/sort/stability-513-elements.js 6
dir test/built-ins/Array/prototype/splice/create-non-array-invalid-len.js 4
dir test/built-ins/Array/prototype/toLocaleString/invoke-element-tolocalestring.js 6
dir test/built-ins/Array/prototype/toReversed/ignores-species.js 6
dir test/built-ins/Array/prototype/toReversed/length-tolength.js 6
dir test/built-ins/Array/prototype/toReversed/this-value-nullish.js 6
dir test/built-ins/Array/prototype/toSorted/comparefn-called-after-get-elements.js 6
dir test/built-ins/Array/prototype/toSorted/comparefn-not-a-function.js 12
dir test/built-ins/Array/prototype/toSorted/comparefn-stop-after-error.js 6
dir test/built-ins/Array/prototype/toSorted/ignores-species.js 6
dir test/built-ins/Array/prototype/toSorted/length-tolength.js 6
dir test/built-ins/Array/prototype/toSorted/this-value-nullish.js 6
dir test/built-ins/Array/prototype/toSpliced/ignores-species.js 6
dir test/built-ins/Array/prototype/toSpliced/length-tolength.js 6
dir test/built-ins/Array/prototype/toSpliced/start-and-deleteCount-missing.js 6
dir test/built-ins/Array/prototype/toSpliced/start-and-deleteCount-undefineds.js 6
dir test/built-ins/Array/prototype/toSpliced/this-value-nullish.js 6
dir test/built-ins/Array/prototype/toString/S15.4.4.2_A1_T4.js 6
dir test/built-ins/Array/prototype/unshift/throws-with-string-receiver.js 6
dir test/built-ins/Array/prototype/values/ 6
dir test/built-ins/Array/prototype/with/ignores-species.js 6
dir test/built-ins/Array/prototype/with/length-tolength.js 6
dir test/built-ins/Array/prototype/with/this-value-boolean.js 6
dir test/built-ins/Array/prototype/with/this-value-nullish.js 6
dir test/built-ins/Boolean/S9.2_A6_T1.js 4
dir test/built-ins/Boolean/prototype/S15.6.3.1_A3.js 6
dir test/built-ins/Boolean/prototype/toString/S15.6.4.2_A2_T1.js 6
dir test/built-ins/Boolean/prototype/toString/S15.6.4.2_A2_T2.js 6
dir test/built-ins/Boolean/prototype/toString/S15.6.4.2_A2_T3.js 4
dir test/built-ins/Boolean/prototype/toString/S15.6.4.2_A2_T4.js 6
dir test/built-ins/Boolean/prototype/toString/S15.6.4.2_A2_T5.js 6
dir test/built-ins/Boolean/prototype/valueOf/S15.6.4.3_A2_T1.js 6
dir test/built-ins/Boolean/prototype/valueOf/S15.6.4.3_A2_T2.js 6
dir test/built-ins/Boolean/prototype/valueOf/S15.6.4.3_A2_T3.js 4
dir test/built-ins/Boolean/prototype/valueOf/S15.6.4.3_A2_T4.js 6
dir test/built-ins/Boolean/prototype/valueOf/S15.6.4.3_A2_T5.js 6
dir test/built-ins/Error/cause_abrupt.js 9
dir test/built-ins/Error/isError/non-error-objects-other-realm.js 5
dir test/built-ins/Error/isError/non-error-objects.js 5
dir test/built-ins/Error/prototype/S15.11.4_A3.js 6
dir test/built-ins/Error/prototype/S15.11.4_A4.js 6
dir test/built-ins/Error/prototype/toString/invalid-receiver.js 6
dir test/built-ins/Function/S15.3_A2_T2.js 8
dir test/built-ins/Function/prototype/apply/argarray-not-object-realm.js 6
dir test/built-ins/Function/prototype/apply/argarray-not-object.js 6
dir test/built-ins/Function/prototype/apply/this-not-callable-realm.js 5
dir test/built-ins/Function/prototype/apply/this-not-callable.js 5
dir test/built-ins/Function/prototype/arguments/ 10
dir test/built-ins/Function/prototype/bind/15.3.4.5-2-9.js 4
dir test/built-ins/Function/prototype/bind/S15.3.4.5_A16.js 5
dir test/built-ins/Function/prototype/bind/S15.3.4.5_A5.js 4
dir test/built-ins/Function/prototype/call/S15.3.4.4_A16.js 5
dir test/built-ins/Function/prototype/caller-arguments/ 6
dir test/built-ins/Function/prototype/caller/ 10
dir test/built-ins/Function/prototype/toString/Function.js 6
dir test/built-ins/Function/prototype/toString/arrow-function.js 6
dir test/built-ins/Function/prototype/toString/bound-function.js 5
dir test/built-ins/Function/prototype/toString/class-declaration-complex-heritage.js 8
dir test/built-ins/Function/prototype/toString/class-declaration-explicit-ctor.js 8
dir test/built-ins/Function/prototype/toString/class-declaration-implicit-ctor.js 8
dir test/built-ins/Function/prototype/toString/class-expression-explicit-ctor.js 8
dir test/built-ins/Function/prototype/toString/class-expression-implicit-ctor.js 8
dir test/built-ins/Function/prototype/toString/function-declaration-non-simple-parameter-list.js 6
dir test/built-ins/Function/prototype/toString/function-declaration.js 6
dir test/built-ins/Function/prototype/toString/function-expression.js 6
dir test/built-ins/Function/prototype/toString/generator-function-declaration.js 7
dir test/built-ins/Function/prototype/toString/generator-function-expression.js 7
dir test/built-ins/Function/prototype/toString/generator-method.js 7
dir test/built-ins/Function/prototype/toString/getter-class-expression-static.js 8
dir test/built-ins/Function/prototype/toString/getter-class-expression.js 8
dir test/built-ins/Function/prototype/toString/getter-class-statement-static.js 8
dir test/built-ins/Function/prototype/toString/getter-class-statement.js 8
dir test/built-ins/Function/prototype/toString/getter-object.js 6
dir test/built-ins/Function/prototype/toString/line-terminator-normalisation-CR-LF.js 6
dir test/built-ins/Function/prototype/toString/line-terminator-normalisation-CR.js 6
dir test/built-ins/Function/prototype/toString/line-terminator-normalisation-LF.js 6
dir test/built-ins/Function/prototype/toString/method-class-expression-static.js 8
dir test/built-ins/Function/prototype/toString/method-class-expression.js 8
dir test/built-ins/Function/prototype/toString/method-class-statement-static.js 8
dir test/built-ins/Function/prototype/toString/method-class-statement.js 8
dir test/built-ins/Function/prototype/toString/method-computed-property-name.js 6
dir test/built-ins/Function/prototype/toString/method-object.js 6
dir test/built-ins/Function/prototype/toString/setter-class-expression-static.js 8
dir test/built-ins/Function/prototype/toString/setter-class-expression.js 8
dir test/built-ins/Function/prototype/toString/setter-class-statement-static.js 8
dir test/built-ins/Function/prototype/toString/setter-class-statement.js 8
dir test/built-ins/Function/prototype/toString/setter-object.js 6
dir test/built-ins/Function/prototype/toString/symbol-named-builtins.js 5
dir test/built-ins/Function/prototype/toString/unicode.js 6
dir test/built-ins/JSON/isRawJSON/basic.js 6
dir test/built-ins/JSON/parse/reviver-call-args-after-forward-modification.js 6
dir test/built-ins/JSON/parse/reviver-context-source-array-literal.js 6
dir test/built-ins/JSON/parse/reviver-context-source-object-literal.js 6
dir test/built-ins/JSON/parse/reviver-context-source-primitive-literal.js 6
dir test/built-ins/JSON/parse/reviver-forward-modifies-object.js 6
dir test/built-ins/JSON/rawJSON/illegal-empty-and-start-end-chars.js 6
dir test/built-ins/JSON/rawJSON/invalid-JSON-text.js 6
dir test/built-ins/JSON/rawJSON/returns-expected-object.js 6
dir test/built-ins/JSON/stringify/value-string-escape-ascii.js 5
dir test/built-ins/JSON/stringify/value-tojson-not-function.js 5
dir test/built-ins/Object/S15.2.2.1_A2_T5.js 4
dir test/built-ins/Object/assign/assignment-to-readonly-property-of-target-must-throw-a-typeerror-exception.js 6
dir test/built-ins/Object/assign/strings-and-symbol-order.js 6
dir test/built-ins/Object/assign/target-is-sealed-existing-accessor-property.js 6
dir test/built-ins/Object/assign/target-is-sealed-existing-data-property.js 6
dir test/built-ins/Object/create/15.2.3.5-4-11.js 4
dir test/built-ins/Object/create/15.2.3.5-4-118.js 4
dir test/built-ins/Object/create/15.2.3.5-4-119.js 5
dir test/built-ins/Object/create/15.2.3.5-4-12.js 5
dir test/built-ins/Object/create/15.2.3.5-4-143.js 4
dir test/built-ins/Object/create/15.2.3.5-4-144.js 5
dir test/built-ins/Object/create/15.2.3.5-4-171.js 4
dir test/built-ins/Object/create/15.2.3.5-4-172.js 5
dir test/built-ins/Object/create/15.2.3.5-4-197.js 4
dir test/built-ins/Object/create/15.2.3.5-4-198.js 5
dir test/built-ins/Object/create/15.2.3.5-4-222.js 4
dir test/built-ins/Object/create/15.2.3.5-4-223.js 5
dir test/built-ins/Object/create/15.2.3.5-4-249.js 4
dir test/built-ins/Object/create/15.2.3.5-4-250.js 5
dir test/built-ins/Object/create/15.2.3.5-4-285.js 4
dir test/built-ins/Object/create/15.2.3.5-4-286.js 5
dir test/built-ins/Object/create/15.2.3.5-4-297.js 4
dir test/built-ins/Object/create/15.2.3.5-4-34.js 4
dir test/built-ins/Object/create/15.2.3.5-4-35.js 5
dir test/built-ins/Object/create/15.2.3.5-4-65.js 4
dir test/built-ins/Object/create/15.2.3.5-4-66.js 5
dir test/built-ins/Object/create/15.2.3.5-4-90.js 4
dir test/built-ins/Object/create/15.2.3.5-4-91.js 5
dir test/built-ins/Object/defineProperties/15.2.3.7-2-12.js 4
dir test/built-ins/Object/defineProperties/15.2.3.7-2-13.js 5
dir test/built-ins/Object/defineProperties/15.2.3.7-5-a-13.js 4
dir test/built-ins/Object/defineProperties/15.2.3.7-5-a-14.js 5
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-103.js 4
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-104.js 5
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-131.js 4
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-132.js 5
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-157.js 4
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-158.js 5
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-182.js 4
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-183.js 5
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-210.js 4
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-211.js 5
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-245.js 4
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-246.js 5
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-25.js 4
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-26.js 5
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-50.js 4
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-51.js 5
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-78.js 4
dir test/built-ins/Object/defineProperties/15.2.3.7-5-b-79.js 5
dir test/built-ins/Object/defineProperties/15.2.3.7-6-a-18.js 4
dir test/built-ins/Object/defineProperties/15.2.3.7-6-a-19.js 5
dir test/built-ins/Object/defineProperties/property-description-must-be-an-object-not-false.js 6
dir test/built-ins/Object/defineProperties/property-description-must-be-an-object-not-null.js 6
dir test/built-ins/Object/defineProperties/property-description-must-be-an-object-not-number.js 6
dir test/built-ins/Object/defineProperties/property-description-must-be-an-object-not-string.js 6
dir test/built-ins/Object/defineProperties/property-description-must-be-an-object-not-true.js 6
dir test/built-ins/Object/defineProperties/property-description-must-be-an-object-not-undefined.js 6
dir test/built-ins/Object/defineProperty/15.2.3.6-3-117.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-118.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-145-1.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-145.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-146-1.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-146.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-171-1.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-171.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-172-1.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-172.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-196.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-197.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-224-1.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-224.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-225-1.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-225.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-254-1.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-254.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-255-1.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-255.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-39-1.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-39.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-40-1.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-40.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-64.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-65.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-92-1.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-92.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-3-93-1.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-3-93.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-4-39.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-4-392.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-4-393.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-4-40.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-4-408.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-4-409.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-4-413.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-4-415.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-4-584.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-4-585.js 5
dir test/built-ins/Object/defineProperty/15.2.3.6-4-589.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-4-591.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-4-622.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-4-623.js 4
dir test/built-ins/Object/defineProperty/15.2.3.6-4-624.js 4
dir test/built-ins/Object/defineProperty/property-description-must-be-an-object-not-false.js 6
dir test/built-ins/Object/defineProperty/property-description-must-be-an-object-not-null.js 6
dir test/built-ins/Object/defineProperty/property-description-must-be-an-object-not-number.js 6
dir test/built-ins/Object/defineProperty/property-description-must-be-an-object-not-string.js 6
dir test/built-ins/Object/defineProperty/property-description-must-be-an-object-not-true.js 6
dir test/built-ins/Object/defineProperty/property-description-must-be-an-object-not-undefined.js 6
dir test/built-ins/Object/defineProperty/redefine-length-with-various-values-and-configurable-true.js 6
dir test/built-ins/Object/entries/order-after-define-property.js 6
dir test/built-ins/Object/entries/primitive-numbers.js 4
dir test/built-ins/Object/freeze/15.2.3.9-2-d-6.js 4
dir test/built-ins/Object/freeze/15.2.3.9-2-d-7.js 5
dir test/built-ins/Object/freeze/throws-when-false.js 9
dir test/built-ins/Object/fromEntries/ 6
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-100.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-101.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-102.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-103.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-104.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-105.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-107.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-108.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-109.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-110.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-111.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-112.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-113.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-114.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-115.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-116.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-117.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-118.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-120.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-121.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-122.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-123.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-124.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-125.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-126.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-127.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-128.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-129.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-130.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-131.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-132.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-133.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-134.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-135.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-136.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-138.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-139.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-140.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-141.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-142.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-143.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-144.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-145.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-146.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-147.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-148.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-149.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-150.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-151.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-152.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-153.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-154.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-156.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-157.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-158.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-159.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-160.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-161.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-162.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-163.js 5
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-165.js 5
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-166.js 5
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-167.js 5
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-202.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-203.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-204.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-205.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-206.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-207.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-208.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-209.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-210.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-211.js 5
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-212.js 5
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-213.js 5
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-214.js 5
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-215.js 5
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-5.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-6.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-68.js 5
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-69.js 5
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-70.js 5
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-72.js 5
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-90.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-91.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-92.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-93.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-96.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-97.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-98.js 4
dir test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-99.js 4
dir test/built-ins/Object/getOwnPropertyDescriptors/primitive-numbers.js 4
dir test/built-ins/Object/getOwnPropertyNames/15.2.3.4-4-1.js 4
dir test/built-ins/Object/getOwnPropertySymbols/ 6
dir test/built-ins/Object/getPrototypeOf/15.2.3.2-2-10.js 5
dir test/built-ins/Object/getPrototypeOf/15.2.3.2-2-25.js 4
dir test/built-ins/Object/getPrototypeOf/15.2.3.2-2-26.js 5
dir test/built-ins/Object/getPrototypeOf/15.2.3.2-2-9.js 4
dir test/built-ins/Object/groupBy/ 6
dir test/built-ins/Object/internals/DefineOwnProperty/consistent-value-function-arguments.js 9
dir test/built-ins/Object/internals/DefineOwnProperty/consistent-value-function-caller.js 9
dir test/built-ins/Object/internals/DefineOwnProperty/consistent-value-regexp-dollar1.js 9
dir test/built-ins/Object/internals/DefineOwnProperty/consistent-writable-regexp-dollar1.js 9
dir test/built-ins/Object/internals/DefineOwnProperty/nan-equivalence-define-own-property-reassign.js 4
dir test/built-ins/Object/internals/DefineOwnProperty/nan-equivalence-define-own-property-reconfigure.js 4
dir test/built-ins/Object/isExtensible/15.2.3.13-2-10.js 5
dir test/built-ins/Object/isExtensible/15.2.3.13-2-19.js 4
dir test/built-ins/Object/isExtensible/15.2.3.13-2-20.js 5
dir test/built-ins/Object/isExtensible/15.2.3.13-2-9.js 4
dir test/built-ins/Object/isFrozen/15.2.3.12-3-15.js 4
dir test/built-ins/Object/isFrozen/15.2.3.12-3-16.js 4
dir test/built-ins/Object/isFrozen/15.2.3.12-3-17.js 5
dir test/built-ins/Object/isFrozen/15.2.3.12-3-18.js 5
dir test/built-ins/Object/isSealed/15.2.3.11-4-15.js 4
dir test/built-ins/Object/isSealed/15.2.3.11-4-16.js 4
dir test/built-ins/Object/isSealed/15.2.3.11-4-17.js 5
dir test/built-ins/Object/isSealed/15.2.3.11-4-18.js 5
dir test/built-ins/Object/keys/15.2.3.14-6-5.js 4
dir test/built-ins/Object/preventExtensions/15.2.3.10-3-18.js 4
dir test/built-ins/Object/preventExtensions/15.2.3.10-3-19.js 5
dir test/built-ins/Object/preventExtensions/15.2.3.10-3-8.js 4
dir test/built-ins/Object/preventExtensions/15.2.3.10-3-9.js 5
dir test/built-ins/Object/preventExtensions/throws-when-false.js 9
dir test/built-ins/Object/prototype/toString/Object.prototype.toString.call-date.js 4
dir test/built-ins/Object/prototype/toString/Object.prototype.toString.call-regexp.js 5
dir test/built-ins/Object/prototype/valueOf/S15.2.4.4_A15.js 6
dir test/built-ins/Object/seal/object-seal-o-is-a-date-object.js 4
dir test/built-ins/Object/seal/object-seal-o-is-a-reg-exp-object.js 5
dir test/built-ins/Object/seal/object-seal-p-is-own-property-of-a-date-object-that-uses-object-s-get-own-property.js 4
dir test/built-ins/Object/seal/object-seal-p-is-own-property-of-a-reg-exp-object-that-uses-object-s-get-own-property.js 5
dir test/built-ins/Object/seal/seal-arraybuffer.js 13
dir test/built-ins/Object/seal/seal-arrowfunction.js 6
dir test/built-ins/Object/seal/seal-asyncarrowfunction.js 10
dir test/built-ins/Object/seal/seal-asyncfunction.js 10
dir test/built-ins/Object/seal/seal-asyncgeneratorfunction.js 10
dir test/built-ins/Object/seal/seal-bigint64array.js 13
dir test/built-ins/Object/seal/seal-biguint64array.js 13
dir test/built-ins/Object/seal/seal-dataview.js 13
dir test/built-ins/Object/seal/seal-date.js 4
dir test/built-ins/Object/seal/seal-float32array.js 13
dir test/built-ins/Object/seal/seal-float64array.js 13
dir test/built-ins/Object/seal/seal-generatorfunction.js 7
dir test/built-ins/Object/seal/seal-int16array.js 13
dir test/built-ins/Object/seal/seal-int32array.js 13
dir test/built-ins/Object/seal/seal-int8array.js 13
dir test/built-ins/Object/seal/seal-map.js 9
dir test/built-ins/Object/seal/seal-promise.js 10
dir test/built-ins/Object/seal/seal-proxy.js 9
dir test/built-ins/Object/seal/seal-regexp.js 5
dir test/built-ins/Object/seal/seal-set.js 9
dir test/built-ins/Object/seal/seal-symbol.js 6
dir test/built-ins/Object/seal/seal-uint16array.js 13
dir test/built-ins/Object/seal/seal-uint32array.js 13
dir test/built-ins/Object/seal/seal-uint8array.js 13
dir test/built-ins/Object/seal/seal-uint8clampedarray.js 13
dir test/built-ins/Object/seal/seal-weakmap.js 9
dir test/built-ins/Object/seal/seal-weakset.js 9
dir test/built-ins/Object/seal/throws-when-false.js 9
dir test/built-ins/Object/values/order-after-define-property.js 9
dir test/built-ins/Object/values/primitive-numbers.js 4
dir test/built-ins/String/15.5.5.5.2-3-5.js 4
dir test/built-ins/String/15.5.5.5.2-3-8.js 4
dir test/built-ins/String/prototype/at/index-argument-tointeger.js 6
dir test/built-ins/String/prototype/at/index-non-numeric-argument-tointeger-invalid.js 6
dir test/built-ins/String/prototype/at/index-non-numeric-argument-tointeger.js 6
dir test/built-ins/String/prototype/at/return-abrupt-from-this.js 6
dir test/built-ins/String/prototype/at/returns-code-unit.js 6
dir test/built-ins/String/prototype/at/returns-item-relative-index.js 6
dir test/built-ins/String/prototype/at/returns-item.js 6
dir test/built-ins/String/prototype/at/returns-undefined-for-out-of-range-index.js 6
dir test/built-ins/String/prototype/endsWith/searchstring-is-regexp-throws.js 5
dir test/built-ins/String/prototype/includes/searchstring-is-regexp-throws.js 5
dir test/built-ins/String/prototype/isWellFormed/to-string-primitive.js 12
dir test/built-ins/String/prototype/localeCompare/15.5.4.9_3.js 5
dir test/built-ins/String/prototype/localeCompare/15.5.4.9_CE.js 5
dir test/built-ins/String/prototype/normalize/ 5
dir test/built-ins/String/prototype/startsWith/searchstring-is-regexp-throws.js 5
dir test/built-ins/String/prototype/substring/S15.5.4.15_A3_T9.js 4
dir test/built-ins/String/prototype/toLocaleLowerCase/Final_Sigma_U180E.js 5
dir test/built-ins/String/prototype/toLocaleLowerCase/S15.5.4.17_A1_T14.js 5
dir test/built-ins/String/prototype/toLocaleLowerCase/special_casing.js 5
dir test/built-ins/String/prototype/toLocaleLowerCase/special_casing_conditional.js 5
dir test/built-ins/String/prototype/toLocaleLowerCase/supplementary_plane.js 5
dir test/built-ins/String/prototype/toLocaleUpperCase/S15.5.4.19_A1_T14.js 5
dir test/built-ins/String/prototype/toLocaleUpperCase/special_casing.js 5
dir test/built-ins/String/prototype/toLocaleUpperCase/supplementary_plane.js 5
dir test/built-ins/String/prototype/toLowerCase/Final_Sigma_U180E.js 5
dir test/built-ins/String/prototype/toLowerCase/S15.5.4.16_A1_T14.js 5
dir test/built-ins/String/prototype/toLowerCase/special_casing.js 5
dir test/built-ins/String/prototype/toLowerCase/special_casing_conditional.js 5
dir test/built-ins/String/prototype/toLowerCase/supplementary_plane.js 5
dir test/built-ins/String/prototype/toString/non-generic-realm.js 6
dir test/built-ins/String/prototype/toString/non-generic.js 6
dir test/built-ins/String/prototype/toUpperCase/S15.5.4.18_A1_T14.js 5
dir test/built-ins/String/prototype/toUpperCase/special_casing.js 5
dir test/built-ins/String/prototype/toUpperCase/supplementary_plane.js 5
dir test/built-ins/String/prototype/toWellFormed/to-string-primitive.js 12
dir test/built-ins/String/prototype/trim/15.5.4.20-2-49.js 5
dir test/built-ins/String/prototype/trimEnd/this-value-symbol-typeerror.js 6
dir test/built-ins/String/prototype/trimStart/this-value-symbol-typeerror.js 6
dir test/built-ins/String/prototype/valueOf/non-generic-realm.js 6
dir test/built-ins/String/prototype/valueOf/non-generic.js 6
dir test/built-ins/String/raw/return-the-string-value-from-template.js 6
dir test/built-ins/String/raw/special-characters.js 6
dir test/built-ins/String/raw/template-substitutions-are-appended-on-same-index.js 6
dir test/built-ins/String/raw/zero-literal-segments.js 6
dir test/built-ins/ThrowTypeError/unique-per-realm-non-simple.js 6
dir test/built-ins/global/S10.2.3_A1.1_T2.js 4
dir test/built-ins/global/S10.2.3_A1.1_T3.js 4
dir test/built-ins/global/S10.2.3_A1.2_T2.js 4
dir test/built-ins/global/S10.2.3_A1.2_T3.js 4
dir test/built-ins/global/S10.2.3_A1.3_T2.js 4
dir test/built-ins/global/S10.2.3_A1.3_T3.js 4
dir test/built-ins/global/global-object.js 4
dir test/built-ins/isFinite/S15.1.2.5_A2.7.js 6
dir test/built-ins/isFinite/return-true-for-valid-finite-numbers.js 4
dir test/built-ins/isNaN/S15.1.2.4_A2.7.js 6
dir test/built-ins/isNaN/return-false-not-nan-numbers.js 4
dir test/built-ins/isNaN/return-true-nan.js 4
dir test/built-ins/Array/prototype/concat/Array.prototype.concat_large-typed-array.js 13
dir test/built-ins/Array/prototype/concat/Array.prototype.concat_small-typed-array.js 13
dir test/built-ins/Array/prototype/flatMap/this-value-ctor-object-species-custom-ctor-poisoned-throws.js 8
dir test/built-ins/Array/prototype/flatMap/this-value-ctor-object-species-custom-ctor.js 8
dir test/built-ins/Array/prototype/splice/create-species-length-exceeding-integer-limit.js 9
dir test/built-ins/Object/keys/proxy-keys.js 9
dir test/built-ins/RegExp/prototype/dotAll/this-val-non-obj.js 12
dir test/built-ins/RegExp/prototype/flags/this-val-non-obj.js 12
dir test/built-ins/RegExp/prototype/global/this-val-non-obj.js 12
dir test/built-ins/RegExp/prototype/hasIndices/this-val-non-obj.js 12
dir test/built-ins/RegExp/prototype/ignoreCase/this-val-non-obj.js 12
dir test/built-ins/RegExp/prototype/multiline/this-val-non-obj.js 12
dir test/built-ins/RegExp/prototype/source/this-val-non-obj.js 12
dir test/built-ins/RegExp/prototype/sticky/this-val-non-obj.js 12
dir test/built-ins/RegExp/prototype/unicode/this-val-non-obj.js 12
dir test/built-ins/RegExp/prototype/unicodeSets/this-val-non-obj.js 12
dir test/built-ins/String/prototype/match/cstm-matcher-on-bigint-primitive.js 12
dir test/built-ins/String/prototype/matchAll/cstm-matchall-on-bigint-primitive.js 12
dir test/built-ins/String/prototype/replace/cstm-replace-on-bigint-primitive.js 12
dir test/built-ins/String/prototype/replaceAll/cstm-replaceall-on-bigint-primitive.js 12
dir test/built-ins/String/prototype/search/cstm-search-on-bigint-primitive.js 12
dir test/built-ins/String/prototype/split/cstm-split-on-bigint-primitive.js 12
dir test/built-ins/Symbol/prototype/description/this-val-non-symbol.js 9
dir test/built-ins/Symbol/species/builtin-getter-name.js 9
dir test/built-ins/Symbol/species/subclassing.js 8
dir test/language/computed-property-names/class/ 8
dir test/language/computed-property-names/object/accessor/getter-super.js 8
dir test/language/computed-property-names/object/accessor/setter-super.js 8
dir test/language/computed-property-names/object/method/generator.js 7
dir test/language/computed-property-names/object/method/super.js 8
dir test/language/computed-property-names/to-name-side-effects/class.js 8
dir test/language/computed-property-names/to-name-side-effects/numbers-class.js 8
dir test/language/destructuring/binding/syntax/destructuring-array-parameters-function-arguments-length.js 10
dir test/language/destructuring/binding/syntax/destructuring-object-parameters-function-arguments-length.js 10
dir test/language/expressions/arrow-function/dstr/ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/arrow-function/dstr/dflt-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/arrow-function/dstr/dflt-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/arrow-function/dstr/obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/arrow-function/lexical-super-call-from-within-constructor.js 8
dir test/language/expressions/arrow-function/lexical-super-property-from-within-constructor.js 8
dir test/language/expressions/arrow-function/lexical-super-property.js 8
dir test/language/expressions/arrow-function/lexical-supercall-from-immediately-invoked-arrow.js 8
dir test/language/expressions/assignment/destructuring/default-expr-throws-iterator-return-is-not-callable.js 12
dir test/language/expressions/assignment/destructuring/target-assign-throws-iterator-return-is-not-callable.js 12
dir test/language/expressions/function/dstr/ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/function/dstr/dflt-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/function/dstr/dflt-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/function/dstr/obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/object/cpn-obj-lit-computed-property-name-from-async-arrow-function-expression.js 10
dir test/language/expressions/object/cpn-obj-lit-computed-property-name-from-generator-function-declaration.js 7
dir test/language/expressions/object/cpn-obj-lit-computed-property-name-from-yield-expression.js 7
dir test/language/expressions/object/dstr/meth-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/object/dstr/meth-dflt-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/object/dstr/meth-dflt-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/object/dstr/meth-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/rest-parameters/with-new-target.js 8
dir test/language/statements/const/dstr/ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/const/dstr/obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-of/decl-cls.js 8
dir test/language/statements/for-of/dstr/const-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-of/dstr/const-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-of/dstr/let-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-of/dstr/let-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-of/dstr/var-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-of/dstr/var-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-of/head-lhs-async-invalid.js 10
dir test/language/statements/for-of/iterator-next-result-type.js 9
dir test/language/statements/for/dstr/const-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for/dstr/const-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for/dstr/let-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for/dstr/let-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for/dstr/var-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for/dstr/var-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/function/dstr/ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/function/dstr/dflt-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/function/dstr/dflt-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/function/dstr/obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/let/dstr/ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/let/dstr/obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/try/dstr/ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/try/dstr/obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/variable/dstr/ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/variable/dstr/obj-ptrn-id-init-fn-name-class.js 8
dir test/built-ins/Array/prototype/concat/Array.prototype.concat_spreadable-reg-exp.js 5
dir test/built-ins/Array/prototype/sort/comparefn-nonfunction-call-throws.js 5
dir test/built-ins/Object/prototype/toString/symbol-tag-override-instances.js 4
dir test/built-ins/String/prototype/endsWith/return-abrupt-from-searchstring-regexp-test.js 5
dir test/built-ins/String/prototype/includes/return-abrupt-from-searchstring-regexp-test.js 5
dir test/built-ins/String/prototype/startsWith/return-abrupt-from-searchstring-regexp-test.js 5
dir test/built-ins/Symbol/prototype/Symbol.toPrimitive/removed-symbol-wrapper-ordinary-toprimitive.js 4
dir test/built-ins/RegExp/match-indices/indices-array-non-unicode-match.js 9
dir test/built-ins/RegExp/match-indices/indices-array-unicode-match.js 9
dir test/built-ins/RegExp/named-groups/groups-object-subclass-sans.js 8
dir test/built-ins/RegExp/named-groups/groups-object-subclass.js 8
dir test/built-ins/RegExp/prototype/Symbol.replace/coerce-lastindex.js 4
dir test/built-ins/RegExp/prototype/Symbol.split/species-ctor-species-non-ctor.js 4
dir test/built-ins/RegExp/prototype/exec/S15.10.6.2_A1_T12.js 4
dir test/built-ins/String/prototype/split/call-split-1-math-pow-2-32-1-instance-is-number.js 4
dir test/built-ins/String/prototype/split/separator-number-limit-math-pow-2-32-1-instance-is-number.js 4
dir test/built-ins/String/prototype/split/separator-undef-limit-custom.js 4
dir test/built-ins/String/prototype/split/separator-undef-limit-zero.js 4
dir test/annexB/built-ins/RegExp/RegExp-decimal-escape-class-range.js 5
dir test/annexB/built-ins/RegExp/RegExp-decimal-escape-not-capturing.js 5
dir test/annexB/built-ins/RegExp/RegExp-invalid-control-escape-character-class-range.js 5
dir test/annexB/built-ins/RegExp/RegExp-leading-escape-BMP.js 5
dir test/annexB/built-ins/RegExp/RegExp-leading-escape.js 5
dir test/annexB/built-ins/RegExp/RegExp-trailing-escape-BMP.js 5
dir test/annexB/built-ins/RegExp/RegExp-trailing-escape.js 5
dir test/annexB/built-ins/RegExp/incomplete_hex_unicode_escape.js 5
dir test/annexB/built-ins/RegExp/named-groups/ 5
dir test/annexB/language/literals/regexp/ 5
dir test/language/arguments-object/cls-decl-gen-meth-args-trailing-comma-multiple.js 8
dir test/language/arguments-object/cls-decl-gen-meth-args-trailing-comma-null.js 8
dir test/language/arguments-object/cls-decl-gen-meth-args-trailing-comma-single-args.js 8
dir test/language/arguments-object/cls-decl-gen-meth-args-trailing-comma-spread-operator.js 8
dir test/language/arguments-object/cls-decl-gen-meth-args-trailing-comma-undefined.js 8
dir test/language/arguments-object/cls-decl-gen-meth-static-args-trailing-comma-multiple.js 8
dir test/language/arguments-object/cls-decl-gen-meth-static-args-trailing-comma-null.js 8
dir test/language/arguments-object/cls-decl-gen-meth-static-args-trailing-comma-single-args.js 8
dir test/language/arguments-object/cls-decl-gen-meth-static-args-trailing-comma-spread-operator.js 8
dir test/language/arguments-object/cls-decl-gen-meth-static-args-trailing-comma-undefined.js 8
dir test/language/arguments-object/cls-expr-gen-meth-args-trailing-comma-multiple.js 8
dir test/language/arguments-object/cls-expr-gen-meth-args-trailing-comma-null.js 8
dir test/language/arguments-object/cls-expr-gen-meth-args-trailing-comma-single-args.js 8
dir test/language/arguments-object/cls-expr-gen-meth-args-trailing-comma-spread-operator.js 8
dir test/language/arguments-object/cls-expr-gen-meth-args-trailing-comma-undefined.js 8
dir test/language/arguments-object/cls-expr-gen-meth-static-args-trailing-comma-multiple.js 8
dir test/language/arguments-object/cls-expr-gen-meth-static-args-trailing-comma-null.js 8
dir test/language/arguments-object/cls-expr-gen-meth-static-args-trailing-comma-single-args.js 8
dir test/language/arguments-object/cls-expr-gen-meth-static-args-trailing-comma-spread-operator.js 8
dir test/language/arguments-object/cls-expr-gen-meth-static-args-trailing-comma-undefined.js 8
dir test/language/block-scope/syntax/redeclaration/class-name-redeclaration-attempt-with-generator.js 8
dir test/language/block-scope/syntax/redeclaration/generator-name-redeclaration-attempt-with-class.js 8
dir test/language/expressions/generators/dstr/ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/generators/dstr/dflt-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/generators/dstr/dflt-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/generators/dstr/obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/object/concise-generator.js 8
dir test/language/expressions/object/dstr/gen-meth-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/object/dstr/gen-meth-dflt-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/object/dstr/gen-meth-dflt-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/object/dstr/gen-meth-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/object/method-definition/generator-super-call-body.js 8
dir test/language/expressions/object/method-definition/generator-super-call-param.js 8
dir test/language/expressions/object/method-definition/generator-super-prop-body.js 8
dir test/language/statements/generators/dstr/ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/generators/dstr/dflt-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/generators/dstr/dflt-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/generators/dstr/obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/switch/syntax/redeclaration/class-name-redeclaration-attempt-with-generator.js 8
dir test/language/statements/switch/syntax/redeclaration/generator-name-redeclaration-attempt-with-class.js 8
dir test/annexB/built-ins/RegExp/RegExp-control-escape-russian-letter.js 7
dir test/annexB/built-ins/RegExp/RegExp-invalid-control-escape-character-class.js 7
dir test/staging/sm/Array/sort_small.js 7
dir test/staging/sm/Symbol/for-in-order.js 7
dir test/staging/sm/class/compPropNames.js 7
dir test/staging/sm/class/methDefnGen.js 7
dir test/staging/sm/expressions/object-literal-__proto__.js 7
dir test/staging/sm/generators/delegating-yield-10.js 7
dir test/staging/sm/generators/delegating-yield-12.js 7
dir test/staging/sm/generators/delegating-yield-2.js 7
dir test/staging/sm/generators/delegating-yield-3.js 7
dir test/staging/sm/generators/delegating-yield-4.js 7
dir test/staging/sm/generators/objects.js 7
dir test/staging/sm/generators/regress-366941.js 7
dir test/staging/sm/generators/return-finally.js 7
dir test/staging/sm/generators/syntax.js 7
dir test/staging/sm/generators/yield-iterator-close.js 7
dir test/staging/sm/generators/yield-star-iterator-close.js 7
dir test/staging/sm/generators/yield-star-iterator-primitive.js 7
dir test/staging/sm/lexical-environment/block-scoped-functions-annex-b-generators.js 7
dir test/staging/sm/regress/regress-530879.js 7
dir test/built-ins/AggregateError/newtarget-proto-fallback.js 9
dir test/built-ins/Array/fromAsync/asyncitems-arraybuffer.js 13
dir test/built-ins/Array/fromAsync/asyncitems-arraylike-length-accessor-throws.js 12
dir test/built-ins/Array/fromAsync/asyncitems-arraylike-promise.js 12
dir test/built-ins/Array/fromAsync/asyncitems-asynciterator-exists.js 12
dir test/built-ins/Array/fromAsync/asyncitems-asynciterator-not-callable.js 12
dir test/built-ins/Array/fromAsync/asyncitems-asynciterator-null.js 12
dir test/built-ins/Array/fromAsync/asyncitems-asynciterator-sync.js 12
dir test/built-ins/Array/fromAsync/asyncitems-bigint.js 12
dir test/built-ins/Array/fromAsync/asyncitems-iterator-exists.js 12
dir test/built-ins/Array/fromAsync/asyncitems-iterator-not-callable.js 12
dir test/built-ins/Array/fromAsync/asyncitems-iterator-null.js 12
dir test/built-ins/Array/fromAsync/asyncitems-iterator-promise.js 12
dir test/built-ins/Array/fromAsync/asyncitems-operations.js 12
dir test/built-ins/Array/fromAsync/mapfn-result-awaited-once-per-iteration.js 12
dir test/built-ins/Array/fromAsync/this-constructor-operations.js 9
dir test/built-ins/Array/fromAsync/this-constructor-with-bad-length-setter.js 8
dir test/built-ins/Array/fromAsync/thisarg-primitive-sloppy.js 12
dir test/built-ins/Array/fromAsync/thisarg-primitive-strict.js 12
dir test/built-ins/AsyncGeneratorPrototype/throw/throw-suspendedYield-try-finally-return.js 8
dir test/built-ins/AsyncIteratorPrototype/Symbol.asyncIterator/return-val.js 12
dir test/built-ins/Function/prototype/toString/async-generator-method-class-expression-static.js 8
dir test/built-ins/Function/prototype/toString/async-generator-method-class-expression.js 8
dir test/built-ins/Function/prototype/toString/async-generator-method-class-statement-static.js 8
dir test/built-ins/Function/prototype/toString/async-generator-method-class-statement.js 8
dir test/built-ins/Function/prototype/toString/async-method-class-expression-static.js 8
dir test/built-ins/Function/prototype/toString/async-method-class-expression.js 8
dir test/built-ins/Function/prototype/toString/async-method-class-statement-static.js 8
dir test/built-ins/Function/prototype/toString/async-method-class-statement.js 8
dir test/built-ins/Promise/all/resolve-throws-iterator-return-is-not-callable.js 12
dir test/built-ins/Promise/all/resolve-throws-iterator-return-null-or-undefined.js 8
dir test/built-ins/Promise/allSettled/resolve-throws-iterator-return-is-not-callable.js 12
dir test/built-ins/Promise/allSettled/resolve-throws-iterator-return-null-or-undefined.js 8
dir test/built-ins/Promise/any/resolve-throws-iterator-return-is-not-callable.js 12
dir test/built-ins/Promise/any/resolve-throws-iterator-return-null-or-undefined.js 8
dir test/built-ins/Promise/prototype/finally/rejected-observable-then-calls-PromiseResolve.js 8
dir test/built-ins/Promise/prototype/finally/resolved-observable-then-calls-PromiseResolve.js 8
dir test/built-ins/Promise/prototype/finally/species-constructor.js 8
dir test/built-ins/Promise/prototype/finally/subclass-reject-count.js 8
dir test/built-ins/Promise/prototype/finally/subclass-resolve-count.js 8
dir test/built-ins/Promise/prototype/finally/subclass-species-constructor-reject-count.js 8
dir test/built-ins/Promise/prototype/finally/subclass-species-constructor-resolve-count.js 8
dir test/built-ins/Promise/prototype/finally/this-value-proxy.js 9
dir test/built-ins/Promise/race/resolve-throws-iterator-return-is-not-callable.js 12
dir test/built-ins/Promise/race/resolve-throws-iterator-return-null-or-undefined.js 8
dir test/language/arguments-object/cls-decl-async-gen-meth-args-trailing-comma-multiple.js 8
dir test/language/arguments-object/cls-decl-async-gen-meth-args-trailing-comma-null.js 8
dir test/language/arguments-object/cls-decl-async-gen-meth-args-trailing-comma-single-args.js 8
dir test/language/arguments-object/cls-decl-async-gen-meth-args-trailing-comma-spread-operator.js 8
dir test/language/arguments-object/cls-decl-async-gen-meth-args-trailing-comma-undefined.js 8
dir test/language/arguments-object/cls-decl-async-gen-meth-static-args-trailing-comma-multiple.js 8
dir test/language/arguments-object/cls-decl-async-gen-meth-static-args-trailing-comma-null.js 8
dir test/language/arguments-object/cls-decl-async-gen-meth-static-args-trailing-comma-single-args.js 8
dir test/language/arguments-object/cls-decl-async-gen-meth-static-args-trailing-comma-spread-operator.js 8
dir test/language/arguments-object/cls-decl-async-gen-meth-static-args-trailing-comma-undefined.js 8
dir test/language/arguments-object/cls-expr-async-gen-meth-args-trailing-comma-multiple.js 8
dir test/language/arguments-object/cls-expr-async-gen-meth-args-trailing-comma-null.js 8
dir test/language/arguments-object/cls-expr-async-gen-meth-args-trailing-comma-single-args.js 8
dir test/language/arguments-object/cls-expr-async-gen-meth-args-trailing-comma-spread-operator.js 8
dir test/language/arguments-object/cls-expr-async-gen-meth-args-trailing-comma-undefined.js 8
dir test/language/arguments-object/cls-expr-async-gen-meth-static-args-trailing-comma-multiple.js 8
dir test/language/arguments-object/cls-expr-async-gen-meth-static-args-trailing-comma-null.js 8
dir test/language/arguments-object/cls-expr-async-gen-meth-static-args-trailing-comma-single-args.js 8
dir test/language/arguments-object/cls-expr-async-gen-meth-static-args-trailing-comma-spread-operator.js 8
dir test/language/arguments-object/cls-expr-async-gen-meth-static-args-trailing-comma-undefined.js 8
dir test/language/block-scope/syntax/redeclaration/async-function-name-redeclaration-attempt-with-class.js 8
dir test/language/block-scope/syntax/redeclaration/async-generator-name-redeclaration-attempt-with-class.js 8
dir test/language/block-scope/syntax/redeclaration/class-name-redeclaration-attempt-with-async-function.js 8
dir test/language/block-scope/syntax/redeclaration/class-name-redeclaration-attempt-with-async-generator.js 8
dir test/language/expressions/async-arrow-function/early-errors-arrow-body-contains-super-call.js 8
dir test/language/expressions/async-arrow-function/early-errors-arrow-body-contains-super-property.js 8
dir test/language/expressions/async-arrow-function/early-errors-arrow-formals-contains-super-call.js 8
dir test/language/expressions/async-arrow-function/early-errors-arrow-formals-contains-super-property.js 8
dir test/language/expressions/async-function/early-errors-expression-body-contains-super-call.js 8
dir test/language/expressions/async-function/early-errors-expression-body-contains-super-property.js 8
dir test/language/expressions/async-function/early-errors-expression-formals-contains-super-call.js 8
dir test/language/expressions/async-function/early-errors-expression-formals-contains-super-property.js 8
dir test/language/expressions/async-function/named-returns-async-arrow-returns-newtarget.js 8
dir test/language/expressions/async-function/named-returns-async-function-returns-newtarget.js 8
dir test/language/expressions/async-generator/dstr/ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/async-generator/dstr/dflt-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/async-generator/dstr/dflt-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/async-generator/dstr/named-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/async-generator/dstr/named-dflt-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/async-generator/dstr/named-dflt-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/async-generator/dstr/named-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/async-generator/dstr/obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/async-generator/early-errors-expression-body-contains-super-call.js 8
dir test/language/expressions/async-generator/early-errors-expression-body-contains-super-property.js 8
dir test/language/expressions/async-generator/early-errors-expression-formals-contains-super-call.js 8
dir test/language/expressions/async-generator/early-errors-expression-formals-contains-super-property.js 8
dir test/language/expressions/object/dstr/async-gen-meth-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/object/dstr/async-gen-meth-dflt-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/expressions/object/dstr/async-gen-meth-dflt-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/object/dstr/async-gen-meth-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/expressions/object/method-definition/async-returns-async-arrow-returns-newtarget.js 8
dir test/language/expressions/object/method-definition/async-returns-async-function-returns-newtarget.js 8
dir test/language/expressions/object/method-definition/async-super-call-body.js 8
dir test/language/expressions/object/method-definition/async-super-call-param.js 8
dir test/language/expressions/object/method-definition/early-errors-object-method-body-contains-super-call.js 8
dir test/language/statements/async-function/early-errors-declaration-body-contains-super-call.js 8
dir test/language/statements/async-function/early-errors-declaration-body-contains-super-property.js 8
dir test/language/statements/async-function/early-errors-declaration-formals-contains-super-call.js 8
dir test/language/statements/async-function/early-errors-declaration-formals-contains-super-property.js 8
dir test/language/statements/async-function/returns-async-arrow-returns-newtarget.js 8
dir test/language/statements/async-function/returns-async-function-returns-newtarget.js 8
dir test/language/statements/async-generator/dstr/ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/async-generator/dstr/dflt-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/async-generator/dstr/dflt-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/async-generator/dstr/obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-func-dstr-const-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-func-dstr-const-async-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-func-dstr-const-async-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-func-dstr-const-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-func-dstr-let-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-func-dstr-let-async-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-func-dstr-let-async-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-func-dstr-let-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-func-dstr-var-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-func-dstr-var-async-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-func-dstr-var-async-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-func-dstr-var-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-gen-dstr-const-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-gen-dstr-const-async-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-gen-dstr-const-async-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-gen-dstr-const-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-gen-dstr-let-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-gen-dstr-let-async-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-gen-dstr-let-async-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-gen-dstr-let-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-gen-dstr-var-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-gen-dstr-var-async-ary-ptrn-elem-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-gen-dstr-var-async-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/for-await-of/async-gen-dstr-var-obj-ptrn-id-init-fn-name-class.js 8
dir test/language/statements/switch/syntax/redeclaration/async-function-name-redeclaration-attempt-with-class.js 8
dir test/language/statements/switch/syntax/redeclaration/async-generator-name-redeclaration-attempt-with-class.js 8
dir test/language/statements/switch/syntax/redeclaration/class-name-redeclaration-attempt-with-async-function.js 8
dir test/language/statements/switch/syntax/redeclaration/class-name-redeclaration-attempt-with-async-generator.js 8
dir test/annexB/language/eval-code/direct/var-env-lower-lex-catch-non-strict.js 10
dir test/staging/sm/AsyncGenerators/async-generator-declaration-in-modules.js 10
dir test/staging/sm/AsyncGenerators/for-await-bad-syntax.js 10
dir test/staging/sm/AsyncGenerators/for-await-of-error.js 10
dir test/staging/sm/Error/AggregateError.js 10
dir test/staging/sm/Promise/bug-1287334.js 10
dir test/staging/sm/Promise/bug-1289040.js 10
dir test/staging/sm/Promise/for-of-iterator-uses-getv.js 10
dir test/staging/sm/Promise/methods-non-enumerable.js 10
dir test/staging/sm/Promise/promise-species.js 10
dir test/staging/sm/Symbol/well-known.js 10
dir test/staging/sm/async-functions/async-contains-unicode-escape.js 10
dir test/staging/sm/async-functions/async-property-name-error.js 10
dir test/staging/sm/async-functions/await-error.js 10
dir test/staging/sm/async-functions/await-in-arrow-parameters.js 10
dir test/staging/sm/async-functions/await-in-parameters-of-async-func.js 10
dir test/staging/sm/async-functions/constructor.js 10
dir test/staging/sm/async-functions/length.js 10
dir test/staging/sm/async-functions/no-expression-closure.js 10
dir test/staging/sm/async-functions/toString.js 10
dir test/staging/sm/extensions/expression-closure-syntax.js 10
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
- The time test262 takes and every speed cost, in this document's text. Corrected on 2026-11-23
  from "The Rust stack one re-entry needs ...: no code exists.": the Rust stack is measured
  (`STACK_BYTES_PER_REENTRY`, 14 KiB per re-entry, and `Limits::min_stack_bytes()`, decision 9,
  "The Rust stack"), and so is the size of `Value` (16 bytes, held by the test
  `value_is_16_bytes` of `crates/swell`).
- The published ES2026 edition was not read.

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
# "flag <name> <slice>", "exclude <feature> <reason>", and "order delivery <s>,<s>,...",
# the numbered slices in the order they are delivered. A case's slice is the slice
# delivered last among its longest matching directory prefix, its features and its flags;
# a case with an excluded feature gets "x-<reason>". Prints: slice, path, flags
# (tab-separated).
FNR == NR {
  if ($1 == "dir") dir[$2] = $3 + 0
  else if ($1 == "feature") feat[$2] = $3 + 0
  else if ($1 == "flag") flag[$2] = $3 + 0
  else if ($1 == "exclude") excl[$2] = $3
  else if ($1 == "order") { n = split($3, o, ","); for (i = 1; i <= n; i++) at[o[i] + 0] = i }
  next
}
{
  split($0, f, "\t"); path = f[1]; best = -1; slice = 0; ex = ""
  for (p in dir)
    if (index(path, p) == 1 && length(p) > best) { best = length(p); slice = dir[p] }
  n = split(f[3], fs, ",")
  for (i = 1; i <= n; i++) {
    if (fs[i] in excl) ex = "x-" excl[fs[i]]
    else if ((fs[i] in feat) && at[feat[fs[i]]] > at[slice]) slice = feat[fs[i]]
  }
  n = split(f[2], gs, ",")
  for (i = 1; i <= n; i++) if ((gs[i] in flag) && at[flag[gs[i]]] > at[slice]) slice = flag[gs[i]]
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
03 7011 13829
04 1612 3184
05 2724 5441
06 3898 7351
07 1472 2801
08 9302 18455
09 1785 3541
10 3759 7258
11 1281 1833
12 358 714
13 3061 6078
14 598 1192
15 1094 2170
16 82 164
17 35 35
18 1134 1451
19 1435 2692
20 4617 9234
21 1662 3324
x-stage3 2591 5008
```

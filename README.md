<div align="center">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/img/swell-logo-dark.svg"><img alt="swell" src="docs/img/swell-logo.svg" width="320"></picture>

**An ECMAScript (JavaScript) engine written from scratch in Rust.**

[Docs](https://beyond10x.github.io/swell/) · [Releases](https://github.com/beyond10x/swell/releases) · [Changelog](CHANGELOG.md)
</div>

# swell

swell runs JavaScript. It parses a script, compiles it to bytecode and interprets it. Modules
are the eleventh of its nineteen delivery slices and are not parsed yet. It is written to be
embedded by [surf](https://github.com/beyond10x/surf), a browser built
from scratch. It is a project of its own: any program that runs JavaScript it does not trust
can use it.

## State

swell runs scripts of the ES5 grammar and of the bindings and iteration of ES2015 in both
modes, with regular expressions, under a step budget and a memory budget.
Its gate holds the test262 counts: 17,775 of the 44,812 cases that run pass (39.7 %), measured on 2026-11-23.
Four of nineteen delivery slices are delivered:

| slice | what it holds | cases |
|---|---|---|
| 1 | the test262 harness itself | 31, every one passes |
| 2 | the ES5-level language with `with`, `arguments`, `eval` and the global `Function` | 4,259, every one passes |
| 3 | the core library: `Object`, `Function`, `Array`, `String`, `JSON`, `globalThis`, the URI functions | 7,010 of 7,011 pass |
| 6 | bindings and iteration: `let`, `const`, templates, arrow functions, destructuring, spread, `Symbol`, iterators, `for`-`of` | 3,898, every one passes |

A delivered slice passes the test262 cases it holds, but for an exception the design states.
The slices are not delivered in the order of their numbers, and a case is counted in the slice
delivered last among the parts it needs: a `RegExp` case that uses `Symbol` counts in slice 5.

What exists, measured on 2026-11-23:

- The design, [`docs/design/engine.md`](docs/design/engine.md): what swell targets, how it is
  built, how it is measured, and the order in which it is delivered.
- The host interface, specified before its code:
  [`docs/design/host-interface.md`](docs/design/host-interface.md) has 191 numbered
  requirements, and 113 of them have a test. An ESS model under [`spec/`](spec/) has 58
  scenarios. All 58 pass on ESS's interpreter. The gate runs them against the engine too, and
  14 pass there.
- The gate, `task check`: test262 pinned by commit and checksum, the Unicode 17.0.0 data
  pinned by checksum, both specifications checked, and 65 planted defects that the gate must
  refuse. Its test steps have a budget of 120 seconds and a test is ended at 60.
- The test262 runner, `swell-test262`. It reads all 53,801 cases, excludes 8,982 by recorded
  decision and runs the rest in both modes under budgets. It counts them per directory, per
  delivery slice and per section of ECMA-262 (42). Its 287 lines are compared
  with a committed file on every run.
- Refusals. A construct the parser does not have yet is refused, and a refusal never counts
  as a pass, also when the script catches its `SyntaxError`. The engine counts its refusals:
  36,178 of the 85,556 runs meet one.
- The first parts of the engine:

  | part | crate |
  |---|---|
  | a parser for the ES5 grammar with strict mode | `crates/syntax` |
  | a compiler to bytecode | `crates/compiler` |
  | the heap, the values, objects, realms, the interpreter, the builtins the test262 harness calls, and the host entry points that parse and evaluate a script | `crates/swell` |
  | number parsing and printing | `crates/num` |

Releases are listed in [`CHANGELOG.md`](CHANGELOG.md), each with what works and its limits.
[`docs/releasing.md`](docs/releasing.md) says how one is cut.

The one case of slice 3 that does not pass, `test/built-ins/String/prop-desc.js`, fails in
the pinned test262 harness before the engine's answer is judged;
[the design, section 12](docs/design/engine.md#12-delivery-order) states it.

Slice 6, bindings and iteration, was delivered on 2026-11-23: `let` and `const`, template
literals with and without a tag, arrow functions, default and rest parameters, object and
array destructuring, computed property names, spread in objects, arrays and calls, `Symbol`,
the iterator protocol, `for`-`of`, function declarations in blocks and `Array.from`. All
3,898 of its cases pass in all 7,351 runs; 402 cases that also need slice 4 or 5 are counted
there. Slice 5, regular expressions, is in work: literals, `RegExp`, `exec` and `test` run, and
1,327 of its 2,724 cases pass. The slices are taken in the order the first
user, a browser, needs them: 6, 5, 7, 10, 4, 9.

## What swell is

- An interpreter for the language of ECMA-262, the ECMAScript standard, held to test262, the
  standard's own test suite.
- A library with a narrow host interface. The program that embeds it supplies the clock, the time
  zone, random numbers, module loading and the scheduling of jobs. swell has no other way to reach
  files, the network, time or randomness, so a run repeats exactly when the host's answers do.
- Built for hostile input. A script cannot hang, crash or exhaust the program that runs it.
  Every run has a step budget, a memory budget and depth limits, and each one ends the run with
  an outcome the host can read.

## What swell is not

- Not fast first. It has no just-in-time compiler and never writes machine code.
- Not a wrapper around V8, SpiderMonkey, JavaScriptCore or any other engine.
- Not complete. Nothing is marked complete yet.
- Not a browser. The DOM and everything else a web page sees beyond the language belong to the
  program that embeds swell.

## Principles

| # | principle | rule |
|---|---|---|
| 1 | Specification first | ECMA-262 states the language. swell follows it by its section numbers and terms. |
| 2 | Complete means complete | A part of the standard is marked complete only when every test262 case for it passes and nothing of it is left out. Counts are measured per directory: cases that exist, run and pass. |
| 3 | No third-party code in what ships | The Rust standard library and the operating system only. |
| 4 | No unsafe code | Every crate forbids it. No just-in-time compiler, no executable memory. |
| 5 | No ambient authority | Everything from outside comes through the host interface. |
| 6 | Bounded | Steps, memory, stack depth and regular expression matching have limits. |
| 7 | Small | Nothing is added before a planned step needs it. |

## How it is built

Three texts say what swell must do. Each is held by something that runs.

| text | where it is | what holds it |
|---|---|---|
| The language | ECMA-262 at a pinned commit | test262 at a pinned commit. The gate counts the cases that exist, are excluded, run and pass, per directory and per delivery slice. |
| The host interface, the part a model can state | An [ESS](https://github.com/beyond10x/ess) model under [`spec/`](spec/) of agents, realms, scripts, modules and jobs, their commands and outcomes | The model validates. Its 58 scenarios pass on ESS's interpreter. 14 of them run against the engine itself. The other 44 state the later slice or the reason they wait for. ESS states about half of the interface: the model has 51 places marked as not expressible. |
| The host interface, the rest | Numbered requirements in `docs/design/host-interface.md`: values, hooks that answer the engine, host objects, budgets, determinism | Each is tied to a test or to a written omission, and the gate checks that. |

The specification comes first and the code second. The engine is written by hand against it,
by agents, in planned and reviewed steps.

## Licence

Apache-2.0, as surf: [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE). If you use swell, say so and
name beyond10x.

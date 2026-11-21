# swell

**An ECMAScript (JavaScript) engine written from scratch in Rust.**

swell runs JavaScript: it parses scripts and modules, compiles them to bytecode and interprets
them. It is written to be embedded by [surf](https://github.com/beyond10x/surf), a browser built
from scratch, and it is a project of its own, usable by any program that runs JavaScript it does
not trust.

## State

swell runs scripts of the ES5 grammar, in both modes, under a step and a memory budget, and its
gate runs test262: 3,969 of the 44,812 cases it runs pass (8.9 %), measured on 2026-11-21. The
first of nineteen delivery slices is delivered: the test262 harness itself, 31 cases, all
passing.

- The design, [`docs/design/engine.md`](docs/design/engine.md): what swell targets, how it is
  built, how it is measured, and the order in which it is delivered.
- The host interface, specified before its code:
  [`docs/design/host-interface.md`](docs/design/host-interface.md) with 164 numbered
  requirements, and an ESS model under [`spec/`](spec/) whose 58 scenarios pass on ESS's
  interpreter; the gate runs them against the engine too, and 14 pass there.
- The gate, `task check`: test262 pinned by commit and checksum, the Unicode 17.0.0 data
  pinned by checksum, both specifications checked, and 31 planted defects that the gate must
  refuse. Measured on 2026-11-21: it passes, here and on the forge, and 70 of 164 host
  requirements have a test.
- The test262 runner, `swell-test262`: it reads all 53,801 cases, excludes 8,982 by recorded
  decision, runs the rest in both modes under budgets, and counts them per directory, per
  delivery slice and per section of ECMA-262 (35). Its 283 lines are compared with a committed
  file on every run. A construct the parser does not have yet is refused, and a refusal never
  counts as a pass: 53,564 of the 85,556 runs end at such a refusal.
- The first parts of the engine: a parser for the ES5 grammar with strict mode
  (`crates/syntax`), a compiler to bytecode (`crates/compiler`), the heap, the values, objects,
  realms, the interpreter, the builtins the test262 harness calls and the host entry points
  that parse and evaluate a script
  (`crates/swell`), number parsing and printing (`crates/num`).

Releases are listed in [`CHANGELOG.md`](CHANGELOG.md), each with what works and its limits;
[`docs/releasing.md`](docs/releasing.md) says how one is cut.

Next: the rest of the ES5 language (`eval`, `with`, `arguments`, labels, semicolon insertion, the
global `Function`): the second of nineteen slices, 5,245 cases, of which 2,125 pass today.

## What swell is

- An interpreter for the language of ECMA-262, the ECMAScript standard, held to test262, the
  standard's own test suite.
- A library with a narrow host interface. The program that embeds it supplies the clock, the time
  zone, random numbers, module loading and the scheduling of jobs. swell has no other way to reach
  files, the network, time or randomness, so a run repeats exactly when the host's answers do.
- Built for hostile input. A script cannot hang, crash or exhaust the program that runs it: every
  run has a step budget, a memory budget and depth limits, and each one ends the run with an
  outcome the host can read.

## What swell is not

- Not fast first. It has no just-in-time compiler and never writes machine code.
- Not a wrapper around V8, SpiderMonkey, JavaScriptCore or any other engine.
- Not complete, and it will say where. Nothing is marked complete yet.
- Not a browser. The DOM and everything else a web page sees beyond the language belong to the
  program that embeds swell.

## Principles

1. **Specification first.** ECMA-262 states the language; swell follows it by its section
   numbers and terms.
2. **Complete means complete.** A part of the standard is marked complete only when every test262
   case for it passes and nothing is deliberately left out of it. Counts are measured, not
   claimed: cases that exist, run and pass, per directory.
3. **No third-party code in what ships.** The Rust standard library and the operating system only.
4. **No unsafe code.** Every crate forbids it. No just-in-time compiler, no executable memory.
5. **No ambient authority.** Everything from outside comes through the host interface.
6. **Bounded.** Steps, memory, stack depth and regular expression matching all have limits.
7. **Small.** Nothing is added before a planned step needs it.

## How it is built

Three texts say what swell must do, and each is held by something that runs:

- **The language**: ECMA-262 at a pinned commit, held by test262 at a pinned commit. The gate
  counts the cases that exist, are excluded, run and pass, per directory and per delivery slice.
- **The host interface, the part a model can state**: an
  [ESS](https://github.com/beyond10x/ess) model under [`spec/`](spec/) of agents, realms,
  scripts, modules and jobs, their commands and outcomes. It validates, and its 58 scenarios
  pass on ESS's interpreter; 14 of them run against the engine itself, and the other 44 name
  the later slice or the reason they wait for. ESS states about half of the interface: the
  model has 51 places marked as not expressible.
- **The host interface, the rest**: numbered requirements in
  `docs/design/host-interface.md` (values, hooks that answer the engine, host objects, budgets,
  determinism), each tied to a test or to a written omission, and checked by the gate.

The specification comes first and the code second; the engine is written by hand against it,
by agents, in planned and reviewed steps.

## Licence

Apache-2.0, as surf: [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE). If you use swell, say so and
name beyond10x.

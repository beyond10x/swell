# Changelog

What changed in each release of swell. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The history starts at the first
release; what came before it is in the git log and in `docs/plan/`.

A release of swell is a version of its source: swell is a library, and no binary is built yet.
Every number below comes from the gate, `task check`, on the released commit.

## [Unreleased]

### Added

- The language of the second delivery slice, first part: automatic semicolon insertion,
  labelled statements, `??`, `?.`, logical assignment, optional catch binding, trailing commas
  in argument lists, numeric separators, a hashbang line, plain method definitions and
  shorthand properties in object literals.
- `Number` (with `toString` for every radix), `Boolean`, `isNaN` and `isFinite`, and the
  `toString` and `valueOf` of the String wrapper.
- The conformance runner writes one line per run with what ended it, and its host has
  `$262.evalScript`.
- The `with` statement and the `arguments` object, mapped and unmapped. A name inside the body
  of a `with` is looked up at run time, each record visited charged as one step; every other
  name keeps the slot the compiler gave it.
- test262 after these: `test262 total 53801 8982 7 44812 6262 38550 0`, 6,262 of the 44,812
  cases that run pass (14.0 %; 3,969 in 0.1.0). The second slice passes 3,743 of its 5,245
  cases. 44,212 of the 85,556 runs end at a refusal of a construct the parser does not have
  yet (53,564 in 0.1.0).
- The gate runs the scenarios of the host interface model against the engine itself
  (`conformance/ess`, the task `conform-engine`): `58 scenarios: 14 passed, 0 failed, 0 error,
  44 unsupported`. An unsupported scenario names the later slice or the reason it waits for,
  and a planted defect (an interrupt that does nothing) must fail a scenario.
- The model states whose realm it is: a script is parsed in a realm of the agent that is
  called, and a realm of another agent is refused as `ForeignHandle`.
- The first delivery slice is delivered: no requirement of the host interface is pending for
  it, and the gate fails if one ever is again.

### Changed

- `Limits::min_stack_bytes()` asks for 14 KiB per re-entry instead of 30 KiB. In 0.1.0 a debug
  build with full debug information could overflow the promised stack: one re-entry took up
  to 31,314 bytes. The run loop no longer holds the locals of every instruction in its frame:
  one re-entry takes up to 11,100 bytes in a debug build (through `toJSON`), and an agent
  with `reentry_depth` 1,000 and `parser_nesting` 1,000 needs a thread of 14,672,640 bytes
  (31,073,024 in 0.1.0).

- The design gives the global `Function`, plain method definitions and shorthand properties,
  `Number`, `Boolean`, `isNaN` and `isFinite` to the second delivery slice. The host interface
  gains three requirements (`HOST-162` to `HOST-164`), among them `Agent::refusals()`, a count
  of the refusals the engine raised; none is implemented yet
  (`host-requirements: 164 requirement(s), 164 line(s): 70 test, 94 pending`).

## [0.1.0] - 2026-11-21

The first release. swell is an ECMAScript engine written from scratch in Rust, with no
third-party crate and no unsafe code. This release runs scripts of the ES5 level of the
language; it is a small part of the standard, and the limits below say how small.

### Added

- The library crate `swell` (`crates/swell`): a host creates an agent with limits and seeds
  (`Agent::new`), creates a realm (`create_realm`), parses a source (`parse_script`) and
  evaluates it (`evaluate_script`), calls and constructs functions (`Agent::call`,
  `Agent::construct`), and works with values inside a scope (`with_cx`).
- The language of this release: the ES5 grammar in strict and non-strict mode, with the early
  errors of strict code. A construct of a later part of the language is refused by the parser
  with a `SyntaxError` whose message names the delivery slice that brings it, for example
  `class is not supported before slice 8`.
- Bounded execution. A run has a step budget and a memory budget, and an interrupt handle ends
  it from another thread: the run ends with an outcome the host reads
  (`Terminated(StepBudget)`, `Terminated(MemoryBudget)`, `Terminated(Interrupted)`), and no
  `catch` or `finally` block of the script runs after it. Frame depth, re-entry depth and
  string length are limited by a `RangeError`, parser nesting by a `SyntaxError`.
  `Limits::min_stack_bytes()` states the thread stack an agent needs for its limits.
- No ambient authority: the engine reads no file, clock, random source or environment
  variable. What a script may observe of the outside comes through the host's hooks, and two
  runs with the same seeds and the same hook answers are the same.
- Misuse of the interface has an answer: a stale, released or foreign handle gets
  `Err(StaleHandle)` or `Err(ForeignHandle)`, and a broken invariant of the engine ends the
  run with `Terminated(EngineFault)` and leaves the agent faulted. A seeded generator of call
  sequences holds this in the gate.
- The builtins that the test262 harness files call: members of `Object`, `Function.prototype`,
  `Array`, `String`, `Math` and `JSON` (`JSON.stringify` among them), the Error constructors,
  and bound functions.
- The supporting crates: `swell-syntax` (lexer and parser), `swell-compiler` (bytecode),
  `swell-num` (number parsing and printing, the keyed hash) and `swell-unicode`.
- The conformance runner `swell-test262`: it runs every test262 case that is not excluded, as
  written and in strict mode, each run in a fresh agent and realm under a step and a memory
  budget, with `print` and `$262` from its host.
- The gate, `task check`, which pins test262 by commit and checksum and holds the counts below.

### Limits of this release

- Not in the language yet: `eval`, `with`, the `arguments` object, labels, automatic semicolon
  insertion (a statement needs its semicolon), regular expression literals, and everything
  after ES5 (`let` and `const`, arrow functions, classes, generators, promises, modules, and
  the rest). The global `Function` and most methods of the builtins are missing.
  [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the nineteen delivery
  slices and what each brings.
- test262: 3,969 of the 44,812 cases the gate runs pass, 8.9 %. Its line reads
  `test262 total 53801 8982 7 44812 3969 40843 0`: 53,801 cases exist, 8,982 are excluded by
  recorded decision (`Temporal`, `Intl` and proposals at stage 3), 7 do not apply, 44,812
  run, 3,969 pass, 40,843 fail, none ends at a budget. The first delivery slice, the test262
  harness itself, passes its 31 cases in all 62 runs (`test262-slice 01 31 62 0 0 31 31 0 0`).
- What that number says: a construct the parser does not have yet is refused, and a refusal
  never counts as the `SyntaxError` a negative case asks for. 53,564 of the 85,556 runs end at
  such a refusal (`test262-refused 53564`). A case outside the first slice passes when it
  needs only the ES5 grammar and the builtins above; 269 negative cases pass on a generic
  syntax error, and 129 passing cases hold no assertion and pass by not throwing.
- The host interface: 70 of its 161 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit (`host-requirements: 161 requirement(s), 161 line(s): 70 test, 91 pending`);
  none is pending for the first slice. The interface will change before 1.0.
- The engine's own tests: `55 test suites: 268 passed, 0 failed, 1 ignored`, the same tests of
  the engine crate in the mode that collects garbage at every safe point (`test-collect: 31
  test suites: 194 passed, 0 failed, 1 ignored`), and the test of an injected engine fault
  (`test-faults: 1 test suites: 1 passed, 0 failed, 0 ignored`).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.1.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

[Unreleased]: https://github.com/beyond10x/swell/compare/0.1.0...HEAD
[0.1.0]: https://github.com/beyond10x/swell/releases/tag/0.1.0

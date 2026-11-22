# Changelog

What changed in each release of swell. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The history starts at the first
release; what came before it is in the git log and in `docs/plan/`.

A release of swell is a version of its source: swell is a library, and no binary is built yet.
Every number below comes from the gate, `task check`, on the released commit.

## [Unreleased]

### Changed

- The gate now fails when a public item of the host interface or a `HOST-<n>` requirement
  changed since the last release and the changelog does not name it under Host interface, in
  the name form of `docs/releasing.md`.

### Host interface

- Breaking: `get` takes `&mut self`. The value read is handed to the running scope and stays
  valid until the scope ends, also when the host drops the root and calls back into the
  engine. A caller that read through a shared `&Cx` or `&HookCx` takes `&mut` instead
  (`HOST-170`).
- `get_prototype_of`, `set_prototype_of`: new methods of `Cx`, the [[GetPrototypeOf]] and
  [[SetPrototypeOf]] of an object, so an interface object takes its parent's interface object
  as [[Prototype]] (`HOST-180`).
- `HOST-165`, `HOST-166`, `HOST-167`, `HOST-168`, `HOST-169`, `HOST-170`, `HOST-171`, `HOST-172`, `HOST-173`, `HOST-174`, `HOST-175`, `HOST-176`, `HOST-177`, `HOST-178`, `HOST-179`, `HOST-180`, `HOST-181`, `HOST-182`, `HOST-183`, `HOST-184`, `HOST-185`, `HOST-186`, `HOST-187`: new requirements.
- `HOST-29`, `HOST-52`, `HOST-102`: changed requirements.

## [0.2.0] - 2026-11-21

The second release. swell now runs the whole ES5 level of the language: a script may leave out
its semicolons, label its loops, and use `with`, `arguments`, `eval` and the global
`Function`. The first two of nineteen delivery slices are delivered, and every test262 case
of both passes. Most of the standard library and everything after ES5 is still missing; the
limits below say how much.

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
- `eval`, direct and indirect, and the global `Function`. A non-strict direct `eval` can add
  bindings to the scope of its caller.
- The host is asked before a string is compiled (`host_ensure_can_compile_strings`) and may
  give the code for an object passed to `eval` (`host_get_code_for_eval`).
- `Agent::refusals()`: a count of the refusals the engine raised, at parse or run time, caught
  by the script or not. The conformance runner fails a run in which it moved.
- `Array.prototype.toString`, before the rest of `Array`: `[] + ""` is `""`.
- The second delivery slice is delivered: all of its 4,259 test262 cases pass in all 7,543
  runs. 1,008 cases that need a later slice moved there, each with its evidence in
  `conformance/test262/moves.tsv`, which the gate checks (`test262-moves-check`).
- test262 after these: `test262 total 53801 8982 7 44812 7742 37070 0`, 7,742 of the 44,812
  cases that run pass (17.3 %; 3,969 in 0.1.0).
  46,493 of the 85,556 runs meet a refusal of a construct the parser does not have
  yet (53,564 ended at one in 0.1.0, where a refusal a script caught was not counted).
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
- The completion value of a `try` statement whose `finally` block is left by `break` or
  `continue` is `undefined`, as the standard says; it was the value of the `try` or `catch`
  block. A script sees it through `eval` and as the result of a script.
- The design gives the global `Function`, plain method definitions and shorthand properties,
  `Number`, `Boolean`, `isNaN` and `isFinite` to the second delivery slice. The host interface
  gains three requirements (`HOST-162` to `HOST-164`), among them `Agent::refusals()`, a count
  of the refusals the engine raised. 80 of the 164 requirements have a test
  (`host-requirements: 164 requirement(s), 164 line(s): 80 test, 84 pending`; 70 in 0.1.0).

### Limits of this release

- Not in the language yet: regular expression literals, identifiers outside ASCII, and
  everything after ES5 (`let` and `const`, arrow functions, classes, generators, promises,
  modules, and the rest). A script that uses one is refused with a `SyntaxError` that names
  the delivery slice that brings it.
  [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the nineteen delivery
  slices and what each brings.
- Most methods of the builtins are missing. Of `Array`, for example, `isArray`, `join`, `map`,
  `push` and `toString` exist; `Function.prototype.apply`, `JSON.parse`, `parseInt`, `Date`,
  `RegExp` and all of `Math` but `max` do not. The third delivery slice brings the core
  library.
- test262: 7,742 of the 44,812 cases the gate runs pass, 17.3 %. Its line reads
  `test262 total 53801 8982 7 44812 7742 37070 0`: 53,801 cases exist, 8,982 are excluded by
  recorded decision (`Temporal`, `Intl` and proposals at stage 3), 7 do not apply, 44,812
  run, 7,742 pass, 37,070 fail, none ends at a budget. The first two delivery slices pass all
  their cases in all their runs (`test262-slice 01 31 62 0 0 31 31 0 0`,
  `test262-slice 02 4259 7543 0 0 4259 4259 0 0`).
- What that number says: the second slice counted 5,267 cases before its close; 1,008 of them
  need a construct or a builtin of a later slice and are counted there now
  (`test262-moves: 1032 case(s) moved`, 24 of them cases of the third slice that need `**`).
  A refusal never counts as a pass, also when the script catches it: `test262-refused 46493`
  of the 85,556 runs meet one.
- A property is found by a scan of its object's property list, and the comparisons of that
  scan are not charged to the step budget: a script with very large objects runs longer than
  its step count says.
- The host interface: 80 of its 164 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit (`host-requirements: 164 requirement(s), 164 line(s): 80 test, 84 pending`);
  none is pending for the first two slices. 14 of the 58 scenarios of the interface's model
  pass on the engine, and the other 44 name what they wait for. The interface will change
  before 1.0.
- The engine's own tests: `62 test suites: 341 passed, 0 failed, 1 ignored`, the same tests of
  the engine crate in the mode that collects garbage at every safe point (`test-collect: 38
  test suites: 238 passed, 0 failed, 1 ignored`), and the test of an injected engine fault
  (`test-faults: 1 test suites: 1 passed, 0 failed, 0 ignored`).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.2.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

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

[Unreleased]: https://github.com/beyond10x/swell/compare/0.2.0...HEAD
[0.2.0]: https://github.com/beyond10x/swell/releases/tag/0.2.0
[0.1.0]: https://github.com/beyond10x/swell/releases/tag/0.1.0

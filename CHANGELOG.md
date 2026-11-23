# Changelog

What changed in each release of swell. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The history starts at the first
release; what came before it is in the git log and in `docs/plan/`.

A release of swell is a version of its source: swell is a library, and no binary is built yet.
Every number below comes from the gate, `task check`, on the released commit.

## [Unreleased]

## [0.6.0] - 2026-11-23

Regular expressions run: the literal, `RegExp`, `exec` and `test`, on the new crate
`swell-regexp`, whose matcher spends the step and memory budgets of its entry. The case
methods, `normalize` and `localeCompare` of `String` take every String. The fifth delivery
slice is in work, with seven of its fifteen stories delivered.

### Added

- Regular expressions: the literal, evaluated to a new object each time, and `RegExp` with
  `exec`, `test`, `toString`, `lastIndex` and the accessors `flags`, `source`, `global`,
  `hasIndices`, `ignoreCase`, `multiline`, `dotAll`, `sticky`, `unicode` and `unicodeSets`;
  named groups, lookbehind, match indices, the flags `d`, `g`, `m`, `s`, `u` and `y` and the
  modifiers `m` and `s`. The new crate `swell-regexp` parses a pattern, compiles it and
  matches it by backtracking with an explicit stack; a match spends the steps and the memory
  of its entry, so `/(a+)+$/` against a long run of `a` ends at the step budget. Not yet
  built, each a counted `SyntaxError` (`HOST-191`): the flags `i` and `v`, a property escape
  and a group name outside ASCII; `replace`, `replaceAll` and `split` given a regular
  expression; `match`, `matchAll` and `search` are missing. 17,775 of the 44,812 selected
  test262 cases pass (16,412 in 0.5.0), 1,327 of the 2,724 of the fifth slice (52).
- `String.prototype.replace`, `replaceAll` and `split` with a String to search for or to split
  at: the replacement patterns of GetSubstitution, a function as the replacement, and a method
  an object keys by `@@replace` or `@@split`.
- The pattern of a regular expression literal is checked when its script is parsed, by the
  grammar of the modes `u` and `v` and of Annex B.1.2 with neither: a pattern with an error
  is a `SyntaxError` of the script at the line and column of the literal, and a pattern
  nested past `parser_nesting` is a `SyntaxError` that names the limit (`HOST-142`).
- `toLowerCase`, `toUpperCase`, `toLocaleLowerCase` and `toLocaleUpperCase` map every String:
  the unconditional mappings of `SpecialCasing.txt`, the simple mappings of `UnicodeData.txt`
  and Final_Sigma, from five tables generated from the vendored Unicode 17.0.0 data. A case
  method charges a step per code unit read and per code unit written.
- `String.prototype.normalize` with the forms NFC, NFD, NFKC and NFKD of Unicode 17.0.0, and
  `localeCompare` for every String: it orders two Strings by the code units of their NFC
  forms and answers `+0` for canonically equivalent Strings. `localeCompare` costs three
  steps per code unit, not one. No method of `String.prototype` refuses a String outside
  ASCII any more.

### Changed

- The test steps of the gate cost about 270 CPU seconds, from about 300: tests of twelve files take a
  gate count, with the full count in the lane `slow`, five tests of the test262 runner that
  counted one fixture tree are one test, and `gate-selftest` starts the next planted script
  when one ends.

### Host interface

- `BINDINGS_REALM_HEAP_BYTES` (changed): 506,112, from 381,696. A realm of 300 host functions and
  40 host objects raises `heap_bytes()` by 449,848 since `String.prototype` has `replace`,
  `replaceAll` and `split`: an arena grows once more inside the window `HOST-187` measures. A
  host that sizes its memory budget by this ceiling raises it.
- `HOST-191` (new): a refusal that `RegExp` raises at run time for a part of the fifth slice
  that is not built yet is counted by the agent, like a refusal of the parser.

### Limits of this release

- Not in the language yet: in a regular expression the flags `i` and `v`, a property escape
  and a group name outside ASCII, each refused with a `SyntaxError` the agent counts
  (`HOST-191`); identifiers outside ASCII; the operator `**`; and of what came after ES5
  everything but the sixth slice: template literals with and without a tag, `let` and
  `const`, arrow functions, default and rest parameters, object and array destructuring,
  spread, computed property names, `for`-`of` and function declarations in blocks (without
  the web-compatibility semantics of Annex B.3.2). Classes, generators, promises, async
  functions and modules are refused with a `SyntaxError` that names the delivery slice that
  brings them. [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the
  nineteen delivery slices, what each brings and the order they are delivered in.
- The builtins end at the core library, what the sixth slice adds to it, and of the fifth
  slice `RegExp` with `exec` and `test`. `String.prototype.match`, `matchAll` and `search` do
  not exist, and `replace`, `replaceAll` and `split` given a regular expression end in a
  counted refusal. `Map`, `Set`, `Promise`, `Reflect`, `Proxy`, `Date`, `parseInt`,
  `parseFloat` and all of `Math` but `max` do not exist.
- test262: 17,775 of the 44,812 cases the gate's digest stands for pass, 39.7 %. The line
  reads `test262 total 53801 8982 7 44812 17775 27036 1`: 53,801 cases exist, 8,982 are
  excluded by recorded decision, 7 do not apply, 44,812 run, 17,775 pass, 27,036 fail, one
  ends at the memory budget. The first, second and sixth delivery slices pass all their
  cases in all their runs, and the third all but one
  (`test262-slice 01 31 62 0 0 31 31 0 0`, `test262-slice 02 4259 7543 0 0 4259 4259 0 0`,
  `test262-slice 03 7011 13829 0 0 7011 7010 1 0`,
  `test262-slice 06 3898 7351 0 0 3898 3898 0 0`). Of the fifth, which is in work, 1,327 of
  2,724 cases pass (`test262-slice 05 2724 5441 0 0 2724 1327 1397 0`). A refusal never
  counts as a pass, also when the script catches it.
- The host interface: 113 of its 191 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit; none is pending for the four delivered slices or for the fifth. 14 of the 58
  scenarios of the interface's model pass on the engine, and the other 44 name what they
  wait for. The interface will change before 1.0.
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.6.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

## [0.5.0] - 2026-11-23

The sixth delivery slice is delivered: `Symbol`, the iterator protocol, `for`-`of`, array
patterns, spread, computed property names and tagged templates run, and every test262 case
the slice counts passes.

### Added

- The rest of the sixth delivery slice. The `Symbol` builtin: the constructor, `Symbol.for` and
  `Symbol.keyFor`, `Symbol.prototype`, twelve well-known symbols and
  `Object.getOwnPropertySymbols`. Computed property names, object spread and object rest. The
  iterator protocol with the Array and String iterators: `Array.prototype.entries`, `keys`
  and `values`, `@@iterator` of an Array, a String and an arguments object. Function
  declarations in blocks and in `switch` cases, without the web-compatibility semantics of
  Annex B.3.2. The `for`-`of` statement, which calls the iterator's `return` once on `break`,
  on a `continue` to an outer loop, on `return` and on a throw from the body. `Array.from`,
  `Object.fromEntries` and `Object.groupBy`. Array patterns as bindings and as assignment
  targets, with elisions, defaults, nesting and a rest element, in declarations, parameters,
  `catch`, assignments and the heads of `for`, `for`-`in` and `for`-`of`; a pattern closes its
  iterator when it ends before the iterator does. Spread in array literals and in the
  arguments of a call, an optional call and `new`. Tagged templates: a site answers the same
  frozen template object at every evaluation, with its `raw` strings, and the object is
  freed with the code that holds the site.
- The sixth delivery slice, `bindings-and-iteration`, is delivered: every case it counts
  passes in every run (`test262-slice 06 3898 7351 0 0 3898 3898 0 0`; 1,364 of 4,436 in
  0.4.0). 143 cases that need a later slice moved there, each with its evidence in
  `conformance/test262/moves.tsv`.
- test262 after these: `test262 total 53801 8982 7 44812 16412 28399 1`, 16,412 of the 44,812
  cases that run pass (36.6 %; 13,575 in 0.4.0). `test262-refused 36178` of the 85,556 runs
  meet a refusal (41,572 in 0.4.0).
- The host interface: 190 numbered requirements, 110 of them with a test
  (`host-requirements: 190 requirement(s), 190 line(s): 110 test, 80 pending`; 108 in 0.4.0).
  None is pending for a delivered slice.

### Changed

- A test262 case is counted in the slice delivered last among the slices it needs, by the
  order of delivery the design's block names (1, 2, 3, 6, 5, 7, 10, 4, 9, 8, 11 to 21), and
  no longer in the slice with the highest number. The slices are not delivered in the order
  of their numbers, so a `RegExp` case that uses `Symbol` can first pass after slice 5, not
  after slice 6. Slice 4 counts 1,598 cases, slice 5 2,724, slice 6 3,898, slice 8 9,144 and
  slice 10 3,879 (1,516, 2,394, 4,436, 6,292 and 6,651 in 0.4.0, where the moves of the
  close also count). No run changed its outcome through the rule.
- The heaviest tests are lighter, so that none is near the 60 s limit of a test when the
  machine is loaded: no test of the gate takes over 1.5 CPU seconds alone in the default mode,
  and none of the lane `slow` over 2.5 at its full count, but for one test of determinism
  that runs its corpus three times. The fuzz test of the host interface runs 400 sequences
  at its full count (2,000 in 0.4.0); the gate's count is unchanged. The full count of the
  parser's stack-bound check is 7,500 levels on three quarters of the stack of 10,000
  levels. Only test code changed.
- One case of slice 18 that passed in 0.4.0 no longer passes:
  `test/annexB/built-ins/Array/from/iterator-method-emulates-undefined.js`. It passed only on
  the error of the missing `Array.from`; the method exists now, and the case needs
  `$262.IsHTMLDDA`, which slice 18 brings.

### Host interface

- `HostIntrinsic::ArrayPrototypeValues`, `HostIntrinsic::ArrayPrototypeEntries`,
  `HostIntrinsic::ArrayPrototypeKeys`, `HostIntrinsic::ArrayPrototypeForEach` and
  `WellKnownSymbol::Iterator` (new): what Web IDL gives an iterable interface (`HOST-185`).

### Limits of this release

- Not in the language yet: regular expression literals, identifiers outside ASCII, the
  operator `**`, and of what came after ES5 everything but the sixth slice: template
  literals with and without a tag, `let` and `const`, arrow functions, default and rest
  parameters, object and array destructuring, spread, computed property names, `for`-`of`
  and function declarations in blocks (without the web-compatibility semantics of Annex
  B.3.2). Classes, generators, promises, async functions and modules are refused with a
  `SyntaxError` that names the delivery slice that brings them.
  [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the nineteen delivery
  slices, what each brings and the order they are delivered in.
- The builtins end at the core library and what the sixth slice adds to it: `Symbol`, the
  Array and String iterators, `Array.from`, `Object.fromEntries` and `Object.groupBy`. `Map`,
  `Set`, `Promise`, `Reflect`, `Proxy`, `Date`, `RegExp`, `parseInt`, `parseFloat` and all of
  `Math` but `max` do not exist. A String method that needs the Unicode tables of a later
  slice refuses a code unit outside ASCII (`HOST-165`).
- test262: 16,412 of the 44,812 cases the gate's digest stands for pass, 36.6 %. The line
  reads `test262 total 53801 8982 7 44812 16412 28399 1`: 53,801 cases exist, 8,982 are
  excluded by recorded decision, 7 do not apply, 44,812 run, 16,412 pass, 28,399 fail, one
  ends at the memory budget. The first, second and sixth delivery slices pass all their
  cases in all their runs, and the third all but one
  (`test262-slice 01 31 62 0 0 31 31 0 0`, `test262-slice 02 4259 7543 0 0 4259 4259 0 0`,
  `test262-slice 03 7011 13829 0 0 7011 7010 1 0`,
  `test262-slice 06 3898 7351 0 0 3898 3898 0 0`). A refusal never counts as a pass, also
  when the script catches it.
- The host interface: 110 of its 190 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit; none is pending for the four delivered slices. 14 of the 58 scenarios of the
  interface's model pass on the engine, and the other 44 name what they wait for. The
  interface will change before 1.0.
- The engine's own tests: `15 test binaries: 977 passed, 0 failed, 1 ignored`, the tests of
  the engine crate in the mode that collects garbage at every safe point
  (`test-collect: 2 test binaries: 796 passed, 0 failed, 1 ignored`), and the test of an
  injected engine fault (`test-faults: 1 test binaries: 1 passed, 0 failed, 629 ignored`; the
  629 are the tests that step's filter leaves out).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.5.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

## [0.4.0] - 2026-11-22

Storage is charged before it is allocated and work before it is done, the host interface gains
what an embedder that generates bindings needs, and the first five parts of the sixth
delivery slice run.

### Added

- The first parts of the sixth delivery slice: template literals without a tag, and `let`,
  `const` and block scopes with the temporal dead zone and a fresh record per iteration of a
  `for (let ...)` loop. A tagged template is refused by name until its story. Arrow functions
  with an expression body or a block body: `this` and `arguments` are those of the code that
  made the arrow, and an arrow has no [[Construct]] and no `prototype`. Default parameters and
  rest parameters, in functions, arrows and the `Function` constructor: a default is evaluated
  only for `undefined`, and a parameter read or written before its initializer has run throws
  a `ReferenceError` that names it. Object destructuring in declarations, parameters,
  assignments, `for` heads and `catch`, with shorthand names, renamed targets, nested object
  patterns and defaults; a rest property, a computed name and array patterns are refused by
  name until their stories.
- test262 after these: `test262 total 53801 8982 7 44812 13575 31236 1`, 13,575 of the 44,812
  cases that run pass (30.2 %; 12,120 in 0.3.0). The sixth slice passes 1,364 of its 4,436
  cases (`test262-slice 06 4436 8413 0 0 4436 1364 3072 0`; 100 in 0.3.0).
  `test262-refused 41572` of the 85,556 runs meet a refusal. One case ends at the memory
  budget, `test/staging/sm/regress/regress-610026.js`: its source of about 4.19 million empty
  blocks has a tree that the 512 MiB of the runner do not hold, now that the tree is charged.
- The host interface: 190 numbered requirements, 108 of them with a test
  (`host-requirements: 190 requirement(s), 190 line(s): 108 test, 82 pending`).

### Changed

- The ESS pins moved from 0.96.0 to 0.97.4 (`spec/ess-inputs.yaml`, the conformance workspace,
  the forge's workflow). The suite is byte-identical: 58 scenarios, 41 refusals, 58 passing on
  the interpreter and 14 against the engine, as before.
- The gate now fails when a public item of the host interface or a `HOST-<n>` requirement
  changed since the last release and the changelog does not name it under Host interface, in
  the name form of `docs/releasing.md`.
- Storage is charged before it is allocated and work before it is done
  (`docs/design/engine.md`, "Memory budget" and "The step budget"). What that changed:
  - The parser's tree, the scope analysis, the compile step's scratch and the code are charged
    to `memory_bytes` before they grow and credited when they drop, and the code's charge
    passes to its script record. Every parse reads the interrupt at least once per 4,096 code
    units or nodes; `eval` and `Function` read it before their copy of the source, and `eval`
    charges a step per code unit of its source.
  - The key list of an object is charged, steps and bytes, before the list is built, and
    credited when its holder drops it. Index keys added out of order are sorted in charged
    passes; a list whose index keys are in order costs the steps it did.
  - The key list a `for`-`in` loop holds is charged to `memory_bytes` before it grows and
    while the loop runs, and credited when the loop ends, by `break`, `return`, a throw or a
    termination included.
  - Setting an array's length, compacting a property or binding list, and calling a bound
    function charge their steps before the work; the totals are as before. The list of keys
    that a shorter length deletes is charged before it is made, and the arguments of a bound
    function are not copied before the gathered list fits the budget.
  - Comparing two Strings, and using a String of more than 32 code units as a property key,
    charge a step per code unit past 32 before the work; shorter ones cost what they did.
  - Builtins no longer copy a whole String they do not use and charge a String's steps before
    they copy it. `JSON.stringify` compares the Strings of a replacer list in place, a step
    per code unit past 32. `JSON.parse` and `JSON.rawJSON` charge the step per code unit of
    their text before they copy it: a text parsed to its end costs what it did, and one that
    fails early has paid for its whole text.
  - The parameter map of a mapped arguments object and the arguments a call records are
    charged to the memory budget before they are made; a call no longer copies its
    function's parameter list.
  - A bound function's arguments, the argument list of a bound call and the arguments a
    builtin call records are charged to the memory budget before they are copied and stay
    charged while they are held.
  - The atom table grows charged first, also when deleted buckets would make it grow.
  - The compile step charges what a node's expansion holds as it is made: the script `1`
    needs 2,228 bytes of room for its parse and compile.
- A termination that ends the deletions of an Array length write leaves the Array's old
  length, so its length is never below an index it still has.
- A pending interrupt ends `parse_script` before the source is copied: it answers
  `Parsed::Terminated(Interrupted)` and consumes the flag (`HOST-166`, `HOST-167`).
- Each crate's integration tests build into one test binary, `tests/main.rs`, generated from
  the test files and checked by the gate: 72 test executables became 15.
- The dev and test builds carry no debug information, and the gate step
  `test-binaries-check` fails when a test executable has a `.debug_` section. The stack
  requirements are also tested in the release profile and in a profile without debug
  information, in the lane `slow`.

### Fixed

- Nested arrow functions keep the parser within `ParseOptions::stack_bytes` (229.3 bytes a
  level, now 181.3; at `parser_nesting` 10,000 the stated stack overflowed), and the gate's
  check of the stack bound runs on a quarter of the stack of 10,000 levels, so it fails wherever
  the full count does.

### Host interface

- Four entries below break a host written for 0.3.0, each marked "Breaking": `get`,
  `HostObjectSpec::error_kind`, the reading of `heap_bytes`, and `HOST-70`.
- Breaking: `get` takes `&mut self`. The value read is handed to the running scope and stays
  valid until the scope ends, also when the host drops the root and calls back into the
  engine. A caller that read through a shared `&Cx` or `&HookCx` takes `&mut` instead
  (`HOST-170`).
- `live_bytes` (new, on `Agent`): what the live records hold, slots included. It falls by
  everything a sweep frees and sets the collection threshold (`HOST-171`).
- Breaking: `heap_bytes` keeps its signature and changes its reading, with `memory_bytes`: an arena's
  slots are counted by capacity, charged when the arena grows and kept while the agent lives,
  together with the collector's workspace and the atom table's buckets. A collection lowers
  `heap_bytes` only by what the freed records owned. Before, a slot was charged with each
  record and credited by the sweep that freed it, and the collector's workspace was not
  charged. A String with no code units that lands in a spare slot is no longer refused by a
  budget with no room left.
- `get_prototype_of`, `set_prototype_of`: new methods of `Cx`, the [[GetPrototypeOf]] and
  [[SetPrototypeOf]] of an object, so an interface object takes its parent's interface object
  as [[Prototype]] (`HOST-180`).
- `charge_host_bytes` (new, on `Cx` and `HookCx`): charges bytes the host holds for the agent
  to `memory_bytes` and answers a `HostBytes` (`HOST-175`, `HOST-176`).
- `HostBytes` (new): bytes the host charged for the agent; neither `Clone` nor `Copy`; dropping
  it gives them back, in a collection or at the next charge.
- `resize_host_bytes` (new, on `Cx` and `HookCx`): changes that charge in place; growing
  charges the difference first, shrinking gives it back at once.
- `credit_host_bytes` (new, on `Agent`): gives one charge back at once and answers its bytes
  (`HOST-176`).
- `charge_steps` (new, on `Cx`): charges the steps of a host function's own work to the
  running entry (`HOST-177`).
- `intrinsic` and `well_known_symbol` (new, on `Cx` and `HookCx`), `HostIntrinsic` (new, with
  `HostIntrinsic::ObjectPrototype`, `HostIntrinsic::FunctionPrototype`,
  `HostIntrinsic::ErrorPrototype`) and `WellKnownSymbol` (new, with
  `WellKnownSymbol::ToStringTag`, `WellKnownSymbol::Unscopables`): the host names these
  intrinsics and symbols from a closed list (`HOST-183`, `HOST-184`).
- A host function charges the box of its data and of its steps, and four constants state
  what a host that generates bindings can count on (`HOST-186`, `HOST-187`), each the
  measured rise plus an eighth: `HOST_FUNCTION_LIVE_BYTES` (576, a host function with a name
  of at most 32 code units, its data apart), `HOST_OBJECT_LIVE_BYTES` (256, a host object,
  its data apart), `PROPERTY_LIVE_BYTES` (448, one definition of a new property, averaged
  over 300 on one object) and `BINDINGS_REALM_HEAP_BYTES` (381,696, a realm of 300 host
  functions and 40 host objects with 600 definitions, arena growth included). A release
  that raises one names it here.
- Breaking: `HostObjectSpec::error_kind` (new field; a literal adds `error_kind: None`) gives
  a host object an [[ErrorData]] slot, as Web IDL gives a `DOMException`: `Error.isError`
  answers true for it (`HOST-179`).
- `error_summary` (new, on `Cx` and `HookCx`), `Agent::error_summary` (new) and `ErrorSummary`
  (new), with `ErrorSummary::kind` and `ErrorSummary::message`: the host reads whether a value
  is an error object, the kind of the constructor that made it and its own `message`, without
  running code (`HOST-179`).
- `last_entry_error_site` (new, on `Agent`): after an entry that answered `Parsed::Errors` or
  `Throw`, where the error arose; after any other entry `None` (`HOST-172`, `HOST-173`,
  `HOST-174`).
- `ErrorSite` (new): what that read answers. `ErrorSite::line` and `ErrorSite::column` are the
  position of the first parse error or of the throwing instruction, counted from 1, a column
  in UTF-16 code units; `ErrorSite::dynamic` is true when the position is in eval code or in
  the source text of a `Function` constructor; `ErrorSite::host_defined` is the
  [[HostDefined]] of the script whose code raised it, `None` for `Function` code and indirect
  eval.
- Breaking: after `Terminated(MemoryBudget)` the agent stays `Alive`, and the next entry runs when the
  free room, `memory_bytes` less `heap_bytes()`, covers it; it ends in
  `Terminated(MemoryBudget)` again when it does not, so a host makes room first: it drops
  roots and calls `collect`, or gives bytes back with `credit_host_bytes`. `HOST-70` said
  before that the script `1` runs whatever room is left. `HOST-188`: the calls that take no
  memory answer with a room of 0. `HOST-189`: an entry the room does not cover ends the same
  way each time. `HOST-190`: with at least 3,072 bytes of room the script `1` answers
  `Normal(1)` as in a fresh agent.
- A handle the host keeps without a root is a stated weak reference (`HOST-178`): it reads
  its object while the object lives and answers `StaleHandle` after a collection freed it,
  also after its slot holds a new object. No code changed for it; it is tested now.
- `HOST-166`, `HOST-167`, `HOST-168`, `HOST-169`, `HOST-170`, `HOST-171`, `HOST-172`, `HOST-173`, `HOST-174`, `HOST-175`, `HOST-176`, `HOST-177`, `HOST-178`, `HOST-179`, `HOST-180`, `HOST-181`, `HOST-182`, `HOST-183`, `HOST-184`, `HOST-185`, `HOST-186`, `HOST-187`, `HOST-188`, `HOST-189`, `HOST-190`: new requirements.
- `HOST-29`, `HOST-52`, `HOST-70`, `HOST-102`: changed requirements.

### Limits of this release

- Not in the language yet: regular expression literals, identifiers outside ASCII, and of
  what came after ES5 everything but template literals without a tag, `let` and `const`,
  arrow functions, default and rest parameters and object destructuring. Array patterns,
  spread, `for`-`of`, computed property names, classes, generators, promises and modules are
  refused with a `SyntaxError` that names the delivery slice that brings them.
  [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the nineteen delivery
  slices and what each brings.
- The builtins end at the core library. `Symbol`, `Map`, `Set`, `Promise`, `Reflect`, `Proxy`,
  `Date`, `RegExp`, `parseInt`, `parseFloat` and all of `Math` but `max` do not exist. A
  String method that needs the Unicode tables of a later slice refuses a code unit outside
  ASCII (`HOST-165`).
- test262: 13,575 of the 44,812 cases the gate's digest stands for pass, 30.2 %. The line
  reads `test262 total 53801 8982 7 44812 13575 31236 1`: 53,801 cases exist, 8,982 are
  excluded by recorded decision, 7 do not apply, 44,812 run, 13,575 pass, 31,236 fail, one
  ends at the memory budget. The first two delivery slices pass all their cases in all their
  runs, the third all but one, and the sixth 1,364 of 4,436
  (`test262-slice 01 31 62 0 0 31 31 0 0`, `test262-slice 02 4259 7543 0 0 4259 4259 0 0`,
  `test262-slice 03 7011 13829 0 0 7011 7010 1 0`,
  `test262-slice 06 4436 8413 0 0 4436 1364 3072 0`). A refusal never counts as a pass, also
  when the script catches it.
- The host interface: 108 of its 190 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit; none is pending for the first three slices. 14 of the 58 scenarios of the
  interface's model pass on the engine, and the other 44 name what they wait for. The
  interface will change before 1.0.
- The engine's own tests: `15 test binaries: 858 passed, 0 failed, 1 ignored`, the tests of
  the engine crate in the mode that collects garbage at every safe point
  (`test-collect: 2 test binaries: 684 passed, 0 failed, 1 ignored`), and the test of an
  injected engine fault (`test-faults: 1 test binaries: 1 passed, 0 failed, 524 ignored`; the
  524 are the tests that step's filter leaves out).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.4.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

## [0.3.0] - 2026-11-22

The third delivery slice, the core library, is delivered: 7,010 of its 7,011 test262 cases
pass.

### Added

- The core library of the third delivery slice: the integrity, prototype, enumeration and copy
  methods of `Object`; `Function.prototype.apply` and the source text of a function;
  `globalThis`, the URI functions and `Error.isError`; `JSON.parse` with a reviver and raw
  JSON; the iteration, search, copy, mutation and sort methods of `Array`; the access, search,
  transform and compare methods of `String`. The records of the waves name each builtin
  (`docs/plan/slice-03-wave-1.md` to `-3.md`).
- An object with more than 8 properties finds a property through an index map, and an array
  keeps its elements dense. The scan of a property list that 0.2.0 named among its limits is
  gone.
- The third delivery slice is delivered with 7,010 of its 7,011 cases
  (`test262-slice 03 7011 13829 0 0 7011 7010 1 0`). The one case that does not pass,
  `test/built-ins/String/prop-desc.js`, fails in the pinned test262 harness before the
  engine's answer is judged; `docs/design/engine.md`, section 12, states it. 624 more cases
  that need a later slice moved there with their evidence
  (`test262-moves: 1656 case(s) moved`).
- test262 after these: `test262 total 53801 8982 7 44812 12120 32692 0`, 12,120 of the 44,812
  cases that run pass (27.0 %; 7,742 in 0.2.0). `test262-refused 46581` of the 85,556 runs
  meet a refusal of a construct the parser does not have.
- A documentation site under `website/`. It types no number: `task site-data` writes its data
  from the gate's committed files, and the gate step `site-data-check` fails when the data
  differs from them.
- The host interface: 165 numbered requirements, 84 of them with a test
  (`host-requirements: 165 requirement(s), 165 line(s): 84 test, 81 pending`; 164 and 80 in
  0.2.0). `HOST-165`: a String method that needs the tables of a later slice refuses a code
  unit outside ASCII with a counted refusal.

### Changed

- The gate runs its tests under cargo-nextest, ends a test at 60 seconds, builds every test
  binary before its clock and holds its test steps to 120 seconds
  (`gate-time test-steps 100.7 of 120`). It no longer makes the full test262 run: it compares a
  digest of the run's sources with a committed one, and the full run is the lane `slow`
  (`task slow`), which a release waits for.
- The compiler is pinned in `rust-toolchain.toml` (1.99.1) for the gate and the forge alike,
  and `task check-runner` runs the gate with the forge runner's variables.
- On the forge a change of planning or prose files alone skips the build and the tests, and a
  change of the documentation site alone runs the site's data check.
- `Object.getOwnPropertySymbols` is counted with the sixth slice, where `Symbol` is.
- Eight cases of later slices that passed in 0.2.0 no longer pass. Each passed only on the
  error of a method that was missing; the method exists now, and the case fails on the part
  of a later slice it needs: one of slice 5 (`Function/prototype/apply/this-not-callable.js`,
  a regular expression literal), five of slice 6 (String cases that call a method with a
  Symbol) and two of slice 18 (`trimLeft/reference-trimStart.js`,
  `trimRight/reference-trimEnd.js`). `docs/plan/slice-03-wave-2.md` names them.

### Limits of this release

- Not in the language yet: regular expression literals, identifiers outside ASCII, and
  everything after ES5 (`let` and `const`, arrow functions, classes, generators, promises,
  modules, and the rest). A script that uses one is refused with a `SyntaxError` that names
  the delivery slice that brings it.
  [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the nineteen delivery
  slices and what each brings.
- The builtins end at the core library. `Symbol`, `Map`, `Set`, `Promise`, `Reflect`, `Proxy`,
  `Date`, `RegExp`, `parseInt`, `parseFloat` and all of `Math` but `max` do not exist. A
  String method that needs the Unicode tables of a later slice refuses a code unit outside
  ASCII (`HOST-165`).
- test262: 12,120 of the 44,812 cases the gate's digest stands for pass, 27.0 %. The line
  reads `test262 total 53801 8982 7 44812 12120 32692 0`: 53,801 cases exist, 8,982 are
  excluded by recorded decision, 7 do not apply, 44,812 run, 12,120 pass, 32,692 fail, none
  ends at a budget. The first two delivery slices pass all their cases in all their runs and
  the third all but one (`test262-slice 01 31 62 0 0 31 31 0 0`,
  `test262-slice 02 4259 7543 0 0 4259 4259 0 0`,
  `test262-slice 03 7011 13829 0 0 7011 7010 1 0`). A refusal never counts as a pass, also
  when the script catches it.
- The host interface: 84 of its 165 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit; none is pending for the first three slices. 14 of the 58 scenarios of the
  interface's model pass on the engine, and the other 44 name what they wait for. The
  interface will change before 1.0.
- The engine's own tests: `68 test binaries: 667 passed, 0 failed, 1 ignored`, the tests of
  the engine crate in the mode that collects garbage at every safe point
  (`test-collect: 49 test binaries: 506 passed, 0 failed, 1 ignored`), and the test of an
  injected engine fault (`test-faults: 1 test binaries: 1 passed, 0 failed, 0 ignored`).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.3.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

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

[Unreleased]: https://github.com/beyond10x/swell/compare/0.6.0...HEAD
[0.6.0]: https://github.com/beyond10x/swell/releases/tag/0.6.0
[0.5.0]: https://github.com/beyond10x/swell/releases/tag/0.5.0
[0.4.0]: https://github.com/beyond10x/swell/releases/tag/0.4.0
[0.3.0]: https://github.com/beyond10x/swell/releases/tag/0.3.0
[0.2.0]: https://github.com/beyond10x/swell/releases/tag/0.2.0
[0.1.0]: https://github.com/beyond10x/swell/releases/tag/0.1.0

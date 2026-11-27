---
format: aep.planning-md/3
id: story:realm-walk-in-parts-under-the-cpu-bound
kind: story
status: active
title: The walk over every function of the realm runs in parts, each under its CPU bound, and a test holds that the parts cover the list
summary: The walk reads the places of the realm's functions from a committed file that one test in the default mode holds equal to the walk at every gate run; it runs in 2 to 8 parts by the index of a place, each resolving only its own places; a cover test holds the partition and the rows of expected, and a row whose answer depends on earlier calls carries the condition its clause tests.
relations:
- decomposes: epic:slice-04-numbers-and-dates
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/swell/tests/builtins_symbol_values.rs
- confidence: cited
  path: crates/swell/tests/fixtures/realm_places.txt
- confidence: cited
  path: crates/swell/tests/support/host.rs
- confidence: cited
  path: taskfiles/realm-places.yml
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T18:44:28Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T18:44:28Z", actor: "human:operator", revision: 3}
---
Waits for: nothing recorded. Base: the wave tip the brief names, `B` below; the figures of this
revision were read on `158888d`, the tip of `wave/big-2` on 2026-11-27, where the unit of
revision 3 stopped. The walk that gives every function of the realm a Symbol,
`builtins_symbol_values::every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_
error`, takes 4.07 CPU seconds alone in the collect mode at its gate count and 6.56 at its full
count on `158888d`, over the bounds of 2 and 2.5, and the realm grows with every unit that adds
builtins: 232 functions at `bef60f5`, 344 at `158888d`. Consumer: every unit of slice 4 and later
that adds functions to the realm (it measures the walk before it reports), and the lane `slow`,
which runs this file at its full count in both modes. The folder rule: a test over its budget is
a defect, gets a story and is fixed before the next feature; a higher bound is not a fix.
Revision 3 cut the walk into parts that each made the whole list of functions; its unit stopped
at that revision's own stop (Change, item 4 there), because the list alone takes 2.33 CPU seconds
in the collect mode and every part paid it. This revision takes the list out of the parts, in the
form revision 3 named as its fallback: a committed file holds the places of the walk's
functions, and each part resolves only its own places from it and makes the calls the walk
makes now. One test in the default mode, where the walk takes 0.05 CPU seconds, makes the list
again at every gate run and holds the file equal to it line for line; that test, not a digest,
keeps the file current (Change, item 2).

## Context

Lines are of `158888d`. The unit measures each count again on `B`.

- The walk is one test, `builtins_symbol_values.rs:491-615`. `functions_of_the_realm` (`:351-446`)
  builds the list: a breadth-first walk from the global object and from `FURTHER_ROOTS` (`:332`)
  over String and Symbol keys, each function once, with its place (a path string) and its holder.
  It runs no script. A place names one function: the list drops a value seen before (`:431`). It
  follows the values of data properties and [[Prototype]] only; the getter and setter of an
  accessor are taken as functions and not walked into (`:411-442`, `walk` false for them).
- The test makes its realm with `agent.create_realm(None, None)` (`:496`), not the padded helper,
  and says so (`:494-495`). `crates/swell/tests/support/host.rs:498-500` lists it as an exception
  of `create_realm`: "asserts over every function of the realm and prints their count; its full
  count is at its bound." `git grep -c '\.create_realm(' --
  crates/swell/tests/builtins_symbol_values.rs` prints 1.
- Per function the test calls it with a Symbol in four places, `this` and arguments 1 to 3
  (`:535-540`), `take(places)` of them (`:541`). `places_per_function` (`:470-478`) answers 2 when
  the feature `collect-every-safe-point` is on and `SWELL_FULL_COUNTS` is not `1`, else 4. So the
  default mode runs 4 places in the gate and in the lane `slow` alike: its gate count is its full
  count.
- The test asserts: the agent is alive (`:560`); `%AsyncFunction.prototype%.constructor` is in the
  list (`:561-566`); the answers other than normal and TypeError are the ten rows of `expected`
  (`:579-613`), each a line `<place> (<what>): <name>` with the least number of places that
  reaches it, kept when it is at most `places` (`:610`); `called >= 100 * places` (`:614`). A
  host error in a call ends the loop at once (`:545-548`). It prints `functions of the realm: <n>,
  calls: <c>` (`:559`). Its doc comment names 144 functions, 576 calls and 288 at the gate count
  (`:483-486`), and that of `places_per_function` "1.8 CPU seconds" (`:468-469`).
- The unit of revision 3 measured on `158888d` (its report of 2026-11-27, not in the tree; the
  pool's build node, 32 cores, load 2 to 9; 2 or 3 runs each, spread under 0.03 s; CPU seconds,
  user plus system, test profile at opt-level 0, the measuring command of the acceptance). 344
  functions; 1376 calls in the default mode and at the full count, 688 at the collect gate count.
  `W` is the cover test of revision 3 alone (the list, no call); `C` is the old test minus `W`.

  | | default | collect, gate count | collect, full count |
  |---|---|---|---|
  | The old test (`W + C`) | 0.068 | 4.07 | 6.56 |
  | The list alone (`W`) | 0.052 | 2.33 | 2.34 |
  | The calls after the list (`C`) | 0.016 | 1.74 | 4.22 |
  | Each of 2 parts that make the whole list | 0.061 / 0.068 | 3.19 / 3.18 | 4.31 / 4.31 |

  In the gate's profile `collect` (opt-level 1): the old test 0.384 at the gate count and 0.62
  at the full count, `W` 0.214, each of 2 parts 0.29 and 0.41.
- Why the cut of revision 3 cannot hold: every part makes the whole list, so every part pays `W`,
  2.33 CPU seconds in the collect mode, over the four fifths of the bound (1.6 at the gate count,
  2.0 at the full count) at any number of parts; and `W` alone is over the bound of 2 itself, so
  even its cover test could not be added in that form.
- This revision's probe, on `158888d` (a scratch variant of the test file, not committed, put
  back after): each function of the list resolved by its place (Gets along the path, no walk, no
  `getOwnPropertyDescriptors`), then the walk's calls for a part. Same measuring command; 2 runs
  each, spread at most 0.001 s; the pool's build node, 32 cores, load 0.3 to 1.4. CPU seconds:

  | | default | collect, gate count | collect, full count |
  |---|---|---|---|
  | The agent, the bare realm and the two further roots | 0.007 | 0.014 | 0.014 |
  | The same, and all 344 places resolved | 0.011 | 0.026 | 0.026 |
  | All 344 resolved and called, in one test | 0.026 | 0.532 | 1.423 |
  | The slowest of 2 parts | 0.016 | 0.213 | 0.512 |
  | The slowest of 3 parts | 0.014 | 0.136 | 0.324 |
  | The slowest of 4 parts | 0.012 | 0.103 | 0.230 |
  | The slowest of 5 parts | 0.011 | 0.080 | 0.174 |
  | The slowest of 8 parts | 0.010 | 0.057 | 0.114 |

  The same 1376 calls in the same order took 4.22 CPU seconds after the walk and about 1.40 after
  the resolution (1.423 less 0.026). The likely cause, read in the code and not profiled: a value
  handed to host code stays in `agent.scope` until the entry ends (`host/agent.rs:1059-1061`), and
  every collection traces that list as roots (`:1078-1082`). The walk hands out, per object, its
  descriptors, two key lists, each key and each field it reads, and a `toString` result per
  value; in the collect mode a collection runs at every safe point, so the walk's cost grows with
  what it has handed out, and so does every call after it in the same entry.
- The answers of a call depend on the calls made before it in the same realm. The probe's run of
  all 344 places in one test met exactly the ten rows of `expected` at the full count and the
  three of the gate count, as the walk does. With 3, 4, 5 and 8 parts, the part that held
  `global.Array.prototype.with` met two rows more at the full count: `global.Array.prototype.with
  (argument 2): RangeError` and `(argument 3): RangeError`. `with` with `this` %Array.prototype%
  and the index 0 throws a RangeError when the length of %Array.prototype% is 0
  (`#sec-array.prototype.with`, step 7), before the Symbol is read; `global.Array.prototype.push`,
  26 places earlier in the list (index 161 against 187), stores the Symbol in %Array.prototype%
  when it is called with it as `this` and the Symbol as argument 1. With 2 parts both fall in one
  part, so the 2-part patch of the unit met the ten rows; with 3, 4, 5 and 8 they do not. The
  length of %Array.prototype% is an own data property of an Array exotic object, so its Get
  through `Cx::get_property` runs no code and calls no builtin.
- A second run of the probe read that length by a Get immediately before each call of `with`
  with the Symbol as argument 2 and 3, with 1, 2, 3, 4, 5 and 8 parts (one run each, load 9 to
  12 on 32 cores; the slowest parts within 0.013 s of the table above). With 1 and 2 parts the
  length was 6 before both calls and both answered normally; with 3, 4, 5 and 8 it was 0 and both
  threw the RangeError. In the default mode and at the full count, in every part, the RangeError
  came exactly when the index 0 was at least the length read; at the gate count the part makes
  no call with the Symbol as argument 2 or 3.
- The 344 places, read from the probe (each resolved to the walk's holder and function: 0 of 344
  differ, in the default and in the collect build alike): 331 start at `global`, 13 at a further
  root (`%AsyncFunction.prototype%`, `%AsyncGeneratorFunction.prototype%`, which the test reaches
  by evaluating the source of `FURTHER_ROOTS`, not by Gets); 14 hold a step `.[[Prototype]]`; 15
  a Symbol key, written `[Symbol(Symbol.<name>)]`, the text `String` gives a well-known symbol
  (`:399-400`); 19 end at an accessor's function, written with ` get` (`:419`): 5 `@@species`
  getters, 10 flag getters of `RegExp.prototype`, `Symbol.prototype.description`,
  `Map.prototype.size`, `Set.prototype.size`, and %ThrowTypeError% as
  `%AsyncFunction.prototype%.[[Prototype]].caller get`. No function is reached only through a
  getter's function object, a bound function or an iterator prototype reached by a call: the
  walk does not reach such objects at all. The longest place has 88 characters.
- The host interface reads a property with `Cx::get_property` and a prototype with
  `Cx::get_prototype_of`, which runs no code for an ordinary object (`host/mod.rs:736-748`,
  `:829-845`); no host call reads an own property's descriptor, so an accessor's function is
  read with one call of `Object.getOwnPropertyDescriptor` (19 calls for the 344 places).
- The list does not depend on the feature `collect-every-safe-point`: the probe's walk wrote the
  same 344 places byte for byte in both builds. In the code, `git grep -n 'feature =
  "collect-every-safe-point"' -- crates/swell/src` prints 14 lines in 8 files
  (`heap/collect.rs`, `host/agent.rs`, `host/jobs.rs`, `interp/dispatch.rs`,
  `object/builtin_function.rs`, `object/ecmascript_function.rs`, `ops/objects.rs`, `script.rs`):
  the collection at a safe point (`host/agent.rs:1048`) and counts of unit tests. None is under
  `crates/swell/src/realm/` or `crates/swell/src/builtins/`, and none defines a property.
- Committed files that a test reads in place of a product, in this repository:
  `crates/swell/tests/fixtures/host_135_math.txt` (header lines starting with `#`, read at run
  time by `math_platform.rs:287-291` through `env!("CARGO_MANIFEST_DIR")`, and compared there
  with the product made again in the test itself); `conformance/test262/expected.txt`, whose
  product, the full test262 run, takes a pool job of up to 30 minutes, so the gate compares a
  digest of its sources in its place (`test262-check`) and the lane `slow` makes the run
  (`taskfiles/test262.yml`, `test262-expected` the one task that writes the file).
- How the repository cuts a heavy test into parts: `language.rs`, `language_part` (`:105-111`)
  takes the scripts whose index leaves `part` when divided by the number of parts, the macro
  `language_parts!` (`:183`) writes one `#[test]` per part and the list `LANGUAGE_PARTS`, the first
  part keeps the old name, the others are `..._part_<n>` from 2; `fuzz.rs`, `PARTS` (`:2223`),
  `part_seeds` (`:2226`), `the_parts_cover_every_sequence` (`:2357`).
- `crates/swell/tests/main.rs` is generated from the files `tests/*.rs` (`taskfiles/test-main.sh`,
  function `modules`); a file under `tests/fixtures/` adds no module.
- The old name outside its file (`git grep -n 'every_function_of_the_realm_given_a_symbol' -- .
  ':!crates/swell/tests/builtins_symbol_values.rs' ':!.engineering/planning/review-result'`):
  `crates/swell/tests/support/host.rs:498`, the exception line, which the unit rewrites (Change,
  item 8); `.engineering/planning/story/tests-hold-at-any-size-of-the-realm.md:173`, an
  acceptance command that runs the test by `test(=...)`, which after the cut runs part 0 only
  (part 0 keeps the name); `docs/plan/slice-10-wave-1.md:326`, a record of a past wave, which
  stays; and this story.
- The bound. `AGENTS.md:316-322`: a test stays under 1.5 CPU seconds alone in the default mode,
  under 2 with a collection at every safe point, under 2.5 at its full count in either mode,
  measured with the test binary run with `--exact <name> --test-threads 1` in a shell of its own
  whose `times` gives user plus system, one job of the pool. The same bullet names
  `determinism::host_133_same_calls_same_hooks_same_outcomes` and the `language` parts
  (`:332-337`); no sentence of `AGENTS.md` names this walk (`grep -n 'symbol_values\|walk'
  AGENTS.md` prints nothing).
- A test is killed at 60 s (`.config/nextest.toml:2`). The gate's test steps fail over 120 s of
  wall time and over the CPU entry of `CPU_BOUNDS_BY_CORES`, now `16:245` (`AGENTS.md:271`); an
  entry changes only with a measured pair (`AGENTS.md:278-283`). The gate's step `test` runs with
  `RUSTFLAGS=-D warnings` and `cargo nextest run --workspace --locked --profile default`
  (`Taskfile.yml:739-758`).

## Change

No file of `crates/swell/src` changes. Figures are of `158888d` (Context); the unit reads them
again on `B`.

1. **The committed file of places.** `crates/swell/tests/fixtures/realm_places.txt`, beside
   `host_135_math.txt`; a file under `tests/fixtures/` adds no module, so `main.rs` does not
   change.
   - Its first two lines start with `#` and say what the file is and that `task
     realm-places-expected` writes it and nothing else does. Every other line is one place, with
     no blank line, and the file ends with a newline. It holds no digest.
   - The order of the place lines is the order of the list of `functions_of_the_realm`, the
     walk's breadth-first order. The file then equals the list line for line, so the test of
     item 3 compares two lists, and a part's indices (item 5) are the list's indices. The place a
     function is named by is the first path the walk meets, which depends on that order anyway.
   - A place is the text the walk gives today (`:397-422`), so the rows of `expected` keep their
     places: a root, then steps, then at most one accessor suffix.
     - The root is `global` (the realm's global object) or the path of an entry of
       `FURTHER_ROOTS` (`%AsyncFunction.prototype%`, `%AsyncGeneratorFunction.prototype%`). Such
       a root is not reached by Gets: it is the object the entry's source answers, evaluated in
       the realm as the walk does now (13 places of 344).
     - `.<name>` is a Get by the String key `<name>`.
     - `[Symbol(Symbol.<name>)]` is a Get by the well-known symbol `Symbol.<name>`, which is read
       first as the Get of `<name>` on `global.Symbol` (15 places).
     - `.[[Prototype]]` is [[GetPrototypeOf]] through `Cx::get_prototype_of`, which runs no
       builtin for an ordinary object (14 places).
     - A last ` get` or ` set` names the getter or the setter of the own accessor property that
       the last step names, on the object the steps before it reach; it is read with one call of
       `Object.getOwnPropertyDescriptor` and the Get of `get` or `set` on the descriptor, since
       no host call reads a descriptor (19 places).
     - The holder of a place, the `this` of the calls with the Symbol as an argument, is the
       object the last step starts from: the object the walk holds the function on now.
   - No place of the 344 needs another form (Context): the walk reaches no function through a
     getter's function object, a bound function or an iterator prototype reached by a call.
2. **The writer, and how the file stays current.**
   - The rule of the organization that an expensive product is made once and never per test
     applies here so: the product is the list of places, a committed file that the parts read
     and do not make; its currency is held by making the list again at every gate run, in the
     default mode where that costs 0.05 CPU seconds, and comparing it with the file line for
     line (item 3), in place of a digest of its sources. The stop: when the walk passes 1.2 CPU
     seconds in the default mode, the test of item 3 moves to the lane `slow` and a check of a
     digest of the file's sources takes its place in the gate, as a story of its own.
   - `realm-places-expected`, in a new `taskfiles/realm-places.yml` of POSIX sh included by
     `Taskfile.yml` as the others are (`flatten: true`), the one run that writes the file, named
     as `test262-expected` is. It runs `cargo nextest run --workspace --locked --profile default
     -E 'test(=builtins_symbol_values::the_realm_places_file_equals_the_walk)'` with
     `RUSTFLAGS=-D warnings`, the build and the flags of the gate's step `test`
     (`Taskfile.yml:751-758`), and with `SWELL_REALM_PLACES_OUT` set to
     `.out/realm-places/walk.txt`; the test of item 3 then writes the walk's places there. The
     task writes the two comment lines and those places to the file and prints
     `realm-places-expected: <t> places written to crates/swell/tests/fixtures/realm_places.txt`.
     It fails when the run fails or the written list is empty. It compiles, so a unit runs it as
     `slot task realm-places-expected`; the gate does not run it.
   - The writer gets no gate selftest: what it writes is held by the test of item 3 at the next
     gate run. That test reads every line of the file but the two comment lines, so a writer that
     drops, adds, changes or reorders a place, or writes a place line where a comment line
     belongs, turns the gate's step `test` red; only a change inside the comment lines passes,
     and no test or part reads those. A selftest of the writer would have to build and run the
     test binary, which no script of `tests/gate/` does.
3. **One test holds the file equal to the walk**, `the_realm_places_file_equals_the_walk`, in
   the default mode only (`#[cfg(not(feature = "collect-every-safe-point"))]`, as
   `proxy.rs:890` does). The walk takes 0.052 CPU seconds there and 2.33 in the collect mode,
   over that mode's bound of 2 (Context).
   - In the bare realm of item 5 it runs `functions_of_the_realm` as the test does now. With
     `SWELL_REALM_PLACES_OUT` set it writes the walk's places to that path, one per line, and
     ends without comparing (the writer of item 2). Without it, the walk's places must equal the
     file's place lines in order; on a difference it fails with one line: `realm places: the walk
     reaches <w> places, the file holds <f>; missing from the file: [<places>]; not reached by the
     walk: [<places>]; first line that differs: <k> (walk <place>, file <place>); slot task
     realm-places-expected writes the file again`.
   - Then it resolves every line by its place (item 5) and asserts that the holder and the
     function are the walk's; the probe found 0 of 344 different.
   - It prints `realm places: <t> lines of crates/swell/tests/fixtures/realm_places.txt equal
     the walk`.
4. **The collect mode, and the rule for the engine's source.**
   - The list does not depend on the feature (Context: the same 344 places in both builds; no
     use of the feature in code that makes the realm). In the collect mode every line is resolved
     by one part (item 6), and a line that does not resolve to an object fails its part naming the
     line, so a function that the collect build lacks is red there. A function that only the
     collect build has is not seen in that mode.
   - So the story states a rule for the engine's source, and the unit checks it: no code that
     defines a property of the realm depends on the feature `collect-every-safe-point`. Its
     check is `git grep -n 'feature = "collect-every-safe-point"' -- crates/swell/src/realm
     crates/swell/src/builtins`, the directories that make the realm's functions, which prints
     nothing on `158888d` (Acceptance, item 8); the 14 lines the feature has elsewhere in
     `crates/swell/src` (Context) are collection and counts of unit tests. The sentence of item 8
     writes the rule into `AGENTS.md`.
5. **The parts.**
   - `WALK_PARTS`, a list of part numbers from 0, and the macro `walk_parts!`, whose invocation
     writes each test as `#[test]` and `fn <name> = <part>;`, so that
     `taskfiles/realm-room-tests.awk` sees each part by its name; it prints them `other`, and
     `test-realm-sizes` does not run them. Part 0 keeps the name
     `every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_error`; the others are
     `..._part_<n>`, `<n>` from 2.
   - `in_a_bare_realm(f)`: the agent, the bare realm (the one call of
     `agent.create_realm(None, None)` in the file, with its comment), the further roots of
     `FURTHER_ROOTS`, and `f(cx, global, roots)` in the one entry of the realm; it answers the
     agent's state after it and the answer of `f`. The parts and the test of item 3 use it.
   - `realm_places()` reads the file at run time through `concat!(env!("CARGO_MANIFEST_DIR"),
     "/tests/fixtures/realm_places.txt")`, as `math_platform.rs:287-291` reads its file, and
     answers the lines that do not start with `#`. `parse_place` reads a line into its root, its
     steps and its accessor suffix, or answers why it cannot; `resolve_place` answers the holder
     and the function of a parsed place in a realm, or the step that reached no object.
   - A place is in part `p` when its index among the place lines, counted from 0, leaves `p`
     when divided by `WALK_PARTS.len()` (`walk_part_indices`).
   - A part reads the file, makes the bare realm and the Symbol, and resolves all its places
     before its first call, since a call can change a property on a later place's path. A place
     that does not resolve to an object fails the part with `walk part <p> of <n>: line <k>
     <place>: <step> reached <value>, not a function`. Then it makes the calls the walk makes
     now: per function the same four calls (`this` the Symbol; arguments 1 to 3 with `this` the
     holder and 0 before the Symbol), `take(places_per_function())` of them, the same answers
     read (`answer`), and the same early end on a host error, whose row then fails the part.
   - `WALK_EXPECTED` becomes a constant of the file: rows of the place of a function, the place
     of the Symbol, the name of the error, the least number of places that reaches the call, and
     a field `when` of the type `When`, which has two forms:
     - `When::Always`: the clause throws before the Symbol is read whatever earlier calls left in
       the realm. The ten rows of today are `Always`, their texts and least numbers unchanged.
     - `When::IndexAtLeastLength(index)`: the clause throws a RangeError when `index` is at least
       the length of the call's `this`, as `#sec-array.prototype.with`, step 7, tests ("If
       actualIndex ≥ len, throw a RangeError"), and that length is what earlier calls of the
       same realm left in the holder. Two rows: `global.Array.prototype.with`, `argument 2`,
       `RangeError`, 3, `IndexAtLeastLength(0)`, and the same with `argument 3`, 4. Their comment
       gives the clause and why the length moves: `global.Array.prototype.push`, 26 places
       earlier, stores the Symbol in %Array.prototype% when it runs in the same realm (Context).
   - Before each call that a row with a condition names (its place, its place of the Symbol, its
     least number at most `places`), the part reads the condition from the realm: the Get of
     `length` on the call's `this` through `Cx::get_property`, a Number converted as
     LengthOfArrayLike does (`#sec-lengthofarraylike`, ToLength). A value that is not a Number
     fails the part naming the row and the value.
   - Each part asserts: the agent is alive; the rows it found equal, in the order of its calls,
     the `Always` rows of its places with their least number at most `places` and the conditional
     rows whose condition held. Each kind of difference has its line: `walk part <p> of <n>: a row
     not in WALK_EXPECTED: <row>`; `walk part <p> of <n>: the row <row> was not met`; `walk part
     <p> of <n>: the row <row> under IndexAtLeastLength(<i>) read length <v>: the condition is
     <true|false> and the call answered <answer>`. A row with a condition is thus met exactly when
     its condition holds.
   - The count of calls: the Get of a condition is not a call of a function of the realm, so `<c>`
     and the assertion that the calls are the part's places times `places` count the calls of the
     functions only, as now. The part asserts no count of steps; the Get reads an own data property
     of an Array and runs no code. A part makes one such Get per conditional row it reaches: two
     in the part that holds `with` in the default mode and at the full count, none at the gate
     count, where the rows' least numbers, 3 and 4, are above its 2 places.
   - On `158888d`, with the probe of this revision (Context): at `n` = 1 and 2 the length read
     before both calls of `with` was 6, the condition false, and the calls answered normally; at
     `n` = 3, 4, 5 and 8 it was 0, the condition true, and both calls threw the RangeError. In
     every part of each `n`, in the default mode and at the full count, the row was met exactly
     when its condition held.
   - Each part prints `walk part <p> of <n>: <f> of <t> places, <c> calls`.
   - The number of parts. The unit measures every part, with the measuring command of the
     acceptance, in each of the three cells. `n` is the smallest from 2 to 8 for which the
     slowest part is at most four fifths of the cell's bound in all three cells: 1.2 CPU seconds
     in the default mode, 1.6 in the collect mode at its gate count, 2.0 in the collect mode at
     its full count (four fifths of 1.5, 2 and 2.5). On `158888d` that is `n` = 2: the slowest
     part takes 0.016, 0.213 and 0.512.
   - Why not one test, though all 344 places in one test took 0.026, 0.532 and 1.423 on
     `158888d`: the cost of a part grows faster than its number of functions (1.423 for 344,
     0.512 for 172, 2.8 times for twice the functions), 1.423 is 71 % of 2.0, and the realm
     grew by 112 functions from `bef60f5` to `158888d`. With 2 parts the slowest is at 26 % of
     2.0. A fifth of each bound stays as room for a realm that grows; a function the realm gains
     adds its resolution and its calls to one part, the part of its index.
   - The stop: the walk is a defect that gets a story, and the unit adds no part and reports
     each reading, when no `n` up to 8 holds the rule, or when the test of item 3 takes more than
     1.2 CPU seconds in the default mode (the stop of item 2).
6. **The cover test.** `the_parts_of_the_walk_cover_every_function_of_the_realm` reads the file
   and makes no agent, so it costs the same in every mode. It asserts, each with its line:
   - `WALK_PARTS` is numbered from 0 without a gap and has at least 2 parts;
   - every place line parses (`line <k> of <file>: <line>: <reason>`);
   - no place is on two lines (`the place <place> is on <k> lines of <file>`);
   - the indices the parts take, together, are the indices of the place lines, each once
     (`places of the file in no part: [...]; in more than one part: [...]`);
   - each row of `WALK_EXPECTED` names a place the file holds exactly once (`the row "<place>
     (<what>): <error>" of WALK_EXPECTED names a place the file holds <k> times`), since a part
     would otherwise drop the row unseen;
   - `%AsyncFunction.prototype%.constructor` is a line, and the file holds at least 100 places
     (the old `called >= 100 * places`, `:614`);
   - it prints `places of the realm: <t>, in <n> parts: <f0> + <f1> + ...`.
7. **When the realm gains or loses a function.**
   - The test of item 3 is red in the gate's step `test` and names `slot task
     realm-places-expected`. The unit runs it and commits the file; `git diff` of the file shows
     the places added or removed. A change of the engine that adds or removes no function leaves
     the file as it is, so units that do not touch the realm do not write it, and two units that
     both add functions meet in the file only where their places are.
   - A new answer other than normal and TypeError gets a row: `Always` when the clause throws
     before the Symbol is read whatever earlier calls left; when it depends on earlier calls, a
     row with the condition the clause itself tests, a form of `When` added for a clause that no
     form says yet, with the clause named in the comment. Never a row without a condition that a
     part may meet or not.
   - The unit measures each part in the three cells. A part over four fifths of its bound makes
     it add the line `..._part_<n+1> = <n>` to `walk_parts!`; the cover test follows. Up to 8
     parts; past them, or with the test of item 3 over 1.2, the stop of item 5 holds.
8. **The records, the files and the sentence.**
   - The doc comment of the parts drops the numbers 144, 576 and 288 and says the count is
     printed. The doc comment of `places_per_function` drops "1.8" and states the unit's
     readings: the slowest part at the chosen `n` and the sum of the parts in each cell, with the
     commit they were read on. The doc comment of `WALK_PARTS` gives, as the comment above
     `language_parts!` does, the readings that chose `n` (the slowest part at each `n` measured,
     each cell, the load and the core count) and the rule of item 5.
   - `support::host::create_realm`: its line for the walk names
     `builtins_symbol_values.rs::in_a_bare_realm` and says that the parts and the test of item 3
     make their realm through it, because the file holds the places of the bare realm; its
     reason no longer says "its full count is at its bound".
   - Writes: `crates/swell/tests/builtins_symbol_values.rs` (items 3 to 6, 8);
     `crates/swell/tests/fixtures/realm_places.txt` (new, written by `task
     realm-places-expected` only); `taskfiles/realm-places.yml` (new, item 2); `Taskfile.yml`
     (its include, and nothing else: `check` gets no step); `crates/swell/tests/support/host.rs`
     (the doc comment line of the walk, and nothing else).
   - Not written by the unit: `AGENTS.md` (the coordinator, below), `crates/swell/tests/main.rs`,
     any file of `crates/swell/src`, `Cargo.lock`, a `Cargo.toml`, `.github/workflows/`,
     `tests/gate/`, and the section "The gate's numbers".
   - Shared with the units of slice 4 that add functions: `builtins_symbol_values.rs` (a row of
     `WALK_EXPECTED`, an entry of `FURTHER_ROOTS`) and the file of places. The unit applies the
     cut on `B` with the rows and roots `B` holds.
   - The sentence for `AGENTS.md`, "Commands and the gate", in the bullet that starts "A test
     stays under 1.5 CPU seconds" (`:316`), after its sentence that ends
     `language::the_language_directory_holds_only_scripts_the_parts_run` (`:337`). The
     coordinator writes it on the merge:

     > The walk that gives every function of the realm a Symbol (the tests of
     > `builtins_symbol_values` named
     > `every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_error` and its
     > `_part_<n>`) reads its functions from `crates/swell/tests/fixtures/realm_places.txt`, one
     > place per line, and runs in parts: a place is in the part of its line's index modulo the
     > length of `WALK_PARTS`. `the_realm_places_file_equals_the_walk` makes the list again in
     > the default mode at every run and fails when the file differs; a unit that adds or
     > removes a function of the realm runs `slot task realm-places-expected` and commits the
     > file. No code that defines a property of the realm depends on the feature
     > `collect-every-safe-point`: `git grep -n 'feature = "collect-every-safe-point"' --
     > crates/swell/src/realm crates/swell/src/builtins` prints nothing. A unit that adds
     > functions to the realm measures each part with the command above, in the default mode
     > and in the collect mode, at the gate count and at the full count; a part over four
     > fifths of its bound (1.2 CPU seconds in the default mode, 1.6 in the collect mode at the
     > gate count, 2.0 at the full count) makes it add a part to `WALK_PARTS`, up to 8 parts.
     > Past 8 parts, or with the walk over 1.2 CPU seconds in the default mode, the walk is a
     > defect that gets a story: the unit reports the readings and adds no part.

**Against the saved patch** of the unit of revision 3 (`walk-in-parts.patch`, on `158888d`).
Kept: `walk_part_indices` as it is; `WALK_EXPECTED` with the field `when` added; the macro
`walk_parts!`, its invocation form and the names of part 0 and `_part_2`; the loop of `walk_part`
with `break 'functions`; the name of the cover test and its assertions of the partition and of
the rows, with their messages. Changed: `functions_of_a_bare_realm`, which made the list and
handed it to `f`, becomes `in_a_bare_realm`, which makes no list; `walk_part` resolves its places
from the file and does not walk; the cover test reads the file and makes no agent; a part reads
the condition of a conditional row before its call, since with 3, 4, 5 or 8 parts the answers of
`with` differ from the walk's (Context); the printed line counts places. New: the file,
`realm_places`, `parse_place`, `resolve_place`, the type `When`, the test of item 3 with its
writing mode, and `taskfiles/realm-places.yml`. The placeholders `READINGS` and `MEASUREMENTS`
of the patch get the readings of item 8.

## Tests

- The parts of Change item 5 in place of the one test: `n` tests, part 0 under the old name.
  Each is at most four fifths of its bound in each of the three cells (1.2, 1.6, 2.0 CPU
  seconds); on `158888d` the probe read 0.016, 0.213 and 0.512 for the slowest of 2.
- `the_realm_places_file_equals_the_walk` (Change item 3), new, in the default mode only: the walk
  and the resolution of every place, about 0.06 CPU seconds on `158888d` (the walk 0.052, the
  resolution of 344 places 0.004 more than the bare realm). It is also what holds the file the
  writer `realm-places-expected` commits (Change item 2).
- `the_parts_of_the_walk_cover_every_function_of_the_realm` (Change item 6), rewritten from the
  saved patch: it reads the file and makes no agent, in every mode.

No script of `tests/gate/` is added, and `check` gets no step. The tests of the file other than
the walk do not change. Each test added took under 1 CPU second in the probe on `158888d`; the
unit prints the time of each in its report and names any over 5 s with its time.

## Acceptance

Commands are written as the gate's tasks write them on `158888d`. A `cargo` command takes its
slot of the pool itself; every other command that compiles or tests starts with `slot`; a command
that compiles nothing (`git grep`, `grep`) starts without it. Each pool job ends within 10
minutes unless it names its time (`SLOT_TIMEOUT=30m cargo ...`, `slot --timeout 30m ...`, at most
30 minutes). A test is killed at 60 s, and the gate's test steps end within 120 s.

The measuring command, from `AGENTS.md:316-326`. The build prints the path of the test binary,
`<bin>` below; the measure job is a shell of its own, so `times` reads the binary alone:

- build, default mode: `SLOT_TIMEOUT=30m cargo test --locked -p swell --test main --no-run`;
- build, collect mode: `SLOT_TIMEOUT=30m cargo test --locked -p swell --features
  collect-every-safe-point --test main --no-run`;
- measure: `slot sh -c 'SWELL_FULL_COUNTS=<f> <bin> --exact builtins_symbol_values::<test>
  --test-threads 1 --nocapture; times'`; the second line `times` prints is the children's user
  and system time, and their sum is the reading. No second job of the same tree runs while one
  builds.

The cells are three, each read with its own `<f>` and binary:

- the default mode, `<f>` empty (its gate count is its full count), against 1.2;
- the collect mode at its gate count, `<f>` empty, against 1.6;
- the collect mode at its full count, `<f>` `1`, against 2.0.

1. **Red first**, on `B`, before any file changes. The measure job with `<test>` the old test name
   prints `functions of the realm: <N0>, calls: <4 x N0>` at the full count, and readings over
   the bound in both collect cells: over 2 at the gate count, over 2.5 at the full count (on
   `158888d`: `N0` 344, 4.07 and 6.56). The unit reports `N0` and the three readings.
2. **The file.** `slot task realm-places-expected` prints `realm-places-expected: <N0> places
   written to crates/swell/tests/fixtures/realm_places.txt`; `grep -cv '^#'
   crates/swell/tests/fixtures/realm_places.txt` prints `<N0>` and `grep -c '^#'` prints 2.
   Running the writer a second time leaves `git diff --stat` of the file empty.
3. **The readings.** With the cut in place, on `B`:
   - the measure job of every part (part 0 by the old name, then `..._part_<n>`) in each cell
     prints `walk part <p> of <n>: <f> of <N0> places, <c> calls` and a reading at most 1.2
     (default), 1.6 (collect, gate count), 2.0 (collect, full count). The report gives `n`, the
     slowest part of each cell at each `n` measured, and the rule of Change item 5; `n` is the
     smallest from 2 to 8 that holds it in all three cells;
   - `the_realm_places_file_equals_the_walk`, default cell: prints `realm places: <N0> lines of
     crates/swell/tests/fixtures/realm_places.txt equal the walk` and a reading at most 1.2;
   - `the_parts_of_the_walk_cover_every_function_of_the_realm`, all three cells: prints `places
     of the realm: <N0>, in <n> parts: <f0> + ...` and a reading at most 1.2, 1.6 and 2.0.
4. **The walk's tests, in both modes.** `cargo nextest run --locked -p swell --profile default
   --no-capture -E
   'test(/^builtins_symbol_values::(every_function|the_parts_of_the_walk|the_realm_places)/)'`
   prints `<n+2> tests run: <n+2> passed`. `cargo nextest run --locked -p swell --features
   collect-every-safe-point --config .cargo/collect-profile.toml --cargo-profile collect --profile
   default --no-capture -E` with the same filter prints `<n+1> tests run: <n+1> passed` (the test
   of Change item 3 is not in that build). Every printed `<t>` is `<N0>`; the `<c>` of the parts
   add up to `<N0> x 4` in the default mode and `<N0> x 2` in the collect mode, shown with the sum
   of the printed numbers. The same two commands as `SLOT_TIMEOUT=30m SWELL_FULL_COUNTS=1 cargo
   nextest run <the arguments above>`: the `<c>` add up to `<N0> x 4` in both modes.
5. **Planted defects.** Each is a scratch edit in the unit's worktree, run, and undone before the
   commit (`git diff --stat` of the edited file empty after); the report quotes each red line.
   - A stale file, three ways, each red in the default mode in
     `the_realm_places_file_equals_the_walk` with one line:
     - one place line removed: `realm places: the walk reaches <N0> places, the file holds
       <N0-1>; missing from the file: [<the place>]; not reached by the walk: []; first line that
       differs: <k> (walk <the place>, file <the next place>); slot task realm-places-expected
       writes the file again`;
     - one line `global.Array.prototype.planted` added: `... the file holds <N0+1>; missing from
       the file: []; not reached by the walk: ["global.Array.prototype.planted"]; first line that
       differs: <k> (walk <place>, file global.Array.prototype.planted); ...`; in the collect mode
       at the gate count the part that owns the line fails with `walk part <p> of <n>: line <k>
       global.Array.prototype.planted: .planted reached undefined, not a function`;
     - two place lines swapped: `... the file holds <N0>; missing from the file: []; not reached
       by the walk: []; first line that differs: <k> (walk <a>, file <b>); ...`.
     After each, `slot task realm-places-expected` writes the file back: `git diff --stat` of it
     is empty and the test passes.
   - The index rule of `walk_part_indices` with `% (WALK_PARTS.len() + 1)`: the cover test fails
     with `places of the file in no part: [...]`.
   - The place of one row of `WALK_EXPECTED` changed by one character: the cover test fails with
     `the row "<place> (<what>): <error>" of WALK_EXPECTED names a place the file holds 0 times`.
   - One place line doubled: the cover test fails with `the place <place> is on 2 lines of
     crates/swell/tests/fixtures/realm_places.txt`.
   - One place line with its root `global` written `globl`: the cover test fails with `line <k>
     of crates/swell/tests/fixtures/realm_places.txt: globl...: <reason>`.
   - The `Always` row `global.RegExp`, `argument 3` removed from `WALK_EXPECTED`: at the full
     count the part that owns `global.RegExp` fails with `walk part <p> of <n>: a row not in
     WALK_EXPECTED: global.RegExp (argument 3): SyntaxError`.
   - The row `global.Array.prototype.with`, `argument 2` made `When::Always`: in the default mode,
     at an `n` where the part that owns `with` also holds `push` (`n` = 2 on `158888d`), that part
     fails with `walk part <p> of <n>: the row global.Array.prototype.with (argument 2):
     RangeError was not met`. At an `n` where it does not, the unit sets `WALK_PARTS` to 2 parts
     for this edit only, or reports why no such `n` is at hand.
   - The same row with `IndexAtLeastLength(<l>)`, `<l>` the length the part reads before the call
     (6 at `n` = 2 on `158888d`): that part fails with `walk part <p> of <n>: the row
     global.Array.prototype.with (argument 2): RangeError under IndexAtLeastLength(<l>) read
     length <l>: the condition is true and the call answered normal`.
6. **The padding does not move it.** `SWELL_TEST_REALM_FUNCTIONS=200 cargo nextest run --locked
   -p swell --profile default --no-capture -E
   'test(/^builtins_symbol_values::(every_function|the_parts_of_the_walk|the_realm_places)/)'`
   prints the same `<N0>` on every line and `<n+2> tests run: <n+2> passed`.
7. **The records are true.** `git grep -n '144 functions\|576 calls\|288 at\|1\.8 CPU'
   crates/swell/tests/builtins_symbol_values.rs` prints nothing. `git grep -c '\.create_realm('
   -- crates/swell/tests/builtins_symbol_values.rs` prints
   `crates/swell/tests/builtins_symbol_values.rs:1`. `git grep -n 'builtins_symbol_values.rs::'
   -- crates/swell/tests/support/host.rs` prints one line,
   `builtins_symbol_values.rs::in_a_bare_realm`, and no line of that file names
   `every_function_of_the_realm`.
8. **The rule for the engine's source** (Change item 4). `git grep -n 'feature =
   "collect-every-safe-point"' -- crates/swell/src/realm crates/swell/src/builtins` prints
   nothing, as on `158888d`.
9. **The gate.** `slot --timeout 30m task check` exits 0. The unit reports the lines `gate-time
   test-steps <s> of 120, cpu <c>, load <l> on <n> cores` and `gate-cpu test-steps ...` on `B` and
   after, and the CPU seconds of the steps `test` and `test-collect` before and after (the step
   `test` adds the test of Change item 3 and the cover test; `test-collect`, in the profile
   `collect`, drops the old test's 0.384 for the parts). When the gate is red on its CPU line or
   its time line, the unit stops, changes no bound (`CPU_BOUNDS_BY_CORES`,
   `AGENTS.md:278-283`), and reports the two readings with `n`.
10. `git diff --stat B -- crates/swell/src Cargo.lock '*/Cargo.toml' tests/gate` prints nothing;
    `sh taskfiles/test-main.sh check` exits 0 and prints its last line `test-main: <k> crate(s)
    with tests, each with one integration-test binary and its tests/main.rs as generated`, `<k>`
    the number it prints on `B`; `git diff --stat B -- crates/swell/tests/main.rs` prints
    nothing.
11. `cargo fmt --all --check`, `slot task clippy` and `slot task forbid-unsafe-check` exit 0.

**The stop.** The unit stops before it commits, adds no part, and reports every reading of items
1 to 3 with `n`, when no `n` from 2 to 8 holds the rule of Change item 5, when the test of Change
item 3 takes more than 1.2 CPU seconds in the default mode, or when a place of the file does not
resolve to the walk's function in the default mode; the walk is then a defect that gets a story.

## Not in this story

- Any file of `crates/swell/src`; a step count, a byte count or a ceiling.
- The two tests that stay over the bound (`determinism::host_133_...`, `language` part 3), and
  every other test of the file.
- A padded realm for the walk: the padding is host functions that answer `undefined`, which the
  walk would call for nothing, and the file would then depend on `SWELL_TEST_REALM_FUNCTIONS`.
- A digest of the file's sources and a gate check of it. The test of Change item 3 makes the
  list again at every gate run for 0.05 CPU seconds; a digest comes with its own story only at
  the stop of Change item 2 (the walk over 1.2 CPU seconds in the default mode).
- A part of the lane `slow` for the file, and a change of the path list of
  `.github/workflows/slow.yml`.
- A cheaper walk in the collect mode (fewer values handed out per entry, or an entry of its own
  per object), which could let the test of Change item 3 run in that mode too. The cause named
  in Context is read in the code, not profiled.
- Running this file's collect mode only in the gate's profile `collect` (opt-level 1), where the
  old test takes 0.384 and 0.62: that changes which build the lane's `slow-tests` uses, a
  decision of the lane.
- Calls that do not depend on the calls before them (a fresh holder per function, or the realm
  set back between functions); a row whose answer depends on them carries its condition instead
  (Change item 5).
- A test that asserts CPU seconds of a part, and a change of `CPU_BOUNDS_BY_CORES`.
- Fewer calls per function in the collect mode (the gate count of `places_per_function` stays,
  2 places there, 4 in the default mode and at the full count).
- The section "The gate's numbers" of `AGENTS.md`, which the coordinator records at a close.

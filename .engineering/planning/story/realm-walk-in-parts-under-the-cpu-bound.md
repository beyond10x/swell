---
format: aep.planning-md/3
id: story:realm-walk-in-parts-under-the-cpu-bound
kind: story
status: active
title: The walk over every function of the realm runs in parts, each under its CPU bound, and a test holds that the parts cover the list
summary: The walk reads the places of the realm's functions from a committed file that one test in the default mode holds equal to the walk at every gate run; it runs in 2 to 8 parts, a place in the part its text hashes to, each part resolving and checking only its own places; a cover test holds the file's form, the partition and the rows of expected, and a row whose answer depends on earlier calls carries the condition its clause tests.
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
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T18:44:28Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T18:44:28Z", actor: "human:operator", revision: 3}
---
Waits for: nothing recorded; `story:map-upsert-and-group-by` waits for this story. Base: the wave
tip the brief names, `B` below; the figures of this revision were read on `158888d`, the tip of
`wave/big-2` on 2026-11-27, where the unit of revision 3 stopped. The walk that gives every
function of the realm a Symbol, the test
`every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_error` of
`builtins_symbol_values`, takes 4.07 CPU seconds alone in the collect mode at its gate count and
6.56 at its full count on `158888d`, over the bounds of 2 and 2.5, and the realm grows with every
unit that adds builtins: 232 functions at `bef60f5`, 344 at `158888d`. Consumer: every unit of any
slice that adds functions to the realm (it measures the walk before it reports;
`story:map-upsert-and-group-by` adds three), and the lane `slow`, which runs this file at its full
count in both modes. The folder rule: a test over its budget is a defect, gets a story and is
fixed before the next feature; a higher bound is not a fix. Revision 3 cut the walk into parts
that each made the whole list of functions; its unit stopped at that revision's own stop (Change,
item 4 there), because the list alone takes 2.33 CPU seconds in the collect mode and every part
paid it. This revision takes the list out of the parts: a committed file holds the places of the
walk's functions, each part takes the places whose text hashes to it, resolves them, checks that
each is callable and makes the calls the walk makes now. One test in the default mode, where the
walk takes 0.05 CPU seconds, makes the list again at every gate run and holds the file equal to
it line for line, in place of a digest; that form of the rule "an expensive product is made once"
was put to the operator on 2026-11-27, and the unit does not start before its answer is recorded
(Change, item 2).

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
- The probe of this story, on `158888d` (a scratch variant of the test file, not committed, put
  back after each run): each function of the list resolved by its place (Gets along the path, no
  walk, no `getOwnPropertyDescriptors`), then the walk's calls for a part. Same measuring
  command. Its first runs took a place into part `p` by its index modulo the number of parts and
  made no callable check; 2 runs each, spread at most 0.001 s; the pool's build node, 32 cores,
  load 0.3 to 1.4. CPU seconds:

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
- Under the index rule an added function moves every later place to the next part, so a unit
  that adds one function early in the list changes the parts of hundreds of others. The last
  run of the probe took a place into the part its text hashes to instead: FNV-1a, 64 bit, over
  the UTF-8 bytes of the place line, modulo the number of parts. Each part made the callable
  check of the walk on every place it resolved (one call of `Function.prototype.toString`,
  `builtins_symbol_values.rs:432-436`), and read the length of the holder before each call of
  `with` with the Symbol as argument 2 or 3. 2 runs each; load 6 to 10 on 32 cores; spread at
  most 0.02 s but for two readings at the full count (0.221 for part 0 of 2, 0.062 for part 0
  of 4), and the table gives the higher. CPU seconds of the slowest part, and the sizes:

  | parts | default | collect, gate count | collect, full count | places per part |
  |---|---|---|---|---|
  | 2 | 0.020 | 0.312 | 0.916 | 181, 163 |
  | 3 | 0.015 | 0.195 | 0.656 | 119, 104, 121 |
  | 4 | 0.014 | 0.141 | 0.398 | 87, 82, 94, 81 |
  | 5 | 0.013 | 0.117 | 0.233 | 60, 69, 75, 71, 69 |
  | 6 | 0.012 | 0.102 | 0.210 | 67, 48, 58, 52, 56, 63 |
  | 7 | 0.011 | 0.092 | 0.166 | 45, 53, 52, 46, 43, 47, 58 |
  | 8 | 0.011 | 0.077 | 0.137 | 45, 42, 45, 50, 42, 40, 49, 31 |

  The callable check costs: the slowest of 2 parts took 0.512 at the full count without it
  under the index rule; the `toString` call is a builtin call, and in the collect mode it
  collects and hands out a String per place.
- The answers of a call depend on the calls made before it in the same realm. The probe's run of
  all 344 places in one test met exactly the ten rows of `expected` at the full count and the
  three of the gate count, as the walk does. In parts, the part that holds
  `global.Array.prototype.with` can meet two rows more: `global.Array.prototype.with (argument
  2): RangeError` and `(argument 3): RangeError`. `with` with `this` %Array.prototype% and the
  index 0 throws a RangeError when the index is at least the length of %Array.prototype%
  (`#sec-array.prototype.with`, step 7), before the Symbol is read. `global.Array.prototype.push`
  stores the Symbol in %Array.prototype% when it is called with it as `this` and the Symbol as
  argument 1, and it comes 26 lines before `with` in the list. The length of %Array.prototype%
  is an own data property of an Array exotic object, so its Get through `Cx::get_property` runs
  no code and calls no builtin.
- By the hash rule, `push` and `with` fall in one part at 2 and 4 parts (both in part 1) and in
  different parts at 3, 5, 6, 7 and 8; the part of each depends on its own text only. Read
  before each call of `with` (default mode and full count, every part, every `n` from 2 to 8):
  at 2 and 4 parts the length was 6 and both calls answered normally; at 3, 5, 6, 7 and 8 it was
  0 and both threw the RangeError. In every part of every `n` the RangeError came exactly when
  the index 0 was at least the length read, and the parts met no other row than the ten rows and
  these two. At the gate count no part calls with the Symbol as argument 2 or 3. An earlier run
  under the index rule read the same: length 6 at 1 and 2 parts, 0 at 3, 4, 5 and 8.
- Callable. The walk keeps a value as a function when `Function.prototype.toString` called with
  it as `this` answers normally (`:432-436`); the method throws a TypeError for a `this` that is
  not callable (`#sec-function.prototype.tostring`, the last step). The host interface has no
  call that asks IsCallable (`host/mod.rs`; the engine's `is_callable` is not public), so that
  call is the cheapest form of the same test.
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
  same 344 places byte for byte in both builds. In the code, `git grep -nE 'feature *=
  *"collect-every-safe-point"' -- crates/swell/src` prints 14 lines in 8 files:
  `heap/collect.rs` 2, `host/agent.rs` 1, `host/jobs.rs` 1, `interp/dispatch.rs` 1,
  `object/builtin_function.rs` 3, `object/ecmascript_function.rs` 1, `ops/objects.rs` 4,
  `script.rs` 1. They are the collection at a safe point (`host/agent.rs:1048`) and counts of
  unit tests; none is under `crates/swell/src/realm/` or `crates/swell/src/builtins/`, and none
  defines a property.
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
  `RUSTFLAGS=-D warnings` and `cargo nextest run --workspace --locked --profile default`, both
  written in the task itself (`Taskfile.yml:739-758`), not in variables; the steps
  `build-collect` and `test-collect` run with `RUSTFLAGS=-D warnings` too
  (`taskfiles/collect-mode.yml`). `test-realm-sizes` hands nextest an absolute path
  (`cfg="$(pwd)/$cfg"`, `Taskfile.yml:424`), because nextest runs a test with the package's
  directory, `crates/swell`, as its working directory.
- Other stories of the store that write the same files: `story:map-upsert-and-group-by` adds
  three functions to the realm (`Map.prototype.getOrInsert`, `getOrInsertComputed`,
  `Map.groupBy`) and writes rows into `builtins_symbol_values.rs` in the form of today;
  `story:realm-size-lane-within-its-share` writes `Taskfile.yml` and `taskfiles/`, including
  `taskfiles/realm-room-tests.awk`, which decides the tests `test-realm-sizes` runs;
  `story:host-object-overrides` writes the `HostObjectSpec` literal of
  `crates/swell/tests/support/host.rs:189`, about 300 lines from the exception line at
  `:498-500`.

## Change

No file of `crates/swell/src` changes. Figures are of `158888d` (Context); the unit reads them
again on `B`. In the lines below, `<k>` is the number of a line of the file of places, counted
from 1 with its two comment lines; `<p>` is the number of a part, from 0; `<n>` is the number of
parts; `<q>` is the suffix of a part's test name, `<p> + 1`, from 2.

1. **The committed file of places.** `crates/swell/tests/fixtures/realm_places.txt`, beside
   `host_135_math.txt`; a file under `tests/fixtures/` adds no module, so `main.rs` does not
   change.
   - Lines 1 and 2 start with `#` and say what the file is and that `task realm-places-expected`
     writes it and nothing else does. Every other line is one place; no other line starts with
     `#`, no line is blank, and the file ends with a newline. It holds no digest.
   - The order of the place lines is the order of the list of `functions_of_the_realm`, the
     walk's breadth-first order. The file then equals the list line for line, so the test of
     item 3 compares two lists, and a part makes its calls in that order (item 5). The place a
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
2. **How the file stays current, and the writer.**
   - The organization's rule says an expensive product is made once and never per test, and
     names two forms. This story takes the first, a committed generated file, with its digest
     check replaced by the regeneration itself: the test of item 3 makes the list again at every
     gate run, in the default mode, at the cost of one call of 0.05 CPU seconds per run, and
     compares it with the file line for line; the parts read the file and never make the list.
     The form was put to the operator on 2026-11-27; the unit does not start before the page of
     its wave in `docs/plan/` records the answer.
   - The stop: when `the_realm_places_file_equals_the_walk` takes more than 1.2 CPU seconds alone
     in the default mode, it moves to the lane `slow` and a check of a digest of the file's
     sources takes its place in the gate, in a story of its own. That fallback has a cost: the
     sources of the list are all of `crates/swell/src`, so with a digest every change of the
     engine, not only one that adds or removes a function, runs the writer, and every two units
     of a wave meet on the digest's line at their join.
   - The writer is `realm-places-expected`, in a new `taskfiles/realm-places.yml` of POSIX sh,
     the one run that writes the file, named as `test262-expected` is. It runs `mkdir -p
     .out/realm-places`, then the gate's step `test` restricted to one test, `cargo nextest run
     {{.TEST_NEXTEST_ARGS}} -E '<filter>'`, `<filter>` being
     `test(=builtins_symbol_values::the_realm_places_file_equals_the_walk)`, with `RUSTFLAGS`
     set to `{{.TEST_RUSTFLAGS}}` and with `SWELL_REALM_PLACES_OUT` set to
     `"$(pwd)/.out/realm-places/walk.txt"`, an absolute path, since nextest runs the test in
     `crates/swell` (as `test-realm-sizes` passes `"$(pwd)/$cfg"`, `Taskfile.yml:424`).
     `TEST_RUSTFLAGS` (`-D warnings`) and `TEST_NEXTEST_ARGS` (`--workspace --locked --profile
     default`) become variables of `Taskfile.yml`, which the step `test` reads in place of the
     values it writes out today, so the writer and the step share one build and cannot drift.
     The writer first prints `realm-places-expected: RUSTFLAGS=<flags> cargo nextest run <args>
     -E <filter>` with the values it read, and exits 1 when either variable is empty.
   - The test of item 3 writes the walk's places to that path; the task writes the two comment
     lines and those places to the file and prints `realm-places-expected: <t> places written to
     crates/swell/tests/fixtures/realm_places.txt`. When the path holds no place it prints
     `realm-places-expected: the walk wrote no place to <path>` and exits 1. It compiles, so a
     unit runs it as `slot --timeout 30m task realm-places-expected`; the gate does not run it.
   - The writer gets no gate selftest. What it writes is held at the next gate run: the test of
     item 3 compares every place line with the walk, and the cover test of item 6 holds the form
     of the file (the two comment lines first, no other, no blank line, the final newline). A
     selftest of the writer would have to build and run the test binary, which no script of
     `tests/gate/` does.
3. **One test holds the file equal to the walk**, `the_realm_places_file_equals_the_walk`, in
   the default mode only (`#[cfg(not(feature = "collect-every-safe-point"))]`, as
   `proxy.rs:890` does). `functions_of_the_realm`, and every function of the file that only it
   or this test calls, carries the same `cfg`, so the collect build, which the gate makes with
   `RUSTFLAGS=-D warnings`, holds no dead code. The walk takes 0.052 CPU seconds in the default
   mode and 2.33 in the collect mode, over that mode's bound of 2 (Context).
   - In the bare realm of item 5 it runs `functions_of_the_realm` as the test does now. With
     `SWELL_REALM_PLACES_OUT` set it writes the walk's places to that path, one per line, and
     ends without comparing (the writer of item 2). Without it, the walk's places must equal the
     file's place lines in order; on a difference it fails with one line: `realm places: the walk
     reaches <w> places, the file holds <f>; missing from the file: [<places>]; not reached by
     the walk: [<places>]; first line that differs: <k> (walk <place>, file <place>); slot
     --timeout 30m task realm-places-expected writes the file again`. The lists are written as
     Rust's `{:?}` writes a list of strings; a side that has no line `<k>` is written `none`.
   - Then it resolves every line by its place (item 5) and asserts that the holder and the
     function are the walk's, else `realm places: line <k> "<place>" resolves to another holder
     or function than the walk's (holder <same|other>, function <same|other>)`; the probe found
     0 of 344 different.
   - It prints `realm places: <t> lines of crates/swell/tests/fixtures/realm_places.txt equal
     the walk`.
4. **The collect mode, and the rule for the engine's source.**
   - The list does not depend on the feature (Context: the same 344 places in both builds). In
     the collect mode every place line is resolved, and checked callable, by one part (item 5),
     so a function the collect build lacks, or a place that reaches no function there, turns its
     part red. A function that only the collect build has is not seen in that mode: no test of
     the collect build makes the walk.
   - The guard of that gap is a rule for the engine's source, which the unit checks: no code
     that defines a property of the realm depends on the feature `collect-every-safe-point`.
     The check: `git grep -cE 'feature *= *"collect-every-safe-point"' -- crates/swell/src`
     takes both spellings and prints, per file, the count of lines that use the feature; on
     `158888d` that is the list below, and the unit compares its output on `B` with it. A file
     or a count that is not in the list fails the acceptance, and the unit reads the new line
     and says whether it defines a property of the realm (then it is a defect of the change that
     added it).

     ```
     crates/swell/src/heap/collect.rs:2
     crates/swell/src/host/agent.rs:1
     crates/swell/src/host/jobs.rs:1
     crates/swell/src/interp/dispatch.rs:1
     crates/swell/src/object/builtin_function.rs:3
     crates/swell/src/object/ecmascript_function.rs:1
     crates/swell/src/ops/objects.rs:4
     crates/swell/src/script.rs:1
     ```

   - The sentence of item 8 writes the rule, the gap and its check into `AGENTS.md`.
5. **The parts.**
   - `WALK_PARTS`, a list of part numbers from 0, and the macro `walk_parts!`, whose invocation
     writes each test as `#[test]` and `fn <name> = <part>;`, so that
     `taskfiles/realm-room-tests.awk` sees each part by its name; it prints them `other`, and
     `test-realm-sizes` does not run them. Part 0 keeps the name
     `every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_error`; the others are
     `..._part_<q>`.
   - `in_a_bare_realm(f)`: the agent, the bare realm (the one call of
     `agent.create_realm(None, None)` in the file, with its comment), the further roots of
     `FURTHER_ROOTS`, and `f(cx, global, roots)` in the one entry of the realm; it answers the
     agent's state after it and the answer of `f`. The parts and the test of item 3 use it.
   - `realm_places()` reads the file at run time through `concat!(env!("CARGO_MANIFEST_DIR"),
     "/tests/fixtures/realm_places.txt")`, as `math_platform.rs:287-291` reads its file, and
     answers its place lines, lines 3 to the end, with their line numbers. `parse_place` reads
     a line into its root, its steps and its accessor suffix, or answers why it cannot.
   - `resolve_place` answers the holder and the function of a parsed place in a realm. It makes
     the walk's own callable check on what it reaches: the call of `Function.prototype.toString`
     with it as `this`, as `builtins_symbol_values.rs:432-436` makes it. That is the same test:
     the method answers for a callable `this` and throws a TypeError for any other
     (`#sec-function.prototype.tostring`, the last step), and the host interface has no call
     that asks IsCallable, so no cheaper form exists. A step that reaches no object fails with
     `walk part <p> of <n>: line <k> "<place>": <step> reached <value>, not an object`; an object
     that is not callable with `walk part <p> of <n>: line <k> "<place>": reached an object that
     is not callable`.
   - The partition. A place is in part `p` when FNV-1a, 64 bit, over the UTF-8 bytes of its
     line, modulo `WALK_PARTS.len()`, is `p`: offset basis `0xcbf29ce484222325`, prime
     `0x100000001b3`, each byte XORed in and then the product taken modulo 2^64, written out in
     the test file as `place_hash`, with no dependency. `walk_part_places(part, places)` answers
     a part's place lines in the order of the file. A function the realm gains or loses moves no
     other place to another part; only a change of the number of parts does.
   - A part reads the file, makes the bare realm and the Symbol, and resolves and checks all its
     places before its first call, since a call can change a property on a later place's path.
     Then it makes the calls the walk makes now, in the order of the file: per function the same
     four calls (`this` the Symbol; arguments 1 to 3 with `this` the holder and 0 before the
     Symbol), `take(places_per_function())` of them, the same answers read (`answer`), and the
     same early end on a host error, whose row then fails the part.
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
       gives the clause and why the length moves: `global.Array.prototype.push`, earlier in the
       file, stores the Symbol in %Array.prototype% when it runs in the same part (Context).
   - Before each call that a row with a condition names (its place, its place of the Symbol, its
     least number at most `places`), the part reads the condition from the realm: the Get of
     `length` on the call's `this`, the holder, through `Cx::get_property`, a Number converted
     as LengthOfArrayLike does (`#sec-lengthofarraylike`, ToLength). A value that is not a Number
     fails with `walk part <p> of <n>: the row <row> under IndexAtLeastLength(<i>) read length
     <value>, not a Number`.
   - Each part asserts: the agent is alive; the rows it found equal, in the order of its calls,
     the `Always` rows of its places with their least number at most `places` and the rows with
     a condition whose condition held. Each kind of difference has its line: `walk part <p> of
     <n>: a row not in WALK_EXPECTED: <row>`; `walk part <p> of <n>: the row <row> was not met`;
     `walk part <p> of <n>: the row <row> under IndexAtLeastLength(<i>) read length <v>: the
     condition is <true|false> and the call answered <answer>`. A row with a condition is thus
     met exactly when its condition holds.
   - The count of calls: the Get of a condition and the `toString` call of the callable check
     are not calls of the walk, so `<c>` and the assertion that the calls are the part's places
     times `places` count the four calls per function only, as now. The part asserts no count of
     steps. A part makes one Get per call of a row with a condition: two in the part that holds
     `with`, in the default mode and at the full count, none at the gate count, where the rows'
     least numbers, 3 and 4, are above its 2 places.
   - On `158888d` (Context): by the hash rule `push` and `with` share part 1 at 2 and 4 parts and
     lie apart at 3, 5, 6, 7 and 8. At 2 and 4 the length read before both calls of `with` was 6,
     the condition false, the answers normal; at 3, 5, 6, 7 and 8 it was 0, the condition true,
     the answers the RangeError. In every part of every `n` from 2 to 8, in the default mode and
     at the full count, a row with a condition was met exactly when its condition held, and no
     part met another row.
   - Each part prints `walk part <p> of <n>: <f> of <t> places, <c> calls`.
   - The number of parts, two rules. First, there are at least 2 parts, always: the partition,
     its cover test and the macro line that adds a part are there from the start, held by the
     gate, so that a unit that needs one more part adds one line to a cut that works, and does
     not make the cut itself under a red gate. Second, above 2: the unit measures every part,
     with the measuring command of the acceptance, in each of the three cells; `n` is the
     smallest from 2 to 8 for which the slowest part is at most four fifths of the cell's bound
     in all three cells: 1.2 CPU seconds in the default mode, 1.6 in the collect mode at its gate
     count, 2.0 in the collect mode at its full count (four fifths of 1.5, 2 and 2.5). On
     `158888d` the rules choose `n` = 2: the slowest part takes 0.020, 0.312 and 0.916; the
     parts hold 181 and 163 places. A fifth of each bound stays as room for a realm that grows;
     a function the realm gains adds its resolution, its check and its calls to the one part its
     place hashes to.
   - The stop: the walk is a defect that gets a story, and the unit adds no part and reports
     each reading, when no `n` up to 8 holds the rule, or when the unit's measure of
     `the_realm_places_file_equals_the_walk` in the default mode is over 1.2 CPU seconds (the
     stop of item 2).
6. **The cover test.** `the_parts_of_the_walk_cover_every_function_of_the_realm` reads the file
   and makes no agent, so it costs the same in every mode. It asserts, each with its line:
   - lines 1 and 2 start with `#` (`line <k> of <file> does not start with #`); no other line
     does (`line <k> of <file> starts with #; only lines 1 and 2 may`); no line is blank (`line
     <k> of <file> is blank`); the file ends with a newline (`<file> does not end with a
     newline`);
   - `WALK_PARTS` is numbered from 0 without a gap (`WALK_PARTS is <list>, not 0 to <n-1>`) and
     has at least 2 parts (`WALK_PARTS has <n> part(s), at least 2`);
   - every place line parses (`line <k> of <file>: "<line>": <reason>`);
   - no place is on two lines (`the place "<place>" is on <j> lines of <file>`);
   - the place lines the parts take by `walk_part_places`, together, are the place lines of the
     file, each once (`places of the file in no part: [...]; in more than one part: [...]`);
   - each row of `WALK_EXPECTED` names a place the file holds exactly once (`the row "<place>
     (<what>): <error>" of WALK_EXPECTED names a place the file holds <j> times`), since a part
     would otherwise drop the row unseen;
   - each row with a condition names a call `argument <i>`, whose `this` is the holder (`the row
     <row> carries a condition on a call whose this is not the holder`);
   - `%AsyncFunction.prototype%.constructor` is a place line (`the file has no line
     %AsyncFunction.prototype%.constructor`), and the file holds at least 100 places (`the file
     holds <t> places, at least 100`; the old `called >= 100 * places`, `:614`);
   - it prints `places of the realm: <t>, in <n> parts: <f0> + <f1> + ...`.
7. **When the realm gains or loses a function, and at a join.**
   - The test of item 3 is red in the gate's step `test` and names the writer. The unit runs
     `slot --timeout 30m task realm-places-expected` and commits the file; `git diff` of the file
     shows the places added or removed. A change of the engine that adds or removes no function
     leaves the file as it is. An added place lands in the part its text hashes to and moves no
     other place.
   - A new answer other than normal and TypeError gets a row: `Always` when the clause throws
     before the Symbol is read whatever earlier calls left; when it depends on earlier calls, a
     row with the condition the clause itself tests, a form of `When` added for a clause that no
     form says yet, with the clause named in the comment. Never a row without a condition that a
     part may meet or not.
   - The unit measures each part and `the_realm_places_file_equals_the_walk` in the cells of
     item 5. A part over four fifths of its bound makes it add the line `..._part_<q> = <p>` to
     `walk_parts!`; the cover test follows. Up to 8 parts; past them, or with the test of item 3
     over 1.2, the stop of item 5 holds.
   - At a join where either side branched before the other's functions, the coordinator runs
     `slot --timeout 30m task realm-places-expected` on the merged tree and commits the file in
     the join's own commit; a conflict in the file is resolved by the writer, never by hand. A
     row a unit added in the old form (a line of the `expected` of today) is rewritten at the
     join as a `WALK_EXPECTED` entry, `When::Always` unless its clause reads what earlier calls
     left. A part that meets a row not in `WALK_EXPECTED` on a merged tree is the join's to fix.
     The coordinator measures the parts once at the wave's close.
8. **The records, the files and the sentence.**
   - The doc comment of the parts drops the numbers 144, 576 and 288 and says the count is
     printed. The doc comment of `places_per_function` drops "1.8" and states the unit's
     readings: the slowest part at the chosen `n` in each cell, with the commit they were read
     on. The doc comment of `WALK_PARTS` gives, as the comment above `language_parts!` does, the
     readings that chose `n` (the slowest part at each `n` measured, each cell, the load and the
     core count) and the two rules of item 5. Neither holds the patch's placeholders
     `READINGS` and `MEASUREMENTS`.
   - `support::host::create_realm`: its line for the walk names
     `builtins_symbol_values.rs::in_a_bare_realm` and says that the parts and the test of item 3
     make their realm through it, because the file holds the places of the bare realm; its
     reason no longer says "its full count is at its bound".
   - Writes, five files: `crates/swell/tests/builtins_symbol_values.rs` (items 3 to 6, 8);
     `crates/swell/tests/fixtures/realm_places.txt` (new, written by the writer only);
     `taskfiles/realm-places.yml` (new, item 2); `Taskfile.yml` (the two variables of item 2,
     the step `test` reading them, and the include of `taskfiles/realm-places.yml` appended to
     `includes:` as one hunk); `crates/swell/tests/support/host.rs` (the doc comment line of the
     walk, and nothing else).
   - Not written by the unit: `AGENTS.md` (the coordinator, below), `crates/swell/tests/main.rs`,
     any file of `crates/swell/src`, `Cargo.lock`, a `Cargo.toml`, `.github/workflows/`,
     `tests/gate/`, `taskfiles/realm-room-tests.awk`, and the section "The gate's numbers".
   - Shared. With the units of any slice that add functions to the realm
     (`story:map-upsert-and-group-by` adds three): `builtins_symbol_values.rs` (a row, an entry of
     `FURTHER_ROOTS`) and the file of places, met at a join as item 7 says. With
     `story:realm-size-lane-within-its-share`: `Taskfile.yml` (this story's include appended as
     one hunk, its variables in their own hunks) and `taskfiles/realm-room-tests.awk`, which
     this story reads and does not write; the unit shows on `B` that the awk prints each part as
     `other`, and the writer takes its flags and profile from the variables the step `test`
     reads, not from a copy. With `story:host-object-overrides`:
     `crates/swell/tests/support/host.rs`, hunks apart (its literal at `:189`, here
     `:498-500`). The unit applies the cut on `B` with the rows and roots `B` holds.
   - The sentence for `AGENTS.md`, "Commands and the gate", in the bullet that starts "A test
     stays under 1.5 CPU seconds" (`:316`), after its sentence that ends
     `language::the_language_directory_holds_only_scripts_the_parts_run` (`:337`). The
     coordinator writes it on the merge:

     > The walk that gives every function of the realm a Symbol (the tests of
     > `builtins_symbol_values` named
     > `every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_error` and its
     > `_part_<q>`) reads its functions from `crates/swell/tests/fixtures/realm_places.txt`, one
     > place per line, and runs in parts: a place is in the part its line hashes to (FNV-1a,
     > 64 bit, modulo the length of `WALK_PARTS`). `the_realm_places_file_equals_the_walk` makes
     > the list again in the default mode at every run and fails when the file differs; a unit
     > that adds or removes a function of the realm runs `slot --timeout 30m task
     > realm-places-expected` and commits the file. No test of the collect build makes the walk,
     > so a function that only the collect build has is unseen there; the guard is that no code
     > that defines a property of the realm depends on the feature `collect-every-safe-point`,
     > checked with `git grep -cE 'feature *= *"collect-every-safe-point"' -- crates/swell/src`
     > against the files and counts the story of the walk lists. A unit that adds functions to
     > the realm measures each part with the command above, in the default mode and in the
     > collect mode, at the gate count and at the full count, and
     > `the_realm_places_file_equals_the_walk` in the default mode; a part over four fifths of
     > its bound (1.2 CPU seconds in the default mode, 1.6 in the collect mode at the gate
     > count, 2.0 at the full count) makes it add a part to `WALK_PARTS`, up to 8 parts. Past 8
     > parts, or with `the_realm_places_file_equals_the_walk` over 1.2 CPU seconds, the walk is
     > a defect that gets a story: the unit reports the readings and adds no part.

**Against the saved patch** of the unit of revision 3 (`walk-in-parts.patch`, on `158888d`).
Kept: `WALK_EXPECTED` with the field `when` added; the macro `walk_parts!`, its invocation form
and the names of part 0 and `_part_2`; the loop of `walk_part` with `break 'functions`; the name of
the cover test and its assertions of the parts' numbering and of the rows, with their messages.
Changed: `walk_part_indices`, the index rule, becomes `walk_part_places`, the hash rule;
`functions_of_a_bare_realm`, which made the list and handed it to `f`, becomes `in_a_bare_realm`,
which makes no list; `walk_part` resolves and checks its places from the file and does not walk;
the cover test reads the file and makes no agent; a part reads the condition of a row before its
call; the printed line counts places. New: the file, `realm_places`, `parse_place`,
`resolve_place`, `place_hash`, the type `When`, the test of item 3 with its writing mode, the
variables of `Taskfile.yml` and `taskfiles/realm-places.yml`. The placeholders `READINGS` and
`MEASUREMENTS` of the patch get the readings of item 8.

## Tests

- The parts of Change item 5 in place of the one test: `n` tests, part 0 under the old name, the
  others `..._part_<q>`. Each is at most four fifths of its bound in each of the three cells
  (1.2, 1.6, 2.0 CPU seconds); on `158888d` the probe read 0.020, 0.312 and 0.916 for the
  slowest of 2 parts by the hash rule, with the callable check.
- `the_realm_places_file_equals_the_walk` (Change item 3), new, in the default mode only: the walk
  and the resolution and check of every place, about 0.07 CPU seconds on `158888d`, summed from
  the probe and not read as one test (the walk 0.052; the resolution of 344 places 0.004 more
  than the bare realm without the check; the slowest of 2 parts, 181 places checked and called,
  0.020 with the realm). It is
  also what holds the file that the writer `realm-places-expected` commits (Change item 2), and
  it is the test the stop of Change items 2 and 5 measures, against 1.2.
- `the_parts_of_the_walk_cover_every_function_of_the_realm` (Change item 6), rewritten from the
  saved patch: it reads the file and makes no agent, in every mode; it holds the file's form, the
  partition and the rows of `WALK_EXPECTED`.

No script of `tests/gate/` is added, and `check` gets no step. The tests of the file other than
the walk do not change. Each test added took under 1 CPU second in the probe on `158888d`; the
unit prints the time of each in its report and names any over 5 s with its time.

## Acceptance

Commands are written as the gate's tasks write them on `158888d`. A `cargo` command takes its
slot of the pool itself; every other command that compiles or tests starts with `slot`; a command
that compiles nothing (`git`, `grep`, `cp`, `cmp`, `head`, `tail`, `od`, `awk`) starts
without it. Each pool job ends within 10 minutes unless it names its time (`SLOT_TIMEOUT=30m
cargo ...`, `slot --timeout 30m ...`, at most 30 minutes). A test is killed at 60 s, and the
gate's test steps end within 120 s. No file of the tree is edited while a pool job of the tree
runs.

Names used below: `<F>` is `crates/swell/tests/fixtures/realm_places.txt`; `<N0>` the number of
places on `B`; `<n>` the number of parts; `<p>` a part, from 0; `<q>` the suffix of a part's test
name, `<p> + 1`, from 2; `<k>` the number of a line of `<F>`, counted from 1 with its two comment
lines; a list of places is written as Rust's `{:?}` writes a list of strings,
`["global.a", "global.b"]`; `<filter>` is
`test(/^builtins_symbol_values::(every_function|the_parts_of_the_walk|the_realm_places)/)`.

The measuring command, from `AGENTS.md:316-326`. The build prints the path of the test binary,
`<bin>` below; the measure job is a shell of its own, so `times` reads the binary alone:

- build, default mode: `SLOT_TIMEOUT=30m cargo test --locked -p swell --test main --no-run`;
- build, collect mode: `SLOT_TIMEOUT=30m cargo test --locked -p swell --features
  collect-every-safe-point --test main --no-run`;
- measure: `slot sh -c 'SWELL_FULL_COUNTS=<f> <bin> --exact builtins_symbol_values::<test>
  --test-threads 1 --nocapture; times'`; the second line `times` prints is the children's user
  and system time, and their sum is the reading.

The cells are three, each read with its own `<f>` and binary:

- the default mode, `<f>` empty (its gate count is its full count), against 1.2;
- the collect mode at its gate count, `<f>` empty, against 1.6;
- the collect mode at its full count, `<f>` `1`, against 2.0.

1. **Red first**, on `B`, before any file changes. The measure job with `<test>` the old test name
   prints `functions of the realm: <N0>, calls: <4 x N0>` at the full count, and readings over
   the bound in both collect cells: over 2 at the gate count, over 2.5 at the full count (on
   `158888d`: `N0` 344, 4.07 and 6.56). The unit reports `N0` and the three readings.
2. **The file and the writer.**
   - `slot --timeout 30m task realm-places-expected` prints first `realm-places-expected:
     RUSTFLAGS=-D warnings cargo nextest run --workspace --locked --profile default -E
     test(=builtins_symbol_values::the_realm_places_file_equals_the_walk)`, the values the step
     `test` reads, and last `realm-places-expected: <N0> places written to
     crates/swell/tests/fixtures/realm_places.txt`.
   - No copy of the step's arguments: `git grep -c -e '--workspace --locked --profile default' --
     Taskfile.yml taskfiles` prints the same counts as on `B`.
   - The form: `head -2 <F> | grep -c '^#'` prints 2; `tail -n +3 <F> | grep -c '^#'` prints 0;
     `grep -c '^$' <F>` prints 0; `tail -c 1 <F> | od -An -c` prints `\n`; `grep -cv '^#' <F>`
     prints `<N0>`.
   - Byte for byte: `cp <F> .out/realm-places/first.txt`, `slot --timeout 30m task
     realm-places-expected` again, then `cmp .out/realm-places/first.txt <F>` prints nothing.
3. **The readings.** With the cut in place, on `B`:
   - the measure job of every part (part 0 by the old name, then `..._part_<q>`) in each cell
     prints `walk part <p> of <n>: <f> of <N0> places, <c> calls` and a reading at most 1.2
     (default), 1.6 (collect, gate count), 2.0 (collect, full count). The report gives `n`, the
     slowest part of each cell at each `n` measured, and the two rules of Change item 5;
   - `the_realm_places_file_equals_the_walk`, default cell: prints `realm places: <N0> lines of
     crates/swell/tests/fixtures/realm_places.txt equal the walk` and a reading at most 1.2; over
     1.2 is the stop below;
   - `the_parts_of_the_walk_cover_every_function_of_the_realm`, all three cells: prints `places
     of the realm: <N0>, in <n> parts: <f0> + ...` and a reading at most 1.2, 1.6 and 2.0.
4. **The walk's tests, in both modes and both counts.** Four commands:
   - `cargo nextest run --locked -p swell --profile default --no-capture -E '<filter>'` prints
     `<n+2> tests run: <n+2> passed`;
   - `RUSTFLAGS='-D warnings' cargo nextest run --locked -p swell --features
     collect-every-safe-point --config .cargo/collect-profile.toml --cargo-profile collect
     --profile default --no-capture -E '<filter>'` prints `<n+1> tests run: <n+1> passed` (the
     test of Change item 3 is not in that build, and the build has no warning);
   - `SLOT_TIMEOUT=30m SWELL_FULL_COUNTS=1 cargo nextest run --locked -p swell --profile default
     --no-capture -E '<filter>'` prints `<n+2> tests run: <n+2> passed`;
   - `SLOT_TIMEOUT=30m SWELL_FULL_COUNTS=1 RUSTFLAGS='-D warnings' cargo nextest run --locked -p
     swell --features collect-every-safe-point --config .cargo/collect-profile.toml
     --cargo-profile collect --profile default --no-capture -E '<filter>'` prints `<n+1> tests
     run: <n+1> passed`.
   Every printed `<t>` is `<N0>`. The `<c>` of the parts add up to `<N0> x 4` in the default mode
   and at the full count in both modes, and to `<N0> x 2` in the collect mode at its gate count;
   the report shows each sum with its printed numbers.
5. **Planted defects.** Each is a scratch edit in the unit's worktree before the commit. Before
   it, the unit copies the file it edits to `.out/realm-places/saved.<name>`; after it, it copies
   the file back and `cmp` of the two prints nothing. Each plant runs with the default command of
   item 4, unless it names the collect mode (the collect command of item 4) or a reading of the
   writer; the report quotes the red line.
   - A stale file, three ways, each red in `the_realm_places_file_equals_the_walk` with one line:
     - a place line that is not the last removed: `realm places: the walk reaches <N0> places,
       the file holds <N0-1>; missing from the file: ["<place>"]; not reached by the walk: [];
       first line that differs: <k> (walk <place>, file <next place>); slot --timeout 30m task
       realm-places-expected writes the file again`;
     - a line `global.Array.prototype.planted` added: `... the file holds <N0+1>; missing from
       the file: []; not reached by the walk: ["global.Array.prototype.planted"]; first line that
       differs: <k> (walk <place>, file global.Array.prototype.planted); ...`; with the collect
       command, the part the line hashes to fails with `walk part <p> of <n>: line <k>
       "global.Array.prototype.planted": .planted reached undefined, not an object`;
     - two place lines swapped: `... the file holds <N0>; missing from the file: []; not reached
       by the walk: []; first line that differs: <k> (walk <a>, file <b>); ...`.
     After each, `slot --timeout 30m task realm-places-expected` writes the file again, `cmp
     .out/realm-places/first.txt <F>` prints nothing, and the test passes.
   - A line `global.Math` added (an object that is not callable): with the collect command, the
     part it hashes to fails with `walk part <p> of <n>: line <k> "global.Math": reached an
     object that is not callable`.
   - `resolve_place` answering the root as the holder of every place: the test of Change item 3
     fails with `realm places: line <k> "<place>" resolves to another holder or function than
     the walk's (holder other, function same)`.
   - The write of the places to `SWELL_REALM_PLACES_OUT` taken out of the test: `slot --timeout
     30m task realm-places-expected` prints `realm-places-expected: the walk wrote no place to
     <path>` and exits 1.
   - The form of `<F>`, four ways, each red in the cover test: the `#` of line 1 removed (`line 1
     of <F> does not start with #`); a line `# planted` after line 5 (`line 6 of <F> starts with
     #; only lines 1 and 2 may`); an empty line after line 5 (`line 6 of <F> is blank`); the last
     byte removed (`<F> does not end with a newline`).
   - The macro line `_part_2 = 1` written `= 2`: the cover test fails with `WALK_PARTS is [0, 2],
     not 0 to 1`, when `n` is 2; at another `n` the last part's number is raised by one, with the
     same line for that list. Every macro line after part 0 removed: `WALK_PARTS has 1 part(s),
     at least 2`.
   - `% WALK_PARTS.len()` written `% (WALK_PARTS.len() + 1)` in `walk_part_places`: the cover
     test fails with `places of the file in no part: [...]`.
   - The least number of places 100 in the cover test written `<N0+1>`: it fails with `the file
     holds <N0> places, at least <N0+1>`.
   - The place of one row of `WALK_EXPECTED` changed by one character: the cover test fails with
     `the row "<place> (<what>): <error>" of WALK_EXPECTED names a place the file holds 0 times`.
   - One place line doubled: the cover test fails with `the place "<place>" is on 2 lines of <F>`.
   - One place line with its root `global` written `globl`: the cover test fails with `line <k>
     of <F>: "<line>": <reason>`.
   - The row `global.Array.prototype.with`, `argument 2` written on the call `this` with its
     condition kept: the cover test fails with `the row <row> carries a condition on a call whose
     this is not the holder`.
   - The `Always` row `global.RegExp`, `argument 3` removed from `WALK_EXPECTED`: the part that
     owns `global.RegExp` fails with `walk part <p> of <n>: a row not in WALK_EXPECTED:
     global.RegExp (argument 3): SyntaxError`.
   - The `with` rows at 2 parts. `global.Array.prototype.push` and `global.Array.prototype.with`
     share a part at 2 parts by the hash of their texts alone (part 1; FNV-1a of the two texts,
     `74ec0189a9f1aa79` and `b638adb2dba7d26d`, both odd), so `push`, earlier in the file,
     always runs before `with` there. When `n` on `B` is not 2, each plant of this group also
     removes the macro lines after `_part_2`, for this edit only. Three plants:
     - the row `with`, `argument 2` made `When::Always`: part 1 fails with `walk part 1 of 2: the
       row global.Array.prototype.with (argument 2): RangeError was not met`;
     - its condition made `IndexAtLeastLength(<l>)`, `<l>` the length part 1 reads before the
       call (6 on `158888d`): part 1 fails with `walk part 1 of 2: the row
       global.Array.prototype.with (argument 2): RangeError under IndexAtLeastLength(<l>) read
       length <l>: the condition is true and the call answered normal`;
     - the Get of the condition made on the key `constructor` in place of `length`: part 1 fails
       with `walk part 1 of 2: the row global.Array.prototype.with (argument 2): RangeError under
       IndexAtLeastLength(0) read length <value>, not a Number`.
   - The `with` row met when its condition holds: at 3 parts (macro line `_part_3 = 2` added for
     this edit when `n` is 2), where `push` and `with` lie in parts 0 and 1, the row `with`,
     `argument 2` removed from `WALK_EXPECTED`: the part of `with` fails with `walk part 1 of 3: a
     row not in WALK_EXPECTED: global.Array.prototype.with (argument 2): RangeError`.
6. **The padding does not move it.** `SWELL_TEST_REALM_FUNCTIONS=200 cargo nextest run --locked
   -p swell --profile default --no-capture -E '<filter>'` prints the same `<N0>` on every line
   and `<n+2> tests run: <n+2> passed`.
7. **The records are true.**
   - `git grep -n '144 functions\|576 calls\|288 at\|1\.8 CPU\|READINGS\|MEASUREMENTS' --
     crates/swell/tests/builtins_symbol_values.rs` prints nothing.
   - `git grep -c '<b>' -- crates/swell/tests/builtins_symbol_values.rs`, `<b>` the short hash of
     the commit the readings were read on, prints a count of at least 2 (the doc comments of
     `WALK_PARTS` and `places_per_function`).
   - `git grep -c '\.create_realm(' -- crates/swell/tests/builtins_symbol_values.rs` prints
     `crates/swell/tests/builtins_symbol_values.rs:1`.
   - `git grep -c 'builtins_symbol_values.rs::in_a_bare_realm' --
     crates/swell/tests/support/host.rs` prints `crates/swell/tests/support/host.rs:1`.
   - `git grep -c 'every_function_of_the_realm\|at its bound' --
     crates/swell/tests/support/host.rs` prints nothing.
8. **The rule for the engine's source** (Change item 4). `git grep -cE 'feature *=
   *"collect-every-safe-point"' -- crates/swell/src` prints exactly the eight lines of the list
   in Change item 4, in its order. A line that differs stops the unit until it has read the new
   use and reported whether it defines a property of the realm.
9. **The gate.** `slot --timeout 30m task check` exits 0, with its steps `build-collect` and
   `test-collect` built under `RUSTFLAGS=-D warnings` (the `cfg` of Change item 3). The unit
   reports the lines `gate-time test-steps <s> of 120, cpu <c>, load <l> on <cores> cores` and
   `gate-cpu test-steps ...` on `B` and after, and the CPU seconds of the steps `test` and
   `test-collect` before and after (the step `test` adds the test of Change item 3 and the cover
   test; `test-collect`, in the profile `collect`, drops the old test's 0.384 for the parts).
   When the gate is red on its CPU line or its time line, the unit stops, changes no bound
   (`CPU_BOUNDS_BY_CORES`, `AGENTS.md:278-283`), and reports the two readings with `n`.
10. **The files.**
    - On the unit's commit, `git diff --name-only B HEAD -- . ':!.engineering'` prints exactly
      the five files of Change item 8.
    - `git diff B HEAD -- Taskfile.yml` shows the two variables, the lines of the step `test`
      that read them, and the include appended to `includes:`, and nothing else.
    - `sh taskfiles/test-main.sh check` exits 0 and prints its last line `test-main: <c> crate(s)
      with tests, each with one integration-test binary and its tests/main.rs as generated`, `<c>`
      the number it prints on `B`.
    - `awk -f taskfiles/realm-room-tests.awk crates/swell/tests/support/*.rs
      crates/swell/tests/builtins_symbol_values.rs | grep 'every_function\|the_realm_places'`,
      the usage the awk's header gives, prints on `B` the old test as `other
      builtins_symbol_values::<name>`, and after the change each part and the test of Change
      item 3 as `other`, one line each.
11. `cargo fmt --all --check`, `slot task clippy` and `slot task forbid-unsafe-check` exit 0.

**The stop.** The unit stops before it commits, adds no part, and reports every reading of items
1 to 3 with `n`, when no `n` from 2 to 8 holds the rule of Change item 5; when
`the_realm_places_file_equals_the_walk` takes more than 1.2 CPU seconds in the default mode; when
a place of the file does not resolve to the walk's holder and function in the default mode; or
when a part at any `n` it measured, 6 and 7 among them when it measures them, meets a row that is
neither in `WALK_EXPECTED` nor met by a clause it can name as a form of `When`, reported with its
row. The walk is then a defect that gets a story.

## Not in this story

- Any file of `crates/swell/src`; a step count, a byte count or a ceiling.
- The two tests that stay over the bound (`determinism::host_133_...`, `language` part 3), and
  every other test of the file.
- A padded realm for the walk: the padding is host functions that answer `undefined`, which the
  walk would call for nothing, and the file would then depend on `SWELL_TEST_REALM_FUNCTIONS`.
- A digest of the file's sources and a gate check of it. The test of Change item 3 makes the
  list again at every gate run for 0.05 CPU seconds; a digest comes with its own story only at
  the stop of Change item 2.
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
- The section "The gate's numbers" of `AGENTS.md`, which the coordinator records at a close, and
  the `scope:` and `depends_on` entries of the front matter, which the coordinator writes.
- The text of `story:map-upsert-and-group-by`, `story:realm-size-lane-within-its-share` and
  `story:host-object-overrides`; how this story meets them is in Change item 7.

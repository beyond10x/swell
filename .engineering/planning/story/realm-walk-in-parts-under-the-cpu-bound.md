---
format: aep.planning-md/3
id: story:realm-walk-in-parts-under-the-cpu-bound
kind: story
status: active
title: The walk over every function of the realm runs in parts, each under its CPU bound, and a test holds that the parts cover the list
summary: The test that gives every function of the realm a Symbol is cut into 2 to 8 parts by the index of a function in the walk's list; a cover test holds the partition and the rows of expected; the number of parts follows a rule with a stop, stated in the test file and in AGENTS.md.
relations:
- decomposes: epic:slice-04-numbers-and-dates
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T18:44:28Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T18:44:28Z", actor: "human:operator", revision: 3}
---
Waits for: nothing recorded. Base: the wave tip that holds the unit of
`story:tests-hold-at-any-size-of-the-realm` (`e51191a` on this tree) and every unit of slice 4
joined by then; the brief names that commit, `B` below. The walk that gives every function of the
realm a Symbol, `builtins_symbol_values::every_function_of_the_realm_given_a_symbol_answers_or_
throws_a_type_error`, is over the bound of a test at its full count with a collection at every
safe point, and the realm grows with every unit that adds builtins: the units of slice 4 now
being joined each add functions. Consumer: every unit of slice 4 and later that adds functions
to the realm (it measures the walk before it reports), and the lane `slow`, which runs this test
at its full count. The folder rule: a test over its budget is a defect, gets a story and is
fixed before the next feature; a higher bound is not a fix. This story cuts
the walk into parts, as `language` and `fuzz` are cut, so that no part is near the bound and a
realm that grows has room.

## Context

Lines are of `bef60f5`, the tip of this tree. The unit measures each count again on `B`.

- The walk is one test, `builtins_symbol_values.rs:491-615`. `functions_of_the_realm` (`:351-446`)
  builds the list: a breadth-first walk from the global object and from `FURTHER_ROOTS` (`:332`)
  over String and Symbol keys, each function once, with its place (a path string) and its holder.
  It runs no script. A place names one function: the list drops a value seen before (`:431`).
- The test makes its realm with `agent.create_realm(None, None)` (`:496`), not the padded helper,
  and says so (`:494-495`). `crates/swell/tests/support/host.rs:367-369` lists it as an exception
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
  calls: <c>` (`:559`).
- Its doc comment names 144 functions, 576 calls and 288 at the gate count (`:483-486`); that was
  the base of `story:symbol-values-in-core-builtins`. The doc comment of `places_per_function`
  says "1.8 CPU seconds" for all four places in collect mode (`:468-469`).
- Its time at the full count (`SWELL_FULL_COUNTS=1`, feature `collect-every-safe-point`, dev
  profile, user plus system): 3.243 CPU seconds on the base of the unit of
  `story:tests-hold-at-any-size-of-the-realm` and 3.081 after it, at 232 functions; 3.577 with
  the realm padded by 200 functions. The source of these four numbers is the report of that unit,
  not in the tree; its commit `e51191a` records 3.24 and 3.08. From
  `docs/plan/slice-10-wave-1.md`, "Findings left open" (`:326`): 2.71 in the unit of
  `Array.fromAsync`, and 3.54 at a machine load of 65 in the unit of `Date`, which reached 217
  functions from 196. No record holds the test's time at its gate count or in the default mode.
- Where the old name is run or written outside the file (`git grep -n
  'every_function_of_the_realm_given_a_symbol' -- .
  ':!crates/swell/tests/builtins_symbol_values.rs'` prints three lines): `crates/swell/tests/support/host.rs:367`, the exception line, which the
  unit rewrites (Change, item 6); `.engineering/planning/story/tests-hold-at-any-size-of-the-
  realm.md:172`, an acceptance command that runs the test by `test(=...)`, which after the cut
  runs part 0 only (part 0 keeps the name); `docs/plan/slice-10-wave-1.md:326`, a record of a
  past wave, which stays as it is. A unit that runs the whole walk uses the filter of
  acceptance item 3.
- The bound. `AGENTS.md:295-297`: a test stays under 1.5 CPU seconds alone in the default mode,
  under 2 with a collection at every safe point, under 2.5 at its full count in either mode,
  measured with the test binary run with `--exact <name> --test-threads 1` in a shell of its own
  whose `times` gives user plus system, one job of the pool (`:295-304`). The paragraph names two
  tests that stay over the bound (`:310-315`); this walk is not one of them, and no sentence of
  `AGENTS.md` names this test or its time (`grep -n 'symbol_values\|walk' AGENTS.md` prints
  nothing).
- How the repository cuts a heavy test into parts:
  - `language.rs`: `language_part` (`:105-111`) takes the scripts whose index in the sorted list
    leaves `part` when divided by the number of parts, so that a new script lands in a part with
    nothing to edit; the macro `language_parts!` (`:177-229`) writes one `#[test]` per part and
    the list `LANGUAGE_PARTS`; the first part keeps the old test name, the others are named
    `..._part_<n>` from 2; `the_parts_cover_every_script` (`:231-251`) asserts the parts are
    numbered from 0 without a gap, hold every script once, and prints the total. The comment
    above the macro (`:195-214`) gives the measurements that chose the number.
  - `fuzz.rs`: `PARTS` (`:2223`), `part_seeds` (`:2226`), tests `..._part_<n>` (`:2251-2310`),
    `the_parts_cover_every_sequence` (`:2356-2366`).
- `crates/swell/tests/main.rs` is generated from the files of `tests/` (`taskfiles/test-main.sh`,
  function `modules`, one `mod <stem>;` per file). Parts added inside an existing file add no
  module, so `main.rs` does not change.
- A test is killed at 60 s (`.config/nextest.toml`, `slow-timeout`). The gate's test steps fail
  over 120 s of wall time and over the CPU entry of `CPU_BOUNDS_BY_CORES`, now `16:245`
  (`AGENTS.md:252`); an entry changes only with a measured pair (`AGENTS.md:259`).

## Change

All of it in the test file and in a doc comment of `support/host.rs`; no file of
`crates/swell/src` changes.

1. **One function makes the list.** A function of `builtins_symbol_values.rs` makes the agent, the
   bare realm (the one call of `agent.create_realm(None, None)` in the file, with its comment),
   the roots of `FURTHER_ROOTS` and the list from `functions_of_the_realm`, and answers the list
   with the agent and the realm. The parts and the cover test of item 3 call it, so the exception
   of `support::host::create_realm` stays one line.
2. **The walk in parts.** A constant `WALK_PARTS`, a list of part numbers from 0, and a macro
   that writes one `#[test]` per part, in the form of `language_parts!`. The part 0 keeps the name
   `every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_error`; the others are
   `..._part_<n>`, `<n>` from 2.
   - A function is in part `p` when its index in the list, counted from 0, leaves `p` when
     divided by `WALK_PARTS.len()`. One function of the file answers the indices of a part from
     the length of the list; the parts and the cover test both call it.
   - A part makes the list (item 1), then calls each of its functions in each of `places` places,
     as the test does now, with the same four calls, the same answers read, the same early end on
     a host error.
   - Each part asserts: the agent is alive; `found` equals the rows of `expected` whose function
     is one of the part's functions, in list order, each kept when its least number of places is
     at most `places`; the number of calls equals the part's functions times `places`.
   - Each part prints `walk part <p> of <n>: <f> of <t> functions, <c> calls`, `<t>` the length of
     the list.
   - `expected` becomes a constant of the file. A row names its function by the place the list
     gives, so that a function that moves to another index, or another part, takes its row along.
     The ten rows, their texts and their numbers of places stay as they are.
3. **The parts cover the list, held by a test.** `the_parts_of_the_walk_cover_every_function_of_
   the_realm`, in the same file, makes the list once and calls no function of it:
   - `WALK_PARTS` is numbered from 0 without a gap and has at least 2 parts;
   - the indices the parts take, taken together, are the indices of the list, each once;
   - each row of `expected` names a place that is in the list exactly once (a row whose function
     left the realm fails here, since a part would drop it unseen);
   - `%AsyncFunction.prototype%.constructor` is in the list, and the list holds at least 100
     functions (the file asserts `called >= 100 * places`, which is 100 functions at any
     `places`);
   - it prints `functions of the realm: <t>, in <n> parts: <f0> + <f1> + ...`.
4. **The number of parts, by a rule.** The unit measures, on `B`, with the measuring command of
   the acceptance, in each of its three cells, the time of the cover test alone (the list, `W`)
   and of the old test (the list and the calls, `W + C`).
   - The number of parts `n` is the smallest from 2 to 8 for which the slowest part is at most
     four fifths of that cell's bound in every one of the three cells: 1.2 CPU seconds in the
     default mode, 1.6 in the collect mode at its gate count, 2.0 in the collect mode at its full
     count. The old test at the full count alone took 3.243 CPU seconds on its base (Context),
     so `n` is at least 2.
   - A fifth of each bound is the room for a realm that grows. A function the realm gains adds
     its list step to every part, since every part makes the whole list, and its calls to one
     part, the part its index falls in. So growth raises `W` in all `n` parts and `C` in one.
   - Every part pays `W`, so a cell is reachable only while `W` is well under its bound. The
     unit stops before it cuts the walk, reports `W` and `C` in each cell, and this story is cut
     again (the list made once, outside the parts), in either of these cases: `W` is over half
     of four fifths of a cell's bound in any cell, that is over 0.6 in the default mode, over 0.8
     in the collect mode at its gate count, over 1.0 in the collect mode at its full count; or no
     `n` up to 8 reaches the rule.
   - Why the list is made once per part and not shared: each part is a test in a process of its
     own under cargo-nextest, and the list holds handles of one agent (`ObjectRef`s of one
     realm), which no other process can use. The halves above are what counts as cheap enough
     for a list made once per part: at or under them the list takes at most half of what a
     part may take, and the other half is for the calls, which the parts share.
5. **The line that adds a part, and where it stops.** The doc comment of `WALK_PARTS` gives, as
   the comment above `language_parts!` does, the measurements that chose `n` (`W`, `C`, the
   slowest part in each cell, the load and the core count of the machine), and says: a part
   over four fifths of its bound in the measuring command of `AGENTS.md` makes the unit that
   sees it add the line `..._part_<n+1> = <n>` to the macro, and the cover test follows, but
   only up to 8 parts and only while `W` is under the three halves of item 4 (0.6, 0.8, 1.0).
   Past either, the walk is a defect that gets a story: the unit reports `W`, `C`, `n` and the
   readings, and adds no part. The rule is a sentence for the unit, not a test: CPU seconds on
   the forge's 4 cores and on this machine differ, and the gate's `gate-cpu` line holds their
   sum.
6. **What stays of the exception.** The walk stays one of the realms that stay bare, in every part
   and in the cover test, now through the one call of item 1. The reason is the one of the
   exception today: each part asserts over the functions of the realm and the cover test prints
   their count, and a realm padded by `SWELL_TEST_REALM_FUNCTIONS` would add the padding's
   functions, reached under a Symbol from the global object, to the list and make `<t>` depend on
   the variable. So `<t>` is the realm's at any k. The unit edits the doc comment of
   `support::host::create_realm`: the line of the walk names the helper of item 1 and says the
   parts and the cover test make their realm through it, and its reason no longer says "its full
   count is at its bound" (each part is under it).
7. **The doc comments are true again.** The doc comment of the test (now of the parts) drops the
   numbers 144, 576 and 288 and says the count is printed. The doc comment of
   `places_per_function` drops "1.8" and states the unit's readings: the CPU seconds of the
   whole walk at the gate count and at the full count in the collect mode and in the default
   mode (the sum of its parts, and the slowest part), with the commit they were read on.
8. **The sentence for `AGENTS.md`**, "Commands and the gate", after the sentence that names the
   two tests over the bound (`:310-315`). The coordinator writes it on the merge; the text:

   > The walk that gives every function of the realm a Symbol
   > (the test of `builtins_symbol_values` named
   > `every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_error`
   > and its `_part_<n>`) runs in parts: a function is in the part of its index in the walk's list
   > modulo the length of `WALK_PARTS`. A unit that adds functions to the realm measures each part
   > with the command above, in the default mode and in the collect mode, at the gate count and at
   > the full count; a part over four fifths of its bound (1.2 CPU seconds in the default mode,
   > 1.6 in the collect mode at the gate count, 2.0 at the full count) makes it add a part to
   > `WALK_PARTS`, but only up to 8 parts and only while the list alone (the test
   > `the_parts_of_the_walk_cover_every_function_of_the_realm`) stays under 0.6, 0.8 and 1.0 CPU
   > seconds in those three cells. Past either limit the walk is a defect that gets a story: the
   > unit reports the readings and adds no part. The walk makes its realm bare, as the exception
   > of `create_realm` says.

   No sentence of `AGENTS.md` names the old test or its time (Context), so nothing there is
   corrected, only added.

Writes:

- `crates/swell/tests/builtins_symbol_values.rs`: items 1 to 5 and 7;
- `crates/swell/tests/support/host.rs`: the doc comment of `create_realm`, the line of the walk
  (item 6), and nothing else.

Not written by the unit: `AGENTS.md` (item 8, the coordinator), `crates/swell/tests/main.rs` (no
module is added), any file of `crates/swell/src`. Shared with the units of slice 4 that add
functions: `builtins_symbol_values.rs`, where such a unit adds a row of `expected` only if a
clause throws before a Symbol is read, and `FURTHER_ROOTS` entries. The unit applies this story's
cut on `B`, with the rows and roots `B` holds; a row a later join adds names its function by
place, as item 2 says.

## Tests

One new test, the cover test of item 3, and the parts of item 2 in place of the one test. The
tests of the file other than the walk do not change. The cover test is under 1.5 CPU seconds in
the default mode and under 2 with a collection at every safe point, as `AGENTS.md` says; the
unit prints its time (`W`) in the report.

## Acceptance

Each command is one job of the pool and ends within 10 minutes unless it names its time. A test
is killed at 60 s and the gate's test steps end within 120 s.

The measuring command, from `AGENTS.md:295-304`. The build prints the path of the test binary,
`<bin>` below; the measure job is a shell of its own, so `times` reads the binary alone:

- build, default mode: `SLOT_TIMEOUT=30m cargo test --locked -p swell --test main --no-run`;
- build, collect mode: `SLOT_TIMEOUT=30m cargo test --locked -p swell --features
  collect-every-safe-point --test main --no-run`;
- measure: `slot sh -c 'SWELL_FULL_COUNTS=<f> <bin> --exact builtins_symbol_values::<test>
  --test-threads 1 --nocapture; times'`; the second line `times` prints is the children's user
  and system time, and their sum is the reading. No second job of the same tree runs while one
  builds.

The cells are three, each read with its own `<f>` and binary:

- the default mode, `<f>` empty (its gate count is its full count, Context), against 1.2;
- the collect mode at its gate count, `<f>` empty, against 1.6;
- the collect mode at its full count, `<f>` `1`, against 2.0.

1. **Red first**, on `B`, before the test file changes. In the collect mode at its full count,
   the measure job with `<test>` the old test name prints `functions of the realm: <N0>,
   calls: <4 x N0>` and a reading over 2.5; the unit reports `N0` (232 on this tree, measured
   again on `B`) and the reading (3.243 on the base of the realm-size unit). The unit reads the
   other two cells of the old test as well and reports the three readings, so that the story
   can state the gate count's time (Context has none).
2. **The list alone.** With item 1 and the cover test written, and `WALK_PARTS` provisionally
   two parts (the walk not yet cut to its `n`), the measure job with `<test>`
   `the_parts_of_the_walk_cover_every_function_of_the_realm` in the three cells prints
   `functions of the realm: <N0>, in 2 parts: <f0> + <f1>` and the unit reads `W` in each cell.
   If `W` is over 0.6, 0.8 or 1.0 in its cell, the unit stops before it cuts, reports `W` and
   `C` in each cell, and does not go on (Change, item 4).
3. **After.** On `B` with the cut in place:
   - the measure job for every part (the name of part 0, then `..._part_<n>`) in each of the
     three cells prints `walk part <p> of <n>: <f> of <t> functions, <c> calls` and a reading at
     most 1.2 (default mode), 1.6 (collect, gate count), 2.0 (collect, full count); the slowest
     part of each cell and the number of parts `n` (from 2 to 8; the smallest that holds the rule
     in all three cells) are in the report, with `W`, `C` and the rule of Change, item 4; if no
     `n` up to 8 holds it, the unit stops and reports, as item 2 says;
   - `slot cargo nextest run --locked -p swell --profile default --no-capture -E
     'test(/^builtins_symbol_values::(every_function_of_the_realm|the_parts_of_the_walk)/)'`
     and the same with `--features collect-every-safe-point --config .cargo/collect-profile.toml
     --cargo-profile collect` print `<n+1> tests run: <n+1> passed`; the printed `<t>` is the same
     on every line and equals the `<N0>` of item 1 (the cut adds no function to the realm);
     the `<c>` of the parts add up to `<t> x 2` in the collect mode and `<t> x 4` in the
     default mode, which the unit shows with the sum of the printed numbers;
   - the same two commands with `SWELL_FULL_COUNTS=1`, run as `slot --timeout 30m sh -c
     'SWELL_FULL_COUNTS=1 cargo nextest run <the arguments above>'`: the `<c>` of the parts add
     up to `<t> x 4` in both modes.
4. **Nothing is lost, shown by a break.** Each of the two scratch edits below, made in the unit's
   worktree, run, and undone before the commit, turns the cover test red, naming the cause;
   the report quotes each failure line.
   - The index rule changed so that one index is in no part (`% WALK_PARTS.len()` replaced by
     `% (WALK_PARTS.len() + 1)` for the selection of the part): the cover test fails on the
     indices of the parts against the indices of the list.
   - The place of one row of `expected` changed by one character: the cover test fails naming
     the row whose place is not in the list.
5. **The padding does not move it.** `slot sh -c 'SWELL_TEST_REALM_FUNCTIONS=200 cargo nextest
   run --locked -p swell --profile default --no-capture -E
   "test(/^builtins_symbol_values::(every_function_of_the_realm|the_parts_of_the_walk)/)"'`
   prints the same `<t>` as without the variable and `<n+1> tests run: <n+1> passed`.
6. **The records are true.** `git grep -n '144 functions\|576 calls\|288 at\|1\.8 CPU'
   crates/swell/tests/builtins_symbol_values.rs` prints nothing. `git grep -c '\.create_realm('
   -- crates/swell/tests/builtins_symbol_values.rs` prints `crates/swell/tests/
   builtins_symbol_values.rs:1`. `git grep -n 'builtins_symbol_values.rs::' --
   crates/swell/tests/support/host.rs` prints one line, the doc comment's line for the walk,
   `builtins_symbol_values.rs::<helper>` with the helper of Change item 1, and no line of that
   file names `every_function_of_the_realm`.
7. **The gate.** `slot --timeout 30m task check` exits 0. The unit reports the lines
   `gate-time test-steps <s> of 120, cpu <c>, load <l> on <n> cores` and `gate-cpu test-steps
   ...` on `B` and after, and the CPU the cut adds to each test step (every part pays the list
   once: about `n - 1` more lists in each mode). When the gate is red on its CPU line or its
   time line, the unit stops, changes no bound (`CPU_BOUNDS_BY_CORES`, `AGENTS.md:259`), and
   reports the two readings, on `B` and after the cut, with `n`, `W` and `C`; it does not
   commit to a smaller `n` that breaks the rule of Change, item 4. No test is over 60 s, and the
   unit names every test it adds that is over 5 s with its time.
8. `git diff --stat B -- crates/swell/src` prints nothing; `git diff --stat B -- Cargo.lock
   '*/Cargo.toml'` prints nothing; `sh taskfiles/test-main.sh check` exits 0 and prints
   its last line `test-main: <k> crate(s) with tests, each with one integration-test binary and
   its tests/main.rs as generated`, `<k>` the number it prints on `B`; `git diff --stat B --
   crates/swell/tests/main.rs` prints nothing.
9. `slot cargo fmt --all --check`, `slot task clippy` and `slot task forbid-unsafe-check` exit 0.

## Not in this story

- Any file of `crates/swell/src`; a step count, a byte count or a ceiling.
- The two tests that stay over the bound (`determinism::host_133_...`, `language` part 3), and
  every other test of the file.
- A padded realm for the walk: the padding is host functions that answer `undefined`, which the
  walk would call for nothing, and the count would move with `k` (Change, item 6).
- A list of the walk's functions held in a committed file with a digest, or built once by a setup
  step, which would take the list step out of every part. It is the cut this story falls back to
  only in the two cases of Change, item 4 (`W` over a half of four fifths of a bound, or no `n`
  up to 8).
- A test that asserts CPU seconds of a part, and a change of `CPU_BOUNDS_BY_CORES`.
- Fewer calls per function in the collect mode (the gate count of `places_per_function` stays,
  2 places there, 4 in the default mode and at the full count).

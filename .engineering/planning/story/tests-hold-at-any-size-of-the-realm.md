---
format: aep.planning-md/3
id: story:tests-hold-at-any-size-of-the-realm
kind: story
status: active
title: 'No test of swell turns on the size of the realm: each takes its room from its own readings, and a realm padded by k functions shows it'
summary: The tests' support code pads every realm of a test by k host functions (SWELL_TEST_REALM_FUNCTIONS); every test red for the realm's size takes its room from readings of its own agents; the task test-realm-sizes runs swell in both modes at k up to 200 in the lane slow and before a unit that adds to the realm reports.
relations:
- decomposes: epic:slice-04-numbers-and-dates
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T15:13:19Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T15:13:19Z", actor: "human:operator", revision: 3}
---
Waits for: `story:math-constants-and-exact-functions` (its changes of `termination.rs`,
`determinism.rs`, `builtins_array_from_async.rs` and `async_functions.rs`). Base: the wave tip
that holds the `Math` unit; the brief names that commit, `B` below. Slice 4 adds about 150
functions to every realm; each unit that adds some has found tests outside its story red, because
a test compares with a room or a `heap_bytes()` reading fitted to one size of the realm. Eight are
known, found one unit at a time. This story fixes every test of the kind and adds the instrument
that finds the next one before a unit reports.

## Context

Line numbers are of `1e7092d`, the wave tip before the `Math` unit.

- How a test gets its realm. `support::host::realm` (`crates/swell/tests/support/host.rs:129`)
  calls `create_realm(None, None)`; `agent` and `agent_with` (`:119`, `:124`) make the agent.
  Outside `support/`, 23 test files call `.create_realm(` on 45 lines (`git grep -n
  '\.create_realm(' -- 'crates/swell/tests/*.rs' ':!crates/swell/tests/support' | wc -l`; `-l`
  gives 23). 13 test files have a `Host` of their own (`git grep -n 'impl Host for' --
  crates/swell/tests`: 14 lines, `TestHost` among them). `TestHost` derives `Default`
  (`host.rs:17`); the unit of the `Date` getters replaces that by a hand-written impl.
- A host function is made by the same CreateBuiltinFunction as a builtin:
  `create_host_function` (`crates/swell/src/object/builtin_function.rs:492`) calls
  `create_builtin_function` (`:444`), which makes the function's record, its name String and its
  `length` and `name` properties; the host function adds its boxed steps. The ceilings of one are
  `HOST_FUNCTION_LIVE_BYTES` 576 and `PROPERTY_LIVE_BYTES` 448 per definition
  (`crates/swell/src/host/limits.rs:78`, `:89`).
- An arena is charged by its capacity and grows by doubling (`crates/swell/src/heap/arena.rs:21`,
  `growth` `:218`, `grow` `:241`): where a doubling falls moves with every record the realm holds.
- Of the 86 test files of `crates/swell/tests` (`ls crates/swell/tests/*.rs | grep -vc main.rs`),
  with the pathspec `-- 'crates/swell/tests/*.rs' ':!crates/swell/tests/support'` on each: 45
  files set `memory_bytes` at 65 sites (`git grep -cE 'memory_bytes *[:=]'`, `-l` for files); 50
  files call `heap_bytes()` at 297 sites (`git grep -c 'heap_bytes()'`); 53 do one or both
  (`git grep -lE 'memory_bytes *[:=]|heap_bytes\(\)' | wc -l`).
- The eight known, with the size each was seen at. With 11 functions more (`Math`), collect
  mode: `termination.rs:157`, `builtins_array_from_async.rs:898`, `determinism.rs:551`. With 4
  more (`Number.prototype`), collect mode: the first and third and `async_functions.rs:691`.
  With the `Date` getters (+17 on a realm of 217 functions): default mode
  `generators.rs:303` (`collect()` raised `heap_bytes()` from 299,661 to 322,860); collect mode
  `builtins_symbol.rs:412` ("13 registrations of 3617 bytes fit 51374 bytes past the realm"),
  `block_functions.rs:275` and `lexical_declarations.rs:401` (`Terminated(MemoryBudget)`), and
  the second and third of the first list.
- The `Math` unit fixes the first four in two forms: a second round, once the first left each
  arena at the capacity the round needs (`async_functions.rs`); and a probe or twin agent that
  finds where the arena of objects grows next (`next_object_doubling` in `termination.rs`) or
  what a script leaves (`builtins_array_from_async.rs`), the budget taken from that reading. Its
  report: with its fix `determinism::host_133_same_calls_same_hooks_same_outcomes` costs 6.4 CPU
  seconds with a collection at every safe point (3.4 before), `termination::an_agent_that_ended_in_memory_budget_runs_again_after_a_collection`
  1.10 (0.10 before). The other four already read their own agents, but put a budget or a
  comparison across a point where an arena doubles (`block_functions.rs:310`,
  `lexical_declarations.rs:415`: a budget halfway between two readings).
- `host_objects::host_187_a_realm_of_300_host_functions_and_40_host_objects`
  (`host_objects.rs:1417`; its realm at `:1192`, `heap_before` read after it) compares a rise
  over the realm with `BINDINGS_REALM_HEAP_BYTES` 506,112 (`limits.rs:95`), a ceiling the host
  interface states. Units read 449,821 before `Math`, 491,805 after it, and 437,533 on the
  `Date` getters' base.
- A realm grown by builtins, from the units' reports (`host_187`, "after the build", first
  reading): `Number.prototype` +4 functions +1,258 bytes; `Date` getters +17, 715,237 to
  722,945 (+7,708); `Math` +11 functions and 8 constants, 663,981 to 711,712 (+47,731, a
  doubling among them). The Symbol walk prints the realm's functions
  (`builtins_symbol_values.rs:555`): 196 before `Math`, 207 with it, 217 to 234 with the getters.
- The Symbol walk (`functions_of_the_realm`, `builtins_symbol_values.rs:351`) follows the String
  and the Symbol keys of every object it reaches from the global. No other test reads the Symbol
  keys of the global: `builtins_object_enumeration.rs:575-640` reads String keys only, and no
  test calls `Reflect.ownKeys` (`git grep -nE 'getOwnPropertySymbols|Reflect\.ownKeys' --
  crates/swell/tests`: 27 lines in 5 files, none on the global).
- `nextest-report` exits 1 on a red run (`taskfiles/nextest.yml:196`, `:224`); `compile-check`
  checks nothing when `BUILT_BY` is empty (`:245`), and the steps of `slow` pass none.
- A variable read by tests exists: `SWELL_FULL_COUNTS` (30 files of `crates/swell/tests`), run by
  the lane `slow` (`slow-tests`, `Taskfile.yml:180`). The edition is 2024
  (`Cargo.toml:6`): `std::env::set_var` is unsafe, so a test cannot set a variable for itself.
- The gate's test steps measured 127.9 s of 120 on the forge's 4 cores and 94.3 in a pool job on
  16 cores (`.engineering/planning/story/gate-test-steps-fit-four-cores.md:29`, `:32-33`); the
  collect run of swell took 16 to 24 s of test time on the build node in the last joins. The
  lane `slow` starts on a change of `crates/**` and of `Taskfile.yml`
  (`.github/workflows/slow.yml:6`, `:12`), so every unit that adds a builtin starts it.

## Change

Red first (Acceptance 1). Then:

1. **The instrument.** `support::host::pad_realm<H: Host>(agent, realm, k)`. At k = 0 it makes
   nothing (no object, no Symbol): a run without the variable is the run of today, byte for
   byte. At k > 0 it makes, in one `with_cx` entry, an ordinary object and k host functions
   `padding0` to `padding<k-1>` (length 0, steps that answer `undefined`), each a
   non-enumerable property of that object, and binds the object, non-enumerable, on the realm's
   global object under a Symbol the support code makes. So the padding is reached from the realm
   only, lives and dies with it as a builtin does, and no String key of the global moves
   (`builtins_global_this.rs:180-193` asserts the global's last names). No field is added to
   `TestHost`. `realm_functions()` reads `SWELL_TEST_REALM_FUNCTIONS`: absent or empty is 0; a
   value that is not an integer stops the test with a panic naming the variable.
   `support::host::realm` and a new `support::host::create_realm<H>(agent, setup,
   host_defined)`, which answers what `Agent::create_realm` answers and pads a `Normal` realm,
   call `pad_realm` with it. The unit confirms with the grep of Context that no test but the
   Symbol walk reads the Symbol keys of the global.
2. **Every realm of a test goes through it, but the listed exceptions.** Each `.create_realm(`
   outside `support/` is replaced by the helper, except the sites listed in the helper's doc
   comment, each as `file::function` with its reason in a few words. The kinds allowed: a call
   whose subject is `create_realm`'s own answer or error (a setup that throws, a released,
   faulted or re-entered agent, an `Err` case, a shape the fuzz or misuse transcript records); a
   test that measures the bare realm; and `host_187` under point 4. The Symbol walk
   (`builtins_symbol_values.rs`) is one: it makes its realm by the unpadded call, because it
   asserts over every function of the realm, prints their count, and its time at the full count
   is at its bound already. So N0, the count it prints, is the base's at any k.
3. **A test of the instrument**, `crates/swell/tests/realm_padding.rs`:
   `a_padded_realm_holds_k_functions_more`, at k = 0, 17, 38 and 200 (passed as the argument,
   not the variable), each in an agent of its own: `Object.getOwnPropertyNames(globalThis)` is the
   same list at each k; after `collect()` `live_bytes()` rises from k = 0 by more than 0 and at
   most k × (`HOST_FUNCTION_LIVE_BYTES` + `PROPERTY_LIVE_BYTES`); once the realm's roots are
   dropped and `collect()` ran twice, `live_bytes()` reads its value from before `create_realm`.
   It prints `heap_bytes()` after the realm and the bytes per function at each k.
4. **Every test red for the realm's size** takes its room from readings of its own agents, in one
   of the `Math` unit's two forms: a second round, or a probe or twin agent whose reading (where
   an arena grows next, what the script leaves) gives the budget. A budget between two readings
   holds only when no arena grows between them, and the test shows that. Each prints its
   readings; its doc comment says how the budget is found and what it was fitted to before.
   `host_187`: if red at some k, what it asserts and `BINDINGS_REALM_HEAP_BYTES` stay; it makes
   its realm unpadded (a listed exception: it measures a ceiling the host interface states over
   the bare realm), and the unit reports its readings per k. Green at every k, it uses the helper.
5. **Where it runs: a lane `slow` task, not a step of `check`.** `test-realm-sizes` in
   `Taskfile.yml` (the workflow's path list names `Taskfile.yml`, not every file under
   `taskfiles/`), a part of `slow` after `slow-tests`. Its step `build-realm-sizes` builds the two
   test sets of swell (the flags of `test` with `-p swell`, and those of `test-collect`) with
   `--no-run`, its log `.out/realm-sizes-build.log`. Then, for each k of `REALM_SIZES` (default
   `4 11 17 38 64 128 200`; a value given replaces the list) and each mode, it runs swell with
   `SWELL_TEST_REALM_FUNCTIONS=k`, a log per run `.out/realm-sizes-<k>-<mode>.log`, and prints
   `realm-sizes <k> <mode>: <p> passed, <f> failed, <i> ignored` with the run's failed tests. A
   red run does not stop the loop (`nextest-report` is not a step per run, since it exits 1). A
   run whose log holds a `Compiling` line counts red, naming `build-realm-sizes`. Last it prints
   `realm-sizes: <n> run(s) red` and exits non-zero when n is not 0. Why not `check`: one padded
   run of swell in both modes adds a whole run of the package to test steps that measured 127.9
   s of 120 on the forge. The command a unit that adds anything to the realm (a builtin, an
   intrinsic, a property of one) runs before it reports, naming every red test with its k and
   mode: `slot --timeout 30m task test-realm-sizes`. If the unit measures it over 30 minutes,
   it cuts the default list to `4 17 64 200`, says so, and reports the measured times of both.
6. **The sentence for `AGENTS.md`, "Rules"** (the coordinator writes it on the merge): "A test
   that needs room over the realm (a `memory_bytes`, a rise of `heap_bytes()`) takes it from
   readings of its own agents: a probe or twin agent that finds where an arena grows next or
   what a script leaves, or a second round once the arenas hold their capacity; a budget between
   two readings holds only when no arena grows between them, and never a number fitted to one
   size of the realm. A unit that adds anything to the realm runs `slot --timeout 30m task
   test-realm-sizes` before it reports and names every red test with its k and mode; the lane
   `slow` runs the same task."

No file under `crates/swell/src` is written: no step count, byte count or host surface moves.
A test that is right where the engine is wrong stops the unit, which reports it.

Writes: `crates/swell/tests/support/host.rs`, `crates/swell/tests/realm_padding.rs` (new),
`crates/swell/tests/main.rs` (written by `sh taskfiles/test-main.sh write`), the test files of
point 2 (at most the 23 of Context), the test files of point 4 (at most `generators.rs`,
`builtins_symbol.rs`, `block_functions.rs`, `lexical_declarations.rs` among the four known, and
each other one named in the report), `Taskfile.yml`. Shared: the four files of the `Math` unit
are written here only if a padded run names one of their tests red on `B`.

## Acceptance

1. **Red first**, on `B`, before a test is changed and with points 1 to 3 and 5 in place: the
   unit reads N0 from the Symbol walk's line (`slot cargo nextest run --locked -p swell --profile
   default --no-capture -E
   'test(=builtins_symbol_values::every_function_of_the_realm_given_a_symbol_answers_or_throws_a_type_error)'`
   prints `functions of the realm: N0, calls: ...`), then runs `slot --timeout 30m task
   test-realm-sizes REALM_SIZES='4 11 17 38 64 128 200 <234 - N0>'` and lists every red test
   per k and mode. Expected: the four tests of the `Date` getters red at k = 234 - N0 in the
   modes they were seen in, if N0 is at most 234.
   - **Search.** For a test of the four red at no k of the list, the unit runs that test alone
     at every k from 0 to 64 in the mode it was seen red in, built once with `--no-run` and with
     no build between: `slot --timeout 30m sh -c 'for k in $(seq 0 64); do
     SWELL_TEST_REALM_FUNCTIONS=$k cargo nextest run --locked -p swell --profile default <mode
     flags> --no-fail-fast -E "test(=<module>::<test>)" || echo "red at k=$k"; done'`, where
     `<mode flags>` are empty or `--features collect-every-safe-point --config
     .cargo/collect-profile.toml --cargo-profile collect`, and reports the first k at which it is
     red (its output has no `Compiling` line).
   - A test red at no k of the list and of the search is not changed: the unit reports its
     readings per k and lists it as unproven. The coordinator then proves it on the join of the
     `Date` getters unit, where it was measured red, and resumes the unit if it is red there.
   - A test red for another reason than the realm's size is reported, not changed.
2. `slot cargo nextest run --locked -p swell --profile default -E 'test(/^realm_padding::/)'`
   and the same with `--features collect-every-safe-point`: 1 passed each; the lines of the four
   k are in the report, with bytes per padding function beside the per-function rises of
   Context (+1,258 / 4, +7,708 / 17).
3. `slot --timeout 30m task test-realm-sizes`: a line per k and mode with `0 failed`, then
   `realm-sizes: 0 run(s) red`, exit 0; `grep -l Compiling .out/realm-sizes-*-*.log` prints
   nothing. Its wall time and CPU seconds are reported.
4. `slot task test` and `slot task test-collect`: 0 failed, exit 0. The unit reports the whole of
   swell in both modes, plain and at k = 200 (wall time, CPU seconds, the ten slowest each); no
   test over 60 s at any k.
5. `slot task check`: exit 0, with `gate-time test-steps <s> of 120`; the unit reports `<s>` and
   that of `B`.
6. `git diff --stat B -- crates/swell/src` prints nothing; `git diff --stat B -- Cargo.lock
   '*/Cargo.toml'` prints nothing.
7. `git grep -n '\.create_realm(' -- 'crates/swell/tests/*.rs' ':!crates/swell/tests/support'`
   prints the list of the helper's doc comment, line for line: one line per listed
   `file::function`, and no other.
8. `sh taskfiles/test-main.sh check`, `slot task clippy`, `cargo fmt --all --check`, `slot task
   forbid-unsafe-check`: exit 0.

Each new test is under 1.5 CPU seconds in the default mode and 2 with a collection at every
safe point, measured as `AGENTS.md` says ("A test stays under 1.5 CPU seconds").

The report says what the instrument cannot show: realms below the base's size (k is at least
0); a size between two k of the list (a test red only in that window); records a builtin makes
other than a function, its name and two properties (a new prototype, data properties such as
`Math`'s constants, Symbols, environments) and records it makes at its first call; the order in
which `create_realm` grows the arenas (the padding comes after it, in its own entry); the
listed exceptions, whose realms are bare; and the lib tests of `crates/swell/src`, which this
story does not write. It names the arenas a padding function grows (objects, strings, the
holder's property list and index map) against those of a builtin.

## Not in this story

- Any file under `crates/swell/src`; a step count, a byte count, a ceiling of `limits.rs`;
  what `host_187` asserts.
- The cost of the four tests the `Math` unit changed (`host_133` at 6.4 CPU seconds in the
  collect mode, `termination::…` at 1.10), unless a padded run names one of them red.- The lib tests of `crates/swell/src`; `hooks.rs` (red in the `Date` getters unit for another
  reason); the Symbol walk's entries for `toPrecision`.
- `CHANGELOG.md`, `website/data/status.json`, the test262 files, `conformance/`.

Rules this unit keeps: no unsafe code, no third-party crate, no Python; the failing run first; a
test killed at 60 s, the gate's test steps failing over 120 s; every command that compiles or
tests through `slot`, a pool job within 10 minutes unless it names its time (`slot --timeout`,
at most 30 minutes); a script that only measures or waits without `slot`.

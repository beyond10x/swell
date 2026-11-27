---
format: aep.planning-md/3
id: review-result:adversary-realmwalk-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit realmwalk
relations:
- reviews: story:realm-walk-in-parts-under-the-cpu-bound
revision: 1
---
Adversary pass 1 on the unit realmwalk, branch impl/realmwalk at 542f9fe, 2026-11-27.
Verdict green: no failing case; one warning and twelve notes.

```
unit: realmwalk (story:realm-walk-in-parts-under-the-cpu-bound)
verdict: green
cases: executed 11→14 (default), 10→13 (collect), red 0
origin: introduced 10, pre-existing 3, undecided 0
wrote-outside-worktree: <scratch>/wave2/realmwalk/adversary-1/
needs-coordinator: yes
```

I could not make the unit go red on anything its story promises: every claim I checked holds on `542f9fe`. There are 13 findings: 1 warning and 12 notes, none of them a blocker. The warning is the gate's variable list (finding 1). Four notes are test-code mutants or plants that survive the unit's tests.

## Findings

1. **Warning: the two new variables are not in the gate's variable list.** `TEST_RUSTFLAGS` and `TEST_NEXTEST_ARGS` are missing from `GATE_VARIABLES` (`Taskfile.yml:512`). That task's own text says a new gate variable is added there in the same change.
   - A value on the command line overrides the file: `task check 'TEST_NEXTEST_ARGS=… -E not(test(/walk/))'` silently runs fewer tests in the gate's `test` step.
   - Proof: `slot task gate-vars 'TEST_NEXTEST_ARGS=…' 'TEST_RUSTFLAGS=-A warnings' GATE_ROOT=planted` printed `GATE_ROOT=planted` and no `TEST_` name.
   - With `taskfiles/realm-places.yml` missing and both names exported, the empty-variable check does not fire (local probe).
   - Origin: introduced.
2. **Mutant m2 stays green.** `let mine = …take(1)` at `builtins_symbol_values.rs:1051` gave `walk part 0 of 2: 1 of 344 places, 4 calls` and 14 of 14 passed.
   - A test that would catch it: take each part's expected `Always` rows from its own place lines, not from the calls it made, or assert the part is not empty.
   - Origin: introduced.
3. **Mutant m3 stays green.** The loop cut to its first function plus `:1168` turned into `called` ran 4 calls and stayed green. The count check at `:1166-1170` is the only guard here; m1 (the loop cut without removing it) is red with `left: 4 right: 724`. The fix in 2 catches this too. Origin: introduced.
4. **Mutant m4 stays green with an alias.** With `:796` cut to `lines.len() != walk.len()`, an alias put in place of `values` (line 168) passes the equality test. A swap of lines is still red through the resolve check: `line 23 … resolves to another holder or function (holder other, function other)`. No test of the file catches the alias case. Origin: introduced.
5. **The cover test's size bound does nothing at 2 parts, and an empty part passes.** With every place hashed as `""`: `places of the realm: 344, in 2 parts: 0 + 344`, `walk part 0 of 2: 0 of 344 places, 0 calls`.
   - The unit's tests stay green in default and collect at full count; only my own-FNV case is red.
   - Under the skew the full part takes 0.031 / 0.068 / 0.155 CPU s (default / gate collect / gate collect at full count), so no CPU bound fails either.
   - Origin: introduced (`:1288-1296`).
6. **Nine parts are accepted.** The story allows 2 to 8 parts, but no test holds the upper limit: at 9 parts every unit test passes. Only my case `adversary_walk_parts_are_at_most_eight` is red (`WALK_PARTS has 9 parts, at most 8`). Origin: introduced (`:1249`).
7. **The text of the two `#` lines is not held.** Both lines swapped: the unit's tests are green and only my case `adversary_the_comment_lines_are_the_writers` is red. A file with CRLF line ends passes everything, because `lines()` drops the `\r` (harmless). Origin: introduced (`:1226-1241`).
8. **The writer can write a file its own parser cannot read, and exits 0.** With engine plants named `planted.dot` and `plantedSpace get`, the writer printed `352 places written`. The next run is red: `line 113 "global.Math.planted.dot": .planted reached undefined` and `.plantedSpace get reached the descriptor undefined`. Nothing passes silently. Origin: introduced (`taskfiles/realm-places.yml:54-63`, `parse_place` at `:543`).
9. **A builtin that answers normally where a TypeError is due stays green.** I planted `Function.prototype.bind` to answer normally for a Symbol `this`; its place is `%AsyncFunction.prototype%.[[Prototype]].bind`, under a further root. Both builds stay green, because the walk accepts both normal and TypeError (`:1142`). The old walk accepted it the same way. Origin: pre-existing.
10. **Kinds of function the walk does not reach.** It misses functions that are properties of an accessor's function object, functions on hidden intrinsics (`%ArrayIteratorPrototype%`, `%GeneratorPrototype%`), and bound targets. Plants of the first three kinds were not listed by the equality test. The real `next`/`return`/`throw` of those prototypes are never called with a Symbol, and `FURTHER_ROOTS` (`:333`) has no root for them. Origin: pre-existing.
11. **Weaker than the old walk: one shared realm.** The old walk ran all 344 functions in one realm. A defect that needs state left by a function in the other part is no longer seen (for example `unshift` is in part 0, `with` and `push` in part 1). This follows from the story's design. Origin: introduced.
12. **"Cannot drift" covers only three readers.** Literal `-D warnings` and `--workspace --locked --profile default` remain in `build` (`Taskfile.yml:663`), `test-doc` (`:791`), `slow-tests` (`:227`, `:254`), `collect-mode.yml:37,75` and `test-binaries.yml:65`. A change of `TEST_RUSTFLAGS` alone would make `test-doc` compile inside the gate's clock. No reader of `.out/build-tests.log` or `.out/test.log` changed its answer. Origin: introduced.
13. **An exported `RUSTFLAGS` wins over each step's `env:`.** go-task 3.54 gives `RUSTFLAGS=outer` for a literal value too. The new template check does not see it, and `gate-vars` refuses only values naming `test_modules`. Origin: pre-existing.

## What held, by attack

1. **Old walk against the parts.** Every assertion of the old test has its line in the new file: the four call forms `:1080-1085`, `take(places)` `:1086`, the stop on a host error `:1114-1121`, the agent alive `:1160`, rows in call order `:1162`, the 100 floor (cover test `:1319` plus `:1166`), `%AsyncFunction.prototype%.constructor` `:1313`. The 10 old rows match the new `WalkRow`s field for field. Neither version checked heap state or pending jobs.
   - Engine plants (`git diff --stat` showed 3 files, +3; each file restored equal to HEAD):
     - `Boolean.prototype.valueOf` answers an engine fault for a Symbol `this` (part 0): red with `walk part 0 of 2: a row not in WALK_EXPECTED: global.Boolean.prototype.valueOf (this): EngineFault`.
     - `Math.ceil` throws a RangeError on a Symbol argument (part 1): red with `walk part 1 of 2: … global.Math.ceil (argument 1): RangeError`.
     - Both red in the default and the collect build, each with its place in the message.
2. **Builds and padding.**
   - (a) Feature checks: 14 lines in 8 files use `collect-every-safe-point`. They are the collection at a safe point (`agent.rs:1048`) and counts of unit tests; none defines a property. The `fault-injection` lines (`agent.rs:243-661`) define no property. No `debug_assertions` check changes the realm.
   - (b) Of my 11 engine plants, the walk listed 8, each named (`missing from the file: ["global.JSON.plantedSetter set", "global.JSON[Symbol(Symbol.hasInstance)]", "global.Math.plantedGetter get", …, "%AsyncGeneratorFunction.prototype%.prototype.planted", "global.Array.prototype.planted"]`). The 3 it did not list are in finding 10.
   - (c) Padding 4, 11 and 64 functions, both builds: 344 places on every line, 14 of 14 and 13 of 13 passed. The padded functions are not places because `in_a_bare_realm` calls `agent.create_realm(None, None)` itself (`:746`), not the padding `support::host::create_realm`.
3. **File form.** The file holds `values`, `Set.prototype.keys` and `Map.prototype.entries`, not their aliases. Each alias is called once.
   - Red in the default build, with the right line: alias put in place (in the collect build the parts stay green), swap within a part, swap across parts, trailing space, tab, BOM, a third `#` line.
   - Green everywhere: the two `#` lines swapped (finding 7) and CRLF line ends.
4. **Partition.** My own FNV-1a 64 (u128 arithmetic) gives `[181, 163]`, each line in the part `walk_part_places` gives it. `_part_2 = 0` is red with `WALK_PARTS is [0, 0], not 0 to 1`.
5. **Order and state.** With the `push` line removed, part 1 reads length 0 and meets the two `with` rows; the parts stay green in the default build and the collect build at full count. At 8 parts all parts are green in the default build and the collect build at full count, so a part's result does not depend on the number of parts.
6. **Writer and task steps.**
   - Planted panic in the walk: `cargo nextest run exited 100`, exit 201, the fixture equal to HEAD.
   - Stale `walk.txt` planted and the write taken out: `the walk wrote no place to …`, exit 201.
   - `realm-places.yml` moved away: `build-tests` and `test` each print their one line and exit 201 before any build.
   - `TEST_RUSTFLAGS: ''`: all three tasks print their line and exit 201.
   - After `task build-tests`, the writer's log has 0 `Compiling` lines (`Finished … in 0.01s`), so the three share one build.
7. **Times.** CPU s, 3 runs each (identical or within 0.001 s), build node of 32 cores at load 2.9; the collect build is the gate's profile `collect` at opt-level 1:

   | test | default | default, full count | collect | collect, full count |
   |---|---|---|---|---|
   | part 0 | 0.020-0.021 | 0.020 | 0.029 | 0.060 |
   | part 1 | 0.018 | 0.018 | 0.025-0.026 | 0.052 |
   | equality test | 0.060-0.061 | 0.060 | not built | not built |
   | cover test | 0.005 | 0.005 | 0.002 | 0.002 |

   Gate steps (load 8 to 11): `task test` exit 0 in 10 s, `19 test binaries: 2293 passed, 0 failed, 1 ignored`, slowest `builtins_map_and_set::churn_without_live_entries_stays_bounded` 2.90 s. `task test-collect` exit 0 in 4 s, `2 test binaries: 1807 passed, 0 failed, 1 ignored`, slowest 2.75 s. No test of this unit is among the ten slowest of either step.

## Cases added (uncommitted)

All three are in `crates/swell/tests/builtins_symbol_values.rs` and green on the committed tree:
- `adversary_partition_of_the_committed_file_by_an_own_fnv`: pinned to `[181, 163]`, so it goes red on any change of the file.
- `adversary_walk_parts_are_at_most_eight`
- `adversary_the_comment_lines_are_the_writers`

## Commands (all logs in `adversary-1/logs/`, summary in `adversary-1/summary.txt`)

- `adv.sh` steps, each a run of `cargo nextest run --locked -p swell --test main -E 'test(/^builtins_symbol_values::/)'` (default, or collect with `--cargo-profile collect`) or a `slot --timeout 30m [--fetch crates/swell/tests/fixtures] …` job:
  - `base-def`: `14 tests run: 14 passed`
  - `base-col`: `13 tests run: 13 passed`
  - `engine-triple` def and col: exit 100, 2 failed each
  - `realm-plant`: def exit 100, writer exit 0 (352 places), def2 exit 100, 4 failed
  - `padding`: six runs, exit 0
  - `m2` and `m3`: exit 0 (survived); `m1` and `m4-swap`: exit 100 (caught)
  - `gate`: `build-tests` exit 0 in 337 s, writer exit 0, `test` exit 0, `test-collect` exit 0
- `measure.sh measure-base 3 …` and `measure-skew 2 …`: `slot exit 0`
- `slot task gate-vars …`: shown in finding 1.
- Local go-task probes (`varprobe.sh`, `varprobe2.sh`, in my scratch directory): the command line wins over the file's vars; the file's vars win over the environment; an exported `RUSTFLAGS` wins over `env:`.
- `git status --short` at the end: ` M crates/swell/tests/builtins_symbol_values.rs` (+64, my cases only). Every engine, fixture, test-file and task-file plant was restored and checked equal to HEAD or to my saved copy before the next.

## Not attacked

- The whole gate (`task check`), the lanes `slow-tests` and `test-realm-sizes`, and the forge workflow.
- go-task's version on the build node: my precedence probes ran on the workstation's 3.54.0. Only the `gate-vars` result ran on the pool.
- The collect build at opt-level 0 (the lane's build) was not timed again; the unit's readings there (0.687 at most) stand.

```findings
- file: Taskfile.yml
  line: 512
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'TEST_RUSTFLAGS and TEST_NEXTEST_ARGS are gate variables missing from GATE_VARIABLES, so a command-line value silently changes what the gate step test runs and gate-vars does not print it'
- file: crates/swell/tests/builtins_symbol_values.rs
  line: 1051
  category: mutant
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'walk_part with its places cut to the first one stays green, since the expected rows and the count both come from the places it walked'
- file: crates/swell/tests/builtins_symbol_values.rs
  line: 1166
  category: mutant
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The count assertion is the only guard against a part that stops after its first function; without it such a part stays green'
- file: crates/swell/tests/builtins_symbol_values.rs
  line: 796
  category: mutant
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'With the equality test cut to a count of lines, a file holding an alias place passes, because the resolve check finds the same holder and function'
- file: crates/swell/tests/builtins_symbol_values.rs
  line: 1291
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'At 2 parts the bound of twice the ceiling is the whole file, so a partition of 0 + 344 passes the cover test and the empty part passes with 0 calls'
- file: crates/swell/tests/builtins_symbol_values.rs
  line: 1249
  category: boundary
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'No test holds the upper limit of 8 parts that the story sets; 9 parts pass every test of the unit'
- file: crates/swell/tests/builtins_symbol_values.rs
  line: 1226
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'The text of the two comment lines is not compared with the writer, so a file with them swapped or edited passes every test'
- file: taskfiles/realm-places.yml
  line: 63
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'The writer exits 0 after writing places that parse_place cannot read (a name with a dot or ending in a space and get), and the parts and the equality test are red at the next run'
- file: crates/swell/tests/builtins_symbol_values.rs
  line: 1142
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: 'A builtin that answers normally where a TypeError is due (Function.prototype.bind with a Symbol this, planted) passes in both builds, as in the old walk'
- file: crates/swell/tests/builtins_symbol_values.rs
  line: 333
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: 'The walk does not reach functions on an accessor function object or on hidden intrinsics such as ArrayIteratorPrototype and GeneratorPrototype, so their next, return and throw are never called with a Symbol'
- file: crates/swell/tests/builtins_symbol_values.rs
  line: 1047
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'Each part runs in its own realm, so a defect that needs state left by a function of the other part is no longer seen as it was in the one-realm walk'
- file: Taskfile.yml
  line: 791
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'Literal copies of the flags stay in build, test-doc, slow-tests and collect-mode, so changing TEST_RUSTFLAGS alone would make test-doc compile inside the clock'
- file: Taskfile.yml
  line: 757
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: 'An exported RUSTFLAGS wins over the env value of each step in go-task 3.54, and the new template check does not see it'
```

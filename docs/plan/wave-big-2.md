# Wave big-2: slice 9, collections, Proxy and Reflect

One wave of [the wave plan](waves.md), "After slice 4: bigger waves". Opened on 2026-11-26,
07:30 CET, on the operator's word of about 05:18 CET (`approval-record:wave-big-2`, which
applies the reading recorded in `approval-record:wave-big-1` and quotes the answer to the
coordinator's question, relayed at 07:31 CET: start). Skill version 0.26.0 of
`aep:implementing`.

The wave branch is `wave/big-2`, cut from `wave/big-1` at `2d04fa12` while that wave still
runs, with the plan branch of slice 9 merged in. `wave/big-1` is merged into it again each
time a unit of that wave lands, and the pull request of this wave is opened after the pull
request of wave big-1 is merged.

## The stories

The ten stories of `epic:slice-09-collections-proxy-reflect`, in the orders of
[the plan page](waves.md), "Slice 9". Each unit has a managed worktree `swell-impl-<unit>`,
a branch `impl/<unit>`, and its work files in the directory `wave2/<unit>` of the coordinating
session's scratch directory. The stage is kept current by the coordinator.

| order | unit | story | stage |
|---|---|---|---|
| 1 | keyed | `story:keyed-entry-table` | `impl/keyed`, cut at `58501bbf`; dispatched on 2026-11-26, 07:48 CET; green at `d9efe364` (2 files: `crates/swell/src/object/keyed_entries.rs`, new, 1,539 lines, and one `mod` line): the story's filter runs 39 tests on the base and 55 after in both modes (16 new, their times sum to 0.28 s), the package `1338 tests run: 1338 passed`; three deviations from the story, each stated by the unit: `next_live` takes the agent and charges a step per hole it passes, released cursors go to a free list in place of `Vec::retain`, a compaction with an index map charges a step per live entry; pass 1 (`review-result:adversary-keyed-pass-1`): 7 findings, 13 cases of which 3 red; a closed `CursorId` reaches the cursor that took its place in the free list, and `clear` and a compaction walk the cursor list without a charge per place (10,000 places for 2 steps and for 1 step); the two u32 limits have no test that reaches them; three mutants the 16 tests do not catch; a model test over 9,000 operations found no difference; correction 1 with the same implementor: the table refuses a stale id by a generation in the id, every place a walk visits is charged; correction 1 returned at `61a171bf` (one file; the module's tests from 16 to 35, the story's filter from 55 to 74 in both modes): a `CursorId` is 16 bytes with a generation from one counter of the table, a place of the cursor list is 32 bytes, `clear` charges `ceil((live + holes) / 64) + places` steps and a compaction `moved + places`; pass 2, the last (`review-result:adversary-keyed-pass-2`): no red case, 7 findings (1 warning, 6 notes), 4 green cases to keep; the end of the generation counter gets its test and answers `EngineFault` as the realm counter does; the two u32 limits are filed once for the engine (`story:a-limit-of-an-integer-width-has-a-test-or-a-stated-reason`, a backlog draft); the Bounds of this story and one test text of `story:map-and-set` now name the charge per place of the cursor list; correction 2 returned at `763d7bf6` (one file, 306 lines added, 5 removed; the module's tests from 35 to 39, the story's filter from 74 to 78): `open_cursor` faults the agent at the counter's end, and the adversary's four cases are tests of the module; the coordinator read the diff (`review-result:keyed-correction-2-coordinator-read`, no defect) and ran the claim again on `763d7bf6` on the build node (`78 tests run: 78 passed` in both modes, the gate's clippy step and the format check exit 0); merged into `wave/big-2` |
| 1 | reflect | `story:reflect-namespace` | `impl/reflect`, cut at `58501bbf`, with the base run copied to `.out/test262-base/runs.tsv`; dispatched on 2026-11-26, 07:48 CET; green at `3cc59442` (9 files, 994 lines added): `crates/swell/src/builtins/reflect.rs` and 13 tests that pass in both modes (the filter runs 79 tests on the base and 92 after); the part run passes the 480 cases of the list and the 249 guard cases (`test262-slice 09 480 960 0 0 480 480 0 0`), and no run ends on `Reflect is not defined` (296 on the base); `KIND_SITES` is a slice; pass 1 (`review-result:adversary-reflect-pass-1`): green, no red case, 5 notes on what the 13 tests cannot catch (the credit of the value list of `ownKeys`, the charge per element of the argument list, `Reflect.get` with an explicit `undefined` receiver, `Reflect.deleteProperty` on a dense array element, where the interrupt test raises its interrupt), 12 green cases; correction 1 returned at `8a2c03f4` (one file, `crates/swell/tests/builtins_reflect.rs`, 594 lines added; the module's tests from 13 to 28, the story's filter from 92 to 107): the charge test reads `heap_bytes()` after every entry, and the three mutants of the notes were shown red; the coordinator read the diff (`review-result:reflect-correction-1-coordinator-read`, no defect) and ran the claim again on `8a2c03f4` on the build node (`28 tests run: 28 passed` in both modes, the gate's clippy step and the format check exit 0); merged into `wave/big-2` |
| 2 | mapset | `story:map-and-set` | `impl/mapset`, cut at `896bcbe9`, with the base run of `dc310c6c`; dispatched on 2026-11-26, about 09:30 CET; green at `9cae087a`: `Map`, `Set` and their iterators on the entry table; pass 1 of the adversary found no defect of the unit, two counts of tests below the count their requirements state and step charges held by no test (`review-result:adversary-mapset-pass-1`); correction 1 takes the adversary's 12 cases, holds each step charge by a test and gives HOST-25 and HOST-214 the 10,000 keys of their requirements in the lane `slow` and 1,000 in the gate (`5d17ff05`, tests only) and merges the wave branch (`a2898842`), read by the coordinator (`review-result:mapset-correction-1-coordinator-read`); verified at `a2898842` (swell 1236 in both modes, the library 250, the part of test262 `passed 532 bound 532 B 156 F 376 R 0`, `realm-sizes: 0 run(s) red`) and merged as `fcb18e5d`, where swell runs 1236 tests green in both modes |
| 2 | proxy | `story:proxy-objects-and-access-traps` | `impl/proxy`, cut at `896bcbe9`, with the base run of `dc310c6c`; dispatched on 2026-11-26, about 09:30 CET; green at `859debad`: Proxy objects, `Proxy.revocable`, the dispatch arms and six access traps; pass 1 of the adversary found no defect of the unit and three changes of the code its 19 tests did not catch (`review-result:adversary-proxy-pass-1`); correction 1 takes 32 of the adversary's 33 cases (`ae1ee7d1`, one test file; the 33rd, `super` through a Proxy, goes to the unit `proxytraps`) and merges the wave branch (`cb029fbd`), read by the coordinator (`review-result:proxy-correction-1-coordinator-read`); verified at `cb029fbd` (swell 1256 in both modes, the library 251, the part of test262 `passed 1325 bound 1325 B 1081 F 244 R 0`, `realm-sizes: 0 run(s) red`, the stack profiles 4 passed each) and merged as `cc8eb943`, where swell runs 1308 tests green in both modes; the 32 cases sum to 61.5 s with `collect-every-safe-point`, read against the gate's 120 s at the wave's close |
| lane | slowlane | `story:realm-size-lane-within-its-share` | ordered ahead of the feature units on 2026-11-26 (`approval-record:wave-big-2-slow-lane`): the lane `slow` took 22 min 35 s on pull request 19; bound 15 minutes on the forge and 10 minutes per part; no feature unit merges before it; `impl/slowlane`, cut at `d382bb69`; dispatched on 2026-11-26, 21:10 CET; green at `949e906a` after one correction (`review-result:adversary-slowlane-pass-1`, `review-result:slowlane-correction-1-coordinator-read`); merged into `wave/big-2` at `11d92ae` on 2026-11-26 |
| lane | lanebuilds | `story:realm-size-lane-within-its-share` (second unit) | `impl/lanebuilds`, cut from `impl/slowlane`; the stack tests are built with their modules only, the collect mode of `test-realm-sizes` runs at every size again; green at `d80db027` after one correction (`review-result:adversary-lanebuilds-pass-1`, `review-result:lanebuilds-correction-1-coordinator-read`); merged at `93e8105` on 2026-11-27, 00:31 CET; the lane on the build node: 503.9 s cold |
| lane | lanetests | `story:realm-size-lane-within-its-share` (third unit) | `impl/lanetests`, cut from `impl/lanebuilds` at `d97404f`; dispatched on 2026-11-26, about 23:50 CET: the tests over 2 s in the lane's collect build; green at `4f4d1a28` after one correction (`review-result:adversary-lanetests-pass-1`, `review-result:lanetests-correction-1-coordinator-read`); merged at `7761c2a` on 2026-11-27, 01:38 CET; the collect runs of `test-realm-sizes` sum to 1,625.6 test-seconds on the build node (2,588.5 at the unit's base, at another load); correction 2 (`d89192ea`, one test file): `promise_statics::the_values_of_a_pending_promise_all_are_charged_until_it_settles`, a test of the unit setmethods that took 2.19 s alone in the collect build of the lane, runs 12 pending promises there (1.30 CPU s) and its earlier 20 with `SWELL_FULL_COUNTS=1` (2.2 CPU s), read by the coordinator (`review-result:lanetests-correction-2-coordinator-read`) and merged at `58790b9` on 2026-11-27, 02:03 CET |
| lane | lanefour | `story:realm-size-lane-within-its-share` (fourth unit) | `impl/lanefour`, cut at `158888d9`; dispatched on 2026-11-27, about 03:25 CET, after the forge's run 45925590365 read `slow-time lane 1137.3 of 900` on `158888d9` (parts 419.3, 268.4, 431.2, 18.4); its target is 800 s or less on the forge without a shard, a raised bound, a skipped test or a build cache between runs; green at `826214f` after two adversary passes (`review-result:adversary-lanefour-pass-1`, `review-result:adversary-lanefour-pass-2`) and two corrections: the lane's collect mode at opt-level 1 except the stack tests, held by a guard at run time, the build of the profile `collect` beside `slow-test262`, a character class that keeps its last answer; one cold lane on a build node of 32 cores reads `slow-time lane 444.2 of 900`; estimated for the forge about 848 s of 900; read by the coordinator (`review-result:lanefour-correction-2-coordinator-read`); merged at `1c8d802` on 2026-11-27, 08:41 CET; the forge measured `slow-time lane 882.6 of 900` on `b9adbe35` (run 45963552396, 2026-11-27, 09:09 to 09:24 CET) |
| lane | lanefive | `story:property-escape-cases-spend-less-in-the-interpreter-loop` | taken into the wave on 2026-11-27 (`approval-record:wave-big-2-lane-unit-5`) after the forge read `slow-time lane 882.6 of 900`; two plan critics read the story once and the coordinator decided their findings (`review-result:lanefive-plan-coordinator-read`); `impl/lanefive`, cut at the commit of this row; its bound is a ratio of at most 0.90 of the CPU seconds of the loop of `buildString` (`task perf-loop`), with no step count, heap reading or stack bound moved; it is in the wave's second pull request |
| lane | lanesix | `story:gate-test-steps-hold-the-budget-on-the-slower-runner` | taken into the wave on 2026-11-27 (`approval-record:wave-big-2-lane-unit-6`) after the forge failed `check` on the first pull request at `fd71252b` with `gate-time: the test steps ran 147.6 s, over the budget of 120 s` (77.2 s on `8d6cfebc`, the same code: the runner took 1.94 times the CPU seconds); two plan critics read the story once and the coordinator decided their findings (`review-result:lanesix-plan-coordinator-read`); `impl/lanesix`, cut at the commit of this row; its bound is the CPU seconds of the test steps at 80 % or less of the base on the build node, with no test, plant or step taken out of the gate, and a line `gate-runner: ...` in the log of `check`; built before the conformance close starts; it is in the wave's second pull request |
| tests | randomcases | none (`story:math-random` is implemented) | `impl/randomcases`, cut at `b9bd2838`; dispatched on 2026-11-26, 21:10 CET: the four cases of `review-result:adversary-random-pass-2`, which passed and were never committed, are taken under the unit's names; tests only; green at `497fa1ff` (2 test files, 271 added lines; swell-num 153 and `math_random::` 15 tests green in both modes), read by the coordinator (`review-result:randomcases-coordinator-read`) and merged in the commit after this one |
| tests | realmwalk | `story:realm-walk-in-parts-under-the-cpu-bound` | `impl/realmwalk`, cut at `158888d9`; its first unit stopped at the story's stop rule on 2026-11-27 (the list of the realm's 344 functions alone took 2.33 CPU s in the lane's collect build, and every part of the walk would pay it); the story was written again: the places of the functions are a committed file, `crates/swell/tests/fixtures/realm_places.txt`, which the parts read. The form of that file: it carries no digest of its sources; one test in the default build makes the list again at every gate run and compares it with the file line for line (0.05 CPU s). This differs from the two forms the folder's rule "An expensive product is made once" names (a committed file with a digest; a fixture built once per run), so it was put to the operator on 2026-11-27, 03:48 CET, with three options (this form as the default; a digest as form 1; a setup step under `target/`). No answer came within the 15 minutes of the rule "A default proceeds", and the default was taken at 04:04 CET. Three plan critics read the rewritten story twice (acceptance, design, parallel safety); each asked for a revision both times, and the coordinator decided the remaining findings and read the change (`review-result:realmwalk-plan-coordinator-read`, story revision 9); the branch was merged up to `wave/big-2` at `edfd254` (head `557206e2`, source code equal to `6f21df6d`) and the unit was dispatched on 2026-11-27, 04:13 CET; it is merged into `wave/big-2` when it is closed; its second start reported green at `542f9fe` (2 parts, 181 and 163 of 344 places; the equality test 0.061 CPU s alone); pass 1 of the adversary was green with one warning and twelve notes (`review-result:adversary-realmwalk-pass-1`); correction 1 at `fa63ea9` (the two variables of the gate's test build listed and checked by `test-vars-check`, what a part owes, at most 8 parts, one definition of the comment lines, a read-back before a write); read by the coordinator (`review-result:realmwalk-correction-1-coordinator-read`), who ran the whole gate on the head; merged at `1923715` on 2026-11-27, 06:23 CET |
| 3 | weak | `story:weak-map-and-weak-set` | `impl/weak`, cut at `b9bd2838`, with the base run of `b9bd2838` (`test262 total 53801 8982 7 44812 27747 17062 3`); dispatched on 2026-11-26, 21:10 CET; the list reads `passed 156 bound 156 B 156 F 247 R 247` on the base; green at `7d6e94c5` after one correction (`review-result:adversary-weak-pass-1`, `review-result:weak-correction-1-coordinator-read`); merged at `6e9b7fa` on 2026-11-27, 00:36 CET |
| join | stackjoin | none (a defect of the join of `story:weak-map-and-weak-set`) | `impl/stackjoin`, cut at `52fd8eb`: since the merge `6e9b7fa` the stack test `limits_scripts::min_stack_bytes_is_enough_through_builtins` overflowed its thread in the profile `stack-debug` (two more arms grew the frame of `Agent::run_builtin` from 1,360 to 1,424 bytes); the first change, `2951b32`, was sent back by the coordinator (`review-result:stackjoin-coordinator-read-1`: the routes of 18 groups grew); green at `22842da5` after one correction, a dispatch through one call into one of four parts (`review-result:stackjoin-correction-1-coordinator-read`), the four stack tests green in both profiles; no adversary pass; merged at `2b5c3b8` on 2026-11-27, 01:56 CET |
| 3 | setmethods | `story:set-methods` | `impl/setmethods`, cut at `b9bd2838`, with the base run of `b9bd2838` (`test262 total 53801 8982 7 44812 27747 17062 3`); dispatched on 2026-11-26, 21:10 CET; the list reads `passed 177 bound 177 B 177 F 108 R 108` on the base; green at `18c744e1` after two corrections (`review-result:adversary-setmethods-pass-1`, `review-result:setmethods-correction-1-coordinator-read`); merged at `fbddbb5` on 2026-11-27, 00:38 CET |
| 3 | proxytraps | `story:proxy-descriptor-prototype-and-extensibility-traps` | `impl/proxytraps`, cut at `b9bd2838`, with the base run of `b9bd2838` (`test262 total 53801 8982 7 44812 27747 17062 3`); dispatched on 2026-11-26, 21:10 CET; the list reads `passed 1134 bound 1134 B 1134 F 153 R 153` on the base; one case of it ends at the memory budget there; green at `c8e54d7f` (6 files; the part run reads `passed 1287 bound 1287 B 1134 F 153 R 0`); pass 1 of the adversary was red in 5 of its 14 added cases (`review-result:adversary-proxytraps-pass-1`): `for`-`in` and JSON lost keys and objects a trap had made once a later trap ran a collection, and `for`-`in` dropped the keys of the prototype a `getPrototypeOf` trap answered; correction 1 (`61b14ed2`) keeps every value a walk uses after a trap (204 call sites read, 7 corrected) and takes HasProperty out of the loop; pass 2 was red in one case (`review-result:adversary-proxytraps-pass-2`): a `getPrototypeOf` trap that answers differently on its second call made the loop drop a key that was never deleted; correction 2 (`95827c0`) makes the loop follow the chain its walk recorded, without a second [[GetPrototypeOf]], releases a trap's arguments when it returns (at most 10 values kept outside a room per method) and holds the pass's mutants by tests; a last step (`14a6133f`) replaces a session's mark in comments by the rule in words, merges the wave branch and splits two tests of the collect build under the lane's CPU bound; read by the coordinator (`review-result:proxytraps-correction-1-coordinator-read`); verified at `14a6133f` (swell 1506 passed in the default mode and 1485 with a collection at every safe point; the four stack tests in both profiles; the lane of realm sizes with 0 runs red, by the unit) and merged at `2359923` on 2026-11-27, 02:27 CET, where swell runs the same 1506 and 1485 tests green and the four stack tests pass in both profiles |
| words | marks | none | `impl/marks`, cut at `2359923`: a comment of `crates/swell/src/interp/class.rs` and a summary line of `taskfiles/gate-time.yml` named a decision by a number of the coordinating session; both state the rule in words now (`176998f`, two files, four lines, no line of code); the coordinator read the diff; no adversary pass and no record in the store, since the unit has no story; merged at `758ad53` on 2026-11-27, 02:31 CET |
| 4 | upsert | `story:map-upsert-and-group-by` | `impl/upsert`, cut at `d90c333` after the merge of the unit `realmwalk`, on whose story it depends; dispatched on 2026-11-27; green at `5458d2e` (11 tests; the part of 202 cases: 202 passed, 165 on the base); pass 1 of the adversary dispatched; pass 1 was green on the code (`review-result:adversary-upsert-pass-1`: two warnings and five notes, each a gap in the tests or texts; two planted defects passed all 11 tests), correction 1 at `5233731` (four tests more, two of them red with a defect planted, and a bound on the bytes per grouped element in the memory test; 15 tests); read by the coordinator (`review-result:upsert-correction-1-coordinator-read`); merged at `6e03591` on 2026-11-27, 08:21 CET |
| 4 | overrides | `story:host-object-overrides` | `impl/overrides`, cut at `6f21df6d`, with the base run of `6f21df6d` (`test262 total 53801 8982 7 44812 33457 11353 2`); dispatched on 2026-11-27, 04:06 CET; the story has no test262 case; the design line HOST-222 is merged (`f3d194e2`); it is merged into `wave/big-2` when it is closed; green at `3d29fd7`; pass 1 of the adversary was red (`review-result:adversary-overrides-pass-1`: a descriptor held across an `is_extensible` override was freed by a collection, the walks of HasProperty, Get and Set held their key and value across an override), correction 1 at `046bfb6`; pass 2 was red (`review-result:adversary-overrides-pass-2`: four invariants broken where the object's record answers for an override the table leaves out, and the question whether a `has_property` false is a fact), correction 2 at `f28eb7c`, `4dccde9` and `2a91ce5` (the record's answer goes through the override's checks; a `has_property` false records nothing and faults only for a non-configurable property; the stack lists name the module); read by the coordinator (`review-result:overrides-correction-2-coordinator-read`); `swell::main` 1657 passed and 1630 with collect-every-safe-point on the unit's head; merged at `0680551` on 2026-11-27, 07:16 CET |
| 5 | close9 | `story:slice-09-conformance-close` | `impl/close9`, cut at `bc45e92` (every other story of slice 9 merged), with the start run of `cf7f253` and the base run of `bc45e92` (`test262-slice 09 1787 3545 0 0 1787 1765 22 0`); dispatched on 2026-11-27; green at `66fe379` (`test262-slice 09 1864 3694 0 0 1864 1864 0 0`: 22 cases moved to slice 12, 99 taken in from slice 19); adversary pass 1 red on 32 more cases the rule takes in (`review-result:adversary-close9-pass-1`); correction 1 at `37aaed6` (31 of the 32 come in, 130 in all; one stays in slice 19); correction 2 at `6cc4ca7` moves the 37 cases that need slice 8 there, as the plan of slice 8 counts them (`test262-slice 09 1858 3678 0 0 1858 1858 0 0`); read by the coordinator in place of a second pass (`review-result:close9-correction-2-coordinator-read`); merged at `65b424e` on 2026-11-27, 09:45 CET |

## The stories of slice 8

The fourteen stories of `epic:slice-08-classes` joined this wave on 2026-11-26
(`approval-record:wave-big-2-slice-8`; the plan branch `plan/slice-08` merged at `3237cf33`),
in the orders of [the plan page](waves.md), "Slice 8, `classes`". Order 1 has no edge to a
story of slice 9 and starts beside slice 9's order 2.

| order | unit | story | stage |
|---|---|---|---|
| 8.1 | newtarget | `story:new-target-meta-property` | `impl/newtarget`, cut at `896bcbe9`, with the base run of `dc310c6c`; dispatched on 2026-11-26, about 09:30 CET; green at `90960dae`: `new.target` is parsed, compiled and run, an arrow keeps it in its box, `EvalContext` carries it into eval code; pass 1 of the adversary found no defect of the unit, three changes of the code the tests did not catch and four re-entry tests older than the wave that overflow in the profile stack-debug (`review-result:adversary-newtarget-pass-1`; the draft `story:stack-tests-hold-in-the-stack-debug-profile`); correction 1 takes the adversary's 7 cases and one fixture (`68c32a34`, no line of code) and merges the wave branch with nine resolved conflicts in the parser (`f73a936c`: `new_target` beside `super_property`, six fixtures state what a class source with `new.target` answers while `class` is held), read by the coordinator (`review-result:newtarget-correction-1-coordinator-read`); verified at `f73a936c` (swell-syntax 61 passed, also at the full counts; swell-compiler 17; swell 1226 in both modes) and merged as `334790ab`, where swell-syntax runs 61, swell-compiler 17 and swell 1257 tests green |
| 8.1 | super | `story:super-property-and-super-syntax` | `impl/super`, cut at `896bcbe9`; dispatched on 2026-11-26, about 09:30 CET; green at `6a89ce2d` (30 files): `super.x` and `super[e]` run in the methods of object literals, the part run reads `passed 4822 bound 4822 B 4756 F 66 R 0`; the unit sets the parser's flag from the function's kind in `open_context` and adds the instructions `SuperBase` and `SuperKey` (the base is read before the key is converted); pass 1 of the adversary found no defect of the unit that a script reaches, one refusal older than the unit (a method in an arrow's parameters inside a method, lifted by `story:derived-classes-and-super-call`) and three changes of the code the tests did not catch (`review-result:adversary-super-pass-1`); correction 1 takes the adversary's 31 cases and four fixtures (`ba13cabb`, no line of the implementation) and merges the wave branch (`96e070c4`: `super_property_of` lists the three function kinds of the class unit, eight fixtures state what a class source with `super` answers while `class` is held; `early-errors/super-*.js` 11 fixtures, `held/` 12, `super_property::` 45 tests), read by the coordinator (`review-result:super-correction-1-coordinator-read`); verified at `96e070c4` (swell-syntax 60 passed, also at the full counts; swell-compiler 17; swell 1205 in both modes) and merged as `66882ac0`, where swell-syntax runs 60, swell-compiler 17 and swell 1205 tests green |
| 8.1 | classsyntax | `story:class-syntax-and-early-errors` | `impl/classsyntax`, cut at `896bcbe9`; dispatched on 2026-11-26, about 09:30 CET; green at `1e671094` (43 files): the parser reads a class and answers its early errors, a class without one is held to the end of the parse (`HOLDS`), the part run reads `passed 2936 bound 2936 B 2469 F 467 R 0`; the base has 62 nesting kinds and 14 refused fixtures where the story says 61 and 16; one test of the base is over its CPU bound at full counts (filed as the draft `story:stack-bound-parts-fit-their-cpu-bounds`); pass 1 of the adversary found no defect and five changes of the code the tests did not catch (`review-result:adversary-classsyntax-pass-1`); correction 1 adds the tests (`7494b14c`, 49 lines, no line of the implementation; `early-errors/class-*.js` 24 fixtures, `held/` 5), read by the coordinator (`review-result:classsyntax-correction-1-coordinator-read`); verified at `7494b14c` (swell-syntax 59 passed, also at the full counts; swell-compiler 17; swell 1154) and merged as `8af6770c`, where swell-syntax runs 59, swell-compiler 17 and swell 1160 tests green |
| 8.2 | classes | `story:class-definition-evaluation` | `impl/classes`, cut at `b9bd2838`, with the base run of `b9bd2838` (`test262 total 53801 8982 7 44812 27747 17062 3`); dispatched on 2026-11-26, 21:10 CET; the list reads `passed 2546 bound 2546 B 2546 F 3297 R 3297` on the base; green at `60103e33` (34 files); pass 1 of the adversary was red in its 2 added cases (`review-result:adversary-classes-pass-1`): the computed keys of a class in non-strict code ran as non-strict code, and an anonymous class under a computed key of an object literal was named after its elements ran; correction 1 (`3e013e7`) makes class code strict on every instruction and names the class first, and its part run reads `passed 5843 bound 5843 B 2546 F 3297 R 0`; pass 2 found it green and left four cases (`review-result:adversary-classes-pass-2`), which correction 2 commits under the unit's names (tests only), read by the coordinator (`review-result:classes-correction-1-coordinator-read`); verified at `4d2d8345` (swell-syntax 79 passed, also at the full counts; swell-compiler 17; swell 1351 in both modes) and merged at `ed7f24b` on 2026-11-27, 01:59 CET, where swell runs 1418 tests green in both modes and the four stack tests pass in both profiles |
| 8.2 | elements | `story:class-element-syntax-and-early-errors` | `impl/elements`, cut at `b9bd2838`, with the base run of `b9bd2838` (`test262 total 53801 8982 7 44812 27747 17062 3`); dispatched on 2026-11-26, 21:10 CET; the list reads `passed 2555 bound 2555 B 2555 F 800 R 800` on the base; green at `5364e45e` after one correction (`review-result:adversary-elements-pass-1`, `review-result:elements-correction-1-coordinator-read`); merged at `5b25b85` on 2026-11-27, 00:33 CET |
| 8.3 | derived | `story:derived-classes-and-super-call` | `impl/derived`, cut at `f8406d5`, with the base run of `f8406d5` (`test262 total 53801 8982 7 44812 32275 12534 3`); dispatched on 2026-11-27, after 02:05 CET; its list has 331 cases; it is merged into `wave/big-2` when it is closed; green at `36b82da` (the part run reads `passed 2877 bound 2877 B 2581 F 296 R 0`); pass 1 of the adversary was green, 24 cases, with five notes (`review-result:adversary-derived-pass-1`): a syntax test that no longer held what its name said, three mutants no test held (the parent read after the arguments of a SuperCall, the step per argument of the default derived constructor, the hops of the kept `this` environment) and the cost of an arrow in a derived constructor (184 bytes per kept construction); correction 1 (`fe1f267`, tests only) keeps the adversary's five cases, adds two tests (26 in all) and renames the syntax test; `6ae01278` merges `wave/big-2` with the unit `fields` (three conflicts, both sides kept); read by the coordinator (`review-result:derived-correction-1-coordinator-read`); merged at `5120c89` on 2026-11-27, 03:54 CET; no committed test holds a field of a derived class yet, it comes with the unit `privatemethods` |
| 8.3 | fields | `story:public-class-fields` | `impl/fields`, cut at `f8406d5`, with the base run of `f8406d5`; dispatched on 2026-11-27, after 02:05 CET; its list has 515 cases; it is merged into `wave/big-2` when it is closed; green at `52307e7b` (the part run reads `passed 1038 bound 1037 B 522 F 515 R 0`); pass 1 of the adversary was red in 1 of 14 cases (`review-result:adversary-fields-pass-1`): a direct eval of `arguments` in an initializer threw a ReferenceError, and three mutants were unheld (a computed key made at run time, the error sites of a field, a literal `__proto__` field); correction 1 (`3986086`) refuses a direct eval in a field initializer until `story:eval-in-class-field-initializers`, adds four tests (17 in all) and runs the 100-field memory test at 3,000 constructions per round at its full count (2.05 CPU s; the story's 10,000 took 6.48); read by the coordinator (`review-result:fields-correction-1-coordinator-read`); merged at `9cfffd9` on 2026-11-27, 03:37 CET |
| 8.3 | privatemethods | `story:private-names-and-private-methods` | `impl/privatemethods`, cut at `f8406d5`, with the base run of `f8406d5`; dispatched on 2026-11-27, after 02:05 CET; its list has 1,977 cases; it is merged into `wave/big-2` when it is closed; green at `a3089e7`; pass 1 of the adversary was red (`review-result:adversary-privatemethods-pass-1`: the Proxy memory sweep at two realm sizes, the room of an instance's list, six findings), correction 1 at `e11fb57` (exact room per class level, the sweep's base read under its own budget, the merge of the units `fields` and `derived`); pass 2 was red (`review-result:adversary-privatemethods-pass-2`: the refusal of a private name in direct eval code rewrote the early error of undeclared names, 7 cases of test262 regressed; a refusal counted beside a thrown error), correction 2 at `d99234d` (refused only when an enclosing class declares the name; the part of 242 files: 211 of 445 runs pass, 193 on the base, none lost; the story's list and guard 2,500 of 2,500); read by the coordinator (`review-result:privatemethods-correction-2-coordinator-read`), whose read sent back one branch without a test, held by correction 3 at `3df8b6b`; merged at `6fcc0f2` on 2026-11-27, 06:18 CET |
| 8.4 | staticblocks | `story:class-static-blocks` | `impl/staticblocks`, cut at `d90c333` after the merge of the unit `privatemethods`; dispatched on 2026-11-27; green at `d61a063` (19 tests; its list of 484 cases: 484 passed, 448 on the base); pass 1 of the adversary dispatched; pass 1 was green (`review-result:adversary-staticblocks-pass-1`: three notes, four planted defects red in both builds, one change of the code no test caught), correction 1 at `5f8a7d3` (a test for a method between static fields, red with the defect planted; 20 tests); read by the coordinator (`review-result:staticblocks-correction-1-coordinator-read`); merged at `a84725a` on 2026-11-27, 07:52 CET |
| 8.4 | evalfields | `story:eval-in-class-field-initializers` | `impl/evalfields`, cut at `6f21df6d`, with the base run of `6f21df6d`; dispatched on 2026-11-27, 04:06 CET, once the two stories it depends on were merged (`fields`, `derived`); its list has 84 cases, of which 20 runs fail on the base (`passed 2610 bound 2610 B 2610 F 20 R 20`); it is merged into `wave/big-2` when it is closed; green at `5451f25` (13 tests; the part run reads `passed 2630 bound 2630 B 2610 F 20 R 0`); pass 1 of the adversary was green with no failing case (`review-result:adversary-evalfields-pass-1`) and left three single-line changes of the code that every test passed (the flag also set in a method, a function declaration looked into by the walk of ContainsArguments, the walk's stack not credited) and one note (the walk's error is answered before a held refusal of the eval code); correction 1 (`6547693`, tests and one doc sentence) holds each of the three with a test (14 tests under `class_field_eval::`, one more in the syntax suite); read by the coordinator (`review-result:evalfields-correction-1-coordinator-read`); merged at `0e336fd` on 2026-11-27, 05:09 CET |
| 8.5 | privatefields | `story:private-fields-and-private-in` | `impl/privatefields`, cut at `a67d445` (after the merge of the unit `overrides`) with `impl/staticblocks` at `d61a063` merged in, base `8857ac25`, with the base run of that commit (`test262 total 53801 8982 7 44812 35989 8821 2`); dispatched on 2026-11-27 while the unit `staticblocks` is in its adversary pass; it is merged into `wave/big-2` only after `impl/staticblocks` is merged there; pass 1 was green (`review-result:adversary-privatefields-pass-1`: no failing case, eight findings on what the tests hold), correction 1 at `b30b601` (four tests, each red with its defect planted: the room of a level, the steps of `#x in`, the static fields left out of an instance's room, the static level reserved once; 17 tests); read by the coordinator (`review-result:privatefields-correction-1-coordinator-read`); merged at `91a6b56` on 2026-11-27, 10:29 CET |
| 8.6 | evalprivate | `story:private-names-in-eval-code` | `impl/evalprivate`, cut at `bc45e92` with `impl/privatefields` at `3ebbc21` merged in, base `82688ae7`, with the base run of that commit (`test262-slice 08 9302 18455 0 0 9302 9281 21 0`); dispatched on 2026-11-27 while the unit `privatefields` is in its adversary pass; it is merged into `wave/big-2` only after that unit; built at `f6f4087`, its list of 2,653 cases passes (`passed 2653 bound 2653 B 2641 F 12 R 0`: the 12 cases that failed on the base pass); the adversary's pass 1 was red (`review-result:adversary-evalprivate-pass-1`: every direct eval read every named slot of its caller's records, 992 steps in a function of 1,000 `var`s); correction 1 at `5b6335d` walks only for an eval String that holds a `#` code unit; pass 2 was green with one warning (`review-result:adversary-evalprivate-pass-2`: the budget sweeps began after the walk's records); correction 2 at `0cbe31e` changes tests and comments only: five sweeps from the walk's first record to the answer, each named after what stops in its range, the adversary's six cases, the clause numbers of PerformEval read from the pinned text, and a library test of the syntax crate that was red on the wave since the unit `privatefields`; read by the coordinator (`review-result:evalprivate-correction-2-coordinator-read`); merged at `79ca8d3` on 2026-11-27, 11:41 CET |
| 8.6 | hook | `story:host-ensure-can-add-private-element` | `impl/hook`, cut at `bc45e92` with `impl/privatefields` at `3ebbc21` merged in, base `bcc10a4e` (the tree of the base of `evalprivate`), no base run: the story has no case of test262; dispatched on 2026-11-27 beside `evalprivate`; it is merged into `wave/big-2` only after the unit `privatefields`; built at `bd1dc46` (10 tests); the adversary's pass 1 was red (`review-result:adversary-hook-pass-1`: one failing case, a hook that refuses an object between the halves of an accessor after it allowed it, and five findings on what the tests hold); correction 1 at `167005b` states the host's same-kind duty in the hook's text and in HOST-119 and HOST-223, makes the case hold the behaviour stated and takes the adversary's cases in, with five planted defects each red (18 tests and two library tests); read by the coordinator (`review-result:hook-correction-1-coordinator-read`); merged at `43284b7` on 2026-11-27, 10:38 CET |
| 8.7 | close8 | `story:slice-08-conformance-close` | waits for every other story and for the close of slice 9 |

`aep plan artifact waves --status active`, read after the moves (aep 0.80.0): the verb puts
each of the three stories of order 8.1 in a wave of its own, since they name the same files
of the syntax and compiler crates (`parser/mod.rs`, `parser/expression.rs`, `ast.rs`,
`early.rs`, `compiler/src/expression.rs`, `resolve.rs`). The decomposition runs them side by
side: each story names the function or the arm it writes in a shared file and the fix at the
merge ("keep both"), and the parallel-safety critic read each pair in two rounds
(`review-result:slice-08-round-2-parallel-safety`; the reader of revision 2 found no site
with two owners). Between a story of slice 8's order 1 and a story of slice 9's order 2 the
verb names two shared files: `crates/swell/tests/main.rs`, which is generated, and
`crates/swell/src/interp/mod.rs` (`story:proxy-objects-and-access-traps` and
`story:super-property-and-super-syntax`). The unit that merges second merges the wave
branch into its own first and runs its checks again.

The base run of the join of order 1 (`dc310c6c`, keyed and reflect merged), one job on the
build node, output under `.out/test262-base`: `test262 total 53801 8982 7 44812 26250 18560
2`, `test262-slice 09 1785 3541 0 0 1785 480 1305 0`, 85,556 run lines. The plan of slice 8
changes no file a test262 run reads, so the run is the base of the units cut at `3237cf33`
and after.

## Why the units wait for one unit of wave big-1

`story:buffer-growth-outside-the-charged-list-is-listed` (unit `growth` of wave big-1) makes
clippy refuse a growth call of a buffer outside the charged list unless the call carries an
attribute with its charge or its bound. Every story of slice 9 writes such calls. A unit that
starts on a base without that lint would write code the gate refuses after the merge, so the
units start on a base that holds it.

The unit returned green at `3a16be13` while its adversary passes were still to
run. Its branch was merged into this branch then (`58501bbf`), ahead of its merge into
`wave/big-1`, so that order 1 starts without waiting for those passes: the list of
`crates/swell/clippy.toml` (15 methods that reserve room in a `Vec`, `VecDeque` or `String`)
and the attribute form are what a unit of this wave needs. A correction of that unit reaches
this branch with the next merge of `wave/big-1`. `slot task clippy` on `58501bbf` exits 0.

## The base run

`task test262-expected` on this branch at `92ea4830`, with its output under
`.out/test262-base` and the committed files untouched (one job on the build node):
`test262 total 53801 8982 7 44812 26026 18784 2`, `test262-slice 09 1785 3541 0 0 1785 268
1517 0`, `test262-slice 04 1612 3184 0 0 1612 1609 3 0`, 85,556 run lines. The unit `growth`
adds attributes and no behaviour, so the run is the base of the units cut at `58501bbf`. Over
it the check of `story:reflect-namespace` reads `passed 517 bound 517 B 517 F 212 R 212` and
296 runs `Reflect is not defined` (the story, on an older base: 516, 213 and 296).

## What the store's check said

`aep plan artifact waves --kind story --status draft`, read for the ten ids before the moves
(aep 0.80.0; the check was computed by the verb):

- The verb puts `keyed-entry-table` and `reflect-namespace` in its wave 1 and each other story
  in a wave of its own, nine waves, because it separates two stories that name one file.
- The decomposition puts two or three stories side by side in the orders 2, 3 and 4. For every
  file two stories of one order write, each story names the existing entry it inserts after,
  and the two entries differ; the parallel-safety critic checked each pair against the code in
  round 2 (`review-result:slice-09-round-2-parallel-safety`).
- `crates/swell/tests/main.rs` and `website/data/status.json` are generated; the coordinator
  generates them again after each merge.
- The coordinator alone writes `CHANGELOG.md`, `AGENTS.md`, `README.md`, `Cargo.lock`, the
  epic, `conformance/slices-delivered.txt`, this page and everything under `.engineering/`.
  A unit hands its changelog entry over in its report.

## A design change before order 4

`story:host-object-overrides` names the requirement HOST-222 (an override whose answer holds
a stale handle or a handle of another agent ends the run with `Terminated(EngineFault)`). The
sentence is written into `docs/design/host-interface.md` and
`conformance/host/requirements.txt` (`HOST-222 pending slice 9`) by a unit of its own before
the story starts, as HOST-214 was written before `math-random`.

Done: unit `host-222`, `impl/host-222` at `248177d2`, merged at `f3d194e2`. It changes two
files and no code; the coordinator read its diff in place of an adversary pass
(`review-result:host-222-coordinator-read`). `task host-requirements-check` reads
`222 requirement(s), 222 line(s): 165 test, 57 pending, 0 omitted`. Until the join with wave
big-1, `task host-surface-check` and `task site-data-check` fail on this branch:
`CHANGELOG.md` does not name HOST-222 yet and `website/data/status.json` holds 221
requirements; both files change in wave big-1's close too and are written here after it.

## The commits of this wave

The unit commits on the unit branches, the merges into `wave/big-2`, the merges of
`wave/big-1` into it, the coordinator's commits on `wave/big-2`, and the pushes of
`wave/big-2`, with one pull request. No tag, no release, no merge into `main` by the
coordinator.

The wave has one pull request. It is opened at the first pushed head of `wave/big-2` on which
the lane `slow` is measured on the forge at 900 s or less
(`approval-record:wave-big-2-slow-lane`). The earlier plan of 2026-11-27 was two pull
requests, the first with the orders 1 to 3 of slice 9, the orders 8.1 and 8.2 of slice 8 and
the units of the lane `slow` (twelve stories closed in the store). The forge's run on that
head read 1,137.3 s (below), so no pull request was opened there; the units of order 8.3 and
after are merged into `wave/big-2` as they close, and a fourth unit of the lane brings the
forge's run under the bound first.

## The gate's lines, what went wrong, where each finding went

This section is the close of the wave's first part: the orders 1 to 3 of slice 9, the orders
8.1 and 8.2 of slice 8 and three units of the lane `slow`. The units after it get their own
section below it.

### The gate

`slot --timeout 30m task check-runner` on `wave/big-2` at `8ab9456a`, on the build node,
2026-11-27, 02:37 to 02:44 CET: exit 0. Its lines are in `AGENTS.md`, "The gate's numbers";
the ones this wave moved:

```
nostd-symbols: 6 libraries, 57 object(s), 9785 undefined symbol(s): 9640 Rust's own, 57 memory functions, 88 compiler runtime routines; none from a C library or the system
host-requirements: 222 requirement(s), 222 line(s): 167 test, 55 pending, 0 omitted; 53 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 8:1 9:5 11:32 12:3 13:2 14:6 16:5 18:1
host-surface: 2 change(s) since 0.12.0, each named
19 test binaries: 2243 passed, 0 failed, 1 ignored
test-collect: 2 test binaries: 1758 passed, 0 failed, 1 ignored
test262-refused 13154
test262 total 53801 8982 7 44812 32455 12355 2
test262-slice 08 9302 18455 0 0 9302 4801 4501 0
test262-slice 09 1787 3545 0 0 1787 1713 74 0
gate-selftest: 99 passed, 0 failed
gate-time test-steps 56.4 of 120, cpu 431.5, load 26.08 on 32 cores
```

On `main` before the wave the total line read `test262 total 53801 8982 7 44812 26026 18784
2`, slice 8 passed 5 of its cases and slice 9 268, `test262-refused` read 23158, the 19 test
binaries ran 1,774 tests and `test-collect` 1,339. The expected file and its digest are those
of one full run on the branch at `758ad532` (`task test262-expected`, exit 0, sources digest
`07fe9eca...`); the commit `8ab9456a` holds the files that run wrote and no other change. On
that run every case of the eleven case lists of the delivered
stories passes in each mode it runs in (`docs/plan/slice-08/check.sh` and
`docs/plan/slice-09/check.sh` against the run of the branch at `f8406d5`), and no run that
passed there fails.

The lane `slow` as one job on the same head and node, 02:44 to 02:52 CET: exit 0.

```
slow-time slow-test262 280.1
slow-time slow-tests 84.8
slow-time test-realm-sizes 91.5
slow-time stack-profiles 16.9
slow-time lane 473.3 of 900, longest part slow-test262 280.1 of 600
realm-sizes: 0 run(s) red
```

The bound of the story of the lane is 900 s on the forge's runner, which is slower than the
node; that run is made by hand on the pushed branch (`.github/workflows/slow.yml`,
`workflow_dispatch`).

The forge's run on `158888d9` (run 45925590365, 2026-11-27, the step `task slow` from
02:58:45 to 03:17:43 CET) ended red at `slow-time-sum` and at no test: `slow-tests: 4 test
binaries: 976 passed`, the collect run `1304 passed`, `realm-sizes: 0 run(s) red`, the
test262 file equal byte for byte. Its lines:

```
slow-time slow-test262 419.3
slow-time slow-tests 268.4
slow-time test-realm-sizes 431.2
slow-time stack-profiles 18.4
slow-time lane 1137.3 of 900, longest part test-realm-sizes 431.2 of 600
```

Each part is under 600 s; the lane is 237.3 s over its 900 s. The forge's runner is slower
than the node by a factor that differs per part (1.5, 3.2, 4.7 and 1.1 for the four lines
above), so the node's 473.3 s did not show it. In the run: the seven collect runs of
`test-realm-sizes` take 37.9 to 60.2 s each (about 312 s) and the seven default runs 16.1 to
17.8 s; slice 05 of the test262 run holds 1,167,844 ms of the run's test time. The fourth
unit of the lane (`impl/lanefour`, cut at `158888d9`) works from these lines; no pull request
is opened before a forge run at 900 s or less.

The gate was run on `8ab9456a`. The commits after it on this branch up to the first push
change `AGENTS.md`, `README.md`, this page and the store; the gate is run once more on the
head that is pushed.

### What went wrong

- The lane `slow` took 22 min 35 s on the forge for the pull request of wave big-1, over the
  two minutes the test-budget rule gives a step. It got the story
  `story:realm-size-lane-within-its-share` with a bound of 900 s for the lane and 600 s for a
  part (`approval-record:wave-big-2-slow-lane`) and was ordered ahead of the feature units.
  Three units delivered it: `slowlane` (the parts are timed and bounded), `lanebuilds` (the
  two profile builds are shared, the stack tests are built with their modules only) and
  `lanetests` (the tests over 2 s in the lane's collect build run smaller counts there and
  their earlier counts with `SWELL_FULL_COUNTS=1`).
- The merge of the unit `weak` (`6e9b7fa`) made one stack test overflow its thread in the
  profile `stack-debug`: two more arms grew the frame of `Agent::run_builtin` from 1,360 to
  1,424 bytes. The gate does not run that profile, so the branch was red in the lane from
  00:36 to 01:56 CET, and two units were verified on a base without the correction, with
  the one red test named as known. The unit `stackjoin` fixed it (`2b5c3b8`); its first change was sent back by the
  coordinator because it grew the routes of 18 groups of builtins. `AGENTS.md` now says that
  a change under `crates/swell/src` runs the stack lane on its merged head.
- Two committed files named a decision by a number of the coordinating session: a comment of
  `crates/swell/src/interp/class.rs` (from the merge of the unit `classes`) and a summary
  line of `taskfiles/gate-time.yml` (from the unit `slowlane`), and two texts of the store
  beside the operator's decision. The unit `marks` and one commit of the coordinator replaced
  them by the rule in words. A line of `approval-record:wave-big-2-slice-8` that quotes the
  operator's answer keeps its mark, since it is a quotation.
- A realm reads larger than in `0.12.0`: `heap_bytes()` after `create_realm` 389,722 bytes
  against 357,960 (the head word per arena slot of the collector's workspace, and the
  intrinsics of this wave). No bound was moved; `CHANGELOG.md` states both readings.
- `story:realm-walk-in-parts-under-the-cpu-bound` was accepted in this wave and is not
  delivered in this pull request: the walk is still one test, 10.37 s in the collect run of
  the lane's part `slow-tests`. Its unit follows the first part of the wave.
- In the collect run of the lane's part `slow-tests` two tests took over 30 s at their full
  counts (`builtins_json_parse::parse_gives_back_its_charges_on_every_way_out` 34.30 s and the
  library test `object::builtin_function::tests::holders_give_the_kept_stack_back_its_room_across_a_call`
  33.93 s), under the 60 s at which a test is ended and over the 2.5 s the wave gave a full
  count. The drafts `story:full-count-tests-fit-the-time-bound-in-the-collect-build` and
  `story:library-tests-fit-the-time-bound-in-the-collect-build` hold them.

### Where each finding went

- Eighteen adversary passes on fifteen units (`review-result:adversary-<unit>-pass-1` for
  each of `keyed`, `reflect`, `mapset`, `proxy`, `weak`, `setmethods`, `proxytraps`,
  `newtarget`, `super`, `classsyntax`, `classes`, `elements`, `slowlane`, `lanebuilds` and
  `lanetests`, and `-pass-2` for `keyed`, `classes` and `proxytraps`). Three units got a second
  pass; for the other twelve the coordinator read the correction's diff in place of it,
  and the record of the read says what the diff changed. Every finding introduced by a unit was corrected by that unit
  before its merge; each story carries the outcome of each pass.
- Nine passes say red in their title or added a red case: pass 1 on `keyed`, `classes`,
  `elements`, `weak`, `setmethods`, `lanebuilds` and `lanetests`, and both passes on
  `proxytraps`. Three of them showed a defect a script reaches: `classes` pass 1 (class
  code in non-strict code ran its computed keys as non-strict code; an anonymous class
  under a computed key was named after its elements ran), `proxytraps` pass 1 (`for`-`in`
  and JSON lost values a trap had made once a later trap ran a collection; `for`-`in`
  dropped the keys of a prototype a trap answered) and `proxytraps` pass 2 (a
  `getPrototypeOf` trap that answers differently on its second call). Each is corrected and
  held by a test; the rows above and the records name the commits.
- Twenty reads by the coordinator are in the store (`review-result:<unit>-correction-<n>-coordinator-read`,
  `review-result:<unit>-coordinator-read` for the units `host-222` and `randomcases`, which
  had no adversary pass, and `review-result:stackjoin-coordinator-read-1`, which sent a
  change back).
- Findings older than the units, or outside a unit's story, are backlog drafts that no wave
  takes before the planned waves are delivered:
  `story:a-limit-of-an-integer-width-has-a-test-or-a-stated-reason`,
  `story:a-string-the-host-makes-is-not-held-to-string-length`,
  `story:clippy-checks-the-collect-feature`,
  `story:full-count-tests-fit-the-time-bound-in-the-collect-build`,
  `story:gate-time-sum-counts-every-step`,
  `story:host-133-compares-the-hooks-it-names`,
  `story:host-206-holds-the-charge-of-the-request-queue`,
  `story:library-tests-fit-the-time-bound-in-the-collect-build`,
  `story:promise-closure-boxes-are-charged-in-a-library-test`,
  `story:stack-bound-parts-fit-their-cpu-bounds` and
  `story:stack-tests-hold-in-the-stack-debug-profile`.
- `story:instance-elements-initialization-order` was drafted in the planning of slice 8 and
  archived after round 1 of the plan critics: its 448 cases went to
  `story:private-fields-and-private-in` (446) and `story:public-class-fields` (2).

### The stories

Implemented with this pull request, each with a `test_result` that names the gate run above
and the commit that merged its unit: `story:keyed-entry-table`, `story:reflect-namespace`,
`story:map-and-set`, `story:proxy-objects-and-access-traps`, `story:weak-map-and-weak-set`,
`story:set-methods`, `story:proxy-descriptor-prototype-and-extensibility-traps`,
`story:new-target-meta-property`, `story:super-property-and-super-syntax`,
`story:class-syntax-and-early-errors`, `story:class-definition-evaluation` and
`story:class-element-syntax-and-early-errors`.

`story:realm-size-lane-within-its-share` stays active until the forge's run of the lane is
read. Slice 8 and slice 9 are not delivered: `conformance/slices-delivered.txt` is unchanged,
and the remaining stories of both epics follow on this branch.

## The second part: the gate's lines, what went wrong, where each finding went

This section is the close of the wave's second part: the orders 8.3 and 8.4 of slice 8, the
order 4 of slice 9, the fourth unit of the lane `slow` and the unit of the realm walk. No
pull request was opened on the first part, because the forge's run of the lane read
1,137.3 s of its 900 there; the wave's first pull request is opened on the head of this
part and holds both. Where the section above says "this pull request", it is that one. The
units after this part (the orders 5 of slice 9 and 8.5 to 8.7 of slice 8, and a fifth unit
of the lane) follow as a second pull request.

### The gate

`slot --timeout 30m task check-runner` on `wave/big-2` at `b9adbe35`, on the build node,
2026-11-27, 08:54 to 09:00 CET: exit 0. Its lines are in `AGENTS.md`, "The gate's numbers";
the ones this part moved:

```
nostd-symbols: 6 libraries, 57 object(s), 10248 undefined symbol(s): 10106 Rust's own, 55 memory functions, 87 compiler runtime routines; none from a C library or the system
host-requirements: 222 requirement(s), 222 line(s): 172 test, 50 pending, 0 omitted; 53 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 8:1 11:32 12:3 13:2 14:6 16:5 18:1
host-surface: 17 change(s) since 0.12.0, each named
19 test binaries: 2445 passed, 0 failed, 1 ignored
test-collect: 2 test binaries: 1944 passed, 0 failed, 1 ignored
test262-refused 5369
test262 total 53801 8982 7 44812 36027 8783 2
test262-slice 08 9302 18455 0 0 9302 8242 1060 0
test262-slice 09 1787 3545 0 0 1787 1765 22 0
gate-selftest: 104 passed, 0 failed
gate-time test-steps 65.5 of 120, cpu 315.0, load 14.17 on 32 cores
```

At the first part the total line read `test262 total 53801 8982 7 44812 32455 12355 2`,
slice 8 passed 4,801 of its cases and slice 9 1,713, and `test262-refused` read 13154. The
expected file and its digest are those of one full run on the branch at `0a290c5c` (`task
test262-expected`, exit 0, sources digest `bb23016e...`); the commit `b9adbe35` holds the
files that run and `task site-data` wrote and no other change.

The lane `slow` as one job on the same head and node, 09:00 to 09:08 CET: exit 0.

```
slow-time slow-test262 272.8
slow-time slow-tests 76.9
slow-time test-realm-sizes 103.8
slow-time stack-profiles 0.9
slow-time lane 454.4 of 900, longest part slow-test262 272.8 of 600
realm-sizes: 0 run(s) red
```

The forge's run on `b9adbe35` (run 45963552396, started by hand on the pushed branch,
2026-11-27, 09:09 to 09:24 CET,
<https://github.com/beyond10x/swell/actions/runs/45963552396>) ended green: `slow-tests: 4
test binaries: 1067 passed`, the collect runs `383 passed` at opt-level 0 and `1066 passed`
at opt-level 1, `realm-sizes: 304 of 1644 tests`, `realm-sizes: 0 run(s) red`, the build of
the profile `collect` 382.2 s beside the first part with `slow-tests` waiting 0 s. Its
lines:

```
slow-time slow-test262 504.8
slow-time slow-tests 170.4
slow-time test-realm-sizes 185.2
slow-time stack-profiles 22.2
slow-time lane 882.6 of 900, longest part slow-test262 504.8 of 600
```

The lane is 17.4 s under its 900 s and 254.7 s under the run on `158888d9`; each part is
under 600 s. The pull request's own run of the lane (`slow.yml`, on `pull_request`) measures
it once more on the head that is pushed.

The gate was run on `b9adbe35`. The commits after it on this branch up to the push change
`AGENTS.md`, `README.md`, this page and the store; the gate is run once more on the head
that is pushed.

### What went wrong

- The first part had no pull request for one forge run: the lane read 473.3 s on the build
  node and 1,137.3 s on the forge's runner, and the bound is the forge's. The fourth unit of
  the lane, `lanefour`, was cut on that run. Its own aim was 800 s or less on the forge and
  its estimate for the merged tree about 848 s; the forge measured 882.6 s. The bound of
  `story:realm-size-lane-within-its-share` holds with 17.4 s of margin.
- On that margin the operator took the backlog draft
  `story:property-escape-cases-spend-less-in-the-interpreter-loop` into the wave as a fifth
  unit of the lane (`approval-record:wave-big-2-lane-unit-5`): the property-escape cases of
  test262 are 73 % of the CPU seconds of the test262 run. It is not in this pull request.
- The first unit of `story:realm-walk-in-parts-under-the-cpu-bound` stopped at the story's
  stop rule: the list of the realm's 344 functions alone took 2.33 CPU s in the lane's
  collect build, and every part of the walk would have paid it. The story was written again
  around a committed file of the places; three plan critics asked for a revision in both of
  their reads, and the coordinator decided what was left
  (`review-result:realmwalk-plan-coordinator-read`). The form of the file was put to the
  operator and taken on the default of the rule "A default proceeds" (the row of the unit
  says which).
- The unit `overrides` was red in both adversary passes on memory the engine holds across a
  host's override: a descriptor held across an `is_extensible` override was freed by a
  collection, and the object's record answered for an override the table leaves out in four
  places that broke an invariant of the essential internal methods. Both are corrected and
  held by tests before the merge.
- The unit `privatemethods` lost 7 cases of test262 in its first correction (the refusal of
  a private name in direct eval code rewrote the early error of an undeclared name); pass 2
  of the adversary found it, correction 2 gave them back, and none was lost at the merge.
- The unit `fields` threw a ReferenceError for a direct eval of `arguments` in a field
  initializer; its correction refused a direct eval there, and the unit `evalfields` of the
  same part runs it.
- A test that reads wall time failed once in the lane on a loaded node
  (`class_elements_scale_linearly` of `crates/syntax/tests/sources.rs`) and passed when run
  again. The draft `story:class-elements-scale-test-reads-no-wall-time` holds it.
- The subject line of the commit `b9adbe35` has 75 columns, over the 72 this repository's
  commits keep. It was pushed before it was seen, and a pushed commit is not rewritten.
- The forge's `task check` on the pull request's head `d16fe575` went red after three green
  runs of the same gate on the build node: `gate-selftest: 103 passed, 1 failed`, the planted
  script `slow-collect-builds-beside-the-lane-and-ends-with-it` ("task slow ended without
  slow-collect-stop ending the build"). The forge's `sh` is dash and the build node's is
  bash. The lane's 17 kills of a process group had two dashes after the signal, which dash's
  `kill` reads as a process number, so on the forge `slow-collect-stop` and the build's
  watcher ended nothing. The unit `lanefix` wrote the kills in the form both shells read and
  added the static check `kill-check` with its planted script, so the local gate refuses
  that form (`gate-selftest: 105 passed`; merged at `175c17f` on 2026-11-27, 10:14 CET;
  `review-result:lanefix-coordinator-read`). No dash is installed where the unit ran: the
  forge's run of the corrected head is the check.
- The forge's run of the lane `slow` on `d16fe575` read `slow-time lane 1142.0 of 900,
  longest part slow-test262 634.0 of 600`, on code that read 882.6 s one run before (the two
  heads differ in no file of code): single tests ran 1.4 to 1.56 times slower, builds 1.11
  to 1.14 times, on the same runner image. The forge's log does not name the runner's CPU;
  from `lanefix` on the lane prints it (`slow-runner`).

### Where each finding went

- Twelve adversary passes on nine units (`review-result:adversary-<unit>-pass-1` for each of
  `derived`, `fields`, `privatemethods`, `staticblocks`, `evalfields`, `upsert`,
  `overrides`, `realmwalk` and `lanefour`, and `-pass-2` for `privatemethods`, `overrides`
  and `lanefour`). Three units got a second pass; for the other six the coordinator read the
  correction's diff in place of it. Every finding introduced by a unit was corrected by that
  unit before its merge; each story carries the outcome of each pass.
- Seven passes are red: pass 1 on `fields`, and both passes on `privatemethods`, `overrides`
  and `lanefour`. Three of them showed a defect a script or a host reaches: `fields` pass 1
  (the direct eval of `arguments`), `overrides` pass 1 (the freed descriptor) and
  `overrides` pass 2 (the four invariants). The five green passes left mutants no test held
  (three on `derived`, three single-line changes on `evalfields`, two planted defects on
  `upsert`, one on `staticblocks`); each is held by a test of the unit's correction.
- Ten reads by the coordinator are in the store: `review-result:<unit>-correction-1-coordinator-read`
  for `derived`, `fields`, `evalfields`, `staticblocks`, `upsert` and `realmwalk`,
  `-correction-2-coordinator-read` for `privatemethods`, `overrides` and `lanefour`, and
  `review-result:realmwalk-plan-coordinator-read`. The read of `privatemethods` sent back
  one branch without a test (correction 3).
- Findings older than the units, or outside a unit's story, are backlog drafts that no wave
  takes before the planned waves are delivered:
  `story:class-elements-scale-test-reads-no-wall-time`,
  `story:gate-refuses-an-exported-rustflags` and
  `story:realm-walk-reaches-hidden-intrinsics-and-holds-type-errors`. A fourth,
  `story:moves-check-sees-a-block-move-without-its-line`, is from the first adversary pass
  on the open unit `close9`.

### The stories

Implemented with the pull request, each with a `test_result` that names the gate run of this
section and the commit that merged its unit: `story:public-class-fields`,
`story:derived-classes-and-super-call`, `story:eval-in-class-field-initializers`,
`story:private-names-and-private-methods`, `story:class-static-blocks`,
`story:map-upsert-and-group-by`, `story:host-object-overrides`,
`story:realm-walk-in-parts-under-the-cpu-bound` and
`story:realm-size-lane-within-its-share`, whose `test_result` also names the forge's run
above. With the twelve of the first part the pull request delivers 21 stories.

Slice 8 and slice 9 are not delivered: `conformance/slices-delivered.txt` is unchanged. Open
on this branch: `story:private-fields-and-private-in`, `story:private-names-in-eval-code`,
`story:host-ensure-can-add-private-element`, `story:slice-09-conformance-close`,
`story:slice-08-conformance-close` and the fifth unit of the lane.

## The close of slice 9

`story:slice-09-conformance-close`, the unit `close9`, on `wave/big-2` after the merge of the
unit `upsert` (`bc45e92`) and, after its correction 1, of `lanefour` (`b9adbe35`). The full run
of its head (`task test262-expected`, exit 0) prints:

```
test262 total 53801 8982 7 44812 36027 8783 2
test262-slice 08 9339 18529 0 0 9339 8279 1060 0
test262-slice 09 1858 3678 0 0 1858 1858 0 0
test262-slice 12 381 760 0 0 381 39 342 0
test262-slice 19 1233 2299 0 0 1233 736 495 2
```

Every case of slice 9 passes in every run. 59 cases that need a later slice move by lines of
the block and of `conformance/test262/moves.tsv`: the 37 that need `class` or `new.target`
to slice 8, which takes them as `docs/plan/slice-08/incoming.tsv` plans, and the 22 that need
a BigInt literal to slice 12. 130 cases of `test/staging/` come in from slice 19
(`docs/plan/slice-09/close-into-09.tsv`). The run changes no outcome against the run of
`bc45e92`. With `9` delivered, `host-requirements-check` exits 0 (`pending by slice: 8:1 11:32
12:3 13:2 14:6 16:5 18:1`). The paragraph "Slice 9's cases that need a later slice" of section
12 of `docs/design/engine.md` gives the reasons.

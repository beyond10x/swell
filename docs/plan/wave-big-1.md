# Wave big-1: the end of slice 4, the core without std, static binaries

One wave of [the wave plan](waves.md), "After slice 4: bigger waves". Opened on 2026-11-26,
05:24 CET, on the operator's word of about 05:18 CET (`approval-record:wave-big-1`): one pull
request carries several stories. Skill version 0.26.0 of `aep:implementing`.

The wave branch is `wave/big-1`, cut from `main` at `b11998b5`, with the branch
`impl/nostd-core` (step 3 of `story:core-builds-without-std`) merged in as its first commit.

## The units

Each unit has a managed worktree (its id below), builds into `target/` inside that tree, and
keeps its work files in the directory `wave/<unit>` of the coordinating session's scratch
directory. The stage is kept current by the coordinator.

| unit | story | branch | worktree id | stage |
|---|---|---|---|---|
| core | `story:core-builds-without-std`, step 3 | `impl/nostd-core` | `swell-impl-nostd-core` | merged into the wave branch at `248a031c`; gate on that head: exit 0, `gate-time test-steps 106.5 of 120` at load 65 on the node |
| gate | `story:core-builds-without-std`, step 4 | `impl/nostd-gate` | `swell-impl-nostd-gate` | green at `c6a7f86c` (13 files; `nostd-build` and `nostd-symbols` are steps of `check`; `gate-selftest: 75 passed`); its two whole-gate runs: exit 201 with `gate-time test-steps 303.3 of 120` at load 59 on the node, then exit 0 with `81.6 of 120` at load 58; pass 1 (`review-result:adversary-gate-pass-1`): 6 failing cases, 10 findings (9 introduced, 1 found on the base too in `code_of`, 2 not reachable while the crates forbid unsafe code); correction 1 with the same implementor takes all ten but the first: the example host with its own allocator stays unbuilt (see the decision below) and the documents say what the emulated host is; the correction also names the bare target and `llvm-tools` in `.github/workflows/check.yml` and makes `forge-check` compare them with `rust-toolchain.toml`; correction 1 at `e2b7749b` (12 files): `source_scan` and `emulated_host` run 11 tests (6 before), `gate-selftest: 86 passed`; the unit also committed regenerated count files (`cc0eb380`), reverted at `4503978a` because the `close` unit writes them for the whole wave; verified by the coordinator on the build node at `4503978a`: `nostd-build: 6 libraries for x86_64-unknown-none`, `nostd-symbols: 6 libraries, 53 object(s), 9030 undefined symbol(s) ... none from a C library or the system`, both exit 0, and `origin/main` has neither task; pass 2, the last (`review-result:adversary-gate-pass-2`, at `4503978a`): needs change, 5 red cases, 7 findings; it closed nine findings of pass 1, and the tenth (the acceptance line on an example host) is answered by the rewritten line of the story on this branch; correction 2 with the same implementor takes the six others (the std scan blanks library code after a test-only item in two forms, `toolchain-check` reads only one-line double-quoted arrays, the emulated host's check passes the output macros of the std prelude, two sentences say more than is true); the coordinator reads its diff, no third pass; correction 2 at `659ee99a` (8 files): the five cases of pass 2 pass, `15 tests run: 15 passed` for `source_scan::` and `emulated_host::`, `gate-selftest: 88 passed, 0 failed`; read by the coordinator (`review-result:gate-correction-2-coordinator-read`, approve); merged at `cff4bbb5` |
| random | `story:math-random` | `impl/math-random` | `swell-impl-math-random` | green at `ed59cd6b`; pass 1 (`review-result:adversary-random-pass-1`): no failing case, 4 findings, 3 of them gaps in the unit's tests; correction 1 at `96fe0ca3` (six of the adversary's cases join the unit; the rotation mutant and the index mutant each fail their new test); the collect-mode counts stay as they are; pass 2 (`review-result:adversary-random-pass-2`): no failing case, 5 planted mutants each caught by a committed test, 1 note on the story's acceptance text (the coordinator's, rewritten at the close); its 4 further cases are not taken (each mutant is already caught, and the gate's test steps are near their bound); merged into the wave branch at `96fe0ca3` |
| static | `story:releases-ship-static-binaries` | `impl/static-binaries` | `swell-impl-static-binaries` | merged into the wave branch at `6256b323`; pass 1: 7 findings fixed; pass 2: 4 new, 0 carried, fixed; the coordinator read the second correction (the check executes nothing of a workflow; the unit's own script was pinned to the new message and got one case more, no assertion dropped) |
| exponent | `story:exponentiation-operator` | `impl/exponentiation` | `swell-impl-exponentiation` | green at `b7d34305` (20 files); the claim is verified by the coordinator with the story's `check.sh`: 70 of the 76 listed cases have a failing run on the base, none after, 950 guard cases pass before and after; pass 1 (`review-result:adversary-exponent-pass-1`): no defect, 12 cases added, 1 note (the unit's interrupt test cannot fail); correction 1 at `3771d526` (3 files): the interrupt now comes from another thread into a loop that calls nothing, and the test fails with `Terminated(StepBudget)` when the check at the loop jump is removed; the adversary's cases joined the existing test files, none dropped; pass 2, the last (`review-result:adversary-exponent-pass-2`): no defect, 8 mutants planted in a copy, 7 caught by committed tests, 1 not (a base in (-1, 0) to +Infinity answering -0; the adversary's case catches it), and 1 chain of `exponent_chains_are_bounded` listed twice; correction 2 at `c8f28a47` (2 test files, 23 insertions, 5 deletions): the case is kept with its assertions, the duplicate chain became the chain as the argument of a call; read by the coordinator, no third pass; merged into the wave branch at `992d1c8d` |
| growth | `story:buffer-growth-outside-the-charged-list-is-listed` | `impl/buffer-growth` | `swell-impl-buffer-growth` | cut from the wave branch after `random` and `exponent` were merged (it adds an attribute at growth calls all over `crates/swell/src`; the `gate` unit writes no file there, so it does not wait for it); dispatched on 2026-11-26, 07:12 CET; green at `3a16be13` (37 files): `crates/swell/clippy.toml` lists 15 reserving methods of `alloc::vec::Vec`, `alloc::collections::VecDeque` and `alloc::string::String`, 58 calls carry an `expect` with their charge or limit (54 in `src`, 4 in `tests`), `1322 tests run: 1322 passed` and `1318` in collect mode before and after; the `std::` path forms flag nothing in the library and `to_vec` cannot be listed; merged into `wave/big-2` ahead of its passes (`58501bbf` there); pass 1 (`review-result:adversary-growth-pass-1`): 6 findings, no case; 2 introduced (an `allow` on a listed call, and an `expect` without a reason, pass the gate's clippy command), 1 reason that is not true (`builtins/json/parse.rs:396`: the stack of open values keeps its capacity while the reviver runs), 3 notes on a charge made after its reservation that are older than the wave (filed as the draft `story:charge-comes-before-the-reservation`); correction 1 with the same implementor takes the first three; correction 1 green at `1d48bbcd` (5 files): `crates/swell/Cargo.toml` denies `allow_attributes` and `allow_attributes_without_reason`, four older attributes became an `expect` with a reason, `json_parse_with` frees the stack of open values before the reviver is called, `1322` and `1318` tests as before; the unit names one gap itself (an inner `#![allow(clippy::disallowed_methods, reason = "…")]` at the top of a module passes the gate's command); pass 2, the last (`review-result:adversary-growth-pass-2`): 5 findings, 1 green case; the gate stays green for a listed call under an inner `allow` with a reason in any position, under an `expect` of a group (`clippy::style`, `clippy::all`, `warnings`), under an `allow` from a macro of another crate of the workspace, and with an empty reason; the reason at `builtins/json/parse.rs:398` does not state the bound the code holds; the wave branch was merged into the unit branch (`08d26ffa`, the conflict of `source_scan.rs` resolved as on `wave/big-2`); correction 2 with the same implementor adds a scan of the source that refuses those forms, and the coordinator reads its diff; correction 2 green at `058eba11` (3 files, 631 lines added): two tests of `source_scan` read 212 files of the engine crate and 97 of the other crates and refuse the four forms, each shown red by a planted control, `1337 tests run: 1337 passed`; the coordinator read (`review-result:growth-correction-2-coordinator-read`) finds no defect; verified on the build node (clippy exit 0, `29 tests run: 29 passed`) and merged (`07c4a8f6`) |
| close | `story:slice-04-conformance-close` | `impl/slice-04-close` | `swell-impl-slice-04-close` | started from the wave branch at `f4759cfe` (merge `1a607d63`), with `random` and `exponent` merged; it does not wait for `gate`, which writes none of its files (two paragraphs of `docs/design/engine.md` outside section 12); dispatched on 2026-11-26, 07:12 CET; green at `e2ae99d5` (8 files): 3 cases moved out (2 to slice 9, 1 to slice 12), 72 cases of slice 19 taken in, `test262-slice 04 1681 3320 0 0 1681 1681 0 0`, the total line unchanged; pass 1 (`review-result:adversary-close-pass-1`): 4 findings, 2 red cases; one incoming case (`staging/sm/regress/regress-596805-2.js`) completes without the parts of slice 4 it names, so it passed at the start of the slice and does not come in by the rule of the closes of slices 7 and 10; the 66 cases said to have failed at the start were inferred from a difference of counts; 3 of the 9 references of the consumer test cannot fail without slice 4; correction 1 with the same implementor makes the test262 run at the start commit `a46fe885` (worktree `swell-run-slice-04-start`) and reads the incoming list case by case; correction 1 returned at `76195b35` (4 files): the run at `a46fe885` equals that commit's `expected.txt` byte for byte; every run of each of the 72 incoming cases failed there on a part of slice 4; one case leaves the list (`staging/sm/regress/regress-596805-2.js`, it passed at the start) and one comes in (`staging/sm/Number/conversion-invalid-precision.js`), so the counts stay (`test262-slice 04 1681 3320 0 0 1681 1681 0 0`, slice 19 at 1,363 cases and 2,550 runs); 98 cases of slice 9 that failed at the start and pass now are kept out by the line `feature Reflect.construct 9`; slice 19 passes 604 cases at the start and 676 on the unit's base, and the 72 between are the list; the consumer test has 3 tests and each of its 9 references fails without its part of slice 4; pass 2, the last (`review-result:adversary-close-pass-2`): the start run is the run of `a46fe885`, all 72 lines of the incoming list hold case by case, the 98 cases of slice 9 and the account of slice 19 hold, and every number equals the run; 5 notes and 2 red cases: the nine references of the consumer test cannot tell a function of `Math` from its rounding neighbour, two of them now call functions the consumer does not call (the story's line on the argument values is no longer true), and two sentences of the close text state more than the start run shows (41 of the 72 messages name no part; 92, not 93, cases); correction 2 with the same implementor: the consumer's script goes back to the consumer's calls and a second script carries the arguments that tell the functions apart; correction 2 returned at `11609c8a` (2 files); the coordinator read its diff (`review-result:close-correction-2-coordinator-read`) and found one sentence of the doc comment that was not true, corrected at `a1ca60ed`; verified by the coordinator on the build node at `a1ca60ed` (the six tests of `consumer_slice_04` in both modes, `test262-check`, clippy, fmt); merged into the wave branch at `a4c1b2e7` |

Dispatches use the agent type `aep:implementor` on Opus; the adversary is `aep:adversary`. The
two decomposers of slices 9 and 8 run as `general-purpose` on Opus with the procedure of
`aep:decomposer`, because that agent type has no tool that writes a file and the shell here
takes no heredoc.

## Why these stories

- `math-random` and `slice-04-conformance-close` are the two stories of slice 4 that are
  accepted and not built: `test262-slice 04 1612 3184 0 0 1612 1533 79 0` in
  `conformance/test262/expected.txt` on `main` (79 cases do not pass), and six lines
  `pending slice 4` in `conformance/host/requirements.txt`.
- `core-builds-without-std` and `releases-ship-static-binaries` were ordered by the operator
  (the approval record quotes the words); the record moved both from `draft` to `active`.
- `story:bench-image-and-first-measurements` is accepted and not built. It is not in this
  wave: its runs need the build node alone for the measurement, and its tree holds work that
  is not committed.

Added at 05:30 CET, from the reading of the store's close: `story:exponentiation-operator`
(the parser still refuses `**`; 1 of 44 cases of `language/expressions/exponentiation` passes)
and `story:buffer-growth-outside-the-charged-list-is-listed` (`docs/plan/slice-10-wave-1.md`:
"planned, not built") are accepted and not built. Both are stories of the plan before this
wave, and the close of slice 4 depends on the first.

## What the store's check said

`aep plan artifact waves --kind story --status active`, read for the wave's ids (aep 0.80.0;
the check was computed by the verb):

- `story:math-random` is placed in the verb's wave 11, `story:slice-04-conformance-close` in
  its wave 19: the close depends on every story of slice 4, so it runs after `random` in this
  wave and not beside it.
- The typed scopes of `core-builds-without-std` and `releases-ship-static-binaries` were
  written by the coordinator from the stories' bodies; their lines are `inferred` except the
  paths the bodies name.
- Files two units write: `crates/swell/tests/main.rs` (`random` and `gate`; the file is a
  generated list of modules, the coordinator generates it again after the merges);
  `conformance/host/requirements.txt` (`random` moves six lines, `gate` changes the line of
  `HOST-2` at most); `docs/design/engine.md` (`gate` and `close`; the close starts after the
  gate unit is merged).
- The coordinator alone writes `CHANGELOG.md`, `AGENTS.md`, `README.md`, `Cargo.lock`,
  `conformance/test262/expected.digest`, this page and everything under `.engineering/`. A
  unit hands its changelog entry over in its report.

## A decision the coordinator took

The acceptance of `core-builds-without-std` asks for an example host built without `std`
"with a fixed block as its allocator". A global allocator is an `unsafe impl` in Rust, and a
program without `std` needs an `unsafe` entry attribute; `AGENTS.md` forbids `unsafe` in every
crate, and the operator decided on 2026-11-26 that the engine takes no `unsafe`, also not for
speed. The unit therefore proves the same thing without `unsafe`: the six crates are built
for the bare target, and a step of the gate reads the undefined symbols of those six
libraries and fails on any symbol of a C library (a float function, an I/O call). The
emulated test host (a fixed clock, a seeded generator) runs on the workstation's target and
names no system API. The story's acceptance line is rewritten at the close with this reason.

## The store's close

Stories that are delivered on `main` and still `active` in the store get their evidence (the
commit that delivered them and the gate's line) and move to `implemented` in this wave. The
list with its commits is made by a reading agent and checked by the coordinator before any
move.

The reading agent found 42 stories delivered. The coordinator's check on the wave branch
(the commit is an ancestor of the branch, the story is `active`, the named test or file is in
the tree): 40 hold; for one of them (`story:async-function-constructor`) the agent named the
wrong file and the test is at `crates/swell/tests/dynamic_function.rs:689`. The other two
(`story:release-0-12-0-with-binaries`, `story:gate-selftests-write-no-tracked-file`) are
delivered and still `draft`; they are not moved by this check.

The 40 stories are `implemented` in the store since 2026-11-26, 06:55 CET. Each carries one
`test_result` record: its source is the gate on `wave/big-1` at `248a031c` (exit 0) with the
commit that delivered the story, an ancestor of that head; its reference is the test or file
the check found in the tree.

## The commits of this wave

The unit commits on the unit branches, the merges into `wave/big-1`, the coordinator's commits
on `wave/big-1`, one push of `wave/big-1` and its pull request. No tag, no release, no merge
into `main` by the coordinator.

## The gate's lines, what went wrong, where each finding went

### The gate

`slot --timeout 30m task check-runner` on `wave/big-1` at `a4c1b2e7` with the coordinator's
files of the close, on the build node, 2026-11-26: exit 0. Its lines are in `AGENTS.md`, "The
gate's numbers"; the ones this wave moved:

```
nostd-build: 6 libraries for x86_64-unknown-none: swell swell_compiler swell_num swell_regexp swell_syntax swell_unicode
nostd-symbols: 6 libraries, 53 object(s), 9035 undefined symbol(s): 8890 Rust's own, 57 memory functions, 88 compiler runtime routines; none from a C library or the system
host-requirements: 221 requirement(s), 221 line(s): 165 test, 56 pending, 0 omitted; 53 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 8:1 9:6 11:32 12:3 13:2 14:6 16:5 18:1
19 test binaries: 1774 passed, 0 failed, 1 ignored
test-collect: 2 test binaries: 1339 passed, 0 failed, 1 ignored
test262 total 53801 8982 7 44812 26026 18784 2
test262-slice 04 1681 3320 0 0 1681 1681 0 0
test262-moves: 1964 case(s) moved, 3:78 4:337 5:278 6:694 7:60 8:341 9:30 10:83 11:4 12:42 13:17; slices 1 to 19: 44931 85794
gate-selftest: 88 passed, 0 failed
gate-time test-steps 89.7 of 120, cpu 304.4, load 45.68 on 32 cores
```

The expected file and its digest are those of one full run on the same tree
(`task test262-expected`, exit 0, sources digest `96626cfa...`); the expected file equals
the one the unit `close` wrote, line for line. On `main` before the wave the total line read
`test262 total 53801 8982 7 44812 25943 18867 2` and slice 4 `1612 3184 0 0 1612 1533 79 0`.

### What went wrong

- The first whole-gate run of the unit `gate` ended with exit 201 and `gate-time test-steps
  303.3 of 120` at load 59 on the node; the second, unchanged, took 81.6 s at load 58. The
  run of the close took 89.7 s at load 46. The test steps are near their bound on a loaded
  node; `story:gate-cpu-table-held-by-a-check` (draft, older than the wave) is the story
  that holds the bound by CPU seconds.
- The unit `gate` committed regenerated count files of its own tree; the commit was reverted
  (`4503978a`), since the unit `close` writes them for the whole wave.
- The first incoming list of the close was inferred from a difference of counts. Pass 1 of
  the adversary found one case in it that passed at the start. Correction 1 made the full
  run at the start commit and read all 72 cases against it.
- The merge of the unit `close` met one conflict, `conformance/test262/expected.digest`: the
  digest of the unit's tree and that of the wave branch. The merge commit `a4c1b2e7` holds
  the unit's lines, and `test262-check` is red on that one commit; the next commit holds the
  run on the wave branch.
- A worktree without a copy of `.cache` fetched test262 on the build node, and the pool's
  return sync brought the checkout back without its empty `.git/refs/`; the next two full
  runs of that tree ended in `test262-fetch` with exit 201. The unit copied `.cache` from
  the primary checkout as `AGENTS.md` says. The draft
  `story:test262-fetch-makes-the-target-of-a-cache-link` (older than the wave) is about the
  same directory.
- Pass 2 of the adversary on the unit `close` started one pool job from the directory
  `taskfiles/`; it ran nothing of the unit and left an empty directory, which was removed.

### Where each finding went

- Twelve adversary passes, two per unit but `core` (`review-result:adversary-<unit>-pass-1`
  and `-pass-2` for `gate`, `random`, `static`, `exponent`, `growth` and `close`). Every
  finding introduced by a unit was corrected by that unit before its merge; each story
  carries the outcome of each pass.
- Three second corrections were read by the coordinator in place of a third pass
  (`review-result:gate-correction-2-coordinator-read`,
  `review-result:growth-correction-2-coordinator-read`,
  `review-result:close-correction-2-coordinator-read`); the read of the close found one
  sentence of a doc comment that was not true, corrected at `a1ca60ed`.
- Three notes of the growth unit's pass 1 are older than the wave (a charge made after its
  reservation): filed as the draft `story:charge-comes-before-the-reservation`, which no
  wave takes before the planned waves are delivered.
- The acceptance of `story:core-builds-without-std` (an example host with its own
  allocator) is not met as written; the story states the reason and what is proved in its
  place (see "A decision the coordinator took").
- `crates/swell/tests/builtins_array_from_async.rs:14` gives "the exponentiation operator is
  slice 4's" as the reason for a number written out; the operator runs now. The comment is
  not changed in this wave.
- Two stories delivered before the wave are still `draft`
  (`story:release-0-12-0-with-binaries`, `story:gate-selftests-write-no-tracked-file`); a
  draft is accepted by the operator, so they are not moved here.

### After the first push: the lane `slow` was red on the forge

- The wave was pushed at `7754edef` with a green gate and a local run of `slow-tests` alone.
  On the forge `task slow` ended red in its part `test-realm-sizes`, at size 200 in both
  modes: `misuse::host_102_reads_change_nothing_and_call_no_hook` and
  `termination::an_agent_that_ended_in_memory_budget_runs_again_after_a_collection`.
- The cause is in the two tests, not in the engine. With `Math.random` in the realm (the
  unit `random`, merge `12a32b66`, the first red commit by bisection) a realm padded by 200
  host functions leaves the arena of Strings two free slots after a collection; the third
  String grows it by 53,248 bytes. The first test made a String between two readings of
  `heap_bytes()` (550,495 and 603,743); the second read a Number as a String in a room of
  39,310 bytes and ended in `Terminated(MemoryBudget)`.
- The fix (`5078361a`, tests only): the first test makes and roots its String before the
  first reading, the second reads its answers as Numbers with no allocation. `task
  test-realm-sizes` on it: 14 runs green, 1343 passed in the default mode and 1339 in the
  collect mode at each size.
- What let it through: the unit `random` added to the realm and did not run `task
  test-realm-sizes`, and the coordinator ran `slow-tests` where the whole lane was due.
  `AGENTS.md` ("The lane `slow` runs here before a hand-over") now names both: the unit's
  run, and every part of `task slow` on the wave branch's tip before each push.

### The stories

`story:core-builds-without-std`, `story:releases-ship-static-binaries`, `story:math-random`,
`story:exponentiation-operator`, `story:buffer-growth-outside-the-charged-list-is-listed`
and `story:slice-04-conformance-close` are `implemented` in the store, each with one
`test_result` record of the gate above. `conformance/slices-delivered.txt` lists slice 4,
and `epic:slice-04-numbers-and-dates` states the counts at the close: 1,681 cases and 3,320
runs.

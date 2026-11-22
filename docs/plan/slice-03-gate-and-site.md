# Slice 3, beside the waves: the gate's budget, the forge's form, the documentation site

The units of the branch `wave/s3-1` that are not builtins, 2026-11-22, each implemented by a
sub-agent on Opus in its own worktree, the record of one hardening pass, the three runs of the
gate on the branch and the lines of the green one. The units ran beside
[wave 1](slice-03-wave-1.md), [wave 2](slice-03-wave-2.md) and [wave 3](slice-03-wave-3.md) of
the [wave plan](waves.md). Every commit is the organization bot's.

## Units

In the order they reached the wave branch.

| unit | what it changed | what it measured | commits |
|---|---|---|---|
| hardening record | `review-result:harden-host-interface-pass-1`: the report of a unit that ran the skill `ess:hardening` on `spec/` against `docs/design/host-interface.md`, on the wave branch with eleven units of slice 3 merged and the pinned ESS release 0.96.0. Three draft stories take what changes the repository: `host-interface-spec-closes-design-gaps`, `host-interface-scenarios-with-engine-evidence`, `spec-diff-in-the-gate`. What concerns ESS itself is in the record as category `ess-limit` | six of the eight techniques ran; 26 findings; the interpreter suite stayed at 58 passed, the engine suite at 14 passed, 44 unsupported; a planted defect (`alive()` answers Ok for a faulted agent) left the engine suite at 14 passed; no file of the tree changed | `6bfccae` |
| `website` (no story) | `website/`, a Docusaurus site on the organization's docs-system: a landing page that draws the engine as the program that embeds it sees it, and pages on what swell is, where it stands, its architecture, embedding, principles and limits. The site types no number: `task site-data` writes `website/data/status.json` from the committed files of the gate with shell and awk, and `task site-data-check` fails when the committed file differs, with a planted script. A logo for a light and a dark ground and a lockup for the README, hand-written SVG. Then the head of `README.md` (the lockup, the one-line description, links to the documentation site, the releases and the changelog) and `AGENTS.md`, "Publishing" and "Layout" | nothing measured; the coordinator looked at the logo sheet and the landing page before the commit. The address `https://beyond10x.github.io/swell/` serves nothing until the session that decides for the operator has added the two workflows and the Pages setting and their first run has ended with success (`55e1f6e`) | `43144dd` (merge `f507af6`), `55e1f6e` |
| `gate-test-steps-within-budget` | Tests run under cargo-nextest: a test is terminated at 60 s and counts as failed, and each step prints its ten slowest tests. Each test step prints `gate-time <step> <seconds>`, and the last line fails the gate over `GATE_TIME_BUDGET` of wall time. The full test262 run left the gate: `test262-check` compares a digest of the run's sources with `conformance/test262/expected.digest`, and the full run is the lane `slow`. The parser's stack test is 17 parts, the language test 13. Heavy tests run a gate count, and the full count with `SWELL_FULL_COUNTS=1` in the lane `slow`. The planted scripts of test262 use a small fixture checkout and run four at a time. `taskfiles/spec.yml` runs the pinned ESS release | before: 538.8 s of wall time for the test steps (the wave plan: 284 s on `29f674c`, `test` 84 s, `test-collect` 110 s, `test262` 90 s). After: 172 to 183 CPU seconds, 166.9 s of wall at load 24 to 30; about 60 s on four idle cores by the unit's estimate, not measured. 37 planted scripts pass. After taking the wave branch (`0292b62`): 216 tests of the library and nine test binaries pass under nextest in both collection modes. Known and left: determinism in collect mode 7.6 s alone, three language scripts 10 to 12 s each in collect mode | `ae58a03` (merge `314c120`) |
| `local-gate-equals-forge` | `rust-toolchain.toml` pins 1.99.1 with clippy for the root workspace and `conformance/ess`; the workflows name no version, and `toolchain-check` fails when one does or when `rustc` in the gate is not the pinned one. `task check-runner` runs `check` with the forge runner's variables, kept in `RUNNER_FORM` of `taskfiles/forge.yml`; `runner-env-check` compares it with `check.yml` both ways. `.github/planning-paths.txt` and `taskfiles/classify-changes.sh`: a change of planning and prose files only runs `python-check` and exits 0. Then a third class, site-only (`website/`, `docs/img/`): `check.yml` runs `python-check` and `site-data-check` and neither builds nor tests the code; the root `Taskfile.yml` includes `taskfiles/site.yml` | five planted scripts, each seen red before its check existed, each under 2.1 s; four more for site-only, each under 0.5 s. `gate-selftest` on the unit's tree: 42 passed, 1 failed (the stale status file, below). One test binary of swell (`budgets`) by debug level of the dev profile: true 25,729,792 bytes with 19,981,428 of `.debug_*`; line-tables-only 14,556,496 with 9,110,365; false 5,448,224 with none | `0785995` (merge `20a3526`), `24e0911` (merge `e077d7d`) |
| `site-slice-3` (no story) | A percentage is floored to one decimal, so a slice or a total with a failed case never reads 100.0; slice 3 reads 99.9. The wording of a slice follows the data ("7,010 of 7,011 cases pass"), with one constant for a delivered slice with a failed case and a link to the design's section 12. The landing page heads the group "Delivered: each case passes or is an exception the design states". The ten `SITE_DATA_` variables are in `GATE_VARIABLES` | `site-data-check` and `gate-vars` exit 0, the site builds and type-checks, no home path in the build output; the planted script for a percentage rounded up to 100 is red before the change and ok after, 0.29 s | `f98d1d0` (merge `8a20dcd`) |
| `tests-under-the-kill` (no story) | No test of `swell` stands near the 60 s kill. No assertion changed its meaning and no engine source changed. A test whose cost was its count got a gate count that still reaches what the test is for, and keeps its full count under `SWELL_FULL_COUNTS=1` (the lane `slow`); where the cost was the collection at every safe point, the count shrinks in collect mode only. The reviver test of `builtins_json_parse` is two tests, depth 40,000 in the gate and 200 in collect mode; `host_133` of `determinism` runs under 2,000 steps, 156 KiB, frame depth 30 and string length 256 in collect mode; `fuzz` runs 160 sequences in the gate (400 at the full count), 15 in collect mode (40); smaller depths, step budgets and loop counts in the other binaries the commit lists (`eval`, `budgets`, `builtins_array*`, `termination`, `tracing` among them), each stated beside its test; `function-recursion.js` computes fib(10), and the two `try-completion-*-abrupt.js` scripts are one script per statement row (7 and 6 files) | before, at a load of 105 to 111 on 17 cores: in collect mode three tests were killed (`determinism` `host_133`, `language` `part_7`, the reviver over 100,000 nested arrays) and 22 took over 15 s; in the default mode 13 took over 15 s. After, at a load of 77 to 97: 506 tests pass in each mode and none is over 15 s; the slowest is `host_133` in collect mode, 12.1 s. The sum of nextest's test times 1,451.8 s to 494.5 s in collect mode, 645.0 s to 117.6 s in the default mode. Full counts of the touched binaries pass in both modes (179 tests each); fmt and clippy exit 0 | `b4d0192` (merge `e29ef96`) |
| `gate-red-runner-and-syntax-times` (no story) | Three defects the first gate run showed (below). `an_unknown_rule_kind_fails_the_run` of `swell-test262` states the count its fixture tree gives now, and its comment names the case. `nextest-report` strips the escape sequences of a coloured log into `<log>.plain` and reads that; a log with no "Starting" line or with 0 binaries fails with a line that says it read nothing to count; `conform-engine` reads the plain log. The deep limit of `swell-syntax`'s `every_nesting_kind_fits_the_stack_bound` and its 17 parts is 2,500 in the gate and 10,000 under `SWELL_FULL_COUNTS=1`, no kind dropped, the bound unchanged. `swell-num`'s `to_string` tests check 500 patterns per radix in the gate and are split into parts (51 tests) | the planted script `nextest-report-reads-a-coloured-log.sh` red before, ok after, 0.43 s. The `swell-syntax` parts 0.5 to 2.7 s each; the sum of nextest's times 324.9 s (load 137) to 22.5 s (load 72); processor time of the parts 37.3 s to 5.4 s. At the full count every part of `swell-num` passes, the slowest 14.8 s at a load of 93 to 104. `swell-syntax` 40 tests, `swell-test262` 55, `swell-num` 51 pass; fmt and clippy exit 0; `nextest-check` and its planted scripts ok | `7d84948` (merge `0c2a293`) |
| `gate-builds-before-timed-steps` (no story) | A task `build-tests`, after `build` and before `gate-time-start`: `cargo nextest run --no-run` for the workspace, then `build-collect`, `build-faults` and `build-conform-engine`, each with its step's own flags, features and manifest. A task `compile-check`: a test step whose log has a Compiling line fails and names the step, the number of crates and the build task that should have built them. The doc tests still compile inside `test-doc`. The two planted scripts that failed in the second gate run expect the refusal in the build phase | before, inside the timed steps: `test-collect` built swell with `collect-every-safe-point` (25.8 s), `test-faults` with `fault-injection` (5.9 s), `conform-engine` the 51 crates of `conformance/ess` (3 m 54 s). The planted script `nextest-report-refuses-a-step-that-compiles.sh` red before, ok after, 1.0 s. The build phase cold, with the machine's compile cache: `build` 1 m 35 s, `build-collect` 1 m 31 s, `build-faults` 6.2 s, `build-conform-engine` 4 m 10 s; with nothing changed 6.8 s in all. The third gate run (below) | `d99dbf0` (merge `a6cf201`) |

## What went wrong, and what was decided

- **A unit asked the pool for all its slots.** The unit of `gate-test-steps-within-budget` was
  told to measure each gate step "alone" and submitted a job that asked for all 17 slots of
  the shared build pool. It stood first in the queue for 13 minutes (`unit-invariants.md`;
  `c2f1861` gives 01:18 to 01:32 CET), collected 11 slots while it waited and held 54 jobs of
  other sessions behind it, until the session that maintains the pool asked for its
  cancellation. The invariants copied into every unit brief now forbid `--tokens` and a
  priority, and say to measure in an ordinary job and report CPU seconds and the load beside
  wall seconds (`c2f1861`).
- **A pool job over 10 minutes is killed** (a rule of the organization since 2026-11-22). Two
  jobs of this repository were killed by it on the day it began (`465855d`). A job names its
  time as `slot --timeout 30m`; a wrapped `timeout` extends nothing since slot 0.22.0
  (`7aa58e0`). Since a change of the rule the same day a job names at most 30 minutes, and
  `AGENTS.md` and `docs/releasing.md` run the gate as `slot --timeout 30m task check-runner`
  (`286f4fb`).
- **The time line under load.** On the shared pool the test steps cost 172 to 183 CPU seconds
  in every run of 2026-11-22 and took 166.9 s to 307.4 s of wall at loads 24 to 62 on 2 to 5
  cores. The operator's ruling of 05:47 CET, in `AGENTS.md`, "How the work is run": when that
  line is the only red line of a local run and its printed load is above the core count, the
  commit is handed over with the line quoted and the forge's run judges the budget; a forge
  run over 120 s is a budget defect (`3999b6d`).
- **The site claimed more than the data.** Until `f98d1d0` the site showed `100.0` per cent
  and "every case of the slice passes" for slice 3, a delivered slice with one failing case
  ([wave 3](slice-03-wave-3.md)).
- **`website/data/status.json` was stale after the site merged.** The site was based on the
  release commit, and its status file was of that commit. The planted script
  `site-data-check-refuses-a-stale-status-file` caught it: red in `gate-selftest` on the forge
  unit's tree (42 passed, 1 failed, `0785995`) and on the close unit's (38 passed, 1 failed,
  `632341d`). `7aa58e0` wrote the file again and made `site-data-check` a step of `check`.
- **Decisions the units took, kept.** The forge unit keeps the host's compile cache and
  refuses an empty `RUSTC_WRAPPER`, as the organization's rule on one compile cache asks,
  where the story had asked for it. The debug level of the dev profile stays `true`:
  `STACK_BYTES_PER_REENTRY` was measured under it. The gate unit's merge of the wave branch had
  one conflict, in `crates/swell/tests/limits_scripts.rs`, resolved by the unit (`0292b62`).

## Where each finding went

| finding | where it went |
|---|---|
| The findings of the hardening pass that change the repository | three draft stories (`6bfccae`) |
| The findings about ESS itself | issue text for the session that does the forge side (`6bfccae`) |
| Determinism in collect mode 7.6 s, three language scripts 10 to 12 s each | `tests-under-the-kill` (`b4d0192`) |
| A count of `swell-test262` that no builtin unit ran | the unit invariants: a unit that changes what a script evaluates to also runs the tests of `swell-test262` (`6a7072b`) |
| The site's address serves nothing yet | the session that decides for the operator: two workflows and the Pages setting (`55e1f6e`) |
| A stale status file and a percentage rounded up to 100 | `7aa58e0` and `f98d1d0`, each with its planted script |

## The three gate runs

The gate ran on the wave branch after the last unit, as the forge's runner runs it
(`task check-runner`), three times; the load is the three figures the coordinator
saved beside each log (`wave-s3-1/gate-load-before.txt`, `gate2-load-before.txt`,
`gate-builds-before-timed-steps/gate-load-before.txt` and `gate-load-after.txt`).

1. **On `e29ef96`, red** (load before 111.18 91.47 89.69; `7d84948`: 117 to 149 on 17 cores).
   The step `test` failed: nextest's summary reads `638 tests run: 637 passed, 1 failed, 1
   skipped` in 235.902 s. The failed test, `swell-test262::runner
   an_unknown_rule_kind_fails_the_run`, asserted `test262 total 22 1 1 20 3 17 0` over its
   fixture tree; the run prints `4 16`, because the fixture case
   `built-ins/String/normative-optional.js` calls `"a".localeCompare("b")`, which slice 3
   delivered (`741f1a8`). The step's report then printed `0 test binaries: 0 passed, 0 failed,
   0 ignored`: `nextest-report` read no test from a log that cargo-nextest wrote in colour,
   and `check-runner` and the forge set `CARGO_TERM_COLOR=always`. Tests outside the package
   `swell` stood near the 60 s kill: `swell-syntax`'s `every_nesting_kind_fits_the_stack_bound`
   and its 17 parts took 13.9 to 44.6 s each (`7d84948`). Fixed by
   `gate-red-runner-and-syntax-times`.
2. **On `0c2a293`, red** (load before 126.96 97.50 92.38). Every test passed (`68 test
   binaries: 667 passed, 0 failed, 1 ignored`; `test-collect: 49 test binaries: 506 passed, 0
   failed, 1 ignored`). The `gate-time` lines: `test 53.5`, `test-doc 13.4`,
   `test-collect 146.7`, `test-faults 6.4`, `conform-engine 241.7`, `test262-check 1.1`,
   `test262-moves-check 17.6`. `gate-selftest: 48 passed, 2 failed`:
   `test-collect-refuses-a-crate-without-the-feature` and
   `test-faults-refuses-a-crate-without-the-feature` ("the task did not print that no test
   ran"); they expected the count line `0 test binaries`, which the fix of `nextest-report`
   in `7d84948` changed (`d99dbf0`). The steps compiled inside their timed part: by
   `d99dbf0` the test steps took 480 s, and the forge caches nothing, so every run there paid
   these builds inside the budget of 120 s for the test steps. Fixed by
   `gate-builds-before-timed-steps`.
3. **On the tree of `a6cf201`, green**, exit 0, run by that unit on its tree
   (`git diff impl/gate-builds-before-timed-steps a6cf201` is empty; load before 52.40 64.45
   68.62, after 39.15 55.89 65.02). Its lines are [the gate](#the-gate).

The coordinator's errors, as the commits state them:

- The builtin units of slice 3 ran the tests of the package `swell` only, as their briefs said,
  so no unit ran `an_unknown_rule_kind_fails_the_run` and its count went stale unseen
  (`7d84948`, `6a7072b`). The unit invariants now say that a unit that changes what a script
  evaluates to also runs `cargo nextest run --locked -p swell-test262` (`6a7072b`).
- The unit `tests-under-the-kill` measured the tests of `swell` ("no test of swell stands
  near the 60 s kill", `b4d0192`; its brief named that package only, by the coordinator's
  account); the tests of `swell-syntax` near the kill were found by the gate (`7d84948`).
- The fix of the test report changed the line two planted scripts expected (`d99dbf0`), and
  the unit that made it had run `nextest-check` and its planted scripts, not `gate-selftest`
  whole (`7d84948`: "nextest-check and its planted scripts ok").

## The gate

The green run, the one gate for the branch after its last unit. `task check-runner`
(`task check` with the variables the forge's runner sets, `CI=true GITHUB_ACTIONS=true
CARGO_TERM_COLOR=always CARGO_INCREMENTAL=0`) on the tree of `a6cf201`, one pool job of 122 s
by the unit's commit (`d99dbf0`), load 52.40 before and 39.15 after on 17 cores, exit 0. The
build phase's `Finished` lines, the `gate-slowest` lines and the single `ok` lines of the
planted scripts are left out here, and of the 22 `test262-slice` lines those of slices 01 to
04 are kept:

```
gate: variables in effect: SPEC_REFUSALS_MAX=41 SPEC_OUT=.out SPEC_SYNTHESIZE_LOG=.out/synthesize.log SPEC_CONFORM_LOG=.out/conform.log ESS_SUITE=.out/suite.json ESS_UNSUPPORTED_MAX=44 SITE_DATA_FILE=website/data/status.json SITE_DATA_OUT=.out/site-data SITE_DATA_EXPECTED=conformance/test262/expected.txt SITE_DATA_DELIVERED=conformance/slices-delivered.txt SITE_DATA_REQUIREMENTS=conformance/host/requirements.txt SITE_DATA_MOVES=conformance/test262/moves.tsv SITE_DATA_DESIGN=docs/design/engine.md SITE_DATA_MANIFEST=Cargo.toml SITE_DATA_LOCK=Cargo.lock SITE_DATA_CRATES=crates
python-check: no Python file among 1235 file(s)
forbid-unsafe-check: 8 crate root(s), each with #![forbid(unsafe_code)]
Cargo.lock: 7 package(s), none from outside this repository
toolchain-check: rustc 1.99.1, the channel rust-toolchain.toml pins; 3 workflow file(s), none names a compiler
runner-env-check: check-runner sets CI=true GITHUB_ACTIONS=true CARGO_TERM_COLOR=always CARGO_INCREMENTAL=0: CI and GITHUB_ACTIONS as the runner sets them, and the 2 variable(s) .github/workflows/check.yml sets for task check
nextest-check: ./.config/nextest.toml: every test is terminated by 60 s at the latest; 13 task file(s): 4 step(s) through cargo-nextest, no cargo test but doc tests
test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked
data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present
unicode-tables: 1 table(s) equal the generator output
swell v1 — 2 file(s), 8 scenario(s), valid
58 scenario(s) (8 authored), 41 refusal(s), written to .out/suite.json
  58 scenarios: 58 passed, 0 failed, 0 error, 0 unsupported
host-requirements: 165 requirement(s), 165 line(s): 84 test, 81 pending, 0 omitted; 51 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1
site-data-check: website/data/status.json equals its regeneration from conformance/test262/expected.txt, conformance/slices-delivered.txt, conformance/host/requirements.txt, conformance/test262/moves.tsv, docs/design/engine.md, Cargo.toml, Cargo.lock and the crate roots under crates
68 test binaries: 667 passed, 0 failed, 1 ignored
gate-time test 18.8
test-doc: 7 test suites: 11 passed, 0 failed, 0 ignored
test-doc collect-every-safe-point: 2 test suites: 11 passed, 0 failed, 0 ignored
gate-time test-doc 5.0
test-collect: 49 test binaries: 506 passed, 0 failed, 1 ignored
gate-time test-collect 27.2
test-faults: 1 test binaries: 1 passed, 0 failed, 0 ignored
gate-time test-faults 0.2
conform-engine: 3 test binaries: 12 passed, 0 failed, 0 ignored
conform-engine: planted defect refused: test a_target_whose_interrupt_does_nothing_fails_a_scenario ... ok
  58 scenarios: 14 passed, 0 failed, 0 error, 44 unsupported
gate-time conform-engine 3.6
test262-refused 46581
test262 total 53801 8982 7 44812 12120 32692 0
test262: 221 directory lines sum to the total
test262-slice 01 31 62 0 0 31 31 0 0
test262-slice 02 4259 7543 0 0 4259 4259 0 0
test262-slice 03 7011 13829 0 0 7011 7010 1 0
test262-slice 04 1516 2992 0 0 1516 249 1267 0
test262: 22 slice lines equal the design
test262-errors 0
test262: 286 lines of conformance/test262/expected.txt; sources digest 02158d3cf750d26db83ea9366c92a7e7785e8fe348a8afc95c24e3c706cbeaff equals conformance/test262/expected.digest
gate-time test262-check 0.1
test262-moves: 1656 case(s) moved, 3:78 4:318 5:276 6:719 7:44 8:104 9:18 10:78 11:4 12:3 13:14; slices 1 to 19: 44931 85794
gate-time test262-moves-check 6.2
gate-selftest: 51 passed, 0 failed
gate-time gate-selftest 19.4
gate-time test-steps 80.5 of 120, cpu 199.2, load 39.15 on 17 cores
```

Passed cases per slice in that run: [wave 3](slice-03-wave-3.md). Every commit on the branch
after `a6cf201` changes only prose or planning files. The coordinator's prose commits before
it: `1534e2e` (the gate's commands in `AGENTS.md` and `docs/releasing.md`) and `286f4fb` (a
pool job names at most 30 minutes).

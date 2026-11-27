---
format: aep.planning-md/3
id: review-result:adversary-lanefour-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the unit lanefour (impl/lanefour at b13c3eb)
relations:
- reviews: story:realm-size-lane-within-its-share
revision: 1
---
Adversary pass 2 (the last) on the unit lanefour of wave big-2, impl/lanefour at b13c3eb, 2026-11-27.
Red without a blocker: 1 failing case, four warnings (one older than the unit) and four notes. The adversary's own report follows.

```
unit: lanefour, pass 2, commits 1b2f222 and b13c3eb (worktree adv2-lanefour, detached at b13c3eb)
verdict: red (strongest finding CONFIRMED, warning; no blocker)
cases: executed 102→104 (gate-selftest), red 1
origin: introduced 7, pre-existing 1, undecided 0
wrote-outside-worktree: 1 directory, <scratch>/wave2/lanefour/adversary-2
needs-coordinator: no
```

**1. Diff of the worktree.** `git --no-pager diff --stat` prints nothing. My two case files are untracked, so they show only in `git status --short`:
```
?? tests/gate/slow-collect-a-lane-reads-only-its-own-build.sh
?? tests/gate/slow-tests-stack-rule-finds-a-stack-test-in-more-forms.sh
```
Both are test files and I changed no other path. The patch of both is `adversary-2/cases.patch`.

**2. My cases** (each run alone with `slot sh <file>`, before the suite)

- `tests/gate/slow-tests-stack-rule-finds-a-stack-test-in-more-forms.sh`: **red now**, 0.02 s, 15 FAIL lines, exit 1. It plants 15 tests that reach `stack_with_room(` and checks that none is printed "other". A control test is printed "stack", so the fixture is read. First line of the red output, verbatim:
  `slow-tests-stack-rule-finds-a-stack-test-in-more-forms: FAIL deep::an_alias_of_the_stack_helper runs on a bounded stack and is printed other: it would run at opt-level 1 only (.out/gate-tests/.../lines.txt)`
  The 14 other FAIL lines name: `a_helper_of_another_module_by_use`, `..._by_use_as`, `a_const_function_pointer`, `a_static_function_pointer`, `a_generic_helper_with_a_nested_type`, `an_array_of_function_pointers`, `a_function_pointer_in_a_struct_field`, `a_function_pointer_by_reference`, `a_spawned_body_and_a_field_of_its_name`, `a_spawned_body_and_a_comment_for_it`, `an_extern_helper`, `a_restricted_helper`, `named::deep_stack` and `named::another_test_of_the_file_calls_the_helper`.
- `tests/gate/slow-collect-a-lane-reads-only-its-own-build.sh`: **green now**, 3.1 s (5.7 s before I ran its case 2 beside cases 1 and 3). It checks three things:
  - a status of 0 left by an earlier lane is not read;
  - the wait ends at `SLOW_COLLECT_WAIT`, and the build's group is gone within 3 s after;
  - the background build and the opt-level 1 run have the same RUSTFLAGS, CARGO_INCREMENTAL and arguments.

  It is red on plants P1a, P1b and P1c, which the unit's selftest leaves green (part 4, F2).

**3. Suite run** (after the cases existed). `slot task gate-selftest`, run in a clone with `expected.digest.new` and the pinned test262 checkout copied in: `gate-selftest: 103 passed, 1 failed`, exit 201, job wall 10.7 s. The one that failed is my red case.
- A first run without the test262 checkout gave 101 passed, 3 failed. The two extra failures were `test262-moves-check-refuses-a-line-without-evidence` and `test262-paths-agree-when-exported-and-passed`, both with "no pinned checkout". With the checkout copied in, both print `ok`. That was the environment, not a finding.

Wall time of each selftest alone, from the plant job at HEAD:

| Selftest | Wall |
|---|---|
| `slow-collect-builds-beside-the-lane-and-ends-with-it` | 5.1 s |
| `slow-tests-stack-rule-finds-a-stack-test-in-every-form` | 0.0 s |
| `slow-tests-runs-the-stack-tests-at-opt-level-0` | 0.3 s |

**Planted defects** (each laid over a copy of the files, one pool job):

| Plant | Selftest | Result |
|---|---|---|
| P1a: no `rm -rf` of `.out/slow-collect` | S1 | green |
| P1a | my case | red: "the lane exited 0 on a failed build, with the status 0 of an earlier lane left" |
| P1b: the wait has no bound | S1 | green |
| P1b | my case | red: "still waited 8 s ... SLOW_COLLECT_WAIT=2" |
| P1c: `slow-collect-start` without `RUSTFLAGS` | S1 | green |
| P1c | my case | red: "build [RUSTFLAGS=[] ...]" differs from the run |
| P2a: every test of every module but `deep` is marked stack | S2 | green (the "two tests other" guard only covers the module `deep`) |
| P2b: every test of `deep` is marked stack | S2 | red, 2 FAIL lines |
| P2c: path resolution removed | S2 | red (`through_another_test_module`) |
| P3a: tests outside the filter counted as opt-level 1 | S3 | red (count line) |
| P3b: no "finds no stack test" guard | S3 | red |

Other commands and their result lines:
- **The rule on the tree**: f9ddc9e gives 67 stack, 1389 other; b13c3eb gives 67 stack, 1390 other. Room goes 280 → 281. The only line that moved, in both lists, is `faults::host_71_injected_fault_at_each_call_faults_the_agent`, the one the unit reported. No test moved between stack and other.
- **Other awks**: none on this host or on the node (the plant job printed `mawk: none; busybox: none; original-awk: none; nawk: none`, with GNU Awk 5.4.1). Under `gawk --posix` and `gawk --traditional`, the stack and room lists on the tree are byte-identical to plain gawk. S1, S2 and S3 give `ok` and my forms case gives the same 15 FAIL lines in both modes.
- **POSIX constructs**: I found no construct outside POSIX awk in `realm-room-tests.awk` or in the new awk of `Taskfile.yml` (no gensub, three-argument match, length of an array, asort, `\<`, `\y`, interval expressions or delete of a whole array).
- **The rule's time**, CPU s, three runs each:

  | Version | Runs |
  |---|---|
  | f9ddc9e | 0.78 / 0.78 / 0.78 |
  | b13c3eb | 2.10 / 2.12 / 2.11 |
  | b13c3eb without `bound()` | 1.81 / 1.80 / 1.87 |
  | b13c3eb without the value form | 1.38 / 1.37 / 1.38 |

  So `bound()` costs about 0.3 s and the value form about 0.43 s. The remaining 0.6 s goes to the path-qualified call regex and the merge of definitions. Every variant gives the same 67 on the tree.
- **The count line against `cargo nextest list` of both builds** (one pool job, `slot --timeout 30m`):
  - At opt-level 0: `swell` 273 plus `swell::main` 98 = 371, where 98 = 67 stack + 31 tests a macro writes.
  - At opt-level 1: 964 = 933 + 31. The line says 935 others, but 2 of them, `proxy_shape_traps::memory_budget_at_every_64_bytes_*`, are `#[cfg(not(feature = "collect-every-safe-point"))]`. They are not in the collect build.
  - Outside the filter: the line says 455; nextest gives 454 expression mismatches plus 1 ignored test (`faults::host_71…`), so 455 holds.
- **Item 5, the bounds**: the diff grep finds one new value, `SLOW_COLLECT_WAIT: '400'` (Taskfile.yml:7). No timeout, bound, thread count or retry count was raised. `git diff --stat f9ddc9e b13c3eb -- .github/workflows/slow.yml` is empty, so `slow.yml` is unchanged.
- **Lifecycle probes** (stub cargo and stub git in a planted tree, run without slot):

  | Case | Result |
  |---|---|
  | `slow-test262` fails first | exit 201 (go-task's code; the part's own exit was 1, the same as before the unit); "slow-collect-stop: ended the build …"; no process of the group left |
  | KILL of the task, during the wait or during `slow-test262` | group gone 0.9 s / 0.7 s after; 0 processes 30 s later |
  | INT to the task's process group | group gone at once; the stop line is printed |
  | TERM to the task process alone | go-task 3.54 catches it and waits for the running command; the lane goes on, and the group still has 4 processes more than 40 s later. That is go-task's handling, and the pool sends KILL 5 s after TERM |
  | stale status from an earlier lane | exit 201, "failed, exit 3", 0 runs (the `rm -rf` holds) |
  | `SLOW_COLLECT_WAIT=3`, build hangs | exit 201 after 3 s with the line; group gone 0.9 s after |
  | `task slow-tests` alone after a killed lane | exit 0; it builds itself and does not wait |

- **sccache, item 7**: I ended a cold collect build after 60 s on this host, run through the shim. At the kill, a `rustc --crate-name main` ran as a child of its sccache client, not of the server, under the pool job's group. Nothing of it was left 2 s after the KILL.

**4. Findings** (they cover b13c3eb)

- **F1a**: `realm-room-tests.awk:62` (`bound()`, used at :130) skips a real call by value. It happens when the test's text has `<name>:` (a struct field such as `Opts { local_deep: true }`) or a comment "for local_deep, … in the". In both cases `spawn(local_deep)` is printed "other". Measured by my forms case. What reaches it: a test written that way; none is in the tree now. Origin: introduced in b13c3eb.
- **F1b**: `realm-room-tests.awk:83` and `:116–128` do not follow 12 more forms, all red in my case:
  - `use … as` aliases, and a helper of another test module imported by `use` and called by its bare name;
  - const and static function pointers;
  - a turbofish whose type nests (`::<Vec<u8>>`);
  - a helper as an array element, as a struct field value, or as `&(f as fn())`;
  - `extern "C" fn` and `pub(in path) fn` helpers;
  - `crate::support::host::x()` called from a module that defines its own `x`. Its doc (:17–18) says a path names the function of its module, but here the module's own `x` is taken.

  I searched the tree for each form and found none, so on the tree the count stays 67. Origin: introduced (the rule came in f9ddc9e; b13c3eb's title "reads every form" does not close these).
- **F2**: the selftest `slow-collect-builds-beside-the-lane-and-ends-with-it.sh` stays green on P1a, P1b and P1c. My case catches all three. Origin: introduced in 1b2f222.
- **F3**: `Taskfile.yml:237` and `:352` set `RUSTFLAGS` in two separate `env` blocks. The summary says COLLECT_MAIN means "the build and the runs cannot differ", but nothing ties these two blocks together. If they drift, cargo rebuilds in the foreground and every check stays green (P1c). Origin: introduced in 1b2f222.
- **F4**: `Taskfile.yml:397` and `:399`: the count line's "935 others at opt-level 1" includes 2 tests that are compiled out of the collect mode; 933 run. Origin: introduced in b13c3eb.
- **F5**: `crates/syntax/tests/sources.rs:3015` asserts `large_time > 10 * small_time + 50ms` on wall times. It failed under load in cold-c ("one private name used n times": 9.4 ms → 267 ms). It runs in `slow-tests`. Origin: pre-existing, from ae0397e, which is in the base 158888d.
- **F6**: `Taskfile.yml:289`: `slow-collect-stop`, run alone, ended a foreign `setsid sleep 60` group named in the pid file and printed "ended the build". In a lane, start always rewrites the pid file first, so only a hand run after a killed lane, with a reused number, reaches it. INFEASIBLE.
- **F7**: `Taskfile.yml:264` sends the stderr of the `setsid` launch to /dev/null. Without `setsid` on PATH, the lane says "did not start (no .out/slow-collect/pid after 5 s)" and exits 201 without naming `setsid`. The forge has `setsid`. INFEASIBLE.

**5. What I attacked and could not break**
- The status is never trusted for freshness: the foreground `cargo nextest run` at Taskfile.yml:455 runs cargo's own check. The `option_env!("SWELL_FULL_COUNTS")` reads (script.rs:809, collect.rs:1325, objects.rs:1918) are inside `mod tests`, so they are not in the `--test main` build.
- No lock wait of a build's length: slow-test262 took 292.3 s with the build beside it against 291.6 s alone on 32 CPUs (cold-a against cold-b). The +56.6 s on 4 CPUs is CPU share. The overlapping steps use different profile directories (`target/collect`, `target/release`, `target/debug`).
- The wait is inside the `slow-tests` slow-time line. The build's own seconds are only on the `slow-build collect` line; no part bound reads it, only `SLOW_COLLECT_WAIT`, counted from the start of the wait, and the lane's 900.
- 400 s against the cold times: about 316 s or 346 s on the forge (229 s and 251 s × 1.38). In a lane the wait starts after `slow-test262`, so it is 0.
- Nothing of a killed build is left after 2 s (item 7, measured above). A compile running inside the sccache server itself was not caught in flight.

Not reached: the awk of the forge runner image (ubuntu-26.04; not checkable from here), and mawk, busybox and BWK awk (none on this host or the node).

**6. Paths written outside the worktree**, all under `<scratch>/wave2/lanefour/adversary-2/`:
- `cases.patch`
- the scripts I wrote: `probe-life.sh`, `rule-lists.sh`, `rule-time.sh`, `other-with-stack.sh`, `path-shadow.sh`, `make-plants.sh`, `run-plants.sh`, `count.sh`, `sccache-after-stop.sh`
- the logs: `probe-life.log`, `plants.log`, `count.log`, `gate-selftest.log`, `gate-selftest-2.log`, `posix-awk.log`, `sccache-after-stop*.log`, `case-*-alone.log`
- the directories `life/`, `forms/` and `lists*/`, and the files `t-*` and `forms-*.txt`
- `clone/`, a full clone at b13c3eb: its `target/` is 1.3 GB, `.cache/test262` is 275 MB, and it holds `plants/` and `Taskfile.hold.yml`
- `cold-target/` (346 MB)

```findings
- file: taskfiles/realm-room-tests.awk
  line: 62
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: bound() takes a struct field or a comment "for <name> ... in" for a binding, so a stack test that passes the helper by value is printed other and runs at opt-level 1 only.
- file: taskfiles/realm-room-tests.awk
  line: 116
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: twelve forms of reaching a stack helper (use import or alias, const or static fn pointer, nested turbofish, array or field or reference value, extern or pub(in) fn, a support path shadowed by a same-named fn of the module) print a stack test as other.
- file: tests/gate/slow-collect-builds-beside-the-lane-and-ends-with-it.sh
  line: 112
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the selftest stays green when slow-collect-start keeps an earlier lane's status, when the wait loses its bound, and when the build loses its RUSTFLAGS.
- file: Taskfile.yml
  line: 237
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: RUSTFLAGS of the background build and of the run that reads it are two separate env blocks, so the summary's promise that build and runs cannot differ is held by nothing.
- file: Taskfile.yml
  line: 397
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the count line's 935 tests at opt-level 1 include two tests compiled out of the collect mode, and nextest runs 933 of them.
- file: crates/syntax/tests/sources.rs
  line: 3015
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: class_elements_scale_linearly bounds a ratio of two wall times and failed under node load in slow-tests (9.4 ms to 267 ms).
- file: Taskfile.yml
  line: 289
  category: concurrency
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: slow-collect-stop run alone ends whatever process group the pid file names when no status exists, and a foreign group was ended in a probe.
- file: Taskfile.yml
  line: 264
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: without setsid on PATH the lane fails with a line that does not name setsid, because the launch's stderr goes to /dev/null.
```

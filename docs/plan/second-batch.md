# The second batch: resource accounting, the host interface of the first consumer, slice 6 part 1

The branch `wave/budget-1` over `main` (release 0.3.0, `1f21c0a`), 2026-11-22, 05:40 to 17:58
CET. It holds 37 merges of units and design texts, 5 merges of the wave branch of slice 3, 2
merges of `main` and 34 commits of the coordinator. Each unit was implemented by a sub-agent in
its own worktree. The source of every line below is a commit message
(`git log --first-parent main..wave/budget-1`). Before: [slice 3, wave 3](slice-03-wave-3.md)
and [the gate and the site](slice-03-gate-and-site.md).

## What the batch delivered

**Resource accounting.** Storage is charged before it is allocated and work before it is done
(`docs/design/engine.md`, "Memory budget" and "The step budget", `284bbe0`). A parse tree, the
compile step's scratch, a key list, the key list of a `for`-`in` loop, a copied String, the
arguments of a bound function and the arguments a call keeps now meet `memory_bytes` or the
step budget before the allocation or the work. Before, several of them were charged after the
work or not at all. A pending interrupt ends `parse_script` before the source is copied. A host
reads `live_bytes()` beside `heap_bytes()`. `HOST-70` and `HOST-188` to `HOST-190` state what
holds for the next entry after a memory-budget termination.

**The host interface the first consumer asked for.** The design text `b6da07a` answers ten
requirements of the browser. A host that generates DOM bindings now sets and reads the prototype
of an object, names intrinsics and well-known symbols from a closed list, and charges its own
bytes and steps. It keeps handles that are weak references. It reads an error object and the
line and column of an error without running code. It counts on four stated ceilings for host
functions, host objects, properties and a realm of bindings. The gate fails when a change of the
host interface is not named in the changelog.

**Slice 6, first part, and the build.** Scripts may use template literals without a tag, `let`
and `const` with block scopes, arrow functions, default and rest parameters, and object
destructuring. Test builds carry no debug information. Each crate has one integration-test
binary. The tests of the collect mode stay far from the 60 s kill. The ESS pins are at 0.97.3.

## Units

In the order they merged. A unit without a story is named by its branch.

| unit | what changed | commit | merge |
|---|---|---|---|
| `template-literals` | template literals without a tag, charged per code unit as `+` is | `df61acf` | `86e8585` |
| design `budget-accounting` | the rule of charging first; `heap_bytes` and `live_bytes`; HOST-166 to 171 | `284bbe0` | `aa2d7f1` |
| design `host-gaps` | the host interface for DOM bindings; HOST-172 to 187 | `b6da07a` | `17c92cf` |
| `let-const-and-block-scopes` | `let`, `const`, the dead zone, a record per iteration, HOST-156 | `89ee023`, `44eb3f4` | `8a5fb15` |
| `interrupt-read-at-every-parse-entry` | a pending interrupt ends `parse_script` before the copy | `d6585ff` | `aa63565` |
| `own-keys-charged-before-the-list-exists` | [[OwnPropertyKeys]] charges steps and list first; a charged merge sort | `d2d59fe` | `d8b1608`, `031fcc0` |
| `the-host-sets-the-prototype-of-an-object-it-made` | `get_prototype_of`, `set_prototype_of` (HOST-180) | `e0692f3` | `b96c268` |
| `every-host-interface-change-is-named-in-the-changelog` | `host-surface-check` and `conformance/host/surface.txt` | `f80ee7e` | `77b5567`, `2e7a177` |
| `values-read-from-a-root-live-for-the-scope` | `get` takes `&mut self`; the value lives for the scope (HOST-170) | `755aecb`, `bf113cb` | `8dc6afa` |
| `stack-requirements-hold-in-the-release-profile` | the stack tests in two more profiles, in the lane `slow` | `3fe9175` | `6c5ed8a`, `b71db97` |
| `arena-capacity-and-collector-workspace-on-the-ledger` | slots, collector workspace and atom table charged by capacity (HOST-171) | `30d1d13`, `7211958`, `68e5562` | `03e31fa` |
| `host-133-collect-limits` (no story) | `host_133`'s collect-mode limit is the realm's charge plus 64 KiB | `7e62597` | `9879d74` |
| `for-in-key-list-charged-while-the-loop-holds-it` | the key list is charged before it grows and until the loop ends | `2031d58` | `9a10240` |
| `test-builds-carry-no-debug-information` | dev and test builds without debug information; `test-binaries-check` | `fb9a355` | `4645ed5`, `680fd61` |
| `a-kept-handle-does-not-keep-its-object-alive` | the weak handle stated and tested (HOST-178) | `d3b5f5c` | `50bc88e` |
| `the-host-charges-its-own-bytes-and-steps` | `charge_host_bytes`, `HostBytes`, `charge_steps` (HOST-175 to 177) | `0a36c29` | `1c1abe9` |
| `the-host-names-a-closed-list-of-intrinsics-and-symbols` | `HostIntrinsic`, `WellKnownSymbol` (HOST-183, 184) | `60aca37` | `4898e69` |
| `budget-test-sweep` (no story) | two red tests of the joined branch; budgets from measured readings | `ae2345a` | `99374f9` |
| `object-operations-charge-before-they-scan` | array length, compaction and bound calls charge steps first | `c8f39a4` | `755a5b6` |
| `string-comparison-and-property-keys-charge-per-code-unit` | a step per code unit past 32, before the work (HOST-181, 182) | `48802fb` | `940bbbe` |
| `two-copies-charged-before-they-are-made` (no story) | the deleted-keys list and the bound arguments charged or not copied | `a34ea7c` | `d13ca38` |
| `frontend-scratch-on-the-memory-ledger` | tree, scope analysis, compile scratch and code on the ledger (HOST-168, 169) | `84522ad` | `7a7bf0a` |
| `string-builtins-charge-before-they-copy` | String builtins charge before they copy, and copy no unused String | `24704ac` | `437a519` |
| `budget-tests-after-the-frontend-charge` (no story) | two tests follow the frontend's charges; two ignored tests state defects | `3a2dd02` | `0b8307d` |
| `a-parse-error-and-an-uncaught-throw-name-their-line-and-column` | `last_entry_error_site`, `ErrorSite` (HOST-172 to 174) | `aa7ade8` | `d2e98bc` |
| `the-next-entry-after-a-memory-budget-termination` | the compile step charges an expansion as made; HOST-70, 188 to 190 | `4fcd962` | `899ddf7` |
| `an-error-object-is-read-without-running-code` | `error_summary`, `HostObjectSpec::error_kind` (HOST-179) | `1f4333f`, `eaa3631` | `28886e7` |
| `arrow-functions` | arrow functions with the `this` and `arguments` of their code | `b4c46f6` | `5263193`, `1d460d4` |
| `host-functions-and-host-objects-are-charged-within-stated-ceilings` | every byte charged; four constants (HOST-186, 187) | `eca6bda` | `e7ba8bc` |
| `ess-pins-to-0-97-3` (no story) | every ESS pin from 0.96.0 to 0.97.3 | `a96ce65` | `a3c1074`, `a0f28bb` |
| `a-terminated-array-length-write-leaves-the-array-as-it-was` | the old length is put back after a termination | `ac86f47` | `63749a2` |
| `bound-functions-and-builtin-calls-charge-before-they-copy` | bound arguments, a bound call's list, a builtin's arguments charged first | `03a3a71` | `add4764` |
| `default-and-rest-parameters` | defaults, a rest parameter, a parameter list that is not simple | `097573f` | `ab4398e` |
| `one-integration-test-binary-per-crate` | one test binary per crate, `tests/main.rs` generated and checked | `f9efb31`, `2e0572a`, `d624042` | `1bb4f10`, `057c6e3` |
| `a-call-charges-what-it-keeps-of-its-arguments-and-parameters` | parameter map and recorded arguments charged; no parameter copy | `4960350` | `2c5db20` |
| `object-destructuring` | object patterns in declarations, parameters, assignments, heads, `catch` | `5cec426`, `7948625`, `69a73dc` | `e1bdfb5` |
| `collect-mode-tests-far-from-the-kill` | gate counts for five heavy tests of the collect mode | `1360327` | `4639a53`, `ae044a0`, `410360c` |

The merge column also names the coordinator's commits that belong to a unit (gate steps, CI
paths, `AGENTS.md`). Each merge has a changelog commit after it; `git log --first-parent` lists
them.

## Numbers

**test262**, as each unit measured it against a run of its own base. The bases differ, so the
rows do not chain.

| unit | base | all cases passed | slice 6 passed | commit |
|---|---|---|---|---|
| template literals | its base | 98 runs of slice 6 gained, none lost; `template-literal` 43 of 57 | | `df61acf` |
| `let`, `const` | its base | 8,594 to 8,902 | 89 to 369 | `89ee023` |
| arrow functions | `a2833ed` | 12,517 to 12,896 | 469 to 698 | `b4c46f6` |
| default and rest parameters | `1d460d4` | 12,896 to 13,053 | 698 to 849 | `097573f` |
| object destructuring | `db0978a` | 13,053 to 13,575 | 849 to 1,364 | `5cec426` |

Every other unit that ran test262 compared its run lines with its base's over 85,556 runs: equal
in each, except one pair of the frontend unit (below). `conformance/test262/expected.txt` on
`main` reads `test262 total 53801 8982 7 44812 12120 32692 0`. It is written again at the close
of the batch.

**Tests of the package swell**, passed per collection mode (default / collect), as the commits
state them:

- `cc41ae6`: 567 of 569 / 568 of 569 (`ae2345a`).
- `6a89c27`: 602 before; 605 with the error object (`1f4333f`), 621 with the String builtins
  (`24704ac`).
- `7a7bf0a`: 611 passed, 3 skipped / 610 passed, 1 failed (`3a2dd02`).
- `3bc1729` with the next-entry unit: 633 in both modes (`4fcd962`).
- joined at `a2833ed`: 642 in each mode, the other packages 172, the fault test 1 (`28886e7`).
- `9792f1f` with the host ceilings: 646 (`eca6bda`).
- `1d460d4`: 651 before, 662 with the parameters (`097573f`).
- `b53d073` with the Array length: 656 (`ac86f47`).
- `37ba6b5` with the bound functions: 660 / 659 (`03a3a71`).
- joined at `db0978a`: 672 / 671, the other packages 172 (`d624042`).
- `db0978a` with the charges of a call: 677 / 675 (`4960350`).
- joined at `7ef3a90`: 686 / 684, the other packages 172 (`69a73dc`).

**Host requirements with a test.** On `main`: 84 of 165 (`632341d`). The design texts raised the
requirements to 171 (`284bbe0`) and 187 (`b6da07a`), the next-entry unit to 190 (`4fcd962`).
With a test: 87 (`9ba8b3d`), 93 (`1c1abe9`), 97 (`940bbbe`), 102 (`aa7ade8`), 108 of 190
(`eca6bda`, `d624042`). `conformance/host/requirements.txt` at `410360c`: 108 test, 82 pending.

**Test executables.** With debug information 70 test executables held 1,502,072,208 bytes,
without it 325,054,920 (`fb9a355`). One integration-test binary per crate: 72 test executables
became 15, 369,710,032 bytes became 57,583,328, and the test list holds 828 tests before and
after (`f9efb31`). A rebuild after touching `support/mod.rs` takes 15.6 CPU seconds where it
took 97 (`f9efb31`).

**The collect mode.** At a load of 112 the slowest test took 58.5 s of its 60 s. After
`1360327`: 675 tests pass in 111.4 CPU seconds where they took 142.9; the default mode 56.6
where it took 66.0.

**Room for a short script.** The room the parse and compile of the script `1` need above
`heap_bytes()`: 3,440 bytes before, 2,228 after; `1 + 1` 3,895 before, 2,322 after. A call
with 10,000 arguments peaked at 5,399,744 bytes for lists of 2,037,728; now 1,835,008 for
lists of 1,835,008 (`4fcd962`).

**The host ceilings** (`eca6bda`), each the measured rise plus an eighth, rounded up to a
multiple of 64: `HOST_FUNCTION_LIVE_BYTES` 576 (measured 496), `HOST_OBJECT_LIVE_BYTES` 256
(192), `PROPERTY_LIVE_BYTES` 448 (360), `BINDINGS_REALM_HEAP_BYTES` 381,696 (339,277).

**The gate** at the batch's last commit:

`slot --timeout 30m task check-runner` on the tree of `7416cda` (the last unit merged and the
test262 counts regenerated), exit 0, load 15 before and 34 after on 18 cores:

```
host-requirements: 190 requirement(s), 190 line(s): 108 test, 82 pending, 0 omitted; 51 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-surface: 63 change(s) since 0.3.0, each named
test-main: 7 crate(s) with tests, each with one integration-test binary and its tests/main.rs as generated
test-binaries-check: 15 test executables, 7 integration-test binaries in 7 packages, none with a .debug_ section
15 test binaries: 858 passed, 0 failed, 1 ignored
gate-time test 32.1
gate-time test-doc 5.1
test-collect: 2 test binaries: 684 passed, 0 failed, 1 ignored
gate-time test-collect 34.7
test-faults: 1 test binaries: 1 passed, 0 failed, 524 ignored
gate-time test-faults 0.5
conform-engine: 3 test binaries: 12 passed, 0 failed, 0 ignored
  58 scenarios: 14 passed, 0 failed, 0 error, 44 unsupported
gate-time conform-engine 2.5
test262-refused 41572
test262 total 53801 8982 7 44812 13575 31236 1
test262-slice 01 31 62 0 0 31 31 0 0
test262-slice 02 4259 7543 0 0 4259 4259 0 0
test262-slice 03 7011 13829 0 0 7011 7010 1 0
test262-slice 06 4436 8413 0 0 4436 1364 3072 0
gate-time test262-check 0.2
test262-moves: 1656 case(s) moved, 3:78 4:318 5:276 6:719 7:44 8:104 9:18 10:78 11:4 12:3 13:14; slices 1 to 19: 44931 85794
gate-time test262-moves-check 4.3
gate-selftest: 64 passed, 0 failed
gate-time gate-selftest 28.2
gate-time test-steps 107.6 of 120, cpu 255.0, load 34.02 on 18 cores
```

The full test262 run behind the digest was made on `410360c` (`7416cda`): no slice and no
directory passes fewer cases than in release 0.3.0, whose total was
`test262 total 53801 8982 7 44812 12120 32692 0` and whose sixth slice passed 100 cases. The
one run pair that ends at a budget is named below. The fault step's line counts the 524 tests
its filter leaves out as ignored. `AGENTS.md`, "The gate's numbers", has the run's other lines.

## Decisions

- **The for-in unit's test budget** (D1). The recursion test takes its budget from a calibrating
  run of `f(0)`, not from the story's formula. On a String object every [[GetOwnProperty]] makes
  a one-unit String that stays until the next collection, so a loop's peak is larger than the
  story assumed (`2031d58`).
- **Two conflicts of the merge of `main`** (D26, D27, `6052bc0`). `Taskfile.yml`: both steps are
  kept, `test-binaries-check` and then `build-tests`. `CHANGELOG.md`: both sides are kept.
- **What `HOST-70` promises after a memory-budget termination** (D28). The question went to the
  session that decides for the operator with option A, narrow the requirement, as the default.
  No answer came within 15 minutes, and A was taken on default. The first consumer raised no
  objection. Reason: a run that ends in the memory budget leaves a room with no lower bound, and
  the engine cannot free what the script still reaches (`4fcd962`).
- **The next-entry story without a round of critics** (D29). Its two failing tests were in the
  tree and are its acceptance (`3bc1729`).
- **A loop of 100 iterations in the collect mode** (D30). The error-site unit's library test runs
  10,000 in the default mode; with a collection at every safe point it took 27.0 s before
  (`aa7ade8`).
- **Three `HostObject` literals outside the story's files** (D31). The crate does not build
  without the field `error_kind` in them (`1f4333f`).
- **The late charges of calls split in three stories** (D32), on the design critic's first round
  (`37ba6b5`, `b53d073`). No third round of critics (D34): their last points are in the text.
- **The tests of the collect mode without a story first** (D37). The unit's brief was its
  specification, and the story entered the store with the implementation (`4639a53`).
- **Release 0.3.0 cut from `main` as it was** (D38), on the operator's order to cut releases from
  green default branches. The branch took the release in `f8f39c9`; `7b6ba50` moved its entries
  back under Unreleased.
- **`opt-level` 1 for the collect-mode tests not applied** (D39). The collect run takes 20.1 CPU
  seconds with it where it takes 111.4, but the build takes 4 min 51 s where it takes 1 min 6 s.
  A story with a profile for that mode alone decides it (`1360327`).
- **The arena unit writes `heap/map.rs`** (D12), outside its story's list: the atom table could
  grow without a charge after sweeps left deleted buckets, and the fix is there (`68e5562`). Its
  CPU bound was measured with seven runs alternating with the base (D13).
- **`get` with `&mut self`: the unit's list of files extended to the callers** (D15), since the
  signature changes every call site (`755aecb`).
- **The stack tests of the two further profiles run in the lane `slow`** (D16), not in the gate
  (`b71db97`).
- **`host_133` at its full count in the collect mode equals the gate's limits** (D17,
  `7e62597`).
- **The no-debug unit landed with its time bounds missed under load** (D20): CPU seconds were
  unchanged, and the bounds are measured again in the gate (`fb9a355`).
- **One integration-test binary per crate waited for the frontend's story** (D21);
  it merged as `1bb4f10`.
- **`regress-610026.js` stays counted as ended by the budget** (D25): the expected file takes
  the result, and the case is not excluded (`84522ad`, `7416cda`).
- **The claim about a termination is cut down to one operation** (D33): the design says a
  termination closes nothing, and the story on the Array length write states its one exception
  there and adds no general rule (`ac86f47`).
- **The parameters unit also wrote `env/mod.rs` and one test of `arrow_functions.rs`** (D35): a
  write to a parameter in its dead zone did not throw through eval, a defect of that change,
  and the check is in that file (`097573f`).
- **The report line of a filtered step still counts filter-skipped tests as ignored** (D36):
  cargo-nextest names both "skipped", and telling them apart needs a second listing per step
  (`d624042`).
- **Conflicts at merges, each resolved by keeping both sides** unless stated:
  `conformance/host/requirements.txt` five times (`9ba8b3d`, `7211958`, `03e31fa`, `1c1abe9`,
  `940bbbe`); the fields of `TestHost` (`1c1abe9`); `scoped_methods!`, the charge methods first
  (`4898e69`); `host/outcome.rs` and `host/agent.rs` (`eaa3631`). The table of
  `crates/syntax/tests/sources.rs` keeps the form of slice 3's branch with the kind `template`
  written in it (`53fd62c`). Two equal definitions of `in_the_collect_gate` became one
  (`30c6255`). The unit of `let` and `const` resolved four conflicts with template literals in
  its branch (`8a5fb15`).

## What changed for a host

The list with its sentences is `CHANGELOG.md`, "Unreleased", "Host interface". In short:

- Breaking: `get` takes `&mut self` (`HOST-170`).
- Breaking: `HostObjectSpec::error_kind`, a new field (`HOST-179`).
- `heap_bytes` keeps its signature and changes its reading; `live_bytes` is new (`HOST-171`).
- New: `get_prototype_of`, `set_prototype_of` (`HOST-180`).
- New: `charge_host_bytes`, `HostBytes`, `resize_host_bytes`, `credit_host_bytes`,
  `charge_steps` (`HOST-175` to `HOST-177`).
- New: `intrinsic`, `well_known_symbol`, `HostIntrinsic`, `WellKnownSymbol` (`HOST-183`,
  `HOST-184`).
- New: `error_summary`, `Agent::error_summary`, `ErrorSummary` (`HOST-179`).
- New: `last_entry_error_site`, `ErrorSite` (`HOST-172` to `HOST-174`).
- New: four ceilings as constants (`HOST-186`, `HOST-187`).
- Changed: `HOST-70`, with `HOST-188` to `HOST-190` for the memory budget.
- `HOST-178`, the kept handle as a weak reference, is stated and tested; no code changed.
- `HOST-185`, the iteration intrinsics, waits for its story with slice 6.

`host-surface-check` at `7b6ba50`: 63 changes since 0.3.0, each named.

## Test262 cases that changed against the direction

- **`test/staging/sm/regress/regress-610026.js`**, in both modes, went from passed to ended by the
  memory budget (`84522ad`). Its third `eval` is about 4.19 million empty blocks, whose tree
  alone is 570 MB under the runner's 512 MiB. Before, the tree was not charged. `expected.txt`
  takes the memory-budget result, and the lost pass is reported to the first consumer.
- **Three arrow cases that passed only on a parse error.** With default and rest parameters,
  `arrowparameters-cover-no-duplicates-binding-array-3`, `-object-4` and `-object-5` of
  `language/expressions/arrow-function/syntax/early-errors` (6 runs) fail on the refusal of a
  pattern. Before, they passed because `...` was an unexpected token (`097573f`). With object
  destructuring the two with object patterns pass again; the one with an array pattern waits
  for array patterns (`5cec426`, `story:array-destructuring-and-spread`).
- Four arrow cases end on a default parameter and moved to the list of the parameters story
  (`1d460d4`).

## Findings left open

| finding | where it is recorded or which story takes it |
|---|---|
| `opt-level` 1 for the collect-mode tests | `1360327`; a story of its own (D39) |
| The report line of a filtered step counts the tests its filter leaves out as ignored | `f9efb31`, `d624042`, "Left as it is" |
| A statement node is 136 bytes for an empty block (570 MB for 4.19 million) | `84522ad`; a task for a smaller node |
| A throw caught inside `finally` overwrites the error site, so the rethrow is sited at the block's end | `aa7ade8`, "Known" |
| The code of the last error site's script stays held without a charge until the next entry | `aa7ade8`, "Known" |
| Compile scratch is credited at the end of the step; the errors of a failed parse are uncharged | `84522ad` |
| With a direct `eval` in the parameters, reads go by name; cover nodes of defaults stay charged | `097573f`, "Known" |
| The mapped arguments object never sees a pattern's slots (read, not tested) | `69a73dc` |
| Rebuild of the two stack profiles 79.8 s and 132.6 s against 60 s, in the lane `slow` | `fb9a355` |
| Wall times of the one-binary story missed at load 75 to 165 | `f9efb31`; measured again in the gate |
| Six construct gaps of ESS not probed on 0.97.3; a spec diff in the gate | `a96ce65`; `story:spec-diff-in-the-gate` (draft) |
| `later-parts.tsv` names the old refusal text for the backtick | `df61acf` |

Closed in the batch: the late Array length write and the five late allocations that `a34ea7c`
found (`ac86f47`, `03a3a71`, `4960350`); the `dead_code` of `HeapString::equals` and the
`STEP_CHUNK` comment that `24704ac` found (`03a3a71`); the paths of the lane `slow` that
`fb9a355` found (`057c6e3`).

Closed after the batch: the argument copies of the `call` and `new` instructions and the
copies `string_units` and `string_units_in` made without a memory charge, by
`story:copies-held-across-a-call-into-script-are-charged` (HOST-215).

## Rule breaches by units

- Three units started the Python interpreter once each: the frontend unit and the parameters
  unit ran `python3 --version`, the arrow unit ran `python3 -c 1`.
- Three units each waited once with `pgrep -f`; the next-entry unit is one of them.
- Units left build copies in the scratch directory, and the coordinator deleted them.

## How the batch was checked

Until the merge `99374f9` the coordinator compiled each merged commit and ran the tests of the
units it joined. A merge had not compiled (`30c6255`, E0428). `ae2345a` ran every test of the
package swell on the joined branch `cc41ae6` for the first time, since no unit had: 2 tests
were red. From `940bbbe` on, the coordinator ran the package in both collection modes after a
merge. From `7a7bf0a` on, it ran every test of the workspace after every merge whose base was
not the branch head: the package swell in both collection modes, the fault test, the other
packages, clippy with warnings denied and the check of the ESS target. The first such run found
three red tests at `7a7bf0a`, from units each tested on an older base (`3a2dd02`).

A unit whose branch was joined with the wave branch ran the same in its own tree, and the merge
has that tree: `eaa3631` (`28886e7`), `2e0572a` and `d624042` (`1bb4f10`), `7948625` and
`69a73dc` (`e1bdfb5`).

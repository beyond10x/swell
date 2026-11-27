---
format: aep.planning-md/3
id: review-result:adversary-close8-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit close8: green, six notes'
relations:
- reviews: story:slice-08-conformance-close
revision: 1
---
Pass 1 of the adversary on the unit close8 (impl/close8 at d61ed17b, one commit on 2b074619).
The adversary's report as it sent it; paths of the workstation are shortened.

```
unit: close8 (story:slice-08-conformance-close, wave big-2, impl/close8 at d61ed17b)
verdict: green
cases: executed 0→11 (22 runs, all passed), red 0
origin: introduced 2, pre-existing 4, undecided 0
wrote-outside-worktree: <scratch>/wave2/close8/adversary-1
needs-coordinator: yes
```

I found no defect that hides a slice-8 failure and no wrong count. Every number the unit states matched my own counts from the three sets of run lines. The six findings are notes: four are gaps in checks or text that were already there, and two are side effects of the close. None blocks the merge.

## Findings

**F1. `lists.sh` fails on the head run** (judgement, note, introduced)
- `sh docs/plan/slice-08/lists.sh .out/test262/runs.tsv` exits 1 with `slice 9478 lists 9339 incoming 0`, 148 `missing` and 9 `outside`.
- On the base run it exits 0 with `slice 9339 lists 9339 incoming 0`, which is what the report quotes. Check (7) only asks for the base, so the unit met its order.
- After the close, the lists no longer cover slice 8: the 148 cases that came in are on no list, and the 9 that moved out are still on `close-later.tsv`.
- Reached by any later run of `lists.sh` or `check.sh` on the wave branch. The lists are the coordinator's files.

**F2. "Every case of PrivateName/ comes in" is not quite true** (contract-drift, note, introduced)
- `docs/design/engine.md:4610` says every case of `test/staging/sm/PrivateName/` comes in, and rule 1 allows a directory line only "when every case of the directory is one".
- The directory holds 17 files. `modify-non-extensible.js` has the feature `nonextensible-applies-to-private`, which is excluded as stage 3. It is not run and still counts in `x-stage3`, which stays at 2591.
- No count moves. Only the sentence is inexact.

**F3. A missing `moves.tsv` line passes the moves check** (mutant, note, pre-existing)
- Plant P2, in the build node's copy of the tree: I removed the `builtins.js` line from `moves.tsv` and kept its rule line `dir … 13`. `test262-moves-check` exited 0 and printed `2030 case(s) moved … 13:21`.
- `task test262-check` went red, but only because the digest no longer matched. `test262-expected` rewrites the digest, after which nothing in the gate sees the gap.
- Only the story's checks (2) and (7) catch it, and they need run lines. My own comparison of the 7 rule lines against `moves.tsv` is clean: 9 of 9 cases have their line.

**F4. A wrong `from` field passes every check** (mutant, note, pre-existing)
- Plant P7, on a copy: the `from` of `builtins.js` set from 8 to 7. Checks (1) to (7) all exit 0, and the moves check reads no run lines, so it cannot see it either.
- The only trace is the groups line, which prints `all 8` instead of `all 9`. Someone has to compare that by eye.

**F5. Leftover "before slice 8" text in the tests** (judgement, note, pre-existing)
- `crates/syntax/tests/sources.rs:3043` builds the message `"{construct} is not supported before slice 8"`, but every shape has an empty `construct`, so that branch never runs.
- Stale comments: `sources.rs:3159`, `crates/swell/tests/async_functions.rs:605`, `crates/swell/tests/private_methods.rs:898` and `:2395`.
- No refusal naming slice 8 is left in `crates/*/src`: the story's refusal grep prints 0, and no run line holds `before slice 8`.

**F6. One slice-19 case fails on how slice 8 parses class fields** (judgement, note, pre-existing; owner `story:class-element-syntax-and-early-errors`)
- `test/staging/sm/fields/await-identifier-script.js` fails both runs with `await expression in a class field initializer (14:9)`.
- The case expects `x = await;` in a class field inside an async function to read `await` as a variable. That is SpiderMonkey's behaviour. Under the spec's grammar `Initializer[+In, ?Yield, ?Await]` it is an `await` with no operand, so it cannot parse there either. The engine documents its reading in `crates/syntax/src/parser/class.rs:36-41`.
- It stays in slice 19 correctly. No change proposed.

## What I checked and found clean

1. **The 9 moves out.** The first failure of each run is `ArrayBuffer`, `DataView`, `Uint8Array` or `__lookup{G,S}etter__`, as stated.
   - I wrote 11 variants with the later part taken out, ran in both modes. All 22 runs passed, so the moves hide no slice-8 defect.
   - Each evidence token is in the case itself. `Uint8Array` is the right token for `builtins.js`: the case never names `ArrayBuffer`.
   - `moves.tsv` lines have five fields, are in byte order, and are kind `part`.
2. **The 148 that came in.** My own scan finds the same 155 candidates and the same 148, with the same reasons for the 7 that stay.
   - 138 cases hold no later-slice token, 3 only in `deepEqual.js`, and 7 only in `propertyHelper.js`.
   - I read `propertyHelper.js`: the `Uint8Array` at line 42 sits behind the `typeof` test of line 40, and `_isTypedArray` catches the call of `false`. Line 236 is only reached by a bigint.
   - None of the 148 carries a flag or a feature line of a later slice.
3. **Rule lines.** No duplicates, no longer line under any new line. The only change of slice between base and head: `08->13` 10 runs, `08->18` 8, `19->08` 292, and no run changed its outcome.
4. **Cases passing for the wrong reason.** I read the messages of all 2,819 passing slice-8 runs that carry one; none is a refusal or a missing global.
   - Two suspicious messages turned out legitimate grammar errors rather than refusals: "unexpected token ==" in `nested-equality-init-err-contains-super`, and "unexpected token =>" in the class-heritage arrow case.
   - No passing run anywhere in the three runs carries a refusal. A token scan of all 9,478 slice-8 cases finds only method names (`escape`, `import`, `export`).
5. **Class-related failures in later slices.** 236 cases, sorted by first failure: all on later parts, except F6. `sm/class/methodsPrototype.js` fails on `.__proto__` (slice 18); my variant without it passes.
6. **Numbers.** The table, the block, the slice-19 counts (755 → 607), the part counts (class 126, `new.target` 12, `super` 7, private name 3) and the start messages (116 / 10 / 4 / 17 / 1) all match my counts. The second full run is byte-identical to the first.
7. **Plants against the checks.**
   - P1a (a run of an incoming case failed) turns (1) and (4) red.
   - P1b (a run of `escaped-new.js` failed) turns (1) and (3) red.
   - P2 turns (2) and (7) red; P3 turns (4) red; P6 turns (7) red.
   - In the moves check: P4 (token not in the case), P5 (rule line removed) and P9 (wrong target) go red; P8 (a rule line taking `sm/Array/for_of_1` into slice 8) passes, as rule 4 of section 12 already says.
8. **Requirements.** HOST-119 and HOST-223 name tests that exist in `private_element_hook.rs` and are not ignored, and their sentences in `host-interface.md` carry `[slice 8]`. No requirement is pending for slice 8.
9. **Documents still showing the pre-close counts.** For the coordinator: `AGENTS.md:696`, `website/data/status.json:154-159`, `docs/plan/waves.md:531`, and `docs/plan/wave-big-2.md:182,352,502`. These are already in `for-coordinator.md`, except the `wave-big-2.md` count lines.

## Cases added (not committed)
- `.out/adversary/v01-arraybuffer-regular-as-array.js` through `v11-sm-methodsPrototype-no-dunder-proto.js`, plus `run.sh` and `plants.sh`. Copies are in `…/adversary-1/cases/`.
- The jobs ran them from a temporary `adversary-cases/` directory at the tree root, which I removed afterwards. `git status --short` prints nothing.
- Run lines of the variants: `.out/adv-part/runs.tsv`. Plant outputs: `.out/adv-plants/`.

## Commands and result lines
- `slot --timeout 20m --fetch .out/adv-part/runs.tsv --fetch .out/adv-part/lines.txt sh adversary-cases/run.sh` → `runner exit 0`, `test262-slice 08 11 22 0 0 11 11 0 0`, then `exit 0`.
- `slot --timeout 15m --fetch .out/adv-plants sh adversary-cases/plants.sh`:
  - P0 (no plant): `2031 case(s) moved`, exit 0
  - P2: moves check exit 0 with `2030 case(s) moved`; `test262-check` exit 201 (digest differs)
  - P4: exit 201, `evidence "ArrayBuffer" is found neither in the case nor in a harness file it includes`
  - P5: exit 201, `the block gives the case slice 8, not 13`
  - P8: exit 0
  - P9: exit 201
  - then `restored`, `exit 0`
- `sh adversary-1/plants-local.sh` → output in `adversary-1/plants-local.out`.
- `lists.sh` → base `slice 9339 lists 9339 incoming 0`, exit 0; head `slice 9478 lists 9339 incoming 0`, exit 1.
- `check.sh story-classes.cases.txt` → `passed 37097 bound 37097 B 37097 F 0 R 0`, exit 0.
- `cmp run1/runs.tsv .out/test262/runs.tsv` → `identical`.

## Not attacked
- I did not read a sample of the positive cases that pass through `assert.throws`. The token scan and the fact that no passing run carries a refusal stand in for it.
- I did not run the host requirement tests themselves.
- No full run of test262, no `task check`.

```findings
- file: docs/plan/slice-08/lists.sh
  line: 10
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'Over the head run lists.sh exits 1 (148 missing, 9 outside, slice 9478 lists 9339 incoming 0): after the close the lists of docs/plan/slice-08 no longer partition slice 8.'
- file: docs/design/engine.md
  line: 4610
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The paragraph says every case of test/staging/sm/PrivateName/ comes in, but 1 of its 17 files (modify-non-extensible.js, excluded as stage3) is not run and stays in x-stage3; no count moves.
- file: taskfiles/test262.yml
  line: 229
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: With the moves.tsv line of a case removed and its rule line kept, test262-moves-check exits 0 (2030 moved) and test262-check fails only on the digest, so after test262-expected only the story's run-line checks see the gap.
- file: conformance/test262/moves.tsv
  line: 1743
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: A wrong from field (8 changed to 7) passes checks (1) to (7) and the moves check; only the groups line printing all 8 instead of 9 shows it.
- file: crates/syntax/tests/sources.rs
  line: 3043
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: A never-taken branch still builds the refusal "is not supported before slice 8", and stale comments name refusals before slice 8 (sources.rs:3159, async_functions.rs:605, private_methods.rs:898 and 2395).
- file: crates/syntax/src/parser/class.rs
  line: 60
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: staging/sm/fields/await-identifier-script.js (slice 19) fails on slice 8's parsing of an await in a class field initializer, but it expects SpiderMonkey behaviour that the spec grammar Initializer[+In, ?Yield, ?Await] does not allow, so it stays in slice 19 with no change.
```

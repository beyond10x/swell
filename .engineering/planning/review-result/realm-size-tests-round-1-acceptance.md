---
format: aep.planning-md/3
id: review-result:realm-size-tests-round-1-acceptance
kind: review-result
status: active
title: Tests hold at any size of the realm, round 1, the acceptance critic
relations:
- reviews: story:tests-hold-at-any-size-of-the-realm
revision: 1
---
needs-revision

Findings (draft line; code; defect; smallest fix of the draft text):

1. blocker: Acceptance 1 (draft 148-151). The fallback "red at none ... a limit of the instrument" lets the unit pass with none of the four shown red, and the story still changes them (Writes, 135-136). Nothing in the draft says the padding reproduces the Date getters' arena order, and the draft's own limit list (176-177) says it does not. Fix: add that a test of the four red at no k of the list stops the unit, which reports its readings per k and lists that test as unproven. Alternatively, require the unit to name another way to show it red.
2. blocker: Change 5 / Acceptance 1 (draft 118, 149). `nextest-report` exits 1 on a red run (`taskfiles/nextest.yml:196,224`), so a task that calls it per k stops at the first red k. "Naming every red test with its k and mode" and "lists every red test per k and mode" cannot be met. Fix: add "runs every k and mode, then exits non-zero if any failed".
3. blocker: Acceptance 6 and 8 (draft 163, 167). The base is "`1e7092d` plus the Math unit" (draft 13), and that unit writes `crates/swell/src`. So `git diff --stat 1e7092d -- crates/swell/src` is not empty. The Acceptance 8 diff, `git diff --stat -- Cargo.lock '*/Cargo.toml'`, names no revision, so it prints nothing once the work is committed. Fix: name the commit the unit starts from, and diff against it in both.
4. warning: Change 5, "no build between" (draft 111-112; `taskfiles/nextest.yml:245` and `Taskfile.yml:172-174`). `compile-check` exits 0 when BUILT_BY is empty, and the `slow` steps pass none. The draft's check is inert as written, and no acceptance tests it. Fix: say the task passes BUILT_BY (a named build step with its own log) to `nextest-report`, and add to Acceptance 3 "no run log has a `Compiling` line".
5. warning: Change 2 / Acceptance 7 (draft 90-93, 164). The exception cannot be checked. "Cannot answer Normal (released, faulted, re-entered)" is narrower than the real sites: setups that throw (`realms.rs:323`, `host_objects.rs:633`), shape-measuring sites (`fuzz.rs:1707`, `misuse.rs:966-969`) and Err cases (`faults.rs:63`, `eval.rs:302`). The helper "answers what `Agent::create_realm` answers", so these could use it unchanged. Fix: list the sites as `file::fn`, each with the answer it asserts, and require Acceptance 7's grep to match that list by file and enclosing function.
6. warning: Acceptance 7 (draft 164; `fuzz.rs:1641,2142,2151`, `fuzz.rs:1711`, `misuse.rs:968`). The pattern `create_realm(` also matches `op_create_realm(` and the label strings. 3 of the "50" are `op_create_realm`, so the real calls number about 44. The output can never be "only the listed sites". Fix: use `'\.create_realm('` in Context (90) and Acceptance 7, and say "50" is lines matched.
7. warning: Change 1 (draft 79-89; `builtins_symbol_values.rs:351-446,387-388,555`). The Symbol walk takes both `getOwnPropertyNames` and `getOwnPropertySymbols` of every object reached, so it reaches the padding object and its k functions. It does not walk Symbol keys on other tests: `builtins_object_enumeration.rs:575-640` reads String keys only, and no `Reflect.ownKeys` use exists in the tests.
   - The printed count becomes N+k, and calls become (N+k) × places (4, or 2 in collect mode, line 470).
   - The expected-error list (574-600) holds, because padding answers `undefined`, which counts as "normal".
   - Its time bound (468-469: 15 to 30 s at 4 places in collect mode) is nearest the 60 s kill at k = 200.
   - The draft says none of this, yet Acceptance 1 relies on the count.
   - Fix: add a sentence to Change 1 giving these effects, and say N0 is read at k = 0.
8. warning: Change 1 (draft 79-89). It does not say what `pad_realm` makes at k = 0. If it still makes the object and the Symbol, then the unset variable changes every test's realm, and "the base" of Acceptance 1 differs from k = 0. Fix: say "k = 0 makes nothing".
9. warning: Acceptance 3 vs Not in this story (draft 156-157, 183-185). If `host_187` is red at some k, the task exits non-zero and the draft says change nothing. `host_objects.rs:1195` takes `heap_before` after the helper would pad, and the ceiling's headroom is about 14,300 bytes (draft 59-60). The lane `slow` would stay red with no exit. Fix: say what the task does with a named, reported red (an exclusion with its story) or that the unit stops.
10. warning: Context (draft 69; `.engineering/planning/story/gate-test-steps-fit-four-cores.md:29,32-33`). 94.3 s was measured in a pool job on 16 cores at load 33, not on the forge's 4 cores. Fix: "127.9 on the forge's 4 cores; 94.3 in the pool".
11. warning: Context (draft 38-41). The first two commands as written, `git grep -cE 'memory_bytes *[:=]'` and the `heap_bytes()` one, have no pathspec. They give 118 sites in 84 files and 583 in 118 files, not 65/45 and 297/50. Only the 53 command carries `-- 'crates/swell/tests/*.rs' ':!crates/swell/tests/support'`. Fix: add that pathspec to both commands.
12. warning: Change 5 (draft 108-109, 106-107). "The k of Acceptance 1 if it is not in it" is unclear, because Acceptance 1 passes the whole list on the command line. `REALM_SIZES_MODE` (119-120) has no acceptance. Fix: say that `REALM_SIZES` replaces the default, drop the clause, and add a check for the mode split or cut it.

Checked and holding: the `file:line` cites to `host.rs`, `limits.rs`, `builtin_function.rs`, `arena.rs`, `Taskfile.yml:180`, `slow.yml:6,12`, `nextest.yml:140,228` and `Cargo.toml:6`. Also the 23 files and 50 lines, 86 files, 53 files and 30 files, and the `Symbol` walk print format. Not confirmed: the `swell` reading and timing figures in Context, and whether the `-p swell` build reuses the `--workspace` build with no `Compiling` line.

What I read: 1 draft (195 lines), `Taskfile.yml`, `taskfiles/nextest.yml` and `collect-mode.yml`, `builtins_symbol_values.rs`, `builtins_object_enumeration.rs`, `host_objects.rs`, and the cited lines in `src` and `tests`. Out of my lane: the choice of Symbol key and the arena-order limits of the instrument (design).

```findings
- file: .engineering/drafts/tests-hold-at-any-size-of-the-realm.md
  line: 148
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the fallback (a test of the four red at no k is a limit of the instrument) lets Acceptance 1 pass with none of the four shown red while the story still rewrites them; state that red at no k stops the unit and the test is reported unproven'
- file: .engineering/drafts/tests-hold-at-any-size-of-the-realm.md
  line: 118
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'nextest-report exits 1 on a red run (taskfiles/nextest.yml:196,224), so the task stops at the first red k and cannot name every red test per k and mode; say the task runs every k and mode and fails at the end'
- file: .engineering/drafts/tests-hold-at-any-size-of-the-realm.md
  line: 163
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'git diff --stat 1e7092d -- crates/swell/src is not empty because the base includes the Math unit, which writes src; the Acceptance 8 diff names no revision and is vacuous once committed; name the commit the unit starts from in both'
- file: .engineering/drafts/tests-hold-at-any-size-of-the-realm.md
  line: 111
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'compile-check checks nothing when BUILT_BY is empty (taskfiles/nextest.yml:245) and the slow steps pass none, so "no build between" is inert and untested; pass BUILT_BY and add "no run log has a Compiling line" to Acceptance 3'
- file: .engineering/drafts/tests-hold-at-any-size-of-the-realm.md
  line: 90
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the exception for an agent that cannot answer Normal is judgement and narrower than the real sites (realms.rs:323, host_objects.rs:633, fuzz.rs:1707, misuse.rs:966, faults.rs:63, eval.rs:302); list sites as file::fn with the answer each asserts and have Acceptance 7 compare against that list'
- file: .engineering/drafts/tests-hold-at-any-size-of-the-realm.md
  line: 164
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the grep pattern create_realm( also matches op_create_realm( (fuzz.rs:1641,2142,2151) and label strings (fuzz.rs:1711, misuse.rs:968), so its output can never be only the listed sites and the count of 50 includes 3+ non-calls; use \.create_realm('
- file: .engineering/drafts/tests-hold-at-any-size-of-the-realm.md
  line: 79
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the draft does not say the Symbol walk (builtins_symbol_values.rs:351-446,387-388) reaches the padding under its Symbol key: count N+k, calls (N+k) x places, expected-error list unchanged, time at k=200 in collect mode; add that and say N0 is read at k=0'
- file: .engineering/drafts/tests-hold-at-any-size-of-the-realm.md
  line: 79
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'pad_realm at k=0 is not said to make nothing; if it still binds an object and a Symbol, unset-variable runs differ from the base; state k=0 makes nothing'
- file: .engineering/drafts/tests-hold-at-any-size-of-the-realm.md
  line: 156
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'Acceptance 3 demands 0 failed at every k but Not-in-this-story (183-185) forbids changing host_187 if it is red (host_objects.rs:1195, ceiling headroom about 14,300 bytes); state the task exclusion or stop rule for a reported red'
- file: .engineering/drafts/tests-hold-at-any-size-of-the-realm.md
  line: 69
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '94.3 s was measured in a pool job on 16 cores at load 33, not on the forge 4 cores (.engineering/planning/story/gate-test-steps-fit-four-cores.md:32-33); correct the sentence'
- file: .engineering/drafts/tests-hold-at-any-size-of-the-realm.md
  line: 38
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the first two grep commands have no pathspec and give 118 sites in 84 files and 583 in 118 files, not 65/45 and 297/50; add the pathspec of the 53 command'
- file: .engineering/drafts/tests-hold-at-any-size-of-the-realm.md
  line: 108
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'REALM_SIZES default plus "the k of Acceptance 1 if not in it" conflicts with Acceptance 1 passing the whole list, and REALM_SIZES_MODE has no acceptance; say REALM_SIZES replaces the default and add a check for the mode split or drop it'
```

---
format: aep.planning-md/3
id: review-result:proxy-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit proxy: tests only and the merge of the wave branch, no defect'
relations:
- reviews: story:proxy-objects-and-access-traps
revision: 1
---
Coordinator's read of correction 1 of the unit proxy (impl/proxy, 859debad to cb029fbd),
2026-11-26. Verdict: no defect; the correction changes one test file and merges the wave
branch without a conflict.

The correction answers review-result:adversary-proxy-pass-1, whose three findings are changes
of the code that the unit's 19 tests did not catch (the code itself had none of the defects).
It is two commits.

Commit ae1ee7d1 (1 file, `crates/swell/tests/proxy.rs`, 998 added lines; no line of the
implementation):

- 32 of the adversary's 33 cases under the unit's names: 14 stack tests from one macro
  (`deep_chain!`) and 18 test functions. The module runs 51 tests where it ran 19.
- The 33rd case (a `super` property read through a Proxy as the prototype) checked nothing on
  the unit's base, which had no `super`; it is not in the file. Run by the implementor in a
  copy after the merge below, it passes in both modes (`Normal("\"px|x:true\"")`). Its text is
  kept for the brief of the unit `proxytraps`, which adds it.
- Measured by the implementor in a copy of the tree, each change planted alone: with SameValue
  replaced by `==` in two invariants (`object/invariants.rs:144,186`),
  `each_invariant_both_sides` is red; with `call_trap` returning on a throw without adding its
  count back (`object/proxy.rs:248`),
  `a_throw_caught_inside_nested_proxies_leaves_the_count_exact` is red (`"20|false:14"`); with
  the IsArray loop charging no step per Proxy (`ops/testing.rs:55`),
  `is_array_and_get_function_realm_charge_a_step_per_proxy` is red. Each time 50 of 51 pass;
  without a planted change all 51 pass.
- Times of the new cases, each run alone on the build node: none over 5 s; the slowest with
  `collect-every-safe-point` is `step_budget_at_every_step_of_proxy_operations_leaves_no_count`
  at 4.82 s. Their sum is 10.4 s in the default mode and 61.5 s in the collect mode (the
  unit's 19 tests: 0.49 s and 7.7 s). In the whole collect run under load one of them took
  7.03 s. The gate's time is read at the wave's close; a step over its budget gets a story.

Commit cb029fbd: the merge of `wave/big-2` at e7263869 (the units classsyntax and super, the
close of slice 4). No conflict; no file of the unit changed in the merge; no count a test
holds moved.

On the merged head the implementor ran: swell main `1256 passed, 1 skipped` in both modes, the
library's tests 251 and 247, the story's 244 cases of test262 with the guard `passed 1325
bound 1325 B 1081 F 244 R 0`, `task test-realm-sizes` green at all seven sizes in both modes
(`realm-sizes: 0 run(s) red`), the stack tests in the profiles release and stack-debug (4
passed each), clippy and fmt clean.

Not checked by the correction: the three changes were planted in the default mode only. Not
read: the 998 lines of the cases line by line; the read took their names and the macro.

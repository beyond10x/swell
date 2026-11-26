---
format: aep.planning-md/3
id: review-result:adversary-proxy-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit proxy of wave big-2
relations:
- reviews: story:proxy-objects-and-access-traps
revision: 1
---
Adversary, pass 1 on the unit proxy of wave big-2 (impl/proxy at 859debad), 2026-11-26. The pass ran in two parts; this is the report of the second part, which reran and extended the first part's cases.
Verdict: no defect found. 3 findings, all introduced notes (changes of the code the unit's 19 tests miss); 52 cases per mode, 0 red; task test-realm-sizes green at all 7 sizes in both modes.

The adversary pass found no defect in the `proxy` unit. 33 new cases pass in both modes. Three planted mutants get past the unit's own 19 tests, and my cases catch each of them.

```
unit: proxy (wave big-2), branch impl/proxy at 859debad, worktree <trees>/swell-impl-proxy
verdict: green
cases: executed 19→52, red 0
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/proxy/adversary-1b/ (logs, mutants.sh, measure.sh, the removed probe test probe-test.rs.txt)
needs-coordinator: no
```

The case count is per mode: `proxy::` in default and in `collect-every-safe-point`, 52 passed in each. The three origins are the three notes below.

## Findings (all notes; the code does not have these defects)

1. **Mutant: SameValue replaced by `==`** in `invariants::get` and `invariants::set` (`object/invariants.rs:144,186`), planted in `.out/adv-mutants`.
   - The unit's 19 tests pass on it. `adv_each_invariant_both_sides` (`crates/swell/tests/proxy.rs:1241`) fails: a `get` trap answering `-0` for `+0` gives `-0` instead of a TypeError, `NaN` gives a TypeError, and `"a"+"b"` against `"ab"` gives a TypeError.
   - The listed test262 cases have no ±0 or NaN case.
2. **Mutant: `call_trap` returns early on a throw without adding its count back** (`object/proxy.rs:248`).
   - The unit's 19 tests pass, because the count saturates at 0 at top level.
   - `adv_a_throw_caught_inside_nested_proxies_leaves_the_count_exact` (`:1089`) fails with `"20|false:14"`. That test catches throws inside a trap that runs under 5 trapless proxies.
3. **Mutant: the IsArray loop charges no step per Proxy** (`ops/testing.rs:55`).
   - The unit's tests pass. `adv_is_array_and_get_function_realm_charge_a_step_per_proxy` (`:1129`) fails.
   - Measured on the real code: IsArray costs 10 steps on the base array and 5,010 through 5,000 proxies. GetFunctionRealm costs 23 steps direct and 623 through 200 proxies.

## Measured, nothing found

**Item 4, stack per level.** Smallest stack found by bisection in child processes, dev profile with `debug = false`. The bound per re-entry is `STACK_BYTES_PER_REENTRY` = 14,336 bytes.

| Chain | Bytes per level | Re-entries per level | Bytes per re-entry |
|---|---|---|---|
| `{get: Reflect.get}` | 4,770 | 1 | 4,770 |
| `{set: Reflect.set}` | 5,074 | 1 | 5,074 |
| `{construct: Reflect.construct}` | 6,991 | 2 (construct and its "prototype" read) | about 5,400 for the construct level |
| trapless `get` | 1,584 | 1 | 1,584 |
| trapless `construct` | 3,489 | 2 | about 1,900 for the construct level |
| script `get` trap | 9,741 | 2 | 4,870 |

- 14 chains go 10,000 deep, and `BUILTINS_REENTRY + 10` deep, with `parseDeep` at the base. They end in the RangeError, never a stack overflow, on 7/8 of `min_stack_bytes()` (12,838,560 bytes at 1,000).
- The chains cover builtin, bound-builtin and trapless traps for all six operations, the seven forwarding methods, and the prototype-chain walk.
- A prototype cycle made through a Proxy ends in a RangeError for read, `in` and write. `instanceof`, `for`-`in` and `isPrototypeOf` end at the step budget.

**Item 5, budgets.** `proxy_nesting` is back to 0 after every way out tested:
- a throwing trap, and a RangeError of the bound inside a trap;
- recursion through `get`, `apply`, `construct`, `has` in `with`, `set` on the prototype chain, `deleteProperty`, and the prototype cycle;
- the step budget, an interrupt, and the memory budget hit inside a trap at depth (`live_bytes` 248,436 before and after);
- the step budget cut at every one of the 357 step offsets of a mixed trap and invariant script. Every next entry reaches exactly `reentry_depth` (20).

**Items 1, 2, 3, 6, 7, 8.** All pass, in both modes.
- **Item 1 (trap reads):** GetMethod runs once. Keys arrive as String or Symbol (`-0`→"0", `1e21`→"1e+21"), the receiver as given, and fresh argument Arrays. `null` or `undefined` traps forward; non-callable traps throw (24 of 24). With `with`, the order is has:v has:x get:@@unscopables has:x get:x.
- **Item 2 (invariants):** each invariant on both sides, including on targets that are themselves Proxies.
- **Item 3 (revocation):** the revocation cases pass. The planted mutant that keeps the handler makes `revoke_is_idempotent_and_drops_target_and_handler` fail in both modes (collect mode: 1,196 bytes freed against 1,600 asserted), so that assertion can still fail there.
- **Item 6 (call and construct):** every kind of function; `this` of every type; the prototype read through the `get` trap; species, callbacks and `bind`.
- **Item 7 (builtins):** about 60 builtins with Proxies of each kind. A revoke function used as a `then` handler, an executor or a thenable's `then` reaches no fault arm.
- **Item 8 (collector):** a target held only by the Proxy while the trap revokes it and allocates before the invariant read, and a Proxy reachable only through its own handler.

**Item 9, code reading (1,041 lines).** I read `object/proxy.rs` (602), `object/invariants.rs` (293) and `builtins/proxy.rs` (146), plus the diffs of `ordinary.rs`, `testing.rs`, `objects.rs`, `interp/mod.rs` and `object/mod.rs`.
- No `unwrap`, `expect` or unchecked index.
- `proxy_nesting` uses saturating arithmetic on every way out.
- The argument Arrays are charged before they are made.
- The values each method keeps go onto the kept stack through `keep()` without a charge. That is the engine's convention, bounded by `reentry_depth`.

**Realm sizes** (`slot --timeout 30m task test-realm-sizes`, on 859debad plus my uncommitted `tests/proxy.rs`), no red test:
```
realm-sizes 4 default: 1457 passed, 0 failed, 1 ignored
realm-sizes 4 collect: 1453 passed, 0 failed, 1 ignored
realm-sizes 11 default: 1457 passed, 0 failed, 1 ignored
realm-sizes 11 collect: 1453 passed, 0 failed, 1 ignored
realm-sizes 17 default: 1457 passed, 0 failed, 1 ignored
realm-sizes 17 collect: 1453 passed, 0 failed, 1 ignored
realm-sizes 38 default: 1457 passed, 0 failed, 1 ignored
realm-sizes 38 collect: 1453 passed, 0 failed, 1 ignored
realm-sizes 64 default: 1457 passed, 0 failed, 1 ignored
realm-sizes 64 collect: 1453 passed, 0 failed, 1 ignored
realm-sizes 128 default: 1457 passed, 0 failed, 1 ignored
realm-sizes 128 collect: 1453 passed, 0 failed, 1 ignored
realm-sizes 200 default: 1457 passed, 0 failed, 1 ignored
realm-sizes 200 collect: 1453 passed, 0 failed, 1 ignored
realm-sizes: 0 run(s) red
```

## Cases left uncommitted

All 33 are in `crates/swell/tests/proxy.rs` (+882 lines, compiles under `-D warnings`), all named `adv_*`: 14 stack tests from the `adv_deep!` macro and 19 test functions. None is red.

Times, each test run alone:
- **Default mode:** the longest is `adv_step_budget_at_every_step…` at 1.65 s; four builtin-trap stack tests take about 1.3 s each.
- **Collect mode, dev profile:** all under 5 s except `adv_a_throw_caught…`, which took 5.5 s. I then shortened its search; that version passed in the realm-sizes run but I did not time it alone again.

The planted mutant copy and its logs are in `.out/adv-mutants` (ignored by git, 38 MB, no `target/`).

## Not checked

- The stack frames in the `stack-debug` and `release` profiles. Only the dev profile was measured.
- `super`, `new.target` and `Object.prototype.__proto__` do not exist on this base. `adv_super_property_through_a_proxy_prototype` passes without checking anything until they land.
- test262 (not mine to run), and whether the listed test262 cases catch mutants 1 to 3.
- The deepest chain length each stack test reached. It is printed only inside the child process, so I have only the assertion that it was at least half of `BUILTINS_REENTRY` minus 4.

```findings
- file: crates/swell/src/object/invariants.rs
  line: 144
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Replacing SameValue with == in invariants::get and invariants::set passes the unit's 19 tests; adv_each_invariant_both_sides catches it (+0/-0, NaN, a computed String).
- file: crates/swell/src/object/proxy.rs
  line: 248
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: A call_trap that returns on a trap's throw without adding back its count passes the unit's 19 tests; adv_a_throw_caught_inside_nested_proxies_leaves_the_count_exact catches it ("20|false:14").
- file: crates/swell/src/ops/testing.rs
  line: 55
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: An IsArray loop that charges no step per Proxy passes the unit's 19 tests; adv_is_array_and_get_function_realm_charge_a_step_per_proxy catches it.
```

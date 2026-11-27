---
format: aep.planning-md/3
id: review-result:adversary-proxytraps-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit proxytraps: red, five failing cases of 14 and ten findings'
relations:
- reviews: story:proxy-descriptor-prototype-and-extensibility-traps
revision: 1
---
Adversary pass 1 on `impl/proxytraps` at `c8e54d7f` (wave big-2, slice 9, order 3), run on 2026-11-26.
The report below is the adversary's last message; paths of the workstation are replaced.

```
unit: proxytraps (wave big-2), branch impl/proxytraps at c8e54d7f, plus my uncommitted cases in crates/swell/tests/proxy_shape_traps.rs
verdict: red
cases: executed 96→110, red 5
origin: introduced 9, pre-existing 1, undecided 0
wrote-outside-worktree: <scratch>/wave2/proxytraps/adversary-1/ (scripts, logs, mutant diffs, measure.txt, the scratch clone mut/ with its target/ removed)
needs-coordinator: yes. The fixes for findings 1 to 4 belong in ops/objects.rs and interp/dispatch.rs, which are outside the story's file list. Finding 10 is a time defect of the wave that was there before the unit.
```

**The unit is not safe to merge.** Five of my 14 new cases fail in every mode. Four of them end in `Terminated(EngineFault)`: for-in and JSON lose keys and objects that a trap made, once a later trap runs a collection. The fifth is a for-in result that breaks the spec's rules.

**1. Diff of the worktree**
```
 crates/swell/tests/proxy_shape_traps.rs | 377 ++++++++++++++++++++++++++++++++
 1 file changed, 377 insertions(+)
```
Only a test file. One more file is inside the worktree but ignored by git: `.out/adv-measure.txt`.

**2. Cases added** (all in `crates/swell/tests/proxy_shape_traps.rs`, named `adv_*`)

Red now, in the default mode, the gate's collect build and the slow lane's collect build:
- `adv_json_stringify_holds_the_keys_an_own_keys_trap_made` (:970): gives `Terminated(EngineFault)` instead of `"{|k1|:|vk1|,…}"`.
- `adv_json_parse_reviver_walk_holds_the_keys_an_own_keys_trap_made` (:990): EngineFault instead of `"a,k1,k2,b,"`.
- `adv_for_in_holds_the_keys_an_own_keys_trap_made` (:1011): EngineFault instead of `"k1,k2,k3"`.
- `adv_for_in_holds_a_prototype_a_get_prototype_of_trap_made` (:1033): EngineFault instead of `"z1,z2"`.
- `adv_for_in_visits_the_keys_of_the_prototype_the_trap_answers` (:1170): `left: Normal("\"a,b\"")`, `right: Normal("\"a,b,c\"")`.

First red output of the four fault cases, verbatim (run `mine3`):
```
thread 'proxy_shape_traps::adv_for_in_holds_a_prototype_a_get_prototype_of_trap_made' (1918232) panicked at crates/swell/tests/proxy_shape_traps.rs:951:5:
  left: Terminated(EngineFault)
 right: Normal("\"z1,z2\"")
```
Before that, two runs of these four cases and `adv_the_target_descriptor_is_held_across_is_extensible` failed because of my own test bugs, not the engine. The first run hit `StepBudget` (`repeat` spends a step per code unit). The second got a `TypeError` (`undefined.length` in my `junk()` helper). I fixed both before the run quoted above.

Green now, each a guard:
- `adv_the_target_descriptor_is_held_across_is_extensible` (:1063): catches mutants 2 and 3.
- `adv_a_target_key_without_a_descriptor_is_configurable` (:1185): catches mutant 1.
- `adv_define_property_checks_i12_before_i13` (:1307): catches mutant 4.
- `adv_super_set_holds_its_key_across_the_receivers_trap` (:1147).
- `adv_a_trapless_chain_reaches_the_same_bound_through_each_builtin` (:1110): 8 builtins and 2 instructions each give `ok/RangeError` at 20/21 proxies.
- `adv_is_compatible_for_generic_and_accessor_descriptors` (:1091).
- `adv_each_builtin_walk_calls_the_traps_in_the_order_of_its_clause` (:1197): 17 builtins checked against their clauses.
- `adv_the_answer_of_a_trap_is_read_in_the_order_of_its_clause` (:1261).
- `adv_own_keys_keeps_strings_of_numbers_as_listed` (:1290).

Two of these I trimmed for time after their alone run; the suite run below covers the trimmed versions.

**3. Suite run, after the cases existed**
- `cargo nextest run --locked -p swell --test main -E 'test(/^proxy_shape_traps::/) | test(/^proxy::/)'`: `Summary [ 17.006s] 110 tests run: 105 passed, 5 failed`, exit 100.
- The same with `--features collect-every-safe-point`: `Summary [ 11.292s] 110 tests run: 105 passed, 5 failed`, exit 100.
- "Before" count, the same filter plus `& not test(/::adv_/)`, default mode: `96 tests run: 96 passed`, exit 0.

**4. Findings** (covering c8e54d7f plus my test file)

1. **JSON keys lost (blocker).** `ops/objects.rs:433`: `enumerable_own_keys` and `push_enumerable_keys` keep no key of `own_keys` while each `getOwnPropertyDescriptor` trap runs code.
   - Reached by `JSON.stringify` of a Proxy, and by `JSON.parse` with a reviver that puts a Proxy in place. Any script, default limits.
   - Origin: introduced. On the base, a Proxy's [[GetOwnProperty]] ran no code.
   - Fix: hold the keys the way `held_own_keys` does.
2. **for-in keys lost (blocker).** `ops/objects.rs:740` and `:762`: `enumerate_chain` and `enumerate_own_keys` keep neither `own_keys`, nor `found`, nor `visited`.
   - Reached by `for (k in proxy)`.
   - Origin: introduced.
3. **for-in prototype lost (blocker).** `ops/objects.rs:744`: `current = self.get_prototype_of(o)` sits in a Rust local, unrooted, while that object's own traps run.
   - Reached by a `getPrototypeOf` trap that returns a new Proxy.
   - Origin: introduced.
4. **for-in skips keys of the trap's prototype (warning).** `interp/dispatch.rs:1691`: `ForInNext` filters each key with `HasProperty` of the Proxy.
   - Without a `has` trap that reads the target's own chain, so a non-deleted enumerable key of the prototype the trap answered is dropped. Test: :1170.
   - `#sec-enumerate-object-properties` requires that key; only a property deleted before it is processed may be ignored. Test262 has no case of this shape (grep).
   - The unit changed its own test (:601) to avoid the case.
   - Origin: introduced (exposed by the `getPrototypeOf` trap).
5. **Mutants the unit's tests miss (notes).** The unit's 96 tests stayed green under each change below; my named case went red. Each was applied alone in a scratch clone; the diffs are in `mutant-*.diff`.
   - `proxy.rs:791`, `is_some_and` changed to `is_none_or`: caught by :1185.
   - `proxy.rs:532`, `keep_property(current)` removed (collect mode): caught by :1063 with EngineFault.
   - `proxy.rs:603`, the same removal in [[DefineOwnProperty]]: caught by :1063.
   - `proxy.rs:343`, `get` and `set` swapped in FromPropertyDescriptor: caught by :1307.
   - Test262's `ownKeys` and `defineProperty` directories have no case of shapes 1 or 4 (grep only; I did not run them).
6. **The unit's chain tests are over the story's time bounds (warning, introduced).** Measured in the slow lane's collect build (opt-level 0); the story names 2 s as its bound:
   - the seven new chains take 2.83 to 3.27 s;
   - `step_budget_at_every_step_of_an_own_keys_call…` takes 2.66 s.

   In the default mode, `builtin_get_prototype_of…` takes 1.83 s and `own_keys_trap_chain…` 1.85 s (story bound 1.5 s). In the gate's collect build (opt-level 1) the same tests take 0.28 to 0.53 s.
   - **Cause.** The search in `tests/proxy.rs:838` is quadratic: the clause reads the chain below again at every level. My timing probe (removed afterwards) shows one successful attempt at 1,000 levels costs 1.4 to 1.8 s and about 1.0M steps; a trapless chain at the same depth costs 0.013 s and 21k steps. The bisection runs about 8 successful attempts.
   - **Smallest form.** One attempt at the expected depth plus one at `over` gives the same stack assertion: the defect it guards is a frame grown past `STACK_BYTES_PER_REENTRY`.
   - **Collect mode.** The stack per level does not depend on the collector, so the collect mode can run one combined chain, as `trapless_forwarding_chains` does, and leave the per-trap chains to the default mode and the slow lane.
7. **Four older chain tests take 14 to 16 s each (warning, pre-existing).** `tests/proxy.rs:905–927, 941`:
   - `builtin_get` 14.45 s, `builtin_set` 14.16 s, `builtin_delete` 16.00 s, `bound_builtin_get` 15.53 s, default mode.
   - Together 60 s of the 73 s the `proxy::` module costs in the default mode.
   - Same quadratic bisection; these entries are unchanged since the base, where the unit measured 13.8 s for `builtin_get`.

**Times**, each test run alone with `--exact --test-threads 1`, CPU user plus system; the full table is in `measure.txt`:

| Build | `proxy::` (62 tests) | `proxy_shape_traps::` (48 tests) | Total |
|---|---|---|---|
| default | 72.99 s | 2.53 s | 75.52 s |
| gate's collect (opt-level 1) | 13.25 s | 1.82 s | 15.07 s |
| slow lane's collect (opt-level 0) | 103.78 s | 14.89 s | 118.67 s |

- None of my cases is over its bound; the slowest is 0.835 s in the slow lane.
- Other tests over their bound in the slow lane: 14 older chain tests at 2.66 to 3.13 s, `is_array…` 4.50 s, `nesting_is_back_to_zero…` 5.49 s, `step_budget…proxy_operations` 3.92 s, `a_throw_caught…` 2.89 s, `each_trap_reads_the_handler_once…` 2.42 s.

**Code read:** about 1,500 lines (`invariants.rs` 867, the new code in `proxy.rs` about 530, the changed part of `builtins/function.rs` about 90). No `unwrap`, `expect` or unchecked index. Every charge comes before its reserve. Every exit of the seven methods passes through `in_proxy_method`.

**5. Attacked, nothing found**
- Trap order of 17 builtins (keys, values, entries, assign, spread, rest, getOwnPropertyNames, getOwnPropertyDescriptors, freeze, isFrozen, seal, isSealed, JSON, instanceof, isPrototypeOf, setPrototypeOf, push on an Array): all follow their clauses.
- The order in which ToPropertyDescriptor and CreateListFromArrayLike read a trap's answer: correct.
- Strings of numbers as `ownKeys` keys, and I20/I21 on index keys: correct.
- I12 is checked before I13: correct.
- IsCompatiblePropertyDescriptor with generic and accessor inputs: correct.
- After the `enter_proxy` change, the re-entry bound is the same through builtins and through instructions.
- `super[k] = v` with a Proxy receiver holds its key.

Not attacked:
- The step budget cut at every step of the other six traps (by reading only).
- Interrupts and the memory budget inside each of the six.
- An answer of 100,000 keys (the 10,000-iteration loop rule).
- A planted leak against the unit's memory test.
- The release and stack-debug stack profiles.
- `refuse` and `room` 0 called directly (crate-private; would need a test module in a source file).

**6. Paths written outside the worktree**
`<scratch>/wave2/proxytraps/adversary-1/`, about 40 MB:
- scripts: `mine.sh`, `mine-new.sh`, `mutants.sh`, `suite.sh`, `measure.sh`, `summarize.awk`, `probe.rs.txt`
- logs: `mine*.log` and `.out`, `mutant-*.log` and `.diff`, `suite*.log`, `probe.log`, `measure-job.log`, `measure.txt`
- backups: `proxy_shape_traps.rs.orig` and `.final`, `proxy.rs.head`, `src.diff`
- `mut/`: a `git clone --shared` of the branch, its `target/` deleted.

```findings
- file: crates/swell/src/ops/objects.rs
  line: 433
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: enumerable_own_keys holds no key of the own keys while a Proxy's getOwnPropertyDescriptor trap runs, so JSON.stringify and the JSON.parse reviver walk end in Terminated(EngineFault) after a collection.
- file: crates/swell/src/ops/objects.rs
  line: 740
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: enumerate_chain keeps neither own_keys, found nor visited across the traps of a Proxy, so for-in over keys an ownKeys trap made ends in Terminated(EngineFault).
- file: crates/swell/src/ops/objects.rs
  line: 744
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The prototype a getPrototypeOf trap answers sits unrooted in a Rust local while its own traps run, so for-in over it ends in Terminated(EngineFault).
- file: crates/swell/src/interp/dispatch.rs
  line: 1691
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: ForInNext filters keys with HasProperty of the Proxy, which reads the target's chain, so for-in drops the enumerable keys of the prototype a getPrototypeOf trap answers, against the rules of sec-enumerate-object-properties.
- file: crates/swell/src/object/proxy.rs
  line: 791
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Counting a target key without a descriptor as non-configurable passes the unit's 96 tests; adv_a_target_key_without_a_descriptor_is_configurable catches it.
- file: crates/swell/src/object/proxy.rs
  line: 532
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Dropping keep_property(current) in [[GetOwnProperty]] passes the unit's 96 tests in collect mode; adv_the_target_descriptor_is_held_across_is_extensible catches it with EngineFault.
- file: crates/swell/src/object/proxy.rs
  line: 603
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Dropping keep_property(current) in [[DefineOwnProperty]] passes the unit's 96 tests in collect mode; adv_the_target_descriptor_is_held_across_is_extensible catches it.
- file: crates/swell/src/object/proxy.rs
  line: 343
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Swapping get and set in FromPropertyDescriptor passes the unit's 96 tests; adv_define_property_checks_i12_before_i13 catches it.
- file: crates/swell/tests/proxy.rs
  line: 838
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The seven new chain tests run 2.83 to 3.27 CPU s in the opt-level 0 collect build and two run 1.83 and 1.85 s in the default mode, over the story's bounds, because the search runs about 8 quadratic attempts where one at the expected depth proves the same stack.
- file: crates/swell/tests/proxy.rs
  line: 905
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: The builtin get, set, delete and bound get chain tests take 14.2 to 16.0 CPU s each in the default mode, 60 of the 73 s the proxy module costs, from the same quadratic bisection.
```

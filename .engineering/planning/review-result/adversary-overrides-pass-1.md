---
format: aep.planning-md/3
id: review-result:adversary-overrides-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit overrides
relations:
- reviews: story:host-object-overrides
revision: 1
---
Adversary pass 1 on the unit overrides, branch impl/overrides at 3d29fd7, 2026-11-27.
Verdict red: 8 failing cases (one blocker, four warnings), three mutants as notes.

unit: overrides (story:host-object-overrides), worktree swell-impl-overrides at 3d29fd7 plus uncommitted cases in crates/swell/tests/host_overrides.rs
verdict: red
cases: executed 13→21, red 8 (6 in the default build, 8 with collect-every-safe-point)
origin: introduced 8, pre-existing 0, undecided 0
wrote-outside-worktree: 11 files in <scratch>/wave2/overrides/adversary-1/
needs-coordinator: yes. F4 is a design question: the design says to use the Proxy checks, but the facts box is a record of past answers, not a live target. The stack test placement in `taskfiles/stack-profiles.yml` is also open.

## 1. Diff

```
 crates/swell/tests/host_overrides.rs | 399 +++++++++++++++++++++++++++++++++++
 1 file changed, 399 insertions(+)
```
Only the test file changed. Nothing is committed.

## 2. Cases added

All are in `<trees>/swell-impl-overrides/crates/swell/tests/host_overrides.rs`. All are red now.

**Red in both builds.** This is the first run of these cases alone, `red-default.log`. The collect run gave the same lines.

| test (line) | asserts | red output |
|---|---|---|
| `adversary_own_keys_must_list_a_non_configurable_property_of_the_record` (:1318) | `own_property_keys` leaves out a non-configurable property of the record that was already observed → EngineFault | `left: Normal("\"false/0\"")`, `right: Terminated(EngineFault)` |
| `adversary_own_keys_of_a_non_extensible_record_list_only_its_own_properties` (:1349) | the record is non-extensible and `own_property_keys` lists `ghost`, which [[GetOwnProperty]] says is absent → fault | `left: Normal("\"true/ghost\"")` |
| `adversary_a_property_absent_from_a_non_extensible_host_object_stays_absent` (:1392) | `is_extensible` answers false, then `x` is reported absent, then present → fault | `left: Normal("\"false/undefined/object\"")` |
| `adversary_define_true_for_a_key_observed_absent_on_a_non_extensible_object` (:1440) | `define_own_property` answers true for a key already observed absent on a non-extensible object → fault | `left: Normal("\"false/undefined/true\"")` |
| `adversary_a_non_extensible_host_object_may_delete_a_configurable_property` (:1500) | a host object that behaves exactly like an ordinary object `p` must not fault | `left: Terminated(EngineFault)`, `right: Normal("\"false,true,0,false/false,x,true,0,false\"")` |
| `adversary_set_prototype_of_stops_its_loop_at_an_overridden_get_prototype_of` (:1536) | `Reflect.setPrototypeOf(a, h)` answers true | `left: Normal("\"true,false,false\"")`, `right: Normal("\"true,true,true\"")` |

**Red only with `collect-every-safe-point`** (`red-collect.log`, `split-collect.log`):
- `adversary_a_descriptor_held_across_an_is_extensible_override_survives_a_collection` (:1613): `left: Terminated(EngineFault)`, `right: Normal("\"true,k3,object,7\"")`.
- `adversary_a_descriptor_value_held_across_an_is_extensible_override_survives_a_collection` (:1673): `left: (Terminated(EngineFault), Faulted)`, `right: (Normal("\"true,k3,7\""), Alive)`.

**Removed after running green.** Their text is in `adversary-1/green-probes.rs`:
- the key-only variant of the descriptor case;
- the ordinary walk across a `get_own_property` override;
- `proxy_nesting` after a run ended inside an override;
- the stack chain test.

## 3. Suite run, after the cases existed

`cargo nextest run --locked -p swell --test main -E 'test(/^host_overrides::/)'` gave `Summary [0.133s] 21 tests run: 15 passed, 6 failed, 1550 skipped`, exit 100.

The same with `--features collect-every-safe-point` gave `Summary [0.388s] 21 tests run: 13 passed, 8 failed, 1529 skipped`, exit 100.

Other commands:
- `slot --timeout 30m task test-stack-profiles`, with `STACK_MODULES` plus `host_overrides`, and `STACK_TESTS` = the 4 default tests plus `overrides_recursing_through_each_other_end_at_the_reentry_limit` plus my chain test: `test-stack-profiles release: 1 test binaries: 6 passed, 0 failed, 28 skipped` and `test-stack-profiles stack-debug: 1 test binaries: 6 passed, 0 failed, 28 skipped`, exit 0.
- `SWELL_SUPPORT_STACK_CHILD=1 cargo nextest run ... --no-capture` (dev profile, 7/8 of `min_stack_bytes()`, 12,838,560 bytes) printed `stack per cycle at reentry_depth 1000: override -> script -> host function -> script -> override 14896 bytes over 334 cycles; override -> script -> override 7568 bytes over 500 cycles`. That is about 4,965 and 3,784 bytes per re-entry, against `STACK_BYTES_PER_REENTRY` 14,336.
- `slot task host-surface-check` exited 201 (the task's exit 1). It named 15 items: `HostObjectSpec`, `HostObjectSpec::overrides`, `Overrides`, the 11 fields `Overrides::get_prototype_of` … `Overrides::own_property_keys`, and `new_host_object`. Its last line: `host-surface: 15 of 17 changed name(s) since 0.12.0 not named`.
- Times: one pool job, `slot --timeout 30m sh -c` running `measure.sh`, each test run as `--exact` with `times`, node with 32 cores at load 6.35 then 4.75. Every one of the 13 unit tests and my 8 cases is at most 0.042 CPU s in the default build, 0.034 s in the collect profile and 0.040 s with `SWELL_FULL_COUNTS=1`. The slowest are `host_222_*` (0.042 default), the two descriptor cases (0.034 collect) and `the_facts_*_charged_*` (0.030 full).

## 4. Findings (tree 3d29fd7 plus the uncommitted cases)

- **F1, blocker, introduced.** `crates/swell/src/object/ordinary.rs:239-240`. `ordinary_define_own_property` holds the descriptor's handles without a root across `get_own_property` and `is_extensible`. Since 9bc8897 those calls can run host overrides. A script run by the override drops the descriptor object's `value`. The collection frees it, the stale handle is written into the record, and the next read faults the agent.
  - Reaches it: `Object.defineProperty`, `Reflect.defineProperty` (`reflect.rs:234` keeps the key, not the descriptor) and every other caller of OrdinaryDefineOwnProperty on a host object whose table has `is_extensible` or `get_own_property` but no `define_own_property`.
  - The default build reaches it too when a collection is due inside the override.
  - Fix: the host arms keep the descriptor's values (as `override_define_own_property` does with `keep_given`) before they fall back to the ordinary method.
  - The `current` property (`ordinary.rs:239`) is also held raw across the `is_extensible` call. My script does not reach that site.
- **F2, warning, introduced.** `crates/swell/src/object/host_overrides.rs:1158`. The `own_property_keys` checks read only `facts.fixed` and the first list recorded.
  - The module's own documentation (lines 15-18) says the record's own properties are facts while the table has no `get_own_property`. The check ignores them.
  - Both invariants of [[OwnPropertyKeys]] can therefore be broken without a fault (two red cases).
  - Reaches it: any table with `own_property_keys` and without `get_own_property`.
- **F3, warning, introduced.** `crates/swell/src/object/host_overrides.rs:791`. An absent answer of `get_own_property` on an object known to be non-extensible is not recorded (`Known::Unknown => return Ok(None)`).
  - Two invariants are not checked: "non-existent stays non-existent" for [[GetOwnProperty]], and [[DefineOwnProperty]] returning false for a non-existent key (two red cases).
  - Design section 15 promises a check after each override, and does not state this gap.
- **F4, warning, introduced.** `crates/swell/src/object/host_overrides.rs:1059`. The same arm is at :915 for `has_property` false and at :790 for `get_own_property` absent.
  - A configurable key listed once by a non-extensible object can never be deleted: the delete faults the agent.
  - The clause permits it (only "previously observed as a non-configurable" binds [[Delete]]), and an ordinary object does it.
  - These are the Proxy's checks applied to a record of past answers: the Proxy compares with its target's current state, which the facts box is not.
- **F5, warning, introduced.** `crates/swell/src/object/ordinary.rs:206`. The cycle loop of OrdinarySetPrototypeOf reads `engine_object(next)?.prototype` and does not stop at an object whose [[GetPrototypeOf]] is not ordinary (step 7.c.i).
  - A Proxy record has no prototype (`proxy.rs:172-173`), so the base was right by accident.
  - Since 9bc8897 a host object with a `get_prototype_of` override has a record prototype. A host object whose record prototype leads back to the target turns a legal `setPrototypeOf` into false or a TypeError.

Notes: three small changes the 13 tests would not catch. I found these by reading. I did not build mutants.
- **N1, `crates/swell/src/host/agent.rs:986`.** No unit test reaches `begin()` with `proxy_nesting > 0` and no activation.
  - `spin` is a host function, which pushes an activation (`builtin_function.rs:770`), and `get_one` makes no `Cx` call. So dropping `&& self.interp.proxy_nesting == 0` stays green.
  - A test that catches it: `with_cx(|cx| cx.get_property(o, "x"))` with a `get` override that calls `cx.get_property(one, "x")` 2×N times, with `steps_per_entry` N, must end in `Terminated(StepBudget)`.
  - The change repairs `host-interface.md:183-187` ("An entry made by host code that runs inside another entry ... is nested").
  - Proxy behaviour is unchanged: `call_trap` lowers the count around the trap (`proxy.rs:287-289`).
- **N2, `crates/swell/src/object/host_overrides.rs:263`.** Changing `Walk::Has` to false stays green.
  - Test: `"x" in Object.create(h)` with `LOGGING` must log exactly `["has_property x"]`. With the change it logs `get_own_property x`.
- **N3, `crates/swell/src/object/host_overrides.rs:1152`.** Accepting a duplicate key stays green.
  - Test: `own_property_keys` answering `["a","a"]`, then `Reflect.ownKeys(o)` must end in `Terminated(EngineFault)`.

## 5. Attacked and not broken

- **Invariants checked with a fault, by reading.** [[GetPrototypeOf]] and [[SetPrototypeOf]] after non-extensible; [[IsExtensible]] staying false; [[PreventExtensions]] true. Note that a table with `prevent_extensions` and no `is_extensible` always faults on true. Also the value and attributes of fixed properties, [[Has]], [[Delete]], [[Get]] and [[Set]] against fixed facts and against the record.
- **`proxy_nesting` after termination.** It is 0 after a step budget and after a throw ended a run inside an override. Then 1,000 top-level `get_property` calls on a 500-step budget all answered, and `for(;;){}` still ended in StepBudget. Green in both builds.
- **Re-entry chains.** override → script → host function → script → override, and override → script → override, both end in a catchable RangeError on 7/8 of `min_stack_bytes()` (dev, release, stack-debug), and the agent stays usable.
- **Values held by the walk.** `ordinary_set`, `has` and `get` walks across a `get_own_property` override that collects: key, value and receiver survive (green in collect). The key of `Reflect.defineProperty` survives (`reflect.rs:221` keeps it).
- **Other direct readers.** I searched for direct reads of the property table, prototype and extensible bit. All go through dispatch except F5, the Array fast path (handled at `array.rs:336`) and `error_summary`, which reads the record by design.
- **HOST-222 and the facts trace, by reading.** Every handle of an `Ok` answer passes `checked_answer`/`check_value` before the invariants. Throws are checked in `call_override` for all 11 overrides. `Facts::trace` covers keys, data values, getter and setter, the prototype and listed keys.
- **Not run:**
  - HOST-222 cases for the other forms (another realm, a stale Symbol key, a throw from a nested run);
  - the 64-byte memory-budget sweep and the 100,000-fact cost;
  - for-in with keys that change during the walk;
  - prototype cycles on a bounded stack;
  - the global-object (HOST-41) order of GlobalDeclarationInstantiation;
  - the per-builtin logging scripts of item 5.
- **Surface.** The 11 field types match design lines 1775-1787 letter for letter. `HostObjectSpec<H>` breaks 0.12.0 struct literals, and neither the design nor the story promised a form that keeps them compiling. An all-`None` table answers like no table (the unit's PROBE) but costs 40 bytes more, and it makes `methods_may_run_code` true, which turns off the Array fast path and makes the for-in walk hold.

## 6. Paths written outside the worktree

All in `<scratch>/wave2/overrides/adversary-1/`:
- `red-default.log`
- `red-collect.log`
- `split-collect.log`
- `stack-default.log`
- `stack-profiles.log`
- `suite-default.log`
- `suite-collect.log`
- `surface.log`
- `times.log`
- `measure.sh`
- `green-probes.rs`

The pool jobs also left build output under the worktree's ignored `target/` and `.out/` (`.out/adv-build-*.txt` on the node).

```findings
- file: crates/swell/src/object/ordinary.rs
  line: 240
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: OrdinaryDefineOwnProperty holds the descriptor's handles unrooted across an is_extensible or get_own_property override that runs script, so a collection frees the value, the stale handle is written into the record and the agent faults.
- file: crates/swell/src/object/host_overrides.rs
  line: 1158
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the own_property_keys check reads only fixed facts and the first recorded list and ignores the record's own properties that the module documentation names as facts, so lists without an observed non-configurable key or with a non-existent key pass.
- file: crates/swell/src/object/host_overrides.rs
  line: 791
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: an absent get_own_property answer on a non-extensible host object is not recorded, so a later present answer or a true define_own_property for that key breaks the invariants without a fault.
- file: crates/swell/src/object/host_overrides.rs
  line: 1059
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a configurable key once listed by a non-extensible host object can never be deleted, reported absent or answered false by has_property without faulting the agent, although the clause allows it and an ordinary object does it.
- file: crates/swell/src/object/ordinary.rs
  line: 206
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the cycle loop of OrdinarySetPrototypeOf reads the record prototype past a host object whose get_prototype_of is overridden instead of stopping there (step 7.c.i), so a legal setPrototypeOf answers false.
- file: crates/swell/src/host/agent.rs
  line: 986
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: none of the 13 tests reaches begin() with an override running and no activation, so dropping the proxy_nesting condition stays green; a with_cx get_property whose override loops cx.get_property under a small step budget would catch it.
- file: crates/swell/src/object/host_overrides.rs
  line: 263
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: no test hands an in-walk over to a has_property override through a prototype chain, so replacing Walk::Has's answer with false stays green.
- file: crates/swell/src/object/host_overrides.rs
  line: 1152
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: no test answers own_property_keys with a duplicate key, so accepting duplicates stays green.
```

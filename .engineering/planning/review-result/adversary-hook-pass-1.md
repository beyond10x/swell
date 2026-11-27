---
format: aep.planning-md/3
id: review-result:adversary-hook-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit hook
relations:
- reviews: story:host-ensure-can-add-private-element
revision: 1
---
Adversary pass 1 on the unit hook of wave big-2, impl/hook at c04824c, 2026-11-27.
Red: one warning (a throw of the hook between the halves of an accessor leaves the getter alone) and five gaps of the unit's tests, each closed by a case of the adversary. The adversary's own report follows.

unit: hook (`story:host-ensure-can-add-private-element`, HOST-119 and HOST-223); findings cover worktree `swell-impl-hook` at `c04824c` plus my uncommitted cases
verdict: red
cases: executed 462→471 (default) and 454→463 (collect), red 1
origin: introduced 5, pre-existing 0, undecided 1
wrote-outside-worktree: <scratch>/wave2/hook/adversary-1/ (list in part 6)
needs-coordinator: yes. Finding 1 needs a route decision: the fix is in `crates/compiler`, which this story may not write, or in the text that tells hosts what the hook must do.

The two `executed <before>` numbers come from the unit's own final logs: `final-1/2.log` 177 + `final-1/4.log` 285 = 462, and `final-1/3.log` 173 + `final-1/5.log` 281 = 454. I did not run the suite before writing my cases. No job of mine is running now; the notification just before this message was the last waiter ending.

## 1. `git --no-pager diff --stat`

```
 crates/swell/src/object/private_names.rs   | 184 ++++++++++++++
 crates/swell/tests/private_element_hook.rs | 378 +++++++++++++++++++++++++++++
 2 files changed, 562 insertions(+)
```

- **A non-test file is in this diff:** `crates/swell/src/object/private_names.rs`. Every added line is inside its `#[cfg(test)] mod tests`: one host type (`BusyHook`) and one test. Your brief allowed this ("a library case in the `tests` module of the file it tests"). No implementation line changed.
- **Change to the unit's test file:** `private_element_hook.rs` got one new `Answer::Custom(fn)` variant and its match arm. No existing case was changed.
- **Plants and mutants:** your brief ordered them built in the worktree. This contradicts the generic adversary procedure ("never mutate a file under attack"). I followed the brief. Each plant file was copied in, built and run, then copied back from a saved original. `plants.status` records `cmp` passing and `git status --short` after every plant.
- One slip: my first attempt at mutant Me never started cargo (zsh read `$H:c` as a path modifier). The file was swapped in and restored anyway. I reran Me with a quoted argument.

## 2. Cases added

All are in `crates/swell/tests/private_element_hook.rs` unless noted.

| Case | What it asserts | Now |
|---|---|---|
| `adversary_a_throw_between_the_halves_of_an_accessor_leaves_the_whole_accessor` (:716) | `get #a; #m(){}; set #a` on host object `h`, hook throws at call 2. Expected: 2 hook calls, `#a` keeps both halves, so `S.write(h)` returns `written` | **red**, both modes |
| `adversary_a_termination_the_hook_swallowed_ends_the_run_and_adds_nothing` (:777) | The hook calls `charge_host_bytes(u64::MAX)`, drops the `Err`, then answers `Ok` or `Throw`. Expected: `MemoryBudget`, no `catch` runs, `#x in h` is false | green |
| `adversary_what_a_hook_makes_is_not_kept_across_many_adds` (:874) | A 256 KiB String made per hook call, every second one thrown. Budget is twice the agent's reading plus 2 MiB; the run completes. Then 3 runs of 500 constructions (10 in the collect build): heap reading equal from run 2 on | green |
| `adversary_an_accessor_pair_in_either_order_is_one_element_with_both_halves` (:909) | Setter-first and getter-first pairs: 1 hook call each, and both halves work | green |
| `adversary_a_second_construction_calls_the_hook_once_and_throws` (:950) | Second return-override construction: 1 hook call, a `TypeError`, no initializer runs. Also: a method and a getter of one name is a parse error | green |
| `adversary_other_kinds_call_no_hook` (:988) | A Proxy over a host object with a table, a revoked Proxy, an Array, a bound function: 0 hook calls, and the Proxy's target gets nothing | green |
| `adversary_a_host_global_with_a_table_calls_the_hook_once_per_element` (:1024) | A host global object with a table: 3 hook calls for 3 elements, each with the global object | green |
| `adversary_a_string_handle_of_another_agent_or_stale_in_the_throw_faults_the_agent` (:1069) | A foreign String and a stale String as the `Throw` value both end in `EngineFault` (HOST-162 covers Strings; the unit's tests did not) | green |
| `private_names.rs` `tests::adversary_values_held_only_by_the_adds_survive_a_hook_that_calls_hookcx` (:862) | A hook calls the `HookCx` String, charge, symbol, error-summary, brand, root, get and duplicate methods, and makes 10,000 host objects. The values of 3 adds, held by nothing else, survive | green |

Red output of the red case, captured from its first run, before any suite run (`cases-default.log`):

```
$ cargo nextest run --locked -p swell --lib --test main -E 'test(/^private_element_hook::adversary_/) | test(/^object::private_names::tests::adversary_/)'
        FAIL [   0.011s] (4/8) swell::main private_element_hook::adversary_a_throw_between_the_halves_of_an_accessor_leaves_the_whole_accessor
    thread 'private_element_hook::adversary_a_throw_between_the_halves_of_an_accessor_leaves_the_whole_accessor' (1827034) panicked at crates/swell/tests/private_element_hook.rs:752:5:
    assertion `left == right` failed
      left: "throw TypeError: the private accessor #a has no setter"
     right: "normal written"
exit 100
```

In that first run the growth case took 4.57 s. I made it smaller (it is my own case) and reran my cases alone in both modes (`cases-3.log`): 8 passed, 1 failed in each mode, the same red.

## 3. Suite run, after the cases existed (`suite.log`)

```
$ cargo nextest run --locked --no-fail-fast -p swell --lib --test main -E 'binary_id(swell) | test(/^private_element_hook::/) | test(/^private_fields::/) | test(/^private_methods::/) | test(/^hooks::/) | test(/^host_objects::/) | test(/^host_overrides::/) | test(/^fuzz::/) | test(/^misuse::/) | test(/^interrupt::/)'
     Summary [   2.541s] 471 tests run: 470 passed, 1 failed, 1541 skipped
        FAIL [   0.017s] (382/471) swell::main private_element_hook::adversary_a_throw_between_the_halves_of_an_accessor_leaves_the_whole_accessor
$ (same with --features collect-every-safe-point)
     Summary [  17.440s] 463 tests run: 462 passed, 1 failed, 1517 skipped
        FAIL [   0.089s] (380/463) swell::main private_element_hook::adversary_a_throw_between_the_halves_of_an_accessor_leaves_the_whole_accessor
exit 100
```

### Plants and mutants

Each was run with `-E 'test(/^private_element_hook::/) | test(/^object::private_names::tests::/) | test(/^private_methods::/) | test(/^private_fields::/)'`. Logs are `plant-<name>-<d|c>.log`. The halves case is red in every row because it is red on the unmodified tree; it is left out of the lists below.

| Plant / mutant | Tests that went red (d = default build, c = collect build) |
|---|---|
| A: room charged before the hook | d and c: `host_119_a_throw…`, `a_second_add_after_an_ok_answer…`, lib `a_level_of_methods_and_fields_reserves_its_room_once`, `adversary_a_second_construction…`, `adversary_what_a_hook_makes…` |
| B: collection right after the hook | c: lib `a_value_held_only_by_the_add_survives_the_hook`, `host_119_a_throw…`, my lib case, my growth case. d: only my lib case and my growth case (`the script did not complete normally`) |
| P1: hook called for every object | d and c: `an_ordinary_object…`, `a_proxy…`, `a_host_object_without…`, `adversary_other_kinds…` |
| P2: hook called after the append | d and c: `host_119_a_throw…`, `host_223_a_throw…`, `a_second_add…`, `an_interrupt…`, plus my swallow, second-construction and growth cases |
| P3: hook not called for methods and accessors | d and c: `host_223_a_throw…`, `host_223_a_stale…`, `host_223_a_handle…`, plus my pair, global, second-construction and String cases |
| P4: `Throw` value not checked | d: `host_223_a_stale…`, `host_223_a_handle…`, my String case. **c: nothing** (finding 6) |
| Ma: drop `self.running()?` (:414) | d: only my swallow case: `left: "normal true"`, `right: "normal false"` at :795 |
| Mb: drop `self.scope.truncate(start)` (:413) | d: only my growth case. c: nothing (my growth case skips its large part in the collect build) |
| Mc: no join when the setter is added first | d: only my pair case: `left: "throw TypeError: the object already has the private element #a"`. `private_methods::` and `private_fields::` stay green |
| Md: a join that drops the setter | d: my pair case, my lib case, and 7 `private_methods::` tests (the older suite catches this one) |
| Me: `scoped_new_string` becomes a safe point (`host/mod.rs:223`) | c: only my lib case: `ObjectRef { … index: 400 … } was freed during an add` |

## 4. Findings

1. **A throw between the two halves of an accessor leaves half an accessor.** NEEDS-CHANGE, introduced by `bd1dc46`. Code: `crates/swell/src/object/private_names.rs:354`.
   - Measured: `private_element_hook.rs:752`, exit 100. After the throw, `o.#a = 2` throws "no setter".
   - The pinned draft (`#sec-runtime-semantics-classdefinitionevaluation`, "Replace existingElement in container with combined") adds the whole accessor before `#m`. The engine adds each half where it stands in the source (`crates/compiler/src/class.rs:145`), so a refused `#m` leaves only the getter.
   - What reaches it: a host whose hook allows one add and refuses a later add to the same object in one construction. The draft forbids that ("Any two calls … must return the same kind of Completion Record"), but neither `hooks.rs:190-200` nor HOST-119/HOST-223 tells a host so. The unit's own `host_223_a_throw…` uses exactly such a host (`at` = 2 to 4).
   - The docs at `private_names.rs:276-279` and `:336-338` say the two halves are "one element", which this case contradicts.
   - Two ways to fix: emit an accessor pair next to each other at the first half's place (in `crates/compiler`), or state the draft's same-kind rule in the hook's documentation.
2. **The unit's tests do not see `running()` being dropped (decision 10).** CONFIRMED, introduced by `bd1dc46`, `private_names.rs:414`. With mutant Ma, an element is added after the run was terminated inside the hook. All 10 unit tests and the library test stay green; `adversary_a_termination_…` (:795) catches it.
3. **The unit's tests do not see the scope truncation being dropped.** CONFIRMED, introduced by `bd1dc46`, `:413`. With mutant Mb, what the hook makes is kept until the entry ends. The unit's tests stay green; `adversary_what_a_hook_makes_…` catches it in the default build.
4. **Nothing in the suite tests a setter-first accessor pair.** CONFIRMED, origin undecided, `:128` (`joined`, first alternative). Mutant Mc stays green in the unit's file, in `private_methods::` and in `private_fields::`. `adversary_an_accessor_pair_in_either_order_…` catches it. The same match is inline at base `bcc10a4` (read with `git show`); I did not run it there.
5. **The unit's library test cannot see a `HookCx` method that becomes a safe point.** CONFIRMED, introduced by `bd1dc46`, `:651`. Its host is `NoHooks`, whose hook calls no `HookCx` method, so mutant Me stays green. `adversary_values_held_only_by_the_adds_…` (:862) catches it in the collect build. Nothing checks this at compile time. For script paths the values are on the interpreter stack and environment anyway, so only direct callers of the add depend on it.
6. **In the collect build the HOST-223 tests cannot fail on their own defect.** CONFIRMED, introduced by `bd1dc46`, tests at `private_element_hook.rs:568` and `:575`. With plant P4 (no handle check) both are green in the collect build: the collector faults on the stale handle anyway. Only the default build catches it.

## 5. Attacked and not broken

- **Order of the clause steps.** The find runs before the hook, and its step charge (only past 8 elements) can end the run with no hook call. Neither a requirement nor a script can tell the two orders apart.
  - A duplicate add, accessor-first or method-first: 1 call, then the `TypeError`.
  - A pair whose first half is refused: 1 call, and the second half never runs.
  - A static pair: added to a function, so no call.
  - A method and a getter of one name: parse error.
- **Which objects get the call.** A Proxy over a host object with a table, a revoked Proxy, an Array and a bound function: 0 calls, and the Proxy's target is untouched. A host global object with a table: one call per element, with that object.
- **What a hook can do.** I read every `scoped_methods!` method and every `safe_point()` caller: all are entries, calls or the run loop, and `HookCx` reaches none of them. 10,000 host objects and many Strings, roots and drops in a hook, under the collect build: the values of the adds survive.
- **The throw.** Stale and foreign Strings fault the agent. Heap readings are stable across repeated runs where the hook throws at every second call.
- **Budgets.** A hook that exhausts the memory budget ends the run with nothing added.
- **Realm sizes:** `realm-sizes: 0 run(s) red` (k = 4 … 200, both modes). `awk -v rule=stack` prints every `private_element_hook` test as "other": none runs on a bounded stack.
- **Times** (CPU s alone, from `measure.log`): unit tests at most 0.017 (default) and 0.182 (collect). My cases at most 0.346 (default) and 0.493 (collect), both for the growth case. None is over 5 s.
- **Not attacked:** the step-budget sweep and the 8-byte memory-budget sweep of item 6; a `Throw` handle from this agent's other realm (reasoned only: same heap, so it is thrown normally); the depth bound (a hook cannot call code).

## 6. Paths written outside the worktree

All in `<scratch>/wave2/hook/adversary-1/`:

- **Scripts:** `run.sh`, `plants.sh`, `plant-summary.sh`, `measure-adv.sh`
- **Saved originals and plant copies:** `private_names.rs.saved`, `host-mod.rs.saved`, `plants/{A,B,P1,P2,P3,P4,Ma,Mb,Mc,Md,Me}.rs`, `files.txt`
- **Logs:** `cases-default.log`, `cases-default-2.log`, `cases-3.log`, `growth-1.log` (each with a `.done` file), `suite.log` (and `.done`), `plant-*-{d,c}.log`, `plants.status`, `plants.done`, `measure.log`, `realm-sizes.log`

On the build node, the measurement job wrote `.out/measure-adv-one.txt` and `test-realm-sizes` wrote its `.out/realm-sizes-*` files; none were fetched.

```findings
- file: crates/swell/src/object/private_names.rs
  line: 354
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a hook throw between the two halves of a split getter/setter pair leaves a getter-only accessor where the draft adds the combined accessor first, and the hook docs do not pass on the draft's same-kind rule that would make this unreachable
- file: crates/swell/src/object/private_names.rs
  line: 414
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: dropping the running check after the hook adds the element after a termination inside the hook and no unit test turns red
- file: crates/swell/src/object/private_names.rs
  line: 413
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: dropping the scope truncation after the hook keeps every value the hook made until the entry ends and no unit test turns red
- file: crates/swell/src/object/private_names.rs
  line: 128
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: no test in the unit or the private_methods and private_fields suites adds a setter before its getter, so losing that join stays green
- file: crates/swell/src/object/private_names.rs
  line: 651
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the library test meant to guard the no-safe-point claim uses NoHooks whose hook calls no HookCx method, so a HookCx method that collects stays green
- file: crates/swell/tests/private_element_hook.rs
  line: 568
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: in the collect build the HOST-223 tests stay green with the handle check on the Throw value removed because the collector faults first
```

---
format: aep.planning-md/3
id: review-result:overrides-correction-2-coordinator-read
kind: review-result
status: active
title: Coordinator's read of correction 2 of the unit overrides
relations:
- reviews: story:host-object-overrides
revision: 1
---
Coordinator's read of correction 2 of the unit overrides (impl/overrides, 08ad2f4 to 2a91ce5),
2026-11-27. Verdict: no defect; one branch decided and sent back, held by the second commit.

The unit's two adversary passes are spent (`review-result:adversary-overrides-pass-1`,
`review-result:adversary-overrides-pass-2`, both red), so the correction after the second
pass is read here. Read: the whole diff of `f28eb7c` and of `4dccde9` in `crates/swell/src`
(`object/host_overrides.rs`, `object/ordinary.rs`), the unit's report with the red line of
each case before the fix, and the lines of the runs below.

Correction 2 (`fa682fa`, the merge of `wave/big-2`; `f28eb7c`; `4dccde9`; `2a91ce5`):

- F-A, F-B, F-C and F-E of pass 2 (the object's record answered where the table leaves an
  override out, and its answer was held to nothing: PreventExtensions true was not a fact,
  SetPrototypeOf true after a report "not extensible" ran, a key list of the record left out
  a key observed non-configurable, a delete of the record on an object that is not extensible
  did not make the key non-existent). Each fallback branch of `host_set_prototype_of`,
  `host_prevent_extensions`, `host_delete` and `host_own_property_keys` now runs the ordinary
  method and then the check the override's answer runs: `set_prototype_of_true`,
  `prevent_extensions_true`, `delete_true`, `key_list_check` with `with_seen`. Read for
  soundness:
  - the extensibility and the prototype the object had reported are read before the record
    answers, since OrdinarySetPrototypeOf changes the record;
  - the record's key list is checked only when the table has `get_own_property`; with
    neither override the record answers both, and no fact can disagree with its own list;
  - the step per key of an override's list is charged in `key_list_check`, after the memory
    charge of the list, once per key as before;
  - the seven other operations need no change: `known_prototype`, `known_extensible` and
    `known_property` read the record live, and the walks of HasProperty, Get and Set answer
    from [[GetOwnProperty]] and [[GetPrototypeOf]], both checked.
  The four cases of the adversary are red on the engine before the fix (40 of 45 passed) and
  green after, in both builds; the unit's report gives each red line.
- F-D of pass 2 (is a `has_property` false a fact that the key does not exist?). Decided by
  the coordinator: no. ECMA-262 6.1.7.3 binds [[HasProperty]] only for a key observed as a
  non-configurable own property; a false for any other key is allowed and records nothing.
  `override_has_property` faults for a non-configurable property, one `get_own_property`
  reported or one the record holds, and answers false otherwise. The read of `f28eb7c` found
  one branch that still ran the Proxy `has` check (a table without `get_own_property`, a
  configurable property of a record that is not extensible faulted): sent back, and `4dccde9`
  gives both paths one rule. Tests:
  `has_property_false_is_not_an_observation_of_a_non_existent_key` (red before:
  `Terminated(EngineFault)`),
  `has_property_false_for_a_key_observed_non_configurable_faults_the_agent` (red with the
  fault for a fixed key planted out), and the test of `4dccde9` for both sides of the record
  branch.
- F-F of pass 2 (can anything reach the keep at `ordinary.rs:506`?). The unit found no way:
  every caller of `internal_set` whose receiver may run code keeps the key and the value
  itself (`proxy_set_steps`, `Reflect.set`, the super references, the host's scope), and the
  others pass a receiver that runs none. The keep is removed and a comment names the callers;
  `swell::main` stays green in both builds. So correction 1 fixed no older defect there, and
  the changelog gets no entry under Fixed.
- F-G, F-H and F-I (three plants of pass 2 that no test turned red): three tests, each red
  with the adversary's plant (the report gives the lines), green on the head. The eight plants
  of pass 1 are red on the changed code.
- `2a91ce5`: `taskfiles/stack-profiles.yml` lists the module `host_overrides` and its three
  tests that run on a bounded stack. The first brief had said the file belongs to another
  unit of the wave; that unit does not write it.

The coordinator's runs on `2a91ce5`, build node, before the merge: `swell::main` 1657 passed
(default) and 1630 passed (collect-every-safe-point), 1 skipped each; `swell` lib 283 passed;
`task test-stack-profiles` 7 passed in the profile release and 7 in stack-debug, with nothing
on the command line; `task clippy` and `cargo fmt --all -- --check` exit 0.

Not covered: `task host-surface-check` is red on the unit's head by design (15 changed names
of the public surface are named by the `CHANGELOG.md` text, which the coordinator writes
after the merge); the test262 part `built-ins/Reflect` and `built-ins/Proxy` was run by the
unit at `f28eb7c` (912 passed, 1 failed, equal to the base) and not again after `4dccde9`,
which changes one function of host objects that no test262 case reaches; the comments
"HasProperty runs no code" in `builtins/array/search.rs` are inaccurate and left.

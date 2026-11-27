---
format: aep.planning-md/3
id: review-result:proxytraps-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of corrections 1 and 2 and of the last step of the unit proxytraps: no defect'
relations:
- reviews: story:proxy-descriptor-prototype-and-extensibility-traps
revision: 1
---
Coordinator's read of corrections 1 and 2 and of the last step of the unit proxytraps
(impl/proxytraps, c8e54d7 to 14a6133f), 2026-11-27. Verdict: no defect.

Correction 1 (9afb6ff, 2b7317b, 4ec0ec9 and the merge of `wave/big-2`, head 61b14ed2) answers
review-result:adversary-proxytraps-pass-1: seven call sites that held a value only in a local
across a trap keep it (all 204 call sites of the seven methods and their wrappers were read
and classified by the implementor), `for`-`in` no longer calls HasProperty, and the
adversary's cases are tests under the unit's names. The adversary's second pass ran on that
correction and is recorded as review-result:adversary-proxytraps-pass-2: red in one case (a
`getPrototypeOf` trap that answers differently on its second call made `for`-`in` drop a key
that was never deleted), with three mutants its own cases hold and the unit's did not. The
coordinator did not read correction 1's code line by line again; the second pass did.

Correction 2 (3a12ad4, 258382b, 3b7c1c8, e02a85d, the merge 1a86e92 and 95827c0) answers
the second pass. Read: the whole diff from 61b14ed2.

- The walk of EnumerateObjectProperties records the chain it goes through from the first
  Proxy on (`Enumeration { found, chain }` in `ops/objects.rs`; `record_chain_before` for the
  ordinary objects before the first Proxy). The list is a charged vector, charged before it
  grows. The loop's enumerator holds it and `Frame::trace` traces it; it is credited with the
  enumerator on every way out, with the frame, and with a collected generator.
- `Agent::recorded_chain_has` (`interp/dispatch.rs`) asks [[GetOwnProperty]] of each recorded
  object in order, with a step per object after the first, and calls no [[GetPrototypeOf]].
  A chain without a Proxy records nothing and takes the earlier path, at the earlier steps.
- `call_trap` (`object/proxy.rs`) releases the trap's arguments when the trap returns. Read
  for each caller that what it uses after the call is kept by the caller itself:
  `proxy_set_steps` keeps key, value and receiver; the descriptor object of
  [[DefineOwnProperty]] is held by `call_trap` and there is no safe point between its making
  and the call. The count of values kept outside a room is read by a test-only module and
  bounded at 10 per method by a library test.
- The adversary's cases of the second pass are committed without their prefix; its three
  mutants, the two it could not hold from outside the crate (one value under-charged in a
  kept room, the room of the kept stack not given back; library tests of `ops::objects`
  hold them now) and three of the implementor's (chain not traced, chain not credited, the
  objects before the first Proxy not kept) each turn a named test red in both modes, by the
  implementor's runs.

The last step (6b2093f, the merge 80d1099 of `wave/big-2` at f8406d5, 14a6133f) follows the
coordinator's read of correction 2, without an adversary pass. Read: the whole diff.

- 6b2093f changes comments only, in five files: a mark of the coordinating session stood in
  10 added lines and is replaced by words that say the rule.
- The merge had no conflict; `sh taskfiles/test-main.sh check` passes on it.
- 14a6133f changes `crates/swell/tests/proxy.rs` only. `each_invariant_both_sides` (1.95 CPU s
  alone in the lane's collect build, of a bound of 2 s) is three tests over one shared text of
  helpers; read: the three expected strings joined are the expected string of the one test,
  entry for entry, and no line of the script is dropped. The collect-only
  `chains_of_every_route_on_the_promised_stack` (1.83 to 2.51 s) is three tests made by one
  macro over `ROUTES[..8]`, `ROUTES[8..15]` and `ROUTES[15..]`, the 22 routes, on the same
  stack and at the same `reentry_depth`. Times alone in the collect build, by the implementor,
  three runs each: 1.03 to 1.04 s, 0.57 s, 0.32 s; 0.68 s, 0.56 s, 0.58 s.

Checks of the implementor on 14a6133f, on the build node: the whole `swell::main`, `1506 tests
run: 1506 passed, 1 skipped` in the default mode and `1485 tests run: 1485 passed, 1 skipped`
with a collection at every safe point; the library tests, 277 and 273 passed; the stack lane,
4 passed in the profile `release` and 4 in `stack-debug`; the lane of realm sizes, 14 runs,
`realm-sizes: 0 run(s) red`, which holds `proxy::revoke_is_idempotent_and_drops_target_and_handler`
at 200 host functions, red on `wave/big-2` before this unit; clippy, the format check and the
check of the generated `tests/main.rs`.

Left as it is, older than the unit: the box of a `for`-`in` enumerator is not charged.

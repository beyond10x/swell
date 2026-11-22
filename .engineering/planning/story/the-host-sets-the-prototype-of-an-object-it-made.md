---
format: aep.planning-md/3
id: story:the-host-sets-the-prototype-of-an-object-it-made
kind: story
status: implemented
title: Cx exposes [[GetPrototypeOf]] and [[SetPrototypeOf]], so the host gives an interface object its parent interface object as [[Prototype]]
summary: Two operations of decision 14 over the engine's existing internal methods; no signature of new_host_function changes.
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:48:23Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T06:48:23Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:52Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Waits for: nothing; starts on the base at once (the base holds the gate unit). It shares
`crates/swell/src/host/mod.rs` (the `impl Cx` of decision 14, its own lines) and
`crates/swell/tests/calls.rs` with later stories. Batch H1. Requirement HOST-180. Consumer: the
browser, requirement 8 of its list (Web IDL "interface object": `Element`'s [[Prototype]] is
`Node`). Decision D8 (taken: option A, with its conditions, Change 2).

## Context

Line numbers are of `wave/budget-1` at `ed56b05`.

- A host function is made by `Cx::new_host_function` (`crates/swell/src/host/mod.rs:523`)
  through `create_host_function`, which passes no prototype to `create_builtin_function`
  (`crates/swell/src/object/builtin_function.rs:219-243`, the `None` at `:240`), so its
  [[Prototype]] is the realm's `%Function.prototype%` (`:188`). Web IDL wants that for an
  interface without a parent ("Let constructorProto be
  realm.[[Intrinsics]].[[%Function.prototype%]]", `.cache/webidl/webidl.txt:12560`) and the
  parent's interface object otherwise.
- The engine has both internal methods: `Agent::get_prototype_of` and `set_prototype_of`
  (SetImmutablePrototype for `%Object.prototype%`, OrdinarySetPrototypeOf otherwise;
  `crates/swell/src/object/mod.rs:261-280`). OrdinarySetPrototypeOf refuses a non-extensible
  object with another prototype and a cycle, and charges a step per object on the new
  prototype's chain (`crates/swell/src/object/ordinary.rs:126-152`, the step at `:146`); it runs
  no script code. `Cx` exposes `get_property`, `set`, `define_property_or_throw`, `has_property`,
  `delete_property_or_throw` and the conversions (`crates/swell/src/host/mod.rs:588-680`), not
  these two.
- A host can reach `Object.setPrototypeOf` today by reading the global's `Object` in `with_cx`
  before the first script; that depends on the global's properties.
- A Web IDL binding step that reads a prototype: the named property visibility algorithm, "Let
  prototype be O.[[GetPrototypeOf]]()" and "Set prototype to prototype.[[GetPrototypeOf]]()"
  (`.cache/webidl/webidl.txt:15607`, `:15615`), which the browser's named properties run (an
  `HTMLCollection` or a `Window` with named properties, once HOST-112 to HOST-114 give it the
  overrides of slice 9). This is why `get_prototype_of` is on `Cx` beside the setter.

## Change

Failing tests first (Acceptance). Then, in the `impl Cx` of decision 14 (`host/mod.rs:519`), in
the form of its neighbours (handles checked first, then `begin`, the thrown value handed out):

```rust
/// O.[[GetPrototypeOf]]().
pub fn get_prototype_of(&mut self, object: ObjectRef) -> Result<Completion<Option<ObjectRef>>, HostError>;
/// O.[[SetPrototypeOf]](V): whether it was set (false for a cycle, a non-extensible object
/// with another prototype, or %Object.prototype%).
pub fn set_prototype_of(&mut self, object: ObjectRef, prototype: Option<ObjectRef>)
    -> Result<Completion<bool>, HostError>;
```

1. **On `Cx` only:** once a host object has an override table (slice 9) these run host code,
   which `HookCx` cannot.
2. **D8's conditions.** They act on any object the host made or was handed, through a handle
   it holds in its scope or a root (a stale handle answers `Err(HostError::StaleHandle)`, a
   foreign one `Err(HostError::ForeignHandle)`); they follow the ordinary [[SetPrototypeOf]]
   rules: a cycle and a non-extensible object with another prototype are refused with
   `Ok(Ok(false))`, an outcome the host reads; and for an ordinary object or a host object
   without overrides they run no script code and call no hook. The steps of the walk are the
   running entry's (each `with_cx` call starts afresh).
3. **Stays as it is.** Every signature; `new_host_function`; the internal methods.

Design preconditions (the design author writes them before the unit starts):
`docs/design/host-interface.md` decision 14, the two methods in the `impl Cx` block, with "An
interface object whose interface has a parent gets the parent's interface object as
[[Prototype]] by `set_prototype_of` after `new_host_function`; `get_prototype_of` serves the
binding steps that walk a chain (Web IDL, named property visibility). Both follow the ordinary
[[SetPrototypeOf]] and [[GetPrototypeOf]] and run no script code for an object without
overrides." And the line of HOST-180 below, with its pending line in
`conformance/host/requirements.txt`.

New requirement:

- **HOST-180** (webidl:interface-object) A host function `Node` and a host function `Element`,
  the second given `Node` as [[Prototype]] by `cx.set_prototype_of` answering `Ok(Ok(true))`,
  make the script `Object.getPrototypeOf(Element) === Node && Object.getPrototypeOf(Node) ===
  Function.prototype` answer `true`, and `cx.get_prototype_of(Element)` answers `Node`;
  `set_prototype_of(Node, Some(Element))` then answers `Ok(Ok(false))` (a cycle), on an object
  made non-extensible another prototype answers `Ok(Ok(false))`, and a stale handle answers
  `Err(HostError::StaleHandle)`; no call runs script code (a getter on `Function.prototype`
  counting its calls stays at 0). [slice 6]

Writes: `crates/swell/src/host/mod.rs` (two methods in `impl Cx`), `crates/swell/tests/calls.rs`,
and in `conformance/host/requirements.txt` the line of HOST-180 only.

## Acceptance

In `crates/swell/tests/calls.rs`, each in both collection modes:

- `host_180_an_interface_object_has_its_parent_as_prototype`: as HOST-180. Today it does not
  compile.
- `set_prototype_of_follows_the_internal_method`: on a non-extensible object (made so by
  `Object.preventExtensions` in a script) another prototype answers `false` and the same
  prototype `true`; on `%Object.prototype%` any other value answers `false`;
  `get_prototype_of` of a host object made with `Prototype::Null` answers `None`. Today it does
  not compile.

Commands, each one job of the build pool (one slot): `slot cargo nextest run --locked -p swell
--profile default --test calls`, the same with `--features collect-every-safe-point`, `slot
cargo clippy --workspace --all-targets --locked -- -D warnings`, and last, once, in the
background, `slot timeout 1800 task host-requirements-check` (it builds every test target of the workspace).
No test262 run: no builtin changes. Never `cargo test -p swell` whole, `slot task test-collect`,
`slot task test` or `slot task check`.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change. A test is killed at 60 s and the gate's test steps fail over 120 s;
each test runs in a process of its own under cargo-nextest, so nothing is shared through a
`static` between tests. Each new test is timed, and one over 5 s is named in the report with
its time. A job of the build pool asks for one slot; one over 10 minutes is killed, so `host-requirements-check` on a cold tree and the full test262 run are named `slot timeout 1800 <command>`. Determinism: the same calls answer the
same. Nothing is added to the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack
tests are unchanged. No third-party crate, no unsafe code.

## Report

The tests with their times, and this line for the changelog's "Host interface" heading:
"`get_prototype_of` and `set_prototype_of` (new, on `Cx`): the [[GetPrototypeOf]] and
[[SetPrototypeOf]] of an object, so an interface object takes its parent's interface object as
[[Prototype]] (`HOST-180`)."

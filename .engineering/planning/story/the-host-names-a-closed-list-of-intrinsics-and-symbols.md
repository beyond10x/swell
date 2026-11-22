---
format: aep.planning-md/3
id: story:the-host-names-a-closed-list-of-intrinsics-and-symbols
kind: story
status: active
title: 'The host reads, from a closed list, the intrinsics and well-known symbols Web IDL names for the DOM: %Object.prototype%, %Function.prototype%, %Error.prototype%, @@toStringTag and @@unscopables'
summary: Two closed enumerations and two reads on Cx and HookCx over the realm's existing Intrinsics and the agent's WellKnown; the iteration entries Web IDL needs follow in story:the-host-names-the-iteration-intrinsics-of-web-idl.
relations:
- depends_on: story:arena-capacity-and-collector-workspace-on-the-ledger
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:48:24Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T06:48:25Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
Waits for: the budget batch's `story:arena-capacity-and-collector-workspace-on-the-ledger` (it
edits the budgets of the `#[cfg(test)]` module of `crates/swell/src/host/host_object.rs`, the
file this story adds its enumerations to). It shares `crates/swell/src/host/mod.rs`
(`scoped_methods!`) with the stories that write that macro, each its own lines. Batch H2.
Requirements HOST-183, HOST-184. Consumer: the browser, requirement 9 of its list, and its "The
DOM from Web IDL" (class strings, `[Unscopable]`, `DOMException`). Decision D9 (taken: option A,
with its conditions: `%Array.prototype.entries%`, `keys` and `forEach` beside `values`, which
the next story adds; the closed list is extended only by a changelog line under the rule of D7).

## Context

Line numbers are of `wave/budget-1` at `ed56b05`.

- The one intrinsic a host can name is `%Object.prototype%`, as `Prototype::ObjectPrototypeOf(realm)`
  of a `HostObjectSpec` (`crates/swell/src/host/host_object.rs:44-53`, its doc comment "the one
  intrinsic a host can name" at `:49-50`, the design's copy at
  `docs/design/host-interface.md:1251`). Decision 7 says the setup closure's `HookCx` "reads no
  property and names no other intrinsic" (`:612-614`) and rejects "an accessor for any intrinsic
  of a realm (a door to every intrinsic for one need ...)" (`:617-621`).
- A realm's intrinsics are `Intrinsics` with the closed enumeration `Intrinsic`
  (`crates/swell/src/realm/intrinsics.rs:32-79`, `get` at `:127`): `ObjectPrototype` (`:33`),
  `FunctionPrototype` (`:34`) and `ErrorPrototype(ErrorKind)` (`:45`) exist. The well-known
  symbols are the agent's `WellKnown` (`crates/swell/src/realm/mod.rs:33-57`), alive while a
  realm keeps them and read by `Agent::well_known` (`:206-213`), which makes nothing;
  `to_string_tag` (`:36`) and `unscopables` (`:40`) exist; `iterator` comes with
  `story:symbol-constructor-and-well-known-symbols` (slice 6).
- What Web IDL takes from the realm for these interfaces (`.cache/webidl/webidl.txt`): an
  interface object's [[Prototype]] without a parent, `%Function.prototype%` (`:12560`); an
  exception's interface prototype object's [[Prototype]], `%Error.prototype%` (`:12687`);
  `%Object.prototype%` (`:12689`); `@@toStringTag` for the class string; `@@unscopables` for
  `[Unscopable]`; and, for an iterable interface, `@@iterator` and the four Array methods
  (`:13660-13688`), the next story's.
- Today a host reaches `Function.prototype` and `Error.prototype` by reading the global object's
  properties through `cx.get_property` in `with_cx`, before the first script; not in the setup
  closure (its `HookCx` reads no property), and not once a script replaced them.

## Change

Failing tests first (Acceptance). Then (decision D9, option A), in `host_object.rs` beside
`Prototype`:

```rust
/// The intrinsics a host can name (Web IDL's need for its interface objects, prototypes and
/// exceptions); a closed list, grown only by a decision of the design with a changelog line.
#[non_exhaustive]
pub enum HostIntrinsic { ObjectPrototype, FunctionPrototype, ErrorPrototype }
/// The well-known symbols a host can name.
#[non_exhaustive]
pub enum WellKnownSymbol { ToStringTag, Unscopables }
```

and in `scoped_methods!`, so on `Cx` and `HookCx`:

```rust
/// The intrinsic `which` of `realm`, handed out.
pub fn intrinsic(&mut self, realm: RealmRef, which: HostIntrinsic) -> Result<ObjectRef, HostError>;
/// The well-known symbol `which` (shared by all realms of the agent), handed out.
pub fn well_known_symbol(&mut self, which: WellKnownSymbol) -> Result<SymbolRef, HostError>;
```

1. Each maps to the existing `Intrinsic` arm or `WellKnown` field; a realm `create_realm` did not
   make answers `Err(StaleHandle)` (`Agent::host_realm`, `crates/swell/src/realm/mod.rs:310`);
   the symbol is read through `Agent::well_known`. Both are reads: no code, no hook, no step, no
   allocation; the value is handed out to the running scope, as every value given to host code
   is.
2. `Prototype::ObjectPrototypeOf` stays (a host may also write
   `Prototype::Object(cx.intrinsic(realm, HostIntrinsic::ObjectPrototype)?)`); its doc comment
   at `host_object.rs:49-50` becomes "The `%Object.prototype%` of that realm, without a call
   (decision 7)."
3. **Stays as it is.** Every intrinsic and symbol; `realm/`.

Design preconditions (the design author writes them before the unit starts):
`docs/design/host-interface.md` decision 7: the sentence at `:612-614` becomes "... it reaches
the one it needs through `Prototype::ObjectPrototypeOf(realm)` of a `HostObjectSpec` (decision
15) or by `intrinsic`: the closure's `HookCx` reads no property and names only the intrinsics
and symbols of the closed list."; "Rejected" (`:617-621`) keeps "an accessor for any intrinsic
of a realm", and the decision gains "The host names the intrinsics and well-known symbols of
`HostIntrinsic` and `WellKnownSymbol`, a closed list of what Web IDL takes from a realm for the
interfaces a host defines; an entry is added by a decision of this document with its Web IDL
clause and a changelog line."; the doc comment at `:1251` as in point 2; decision 15 gains the
two reads. And the lines of HOST-183 and HOST-184 below, with their pending lines in
`conformance/host/requirements.txt`.

New requirements:

- **HOST-183** (webidl:interface-object) In the setup closure and in `with_cx`,
  `intrinsic(realm, FunctionPrototype)`, `intrinsic(realm, ErrorPrototype)` and
  `intrinsic(realm, ObjectPrototype)` answer the objects the script `Function.prototype`,
  `Error.prototype` and `Object.prototype` read in that realm, also after a script set the
  globals `Function`, `Error` and `Object` to `null`; a second realm's answers differ from the
  first's; a stale realm answers `Err(HostError::StaleHandle)`. [slice 6]
- **HOST-184** (webidl:interface-prototype-object) A host object whose own `@@toStringTag` data
  property, defined with `well_known_symbol(ToStringTag)`, is `"Node"` makes the script
  `Object.prototype.toString.call(n)` answer `"[object Node]"`; one with an own `@@unscopables`
  object whose `before` is `true` makes `with (n) { before }` read the outer binding; the symbols
  are the same in two realms of the agent. [slice 6]

Writes: `crates/swell/src/host/host_object.rs` (the two enumerations and the doc comment of
`ObjectPrototypeOf`; not its `#[cfg(test)]` module), `crates/swell/src/host/mod.rs` (the two
reads in `scoped_methods!`), `crates/swell/tests/host_objects.rs`, and in
`conformance/host/requirements.txt` the lines of HOST-183 and HOST-184 only. Not written:
`crates/swell/src/realm/`.

## Acceptance

In `crates/swell/tests/host_objects.rs`, in both collection modes:

- `host_183_the_host_names_three_intrinsics`: as HOST-183. Today it does not compile.
- `host_184_the_host_names_two_symbols`: as HOST-184. Today it does not compile.
- `a_dom_exception_reaches_error_prototype`: a host object `e` whose prototype is a host object
  whose prototype is `intrinsic(realm, ErrorPrototype)` makes `e instanceof Error` answer
  `true`. (Whether `e` has [[ErrorData]] is the host's choice of
  `story:an-error-object-is-read-without-running-code`, `HostObjectSpec::error_kind`; this test
  asserts nothing of it.) Today it does not compile.

Commands, each one job of the build pool (one slot): `slot cargo nextest run --locked -p swell
--profile default --test host_objects`, the same with `--features collect-every-safe-point`,
`slot cargo clippy --workspace --all-targets --locked -- -D warnings`, and last, once, in the
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
"`intrinsic` and `well_known_symbol` (new, on `Cx` and `HookCx`), `HostIntrinsic` (new, with
`HostIntrinsic::ObjectPrototype`, `HostIntrinsic::FunctionPrototype`,
`HostIntrinsic::ErrorPrototype`) and `WellKnownSymbol` (new, with `WellKnownSymbol::ToStringTag`,
`WellKnownSymbol::Unscopables`): the host names these intrinsics and symbols from a closed list
(`HOST-183`, `HOST-184`)."

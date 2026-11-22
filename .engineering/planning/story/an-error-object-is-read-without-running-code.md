---
format: aep.planning-md/3
id: story:an-error-object-is-read-without-running-code
kind: story
status: active
title: The host reads whether a value is an error object, its kind and its own message, without running code or calling a hook; a host object can carry [[ErrorData]] as Web IDL gives DOMException
summary: '[[ErrorData]] records the kind of the constructor that made the object; HostObjectSpec gains an optional error kind that gives a host object the slot; a read on Agent, Cx and HookCx answers that kind and the own "message" data property when it holds a String, from the property list alone.'
relations:
- depends_on: story:own-keys-charged-before-the-list-exists
- depends_on: story:the-host-charges-its-own-bytes-and-steps
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:48:26Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T06:48:26Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
Waits for: the budget batch's `story:own-keys-charged-before-the-list-exists` (it writes
`crates/swell/src/object/ordinary.rs`, `ordinary_own_property_keys`, and
`crates/swell/src/builtins/object/mod.rs`, `get_own_property_keys`; this story writes other
lines of both) and `story:string-comparison-and-property-keys-charge-per-code-unit` (it writes
the atom lookup that makes nothing in `crates/swell/src/heap/string.rs`, which this story reads;
that story waits for the batch's arena story, which edits the tests of `heap/arena.rs` and
`host/host_object.rs` that this story's changes touch) and
`story:the-host-charges-its-own-bytes-and-steps` (both write the helper
`answers_faulted_everywhere` of `crates/swell/tests/faults.rs`, each one parameter). Lands before
`story:host-functions-and-host-objects-are-charged-within-stated-ceilings`, which waits for it
(both write `crates/swell/tests/host_objects.rs`; that story's 40 host objects are written with
`error_kind: None` from the start). Batch H3. Requirement HOST-179.
Consumer: the browser, requirement 5 of its list. Decision D5 (taken: option A, the kind in the
error's [[ErrorData]] slot).

## Context

Line numbers are of `wave/budget-1` at `ed56b05`.

- [[ErrorData]] is `OrdinaryData::Error`, a slot with no value
  (`crates/swell/src/object/ordinary.rs:32-33`). It is set by `create_error`
  (`crates/swell/src/builtins/error.rs:100-114`), which every error the engine throws goes
  through (`throw_error`, `:118-127`; `new_error_in`, `:251-259`), and by the `Error` and
  NativeError constructors (`error_constructor`, `:130-149`, the slot at `:142`). It is read by
  `Error.isError` (`:187-198`) and by `Object.prototype.toString` (`builtin_tag`,
  `crates/swell/src/builtins/object/mod.rs:478-500`, "Error" at `:491-492`). Both read only an
  ordinary object's slot.
- A host object has no [[ErrorData]]: `HostObjectSpec` is `prototype`, `brand`, `data`
  (`crates/swell/src/host/host_object.rs:55-61`), and `HostObject` `brand`, `data` (`:63-67`).
  Web IDL gives a `DOMException` instance the slot: "If interface is DOMException, append
  [[ErrorData]] to slots." (`.cache/webidl/webidl.txt:15115`, in "internally create a new
  object implementing the interface") and "It also has [[ErrorData]] and [[Stack]] slots, like
  all built-in exceptions." (`:16564`, "DOMException custom bindings"). So `Error.isError(e)` of
  a `DOMException` is true and its builtinTag is "Error".
- `HostObjectSpec` has no `#[non_exhaustive]` and is built by struct literal: `grep -rn
  "HostObjectSpec {" crates` gives 35 lines in 17 files, the definition
  (`crates/swell/src/host/host_object.rs:55`) and 34 literals in 16 files:
  `crates/test262/src/host.rs`, `crates/swell/src/heap/arena.rs`, and
  `crates/swell/tests/{collect,scripts,realms,builtins_global_this,calls,host_objects,eval,limits_scripts,budgets,conversion,interrupt,hooks,fuzz,misuse}.rs`.
- A faulted agent answers every read with `Err(HostError::Faulted)` (HOST-48), which the helper
  `answers_faulted_everywhere` asserts (`crates/swell/tests/faults.rs:30-67`).
- Which kind an error is can be told today only from its [[Prototype]] or its inherited `name`,
  which a script can change, and only by `cx.get_property`, which runs a getter a script defined.
- An own data property is found without code by the object's property list
  (`KeyedList::get`, `crates/swell/src/object/property.rs:266-270`). The key "message" is an
  atom; `name_key` makes it if it does not exist (`crates/swell/src/ops/conversion.rs:353-356`),
  which allocates; `Heap::intern` (`crates/swell/src/heap/string.rs:136-167`) makes one on a
  miss. `story:string-comparison-and-property-keys-charge-per-code-unit` splits its lookup half
  (`:137-143`) into a function that answers the atom or none and makes nothing; this story calls
  it, and an absent atom "message" means no object has an own `message`.

## Change

Failing tests first (Acceptance). Then:

1. **The kind in the slot** (decision D5, option A). `OrdinaryData::Error` becomes
   `OrdinaryData::Error(ErrorKind)`: `create_error` takes the kind (its two callers know it), and
   `error_constructor` records the kind of the constructor called (`kind`, `:132`), whatever
   prototype `new_target` gives. A subclass's construction through `super(...)` records the kind
   of the constructor `super` reaches (slice 8).
2. **A host object with [[ErrorData]].** `HostObjectSpec` gains `pub error_kind:
   Option<ErrorKind>`; `Some(kind)` gives the host object the slot with that kind (a field of
   `HostObject`). The browser makes a `DOMException` with `Some(ErrorKind::Error)`. Every
   literal of the Context gets `error_kind: None`; a breaking change of the interface, named so
   in the changelog line.
3. **The slot, read by the engine.** `Error.isError` and `builtin_tag` answer for an ordinary
   object with `Error(_)` and for a host object with `error_kind: Some(_)` as for an error: true
   and "Error" (an own or inherited `@@toStringTag` still wins in `Object.prototype.toString`).
4. **The read**, in `crates/swell/src/host/outcome.rs`, `scoped_methods!`, and on `Agent` over
   a root:

   ```rust
   pub struct ErrorSummary {
       pub kind: ErrorKind,
       /// The own "message" property's value when it is a data property holding a String.
       pub message: Option<Vec<u16>>,
   }
   impl<H: Host> Agent<H> {
       pub fn error_summary(&self, value: &Root) -> Result<Option<ErrorSummary>, HostError>;
   }
   // Cx and HookCx:
   pub fn error_summary(&self, value: Value) -> Result<Option<ErrorSummary>, HostError>;
   ```

   `None` for a value that is not an object with [[ErrorData]] (a primitive, a plain object, a
   host object made with `error_kind: None`). The message is read from the own property list
   with no [[Get]]: an accessor, a non-String value or no own "message" answers `message: None`;
   a `message` inherited from the prototype is not read. The code units are exact (`DOMString`).
   It is a read of decision 13: no code, no hook, no step, no allocation in the heap, so
   `heap_bytes()` and `live_bytes()` read the same after it. A faulted agent answers
   `Agent::error_summary` with `Err(HostError::Faulted)`.
5. **Stays as it is.** Every error object's properties and prototype; the answers of
   `Error.isError` and `Object.prototype.toString` for every object the engine makes; the
   messages; determinism.
6. **Out of scope, named.** `AggregateError` (slice 10) adds its kind to `ErrorKind` with its
   constructor; `cause` and `name` are not read; the [[Stack]] slot of Web IDL (no engine object
   has one).

Design preconditions (the design author writes them before the unit starts):
`docs/design/host-interface.md` decision 13, the read and the type, with "`error_summary` reads
[[ErrorData]] and the own `message` data property: the kind recorded by the constructor that
made the object, or given by the host in `HostObjectSpec::error_kind`, never its prototype or
`name`."; decision 15, the field, with "A host object made with an `error_kind` has an
[[ErrorData]] slot of that kind, as Web IDL gives a `DOMException` (`webidl:` "internally create
a new object implementing the interface", "DOMException custom bindings"): `Error.isError`
answers true for it." And the line of HOST-179 below, with its pending line in
`conformance/host/requirements.txt`.

New requirement:

- **HOST-179** (swell:D13) After the script `throw new TypeError("a\uD800")` answered `Throw`,
  `error_summary` of its root answers kind `TypeError` and message `[0x61, 0xD800]`, also after
  the script set the error's prototype to `null` and defined an inherited getter `message` on
  `Object.prototype`; for an error whose own `message` is an accessor whose getter throws it
  answers `message: None` and runs no code (a counter in the getter stays 0); a host object made
  with `error_kind: Some(TypeError)` on which the host defined an own data property `message`
  holding `"m"` answers kind `TypeError` and message `[0x6D]`, `Error.isError` of it answers
  `true` and `Object.prototype.toString.call` of it `"[object Error]"`; for `throw 1`, a plain
  object and a host object made with `error_kind: None` and the same own `message` it answers
  `None`, and `Error.isError` of that host object `false`; and `last_entry_steps()`,
  `heap_bytes()` and the count of hook calls are unchanged by it. [slice 6]

Writes: `crates/swell/src/object/ordinary.rs` (the slot, lines 32-33),
`crates/swell/src/builtins/error.rs` (`create_error`, `throw_error`, `error_constructor`,
`error_is_error`, `new_error_in`), `crates/swell/src/builtins/object/mod.rs` (`builtin_tag`,
lines 478-500), `crates/swell/src/host/host_object.rs` (`HostObjectSpec`, `HostObject`),
`crates/swell/src/host/mod.rs` (`scoped_new_host_object`, lines 215-255, and the scoped
read), `crates/swell/src/host/outcome.rs` (`ErrorSummary`), `crates/swell/src/host/agent.rs`
(`Agent::error_summary`), the line `error_kind: None` in each of the 34 `HostObjectSpec`
literals of the 16 files of the Context (no other line of them), `crates/swell/tests/errors.rs`,
`crates/swell/tests/faults.rs` (the helper `answers_faulted_everywhere`: one parameter, a `Root`,
and its assertion; HOST-71's test: the root taken before the fault, its argument), and in
`conformance/host/requirements.txt` the line of HOST-179 only. Not written:
`crates/swell/src/heap/string.rs`.

## Acceptance

In `crates/swell/tests/errors.rs`, each in both collection modes:

- `host_179_an_error_is_read_without_running_code`: as HOST-179, the read made through
  `Agent::error_summary` of a root, through `cx.error_summary` in `with_cx`, and through the
  `HookCx` of the setup closure of a second `create_realm`, which reads the error from a root
  the closure captured (`get`, then `error_summary`). Today it does not compile; with a stub
  answering `None` it fails on the kind.
- `every_engine_error_records_its_kind`: for each `ErrorKind`, an error the engine throws (a
  `TypeError` of `null.x`, a `RangeError` of `frame_depth`, a `SyntaxError` of `eval("var 1")`, a
  `ReferenceError` of an unbound name, a `URIError` of `decodeURI("%")`, an `EvalError` and an
  `Error` made by their constructors) answers its kind. Today it does not compile.
- In `crates/swell/tests/faults.rs`, with the feature `fault-injection` as today: HOST-71's test
  takes a root of the global object (`global_object`) before it injects the fault and passes it
  to `answers_faulted_everywhere`, which asserts that `agent.error_summary(&root)` answers
  `Err(HostError::Faulted)`. Today it does not compile.
- `error_is_error_and_to_string_are_unchanged` (a guard): `Error.isError` and
  `Object.prototype.toString.call(e)` of each kind of engine error answer as on the base.

Commands, each one job of the build pool (one slot): `slot cargo nextest run --locked -p swell
--profile default --test <b>` for each `<b>` of `errors`, `builtins_error_is_error`,
`builtins_object` and `host_objects`, the same with `--features collect-every-safe-point`, `slot
cargo nextest run --locked -p swell --profile default --features fault-injection --test faults`,
`slot
cargo nextest run --locked -p swell --profile default --lib` in both modes, `slot cargo build
--locked -p swell-test262` (its host's literal), `slot cargo clippy --workspace --all-targets
--locked -- -D warnings`; at the end, once, `slot timeout 1800 task test262 TEST262_OUT=.out/test262-unit`,
whose `runs.tsv` is compared with that of `slot timeout 1800 task test262 TEST262_OUT=.out/test262-base` run
on the base: `awk -F'\t' '{ print $3, $4, $5 }'` over each, and `cmp` of the two outputs exits 0
(the directories at risk: `test/built-ins/Error`, `test/built-ins/NativeErrors`,
`test/built-ins/Object/prototype/toString`); and last, once, in the background, `slot timeout 1800 task
host-requirements-check` (it builds every test target of the workspace, and so every file whose
literal changed). Never `cargo test -p swell` whole, `slot task test-collect`, `slot task test`
or `slot task check`.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change, except the one named as a guard. A test is killed at 60 s and the
gate's test steps fail over 120 s; each test runs in a process of its own under cargo-nextest, so
nothing is shared through a `static` between tests. Each new test is timed, and one over 5 s is
named in the report with its time. A job of the build pool asks for one slot; one over 10 minutes is killed, so `host-requirements-check` on a cold tree and the full test262 run are named `slot timeout 1800 <command>`. Determinism: the
same script ends the same way at the same count. Nothing is added to the frame of
`Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack tests are unchanged. No third-party
crate, no unsafe code.

## Report

The new tests with their times, and this line for the changelog's "Host interface" heading:
"`error_summary` (new, on `Cx` and `HookCx`), `Agent::error_summary` (new) and `ErrorSummary`
(new), with `ErrorSummary::kind` and `ErrorSummary::message`, read whether a value is an error
object, the kind of the constructor that made it and its own `message`, without running code;
**breaking**: `HostObjectSpec::error_kind` (new field; a literal adds `error_kind: None`) gives a
host object an [[ErrorData]] slot, as Web IDL gives a `DOMException` (`HOST-179`)."

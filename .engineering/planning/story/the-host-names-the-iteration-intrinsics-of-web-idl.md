---
format: aep.planning-md/3
id: story:the-host-names-the-iteration-intrinsics-of-web-idl
kind: story
status: active
title: The closed list gains @@iterator and %Array.prototype.values%, entries, keys and forEach, which Web IDL gives an interface with an indexed getter and a value iterator
summary: Five entries more in HostIntrinsic and WellKnownSymbol once slice 6 has made them; three new intrinsic slots of the realm record the original Array methods.
relations:
- depends_on: story:the-host-names-a-closed-list-of-intrinsics-and-symbols
- depends_on: story:symbol-constructor-and-well-known-symbols
- depends_on: story:iterator-protocol-and-array-string-iterators
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:48:27Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T06:48:28Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
Waits for: `story:the-host-names-a-closed-list-of-intrinsics-and-symbols` (the enumerations and
the reads), and two stories of slice 6 (`docs/plan/waves.md`, "Slice 6"):
`story:symbol-constructor-and-well-known-symbols` (`WellKnown::iterator`) and
`story:iterator-protocol-and-array-string-iterators` (`Array.prototype.values`, `keys`,
`entries`, `Intrinsic::ArrayPrototypeValues`, and the test binary
`crates/swell/tests/builtins_iterators.rs`, a new file of its Writes). Batch H3. Requirement
HOST-185. Consumer: the browser, requirement 9 of its list and its "The DOM from Web IDL"
(`NodeList`, `iterable<Node>`). Decision D9 (taken: option A, with its conditions: the four
Array methods beside `@@iterator`, and the list grown only with a changelog line under D7).

## Context

Line numbers are of `wave/budget-1` at `ed56b05`; the slice 6 stories named above change them
first.

- Web IDL, "Iterable declarations" (`.cache/webidl/webidl.txt:13660-13688`): an interface with an
  indexed property getter gets `@@iterator` = `%Array.prototype.values%`; one that also has a
  value iterator (`NodeList`'s `iterable<Node>`) gets `entries`, `keys`, `values` and `forEach` =
  `%Array.prototype.entries%`, `%Array.prototype.keys%`, `%Array.prototype.values%` and
  `%Array.prototype.forEach%`. The browser's list names `@@iterator` and
  `%Array.prototype.values%`; the other three are needed by the same rule.
- `Intrinsic` has no slot for `forEach` (`crates/swell/src/builtins/array/iteration.rs:75`
  installs it; `crates/swell/src/realm/intrinsics.rs:32-79`, no entry), nor for `entries` and
  `keys` after the iterator story, which adds only `ArrayPrototypeValues` (its Writes,
  `realm/intrinsics.rs`). An intrinsic must be the original function, whatever a script did to
  `Array.prototype` since.

## Change

Failing tests first (Acceptance). Then:

1. `realm/intrinsics.rs`: `Intrinsic::ArrayPrototypeForEach`, `ArrayPrototypeEntries` and
   `ArrayPrototypeKeys`, each recorded where its builtin is installed (`builtins/array/iteration.rs`
   for `forEach`, `builtins/array/mod.rs` for the two others), traced with the others, with their
   arms in `Intrinsics::get` (a match with no wildcard).
2. `HostIntrinsic` gains `ArrayPrototypeValues`, `ArrayPrototypeEntries`, `ArrayPrototypeKeys`,
   `ArrayPrototypeForEach`; `WellKnownSymbol` gains `Iterator`; the reads map them.
3. **Stays as it is.** Every builtin and its properties; the realm's bytes grow by three slots
   (stated in the report); determinism.

Design preconditions (the design author writes them before the unit starts): decision 7's list
gains the five entries with the clause "Iterable declarations" of Web IDL; and the line of
HOST-185 below, with its pending line in `conformance/host/requirements.txt`.

New requirement:

- **HOST-185** (webidl:define-the-iteration-methods) A host object `list` with indexed own
  data properties `0` and `1` holding the host objects `a` and `b`, a `length` of 2, and own data
  properties `@@iterator`, `entries`, `keys`, `values` and `forEach` defined from
  `well_known_symbol(Iterator)` and the four intrinsics, makes the script
  `var r = [], it = list[Symbol.iterator](); r.push(it.next().value === a, it.next().value === b,
  it.next().done); var e = list.entries().next().value; r.push(e[0] === 0 && e[1] === a,
  list.keys().next().value === 0, list.values().next().value === a); list.forEach(function (n,
  i) { r.push(n === (i ? b : a)); }); r.join()` answer `"true,true,true,true,true,true,true,true"`
  (no `for`-`of`, which a later story of slice 6 brings), also after a script replaced
  `Array.prototype.values`, `entries`, `keys` and `forEach` before the host defined them;
  `intrinsic(realm, ArrayPrototypeValues)` answers the same object as
  `Array.prototype[Symbol.iterator]` read before that script. [slice 6]

Writes: `crates/swell/src/realm/intrinsics.rs`, `crates/swell/src/builtins/array/{mod,iteration}.rs`
(the three registrations only), `crates/swell/src/host/host_object.rs` (the enumerations'
entries), `crates/swell/src/host/mod.rs` (the arms of the two reads),
`crates/swell/tests/host_objects.rs`, and in `conformance/host/requirements.txt` the line of
HOST-185 only.

## Acceptance

In `crates/swell/tests/host_objects.rs`, in both collection modes:

- `host_185_a_node_list_iterates_with_the_array_methods`: as HOST-185. Today it does not compile
  (no `ArrayPrototypeEntries` nor `WellKnownSymbol::Iterator`).
- `the_iteration_intrinsics_are_the_originals`: each of the four answers the function the script
  reads from `Array.prototype` in a fresh realm, and still answers it after a script deleted
  them from `Array.prototype`. Today it does not compile.

Commands, each one job of the build pool (one slot): `slot cargo nextest run --locked -p swell
--profile default --test <b>` for each `<b>` of `host_objects`, `builtins_array_iteration` and
`builtins_iterators` (the iterator story's binary), the same with `--features
collect-every-safe-point`, `slot cargo clippy --workspace --all-targets --locked -- -D
warnings`; at the end, once, `slot timeout 1800 task test262 TEST262_OUT=.out/test262-unit`, whose
`runs.tsv` is compared with that of `slot timeout 1800 task test262 TEST262_OUT=.out/test262-base` run on the
base: `awk -F'\t' '{ print $3, $4, $5 }'` over each, and `cmp` of the two outputs exits 0; and
last, once, in the background, `slot timeout 1800 task host-requirements-check` (it builds every test target
of the workspace). Never `cargo test -p swell` whole, `slot task test-collect`, `slot task test`
or `slot task check`.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change. A test is killed at 60 s and the gate's test steps fail over 120 s;
each test runs in a process of its own under cargo-nextest, so nothing is shared through a
`static` between tests. Each new test is timed, and one over 5 s is named in the report with
its time. A job of the build pool asks for one slot; one over 10 minutes is killed, so `host-requirements-check` on a cold tree and the full test262 run are named `slot timeout 1800 <command>`. Determinism: the same calls answer the
same. Nothing is added to the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack
tests are unchanged. No third-party crate, no unsafe code.

## Report

The tests with their times, the realm's bytes before and after, and this line for the
changelog's "Host interface" heading: "`HostIntrinsic::ArrayPrototypeValues`,
`HostIntrinsic::ArrayPrototypeEntries`, `HostIntrinsic::ArrayPrototypeKeys`,
`HostIntrinsic::ArrayPrototypeForEach` and `WellKnownSymbol::Iterator` (new): what Web IDL gives
an iterable interface (`HOST-185`)."

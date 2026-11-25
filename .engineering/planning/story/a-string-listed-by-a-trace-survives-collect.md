---
format: aep.planning-md/3
id: story:a-string-listed-by-a-trace-survives-collect
kind: story
status: active
title: A String or Symbol that host data lists with Tracer::value survives collect
summary: HOST-218 for a String and a Symbol held only through a traced value, HOST-219 pending slice 12 for a BigInt; the design sentence says a value, not an object.
relations:
- depends_on: story:interrupt-read-after-a-host-function-throws
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: inferred
  path: crates/swell/src/heap/collect.rs
- confidence: cited
  path: crates/swell/tests/host_objects.rs
- confidence: cited
  path: docs/design/host-interface.md
- confidence: cited
  path: website/data/status.json
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T03:16:25Z", actor: "human:operator", revision: 9}
- {from: "proposed", to: "active", at: "2026-11-25T03:16:25Z", actor: "human:operator", revision: 10}
---
Waits for: `story:interrupt-read-after-a-host-function-throws` (it takes the numbers HOST-216 and
217). Base: `main` after that story. Asked by the first consumer (surf, finding F1): it keeps the
name and message of a DOMException as engine Strings in host data and lists them with
`Tracer::value`.

## Context

Line numbers are of `main` at `d49a09b`.

- `docs/design/host-interface.md:394-396` promises that a handle that host data lists in its
  trace is strong, and says "an object". `Tracer::value(&mut self, value: Value)`
  (`crates/swell/src/host/host_object.rs:38`, decision 15 at `host-interface.md:1736`) takes any
  `Value`; the collector traces Strings and Symbols there (`crates/swell/src/heap/collect.rs:156-161`).
- HOST-109 (`host-interface.md:1819`, test
  `swell::host_objects::host_109_trace_keeps_listed_values`) holds only an object reachable
  through a listed value; its test reads the unlisted object's `cx.brand` as
  `Err(HostError::StaleHandle)` after `collect` (`crates/swell/tests/host_objects.rs:158-163`).
  No requirement holds a String or a Symbol listed that way.
- No host call takes a `SymbolRef` and reads it; a Symbol is read back by a script.
- `Value::BigInt` exists (`crates/swell/src/value.rs:28`); a BigInt is made by script from slice
  12 on, which is not delivered.

## Change

Failing test first, if one fails (Acceptance). Then:

- `docs/design/host-interface.md`: the sentence at `:394-396` says "a value" for "an object",
  and two requirements in the section of HOST-109, after it:
  - **HOST-218** (swell:D15) A String made by `String.fromCharCode(0x61, 0xD800)` and a Symbol
    made by `Symbol("d")`, each kept only in a host object's data and listed by its `trace`
    with `Tracer::value`, survive `collect`: after it `to_utf16` of the String gives
    `[0x61, 0xD800]`, and a host function that returns the kept Symbol, called by the script
    `get().description`, answers `Normal("d")`. A String kept the same way but not listed
    answers `Err(HostError::StaleHandle)` from `to_utf16` after `collect`. [slice 1]
  - **HOST-219** (swell:D15) A BigInt kept only in a host object's data and listed by its
    `trace` survives `collect`, and a script reading it back through a host function compares
    equal to the BigInt it made. [slice 12]
- `conformance/host/requirements.txt`: `HOST-218 test swell::host_objects::<name>`,
  `HOST-219 pending slice 12`; `task site-data` regenerates `website/data/status.json`.
- `CHANGELOG.md`, `## [Unreleased]`, `### Host interface`: one entry that names `HOST-218` and
  `HOST-219` in backticks, as `host-surface-check` (`taskfiles/host-surface.yml:92-108`)
  requires.
- If the test fails on the base, the smallest change in the collector's trace of host data
  (`crates/swell/src/heap/collect.rs:156`) makes it pass; a change to `crates/*/src` then moves
  the test262 sources digest, and the full run writes it with `expected.txt` unchanged.

## Acceptance

- The HOST-218 test is written first; its result on the base is reported. It passes after the
  change in both collection modes:
  `slot cargo nextest run --locked -p swell --test main -E 'test(/^host_objects::/)'` and the
  same with `--features collect-every-safe-point`.
- `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` prints 219
  requirements, 151 test, `pending by slice` with `12:3`; `slot --local task site-data-check`
  exits 0; `sh taskfiles/test-main.sh check` exits 0; `slot task clippy` exits 0.
- The repository's gate `slot task check` exits 0 on the branch, its `host-surface` line
  included.

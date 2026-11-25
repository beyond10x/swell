---
format: aep.planning-md/3
id: story:the-host-reads-the-length-of-a-string
kind: story
status: active
title: The host reads the length of a String in code units without a copy
summary: Cx::to_utf16_len beside to_utf16, additive, and HOST-220.
relations:
- depends_on: story:a-string-listed-by-a-trace-survives-collect
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: conformance/test262/expected.digest
- confidence: inferred
  path: crates/swell/src/heap/string.rs
- confidence: cited
  path: crates/swell/src/host/mod.rs
- confidence: cited
  path: docs/design/host-interface.md
- confidence: cited
  path: website/data/status.json
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T03:16:26Z", actor: "human:operator", revision: 10}
- {from: "proposed", to: "active", at: "2026-11-25T03:16:26Z", actor: "human:operator", revision: 11}
---
Waits for: `story:a-string-listed-by-a-trace-survives-collect` (it takes the numbers HOST-218 and
219). Base: `main` after that story. Asked by the first consumer (surf, finding F2, a wish):
`DOMException.code` compares a name with 25 legacy names and copies it to do so.

## Context

Line numbers are of `main` at `d49a09b`.

- The host reads a String only through `to_utf16` (`crates/swell/src/host/mod.rs:532`, HOST-32)
  and `to_rust_string_lossy` (`:538`, HOST-33). Both return a whole copy:
  `scoped_to_utf16` (`:232-240`) gets the String from `heap.strings` and calls its `to_utf16`
  (`crates/swell/src/heap/string.rs:158-163`). No call answers the length of a String in code
  units.
- The copy `to_utf16` makes is a plain `Vec` the heap's ledger does not count, so no reading
  the host has tells a copy from no copy. The length call's lack of a copy is a property of
  its code, stated in Change and read in review; the requirement states what the host sees.

## Change

Failing test first (Acceptance). Then:

- `Cx::to_utf16_len(&self, string: StringRef) -> Result<usize, HostError>` beside `to_utf16`,
  through a `scoped_to_utf16_len` beside `scoped_to_utf16`: `alive()`, then the String from
  `heap.strings`, then the number of code units its storage holds (a length the storage has,
  or one added to `heap/string.rs` that reads it), with no `Vec`, no iteration over units, no
  step and no hook. A stale handle answers `Err(HostError::StaleHandle)` as `to_utf16` does. The
  same method on every scope that has `to_utf16`.
- `docs/design/host-interface.md`: the method in the listing of decision 15 next to `to_utf16`,
  and one requirement in the section of HOST-32, after HOST-33:
  - **HOST-220** (webidl:idl-DOMString) `to_utf16_len` answers the number of code units of a
    String, lone surrogates included, equal to the length of what `to_utf16` gives:
    `new_string_utf16(&[0xD800, 0x61])` gives 2, the empty String gives 0, and a String of
    1,048,576 code units gives 1,048,576; a handle freed by `collect` answers
    `Err(HostError::StaleHandle)`. [slice 1]
- `conformance/host/requirements.txt`: `HOST-220 test swell::<module>::<name>`.
- `CHANGELOG.md`, `## [Unreleased]`, `### Host interface`: `Cx::to_utf16_len` (and each other
  scope's method) and `HOST-220`, each in backticks in the form `docs/releasing.md` states, as
  `host-surface-check` (`taskfiles/host-surface.yml:7-29`, `:92-108`) requires; additive, not
  breaking. `conformance/host/surface.txt` stays as released.
- `task site-data` regenerates `website/data/status.json`. The test262 sources digest moves and
  the full run writes it with `expected.txt` unchanged.

## Acceptance

- The HOST-220 test fails to compile on the base (no method); it passes after the change in
  both collection modes.
- `git diff -- crates/swell/src/host/mod.rs` shows the new method's body calls no `to_utf16`,
  `collect`, `iter` or `Vec` (read in review).
- `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` prints 220
  requirements, 152 test; `slot --local task site-data-check` exits 0; `slot task clippy` exits
  0; `sh taskfiles/test-main.sh check` exits 0.
- The repository's gate `slot task check` exits 0 on the branch, its `host-surface` line
  included.

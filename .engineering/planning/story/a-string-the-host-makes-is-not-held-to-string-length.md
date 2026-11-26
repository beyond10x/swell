---
format: aep.planning-md/3
id: story:a-string-the-host-makes-is-not-held-to-string-length
kind: story
status: draft
title: A String the host makes is not held to string_length; the design and a requirement say so
summary: new_string and new_string_utf16 make a String of any length; string_length holds Strings created at run time from other values; HOST-<next> states the exemption with a test.
revision: 1
---
Reported by the first consumer (surf), 2026-11-25, as information: `Cx::new_string` does not
apply `Limits::string_length`. Draft: not scoped, not ordered.

## Context

Line numbers are of `main` at `b9d3d46` (release `0.10.0`).

- `Limits::string_length` (`crates/swell/src/host/limits.rs:25`) is documented as "UTF-16 code
  units of a String created at run time from other values: a `RangeError`". HOST-20, 21 and 22
  hold it for concatenation and `repeat`.
- `new_string` and `new_string_utf16` (`crates/swell/src/host/mod.rs`, `scoped_methods!`) make a
  String of any length; no requirement says whether `string_length` applies to them.
- Measured by surf at `0.8.0`: a host function answers a String of 16,777,217 code units under
  `string_length` 16,777,216; the script reads it and the run ends normally. surf now checks the
  length in its own code before `new_string`.

## Change

The coordinator's decision (2026-11-25, reversible): a String the host makes is the host's
input, not a String created at run time from other values, so `string_length` does not apply to
it; the memory budget does. The design states it; no code changes.

- `docs/design/host-interface.md`: one sentence at `string_length` that a String made by
  `new_string` or `new_string_utf16` is not held to it, and one requirement:
  **HOST-<next>** (swell:D2) With `string_length` L, a host function that answers
  `new_string_utf16` of L + 1 code units makes a String the script reads with `.length` as
  L + 1, and the script `s + "x"` on it answers `Throw` of a `RangeError`. [slice 1]
- `limits.rs:25`: the doc comment names the exemption.
- `conformance/host/requirements.txt`, `CHANGELOG.md` `### Host interface`, `task site-data`.

## Acceptance

- The requirement's test passes on the base (no engine change) in both collection modes; a
  mutant that applies the limit in `new_string_utf16` makes it fail.
- `host-requirements-check`, `site-data-check` and the gate exit 0.

---
format: aep.planning-md/3
id: story:syntax-surface-is-promised
kind: story
status: draft
title: The syntax crate's public items are a promised, versioned surface
summary: A snapshot, a changelog rule and a span on every node, for codespec's ECMAScript reader
revision: 2
---
Asked by codespec, 2026-11-25, on the owner's order to build a path JavaScript → `cir/1` →
assembly or WebAssembly. Codespec's ECMAScript reader (`adapters/ecmascript-read` in the
codespec repository) reads through swell's `syntax` crate, pinned to a swell release tag by a
git dependency, as its Rust reader reads through `syn`. Base: `main` at `3785379` (0.11.0).

## Context

- `crates/syntax` (`swell-syntax`) exports `ast`, `error`, `ledger`, `regexp` and the parse
  entries (`crates/syntax/src/lib.rs`). Nothing says which of its public items a user outside
  this repository may rely on; a change to `ast` is not named anywhere before it ships.
- `ast::Span` (`crates/syntax/src/ast.rs:12-20`) holds `start`, `end`, `line` and `column`.
  Whether every expression, statement, pattern, property and function node carries one is not
  checked.
- The host interface already has this kind of promise: `conformance/host/surface.txt` (a scan of
  every public item), `host-surface-check` (a change must be named under `### Host interface`
  of `## [Unreleased]` in `CHANGELOG.md`) and `host-surface-snapshot` at each release
  (`docs/releasing.md`).

## Change

Failing checks first (Acceptance). Then:

- `conformance/syntax/surface.txt`: a scan of every public item of `swell-syntax`, made by the
  same scanner as the host surface, and `syntax-surface-check` in `task check`: a changed item
  must be named in backticks under `### Syntax` of `## [Unreleased]`.
- `docs/design/syntax-surface.md`: what is promised (the public items of `swell-syntax` and
  their meaning), the version rule (before 1.0 a removed or changed item is a minor release, an
  added item or variant is a minor release too, since `ast` enums are matched exhaustively by a
  reader), and that every node carries a `Span`.
- A test that walks the tree of a parse of every file of a corpus and fails on a node whose
  `Span` is the default (`0, 0, 0, 0`) where the source is not empty.
- The sample corpus for the reader: `conformance/syntax/corpus.txt`, a list of test262 paths at
  the pinned revision (BSD-3-Clause, the license of test262) that swell parses and runs to a
  pass, one per ES5 statement and expression form, at most 200 lines.
- `docs/releasing.md`: `syntax-surface-snapshot` beside `host-surface-snapshot`.
- `CHANGELOG.md` `## [Unreleased]` `### Syntax`: the surface is promised from the next release.

## Acceptance

- `syntax-surface-check` refuses a planted change to a public item of `ast` not named in the
  changelog, and passes once it is named (a case of `gate-selftest`).
- The span test passes on the corpus; a planted node with a default span fails it.
- `slot --timeout 30m task check-runner` exits 0.

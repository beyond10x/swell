---
format: aep.planning-md/3
id: story:release-0-12-0-with-binaries
kind: story
status: draft
title: Release 0.12.0 with the swell binary for five targets
summary: 'The first release with assets: HOST-221, the compare page and the shell for Linux, macOS and Windows'
revision: 2
---
Wave A of the binaries plan, 2026-11-25. No build node needed beyond the local gate (the
workstation pool while the node is away); the release builds run on the forge's runners.

## Context

- `main` holds HOST-221 (`#8`), the designs (`#9`) and the compare page (`#10`); `#11` adds the
  shell binary and `.github/workflows/release-build.yml`.
- `docs/releasing.md` (as changed by `#11`): version, changelog, surface snapshot, test262 digest,
  site data, the release PR, then on a green main the tag, the "Release build" run on the tag, and
  the release with the five archives and `SHA256SUMS`.

## Change

- After `#11` is merged: the release PR for 0.12.0 by `docs/releasing.md`: version 0.12.0 in the
  manifests, `## [0.12.0]` in `CHANGELOG.md` (breaking for embedders: `cx.call` of a host function
  that sets the interrupt now answers `Terminated(Interrupted)`), `host-surface-snapshot`,
  `site-data`.
- On a green main: the tag through the bot, the tag's "Release build" run green on all five
  targets, the release created with six assets and read back.

## Acceptance

- `gh release view 0.12.0` names author `b10x-bot[bot]`, not a draft, and six assets: five
  archives (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`,
  `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`) and `SHA256SUMS`.
- Each archive's smoke test passed on its own runner in the tag's run.

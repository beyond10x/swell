---
format: aep.planning-md/3
id: story:unsigned-binaries-documented
kind: story
status: draft
title: The download page says how to run the unsigned binaries
summary: Checksum and first run per system while the binaries are unsigned; signing is story binaries-signed
revision: 5
---
Wave B of the binaries plan, 2026-11-25. No build node needed (the forge's runners and docs).
The release binaries of `story:releases-ship-the-shell-for-three-systems` are not code-signed:
macOS quarantines a downloaded unsigned binary (Gatekeeper), and Windows SmartScreen warns on an
unsigned exe.

Decided by the owner, 2026-11-25 18:03 CET: the binaries are signed (`story:binaries-signed`).
Until that story lands a release ships unsigned binaries, and the docs say so and say how to run them.

## Change

- `website/docs/embedding.md`, the download section: per system, how to verify the archive with
  `SHA256SUMS` and run the binary (macOS: remove the quarantine attribute with
  `xattr -d com.apple.quarantine swell`; Windows: SmartScreen "More info", "Run anyway").
- The release build's smoke test runs on each of the five runners from the unpacked archive, so a
  binary that does not start fails the release (already in the workflow); the docs name that.
- The docs say the binaries are not signed, by decision, and that signing may follow.

## Acceptance

- The download section names the three systems, the checksum step and the unsigned state; the
  site builds (`slot --timeout 30m npm --prefix website run build` or the forge's docs job).

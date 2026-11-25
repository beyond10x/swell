---
format: aep.planning-md/3
id: story:unsigned-binaries-documented
kind: story
status: draft
title: The download page says how to run the unsigned binaries
summary: Checksum and first run per system; signing waits for the owner's word
revision: 2
---
Wave B of the binaries plan, 2026-11-25. No build node needed (the forge's runners and docs).
The release binaries of `story:releases-ship-the-shell-for-three-systems` are not code-signed:
macOS quarantines a downloaded unsigned binary (Gatekeeper), and Windows SmartScreen warns on an
unsigned exe.

## Change

- `website/docs/embedding.md`, the download section: per system, how to verify the archive with
  `SHA256SUMS` and run the binary (macOS: remove the quarantine attribute with
  `xattr -d com.apple.quarantine swell`; Windows: SmartScreen "More info", "Run anyway").
- The release build's smoke test runs on each of the five runners from the unpacked archive, so a
  binary that does not start fails the release (already in the workflow); the docs name that.
- Open, needs the owner's word (money and credentials): signing and notarizing the macOS
  binaries with an Apple Developer ID, and signing the Windows exe with a code-signing
  certificate. Until then the binaries ship unsigned and the docs say so.

## Acceptance

- The download section names the three systems, the checksum step and the unsigned state; the
  site builds (`slot --timeout 30m npm --prefix website run build` or the forge's docs job).

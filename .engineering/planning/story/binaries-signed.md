---
format: aep.planning-md/3
id: story:binaries-signed
kind: story
status: draft
title: The macOS and Windows binaries of a release are signed
summary: Developer ID and notarization on macOS, a signing service on Windows, secrets held by the owner
revision: 2
---
Decided by the owner, 2026-11-25 18:03 CET: the macOS and Windows binaries of a release are
signed ("sign them"; option B of three: A ship unsigned, B sign, C ship Linux only). That spends
money and needs credentials; both are the owner's.

## Context

- `.github/workflows/release-build.yml` (story `releases-ship-the-shell-for-three-systems`)
  builds five archives on the forge's runners, smoke-tests each on its own runner, and the
  release carries the five archives and `SHA256SUMS`.
- macOS: a bare command-line binary is signed with a Developer ID Application certificate and the
  hardened runtime, then notarized with `xcrun notarytool`; a bare binary cannot be stapled, so
  Gatekeeper looks the ticket up online on first run.
- Windows: code-signing keys must live in a hardware module or a cloud signing service; the
  release job signs `swell.exe` through such a service before packaging.

## What the owner provides (none of it in the repository)

- An Apple Developer Program membership for the organization, a Developer ID Application
  certificate exported as .p12 with its password, the Team ID, and an App Store Connect API key
  (key id, issuer id, .p8) for notarization.
- A Windows signing account (recommended: a cloud signing service usable from a GitHub runner)
  and its credentials for the runner.
- Each value stored as a GitHub Actions secret of the repository; the names are listed in
  `docs/releasing.md`.

## Change

- `release-build.yml`: on the two macOS jobs, import the certificate into a temporary keychain,
  `codesign --options runtime --timestamp` the binary, zip it and `notarytool submit --wait`,
  fail on any status but Accepted; on the Windows job, sign `swell.exe` with the signing service
  and verify the signature. Both before Package, so the archive and `SHA256SUMS` hold the signed
  binary. A job verifies its signature (`codesign --verify --strict`, `spctl` on macOS;
  `signtool verify /pa` or the service's verify on Windows).
- When the secrets are absent the jobs fail on a tag (a release is never published unsigned
  after this story lands), and on a pull request they skip signing and say so in one line.
- `docs/releasing.md`: the secret names, what each holds, and how to rotate them.
- `website/docs/embedding.md`: the download section says the binaries are signed and how to
  check a signature.

## Acceptance

- The tag run of the first release after this story: both macOS archives hold a binary that
  `codesign --verify --strict` accepts, signed by a Developer ID with the hardened runtime (`codesign -dv`), with a notarization status of Accepted, and the Windows
  archive's `swell.exe` passes the signature check, each printed in the run's log.
- A pull request run without secrets is green and prints that signing was skipped.

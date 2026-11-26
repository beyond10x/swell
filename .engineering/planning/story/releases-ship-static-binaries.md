---
format: aep.planning-md/3
id: story:releases-ship-static-binaries
kind: story
status: draft
title: Releases ship static musl Linux binaries and a Windows binary with its runtime linked in
summary: Two musl archives beside the glibc ones, the static C runtime on Windows, seven archives per release
revision: 2
---
Decided by the owner on 2026-11-25, about 23:10 CET, for every Rust tool of the organization:
releases carry static musl Linux binaries ("A: all Rust tools"; relayed by the operator session
on 2026-11-26). For swell the operator session confirmed option A of this session's question:
the glibc archives stay beside the musl ones, and the Windows binary links its C runtime in.
The work starts after the organization's current goal; until then nothing is built.

## Context

- The binaries of 0.12.0, read from the published assets: the two Linux binaries are
  dynamically linked (`libc.so.6`, `libgcc_s.so.1`, glibc 2.34 or newer); the Windows binary
  needs `VCRUNTIME140.dll` and the `api-ms-win-crt-*` libraries; macOS links the system
  libraries, which is the only supported form there.
- `.github/workflows/release-build.yml` builds five targets; a release has five archives and
  `SHA256SUMS`.

## Change

- `release-build.yml`: two more targets, `x86_64-unknown-linux-musl` and
  `aarch64-unknown-linux-musl`, each built, packaged and smoke-tested like the others; the
  smoke test also checks that the binary has no dynamic section entry `NEEDED`.
- The Windows job builds with the static C runtime (`-C target-feature=+crt-static`); its smoke
  test checks that the import table names no `VCRUNTIME` and no `api-ms-win-crt` library.
- A release has seven archives and `SHA256SUMS`; `docs/releasing.md` ("The binary of a
  release") and `website/docs/embedding.md` name them and say which to take (musl: any Linux,
  an empty container; glibc: glibc 2.34 or newer).
- The benchmark image takes both Linux binaries as configurations, so the difference is a
  measured row (`story:swell-in-the-benchmarks`).

## Acceptance

- The tag run of the first release after this story is green on seven targets, and the release
  read back has eight assets.
- The two musl binaries run in a container image with no files but the binary (the smoke test
  of the release build, or a test in the bench image).

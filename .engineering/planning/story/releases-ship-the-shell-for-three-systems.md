---
format: aep.planning-md/3
id: story:releases-ship-the-shell-for-three-systems
kind: story
status: implemented
title: Every release ships the swell binary for Linux, macOS and Windows
summary: A tag builds, smoke-tests and packages swell-cli for five targets on the forge's runners
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T14:03:47Z", actor: "human:operator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-25T14:03:47Z", actor: "human:operator", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-26T05:55:48Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
The owner, 2026-11-25: "make sure swell ships as binary: linux, mac, windows". Every release
from the next one on carries the `swell` shell binary of `story:cli-runs-a-script-file` for
Linux, macOS and Windows. Until now a release has no asset ("swell is a library, and no binary is
built yet", `CHANGELOG.md`).

## Change

- `.github/workflows/release-build.yml`, on a pushed tag: one job per target, on the forge's own
  runners, with the toolchain of `rust-toolchain.toml`:
  `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`,
  `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`.
- Each job builds `cargo build --release --locked -p swell-cli` with paths remapped (no home path
  in the binary), runs a smoke test (`swell --version` names the tag; `swell -e "print(1 + 1)"`
  prints `2`; a step budget ends a loop with status 3), and packages `swell-<tag>-<target>.tar.gz`
  (`.zip` for Windows) with `README.md` and `LICENSE`, uploaded as a workflow artifact.
- `docs/releasing.md`: after the tag's runs pass, the release is created with the five archives
  and a `SHA256SUMS` file as assets; the read-back lists six assets.
- `website/docs/embedding.md` (or the page of the shell): where to download the binary.

## Acceptance

- The workflow's jobs pass on the forge for a test tag on a branch, or the first release tag
  after the merge; each archive runs its smoke test on its own runner.
- The release read-back names six assets: five archives and `SHA256SUMS`.

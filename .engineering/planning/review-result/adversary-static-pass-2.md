---
format: aep.planning-md/3
id: review-result:adversary-static-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on unit static of wave big-1: 4 findings, all introduced by the correction'
relations:
- reviews: story:releases-ship-static-binaries
revision: 1
---
Adversary pass 2, the last pass, on unit `static` of wave big-1
(`story:releases-ship-static-binaries`), the tree `impl/static-binaries` at `d8af68fe`,
2026-11-26. The report as the adversary returned it; its list of what it could not break is
shortened to one paragraph at the end.

```
unit: static (releases-ship-static-binaries), tree impl/static-binaries at d8af68fe plus 3 untracked test files
verdict: red
cases: executed 5→8, red 3
origin: introduced 4, pre-existing 0, undecided 0
needs-coordinator: no
```

The correction still lets the check pass a static-check step that cannot fail. It also refuses
some correct workflows and documents, and it runs real commands from the Build step on the gate
machine. The four scripts of pass 1 now pass.

Three new failing scripts, each run alone with `slot sh <script>` (exit 1):

- `tests/gate/release-targets-check-refuses-a-static-check-that-cannot-fail.sh`
- `tests/gate/release-targets-check-passes-innocent-edits-of-the-workflow-and-the-documents.sh`
- `tests/gate/release-targets-check-runs-nothing-of-the-build-step-but-the-stub.sh`

`slot sh -c 'task gate-selftest GATE_SELFTEST_DIR=.out/adv-suite2'` over the 5 committed
release-targets scripts plus the 3: `gate-selftest: 5 passed, 3 failed`, wall time 6 s, exit 201.

The cases:

- **F8:** the step "Check that the C runtime is linked in" with `continue-on-error: true`
  (case 1); every line of its `*-linux-musl)` branch commented out (case 2). Both: `task
  release-targets-check exited 0`.
- **F9 and F10:** four innocent edits, each refused. `runner-os` (the Build step adds
  `echo "... $RUNNER_OS"`): `RUNNER_OS: unbound variable`. `rustc-version` (the Build step
  adds `rustc -vV`): `could not create home directory: '/runner/home/.rustup': Permission
  denied`. `expression` (`--target "${{ matrix.target }}"`): `bad substitution`. `history`
  (`docs/releasing.md` gets "Releases up to 0.12.0 have five archives"): `says "five
  archives"; ... builds 7 archives, so it is seven`.
- **F11:** one line added to the Build step that writes a marker file: the check exits 0 and
  the marker holds the three targets.

Findings (all against `d8af68fe`):

- **F8** `taskfiles/forge.yml:222`: the keyword flags (`readelf -d`, `(NEEDED)`, `docker run`)
  are set from every line of the branch, comment lines included. Step keys other than name,
  if, run and env are never read, so `continue-on-error` goes unseen. This is F1 of pass 1 in
  another spelling. Reached by: switching the check off for a while, or a step key edit.
  Introduced.
- **F9** `taskfiles/forge.yml:265`: the Build step runs under `env -i` with only PATH, HOME,
  GITHUB_WORKSPACE and TARGET set, and with `${{ }}` left as written. A correct Build step
  that reads a runner variable, calls `rustc`, or uses an expression is refused, which blocks
  every later pull request. Reached by: an edit of the Build step. Introduced.
- **F10** `taskfiles/forge.yml:306`: any "<number> archives/targets" anywhere in
  `docs/releasing.md` is taken as a count of this release, so a sentence about history is
  refused. Reached by: a document edit. Introduced.
- **F11** `taskfiles/forge.yml:261`: only `cargo` is replaced. Every other command of the
  Build step runs on the gate machine with the host's PATH, with no time bound, in the
  repository root. A future `sudo apt-get install musl-tools`, `rustup target add` or `curl`
  line would run (or hang) in the gate. Reached by: an edit of the Build step. Introduced.

Attacked and not broken: the merge with `impl/binaries-signed` (no conflict, the check exits 0
on the result); a renamed step, a comment in a run block, a second RUSTFLAGS flag,
`-Ctarget-feature` without the blank, an `if:` inside `${{ }}`, another `shell:` form; the
time (the check takes 0.5 to 0.7 s, the 8 planted scripts 6 s together); F1 to F4 of pass 1
(all four scripts pass); F5, F6 and F7 as corrected.

```findings
- file: taskfiles/forge.yml
  line: 222
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: release-targets-check exits 0 when the static check step has continue-on-error true or its musl branch is entirely commented out.
- file: taskfiles/forge.yml
  line: 265
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The stubbed Build step runs under env -i without runner variables or expression substitution, so a correct step using RUNNER_OS, rustc or an expression is refused.
- file: taskfiles/forge.yml
  line: 306
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Any number before archives or targets anywhere in docs/releasing.md is read as the release count, so a sentence about earlier releases is refused.
- file: taskfiles/forge.yml
  line: 261
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Only cargo is stubbed, so every other Build step command runs on the gate machine with the host PATH, no time bound and write access to the repository root.
```

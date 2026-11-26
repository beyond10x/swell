---
format: aep.planning-md/3
id: review-result:adversary-static-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on unit static of wave big-1: 7 findings, all introduced'
relations:
- reviews: story:releases-ship-static-binaries
revision: 1
---
Adversary pass 1 on unit `static` of wave big-1 (`story:releases-ship-static-binaries`), the
tree `impl/static-binaries` at `662e3ea8`, 2026-11-26. The report as the adversary returned it,
without its list of what it could not break; the findings block is the one the adversary sent
again after the store refused the fields of the first one.

```
unit: static (releases-ship-static-binaries), tree impl/static-binaries at 662e3ea8 plus 4 untracked test files
verdict: red
cases: executed 1→5, red 4
origin: introduced 7, pre-existing 0, undecided 0
needs-coordinator: no
```

The four failing cases, each run alone through `slot sh <script>` (exit 1), defects planted in
copies under `.out/gate-tests/<name>/root`:

- `tests/gate/release-targets-check-refuses-a-musl-job-without-its-static-check.sh`
- `tests/gate/release-targets-check-refuses-crt-static-on-a-pattern-that-does-not-match.sh`
- `tests/gate/release-targets-check-refuses-documents-that-count-another-number-of-archives.sh`
- `tests/gate/release-targets-check-refuses-an-archive-name-with-another-extension.sh`

`slot task gate-selftest GATE_SELFTEST_DIR=.out/adv-suite`: `gate-selftest: 1 passed, 4 failed`,
exit 201.

Findings:

- **F1** `taskfiles/forge.yml:76`: the check reads only the matrix and the flag line, never
  the step that runs `readelf` and `dumpbin`. Reached by: the story's acceptance (no `NEEDED`,
  the binary runs in an empty image). If that step is deleted or switched off, the local gate
  stays green and the forge stops checking. Introduced, confirmed.
- **F2** `taskfiles/forge.yml:91`: any non-comment line that contains both `windows-msvc` and
  `target-feature=+crt-static` satisfies the check, whether or not the flag reaches the
  Windows build. Reached by: an edit of the Build step. Introduced, confirmed.
- **F3** `taskfiles/forge.yml:110`: nothing ties the counts in the documents ("seven
  targets", "seven archives", "eight" assets) to the size of the matrix. Reached by: removing
  a matrix entry together with its table rows. Introduced, confirmed.
- **F4** `taskfiles/forge.yml:96`: the extra-archive check only sees names ending in
  `.tar.gz` or `.zip`. Reached by: a document edit. Introduced, confirmed.
- **F5** `docs/releasing.md:221` (reading only): on a `pull_request` run `GITHUB_SHA` is the
  forge's merge commit, not the head of the branch, so the label names a commit that is not
  on the branch. Introduced.
- **F6** `docs/releasing.md:218` (reading only): a run started by hand on a tag ref is
  labelled with the tag, not version plus commit as the document says. Introduced.
- **F7** `.github/workflows/release-build.yml:234` (reading only): pull-request runs upload
  the seven archives as workflow artifacts kept for 30 days, while `docs/releasing.md:219`
  says such a run publishes nothing. No release is made. Introduced.

The adversary's verdict words in its second sending: F1 to F5 and F7 CONFIRMED (F5 and F7 by
reading), F6 INFEASIBLE (no documented workflow starts the run by hand on a tag).

```findings
- file: taskfiles/forge.yml
  line: 76
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "release-targets-check exits 0 on a workflow without the readelf/dumpbin step, or with an if: that no target meets."
- file: taskfiles/forge.yml
  line: 91
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "The crt-static check is a same-line word match, so a case pattern windows-msvc) that matches no target passes."
- file: taskfiles/forge.yml
  line: 110
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: A target removed from the matrix and both documents passes while docs/releasing.md still says seven archives and eight assets.
- file: taskfiles/forge.yml
  line: 96
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: A document naming an archive with another extension (.tar.xz, .tgz) or a concrete version is not seen by the check.
- file: docs/releasing.md
  line: 221
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: On pull_request GITHUB_SHA is the merge commit, so the label's seven characters name no commit of the branch (by reading).
- file: docs/releasing.md
  line: 218
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: A run started by hand on a tag ref is labelled with the tag, not version-commit as documented; no documented workflow does this.
- file: .github/workflows/release-build.yml
  line: 234
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Pull-request runs upload the archives as 30-day artifacts while the document says such a run publishes nothing (by reading).
```

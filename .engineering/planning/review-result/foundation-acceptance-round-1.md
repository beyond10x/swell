---
format: aep.planning-md/3
id: review-result:foundation-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, foundation stories, round 1
relations:
- reviews: story:workspace-and-gate
- reviews: story:test262-fetch
- reviews: story:unicode-data-pin
- reviews: story:spec-gate
- reviews: story:test262-counting-runner
- reviews: story:test262-expected-and-sections
revision: 1
---
needs-revision
story:workspace-and-gate — the summary says `check` refuses warnings "shown by a planted test", but no `tests/gate/` script plants a warning or a denied `unwrap()`. As written, `slot task check` exits 0 the same way whether or not `RUSTFLAGS=-D warnings` and the four decision-13 clippy denials are in place — .engineering/planning/story/workspace-and-gate.md:123
story:unicode-data-pin — the acceptance sets `<n>` "equal to the number of `SOURCE` lines", which the task already guarantees by construction, and it names no file count and no path. A directory holding only `LICENSE` and a one-line `SOURCE` would pass, so the 14 decision-8 files and the two emoji sequence files go unchecked — .engineering/planning/story/unicode-data-pin.md:93
story:test262-expected-and-sections — the acceptance checks only that there are 33 `test262-section` lines, each `incomplete`, and that 278 lines equal an `expected.txt` written from that same output. Both hold whatever paths and omissions `sections.txt` has, so it names no section line's case or omission count (for example `test262-section sec-string-objects incomplete 1223 0 1`) — .engineering/planning/story/test262-expected-and-sections.md:150

What I read: all 6 of the 6 given ids, in full, with `aep plan artifact show story:<id>`. I also read `epic:foundation`, `aep plan artifact kinds`, `aep plan artifact lifecycle story`, `AGENTS.md`, `docs/plan/unit-invariants.md`, and design decisions 1, 2, 8, 10 and 11 (the other decisions I skimmed only by their headings). I checked these numbers again myself:
- The sorted `meta.tsv` gives sha256 `413ef567…919a` over 53801 lines.
- count.awk's directory rule gives 221 directories.
- The sections rule command prints 33 sections and 17010 cases.
- The test counts are 13 and 8, and the `expected.txt` line count is 221+1+22+1+33=278.
- The design heading "Its output on this document" exists at docs/design/engine.md:1211.

What I could not establish:
- spec-gate: what the last line of `ess` conform prints. surf's task only `tail`s the log, so the `<n> scenarios: … 0 unsupported` form is not confirmed. Its counts are measured when `spec/` lands (known, already queued).
- Not my lane, mentioned only: every acceptance except workspace-and-gate's joins several independent checks (task run, gate scripts, test suite, report content) with "and" into one sentence. In each case the story is done only when all of them pass, so I did not count it as a defect.
- Not my lane (design): test262-fetch's acceptance plants only a wrong tree hash. Nothing plants a wrong HEAD or a wrong ls-tree sum. I left this out as a question of how much the acceptance covers, not whether it can be checked.

```findings
- file: .engineering/planning/story/workspace-and-gate.md
  line: 123
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the summary says check refuses warnings \"shown by a planted test\", but no tests/gate/ script plants a warning or a denied unwrap(); as written, slot task check exits 0 the same way whether or not RUSTFLAGS=-D warnings and the four decision-13 clippy denials are in place"
- file: .engineering/planning/story/unicode-data-pin.md
  line: 93
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance sets <n> equal to the number of SOURCE lines, which the task already guarantees by construction, and it names no file count and no path; a directory holding only LICENSE and a one-line SOURCE would pass, so the 14 decision-8 files and the two emoji sequence files go unchecked"
- file: .engineering/planning/story/test262-expected-and-sections.md
  line: 150
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance checks only that there are 33 test262-section lines, each incomplete, and that 278 lines equal an expected.txt written from that same output; both hold whatever paths and omissions sections.txt has, so it names no section line's case or omission count (for example test262-section sec-string-objects incomplete 1223 0 1)"
```

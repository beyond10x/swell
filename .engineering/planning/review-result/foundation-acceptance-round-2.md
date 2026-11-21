---
format: aep.planning-md/3
id: review-result:foundation-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, foundation stories, round 2
relations:
- reviews: story:workspace-and-gate
- reviews: story:test262-fetch
- reviews: story:unicode-data-pin
- reviews: story:spec-gate
- reviews: story:test262-counting-runner
- reviews: story:test262-expected-and-sections
- reviews: story:host-requirements-gate
revision: 1
---
needs-revision
story:host-requirements-gate — the acceptance only checks that the list has `<r> omitted` lines, and the task takes any `omitted <reason>`; it names no check that each line reads `HOST-<n> omitted slice <s> not delivered` with the slice the document gives, so the "with the slice that will meet it" part of the epic's outcome goes unchecked — .engineering/planning/story/host-requirements-gate.md:108

**What I read:** all 7 of the 7 ids, in full, with `aep plan artifact show story:<id>`, plus the changed Outcome bullet and acceptance item 9 of `epic:foundation`. I re-measured these on the store worktree at `609b568`:
- **Slice rules:** the `swell-slices` block extracts to 253 rule lines and 34 exclude features.
- **Counting runner:** the story's count2.awk prints `test262 total 53801 8982 7 44812 0 44812 0`, `test262-slice 14 598 1192 112 7 479 0 479 0`, `test262-slice 21 1662 3324 1662 0 0 0 0 0` and `test262 built-ins/Atomics 389 121 7 261 0 261 0`, as the story says.
- **Sections:** the rule command prints 33 sections and 17010 cases.
- **Spec:** `ess --strict-requires specify validate --path spec` prints `swell v1 — 2 file(s), 3 scenario(s), valid`.
- **Markers:** `grep -r 'UNMAPPED:' spec | wc -l` prints 50.
- **Test counts:** 15 tests in the counting runner and 8 in expected-and-sections match the named tests.

The three round-1 findings are fixed:
- workspace-and-gate's acceptance now includes the planted-script lines `build-refuses-a-compiler-warning: ok` and `clippy-refuses-an-unwrap-in-an-engine-crate: ok`.
- unicode-data-pin's acceptance now names the 17 paths and `17 required path(s) present`.
- test262-expected-and-sections' acceptance now quotes five section lines in full, with their case and omission counts.

**What I could not establish:**
- The `<r>` of host-requirements-gate: `docs/design/host-interface.md` is not on `609b568` yet.
- The synthesize and conform lines of spec-gate: I did not re-run them, because each writes output files into the worktree and I write nothing.
- Not my lane (design): workspace-and-gate's clippy and build scripts make a sandbox workspace under `.out/` inside the root workspace. Whether cargo treats it as its own workspace root was not checked.

```findings
- file: .engineering/planning/story/host-requirements-gate.md
  line: 108
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance only checks that the list has <r> omitted lines, and the task takes any omitted <reason>; it names no check that each line reads HOST-<n> omitted slice <s> not delivered with the slice the document gives, so the \"with the slice that will meet it\" part of the epic's outcome goes unchecked"
```

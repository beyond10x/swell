---
format: aep.planning-md/3
id: review-result:slice-01-acceptance-round-1
kind: review-result
status: active
title: acceptance critic, slice 1 stories, round 1
relations:
- reviews: story:num-and-unicode-early-parts
- reviews: story:syntax-es5-and-strict
- reviews: story:heap-values-and-host-surface
- reviews: story:bytecode-compiler
- reviews: story:object-model-and-realms
- reviews: story:interpreter-and-script-entry-points
- reviews: story:harness-builtins
- reviews: story:host-interface-hardening
- reviews: story:test262-run-path
- reviews: story:ess-conformance-target
revision: 1
---
needs-revision
story:ess-conformance-target — the acceptance quotes no measured scenario count: `<p>` at least 1 and `<u>` at most an `ESS_UNSUPPORTED_MAX` the unit sets itself would also pass for a target that maps only `CreateAgent`, so it can't show that six commands are mapped. The suite on this base can be measured: 61 scenarios, 19 of which use only the six commands (`jq` over the `execute_command` steps of the synthesized `suite.json`) — .engineering/planning/story/ess-conformance-target.md:78
story:object-model-and-realms — the acceptance gives no number of `host_124_*` tests, and the gate's `*` match needs only one, so a single default test passes. `HOST-124` asks for one test per provided method in the section 16 table, and the acceptance does not say how many that is — .engineering/planning/story/object-model-and-realms.md:124

**What I read:** all 10 of the 10 stories, in full, with `aep plan artifact show story:<id>`, plus `epic:slice-01-harness`, `aep plan artifact list --kind story`, `taskfiles/host-requirements.yml` (its output lines), and `docs/design/host-interface.md` (section 16 table, `HOST-124`). I measured these myself:
- **Requirements:** `docs/design/host-interface.md` has 148 `**HOST-**` lines, 68 of them `[slice 1]`, and `requirements.txt` has 68 lines reading `pending slice 1`. So every story's test/pending/`1:<k>` arithmetic adds up: 17, 26, 54 and 63 test lines, leaving 51, 42, 14 and 5 pending in slice 1.
- **HOST lines per story:** 9 in object-model, 28 in the interpreter story, 13 in hardening with 9 moving now.
- **Named tests:** 11, 10, 7, 20 and 12, as the acceptances say.
- **`Zs`:** 17 code points.
- **Section case counts:** 67, 268, 240, 42, 92 and 48, none with a fixture file.
- **Moved cases:** 51 in all (1+23+2+16+6+1+2). The new slice lines 02, 03, 04, 06, 09, 12 and 13 equal the old ones plus the moved cases and runs.
- **`expected.txt`:** 278+3=281 lines, then 284.

My scratch directory `.out/critic-acceptance/` is removed.

**What I could not establish:**
- **`expected.txt` not on this branch:** the 281 and 284 rest on `story:test262-expected-and-sections`, which is still `active`.
- **The ESS model has changed:** synthesize now prints `61 scenario(s) (3 authored), 32 refusal(s)`, against 64 and 33 recorded by `story:spec-gate`. That is outside this set and is the coordinator's to update.
- **How many `host_124_*` tests:** the exact count depends on how "the three others" (time zones), the prepare and clean-up pair, and the `SUPPORTED_IMPORT_ATTRIBUTES` constant count as methods. By my reading it is 18 or 19; the story should state its own count.
- **Not counted as findings:** I left out the conditional counts in the interpreter and hardening acceptances, because they come from the known `HOST` splits (brief item 2). Also, the "over every source file" tests in the syntax, compiler and interpreter stories pass even when their source folder is empty. They have a clear before and after, so I kept that as a note on how much they cover, not a defect.

```findings
- file: .engineering/planning/story/ess-conformance-target.md
  line: 78
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance quotes no measured scenario count: <p> at least 1 and <u> at most an ESS_UNSUPPORTED_MAX the unit sets itself would also pass for a target that maps only CreateAgent, so it can't show that six commands are mapped; the suite on this base can be measured: 61 scenarios, 19 of which use only the six commands (jq over the execute_command steps of the synthesized suite.json)"
- file: .engineering/planning/story/object-model-and-realms.md
  line: 124
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance gives no number of host_124_* tests, and the gate's * match needs only one, so a single default test passes; HOST-124 asks for one test per provided method in the section 16 table, and the acceptance does not say how many that is"
```

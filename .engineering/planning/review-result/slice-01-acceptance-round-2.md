---
format: aep.planning-md/3
id: review-result:slice-01-acceptance-round-2
kind: review-result
status: active
title: acceptance critic, slice 1 stories, round 2
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
approve

**What I read:** all 10 of the 10 stories, in full, with `aep plan artifact show story:<id>`, plus `epic:slice-01-harness`, the output part of `taskfiles/host-requirements.yml`, and the current `conformance/host/requirements.txt`. On `wave/3-expected` I re-measured or recounted:
- **Requirement counts:** 159 requirements, 68 of them `[slice 1]`, and 50 `UNMAPPED:` markers. Pending by slice is now `1:68 2:5 3:5 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1`.
- **Gate lines per story:** each story's predicted lines follow from that. The heap story gives 17 test and `1:51`, the object model 26 and `1:42`, the interpreter 54 and `1:14`, and hardening 68 with no slice 1 entry. The rest of each line is unchanged.
- **Named tests and `expected.txt`:** the counts named in each acceptance match the Change sections (11, 12, 8, 17 `host_124_*`, 22, 14 plus 1 plus 4, 13). `expected.txt` goes 278+2=280, then +3=283.
- **ESS suite:** `ess verify conform synthesize` prints 61 scenarios. 19 of them use only the six slice 1 commands, and they split 10 / 3 / 5 / 1 exactly as the story lists, which gives `61 scenarios: 13 passed, 0 failed, 0 error, 48 unsupported`.
- **Delivered-slices check:** `HOST_REQ_DELIVERED` exists in the task.

The scratch directory `.out/critic-acceptance/` is removed.

**Both round-1 findings are fixed:**
- **`story:object-model-and-realms`:** its acceptance now has `cargo test --locked -p swell --test hooks -- --list` print 17 `host_124_` lines, and Requirements names all 17.
- **`story:ess-conformance-target`:** its acceptance now quotes the measured summary `61 scenarios: 13 passed, 0 failed, 0 error, 48 unsupported`, with 48 as the ceiling.

**What I could not establish:**
- **Line counts not yet checkable:** `expected.txt` and `sections.txt` are not on this branch yet, so the 280 and 283 rest on `story:test262-expected-and-sections`, as the brief says.
- **Not my lane (design):** the ESS target answers `unsupported` for the five outcomes the scenario input cannot produce. I did not check whether the `ess-conformance` runner accepts that as a defined answer.

```findings
[]
```

---
format: aep.planning-md/3
id: review-result:host-222-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s reading of the design line HOST-222 (wave big-2): approve'
relations:
- reviews: story:host-object-overrides
revision: 1
---
The coordinator's reading of the unit `host-222` of wave big-2, branch `impl/host-222` at
`248177d2` (base `wave/big-2` at `92ea4830`), 2026-11-26. No adversary ran: the unit changes two
files, a design document and the list of requirements, and no code.

Verdict: approve.

## What the diff holds (2 files, 9 lines added, 1 removed)

- `conformance/host/requirements.txt`: the line `HOST-222 pending slice 9` at the end.
- `docs/design/host-interface.md`: the requirement after HOST-114, word for word the sentence
  `story:host-object-overrides` proposes; one paragraph in section 15 that says every handle
  in an override's `Ok` or `Abrupt::Throw` value is checked before any invariant; and one
  sentence in section 6 that names HOST-222 as the requirement of that return site, beside
  HOST-162.

## What was checked

- The two greps of the story's start gate print 1 on `wave/big-2` after the merge
  (`f3d194e2`), read by the coordinator.
- The unit's runs, from its report: `task host-requirements-check` exits 0 with
  `222 requirement(s), 222 line(s): 165 test, 57 pending, 0 omitted` and
  `pending by slice: 8:1 9:7 11:32 12:3 13:2 14:6 16:5 18:1` (221, 56 and `9:6` on its base).
- The tag `swell:D6` and the model HOST-160 (a host function's answer) were checked by the
  unit against sections 6 and 15; the coordinator read the three added passages against the
  proposed sentence and found no difference.

## What stays open until the wave closes

`task host-surface-check` and `task site-data-check` fail on the wave branch from this merge
on: `CHANGELOG.md` does not name HOST-222 under `### Host interface` yet, and
`website/data/status.json` holds 221 requirements. Both files are the coordinator's and are
written once, at the join with wave big-1, where the same two files change.

```findings
[]
```

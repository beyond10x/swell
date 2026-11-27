---
format: aep.planning-md/3
id: review-result:slice-09-round-2-scope
kind: review-result
status: active
title: 'Slice 9, round 2, the scope critic: approve, no finding'
relations:
- reviews: story:keyed-entry-table
- reviews: story:reflect-namespace
- reviews: story:map-and-set
- reviews: story:proxy-objects-and-access-traps
- reviews: story:weak-map-and-weak-set
- reviews: story:set-methods
- reviews: story:proxy-descriptor-prototype-and-extensibility-traps
- reviews: story:map-upsert-and-group-by
- reviews: story:host-object-overrides
- reviews: story:slice-09-conformance-close
revision: 1
---
Plan critic, scope, round 2 on the ten draft stories of
`epic:slice-09-collections-proxy-reflect` (`plan/slice-09` at `4aca865a`), 2026-11-26.
Verdict: approve, no finding; the round-1 finding is closed. The critic's answer, as returned:

approve

**My round-1 finding.** It is closed. The epic's "Out of scope" excluded "host overrides of the internal methods", yet `story:host-object-overrides` claims them. The coordinator's list in `.engineering/planning/story/slice-09-conformance-close.md:66` now includes rewriting that line to "the Web IDL bindings that use the table: surf's", citing decision 9 and question D5. That is the wording I asked for.

**The revised set, judged again for scope.**
- **Promises:** I listed 20 and traced all 20, each to exactly one story:
  - From the epic: 5 outcomes, 3 open questions and 5 acceptance items.
  - From the host requirements: the 6 marked `pending slice 9` (HOST-25, 41, 112, 113, 114, 214), plus HOST-222.
- **Who claims what among the shared items:**
  - HOST-25 and HOST-214 are moved only by `map-and-set`. `keyed-entry-table:48` says "None closed".
  - HOST-112 and HOST-113 are closed only by `host-object-overrides`. Both proxy stories say "None closed" (`proxy-objects-and-access-traps.md:85`, `proxy-descriptor-prototype-and-extensibility-traps.md:62`).
  - `getOrInsert*` is split by receiver: `Map` in `map-upsert-and-group-by`, `WeakMap` in `weak-map-and-weak-set`.
  - `Map.groupBy` is claimed only by `map-upsert-and-group-by`.
- **Nothing the revision added reaches beyond the epic:**
  - `Heap::intern_string` and `Heap::atom_of` already exist (`crates/swell/src/heap/string.rs:289`, `:315`, from an implemented story). `keyed-entry-table` only uses them.
  - The release of cursors is written once (`keyed-entry-table`) and called once (`map-and-set`). `weak-map-and-weak-set` clears its entries in that same pass and does not add another.
  - The new arm in `object/array.rs` and the tracing of the facts box in `host-object-overrides` serve HOST-114: every internal method of a host object with an override table runs its override.
  - Turning `KIND_SITES` into a slice in `reflect-namespace` is a two-line change that makes room for the parallel stories, not a new outcome.
  - The doc comments at the head of three files are written by no story. The coordinator may add one sentence each (`slice-09-conformance-close.md:66`).
- **Case lists:** `git diff 33330e14..4aca865a -- docs/plan/slice-09/` changes only `check.sh` and `lists.sh`. So the round-1 partition still holds: 1,787 list cases = the 1,785 cases of slice 9 + 2 incoming, no case twice.
- **Removed cases:** each story that has a case list now says a case taken off its list goes to `close-later.tsv`. Check (7) of the close settles each such case, either passing or with a line in `moves.tsv`.

**What I read:** the 10 stories, the epic, my round-1 record, `revision-1.md`, `waves-section.md`, and the diffs `66f11d1b..4aca865a` (stories and scripts) and `33330e14..4aca865a` (lists and epic). I also used `rg` across the stories for host requirements, features and the new symbols, and checked `requirements.txt` and `crates/swell/src/heap/string.rs`.

**What I could not establish:**
- `scan.awk` (the script that assigns cases to lists) and `lists.sh` were not run again, because the brief allows no runs. The partition rests on the lists being byte-unchanged since round 1.
- I had no decomposer answer to read. `revision-1.md` is the brief the decomposer was given, so I judged each closure from the diff.
- Outside my lane (for the design critic or the coordinator): `waves-section.md` says `host-object-overrides` "adds HOST-222 by a design change before it starts", while your brief says you rewrite the HOST-222 design line yourself.

```findings
[]
```

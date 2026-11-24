---
format: aep.planning-md/3
id: review-result:slice-04-round-2-b-parallel-safety
kind: review-result
status: active
title: 'Slice 4, round 2, the parallel-safety critic: the twelve later stories'
relations:
- reviews: story:swell-num-date-text
- reviews: story:math-sum-precise
- reviews: story:number-functions
- reviews: story:swell-num-math-exp-and-log
- reviews: story:swell-num-math-trigonometry
- reviews: story:date-setters
- reviews: story:math-functions
- reviews: story:date-text-formats
- reviews: story:exponentiation-operator
- reviews: story:math-random
- reviews: story:slice-04-conformance-close
- reviews: story:spec-scenarios-for-the-numbers-and-dates-requirements
revision: 1
---
needs-revision — panel B (one blocker, `story:date-text-formats`)

- `story:swell-num-date-text`: approve
- `story:math-sum-precise`: approve
- `story:number-functions`: approve
- `story:swell-num-math-exp-and-log`: approve
- `story:swell-num-math-trigonometry`: approve
- `story:date-setters`: approve
- `story:math-functions`: approve
- `story:date-text-formats`: needs-revision
- `story:exponentiation-operator`: approve
- `story:math-random`: approve
- `story:slice-04-conformance-close`: approve
- `story:spec-scenarios-for-the-numbers-and-dates-requirements`: approve

These per-story verdicts follow the brief: only a blocker makes a story `needs-revision`. The rubric would count every warning as needs-revision. Paths below are relative to `.engineering/planning/story/`.

**Findings**

1. **Blocker, `date-text-formats`, lines 58 and 66.**
   - The story asserts an absolute count: `pending slice 4$` prints "5 after (6 before)", and `host-requirements-check` prints `4:6` then `4:5`.
   - The 6 only holds if HOST-135 and HOST-26/27/144 are still pending. `math-functions` (wave 4) moves HOST-135. `math-random` (wave 6) moves the others.
   - Of the 12 pending lines on the plan base, `date-object-clock-and-time-value` takes 2 and `date-object-constructor-now-utc-and-getters` takes 4. HOST-135 is then the 5th.
   - On the wave-5 base the count is 5, so the acceptance fails as written.
   - There is no `depends_on` edge from `date-text-formats` to `math-functions` or `math-random`.
   - Fix: state the count relative to the unit's own base ("one less", as `math-functions` and `math-random` do), or add the edges.

2. **Warning, `date-text-formats`, line 62.** Writes names `conformance/host/requirements.txt` (line 127) and `website/data/status.json` as shared only with slice 10. The tool lists seven collision lines for these two files against the `math-*` stories (the clock and zone story pairs are ordered by edges).
   - Neither `math-functions` nor `math-random` is named.
   - The lines are disjoint, so the join is safe, but the plan does not say so.
   - Remedy: name them and the line each takes, or add the edge.

3. **Warning, `math-functions`, line 64.** Writes names `requirements.txt` (line 135) and `status.json`. It names only slice-10 stories and `math-random` for `math.rs`, and does not name `date-object-clock-and-time-value`.
   - The tool reports `builtin_function.rs`, `requirements.txt` and `status.json` against the clock story, with no edge between them.
   - The `builtin_function.rs` hunks differ. The clock story takes `BuiltinId::Date` at lines 62 and 586, and `math-functions` takes a library test.
   - The date text stories take other lines of `requirements.txt`.
   - Remedy: name the clock, zone and text stories and each hunk, or add the edge.

4. **Warning, `math-functions`, line 54.** The test "`Math` has exactly the own keys the table and the functions give, in the order of definition" reads state that `math-sum-precise` (wave 2, no edge) and `math-random` also write. The story does not say whether the key list is derived from the table or written out, or where `sumPrecise` and `random` fall in the order.
   - Remedy: state the source of the expected keys and the order, or order the stories.

5. **Warning, `number-functions`, lines 56 and 64.**
   - It claims the header lines 10 to 16 of `builtins/number.rs`, and says `number-format` "rewrites the rest of it".
   - `number-format` line 51 rewrites the sentence "The rest of `Number` is slice 4", which is the same lines (15 to 17 in the wave tree).
   - There is no edge between the two stories. Two units rewrite one three-line sentence, and the split is stated differently in each body.
   - Remedy: say which story writes which lines in both bodies, or add an edge from `number-functions` to `number-format` with this file as the reason.

6. **Warning, `swell-num-math-exp-and-log` and `swell-num-math-trigonometry`, line 58 of each.**
   - Both write `crates/num/src/math.rs` (the `pub mod` and `pub use` lines) and `data/math/SOURCE` (one line). Both bodies name each other and the hunk correctly.
   - Neither scope block lists those two files, so the tool prints no collision line for this pair.
   - Remedy: add the two paths to both scopes.

7. **Warning, `swell-num-date-text`, lines 46 and 48.**
   - It lists the other stories that add a `swell-num` module, but names `story:swell-num-math`, which does not exist. It means the three `swell-num-math-*` stories, of which only `swell-num-math-double-double` adds `pub mod math` to `lib.rs`.
   - It is the only story that names the `lib.rs` header comment (lines 15 to 17: "Slice 4 adds `radix.rs` ... `math.rs` and `random.rs`"). `math-sum-precise`, `number-functions` and `math-random` say "one `pub mod` line" only.
   - Remedy: fix the ids, and have each module-adding story name the header hunk.

8. **Warning, `math-sum-precise` (line 55), `number-functions` (line 64), `math-random` (line 67), `swell-num-date-text`.** Each Writes section names `crates/num/src/lib.rs` and `crates/num/tests/main.rs` (and `status.json` for the first three), but none of the scope blocks lists them. The tool therefore shows none of the module-line collisions among the stories that add a `swell-num` module. Remedy: list the files in the scopes.

9. **Warning, `spec-scenarios-for-the-numbers-and-dates-requirements`, lines 55 and 59.**
   - Every scenario file must be listed under `scenarios:` in `spec/ess-inputs.yaml` (10 entries on the wave tree). Writes does not name that file.
   - Three stories outside the set write the same files:
     - `ess-target-checkpoint-and-promise-hooks` (active): `target.rs`, `host.yaml`, the floor in `conform-engine.yml`, and `spec/ess-inputs.yaml`.
     - `host-interface-scenarios-with-engine-evidence` (draft): `target.rs`, the floor in `conform-engine.yml`, `spec/scenarios/`.
     - `host-interface-spec-closes-design-gaps` (draft): `host.yaml`, `spec/scenarios/`, `conformance/ess/src/`.
   - The body names only `spec-diff-in-the-gate`.
   - Its floors are stale: "fewer than 14 passed" and `ESS_UNSUPPORTED_MAX` 44 and "8 files", against 18, 42 and 10 in the wave tree. Re-measure them before building.

10. **Note, line numbers.** The stories quote lines of the older base `c6d44d1`; the wave tree differs.

| Quoted line | Wave tree |
|---|---|
| `expression.rs:391` and `:465` | 412 and 486 |
| `bytecode.rs:723` | 790 |
| `builtin_function.rs:746` | 942 |
| `ops/iteration.rs` 173 and 198 | 216 and 241 |
| `RealmRecord` 109, 131, 319 | 114, 137, 326 |

    The wave tree has 14 pending slice-4 lines, since HOST-212 and HOST-213 are there. The units must re-read the numbers.

11. **Note, `math-random`, line 67.** It says `realm/mod.rs`, `host/agent.rs` and `host/limits.rs` are "written by none of the open stories". The store shows four stories with status `active` listing those paths: `promise-rejection-tracker-and-promise-reads`, `job-queue-and-microtask-checkpoint`, `promise-constructor-then-and-jobs` and `for-await-and-async-from-sync-iterator`. Their commits are on `wave/s10-1`, so the status is probably stale. The unit's `git log` check covers it.

**Waves and open points**
- No two panel-B stories in one wave share a source file without naming it.
- The wave-2 and wave-3 module-line pairs, and `exp-and-log` with `trigonometry`, name each other and the hunk.
- Wave 4 (`date-setters` with `math-functions`), wave 5 (`date-text-formats` with `exponentiation-operator`) and wave 7 (`slice-04-conformance-close` with the spec-scenarios story) share only generated files.
- `math-sum-precise` has no edge to `math-functions` or `math-random`. All three bodies name `math.rs` and agree that the join takes all three. That is a named, deliberate collision, and a person decides.
- Open point 1 (HOST-147): `math-random` handles both branches, and the close handles "0 or 1". No file collision.
- Open point 4 (the data split): `exp-and-log` and `trigonometry` agree on one file each plus a `SOURCE` line.
- Open point 6: the edge to `slice-10-conformance-close` exists. It is active, and the same five files are named.

**What I read**
- All 12 panel-B stories whole.
- The scope, Writes and relations of the 6 panel-A stories and of the open slice-10 stories on shared paths.
- `aep plan artifact waves --kind story --status draft`.
- The wave-tree code, for line numbers, `spec/ess-inputs.yaml`, `taskfiles/conform-engine.yml` and the `host_187` test.

**Surfaces:** 12 cited from the bodies' Writes sections, 0 inferred, 0 unplaced. The tool marks most of them "inferred" only because of their scope metadata.

**Acceptance reads:** 43 traced to a producer in the set; `depends_on` edges record 41. The two unrecorded are finding 1 and finding 4.

**Not established**
- Whether the `host_186`/`host_187` ceiling (506,112 bytes) survives all the function additions together. Each story reports its own reading, and nobody owns the join. I did not build or run anything.
- Whether `math-random`'s `RealmRecord` hunks are free of slice-10 changes, beyond the commit log.
- `spec-diff-in-the-gate` and the other two spec stories have no scope. I judged them only by their Writes lines.
- Not my lane: cross-slice design edits to `engine.md` (design author) and the `toLocale*String` and `parseInt` decisions (design critic).

```findings
- file: .engineering/planning/story/date-text-formats.md
  line: 58
  category: parallel-safety
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the acceptance reads an absolute count of pending slice 4 lines (6 before, 5 after) that math-functions (HOST-135, wave 4) and math-random (HOST-26, 27, 144, wave 6) write, with no depends_on edge, so on the wave-5 base it reads 5 and the check fails; state it relative or add the edges"
- file: .engineering/planning/story/date-text-formats.md
  line: 62
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Writes names requirements.txt line 127 and status.json as shared only with slice 10, not with math-functions and math-random, which the tool lists on the same files with no edge"
- file: .engineering/planning/story/math-functions.md
  line: 64
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Writes does not name the clock, zone and text stories that share requirements.txt, status.json and (clock) builtin_function.rs with no edge, nor the hunk each takes"
- file: .engineering/planning/story/math-functions.md
  line: 54
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the own-keys test of Math reads the table that math-sum-precise and math-random also extend, and the body does not say whether the expected keys come from the table or are written out, nor the place of sumPrecise and random in the order"
- file: .engineering/planning/story/number-functions.md
  line: 64
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "it claims header lines 10 to 16 of builtins/number.rs and says number-format rewrites the rest, but number-format line 51 rewrites the same sentence, and the two stories have no edge"
- file: .engineering/planning/story/swell-num-math-exp-and-log.md
  line: 58
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the scope block omits crates/num/src/math.rs and data/math/SOURCE, which the body shares with swell-num-math-trigonometry, so aep waves prints no collision line for the pair"
- file: .engineering/planning/story/swell-num-math-trigonometry.md
  line: 58
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the scope block omits crates/num/src/math.rs and data/math/SOURCE, which the body shares with swell-num-math-exp-and-log, so aep waves prints no collision line for the pair"
- file: .engineering/planning/story/swell-num-date-text.md
  line: 48
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the list of stories that add a swell-num module names story:swell-num-math, which does not exist, and the lib.rs header hunk it names is not named by the other module-adding stories"
- file: .engineering/planning/story/math-sum-precise.md
  line: 55
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Writes names crates/num/src/lib.rs, crates/num/tests/main.rs and status.json but the scope block lists none of them, so the tool shows none of the collisions with the other stories that add a swell-num module"
- file: .engineering/planning/story/math-random.md
  line: 67
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Writes names crates/num/src/lib.rs, crates/num/tests/main.rs and requirements.txt but the scope block lists only requirements.txt; and the claim that realm/mod.rs and host/agent.rs are written by no open story disagrees with four active stories in the store (their commits are on wave/s10-1)"
- file: .engineering/planning/story/spec-scenarios-for-the-numbers-and-dates-requirements.md
  line: 59
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Writes omits spec/ess-inputs.yaml, which every new scenario file must be listed in, and names only spec-diff-in-the-gate as sharing target.rs, host.yaml and the conform-engine.yml floor while ess-target-checkpoint-and-promise-hooks, host-interface-scenarios-with-engine-evidence and host-interface-spec-closes-design-gaps write the same files; its floors (14, 44, 8 files) are stale against 18, 42 and 10 on the wave tree"
- file: .engineering/planning/story/exponentiation-operator.md
  line: 43
  category: parallel-safety
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the quoted lines (expression.rs 391 and 465, bytecode.rs 723) are of the older base; the wave tree has 412, 486 and 790, so the unit must re-read them"
```

---
format: aep.planning-md/3
id: review-result:buffer-growth-round-1-design
kind: review-result
status: active
title: Buffer growth outside the charged list, round 1, the design critic
relations:
- reviews: story:buffer-growth-outside-the-charged-list-is-listed
revision: 1
---
needs-revision

Findings (design lane; the acceptance lane's points are marked):

1. Coverage claim, draft lines 7-13 and 59-60, `## Not in this story` lines 125-133. The head says every call that grows a standard buffer "names its charge or fails the gate". The list holds only 4 reservation paths and `to_vec`.
   - Unlisted growth exists today in `crates/swell/src`: `.resize(` (11 lines, e.g. `heap/arena.rs` `Arena::grow` `marks.resize`), `.extend_from_slice(` (14), `.extend(` (29), `.reserve(` (6, including `ops/objects.rs` `visited.keys.reserve(1)`), `.repeat(` (15), `vec![` (32) and `into_boxed_slice` (8).
   - Also unlisted: `String` methods, `VecDeque::try_reserve` and `with_capacity`, `String::with_capacity`.
   - Say the list guards reservation points only. Either add `Vec::reserve`, `reserve_exact`, `String::try_reserve*` and `VecDeque::try_reserve` (all cheap), or state why each is left out.
   - `Not in this story` must also name where the lint does not reach: macro-expanded growth, `resize`/`extend*`/`repeat`/`clone`, `swell` code behind `fault-injection` or `collect-every-safe-point` (the gate builds without features), and every other crate. Today it names only `push`, `collect()`, `HashMap` and `BTreeSet`, and gives no reason.

2. `to_vec` fallback, draft lines 60-61 and 120-121. If clippy has no path for `<[T]>::to_vec`, "held by a grep with its count in the acceptance" is a one-time reading. A later `.to_vec()` goes unguarded, yet the `AGENTS.md` sentence (line 83) still promises that `to_vec` needs an `expect`. Pick one:
   - a gate-run scan, such as a test beside `crates/swell/tests/source_scan.rs`. That contradicts "No new test function" (line 102), so the draft must say so.
   - drop `to_vec` from the `AGENTS.md` sentence and mark it unguarded.
   - the draft should also say whether `to_vec` is verified to resolve in the path form, since this base was never run.

3. Unfulfilled expectations are unused. `unfulfilled_lint_expectations` is warn-by-default, so `-D warnings` turns a stale `expect` red. The draft names only the first direction, an unannotated call (item 4, line 78).
   - Add the second direction to the acceptance. An `expect` on a non-flagged call gives exactly one `unfulfilled_lint_expectations` and a non-zero exit.
   - Say that a missing or misnamed `crates/swell/clippy.toml` turns every `expect` red.

4. Granularity, item 2, line 62 ("smallest item"). Several sites share one function, e.g. `heap/arena.rs` `Arena::grow` has 4 and `object/arguments.rs:211` has 2. One `expect` on the function covers all of them, and a later call added there inherits it without its own reason.
   - Require the `expect` on the statement (`let`, `if`, expression statement) where one is allowed.
   - Where a function-level `expect` is used, its reason names every site it covers.
   - The negative control (line 117) must name one single-site `expect`. Removing the one on `Arena::grow` gives 4 warnings, not 1.

5. Config drift, lines 25-28 and 52-53. `crates/swell/clippy.toml` repeats the root file's four keys (they match the root `clippy.toml` today). Nothing keeps them in step, and a rule of the project bars comments in config.
   - The `AGENTS.md` sentence (lines 83-87) must tell the author to change both files together.
   - A cheap check on the four lines would be better.

6. Acceptance (acceptance lane), line 111: `grep -c 'disallowed_methods'` is wrong. Clippy prints the lint name once, in the first warning's `#[warn(clippy::disallowed_methods)] on by default` note. Every warning's message reads "use of a disallowed method `…`". So the command reads 1, not N. Count `grep -c 'use of a disallowed method'`, or use `--message-format=json` and count by code.

7. Acceptance (acceptance lane), line 115: `grep -rn 'expect(clippy::disallowed_methods'` misses rustfmt's wrapped form. The repo already wraps long reasons across lines, e.g. `ops/conversion.rs` `#[expect(` / `clippy::wrong_self_convention,`. It also cannot equal the site count when one `expect` covers several sites (finding 4). Match `clippy::disallowed_methods,` on its own line as well, or drop the equality and rely on the clippy count.

8. Acceptance (acceptance lane), line 104: "no step, byte or result moves" has no command. Name one. `slot cargo nextest run --locked -p swell --profile default` before and after, with the same pass count. The single `--lib` run on line 122 omits the `tests/` crate, which also gets edits.

9. Line 74: the unit-test `allow` does not reach the `tests/` crate, so the `expect` sites there are correct. Note that `tests/source_scan.rs` and `tests/builtins_uri.rs` use `String::with_capacity`, which is not listed. The `grep -rlE` at line 94 therefore over-lists files, and the writes must be "flagged sites only".

10. Relations (note, not a blocker). Only `depends_on: copies-held-…` is recorded. The "every merged slice-4 unit" ordering lives in prose (line 1). Say that the coordinator checks the base before starting. The new `crates/swell/clippy.toml` collides with no other story. The root file belongs to `workspace-and-gate`. `crates/swell/src/lib.rs` is claimed by no other story, though its header says no later story edits it.

```findings
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 7
  category: coverage
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The head claims every growing call fails the gate, but the list covers only reserve forms and to_vec; resize, extend_from_slice, extend, reserve, repeat, vec!, String and VecDeque growth, macros and feature-gated code are outside it and the Not-in-this-story section does not say so.'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 60
  category: coverage
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The to_vec fallback is a one-time grep that does not hold later sites, while the AGENTS.md sentence promises enforcement; hold it by a gate-run scan or take it out of the sentence, and say whether the path form was verified.'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 78
  category: gate
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The draft never states that unfulfilled_lint_expectations under -D warnings turns a stale expect red; add that direction and a second negative control (an expect on a non-flagged call gives one warning and a non-zero exit).'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 62
  category: granularity
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'An expect on the smallest item can be a whole function holding several sites (Arena::grow has 4), so a later call there is silently covered; require statement level or a reason naming each site, and pick a single-site negative control.'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 52
  category: config
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The repeated root keys can drift and config files may not carry comments; the AGENTS.md sentence must tell an author to change both clippy.toml files together, or a check must compare the four keys.'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 111
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'grep -c disallowed_methods reads 1, because clippy prints the lint name once in the first warning note; count the message text "use of a disallowed method" or the JSON code instead.'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 115
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The grep for expect(clippy::disallowed_methods misses the rustfmt-wrapped multi-line form and cannot equal the site count when one expect covers several sites; match the wrapped form and drop the equality.'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 104
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The claim that no step, byte or result moves has no command; name a before and after nextest run of the swell package including tests/, not only --lib.'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 94
  category: tests
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The grep over crates/swell/tests also matches unlisted String::with_capacity and to_vec uses; the writes are the flagged sites only.'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 1
  category: relations
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: 'Only the first story is a recorded edge; the slice-4 ordering lives in prose, so say the coordinator checks the base before the unit starts.'
```

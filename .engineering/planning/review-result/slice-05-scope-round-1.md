---
format: aep.planning-md/3
id: review-result:slice-05-scope-round-1
kind: review-result
status: active
title: scope critic, the six stories of the first wave of slice 5, round 1
relations:
- reviews: story:regexp-crate-and-pattern-parser
- reviews: story:string-replace-replace-all-and-split
- reviews: story:regexp-literal-early-errors
- reviews: story:regexp-compiler-and-backtracking-matcher
- reviews: story:regexp-objects-exec-and-literals
- reviews: story:regexp-symbol-methods-and-string-match-search
revision: 1
---
needs-revision

epic:slice-05-regexp — the open item "How B.1.2 (decision 7, in this slice) and slice 18 ("Annex B") divide the Annex B regular expression cases between them" is listed "to be settled by this epic's first story", yet no drafted story settles it or says who will. `story:regexp-crate-and-pattern-parser` delivers B.1.2, so it should either settle the item or name the conformance close as the place it is settled. Today only `story:regexp-symbol-methods-and-string-match-search` touches it, and only for `Symbol.split`, by placing those cases in slice 18 without saying so (the block line `dir test/annexB/ 18` puts the 62 `test/annexB/built-ins/RegExp` and 8 `test/annexB/language/literals/regexp` cases in slice 18) — .engineering/planning/epic/slice-05-regexp.md:51

**What I read:** the epic and all 6 stories in full (`aep plan artifact show`), `aep plan artifact graph` (nothing outside the set decomposes the epic) and `aep plan artifact relations`. I also read `docs/plan/slice-05/` (`consumer-regexps.tsv`, `scan.tsv` and the four case lists), decision 7 in `docs/design/engine.md`, the `dir test/annexB/ 18` line, the HOST-142/143/164/165 entries, `crates/unicode/Cargo.toml` (lints) and `crates/unicode/src/lib.rs` (`is_space_separator`).

**Promises:** I took 33 from the epic.
- 19 are claimed by a story of this set:
  - parser with the 22.2.1 and B.1.2 early errors, compiler, matcher with an explicit stack
  - named and duplicate groups, look-behind, flags `d g m s u y`, the `m` and `s` modifiers, Annex B grammar
  - nesting bound, literal checked from `swell-syntax`, `RegExp`, `replace`/`replaceAll`/`split`/`match`/`search`
  - nothing third-party, steps and backtrack stack charged with a limit test, no panics, omission recorded, clause cited
  - no regression of earlier passes, epic acceptances 5 and 6
- 13 are later waves under brief notes 1 and 2:
  - Unicode tables and `swell-unicode-gen`, the emoji files, the licence, which string methods use the tables
  - case folding and `i`, `v` and set operations, property escapes, `RegExp.escape`
  - `matchAll` and `RegExpStringIteratorPrototype`
  - the RangeError wording, the slice-line count, committing `expected.txt`, rule-line moves, the table regeneration gate
- 1 is not traced: the finding above.

The four case lists share no case. Every failing slice-5 case in `scan.tsv` is assigned to a story tag or to `later`. I found no reach beyond the parent and no outcome claimed twice. Epic acceptance 5 is claimed by `story:regexp-objects-exec-and-literals` alone; `story:regexp-compiler-and-backtracking-matcher` only tests it inside the crate.

**What I could not establish:**
- Whether some ruling outside the epic and `docs/design/` already settles the Annex B division. `grep` of `engine.md`, `waves.md`, `unit-invariants.md` and `AGENTS.md` for `B.1.2` and "open item" found none.
- Out of my lane, without counting it: I did not check whether the acceptance awk commands are sound, or whether the steps can run side by side.
- My one scratch file was written to `.out/critic-scope-stories.txt`, not under `.out/critic-scope/`. I removed it before this report.

```findings
- file: .engineering/planning/epic/slice-05-regexp.md
  line: 51
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the open item \"How B.1.2 (decision 7, in this slice) and slice 18 (\\\"Annex B\\\") divide the Annex B regular expression cases between them\" is to be settled by this epic's first story, and no drafted story settles it or names who does; story:regexp-crate-and-pattern-parser, which delivers B.1.2, is the natural taker (settle it, or name the conformance close), while only story:regexp-symbol-methods-and-string-match-search implicitly assigns the Symbol.split Annex B cases to slice 18"
```
